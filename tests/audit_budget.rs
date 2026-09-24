//! Regression tests for deterministic JSON Schema evaluation bounds (S-11).

use std::time::{Duration, Instant};

use colander::schema::validate_text;

const LIMIT_MESSAGE: &str = "SCHEMA_EVALUATION_LIMIT: schema evaluation exceeded the step budget";
const DEPTH_MESSAGE: &str = "SCHEMA_DEPTH_LIMIT: schema evaluation exceeded the nesting depth";
const ATTACK_CEILING: Duration = Duration::from_secs(2);

/// The `$defs` block of a shared-reference chain: every level lists the next
/// reference twice inside `keyword`, and the last level inlines `leaf`.
fn shared_ref_defs(keyword: &str, depth: usize, leaf: &str) -> String {
    let mut definitions = String::new();
    for level in 0..depth {
        if level > 0 {
            definitions.push(',');
        }
        let next = level + 1;
        let body = if next == depth {
            leaf.to_string()
        } else {
            format!(
                r##"{{"{keyword}":[{{"$ref":"#/$defs/n{next}"}},{{"$ref":"#/$defs/n{next}"}}]}}"##
            )
        };
        definitions.push_str(&format!(r#""n{level}":{body}"#));
    }
    definitions
}

/// The same chain, rooted so the whole document is the reference target.
fn shared_ref_chain(keyword: &str, depth: usize, leaf: &str) -> String {
    let mut schema = String::from(r##"{"$ref":"#/$defs/n0","$defs":{"##);
    schema.push_str(&shared_ref_defs(keyword, depth, leaf));
    schema.push_str("}}");
    schema
}

fn if_then_dag(depth: usize) -> String {
    let mut definitions = String::new();
    for level in 0..depth {
        if level > 0 {
            definitions.push(',');
        }
        let next = level + 1;
        let body = if next == depth {
            "true".to_string()
        } else {
            format!(
                r##"{{"if":{{"$ref":"#/$defs/n{next}"}},"then":{{"$ref":"#/$defs/n{next}"}}}}"##
            )
        };
        definitions.push_str(&format!(r#""n{level}":{body}"#));
    }
    format!(r##"{{"$ref":"#/$defs/n0","$defs":{{{definitions}}}}}"##)
}

fn assert_budget_error(schema: &str) {
    let started = Instant::now();
    let error = validate_text(schema, "{}", "instance")
        .expect_err("the amplification shape must exceed its deterministic budget");
    let elapsed = started.elapsed();
    assert!(
        elapsed < ATTACK_CEILING,
        "schema evaluation took {elapsed:?}, over the {ATTACK_CEILING:?} ceiling"
    );
    assert!(
        error.message.contains(LIMIT_MESSAGE),
        "expected a visible limit error, got {error}"
    );
}

#[test]
fn shared_ref_any_of_amplification_is_bounded() {
    assert_budget_error(&shared_ref_chain("anyOf", 26, r#"{"type":"string"}"#));
}

#[test]
fn shared_ref_all_of_error_collection_is_bounded() {
    let error = validate_text(
        &shared_ref_chain("allOf", 26, r#"{"type":"string"}"#),
        "{}",
        "instance",
    )
    .expect_err("the allOf shape must fail closed");
    assert!(
        error.message.contains(LIMIT_MESSAGE),
        "expected a limit error, got {error}"
    );
    assert!(
        error.message.contains("truncated: 5 of 1000 errors shown"),
        "expected bounded error rendering, got {error}"
    );
}

#[test]
fn if_then_dag_amplification_is_bounded() {
    assert_budget_error(&if_then_dag(25));
}

#[test]
fn legitimate_large_instance_stays_within_budget() {
    validate_text(
        include_str!("../schemas/golden-vectors.schema.json"),
        include_str!("golden/vectors/validate.json"),
        "instance",
    )
    .expect("the shipped golden-vectors workload remains valid");
}

#[test]
fn shared_dag_is_not_mistaken_for_a_budget_failure() {
    validate_text(&if_then_dag(5), "{}", "instance")
        .expect("a bounded shared DAG is valid within the budget");
}

#[test]
fn recursive_ref_is_still_rejected_by_classification() {
    let error = validate_text(r##"{"$ref":"#"}"##, "{}", "instance")
        .expect_err("a recursive reference remains invalid")
        .to_string();
    assert!(error.contains("recursive reference '#'"), "{error}");
    assert!(!error.contains(LIMIT_MESSAGE), "{error}");
}

#[test]
fn limit_error_is_byte_stable_across_repeated_calls() {
    let schema = shared_ref_chain("anyOf", 26, r#"{"type":"string"}"#);
    let first = validate_text(&schema, "{}", "instance")
        .expect_err("the amplification shape must fail")
        .to_string();
    let second = validate_text(&schema, "{}", "instance")
        .expect_err("the amplification shape must fail again")
        .to_string();
    assert_eq!(first, second);
}

/// The budget must fail closed even when a later branch would have matched.
/// `anyOf` stops at the first branch that passes, so an exhaustion recorded
/// only inside a discarded branch's error vector would be lost and the
/// instance would be reported valid without the schema ever being evaluated.
#[test]
fn exhaustion_is_not_hidden_by_a_short_circuit() {
    // The chain and the always-matching branch share one root, so the
    // `$defs` resolve: branch 0 exhausts the budget, branch 1 would pass.
    let definitions = shared_ref_defs("anyOf", 20, r#"{"type":"string"}"#);
    for combinator in ["anyOf", "oneOf", "allOf"] {
        let schema = format!(
            r##"{{"$defs":{{{definitions}}},"{combinator}":[{{"$ref":"#/$defs/n0"}},true]}}"##
        );
        let error = validate_text(&schema, "{}", "instance")
            .expect_err("exhaustion must fail closed, not pass behind a matching branch")
            .to_string();
        assert!(
            error.contains(LIMIT_MESSAGE),
            "{combinator}: exhaustion was hidden, got {error}"
        );
    }
    // `not` and `if` evaluate their operand completely, so they cannot hide it
    // either; both must still surface the limit.
    for keyword in ["not", "if"] {
        let schema =
            format!(r##"{{"$defs":{{{definitions}}},"{keyword}":{{"$ref":"#/$defs/n0"}}}}"##);
        let error = validate_text(&schema, "{}", "instance")
            .expect_err("exhaustion must fail closed")
            .to_string();
        assert!(error.contains(LIMIT_MESSAGE), "{keyword}: got {error}");
    }
}

/// A deep chain of *distinct* definitions is not recursive, so S-10 accepts it
/// and the step budget is sized for the instance rather than the depth. It
/// still recurses once per level, and an unbounded recursion aborts the process
/// with a stack overflow, so nesting needs its own bound.
#[test]
fn deep_reference_chain_is_bounded_not_aborted() {
    let mut definitions = String::new();
    for level in 0..30_000 {
        if level > 0 {
            definitions.push(',');
        }
        definitions.push_str(&format!(
            r##""n{level}":{{"$ref":"#/$defs/n{}"}}"##,
            level + 1
        ));
    }
    let mut schema = String::from(r##"{"$ref":"#/$defs/n0","$defs":{"##);
    schema.push_str(&definitions);
    schema.push_str(r##","leaf":{"type":"string"}}"##);
    schema.push('}');

    let started = Instant::now();
    let error = validate_text(&schema, "{}", "instance")
        .expect_err("a 30 000-deep reference chain must fail closed, not abort")
        .to_string();
    assert!(
        started.elapsed() < ATTACK_CEILING,
        "deep chain took {:?}, over the {ATTACK_CEILING:?} ceiling",
        started.elapsed()
    );
    assert!(
        error.contains(DEPTH_MESSAGE),
        "expected a depth error, got {error}"
    );
    assert!(
        !error.contains(LIMIT_MESSAGE),
        "the step budget was not the limit: {error}"
    );
}

/// The nesting bound must not reject a chain a human could plausibly write.
#[test]
fn a_moderately_deep_reference_chain_still_evaluates() {
    let mut definitions = String::new();
    for level in 0..200 {
        if level > 0 {
            definitions.push(',');
        }
        if level == 199 {
            definitions.push_str(r#""n199":{"type":"integer"}"#);
        } else {
            definitions.push_str(&format!(
                r##""n{level}":{{"$ref":"#/$defs/n{}"}}"##,
                level + 1
            ));
        }
    }
    let mut schema = String::from(r##"{"$ref":"#/$defs/n0","$defs":{"##);
    schema.push_str(&definitions);
    schema.push_str("}}");
    // The chain is pure references, so the instance is whatever the leaf
    // asserts; the depth comes from the schema, not from the instance.
    validate_text(&schema, "7", "instance").expect("a 200-deep chain is within the nesting bound");
}

#[test]
fn collected_errors_are_capped_before_rendering() {
    let instance = format!("[{}]", vec!["null"; 1001].join(","));
    let error = validate_text(r#"{"type":"array","items":false}"#, &instance, "instance")
        .expect_err("the repeated false items must fail");
    assert!(
        error.message.contains("truncated: 5 of 1000 errors shown"),
        "{error}"
    );
}

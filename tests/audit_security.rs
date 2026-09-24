//! Regression tests for the findings of the adversarial security audit.
//!
//! Each test pins one invariant that was broken before the audit round:
//! malformed rules documents, aggregates aimed at a repeater child, integer
//! arithmetic that rounded, calculated values that skipped their field's own
//! constraints, and an over-budget `pattern`.
//!
//! The `pattern` cases also pin the budget's *headroom*: a realistic rule must
//! keep working, or the budget is a breaking change rather than a bound.

use colander::pattern::{MAX_PATTERN_UNITS, complexity_units, is_within_budget};
use colander::schema::validate_text;

fn form(fields: &str) -> String {
    format!(r#"{{"schemaVersion":"1.0.0","fields":{fields}}}"#)
}

fn rules(fields: &str) -> String {
    format!(r#"{{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{fields}}}"#)
}

/// A JSON string literal. Fixtures are written as multi-line raw strings, so
/// this has to escape the control characters too or the request is invalid.
fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other if (other as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", other as u32)),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn evaluate(form_json: &str, rules_json: &str, values: &str) -> String {
    let envelope = colander::ffi::session::call_with_text(
        &format!(
            r#"{{"formSchemaJson":{},"rulesSchemaJson":{},"values":{}}}"#,
            json_string(form_json),
            json_string(rules_json),
            values
        ),
        colander::ffi::rules::colander_evaluate_rules,
    );
    assert!(!envelope.is_empty(), "the entry point returned nothing");
    envelope
}

fn validate_response(form_json: &str, rules_json: &str, answers: &str) -> String {
    let envelope = colander::ffi::session::call_with_text(
        &format!(
            r#"{{"formSchemaJson":{},"rulesSchemaJson":{},"answersJson":{},"mode":"Complete"}}"#,
            json_string(form_json),
            json_string(rules_json),
            json_string(answers)
        ),
        colander::ffi::response::colander_validate_response,
    );
    assert!(!envelope.is_empty(), "the entry point returned nothing");
    envelope
}

const REPEATER_FORM: &str = r#"[{"id":"lines","code":"lines","type":"repeater","items":[{"id":"qty","code":"qty","type":"number"}]},
    {"id":"n","code":"n","type":"integer","readOnly":true},
    {"id":"t","code":"t","type":"number","readOnly":true}]"#;
const THREE_ROWS: &str = r#"{"lines":[{"qty":1},{"qty":2},{"qty":3}]}"#;

/// A malformed container must be refused. It used to be accepted and to
/// silently apply no rules at all, so the caller believed they were enforced.
#[test]
fn a_malformed_rules_container_is_refused() {
    let form_json = form(r#"[{"id":"a","code":"a","type":"integer"}]"#);
    for malformed in [r#"[]"#, "true", r#""x""#, "7"] {
        let envelope = evaluate(&form_json, &rules(malformed), r#"{"a":1}"#);
        assert!(
            envelope.contains("RULE_FIELDS_NOT_OBJECT"),
            "fields: {malformed} was accepted: {envelope}"
        );
    }
    // The same for `validations`.
    let rules_json = format!(
        r#"{{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{{}},"validations":{}}}"#,
        "{}"
    );
    let envelope = evaluate(&form_json, &rules_json, r#"{"a":1}"#);
    assert!(
        envelope.contains("RULE_VALIDATIONS_NOT_ARRAY"),
        "a non-array validations was accepted: {envelope}"
    );
}

/// An absent container is still fine: rules are optional.
#[test]
fn an_absent_rules_container_is_still_accepted() {
    let form_json = form(r#"[{"id":"a","code":"a","type":"integer"}]"#);
    let envelope = evaluate(
        &form_json,
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0"}"#,
        r#"{"a":1}"#,
    );
    assert!(envelope.contains("\"ok\":true"), "{envelope}");
}

/// An aggregate names a repeater. Pointing one at a repeater *child* used to
/// produce a silently wrong `0` instead of an error.
#[test]
fn an_aggregate_aimed_at_a_child_is_refused() {
    let form_json = form(REPEATER_FORM);
    let count_child = evaluate(
        &form_json,
        &rules(r#"{"n":{"calculate":{"op":"count","args":[{"ref":"qty"}]}}}"#),
        THREE_ROWS,
    );
    assert!(
        count_child.contains("RULE_AGGREGATE_NOT_REPEATER"),
        "{count_child}"
    );
    let sum_child_first = evaluate(
        &form_json,
        &rules(r#"{"t":{"calculate":{"op":"sum","args":[{"ref":"qty"},{"ref":"lines"}]}}}"#),
        THREE_ROWS,
    );
    assert!(
        sum_child_first.contains("RULE_AGGREGATE_NOT_REPEATER"),
        "{sum_child_first}"
    );
}

/// The documented aggregate shapes keep working: `count` over the repeater, and
/// `sum` over the repeater's child column. `sum` takes (repeater, child), so the
/// child in the second argument is correct and must not be refused.
#[test]
fn the_documented_aggregate_shapes_still_work() {
    let form_json = form(REPEATER_FORM);
    let counted = evaluate(
        &form_json,
        &rules(r#"{"n":{"calculate":{"op":"count","args":[{"ref":"lines"}]}}}"#),
        THREE_ROWS,
    );
    assert!(counted.contains(r#""n":3"#), "{counted}");
    let summed = evaluate(
        &form_json,
        &rules(r#"{"t":{"calculate":{"op":"sum","args":[{"ref":"lines"},{"ref":"qty"}]}}}"#),
        THREE_ROWS,
    );
    assert!(summed.contains(r#""t":6"#), "{summed}");
}

/// Integer arithmetic must not round through `f64`. Both cases used to come
/// back wrong: `2^53+1 + 1` lost the increment, and `i64::MAX-1 + 1` was
/// reported as a double one greater than `i64::MAX`.
#[test]
fn integer_arithmetic_does_not_round() {
    let form_json = form(r#"[{"id":"c","code":"c","type":"integer","readOnly":true}]"#);

    let over_two_pow_53 = evaluate(
        &form_json,
        &rules(r#"{"c":{"calculate":{"op":"add","args":[{"lit":9007199254740993},{"lit":1}]}}}"#),
        "{}",
    );
    assert!(
        over_two_pow_53.contains(r#""c":9007199254740994"#),
        "2^53+1 + 1 was rounded: {over_two_pow_53}"
    );

    let at_i64_max = evaluate(
        &form_json,
        &rules(
            r#"{"c":{"calculate":{"op":"add","args":[{"lit":9223372036854775806},{"lit":1}]}}}"#,
        ),
        "{}",
    );
    assert!(
        at_i64_max.contains(r#""c":9223372036854775807"#),
        "i64::MAX - 1 + 1 was not exact: {at_i64_max}"
    );

    // Subtraction and multiplication take the same exact path.
    let subtracted = evaluate(
        &form_json,
        &rules(
            r#"{"c":{"calculate":{"op":"sub","args":[{"lit":1},{"lit":9223372036854775807}]}}}"#,
        ),
        "{}",
    );
    assert!(
        subtracted.contains(r#""c":-9223372036854775806"#),
        "{subtracted}"
    );
    // 3037000499^2 = 9223372030926249001, past 2^53 and not a double.
    let multiplied = evaluate(
        &form_json,
        &rules(
            r#"{"c":{"calculate":{"op":"mul","args":[{"lit":3037000499},{"lit":3037000499}]}}}"#,
        ),
        "{}",
    );
    assert!(
        multiplied.contains(r#""c":9223372030926249001"#),
        "a product past 2^53 was rounded: {multiplied}"
    );
}

/// The documented result type is unchanged wherever the old value was right:
/// an integer a double holds exactly still serializes as before.
#[test]
fn small_integer_arithmetic_keeps_its_documented_shape() {
    let form_json = form(r#"[{"id":"c","code":"c","type":"integer","readOnly":true}]"#);
    let envelope = evaluate(
        &form_json,
        &rules(r#"{"c":{"calculate":{"op":"add","args":[{"lit":1},{"lit":2}]}}}"#),
        "{}",
    );
    assert!(envelope.contains(r#""c":3"#), "{envelope}");
    // Division by zero is still `null`, not an integer error.
    let divided = evaluate(
        &form_json,
        &rules(r#"{"c":{"calculate":{"op":"div","args":[{"lit":1},{"lit":0}]}}}"#),
        "{}",
    );
    assert!(divided.contains(r#""c":null"#), "{divided}");
}

/// A calculated value is held to its own field's type and constraints. A rule
/// producing `100` for a field declared `maximum: 10` used to be reported as a
/// valid submission.
#[test]
fn a_calculated_value_must_satisfy_its_field() {
    let form_json = form(r#"[{"id":"x","code":"x","type":"number","readOnly":true,"maximum":10}]"#);
    let envelope = validate_response(
        &form_json,
        &rules(r#"{"x":{"calculate":{"lit":100}}}"#),
        "{}",
    );
    assert!(envelope.contains("CALCULATED_VALUE_INVALID"), "{envelope}");
    assert!(envelope.contains(r#""isValid":false"#), "{envelope}");
    // The offending value must not be published as a normalized answer.
    assert!(
        !envelope.contains(r#"\u0022x\u0022:100"#),
        "an out-of-constraint value was normalized: {envelope}"
    );
}

/// The same path must not reject a value that *does* satisfy the field.
#[test]
fn a_calculated_value_inside_its_constraints_is_accepted() {
    let form_json =
        form(r#"[{"id":"x","code":"x","type":"number","readOnly":true,"maximum":10,"minimum":1}]"#);
    let envelope = validate_response(&form_json, &rules(r#"{"x":{"calculate":{"lit":5}}}"#), "{}");
    assert!(envelope.contains(r#""isValid":true"#), "{envelope}");
    assert!(envelope.contains(r#"\u0022x\u0022:5"#), "{envelope}");
}

/// An over-budget pattern is refused during classification, so it never reaches
/// the engine. Before the budget, 32 000 groups cost about 17.6 s in one call.
#[test]
fn an_over_budget_pattern_is_refused() {
    for groups in [600usize, 4_000, 32_000] {
        let pattern: String = "a{1,2}".repeat(groups);
        let error = validate_text(
            &format!(r#"{{"type":"string","pattern":{}}}"#, json_string(&pattern)),
            &json_string(&"a".repeat(groups)),
            "instance",
        )
        .expect_err("an over-budget pattern must be refused")
        .to_string();
        assert!(error.contains("PATTERN_TOO_COMPLEX"), "{error}");
    }
}

/// The budget is a bound, not a policy change: realistic patterns keep working,
/// including look-around, escaped metacharacters and character classes.
#[test]
fn realistic_patterns_stay_within_the_budget() {
    // Each pattern is paired with the text it is meant to accept, so the test
    // cannot confuse one realistic rule for another.
    let cases = [
        (
            r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$",
            "user@example.com",
        ),
        (
            r"^(?=.*[a-z])(?=.*[A-Z])(?=.*\d)(?=.*[@$!%*?&])[A-Za-z\d@$!%*?&]{8,}$",
            "Abcdef1!",
        ),
        (r"^\d{4}-\d{2}-\d{2}$", "2026-01-15"),
        (r"a\*b", "a*b"),
        (r"[+?*]{3}", "+?*"),
        (
            r"^(?:https?://)?(?:[\w-]+\.)+[a-z]{2,}(?:/\S*)?$",
            "https://example.com/x",
        ),
    ];
    for (pattern, text) in cases {
        assert!(
            is_within_budget(pattern),
            "a realistic pattern was refused: {pattern} ({} units)",
            complexity_units(pattern)
        );
        // And it still matches the text it is meant to match.
        let matched = validate_text(
            &format!(r#"{{"type":"string","pattern":{}}}"#, json_string(pattern)),
            &json_string(text),
            "instance",
        );
        assert!(
            matched.is_ok(),
            "a realistic pattern stopped matching {text:?}: {pattern}"
        );
    }
}

/// The complexity count is structural: escapes and character classes do not
/// count, quantifiers and alternations do. It must also be deterministic.
#[test]
fn the_complexity_count_is_structural_and_deterministic() {
    assert_eq!(complexity_units("abc"), 0);
    assert_eq!(complexity_units(r"a\*b"), 0, "an escaped star is a literal");
    assert_eq!(complexity_units(r"[+?*]"), 0, "a class is a literal set");
    assert_eq!(complexity_units("a{1,2}"), 1);
    assert_eq!(complexity_units("a*b"), 1);
    assert_eq!(complexity_units("a|b"), 1);
    assert_eq!(complexity_units("a{2}"), 1);
    assert_eq!(
        complexity_units("a{x}"),
        0,
        "braces are not a quantifier here"
    );
    assert_eq!(
        complexity_units("(?:a|b)+"),
        2,
        "a group opener is not a quantifier"
    );
    // A pattern of plain characters has no quantifiers, so length alone is not
    // what the budget measures.
    assert!(
        is_within_budget(&"a".repeat(MAX_PATTERN_UNITS)),
        "a literal pattern is never over budget"
    );
    assert!(!is_within_budget(&"a*".repeat(MAX_PATTERN_UNITS + 1)));
    // Same input, same answer, every time.
    let pattern = "a{1,2}".repeat(100);
    assert_eq!(complexity_units(&pattern), complexity_units(&pattern));
    // The realistic-pattern test above is the real guard on headroom; this only
    // records that the documented limit is the one the code uses.
    assert_eq!(MAX_PATTERN_UNITS, 512, "the documented pattern budget");
}

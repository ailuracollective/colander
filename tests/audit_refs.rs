//! Regression tests for `$ref` cycle rejection (SPEC S-10).
//!
//! A schema whose `$ref` graph reaches itself is not evaluable by colander:
//! evaluation would recurse without bound. Classification detects the cycle and
//! reports one deterministic error instead, so a recursive schema can never
//! abort the process. The non-recursive cases guard against false positives —
//! a shared (diamond) reference and a bounded instance-level recursion are
//! legitimate and must keep validating.

use colander::schema::validate_text;

/// The message every cyclic schema produces, independent of the shape that
/// created the cycle.
const RECURSIVE: &str = "recursive reference";

fn assert_rejected(label: &str, schema: &str, instance: &str, reference: &str) {
    let error = validate_text(schema, instance, "instance")
        .expect_err("a recursive schema must be rejected")
        .to_string();
    assert!(
        error.contains(RECURSIVE),
        "{label}: expected a recursive-reference error, got {error}"
    );
    assert!(
        error.contains(reference),
        "{label}: expected the offending reference {reference:?}, got {error}"
    );
    // Determinism: the same input must produce the same message, so the
    // rejection is a property of the schema and not of a traversal order.
    let repeated = validate_text(schema, instance, "instance")
        .expect_err("rejection is repeatable")
        .to_string();
    assert_eq!(error, repeated, "{label}: rejection must be deterministic");
}

#[test]
fn self_reference_is_rejected() {
    assert_rejected("object", r##"{"$ref":"#"}"##, "{}", "#");
    assert_rejected("array", r##"{"$ref":"#"}"##, "[]", "#");
}

#[test]
fn indirect_cycle_is_rejected() {
    // a -> b -> a through combinators.
    assert_rejected(
        "anyOf",
        r##"{"$defs":{"a":{"anyOf":[{"$ref":"#/$defs/b"}]},
            "b":{"anyOf":[{"$ref":"#/$defs/a"}]}},"$ref":"#/$defs/a"}"##,
        "{}",
        "#/$defs/a",
    );
    assert_rejected(
        "allOf",
        r##"{"$defs":{"a":{"allOf":[{"$ref":"#/$defs/b"}]},
            "b":{"allOf":[{"$ref":"#/$defs/a"}]}},"$ref":"#/$defs/a"}"##,
        "{}",
        "#/$defs/a",
    );
}

#[test]
fn cycle_behind_a_blocking_keyword_is_rejected() {
    // The cycle sits under `not` and under `if`/`then`; classification must
    // walk those positions rather than treat them as opaque.
    assert_rejected(
        "not",
        r##"{"$defs":{"a":{"not":{"$ref":"#/$defs/a"}}},"$ref":"#/$defs/a"}"##,
        "1",
        "#/$defs/a",
    );
    assert_rejected(
        "if/then",
        r##"{"$defs":{"a":{"if":{"type":"string"},"then":{"$ref":"#/$defs/a"}}},"$ref":"#/$defs/a"}"##,
        "1",
        "#/$defs/a",
    );
}

#[test]
fn recursive_defs_pattern_is_rejected() {
    // The recursive linked-list schema: the node references itself through
    // `next`. Rejected at the schema, before any instance can drive the
    // recursion.
    assert_rejected(
        "node/next",
        r##"{"$defs":{"node":{"type":"object","properties":{
                    "next":{"$ref":"#/$defs/node"}}}},"$ref":"#/$defs/node"}"##,
        "{}",
        "#/$defs/node",
    );
}

#[test]
fn shared_non_recursive_reference_still_validates() {
    // A diamond — the same definition reached through two branches — is not a
    // cycle and must keep working.
    let schema = r##"{"$defs":{"leaf":{"type":"integer"}},
        "allOf":[{"properties":{"a":{"$ref":"#/$defs/leaf"}}},
                 {"properties":{"b":{"$ref":"#/$defs/leaf"}}}]}"##;
    assert!(validate_text(schema, r##"{"a":1,"b":2}"##, "instance").is_ok());
    assert!(validate_text(schema, r##"{"a":"x","b":2}"##, "instance").is_err());
}

#[test]
fn sibling_reuse_of_one_definition_is_not_a_cycle() {
    // The same `$ref` twice in independent positions must not look recursive.
    let schema = r##"{"$defs":{"leaf":{"type":"integer"}},
        "properties":{"a":{"$ref":"#/$defs/leaf"},"b":{"$ref":"#/$defs/leaf"}}}"##;
    assert!(validate_text(schema, r##"{"a":1,"b":2}"##, "instance").is_ok());
}

#[test]
fn self_reference_inside_a_nested_property_is_rejected() {
    // The cycle is not on the top-level path: `outer` is clean, `inner` is
    // recursive. Both the outer and the inner reference must be reported
    // through the same rule.
    assert_rejected(
        "nested",
        r##"{"$defs":{"outer":{"properties":{
                    "inner":{"$ref":"#/$defs/loop"}}},"loop":{"$ref":"#/$defs/loop"}},
            "$ref":"#/$defs/outer"}"##,
        "{}",
        "#/$defs/loop",
    );
}

#[test]
fn unresolvable_reference_is_still_its_own_error() {
    // The cycle rule must not swallow the unrelated "cannot resolve" case.
    let error = validate_text(r##"{"$ref":"#/$defs/missing"}"##, "{}", "instance")
        .expect_err("an unresolvable reference is still invalid")
        .to_string();
    assert!(
        !error.contains(RECURSIVE),
        "an unresolvable reference is not a cycle: {error}"
    );
}

//! Cross-runtime conformance corpus.
//!
//! Every expected envelope below was produced by the native C ABI and
//! reproduced byte-for-byte by the same library compiled to
//! `wasm32-unknown-unknown` and driven from Node (14/14 identical, third
//! audit 2026-09-25). A wrapper — TypeScript, .NET, any language — must
//! reproduce these bytes for the same request. Escaping, key order, error
//! ordering and numeric rendering are part of what a wrapper inherits, not
//! wrapper detail; a difference in any of them is a conformance break.
//!
//! The `bigint_precision` case is deliberately pinned with the mismatch the
//! core now reports: an exact integer answer at 2^53+1 cannot be reproduced
//! by a `f64` calculation, so the core says so instead of quietly storing the
//! rounded value. A future change to that behaviour is a deliberate, visible
//! diff rather than a silent drift.

use std::ffi::{CStr, c_char};

use colander::ffi::envelope::{colander_abi_version, colander_free_string};
use colander::ffi::response::colander_validate_response;
use colander::ffi::rules::colander_evaluate_rules;
use colander::ffi::schema::colander_validate_schema;
use colander::ffi::session::call_with_text;
use colander::ffi::version::{colander_content_hash, colander_next_version};

type Entry = unsafe extern "C" fn(*const c_char) -> *mut c_char;

const FORM: &str = r#"{"schemaVersion":"1.0.0","fields":[{"id":"a","code":"a","type":"number"},{"id":"b","code":"b","type":"number"},{"id":"c","code":"c","type":"number","readOnly":true}]}"#;
const RULES: &str = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{"c":{"calculate":{"op":"add","args":[{"ref":"a"},{"ref":"b"}]}}}}"#;

fn shared_ref_chain(depth: usize) -> String {
    let mut definitions = String::new();
    for level in 0..depth {
        if level > 0 {
            definitions.push(',');
        }
        let next = level + 1;
        let body = if next == depth {
            r#"{"type":"string"}"#.to_string()
        } else {
            format!(r##"{{"anyOf":[{{"$ref":"#/$defs/n{next}"}},{{"$ref":"#/$defs/n{next}"}}]}}"##)
        };
        definitions.push_str(&format!(r#""n{level}":{body}"#));
    }
    format!(r##"{{"$ref":"#/$defs/n0","$defs":{{{definitions}}}}}"##)
}

/// A pattern with `units` quantifier constructs: the shape whose compilation
/// and evaluation cost is superlinear in the construct count.
fn complex_pattern_schema(units: usize) -> String {
    format!(
        r#"{{"type":"string","pattern":"{}"}}"#,
        "a{1,2}".repeat(units)
    )
}

/// A chain of *distinct* references: not a cycle, so classification accepts
/// it, but it still nests once per level.
fn deep_ref_chain(depth: usize) -> String {
    let mut definitions = String::new();
    for level in 0..depth {
        if level > 0 {
            definitions.push(',');
        }
        let body = if level + 1 == depth {
            r#"{"type":"string"}"#.to_string()
        } else {
            format!(r##"{{"$ref":"#/$defs/n{}"}}"##, level + 1)
        };
        definitions.push_str(&format!(r#""n{level}":{body}"#));
    }
    format!(r##"{{"$ref":"#/$defs/n0","$defs":{{{definitions}}}}}"##)
}

#[test]
fn cross_runtime_corpus_is_byte_identical() {
    let cases: Vec<(&str, Entry, String, &str)> = vec![
        (
            "evaluate",
            colander_evaluate_rules,
            format!(
                r#"{{"formSchemaJson":{},"rulesSchemaJson":{},"values":{{"a":1,"b":2}}}}"#,
                quoted(FORM),
                quoted(RULES)
            ),
            r#"{"ok":true,"result":{"visibility":{"a":true,"b":true,"c":true},"enabled":{"a":true,"b":true,"c":false},"required":{"a":false,"b":false,"c":false},"calculatedValues":{"c":3},"validationErrors":[]}}"#,
        ),
        (
            "evaluate_missing_value",
            colander_evaluate_rules,
            format!(
                r#"{{"formSchemaJson":{},"rulesSchemaJson":{},"values":{{}}}}"#,
                quoted(FORM),
                quoted(RULES)
            ),
            r#"{"ok":true,"result":{"visibility":{"a":true,"b":true,"c":true},"enabled":{"a":true,"b":true,"c":false},"required":{"a":false,"b":false,"c":false},"calculatedValues":{"c":null},"validationErrors":[]}}"#,
        ),
        (
            "validate_complete",
            colander_validate_response,
            format!(
                r#"{{"formSchemaJson":{},"rulesSchemaJson":{},"answersJson":"{{\"a\":1,\"b\":2,\"c\":3}}","mode":"Complete"}}"#,
                quoted(FORM),
                quoted(RULES)
            ),
            r#"{"ok":true,"result":{"normalizedAnswersJson":"{\u0022a\u0022:1,\u0022b\u0022:2,\u0022c\u0022:3}","errors":[],"isValid":true}}"#,
        ),
        (
            "validate_draft_missing",
            colander_validate_response,
            format!(
                r#"{{"formSchemaJson":{},"rulesSchemaJson":{},"answersJson":"{{}}","mode":"Draft"}}"#,
                quoted(FORM),
                quoted(RULES)
            ),
            r#"{"ok":true,"result":{"normalizedAnswersJson":"{\u0022c\u0022:null}","errors":[],"isValid":true}}"#,
        ),
        (
            "validate_badtype",
            colander_validate_response,
            format!(
                r#"{{"formSchemaJson":{},"rulesSchemaJson":{},"answersJson":"{{\"a\":\"x\",\"b\":2}}","mode":"Complete"}}"#,
                quoted(FORM),
                quoted(RULES)
            ),
            r#"{"ok":false,"error":{"kind":"validation","message":"Cannot convert the string \u0027x\u0027 to a number."}}"#,
        ),
        (
            "hash",
            colander_content_hash,
            format!(r#"{{"formSchemaJson":{}}}"#, quoted(FORM)),
            r#"{"ok":true,"result":{"contentHash":"072865a0e0b6625474d270415626ceaf674396291c4b4931ba0dc242276ec3e9"}}"#,
        ),
        (
            "malformed",
            colander_evaluate_rules,
            r#"{"formSchemaJson":"#.to_string(),
            r#"{"ok":false,"error":{"kind":"invalid_request","message":"JSON_PARSE_ERROR: Invalid request: unexpected end of input at byte offset 18."}}"#,
        ),
        (
            "next_version",
            colander_next_version,
            r#"{"published":["1.0.0","1.2.3","1.10.0"],"bump":"minor"}"#.to_string(),
            r#"{"ok":true,"result":{"next":"1.11.0"}}"#,
        ),
        (
            "validate_schema_instance",
            colander_validate_schema,
            r#"{"kind":"instance","schemaJson":"{\"type\":\"array\",\"minItems\":2}","instanceJson":"[1]"}"#
                .to_string(),
            r#"{"ok":false,"error":{"kind":"validation","message":"Invalid instance: minItems: array must have at least 2 items"}}"#,
        ),
        (
            "schema_unknown_keyword",
            colander_validate_schema,
            r#"{"kind":"instance","schemaJson":"{\"type\":\"strnig\"}","instanceJson":"\"x\""}"#
                .to_string(),
            r#"{"ok":false,"error":{"kind":"validation","message":"Invalid instance: type: unknown type \u0027strnig\u0027"}}"#,
        ),
        (
            "rule_arity_error",
            colander_evaluate_rules,
            format!(
                r#"{{"formSchemaJson":{},"rulesSchemaJson":{},"values":{{}}}}"#,
                quoted(FORM),
                quoted(
                    r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{"c":{"calculate":{"op":"add","args":[{"lit":1},{"lit":2},{"lit":3}]}}}}"#
                )
            ),
            r#"{"ok":false,"error":{"kind":"validation","message":"RULE_INVALID_EXPRESSION_ARITY: expression \u0027add\u0027 at /fields/c/calculate has 3 argument(s), expected exactly 2."}}"#,
        ),
        (
            "repeater_rows",
            colander_validate_response,
            r#"{"formSchemaJson":"{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"l\",\"code\":\"l\",\"type\":\"repeater\",\"items\":[{\"id\":\"q\",\"code\":\"q\",\"type\":\"number\"}]}]}","rulesSchemaJson":"{\"schemaVersion\":\"1.0.0\",\"formSchemaVersion\":\"1.0.0\",\"fields\":{}}","answersJson":"{\"l\":[{\"q\":1},{\"q\":2}]}","mode":"Complete"}"#.to_string(),
            r#"{"ok":true,"result":{"normalizedAnswersJson":"{\u0022l\u0022:[{\u0022q\u0022:1},{\u0022q\u0022:2}]}","errors":[],"isValid":true}}"#,
        ),
        (
            "bigint_precision",
            colander_validate_response,
            r#"{"formSchemaJson":"{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"n\",\"code\":\"n\",\"type\":\"integer\"},{\"id\":\"c\",\"code\":\"c\",\"type\":\"integer\",\"readOnly\":true}]}","rulesSchemaJson":"{\"schemaVersion\":\"1.0.0\",\"formSchemaVersion\":\"1.0.0\",\"fields\":{\"c\":{\"calculate\":{\"op\":\"add\",\"args\":[{\"ref\":\"n\"},{\"lit\":0}]}}}}","answersJson":"{\"n\":9007199254740993,\"c\":9007199254740993}","mode":"Complete"}"#.to_string(),
            r#"{"ok":true,"result":{"normalizedAnswersJson":"{\u0022n\u0022:9007199254740993,\u0022c\u0022:9007199254740992}","errors":[{"code":"CALCULATED_VALUE_MISMATCH","path":"/fields/1","message":"Field \u0027c\u0027 must match the server-calculated value."}],"isValid":false}}"#,
        ),
        (
            // S-10: a recursive reference is one deterministic error, never a
            // process abort. Pinned on both runtimes.
            "schema_recursive_ref",
            colander_validate_schema,
            r##"{"kind":"instance","schemaJson":"{\"$ref\":\"#\"}","instanceJson":"{}"}"##.to_string(),
            r#"{"ok":false,"error":{"kind":"validation","message":"Invalid instance: $ref: recursive reference \u0027#\u0027 is not supported"}}"#,
        ),
        (
            // S-11: shared references cannot spend an unbounded number of
            // deterministic evaluation steps. Pinned on both runtimes.
            "schema_evaluation_limit",
            colander_validate_schema,
            format!(
                r#"{{"kind":"instance","schemaJson":{},"instanceJson":"{{}}"}}"#,
                quoted(&shared_ref_chain(26))
            ),
            r#"{"ok":false,"error":{"kind":"validation","message":"Invalid instance: schema: SCHEMA_EVALUATION_LIMIT: schema evaluation exceeded the step budget"}}"#,
        ),
        (
            // S-13: an over-budget pattern is refused during classification,
            // before anything is matched. Pinned on both runtimes.
            "pattern_too_complex",
            colander_validate_schema,
            format!(
                r#"{{"kind":"instance","schemaJson":{},"instanceJson":"\"a\""}}"#,
                quoted(&complex_pattern_schema(600))
            ),
            r#"{"ok":false,"error":{"kind":"validation","message":"Invalid instance: pattern: PATTERN_TOO_COMPLEX: pattern has 600 quantifier or alternation constructs, the limit is 512"}}"#,
        ),
        (
            // S-12: nesting depth is bounded on its own, because a step
            // budget cannot bound recursion depth. Pinned on both runtimes.
            "schema_depth_limit",
            colander_validate_schema,
            format!(
                r#"{{"kind":"instance","schemaJson":{},"instanceJson":"{{}}"}}"#,
                quoted(&deep_ref_chain(30_000))
            ),
            r#"{"ok":false,"error":{"kind":"validation","message":"Invalid instance: schema: SCHEMA_DEPTH_LIMIT: schema evaluation exceeded the nesting depth"}}"#,
        ),
        (
            // S-9: signed zero is one value, so the pair is a duplicate.
            "unique_signed_zero",
            colander_validate_schema,
            r#"{"kind":"instance","schemaJson":"{\"uniqueItems\":true}","instanceJson":"[0,-0.0]"}"#
                .to_string(),
            r#"{"ok":false,"error":{"kind":"validation","message":"Invalid instance: uniqueItems: array items must be unique (index 1 repeats)"}}"#,
        ),
        (
            // S-9: a large integer and the next double down are two values.
            "unique_large_integer",
            colander_validate_schema,
            r#"{"kind":"instance","schemaJson":"{\"uniqueItems\":true}","instanceJson":"[9007199254740993,9007199254740992.0]"}"#
                .to_string(),
            r#"{"ok":true,"result":{"valid":true}}"#,
        ),
        (
            // R-16: the operators do not collapse two distinct integers.
            "int_operators_exact",
            colander_evaluate_rules,
            format!(
                r#"{{"formSchemaJson":{},"rulesSchemaJson":{},"values":{{"n":9007199254740993,"a":true,"b":false}}}}"#,
                quoted(
                    r#"{"schemaVersion":"1.0.0","fields":[{"id":"n","code":"n","type":"integer"},{"id":"a","code":"a","type":"boolean"},{"id":"b","code":"b","type":"boolean"}]}"#
                ),
                quoted(
                    r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{"a":{"visibleWhen":{"op":"eq","args":[{"ref":"n"},{"lit":9007199254740992.0}]}},"b":{"visibleWhen":{"op":"eq","args":[{"ref":"n"},{"lit":9007199254740993}]}}}}"#
                )
            ),
            r#"{"ok":true,"result":{"visibility":{"n":true,"a":false,"b":true},"enabled":{"n":true,"a":true,"b":true},"required":{"n":false,"a":false,"b":false},"calculatedValues":{},"validationErrors":[]}}"#,
        ),
    ];

    for (name, entry, request, expected) in &cases {
        let got = call_with_text(request, *entry);
        assert_eq!(&got, expected, "corpus case {name} diverged");
    }
}

#[test]
fn null_request_envelope_is_the_contract_one() {
    let pointer = unsafe { colander_evaluate_rules(std::ptr::null()) };
    let envelope = unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned();
    unsafe { colander_free_string(pointer) };
    assert_eq!(
        envelope,
        r#"{"ok":false,"error":{"kind":"invalid_request","message":"NULL_REQUEST: request pointer is null"}}"#
    );
}

#[test]
fn abi_version_is_one_for_wrappers_to_bind() {
    assert_eq!(colander_abi_version(), 1);
}

/// Embed a document as a JSON string literal, escaping exactly as a wrapper
/// that uses the library's own writer would.
fn quoted(document: &str) -> String {
    let mut out = String::new();
    colander::json::write_string(&mut out, document);
    out
}

/// The first 99 errors of a capped response are part of the observable API
/// (the truncation boundary makes order observable), so the order must be
/// stable across runs of the same input.
#[test]
fn truncated_error_order_is_stable_across_runs() {
    let n = 3000;
    let mut fields = String::new();
    let mut answers = String::new();
    for i in 0..n {
        fields.push_str(&format!(
            "{{\"id\":\"f{i}\",\"code\":\"c{i}\",\"type\":\"text\"}},"
        ));
        answers.push_str(&format!("\"c{i}\":5,"));
    }
    fields.pop();
    answers.pop();
    let form = format!("{{\"schemaVersion\":\"1.0.0\",\"fields\":[{fields}]}}");
    let answers = format!("{{{answers}}}");
    let request = format!(
        r#"{{"formSchemaJson":{},"answersJson":{},"mode":"Complete"}}"#,
        quoted(&form),
        quoted(&answers)
    );
    let first = call_with_text(&request, colander_validate_response);
    for _ in 0..4 {
        assert_eq!(
            call_with_text(&request, colander_validate_response),
            first,
            "error order under truncation is not stable"
        );
    }
    assert!(first.contains("VALIDATION_ERRORS_TRUNCATED"));
}

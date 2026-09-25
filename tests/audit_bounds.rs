//! Bounds and value-semantics regressions from the third adversarial audit.
//!
//! Split from `audit_complexity.rs` to respect the 400-line file limit: this
//! half covers response bounds, numeric exactness, error codes and the
//! memoization guarantees; the complexity half covers the time-sensitive
//! linearity checks.

use colander::compile::{ComponentVersionData, compile};
use colander::json::{self, Json};
use colander::rules::validate_dependencies;
use colander::validate::{FormResponseValidationMode, validate};

// ---------------------------------------------------------------------------
// V-10: the error list is bounded in count *and* in bytes.
// ---------------------------------------------------------------------------

#[test]
fn a_long_field_code_cannot_amplify_the_error_list() {
    let long_code = "x".repeat(4 * 1024 * 1024);
    let form = format!(
        "{{\"schemaVersion\":\"1.0.0\",\"fields\":[{{\"id\":\"lines\",\"code\":\"lines\",\"type\":\"repeater\",\"items\":[{{\"id\":\"child\",\"code\":\"{long_code}\",\"type\":\"text\",\"required\":true}}]}}]}}"
    );
    let answers = format!(
        "{{\"lines\":[{}]}}",
        std::iter::repeat_n("{}", 500).collect::<Vec<_>>().join(",")
    );
    let result = validate(
        &form,
        None,
        None,
        &answers,
        FormResponseValidationMode::Complete,
    )
    .unwrap();
    let bytes: usize = result
        .errors
        .iter()
        .map(|error| error.message.len() + error.path.len())
        .sum();
    assert!(
        bytes < 64 * 1024,
        "error payload grew to {bytes} bytes from a long field code"
    );
    // The elision is visible rather than silent.
    assert!(
        result
            .errors
            .iter()
            .any(|error| error.message.contains("more chars")),
        "no message reported the elision"
    );
}

// ---------------------------------------------------------------------------
// V-11: a calculated integer outside the exact range reports instead of drifting.
// ---------------------------------------------------------------------------

#[test]
fn an_unrepresentable_calculated_integer_reports_instead_of_drifting() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"n","code":"n","type":"integer"},
        {"id":"calc","code":"calc","type":"integer","readOnly":true}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "calc":{"calculate":{"op":"add","args":[{"ref":"n"},{"lit":0}]}}}}"#;
    // 2^53 + 1 is exact as an answer and not representable as a double, so the
    // server cannot reproduce it: the mismatch is reported instead of the
    // rounded value being stored as if it were the client's.
    let result = validate(
        form,
        None,
        Some(rules),
        r#"{"n":9007199254740993,"calc":9007199254740993}"#,
        FormResponseValidationMode::Complete,
    )
    .unwrap();
    assert!(
        result
            .errors
            .iter()
            .any(|error| error.code == "CALCULATED_VALUE_MISMATCH"),
        "{:?}",
        result.errors
    );
    // The exact submitted input is untouched.
    assert!(
        result
            .normalized_answers_json
            .contains(r#""n":9007199254740993"#),
        "{}",
        result.normalized_answers_json
    );
    // Inside the exact range nothing changes.
    let result = validate(
        form,
        None,
        Some(rules),
        r#"{"n":42,"calc":42}"#,
        FormResponseValidationMode::Complete,
    )
    .unwrap();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert!(result.normalized_answers_json.contains(r#""calc":42"#));

    // A calculation that lands outside the exactly-representable range is
    // reported as invalid rather than rounded into a wrong integer.
    let number_form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"n","code":"n","type":"number"},
        {"id":"calc","code":"calc","type":"integer","readOnly":true}]}"#;
    let result = validate(
        number_form,
        None,
        Some(rules),
        r#"{"n":1e300,"calc":1e300}"#,
        FormResponseValidationMode::Complete,
    )
    .unwrap();
    assert!(
        result
            .errors
            .iter()
            .any(|error| error.code == "CALCULATED_VALUE_INVALID"),
        "{:?}",
        result.errors
    );
    assert!(
        !result.normalized_answers_json.contains("calc"),
        "{}",
        result.normalized_answers_json
    );
}

// ---------------------------------------------------------------------------
// R-15: `sum` is compensated.
// ---------------------------------------------------------------------------

#[test]
fn summing_many_small_decimals_does_not_drift() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"lines","code":"lines","type":"repeater","items":[{"id":"v","code":"v","type":"number"}]},
        {"id":"total","code":"total","type":"number","readOnly":true}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "total":{"calculate":{"op":"sum","args":[{"ref":"lines"},{"ref":"v"}]}}}}"#;
    let rows: Vec<String> = std::iter::repeat_n("{\"v\":0.1}".to_string(), 1_000).collect();
    let answers = format!("{{\"lines\":[{}]}}", rows.join(","));
    let result = validate(
        form,
        None,
        Some(rules),
        &answers,
        FormResponseValidationMode::Draft,
    )
    .unwrap();
    // Uncompensated, 1000 additions of 0.1 land at 99.9999999999986; the
    // compensated sum is 100.
    assert!(
        result.normalized_answers_json.contains(r#""total":100"#),
        "{}",
        result.normalized_answers_json
    );
}

// ---------------------------------------------------------------------------
// C-11: the remaining error families carry codes.
// ---------------------------------------------------------------------------

#[test]
fn parse_and_semver_failures_carry_contract_codes() {
    let error = validate("{", None, None, "{}", FormResponseValidationMode::Draft).unwrap_err();
    assert!(error.message.starts_with("JSON_PARSE_ERROR"), "{error}");

    let error = validate(
        "{\"fields\":[]}",
        None,
        None,
        "[]",
        FormResponseValidationMode::Draft,
    )
    .unwrap_err();
    assert!(error.message.starts_with("JSON_NOT_OBJECT"), "{error}");

    let error = compile(
        r#"{"fields":[{"id":"a","code":"a","type":"component-ref","componentCode":"X","componentVersion":"1.0"}]}"#,
        None,
        None,
        &[],
    )
    .unwrap_err();
    assert!(error.message.starts_with("INVALID_SEMVER"), "{error}");
}

// ---------------------------------------------------------------------------
// P-11: the source form is not retained for the whole call.
// ---------------------------------------------------------------------------

#[test]
fn a_shared_component_expands_once_and_keeps_its_source_only_until_then() {
    let source = "{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"a\",\"code\":\"a\",\"type\":\"text\"}]}";
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"r1","code":"r1","type":"component-ref","componentCode":"X","componentVersion":"1.0.0"},
        {"id":"r2","code":"r2","type":"component-ref","componentCode":"X","componentVersion":"1.0.0"},
        {"id":"r3","code":"r3","type":"component-ref","componentCode":"X","componentVersion":"1.0.0"}]}"#;
    let components = vec![ComponentVersionData {
        code: "X".into(),
        version: "1.0.0".into(),
        form_schema_json: source.into(),
        ui_schema_json: None,
        content_hash: None,
    }];
    let result = compile(form, None, None, &components).unwrap();
    let compiled = json::parse(&result.form_schema_json).unwrap();
    let fields = compiled.as_object().unwrap()["fields"].as_array().unwrap();
    let first = json::ordered(&fields[0].as_object().unwrap()["items"]);
    assert_eq!(
        first,
        json::ordered(&fields[2].as_object().unwrap()["items"])
    );
    // One dependency entry: memoized by (code, version).
    assert_eq!(
        result.dependency_metadata_json.matches("\"code\"").count(),
        1
    );
}

// ---------------------------------------------------------------------------
// A guard on the value the conformance corpus pins for 2^53+1.
// ---------------------------------------------------------------------------

#[test]
fn a_calculated_integer_beyond_the_exact_range_is_never_stored() {
    // The FFI path agrees with the direct API on the same input.
    let form = Json::String(
        r#"{"schemaVersion":"1.0.0","fields":[{"id":"n","code":"n","type":"integer"},{"id":"c","code":"c","type":"integer","readOnly":true}]}"#
            .to_string(),
    );
    let rules = Json::String(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{"c":{"calculate":{"op":"add","args":[{"ref":"n"},{"lit":0}]}}}}"#
            .to_string(),
    );
    let answers = Json::String(r#"{"n":9007199254740993,"c":9007199254740993}"#.to_string());
    let request = format!(
        "{{\"formSchemaJson\":{},\"rulesSchemaJson\":{},\"answersJson\":{},\"mode\":\"Complete\"}}",
        json::ordered(&form),
        json::ordered(&rules),
        json::ordered(&answers)
    );
    let envelope = colander::ffi::session::call_with_text(
        &request,
        colander::ffi::response::colander_validate_response,
    );
    assert!(envelope.contains("CALCULATED_VALUE_MISMATCH"), "{envelope}");
}

/// A dependency that several fields share must be resolved for each of them:
/// deduplicating across fields would hide a cycle that only closes through a
/// second field.
#[test]
fn a_shared_dependency_is_recorded_for_every_field_that_references_it() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"a","code":"a","type":"number","readOnly":true},
        {"id":"b","code":"b","type":"number","readOnly":true},
        {"id":"c","code":"c","type":"number","readOnly":true}]}"#;
    // a is independent; b and c both read a; c also reads b.
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "a":{"calculate":{"op":"add","args":[{"lit":1},{"lit":1}]}},
        "b":{"calculate":{"op":"add","args":[{"ref":"a"},{"lit":1}]}},
        "c":{"calculate":{"op":"add","args":[{"ref":"a"},{"ref":"b"}]}}}}"#;
    let form = json::parse_object(form, "form").unwrap();
    let rules = json::parse_object(rules, "rules").unwrap();
    let metadata = colander::rules::analyze(&form, &rules).unwrap();
    let position = |id: &str| {
        metadata
            .evaluation_order
            .iter()
            .position(|entry| entry == id)
            .unwrap()
    };
    assert!(position("a") < position("b"));
    assert!(position("b") < position("c"));
    // And the cycle through a shared dependency is still detected.
    let cyclic = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "a":{"calculate":{"op":"add","args":[{"ref":"c"},{"lit":1}]}},
        "b":{"calculate":{"op":"add","args":[{"ref":"a"},{"lit":1}]}},
        "c":{"calculate":{"op":"add","args":[{"ref":"a"},{"ref":"b"}]}}}}"#;
    let rules = json::parse_object(cyclic, "rules").unwrap();
    let error = validate_dependencies(&form, &rules).unwrap_err();
    assert!(
        error.message.starts_with("RULE_CYCLIC_DEPENDENCY"),
        "{error}"
    );
}

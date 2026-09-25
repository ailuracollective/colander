//! Regression tests for the 2026-09-25 adversarial audit (post-`b675207`).
//!
//! These pin the guarantees the second audit tried to break: bounded
//! expansion, memoised component resolution, one numeric equality across
//! subsystems, fail-closed expression shapes, strictness outside `fields.*`,
//! a bounded error list, and linear-ish per-row calculation.

use colander::compile::{ComponentVersionData, compile};
use colander::ffi::envelope::MAX_REQUEST_BYTES;
use colander::json;
use colander::rules::{RowSet, Val, compare_values, validate_dependencies};
use colander::validate::{FormResponseValidationMode, validate};

fn comp(code: &str, version: &str, form: String) -> ComponentVersionData {
    ComponentVersionData {
        code: code.into(),
        version: version.into(),
        form_schema_json: form,
        ui_schema_json: None,
        content_hash: None,
    }
}

// ---------------------------------------------------------------------------
// P-8: a component referenced from N sites expands once and is cloned N times.
// ---------------------------------------------------------------------------

#[test]
fn shared_component_output_is_stable_across_reference_order() {
    let x = "{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"x1\",\"code\":\"x1\",\"type\":\"text\"}]}";
    let y = "{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"y1\",\"code\":\"y1\",\"type\":\"text\"}]}";
    let form = "{\"schemaVersion\":\"1.0.0\",\"fields\":[\
        {\"id\":\"r1\",\"code\":\"r1\",\"type\":\"component-ref\",\"componentCode\":\"X\",\"componentVersion\":\"1.0.0\"},\
        {\"id\":\"r2\",\"code\":\"r2\",\"type\":\"component-ref\",\"componentCode\":\"Y\",\"componentVersion\":\"1.0.0\"},\
        {\"id\":\"r3\",\"code\":\"r3\",\"type\":\"component-ref\",\"componentCode\":\"X\",\"componentVersion\":\"1.0.0\"}]}";
    let comps = vec![comp("X", "1.0.0", x.into()), comp("Y", "1.0.0", y.into())];
    let result = compile(form, None, None, &comps).unwrap();
    let compiled = json::parse(&result.form_schema_json).unwrap();
    let fields = compiled.as_object().unwrap()["fields"].as_array().unwrap();
    // Both X references carry the same compiled items (memoized, then cloned).
    let first = json::ordered(&fields[0].as_object().unwrap()["items"]);
    let third = json::ordered(&fields[2].as_object().unwrap()["items"]);
    assert_eq!(first, third);
    // Three dependency entries? No: two — one per (code, version).
    assert_eq!(
        result.dependency_metadata_json,
        r#"{"components":[{"code":"X","contentHash":"","version":"1.0.0"},{"code":"Y","contentHash":"","version":"1.0.0"}]}"#
    );
}

#[test]
fn exponential_fanout_stays_within_the_budget_and_terminates() {
    // Binary fanout would produce 2^depth output; the budget rejects it
    // before materialising more than the bound, so the call fails fast
    // instead of exhausting memory. The depth limit is not the defence.
    let mut comps = Vec::new();
    for level in 0..40 {
        let form = if level + 1 == 40 {
            "{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"leaf\",\"code\":\"leaf\",\"type\":\"text\"}]}".to_string()
        } else {
            format!(
                "{{\"schemaVersion\":\"1.0.0\",\"fields\":[\
                {{\"id\":\"a{level}\",\"code\":\"a{level}\",\"type\":\"component-ref\",\"componentCode\":\"C{n}\",\"componentVersion\":\"1.0.0\"}},\
                {{\"id\":\"b{level}\",\"code\":\"b{level}\",\"type\":\"component-ref\",\"componentCode\":\"C{n}\",\"componentVersion\":\"1.0.0\"}}]}}",
                n = level + 1
            )
        };
        comps.push(comp(&format!("C{level}"), "1.0.0", form));
    }
    let form = "{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"r\",\"code\":\"r\",\"type\":\"component-ref\",\"componentCode\":\"C0\",\"componentVersion\":\"1.0.0\"}]}";
    let error = compile(form, None, None, &comps).unwrap_err();
    assert!(
        error.message.starts_with("COMPONENT_BUDGET_EXCEEDED"),
        "{error}"
    );
}

#[test]
fn wide_fanout_below_the_budget_compiles_and_hashes() {
    // 20 references to a 100-field component: 2020 fields, far below the
    // budget; memoization keeps it linear in the output.
    let mut x_fields = String::new();
    for i in 0..100 {
        x_fields.push_str(&format!(
            "{{\"id\":\"x{i}\",\"code\":\"x{i}\",\"type\":\"text\"}},"
        ));
    }
    x_fields.pop();
    let x = format!("{{\"schemaVersion\":\"1.0.0\",\"fields\":[{x_fields}]}}");
    let mut refs = String::new();
    for i in 0..20 {
        refs.push_str(&format!(
            "{{\"id\":\"r{i}\",\"code\":\"r{i}\",\"type\":\"component-ref\",\"componentCode\":\"X\",\"componentVersion\":\"1.0.0\"}},"
        ));
    }
    refs.pop();
    let form = format!("{{\"schemaVersion\":\"1.0.0\",\"fields\":[{refs}]}}");
    let comps = vec![comp("X", "1.0.0", x)];
    let result = compile(&form, None, None, &comps).unwrap();
    assert_eq!(result.content_hash.len(), 64);
}

// ---------------------------------------------------------------------------
// P-9: a pin is verified once per (code, version), not once per site.
// ---------------------------------------------------------------------------

#[test]
fn a_matching_pin_on_a_shared_component_verifies() {
    let x = "{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"x1\",\"code\":\"x1\",\"type\":\"text\"}]}";
    let pin = compile(x, None, None, &[]).unwrap().content_hash;
    let form = "{\"schemaVersion\":\"1.0.0\",\"fields\":[\
        {\"id\":\"r1\",\"code\":\"r1\",\"type\":\"component-ref\",\"componentCode\":\"X\",\"componentVersion\":\"1.0.0\"},\
        {\"id\":\"r2\",\"code\":\"r2\",\"type\":\"component-ref\",\"componentCode\":\"X\",\"componentVersion\":\"1.0.0\"}]}";
    let comps = vec![ComponentVersionData {
        code: "X".into(),
        version: "1.0.0".into(),
        form_schema_json: x.into(),
        ui_schema_json: None,
        content_hash: Some(pin),
    }];
    compile(form, None, None, &comps).unwrap();
}

// ---------------------------------------------------------------------------
// V-8: one numeric equality everywhere.
// ---------------------------------------------------------------------------

#[test]
fn eq_and_values_equal_agree_within_epsilon() {
    use colander::rules::evaluate_expression;
    for (left, right) in [
        (Val::Int(1), Val::Double(1.0000005)),
        (Val::Int(0), Val::Double(0.0000005)),
        (Val::Double(1.0), Val::Double(1.000002)),
    ] {
        let ordered = compare_values(&left, &right).unwrap();
        let values_equal = Val::values_equal(&left, &right);
        assert_eq!(
            ordered.is_eq(),
            values_equal,
            "{left:?} vs {right:?}: ordering and values_equal disagree"
        );
    }
    // The operator path agrees with the value comparison for a tolerant pair.
    let expression = json::parse(r#"{"op":"eq","args":[{"lit":1},{"lit":1.0000005}]}"#).unwrap();
    let values = indexmap::IndexMap::new();
    let result = evaluate_expression(&expression, &values, &RowSet::empty()).unwrap();
    assert_eq!(result, Val::Bool(true));
}

// ---------------------------------------------------------------------------
// R-12: fail-closed expression shapes.
// ---------------------------------------------------------------------------

fn shape_error(rules: &str) -> String {
    let form = json::parse_object(
        r#"{"schemaVersion":"1.0.0","fields":[{"id":"a","code":"a","type":"number"}]}"#,
        "f",
    )
    .unwrap();
    let rules = json::parse_object(rules, "r").unwrap();
    validate_dependencies(&form, &rules).unwrap_err().message
}

#[test]
fn null_arguments_are_rejected_by_the_analyzer() {
    let error = shape_error(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
            "a":{"visibleWhen":{"op":"eq","args":[null,{"lit":1}]}}}}"#,
    );
    assert!(error.starts_with("RULE_INVALID_EXPRESSION"), "{error}");
}

#[test]
fn empty_ref_and_mixed_nodes_are_rejected() {
    let error = shape_error(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
            "a":{"visibleWhen":{"ref":""}}}}"#,
    );
    assert!(error.starts_with("RULE_INVALID_EXPRESSION"), "{error}");
    let error = shape_error(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
            "a":{"visibleWhen":{"lit":1,"op":"eq","args":[]}}}}"#,
    );
    assert!(error.starts_with("RULE_INVALID_EXPRESSION"), "{error}");
}

#[test]
fn unknown_operators_carry_a_code_from_the_analyzer() {
    let error = shape_error(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
            "a":{"visibleWhen":{"op":"pow","args":[{"lit":2},{"lit":3}]}}}}"#,
    );
    assert!(
        error.starts_with("RULE_UNSUPPORTED_EXPRESSION_OPERATOR"),
        "{error}"
    );
}

#[test]
fn aggregate_arguments_must_be_references() {
    let error = shape_error(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
            "a":{"calculate":{"op":"count","args":[{"lit":1}]}}}}"#,
    );
    assert!(error.starts_with("RULE_INVALID_EXPRESSION"), "{error}");
}

#[test]
fn unknown_validation_keys_are_rejected() {
    let error = shape_error(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{},"validations":[
            {"code":"V","assert":{"lit":true},"message":"m","extra":1}]}"#,
    );
    assert!(error.starts_with("RULE_UNKNOWN_VALIDATION_KEY"), "{error}");
}

// ---------------------------------------------------------------------------
// Strictness outside `fields.*`.
// ---------------------------------------------------------------------------

#[test]
fn unknown_ui_top_level_keys_are_rejected_by_compile() {
    let form = "{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"a\",\"code\":\"a\",\"type\":\"text\"}]}";
    let ui =
        "{\"schemaVersion\":\"1.0.0\",\"formSchemaVersion\":\"1.0.0\",\"fields\":{},\"layuot\":[]}";
    let error = compile(form, Some(ui), None, &[]).unwrap_err();
    assert!(error.message.starts_with("UI_UNKNOWN_KEY"), "{error}");
}

#[test]
fn unknown_layout_node_keys_are_rejected_by_compile() {
    let form = "{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"a\",\"code\":\"a\",\"type\":\"text\"}]}";
    let ui = "{\"schemaVersion\":\"1.0.0\",\"formSchemaVersion\":\"1.0.0\",\"fields\":{},\
        \"layout\":[{\"type\":\"section\",\"children\":[{\"type\":\"field\",\"fieldId\":\"a\"}],\"zzzz\":1}]}";
    let error = compile(form, Some(ui), None, &[]).unwrap_err();
    assert!(error.message.starts_with("UI_UNKNOWN_KEY"), "{error}");
}

#[test]
fn structural_field_errors_carry_codes() {
    // A non-object field node and a wrong-typed key are both coded now.
    let error = compile(
        "{\"schemaVersion\":\"1.0.0\",\"fields\":[42]}",
        None,
        None,
        &[],
    )
    .unwrap_err();
    assert!(error.message.starts_with("FIELD_NOT_OBJECT"), "{error}");
    let error = compile(
        "{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":7,\"code\":\"a\",\"type\":\"text\"}]}",
        None,
        None,
        &[],
    )
    .unwrap_err();
    assert!(error.message.starts_with("FIELD_INVALID_TYPE"), "{error}");
}

// ---------------------------------------------------------------------------
// S-7 applied to response validation: the error list is bounded.
// ---------------------------------------------------------------------------

#[test]
fn the_error_list_is_capped_with_a_truncation_marker() {
    let n = 500;
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
    let result = validate(
        &form,
        None,
        None,
        &answers,
        FormResponseValidationMode::Complete,
    )
    .unwrap();
    assert_eq!(result.errors.len(), 100);
    let last = result.errors.last().unwrap();
    assert_eq!(last.code, "VALIDATION_ERRORS_TRUNCATED");
    assert!(last.message.contains("of 500"), "{}", last.message);
}

// ---------------------------------------------------------------------------
// C-10: a request over the byte cap is refused before parsing.
// ---------------------------------------------------------------------------

#[test]
fn the_request_byte_cap_is_explicit() {
    // The cap is a documented constant; this test pins its presence and its
    // order of magnitude so a future change to it is deliberate.
    assert_eq!(MAX_REQUEST_BYTES, 64 * 1024 * 1024);
}

// ---------------------------------------------------------------------------
// R-13: per-row calculation does not clone the growing row payload per row.
// ---------------------------------------------------------------------------

#[test]
fn a_ref_to_an_empty_repeater_is_falsy_on_both_paths() {
    // R-13: the FFI path used to leave the row payload (an empty list, which
    // is truthy) in the flat values while the validator carried a zero count
    // (falsy), so the same predicate could read differently per entry point.
    let form = json::parse_object(
        r#"{"schemaVersion":"1.0.0","fields":[
            {"id":"lines","code":"lines","type":"repeater","items":[{"id":"q","code":"q","type":"text"}]},
            {"id":"flag","code":"flag","type":"boolean"}]}"#,
        "f",
    )
    .unwrap();
    let rules = json::parse_object(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
            "flag":{"visibleWhen":{"op":"not","args":[{"ref":"lines"}]}}}}"#,
        "r",
    )
    .unwrap();
    let mut values = indexmap::IndexMap::new();
    values.insert("lines".to_string(), Val::List(Vec::new()));
    let mut rows = RowSet::from_values(&form, &values);
    let result = colander::rules::evaluate(&form, &rules, &values, None, &mut rows).unwrap();
    assert_eq!(result.visibility.get("flag"), Some(&true));
}

#[test]
fn per_row_calculation_projects_each_row_independently() {
    let form = "{\"schemaVersion\":\"1.0.0\",\"fields\":[\
        {\"id\":\"lines\",\"code\":\"lines\",\"type\":\"repeater\",\"items\":[\
        {\"id\":\"qty\",\"code\":\"qty\",\"type\":\"integer\"},\
        {\"id\":\"total\",\"code\":\"total\",\"type\":\"number\",\"readOnly\":true}]}]}";
    let rules = "{\"schemaVersion\":\"1.0.0\",\"formSchemaVersion\":\"1.0.0\",\"fields\":{\
        \"total\":{\"calculate\":{\"op\":\"mul\",\"args\":[{\"ref\":\"qty\"},{\"lit\":2}]}}}}";
    let answers = r#"{"lines":[{"qty":1},{"qty":2},{"qty":3}]}"#;
    let result = validate(
        form,
        None,
        Some(rules),
        answers,
        FormResponseValidationMode::Draft,
    )
    .unwrap();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let normalized = json::parse(&result.normalized_answers_json).unwrap();
    let rows = normalized.as_object().unwrap()["lines"].as_array().unwrap();
    let totals: Vec<_> = rows
        .iter()
        .map(|row| json::ordered(&row.as_object().unwrap()["total"]))
        .collect();
    // Each row sees only its own values: no leakage from the previous row.
    assert_eq!(totals, vec!["2", "4", "6"]);
}

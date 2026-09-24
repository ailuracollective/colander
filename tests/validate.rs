use colander::validate::*;

const FORM: &str = r#"{"schemaVersion":"1.0.0","fields":[
    {"id":"patient-name","code":"patient.name","type":"text","required":true}]}"#;

#[test]
fn rejects_missing_required_in_complete_mode() {
    let result = validate(FORM, None, None, "{}", FormResponseValidationMode::Complete).unwrap();
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].code, "REQUIRED_FIELD_MISSING");
    assert_eq!(result.errors[0].path, "/fields/0");
    assert_eq!(result.normalized_answers_json, "{}");
}

#[test]
fn draft_accepts_missing_required() {
    let result = validate(FORM, None, None, "{}", FormResponseValidationMode::Draft).unwrap();
    assert!(result.is_valid());
}

#[test]
fn normalizes_present_values() {
    let result = validate(
        FORM,
        None,
        None,
        r#"{"patient.name":"Ada"}"#,
        FormResponseValidationMode::Draft,
    )
    .unwrap();
    assert!(result.is_valid());
    assert_eq!(result.normalized_answers_json, r#"{"patient.name":"Ada"}"#);
}

#[test]
fn rejects_unknown_fields_and_type_errors() {
    let result = validate(
        FORM,
        None,
        None,
        r#"{"nope":1,"patient.name":5}"#,
        FormResponseValidationMode::Draft,
    )
    .unwrap();
    assert_eq!(result.errors[0].code, "UNKNOWN_FIELD");
    assert_eq!(result.errors[0].path, "/answers/nope");
    assert_eq!(result.errors[1].code, "INVALID_TYPE");
}

#[test]
fn hidden_fields_reject_values() {
    let ui = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0",
        "fields":{"patient-name":{"hidden":true}}}"#;
    let result = validate(
        FORM,
        Some(ui),
        None,
        r#"{"patient.name":"Ada"}"#,
        FormResponseValidationMode::Draft,
    )
    .unwrap();
    assert_eq!(result.errors[0].code, "HIDDEN_FIELD_VALUE");
}

#[test]
fn calculates_and_overwrites_bmi() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"weight-kg","code":"body.weight.kg","type":"number"},
        {"id":"height-m","code":"body.height.m","type":"number"},
        {"id":"bmi","code":"body.bmi","type":"number","readOnly":true}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "bmi":{"calculate":{"op":"div","args":[
            {"ref":"body.weight.kg"},
            {"op":"mul","args":[{"ref":"body.height.m"},{"ref":"body.height.m"}]}]}}}}"#;
    let result = validate(
        form,
        None,
        Some(rules),
        r#"{"body.weight.kg":70,"body.height.m":1.75,"body.bmi":999}"#,
        FormResponseValidationMode::Draft,
    )
    .unwrap();
    assert!(result.is_valid(), "{:?}", result.errors);
    assert!(
        result
            .normalized_answers_json
            .contains(r#""body.bmi":22.86"#)
    );
}

#[test]
fn rejects_a_duplicate_field_code_when_rules_are_present() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"a","code":"dup","type":"text"},
        {"id":"b","code":"dup","type":"number"}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{}}"#;
    let error = validate(
        form,
        None,
        Some(rules),
        "{}",
        FormResponseValidationMode::Draft,
    )
    .unwrap_err();
    assert!(
        error.message.starts_with("An item with the same key"),
        "{error}"
    );
}

#[test]
fn rejects_a_dependency_error() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"tgt","code":"tgt","type":"text"}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "ghost":{"visibleWhen":{"lit":false}}}}"#;
    let error = validate(
        form,
        None,
        Some(rules),
        "{}",
        FormResponseValidationMode::Draft,
    )
    .unwrap_err();
    assert!(error.message.starts_with("RULE_UNKNOWN_FIELD"), "{error}");
}

#[test]
fn a_duplicate_code_passes_when_no_rules_document_is_supplied() {
    // Absent rules mean there is no form/rules pair to check, so the dependency
    // check (including its duplicate-code rejection) does not run.
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"a","code":"dup","type":"text"},
        {"id":"b","code":"dup","type":"number"}]}"#;
    let result = validate(form, None, None, "{}", FormResponseValidationMode::Draft).unwrap();
    assert!(result.is_valid(), "{:?}", result.errors);
}

#[test]
fn normalizes_dates_and_times() {
    let form = r#"{"fields":[
        {"id":"d","code":"a.d","type":"date"},
        {"id":"t","code":"a.t","type":"time"},
        {"id":"dt","code":"a.dt","type":"datetime"}]}"#;
    let result = validate(
        form,
        None,
        None,
        r#"{"a.d":"2024-1-5","a.t":"9:5:3","a.dt":"2024-01-05T10:00:00Z"}"#,
        FormResponseValidationMode::Draft,
    )
    .unwrap();
    assert!(result.is_valid(), "{:?}", result.errors);
    assert!(
        result
            .normalized_answers_json
            .contains(r#""a.d":"2024-01-05""#)
    );
    assert!(
        result
            .normalized_answers_json
            .contains(r#""a.t":"09:05:03""#)
    );
    assert!(
        result
            .normalized_answers_json
            .contains(r#""a.dt":"2024-01-05T10:00:00.0000000\u002B00:00""#)
    );
}

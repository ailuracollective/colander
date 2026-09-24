use colander::compile::*;
use colander::hash;
use colander::json::{self};

const FORM: &str = r#"{"schemaVersion":"1.0.0","fields":[
    {"id":"patient-name","code":"patient.name","type":"text","required":true}]}"#;

#[test]
fn compiles_without_components() {
    let result = compile(FORM, None, None, &[]).unwrap();
    assert_eq!(
        result.form_schema_json,
        r#"{"fields":[{"code":"patient.name","id":"patient-name","required":true,"type":"text"}],"schemaVersion":"1.0.0"}"#
    );
    assert!(result.ui_schema_json.is_none());
    assert_eq!(result.dependency_metadata_json, r#"{"components":[]}"#);
    assert_eq!(
        result.content_hash,
        hash::content_hash(&result.form_schema_json, None, None).unwrap()
    );
}

#[test]
fn expands_component_references_with_default_layout() {
    let component = ComponentVersionData {
        code: "demographics".into(),
        version: "1.0.0".into(),
        form_schema_json: FORM.into(),
        ui_schema_json: Some(
            r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0",
                "fields":{"patient-name":{"label":"Patient name"}}}"#
                .into(),
        ),
        content_hash: Some("abc".into()),
    };
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"section","code":"section.demographics","type":"component-ref",
         "componentCode":"demographics","componentVersion":"1.0.0"}]}"#;
    let ui = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0",
        "fields":{"section":{"label":"Demographics","widget":"component"}},
        "layout":[{"type":"field","fieldId":"section"}]}"#;

    let result = compile(form, Some(ui), None, std::slice::from_ref(&component)).unwrap();
    let compiled_form = json::parse(&result.form_schema_json).unwrap();
    let fields = json::get_array(compiled_form.as_object().unwrap(), "fields").unwrap();
    let group = fields[0].as_object().unwrap();
    assert_eq!(json::get_str(group, "type"), Some("group"));
    assert_eq!(
        result.dependency_metadata_json,
        r#"{"components":[{"code":"demographics","contentHash":"abc","version":"1.0.0"}]}"#
    );

    let compiled_ui = json::parse(result.ui_schema_json.as_deref().unwrap()).unwrap();
    let layout = json::get_array(compiled_ui.as_object().unwrap(), "layout").unwrap();
    let node = layout[0].as_object().unwrap();
    assert_eq!(json::get_str(node, "type"), Some("group"));
    assert_eq!(
        json::get_array(node, "children").unwrap()[0]
            .as_object()
            .unwrap()
            .get("fieldId")
            .unwrap()
            .as_str(),
        Some("patient-name")
    );
}

#[test]
fn rejects_a_duplicate_field_code_when_rules_are_present() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"a","code":"dup","type":"text"},
        {"id":"b","code":"dup","type":"number"}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{}}"#;
    let error = compile(form, None, Some(rules), &[]).unwrap_err();
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
        "tgt":{"visibleWhen":{"op":"eq","args":[{"ref":"ghost"},{"lit":1}]}}}}"#;
    let error = compile(form, None, Some(rules), &[]).unwrap_err();
    assert!(
        error.message.starts_with("RULE_UNKNOWN_FIELD_REF"),
        "{error}"
    );
}

#[test]
fn accepts_a_duplicate_code_when_no_rules_document_is_supplied() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"a","code":"dup","type":"text"},
        {"id":"b","code":"dup","type":"number"}]}"#;
    compile(form, None, None, &[]).unwrap();
}

#[test]
fn rejects_unpinned_and_missing_components() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"s","code":"s","type":"component-ref","componentCode":"x"}]}"#;
    let error = compile(form, None, None, &[]).unwrap_err();
    assert!(error.message.starts_with("COMPONENT_VERSION_REQUIRED"));

    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"s","code":"s","type":"component-ref","componentCode":"x","componentVersion":"9.9.9"}]}"#;
    let error = compile(form, None, None, &[]).unwrap_err();
    assert!(error.message.starts_with("COMPONENT_VERSION_NOT_FOUND"));
}

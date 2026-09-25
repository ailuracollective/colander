use colander::compile::*;
use colander::ffi::compile::colander_compile;
use colander::ffi::session::call_with_text;
use colander::hash;
use colander::json::{self, Json};

const FORM: &str = r#"{"schemaVersion":"1.0.0","fields":[
    {"id":"patient-name","code":"patient.name","type":"text","required":true}]}"#;

const COMPONENT_FORM: &str = r#"{"schemaVersion":"1.0.0","fields":[
    {"id":"patient-name","code":"patient.name","type":"text","required":true}]}"#;
const COMPONENT_UI: &str = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0",
    "fields":{"patient-name":{"label":"Patient name"}}}"#;
const REF_FORM: &str = r#"{"schemaVersion":"1.0.0","fields":[
    {"id":"section","code":"section.demographics","type":"component-ref",
     "componentCode":"demographics","componentVersion":"1.0.0"}]}"#;

/// The `demographics` component with `pin` as its declared `contentHash`.
fn demographics(pin: Option<&str>) -> ComponentVersionData {
    ComponentVersionData {
        code: "demographics".into(),
        version: "1.0.0".into(),
        form_schema_json: COMPONENT_FORM.into(),
        ui_schema_json: Some(COMPONENT_UI.into()),
        content_hash: pin.map(str::to_string),
    }
}

/// The digest `colander_compile` reports for the component on its own.
fn demographics_pin() -> String {
    compile(COMPONENT_FORM, Some(COMPONENT_UI), None, &[])
        .unwrap()
        .content_hash
}

fn compile_envelope(request: &str) -> Json {
    json::parse(&call_with_text(request, colander_compile)).expect("envelope is JSON")
}

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
fn rejects_invalid_file_configuration() {
    let form = r#"{"fields":[{"id":"file","code":"file","type":"file","maxTotalSize":10}]}"#;
    let error = compile(form, None, None, &[]).unwrap_err();
    assert!(error.message.starts_with("FILE_INVALID_CONFIG"), "{error}");
}

#[test]
fn expands_component_references_with_default_layout() {
    let component = demographics(None);
    let ui = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0",
        "fields":{"section":{"label":"Demographics","widget":"component"}},
        "layout":[{"type":"field","fieldId":"section"}]}"#;

    let result = compile(REF_FORM, Some(ui), None, std::slice::from_ref(&component)).unwrap();
    let compiled_form = json::parse(&result.form_schema_json).unwrap();
    let fields = json::get_array(compiled_form.as_object().unwrap(), "fields").unwrap();
    let group = fields[0].as_object().unwrap();
    assert_eq!(json::get_str(group, "type"), Some("group"));
    // An absent pin is not a pin: the metadata carries the empty string.
    assert_eq!(
        result.dependency_metadata_json,
        r#"{"components":[{"code":"demographics","contentHash":"","version":"1.0.0"}]}"#
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
        error.message.starts_with("RULE_DUPLICATE_FIELD_CODE"),
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
fn rejects_a_duplicate_code_when_no_rules_document_is_supplied() {
    // R-5: the check runs on the effective (compiled) documents, with or
    // without rules.
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"a","code":"dup","type":"text"},
        {"id":"b","code":"dup","type":"number"}]}"#;
    let error = compile(form, None, None, &[]).unwrap_err();
    assert!(
        error.message.starts_with("RULE_DUPLICATE_FIELD_CODE"),
        "{error}"
    );
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

// Component hash pins (SPEC P-3)
// ---------------------------------------------------------------------------

#[test]
fn verifies_a_matching_component_hash() {
    let pin = demographics_pin();
    let component = demographics(Some(&pin));
    let result = compile(REF_FORM, None, None, std::slice::from_ref(&component)).unwrap();
    assert_eq!(
        result.dependency_metadata_json,
        format!(
            r#"{{"components":[{{"code":"demographics","contentHash":"{pin}","version":"1.0.0"}}]}}"#
        )
    );
}

#[test]
fn rejects_a_component_hash_mismatch() {
    let component = demographics(Some(
        "0000000000000000000000000000000000000000000000000000000000000000",
    ));
    let error = compile(REF_FORM, None, None, std::slice::from_ref(&component)).unwrap_err();
    assert!(
        error.message.starts_with("COMPONENT_HASH_MISMATCH"),
        "{error}"
    );
}

#[test]
fn treats_an_empty_component_hash_as_unpinned() {
    let component = demographics(Some(""));
    let result = compile(REF_FORM, None, None, std::slice::from_ref(&component)).unwrap();
    assert!(
        result
            .dependency_metadata_json
            .contains(r#""contentHash":"""#)
    );
}

#[test]
fn verifies_a_nested_component_hash() {
    let b = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"b1","code":"b.1","type":"text"}]}"#;
    let a = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"a1","code":"a.1","type":"text"},
        {"id":"refB","code":"refB","type":"component-ref",
         "componentCode":"B","componentVersion":"2.0.0"}]}"#;
    let a_ui = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0",
        "fields":{"a1":{"label":"A1"}},
        "layout":[{"type":"field","fieldId":"a1"},{"type":"field","fieldId":"refB"}]}"#;
    let b_pin = compile(b, None, None, &[]).unwrap().content_hash;
    let b_component = ComponentVersionData {
        code: "B".into(),
        version: "2.0.0".into(),
        form_schema_json: b.into(),
        ui_schema_json: None,
        content_hash: Some(b_pin),
    };
    let batch = std::slice::from_ref(&b_component);
    let a_pin = compile(a, Some(a_ui), None, batch).unwrap().content_hash;
    let a_component = ComponentVersionData {
        code: "A".into(),
        version: "1.0.0".into(),
        form_schema_json: a.into(),
        ui_schema_json: Some(a_ui.into()),
        content_hash: Some(a_pin.clone()),
    };
    compile(
        r#"{"schemaVersion":"1.0.0","fields":[
            {"id":"refA","code":"refA","type":"component-ref",
             "componentCode":"A","componentVersion":"1.0.0"}]}"#,
        None,
        None,
        &[a_component, b_component],
    )
    .unwrap();

    // A matches, but a wrong pin on the nested component is still caught.
    let broken = ComponentVersionData {
        code: "B".into(),
        version: "2.0.0".into(),
        form_schema_json: b.into(),
        ui_schema_json: None,
        content_hash: Some("deadbeef".into()),
    };
    let error = compile(
        r#"{"schemaVersion":"1.0.0","fields":[
            {"id":"refA","code":"refA","type":"component-ref",
             "componentCode":"A","componentVersion":"1.0.0"}]}"#,
        None,
        None,
        &[
            ComponentVersionData {
                code: "A".into(),
                version: "1.0.0".into(),
                form_schema_json: a.into(),
                ui_schema_json: Some(a_ui.into()),
                content_hash: Some(a_pin),
            },
            broken,
        ],
    )
    .unwrap_err();
    assert!(
        error.message.starts_with("COMPONENT_HASH_MISMATCH"),
        "{error}"
    );
}

// C-8: the C-6 type check reaches nested request keys
// ---------------------------------------------------------------------------

#[test]
fn compile_rejects_a_wrong_typed_component_optional_key() {
    let request = r#"{"formSchemaJson":"{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"r\",\"code\":\"r\",\"type\":\"component-ref\",\"componentCode\":\"c\",\"componentVersion\":\"1.0.0\"}]}","components":[{"code":"c","version":"1.0.0","formSchemaJson":"{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"x\",\"code\":\"x\",\"type\":\"text\"}]}","KEY":7}]}"#;
    for key in ["uiSchemaJson", "contentHash"] {
        let envelope = compile_envelope(&request.replace("KEY", key));
        let object = envelope.as_object().expect("object envelope");
        assert_eq!(json::get_bool(object, "ok"), Some(false), "{key}");
        let error = object.get("error").unwrap().as_object().unwrap();
        assert_eq!(
            json::get_str(error, "kind"),
            Some("validation"),
            "{key} kind"
        );
        let message = json::get_str(error, "message").expect("message");
        assert!(message.contains(key), "{message}");
    }
}

#[test]
fn compile_treats_an_absent_component_hash_as_unpinned() {
    let request = r#"{"formSchemaJson":"{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"r\",\"code\":\"r\",\"type\":\"component-ref\",\"componentCode\":\"c\",\"componentVersion\":\"1.0.0\"}]}","components":[{"code":"c","version":"1.0.0","formSchemaJson":"{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"x\",\"code\":\"x\",\"type\":\"text\"}]}"}]}"#;
    let envelope = compile_envelope(request);
    assert_eq!(
        json::get_bool(envelope.as_object().unwrap(), "ok"),
        Some(true)
    );
}

#[test]
fn compile_reports_a_component_hash_mismatch_as_validation() {
    let request = r#"{"formSchemaJson":"{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"r\",\"code\":\"r\",\"type\":\"component-ref\",\"componentCode\":\"c\",\"componentVersion\":\"1.0.0\"}]}","components":[{"code":"c","version":"1.0.0","formSchemaJson":"{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"x\",\"code\":\"x\",\"type\":\"text\"}]}","contentHash":"deadbeef"}]}"#;
    let envelope = compile_envelope(request);
    let object = envelope.as_object().unwrap();
    assert_eq!(json::get_bool(object, "ok"), Some(false));
    let error = object.get("error").unwrap().as_object().unwrap();
    assert_eq!(json::get_str(error, "kind"), Some("validation"));
    let message = json::get_str(error, "message").expect("message");
    assert!(message.starts_with("COMPONENT_HASH_MISMATCH"), "{message}");
}

use std::ffi::{CStr, c_char};

use colander::ffi::compile::colander_compile;
use colander::ffi::envelope::colander_free_string;
use colander::ffi::memory::{colander_alloc, colander_free_buffer};
use colander::ffi::response::colander_validate_response;
use colander::ffi::rules::colander_evaluate_rules;
use colander::ffi::schema::colander_validate_schema;
use colander::ffi::session::call_with_text;
use colander::ffi::version::{colander_content_hash, colander_next_version, colander_version_info};
use colander::json::{self, Json};

fn call(entry: unsafe extern "C" fn(*const c_char) -> *mut c_char, text: &str) -> Json {
    let output = call_with_text(text, entry);
    json::parse(&output).expect("envelope is JSON")
}

#[test]
fn compiles_through_the_abi() {
    let envelope = call(
        colander_compile,
        r#"{"formSchemaJson":"{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"a\",\"code\":\"a\",\"type\":\"text\"}]}"}"#,
    );
    assert_eq!(
        json::get_bool(envelope.as_object().unwrap(), "ok"),
        Some(true)
    );
    assert_eq!(
        json::get_str(
            envelope
                .as_object()
                .unwrap()
                .get("result")
                .unwrap()
                .as_object()
                .unwrap(),
            "contentHash"
        )
        .unwrap()
        .len(),
        64
    );
}

#[test]
fn reports_errors_as_an_envelope() {
    let envelope = call(colander_compile, r#"{"formSchemaJson":"nope"}"#);
    let object = envelope.as_object().unwrap();
    assert_eq!(json::get_bool(object, "ok"), Some(false));
    assert_eq!(
        json::get_str(object.get("error").unwrap().as_object().unwrap(), "kind"),
        Some("validation")
    );
}

#[test]
fn unknown_mode_is_an_error() {
    let envelope = call(
        colander_validate_response,
        r#"{"formSchemaJson":"{\"fields\":[]}","answersJson":"{}","mode":"Sideways"}"#,
    );
    assert_eq!(
        json::get_bool(envelope.as_object().unwrap(), "ok"),
        Some(false)
    );
}

#[test]
fn validates_against_a_caller_supplied_schema() {
    // `instance` is the domain-free entry point: any document, any schema.
    let envelope = call(
        colander_validate_schema,
        r#"{"kind":"instance","schemaJson":"{\"type\":\"object\",\"required\":[\"id\"]}",
            "instanceJson":"{\"id\":1}","label":"thing"}"#,
    );
    assert_eq!(
        envelope
            .as_object()
            .unwrap()
            .get("result")
            .and_then(Json::as_object)
            .and_then(|result| json::get_bool(result, "valid")),
        Some(true)
    );

    let envelope = call(
        colander_validate_schema,
        r#"{"kind":"instance","schemaJson":"{\"type\":\"object\",\"required\":[\"id\"]}",
            "instanceJson":"{}","label":"thing"}"#,
    );
    let object = envelope.as_object().unwrap();
    assert_eq!(json::get_bool(object, "ok"), Some(false));
    let message = json::get_str(object.get("error").unwrap().as_object().unwrap(), "message")
        .expect("message");
    assert!(message.starts_with("Invalid thing: "), "{message}");
}

#[test]
fn a_kind_that_needs_a_schema_says_so() {
    let envelope = call(
        colander_validate_schema,
        r#"{"kind":"form","formSchemaJson":"{\"fields\":[]}"}"#,
    );
    let object = envelope.as_object().unwrap();
    assert_eq!(json::get_bool(object, "ok"), Some(false));
    let message = json::get_str(object.get("error").unwrap().as_object().unwrap(), "message")
        .expect("message");
    assert!(message.contains("schemas is required"), "{message}");

    let envelope = call(
        colander_validate_schema,
        r#"{"kind":"form","formSchemaJson":"{\"fields\":[]}","schemas":{"uiSchema":"{}"}}"#,
    );
    let object = envelope.as_object().unwrap();
    let message = json::get_str(object.get("error").unwrap().as_object().unwrap(), "message")
        .expect("message");
    assert!(
        message.contains("schemas.formSchema is required"),
        "{message}"
    );

    let envelope = call(
        colander_validate_schema,
        r#"{"kind":"form","formSchemaJson":"{\"schemaVersion\":\"1.0.0\"}",
            "schemas":{"formSchema":"{\"type\":\"object\",\"required\":[\"schemaVersion\"]}"}}"#,
    );
    assert_eq!(
        envelope
            .as_object()
            .unwrap()
            .get("result")
            .and_then(Json::as_object)
            .and_then(|result| json::get_bool(result, "valid")),
        Some(true)
    );
}

fn quoted(text: &str) -> String {
    let mut out = String::new();
    json::write_string(&mut out, text);
    out
}

fn assert_validation_envelope(form: &str, rules: &str, needle: &str) {
    let request = format!(
        "{{\"formSchemaJson\":{},\"rulesSchemaJson\":{},\"answersJson\":\"{{}}\"}}",
        quoted(form),
        quoted(rules)
    );
    for entry in [
        colander_evaluate_rules,
        colander_validate_response,
        colander_compile,
    ] {
        let envelope = call(entry, &request);
        let object = envelope.as_object().unwrap();
        assert_eq!(json::get_bool(object, "ok"), Some(false));
        let error = object.get("error").unwrap().as_object().unwrap();
        assert_eq!(json::get_str(error, "kind"), Some("validation"));
        let message = json::get_str(error, "message").expect("message");
        assert!(message.contains(needle), "{message}");
    }
}

#[test]
fn duplicate_field_codes_are_validation_envelopes() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"a","code":"dup","type":"text"},
        {"id":"b","code":"dup","type":"number"}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{}}"#;
    assert_validation_envelope(form, rules, "same key");
}

#[test]
fn rule_dependency_errors_are_validation_envelopes() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"tgt","code":"tgt","type":"text"}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "tgt":{"visibleWhen":{"op":"eq","args":[{"ref":"ghost"},{"lit":1}]}}}}"#;
    assert_validation_envelope(form, rules, "RULE_UNKNOWN_FIELD_REF");
}

#[test]
fn version_info_is_reported() {
    let pointer = colander_version_info();
    let envelope =
        json::parse(&unsafe { CStr::from_ptr(pointer) }.to_string_lossy()).expect("JSON");
    unsafe { colander_free_string(pointer) };
    assert_eq!(
        json::get_bool(envelope.as_object().unwrap(), "ok"),
        Some(true)
    );
}

#[test]
fn null_and_invalid_utf8_are_handled() {
    let pointer = unsafe { colander_compile(std::ptr::null()) };
    assert!(!pointer.is_null());
    let text = unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned();
    unsafe { colander_free_string(pointer) };
    assert!(text.contains("invalid_request"));

    let pointer = unsafe { colander_compile(c"not json".as_ptr()) };
    assert!(!pointer.is_null());
    let text = unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned();
    unsafe { colander_free_string(pointer) };
    assert!(text.contains("\"ok\":false"));
}

#[test]
fn a_caller_can_build_its_own_request_buffer() {
    // The path to take when the caller has no allocator for this library's
    // heap, so the library hands one out.
    let text = r#"{"formSchemaJson":"{\"fields\":[]}"}"#;
    let length = text.len() + 1;
    let buffer = colander_alloc(length);
    assert!(!buffer.is_null());
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr(), buffer, text.len());
        *buffer.add(text.len()) = 0;
    }

    let pointer = unsafe { colander_compile(buffer.cast_const().cast()) };
    unsafe { colander_free_buffer(buffer, length) };

    let envelope =
        json::parse(&unsafe { CStr::from_ptr(pointer) }.to_string_lossy()).expect("JSON");
    unsafe { colander_free_string(pointer) };
    assert_eq!(
        json::get_bool(envelope.as_object().unwrap(), "ok"),
        Some(true)
    );
}

#[test]
fn the_allocator_refuses_a_zero_length() {
    assert!(colander_alloc(0).is_null());
    // Freeing null or zero is a no-op rather than a double free.
    unsafe { colander_free_buffer(std::ptr::null_mut(), 16) };
    unsafe { colander_free_buffer(colander_alloc(16), 0) };
}

/// A present optional key of the wrong JSON type must fail, and the failure
/// must be the request failing its own type check: `kind: "validation"`, the
/// same envelope `kind` a required-key type failure surfaces with — not the
/// structural `invalid_request` of an envelope that never parsed.
fn assert_validation_failure(
    entry: unsafe extern "C" fn(*const c_char) -> *mut c_char,
    request: &str,
) {
    let envelope = call(entry, request);
    let object = envelope.as_object().unwrap();
    assert_eq!(json::get_bool(object, "ok"), Some(false), "{request}");
    let error = object.get("error").unwrap().as_object().unwrap();
    assert_eq!(
        json::get_str(error, "kind"),
        Some("validation"),
        "{request}"
    );
}

#[test]
fn compile_rejects_a_wrong_typed_optional_key() {
    // A required key of the wrong type already failed with this `kind`; the
    // optional failures below must surface the same way.
    assert_validation_failure(colander_compile, r#"{"formSchemaJson":3}"#);
    // `uiSchemaJson` must be a string when present.
    assert_validation_failure(
        colander_compile,
        r#"{"formSchemaJson":"{\"fields\":[]}","uiSchemaJson":3}"#,
    );
    // `components` must be an array when present.
    assert_validation_failure(
        colander_compile,
        r#"{"formSchemaJson":"{\"fields\":[]}","components":{}}"#,
    );
}

#[test]
fn evaluate_rules_rejects_a_wrong_typed_optional_key() {
    // `values` must be an object when present.
    assert_validation_failure(
        colander_evaluate_rules,
        r#"{"formSchemaJson":"{\"fields\":[]}","rulesSchemaJson":"{\"fields\":{}}","values":3}"#,
    );
}

#[test]
fn validate_response_rejects_a_wrong_typed_optional_key() {
    // `mode` must be a string when present.
    assert_validation_failure(
        colander_validate_response,
        r#"{"formSchemaJson":"{\"fields\":[]}","answersJson":"{}","mode":3}"#,
    );
}

#[test]
fn validate_schema_rejects_a_wrong_typed_optional_key() {
    // `kind` must be a string when present.
    assert_validation_failure(colander_validate_schema, r#"{"kind":3}"#);
    // `label` must be a string when present.
    assert_validation_failure(
        colander_validate_schema,
        r#"{"kind":"instance","schemaJson":"{}","instanceJson":"{}","label":3}"#,
    );
}

// C-9: a present `schemas` must be an object in every kind, `instance` included.
#[test]
fn instance_rejects_a_wrong_typed_schemas() {
    assert_validation_failure(
        colander_validate_schema,
        r#"{"kind":"instance","schemaJson":"{}","instanceJson":"{}","schemas":3}"#,
    );
    // A request with no `schemas` at all still validates.
    let envelope = call(
        colander_validate_schema,
        r#"{"kind":"instance","schemaJson":"{}","instanceJson":"{}"}"#,
    );
    assert_eq!(
        json::get_bool(envelope.as_object().unwrap(), "ok"),
        Some(true)
    );
}

#[test]
fn workflow_retires_published() {
    // `published` was accepted and read by nothing; it is not accepted now (SPEC X-2).
    let base = r#"{"kind":"workflow","workflowSchemaJson":"{}","published":@,
        "schemas":{"workflowSchema":"{}"}}"#;
    assert_validation_failure(colander_validate_schema, &base.replace('@', "true"));
    assert_validation_failure(colander_validate_schema, &base.replace('@', "3"));

    let envelope = call(
        colander_validate_schema,
        r#"{"kind":"workflow","workflowSchemaJson":"{}","schemas":{"workflowSchema":"{}"}}"#,
    );
    let result = envelope.as_object().unwrap().get("result").unwrap();
    let valid = json::get_bool(result.as_object().unwrap(), "valid");
    assert_eq!(valid, Some(true));
}

#[test]
fn content_hash_rejects_a_wrong_typed_optional_key() {
    // `rulesSchemaJson` must be a string when present.
    assert_validation_failure(
        colander_content_hash,
        r#"{"formSchemaJson":"{}","rulesSchemaJson":3}"#,
    );
}

#[test]
fn next_version_rejects_a_wrong_typed_optional_key() {
    // `published` must be an array when present.
    assert_validation_failure(colander_next_version, r#"{"published":3}"#);
}

#[test]
fn an_explicit_null_is_a_wrong_type_not_an_absence() {
    // C-6 treats `null` as a present value of the wrong type, so a caller that
    // serialises an absent optional value as `null` must omit the key instead.
    assert_validation_failure(
        colander_validate_response,
        r#"{"formSchemaJson":"{\"fields\":[]}","answersJson":"{}","mode":null}"#,
    );
    assert_validation_failure(
        colander_validate_response,
        r#"{"formSchemaJson":"{\"fields\":[]}","answersJson":"{}","rulesSchemaJson":null}"#,
    );
}

#[test]
fn absent_optional_keys_take_their_default() {
    // Absent `mode` defaults to Draft and absent rules mean "no rules".
    let envelope = call(
        colander_validate_response,
        r#"{"formSchemaJson":"{\"schemaVersion\":\"1.0.0\",\"fields\":[]}","answersJson":"{}"}"#,
    );
    assert_eq!(
        json::get_bool(envelope.as_object().unwrap(), "ok"),
        Some(true)
    );

    // Blank `rulesSchemaJson` is still "no rules", as documented.
    let envelope = call(
        colander_validate_response,
        r#"{"formSchemaJson":"{\"schemaVersion\":\"1.0.0\",\"fields\":[]}",
            "answersJson":"{}","rulesSchemaJson":""}"#,
    );
    assert_eq!(
        json::get_bool(envelope.as_object().unwrap(), "ok"),
        Some(true)
    );

    // Absent `components` means "no components".
    let envelope = call(colander_compile, r#"{"formSchemaJson":"{\"fields\":[]}"}"#);
    assert_eq!(
        json::get_bool(envelope.as_object().unwrap(), "ok"),
        Some(true)
    );

    // Absent `published` means "nothing published".
    let envelope = call(colander_next_version, "{}");
    assert_eq!(
        envelope
            .as_object()
            .unwrap()
            .get("result")
            .and_then(Json::as_object)
            .and_then(|result| json::get_str(result, "next")),
        Some("1.0.0")
    );
}

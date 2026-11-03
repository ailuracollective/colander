use std::ffi::c_char;

use colander::ffi::response::colander_validate_response;
use colander::ffi::session::call_with_text;
use colander::json::{self, Json};
use colander::validate::*;

fn call(entry: unsafe extern "C" fn(*const c_char) -> *mut c_char, text: &str) -> Json {
    let output = call_with_text(text, entry);
    json::parse(&output).expect("envelope is JSON")
}

// V-12: a page-scoped Complete checks the page's fields only; the final
// unscoped call still reports the other page's required field.
#[test]
fn page_scoped_complete_checks_only_that_page() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"first","code":"first","type":"text","required":true},
        {"id":"second","code":"second","type":"text","required":true}]}"#;
    let ui = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","layout":[
        {"type":"page","id":"one","title":"One","children":[
            {"type":"group","children":[{"type":"field","fieldId":"first"}]}]},
        {"type":"page","id":"two","title":"Two","children":[
            {"type":"field","fieldId":"second"}]}]}"#;

    let page = validate_page(form, Some(ui), None, r#"{"first":"Ada"}"#, "one").unwrap();
    assert!(page.is_valid(), "{:?}", page.errors);
    assert_eq!(page.normalized_answers_json, r#"{"first":"Ada"}"#);

    let complete = validate(
        form,
        Some(ui),
        None,
        r#"{"first":"Ada"}"#,
        FormResponseValidationMode::Complete,
    )
    .unwrap();
    assert_eq!(complete.errors.len(), 1);
    assert_eq!(complete.errors[0].code, "REQUIRED_FIELD_MISSING");
    assert_eq!(complete.errors[0].path, "/fields/1");
}

// V-12: cross-field, calculated and repeater checks outside the page wait for
// the final call; the page's normalized answers hold only the page.
#[test]
fn page_scoped_complete_defers_checks_outside_the_page() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"kind","code":"kind","type":"text"},
        {"id":"age","code":"age","type":"integer"},
        {"id":"total","code":"total","type":"number","readOnly":true},
        {"id":"lines","code":"lines","type":"repeater","minItems":1,"items":[
            {"id":"qty","code":"qty","type":"integer"}]}]}"#;
    let ui = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","layout":[
        {"type":"page","id":"one","children":[{"type":"field","fieldId":"kind"}]},
        {"type":"page","id":"two","children":[
            {"type":"field","fieldId":"age"},
            {"type":"field","fieldId":"total"},
            {"type":"repeater","fieldId":"lines","itemTemplate":[
                {"type":"field","fieldId":"qty"}]}]}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "total":{"calculate":{"op":"add","args":[{"ref":"age"},{"lit":1}]}}},
        "validations":[{"code":"AGE_MIN",
            "assert":{"op":"gte","args":[{"ref":"age"},{"lit":18}]},
            "message":"Age must be 18 or over."}]}"#;
    let answers = r#"{"kind":"x","age":10,"total":999,"lines":[]}"#;

    let page = validate_page(form, Some(ui), Some(rules), answers, "one").unwrap();
    assert!(page.is_valid(), "{:?}", page.errors);
    assert_eq!(page.normalized_answers_json, r#"{"kind":"x"}"#);

    let complete = validate(
        form,
        Some(ui),
        Some(rules),
        answers,
        FormResponseValidationMode::Complete,
    )
    .unwrap();
    let codes: Vec<&str> = complete
        .errors
        .iter()
        .map(|error| error.code.as_str())
        .collect();
    assert!(codes.contains(&"AGE_MIN"), "{codes:?}");
    assert!(codes.contains(&"CALCULATED_VALUE_MISMATCH"), "{codes:?}");
    assert!(codes.contains(&"REPEATER_MIN_ITEMS"), "{codes:?}");
}

#[test]
fn page_scoped_validation_rejects_an_unknown_page() {
    let ui = r#"{"layout":[{"type":"page","id":"one","children":[]}]}"#;
    let error = validate_page(r#"{"fields":[]}"#, Some(ui), None, "{}", "nope").unwrap_err();
    assert!(error.message.starts_with("UI_UNKNOWN_PAGE"), "{error}");

    let error = validate_page(r#"{"fields":[]}"#, None, None, "{}", "one").unwrap_err();
    assert!(error.message.starts_with("UI_UNKNOWN_PAGE"), "{error}");
}

#[test]
fn page_id_scopes_a_complete_response_through_the_abi() {
    let form = r#"{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"first\",\"code\":\"first\",\"type\":\"text\",\"required\":true},{\"id\":\"second\",\"code\":\"second\",\"type\":\"text\",\"required\":true}]}"#;
    let ui = r#"{\"layout\":[{\"type\":\"page\",\"id\":\"one\",\"children\":[{\"type\":\"field\",\"fieldId\":\"first\"}]}]}"#;
    let request = format!(
        r#"{{"formSchemaJson":"{form}","uiSchemaJson":"{ui}","answersJson":"{{\"first\":\"Ada\"}}","mode":"Complete","pageId":"one"}}"#
    );
    let envelope = call(colander_validate_response, &request);
    let result = envelope
        .as_object()
        .unwrap()
        .get("result")
        .unwrap()
        .as_object()
        .unwrap();
    assert_eq!(json::get_bool(result, "isValid"), Some(true));
    assert_eq!(
        json::get_str(result, "normalizedAnswersJson"),
        Some(r#"{"first":"Ada"}"#)
    );

    let request = format!(
        r#"{{"formSchemaJson":"{form}","uiSchemaJson":"{ui}","answersJson":"{{}}","pageId":"one"}}"#
    );
    let envelope = call(colander_validate_response, &request);
    let object = envelope.as_object().unwrap();
    assert_eq!(json::get_bool(object, "ok"), Some(false));
    let message = json::get_str(object.get("error").unwrap().as_object().unwrap(), "message")
        .expect("message");
    assert!(
        message.starts_with("UI_PAGE_REQUIRES_COMPLETE"),
        "{message}"
    );
}

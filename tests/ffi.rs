use std::ffi::{CStr, c_char};

use colander::ffi::compile::colander_compile;
use colander::ffi::envelope::colander_free_string;
use colander::ffi::memory::{colander_alloc, colander_free_buffer};
use colander::ffi::response::colander_validate_response;
use colander::ffi::schema::colander_validate_schema;
use colander::ffi::session::call_with_text;
use colander::ffi::version::colander_version_info;
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

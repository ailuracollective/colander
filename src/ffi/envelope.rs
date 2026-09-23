//! The error envelope and the panic boundary.

use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::error::{ColanderError, Result};
use crate::json::{self, Json, JsonMap};

pub const ABI_VERSION: u32 = 1;

/// Error taxonomy exposed to callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The request envelope was not valid JSON / had missing fields.
    InvalidRequest,
    /// The core rejected the payload as invalid.
    Validation,
    /// A Rust panic at the boundary, caught before it could unwind.
    Panic,
}

impl ErrorKind {
    fn as_str(self) -> &'static str {
        match self {
            ErrorKind::InvalidRequest => "invalid_request",
            ErrorKind::Validation => "validation",
            ErrorKind::Panic => "panic",
        }
    }
}

/// Release a string this library handed out.
///
/// # Safety
/// `pointer` must come from a `colander_*` function and must not be used after.
pub unsafe fn free_string(pointer: *mut c_char) {
    if pointer.is_null() {
        return;
    }
    // SAFETY: the caller's contract is that this pointer came from CString::into_raw.
    unsafe {
        drop(CString::from_raw(pointer));
    }
}

/// `colander_free_string`.
///
/// # Safety
/// See [`free_string`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colander_free_string(pointer: *mut c_char) {
    unsafe { free_string(pointer) };
}

/// ABI version, so bindings can fail fast on a mismatch.
#[unsafe(no_mangle)]
pub extern "C" fn colander_abi_version() -> u32 {
    ABI_VERSION
}

// ---------------------------------------------------------------------------
// Plumbing
// ---------------------------------------------------------------------------

pub fn require_string(request: &JsonMap, key: &str) -> Result<String> {
    json::get_str(request, key)
        .map(str::to_string)
        .ok_or_else(|| ColanderError::new(format!("'{key}' is required and must be a string.")))
}

pub fn optional_string(request: &JsonMap, key: &str) -> Option<String> {
    json::get_str(request, key).map(str::to_string)
}

pub fn optional_text(value: Option<String>) -> Json {
    value.map_or(Json::Null, Json::String)
}

/// Parse the request, run `body`, and always return an envelope.
///
/// The `catch_unwind` calls below contain a panic and turn it into a failure
/// envelope, so no panic ever unwinds into the C caller.
///
/// # Safety
/// `request` follows the contract of the exported entry points: it must be a
/// valid NUL-terminated UTF-8 C string, or null.
pub unsafe fn dispatch<F>(request: *const c_char, body: F) -> *mut c_char
where
    F: FnOnce(&JsonMap) -> Result<Json>,
{
    // Stage 1: bytes -> request object. Failures here are the caller's fault.
    let decoded = catch_unwind(AssertUnwindSafe(|| -> Result<JsonMap> {
        // SAFETY: the exported entry points forward their caller's contract.
        let text = unsafe { read_request(request) }?;
        json::parse_object(&text, "request")
    }));

    let parsed = match decoded {
        Ok(Ok(parsed)) => parsed,
        Ok(Err(error)) => return into_envelope_with_kind(Err(error), ErrorKind::InvalidRequest),
        Err(payload) => {
            return into_envelope_with_kind(Err(panic_message(payload)), ErrorKind::Panic);
        }
    };

    // Stage 2: the core itself. Failures here are validation failures.
    match catch_unwind(AssertUnwindSafe(|| body(&parsed))) {
        Ok(result) => into_envelope(result),
        Err(payload) => into_envelope_with_kind(Err(panic_message(payload)), ErrorKind::Panic),
    }
}

/// # Safety
/// `request` must be null or NUL-terminated valid UTF-8.
unsafe fn read_request(request: *const c_char) -> Result<String> {
    if request.is_null() {
        return Err(ColanderError::new("request pointer is null"));
    }
    // SAFETY: the caller guarantees a NUL-terminated string.
    let text = unsafe { CStr::from_ptr(request) };
    text.to_str()
        .map(str::to_string)
        .map_err(|_| ColanderError::new("request is not valid UTF-8"))
}

pub fn panic_message(payload: Box<dyn std::any::Any + Send>) -> ColanderError {
    let message = match payload.downcast_ref::<&str>() {
        Some(message) => (*message).to_string(),
        None => match payload.downcast_ref::<String>() {
            Some(message) => message.clone(),
            None => "unknown panic".to_string(),
        },
    };
    ColanderError::new(format!("colander panicked: {message}"))
}

pub fn into_envelope(result: Result<Json>) -> *mut c_char {
    into_envelope_with_kind(result, ErrorKind::Validation)
}

pub fn into_envelope_with_kind(result: Result<Json>, kind: ErrorKind) -> *mut c_char {
    let envelope = match result {
        Ok(value) => {
            let mut out = JsonMap::new();
            out.insert("ok".to_string(), Json::Bool(true));
            out.insert("result".to_string(), value);
            Json::Object(out)
        }
        Err(error) => {
            let mut detail = JsonMap::new();
            detail.insert("kind".to_string(), Json::String(kind.as_str().to_string()));
            detail.insert("message".to_string(), Json::String(error.message));
            let mut out = JsonMap::new();
            out.insert("ok".to_string(), Json::Bool(false));
            out.insert("error".to_string(), Json::Object(detail));
            Json::Object(out)
        }
    };

    let text = json::ordered(&envelope);
    match CString::new(text) {
        Ok(value) => value.into_raw(),
        Err(_) => CString::new(
            r#"{"ok":false,"error":{"kind":"panic","message":"response contained a NUL byte"}}"#,
        )
        .expect("static envelope has no NUL")
        .into_raw(),
    }
}

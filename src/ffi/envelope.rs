//! The error envelope and the panic boundary.

use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::codec::Codec;
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

/// Read an optional key that must be a string when present.
///
/// An absent key is `Ok(None)` and takes its default. A present key of the
/// wrong JSON type is an error: "present but wrong" must not be
/// indistinguishable from "absent" ([SPEC](../../SPEC.md) C-6). The error
/// propagates out of the entry point's body, so it surfaces with the same
/// envelope `kind` (`validation`) as a required-key type failure.
pub fn optional_string(request: &JsonMap, key: &str) -> Result<Option<String>> {
    optional_typed(request, key, "string", |value| {
        value.as_str().map(str::to_string)
    })
}

/// Read an optional key that must be an array when present. See
/// [`optional_string`].
pub fn optional_array<'a>(request: &'a JsonMap, key: &str) -> Result<Option<&'a Vec<Json>>> {
    optional_typed(request, key, "array", Json::as_array)
}

/// Read an optional key that must be an object when present. See
/// [`optional_string`].
pub fn optional_object<'a>(request: &'a JsonMap, key: &str) -> Result<Option<&'a JsonMap>> {
    optional_typed(request, key, "object", Json::as_object)
}

/// Read an optional key that must be a boolean when present. See
/// [`optional_string`].
pub fn optional_bool(request: &JsonMap, key: &str) -> Result<Option<bool>> {
    optional_typed(request, key, "boolean", Json::as_bool)
}

/// The shared shape of the optional accessors: absent is `None`, a value of
/// the expected type is `Some`, and any other type is a validation error.
fn optional_typed<'a, T>(
    request: &'a JsonMap,
    key: &str,
    expected: &str,
    read: impl FnOnce(&'a Json) -> Option<T>,
) -> Result<Option<T>> {
    match request.get(key) {
        None => Ok(None),
        Some(value) => read(value).map(Some).ok_or_else(|| {
            ColanderError::new(format!("'{key}' must be a {expected} when present."))
        }),
    }
}

pub fn optional_text(value: Option<String>) -> Json {
    value.map_or(Json::Null, Json::String)
}

/// Decode `bytes` with `codec` and require the result to be a request object.
///
/// The two failure shapes are the ones `json::parse_object` produced before the
/// seam: a codec failure (syntax or encoding), or the `expected a JSON object`
/// message for a document of the wrong type.
fn request_object<C: Codec>(codec: &C, bytes: &[u8]) -> Result<JsonMap> {
    let value = codec.decode(bytes, "request")?;
    match value {
        Json::Object(map) => Ok(map),
        _ => Err(ColanderError::new(
            "Invalid request: expected a JSON object.",
        )),
    }
}

/// The codec-generic entry point: decode `bytes`, require a request object, and
/// run `body`.
///
/// The FFI shares [`request_object`] with it so the two boundaries cannot drift,
/// and Rust callers can drive an alternate codec end to end (binary cannot cross
/// the NUL-terminated `char *` ABI).
pub fn run<C: Codec, F>(codec: &C, bytes: &[u8], body: F) -> Result<Json>
where
    F: FnOnce(&JsonMap) -> Result<Json>,
{
    let request = request_object(codec, bytes)?;
    body(&request)
}

/// Parse the request, run `body`, and always return an envelope.
///
/// The `catch_unwind` calls below contain a panic and turn it into a failure
/// envelope, so no panic ever unwinds into the C caller.
///
/// # Safety
/// `request` follows the contract of the exported entry points: it must be a
/// valid NUL-terminated C string, or null.
pub unsafe fn dispatch<C, F>(codec: &C, request: *const c_char, body: F) -> *mut c_char
where
    C: Codec,
    F: FnOnce(&JsonMap) -> Result<Json>,
{
    // Stage 1: bytes -> request object. Failures here are the caller's fault.
    let decoded = catch_unwind(AssertUnwindSafe(|| -> Result<JsonMap> {
        // SAFETY: the exported entry points forward their caller's contract.
        let bytes = unsafe { read_request(request) }?;
        request_object(codec, bytes)
    }));

    let parsed = match decoded {
        Ok(Ok(parsed)) => parsed,
        Ok(Err(error)) => {
            return into_envelope_with_kind(codec, Err(error), ErrorKind::InvalidRequest);
        }
        Err(payload) => {
            return into_envelope_with_kind(codec, Err(panic_message(payload)), ErrorKind::Panic);
        }
    };

    // Stage 2: the core itself. Failures here are validation failures.
    match catch_unwind(AssertUnwindSafe(|| body(&parsed))) {
        Ok(result) => into_envelope(codec, result),
        Err(payload) => {
            into_envelope_with_kind(codec, Err(panic_message(payload)), ErrorKind::Panic)
        }
    }
}

/// # Safety
/// `request` must be null or NUL-terminated. The returned bytes borrow the
/// pointer's storage and must not outlive it.
unsafe fn read_request<'a>(request: *const c_char) -> Result<&'a [u8]> {
    if request.is_null() {
        return Err(ColanderError::new("request pointer is null"));
    }
    // SAFETY: the caller guarantees a NUL-terminated string.
    Ok(unsafe { CStr::from_ptr(request) }.to_bytes())
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

pub fn into_envelope<C: Codec>(codec: &C, result: Result<Json>) -> *mut c_char {
    into_envelope_with_kind(codec, result, ErrorKind::Validation)
}

pub fn into_envelope_with_kind<C: Codec>(
    codec: &C,
    result: Result<Json>,
    kind: ErrorKind,
) -> *mut c_char {
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

    let encoded = codec.encode_ordered(&envelope);
    match CString::new(encoded) {
        Ok(value) => value.into_raw(),
        Err(_) => CString::new(
            r#"{"ok":false,"error":{"kind":"panic","message":"response contained a NUL byte"}}"#,
        )
        .expect("static envelope has no NUL")
        .into_raw(),
    }
}

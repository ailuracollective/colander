//! `call_with_text`: the entry point used by Rust callers.

use std::ffi::{CStr, CString, c_char};

use super::envelope::colander_free_string;

/// Convenience wrapper for Rust callers and tests: returns the envelope text
/// instead of a raw pointer.
pub fn call_with_text(
    text: &str,
    entry: unsafe extern "C" fn(*const c_char) -> *mut c_char,
) -> String {
    let input = CString::new(text).expect("no NUL in test input");
    // SAFETY: `input` outlives the call and is NUL-terminated.
    let pointer = unsafe { entry(input.as_ptr()) };
    // SAFETY: the pointer came from CString::into_raw inside the entry point.
    let output = unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned();
    unsafe { colander_free_string(pointer) };
    output
}

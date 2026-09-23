//! `colander_content_hash` and `colander_next_version`.

use std::ffi::c_char;

use crate::error::{ColanderError, Result};
use crate::json::{Json, JsonMap};
use crate::{hash, semver};

use super::envelope::{ABI_VERSION, dispatch, into_envelope, optional_string, require_string};

/// `colander_content_hash`.
///
/// # Safety
/// `request` must be a valid NUL-terminated UTF-8 C string (or null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colander_content_hash(request: *const c_char) -> *mut c_char {
    unsafe {
        dispatch(request, |request| {
            let form = require_string(request, "formSchemaJson")?;
            let ui = optional_string(request, "uiSchemaJson");
            let rules_json = optional_string(request, "rulesSchemaJson");
            let hash = hash::content_hash(&form, ui.as_deref(), rules_json.as_deref())?;
            let mut out = JsonMap::new();
            out.insert("contentHash".to_string(), Json::String(hash));
            Ok(Json::Object(out))
        })
    }
}

/// `colander_next_version`.
///
/// # Safety
/// `request` must be a valid NUL-terminated UTF-8 C string (or null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colander_next_version(request: *const c_char) -> *mut c_char {
    unsafe {
        dispatch(request, |request| {
            let published: Vec<String> = request
                .get("published")
                .and_then(Json::as_array)
                .map(|items| {
                    items
                        .iter()
                        .map(|item| {
                            item.as_str().map(str::to_string).ok_or_else(|| {
                                ColanderError::new("published[] must contain strings.")
                            })
                        })
                        .collect::<Result<Vec<_>>>()
                })
                .transpose()?
                .unwrap_or_default();

            let mut out = JsonMap::new();
            out.insert(
                "next".to_string(),
                Json::String(semver::next_version(&published)?),
            );
            Ok(Json::Object(out))
        })
    }
}

/// `colander_version_info`.
#[unsafe(no_mangle)]
pub extern "C" fn colander_version_info() -> *mut c_char {
    let mut out = JsonMap::new();
    out.insert("name".to_string(), Json::String("colander".to_string()));
    out.insert(
        "version".to_string(),
        Json::String(env!("CARGO_PKG_VERSION").to_string()),
    );
    out.insert("abi".to_string(), Json::integer(ABI_VERSION as i64));
    into_envelope(Ok(Json::Object(out)))
}

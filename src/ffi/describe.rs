//! `colander_describe_form`.

use std::ffi::c_char;

use crate::codec::json::JsonCodec;
use crate::compile;
use crate::describe;
use crate::error::Result;
use crate::json::JsonMap;

use super::compile::component_from_json;
use super::envelope::{dispatch, optional_array, optional_string, require_string};

/// `colander_describe_form`.
///
/// # Safety
/// `request` must be a valid NUL-terminated UTF-8 C string (or null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colander_describe_form(request: *const c_char) -> *mut c_char {
    unsafe {
        dispatch(&JsonCodec, request, |request| {
            let form = require_string(request, "formSchemaJson")?;
            let ui = optional_string(request, "uiSchemaJson")?;
            let rules_json = optional_string(request, "rulesSchemaJson")?;
            let components = components(request)?;

            // The description is of the *compiled* triple, so a `component-ref`
            // is reported by its expansion and the hash describes exactly what
            // was described. Compiling first also means a document the core
            // refuses never produces a partial index: it produces an envelope.
            let compiled =
                compile::compile(&form, ui.as_deref(), rules_json.as_deref(), &components)?;
            describe::describe(&compiled)
        })
    }
}

fn components(request: &JsonMap) -> Result<Vec<compile::ComponentVersionData>> {
    optional_array(request, "components")?
        .map(|items| {
            items
                .iter()
                .map(component_from_json)
                .collect::<Result<Vec<_>>>()
        })
        .transpose()
        .map(|components| components.unwrap_or_default())
}

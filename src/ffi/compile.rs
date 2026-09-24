//! `colander_compile`.

use std::ffi::c_char;

use crate::codec::json::JsonCodec;
use crate::compile;
use crate::error::{ColanderError, Result};
use crate::json::{self, Json, JsonMap};

use super::envelope::{dispatch, optional_array, optional_string, optional_text, require_string};

/// `colander_compile`.
///
/// # Safety
/// `request` must be a valid NUL-terminated UTF-8 C string (or null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colander_compile(request: *const c_char) -> *mut c_char {
    unsafe {
        dispatch(&JsonCodec, request, |request| {
            let form = require_string(request, "formSchemaJson")?;
            let ui = optional_string(request, "uiSchemaJson")?;
            let rules_json = optional_string(request, "rulesSchemaJson")?;
            let components = optional_array(request, "components")?
                .map(|items| {
                    items
                        .iter()
                        .map(component_from_json)
                        .collect::<Result<Vec<_>>>()
                })
                .transpose()?
                .unwrap_or_default();

            let result =
                compile::compile(&form, ui.as_deref(), rules_json.as_deref(), &components)?;

            let mut out = JsonMap::new();
            out.insert(
                "formSchemaJson".to_string(),
                Json::String(result.form_schema_json),
            );
            out.insert(
                "uiSchemaJson".to_string(),
                optional_text(result.ui_schema_json),
            );
            out.insert(
                "rulesSchemaJson".to_string(),
                optional_text(result.rules_schema_json),
            );
            out.insert(
                "dependencyMetadataJson".to_string(),
                Json::String(result.dependency_metadata_json),
            );
            out.insert("contentHash".to_string(), Json::String(result.content_hash));
            Ok(Json::Object(out))
        })
    }
}

pub(super) fn component_from_json(value: &Json) -> Result<compile::ComponentVersionData> {
    let object = value
        .as_object()
        .ok_or_else(|| ColanderError::new("components[] entries must be objects."))?;
    Ok(compile::ComponentVersionData {
        code: json::get_str(object, "code")
            .ok_or_else(|| ColanderError::new("components[].code is required."))?
            .to_string(),
        version: json::get_str(object, "version")
            .ok_or_else(|| ColanderError::new("components[].version is required."))?
            .to_string(),
        form_schema_json: json::get_str(object, "formSchemaJson")
            .ok_or_else(|| ColanderError::new("components[].formSchemaJson is required."))?
            .to_string(),
        ui_schema_json: optional_string(object, "uiSchemaJson")?,
        content_hash: optional_string(object, "contentHash")?,
    })
}

//! `colander_validate_schema`.

use std::ffi::c_char;

use crate::codec::json::JsonCodec;
use crate::error::{ColanderError, Result};
use crate::json::{self, Json, JsonMap};
use crate::schema;

use super::envelope::{dispatch, optional_string, require_string};

/// `colander_validate_schema`: Draft 2020-12 structural validation.
///
/// colander ships no schemas, so `schemas` carries the JSON Schema text for
/// whichever document kinds the call needs. `kind: "instance"` is the
/// domain-free entry point: it validates `instanceJson` against `schemaJson`.
///
/// # Safety
/// `request` must be a valid NUL-terminated UTF-8 C string (or null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colander_validate_schema(request: *const c_char) -> *mut c_char {
    unsafe {
        dispatch(&JsonCodec, request, |request| {
            let kind = json::get_str(request, "kind").unwrap_or("form");
            match kind {
                "component" => {
                    let form = require_string(request, "formSchemaJson")?;
                    let ui = optional_string(request, "uiSchemaJson");
                    let schemas =
                        read_schemas(request, &[("formSchema", true), ("uiSchema", ui.is_some())])?;
                    schema::validate_component_draft(&form, ui.as_deref(), &schemas)?;
                }
                "form" => {
                    let form = require_string(request, "formSchemaJson")?;
                    let ui = optional_string(request, "uiSchemaJson");
                    let rules_json = optional_string(request, "rulesSchemaJson");
                    let schemas = read_schemas(
                        request,
                        &[
                            ("formSchema", true),
                            ("uiSchema", ui.is_some()),
                            ("rulesSchema", rules_json.is_some()),
                        ],
                    )?;
                    schema::validate_form_draft(
                        &form,
                        ui.as_deref(),
                        rules_json.as_deref(),
                        &schemas,
                    )?;
                }
                "workflow" => {
                    let workflow = require_string(request, "workflowSchemaJson")?;
                    let schemas = read_schemas(request, &[("workflowSchema", true)])?;
                    schema::validate_workflow(
                        &workflow,
                        json::get_bool(request, "published").unwrap_or(false),
                        &schemas,
                    )?;
                }
                "instance" => {
                    let schema_json = require_string(request, "schemaJson")?;
                    let instance_json = require_string(request, "instanceJson")?;
                    let label = json::get_str(request, "label").unwrap_or("instance");
                    schema::validate_text(&schema_json, &instance_json, label)?;
                }
                other => {
                    return Err(ColanderError::new(format!(
                        "Unknown schema kind '{other}' (expected 'form', 'component', \
                     'workflow' or 'instance')."
                    )));
                }
            }
            let mut out = JsonMap::new();
            out.insert("valid".to_string(), Json::Bool(true));
            Ok(Json::Object(out))
        })
    }
}

/// Read the `schemas` object, requiring the entries this call actually uses.
pub(super) fn read_schemas<'a>(
    request: &'a JsonMap,
    needed: &[(&str, bool)],
) -> Result<schema::PublishedSchemas<'a>> {
    let schemas = json::get(request, "schemas")
        .and_then(Json::as_object)
        .ok_or_else(|| {
            ColanderError::new(
                "schemas is required: pass the JSON Schema text for each document kind, \
                 e.g. {\"formSchema\":\"…\",\"uiSchema\":\"…\"}.",
            )
        })?;
    for (key, required) in needed {
        if *required && json::get_str(schemas, key).is_none() {
            return Err(ColanderError::new(format!(
                "schemas.{key} is required to validate this request."
            )));
        }
    }
    Ok(schema::PublishedSchemas {
        form_schema: json::get_str(schemas, "formSchema").unwrap_or(""),
        ui_schema: json::get_str(schemas, "uiSchema").unwrap_or(""),
        rules_schema: json::get_str(schemas, "rulesSchema").unwrap_or(""),
        workflow_schema: json::get_str(schemas, "workflowSchema").unwrap_or(""),
    })
}

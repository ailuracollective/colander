//! JSON Schema (Draft 2020-12) validation.
//!
//! colander implements the subset of JSON Schema keywords the form schemas
//! actually use. Errors are shaped `Invalid {label}: ...` with `"; "`-joined
//! `keyword: message` entries; the wording of each message is colander's own.
//!
//! colander does not ship any schema of its own. Documents belong to the
//! caller's domain, so the caller passes the JSON Schema text next to each
//! document. `check` and `validate_json` are the generic entry points; the
//! `validate_*_draft` functions are the compositions of them that the triple
//! input expects.
//!
//! Schemas are supplied as text and parsed per call. A caller that validates
//! in a loop should parse once and use [`validate_json`] with the `&Json`.
//!
//! Out of scope here: workflow semantic validation, which is workflow domain
//! logic and is not covered by this module.

use crate::error::{ColanderError, Result};
use crate::json::{self, Json};
use crate::rules;

mod check;
mod keywords;
mod model;

pub use check::{check, value_equal};
pub use model::{PublishedSchemas, SchemaError};

pub fn validate_component_draft(
    form_schema_json: &str,
    ui_schema_json: Option<&str>,
    schemas: &PublishedSchemas<'_>,
) -> Result<()> {
    validate_draft(form_schema_json, ui_schema_json, None, schemas)
}

/// Validate a form draft against the published schemas.
pub fn validate_form_draft(
    form_schema_json: &str,
    ui_schema_json: Option<&str>,
    rules_schema_json: Option<&str>,
    schemas: &PublishedSchemas<'_>,
) -> Result<()> {
    validate_draft(form_schema_json, ui_schema_json, rules_schema_json, schemas)
}

/// Validate a workflow document against the published workflow schema.
///
/// `published` is accepted but ignored: only JSON Schema validation is
/// performed here, not workflow semantic validation.
pub fn validate_workflow(
    workflow_schema_json: &str,
    published: bool,
    schemas: &PublishedSchemas<'_>,
) -> Result<()> {
    let _ = published;
    validate_text(
        schemas.workflow_schema,
        workflow_schema_json,
        "workflow schema",
    )
}

fn validate_draft(
    form_schema_json: &str,
    ui_schema_json: Option<&str>,
    rules_schema_json: Option<&str>,
    schemas: &PublishedSchemas<'_>,
) -> Result<()> {
    validate_text(schemas.form_schema, form_schema_json, "form schema")?;
    if let Some(ui) = ui_schema_json {
        validate_text(schemas.ui_schema, ui, "UI schema")?;
    }
    if let Some(rules_json) = rules_schema_json {
        validate_text(schemas.rules_schema, rules_json, "rules schema")?;
        let form_root = json::parse_object(form_schema_json, "form schema")?;
        let rules_root = json::parse_object(rules_json, "rules schema")?;
        rules::validate_dependencies(&form_root, &rules_root)?;
    }
    Ok(())
}

/// Validate `json_text` against a schema supplied as text.
pub fn validate_text(schema_json: &str, json_text: &str, label: &str) -> Result<()> {
    let schema = json::parse(schema_json)
        .map_err(|e| ColanderError::new(format!("Invalid {label} definition: {e}")))?;
    validate_json(&schema, json_text, label)
}

/// Validate `json_text` against an already-parsed `schema`.
pub fn validate_json(schema: &Json, json_text: &str, label: &str) -> Result<()> {
    let instance =
        json::parse(json_text).map_err(|e| ColanderError::new(format!("Invalid {label}: {e}")))?;
    let errors = check(schema, &instance, schema);
    if errors.is_empty() {
        return Ok(());
    }
    Err(ColanderError::new(format!(
        "Invalid {label}: {}",
        format_errors(&errors)
    )))
}

fn format_errors(errors: &[SchemaError]) -> String {
    let messages: Vec<String> = errors
        .iter()
        .take(5)
        .map(|error| format!("{}: {}", error.keyword, error.message))
        .collect();
    if messages.is_empty() {
        "schema validation failed".to_string()
    } else {
        messages.join("; ")
    }
}

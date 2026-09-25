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

use check::Stop;

mod check;
mod classify;
mod format;
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
pub fn validate_workflow(workflow_schema_json: &str, schemas: &PublishedSchemas<'_>) -> Result<()> {
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

/// At most this many assertion failures are rendered (SPEC S-7).
const MAX_REPORTED_ERRORS: usize = 5;

/// Evaluation errors are capped while they are collected, separately from the
/// rendering cap. A combinator such as `allOf` can produce millions of failures
/// before `format_errors` gets a chance to render only five of them.
pub(super) const MAX_COLLECTED_ERRORS: usize = 1000;

/// Keep the limit text independent of the size-derived budget. The budget is
/// deterministic, but omitting its value makes this control-plane error a stable
/// byte sequence on every runtime and leaves sizing changes out of the contract.
pub(super) const SCHEMA_EVALUATION_LIMIT_MESSAGE: &str =
    "SCHEMA_EVALUATION_LIMIT: schema evaluation exceeded the step budget";

/// The nesting bound reports its own cause: a caller debugging a deep `$ref`
/// chain needs to know the recursion stopped, not that the work ran out.
pub(super) const SCHEMA_DEPTH_LIMIT_MESSAGE: &str =
    "SCHEMA_DEPTH_LIMIT: schema evaluation exceeded the nesting depth";

/// The deepest chain of nested schema nodes one evaluation may enter, through
/// keywords or through `$ref` targets.
///
/// A step budget alone cannot bound this: recursion depth is at most the step
/// count, so a budget sized for a big instance also admits a `$ref` chain deep
/// enough to exhaust the native stack, and a stack overflow aborts the process
/// — the very failure the budget exists to prevent. The parser already caps
/// JSON nesting at 64, so a schema only gets past that through `$ref` chains;
/// 512 leaves an 8x margin over the parser's own limit and sits an order of
/// magnitude below the measured overflow cliff (between 5 000 and 10 000 frames
/// on the default 8 MiB stack). Classification and instance evaluation share
/// it, so one number describes how deep a schema may be.
pub(super) const MAX_SCHEMA_DEPTH: usize = 512;

/// The evaluation-only error sink. Its `push` method is the collection boundary;
/// callers still receive the ordinary `Vec<SchemaError>` from public `check`.
#[derive(Default)]
pub(super) struct ErrorSink {
    errors: Vec<SchemaError>,
}

impl ErrorSink {
    pub(super) fn push(&mut self, error: SchemaError) {
        if self.errors.len() < MAX_COLLECTED_ERRORS {
            self.errors.push(error);
        }
    }

    /// Keep a control failure visible even when ordinary failures have already
    /// filled the collection cap. The final slot is reserved for it, preserving
    /// both the hard bound and the fail-closed result.
    pub(super) fn push_stop(&mut self, stop: Stop) {
        let error = SchemaError {
            keyword: "schema".to_string(),
            message: stop.message().to_string(),
        };
        self.errors.truncate(MAX_COLLECTED_ERRORS - 1);
        self.errors.push(error);
    }

    pub(super) fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub(super) fn into_vec(self) -> Vec<SchemaError> {
        self.errors
    }
}

fn format_errors(errors: &[SchemaError]) -> String {
    // The control failure must stay visible even when ordinary errors filled
    // the collection cap and the limit occupies the last retained slot.
    let is_limit = |error: &SchemaError| error.message == SCHEMA_EVALUATION_LIMIT_MESSAGE;
    let limit = errors.iter().find(|error| is_limit(error));
    let mut messages: Vec<String> = Vec::with_capacity(MAX_REPORTED_ERRORS);
    if let Some(error) = limit {
        messages.push(format!("{}: {}", error.keyword, error.message));
    }
    for error in errors.iter().filter(|error| !is_limit(error)) {
        if messages.len() == MAX_REPORTED_ERRORS {
            break;
        }
        messages.push(format!("{}: {}", error.keyword, error.message));
    }
    if messages.is_empty() {
        return "schema validation failed".to_string();
    }
    let mut rendered = messages.join("; ");
    if errors.len() > MAX_REPORTED_ERRORS {
        rendered.push_str(&format!(
            " (truncated: {MAX_REPORTED_ERRORS} of {} errors shown)",
            errors.len()
        ));
    }
    rendered
}

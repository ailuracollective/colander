//! Compiles the form/ui/rules triple, expanding `component-ref` fields.
//!
//! Components are resolved from the batch of [`ComponentVersionData`] the
//! caller passes in. There is no repository, no callback and no I/O in the
//! core; the caller decides what "published" means.

use crate::error::Result;
use crate::hash;
use crate::json::{self, Json, JsonMap};
use crate::keys::schema_json_keys;

mod context;
mod fields;
mod schemas;
mod util;

use context::CompilationContext;
use fields::compile_field_array;
use schemas::{compile_rules_schema, compile_ui_schema};
use util::{clone_or_null, present, require_array};

/// One resolved component version, as handed in by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentVersionData {
    pub code: String,
    pub version: String,
    pub form_schema_json: String,
    pub ui_schema_json: Option<String>,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormCompilationResult {
    pub form_schema_json: String,
    pub ui_schema_json: Option<String>,
    pub rules_schema_json: Option<String>,
    pub dependency_metadata_json: String,
    pub content_hash: String,
}

/// Compile the triple and expand every `component-ref` against `components`.
pub fn compile(
    form_schema_json: &str,
    ui_schema_json: Option<&str>,
    rules_schema_json: Option<&str>,
    components: &[ComponentVersionData],
) -> Result<FormCompilationResult> {
    let form_root = json::parse_object(form_schema_json, "form schema")?;
    let ui_root = match ui_schema_json {
        Some(text) => Some(json::parse_object(text, "UI schema")?),
        None => None,
    };
    let rules_root = match rules_schema_json {
        Some(text) => Some(json::parse_object(text, "rules schema")?),
        None => None,
    };

    let mut context = CompilationContext::new(components);

    let fields = require_array(json::get(&form_root, schema_json_keys::FIELDS), "/fields")?;
    let compiled_fields = compile_field_array(fields, "/fields", &mut context)?;

    let mut compiled_form = JsonMap::new();
    compiled_form.insert(
        schema_json_keys::SCHEMA_VERSION.to_string(),
        clone_or_null(&form_root, schema_json_keys::SCHEMA_VERSION),
    );
    if let Some(schema_uri) = present(&form_root, schema_json_keys::SCHEMA) {
        compiled_form.insert(schema_json_keys::SCHEMA.to_string(), schema_uri.clone());
    }
    compiled_form.insert(
        schema_json_keys::FIELDS.to_string(),
        Json::Array(compiled_fields),
    );

    // R-5: a duplicated field code is rejected on the effective (compiled)
    // documents, whether or not a rules document is present.
    crate::index::ensure_unique_codes(&compiled_form)?;

    let compiled_ui = match &ui_root {
        Some(root) => Some(compile_ui_schema(root, &context)?),
        None => None,
    };
    let compiled_rules = rules_root.as_ref().map(compile_rules_schema);
    let compiled_form_json = json::canonical(&Json::Object(compiled_form));
    let compiled_ui_json = compiled_ui.map(|value| json::canonical(&value));
    let compiled_rules_json = compiled_rules.map(|value| json::canonical(&value));

    let dependency_metadata_json = context
        .build_dependency_metadata_json(&compiled_form_json, compiled_rules_json.as_deref())?;
    let content_hash = hash::content_hash(
        &compiled_form_json,
        compiled_ui_json.as_deref(),
        compiled_rules_json.as_deref(),
    )?;

    Ok(FormCompilationResult {
        form_schema_json: compiled_form_json,
        ui_schema_json: compiled_ui_json,
        rules_schema_json: compiled_rules_json,
        dependency_metadata_json,
        content_hash,
    })
}

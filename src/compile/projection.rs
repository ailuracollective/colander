//! Deterministic projections shared by the public compile façade and pin checks.

use crate::error::Result;
use crate::hash;
use crate::json::{self, Json, JsonMap};
use crate::keys::schema_json_keys;

use super::ComponentVersionData;
use super::context::CompilationContext;
use super::fields::compile_field_array;
use super::schemas::{compile_rules_schema, compile_ui_schema};
use super::util::{clone_or_null, present, require_array};

/// The lossless form/UI/rules inputs kept together for one compile operation.
///
/// Keeping the three roots together makes the semantic boundary explicit
/// without introducing a registry or changing any of the public documents.
pub(super) struct CompileSemantics<'a> {
    pub(super) form_root: &'a JsonMap,
    pub(super) ui_root: Option<&'a JsonMap>,
    pub(super) rules_root: Option<&'a JsonMap>,
}

impl<'a> CompileSemantics<'a> {
    pub(super) fn project_form(&self, context: &mut CompilationContext<'_>) -> Result<Json> {
        project_form(self.form_root, "/fields", context)
    }

    pub(super) fn project_ui_rules(
        &self,
        context: &mut CompilationContext<'_>,
    ) -> Result<(Option<Json>, Option<Json>)> {
        let ui = match self.ui_root {
            Some(root) => Some(compile_ui_schema(root, context)?),
            None => None,
        };
        let rules = self.rules_root.map(compile_rules_schema);
        Ok((ui, rules))
    }
}

/// The three documents produced by one compile operation before serialization.
pub(super) struct CompiledDocuments {
    pub(super) form: Json,
    pub(super) ui: Option<Json>,
    pub(super) rules: Option<Json>,
}

impl CompiledDocuments {
    pub(super) fn form_root(&self) -> &JsonMap {
        self.form
            .as_object()
            .expect("compiled form projection is an object")
    }

    pub(super) fn rules_root(&self) -> Option<&JsonMap> {
        self.rules.as_ref().and_then(Json::as_object)
    }
}

/// Projects one form document, expanding component references in place.
pub(super) fn project_form(
    form_root: &JsonMap,
    fields_path: &str,
    context: &mut CompilationContext<'_>,
) -> Result<Json> {
    let fields = require_array(json::get(form_root, schema_json_keys::FIELDS), fields_path)?;
    let compiled_fields = compile_field_array(fields, fields_path, context)?;

    let mut compiled_form = JsonMap::new();
    compiled_form.insert(
        schema_json_keys::SCHEMA_VERSION.to_string(),
        clone_or_null(form_root, schema_json_keys::SCHEMA_VERSION),
    );
    if let Some(schema_uri) = present(form_root, schema_json_keys::SCHEMA) {
        compiled_form.insert(schema_json_keys::SCHEMA.to_string(), schema_uri.clone());
    }
    compiled_form.insert(
        schema_json_keys::FIELDS.to_string(),
        Json::Array(compiled_fields),
    );
    Ok(Json::Object(compiled_form))
}

/// Computes a component pin from the same projections used by the façade.
pub(super) fn component_hash(
    component: &ComponentVersionData,
    components: &[ComponentVersionData],
    ui_root: Option<&JsonMap>,
) -> Result<String> {
    let form_root = json::parse_object(
        &component.form_schema_json,
        &format!("component '{}' form schema", component.code),
    )?;

    let mut context = CompilationContext::new(components)?;
    context.verify_hashes = false;

    let fields_path = format!("/components/{}/fields", component.code);
    let compiled_form = project_form(&form_root, &fields_path, &mut context)?;
    let compiled_form_json = json::canonical(&compiled_form);

    let compiled_ui = match ui_root {
        Some(root) => Some(compile_ui_schema(root, &context)?),
        None => None,
    };
    let compiled_ui_json = compiled_ui.as_ref().map(json::canonical);
    let digest = hash::content_hash_from_canonical_documents(
        &compiled_form_json,
        compiled_ui_json.as_deref(),
        None,
    );
    Ok(digest)
}

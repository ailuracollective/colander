//! Compilation of the UI and rules documents.

use crate::error::{ColanderError, Result};
use crate::json::{self, Json, JsonMap};
use crate::keys::{field_type_names, schema_json_keys};

use super::context::{CompilationContext, ResolvedComponentDependency};
use super::util::{clone_or_null, copy_if_present, present, require_string};

pub(super) fn compile_rules_schema(rules_root: &JsonMap) -> Json {
    let mut compiled_fields = JsonMap::new();
    if let Some(source_fields) = json::get_object(rules_root, schema_json_keys::FIELDS) {
        for key in json::sorted_keys(source_fields) {
            compiled_fields.insert(key.to_string(), source_fields[key].clone());
        }
    }

    let compiled_validations = json::get_array(rules_root, schema_json_keys::VALIDATIONS)
        .map(|validations| Json::Array(validations.to_vec()));

    let mut compiled_rules = JsonMap::new();
    compiled_rules.insert(
        schema_json_keys::SCHEMA_VERSION.to_string(),
        clone_or_null(rules_root, schema_json_keys::SCHEMA_VERSION),
    );
    compiled_rules.insert(
        schema_json_keys::FORM_SCHEMA_VERSION.to_string(),
        clone_or_null(rules_root, schema_json_keys::FORM_SCHEMA_VERSION),
    );
    compiled_rules.insert(
        schema_json_keys::FIELDS.to_string(),
        Json::Object(compiled_fields),
    );
    if let Some(schema_uri) = present(rules_root, schema_json_keys::SCHEMA) {
        compiled_rules.insert(schema_json_keys::SCHEMA.to_string(), schema_uri.clone());
    }
    if let Some(validations) = compiled_validations {
        compiled_rules.insert(schema_json_keys::VALIDATIONS.to_string(), validations);
    }

    Json::Object(compiled_rules)
}

pub(super) fn compile_ui_schema(ui_root: &JsonMap, context: &CompilationContext) -> Result<Json> {
    // Top-level UI keys are closed: a typo that would otherwise be dropped
    // must fail, or "unknown layout" silently renders as an empty form.
    for key in ui_root.keys() {
        if !matches!(
            key.as_str(),
            schema_json_keys::SCHEMA_VERSION
                | schema_json_keys::FORM_SCHEMA_VERSION
                | schema_json_keys::FIELDS
                | schema_json_keys::LAYOUT
                | schema_json_keys::SCHEMA
        ) {
            return Err(ColanderError::new(format!(
                "UI_UNKNOWN_KEY: UI schema carries unknown top-level key '{key}' (expected 'schemaVersion', 'formSchemaVersion', 'fields', 'layout' or '$schema')."
            )));
        }
    }
    let mut compiled_fields = JsonMap::new();
    if let Some(source_fields) = json::get_object(ui_root, schema_json_keys::FIELDS) {
        for key in json::sorted_keys(source_fields) {
            compiled_fields.insert(key.to_string(), source_fields[key].clone());
        }
    }

    let mut dependencies: Vec<&ResolvedComponentDependency> =
        context.dependencies.values().collect();
    dependencies.sort_by(|left, right| left.code.cmp(&right.code));
    for dependency in dependencies {
        let Some(ui_fields) = &dependency.ui_fields else {
            continue;
        };
        for key in json::sorted_keys(ui_fields) {
            if !ui_fields[key].is_null() {
                compiled_fields.insert(key.to_string(), ui_fields[key].clone());
            }
        }
    }

    let mut compiled_ui = JsonMap::new();
    compiled_ui.insert(
        schema_json_keys::SCHEMA_VERSION.to_string(),
        clone_or_null(ui_root, schema_json_keys::SCHEMA_VERSION),
    );
    compiled_ui.insert(
        schema_json_keys::FORM_SCHEMA_VERSION.to_string(),
        clone_or_null(ui_root, schema_json_keys::FORM_SCHEMA_VERSION),
    );
    compiled_ui.insert(
        schema_json_keys::FIELDS.to_string(),
        Json::Object(compiled_fields),
    );
    if let Some(schema_uri) = present(ui_root, schema_json_keys::SCHEMA) {
        compiled_ui.insert(schema_json_keys::SCHEMA.to_string(), schema_uri.clone());
    }
    if let Some(layout) = json::get_array(ui_root, schema_json_keys::LAYOUT) {
        compiled_ui.insert(
            schema_json_keys::LAYOUT.to_string(),
            Json::Array(compile_layout_array(layout, context)?),
        );
    }

    Ok(Json::Object(compiled_ui))
}

pub(super) fn compile_layout_array(
    layout: &[Json],
    context: &CompilationContext,
) -> Result<Vec<Json>> {
    let mut compiled = Vec::with_capacity(layout.len());
    for node in layout {
        let object = node
            .as_object()
            .ok_or_else(|| ColanderError::new("Expected layout node object."))?;
        compiled.push(compile_layout_node(object, context)?);
    }
    Ok(compiled)
}

pub(super) fn compile_layout_node(node: &JsonMap, context: &CompilationContext) -> Result<Json> {
    let node_type = require_string(json::get(node, schema_json_keys::TYPE), "layout type")?;

    if node_type == "field"
        && let Some(field_id) = json::get_str(node, schema_json_keys::FIELD_ID)
        && let Some(expanded_children) = context.expanded_reference_layouts.get(field_id)
    {
        let mut compiled = JsonMap::new();
        compiled.insert("type".to_string(), Json::string(field_type_names::GROUP));
        compiled.insert(
            schema_json_keys::FIELD_ID.to_string(),
            clone_or_null(node, schema_json_keys::FIELD_ID),
        );
        compiled.insert(
            schema_json_keys::CHILDREN.to_string(),
            expanded_children.clone(),
        );
        copy_if_present(node, &mut compiled, "id");
        copy_if_present(node, &mut compiled, "title");
        copy_if_present(node, &mut compiled, "description");
        return Ok(Json::Object(compiled));
    }

    let mut result = JsonMap::new();
    for key in node.keys() {
        if !matches!(
            key.as_str(),
            "type"
                | "id"
                | "title"
                | "description"
                | schema_json_keys::FIELD_ID
                | "addButtonLabel"
                | "removeButtonLabel"
                | schema_json_keys::CHILDREN
                | "itemTemplate"
        ) {
            return Err(ColanderError::new(format!(
                "UI_UNKNOWN_KEY: layout node carries unknown key '{key}'."
            )));
        }
    }
    result.insert("type".to_string(), Json::string(node_type));
    for key in [
        "id",
        "title",
        "description",
        schema_json_keys::FIELD_ID,
        "addButtonLabel",
        "removeButtonLabel",
    ] {
        copy_if_present(node, &mut result, key);
    }
    if let Some(children) = json::get_array(node, schema_json_keys::CHILDREN) {
        result.insert(
            schema_json_keys::CHILDREN.to_string(),
            Json::Array(compile_layout_array(children, context)?),
        );
    }
    if let Some(item_template) = json::get_array(node, "itemTemplate") {
        result.insert(
            "itemTemplate".to_string(),
            Json::Array(compile_layout_array(item_template, context)?),
        );
    }

    Ok(Json::Object(result))
}

pub(super) fn build_default_layout(compiled_items: &[Json]) -> Result<Vec<Json>> {
    let mut layout = Vec::with_capacity(compiled_items.len());
    for item in compiled_items {
        let item_id = match item.as_object() {
            Some(object) => {
                require_string(json::get(object, schema_json_keys::ID), "compiled field id")?
            }
            None => return Err(ColanderError::new("Expected compiled field object.")),
        };
        let mut node = JsonMap::new();
        node.insert("type".to_string(), Json::string("field"));
        node.insert(
            schema_json_keys::FIELD_ID.to_string(),
            Json::String(item_id.to_string()),
        );
        layout.push(Json::Object(node));
    }
    Ok(layout)
}

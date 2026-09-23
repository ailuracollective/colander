//! Field arrays and `component-ref` expansion.

use crate::error::{ColanderError, Result};
use crate::json::{self, Json, JsonMap};
use crate::keys::{field_type_names, schema_json_keys};
use crate::semver;

use super::context::CompilationContext;
use super::schemas::{build_default_layout, compile_layout_array};
use super::util::{
    clone_field_shell, clone_or_null, copy_if_present, require_array, require_string,
};

pub(super) fn compile_field_array(
    fields: &[Json],
    path: &str,
    context: &mut CompilationContext<'_>,
) -> Result<Vec<Json>> {
    let mut compiled = Vec::with_capacity(fields.len());
    for (position, value) in fields.iter().enumerate() {
        let field = value.as_object().ok_or_else(|| {
            ColanderError::new(format!("Expected field object at {path}/{position}."))
        })?;
        compiled.push(compile_field(
            field,
            &format!("{path}/{position}"),
            context,
        )?);
    }
    Ok(compiled)
}

pub(super) fn compile_field(
    field: &JsonMap,
    path: &str,
    context: &mut CompilationContext<'_>,
) -> Result<Json> {
    let field_type = require_string(
        json::get(field, schema_json_keys::TYPE),
        &format!("{path}/type"),
    )?;
    if field_type == field_type_names::COMPONENT_REF {
        return expand_component_reference(field, path, context);
    }

    let mut compiled = clone_field_shell(field);
    if let Some(items) = json::get_array(field, schema_json_keys::ITEMS) {
        compiled.insert(
            schema_json_keys::ITEMS.to_string(),
            Json::Array(compile_field_array(
                items,
                &format!("{path}/items"),
                context,
            )?),
        );
    }
    Ok(Json::Object(compiled))
}

pub(super) fn expand_component_reference(
    field: &JsonMap,
    path: &str,
    context: &mut CompilationContext<'_>,
) -> Result<Json> {
    let component_code = require_string(
        json::get(field, "componentCode"),
        &format!("{path}/componentCode"),
    )?
    .to_string();
    let component_version = json::get_str(field, "componentVersion").unwrap_or("");
    if component_version.trim().is_empty() {
        return Err(ColanderError::new(format!(
            "COMPONENT_VERSION_REQUIRED: component-ref at {path} must include componentVersion before publication."
        )));
    }

    semver::ensure_valid(component_version)?;

    if context
        .resolution_stack
        .iter()
        .any(|item| item == &component_code)
    {
        // The stack is walked top-first, so the chain reads from the
        // innermost reference outward.
        let chain = context
            .resolution_stack
            .iter()
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(ColanderError::new(format!(
            "CIRCULAR_COMPONENT_REFERENCE: component '{component_code}' references itself through {chain} -> {component_code}."
        )));
    }

    let dependency = context.resolve(&component_code, component_version, path)?;
    let dependency_form = dependency.form_schema_json.clone();
    let dependency_layout = dependency.layout_children.clone();

    let component_form = json::parse_object(
        &dependency_form,
        &format!("component '{component_code}' form schema"),
    )?;
    let component_fields = require_array(
        json::get(&component_form, schema_json_keys::FIELDS),
        &format!("/components/{component_code}/fields"),
    )?;

    context.resolution_stack.push(component_code.clone());
    let compiled_items = compile_field_array(
        component_fields,
        &format!("/components/{component_code}/fields"),
        context,
    );
    context.resolution_stack.pop();
    let compiled_items = compiled_items?;

    let field_id = require_string(
        json::get(field, schema_json_keys::ID),
        &format!("{path}/id"),
    )?;
    let expanded_layout = match dependency_layout {
        Some(children) => Json::Array(compile_layout_array(&children, context)?),
        None => Json::Array(build_default_layout(&compiled_items)?),
    };
    context
        .expanded_reference_layouts
        .insert(field_id.to_string(), expanded_layout);

    let mut compiled = JsonMap::new();
    compiled.insert(
        schema_json_keys::ID.to_string(),
        clone_or_null(field, schema_json_keys::ID),
    );
    compiled.insert(
        schema_json_keys::CODE.to_string(),
        clone_or_null(field, schema_json_keys::CODE),
    );
    compiled.insert("type".to_string(), Json::string(field_type_names::GROUP));
    compiled.insert(
        schema_json_keys::ITEMS.to_string(),
        Json::Array(compiled_items),
    );
    for key in ["required", "readOnly", "description"] {
        copy_if_present(field, &mut compiled, key);
    }

    Ok(Json::Object(compiled))
}

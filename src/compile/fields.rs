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

/// Maximum nesting of component expansion. Mirrors the JSON parser's
/// `MAX_DEPTH`: deep but finite, and a hard error instead of a stack
/// overflow on adversarial input.
const MAX_COMPONENT_DEPTH: usize = 64;

pub(super) fn compile_field_array(
    fields: &[Json],
    path: &str,
    context: &mut CompilationContext<'_>,
) -> Result<Vec<Json>> {
    let mut compiled = Vec::with_capacity(fields.len());
    for (position, value) in fields.iter().enumerate() {
        let field = value.as_object().ok_or_else(|| {
            ColanderError::new(format!(
                "FIELD_NOT_OBJECT: {path}/{position} must be an object."
            ))
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

    // The stack keys on `(code, version)`: the same code at another version
    // is another triple, not a cycle back to this one.
    let reference_key = format!("{component_code}@{component_version}");
    if context
        .resolution_stack
        .iter()
        .any(|item| item == &reference_key)
    {
        // The stack is walked top-first, so the chain reads from the
        // innermost reference outward.
        let chain = context
            .resolution_stack
            .iter()
            .rev()
            .filter_map(|item| item.split('@').next())
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(ColanderError::new(format!(
            "CIRCULAR_COMPONENT_REFERENCE: component '{component_code}' references itself through {chain} -> {component_code}."
        )));
    }

    // Component nesting is bounded like JSON parsing (`MAX_DEPTH` in the
    // parser): an adversarial batch of deeply nested components would
    // otherwise recurse one stack frame per level. The bounded depth alone
    // is not a complexity bound — the expansion budget in
    // `CompilationContext` also caps materialised fields and output bytes.
    if context.resolution_stack.len() >= MAX_COMPONENT_DEPTH {
        return Err(ColanderError::new(format!(
            "COMPONENT_DEPTH_EXCEEDED: component-ref at {path} nests components deeper than {MAX_COMPONENT_DEPTH} levels."
        )));
    }

    let dependency = context.resolve(&component_code, component_version, path)?;
    let dependency_layout = dependency.layout_children.clone();

    // P-8: expand the component once per `(code, version)` and clone the
    // compiled field array for every later reference. The clone is charged
    // to the same budget, so materialising N copies of an X-sized component
    // still costs N·X — it just no longer re-resolves, re-verifies and
    // re-parses the sub-tree per site.
    let reference_key = format!("{component_code}@{component_version}");
    let compiled_items = match context.compiled_components.get(&reference_key) {
        Some(cached) => {
            context.budget.charge_fields(count_fields(cached))?;
            cached.clone()
        }
        None => {
            // P-11: the source form is needed only for this first expansion;
            // take it out of the dependency so a large batch does not keep
            // every source document alive until the call ends.
            let source = context
                .dependencies
                .get_mut(&reference_key)
                .and_then(|dependency| dependency.form_schema_json.take())
                .ok_or_else(|| {
                    ColanderError::new(format!(
                        "COMPONENT_SOURCE_UNAVAILABLE: component '{component_code}' version '{component_version}' was already expanded but its source form is gone."
                    ))
                })?;
            let component_form = json::parse_object(
                &source,
                &format!("component '{component_code}' form schema"),
            )?;
            drop(source);
            let component_fields = require_array(
                json::get(&component_form, schema_json_keys::FIELDS),
                &format!("/components/{component_code}/fields"),
            )?
            .clone();
            drop(component_form);

            context.resolution_stack.push(reference_key.clone());
            let compiled = compile_field_array(
                &component_fields,
                &format!("/components/{component_code}/fields"),
                context,
            );
            context.resolution_stack.pop();
            let compiled = compiled?;
            context.budget.charge_fields(count_fields(&compiled))?;
            context
                .compiled_components
                .insert(reference_key, compiled.clone());
            compiled
        }
    };

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

    let group = Json::Object(compiled);
    context.budget.charge_bytes(json::ordered(&group).len())?;
    Ok(group)
}

/// Field nodes in a compiled array, counting nested `items`.
fn count_fields(fields: &[Json]) -> usize {
    fields
        .iter()
        .map(|field| {
            1 + field
                .as_object()
                .and_then(|object| object.get(schema_json_keys::ITEMS))
                .and_then(Json::as_array)
                .map(|items| count_fields(items))
                .unwrap_or(0)
        })
        .sum()
}

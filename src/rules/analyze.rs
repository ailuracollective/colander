//! Dependency metadata and structural rule validation.

use std::collections::{HashSet, VecDeque};

use indexmap::IndexMap;

use crate::error::{ColanderError, Result};
use crate::index::{self, FieldInfo};
use crate::json::{self, Json, JsonMap};
use crate::keys::schema_json_keys;

use super::model::RuleDependencyMetadata;
use super::refs::{collect_references, validate_aggregate_targets, validate_row_scope};
use super::rows::RowSet;
use super::shape::{validate_aggregate_arguments, validate_expression_shape};

/// Builds dependency metadata: the calculated field ids in document order and
/// their topological evaluation order.
pub fn analyze(form_root: &JsonMap, rules_root: &JsonMap) -> Result<RuleDependencyMetadata> {
    let fields_by_id = index::build_by_id(form_root)?;

    let Some(field_rules) = json::get_object(rules_root, schema_json_keys::FIELDS) else {
        return Ok(RuleDependencyMetadata::default());
    };

    // One code → id map for the whole analysis: resolving a reference used to
    // scan every field, which made a dense dependency graph quadratic
    // (SPEC R-14).
    let id_by_code: IndexMap<&str, &str> = fields_by_id
        .values()
        .map(|field| (field.code.as_str(), field.id.as_str()))
        .collect();

    let mut calculated_field_ids: Vec<String> = Vec::new();
    let mut dependencies: IndexMap<String, Vec<String>> = IndexMap::new();

    for (field_id, rules_node) in field_rules {
        let Some(rules) = rules_node.as_object() else {
            continue;
        };
        let Some(calculate) = json::get(rules, schema_json_keys::CALCULATE) else {
            continue;
        };
        if calculate.is_null() || !fields_by_id.contains_key(field_id) {
            continue;
        }

        calculated_field_ids.push(field_id.clone());
        let mut deps: Vec<String> = Vec::new();
        // Per-field dedupe: a dependency is resolved for every field that
        // references it, so the set must not leak across fields.
        let mut seen: HashSet<&str> = HashSet::new();
        for code in collect_references(calculate) {
            if let Some(id) = id_by_code.get(code.as_str())
                && seen.insert(*id)
            {
                deps.push((*id).to_string());
            }
        }
        dependencies.insert(field_id.clone(), deps);
    }

    let evaluation_order = topological_sort(&calculated_field_ids, &dependencies);
    Ok(RuleDependencyMetadata {
        calculated_field_ids,
        evaluation_order,
    })
}

/// Validates rule references, calculated-field read-only status and the
/// form/rules version match, and returns the analysis it computed so callers
/// that need both pay for one pass (SPEC R-14).
pub fn analyze_checked(
    form_root: &JsonMap,
    rules_root: &JsonMap,
) -> Result<RuleDependencyMetadata> {
    let fields_by_id = index::build_by_id(form_root)?;
    let fields_by_code = index::build_by_code(form_root)?;

    validate_form_version_match(form_root, rules_root)?;

    let child_repeaters = RowSet::child_repeater_by_code(form_root);
    let repeater_parents = RowSet::repeater_parents(form_root);

    // A malformed container must be an error, not a silently ignored one: a
    // caller that sends `fields: []` and gets `ok: true` believes its rules
    // were applied when none were (SPEC R-4).
    if let Some(visible_value) = json::get(rules_root, schema_json_keys::FIELDS)
        && !visible_value.is_null()
        && visible_value.as_object().is_none()
    {
        return Err(ColanderError::new(format!(
            "RULE_FIELDS_NOT_OBJECT: 'fields' must be an object keyed by field id, found {}.",
            crate::json::type_name(visible_value)
        )));
    }
    if let Some(visible_value) = json::get(rules_root, schema_json_keys::VALIDATIONS)
        && !visible_value.is_null()
        && visible_value.as_array().is_none()
    {
        return Err(ColanderError::new(format!(
            "RULE_VALIDATIONS_NOT_ARRAY: 'validations' must be an array, found {}.",
            crate::json::type_name(visible_value)
        )));
    }

    if let Some(field_rules) = json::get_object(rules_root, schema_json_keys::FIELDS) {
        validate_field_rules(
            field_rules,
            &fields_by_id,
            &fields_by_code,
            &child_repeaters,
            &repeater_parents,
        )?;
    }

    if let Some(validations) = json::get_array(rules_root, schema_json_keys::VALIDATIONS) {
        validate_validation_entries(validations, &fields_by_code, &child_repeaters)?;
    }

    let metadata = analyze(form_root, rules_root)?;
    if metadata.evaluation_order.len() != metadata.calculated_field_ids.len() {
        return Err(ColanderError::new(
            "RULE_CYCLIC_DEPENDENCY: calculated fields contain a cyclic dependency.",
        ));
    }
    Ok(metadata)
}

/// Validates rule references, calculated-field read-only status and the
/// form/rules version match.
pub fn validate_dependencies(form_root: &JsonMap, rules_root: &JsonMap) -> Result<()> {
    analyze_checked(form_root, rules_root).map(|_| ())
}

fn validate_form_version_match(form_root: &JsonMap, rules_root: &JsonMap) -> Result<()> {
    if let (Some(rules_version), Some(form_version)) = (
        json::get_str(rules_root, schema_json_keys::FORM_SCHEMA_VERSION),
        json::get_str(form_root, schema_json_keys::SCHEMA_VERSION),
    ) && rules_version != form_version
    {
        return Err(ColanderError::new(format!(
            "RULE_SCHEMA_VERSION_MISMATCH: rules formSchemaVersion '{rules_version}' does not match form schemaVersion '{form_version}'."
        )));
    }
    Ok(())
}

fn validate_field_rules(
    field_rules: &JsonMap,
    fields_by_id: &IndexMap<String, FieldInfo>,
    fields_by_code: &IndexMap<String, FieldInfo>,
    child_repeaters: &IndexMap<String, String>,
    repeater_parents: &IndexMap<String, String>,
) -> Result<()> {
    for (field_id, rules_node) in field_rules {
        let path = format!("/fields/{field_id}");
        let Some(field_info) = fields_by_id.get(field_id) else {
            return Err(ColanderError::new(format!(
                "RULE_UNKNOWN_FIELD: rules reference unknown form field id '{field_id}' at {path}."
            )));
        };

        let Some(rules) = rules_node.as_object() else {
            continue;
        };

        for key in rules.keys() {
            if !matches!(
                key.as_str(),
                "visibleWhen" | "enabledWhen" | "requiredWhen" | "calculate"
            ) {
                return Err(ColanderError::new(format!(
                    "RULE_UNKNOWN_RULE_KEY: rules for field '{field_id}' at {path}/{key} use an unknown rule key '{key}' (expected 'visibleWhen', 'enabledWhen', 'requiredWhen' or 'calculate')."
                )));
            }
        }

        for key in ["visibleWhen", "enabledWhen", "requiredWhen"] {
            let expression = json::get(rules, key);
            let expression_path = format!("{path}/{key}");
            validate_expression_shape(expression, &expression_path)?;
            validate_aggregate_arguments(expression, &expression_path)?;
            validate_expression_references(expression, &expression_path, fields_by_code)?;
            validate_row_scope(expression, &expression_path, child_repeaters, None)?;
            validate_aggregate_targets(expression, &expression_path, child_repeaters)?;
        }
        // A `calculate` on a repeater child runs in that row's scope, so it
        // may read its sibling child codes; anywhere else a child code has
        // no defined row to read from (the flat values keep only the last
        // row), and referencing one is an error rather than a silent
        // last-row read.
        let home = repeater_parents.get(field_id).map(String::as_str);
        let calculate_path = format!("{path}/calculate");
        let calculate = json::get(rules, schema_json_keys::CALCULATE);
        validate_expression_shape(calculate, &calculate_path)?;
        validate_aggregate_arguments(calculate, &calculate_path)?;
        validate_expression_references(calculate, &calculate_path, fields_by_code)?;
        validate_row_scope(calculate, &calculate_path, child_repeaters, home)?;
        validate_aggregate_targets(calculate, &calculate_path, child_repeaters)?;

        let calculate = json::get(rules, schema_json_keys::CALCULATE);
        if let Some(calculate) = calculate
            && !calculate.is_null()
        {
            if !field_info.read_only {
                return Err(ColanderError::new(format!(
                    "RULE_CALCULATE_NOT_READONLY: calculated field '{field_id}' at {path}/calculate must be readOnly in the form schema."
                )));
            }
            let target_code = &field_info.code;
            if collect_references(calculate)
                .iter()
                .any(|code| code == target_code)
            {
                return Err(ColanderError::new(format!(
                    "RULE_SELF_REFERENCE: calculated field '{field_id}' at {path}/calculate must not reference its own code '{target_code}'."
                )));
            }
        }
    }
    Ok(())
}

fn validate_validation_entries(
    validations: &[Json],
    fields_by_code: &IndexMap<String, FieldInfo>,
    child_repeaters: &IndexMap<String, String>,
) -> Result<()> {
    let mut seen_codes: HashSet<String> = HashSet::new();
    for (position, value) in validations.iter().enumerate() {
        let path = format!("/validations/{position}");
        let validation = value.as_object().ok_or_else(|| {
            ColanderError::new(format!(
                "RULE_INVALID_VALIDATION: {path} must be an object."
            ))
        })?;
        for key in validation.keys() {
            if !matches!(key.as_str(), "code" | "when" | "assert" | "message") {
                return Err(ColanderError::new(format!(
                    "RULE_UNKNOWN_VALIDATION_KEY: validation at {path} carries unknown key '{key}' (expected 'code', 'when', 'assert' or 'message')."
                )));
            }
        }
        let code = json::get_str(validation, schema_json_keys::CODE).ok_or_else(|| {
            ColanderError::new(format!(
                "RULE_INVALID_VALIDATION: {path} is missing a string 'code'."
            ))
        })?;
        let has_assert = matches!(
            json::get(validation, "assert"),
            Some(assert) if !assert.is_null()
        );
        if !has_assert {
            return Err(ColanderError::new(format!(
                "RULE_MISSING_ASSERT: validation at {path} has no 'assert' to evaluate."
            )));
        }

        if !seen_codes.insert(code.to_string()) {
            return Err(ColanderError::new(format!(
                "RULE_DUPLICATE_VALIDATION_CODE: validation code '{code}' at {path}/code is duplicated."
            )));
        }

        for key in ["when", "assert"] {
            let expression = json::get(validation, key);
            let expression_path = format!("{path}/{key}");
            validate_expression_shape(expression, &expression_path)?;
            validate_aggregate_arguments(expression, &expression_path)?;
            validate_expression_references(expression, &expression_path, fields_by_code)?;
            validate_row_scope(expression, &expression_path, child_repeaters, None)?;
            validate_aggregate_targets(expression, &expression_path, child_repeaters)?;
        }
    }
    Ok(())
}

fn validate_expression_references(
    expression: Option<&Json>,
    path: &str,
    fields_by_code: &IndexMap<String, FieldInfo>,
) -> Result<()> {
    let Some(expression) = expression else {
        return Ok(());
    };
    if expression.is_null() {
        return Ok(());
    }

    if let Some(unknown) = collect_references(expression)
        .into_iter()
        .find(|code| !fields_by_code.contains_key(code))
    {
        return Err(ColanderError::new(format!(
            "RULE_UNKNOWN_FIELD_REF: expression at {path} references unknown field code '{unknown}'."
        )));
    }
    Ok(())
}

fn topological_sort(
    calculated_field_ids: &[String],
    dependencies: &IndexMap<String, Vec<String>>,
) -> Vec<String> {
    let mut in_degree: IndexMap<&str, usize> = IndexMap::new();
    // Reverse adjacency, built once: `dependents[dep]` lists the fields that
    // wait on `dep`. Kahn's algorithm then touches every edge once instead of
    // rescanning every dependency list per dequeued node (SPEC R-14).
    let mut dependents: IndexMap<&str, Vec<&str>> = IndexMap::new();
    for field_id in calculated_field_ids {
        in_degree.insert(field_id.as_str(), 0);
        dependents.insert(field_id.as_str(), Vec::new());
    }
    for field_id in calculated_field_ids {
        if let Some(deps) = dependencies.get(field_id) {
            for dependency in deps {
                if let Some(waiters) = dependents.get_mut(dependency.as_str()) {
                    waiters.push(field_id.as_str());
                    *in_degree.get_mut(field_id.as_str()).expect("present") += 1;
                }
            }
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(field_id, _)| *field_id)
        .collect();
    let mut order = Vec::new();

    while let Some(current) = queue.pop_front() {
        order.push(current.to_string());
        for field_id in dependents
            .get(current)
            .map(|waiters| waiters.as_slice())
            .unwrap_or(&[])
        {
            let degree = in_degree.get_mut(field_id).expect("present");
            *degree -= 1;
            if *degree == 0 {
                queue.push_back(field_id);
            }
        }
    }

    order
}

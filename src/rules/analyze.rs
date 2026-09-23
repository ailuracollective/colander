//! Dependency metadata and structural rule validation.

use std::collections::{HashSet, VecDeque};

use indexmap::IndexMap;

use crate::error::{ColanderError, Result};
use crate::index::{self, FieldInfo};
use crate::json::{self, Json, JsonMap};
use crate::keys::schema_json_keys;

use super::model::RuleDependencyMetadata;

/// Builds dependency metadata: the calculated field ids in document order and
/// their topological evaluation order.
pub fn analyze(form_root: &JsonMap, rules_root: &JsonMap) -> Result<RuleDependencyMetadata> {
    let fields_by_id = index::build_by_id(form_root)?;

    let Some(field_rules) = json::get_object(rules_root, schema_json_keys::FIELDS) else {
        return Ok(RuleDependencyMetadata::default());
    };

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
        for code in collect_references(calculate) {
            if let Some(id) = resolve_field_id(&fields_by_id, &code)
                && !deps.iter().any(|item| item == id)
            {
                deps.push(id.to_string());
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
/// form/rules version match.
pub fn validate_dependencies(form_root: &JsonMap, rules_root: &JsonMap) -> Result<()> {
    let fields_by_id = index::build_by_id(form_root)?;
    let fields_by_code = index::build_by_code(form_root)?;

    validate_form_version_match(form_root, rules_root)?;

    if let Some(field_rules) = json::get_object(rules_root, schema_json_keys::FIELDS) {
        validate_field_rules(field_rules, &fields_by_id, &fields_by_code)?;
    }

    if let Some(validations) = json::get_array(rules_root, schema_json_keys::VALIDATIONS) {
        validate_validation_entries(validations, &fields_by_code)?;
    }

    let metadata = analyze(form_root, rules_root)?;
    if metadata.evaluation_order.len() != metadata.calculated_field_ids.len() {
        return Err(ColanderError::new(
            "RULE_CYCLIC_DEPENDENCY: calculated fields contain a cyclic dependency.",
        ));
    }
    Ok(())
}

/// Collects referenced field codes in document order.
///
/// Callers that report "the first unknown code" always name the first one in
/// document order.
pub fn collect_references(expression: &Json) -> Vec<String> {
    let mut references = Vec::new();
    collect_references_recursive(expression, &mut references);
    references
}

fn collect_references_recursive(node: &Json, references: &mut Vec<String>) {
    let Some(object) = node.as_object() else {
        return;
    };

    if let Some(code) = json::get_str(object, "ref")
        && !code.is_empty()
    {
        if !references.iter().any(|item| item == code) {
            references.push(code.to_string());
        }
        return;
    }

    let Some(args) = json::get_array(object, "args") else {
        return;
    };
    for arg in args {
        if !arg.is_null() {
            collect_references_recursive(arg, references);
        }
    }
}

fn resolve_field_id<'a>(
    fields_by_id: &'a IndexMap<String, FieldInfo>,
    code: &str,
) -> Option<&'a str> {
    fields_by_id
        .values()
        .find(|field| field.code == code)
        .map(|field| field.id.as_str())
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

        for key in ["visibleWhen", "enabledWhen", "requiredWhen"] {
            validate_expression_references(
                json::get(rules, key),
                &format!("{path}/{key}"),
                fields_by_code,
            )?;
        }
        validate_expression_references(
            json::get(rules, schema_json_keys::CALCULATE),
            &format!("{path}/calculate"),
            fields_by_code,
        )?;

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
) -> Result<()> {
    let mut seen_codes: HashSet<String> = HashSet::new();
    for (position, value) in validations.iter().enumerate() {
        let path = format!("/validations/{position}");
        let validation = value
            .as_object()
            .ok_or_else(|| ColanderError::new(format!("Expected validation object at {path}.")))?;
        let code = json::get_str(validation, schema_json_keys::CODE).ok_or_else(|| {
            ColanderError::new(format!("Expected validation code at {path}/code."))
        })?;

        if !seen_codes.insert(code.to_string()) {
            return Err(ColanderError::new(format!(
                "RULE_DUPLICATE_VALIDATION_CODE: validation code '{code}' at {path}/code is duplicated."
            )));
        }

        validate_expression_references(
            json::get(validation, "when"),
            &format!("{path}/when"),
            fields_by_code,
        )?;
        validate_expression_references(
            json::get(validation, "assert"),
            &format!("{path}/assert"),
            fields_by_code,
        )?;
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
    for field_id in calculated_field_ids {
        in_degree.insert(field_id.as_str(), 0);
    }
    for field_id in calculated_field_ids {
        if let Some(deps) = dependencies.get(field_id) {
            for dependency in deps {
                if in_degree.contains_key(dependency.as_str()) {
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
        for (field_id, field_dependencies) in dependencies {
            if !field_dependencies.iter().any(|item| item == current) {
                continue;
            }
            let degree = in_degree.get_mut(field_id.as_str()).expect("present");
            *degree -= 1;
            if *degree == 0 {
                queue.push_back(field_id.as_str());
            }
        }
    }

    order
}

//! Rule evaluation: predicates, calculations and validations.

use indexmap::IndexMap;

use crate::error::{ColanderError, Result};
use crate::index::{self, FieldInfo};
use crate::json::{self, JsonMap};
use crate::keys::schema_json_keys;

use super::analyze::analyze;
use super::expression::evaluate_expression;
use super::model::{FormRuleEvaluationResult, RuleDependencyMetadata, RuleValidationError};
use super::number::normalize_calculated_value;
use super::rows::RowSet;
use super::value::Val;

/// Evaluates calculations, per-field predicates and cross-field validations.
///
/// The form/rules dependency contract is enforced first, so a rules document
/// the analyzer would reject is refused before any expression runs. This is the
/// checked entry point; [`evaluate_core`] skips the contract for a synthesized
/// rules document.
pub fn evaluate(
    form_root: &JsonMap,
    rules_root: &JsonMap,
    values: &IndexMap<String, Val>,
    ui_schema_json: Option<&str>,
    rows: &mut RowSet,
) -> Result<FormRuleEvaluationResult> {
    let metadata = super::analyze::analyze_checked(form_root, rules_root)?;
    evaluate_core(
        form_root,
        rules_root,
        values,
        ui_schema_json,
        rows,
        Some(metadata),
    )
}

/// Evaluates a form/rules pair whose dependency contract is already satisfied.
///
/// The response validator synthesizes an empty rules document when the caller
/// supplies none, and that document must not be run through the dependency
/// check, so it reaches this function instead of [`evaluate`]. `metadata` is
/// passed in by callers that already analyzed the pair, so one call analyzes
/// once (SPEC R-14).
pub(crate) fn evaluate_core(
    form_root: &JsonMap,
    rules_root: &JsonMap,
    values: &IndexMap<String, Val>,
    ui_schema_json: Option<&str>,
    rows: &mut RowSet,
    metadata: Option<super::RuleDependencyMetadata>,
) -> Result<FormRuleEvaluationResult> {
    let fields_by_id = index::build_by_id(form_root)?;
    let metadata = match metadata {
        Some(metadata) => metadata,
        None => analyze(form_root, rules_root)?,
    };

    let mut working_values = values.clone();
    // R-7a/R-13: a repeater's rows live in `RowSet`, never in the flat working
    // values — only the row count does, exactly as the response-validator
    // path builds it. Leaving the row payload in the map would make every
    // per-row scope clone copy all N rows (quadratic), and it would make a
    // `ref` to an empty repeater read an empty list (truthy) here while the
    // validator reads a zero count (falsy).
    for code in rows.codes() {
        let count = rows.rows(code).map(|rows| rows.len()).unwrap_or(0);
        working_values.insert(code.to_string(), Val::Int(count as i64));
    }
    let mut result = FormRuleEvaluationResult {
        visibility: IndexMap::new(),
        enabled: IndexMap::new(),
        required: IndexMap::new(),
        calculated_values: IndexMap::new(),
        validation_errors: Vec::new(),
    };

    let ui_root = match ui_schema_json {
        Some(text) => Some(json::parse_object(text, "UI schema")?),
        None => None,
    };

    for field in fields_by_id.values() {
        let hidden = is_ui_hidden(ui_root.as_ref(), &field.id);
        result.visibility.insert(field.id.clone(), !hidden);
        result.enabled.insert(field.id.clone(), !field.read_only);
        result.required.insert(field.id.clone(), field.required);
    }

    if let Some(field_rules) = json::get_object(rules_root, schema_json_keys::FIELDS) {
        let parents = RowSet::repeater_parents(form_root);
        apply_calculations(
            field_rules,
            &metadata,
            &fields_by_id,
            &mut working_values,
            &mut result.calculated_values,
            &parents,
            rows,
        )?;
        apply_field_predicates(
            field_rules,
            &fields_by_id,
            &working_values,
            &mut result,
            rows,
        )?;
    }

    result.validation_errors = collect_validation_errors(rules_root, &working_values, rows)?;
    Ok(result)
}

fn is_ui_hidden(ui_root: Option<&JsonMap>, field_id: &str) -> bool {
    let Some(ui_root) = ui_root else {
        return false;
    };
    json::get_object(ui_root, schema_json_keys::FIELDS)
        .and_then(|fields| json::get_object(fields, field_id))
        .and_then(|presentation| json::get_bool(presentation, "hidden"))
        .unwrap_or(false)
}

fn apply_calculations(
    field_rules: &JsonMap,
    metadata: &RuleDependencyMetadata,
    fields_by_id: &IndexMap<String, FieldInfo>,
    working_values: &mut IndexMap<String, Val>,
    calculated_values: &mut IndexMap<String, Val>,
    parents: &IndexMap<String, String>,
    rows: &mut RowSet,
) -> Result<()> {
    // Calculated codes already materialised per row, with the repeater they
    // belong to; used to keep those arrays out of the next row template.
    let mut per_row_outputs: IndexMap<String, String> = IndexMap::new();
    for field_id in &metadata.evaluation_order {
        let Some(rules) = json::get_object(field_rules, field_id) else {
            continue;
        };
        let Some(calculate) = json::get(rules, schema_json_keys::CALCULATE) else {
            continue;
        };
        if calculate.is_null() {
            continue;
        }
        let Some(field_info) = fields_by_id.get(field_id) else {
            continue;
        };

        // R-7: per-row evaluation in row scope; results are written back so
        // later calculations observe computed values, and the array replaces
        // the flattened value downstream.
        if let Some(repeater_code) = parents.get(field_id)
            && let Some(row_list) = rows.rows(repeater_code)
            && !row_list.is_empty()
        {
            // The row overlays a template of the outer values, and only the
            // row's own keys change between iterations: a child code shadows
            // the template, and anything written back into the template (a
            // later calculation's array, or the repeater's own count) stays
            // visible to the next row. Cloning the template once per
            // calculation instead of once per row is what keeps an N-row
            // calculation linear rather than quadratic (SPEC R-13).
            // R-7/R-13/R-14: the per-row scope is the row over the outer
            // values, and the working set carries only the row *count* for a
            // repeater — never its rows and never the arrays a previous
            // per-row calculation produced. Cloning an N-element array once
            // per row would make chained per-row calculations quadratic, so
            // those arrays stay out of the template; a row referencing a
            // sibling calculated child reads that child's value *for this
            // row* from the row map (where the write-back already put it),
            // not the whole array. Outside row scope the array is still what
            // a reference observes.
            let mut template = working_values.clone();
            for code in per_row_calculated_codes(repeater_code, &per_row_outputs) {
                template.shift_remove(code);
            }
            let mut results = Vec::with_capacity(row_list.len());
            for row in row_list {
                let mut scoped = template.clone();
                for (child_code, child_value) in row {
                    scoped.insert(child_code.clone(), child_value.clone());
                }
                let computed = normalize_calculated_value(
                    evaluate_expression(calculate, &scoped, rows)?,
                    field_info,
                );
                results.push(computed);
            }
            if let Some(row_list) = rows.rows_mut(repeater_code) {
                for (row, computed) in row_list.iter_mut().zip(results.iter()) {
                    row.insert(field_info.code.clone(), computed.clone());
                }
            }
            let array = Val::List(results);
            calculated_values.insert(field_info.code.clone(), array.clone());
            working_values.insert(field_info.code.clone(), array);
            per_row_outputs.insert(field_info.code.clone(), repeater_code.to_string());
            continue;
        }

        let calculated = normalize_calculated_value(
            evaluate_expression(calculate, working_values, rows)?,
            field_info,
        );
        calculated_values.insert(field_info.code.clone(), calculated.clone());
        working_values.insert(field_info.code.clone(), calculated);
    }
    Ok(())
}

/// Codes whose per-row arrays belong to `repeater_code`.
fn per_row_calculated_codes<'a>(
    repeater_code: &str,
    per_row_outputs: &'a IndexMap<String, String>,
) -> impl Iterator<Item = &'a String> {
    per_row_outputs
        .iter()
        .filter(move |(_, owner)| owner.as_str() == repeater_code)
        .map(|(code, _)| code)
}

fn apply_field_predicates(
    field_rules: &JsonMap,
    fields_by_id: &IndexMap<String, FieldInfo>,
    working_values: &IndexMap<String, Val>,
    result: &mut FormRuleEvaluationResult,
    rows: &RowSet,
) -> Result<()> {
    for (field_id, rules_node) in field_rules {
        let Some(rules) = rules_node.as_object() else {
            continue;
        };
        if !fields_by_id.contains_key(field_id) {
            continue;
        }

        if let Some(node) = json::get(rules, "visibleWhen")
            && !node.is_null()
        {
            result.visibility.insert(
                field_id.clone(),
                evaluate_expression(node, working_values, rows)?.to_bool(),
            );
        }
        if let Some(node) = json::get(rules, "enabledWhen")
            && !node.is_null()
        {
            result.enabled.insert(
                field_id.clone(),
                evaluate_expression(node, working_values, rows)?.to_bool(),
            );
        }
        if let Some(node) = json::get(rules, "requiredWhen")
            && !node.is_null()
        {
            result.required.insert(
                field_id.clone(),
                evaluate_expression(node, working_values, rows)?.to_bool(),
            );
        }
    }
    Ok(())
}

fn collect_validation_errors(
    rules_root: &JsonMap,
    working_values: &IndexMap<String, Val>,
    rows: &RowSet,
) -> Result<Vec<RuleValidationError>> {
    let mut errors = Vec::new();
    let Some(validations) = json::get_array(rules_root, schema_json_keys::VALIDATIONS) else {
        return Ok(errors);
    };

    for (position, value) in validations.iter().enumerate() {
        let validation = value.as_object().ok_or_else(|| {
            ColanderError::new(format!(
                "Expected validation object at /validations/{position}."
            ))
        })?;
        let code = json::get_str(validation, schema_json_keys::CODE)
            .map(str::to_string)
            .unwrap_or_else(|| format!("VALIDATION_{position}"));
        let message = json::get_str(validation, schema_json_keys::MESSAGE)
            .map(str::to_string)
            .unwrap_or_else(|| "Validation failed.".to_string());

        if let Some(when) = json::get(validation, "when")
            && !when.is_null()
            && !evaluate_expression(when, working_values, rows)?.to_bool()
        {
            continue;
        }

        if let Some(assert) = json::get(validation, "assert")
            && !assert.is_null()
            && !evaluate_expression(assert, working_values, rows)?.to_bool()
        {
            errors.push(RuleValidationError { code, message });
        }
    }

    Ok(errors)
}

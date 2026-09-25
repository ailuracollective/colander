//! Repeater row and item-count validation.

use indexmap::IndexMap;
use std::collections::HashSet;

use crate::error::Result;
use crate::index::AnswerFieldDefinition;
use crate::json::{self, Json};
use crate::rules::{self, Val};

use super::fields::{is_empty_element, is_empty_value, validate_scalar_field};
use super::model::{FormResponseFieldError, FormResponseValidationMode};

pub(super) fn validate_repeater(
    repeater: &AnswerFieldDefinition,
    raw_rows: Option<&Json>,
    evaluation: &rules::FormRuleEvaluationResult,
    mode: FormResponseValidationMode,
    errors: &mut Vec<FormResponseFieldError>,
    normalized: &mut IndexMap<String, Val>,
) -> Result<()> {
    let mut raw_rows = raw_rows;
    if !gate_repeater_access(
        repeater,
        &mut raw_rows,
        evaluation,
        mode,
        errors,
        normalized,
    ) {
        return Ok(());
    }

    let rows_slice = raw_rows
        .and_then(Json::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let row_count = rows_slice.len();
    validate_repeater_item_count(repeater, row_count, mode, errors);

    let child_codes: HashSet<&str> = repeater
        .children
        .iter()
        .map(|child| child.code.as_str())
        .collect();

    let mut rows: Vec<IndexMap<String, Val>> = Vec::new();
    for (row_index, row) in rows_slice.iter().enumerate() {
        validate_repeater_row(
            repeater,
            row,
            row_index,
            &child_codes,
            evaluation,
            mode,
            errors,
            &mut rows,
        )?;
    }

    normalized.insert(repeater.code.clone(), Val::Rows(rows));
    Ok(())
}

pub(super) fn gate_repeater_access(
    repeater: &AnswerFieldDefinition,
    raw_rows: &mut Option<&Json>,
    evaluation: &rules::FormRuleEvaluationResult,
    mode: FormResponseValidationMode,
    errors: &mut Vec<FormResponseFieldError>,
    normalized: &mut IndexMap<String, Val>,
) -> bool {
    let visible = evaluation
        .visibility
        .get(&repeater.id)
        .copied()
        .unwrap_or(true);
    let enabled = evaluation
        .enabled
        .get(&repeater.id)
        .copied()
        .unwrap_or(true);

    if !visible || !enabled {
        if let Some(Json::Array(rows)) = raw_rows
            && !rows.is_empty()
        {
            errors.push(FormResponseFieldError {
                code: if visible {
                    "DISABLED_FIELD_VALUE"
                } else {
                    "HIDDEN_FIELD_VALUE"
                }
                .to_string(),
                path: repeater.path.clone(),
                message: format!(
                    "Repeater '{}' cannot accept values while hidden or disabled.",
                    crate::validate::ellipsize(&repeater.code)
                ),
            });
        }
        normalized.insert(repeater.code.clone(), Val::Rows(Vec::new()));
        return false;
    }

    if matches!(raw_rows, None | Some(Json::Null)) {
        *raw_rows = None;
    }

    if !matches!(raw_rows, Some(Json::Array(_))) {
        if !is_empty_element(*raw_rows) {
            errors.push(FormResponseFieldError {
                code: "INVALID_TYPE".to_string(),
                path: repeater.path.clone(),
                message: format!("Repeater '{}' must be an array.", repeater.code),
            });
        }
        normalized.insert(repeater.code.clone(), Val::Rows(Vec::new()));
        validate_repeater_item_count(repeater, 0, mode, errors);
        return false;
    }

    true
}

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_repeater_row(
    repeater: &AnswerFieldDefinition,
    row: &Json,
    row_index: usize,
    child_codes: &HashSet<&str>,
    evaluation: &rules::FormRuleEvaluationResult,
    mode: FormResponseValidationMode,
    errors: &mut Vec<FormResponseFieldError>,
    rows: &mut Vec<IndexMap<String, Val>>,
) -> Result<()> {
    let row_path = format!("{}/{}", repeater.path, row_index);

    let Some(row_object) = row.as_object() else {
        errors.push(FormResponseFieldError {
            code: "INVALID_TYPE".to_string(),
            path: row_path,
            message: format!("Repeater row {row_index} must be an object."),
        });
        return Ok(());
    };

    let mut row_answers: IndexMap<&str, &Json> = IndexMap::new();
    for (name, value) in row_object {
        if !child_codes.contains(name.as_str()) {
            errors.push(FormResponseFieldError {
                code: "UNKNOWN_FIELD".to_string(),
                path: format!("{row_path}/{}", crate::validate::ellipsize(name)),
                message: format!(
                    "Unknown repeater field '{}'.",
                    crate::validate::ellipsize(name)
                ),
            });
            continue;
        }
        row_answers.insert(name.as_str(), value);
    }

    let mut normalized_row: IndexMap<String, Val> = IndexMap::new();
    for child in &repeater.children {
        let mut child_field = child.clone();
        child_field.path = format!("{row_path}/{}", child.id);
        // A calculated child is projected per row from the array R-7 stores
        // under its code, with the same mismatch/required/invalid contract
        // as a scalar calculated field (SPEC V-3 symmetry).
        if let Some(calculated) = evaluation.calculated_values.get(&child.code) {
            let element = match calculated {
                Val::List(items) => items.get(row_index).cloned().unwrap_or(Val::Null),
                scalar => scalar.clone(),
            };
            let submitted = row_answers.get(child.code.as_str()).copied();
            apply_calculated_repeater_child(
                &child_field,
                submitted,
                &element,
                evaluation,
                mode,
                errors,
                &mut normalized_row,
            )?;
            continue;
        }
        let value = row_answers.get(child.code.as_str()).copied();
        validate_scalar_field(
            &child_field,
            value,
            evaluation,
            mode,
            errors,
            &mut normalized_row,
        )?;
    }

    rows.push(normalized_row);
    Ok(())
}

pub(super) fn validate_repeater_item_count(
    repeater: &AnswerFieldDefinition,
    row_count: usize,
    mode: FormResponseValidationMode,
    errors: &mut Vec<FormResponseFieldError>,
) {
    if mode != FormResponseValidationMode::Complete {
        return;
    }

    let min_items = json::get_i64(&repeater.schema, "minItems").unwrap_or(0);
    let max_items = json::get_i64(&repeater.schema, "maxItems");

    if (row_count as i64) < min_items {
        errors.push(FormResponseFieldError {
            code: "REPEATER_MIN_ITEMS".to_string(),
            path: repeater.path.clone(),
            message: format!(
                "Repeater '{}' requires at least {min_items} items.",
                repeater.code
            ),
        });
    }

    if let Some(max_items) = max_items
        && (row_count as i64) > max_items
    {
        errors.push(FormResponseFieldError {
            code: "REPEATER_MAX_ITEMS".to_string(),
            path: repeater.path.clone(),
            message: format!(
                "Repeater '{}' allows at most {max_items} items.",
                repeater.code
            ),
        });
    }
}

/// Per-row twin of `calculated::apply_calculated_fields`: compares the
/// submitted cell against the row's calculated element and stores the
/// element, so calculated repeater children are neither dropped from the
/// normalized rows nor exempt from the mismatch contract.
#[allow(clippy::too_many_arguments)]
fn apply_calculated_repeater_child(
    field: &AnswerFieldDefinition,
    submitted: Option<&Json>,
    element: &Val,
    evaluation: &rules::FormRuleEvaluationResult,
    mode: FormResponseValidationMode,
    errors: &mut Vec<FormResponseFieldError>,
    normalized_row: &mut IndexMap<String, Val>,
) -> Result<()> {
    let submitted_value = match submitted {
        None | Some(Json::Null) => None,
        Some(value) => match Val::from_json_element(value) {
            Ok(converted) => Some(converted),
            Err(_) => {
                errors.push(FormResponseFieldError {
                    code: "INVALID_TYPE".to_string(),
                    path: field.path.clone(),
                    message: format!("Field '{}' has the wrong JSON type.", field.code),
                });
                return Ok(());
            }
        },
    };

    if let Some(submitted) = &submitted_value
        && !Val::values_equal(submitted, element)
        && mode == FormResponseValidationMode::Complete
    {
        errors.push(FormResponseFieldError {
            code: "CALCULATED_VALUE_MISMATCH".to_string(),
            path: field.path.clone(),
            message: format!(
                "Field '{}' must match the server-calculated value.",
                field.code
            ),
        });
    }

    if let Val::Double(number) = element
        && (!number.is_finite() || !super::calculated::is_representable(field, *number))
    {
        if mode == FormResponseValidationMode::Complete {
            errors.push(FormResponseFieldError {
                code: "CALCULATED_VALUE_INVALID".to_string(),
                path: field.path.clone(),
                message: format!(
                    "Calculated field '{}' could not be determined from the current answers.",
                    field.code
                ),
            });
        }
        return Ok(());
    }

    if mode == FormResponseValidationMode::Complete
        && evaluation.required.get(&field.id).copied().unwrap_or(false)
        && is_empty_value(element)
    {
        errors.push(FormResponseFieldError {
            code: "REQUIRED_FIELD_MISSING".to_string(),
            path: field.path.clone(),
            message: format!("Calculated field '{}' is required.", field.code),
        });
    }

    normalized_row.insert(field.code.clone(), element.clone());
    Ok(())
}

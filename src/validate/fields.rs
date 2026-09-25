//! Field-level answer validation.

use indexmap::IndexMap;
use std::collections::HashSet;

use crate::error::Result;
use crate::index::AnswerFieldDefinition;
use crate::json::{Json, JsonMap};
use crate::keys::field_type_names;
use crate::rules::{self, Val};

use super::constraints::validate_constraints;
use super::conversion::try_convert_value;
use super::model::{FormResponseFieldError, FormResponseValidationMode};

pub(super) fn validate_known_top_level_keys(
    answers: &JsonMap,
    fields_by_code: &IndexMap<String, AnswerFieldDefinition>,
    errors: &mut Vec<FormResponseFieldError>,
) {
    let allowed: HashSet<&str> = fields_by_code
        .values()
        .filter(|field| {
            !matches!(
                field.field_type.as_str(),
                field_type_names::GROUP | field_type_names::COMPONENT_REF
            )
        })
        .map(|field| field.code.as_str())
        .collect();

    for key in answers.keys() {
        if !allowed.contains(key.as_str()) {
            errors.push(FormResponseFieldError {
                code: "UNKNOWN_FIELD".to_string(),
                path: format!("/answers/{}", crate::validate::ellipsize(key)),
                message: format!(
                    "Unknown answer field '{}'.",
                    crate::validate::ellipsize(key)
                ),
            });
        }
    }
}

pub(super) fn validate_scalar_field(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
    evaluation: &rules::FormRuleEvaluationResult,
    mode: FormResponseValidationMode,
    errors: &mut Vec<FormResponseFieldError>,
    normalized: &mut IndexMap<String, Val>,
) -> Result<()> {
    if evaluation.calculated_values.contains_key(&field.code) {
        return Ok(());
    }

    let (visible, enabled, required) = resolve_field_flags(field, evaluation);

    if try_reject_inactive_value(field, value, visible, enabled, errors)
        || try_reject_read_only_value(field, value, evaluation, errors)
        || try_reject_required_empty(field, value, mode, required, errors)
    {
        return Ok(());
    }

    let Some(converted) = try_convert_value(field, value, errors)? else {
        return Ok(());
    };

    if !validate_constraints(field, &converted, errors)? {
        return Ok(());
    }

    normalized.insert(field.code.clone(), converted);
    Ok(())
}

/// Flag resolution: the rule evaluation owns all three flags. It seeds them
/// from the schema (`required`, `readOnly`, UI `hidden`), and a predicate
/// overwrites its flag — including `requiredWhen: false` unsetting a schema
/// `required: true`, symmetric with `visibleWhen`/`enabledWhen`.
pub(super) fn resolve_field_flags(
    field: &AnswerFieldDefinition,
    evaluation: &rules::FormRuleEvaluationResult,
) -> (bool, bool, bool) {
    let visible = evaluation
        .visibility
        .get(&field.id)
        .copied()
        .unwrap_or(true);
    let enabled = evaluation.enabled.get(&field.id).copied().unwrap_or(true);
    let required = evaluation
        .required
        .get(&field.id)
        .copied()
        .unwrap_or(field.required);
    (visible, enabled, required)
}

pub(super) fn try_reject_inactive_value(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
    visible: bool,
    enabled: bool,
    errors: &mut Vec<FormResponseFieldError>,
) -> bool {
    if visible && enabled {
        return false;
    }

    if !is_empty_element(value) {
        errors.push(FormResponseFieldError {
            code: if visible {
                "DISABLED_FIELD_VALUE"
            } else {
                "HIDDEN_FIELD_VALUE"
            }
            .to_string(),
            path: field.path.clone(),
            message: format!(
                "Field '{}' cannot accept values while hidden or disabled.",
                crate::validate::ellipsize(&field.code)
            ),
        });
    }

    true
}

pub(super) fn try_reject_read_only_value(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
    evaluation: &rules::FormRuleEvaluationResult,
    errors: &mut Vec<FormResponseFieldError>,
) -> bool {
    if !field.read_only || evaluation.calculated_values.contains_key(&field.code) {
        return false;
    }

    if !is_empty_element(value) {
        errors.push(FormResponseFieldError {
            code: "READONLY_FIELD_MODIFIED".to_string(),
            path: field.path.clone(),
            message: format!(
                "Field '{}' is read-only.",
                crate::validate::ellipsize(&field.code)
            ),
        });
    }

    true
}

pub(super) fn try_reject_required_empty(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
    mode: FormResponseValidationMode,
    required: bool,
    errors: &mut Vec<FormResponseFieldError>,
) -> bool {
    if !is_empty_element(value)
        || (field.field_type == crate::keys::field_type_names::FILE
            && matches!(value, Some(Json::Object(_))))
    {
        return false;
    }

    if mode == FormResponseValidationMode::Complete && required {
        errors.push(FormResponseFieldError {
            code: "REQUIRED_FIELD_MISSING".to_string(),
            path: field.path.clone(),
            message: format!(
                "Field '{}' is required.",
                crate::validate::ellipsize(&field.code)
            ),
        });
    }

    true
}

/// An absent value, `null`, an empty string, array or object counts as empty;
/// numbers and booleans never do.
pub(super) fn is_empty_element(value: Option<&Json>) -> bool {
    match value {
        None | Some(Json::Null) => true,
        Some(Json::String(text)) => text.is_empty(),
        Some(Json::Array(items)) => items.is_empty(),
        Some(Json::Object(map)) => map.is_empty(),
        Some(Json::Number(_)) | Some(Json::Bool(_)) => false,
    }
}

/// A `null`, an empty string, list or row set counts as empty; booleans,
/// numbers and raw values never do.
pub(super) fn is_empty_value(value: &Val) -> bool {
    match value {
        Val::Null => true,
        Val::Str(text) => text.is_empty(),
        Val::List(items) => items.is_empty(),
        Val::Rows(rows) => rows.is_empty(),
        Val::Bool(_) | Val::Int(_) | Val::Double(_) | Val::Raw(_) => false,
    }
}

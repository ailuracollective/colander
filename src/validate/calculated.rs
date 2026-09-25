//! Calculated fields and the rules hand-off.

use indexmap::IndexMap;

use crate::error::Result;
use crate::index::AnswerFieldDefinition;
use crate::json::{self, Json, JsonMap};
use crate::keys::{field_type_names, schema_json_keys};
use crate::rules::{self, Val};
use crate::semantic::FormSemantics;

use super::fields::is_empty_value;
use super::model::{FormResponseFieldError, FormResponseValidationMode};

pub(super) fn apply_calculated_fields(
    answers: &JsonMap,
    fields_by_code: &IndexMap<String, AnswerFieldDefinition>,
    evaluation: &rules::FormRuleEvaluationResult,
    mode: FormResponseValidationMode,
    errors: &mut Vec<FormResponseFieldError>,
    normalized: &mut IndexMap<String, Val>,
) -> Result<()> {
    for (code, calculated_value) in &evaluation.calculated_values {
        let Some(field) = fields_by_code.get(code) else {
            continue;
        };

        let submitted_value = match answers.get(code) {
            None | Some(Json::Null) => None,
            Some(value) => match Val::from_json_element(value) {
                Ok(converted) => Some(converted),
                Err(_) => {
                    errors.push(FormResponseFieldError {
                        code: "INVALID_TYPE".to_string(),
                        path: field.path.clone(),
                        message: format!("Field '{}' has the wrong JSON type.", field.code),
                    });
                    continue;
                }
            },
        };

        if let Some(submitted) = &submitted_value
            && !Val::values_equal(submitted, calculated_value)
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

        if let Val::Double(number) = calculated_value
            && (!number.is_finite() || !is_representable(field, *number))
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
            continue;
        }

        if mode == FormResponseValidationMode::Complete
            && evaluation.required.get(&field.id).copied().unwrap_or(false)
            && is_empty_value(calculated_value)
        {
            errors.push(FormResponseFieldError {
                code: "REQUIRED_FIELD_MISSING".to_string(),
                path: field.path.clone(),
                message: format!("Calculated field '{}' is required.", field.code),
            });
        }

        // A calculated value is still held to the field's own type and
        // constraints. Skipping this let a rule publish `100` for a field
        // declared `maximum: 10`, and the response reported it as valid: the
        // client did nothing wrong, so this is reported as a calculated-value
        // failure rather than a client error.
        if mode == FormResponseValidationMode::Complete
            && !calculated_value.is_empty()
            && !calculated_value_satisfies_field(field, calculated_value)
        {
            errors.push(FormResponseFieldError {
                code: "CALCULATED_VALUE_INVALID".to_string(),
                path: field.path.clone(),
                message: format!(
                    "Calculated value for field '{}' does not satisfy the field's type and constraints.",
                    field.code
                ),
            });
            continue;
        }

        normalized.insert(code.clone(), calculated_value.clone());
    }

    Ok(())
}

/// Whether a calculated value is exactly representable for its field.
///
/// Arithmetic runs in `f64`, so an `integer` field only keeps full precision
/// while its value fits the exactly-representable integer range (2^53). Above
/// it, rounding would silently rewrite the client's exact integer (SPEC V-11):
/// the safety net reports `CALCULATED_VALUE_INVALID` instead of storing a
/// value the client never sent.
pub(crate) fn is_representable(field: &AnswerFieldDefinition, number: f64) -> bool {
    if field.field_type != field_type_names::INTEGER {
        return true;
    }
    number.abs() <= 9_007_199_254_740_992.0 // 2^53
}
// Rules input and evaluation
// ---------------------------------------------------------------------------

/// Whether a calculated value satisfies the field's declared type and its
/// constraints. The rules that decide this are the same ones a submitted value
/// goes through, so a calculated field cannot hold something a client could
/// never submit.
fn calculated_value_satisfies_field(field: &AnswerFieldDefinition, value: &Val) -> bool {
    let number = match (field.field_type.as_str(), value) {
        (field_type_names::NUMBER, Val::Double(number)) => Some(*number),
        (field_type_names::NUMBER, Val::Int(number)) => Some(*number as f64),
        (field_type_names::INTEGER, Val::Double(number)) => {
            if number.fract() == 0.0 {
                Some(*number)
            } else {
                return false;
            }
        }
        (field_type_names::INTEGER, Val::Int(_)) => None,
        (field_type_names::TEXT, Val::Str(_)) | (field_type_names::BOOLEAN, Val::Bool(_)) => {
            return super::constraints::validate_constraints(field, value, &mut Vec::new())
                .unwrap_or(true);
        }
        // An empty or absent value is handled by the required check above, and
        // a value of another shape is a type error, not a constraint error.
        _ => return !matches!(value, Val::List(_) | Val::Rows(_) | Val::Raw(_)),
    };

    let Some(number) = number else {
        return true;
    };
    if !number.is_finite() {
        return false;
    }
    super::constraints::validate_constraints(field, &Val::Double(number), &mut Vec::new())
        .unwrap_or(true)
}

pub(super) fn flatten_for_rules(
    answers: &JsonMap,
    fields_by_code: &IndexMap<String, AnswerFieldDefinition>,
) -> IndexMap<String, Val> {
    let mut flat: IndexMap<String, Val> = IndexMap::new();

    for (code, value) in answers {
        if fields_by_code
            .get(code)
            .is_some_and(|field| field.field_type == field_type_names::REPEATER)
        {
            flatten_repeater_answer(&mut flat, code, value);
            continue;
        }
        match Val::from_json_element(value) {
            Ok(converted) => {
                flat.insert(code.clone(), converted);
            }
            Err(_) => {
                flat.insert(code.clone(), Val::Null);
            }
        }
    }

    flat
}

pub(super) fn flatten_repeater_answer(flat: &mut IndexMap<String, Val>, code: &str, value: &Json) {
    let Some(rows) = value.as_array() else {
        flat.insert(code.to_string(), Val::Int(0));
        return;
    };

    flat.insert(code.to_string(), Val::Int(rows.len() as i64));
    for row in rows {
        let Some(row_object) = row.as_object() else {
            continue;
        };
        for (name, property) in row_object {
            match Val::from_json_element(property) {
                Ok(converted) => {
                    flat.insert(name.clone(), converted);
                }
                Err(_) => {
                    flat.insert(name.clone(), Val::Null);
                }
            }
        }
    }
}

pub(super) fn evaluate_rules(
    form_root: &JsonMap,
    form_semantics: &FormSemantics,
    rules_schema_json: Option<&str>,
    ui_schema_json: Option<&str>,
    rule_values: &IndexMap<String, Val>,
    rows: &mut rules::RowSet,
) -> Result<rules::FormRuleEvaluationResult> {
    if let Some(text) = rules_schema_json
        && !text.trim().is_empty()
    {
        let rules_root = json::parse_object(text, "rules schema")?;
        return rules::RowSet::evaluate_checked_with_semantics(
            form_root,
            form_semantics,
            &rules_root,
            rule_values,
            ui_schema_json,
            rows,
        );
    }

    let form_version =
        json::get_str(form_root, schema_json_keys::SCHEMA_VERSION).unwrap_or("1.0.0");
    let synthesized = format!(
        "{{\n  \"schemaVersion\": \"1.0.0\",\n  \"formSchemaVersion\": \"{form_version}\",\n  \"fields\": {{}}\n}}"
    );
    let rules_root = json::parse_object(&synthesized, "rules schema")?;
    rules::RowSet::evaluate_core_with_semantics(
        form_root,
        form_semantics,
        &rules_root,
        rule_values,
        ui_schema_json,
        rows,
    )
}

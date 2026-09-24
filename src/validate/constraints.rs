//! Constraint evaluation over converted values.

use crate::error::{ColanderError, Result};
use crate::index::AnswerFieldDefinition;
use crate::json::{self};
use crate::keys::field_type_names;
use crate::rules::Val;

use super::model::FormResponseFieldError;

pub(super) fn validate_constraints(
    field: &AnswerFieldDefinition,
    value: &Val,
    errors: &mut Vec<FormResponseFieldError>,
) -> Result<bool> {
    let error = match field.field_type.as_str() {
        field_type_names::TEXT | field_type_names::TEXTAREA => {
            let text = match value {
                Val::Str(text) => text,
                _ => return Ok(true),
            };
            validate_string_constraints(field, text)?
        }
        field_type_names::NUMBER => match value.to_double() {
            Ok(number) => validate_numeric_constraints(field, number),
            Err(_) => None,
        },
        field_type_names::INTEGER => {
            let number = match value {
                Val::Int(number) => *number as f64,
                _ => return Ok(true),
            };
            validate_numeric_constraints(field, number)
        }
        _ => None,
    };

    match error {
        Some(error) => {
            errors.push(error);
            Ok(false)
        }
        None => Ok(true),
    }
}

pub(super) fn constraint_error(
    field: &AnswerFieldDefinition,
    message: String,
) -> FormResponseFieldError {
    FormResponseFieldError {
        code: "CONSTRAINT_VIOLATION".to_string(),
        path: field.path.clone(),
        message: format!("Field '{}' {message}", field.code),
    }
}

/// Length is counted in UTF-16 code units, so an astral character counts as two.
pub(super) fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

pub(super) fn validate_string_constraints(
    field: &AnswerFieldDefinition,
    value: &str,
) -> Result<Option<FormResponseFieldError>> {
    let length = utf16_len(value);

    if let Some(min_length) = json::get_i64(&field.schema, "minLength")
        && (length as i64) < min_length
    {
        return Ok(Some(constraint_error(
            field,
            format!("must be at least {min_length} characters."),
        )));
    }

    if let Some(max_length) = json::get_i64(&field.schema, "maxLength")
        && (length as i64) > max_length
    {
        return Ok(Some(constraint_error(
            field,
            format!("must be at most {max_length} characters."),
        )));
    }

    if let Some(pattern) = json::get_str(&field.schema, "pattern")
        && !pattern.is_empty()
    {
        // S-6: the same ECMA-262 engine as the JSON Schema subset, and the
        // same rule. An uncompilable pattern fails the call instead of
        // silently never matching.
        let matched = crate::pattern::is_match(pattern, value).map_err(ColanderError::new)?;
        if !matched {
            return Ok(Some(constraint_error(
                field,
                "does not match the required pattern.".to_string(),
            )));
        }
    }

    Ok(None)
}

pub(super) fn validate_numeric_constraints(
    field: &AnswerFieldDefinition,
    value: f64,
) -> Option<FormResponseFieldError> {
    if let Some(minimum) = json::get_f64(&field.schema, "minimum")
        && value < minimum
    {
        return Some(constraint_error(
            field,
            format!(
                "must be greater than or equal to {}.",
                json::format_double(minimum)
            ),
        ));
    }

    if let Some(maximum) = json::get_f64(&field.schema, "maximum")
        && value > maximum
    {
        return Some(constraint_error(
            field,
            format!(
                "must be less than or equal to {}.",
                json::format_double(maximum)
            ),
        ));
    }

    if let Some(multiple_of) = json::get_f64(&field.schema, "multipleOf")
        && multiple_of != 0.0
        && ((value / multiple_of) - (value / multiple_of).round_ties_even()).abs() > 0.000001
    {
        return Some(constraint_error(
            field,
            format!(
                "must be a multiple of {}.",
                json::format_double(multiple_of)
            ),
        ));
    }

    None
}

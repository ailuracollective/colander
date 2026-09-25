//! Constraint evaluation over converted values.

use crate::error::{ColanderError, Result};
use crate::index::AnswerFieldDefinition;
use crate::json::{self, Json};
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
        field_type_names::FILE => return validate_file_constraints(field, value, errors),
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

fn validate_file_constraints(
    field: &AnswerFieldDefinition,
    value: &Val,
    errors: &mut Vec<FormResponseFieldError>,
) -> Result<bool> {
    let values: Vec<(Json, String)> = match value {
        Val::Raw(value) => vec![(json::parse(value).unwrap_or(Json::Null), field.path.clone())],
        Val::List(values) => values
            .iter()
            .enumerate()
            .filter_map(|(index, value)| match value {
                Val::Raw(value) => json::parse(value)
                    .ok()
                    .map(|value| (value, format!("{}/{index}", field.path))),
                _ => None,
            })
            .collect(),
        _ => return Ok(true),
    };
    let max_size = json::get_i64(&field.schema, "maxSize").unwrap_or(0);
    let max_total_size = json::get_i64(&field.schema, "maxTotalSize").unwrap_or(0);
    let max_name_length = json::get_i64(&field.schema, "maxNameLength").unwrap_or(0);
    let accept = json::get_array(&field.schema, "accept")
        .cloned()
        .unwrap_or_default();
    let allowed = accept
        .iter()
        .filter_map(Json::as_str)
        .filter_map(normalize_mime)
        .collect::<std::collections::HashSet<_>>();
    let mut total = 0i64;
    for (value, path) in &values {
        let Some(object) = value.as_object() else {
            continue;
        };
        let size = object.get("size").and_then(Json::as_i64).unwrap_or(0);
        total = total.saturating_add(size);
        if max_size > 0 && size > max_size {
            errors.push(file_constraint_error(
                field,
                &format!("{path}/size"),
                "FILE_SIZE_LIMIT",
                format!("at most {max_size} bytes"),
            ));
            return Ok(false);
        }
        if max_name_length > 0
            && object
                .get("name")
                .and_then(Json::as_str)
                .is_some_and(|name| utf16_len(name) as i64 > max_name_length)
        {
            errors.push(file_constraint_error(
                field,
                &format!("{path}/name"),
                "FILE_CONSTRAINT_VIOLATION",
                format!("at most {max_name_length} UTF-16 code units"),
            ));
            return Ok(false);
        }
        if !allowed.is_empty()
            && object
                .get("contentType")
                .and_then(Json::as_str)
                .is_some_and(|value| !allowed.contains(&normalize_mime(value).unwrap_or_default()))
        {
            errors.push(file_constraint_error(
                field,
                &format!("{path}/contentType"),
                "FILE_MIME_NOT_ALLOWED",
                "an accepted MIME type".to_string(),
            ));
            return Ok(false);
        }
    }
    if max_total_size > 0 && total > max_total_size {
        errors.push(file_constraint_error(
            field,
            &field.path,
            "FILE_TOTAL_SIZE_LIMIT",
            format!("at most {max_total_size} bytes"),
        ));
        return Ok(false);
    }
    Ok(true)
}

fn normalize_mime(value: &str) -> Option<String> {
    let value = value.split(';').next()?.trim().to_ascii_lowercase();
    let (kind, subtype) = value.split_once('/')?;
    if kind.is_empty()
        || subtype.is_empty()
        || !kind.bytes().all(valid_mime_byte)
        || !subtype.bytes().all(valid_mime_byte)
    {
        return None;
    }
    Some(value)
}

fn valid_mime_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"-._+".contains(&byte)
}

fn file_constraint_error(
    field: &AnswerFieldDefinition,
    path: &str,
    code: &str,
    expected: String,
) -> FormResponseFieldError {
    FormResponseFieldError {
        code: code.to_string(),
        path: path.to_string(),
        message: format!("File field '{}' must be {expected}.", field.code),
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
        && ((value / multiple_of) - (value / multiple_of).round_ties_even()).abs()
            > crate::rules::EPSILON
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

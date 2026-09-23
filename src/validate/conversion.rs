//! Answer conversion to the typed value domain.

use std::collections::HashSet;

use crate::error::Result;
use crate::index::AnswerFieldDefinition;
use crate::json::{self, Json};
use crate::keys::{field_type_names, schema_json_keys};
use crate::rules::Val;

use super::datetime::{parse_date, parse_datetime, parse_time};
use super::model::FormResponseFieldError;

pub(super) fn try_convert_value(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
    errors: &mut Vec<FormResponseFieldError>,
) -> Result<Option<Val>> {
    let converted = match field.field_type.as_str() {
        field_type_names::TEXT | field_type_names::TEXTAREA => convert_string(field, value),
        field_type_names::NUMBER => convert_number(field, value),
        field_type_names::INTEGER => convert_integer(field, value),
        field_type_names::BOOLEAN => convert_boolean(field, value),
        field_type_names::DATE => convert_date(field, value),
        field_type_names::DATETIME => convert_datetime(field, value),
        field_type_names::TIME => convert_time(field, value),
        field_type_names::CHOICE => convert_choice(field, value),
        _ => Err(FormResponseFieldError {
            code: "UNSUPPORTED_FIELD_TYPE".to_string(),
            path: field.path.clone(),
            message: format!(
                "Field type '{}' is not supported for answer validation.",
                field.field_type
            ),
        }),
    };

    match converted {
        Ok(converted) => Ok(Some(converted)),
        Err(error) => {
            errors.push(error);
            Ok(None)
        }
    }
}

pub(super) fn type_error(field: &AnswerFieldDefinition, expected: &str) -> FormResponseFieldError {
    FormResponseFieldError {
        code: "INVALID_TYPE".to_string(),
        path: field.path.clone(),
        message: format!("Field '{}' must be {expected}.", field.code),
    }
}

pub(super) fn convert_string(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
) -> std::result::Result<Val, FormResponseFieldError> {
    match value {
        Some(Json::String(text)) => Ok(Val::Str(text.clone())),
        _ => Err(type_error(field, "a string")),
    }
}

pub(super) fn convert_number(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
) -> std::result::Result<Val, FormResponseFieldError> {
    match value {
        Some(Json::Number(_)) => match value.and_then(Json::as_f64) {
            Some(number) => Ok(Val::Double(number)),
            None => Err(type_error(field, "a number")),
        },
        _ => Err(type_error(field, "a number")),
    }
}

pub(super) fn convert_integer(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
) -> std::result::Result<Val, FormResponseFieldError> {
    match value.and_then(Json::as_i64) {
        Some(integer) => Ok(Val::Int(integer)),
        None => Err(type_error(field, "an integer")),
    }
}

pub(super) fn convert_boolean(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
) -> std::result::Result<Val, FormResponseFieldError> {
    match value.and_then(Json::as_bool) {
        Some(boolean) => Ok(Val::Bool(boolean)),
        None => Err(type_error(field, "a boolean")),
    }
}

pub(super) fn convert_date(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
) -> std::result::Result<Val, FormResponseFieldError> {
    let Some(Json::String(text)) = value else {
        return Err(type_error(field, "an ISO date (YYYY-MM-DD)"));
    };
    match parse_date(text) {
        Some(date) => Ok(Val::Str(date)),
        None => Err(type_error(field, "an ISO date (YYYY-MM-DD)")),
    }
}

pub(super) fn convert_datetime(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
) -> std::result::Result<Val, FormResponseFieldError> {
    let Some(Json::String(text)) = value else {
        return Err(type_error(field, "an ISO date-time"));
    };
    match parse_datetime(text) {
        Some(date_time) => Ok(Val::Str(date_time)),
        None => Err(type_error(field, "an ISO date-time")),
    }
}

pub(super) fn convert_time(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
) -> std::result::Result<Val, FormResponseFieldError> {
    let Some(Json::String(text)) = value else {
        return Err(type_error(field, "an ISO time"));
    };
    match parse_time(text) {
        Some(time) => Ok(Val::Str(time)),
        None => Err(type_error(field, "an ISO time")),
    }
}

pub(super) fn convert_choice(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
) -> std::result::Result<Val, FormResponseFieldError> {
    let Some(options) = json::get_array(&field.schema, schema_json_keys::OPTIONS) else {
        return Err(invalid_schema_error(field));
    };
    if options.is_empty() {
        return Err(invalid_schema_error(field));
    }

    let allowed: HashSet<&str> = options
        .iter()
        .filter_map(Json::as_object)
        .filter_map(|option| json::get_str(option, "value"))
        .collect();

    let allow_multiple = json::get_bool(&field.schema, "allowMultiple").unwrap_or(false);
    if allow_multiple {
        convert_multi_choice(field, value, &allowed)
    } else {
        convert_single_choice(field, value, &allowed)
    }
}

pub(super) fn invalid_schema_error(field: &AnswerFieldDefinition) -> FormResponseFieldError {
    FormResponseFieldError {
        code: "INVALID_SCHEMA".to_string(),
        path: field.path.clone(),
        message: format!("Choice field '{}' is missing options.", field.code),
    }
}

pub(super) fn convert_multi_choice(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
    allowed: &HashSet<&str>,
) -> std::result::Result<Val, FormResponseFieldError> {
    let Some(Json::Array(items)) = value else {
        return Err(type_error(field, "an array of choice values"));
    };

    let mut selected = Vec::with_capacity(items.len());
    for item in items {
        let Some(choice) = item.as_str() else {
            return Err(type_error(field, "an array of choice values"));
        };
        if !allowed.contains(choice) {
            return Err(FormResponseFieldError {
                code: "CONSTRAINT_VIOLATION".to_string(),
                path: field.path.clone(),
                message: format!("Field '{}' contains an invalid choice value.", field.code),
            });
        }
        selected.push(Val::Str(choice.to_string()));
    }

    Ok(Val::List(selected))
}

pub(super) fn convert_single_choice(
    field: &AnswerFieldDefinition,
    value: Option<&Json>,
    allowed: &HashSet<&str>,
) -> std::result::Result<Val, FormResponseFieldError> {
    let Some(choice) = value.and_then(Json::as_str) else {
        return Err(type_error(field, "a choice value"));
    };

    if !allowed.contains(choice) {
        return Err(FormResponseFieldError {
            code: "CONSTRAINT_VIOLATION".to_string(),
            path: field.path.clone(),
            message: format!("Field '{}' contains an invalid choice value.", field.code),
        });
    }

    Ok(Val::Str(choice.to_string()))
}

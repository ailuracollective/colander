//! Field indexes over a form schema.
//!
//! Two indexes are built from the same form: one keyed by field id, carrying the
//! `multipleOf`/`decimalPlaces` hints the rule engine needs, and one keyed by
//! field code, carrying the repeater children the response validator walks.

use indexmap::IndexMap;

use crate::error::{ColanderError, Result};
use crate::json::{self, Json, JsonMap};
use crate::keys::{field_type_names, schema_json_keys};

#[derive(Debug, Clone, PartialEq)]
pub struct FieldInfo {
    pub id: String,
    pub code: String,
    pub field_type: String,
    pub required: bool,
    pub read_only: bool,
    /// JSON pointer of the field, e.g. `/fields/0/items/1`.
    pub path: String,
    pub multiple_of: Option<f64>,
    pub decimal_places: Option<i32>,
}

/// Builds the id-keyed index in depth-first document order.
pub fn build_by_id(form_root: &JsonMap) -> Result<IndexMap<String, FieldInfo>> {
    let mut by_id = IndexMap::new();
    if let Some(fields) = json::get_array(form_root, schema_json_keys::FIELDS) {
        index_fields(fields, "/fields", &mut by_id)?;
    }
    Ok(by_id)
}

/// Builds the code-keyed index; a duplicated field code is an error.
pub fn build_by_code(form_root: &JsonMap) -> Result<IndexMap<String, FieldInfo>> {
    let by_id = build_by_id(form_root)?;
    let mut by_code = IndexMap::new();
    for info in by_id.into_values() {
        if by_code.contains_key(&info.code) {
            return Err(ColanderError::new(format!(
                "An item with the same key has already been added. Key: {}",
                info.code
            )));
        }
        by_code.insert(info.code.clone(), info);
    }
    Ok(by_code)
}

fn index_fields(
    fields: &[Json],
    path: &str,
    by_id: &mut IndexMap<String, FieldInfo>,
) -> Result<()> {
    for (index, value) in fields.iter().enumerate() {
        let field = value.as_object().ok_or_else(|| {
            ColanderError::new("The node must be of type 'JsonObject'.".to_string())
        })?;
        let field_path = format!("{path}/{index}");
        let id = require_string(field, schema_json_keys::ID, &field_path)?;
        let code = require_string(field, schema_json_keys::CODE, &field_path)?;
        let field_type = require_string(field, schema_json_keys::TYPE, &field_path)?;

        by_id.insert(
            id.to_string(),
            FieldInfo {
                id: id.to_string(),
                code: code.to_string(),
                field_type: field_type.to_string(),
                required: field_bool(field, "required")?,
                read_only: field_bool(field, "readOnly")?,
                path: field_path.clone(),
                multiple_of: field_double(field, "multipleOf")?,
                decimal_places: field_int(field, "decimalPlaces")?,
            },
        );

        if let Some(items) = json::get_array(field, schema_json_keys::ITEMS) {
            index_fields(items, &format!("{field_path}/items"), by_id)?;
        }
    }
    Ok(())
}

/// An absent or JSON-null key reports the missing-field message; a present key
/// of the wrong type reports a conversion error.
fn require_string<'a>(field: &'a JsonMap, key: &str, path: &str) -> Result<&'a str> {
    match json::get(field, key) {
        None | Some(Json::Null) => Err(ColanderError::new(format!(
            "Expected field {key} at {path}/{key}."
        ))),
        Some(Json::String(text)) => Ok(text),
        Some(other) => Err(type_conversion_error(other, "String")),
    }
}

/// An absent or JSON-null key falls back to `false`; a present key of the wrong
/// type is an error.
fn field_bool(field: &JsonMap, key: &str) -> Result<bool> {
    match json::get(field, key) {
        None | Some(Json::Null) => Ok(false),
        Some(Json::Bool(value)) => Ok(*value),
        Some(other) => Err(type_conversion_error(other, "Boolean")),
    }
}

fn field_double(field: &JsonMap, key: &str) -> Result<Option<f64>> {
    match json::get(field, key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Number(_)) => Ok(json::get_f64(field, key)),
        Some(other) => Err(type_conversion_error(other, "Number")),
    }
}

fn field_int(field: &JsonMap, key: &str) -> Result<Option<i32>> {
    match json::get(field, key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Number(text)) => match text.parse::<i32>() {
            Ok(value) => Ok(Some(value)),
            Err(_) => Err(type_conversion_error(
                json::get(field, key).expect("present"),
                "Integer",
            )),
        },
        Some(other) => Err(type_conversion_error(other, "Integer")),
    }
}

/// Builds the conversion error, naming the value's JSON kind and the target type.
fn type_conversion_error(value: &Json, target: &str) -> ColanderError {
    let kind = match value {
        Json::Null => "Null",
        Json::Bool(true) => "True",
        Json::Bool(false) => "False",
        Json::Number(_) => "Number",
        Json::String(_) => "String",
        Json::Array(_) => "Array",
        Json::Object(_) => "Object",
    };
    ColanderError::new(format!(
        "An element of type '{kind}' cannot be converted to a '{target}'."
    ))
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnswerFieldDefinition {
    pub id: String,
    pub code: String,
    pub field_type: String,
    pub path: String,
    pub required: bool,
    pub read_only: bool,
    /// The raw form field object; constraints are read from here.
    pub schema: JsonMap,
    pub children: Vec<AnswerFieldDefinition>,
}

/// Builds the code-keyed index over non-group fields only; a repeated code
/// overwrites the earlier entry silently.
pub fn build_answer_index(form_root: &JsonMap) -> Result<IndexMap<String, AnswerFieldDefinition>> {
    let mut by_code = IndexMap::new();
    if let Some(fields) = json::get_array(form_root, schema_json_keys::FIELDS) {
        index_answer_fields(fields, "/fields", &mut by_code)?;
    }
    Ok(by_code)
}

fn index_answer_fields(
    fields: &[Json],
    path: &str,
    by_code: &mut IndexMap<String, AnswerFieldDefinition>,
) -> Result<()> {
    for (index, value) in fields.iter().enumerate() {
        let field = value.as_object().ok_or_else(|| {
            ColanderError::new("The node must be of type 'JsonObject'.".to_string())
        })?;
        let field_path = format!("{path}/{index}");
        let id = require_string(field, schema_json_keys::ID, &field_path)?;
        let code = require_string(field, schema_json_keys::CODE, &field_path)?;
        let field_type = require_string(field, schema_json_keys::TYPE, &field_path)?;

        let mut children = Vec::new();
        if let Some(items) = json::get_array(field, schema_json_keys::ITEMS) {
            index_child_fields(items, &format!("{field_path}/items"), &mut children)?;
        }

        let definition = AnswerFieldDefinition {
            id: id.to_string(),
            code: code.to_string(),
            field_type: field_type.to_string(),
            path: field_path.clone(),
            required: field_bool(field, "required")?,
            read_only: field_bool(field, "readOnly")?,
            schema: field.clone(),
            children,
        };

        if field_type != field_type_names::GROUP {
            by_code.insert(code.to_string(), definition);
        }

        if field_type == field_type_names::GROUP
            && let Some(items) = json::get_array(field, schema_json_keys::ITEMS)
        {
            index_answer_fields(items, &format!("{field_path}/items"), by_code)?;
        }
    }
    Ok(())
}

fn index_child_fields(
    items: &[Json],
    path: &str,
    children: &mut Vec<AnswerFieldDefinition>,
) -> Result<()> {
    for (index, value) in items.iter().enumerate() {
        let field = value.as_object().ok_or_else(|| {
            ColanderError::new("The node must be of type 'JsonObject'.".to_string())
        })?;
        let field_path = format!("{path}/{index}");
        let id = require_string(field, schema_json_keys::ID, &field_path)?;
        let code = require_string(field, schema_json_keys::CODE, &field_path)?;
        let field_type = require_string(field, schema_json_keys::TYPE, &field_path)?;

        children.push(AnswerFieldDefinition {
            id: id.to_string(),
            code: code.to_string(),
            field_type: field_type.to_string(),
            path: field_path,
            required: field_bool(field, "required")?,
            read_only: field_bool(field, "readOnly")?,
            schema: field.clone(),
            children: Vec::new(),
        });
    }
    Ok(())
}

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
        index_fields(fields, "/fields", &mut by_id, false)?;
    }
    Ok(by_id)
}

/// Builds the code-keyed index; a duplicated field code is an error.
pub fn build_by_code(form_root: &JsonMap) -> Result<IndexMap<String, FieldInfo>> {
    let by_id = build_by_id(form_root)?;
    build_by_code_from_id(&by_id)
}

/// Derives the code-keyed index from the already-built id-keyed index.
pub(crate) fn build_by_code_from_id(
    fields_by_id: &IndexMap<String, FieldInfo>,
) -> Result<IndexMap<String, FieldInfo>> {
    let mut by_code = IndexMap::new();
    for info in fields_by_id.values() {
        if by_code.contains_key(&info.code) {
            return Err(ColanderError::new(format!(
                "RULE_DUPLICATE_FIELD_CODE: duplicate field code '{}'.",
                info.code
            )));
        }
        by_code.insert(info.code.clone(), info.clone());
    }
    Ok(by_code)
}

/// Derives the legacy, overwrite-on-duplicate code index used by lenient
/// analysis callers that historically did not reject duplicate codes.
pub(crate) fn build_by_code_from_id_unchecked(
    fields_by_id: &IndexMap<String, FieldInfo>,
) -> IndexMap<String, FieldInfo> {
    let mut by_code = IndexMap::new();
    for info in fields_by_id.values() {
        by_code.insert(info.code.clone(), info.clone());
    }
    by_code
}

/// Rejects a duplicated field code on any form, with or without a rules
/// document. The answer index overwrites silently, so entry points that
/// validate answers call this first: two fields sharing a code would
/// otherwise validate against whichever definition came last.
pub fn ensure_unique_codes(form_root: &JsonMap) -> Result<()> {
    build_by_code(form_root).map(|_| ())
}

fn index_fields(
    fields: &[Json],
    path: &str,
    by_id: &mut IndexMap<String, FieldInfo>,
    under_repeater: bool,
) -> Result<()> {
    for (index, value) in fields.iter().enumerate() {
        let field_path = format!("{path}/{index}");
        let field = value.as_object().ok_or_else(|| {
            ColanderError::new(format!("FIELD_NOT_OBJECT: {field_path} must be an object."))
        })?;
        let id = require_string(field, schema_json_keys::ID, &field_path)?;
        let code = require_string(field, schema_json_keys::CODE, &field_path)?;
        let field_type = require_string(field, schema_json_keys::TYPE, &field_path)?;
        validate_file_configuration(field, field_type, &field_path)?;

        if under_repeater && json::get_array(field, schema_json_keys::ITEMS).is_some() {
            return Err(ColanderError::new(format!(
                "REPEATER_NESTED_FIELD: field '{id}' at {field_path} nests items under a repeater; repeater children must be flat scalar fields."
            )));
        }

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
            let nested = under_repeater || field_type == field_type_names::REPEATER;
            index_fields(items, &format!("{field_path}/items"), by_id, nested)?;
        }
    }
    Ok(())
}

/// An absent or JSON-null key reports the missing-field message; a present key
/// of the wrong type reports a conversion error.
fn validate_file_configuration(field: &JsonMap, field_type: &str, path: &str) -> Result<()> {
    let file_keys = ["maxSize", "maxTotalSize", "maxNameLength", "accept"];
    if field_type != field_type_names::FILE {
        if let Some(key) = file_keys.iter().find(|key| field.contains_key(**key)) {
            return Err(ColanderError::new(format!(
                "FILE_INVALID_CONFIG: key '{key}' at {path} is only valid for file fields."
            )));
        }
        return Ok(());
    }

    for key in ["maxSize", "maxTotalSize", "maxNameLength"] {
        if let Some(value) = json::get(field, key) {
            let text = match value {
                Json::Number(text) => text,
                _ => {
                    return Err(ColanderError::new(format!(
                        "FILE_INVALID_CONFIG: key '{key}' at {path} must be a non-negative integer."
                    )));
                }
            };
            if text.parse::<i64>().map_or(true, |value| value < 0) {
                return Err(ColanderError::new(format!(
                    "FILE_INVALID_CONFIG: key '{key}' at {path} must be a non-negative integer."
                )));
            }
        }
    }

    if let Some(value) = json::get(field, "allowMultiple")
        && value.as_bool().is_none()
    {
        return Err(ColanderError::new(format!(
            "FILE_INVALID_CONFIG: key 'allowMultiple' at {path} must be a boolean."
        )));
    }
    let allow_multiple = json::get_bool(field, "allowMultiple").unwrap_or(false);
    if !allow_multiple && field.contains_key("maxTotalSize") {
        return Err(ColanderError::new(format!(
            "FILE_INVALID_CONFIG: maxTotalSize at {path} requires allowMultiple:true."
        )));
    }

    if let Some(value) = json::get(field, "accept") {
        let Some(values) = value.as_array() else {
            return Err(ColanderError::new(format!(
                "FILE_INVALID_CONFIG: key 'accept' at {path} must be an array."
            )));
        };
        for value in values {
            let Some(text) = value.as_str() else {
                return Err(ColanderError::new(format!(
                    "FILE_INVALID_CONFIG: key 'accept' at {path} must contain MIME strings."
                )));
            };
            if !is_mime_type(text) {
                return Err(ColanderError::new(format!(
                    "FILE_INVALID_CONFIG: key 'accept' at {path} contains invalid MIME '{text}'."
                )));
            }
        }
    }

    Ok(())
}

fn is_mime_type(value: &str) -> bool {
    let value = value
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let Some((kind, subtype)) = value.split_once('/') else {
        return false;
    };
    !kind.is_empty()
        && !subtype.is_empty()
        && kind
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._+".contains(&byte))
        && subtype
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._+".contains(&byte))
}

fn require_string<'a>(field: &'a JsonMap, key: &str, path: &str) -> Result<&'a str> {
    match json::get(field, key) {
        None | Some(Json::Null) => Err(ColanderError::new(format!(
            "FIELD_MISSING_KEY: field {key} at {path}/{key} is required."
        ))),
        Some(Json::String(text)) => Ok(text),
        Some(other) => Err(ColanderError::new(format!(
            "FIELD_INVALID_TYPE: {path}/{key} is {}, expected String.",
            json_kind(other)
        ))),
    }
}

/// An absent or JSON-null key falls back to `false`; a present key of the wrong
/// type is an error.
fn field_bool(field: &JsonMap, key: &str) -> Result<bool> {
    match json::get(field, key) {
        None | Some(Json::Null) => Ok(false),
        Some(Json::Bool(value)) => Ok(*value),
        Some(other) => Err(ColanderError::new(format!(
            "FIELD_INVALID_TYPE: key '{key}' is {}, expected Boolean.",
            json_kind(other)
        ))),
    }
}

fn field_double(field: &JsonMap, key: &str) -> Result<Option<f64>> {
    match json::get(field, key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Number(_)) => Ok(json::get_f64(field, key)),
        Some(other) => Err(ColanderError::new(format!(
            "FIELD_INVALID_TYPE: key '{key}' is {}, expected Number.",
            json_kind(other)
        ))),
    }
}

fn field_int(field: &JsonMap, key: &str) -> Result<Option<i32>> {
    match json::get(field, key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Number(text)) => match text.parse::<i32>() {
            Ok(value) => Ok(Some(value)),
            Err(_) => Err(ColanderError::new(format!(
                "FIELD_INVALID_TYPE: key '{key}' is Number, expected an i32 Integer."
            ))),
        },
        Some(other) => Err(ColanderError::new(format!(
            "FIELD_INVALID_TYPE: key '{key}' is {}, expected Integer.",
            json_kind(other)
        ))),
    }
}

/// The JSON kind of a value, named for an error message.
fn json_kind(value: &Json) -> &'static str {
    match value {
        Json::Null => "Null",
        Json::Bool(true) => "True",
        Json::Bool(false) => "False",
        Json::Number(_) => "Number",
        Json::String(_) => "String",
        Json::Array(_) => "Array",
        Json::Object(_) => "Object",
    }
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
        let field_path = format!("{path}/{index}");
        let field = value.as_object().ok_or_else(|| {
            ColanderError::new(format!("FIELD_NOT_OBJECT: {field_path} must be an object."))
        })?;
        let id = require_string(field, schema_json_keys::ID, &field_path)?;
        let code = require_string(field, schema_json_keys::CODE, &field_path)?;
        let field_type = require_string(field, schema_json_keys::TYPE, &field_path)?;
        validate_file_configuration(field, field_type, &field_path)?;

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
        let field_path = format!("{path}/{index}");
        let field = value.as_object().ok_or_else(|| {
            ColanderError::new(format!("FIELD_NOT_OBJECT: {field_path} must be an object."))
        })?;
        let id = require_string(field, schema_json_keys::ID, &field_path)?;
        let code = require_string(field, schema_json_keys::CODE, &field_path)?;
        let field_type = require_string(field, schema_json_keys::TYPE, &field_path)?;
        validate_file_configuration(field, field_type, &field_path)?;

        if json::get_array(field, schema_json_keys::ITEMS).is_some() {
            return Err(ColanderError::new(format!(
                "REPEATER_NESTED_FIELD: field '{id}' at {field_path} nests items under a repeater; repeater children must be flat scalar fields."
            )));
        }

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

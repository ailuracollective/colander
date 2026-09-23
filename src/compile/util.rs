//! Small accessors over JSON nodes.

use crate::error::{ColanderError, Result};
use crate::json::{self, Json, JsonMap};
use crate::keys::schema_json_keys;

/// Deep-clone every key except `items`, which `compile_field` handles itself.
pub(super) fn clone_field_shell(field: &JsonMap) -> JsonMap {
    let mut clone = JsonMap::new();
    for (key, value) in field {
        if key == schema_json_keys::ITEMS {
            continue;
        }
        clone.insert(key.clone(), value.clone());
    }
    clone
}

pub(super) fn copy_if_present(source: &JsonMap, target: &mut JsonMap, key: &str) {
    if let Some(value) = present(source, key) {
        target.insert(key.to_string(), value.clone());
    }
}

pub(super) fn present<'a>(source: &'a JsonMap, key: &str) -> Option<&'a Json> {
    json::get(source, key).filter(|value| !value.is_null())
}

pub(super) fn clone_or_null(source: &JsonMap, key: &str) -> Json {
    json::get(source, key).cloned().unwrap_or(Json::Null)
}

pub(super) fn require_array<'a>(node: Option<&'a Json>, path: &str) -> Result<&'a Vec<Json>> {
    node.and_then(Json::as_array)
        .ok_or_else(|| ColanderError::new(format!("Expected array at {path}.")))
}

pub(super) fn require_string<'a>(node: Option<&'a Json>, path: &str) -> Result<&'a str> {
    match node.and_then(Json::as_str) {
        Some(value) if !value.is_empty() => Ok(value),
        _ => Err(ColanderError::new(format!(
            "Expected non-empty string at {path}."
        ))),
    }
}

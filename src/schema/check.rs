//! Recursive descent over a schema instance pair.

use crate::json::{self, Json};

use super::keywords::{
    check_array_keywords, check_combinators, check_enum_and_const, check_numeric_keywords,
    check_object_keywords, check_string_keywords,
};
use super::model::SchemaError;

use super::keywords::check_type;

/// Evaluate `instance` against `schema`. `root` anchors local `$ref`s.
///
/// The schema tree under `root` is classified first (unsupported keywords and
/// wrong-typed values), so a malformed schema fails even when the instance
/// never reaches the offending subschema.
pub fn check(schema: &Json, instance: &Json, root: &Json) -> Vec<SchemaError> {
    let mut errors = super::classify::classify(root);
    check_into(schema, instance, root, &mut errors);
    errors
}

/// Instance evaluation alone, without re-classifying the schema. Internal
/// combinator probes use this so a multi-branch schema is classified once,
/// when `check` runs.
pub(super) fn evaluate(schema: &Json, instance: &Json, root: &Json) -> Vec<SchemaError> {
    let mut errors = Vec::new();
    check_into(schema, instance, root, &mut errors);
    errors
}

pub(super) fn check_into(
    schema: &Json,
    instance: &Json,
    root: &Json,
    errors: &mut Vec<SchemaError>,
) {
    let Json::Object(schema) = schema else {
        // Boolean schemas are legal in 2020-12.
        if matches!(schema, Json::Bool(false)) {
            errors.push(SchemaError {
                keyword: "false".to_string(),
                message: "no value is valid against this schema".to_string(),
            });
        }
        return;
    };

    if let Some(reference) = json::get_str(schema, "$ref") {
        match resolve_ref(reference, root) {
            Some(target) => check_into(target, instance, root, errors),
            None => errors.push(SchemaError {
                keyword: "$ref".to_string(),
                message: format!("cannot resolve reference '{reference}'"),
            }),
        }
    }

    check_type(schema, instance, errors);
    check_enum_and_const(schema, instance, errors);
    check_combinators(schema, instance, root, errors);
    check_object_keywords(schema, instance, root, errors);
    check_array_keywords(schema, instance, root, errors);
    check_string_keywords(schema, instance, errors);
    check_numeric_keywords(schema, instance, errors);
}

pub(super) fn resolve_ref<'a>(reference: &str, root: &'a Json) -> Option<&'a Json> {
    let pointer = reference.strip_prefix('#')?;
    if pointer.is_empty() {
        return Some(root);
    }
    let pointer = pointer.strip_prefix('/')?;

    let mut current = root;
    for raw in pointer.split('/') {
        let token = raw.replace("~1", "/").replace("~0", "~");
        current = match current {
            Json::Object(map) => map.get(&token)?,
            Json::Array(items) => items.get(token.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

/// JSON value equality with numeric-value semantics (`1` equals `1.0`).
pub fn value_equal(left: &Json, right: &Json) -> bool {
    match (left, right) {
        (Json::Number(_), Json::Number(_)) => match (left.as_f64(), right.as_f64()) {
            (Some(left), Some(right)) => left == right,
            _ => left.as_number_text() == right.as_number_text(),
        },
        (Json::Array(left), Json::Array(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(l, r)| value_equal(l, r))
        }
        (Json::Object(left), Json::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right
                        .get(key)
                        .is_some_and(|other| value_equal(value, other))
                })
        }
        _ => left == right,
    }
}

/// A hash consistent with [`value_equal`]: equal values hash equal, so a
/// hash bucket can stand in for the pairwise comparison. Numbers hash by
/// their `f64` value (so `1`, `1.0` and `1e0` share a bucket) and fall back to
/// their literal text when they do not parse. Object keys are visited in
/// sorted order so key order does not change the hash.
pub fn value_hash(node: &Json) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn write(node: &Json, hasher: &mut DefaultHasher) {
        match node {
            Json::Null => 0u8.hash(hasher),
            Json::Bool(value) => {
                1u8.hash(hasher);
                value.hash(hasher);
            }
            Json::Number(_) => {
                2u8.hash(hasher);
                match node.as_f64() {
                    Some(number) => number.to_bits().hash(hasher),
                    None => node.as_number_text().hash(hasher),
                }
            }
            Json::String(text) => {
                3u8.hash(hasher);
                text.hash(hasher);
            }
            Json::Array(items) => {
                4u8.hash(hasher);
                items.len().hash(hasher);
                for item in items {
                    write(item, hasher);
                }
            }
            Json::Object(map) => {
                5u8.hash(hasher);
                map.len().hash(hasher);
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort_unstable();
                for key in keys {
                    key.hash(hasher);
                    write(&map[key], hasher);
                }
            }
        }
    }

    let mut hasher = DefaultHasher::new();
    write(node, &mut hasher);
    hasher.finish()
}

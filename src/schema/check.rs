//! Recursive descent over a schema instance pair.

use crate::json::{self, Json};

use super::keywords::{
    check_array_keywords, check_combinators, check_enum_and_const, check_numeric_keywords,
    check_object_keywords, check_string_keywords,
};
use super::model::SchemaError;

use super::keywords::check_type;

/// Evaluate `instance` against `schema`. `root` anchors local `$ref`s.
pub fn check(schema: &Json, instance: &Json, root: &Json) -> Vec<SchemaError> {
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

fn resolve_ref<'a>(reference: &str, root: &'a Json) -> Option<&'a Json> {
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

//! Structural well-formedness of a schema document (SPEC S-3, S-4).
//!
//! Evaluation only descends into a subschema when the instance reaches it, so
//! a keyword that is unsupported or wrongly typed could hide behind an absent
//! property and never be noticed. This walk is independent of the instance: it
//! visits every subschema position and every `$ref` target, classifies each
//! keyword as implemented, annotation-only or unsupported, and checks the JSON
//! type each implemented keyword expects.
//!
//! The rule is open: a keyword is unsupported when it is neither implemented
//! nor annotation-only, so a keyword that is not on any list is still caught.

use std::collections::HashSet;

use crate::json::Json;

use super::check::resolve_ref;
use super::model::SchemaError;

/// Keywords that assert something the implemented subset supports.
const IMPLEMENTED: &[&str] = &[
    "type",
    "enum",
    "const",
    "allOf",
    "anyOf",
    "oneOf",
    "not",
    "if",
    "then",
    "else",
    "properties",
    "required",
    "additionalProperties",
    "minProperties",
    "maxProperties",
    "items",
    "minItems",
    "maxItems",
    "uniqueItems",
    "minLength",
    "maxLength",
    "pattern",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "multipleOf",
    "format",
    "$ref",
];

/// Keywords that only describe a schema. They never assert, so they stay
/// ignored, whatever their value.
const ANNOTATION_ONLY: &[&str] = &[
    "title",
    "description",
    "default",
    "examples",
    "deprecated",
    "$comment",
    "$id",
    "$schema",
    "$defs",
    "$anchor",
    "readOnly",
    "writeOnly",
];

/// Walk `root` and report every unsupported keyword and every wrong-typed
/// implemented keyword. `root` anchors local `$ref` resolution.
pub(super) fn classify(root: &Json) -> Vec<SchemaError> {
    let mut errors = Vec::new();
    if !is_schema(root) {
        errors.push(SchemaError {
            keyword: "schema".to_string(),
            message: "a schema must be an object or a boolean".to_string(),
        });
        return errors;
    }
    let mut seen = HashSet::new();
    // Nodes currently on the walk stack. A `$ref` that resolves to one of
    // them is recursive (S-10).
    let mut ancestors: Vec<usize> = Vec::new();
    walk(root, root, &mut errors, &mut seen, &mut ancestors);
    errors
}

fn is_schema(value: &Json) -> bool {
    matches!(value, Json::Object(_) | Json::Bool(_))
}

/// Visit one schema node, skipping a node already visited. `root` is the
/// anchor for every `$ref`; `ref_stack` is the chain of `$ref` targets
/// currently being resolved, which is how a recursive reference is detected
/// (SPEC S-10).
fn walk(
    schema: &Json,
    root: &Json,
    errors: &mut Vec<SchemaError>,
    seen: &mut HashSet<usize>,
    ancestors: &mut Vec<usize>,
) {
    let Json::Object(map) = schema else {
        // A boolean schema has no keywords.
        return;
    };
    let address = schema as *const Json as usize;
    if !seen.insert(address) {
        return;
    }
    ancestors.push(address);
    for (keyword, value) in map {
        visit(keyword, value, root, errors, seen, ancestors);
    }
    ancestors.pop();
}

/// Classify one keyword and recurse into the subschema positions it owns.
fn visit(
    keyword: &str,
    value: &Json,
    root: &Json,
    errors: &mut Vec<SchemaError>,
    seen: &mut HashSet<usize>,
    ancestors: &mut Vec<usize>,
) {
    if ANNOTATION_ONLY.contains(&keyword) {
        return;
    }
    if !IMPLEMENTED.contains(&keyword) {
        errors.push(SchemaError {
            keyword: keyword.to_string(),
            message: "unsupported keyword; the implemented subset does not assert it".to_string(),
        });
        return;
    }

    match keyword {
        "const" => {}
        "enum" => {
            if !matches!(value, Json::Array(_)) {
                errors.push(wrong_type(keyword, "an array"));
            }
        }
        "type" => check_type_keyword(value, errors),
        "required" => check_required(value, errors),
        "minProperties" | "maxProperties" | "minItems" | "maxItems" | "minLength" | "maxLength" => {
            check_integer(keyword, value, errors)
        }
        "uniqueItems" => check_boolean(keyword, value, errors),
        "format" => check_format(value, errors),
        "pattern" => check_pattern(value, errors),
        "minimum" | "maximum" | "exclusiveMinimum" | "exclusiveMaximum" | "multipleOf" => {
            check_number(keyword, value, errors)
        }
        "not" | "if" | "then" | "else" | "items" => {
            walk_subschema(keyword, value, root, errors, seen, ancestors)
        }
        "additionalProperties" => {
            if matches!(value, Json::Bool(_)) {
                return;
            }
            walk_subschema(keyword, value, root, errors, seen, ancestors);
        }
        "properties" => {
            let Some(properties) = expect_object(keyword, value, errors) else {
                return;
            };
            for sub_schema in properties.values() {
                walk_subschema(keyword, sub_schema, root, errors, seen, ancestors);
            }
        }
        "allOf" | "anyOf" | "oneOf" => {
            let Some(branches) = expect_array(keyword, value, errors) else {
                return;
            };
            for branch in branches {
                walk_subschema(keyword, branch, root, errors, seen, ancestors);
            }
        }
        "$ref" => {
            let Some(reference) = value.as_str() else {
                errors.push(wrong_type(keyword, "string"));
                return;
            };
            // A `$defs` entry is annotation-only until a `$ref` reaches it; the
            // target is walked here so a nested keyword is still classified.
            if let Some(target) = resolve_ref(reference, root) {
                let address = target as *const Json as usize;
                if ancestors.contains(&address) {
                    errors.push(SchemaError {
                        keyword: keyword.to_string(),
                        message: format!("recursive reference '{reference}' is not supported"),
                    });
                    return;
                }
                // S-10: a `$ref` chain of distinct definitions is not a cycle,
                // but it still recurses once per level, and a deep enough one
                // exhausts the stack before instance evaluation ever starts.
                // `ancestors` is the live walk depth, so its length is the guard.
                if ancestors.len() >= super::MAX_SCHEMA_DEPTH {
                    errors.push(SchemaError {
                        keyword: "schema".to_string(),
                        message: super::SCHEMA_DEPTH_LIMIT_MESSAGE.to_string(),
                    });
                    return;
                }
                walk_subschema(keyword, target, root, errors, seen, ancestors);
            }
        }
        _ => unreachable!("IMPLEMENTED and the match arms move together"),
    }
}

fn walk_subschema(
    keyword: &str,
    value: &Json,
    root: &Json,
    errors: &mut Vec<SchemaError>,
    seen: &mut HashSet<usize>,
    ancestors: &mut Vec<usize>,
) {
    if is_schema(value) {
        walk(value, root, errors, seen, ancestors);
    } else {
        errors.push(wrong_type(keyword, "a schema (object or boolean)"));
    }
}

fn check_type_keyword(value: &Json, errors: &mut Vec<SchemaError>) {
    let names: Vec<&str> = match value {
        Json::String(name) => vec![name.as_str()],
        Json::Array(names) => {
            if names.iter().any(|name| !matches!(name, Json::String(_))) {
                errors.push(wrong_type("type", "an array of strings"));
                return;
            }
            names.iter().filter_map(Json::as_str).collect()
        }
        _ => {
            errors.push(wrong_type("type", "a string or an array of strings"));
            return;
        }
    };
    // A mistyped name is a schema error, not an assertion nothing can pass:
    // `{"type":"strnig"}` must fail the schema, not every instance.
    for name in names {
        if !super::keywords::KNOWN_TYPES.contains(&name) {
            errors.push(SchemaError {
                keyword: "type".to_string(),
                message: format!("unknown type '{name}'"),
            });
        }
    }
}

fn check_required(value: &Json, errors: &mut Vec<SchemaError>) {
    let names = matches!(value, Json::Array(items)
        if items.iter().all(|item| matches!(item, Json::String(_))));
    if !names {
        errors.push(wrong_type("required", "an array of strings"));
    }
}

fn check_integer(keyword: &str, value: &Json, errors: &mut Vec<SchemaError>) {
    if value.as_i64().is_none() {
        errors.push(wrong_type(keyword, "an integer"));
    }
}

fn check_number(keyword: &str, value: &Json, errors: &mut Vec<SchemaError>) {
    if value.as_f64().is_none() {
        errors.push(wrong_type(keyword, "a number"));
    }
}

fn check_boolean(keyword: &str, value: &Json, errors: &mut Vec<SchemaError>) {
    if value.as_bool().is_none() {
        errors.push(wrong_type(keyword, "a boolean"));
    }
}

/// S-6: a `pattern` that cannot be compiled is an error. This walk runs over
/// every subschema whether or not the instance reaches it, so a pattern behind
/// an `anyOf` branch cannot pass by never being compiled.
fn check_pattern(value: &Json, errors: &mut Vec<SchemaError>) {
    let Some(pattern) = value.as_str() else {
        errors.push(wrong_type("pattern", "a string"));
        return;
    };
    if let Err(message) = crate::pattern::compile(pattern) {
        errors.push(SchemaError {
            keyword: "pattern".to_string(),
            message,
        });
    }
}

/// S-5: a `format` must belong to the closed asserted set. This walk runs over
/// every subschema whether or not the instance reaches it, so a mistyped format
/// name behind an `anyOf` branch is still an error, not a silent pass.
fn check_format(value: &Json, errors: &mut Vec<SchemaError>) {
    let Some(name) = value.as_str() else {
        errors.push(wrong_type("format", "a string"));
        return;
    };
    if !super::format::is_known(name) {
        errors.push(SchemaError {
            keyword: "format".to_string(),
            message: format!("unknown format '{name}'"),
        });
    }
}

fn expect_object<'a>(
    keyword: &str,
    value: &'a Json,
    errors: &mut Vec<SchemaError>,
) -> Option<&'a crate::json::JsonMap> {
    match value.as_object() {
        Some(map) => Some(map),
        None => {
            errors.push(wrong_type(keyword, "an object"));
            None
        }
    }
}

fn expect_array<'a>(
    keyword: &str,
    value: &'a Json,
    errors: &mut Vec<SchemaError>,
) -> Option<&'a Vec<Json>> {
    match value.as_array() {
        Some(items) => Some(items),
        None => {
            errors.push(wrong_type(keyword, "an array"));
            None
        }
    }
}

fn wrong_type(keyword: &str, expected: &str) -> SchemaError {
    SchemaError {
        keyword: keyword.to_string(),
        message: format!("keyword value must be {expected}"),
    }
}

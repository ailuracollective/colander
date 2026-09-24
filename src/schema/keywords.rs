//! The JSON Schema keyword subset colander supports.

use crate::json::{self, Json, JsonMap};

use super::check::{check_into, evaluate, value_equal};
use super::model::SchemaError;

/// The closed set of `type` names. The structural classifier rejects anything
/// else as a schema error, so a typo can never become an assertion that every
/// instance fails.
pub const KNOWN_TYPES: &[&str] = &[
    "object", "array", "string", "boolean", "null", "number", "integer",
];

pub(super) fn type_name(value: &Json) -> &'static str {
    match value {
        Json::Null => "null",
        Json::Bool(_) => "boolean",
        Json::Number(_) => "number",
        Json::String(_) => "string",
        Json::Array(_) => "array",
        Json::Object(_) => "object",
    }
}

pub(super) fn matches_type(expected: &str, instance: &Json) -> bool {
    match expected {
        "object" => matches!(instance, Json::Object(_)),
        "array" => matches!(instance, Json::Array(_)),
        "string" => matches!(instance, Json::String(_)),
        "boolean" => matches!(instance, Json::Bool(_)),
        "null" => matches!(instance, Json::Null),
        "number" => matches!(instance, Json::Number(_)),
        "integer" => match instance {
            Json::Number(_) => instance
                .as_f64()
                .is_some_and(|number| number.fract() == 0.0),
            _ => false,
        },
        _ => false,
    }
}

pub(super) fn check_type(schema: &JsonMap, instance: &Json, errors: &mut Vec<SchemaError>) {
    let Some(expected) = json::get(schema, "type") else {
        return;
    };

    let matched = match expected {
        Json::String(name) => matches_type(name, instance),
        Json::Array(names) => names
            .iter()
            .filter_map(Json::as_str)
            .any(|name| matches_type(name, instance)),
        _ => true,
    };

    if !matched {
        errors.push(SchemaError {
            keyword: "type".to_string(),
            message: format!(
                "expected {} but found {}",
                describe_types(expected),
                type_name(instance)
            ),
        });
    }
}

pub(super) fn describe_types(expected: &Json) -> String {
    match expected {
        Json::String(name) => name.clone(),
        Json::Array(names) => names
            .iter()
            .filter_map(Json::as_str)
            .collect::<Vec<_>>()
            .join(" or "),
        _ => "a valid type".to_string(),
    }
}

pub(super) fn check_enum_and_const(
    schema: &JsonMap,
    instance: &Json,
    errors: &mut Vec<SchemaError>,
) {
    if let Some(allowed) = json::get_array(schema, "enum")
        && !allowed.iter().any(|item| value_equal(item, instance))
    {
        errors.push(SchemaError {
            keyword: "enum".to_string(),
            message: "value is not one of the allowed values".to_string(),
        });
    }

    if let Some(expected) = json::get(schema, "const")
        && !value_equal(expected, instance)
    {
        errors.push(SchemaError {
            keyword: "const".to_string(),
            message: format!("value must be {}", json::ordered(expected)),
        });
    }
}

pub(super) fn check_combinators(
    schema: &JsonMap,
    instance: &Json,
    root: &Json,
    errors: &mut Vec<SchemaError>,
) {
    if let Some(all_of) = json::get_array(schema, "allOf") {
        for sub_schema in all_of {
            check_into(sub_schema, instance, root, errors);
        }
    }

    if let Some(any_of) = json::get_array(schema, "anyOf")
        && !any_of
            .iter()
            .any(|sub| evaluate(sub, instance, root).is_empty())
    {
        errors.push(SchemaError {
            keyword: "anyOf".to_string(),
            message: "value does not match any of the listed schemas".to_string(),
        });
    }

    if let Some(one_of) = json::get_array(schema, "oneOf") {
        let matches = one_of
            .iter()
            .filter(|sub| evaluate(sub, instance, root).is_empty())
            .count();
        if matches != 1 {
            errors.push(SchemaError {
                keyword: "oneOf".to_string(),
                message: format!(
                    "value matches {matches} of the listed schemas, expected exactly 1"
                ),
            });
        }
    }

    if let Some(negated) = json::get(schema, "not")
        && evaluate(negated, instance, root).is_empty()
    {
        errors.push(SchemaError {
            keyword: "not".to_string(),
            message: "value must not match the negated schema".to_string(),
        });
    }

    if let Some(condition) = json::get(schema, "if") {
        if evaluate(condition, instance, root).is_empty() {
            if let Some(then_schema) = json::get(schema, "then") {
                check_into(then_schema, instance, root, errors);
            }
        } else if let Some(else_schema) = json::get(schema, "else") {
            check_into(else_schema, instance, root, errors);
        }
    }
}

pub(super) fn check_object_keywords(
    schema: &JsonMap,
    instance: &Json,
    root: &Json,
    errors: &mut Vec<SchemaError>,
) {
    let Json::Object(object) = instance else {
        return;
    };

    let properties = json::get_object(schema, "properties");
    if let Some(properties) = properties {
        for (name, sub_schema) in properties {
            if let Some(value) = object.get(name) {
                check_into(sub_schema, value, root, errors);
            }
        }
    }

    if let Some(required) = json::get_array(schema, "required") {
        for name in required.iter().filter_map(Json::as_str) {
            if !object.contains_key(name) {
                errors.push(SchemaError {
                    keyword: "required".to_string(),
                    message: format!("required property '{name}' is missing"),
                });
            }
        }
    }

    match schema.get("additionalProperties") {
        Some(Json::Bool(false)) => {
            let allowed: Vec<&str> = properties
                .map(|map| map.keys().map(String::as_str).collect())
                .unwrap_or_default();
            for name in object.keys() {
                if !allowed.contains(&name.as_str()) {
                    errors.push(SchemaError {
                        keyword: "additionalProperties".to_string(),
                        message: format!("additional property '{name}' is not allowed"),
                    });
                }
            }
        }
        Some(sub_schema @ Json::Object(_)) => {
            let allowed: Vec<&str> = properties
                .map(|map| map.keys().map(String::as_str).collect())
                .unwrap_or_default();
            for (name, value) in object {
                if !allowed.contains(&name.as_str()) {
                    check_into(sub_schema, value, root, errors);
                }
            }
        }
        _ => {}
    }

    if let Some(min_properties) = json::get_i64(schema, "minProperties")
        && (object.len() as i64) < min_properties
    {
        errors.push(SchemaError {
            keyword: "minProperties".to_string(),
            message: format!("object must have at least {min_properties} properties"),
        });
    }
    if let Some(max_properties) = json::get_i64(schema, "maxProperties")
        && (object.len() as i64) > max_properties
    {
        errors.push(SchemaError {
            keyword: "maxProperties".to_string(),
            message: format!("object must have at most {max_properties} properties"),
        });
    }
}

pub(super) fn check_array_keywords(
    schema: &JsonMap,
    instance: &Json,
    root: &Json,
    errors: &mut Vec<SchemaError>,
) {
    let Json::Array(items) = instance else {
        return;
    };

    if let Some(item_schema) = json::get(schema, "items") {
        for item in items {
            check_into(item_schema, item, root, errors);
        }
    }

    if let Some(min_items) = json::get_i64(schema, "minItems")
        && (items.len() as i64) < min_items
    {
        errors.push(SchemaError {
            keyword: "minItems".to_string(),
            message: format!("array must have at least {min_items} items"),
        });
    }
    if let Some(max_items) = json::get_i64(schema, "maxItems")
        && (items.len() as i64) > max_items
    {
        errors.push(SchemaError {
            keyword: "maxItems".to_string(),
            message: format!("array must have at most {max_items} items"),
        });
    }

    if json::get_bool(schema, "uniqueItems").unwrap_or(false) {
        for (index, item) in items.iter().enumerate() {
            if items[..index].iter().any(|other| value_equal(other, item)) {
                errors.push(SchemaError {
                    keyword: "uniqueItems".to_string(),
                    message: format!("array items must be unique (index {index} repeats)"),
                });
                break;
            }
        }
    }
}

pub(super) fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

pub(super) fn check_string_keywords(
    schema: &JsonMap,
    instance: &Json,
    errors: &mut Vec<SchemaError>,
) {
    let Json::String(text) = instance else {
        return;
    };
    let length = utf16_len(text);

    if let Some(min_length) = json::get_i64(schema, "minLength")
        && (length as i64) < min_length
    {
        errors.push(SchemaError {
            keyword: "minLength".to_string(),
            message: format!("string must be at least {min_length} characters"),
        });
    }
    if let Some(max_length) = json::get_i64(schema, "maxLength")
        && (length as i64) > max_length
    {
        errors.push(SchemaError {
            keyword: "maxLength".to_string(),
            message: format!("string must be at most {max_length} characters"),
        });
    }
    if let Some(pattern) = json::get_str(schema, "pattern") {
        // A pattern that cannot be compiled is reported once by the structural
        // classifier (S-6). Here only the match outcome remains: a match, a
        // non-match, or the engine's runtime failure (the backtracking limit).
        if let Ok(regex) = crate::pattern::compile(pattern) {
            match regex.is_match(text) {
                Ok(true) => {}
                Ok(false) => errors.push(SchemaError {
                    keyword: "pattern".to_string(),
                    message: format!("string does not match the pattern '{pattern}'"),
                }),
                Err(error) => errors.push(SchemaError {
                    keyword: "pattern".to_string(),
                    message: format!("pattern '{pattern}' could not be evaluated: {error}"),
                }),
            }
        }
    }
    // `format` asserts the closed set in `super::format`. A name the classifier
    // rejects is reported there, so here only a known name asserts and an unknown
    // one is skipped rather than reported twice.
    if let Some(name) = json::get_str(schema, "format")
        && super::format::is_known(name)
        && !super::format::matches(name, text)
    {
        errors.push(SchemaError {
            keyword: "format".to_string(),
            message: format!("string is not a valid {name}"),
        });
    }
}

pub(super) fn check_numeric_keywords(
    schema: &JsonMap,
    instance: &Json,
    errors: &mut Vec<SchemaError>,
) {
    let Some(number) = instance.as_f64() else {
        return;
    };

    if let Some(minimum) = json::get_f64(schema, "minimum")
        && number < minimum
    {
        errors.push(SchemaError {
            keyword: "minimum".to_string(),
            message: format!("number must be >= {}", json::format_double(minimum)),
        });
    }
    if let Some(maximum) = json::get_f64(schema, "maximum")
        && number > maximum
    {
        errors.push(SchemaError {
            keyword: "maximum".to_string(),
            message: format!("number must be <= {}", json::format_double(maximum)),
        });
    }
    if let Some(exclusive_minimum) = json::get_f64(schema, "exclusiveMinimum")
        && number <= exclusive_minimum
    {
        errors.push(SchemaError {
            keyword: "exclusiveMinimum".to_string(),
            message: format!(
                "number must be > {}",
                json::format_double(exclusive_minimum)
            ),
        });
    }
    if let Some(exclusive_maximum) = json::get_f64(schema, "exclusiveMaximum")
        && number >= exclusive_maximum
    {
        errors.push(SchemaError {
            keyword: "exclusiveMaximum".to_string(),
            message: format!(
                "number must be < {}",
                json::format_double(exclusive_maximum)
            ),
        });
    }
    if let Some(multiple_of) = json::get_f64(schema, "multipleOf")
        && multiple_of != 0.0
        && ((number / multiple_of) - (number / multiple_of).round_ties_even()).abs()
            > crate::rules::EPSILON
    {
        errors.push(SchemaError {
            keyword: "multipleOf".to_string(),
            message: format!(
                "number must be a multiple of {}",
                json::format_double(multiple_of)
            ),
        });
    }
}

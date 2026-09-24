//! Value representation shared by the rule engine and the response validator.

use indexmap::IndexMap;

use crate::error::{ColanderError, Result};
use crate::json::{self, Json, JsonMap};

/// The value domain the rule engine operates on.
///
/// `Raw` holds a nested JSON object from a literal, kept in its serialized form
/// rather than decomposed.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Null,
    Bool(bool),
    Str(String),
    Int(i64),
    Double(f64),
    List(Vec<Val>),
    Rows(Vec<IndexMap<String, Val>>),
    Raw(String),
}

/// Absolute tolerance for all double comparisons: calculated-value
/// mismatch, `multipleOf` quotients (here and in the response validator and
/// the JSON Schema subset share this constant).
pub const EPSILON: f64 = 0.000001;

impl Val {
    /// `null`, non-finite numbers, empty strings, lists and rows are empty;
    /// booleans, integers and `Raw` never are.
    pub fn is_empty(&self) -> bool {
        match self {
            Val::Null => true,
            Val::Double(number) => !number.is_finite(),
            Val::Str(text) => text.is_empty(),
            Val::List(items) => items.is_empty(),
            Val::Rows(rows) => rows.is_empty(),
            Val::Bool(_) | Val::Int(_) | Val::Raw(_) => false,
        }
    }

    /// `null` is `false`, non-empty strings are `true`, lists and rows are
    /// always `true`, and numbers are `true` when non-zero.
    pub fn to_bool(&self) -> bool {
        match self {
            Val::Bool(value) => *value,
            Val::Null => false,
            Val::Str(text) => !text.is_empty(),
            Val::Double(number) => *number != 0.0,
            Val::Int(number) => *number != 0,
            Val::List(_) | Val::Rows(_) | Val::Raw(_) => true,
        }
    }

    /// Converts to `f64`: `null` becomes `0.0`, `true`/`false` become
    /// `1.0`/`0.0`, and strings are parsed; lists, rows and `Raw` are an error.
    pub fn to_double(&self) -> Result<f64> {
        match self {
            Val::Null => Ok(0.0),
            Val::Double(number) => Ok(*number),
            Val::Int(number) => Ok(*number as f64),
            Val::Bool(value) => Ok(if *value { 1.0 } else { 0.0 }),
            Val::Str(text) => parse_invariant_double(text),
            Val::List(_) | Val::Rows(_) | Val::Raw(_) => Err(ColanderError::new(format!(
                "Cannot convert '{}' to a number.",
                self.describe()
            ))),
        }
    }

    fn describe(&self) -> String {
        match self {
            Val::Null => "null".to_string(),
            Val::Bool(value) => value.to_string(),
            Val::Str(text) => text.clone(),
            Val::Int(number) => number.to_string(),
            Val::Double(number) => crate::json::format_double(*number),
            Val::List(_) => "list".to_string(),
            Val::Rows(_) => "list".to_string(),
            Val::Raw(text) => text.clone(),
        }
    }

    /// Conversion used for `lit` nodes and for caller-supplied value
    /// dictionaries; nested objects become `Raw`.
    pub fn from_json_node(node: &Json) -> Val {
        match node {
            Json::Null => Val::Null,
            Json::Bool(value) => Val::Bool(*value),
            Json::String(text) => Val::Str(text.clone()),
            Json::Number(_) => match node.as_i64() {
                Some(value) => Val::Int(value),
                None => Val::Double(node.as_f64().unwrap_or(0.0)),
            },
            Json::Array(items) => Val::List(items.iter().map(Val::from_json_node).collect()),
            Json::Object(_) => Val::Raw(json::ordered(node)),
        }
    }

    /// Submitted answers: nested arrays become lists (SPEC E-8); nested objects
    /// are rejected but never abort the call (SPEC E-9).
    pub fn from_json_element(node: &Json) -> Result<Val> {
        match node {
            Json::Null => Ok(Val::Null),
            Json::Bool(value) => Ok(Val::Bool(*value)),
            Json::String(text) => Ok(Val::Str(text.clone())),
            Json::Number(_) => Ok(match node.as_i64() {
                Some(value) => Val::Int(value),
                None => Val::Double(node.as_f64().unwrap_or(0.0)),
            }),
            Json::Array(items) => items
                .iter()
                .map(Val::from_json_element)
                .collect::<Result<Vec<_>>>()
                .map(Val::List),
            Json::Object(_) => Err(ColanderError::new(
                "Nested JSON objects are not supported as answer values.",
            )),
        }
    }

    /// Doubles compare equal within [`EPSILON`] absolute tolerance.
    /// Integers and doubles compare numerically, so a calculated `3.0`
    /// matches a submitted integer literal `3`: the two spell different
    /// variants of the same value, not different values. Lists compare
    /// element-wise with the same rule.
    pub fn values_equal(left: &Val, right: &Val) -> bool {
        match (left, right) {
            (Val::Double(left), Val::Double(right)) => (left - right).abs() < EPSILON,
            (Val::Int(left), Val::Int(right)) => left == right,
            (Val::Int(left), Val::Double(right)) => (*left as f64 - right).abs() < EPSILON,
            (Val::Double(left), Val::Int(right)) => (left - *right as f64).abs() < EPSILON,
            (Val::List(left), Val::List(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right.iter())
                        .all(|(l, r)| Val::values_equal(l, r))
            }
            _ => left == right,
        }
    }

    /// Renders back to JSON; numbers use the crate's JSON number formatting.
    pub fn to_json(&self) -> Json {
        match self {
            Val::Null => Json::Null,
            Val::Bool(value) => Json::Bool(*value),
            Val::Str(text) => Json::String(text.clone()),
            Val::Int(number) => Json::integer(*number),
            Val::Double(number) => Json::double(*number),
            Val::List(items) => Json::Array(items.iter().map(Val::to_json).collect()),
            Val::Rows(rows) => Json::Array(
                rows.iter()
                    .map(|row| {
                        let mut map = JsonMap::new();
                        for (key, value) in row {
                            map.insert(key.clone(), value.to_json());
                        }
                        Json::Object(map)
                    })
                    .collect(),
            ),
            // Already-serialized JSON; re-parse so it re-serializes faithfully.
            Val::Raw(text) => json::parse(text).unwrap_or(Json::Null),
        }
    }
}

/// Parses a string as a number: signs, exponents and the `Infinity`/`NaN`
/// spellings are accepted, but thousands separators are not.
fn parse_invariant_double(text: &str) -> Result<f64> {
    let trimmed = text.trim();
    match trimmed {
        "Infinity" | "+Infinity" | "inf" | "+inf" => return Ok(f64::INFINITY),
        "-Infinity" | "-inf" => return Ok(f64::NEG_INFINITY),
        "NaN" | "nan" => return Ok(f64::NAN),
        _ => {}
    }
    trimmed
        .parse::<f64>()
        .map_err(|_| ColanderError::new(format!("Cannot convert the string '{text}' to a number.")))
}

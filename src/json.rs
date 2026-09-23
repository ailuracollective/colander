//! JSON parsing, the colander value domain, and byte-exact serialization.
//!
//! The core does not use a general-purpose JSON library: byte-exact output
//! requires control over three things such a library normalizes away.
//!
//! 1. **Raw number literals.** `canonical` and `ordered` write `Json` nodes
//!    that keep the source text, so `1.50` stays `1.50` and `1e3` stays
//!    `1e3`. `Json` therefore stores numbers as their literal source text.
//! 2. **Key order.** `canonical` sorts keys by UTF-8 byte order; the
//!    content-hash payload and `ordered` instead keep document order. Both
//!    writers live here.
//! 3. **Escaping.** `write_string` escapes `"`, `\`, `&`, `'`, `+`, `<`, `>`,
//!    `` ` `` and every code point U+007F and above as `\uXXXX` (uppercase,
//!    surrogate pairs for non-BMP), while `\b \t \n \f \r` stay short escapes.
//!
//! Doubles created *by* the core (calculated values) are rendered with
//! shortest round-trip digits, plain notation while the decimal exponent is in
//! `[-4, 16]`, otherwise `1.5E+17` / `1E-05` with a signed exponent of at
//! least two digits.

use indexmap::IndexMap;

mod parse;
mod write;

pub use parse::{ordered_stream, parse, parse_answers, parse_object};
pub use write::{canonical, format_double, ordered, sorted_keys, write_string};

/// An order-preserving JSON object; keys keep insertion order.
pub type JsonMap = IndexMap<String, Json>;

/// A JSON document. `Number` holds the literal text, never a parsed float.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Json>),
    Object(JsonMap),
}

impl Json {
    pub fn object() -> Self {
        Json::Object(JsonMap::new())
    }

    pub fn array(items: Vec<Json>) -> Self {
        Json::Array(items)
    }

    pub fn string(value: impl Into<String>) -> Self {
        Json::String(value.into())
    }

    pub fn integer(value: i64) -> Self {
        Json::Number(value.to_string())
    }

    /// A number holding `value`, rendered in the core's canonical double form.
    pub fn double(value: f64) -> Self {
        Json::Number(format_double(value))
    }

    /// A number carrying pre-formatted text (used for raw pass-through).
    pub fn number_literal(text: impl Into<String>) -> Self {
        Json::Number(text.into())
    }

    pub fn as_object(&self) -> Option<&JsonMap> {
        match self {
            Json::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_number_text(&self) -> Option<&str> {
        match self {
            Json::Number(text) => Some(text),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }

    /// Only literals without a fraction or an exponent count as integers, and
    /// only when they fit in `i64`.
    pub fn as_i64(&self) -> Option<i64> {
        let text = self.as_number_text()?;
        if text.contains(['.', 'e', 'E']) {
            return None;
        }
        text.parse::<i64>().ok()
    }

    pub fn as_f64(&self) -> Option<f64> {
        self.as_number_text()?.parse::<f64>().ok()
    }
}

impl From<&str> for Json {
    fn from(value: &str) -> Self {
        Json::String(value.to_string())
    }
}

impl From<String> for Json {
    fn from(value: String) -> Self {
        Json::String(value)
    }
}

impl From<bool> for Json {
    fn from(value: bool) -> Self {
        Json::Bool(value)
    }
}

impl From<i64> for Json {
    fn from(value: i64) -> Self {
        Json::integer(value)
    }
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

pub fn get<'a>(map: &'a JsonMap, key: &str) -> Option<&'a Json> {
    map.get(key)
}

pub fn get_str<'a>(map: &'a JsonMap, key: &str) -> Option<&'a str> {
    map.get(key).and_then(Json::as_str)
}

pub fn get_bool(map: &JsonMap, key: &str) -> Option<bool> {
    map.get(key).and_then(Json::as_bool)
}

pub fn get_i64(map: &JsonMap, key: &str) -> Option<i64> {
    map.get(key).and_then(Json::as_i64)
}

pub fn get_i32(map: &JsonMap, key: &str) -> Option<i32> {
    get_i64(map, key).and_then(|value| i32::try_from(value).ok())
}

pub fn get_f64(map: &JsonMap, key: &str) -> Option<f64> {
    map.get(key).and_then(Json::as_f64)
}

pub fn get_array<'a>(map: &'a JsonMap, key: &str) -> Option<&'a Vec<Json>> {
    map.get(key).and_then(Json::as_array)
}

pub fn get_object<'a>(map: &'a JsonMap, key: &str) -> Option<&'a JsonMap> {
    map.get(key).and_then(Json::as_object)
}

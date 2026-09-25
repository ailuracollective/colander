//! JSON codec: delegates to the handwritten parser and writers.

use crate::error::{ColanderError, Result};
use crate::json::{self, Json};

use super::Codec;

/// The default codec. It changes nothing: every method delegates to the
/// existing JSON reader and writers.
pub struct JsonCodec;

impl Codec for JsonCodec {
    type Encoded = String;

    fn name(&self) -> &'static str {
        "json"
    }

    /// Validate UTF-8, then parse.
    ///
    /// Both failures carry a contract code so a consumer can branch without
    /// reading prose (SPEC C-11): `INVALID_UTF8` for a byte sequence that is
    /// not UTF-8, `JSON_PARSE_ERROR` for well-formed UTF-8 that is not JSON.
    fn decode(&self, bytes: &[u8], what: &str) -> Result<Json> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| ColanderError::new(format!("INVALID_UTF8: {what} is not valid UTF-8")))?;
        json::parse(text).map_err(|error| {
            ColanderError::new(format!("JSON_PARSE_ERROR: Invalid {what}: {error}"))
        })
    }

    fn encode_canonical(&self, node: &Json) -> String {
        json::canonical(node)
    }

    fn encode_ordered(&self, node: &Json) -> String {
        json::ordered(node)
    }
}

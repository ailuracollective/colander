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
    /// The two failure shapes are deliberately different because they always
    /// were: a UTF-8 failure keeps the bare `"request is not valid UTF-8"`
    /// text the request path produced before the seam, while a syntax failure
    /// keeps the `Invalid request: …` label that [`json::parse_object`] added.
    /// Preserving both is what makes the JSON path byte-identical.
    fn decode(&self, bytes: &[u8], what: &str) -> Result<Json> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| ColanderError::new(format!("{what} is not valid UTF-8")))?;
        json::parse(text).map_err(|error| ColanderError::new(format!("Invalid {what}: {error}")))
    }

    fn encode_canonical(&self, node: &Json) -> String {
        json::canonical(node)
    }

    fn encode_ordered(&self, node: &Json) -> String {
        json::ordered(node)
    }
}

//! SHA-256 over the canonical component triple.
//!
//! The payload is
//! `{"form":<doc>,"ui":<doc|null>,"rules":<doc|null>}` with the three keys
//! in insertion order, `null` written literally, and each document serialized
//! in its own document order with raw numbers preserved and strings escaped by
//! [`json::write_string`]. It is emphatically **not** the key-sorted
//! [`json::canonical`] form: the sorted writer only produces the documents the
//! hash is computed over.

use sha2::{Digest, Sha256};

use crate::error::{ColanderError, Result};
use crate::json;

/// The exact UTF-8 string that is hashed. Exposed for tests and diagnosis.
pub fn canonical_payload(
    form_schema_json: &str,
    ui_schema_json: Option<&str>,
    rules_schema_json: Option<&str>,
) -> Result<String> {
    let form = document(form_schema_json, "form")?;
    let ui = match ui_schema_json {
        Some(text) => Some(document(text, "ui")?),
        None => None,
    };
    let rules = match rules_schema_json {
        Some(text) => Some(document(text, "rules")?),
        None => None,
    };

    let mut payload = String::new();
    payload.push_str("{\"form\":");
    payload.push_str(&form);
    payload.push_str(",\"ui\":");
    payload.push_str(ui.as_deref().unwrap_or("null"));
    payload.push_str(",\"rules\":");
    payload.push_str(rules.as_deref().unwrap_or("null"));
    payload.push('}');
    Ok(payload)
}

fn document(text: &str, label: &str) -> Result<String> {
    json::ordered_stream(text)
        .map_err(|e| ColanderError::new(format!("Invalid {label} schema: {e}")))
}

/// Lowercase hex SHA-256 of the canonical payload.
pub fn content_hash(
    form_schema_json: &str,
    ui_schema_json: Option<&str>,
    rules_schema_json: Option<&str>,
) -> Result<String> {
    let payload = canonical_payload(form_schema_json, ui_schema_json, rules_schema_json)?;
    Ok(sha256_hex(payload.as_bytes()))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push(char::from_digit((byte >> 4) as u32, 16).expect("nibble"));
        out.push(char::from_digit((byte & 0xF) as u32, 16).expect("nibble"));
    }
    out
}

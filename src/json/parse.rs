//! JSON parser.

use super::write::{ordered, write_char_escaped, write_string_body};
use super::{Json, JsonMap};
use crate::error::{ColanderError, Result};

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

const MAX_DEPTH: usize = 64;

/// Parse any JSON text. Numbers keep their source text; a repeated object key
/// keeps its first position and its last value.
pub fn parse(text: &str) -> Result<Json> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        position: 0,
        seen: Vec::new(),
        repeated: false,
    };
    parser.skip_whitespace();
    let value = parser.parse_value(0)?;
    parser.skip_whitespace();
    if parser.position != parser.bytes.len() {
        return Err(parser.error("unexpected trailing content"));
    }
    Ok(value)
}

/// The [`json::ordered`] form of `text`, written without building a `Json`.
///
/// One pass validates the document and emits the canonical text. A document that
/// repeats an object key needs the whole object to apply the DOM's rule (last
/// value wins, first position kept), so that case is reparsed.
pub fn ordered_stream(text: &str) -> Result<String> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        position: 0,
        seen: Vec::new(),
        repeated: false,
    };
    let mut out = String::with_capacity(text.len());
    parser.skip_whitespace();
    parser.parse_value_stream(&mut out, 0)?;
    parser.skip_whitespace();
    if parser.position != parser.bytes.len() {
        return Err(parser.error("unexpected trailing content"));
    }
    if parser.repeated {
        return Ok(ordered(&parse(text)?));
    }
    Ok(out)
}

/// Parse a JSON object, labelling failures with `label`.
///
/// A document that is valid JSON but not an object is a deliberate ordinary
/// error: callers never see a panic. Both shapes carry a contract code
/// (`JSON_PARSE_ERROR`, `JSON_NOT_OBJECT`) so callers branch on the code, not
/// on the prose (SPEC C-11).
pub fn parse_object(text: &str, label: &str) -> Result<JsonMap> {
    match parse(text) {
        Ok(Json::Object(map)) => Ok(map),
        Ok(_) => Err(ColanderError::new(format!(
            "JSON_NOT_OBJECT: Invalid {label}: expected a JSON object."
        ))),
        Err(e) => Err(ColanderError::new(format!(
            "JSON_PARSE_ERROR: Invalid {label}: {e}"
        ))),
    }
}

/// Parse the answers document; it must be a JSON object.
pub fn parse_answers(text: &str) -> Result<JsonMap> {
    match parse(text) {
        Ok(Json::Object(map)) => Ok(map),
        Ok(_) => Err(ColanderError::new(
            "JSON_NOT_OBJECT: Answers must be a JSON object.",
        )),
        Err(e) => Err(ColanderError::new(format!(
            "JSON_PARSE_ERROR: Answers must be valid JSON: {e}"
        ))),
    }
}

/// Where a decoded JSON string goes: either decoded into a `String`, or written
/// out escaped, which is what the streaming canonicaliser needs.
enum StrSink<'a> {
    Value(&'a mut String),
    Escaped(&'a mut String),
}

struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
    /// One entry per key of every object currently open: FNV-1a 64 of the escaped
    /// key token plus its start/end offsets in the output buffer. `Vec::new()` does
    /// not allocate, so the DOM path pays nothing for it.
    seen: Vec<(u64, usize, usize)>,
    /// Set once the document repeats an object key; the streamed bytes are then
    /// discarded in favour of a DOM reparse, which applies the insert rule.
    repeated: bool,
}

impl Parser<'_> {
    fn error(&self, message: &str) -> ColanderError {
        ColanderError::new(format!("{message} at byte offset {}.", self.position))
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.position += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> Result<()> {
        if self.peek() == Some(byte) {
            self.position += 1;
            Ok(())
        } else {
            Err(self.error(&format!("expected '{}'", byte as char)))
        }
    }

    fn parse_value(&mut self, depth: usize) -> Result<Json> {
        if depth > MAX_DEPTH {
            return Err(self.error("JSON exceeds the maximum nesting depth"));
        }
        match self.peek() {
            Some(b'{') => self.parse_object(depth),
            Some(b'[') => self.parse_array(depth),
            Some(b'"') => Ok(Json::String(self.parse_string()?)),
            Some(b't') => self.parse_literal("true", Json::Bool(true)),
            Some(b'f') => self.parse_literal("false", Json::Bool(false)),
            Some(b'n') => self.parse_literal("null", Json::Null),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(other) => Err(self.error(&format!("unexpected byte 0x{other:02X}"))),
            None => Err(self.error("unexpected end of input")),
        }
    }

    /// [`Parser::parse_value`] with the canonical bytes appended as they are read.
    fn parse_value_stream(&mut self, out: &mut String, depth: usize) -> Result<()> {
        if depth > MAX_DEPTH {
            return Err(self.error("JSON exceeds the maximum nesting depth"));
        }
        match self.peek() {
            Some(b'{') => self.parse_object_stream(out, depth),
            Some(b'[') => self.parse_array_stream(out, depth),
            Some(b'"') => self.write_string_stream(out),
            Some(b't') => self.parse_literal_stream("true", out),
            Some(b'f') => self.parse_literal_stream("false", out),
            Some(b'n') => self.parse_literal_stream("null", out),
            Some(b'-' | b'0'..=b'9') => {
                out.push_str(self.scan_number()?);
                Ok(())
            }
            Some(other) => Err(self.error(&format!("unexpected byte 0x{other:02X}"))),
            None => Err(self.error("unexpected end of input")),
        }
    }

    fn parse_literal(&mut self, literal: &str, value: Json) -> Result<Json> {
        if self.bytes[self.position..].starts_with(literal.as_bytes()) {
            self.position += literal.len();
            Ok(value)
        } else {
            Err(self.error(&format!("expected {literal}")))
        }
    }

    /// [`Parser::parse_literal`] with the literal text appended on success.
    fn parse_literal_stream(&mut self, literal: &str, out: &mut String) -> Result<()> {
        if self.bytes[self.position..].starts_with(literal.as_bytes()) {
            self.position += literal.len();
            out.push_str(literal);
            Ok(())
        } else {
            Err(self.error(&format!("expected {literal}")))
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<Json> {
        self.expect(b'{')?;
        let mut map = JsonMap::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.position += 1;
            return Ok(Json::Object(map));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            self.skip_whitespace();
            let value = self.parse_value(depth + 1)?;
            // Duplicate keys: last value wins, first position kept.
            map.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.position += 1;
                }
                Some(b'}') => {
                    self.position += 1;
                    return Ok(Json::Object(map));
                }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    /// [`Parser::parse_object`] with the canonical bytes appended as they are read.
    ///
    /// Object keys are the only JSON construct whose duplicate handling (last value
    /// wins, first position kept) cannot be reproduced by copying bytes through.
    /// Each key's escaped token is recorded; a repeat anywhere in the document marks
    /// [`Parser::repeated`], and the caller falls back to the DOM writer.
    fn parse_object_stream(&mut self, out: &mut String, depth: usize) -> Result<()> {
        self.expect(b'{')?;
        out.push('{');
        // Keys of the objects enclosing this one live below `mark`; every exit from
        // this object truncates back to it, so a nested object cannot leak into its
        // parent's scope. A local, not a stack: the parent's own mark stays valid.
        let mark = self.seen.len();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.position += 1;
            out.push('}');
            self.seen.truncate(mark);
            return Ok(());
        }
        loop {
            self.skip_whitespace();
            let key_start = out.len();
            self.write_string_stream(out)?;
            let key_end = out.len();
            if !self.repeated {
                let token = &out.as_bytes()[key_start..key_end];
                let hash = hash_bytes(token);
                if self.seen[mark..].iter().any(|(seen_hash, start, end)| {
                    *seen_hash == hash && out.as_bytes()[*start..*end] == *token
                }) {
                    self.repeated = true;
                } else {
                    self.seen.push((hash, key_start, key_end));
                }
            }
            self.skip_whitespace();
            self.expect(b':')?;
            out.push(':');
            self.skip_whitespace();
            self.parse_value_stream(out, depth + 1)?;
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.position += 1;
                    out.push(',');
                }
                Some(b'}') => {
                    self.position += 1;
                    out.push('}');
                    self.seen.truncate(mark);
                    return Ok(());
                }
                _ => {
                    self.seen.truncate(mark);
                    return Err(self.error("expected ',' or '}'"));
                }
            }
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<Json> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.position += 1;
            return Ok(Json::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.parse_value(depth + 1)?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.position += 1;
                }
                Some(b']') => {
                    self.position += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    /// [`Parser::parse_array`] with the canonical bytes appended as they are read.
    fn parse_array_stream(&mut self, out: &mut String, depth: usize) -> Result<()> {
        self.expect(b'[')?;
        out.push('[');
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.position += 1;
            out.push(']');
            return Ok(());
        }
        loop {
            self.skip_whitespace();
            self.parse_value_stream(out, depth + 1)?;
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => {
                    self.position += 1;
                    out.push(',');
                }
                Some(b']') => {
                    self.position += 1;
                    out.push(']');
                    return Ok(());
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    /// [`Parser::parse_string`] with the decoded string appended as it is read.
    fn write_string_stream(&mut self, out: &mut String) -> Result<()> {
        out.push('"');
        // Reborrow `out` so the enum's borrow ends before the closing quote.
        {
            let mut sink = StrSink::Escaped(&mut *out);
            self.read_string(&mut sink)?;
        }
        out.push('"');
        Ok(())
    }

    fn parse_string(&mut self) -> Result<String> {
        let mut out = String::new();
        self.read_string(&mut StrSink::Value(&mut out))?;
        Ok(out)
    }

    /// Read the body of a JSON string into `sink`, having consumed the opening quote.
    ///
    /// The DOM parser and the streaming canonicaliser share this reader so their
    /// validation, offsets and escape decoding cannot drift; only the sink differs.
    fn read_string(&mut self, sink: &mut StrSink<'_>) -> Result<()> {
        self.expect(b'"')?;
        loop {
            // Copy the run of bytes that need no interpretation in one go. Strings
            // are most of a document's bytes, and taking them one at a time costs a
            // UTF-8 decode and a separate append per character.
            let start = self.position;
            while self.position < self.bytes.len() && is_unquoted(self.bytes[self.position]) {
                self.position += 1;
            }
            if self.position > start {
                let text = std::str::from_utf8(&self.bytes[start..self.position])
                    .map_err(|error| utf8_error(self.bytes, start + error.valid_up_to()))?;
                sink_push_str(sink, text);
            }

            let byte = self
                .peek()
                .ok_or_else(|| self.error("unterminated string"))?;
            match byte {
                b'"' => {
                    self.position += 1;
                    return Ok(());
                }
                b'\\' => {
                    self.position += 1;
                    let escape = self
                        .peek()
                        .ok_or_else(|| self.error("unterminated escape"))?;
                    self.position += 1;
                    let ch = match escape {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{0008}',
                        b'f' => '\u{000C}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => self.parse_unicode_escape()?,
                        other => {
                            return Err(
                                self.error(&format!("invalid escape '\\{}'", other as char))
                            );
                        }
                    };
                    sink_push_char(sink, ch);
                }
                // The run above stops at nothing else, so what is left is a control
                // character: the one thing JSON forbids unescaped inside a string.
                _ => return Err(self.error("unescaped control character")),
            }
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<char> {
        let high = self.parse_hex4()?;
        if (0xD800..0xDC00).contains(&high) {
            // A surrogate pair is two \u escapes.
            if self.peek() == Some(b'\\') && self.bytes.get(self.position + 1) == Some(&b'u') {
                self.position += 2;
                let low = self.parse_hex4()?;
                if (0xDC00..0xE000).contains(&low) {
                    let code = 0x1_0000 + ((high - 0xD800) << 10) + (low - 0xDC00);
                    return char::from_u32(code)
                        .ok_or_else(|| self.error("invalid surrogate pair"));
                }
                return Err(self.error("invalid low surrogate"));
            }
            return Err(self.error("lone high surrogate"));
        }
        if (0xDC00..0xE000).contains(&high) {
            return Err(self.error("lone low surrogate"));
        }
        char::from_u32(high).ok_or_else(|| self.error("invalid unicode escape"))
    }

    fn parse_hex4(&mut self) -> Result<u32> {
        let slice = self
            .bytes
            .get(self.position..self.position + 4)
            .ok_or_else(|| self.error("truncated unicode escape"))?;
        let text = std::str::from_utf8(slice).map_err(|_| self.error("invalid unicode escape"))?;
        let value =
            u32::from_str_radix(text, 16).map_err(|_| self.error("invalid unicode escape"))?;
        self.position += 4;
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<Json> {
        Ok(Json::Number(self.scan_number()?.to_string()))
    }

    /// Scan a number, keeping its source text, and return that slice.
    fn scan_number(&mut self) -> Result<&str> {
        let start = self.position;
        if self.peek() == Some(b'-') {
            self.position += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.position += 1;
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.position += 1;
                }
            }
            _ => return Err(self.error("invalid number")),
        }
        if self.peek() == Some(b'.') {
            self.position += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("invalid number fraction"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("invalid number exponent"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.position])
            .map_err(|_| self.error("invalid number"))?;
        Ok(text)
    }
}

/// Decode a literal run into the string sink, or escape it into the output sink.
fn sink_push_str(sink: &mut StrSink<'_>, text: &str) {
    match sink {
        StrSink::Value(out) => out.push_str(text),
        StrSink::Escaped(out) => write_string_body(out, text),
    }
}

/// Decode one character into the string sink, or escape it into the output sink.
fn sink_push_char(sink: &mut StrSink<'_>, ch: char) {
    match sink {
        StrSink::Value(out) => out.push(ch),
        StrSink::Escaped(out) => write_char_escaped(out, ch),
    }
}

/// FNV-1a 64-bit hash, used only to bucket escaped key tokens for the duplicate
/// check; byte equality is what actually decides a duplicate.
fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

/// Bytes a JSON string may hold literally: neither the closing quote, nor the
/// escape introducer, nor a control character. Bytes at or above 0x80 are part of
/// a multi-byte sequence and are copied whole, then validated in bulk.
fn is_unquoted(byte: u8) -> bool {
    byte >= 0x20 && byte != b'"' && byte != b'\\'
}

/// Diagnose a run that is not valid UTF-8 exactly as a byte-at-a-time walk would
/// have: the same two messages, at the same offset, so the text a caller sees for
/// a malformed document did not change with the copy strategy.
fn utf8_error(bytes: &[u8], at: usize) -> ColanderError {
    let width = utf8_width(bytes[at]);
    if at + width > bytes.len() {
        ColanderError::new(format!("truncated UTF-8 sequence at byte offset {at}."))
    } else {
        ColanderError::new(format!("invalid UTF-8 at byte offset {at}."))
    }
}

fn utf8_width(byte: u8) -> usize {
    match byte {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

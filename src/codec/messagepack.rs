//! Handwritten MessagePack codec.
//!
//! MessagePack is a binary format: a one-byte tag followed by its payload. The
//! [`Json`] model has no byte-string or extension variant, so the `bin*` and
//! `ext*` tags are rejected rather than forced into a variant they are not.
//!
//! # Number policy
//!
//! [`Json`] stores numbers as their source text, while MessagePack stores an
//! integer or a float. Encoding therefore has to choose, and the choice is
//! fixed so the output is deterministic:
//!
//! - A literal that parses as `i64` is written with the smallest integer
//!   encoding that holds it: positive fixint (`0x00..=0x7F`), negative fixint
//!   (`0xE0..=0xFF`), `int8` (`0xD0`), `int16` (`0xD1`), `int32` (`0xD2`) or
//!   `int64` (`0xD3`).
//! - Otherwise, a literal that parses as `u64` is written as `uint64` (`0xCF`),
//!   which covers values above `i64::MAX`.
//! - Otherwise the literal is parsed as `f64` and written as `float64`
//!   (`0xCB`). Non-integers are always `float64`, never `float32`, so the
//!   output does not depend on a lossy narrowing pass.
//!
//! Decoding is the inverse but not a round trip: `1.50` returns as `1.5` and
//! `1e3` returns as `1000`, because the literal spelling is gone. See
//! `docs/codecs.md`.

use crate::error::{ColanderError, Result};
use crate::json::{Json, JsonMap};

use super::Codec;

/// The MessagePack implementation of the codec seam.
pub struct MessagePackCodec;

/// Matches `MAX_DEPTH` in `src/json/parse.rs`, so both codecs reject the same
/// nesting shapes and a hostile payload cannot exhaust the stack.
const MAX_DEPTH: usize = 64;

impl Codec for MessagePackCodec {
    type Encoded = Vec<u8>;

    fn name(&self) -> &'static str {
        "messagepack"
    }

    fn decode(&self, bytes: &[u8], what: &str) -> Result<Json> {
        let mut reader = Reader {
            bytes,
            position: 0,
            what,
        };
        reader.read_document()
    }

    fn encode_canonical(&self, node: &Json) -> Vec<u8> {
        let mut out = Vec::new();
        write_value(&mut out, node, true);
        out
    }

    fn encode_ordered(&self, node: &Json) -> Vec<u8> {
        let mut out = Vec::new();
        write_value(&mut out, node, false);
        out
    }
}

// ---------------------------------------------------------------------------
// Encoder
// ---------------------------------------------------------------------------

fn write_value(out: &mut Vec<u8>, node: &Json, sorted: bool) {
    match node {
        Json::Null => out.push(0xC0),
        Json::Bool(false) => out.push(0xC2),
        Json::Bool(true) => out.push(0xC3),
        Json::Number(text) => write_number(out, text),
        Json::String(text) => write_string(out, text),
        Json::Array(items) => {
            write_array_header(out, items.len());
            for item in items {
                write_value(out, item, sorted);
            }
        }
        Json::Object(map) => {
            write_map_header(out, map.len());
            if sorted {
                // Sort key and value together: sorting the names and then
                // indexing the map back would hash every key a second time.
                // This mirrors `write_canonical` in `src/json/write.rs`.
                let mut entries: Vec<(&str, &Json)> = map
                    .iter()
                    .map(|(key, value)| (key.as_str(), value))
                    .collect();
                entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
                for (key, value) in entries {
                    write_string(out, key);
                    write_value(out, value, sorted);
                }
            } else {
                for (key, value) in map {
                    write_string(out, key);
                    write_value(out, value, sorted);
                }
            }
        }
    }
}

fn write_number(out: &mut Vec<u8>, text: &str) {
    if let Ok(value) = text.parse::<i64>() {
        write_int(out, value);
    } else if let Ok(value) = text.parse::<u64>() {
        out.push(0xCF);
        out.extend_from_slice(&value.to_be_bytes());
    } else if let Ok(value) = text.parse::<f64>() {
        out.push(0xCB);
        out.extend_from_slice(&value.to_be_bytes());
    } else {
        // Unreachable for parsed documents; keeps the encoder total.
        out.push(0xC0);
    }
}

fn write_int(out: &mut Vec<u8>, value: i64) {
    if (-32..=0x7F).contains(&value) {
        out.push(value as u8);
    } else if (-128..=-33).contains(&value) {
        out.push(0xD0);
        out.push(value as u8);
    } else if (-32_768..=i64::from(i16::MAX)).contains(&value) {
        out.push(0xD1);
        out.extend_from_slice(&(value as i16).to_be_bytes());
    } else if (i64::from(i32::MIN)..=i64::from(i32::MAX)).contains(&value) {
        out.push(0xD2);
        out.extend_from_slice(&(value as i32).to_be_bytes());
    } else {
        out.push(0xD3);
        out.extend_from_slice(&value.to_be_bytes());
    }
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    let len = value.len();
    if len <= 31 {
        out.push(0xA0 | len as u8);
    } else if len <= u8::MAX as usize {
        out.push(0xD9);
        out.push(len as u8);
    } else if len <= u16::MAX as usize {
        out.push(0xDA);
        out.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        out.push(0xDB);
        out.extend_from_slice(&(len as u32).to_be_bytes());
    }
    out.extend_from_slice(value.as_bytes());
}

fn write_array_header(out: &mut Vec<u8>, len: usize) {
    if len <= 15 {
        out.push(0x90 | len as u8);
    } else if len <= u16::MAX as usize {
        out.push(0xDC);
        out.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        out.push(0xDD);
        out.extend_from_slice(&(len as u32).to_be_bytes());
    }
}

fn write_map_header(out: &mut Vec<u8>, len: usize) {
    if len <= 15 {
        out.push(0x80 | len as u8);
    } else if len <= u16::MAX as usize {
        out.push(0xDE);
        out.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        out.push(0xDF);
        out.extend_from_slice(&(len as u32).to_be_bytes());
    }
}

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
    what: &'a str,
}

impl<'a> Reader<'a> {
    fn error(&self, message: impl std::fmt::Display) -> ColanderError {
        ColanderError::new(format!(
            "Invalid {}: {message} at byte offset {}.",
            self.what, self.position
        ))
    }

    fn read_document(&mut self) -> Result<Json> {
        let value = self.read_value(0)?;
        if self.position != self.bytes.len() {
            return Err(self.error("trailing bytes after the value"));
        }
        Ok(value)
    }

    /// Copy `width` big-endian bytes, bounds-checked.
    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let bytes = self.bytes;
        let end = self
            .position
            .checked_add(len)
            .ok_or_else(|| self.error("length overflows the input"))?;
        let slice = bytes
            .get(self.position..end)
            .ok_or_else(|| self.error("truncated input"))?;
        self.position = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8> {
        let byte = self
            .bytes
            .get(self.position)
            .copied()
            .ok_or_else(|| self.error("unexpected end of input"))?;
        self.position += 1;
        Ok(byte)
    }

    fn read_uint(&mut self, width: usize) -> Result<u64> {
        let slice = self.take(width)?;
        let mut value = 0u64;
        for byte in slice {
            value = (value << 8) | u64::from(*byte);
        }
        Ok(value)
    }

    fn read_value(&mut self, depth: usize) -> Result<Json> {
        if depth > MAX_DEPTH {
            return Err(self.error("MessagePack exceeds the maximum nesting depth"));
        }
        let tag = self.read_u8()?;
        match tag {
            0x00..=0x7F => Ok(Json::integer(i64::from(tag))),
            0x80..=0x8F => self.read_map((tag & 0x0F) as usize, depth),
            0x90..=0x9F => self.read_array((tag & 0x0F) as usize, depth),
            0xA0..=0xBF => self.read_str((tag & 0x1F) as usize).map(Json::String),
            0xC0 => Ok(Json::Null),
            0xC1 => Err(self.error("invalid MessagePack tag 0xC1")),
            0xC2 => Ok(Json::Bool(false)),
            0xC3 => Ok(Json::Bool(true)),
            0xC4..=0xC6 => Err(self.error("MessagePack bin values are not supported")),
            0xC7..=0xC9 | 0xD4..=0xD8 => {
                Err(self.error("MessagePack extension values are not supported"))
            }
            0xCA => Ok(Json::double(f64::from(f32::from_bits(
                self.read_uint(4)? as u32
            )))),
            0xCB => Ok(Json::double(f64::from_bits(self.read_uint(8)?))),
            0xCC => Ok(Json::integer(self.read_uint(1)? as i64)),
            0xCD => Ok(Json::integer(self.read_uint(2)? as i64)),
            0xCE => Ok(Json::integer(self.read_uint(4)? as i64)),
            0xCF => {
                let value = self.read_uint(8)?;
                Ok(match i64::try_from(value) {
                    Ok(signed) => Json::integer(signed),
                    Err(_) => Json::number_literal(value.to_string()),
                })
            }
            0xD0 => Ok(Json::integer(self.read_uint(1)? as i8 as i64)),
            0xD1 => Ok(Json::integer(self.read_uint(2)? as i16 as i64)),
            0xD2 => Ok(Json::integer(self.read_uint(4)? as i32 as i64)),
            0xD3 => Ok(Json::integer(self.read_uint(8)? as i64)),
            0xD9 => {
                let len = self.read_uint(1)? as usize;
                self.read_str(len).map(Json::String)
            }
            0xDA => {
                let len = self.read_uint(2)? as usize;
                self.read_str(len).map(Json::String)
            }
            0xDB => {
                let len = self.read_uint(4)? as usize;
                self.read_str(len).map(Json::String)
            }
            0xDC => {
                let count = self.read_uint(2)? as usize;
                self.read_array(count, depth)
            }
            0xDD => {
                let count = self.read_uint(4)? as usize;
                self.read_array(count, depth)
            }
            0xDE => {
                let count = self.read_uint(2)? as usize;
                self.read_map(count, depth)
            }
            0xDF => {
                let count = self.read_uint(4)? as usize;
                self.read_map(count, depth)
            }
            0xE0..=0xFF => Ok(Json::integer(i64::from(tag as i8))),
        }
    }

    fn read_str(&mut self, len: usize) -> Result<String> {
        let slice = self.take(len)?;
        std::str::from_utf8(slice)
            .map(str::to_string)
            .map_err(|_| self.error("string is not valid UTF-8"))
    }

    fn read_array(&mut self, count: usize, depth: usize) -> Result<Json> {
        // `Vec::new`, not `with_capacity`: a hostile count must not reserve.
        let mut items = Vec::new();
        for _ in 0..count {
            items.push(self.read_value(depth + 1)?);
        }
        Ok(Json::Array(items))
    }

    fn read_map(&mut self, count: usize, depth: usize) -> Result<Json> {
        // `JsonMap::new`, not `with_capacity`: a hostile count must not reserve.
        let mut map = JsonMap::new();
        for _ in 0..count {
            let key = self.read_key()?;
            let value = self.read_value(depth + 1)?;
            // IndexMap::insert keeps the first position and the last value,
            // matching the JSON DOM's duplicate-key rule.
            map.insert(key, value);
        }
        Ok(Json::Object(map))
    }

    fn read_key(&mut self) -> Result<String> {
        let tag = self.read_u8()?;
        match tag {
            0xA0..=0xBF => self.read_str((tag & 0x1F) as usize),
            0xD9 => {
                let len = self.read_uint(1)? as usize;
                self.read_str(len)
            }
            0xDA => {
                let len = self.read_uint(2)? as usize;
                self.read_str(len)
            }
            0xDB => {
                let len = self.read_uint(4)? as usize;
                self.read_str(len)
            }
            _ => Err(self.error("object key is not a string")),
        }
    }
}

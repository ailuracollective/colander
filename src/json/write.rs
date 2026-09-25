//! Byte-exact JSON writers.

use super::{Json, JsonMap};

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

/// Keys are ordered by UTF-8 byte order, not locale collation and not UTF-16
/// code-unit order: keys mixing non-BMP characters with U+E000..U+FFFF sort
/// differently than a UTF-16 code-unit comparison would.
pub fn sorted_keys(map: &JsonMap) -> Vec<&str> {
    let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys
}

/// The canonical form: keys sorted by UTF-8 byte order, no whitespace.
pub fn canonical(node: &Json) -> String {
    let mut out = String::new();
    write_canonical(&mut out, node);
    out
}

fn write_canonical(out: &mut String, node: &Json) {
    match node {
        Json::Null => out.push_str("null"),
        Json::Bool(true) => out.push_str("true"),
        Json::Bool(false) => out.push_str("false"),
        Json::Number(text) => out.push_str(text),
        Json::String(text) => write_string(out, text),
        Json::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(out, item);
            }
            out.push(']');
        }
        Json::Object(map) => {
            // Sort key and value together: sorting the names and then indexing the
            // map back would hash every key a second time.
            let mut entries: Vec<(&str, &Json)> = map
                .iter()
                .map(|(key, value)| (key.as_str(), value))
                .collect();
            entries.sort_unstable_by(|left, right| left.0.cmp(right.0));

            out.push('{');
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_string(out, key);
                out.push(':');
                write_canonical(out, value);
            }
            out.push('}');
        }
    }
}

/// The ordered form: document order preserved, raw numbers and the
/// [`write_string`] escaping kept. Used for the content-hash payload and as
/// the object fallback in the rule-value conversion.
pub fn ordered(node: &Json) -> String {
    let mut out = String::new();
    write_ordered(&mut out, node);
    out
}

fn write_ordered(out: &mut String, node: &Json) {
    match node {
        Json::Null => out.push_str("null"),
        Json::Bool(true) => out.push_str("true"),
        Json::Bool(false) => out.push_str("false"),
        Json::Number(text) => out.push_str(text),
        Json::String(text) => write_string(out, text),
        Json::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_ordered(out, item);
            }
            out.push(']');
        }
        Json::Object(map) => {
            out.push('{');
            for (index, (key, value)) in map.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_string(out, key);
                out.push(':');
                write_ordered(out, value);
            }
            out.push('}');
        }
    }
}

/// Append the quoted, escaped form of `value`; the module docs list the
/// escaping rules.
pub fn write_string(out: &mut String, value: &str) {
    out.push('"');
    write_string_body(out, value);
    out.push('"');
}

/// Append the body of the escaped form of `value`, without the surrounding
/// quotes. Shared with the streaming canonicaliser, whose reader decodes escapes
/// only to hand each character back to [`write_char_escaped`].
pub(crate) fn write_string_body(out: &mut String, value: &str) {
    // Copy the runs that need no escaping in one go. Escaping is per code point,
    // but a run of characters that pass through untouched is always ASCII, so the
    // slice boundaries below are character boundaries by construction.
    let bytes = value.as_bytes();
    let mut index = 0;
    let mut literal_start = 0;

    while index < bytes.len() {
        if is_unescaped(bytes[index]) {
            index += 1;
            continue;
        }

        if literal_start < index {
            out.push_str(&value[literal_start..index]);
        }

        let ch = value[index..]
            .chars()
            .next()
            .expect("the scan stops on a character boundary");
        write_char_escaped(out, ch);

        index += ch.len_utf8();
        literal_start = index;
    }

    if literal_start < bytes.len() {
        out.push_str(&value[literal_start..]);
    }
}

/// Append `ch` in its escaped form.
///
/// The unescaped-passthrough check matters for the streaming caller: a character
/// decoded from an escape (`\u0041` -> `A`, `\/` -> `/`) must re-encode exactly as
/// [`write_string`] would encode it, which is literally for these characters and
/// escaped for everything else. [`write_string_body`] only calls this for
/// characters it cannot copy, so the check is a no-op on that path.
pub(crate) fn write_char_escaped(out: &mut String, ch: char) {
    if ch.is_ascii() && is_unescaped(ch as u8) {
        out.push(ch);
        return;
    }
    match ch {
        '"' => out.push_str("\\u0022"),
        '\\' => out.push_str("\\\\"),
        '\u{0008}' => out.push_str("\\b"),
        '\u{0009}' => out.push_str("\\t"),
        '\u{000A}' => out.push_str("\\n"),
        '\u{000C}' => out.push_str("\\f"),
        '\u{000D}' => out.push_str("\\r"),
        '&' | '\'' | '+' | '<' | '>' | '`' => write_u16_escape(out, ch as u16),
        other => {
            let code = other as u32;
            if code > 0xFFFF {
                let adjusted = code - 0x1_0000;
                write_u16_escape(out, (0xD800 + (adjusted >> 10)) as u16);
                write_u16_escape(out, (0xDC00 + (adjusted & 0x3FF)) as u16);
            } else {
                write_u16_escape(out, code as u16);
            }
        }
    }
}

/// Bytes [`write_string`] copies through untouched: printable ASCII minus the ones
/// the escaping rules rewrite. Everything at or above 0x7F is escaped, so a run of
/// these is ASCII and slicing it can never split a character.
fn is_unescaped(byte: u8) -> bool {
    matches!(byte, 0x20..=0x7E)
        && !matches!(
            byte,
            b'"' | b'\\' | b'&' | b'\'' | b'+' | b'<' | b'>' | b'`'
        )
}

fn write_u16_escape(out: &mut String, unit: u16) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push_str("\\u");
    out.push(HEX[(unit >> 12) as usize] as char);
    out.push(HEX[((unit >> 8) & 0xF) as usize] as char);
    out.push(HEX[((unit >> 4) & 0xF) as usize] as char);
    out.push(HEX[(unit & 0xF) as usize] as char);
}

/// The core's canonical rendering of a `double`.
///
/// `3.0 -> "3"`, `-0.0 -> "-0"`, `1e16 -> "10000000000000000"`,
/// `1e17 -> "1E+17"`, `1e-4 -> "0.0001"`, `1e-5 -> "1E-05"`,
/// `5e-324 -> "5E-324"`, `22.86 -> "22.86"`.
pub fn format_double(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() { "-0" } else { "0" }.to_string();
    }
    if !value.is_finite() {
        // Callers turn NaN and Infinity into JSON null before reaching here;
        // the `"null"` fallback only keeps the output deterministic.
        return "null".to_string();
    }

    // LowerExp gives the shortest round-trip digits in `d.dddde<exp>` form,
    // which is the digit string to re-format below.
    let scientific = format!("{value:e}");
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("LowerExp always contains an exponent");
    let negative = mantissa.starts_with('-');
    let mantissa = mantissa.trim_start_matches('-');
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let exponent: i32 = exponent.parse().expect("LowerExp exponent is an integer");

    let mut out = String::new();
    if negative {
        out.push('-');
    }

    if !(-4..=16).contains(&exponent) {
        out.push_str(&digits[..1]);
        if digits.len() > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('E');
        out.push(if exponent < 0 { '-' } else { '+' });
        let magnitude = exponent.unsigned_abs();
        if magnitude < 10 {
            out.push('0');
        }
        out.push_str(&magnitude.to_string());
        return out;
    }

    if exponent >= 0 {
        let integer_digits = exponent as usize + 1;
        if digits.len() <= integer_digits {
            out.push_str(&digits);
            for _ in digits.len()..integer_digits {
                out.push('0');
            }
        } else {
            out.push_str(&digits[..integer_digits]);
            out.push('.');
            out.push_str(&digits[integer_digits..]);
        }
    } else {
        out.push_str("0.");
        for _ in 0..(-exponent - 1) {
            out.push('0');
        }
        out.push_str(&digits);
    }
    out
}

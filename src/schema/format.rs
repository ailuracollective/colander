//! The closed `format` vocabulary and its checks (SPEC S-5).
//!
//! `format` is asserted for exactly the nine names below, and only for string
//! instances. An unrecognised name is an error rather than an ignored
//! annotation. Whether a check is exact or a pragmatic approximation is stated
//! on the check itself, because a validator that overstates what it validated
//! fails open by prose.

/// The formats colander asserts.
pub const KNOWN: &[&str] = &[
    "email",
    "date",
    "date-time",
    "time",
    "uuid",
    "ipv4",
    "ipv6",
    "hostname",
    "uri",
];

/// Whether the core asserts this format name at all.
pub fn is_known(name: &str) -> bool {
    KNOWN.contains(&name)
}

/// Whether `text` has the named format. The name is expected to be known; an
/// unknown name matches nothing, and the structural classifier owns the error
/// for it, so evaluation never reports it twice.
pub fn matches(name: &str, text: &str) -> bool {
    match name {
        "email" => is_email(text),
        "date" => is_date(text),
        "date-time" => is_datetime(text),
        "time" => is_time(text),
        "uuid" => is_uuid(text),
        "ipv4" => is_ipv4(text),
        "ipv6" => is_ipv6(text),
        "hostname" => is_hostname(text),
        "uri" => is_uri(text),
        _ => false,
    }
}

fn two_digits(bytes: &[u8]) -> Option<u32> {
    if bytes.len() != 2 || !bytes[0].is_ascii_digit() || !bytes[1].is_ascii_digit() {
        return None;
    }
    Some(u32::from(bytes[0] - b'0') * 10 + u32::from(bytes[1] - b'0'))
}

fn valid_month_day(year: u32, month: u32, day: u32) -> bool {
    if !(1..=12).contains(&month) || day < 1 {
        return false;
    }
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            let leap =
                year.is_multiple_of(4) && !year.is_multiple_of(100) || year.is_multiple_of(400);
            if leap { 29 } else { 28 }
        }
    };
    day <= days
}

/// Exact against RFC 3339 `full-date`: `YYYY-MM-DD`, four digits, a valid month
/// and a day that exists in that month, including leap years. No other
/// separator or width is accepted, so `24-01-01` and `2024/01/01` fail.
fn is_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits = |range: std::ops::Range<usize>| -> Option<u32> {
        let mut value = 0u32;
        for &byte in &bytes[range] {
            if !byte.is_ascii_digit() {
                return None;
            }
            value = value * 10 + u32::from(byte - b'0');
        }
        Some(value)
    };
    let (Some(year), Some(month), Some(day)) = (digits(0..4), digits(5..7), digits(8..10)) else {
        return false;
    };
    valid_month_day(year, month, day)
}

/// Exact against RFC 3339 `full-time` except for leap seconds: `HH:MM:SS`, an
/// optional fraction of one or more digits, then `Z` or `±HH:MM`. Second 60 and
/// hour 24 are rejected. Used by both `time` and `date-time`.
fn is_full_time(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() < 8 || bytes[2] != b':' || bytes[5] != b':' {
        return false;
    }
    let (Some(hour), Some(minute), Some(second)) = (
        two_digits(&bytes[0..2]),
        two_digits(&bytes[3..5]),
        two_digits(&bytes[6..8]),
    ) else {
        return false;
    };
    if hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let mut rest = &bytes[8..];
    if rest.first() == Some(&b'.') {
        rest = &rest[1..];
        let digits = rest.iter().take_while(|b| b.is_ascii_digit()).count();
        if digits == 0 {
            return false;
        }
        rest = &rest[digits..];
    }
    if rest == b"Z" || rest == b"z" {
        return true;
    }
    if rest.len() != 6 || (rest[0] != b'+' && rest[0] != b'-') || rest[3] != b':' {
        return false;
    }
    let (Some(off_hour), Some(off_minute)) = (two_digits(&rest[1..3]), two_digits(&rest[4..6]))
    else {
        return false;
    };
    off_hour <= 23 && off_minute <= 59
}

/// Exact against RFC 3339 `date-time` except for leap seconds: a `full-date`, a
/// `T` separator, and a `full-time`. A space separator is not accepted.
fn is_datetime(text: &str) -> bool {
    let (date_part, time_part) = match text.split_once('T').or_else(|| text.split_once('t')) {
        Some(pair) => pair,
        None => return false,
    };
    is_date(date_part) && is_full_time(time_part)
}

/// Exact against RFC 3339 `full-time` except for leap seconds (see
/// `is_full_time`).
fn is_time(text: &str) -> bool {
    is_full_time(text)
}

/// Structural against RFC 4122's layout: 8-4-4-4-12 hexadecimal digits,
/// case-insensitive. It does not check the version or variant nibbles and it
/// does not accept braces.
fn is_uuid(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    if bytes[8] != b'-' || bytes[13] != b'-' || bytes[18] != b'-' || bytes[23] != b'-' {
        return false;
    }
    bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| matches!(index, 8 | 13 | 18 | 23) || byte.is_ascii_hexdigit())
}

/// Pragmatic dotted decimal: four groups of 1-3 ASCII digits, each 0-255, with
/// no leading zeros except a group that is exactly "0". Rejects empty groups,
/// trailing dots and anything that is not a group, so `01.2.3.4` fails rather
/// than being read as octal.
fn is_ipv4(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|part| {
        !part.is_empty()
            && part.len() <= 3
            && part.bytes().all(|b| b.is_ascii_digit())
            && (part.len() == 1 || !part.starts_with('0'))
            && part.parse::<u32>().is_ok_and(|value| value <= 255)
    })
}

fn is_hex_group(token: &str) -> bool {
    !token.is_empty() && token.len() <= 4 && token.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Counts address groups; an embedded IPv4 address is valid only as the final
/// 32 bits and counts as two groups.
fn count_groups(groups: &[&str]) -> Option<usize> {
    let mut count = 0;
    for (index, token) in groups.iter().enumerate() {
        if token.contains('.') {
            if index != groups.len() - 1 || !is_ipv4(token) {
                return None;
            }
            count += 2;
        } else if is_hex_group(token) {
            count += 1;
        } else {
            return None;
        }
    }
    Some(count)
}

/// Pragmatic RFC 4291: eight 1-4-hex-digit groups, or `::` compression used
/// exactly once and compressing at least one group, with an IPv4 address allowed
/// in the final 32 bits. Zone IDs (`%…`) are rejected.
fn is_ipv6(text: &str) -> bool {
    if text.is_empty() || text.contains('%') {
        return false;
    }
    if let Some((left, right)) = text.split_once("::") {
        if right.contains("::") {
            return false;
        }
        let left: Vec<&str> = if left.is_empty() {
            Vec::new()
        } else {
            left.split(':').collect()
        };
        let right: Vec<&str> = if right.is_empty() {
            Vec::new()
        } else {
            right.split(':').collect()
        };
        let (Some(left_count), Some(right_count)) = (count_groups(&left), count_groups(&right))
        else {
            return false;
        };
        left_count + right_count < 8
    } else {
        let groups: Vec<&str> = text.split(':').collect();
        count_groups(&groups) == Some(8)
    }
}

/// Pragmatic, not RFC 5322: exactly one `@`, a non-empty local part without
/// whitespace, and a domain of non-empty dot-separated labels ending in a
/// top-level label of at least two ASCII letters. It rejects what is clearly
/// not an address and accepts the overwhelmingly common shapes.
fn is_email(text: &str) -> bool {
    let Some((local, domain)) = text.split_once('@') else {
        return false;
    };
    if domain.contains('@') {
        return false;
    }
    if local.is_empty() || domain.is_empty() {
        return false;
    }
    if local
        .bytes()
        .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
    {
        return false;
    }
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 || labels.iter().any(|label| label.is_empty()) {
        return false;
    }
    let tld = labels[labels.len() - 1];
    tld.len() >= 2 && tld.bytes().all(|b| b.is_ascii_alphabetic())
}

/// Structural against RFC 1034 labels: dot-separated labels of 1-63 ASCII
/// letters, digits or hyphens, never starting or ending with a hyphen, at most
/// 253 characters in total. A single label such as `localhost` is valid, and a
/// numeric-looking final label is accepted, so an IPv4 literal also passes as a
/// hostname; callers that must distinguish use `ipv4`.
fn is_hostname(text: &str) -> bool {
    if text.is_empty() || text.len() > 253 {
        return false;
    }
    text.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    })
}

/// Syntactic shape only: an ASCII ALPHA scheme start, then ALPHA, DIGIT, `+`,
/// `-` or `.`, a `:`, and a non-empty remainder. It does not validate the
/// authority or the path; `https`, `mailto`, `tel` and custom app schemes pass
/// as long as they name one.
fn is_uri(text: &str) -> bool {
    let Some((scheme, rest)) = text.split_once(':') else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    let mut bytes = scheme.bytes();
    match bytes.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
}

//! Date, datetime and time parsing.

/// Parses a date from three numeric components separated by `-` or `/`.
///
/// The first component is the year when it is greater than 31, otherwise the
/// last one is: `YYYY-M-D`, `YYYY/M/D`, `M/D/YYYY`, `M-D-YYYY` and `M/D/YY`
/// all parse. A two-digit year uses the pivot 00-29 -> 2000-2029, 30-99 ->
/// 1930-1999; a year above 9999, a month outside 1-12 or a day outside the
/// month is rejected.
pub(super) fn parse_date(text: &str) -> Option<String> {
    let text = text.trim();
    let (first, second, third) = split_date(text)?;

    let (year, month, day) = if first > 31 {
        (first, second, third)
    } else {
        (third, first, second)
    };
    let year = expand_year(year)?;

    if !is_leap_ok(year, month, day) {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

pub(super) fn expand_year(year: i32) -> Option<i32> {
    match year {
        0..=29 => Some(year + 2000),
        30..=99 => Some(year + 1900),
        100..=9999 => Some(year),
        _ => None,
    }
}

pub(super) fn split_date(text: &str) -> Option<(i32, i32, i32)> {
    let parts: Vec<&str> = text.split(['-', '/']).map(str::trim).collect();
    if parts.len() != 3 {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

pub(super) fn is_leap_ok(year: i32, month: i32, day: i32) -> bool {
    if !(1..=12).contains(&month) || day < 1 {
        return false;
    }
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            if leap { 29 } else { 28 }
        }
    };
    day <= days
}

/// Parses a date and time of day, then renders it with an explicit offset and
/// seven fractional digits: `YYYY-MM-DDTHH:MM:SS.fffffff±HH:MM`.
///
/// An input without a trailing offset is assumed to be UTC, never the
/// machine's local offset.
pub(super) fn parse_datetime(text: &str) -> Option<String> {
    let text = text.trim();
    let (date_part, rest) = text.split_once(['T', ' '])?;
    let (year, month, day) = {
        let (first, second, third) = split_date(date_part)?;
        if first > 31 {
            (first, second, third)
        } else {
            (third, first, second)
        }
    };
    let year = expand_year(year)?;
    if !is_leap_ok(year, month, day) {
        return None;
    }

    let mut rest = rest.trim();
    let mut offset = "+00:00".to_string();
    if let Some(stripped) = rest.strip_suffix(['Z', 'z']) {
        rest = stripped.trim_end();
    } else {
        // A trailing +HH:MM / -HH:MM offset, but not the sign of the hour.
        let bytes = rest.as_bytes();
        let mut found = None;
        for index in (1..bytes.len()).rev() {
            if bytes[index] == b'+' || bytes[index] == b'-' {
                let candidate = &rest[index..];
                if let Some(parsed) = parse_offset(candidate) {
                    found = Some((index, parsed));
                }
                break;
            }
        }
        if let Some((index, parsed)) = found {
            rest = rest[..index].trim_end();
            offset = parsed;
        }
    }

    let (hour, minute, second, fraction) = parse_clock(rest)?;
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{fraction}{offset}"
    ))
}

pub(super) fn parse_offset(text: &str) -> Option<String> {
    let text = text.trim();
    let sign = text.chars().next()?;
    if sign != '+' && sign != '-' {
        return None;
    }
    let body = &text[1..];
    let (hours, minutes) = match body.split_once(':') {
        Some((hours, minutes)) => (hours.parse::<i32>().ok()?, minutes.parse::<i32>().ok()?),
        None => {
            if body.len() == 4 {
                (body[..2].parse().ok()?, body[2..].parse().ok()?)
            } else if body.len() == 2 {
                (body.parse().ok()?, 0)
            } else {
                return None;
            }
        }
    };
    if hours > 14 || minutes > 59 {
        return None;
    }
    Some(format!("{sign}{hours:02}:{minutes:02}"))
}

/// Accepts `HH:MM` or `HH:MM:SS` on a 24-hour clock and renders it as
/// `HH:mm:ss`; a fractional part is accepted but dropped.
pub(super) fn parse_time(text: &str) -> Option<String> {
    let text = text.trim();
    let (hour, minute, second, _) = parse_clock(text)?;
    Some(format!("{hour:02}:{minute:02}:{second:02}"))
}

pub(super) fn parse_clock(text: &str) -> Option<(i32, i32, i32, String)> {
    let (clock, fraction) = match text.split_once('.') {
        Some((clock, fraction)) => (clock.trim(), normalize_fraction(fraction)?),
        None => (text.trim(), "0000000".to_string()),
    };

    let parts: Vec<&str> = clock.split(':').map(str::trim).collect();
    if parts.len() < 2 || parts.len() > 3 {
        return None;
    }
    let hour: i32 = parts[0].parse().ok()?;
    let minute: i32 = parts[1].parse().ok()?;
    let second: i32 = if parts.len() == 3 {
        parts[2].parse().ok()?
    } else {
        0
    };

    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) || !(0..=59).contains(&second) {
        return None;
    }
    Some((hour, minute, second, fraction))
}

pub(super) fn normalize_fraction(fraction: &str) -> Option<String> {
    let digits: String = fraction.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let mut padded = digits;
    while padded.len() < 7 {
        padded.push('0');
    }
    padded.truncate(7);
    Some(padded)
}

//! The `pattern` engine shared by the JSON Schema subset and response
//! validation (SPEC S-6).
//!
//! Both sites compile through [`compile`] and match through [`is_match`], so
//! they agree on one engine and on one rule: a pattern that cannot be compiled
//! is an error, never a silent non-match. `fancy-regex` adds the ECMA-262
//! constructs the Rust `regex` crate rejects, such as look-around and
//! backreferences, delegating the rest to the linear-time `regex-automata`
//! engine.

use fancy_regex::Regex;

/// The most quantifier and alternation constructs one pattern may contain.
///
/// The engine's cost is driven by these, not by the text it matches: a pattern
/// of N repeated groups costs superlinear time to compile and to run, and it is
/// recompiled for every string the schema validates. Measured on this engine,
/// 1 000 groups take about 29 ms, 4 000 about 374 ms and 32 000 about 17.6 s,
/// while the length of the text being matched makes no difference once the
/// group count is fixed. 512 units leaves a two-order-of-magnitude margin over
/// any hand-written pattern (a password or email rule uses single digits) and
/// caps the worst case at a few milliseconds.
pub const MAX_PATTERN_UNITS: usize = 512;

/// Counts the constructs that make a pattern expensive: every quantifier
/// (`*`, `+`, `?`, `{n,m}`) and every alternation. Escaped characters and
/// characters inside a character class do not count, so `a\*` and `[+?]` are
/// literals rather than quantifiers. This is a deterministic structural count,
/// not a timing measurement, so it is identical on every runtime.
pub fn complexity_units(pattern: &str) -> usize {
    let mut units = 0usize;
    let mut in_class = false;
    let mut previous = '\0';
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            // The escaped character is a literal, whatever it is.
            chars.next();
            previous = '\0';
            continue;
        }
        if in_class {
            if ch == ']' {
                in_class = false;
            }
            previous = ch;
            continue;
        }
        match ch {
            '[' => in_class = true,
            '*' | '+' => {
                // A quantifier with nothing to repeat is a compile error, which
                // the compile check reports; counting it here is harmless.
                if previous != '\0' {
                    units += 1;
                }
            }
            '?' => {
                // `(?:` and the look-around forms open a group, they do not
                // repeat what came before, so they are not quantifiers.
                if previous != '(' && previous != '\0' {
                    units += 1;
                }
            }
            '|' => units += 1,
            '{' => {
                // `{n}`/`{n,}`/`{n,m}` is a quantifier; a literal brace is not.
                let mut lookahead = String::new();
                for next in chars.by_ref() {
                    if next == '}' {
                        break;
                    }
                    if !(next.is_ascii_digit() || next == ',') {
                        break;
                    }
                    lookahead.push(next);
                }
                if !lookahead.is_empty() && lookahead.chars().any(|c| c.is_ascii_digit()) {
                    units += 1;
                }
            }
            _ => {}
        }
        previous = ch;
    }
    units
}

/// Compile an ECMA-262-style `pattern`. The error text names the pattern and
/// the engine's reason, ready to be reported by the caller.
pub fn compile(pattern: &str) -> Result<Regex, String> {
    Regex::new(pattern).map_err(|error| format!("pattern '{pattern}' cannot be compiled: {error}"))
}

/// Whether `pattern` is within the complexity budget. Reported separately from
/// compilation so the caller can distinguish "this pattern is too expensive"
/// from "this pattern is invalid".
pub fn is_within_budget(pattern: &str) -> bool {
    complexity_units(pattern) <= MAX_PATTERN_UNITS
}

/// Whether `text` matches `pattern`. A compilation failure and a runtime
/// failure (the backtracking limit) are both errors rather than a non-match.
pub fn is_match(pattern: &str, text: &str) -> Result<bool, String> {
    compile(pattern)?
        .is_match(text)
        .map_err(|error| format!("pattern '{pattern}' could not be evaluated: {error}"))
}

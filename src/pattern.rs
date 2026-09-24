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

/// Compile an ECMA-262-style `pattern`. The error text names the pattern and
/// the engine's reason, ready to be reported by the caller.
pub fn compile(pattern: &str) -> Result<Regex, String> {
    Regex::new(pattern).map_err(|error| format!("pattern '{pattern}' cannot be compiled: {error}"))
}

/// Whether `text` matches `pattern`. A compilation failure and a runtime
/// failure (the backtracking limit) are both errors rather than a non-match.
pub fn is_match(pattern: &str, text: &str) -> Result<bool, String> {
    compile(pattern)?
        .is_match(text)
        .map_err(|error| format!("pattern '{pattern}' could not be evaluated: {error}"))
}

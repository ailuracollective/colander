#![allow(dead_code)]

//! Golden-vector fixtures for colander's public API.
//!
//! The files under `tests/golden/vectors/` are frozen golden fixtures. The
//! `canonical`, `hash`, `semver`, `rules`, `validate`, `compile` and `errors`
//! groups were recorded from an external implementation and cannot be
//! regenerated, so treat them as read-only inputs. `describe` is the exception:
//! it covers an operation this crate has no external counterpart for, so it was
//! recorded from colander itself and is frozen the same way.
//!
//! A fixture that recorded a payload is compared byte-for-byte. A fixture that
//! recorded an error only has to fail the same way: the `SCREAMING_SNAKE`
//! contract code must match, while the message wording is colander's own and is
//! not compared.
//!
//! The fixtures speak colander's own vocabulary, so a document reaches the code
//! under test exactly as recorded. The only rewriting left is re-serialization:
//! [`ordered_document`] keeps document order and raw number literals,
//! [`canonical_document`] key-sorts for the comparisons that must ignore order.
//!
//! Entries in the per-group exclusions table are not replayed; each one is
//! either a gap in what the fixtures captured or a documented colander hardening
//! divergence.

use colander::json::{self, Json, JsonMap};

pub const VECTOR_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/vectors");

// ---------------------------------------------------------------------------
// Documents
// ---------------------------------------------------------------------------

/// A document held as text, re-serialized in its own document order with raw
/// number literals preserved. Text that is not JSON at all passes through
/// untouched, so malformed-input vectors reach the code under test exactly as
/// recorded.
pub fn ordered_document(text: &str) -> String {
    match json::parse(text) {
        Ok(node) => json::ordered(&node),
        Err(_) => text.to_string(),
    }
}

/// The same document key-sorted, for the comparisons that must ignore key
/// order: compiled documents are sorted, a fixture's own text usually is not.
pub fn canonical_document(text: &str) -> String {
    match json::parse(text) {
        Ok(node) => json::canonical(&node),
        Err(_) => text.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

pub struct Report {
    group: &'static str,
    checked: usize,
    skipped: Vec<(&'static str, &'static str)>,
    failures: Vec<String>,
}

impl Report {
    pub fn new(group: &'static str) -> Self {
        Self {
            group,
            checked: 0,
            skipped: Vec::new(),
            failures: Vec::new(),
        }
    }

    pub fn skip(&mut self, name: &str, reason: &'static str) {
        self.skipped.push((name.to_string().leak(), reason));
    }

    pub fn fail(&mut self, name: &str, detail: String) {
        self.failures.push(format!("  [{name}] {detail}"));
    }

    pub fn check_eq<T: PartialEq + std::fmt::Debug>(
        &mut self,
        name: &str,
        what: &str,
        expected: &T,
        actual: &T,
    ) {
        self.checked += 1;
        if expected != actual {
            self.fail(
                name,
                format!("{what}\n      fixture: {expected:?}\n      colander:  {actual:?}"),
            );
        }
    }

    pub fn check_json(&mut self, name: &str, what: &str, expected: &Json, actual: &Json) {
        self.checked += 1;
        let left = json::canonical(expected);
        let right = json::canonical(actual);
        if left != right {
            self.fail(
                name,
                format!("{what}\n      fixture: {left}\n      colander:  {right}"),
            );
        }
    }

    /// Resolves the fixture outcome against colander's. Returns colander's value when
    /// the fixture succeeded.
    pub fn expect<T>(
        &mut self,
        name: &str,
        recorded_error: Option<&str>,
        actual: std::result::Result<T, colander::ColanderError>,
    ) -> Option<T> {
        self.checked += 1;
        match (recorded_error, actual) {
            (None, Ok(value)) => Some(value),
            (None, Err(error)) => {
                self.fail(
                    name,
                    format!("fixture succeeded, colander failed: {}", error.message),
                );
                None
            }
            (Some(recorded), Ok(_)) => {
                self.fail(
                    name,
                    format!("fixture failed, colander succeeded. fixture: {recorded}"),
                );
                None
            }
            (Some(recorded), Err(error)) => {
                // Only the contract code is part of the surface: it is what a
                // caller branches on. The wording around it is colander's own and
                // is deliberately not compared.
                if let Some(code) = contract_code(recorded)
                    && !opens_with_code(&error.message, code)
                {
                    self.fail(
                        name,
                        format!(
                            "error code\n      fixture: {code}\n      colander:  {}",
                            error.message
                        ),
                    );
                }
                None
            }
        }
    }

    pub fn finish(self) {
        if !self.failures.is_empty() {
            panic!(
                "{} conformance failures ({} checks, {} skipped):\n{}",
                self.group,
                self.checked,
                self.skipped.len(),
                self.failures.join("\n")
            );
        }
        println!(
            "{}: {} checks passed, {} skipped",
            self.group,
            self.checked,
            self.skipped.len()
        );
    }
}

/// The `SCREAMING_SNAKE` contract code a fixture error exposes, if any.
///
/// The recorded errors are shaped `{Type}Exception: {message}`, and the code is
/// the first colon-separated token of the message when that token is
/// `SCREAMING_SNAKE`. Prose such as `Invalid form schema: ...` exposes no code,
/// and then only "both sides failed" is asserted.
pub fn contract_code(message: &str) -> Option<&str> {
    let body = match message.split_once(": ") {
        Some((exception, rest)) if exception.ends_with("Exception") => rest,
        _ => message,
    };
    let head = body.split(':').next()?.trim();
    let looks_like_code = head.len() >= 3
        && head.chars().any(|c| c.is_ascii_uppercase())
        && head
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
    looks_like_code.then_some(head)
}

/// Whether a colander error is introduced by `code`, as a whole token.
pub fn opens_with_code(message: &str, code: &str) -> bool {
    message
        .strip_prefix(code)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(':'))
}

// ---------------------------------------------------------------------------
// Vector loading
// ---------------------------------------------------------------------------

pub fn load(group: &str) -> Vec<JsonMap> {
    let path = format!("{VECTOR_DIR}/{group}.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {path}: {error}"));
    let parsed = json::parse(&text).unwrap_or_else(|error| panic!("{path}: {}", error.message));
    parsed
        .as_array()
        .unwrap_or_else(|| panic!("{path}: expected a JSON array"))
        .iter()
        .map(|entry| {
            entry
                .as_object()
                .expect("vector entries are objects")
                .clone()
        })
        .collect()
}

pub fn name_of(entry: &JsonMap) -> &str {
    json::get_str(entry, "name").expect("vector entries have a name")
}

pub fn recorded_error(entry: &JsonMap) -> Option<String> {
    json::get(entry, "error")
        .and_then(Json::as_str)
        .map(str::to_string)
}

/// Reads the exact raw JSON text the fixture fed to the code under test.
pub fn raw(entry: &JsonMap, key: &str) -> Option<String> {
    json::get_object(entry, "raw")
        .and_then(|object| json::get(object, key))
        .and_then(|value| match value {
            Json::Null => None,
            Json::String(text) => Some(text.clone()),
            // Older vectors stored the parsed node instead of the text.
            other => Some(json::ordered(other)),
        })
}

/// Like [`raw`], re-serialized in its own document order.
pub fn raw_document(entry: &JsonMap, key: &str) -> Option<String> {
    raw(entry, key).map(|text| ordered_document(&text))
}

/// Drives the exclusions table and asserts every listed name really exists.
pub fn excluded<'a>(report: &mut Report, entries: &'a [JsonMap], group: &str) -> Vec<&'a JsonMap> {
    let mut kept = Vec::new();
    for entry in entries {
        let name = name_of(entry);
        match exclusions(group)
            .iter()
            .find(|(excluded, _)| *excluded == name)
        {
            Some((_, reason)) => report.skip(name, reason),
            None => kept.push(entry),
        }
    }
    for (excluded_name, _) in exclusions(group) {
        assert!(
            entries.iter().any(|entry| name_of(entry) == *excluded_name),
            "{group}.json no longer contains the excluded entry '{excluded_name}'"
        );
    }
    kept
}

/// Entries that cannot be replayed one-for-one.
pub fn exclusions(group: &str) -> &'static [(&'static str, &'static str)] {
    match group {
        "canonical" => &[],
        "hash" => &[],
        "semver" => &[
            (
                "next: invalid-entry",
                "the fixture entry threw before echoing the published list; covered by a semver unit test",
            ),
            (
                "next: empty-string-entry",
                "the fixture entry threw before echoing the published list; covered by a semver unit test",
            ),
        ],
        "rules" => &[
            (
                "empty-json-array-value",
                "error comes from the test helper ParseValues, which the engine never sees",
            ),
            (
                "values-with-nested-array",
                "error comes from the test helper ParseValues, which the engine never sees",
            ),
            (
                "values-with-nested-object",
                "error comes from the test helper ParseValues, which the engine never sees",
            ),
        ],
        "validate" => &[],
        "compile" => &[],
        "errors" => &[],
        "describe" => &[],
        other => panic!("no exclusions table for group '{other}'"),
    }
}

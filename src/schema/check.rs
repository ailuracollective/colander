//! Recursive descent over a schema instance pair.

use crate::json::{self, Json};

use super::keywords::{check_enum_and_const, check_numeric_keywords, check_string_keywords};
use super::model::SchemaError;

use super::ErrorSink;
use super::keywords::check_type;

/// Deterministic work available to one top-level schema evaluation.
///
/// One unit is charged at every `check_into` entry, including boolean schemas
/// and probes made by combinators. The instance-size term leaves legitimate
/// work room while keeping a small instance from buying exponential work.
/// `depth` is the live recursion depth, bounded separately by
/// [`MAX_SCHEMA_DEPTH`](super::MAX_SCHEMA_DEPTH): a step budget cannot bound
/// recursion, because depth is at most the step count.
pub(super) struct EvalBudget {
    remaining: usize,
    depth: usize,
}

impl EvalBudget {
    pub(super) fn for_instance(instance: &Json) -> Self {
        Self {
            remaining: 10_000 + 20 * instance_node_count(instance),
            depth: 0,
        }
    }

    /// Why an evaluation stopped, so the caller reports the real cause.
    pub(super) fn consume(&mut self) -> Result<(), Stop> {
        if self.depth >= super::MAX_SCHEMA_DEPTH {
            return Err(Stop::Depth);
        }
        if self.remaining == 0 {
            return Err(Stop::Steps);
        }
        self.remaining -= 1;
        self.depth += 1;
        Ok(())
    }

    /// Leave one nesting level. Paired with every [`Self::consume`] that
    /// returned `Ok`, including on the early returns that stop evaluation.
    pub(super) fn leave(&mut self) {
        self.depth -= 1;
    }
}

/// The bounded outcome of one evaluation step.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stop {
    /// The step budget ran out.
    Steps,
    /// The nesting limit was reached.
    Depth,
}

impl Stop {
    /// The control-plane message, kept stable so every runtime and the
    /// conformance corpus can pin it byte for byte.
    pub(crate) fn message(self) -> &'static str {
        match self {
            Stop::Steps => super::SCHEMA_EVALUATION_LIMIT_MESSAGE,
            Stop::Depth => super::SCHEMA_DEPTH_LIMIT_MESSAGE,
        }
    }
}

fn instance_node_count(instance: &Json) -> usize {
    let descendants = match instance {
        Json::Array(items) => items.iter().map(instance_node_count).sum(),
        Json::Object(map) => map.values().map(instance_node_count).sum(),
        Json::Null | Json::Bool(_) | Json::Number(_) | Json::String(_) => 0,
    };
    1 + descendants
}

/// Evaluate `instance` against `schema`. `root` anchors local `$ref`s.
///
/// The schema tree under `root` is classified first (unsupported keywords and
/// wrong-typed values), so a malformed schema fails even when the instance
/// never reaches the offending subschema. A schema that fails classification
/// is **not** evaluated: the classification already decided the document is
/// not a usable schema, and evaluating it could follow a structure the
/// classifier rejected (a recursive `$ref`, for instance, would otherwise
/// recurse without bound).
pub fn check(schema: &Json, instance: &Json, root: &Json) -> Vec<SchemaError> {
    let errors = super::classify::classify(root);
    if !errors.is_empty() {
        return errors;
    }
    let mut errors = ErrorSink::default();
    let mut context = EvaluationContext::new(root, instance);
    context.check_into(schema, instance, &mut errors);
    errors.into_vec()
}

/// State one schema evaluation shares across every recursive call: the root
/// that anchors local `$ref` resolution, and the deterministic bounds.
///
/// Mirrors `compile::CompilationContext`, and for the same reason: the state
/// has the run's lifetime and never changes meaning between calls, so passing
/// it by hand only invited a caller to pair the wrong reference with the wrong
/// budget. Node-local values — the current schema, the current instance — stay
/// explicit arguments.
pub(super) struct EvaluationContext<'a> {
    root: &'a Json,
    budget: EvalBudget,
}

impl<'a> EvaluationContext<'a> {
    /// `instance` sets the budget: legitimate work is bounded by the instance
    /// size, amplified work is not (SPEC S-11).
    pub(super) fn new(root: &'a Json, instance: &Json) -> Self {
        Self {
            root,
            budget: EvalBudget::for_instance(instance),
        }
    }

    /// Instance evaluation alone, without re-classifying the schema.
    /// Combinator probes use this so a multi-branch schema is classified once,
    /// when `check` runs, and so each branch keeps its own error sink while
    /// sharing this run's budget. The `Option<Stop>` lets the caller propagate
    /// a bound failure instead of treating the probe as an ordinary mismatch.
    pub(super) fn evaluate_branch(
        &mut self,
        schema: &Json,
        instance: &Json,
    ) -> (ErrorSink, Option<Stop>) {
        let mut errors = ErrorSink::default();
        let stop = self.check_into(schema, instance, &mut errors);
        (errors, stop)
    }

    /// Evaluate one schema node. Returns the reason evaluation stopped, or
    /// `None` when this subtree finished. The nesting level is released on
    /// every exit path, including the early returns, by keeping the body in
    /// `check_node`.
    pub(super) fn check_into(
        &mut self,
        schema: &Json,
        instance: &Json,
        errors: &mut ErrorSink,
    ) -> Option<Stop> {
        if let Err(stop) = self.budget.consume() {
            errors.push_stop(stop);
            return Some(stop);
        }
        let stop = self.check_node(schema, instance, errors);
        self.budget.leave();
        stop
    }

    fn check_node(
        &mut self,
        schema: &Json,
        instance: &Json,
        errors: &mut ErrorSink,
    ) -> Option<Stop> {
        let Json::Object(schema) = schema else {
            // Boolean schemas are legal in 2020-12.
            if matches!(schema, Json::Bool(false)) {
                errors.push(SchemaError {
                    keyword: "false".to_string(),
                    message: "no value is valid against this schema".to_string(),
                });
            }
            return None;
        };

        if let Some(reference) = json::get_str(schema, "$ref") {
            match resolve_ref(reference, self.root) {
                Some(target) => {
                    // A reference's siblings still run after its target (S-1).
                    if let Some(stop) = self.check_into(target, instance, errors) {
                        return Some(stop);
                    }
                }
                None => errors.push(SchemaError {
                    keyword: "$ref".to_string(),
                    message: format!("cannot resolve reference '{reference}'"),
                }),
            }
        }

        check_type(schema, instance, errors);
        check_enum_and_const(schema, instance, errors);
        if let Some(stop) = self.check_combinators(schema, instance, errors) {
            return Some(stop);
        }
        if let Some(stop) = self.check_object_keywords(schema, instance, errors) {
            return Some(stop);
        }
        if let Some(stop) = self.check_array_keywords(schema, instance, errors) {
            return Some(stop);
        }
        check_string_keywords(schema, instance, errors);
        check_numeric_keywords(schema, instance, errors);
        None
    }
}

pub(super) fn resolve_ref<'a>(reference: &str, root: &'a Json) -> Option<&'a Json> {
    let pointer = reference.strip_prefix('#')?;
    if pointer.is_empty() {
        return Some(root);
    }
    let pointer = pointer.strip_prefix('/')?;

    let mut current = root;
    for raw in pointer.split('/') {
        let token = raw.replace("~1", "/").replace("~0", "~");
        current = match current {
            Json::Object(map) => map.get(&token)?,
            Json::Array(items) => items.get(token.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

/// JSON value equality with numeric-value semantics (`1` equals `1.0`).
pub fn value_equal(left: &Json, right: &Json) -> bool {
    match (left, right) {
        (Json::Number(_), Json::Number(_)) => numbers_equal(left, right),
        (Json::Array(left), Json::Array(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(l, r)| value_equal(l, r))
        }
        (Json::Object(left), Json::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right
                        .get(key)
                        .is_some_and(|other| value_equal(value, other))
                })
        }
        _ => left == right,
    }
}

/// Numeric equality by mathematical value, not by `f64` rounding: two numbers
/// are equal when they denote the same number, so `1` equals `1.0` but
/// `9007199254740993` does **not** equal `9007199254740992.0` (a double cannot
/// hold the former exactly). Integers within the exactly-representable double
/// range are compared as integers; beyond it, the `f64` reading is the best
/// available and is used consistently.
fn numbers_equal(left: &Json, right: &Json) -> bool {
    /// Exactly-known integer value, if the literal spells one. Read from the
    /// preserved number text rather than through `f64`, so an integer that no
    /// double can hold (`9007199254740993`) keeps its exact value, and
    /// `9007199254740993.0` is recognised as the same integer.
    fn exact_integer(value: &Json) -> Option<i128> {
        let text = value.as_number_text()?;
        let (negative, digits) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text.strip_prefix('+').unwrap_or(text)),
        };
        let (whole, fraction) = match digits.split_once('.') {
            Some((whole, fraction)) => (whole, fraction),
            None => (digits, ""),
        };
        if whole.is_empty()
            || !whole.bytes().all(|byte| byte.is_ascii_digit())
            || !fraction.bytes().all(|byte| byte == b'0')
        {
            return None;
        }
        let magnitude = whole.parse::<i128>().ok()?;
        Some(if negative { -magnitude } else { magnitude })
    }

    if let (Some(left), Some(right)) = (exact_integer(left), exact_integer(right)) {
        return left == right;
    }
    match (left.as_f64(), right.as_f64()) {
        (Some(left), Some(right)) => left == right,
        _ => left.as_number_text() == right.as_number_text(),
    }
}

/// A hash consistent with [`value_equal`]: equal values hash equal, so a
/// hash bucket can stand in for the pairwise comparison. Numbers hash by
/// their `f64` value (so `1`, `1.0` and `1e0` share a bucket) and fall back to
/// their literal text when they do not parse. Object keys are visited in
/// sorted order so key order does not change the hash.
pub fn value_hash(node: &Json) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn write(node: &Json, hasher: &mut DefaultHasher) {
        match node {
            Json::Null => 0u8.hash(hasher),
            Json::Bool(value) => {
                1u8.hash(hasher);
                value.hash(hasher);
            }
            Json::Number(_) => {
                2u8.hash(hasher);
                // `-0.0` and `0.0` are the same JSON Schema value, so they
                // must hash the same: `value_equal` says they are equal, and
                // equality has to imply equal hashes or the `uniqueItems`
                // buckets would never compare them (SPEC S-9).
                match node.as_f64() {
                    // Read through a binding and an `if`, not a `0.0` pattern:
                    // float patterns compare with `PartialEq`, so `Some(0.0)`
                    // would match `-0.0` too — correct, but only to whoever
                    // remembers that rule.
                    Some(number) => (if number == 0.0 { 0.0 } else { number })
                        .to_bits()
                        .hash(hasher),
                    None => node.as_number_text().hash(hasher),
                }
            }
            Json::String(text) => {
                3u8.hash(hasher);
                text.hash(hasher);
            }
            Json::Array(items) => {
                4u8.hash(hasher);
                items.len().hash(hasher);
                for item in items {
                    write(item, hasher);
                }
            }
            Json::Object(map) => {
                5u8.hash(hasher);
                map.len().hash(hasher);
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort_unstable();
                for key in keys {
                    key.hash(hasher);
                    write(&map[key], hasher);
                }
            }
        }
    }

    let mut hasher = DefaultHasher::new();
    write(node, &mut hasher);
    hasher.finish()
}

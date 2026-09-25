//! Expression shape validation: the fail-closed gate run before any
//! expression is evaluated.
//!
//! The evaluator alone is permissive about a few shapes the contract calls
//! invalid — extra operands are ignored, `null` operands are skipped by the
//! reference collectors and then crash the evaluator as "Invalid expression
//! node". This module makes every rejection happen in the analyzer, with a
//! `RULE_*` code, so authoring mistakes surface before evaluation and
//! evaluation never fails on a shape the analyzer already saw (SPEC R-12).

use crate::error::{ColanderError, Result};
use crate::json::{self, Json};

/// Operators and their accepted argument counts. `min` zero means the
/// operator folds over the whole list; `max` `None` means "no upper bound
/// declared" — the contract's R-11 keeps extra operands legal for those,
/// while the fixed-arity operators are checked against their exact count.
const OPERATORS: &[(&str, usize, Option<usize>)] = &[
    ("eq", 2, Some(2)),
    ("neq", 2, Some(2)),
    ("gt", 2, Some(2)),
    ("gte", 2, Some(2)),
    ("lt", 2, Some(2)),
    ("lte", 2, Some(2)),
    ("and", 0, None),
    ("or", 0, None),
    ("not", 1, Some(1)),
    ("empty", 1, Some(1)),
    ("coalesce", 0, None),
    ("add", 2, Some(2)),
    ("sub", 2, Some(2)),
    ("mul", 2, Some(2)),
    ("div", 2, Some(2)),
    ("count", 1, Some(1)),
    ("sum", 2, Some(2)),
];

fn arity_of(op: &str) -> Option<(usize, Option<usize>)> {
    OPERATORS
        .iter()
        .find(|(name, _, _)| *name == op)
        .map(|(_, min, max)| (*min, *max))
}

/// Validate one expression tree, reporting the first problem with `path`.
pub fn validate_expression_shape(node: Option<&Json>, path: &str) -> Result<()> {
    let Some(node) = node else {
        return Ok(());
    };
    if node.is_null() {
        return Ok(());
    }
    let Some(object) = node.as_object() else {
        return Err(ColanderError::new(format!(
            "RULE_INVALID_EXPRESSION: expression at {path} must be a 'ref', 'lit' or 'op' object."
        )));
    };

    if let Some(ref_value) = object.get("ref") {
        if !matches!(ref_value, Json::String(code) if !code.is_empty()) {
            return Err(ColanderError::new(format!(
                "RULE_INVALID_EXPRESSION: 'ref' at {path} must be a non-empty string."
            )));
        }
        return Ok(());
    }

    if let Some(literal) = object.get("lit") {
        // A node carrying both `lit` and `op` is a generator bug: the
        // evaluator silently prefers `lit`, so a mistyped `lit` would read as
        // a whole different expression. Fail instead.
        if object.contains_key("op") || object.contains_key("args") {
            return Err(ColanderError::new(format!(
                "RULE_INVALID_EXPRESSION: 'lit' at {path} must not be combined with 'op' or 'args'."
            )));
        }
        let _ = literal;
        return Ok(());
    }

    let op = object.get("op").and_then(Json::as_str).ok_or_else(|| {
        ColanderError::new(format!(
            "RULE_INVALID_EXPRESSION: expression at {path} is missing an 'op' string."
        ))
    })?;
    let (min, max) = arity_of(op).ok_or_else(|| {
        ColanderError::new(format!(
            "RULE_UNSUPPORTED_EXPRESSION_OPERATOR: expression at {path} uses unsupported operator '{op}'."
        ))
    })?;
    let args = object.get("args").and_then(Json::as_array).ok_or_else(|| {
        ColanderError::new(format!(
            "RULE_INVALID_EXPRESSION: expression '{op}' at {path} must carry an 'args' array."
        ))
    })?;

    if args.len() < min || max.is_some_and(|max| args.len() > max) {
        let expected = match max {
            Some(max) if max == min => format!("exactly {min}"),
            Some(max) => format!("between {min} and {max}"),
            None => format!("at least {min}"),
        };
        return Err(ColanderError::new(format!(
            "RULE_INVALID_EXPRESSION_ARITY: expression '{op}' at {path} has {} argument(s), expected {expected}.",
            args.len()
        )));
    }

    for (position, arg) in args.iter().enumerate() {
        if arg.is_null() {
            return Err(ColanderError::new(format!(
                "RULE_INVALID_EXPRESSION: argument {position} of '{op}' at {path} is null; every argument must be an expression."
            )));
        }
        validate_expression_shape(Some(arg), &format!("{path}/args/{position}"))?;
    }
    Ok(())
}

/// The `count`/`sum` positions address fields, not nested expressions: keep
/// the evaluator's requirement in the analyzer so a literal there fails with
/// a code rather than mid-evaluation.
pub fn validate_aggregate_arguments(node: Option<&Json>, path: &str) -> Result<()> {
    let Some(node) = node else {
        return Ok(());
    };
    let Some(object) = node.as_object() else {
        return Ok(());
    };
    let op = object.get("op").and_then(Json::as_str);
    if !matches!(op, Some("count") | Some("sum")) {
        return Ok(());
    }
    if let Some(args) = json::get_array(object, "args") {
        let op_name = op.unwrap_or("aggregate");
        for position in 0..args.len() {
            let arg = &args[position];
            let is_ref = arg
                .as_object()
                .and_then(|object| json::get_str(object, "ref"))
                .is_some_and(|code| !code.is_empty());
            if !is_ref {
                return Err(ColanderError::new(format!(
                    "RULE_INVALID_EXPRESSION: argument {position} of '{op_name}' at {path} must be a field reference."
                )));
            }
        }
    }
    Ok(())
}

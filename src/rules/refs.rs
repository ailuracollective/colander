//! Reference collectors and the row-scope check over rule expressions.

use std::collections::HashSet;

use indexmap::IndexMap;

use crate::error::{ColanderError, Result};
use crate::json::{self, Json};

/// Collects referenced field codes in document order.
///
/// Callers that report "the first unknown code" always name the first one in
/// document order. Deduplication uses a set so a wide expression costs
/// O(refs), not O(refs²) (SPEC R-14).
pub fn collect_references(expression: &Json) -> Vec<String> {
    let mut references = Vec::new();
    let mut seen = HashSet::new();
    collect_references_recursive(expression, &mut references, &mut seen);
    references
}

fn collect_references_recursive(
    node: &Json,
    references: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    let Some(object) = node.as_object() else {
        return;
    };

    if let Some(code) = json::get_str(object, "ref")
        && !code.is_empty()
        && seen.insert(code.to_string())
    {
        references.push(code.to_string());
        return;
    }

    let Some(args) = json::get_array(object, "args") else {
        return;
    };
    for arg in args {
        if !arg.is_null() {
            collect_references_recursive(arg, references, seen);
        }
    }
}

/// References that read a value directly, excluding aggregate positions: the
/// arguments of `count`/`sum` address a repeater (and, for `sum`, one of its
/// children) as the aggregate's subject, which the row-scope rule allows, so
/// they are not direct reads.
pub fn collect_direct_references(expression: &Json) -> Vec<String> {
    let mut references = Vec::new();
    let mut seen = HashSet::new();
    collect_direct_references_recursive(expression, &mut references, &mut seen);
    references
}

fn collect_direct_references_recursive(
    node: &Json,
    references: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    let Some(object) = node.as_object() else {
        return;
    };

    if let Some(code) = json::get_str(object, "ref")
        && !code.is_empty()
        && seen.insert(code.to_string())
    {
        references.push(code.to_string());
        return;
    }

    if matches!(json::get_str(object, "op"), Some("count" | "sum")) {
        return;
    }

    let Some(args) = json::get_array(object, "args") else {
        return;
    };
    for arg in args {
        if !arg.is_null() {
            collect_direct_references_recursive(arg, references, seen);
        }
    }
}

/// The repeater codes an aggregate (`count`/`sum`) is pointed at: its **first**
/// argument only. `sum` takes the repeater and then the child column to
/// accumulate across its rows, so a child code in the second argument is the
/// documented usage; a child code in the first is not.
pub fn collect_aggregate_repeaters(expression: &Json) -> Vec<String> {
    let mut references = Vec::new();
    let mut seen = HashSet::new();
    collect_aggregate_repeaters_recursive(expression, &mut references, &mut seen);
    references
}

fn collect_aggregate_repeaters_recursive(
    node: &Json,
    references: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    let Some(object) = node.as_object() else {
        return;
    };
    if matches!(json::get_str(object, "op"), Some("count" | "sum")) {
        if let Some(args) = json::get_array(object, "args")
            && let Some(first) = args.first()
            && let Some(code) = first.as_object().and_then(|arg| json::get_str(arg, "ref"))
            && !code.is_empty()
            && seen.insert(code.to_string())
        {
            references.push(code.to_string());
        }
        return;
    }
    let Some(args) = json::get_array(object, "args") else {
        return;
    };
    for arg in args {
        if !arg.is_null() {
            collect_aggregate_repeaters_recursive(arg, references, seen);
        }
    }
}

/// An aggregate's first argument names a repeater, never one of its children.
/// Accepting a child code there produced a silently wrong count (0) rather than
/// an error, because the row set has no rows for a code that is not a repeater.
pub fn validate_aggregate_targets(
    expression: Option<&Json>,
    path: &str,
    child_repeaters: &IndexMap<String, String>,
) -> Result<()> {
    let Some(expression) = expression else {
        return Ok(());
    };
    if expression.is_null() {
        return Ok(());
    }
    for code in collect_aggregate_repeaters(expression) {
        if let Some(enclosing) = child_repeaters.get(&code) {
            return Err(ColanderError::new(format!(
                "RULE_AGGREGATE_NOT_REPEATER: aggregate at {path} points at repeater-child code '{code}'; an aggregate must point at the repeater '{enclosing}' itself."
            )));
        }
    }
    Ok(())
}

/// Rejects direct reads of repeater-child codes from outside their row scope.
/// `home_repeater` is the enclosing repeater when the expression is a
/// `calculate` on one of its children, and `None` for predicates and
/// cross-field validations, which never run per row.
pub fn validate_row_scope(
    expression: Option<&Json>,
    path: &str,
    child_repeaters: &IndexMap<String, String>,
    home_repeater: Option<&str>,
) -> Result<()> {
    let Some(expression) = expression else {
        return Ok(());
    };
    if expression.is_null() {
        return Ok(());
    }

    for code in collect_direct_references(expression) {
        if let Some(enclosing) = child_repeaters.get(&code)
            && home_repeater != Some(enclosing.as_str())
        {
            return Err(ColanderError::new(format!(
                "RULE_INVALID_ROW_REFERENCE: expression at {path} references repeater-child code '{code}' outside the row scope of repeater '{enclosing}'."
            )));
        }
    }
    Ok(())
}

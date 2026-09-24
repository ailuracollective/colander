//! Reference collectors and the row-scope check over rule expressions.

use indexmap::IndexMap;

use crate::error::{ColanderError, Result};
use crate::json::{self, Json};

/// Collects referenced field codes in document order.
///
/// Callers that report "the first unknown code" always name the first one in
/// document order.
pub fn collect_references(expression: &Json) -> Vec<String> {
    let mut references = Vec::new();
    collect_references_recursive(expression, &mut references);
    references
}

fn collect_references_recursive(node: &Json, references: &mut Vec<String>) {
    let Some(object) = node.as_object() else {
        return;
    };

    if let Some(code) = json::get_str(object, "ref")
        && !code.is_empty()
    {
        if !references.iter().any(|item| item == code) {
            references.push(code.to_string());
        }
        return;
    }

    let Some(args) = json::get_array(object, "args") else {
        return;
    };
    for arg in args {
        if !arg.is_null() {
            collect_references_recursive(arg, references);
        }
    }
}

/// References that read a value directly, excluding aggregate positions: the
/// arguments of `count`/`sum` address a repeater (and, for `sum`, one of its
/// children) as the aggregate's subject, which the row-scope rule allows, so
/// they are not direct reads.
pub fn collect_direct_references(expression: &Json) -> Vec<String> {
    let mut references = Vec::new();
    collect_direct_references_recursive(expression, &mut references);
    references
}

fn collect_direct_references_recursive(node: &Json, references: &mut Vec<String>) {
    let Some(object) = node.as_object() else {
        return;
    };

    if let Some(code) = json::get_str(object, "ref")
        && !code.is_empty()
    {
        if !references.iter().any(|item| item == code) {
            references.push(code.to_string());
        }
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
            collect_direct_references_recursive(arg, references);
        }
    }
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

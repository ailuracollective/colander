//! Rule expression evaluation (JSON AST -> `Val`).

use indexmap::IndexMap;

use crate::error::{ColanderError, Result};
use crate::json::{self, Json};

use super::rows::RowSet;
use super::value::Val;

pub fn evaluate_expression(
    node: &Json,
    values: &IndexMap<String, Val>,
    rows: &RowSet,
) -> Result<Val> {
    let Some(object) = node.as_object() else {
        return Err(ColanderError::new("Invalid expression node."));
    };

    if let Some(code) = json::get_str(object, "ref")
        && !code.is_empty()
    {
        return Ok(values.get(code).cloned().unwrap_or(Val::Null));
    }

    if let Some(literal) = json::get(object, "lit") {
        return Ok(Val::from_json_node(literal));
    }

    let op = json::get_str(object, "op")
        .ok_or_else(|| ColanderError::new("Expression node is missing op."))?;
    let args = json::get_array(object, "args")
        .ok_or_else(|| ColanderError::new(format!("Expression '{op}' is missing args.")))?;

    match op {
        "eq" | "neq" | "gt" | "gte" | "lt" | "lte" => {
            let (left, right) = binary_args(op, args, values, rows)?;
            let ordering = compare_values(&left, &right)?;
            Ok(Val::Bool(match op {
                "eq" => ordering == std::cmp::Ordering::Equal,
                "neq" => ordering != std::cmp::Ordering::Equal,
                "gt" => ordering == std::cmp::Ordering::Greater,
                "gte" => ordering != std::cmp::Ordering::Less,
                "lt" => ordering == std::cmp::Ordering::Less,
                _ => ordering != std::cmp::Ordering::Greater,
            }))
        }
        "and" => {
            for arg in args {
                if !evaluate_expression(arg, values, rows)?.to_bool() {
                    return Ok(Val::Bool(false));
                }
            }
            Ok(Val::Bool(true))
        }
        "or" => {
            for arg in args {
                if evaluate_expression(arg, values, rows)?.to_bool() {
                    return Ok(Val::Bool(true));
                }
            }
            Ok(Val::Bool(false))
        }
        "not" => {
            let first = nth_arg(op, args, 0)?;
            Ok(Val::Bool(
                !evaluate_expression(first, values, rows)?.to_bool(),
            ))
        }
        "empty" => {
            let first = nth_arg(op, args, 0)?;
            Ok(Val::Bool(
                evaluate_expression(first, values, rows)?.is_empty(),
            ))
        }
        "coalesce" => {
            for arg in args {
                let value = evaluate_expression(arg, values, rows)?;
                if !value.is_empty() {
                    return Ok(value);
                }
            }
            Ok(Val::Null)
        }
        "add" | "sub" | "mul" | "div" => {
            let (left, right) = binary_args(op, args, values, rows)?;
            if left.is_empty() || right.is_empty() {
                return Ok(Val::Null);
            }
            let left = left.to_double()?;
            let right = right.to_double()?;
            let result = match op {
                "add" => left + right,
                "sub" => left - right,
                "mul" => left * right,
                _ => left / right,
            };
            Ok(if result.is_finite() {
                Val::Double(result)
            } else {
                Val::Null
            })
        }
        // R-7: aggregates over a repeater's rows. Both take field references,
        // so the analyzer collects them like any other `ref`.
        "count" => {
            let target = nth_arg(op, args, 0)?;
            let code = ref_code(target, op)?;
            Ok(Val::Int(row_count(values, rows, code) as i64))
        }
        "sum" => {
            let repeater = nth_arg(op, args, 0)?;
            let child = nth_arg(op, args, 1)?;
            let repeater_code = ref_code(repeater, op)?;
            let child_code = ref_code(child, op)?;
            Ok(sum_child(rows, repeater_code, child_code))
        }
        _ => Err(ColanderError::new(format!(
            "Unsupported expression operator '{op}'."
        ))),
    }
}

/// The field code a `count`/`sum` argument must name. Aggregates address fields
/// by reference, never by literal, so a non-reference argument is an error.
fn ref_code<'a>(node: &'a Json, op: &str) -> Result<&'a str> {
    node.as_object()
        .and_then(|object| json::get_str(object, "ref"))
        .filter(|code| !code.is_empty())
        .ok_or_else(|| ColanderError::new(format!("Expression '{op}' expects a field reference.")))
}

/// How many rows a repeater has: the row set when it carries rows, otherwise
/// the caller's count when it is a number, otherwise zero.
fn row_count(values: &IndexMap<String, Val>, rows: &RowSet, code: &str) -> usize {
    if let Some(row_list) = rows.rows(code) {
        return row_list.len();
    }
    if let Some(Val::Int(count)) = values.get(code) {
        return (*count).max(0) as usize;
    }
    0
}

/// The sum of a child code across a repeater's rows. A missing or non-numeric
/// child contributes zero; with no rows there is no sum. The result is always
/// a `Double`, which the number writer spells exactly like an integer when the
/// value is one.
fn sum_child(rows: &RowSet, repeater_code: &str, child_code: &str) -> Val {
    let Some(row_list) = rows.rows(repeater_code) else {
        return Val::Null;
    };
    if row_list.is_empty() {
        return Val::Null;
    }
    let mut total = 0.0;
    for row in row_list {
        if let Some(value) = row.get(child_code) {
            total += value.to_double().unwrap_or(0.0);
        }
    }
    if total.is_finite() {
        Val::Double(total)
    } else {
        Val::Null
    }
}

fn nth_arg<'a>(op: &str, args: &'a [Json], position: usize) -> Result<&'a Json> {
    args.get(position).ok_or_else(|| {
        ColanderError::new(format!(
            "Expression '{op}' requires at least {} argument(s).",
            position + 1
        ))
    })
}

fn binary_args(
    op: &str,
    args: &[Json],
    values: &IndexMap<String, Val>,
    rows: &RowSet,
) -> Result<(Val, Val)> {
    let left = evaluate_expression(nth_arg(op, args, 0)?, values, rows)?;
    let right = evaluate_expression(nth_arg(op, args, 1)?, values, rows)?;
    Ok((left, right))
}

/// Ordering contract: `null` sorts below every other value, two strings compare
/// lexically, NaN sorts below every non-NaN number, and `-0.0` equals `0.0`.
pub fn compare_values(left: &Val, right: &Val) -> Result<std::cmp::Ordering> {
    use std::cmp::Ordering;

    match (left, right) {
        (Val::Null, Val::Null) => return Ok(Ordering::Equal),
        (Val::Null, _) => return Ok(Ordering::Less),
        (_, Val::Null) => return Ok(Ordering::Greater),
        _ => {}
    }

    if let (Val::Str(left), Val::Str(right)) = (left, right) {
        return Ok(left.cmp(right));
    }

    let left = left.to_double()?;
    let right = right.to_double()?;
    let ordering = match (left.is_nan(), right.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
    };
    // One equality definition for every surface: `eq` here, and
    // `CALCULATED_VALUE_MISMATCH` in `Val::values_equal`, decide the same
    // pair the same way (SPEC V-8). `eq` is also the identity used by
    // `gt`/`lt`/`gte`/`lte`, so a difference within EPSILON now reads as
    // equality there too instead of ordering on a float difference the
    // contract considers insignificant. The tolerance is the shared
    // `EPSILON`; the two integer paths above are exact and exempt.
    if (left - right).abs() < super::EPSILON {
        return Ok(Ordering::Equal);
    }
    Ok(ordering)
}

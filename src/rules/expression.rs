//! Rule expression evaluation (JSON AST -> `Val`).

use indexmap::IndexMap;

use crate::error::{ColanderError, Result};
use crate::json::{self, Json};

use super::value::Val;

pub fn evaluate_expression(node: &Json, values: &IndexMap<String, Val>) -> Result<Val> {
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
            let (left, right) = binary_args(op, args, values)?;
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
                if !evaluate_expression(arg, values)?.to_bool() {
                    return Ok(Val::Bool(false));
                }
            }
            Ok(Val::Bool(true))
        }
        "or" => {
            for arg in args {
                if evaluate_expression(arg, values)?.to_bool() {
                    return Ok(Val::Bool(true));
                }
            }
            Ok(Val::Bool(false))
        }
        "not" => {
            let first = nth_arg(op, args, 0)?;
            Ok(Val::Bool(!evaluate_expression(first, values)?.to_bool()))
        }
        "empty" => {
            let first = nth_arg(op, args, 0)?;
            Ok(Val::Bool(evaluate_expression(first, values)?.is_empty()))
        }
        "coalesce" => {
            for arg in args {
                let value = evaluate_expression(arg, values)?;
                if !value.is_empty() {
                    return Ok(value);
                }
            }
            Ok(Val::Null)
        }
        "add" | "sub" | "mul" | "div" => {
            let (left, right) = binary_args(op, args, values)?;
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
        _ => Err(ColanderError::new(format!(
            "Unsupported expression operator '{op}'."
        ))),
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

fn binary_args(op: &str, args: &[Json], values: &IndexMap<String, Val>) -> Result<(Val, Val)> {
    let left = evaluate_expression(nth_arg(op, args, 0)?, values)?;
    let right = evaluate_expression(nth_arg(op, args, 1)?, values)?;
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
    Ok(match (left.is_nan(), right.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
    })
}

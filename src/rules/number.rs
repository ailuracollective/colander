//! Numeric normalization: decimal places, rounding and step snapping.

use crate::index::FieldInfo;
use crate::keys::field_type_names;

use super::value::Val;

const MAX_DECIMAL_PLACES: i32 = 10;
const DEFAULT_DECIMAL_PLACES: i32 = 2;

/// Normalizes a calculated value for the field it belongs to: integers are
/// rounded, numbers are snapped to `multipleOf` and rounded to the resolved
/// decimal places, and non-numeric or non-finite values pass through or become
/// `Val::Null`.
pub fn normalize_calculated_value(value: Val, field: &FieldInfo) -> Val {
    let numeric = match &value {
        Val::Double(number) => *number,
        Val::Int(number) => *number as f64,
        _ => return value,
    };

    if !numeric.is_finite() {
        return Val::Null;
    }

    if field.field_type == field_type_names::INTEGER {
        return Val::Double(numeric.round());
    }

    if field.field_type != field_type_names::NUMBER {
        return value;
    }

    let mut numeric = numeric;
    if let Some(step) = field.multiple_of
        && step > 0.0
    {
        numeric = snap_to_step(numeric, step);
    }

    let decimals = resolve_decimal_places(field.decimal_places, field.multiple_of);
    Val::Double(round_to_decimals(numeric, decimals))
}

fn resolve_decimal_places(decimal_places: Option<i32>, multiple_of: Option<f64>) -> i32 {
    match decimal_places {
        Some(explicit) => clamp_decimal_places(explicit),
        None => match multiple_of {
            Some(step) if step > 0.0 => decimal_places_from_step(step),
            _ => DEFAULT_DECIMAL_PLACES,
        },
    }
}

fn clamp_decimal_places(decimals: i32) -> i32 {
    decimals.clamp(0, MAX_DECIMAL_PLACES)
}

fn round_to_decimals(value: f64, decimals: i32) -> f64 {
    let decimals = clamp_decimal_places(decimals);
    if decimals <= 0 {
        return value.round();
    }
    let factor = 10f64.powi(decimals);
    let scaled = value * factor;
    // The scaled product can overflow to infinity for a finite input near the
    // top of the f64 range. V-5 promises the caller sees the unrepresentable
    // result, not a silent `null` from a later `is_finite` check, so the
    // overflow is returned as-is here and normalised by the caller.
    if !scaled.is_finite() {
        return value;
    }
    (scaled.round()) / factor
}

fn snap_to_step(value: f64, step: f64) -> f64 {
    if !value.is_finite() || !step.is_finite() || step <= 0.0 {
        return value;
    }

    let decimals = decimal_places_from_step(step);
    let factor = 10f64.powi(decimals);
    // The rounded doubles are cast to `i64` (saturating on overflow) and the
    // step multiplication wraps, so extreme inputs stay defined.
    let scaled_step = (step * factor).round() as i64;
    let scaled_value = (value * factor).round() as i64;
    let snapped = ((scaled_value as f64 / scaled_step as f64).round()) as i64;
    let snapped = snapped.wrapping_mul(scaled_step);
    snapped as f64 / factor
}

fn decimal_places_from_step(step: f64) -> i32 {
    let mut decimals = 0;
    let mut scaled = step;
    while decimals < MAX_DECIMAL_PLACES && (scaled.round_ties_even() - scaled).abs() > 1e-9 {
        scaled *= 10.0;
        decimals += 1;
    }
    decimals
}

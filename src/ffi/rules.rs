//! `colander_evaluate_rules`.

use std::ffi::c_char;

use crate::codec::json::JsonCodec;
use crate::json::{self, Json, JsonMap};
use crate::rules;

use super::envelope::{dispatch, optional_object, optional_string, require_string};

/// `colander_evaluate_rules`.
///
/// # Safety
/// `request` must be a valid NUL-terminated UTF-8 C string (or null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colander_evaluate_rules(request: *const c_char) -> *mut c_char {
    unsafe {
        dispatch(&JsonCodec, request, |request| {
            let form =
                json::parse_object(&require_string(request, "formSchemaJson")?, "form schema")?;
            let rules_root =
                json::parse_object(&require_string(request, "rulesSchemaJson")?, "rules schema")?;
            let ui = optional_string(request, "uiSchemaJson")?;

            let mut values = indexmap::IndexMap::new();
            if let Some(map) = optional_object(request, "values")? {
                for (key, value) in map {
                    values.insert(key.clone(), rules::Val::from_json_node(value));
                }
            }

            let evaluation = rules::evaluate_values(&form, &rules_root, &values, ui.as_deref())?;

            Ok(project_evaluation(&evaluation))
        })
    }
}

fn project_evaluation(evaluation: &rules::FormRuleEvaluationResult) -> Json {
    let mut out = JsonMap::new();
    out.insert("visibility".to_string(), bool_map(&evaluation.visibility));
    out.insert("enabled".to_string(), bool_map(&evaluation.enabled));
    out.insert("required".to_string(), bool_map(&evaluation.required));

    let mut calculated = JsonMap::new();
    for (key, value) in &evaluation.calculated_values {
        calculated.insert(key.clone(), value.to_json());
    }
    out.insert("calculatedValues".to_string(), Json::Object(calculated));

    out.insert(
        "validationErrors".to_string(),
        Json::Array(
            evaluation
                .validation_errors
                .iter()
                .map(|error| {
                    let mut entry = JsonMap::new();
                    entry.insert("code".to_string(), Json::String(error.code.clone()));
                    entry.insert("message".to_string(), Json::String(error.message.clone()));
                    Json::Object(entry)
                })
                .collect(),
        ),
    );

    Json::Object(out)
}

pub(super) fn bool_map(map: &indexmap::IndexMap<String, bool>) -> Json {
    let mut out = JsonMap::new();
    for (key, value) in map {
        out.insert(key.clone(), Json::Bool(*value));
    }
    Json::Object(out)
}

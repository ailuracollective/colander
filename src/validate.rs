//! Answer validation and normalization.

mod calculated;
mod constraints;
mod conversion;
mod datetime;
mod fields;
mod model;
mod repeater;

pub use model::{FormResponseFieldError, FormResponseValidationMode, FormResponseValidationResult};

use indexmap::IndexMap;

use crate::error::Result;
use crate::index::{self, AnswerFieldDefinition};
use crate::json::{self};
use crate::keys::field_type_names;
use crate::rules::{RowSet, Val};

use calculated::{apply_calculated_fields, evaluate_rules, flatten_for_rules};
use fields::{validate_known_top_level_keys, validate_scalar_field};
use repeater::validate_repeater;

/// Validate `answers_json` against the form/ui/rules triple.
pub fn validate(
    form_schema_json: &str,
    ui_schema_json: Option<&str>,
    rules_schema_json: Option<&str>,
    answers_json: &str,
    mode: FormResponseValidationMode,
) -> Result<FormResponseValidationResult> {
    let form_root = json::parse_object(form_schema_json, "form schema")?;
    // R-5: duplicates are rejected even when no rules document is supplied;
    // the answer index would otherwise validate against the last definition
    // in silence.
    index::ensure_unique_codes(&form_root)?;
    let fields_by_code = index::build_answer_index(&form_root)?;
    let repeaters_by_code: IndexMap<String, AnswerFieldDefinition> = fields_by_code
        .iter()
        .filter(|(_, field)| field.field_type == field_type_names::REPEATER)
        .map(|(code, field)| (code.clone(), field.clone()))
        .collect();

    let answers = json::parse_answers(answers_json)?;
    let mut errors: Vec<FormResponseFieldError> = Vec::new();

    validate_known_top_level_keys(&answers, &fields_by_code, &mut errors);

    let rule_values = flatten_for_rules(&answers, &fields_by_code);
    let mut rowset = RowSet::from_answers(&form_root, &answers);
    let evaluation = evaluate_rules(
        &form_root,
        rules_schema_json,
        ui_schema_json,
        &rule_values,
        &mut rowset,
    )?;

    let mut normalized: IndexMap<String, Val> = IndexMap::new();

    for field in fields_by_code.values() {
        if matches!(
            field.field_type.as_str(),
            field_type_names::GROUP | field_type_names::REPEATER | field_type_names::COMPONENT_REF
        ) {
            continue;
        }
        let value = answers.get(&field.code);
        validate_scalar_field(
            field,
            value,
            &evaluation,
            mode,
            &mut errors,
            &mut normalized,
        )?;
    }

    for (repeater_code, repeater) in &repeaters_by_code {
        let raw_rows = answers.get(repeater_code);
        validate_repeater(
            repeater,
            raw_rows,
            &evaluation,
            mode,
            &mut errors,
            &mut normalized,
        )?;
    }

    apply_calculated_fields(
        &answers,
        &fields_by_code,
        &evaluation,
        mode,
        &mut errors,
        &mut normalized,
    )?;

    if mode == FormResponseValidationMode::Complete {
        for error in &evaluation.validation_errors {
            errors.push(FormResponseFieldError {
                code: error.code.clone(),
                path: "/rules/validations".to_string(),
                message: error.message.clone(),
            });
        }
    }

    Ok(FormResponseValidationResult {
        normalized_answers_json: serialize_normalized(&normalized),
        errors,
    })
}

/// Serialize without indentation, preserving insertion order. String escaping
/// and double formatting follow the `json` module.
fn serialize_normalized(normalized: &IndexMap<String, Val>) -> String {
    let mut out = String::from("{");
    for (position, (key, value)) in normalized.iter().enumerate() {
        if position > 0 {
            out.push(',');
        }
        json::write_string(&mut out, key);
        out.push(':');
        out.push_str(&json::ordered(&value.to_json()));
    }
    out.push('}');
    out
}

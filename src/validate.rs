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
use crate::json::{self, Json, JsonMap};
use crate::keys::field_type_names;
use crate::rules::{RowSet, Val};
use crate::semantic::FormSemantics;

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
    // R-5: the checked semantic model rejects duplicate codes before the
    // response-specific answer index can silently select the last definition.
    let form_semantics = FormSemantics::from_form(&form_root)?;
    let mut response = ResponseSemantics::from_form(&form_root)?;
    let answers = json::parse_answers(answers_json)?;
    response.load_answers(&form_semantics, &answers);

    validate_with_semantics(
        &form_root,
        &form_semantics,
        &mut response,
        &answers,
        ui_schema_json,
        rules_schema_json,
        mode,
    )
}

/// Response-only answer index and row set; form topology and rule indexes stay shared.
#[derive(Debug)]
struct ResponseSemantics {
    answer_fields_by_code: IndexMap<String, AnswerFieldDefinition>,
    rows: RowSet,
}

impl ResponseSemantics {
    fn from_form(form_root: &JsonMap) -> Result<Self> {
        Ok(Self {
            answer_fields_by_code: index::build_answer_index(form_root)?,
            rows: RowSet::empty(),
        })
    }

    fn load_answers(&mut self, form_semantics: &FormSemantics, answers: &JsonMap) {
        self.rows = RowSet::from_answers_with_semantics(form_semantics, answers);
    }
}

fn validate_with_semantics(
    form_root: &JsonMap,
    form_semantics: &FormSemantics,
    response: &mut ResponseSemantics,
    answers: &JsonMap,
    ui_schema_json: Option<&str>,
    rules_schema_json: Option<&str>,
    mode: FormResponseValidationMode,
) -> Result<FormResponseValidationResult> {
    let answer_fields_by_code = &response.answer_fields_by_code;
    let rows = &mut response.rows;
    let mut errors: Vec<FormResponseFieldError> = Vec::new();

    validate_known_top_level_keys(answers, answer_fields_by_code, &mut errors);

    let rule_values = flatten_for_rules(answers, answer_fields_by_code);
    let evaluation = evaluate_rules(
        form_root,
        form_semantics,
        rules_schema_json,
        ui_schema_json,
        &rule_values,
        rows,
    )?;

    let mut normalized: IndexMap<String, Val> = IndexMap::new();

    for field in answer_fields_by_code.values() {
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

    for repeater in answer_fields_by_code
        .values()
        .filter(|field| field.field_type == field_type_names::REPEATER)
    {
        let raw_rows = answers.get(&repeater.code);
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
        answers,
        answer_fields_by_code,
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

    let errors = cap_errors(errors);

    Ok(FormResponseValidationResult {
        normalized_answers_json: serialize_normalized(&normalized),
        errors,
    })
}

/// At most `MAX_ERRORS` field errors are returned and their combined size is
/// bounded by `MAX_ERROR_BYTES`, so a caller cannot make one response huge by
/// choosing long field codes or answer keys: a present-but-over-long string is
/// elided inside the message (S-7's rule applied to response validation, not
/// only to the schema subset).
const MAX_ERRORS: usize = 100;
const MAX_ERROR_BYTES: usize = 64 * 1024;

/// The longest caller-controlled string echoed inside an error message.
pub(crate) const MAX_ECHO_CHARS: usize = 256;

/// Shortens a caller-controlled string for use in an error message, marking
/// what was elided so the reader knows the value was truncated.
pub(crate) fn ellipsize(text: &str) -> String {
    let count = text.chars().count();
    if count <= MAX_ECHO_CHARS {
        return text.to_string();
    }
    let head: String = text.chars().take(MAX_ECHO_CHARS).collect();
    format!("{head}… ({} more chars)", count - MAX_ECHO_CHARS)
}

fn cap_errors(errors: Vec<FormResponseFieldError>) -> Vec<FormResponseFieldError> {
    let total = errors.len();
    let mut capped: Vec<FormResponseFieldError> = Vec::new();
    let mut bytes = 0usize;
    for error in errors {
        if capped.len() >= MAX_ERRORS - 1 {
            break;
        }
        let cost = error.message.len() + error.path.len();
        if !capped.is_empty() && bytes + cost > MAX_ERROR_BYTES {
            break;
        }
        bytes += cost;
        capped.push(error);
    }
    if capped.len() < total {
        capped.push(FormResponseFieldError {
            code: "VALIDATION_ERRORS_TRUNCATED".to_string(),
            path: "/".to_string(),
            message: format!("Showing {} of {total} validation errors.", capped.len()),
        });
    }
    capped
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

pub(crate) fn project_result(result: &FormResponseValidationResult) -> Json {
    let mut out = JsonMap::new();
    out.insert(
        "normalizedAnswersJson".to_string(),
        Json::String(result.normalized_answers_json.clone()),
    );
    out.insert(
        "errors".to_string(),
        Json::Array(
            result
                .errors
                .iter()
                .map(|error| {
                    let mut entry = JsonMap::new();
                    entry.insert("code".to_string(), Json::String(error.code.clone()));
                    entry.insert("path".to_string(), Json::String(error.path.clone()));
                    entry.insert("message".to_string(), Json::String(error.message.clone()));
                    Json::Object(entry)
                })
                .collect(),
        ),
    );
    out.insert("isValid".to_string(), Json::Bool(result.is_valid()));
    Json::Object(out)
}

#[cfg(test)]
mod tests {
    use super::{FormResponseValidationMode, ResponseSemantics, validate_with_semantics};
    use crate::json;
    use crate::semantic::FormSemantics;

    #[test]
    fn validates_a_prepared_response_with_shared_form_semantics() {
        let form_text = r#"{"schemaVersion":"1.0.0","fields":[
            {"id":"lines","code":"lines","type":"repeater","items":[
                {"id":"qty","code":"qty","type":"integer"},
                {"id":"total","code":"total","type":"number","readOnly":true}]},
            {"id":"grand","code":"grand","type":"number","readOnly":true}]}"#;
        let answers_text = r#"{"lines":[{"qty":2}]}"#;
        let form = json::parse_object(form_text, "form schema").unwrap();
        let answers = json::parse_answers(answers_text).unwrap();
        let semantics = FormSemantics::from_form(&form).unwrap();
        let mut response = ResponseSemantics::from_form(&form).unwrap();
        response.load_answers(&semantics, &answers);
        let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
            "total":{"calculate":{"op":"mul","args":[{"ref":"qty"},{"lit":2}]}},
            "grand":{"calculate":{"op":"sum","args":[{"ref":"lines"},{"ref":"total"}]}}}}"#;
        let mode = FormResponseValidationMode::Draft;

        let result = validate_with_semantics(
            &form,
            &semantics,
            &mut response,
            &answers,
            None,
            Some(rules),
            mode,
        )
        .unwrap();

        assert!(result.is_valid(), "{:?}", result.errors);
        assert_eq!(
            result.normalized_answers_json,
            r#"{"lines":[{"qty":2,"total":4}],"grand":4}"#
        );
        assert_eq!(
            super::validate(form_text, None, Some(rules), answers_text, mode).unwrap(),
            result
        );
    }
}

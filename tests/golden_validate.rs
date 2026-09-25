mod common;

use colander::json::{self, Json, JsonMap};
use colander::validate;
use common::*;

// validate.json
// ---------------------------------------------------------------------------

#[test]
fn golden_validate() {
    let mut report = Report::new("validate");
    let entries = load("validate");
    let entries = excluded(&mut report, &entries, "validate");

    for entry in entries {
        let name = name_of(entry);
        let form = raw_document(entry, "formRaw").expect("form raw text");
        let ui = raw_document(entry, "uiRaw");
        let rules_text = raw_document(entry, "rulesRaw");
        let answers = raw_document(entry, "answersRaw").expect("answers raw text");
        let mode = match json::get_str(entry, "mode").unwrap_or("Draft") {
            "Complete" => validate::FormResponseValidationMode::Complete,
            _ => validate::FormResponseValidationMode::Draft,
        };

        let actual =
            validate::validate(&form, ui.as_deref(), rules_text.as_deref(), &answers, mode);
        let Some(actual) = report.expect(name, recorded_error(entry).as_deref(), actual) else {
            continue;
        };

        let Some(expected) = json::get_object(entry, "expected") else {
            report.fail(name, "no expected payload".to_string());
            continue;
        };

        let expected_answers = json::get_str(expected, "normalizedAnswersJson")
            .expect("normalizedAnswersJson is a string");
        report.check_eq(
            name,
            "normalizedAnswersJson",
            &expected_answers.to_string(),
            &actual.normalized_answers_json,
        );

        if let Some(expected_errors) = json::get_array(expected, "errors") {
            let actual_errors: Vec<Json> = actual
                .errors
                .iter()
                .map(|error| {
                    let mut item = JsonMap::new();
                    item.insert("code".to_string(), Json::String(error.code.clone()));
                    item.insert("path".to_string(), Json::String(error.path.clone()));
                    item.insert("message".to_string(), Json::String(error.message.clone()));
                    Json::Object(item)
                })
                .collect();
            report.check_json(
                name,
                "errors",
                &Json::Array(expected_errors.clone()),
                &Json::Array(actual_errors),
            );
        }
    }

    report.finish();
}

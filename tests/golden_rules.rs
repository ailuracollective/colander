mod common;

use colander::json::{self, Json, JsonMap};
use colander::rules;
use common::*;
use indexmap::IndexMap;

// rules.json
// ---------------------------------------------------------------------------

#[test]
fn golden_rules() {
    let mut report = Report::new("rules");
    let entries = load("rules");
    let entries = excluded(&mut report, &entries, "rules");

    for entry in entries {
        let name = name_of(entry);
        let form_text = raw_document(entry, "formRaw").expect("form raw text");
        let rules_text = raw_document(entry, "rulesRaw").expect("rules raw text");
        let ui_text = raw_document(entry, "uiRaw");

        // Some vectors drive the analyzer/dependency checks instead of the
        // evaluator; they have no `expected` payload, only a possible error.
        if name.starts_with("analyze:") {
            let actual = (|| {
                let form_root = json::parse_object(&form_text, "form schema")?;
                let rules_root = json::parse_object(&rules_text, "rules schema")?;
                rules::analyze(&form_root, &rules_root)
            })();
            let Some(actual) = report.expect(name, recorded_error(entry).as_deref(), actual) else {
                continue;
            };
            if let Some(expected) = json::get_object(entry, "analyze") {
                for (key, actual_items) in [
                    ("calculatedFieldIds", &actual.calculated_field_ids),
                    ("evaluationOrder", &actual.evaluation_order),
                ] {
                    let expected_items: Vec<Json> = json::get_array(expected, key)
                        .map(|items| items.to_vec())
                        .unwrap_or_default();
                    let actual_json: Vec<Json> = actual_items
                        .iter()
                        .map(|item| Json::String(item.clone()))
                        .collect();
                    report.check_json(
                        name,
                        &format!("analyze.{key}"),
                        &Json::Array(expected_items),
                        &Json::Array(actual_json),
                    );
                }
            }
            continue;
        }

        if name.starts_with("validateDependencies:") {
            // These vectors nest the fixture error one level down.
            let recorded = json::get_object(entry, "validateDependencies")
                .and_then(|object| json::get(object, "error"))
                .and_then(Json::as_str)
                .map(str::to_string);
            let actual = (|| {
                let form_root = json::parse_object(&form_text, "form schema")?;
                let rules_root = json::parse_object(&rules_text, "rules schema")?;
                rules::validate_dependencies(&form_root, &rules_root)
            })();
            report.expect::<()>(name, recorded.as_deref(), actual);
            continue;
        }

        let form_root = match json::parse_object(&form_text, "form schema") {
            Ok(root) => root,
            Err(error) => {
                report.expect::<()>(name, recorded_error(entry).as_deref(), Err(error));
                continue;
            }
        };
        let rules_root = match json::parse_object(&rules_text, "rules schema") {
            Ok(root) => root,
            Err(error) => {
                report.expect::<()>(name, recorded_error(entry).as_deref(), Err(error));
                continue;
            }
        };

        let mut values: IndexMap<String, rules::Val> = IndexMap::new();
        if let Some(map) = json::get_object(entry, "values") {
            for (key, value) in map {
                values.insert(key.clone(), rules::Val::from_json_node(value));
            }
        }

        let actual = rules::evaluate(
            &form_root,
            &rules_root,
            &values,
            ui_text.as_deref(),
            &mut rules::RowSet::empty(),
        );
        let Some(actual) = report.expect(name, recorded_error(entry).as_deref(), actual) else {
            continue;
        };

        let Some(expected) = json::get_object(entry, "expected") else {
            report.fail(name, "no expected payload".to_string());
            continue;
        };

        if let Some(map) = json::get_object(expected, "visibility") {
            for (key, value) in map {
                let got = actual.visibility.get(key).copied().unwrap_or(false);
                report.check_eq(
                    name,
                    &format!("visibility.{key}"),
                    &value.as_bool().unwrap_or(false),
                    &got,
                );
            }
        }
        if let Some(map) = json::get_object(expected, "enabled") {
            for (key, value) in map {
                let got = actual.enabled.get(key).copied().unwrap_or(false);
                report.check_eq(
                    name,
                    &format!("enabled.{key}"),
                    &value.as_bool().unwrap_or(false),
                    &got,
                );
            }
        }
        if let Some(map) = json::get_object(expected, "required") {
            for (key, value) in map {
                let got = actual.required.get(key).copied().unwrap_or(false);
                report.check_eq(
                    name,
                    &format!("required.{key}"),
                    &value.as_bool().unwrap_or(false),
                    &got,
                );
            }
        }

        let mut calculated = JsonMap::new();
        for (key, value) in &actual.calculated_values {
            calculated.insert(key.clone(), value.to_json());
        }
        let expected_calculated = json::get_object(expected, "calculatedValues")
            .cloned()
            .unwrap_or_default();
        report.check_json(
            name,
            "calculatedValues",
            &Json::Object(expected_calculated),
            &Json::Object(calculated),
        );

        let expected_errors: Vec<Json> = json::get_array(expected, "validationErrors")
            .cloned()
            .unwrap_or_default();
        let actual_errors: Vec<Json> = actual
            .validation_errors
            .iter()
            .map(|error| {
                let mut item = JsonMap::new();
                item.insert("code".to_string(), Json::String(error.code.clone()));
                item.insert("message".to_string(), Json::String(error.message.clone()));
                Json::Object(item)
            })
            .collect();
        report.check_json(
            name,
            "validationErrors",
            &Json::Array(expected_errors),
            &Json::Array(actual_errors),
        );
    }

    report.finish();
}

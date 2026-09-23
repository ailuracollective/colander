mod common;

use colander::json::{self, Json};
use colander::rules;
use common::*;

// errors.json
// ---------------------------------------------------------------------------

#[test]
fn golden_errors() {
    let mut report = Report::new("errors");
    let entries = load("errors");
    let entries = excluded(&mut report, &entries, "errors");

    for entry in entries {
        let name = name_of(entry);
        let call = json::get_str(entry, "call").expect("call");
        let input = json::get_object(entry, "input").expect("input");
        let recorded = recorded_error(entry);

        if call.starts_with("json.parseObject") {
            let raw_text = json::get_str(input, "form").expect("form text");
            let label = json::get_str(input, "label").unwrap_or("label");
            let actual = json::parse_object(raw_text, label).map(Json::Object);
            report.expect(name, recorded.as_deref(), actual);
            continue;
        }

        if call.starts_with("index.build") {
            let form_text = ordered_document(json::get_str(input, "form").expect("form text"));
            let root = match json::parse_object(&form_text, "form schema") {
                Ok(root) => root,
                Err(error) => {
                    report.expect::<()>(name, recorded.as_deref(), Err(error));
                    continue;
                }
            };
            let actual = if call.starts_with("index.buildByCode") {
                colander::index::build_by_code(&root).map(|_| ())
            } else {
                colander::index::build_by_id(&root).map(|_| ())
            };
            report.expect(name, recorded.as_deref(), actual);
            continue;
        }

        if call.starts_with("rules.validateDependencies") {
            let form_text = ordered_document(json::get_str(input, "form").expect("form text"));
            let rules_text = json::get_str(input, "rules")
                .map(ordered_document)
                .unwrap_or_else(|| "{}".to_string());
            let actual = (|| {
                let form_root = json::parse_object(&form_text, "form schema")?;
                let rules_root = json::parse_object(&rules_text, "rules schema")?;
                rules::validate_dependencies(&form_root, &rules_root)
            })();
            report.expect(name, recorded.as_deref(), actual);
            continue;
        }

        if call.starts_with("rules.analyze") {
            let form_text = ordered_document(json::get_str(input, "form").expect("form text"));
            let rules_text = json::get_str(input, "rules")
                .map(ordered_document)
                .unwrap_or_else(|| "{}".to_string());
            let actual = (|| {
                let form_root = json::parse_object(&form_text, "form schema")?;
                let rules_root = json::parse_object(&rules_text, "rules schema")?;
                rules::analyze(&form_root, &rules_root).map(|_| ())
            })();
            report.expect(name, recorded.as_deref(), actual);
            continue;
        }

        report.fail(name, format!("unknown call '{call}'"));
    }

    report.finish();
}

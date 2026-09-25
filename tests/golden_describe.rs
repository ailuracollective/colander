mod common;

use colander::json::{self, JsonMap};
use colander::{compile, describe};
use common::*;

// describe.json
// ---------------------------------------------------------------------------

/// Replays the `describe` group.
///
/// A described field is compared as a whole, so one fixture pins the identifier,
/// the pointer, the parent, the type and both baseline flags together. The
/// pointers are the point: a caller maps an `errors[].path` back to a field with
/// them, so a fixture that only checked the codes would not catch a pointer
/// drifting away from the string `colander_validate_response` reports.
#[test]
fn golden_describe() {
    let mut report = Report::new("describe");
    let entries = load("describe");
    let entries = excluded(&mut report, &entries, "describe");

    for entry in entries {
        let name = name_of(entry);
        let form_root = raw_document(entry, "formRaw").expect("form raw text");
        let ui = raw_document(entry, "uiRaw");
        let rules_text = raw_document(entry, "rulesRaw");
        let components = components_of(entry);

        let actual = || -> colander::Result<json::Json> {
            let compiled = compile::compile(
                &form_root,
                ui.as_deref(),
                rules_text.as_deref(),
                &components,
            )?;
            describe::describe(&compiled)
        }();

        let Some(actual) = report.expect(name, recorded_error(entry).as_deref(), actual) else {
            continue;
        };

        let expected = json::get(entry, "expected").expect("a successful fixture has a result");
        report.check_json(name, "described form", expected, &actual);

        // The description reports the compiled triple, so the hash it returns is
        // the hash of the documents it described. A caller pins a form version
        // with it, which only holds if the two agree.
        let described = actual
            .as_object()
            .and_then(|result| json::get_str(result, "contentHash"))
            .unwrap_or_default()
            .to_string();
        let rehashed = compile::compile(
            &form_root,
            ui.as_deref(),
            rules_text.as_deref(),
            &components,
        )
        .map(|recompiled| recompiled.content_hash)
        .unwrap_or_default();
        report.check_eq(
            name,
            "content hash of the described triple",
            &rehashed,
            &described,
        );
    }

    report.finish();
}

fn components_of(entry: &JsonMap) -> Vec<compile::ComponentVersionData> {
    json::get_array(entry, "components")
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    let object = item.as_object().expect("components are objects");
                    compile::ComponentVersionData {
                        code: json::get_str(object, "code")
                            .unwrap_or_default()
                            .to_string(),
                        version: json::get_str(object, "version")
                            .unwrap_or_default()
                            .to_string(),
                        form_schema_json: json::get_str(object, "formSchemaJson")
                            .map(ordered_document)
                            .unwrap_or_default(),
                        ui_schema_json: json::get_str(object, "uiSchemaJson").map(ordered_document),
                        content_hash: json::get_str(object, "contentHash").map(str::to_string),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

mod common;

use colander::json::{self};
use colander::{compile, hash};
use common::*;

// compile.json
// ---------------------------------------------------------------------------

#[test]
fn golden_compile() {
    let mut report = Report::new("compile");
    let entries = load("compile");
    let entries = excluded(&mut report, &entries, "compile");

    for entry in entries {
        let name = name_of(entry);
        let form_root = raw_document(entry, "formRaw").expect("form raw text");
        let ui = raw_document(entry, "uiRaw");
        let rules_text = raw_document(entry, "rulesRaw");

        let mut components = Vec::new();
        if let Some(items) = json::get_array(entry, "components") {
            for item in items {
                let object = item.as_object().expect("components are objects");
                components.push(compile::ComponentVersionData {
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
                });
            }
        }

        let actual = compile::compile(
            &form_root,
            ui.as_deref(),
            rules_text.as_deref(),
            &components,
        );
        let Some(actual) = report.expect(name, recorded_error(entry).as_deref(), actual) else {
            continue;
        };

        let Some(expected) = json::get_object(entry, "expected") else {
            report.fail(name, "no expected payload".to_string());
            continue;
        };

        let expected_text = |key: &str| json::get_str(expected, key).map(canonical_document);
        let canonical = |text: &str| canonical_document(text);
        report.check_eq(
            name,
            "formSchemaJson",
            &expected_text("formSchemaJson").unwrap_or_default(),
            &canonical(&actual.form_schema_json),
        );
        report.check_eq(
            name,
            "uiSchemaJson",
            &expected_text("uiSchemaJson"),
            &actual.ui_schema_json.as_deref().map(canonical),
        );
        report.check_eq(
            name,
            "rulesSchemaJson",
            &expected_text("rulesSchemaJson"),
            &actual.rules_schema_json.as_deref().map(canonical),
        );
        report.check_eq(
            name,
            "dependencyMetadataJson",
            &expected_text("dependencyMetadataJson").unwrap_or_default(),
            &canonical(&actual.dependency_metadata_json),
        );
        // The digest covers the fixture's own compiled documents, so it pins
        // the payload shape, its separators and the digest. colander's published
        // digest is the same builder over its own output, which is key-sorted
        // and is therefore compared canonically above.
        let expected_hash = json::get_str(expected, "contentHash").unwrap_or_default();
        let from_fixture = json::get_str(expected, "formSchemaJson")
            .map(|form| {
                hash::canonical_payload(
                    form,
                    json::get_str(expected, "uiSchemaJson"),
                    json::get_str(expected, "rulesSchemaJson"),
                )
                .map(|payload| hash::sha256_hex(payload.as_bytes()))
                .unwrap_or_default()
            })
            .unwrap_or_default();
        report.check_eq(
            name,
            "contentHash (fixture documents)",
            &expected_hash.to_string(),
            &from_fixture,
        );
        report.check_eq(
            name,
            "contentHash (compiled documents)",
            &hash::content_hash(
                &actual.form_schema_json,
                actual.ui_schema_json.as_deref(),
                actual.rules_schema_json.as_deref(),
            )
            .unwrap_or_default(),
            &actual.content_hash,
        );
    }

    report.finish();
}

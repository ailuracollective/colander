mod common;

use colander::hash;
use colander::json::{self};
use common::*;

// hash.json
// ---------------------------------------------------------------------------

#[test]
fn golden_hash() {
    let mut report = Report::new("hash");
    let entries = load("hash");
    let entries = excluded(&mut report, &entries, "hash");

    for entry in entries {
        let name = name_of(entry);
        let form = json::get_str(entry, "form").expect("the form document is a string");
        let ui = json::get_str(entry, "ui");
        let rules_json = json::get_str(entry, "rules");

        // The digest covers the three documents exactly as the fixture stored
        // them, so the payload shape, its separators, the document order, the
        // escaping and the digest itself are all pinned by the recorded value.
        let actual = || -> colander::Result<String> {
            let payload = hash::canonical_payload(form, ui, rules_json)?;
            Ok(hash::sha256_hex(payload.as_bytes()))
        }();

        let Some(actual) = report.expect(name, recorded_error(entry).as_deref(), actual) else {
            continue;
        };
        let expected = json::get_str(entry, "hash").expect("hash is a string");
        report.check_eq(name, "content hash", &expected.to_string(), &actual);

        // And the published entry point is that same computation.
        let neutral = hash::canonical_payload(form, ui, rules_json)
            .map(|payload| hash::sha256_hex(payload.as_bytes()));
        report.check_eq(
            name,
            "content hash (entry point)",
            &hash::content_hash(form, ui, rules_json).unwrap_or_default(),
            &neutral.unwrap_or_default(),
        );
    }

    report.finish();
}

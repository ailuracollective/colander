mod common;

use colander::json::{self};
use common::*;

// canonical.json
// ---------------------------------------------------------------------------

#[test]
fn golden_canonical() {
    let mut report = Report::new("canonical");
    let entries = load("canonical");
    let entries = excluded(&mut report, &entries, "canonical");

    for entry in entries {
        let name = name_of(entry);
        let text =
            json::get_str(entry, "rawInput").expect("canonical vectors carry the raw input text");
        let expected = json::get_str(entry, "canonical").expect("canonical vectors carry output");

        match json::parse(text) {
            Ok(node) => report.check_eq(
                name,
                "canonical JSON",
                &expected.to_string(),
                &json::canonical(&node),
            ),
            Err(error) => report.fail(
                name,
                format!("colander cannot parse the raw input: {}", error.message),
            ),
        }
    }

    report.finish();
}

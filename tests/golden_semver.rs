mod common;

use colander::json::{self};
use colander::semver;
use common::*;

// semver.json
// ---------------------------------------------------------------------------

#[test]
fn golden_semver() {
    let mut report = Report::new("semver");
    let entries = load("semver");
    let entries = excluded(&mut report, &entries, "semver");

    for entry in entries {
        let name = name_of(entry);

        if let Some(published) = json::get_array(entry, "published") {
            let published: Vec<String> = published
                .iter()
                .map(|item| item.as_str().unwrap_or_default().to_string())
                .collect();
            // The fixtures were recorded against the patch-only behaviour, so
            // they replay on the patch path.
            let actual = semver::next_version(&published, semver::Bump::Patch);
            let Some(actual) = report.expect(name, recorded_error(entry).as_deref(), actual) else {
                continue;
            };
            let expected = json::get_str(entry, "next").expect("next is a string");
            report.check_eq(name, "next version", &expected.to_string(), &actual);
            continue;
        }

        if let Some(version) = json::get_str(entry, "version") {
            let actual = semver::ensure_valid(version);
            report.expect(name, recorded_error(entry).as_deref(), actual);
            continue;
        }

        if let (Some(left), Some(right)) =
            (json::get_str(entry, "left"), json::get_str(entry, "right"))
        {
            let actual = semver::compare(left, right).map(|ordering| match ordering {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Greater => 1,
            });
            let Some(actual) = report.expect(name, recorded_error(entry).as_deref(), actual) else {
                continue;
            };
            let expected = json::get_i64(entry, "result").expect("result is an integer");
            report.check_eq(name, "compare", &expected, &actual);
            continue;
        }

        report.fail(name, "unrecognised semver vector shape".to_string());
    }

    report.finish();
}

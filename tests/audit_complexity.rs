//! Regression tests for the third adversarial audit (complexity layer).
//!
//! Each test pins a guarantee that the audit broke. The timing ceilings are
//! deliberately loose — they are there to catch a return of the quadratic or
//! cubic behaviour (which cost seconds to minutes at these sizes), not to
//! measure performance. At the sizes used here the fixed code runs in
//! milliseconds, and the removed behaviour ran one to three orders of
//! magnitude slower, so a five-second ceiling separates them with room for a
//! slow machine.

use std::time::{Duration, Instant};

use colander::compile::{ComponentVersionData, compile};
use colander::json;
use colander::rules::validate_dependencies;
use colander::schema::validate_text;
use colander::validate::{FormResponseValidationMode, validate};

/// Loose ceiling that a return of the audited super-linear behaviour breaks.
const CEILING: Duration = Duration::from_secs(5);

fn timed(label: &str, body: impl FnOnce()) {
    let start = Instant::now();
    body();
    let elapsed = start.elapsed();
    assert!(
        elapsed < CEILING,
        "{label} took {elapsed:?}, over the {CEILING:?} ceiling: a super-linear path is back"
    );
}

// ---------------------------------------------------------------------------
// S-9: `uniqueItems` is linear and still exact.
// ---------------------------------------------------------------------------

#[test]
fn unique_items_handles_twenty_thousand_values_without_a_pairwise_scan() {
    let instance: String = (0..20_000)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let instance = format!("[{instance}]");
    timed("uniqueItems 20k distinct", || {
        validate_text(
            r#"{"type":"array","uniqueItems":true}"#,
            &instance,
            "instance",
        )
        .unwrap();
    });
}

#[test]
fn unique_items_still_compares_numbers_by_value_and_catches_a_late_duplicate() {
    // `1` and `1.0` are the same JSON Schema value, so this array is not unique.
    let error = validate_text(
        r#"{"type":"array","uniqueItems":true}"#,
        "[1,1.0]",
        "instance",
    )
    .unwrap_err();
    assert!(error.message.contains("uniqueItems"), "{error}");

    // A duplicate at the very end of a large array is still found.
    let mut items: Vec<String> = (0..5_000).map(|value| value.to_string()).collect();
    items.push("0".to_string());
    let instance = format!("[{}]", items.join(","));
    let error = validate_text(
        r#"{"type":"array","uniqueItems":true}"#,
        &instance,
        "instance",
    )
    .unwrap_err();
    assert!(error.message.contains("index 5000 repeats"), "{error}");
}

// ---------------------------------------------------------------------------
// R-14: chained per-row calculations are linear, and a row reads its sibling.
// ---------------------------------------------------------------------------

/// `ch0` is submitted; `ch1 = ch0 * 2`; `ch2 = ch1 * 2`.
fn chained_rows(rows: usize) -> (String, String, String) {
    let form = r#"{"schemaVersion":"1.0.0","fields":[{"id":"lines","code":"lines","type":"repeater","items":[
        {"id":"ch0","code":"ch0","type":"number"},
        {"id":"ch1","code":"ch1","type":"number","readOnly":true},
        {"id":"ch2","code":"ch2","type":"number","readOnly":true}]}]}"#
    .to_string();
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "ch1":{"calculate":{"op":"mul","args":[{"ref":"ch0"},{"lit":2}]}},
        "ch2":{"calculate":{"op":"mul","args":[{"ref":"ch1"},{"lit":2}]}}}}"#
        .to_string();
    let mut answers = String::from("{\"lines\":[");
    for row in 0..rows {
        if row > 0 {
            answers.push(',');
        }
        answers.push_str(&format!("{{\"ch0\":{}}}", row + 1));
    }
    answers.push_str("]}");
    (form, rules, answers)
}

#[test]
fn chained_per_row_calculations_stay_linear_in_rows() {
    timed("chained per-row calculations, 20k rows", || {
        let (form, rules, answers) = chained_rows(20_000);
        let result = validate(
            &form,
            None,
            Some(&rules),
            &answers,
            FormResponseValidationMode::Draft,
        )
        .unwrap();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert!(result.normalized_answers_json.contains(r#""ch2":80000"#));
    });
}

#[test]
fn a_row_calculation_reads_its_own_sibling_value() {
    // `ch2` depends on `ch1`, which was calculated for the same row: the row
    // scope must see the row's value, not the whole-column array.
    let (form, rules, answers) = chained_rows(3);
    let result = validate(
        &form,
        None,
        Some(&rules),
        &answers,
        FormResponseValidationMode::Draft,
    )
    .unwrap();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert!(
        result
            .normalized_answers_json
            .contains(r#"{"ch0":1,"ch1":2,"ch2":4}"#),
        "{}",
        result.normalized_answers_json
    );
    // Outside row scope the aggregate still sees the whole column.
    let with_total = r#"{"schemaVersion":"1.0.0","fields":[{"id":"lines","code":"lines","type":"repeater","items":[
        {"id":"ch0","code":"ch0","type":"number"},
        {"id":"ch1","code":"ch1","type":"number","readOnly":true}]},
        {"id":"sum","code":"sum","type":"number","readOnly":true}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "ch1":{"calculate":{"op":"mul","args":[{"ref":"ch0"},{"lit":2}]}},
        "sum":{"calculate":{"op":"sum","args":[{"ref":"lines"},{"ref":"ch1"}]}}}}"#;
    let result = validate(
        with_total,
        None,
        Some(rules),
        r#"{"lines":[{"ch0":1},{"ch0":2},{"ch0":3}]}"#,
        FormResponseValidationMode::Draft,
    )
    .unwrap();
    assert!(
        result.normalized_answers_json.contains(r#""sum":12"#),
        "{}",
        result.normalized_answers_json
    );
}

// ---------------------------------------------------------------------------
// R-14: dependency analysis is linear in fields and edges.
// ---------------------------------------------------------------------------

fn dense_rules(fields: usize) -> (String, String) {
    let half = fields / 2;
    let mut form_fields = String::new();
    let mut rule_fields = String::new();
    for index in 0..fields {
        form_fields.push_str(&format!(
            "{{\"id\":\"f{index}\",\"code\":\"c{index}\",\"type\":\"number\",\"readOnly\":true}},"
        ));
        let args: Vec<String> = (0..half.min(index))
            .map(|dependency| format!("{{\"ref\":\"c{dependency}\"}}"))
            .chain(std::iter::once("{\"lit\":1}".to_string()))
            .collect();
        rule_fields.push_str(&format!(
            "\"f{index}\":{{\"calculate\":{{\"op\":\"coalesce\",\"args\":[{}]}}}}{}",
            args.join(","),
            if index + 1 == fields { "" } else { "," }
        ));
    }
    form_fields.pop();
    (
        format!("{{\"schemaVersion\":\"1.0.0\",\"fields\":[{form_fields}]}}"),
        format!(
            "{{\"schemaVersion\":\"1.0.0\",\"formSchemaVersion\":\"1.0.0\",\"fields\":{{{rule_fields}}}}}"
        ),
    )
}

#[test]
fn a_dense_dependency_graph_is_analyzed_without_a_rescan_per_node() {
    timed("dense dependency graph, 800 fields", || {
        let (form, rules) = dense_rules(800);
        let form = json::parse_object(&form, "form").unwrap();
        let rules = json::parse_object(&rules, "rules").unwrap();
        validate_dependencies(&form, &rules).unwrap();
    });
}

#[test]
fn a_wide_expression_collects_its_references_linearly() {
    timed("16k references in one expression", || {
        let k = 16_000;
        let mut form_fields = String::new();
        for index in 0..k {
            form_fields.push_str(&format!(
                "{{\"id\":\"f{index}\",\"code\":\"c{index}\",\"type\":\"number\"}},"
            ));
        }
        form_fields.pop();
        let args: Vec<String> = (0..k)
            .map(|index| format!("{{\"ref\":\"c{index}\"}}"))
            .collect();
        let form = json::parse_object(
            &format!("{{\"schemaVersion\":\"1.0.0\",\"fields\":[{form_fields}]}}"),
            "form",
        )
        .unwrap();
        let rules = json::parse_object(
            &format!(
                "{{\"schemaVersion\":\"1.0.0\",\"formSchemaVersion\":\"1.0.0\",\"fields\":{{\"f0\":{{\"visibleWhen\":{{\"op\":\"and\",\"args\":[{}]}}}}}}}}",
                args.join(",")
            ),
            "rules",
        )
        .unwrap();
        validate_dependencies(&form, &rules).unwrap();
    });
}

#[test]
fn the_evaluation_order_is_still_topological() {
    // The optimization must not reorder anything: c1 before c3.
    let (form, rules) = dense_rules(400);
    let form = json::parse_object(&form, "form").unwrap();
    let rules = json::parse_object(&rules, "rules").unwrap();
    let metadata = colander::rules::analyze(&form, &rules).unwrap();
    let position = |id: &str| {
        metadata
            .evaluation_order
            .iter()
            .position(|entry| entry == id)
            .unwrap()
    };
    assert!(position("f0") < position("f1"));
    assert!(position("f200") < position("f201"));
    assert_eq!(metadata.calculated_field_ids.len(), 400);
}

// ---------------------------------------------------------------------------
// P-10: the component batch is indexed, and duplicates are rejected.
// ---------------------------------------------------------------------------

#[test]
fn a_batch_of_five_thousand_components_resolves_by_index() {
    timed("5k component references over a 5k batch", || {
        let mut components = Vec::new();
        let mut refs = String::new();
        for index in 0..5_000usize {
            components.push(ComponentVersionData {
                code: format!("C{index}"),
                version: "1.0.0".into(),
                form_schema_json: format!(
                    "{{\"schemaVersion\":\"1.0.0\",\"fields\":[{{\"id\":\"a{index}\",\"code\":\"a{index}\",\"type\":\"text\"}}]}}"
                ),
                ui_schema_json: None,
                content_hash: None,
            });
            refs.push_str(&format!(
                "{{\"id\":\"r{index}\",\"code\":\"r{index}\",\"type\":\"component-ref\",\"componentCode\":\"C{index}\",\"componentVersion\":\"1.0.0\"}},"
            ));
        }
        refs.pop();
        let form = format!("{{\"schemaVersion\":\"1.0.0\",\"fields\":[{refs}]}}");
        let result = compile(&form, None, None, &components).unwrap();
        assert_eq!(result.content_hash.len(), 64);
    });
}

#[test]
fn a_batch_that_lists_the_same_version_twice_is_rejected() {
    let component = |code: &str| {
        ComponentVersionData {
        code: code.into(),
        version: "1.0.0".into(),
        form_schema_json:
            "{\"schemaVersion\":\"1.0.0\",\"fields\":[{\"id\":\"a\",\"code\":\"a\",\"type\":\"text\"}]}"
                .into(),
        ui_schema_json: None,
        content_hash: None,
    }
    };
    let form = r#"{"schemaVersion":"1.0.0","fields":[{"id":"r","code":"r","type":"component-ref","componentCode":"X","componentVersion":"1.0.0"}]}"#;
    let error = compile(form, None, None, &[component("X"), component("X")]).unwrap_err();
    assert!(
        error.message.starts_with("COMPONENT_DUPLICATE_VERSION"),
        "{error}"
    );
}

use indexmap::IndexMap;

use colander::error::Result;
use colander::index::FieldInfo;
use colander::json::{self, JsonMap};
use colander::rules::*;

fn root(text: &str, label: &str) -> JsonMap {
    json::parse_object(text, label).unwrap()
}

fn eval(expression: &str, values: &[(&str, Val)]) -> Result<Val> {
    let node = json::parse(expression).unwrap();
    let mut map = IndexMap::new();
    for (key, value) in values {
        map.insert((*key).to_string(), value.clone());
    }
    evaluate_expression(&node, &map)
}

#[test]
fn evaluates_arithmetic_and_comparisons() {
    assert_eq!(
        eval(r#"{"op":"add","args":[{"lit":1},{"lit":2}]}"#, &[]).unwrap(),
        Val::Double(3.0)
    );
    assert_eq!(
        eval(r#"{"op":"div","args":[{"lit":1},{"lit":0}]}"#, &[]).unwrap(),
        Val::Null
    );
    assert_eq!(
        eval(
            r#"{"op":"eq","args":[{"ref":"x"},{"lit":9}]}"#,
            &[("x", Val::Str("10".into()))]
        )
        .unwrap(),
        Val::Bool(false)
    );
    assert_eq!(
        eval(r#"{"op":"and","args":[]}"#, &[]).unwrap(),
        Val::Bool(true)
    );
    assert_eq!(
        eval(r#"{"op":"or","args":[]}"#, &[]).unwrap(),
        Val::Bool(false)
    );
    assert_eq!(
        eval(
            r#"{"op":"coalesce","args":[{"ref":"missing"},{"lit":"a"}]}"#,
            &[]
        )
        .unwrap(),
        Val::Str("a".into())
    );
    assert_eq!(
        eval(r#"{"op":"empty","args":[{"lit":0}]}"#, &[]).unwrap(),
        Val::Bool(false)
    );
    assert_eq!(
        eval(r#"{"op":"empty","args":[{"lit":""}]}"#, &[]).unwrap(),
        Val::Bool(true)
    );
}

#[test]
fn normalizes_calculated_values() {
    let number = FieldInfo {
        id: "bmi".into(),
        code: "body.bmi".into(),
        field_type: "number".into(),
        required: false,
        read_only: true,
        path: "/fields/2".into(),
        multiple_of: Some(0.01),
        decimal_places: None,
    };
    assert_eq!(
        normalize_calculated_value(Val::Double(70.0 / (1.75 * 1.75)), &number),
        Val::Double(22.86)
    );

    let integer = FieldInfo {
        field_type: "integer".into(),
        ..number.clone()
    };
    assert_eq!(
        normalize_calculated_value(Val::Double(2.5), &integer),
        Val::Double(3.0)
    );
}

#[test]
fn analyzes_calculation_order() {
    let form = root(
        r#"{"schemaVersion":"1.0.0","fields":[
             {"id":"a","code":"calc.a","type":"number","readOnly":true},
             {"id":"b","code":"calc.b","type":"number","readOnly":true}]}"#,
        "form schema",
    );
    let rules = root(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
             "b":{"calculate":{"op":"add","args":[{"ref":"calc.a"},{"lit":1}]}},
             "a":{"calculate":{"lit":1}}}}"#,
        "rules schema",
    );
    let metadata = analyze(&form, &rules).unwrap();
    assert_eq!(metadata.calculated_field_ids, vec!["b", "a"]);
    assert_eq!(metadata.evaluation_order, vec!["a", "b"]);
    validate_dependencies(&form, &rules).unwrap();
}

#[test]
fn detects_cycles() {
    let form = root(
        r#"{"fields":[
             {"id":"a","code":"c.a","type":"number","readOnly":true},
             {"id":"b","code":"c.b","type":"number","readOnly":true}]}"#,
        "form schema",
    );
    let rules = root(
        r#"{"fields":{
             "a":{"calculate":{"op":"add","args":[{"ref":"c.b"},{"lit":1}]}},
             "b":{"calculate":{"op":"add","args":[{"ref":"c.a"},{"lit":1}]}}}}"#,
        "rules schema",
    );
    let error = validate_dependencies(&form, &rules).unwrap_err();
    assert!(error.message.contains("RULE_CYCLIC_DEPENDENCY"));
}

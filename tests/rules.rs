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
    evaluate_expression(&node, &map, &RowSet::empty())
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
fn evaluate_rejects_a_duplicate_field_code() {
    let form = root(
        r#"{"schemaVersion":"1.0.0","fields":[
             {"id":"a","code":"dup","type":"text"},
             {"id":"b","code":"dup","type":"number"}]}"#,
        "form schema",
    );
    let rules = root(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{}}"#,
        "rules schema",
    );
    let error = evaluate(&form, &rules, &IndexMap::new(), None, &mut RowSet::empty()).unwrap_err();
    assert!(
        error.message.starts_with("An item with the same key"),
        "{error}"
    );
}

#[test]
fn evaluate_rejects_a_dependency_error() {
    let form = root(
        r#"{"schemaVersion":"1.0.0","fields":[
             {"id":"tgt","code":"tgt","type":"text"}]}"#,
        "form schema",
    );
    let rules = root(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
             "tgt":{"visibleWhen":{"op":"eq","args":[{"ref":"ghost"},{"lit":1}]}}}}"#,
        "rules schema",
    );
    let error = evaluate(&form, &rules, &IndexMap::new(), None, &mut RowSet::empty()).unwrap_err();
    assert!(
        error.message.starts_with("RULE_UNKNOWN_FIELD_REF"),
        "{error}"
    );
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

fn repeater_form() -> JsonMap {
    root(
        r#"{"schemaVersion":"1.0.0","fields":[
             {"id":"lines","code":"lines","type":"repeater","items":[
               {"id":"qty","code":"qty","type":"integer"},
               {"id":"price","code":"price","type":"number"},
               {"id":"total","code":"total","type":"number","readOnly":true}]},
             {"id":"grand","code":"grand","type":"number","readOnly":true},
             {"id":"nlines","code":"nlines","type":"integer","readOnly":true}]}"#,
        "form schema",
    )
}

fn repeater_rules() -> JsonMap {
    root(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
             "total":{"calculate":{"op":"mul","args":[{"ref":"qty"},{"ref":"price"}]}},
             "grand":{"calculate":{"op":"sum","args":[{"ref":"lines"},{"ref":"total"}]}},
             "nlines":{"calculate":{"op":"count","args":[{"ref":"lines"}]}}}}"#,
        "rules schema",
    )
}

// R-7: a calculated repeater child evaluates once per row, and the array
// replaces the flattened value everywhere downstream.
#[test]
fn calculates_repeater_children_per_row() {
    let form = repeater_form();
    let rules = repeater_rules();
    let answers = root(
        r#"{"lines":[{"qty":2,"price":10},{"qty":3,"price":5}]}"#,
        "answers",
    );
    let mut rows = RowSet::from_answers(&form, &answers);
    // The flattened map a caller would supply alongside: the count and the
    // last surviving row. Per-row calculation must override it, not read it.
    let mut values = IndexMap::new();
    values.insert("lines".to_string(), Val::Int(2));
    values.insert("qty".to_string(), Val::Int(3));
    values.insert("price".to_string(), Val::Double(5.0));

    let result = evaluate(&form, &rules, &values, None, &mut rows).unwrap();
    assert_eq!(
        result.calculated_values.get("total"),
        Some(&Val::List(vec![Val::Double(20.0), Val::Double(15.0)]))
    );
    assert_eq!(
        result.calculated_values.get("grand"),
        Some(&Val::Double(35.0))
    );
    assert_eq!(
        result.calculated_values.get("nlines"),
        Some(&Val::Double(2.0))
    );
}

// R-7a: a `List` under a repeater code carries that repeater's rows, which is
// how the FFI path supplies them.
#[test]
fn interprets_a_list_under_a_repeater_code_as_rows() {
    let form = repeater_form();
    let rules = repeater_rules();
    let mut values = IndexMap::new();
    values.insert(
        "lines".to_string(),
        Val::List(vec![
            Val::Raw(r#"{"qty":2,"price":10}"#.to_string()),
            Val::Raw(r#"{"qty":3,"price":5}"#.to_string()),
        ]),
    );
    let mut rows = RowSet::from_values(&form, &values);

    let result = evaluate(&form, &rules, &values, None, &mut rows).unwrap();
    assert_eq!(
        result.calculated_values.get("total"),
        Some(&Val::List(vec![Val::Double(20.0), Val::Double(15.0)]))
    );
    assert_eq!(
        result.calculated_values.get("grand"),
        Some(&Val::Double(35.0))
    );
}

// R-7: with no rows there is nothing to calculate per row, so the child falls
// back to the scalar path like any calculation with missing inputs, and the
// result is null. `count` is 0 and `sum` is null.
#[test]
fn empty_rows_calculate_nothing() {
    let form = repeater_form();
    let rules = repeater_rules();
    let answers = root(r#"{}"#, "answers");
    let mut rows = RowSet::from_answers(&form, &answers);

    let result = evaluate(&form, &rules, &IndexMap::new(), None, &mut rows).unwrap();
    assert_eq!(result.calculated_values.get("total"), Some(&Val::Null));
    assert_eq!(result.calculated_values.get("grand"), Some(&Val::Null));
    assert_eq!(
        result.calculated_values.get("nlines"),
        Some(&Val::Double(0.0))
    );
}

// R-4: a `validations` entry with no `assert` is rejected, with or without `when`.
#[test]
fn rejects_a_validation_without_assert() {
    let form = root(
        r#"{"schemaVersion":"1.0.0","fields":[
             {"id":"a","code":"a","type":"text"}]}"#,
        "form schema",
    );
    let no_assert = root(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{},
             "validations":[{"code":"V1","message":"M"}]}"#,
        "rules schema",
    );
    let error = validate_dependencies(&form, &no_assert).unwrap_err();
    assert!(error.message.starts_with("RULE_MISSING_ASSERT"), "{error}");

    let when_only = root(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{},
             "validations":[{"code":"V2","when":{"lit":true}}]}"#,
        "rules schema",
    );
    let error = validate_dependencies(&form, &when_only).unwrap_err();
    assert!(error.message.starts_with("RULE_MISSING_ASSERT"), "{error}");

    // An entry with `assert` still passes the check.
    let with_assert = root(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{},
             "validations":[{"code":"V3","assert":{"lit":true}}]}"#,
        "rules schema",
    );
    validate_dependencies(&form, &with_assert).unwrap();
}

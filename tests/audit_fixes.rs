//! Regression tests for the 2026-09-24 adversarial audit findings.
//!
//! Each test names the finding it pins: silent wrong-data behavior must stay
//! fixed, and deliberate semantics must stay explicit.

use colander::compile::{ComponentVersionData, compile};
use colander::json;
use colander::rules::{Val, validate_dependencies};
use colander::schema::validate_text;
use colander::validate::{FormResponseValidationMode, validate};

// ---------------------------------------------------------------------------
// C1: integers and doubles compare numerically.
// ---------------------------------------------------------------------------

#[test]
fn values_equal_matches_int_against_double() {
    assert!(Val::values_equal(&Val::Int(3), &Val::Double(3.0)));
    assert!(Val::values_equal(&Val::Double(3.0), &Val::Int(3)));
    assert!(!Val::values_equal(&Val::Int(3), &Val::Double(3.5)));
    assert!(Val::values_equal(
        &Val::List(vec![Val::Int(1), Val::Double(2.0)]),
        &Val::List(vec![Val::Double(1.0), Val::Int(2)]),
    ));
}

#[test]
fn integer_calculated_matches_an_integer_literal_answer() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"qty","code":"qty","type":"integer"},
        {"id":"total","code":"total","type":"integer","readOnly":true}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "total":{"calculate":{"op":"add","args":[{"ref":"qty"},{"lit":1}]}}}}"#;
    let result = validate(
        form,
        None,
        Some(rules),
        r#"{"qty":2,"total":3}"#,
        FormResponseValidationMode::Complete,
    )
    .unwrap();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert!(
        result.normalized_answers_json.contains(r#""total":3"#),
        "{}",
        result.normalized_answers_json
    );
}

#[test]
fn number_calculated_matches_an_integer_literal_answer() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"total","code":"total","type":"number","readOnly":true}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "total":{"calculate":{"op":"add","args":[{"lit":1},{"lit":2}]}}}}"#;
    let result = validate(
        form,
        None,
        Some(rules),
        r#"{"total":3}"#,
        FormResponseValidationMode::Complete,
    )
    .unwrap();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
}

// ---------------------------------------------------------------------------
// C2: child-code references outside row scope fail closed.
// ---------------------------------------------------------------------------

fn repeater_form() -> &'static str {
    r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"lines","code":"lines","type":"repeater","items":[
            {"id":"price","code":"price","type":"number"}]}]}"#
}

#[test]
fn validation_referencing_a_repeater_child_is_rejected() {
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0",
        "fields":{},"validations":[
        {"code":"X","message":"too big",
         "assert":{"op":"lt","args":[{"ref":"price"},{"lit":100}]}}]}"#;
    let form = json::parse_object(repeater_form(), "form schema").unwrap();
    let rules = json::parse_object(rules, "rules schema").unwrap();
    let error = validate_dependencies(&form, &rules).unwrap_err();
    assert!(
        error.message.starts_with("RULE_INVALID_ROW_REFERENCE"),
        "{error}"
    );
}

#[test]
fn predicate_referencing_a_repeater_child_is_rejected() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"flag","code":"flag","type":"boolean"},
        {"id":"lines","code":"lines","type":"repeater","items":[
            {"id":"price","code":"price","type":"number"}]}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "flag":{"visibleWhen":{"op":"gt","args":[{"ref":"price"},{"lit":0}]}}}}"#;
    let form = json::parse_object(form, "form schema").unwrap();
    let rules = json::parse_object(rules, "rules schema").unwrap();
    let error = validate_dependencies(&form, &rules).unwrap_err();
    assert!(
        error.message.starts_with("RULE_INVALID_ROW_REFERENCE"),
        "{error}"
    );
}

#[test]
fn aggregates_may_still_reference_repeater_children() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"lines","code":"lines","type":"repeater","items":[
            {"id":"price","code":"price","type":"number"}]},
        {"id":"grand","code":"grand","type":"number","readOnly":true}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "grand":{"calculate":{"op":"sum","args":[{"ref":"lines"},{"ref":"price"}]}}}}"#;
    let form = json::parse_object(form, "form schema").unwrap();
    let rules = json::parse_object(rules, "rules schema").unwrap();
    validate_dependencies(&form, &rules).unwrap();
}

// ---------------------------------------------------------------------------
// C3: calculated repeater children round-trip through normalized rows.
// ---------------------------------------------------------------------------

fn calculated_repeater_form() -> &'static str {
    r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"lines","code":"lines","type":"repeater","items":[
            {"id":"qty","code":"qty","type":"integer"},
            {"id":"amount","code":"amount","type":"number","readOnly":true}]}]}"#
}

fn calculated_repeater_rules() -> &'static str {
    r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "amount":{"calculate":{"op":"mul","args":[{"ref":"qty"},{"lit":10}]}}}}"#
}

#[test]
fn calculated_repeater_children_appear_in_normalized_rows() {
    let result = validate(
        calculated_repeater_form(),
        None,
        Some(calculated_repeater_rules()),
        r#"{"lines":[{"qty":2},{"qty":3}]}"#,
        FormResponseValidationMode::Draft,
    )
    .unwrap();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let normalized = json::parse(&result.normalized_answers_json).unwrap();
    let rows = normalized.as_object().unwrap()["lines"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(json::ordered(&rows[0].as_object().unwrap()["amount"]), "20");
    assert_eq!(json::ordered(&rows[1].as_object().unwrap()["amount"]), "30");
}

#[test]
fn calculated_repeater_child_mismatch_is_reported_in_complete_mode() {
    let result = validate(
        calculated_repeater_form(),
        None,
        Some(calculated_repeater_rules()),
        r#"{"lines":[{"qty":2,"amount":999}]}"#,
        FormResponseValidationMode::Complete,
    )
    .unwrap();
    assert!(
        result
            .errors
            .iter()
            .any(|error| error.code == "CALCULATED_VALUE_MISMATCH"),
        "{:?}",
        result.errors
    );
}

// ---------------------------------------------------------------------------
// H1: requiredWhen overwrites the schema default.
// ---------------------------------------------------------------------------

#[test]
fn required_when_false_unsets_a_schema_required_field() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"nick","code":"nick","type":"text","required":true}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "nick":{"requiredWhen":{"lit":false}}}}"#;
    let result = validate(
        form,
        None,
        Some(rules),
        r#"{}"#,
        FormResponseValidationMode::Complete,
    )
    .unwrap();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
}

// ---------------------------------------------------------------------------
// H2: unknown rule keys fail closed.
// ---------------------------------------------------------------------------

#[test]
fn unknown_rule_keys_are_rejected() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"a","code":"a","type":"text"}]}"#;
    let rules = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "a":{"visiblewhen":{"lit":true}}}}"#;
    let form = json::parse_object(form, "form schema").unwrap();
    let rules = json::parse_object(rules, "rules schema").unwrap();
    let error = validate_dependencies(&form, &rules).unwrap_err();
    assert!(
        error.message.starts_with("RULE_UNKNOWN_RULE_KEY"),
        "{error}"
    );
}

// ---------------------------------------------------------------------------
// M2: nothing nests under a repeater.
// ---------------------------------------------------------------------------

#[test]
fn groups_cannot_nest_under_a_repeater() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"lines","code":"lines","type":"repeater","items":[
            {"id":"g","code":"g","type":"group","items":[
                {"id":"x","code":"x","type":"text"}]}]}]}"#;
    let error = validate(
        form,
        None,
        None,
        r#"{"lines":[]}"#,
        FormResponseValidationMode::Draft,
    )
    .unwrap_err();
    assert!(
        error.message.starts_with("REPEATER_NESTED_FIELD"),
        "{error}"
    );
}

/// The D-4 nesting rule is scoped to repeaters. A group nested in a group was
/// rejected with the same code and the same message, which named a repeater the
/// document did not contain: the answer index called the row-children builder
/// for every field carrying `items`, and that builder refused any child with
/// `items`. It also made `index_answer_fields`' own nested-group recursion
/// unreachable, so a group child that was itself a group could not be indexed at
/// all.
#[test]
fn groups_may_nest_under_a_group() {
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"outer","code":"outer","type":"group","items":[
            {"id":"inner","code":"inner","type":"group","items":[
                {"id":"x","code":"x","type":"text"}]}]}]}"#;
    let result = validate(
        form,
        None,
        None,
        r#"{"x":"value"}"#,
        FormResponseValidationMode::Draft,
    )
    .expect("a group inside a group is a legal document");
    assert!(result.is_valid(), "{:?}", result.errors);

    // The nested child's code must be answerable, which is the part the
    // unreachable recursion existed for.
    assert!(
        result.normalized_answers_json.contains("\"x\""),
        "the nested child's code is not answerable: {}",
        result.normalized_answers_json
    );
}

/// A group's children are not row children: a group that carries `items` keeps
/// its children out of the repeater-only `children` list, while a repeater keeps
/// exactly its flat row scalars.
#[test]
fn only_a_repeater_reports_row_children() {
    use colander::index::build_answer_index;

    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"outer","code":"outer","type":"group","items":[
            {"id":"inner","code":"inner","type":"group","items":[
                {"id":"a","code":"a","type":"text"}]}]},
        {"id":"rows","code":"rows","type":"repeater","items":[
            {"id":"b","code":"b","type":"text"}]}]}"#;
    let root = json::parse_object(form, "form").unwrap();
    let by_code = build_answer_index(&root).expect("index");

    // A group carries no answer of its own, so neither group is a key. Their
    // leaves are, and `a` sits two levels down: reaching it is what the
    // nested-group recursion exists for.
    assert!(!by_code.contains_key("outer"), "a group is not an answer");
    assert!(
        !by_code.contains_key("inner"),
        "a nested group is not an answer"
    );
    assert!(by_code.contains_key("a"), "the nested leaf is answerable");

    let rows = by_code.get("rows").expect("the repeater is indexed");
    let codes: Vec<&str> = rows
        .children
        .iter()
        .map(|child| child.code.as_str())
        .collect();
    assert_eq!(codes, vec!["b"], "a repeater reports its row children");
}

// ---------------------------------------------------------------------------
// M4: the schema `type` vocabulary is closed.
// ---------------------------------------------------------------------------

#[test]
fn unknown_schema_type_names_are_schema_errors() {
    let error = validate_text(r#"{"type":"strnig"}"#, r#"1"#, "instance").unwrap_err();
    assert!(error.message.contains("unknown type"), "{error}");
}

// ---------------------------------------------------------------------------
// M5: cycles key on (code, version).
// ---------------------------------------------------------------------------

#[test]
fn same_code_at_another_version_is_not_a_cycle() {
    let inner = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"leaf","code":"leaf","type":"text"}]}"#;
    let outer = |version: &str| {
        format!(
            r#"{{"schemaVersion":"1.0.0","fields":[{{"id":"r","code":"r","type":"component-ref","componentCode":"C","componentVersion":"{version}"}}]}}"#
        )
    };
    let components = vec![
        ComponentVersionData {
            code: "C".into(),
            version: "1.0.0".into(),
            form_schema_json: outer("2.0.0"),
            ui_schema_json: None,
            content_hash: None,
        },
        ComponentVersionData {
            code: "C".into(),
            version: "2.0.0".into(),
            form_schema_json: inner.into(),
            ui_schema_json: None,
            content_hash: None,
        },
    ];
    let form = outer("1.0.0");
    compile(&form, None, None, &components).unwrap();
}

// ---------------------------------------------------------------------------
// M7: component nesting is bounded.
// ---------------------------------------------------------------------------

#[test]
fn deeply_nested_components_are_rejected() {
    let depth = 80;
    let mut components = Vec::new();
    for level in 0..depth {
        let next = if level + 1 == depth {
            r#"{"schemaVersion":"1.0.0","fields":[{"id":"leaf","code":"leaf","type":"text"}]}"#
                .to_string()
        } else {
            format!(
                r#"{{"schemaVersion":"1.0.0","fields":[{{"id":"r{level}","code":"r{level}","type":"component-ref","componentCode":"C{n}","componentVersion":"1.0.0"}}]}}"#,
                n = level + 1
            )
        };
        components.push(ComponentVersionData {
            code: format!("C{level}"),
            version: "1.0.0".into(),
            form_schema_json: next,
            ui_schema_json: None,
            content_hash: None,
        });
    }
    let form = r#"{"schemaVersion":"1.0.0","fields":[
        {"id":"r","code":"r","type":"component-ref","componentCode":"C0","componentVersion":"1.0.0"}]}"#;
    let error = compile(form, None, None, &components).unwrap_err();
    assert!(
        error.message.starts_with("COMPONENT_DEPTH_EXCEEDED"),
        "{error}"
    );
}

// ---------------------------------------------------------------------------
// M15: extra operands are a rule error, not silently ignored.
// ---------------------------------------------------------------------------

#[test]
fn extra_comparison_operands_are_rejected() {
    let form = json::parse_object(
        r#"{"schemaVersion":"1.0.0","fields":[{"id":"a","code":"a","type":"number"}]}"#,
        "form schema",
    )
    .unwrap();
    let rules = json::parse_object(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
            "a":{"visibleWhen":{"op":"eq","args":[{"ref":"a"},{"lit":1},{"lit":2}]}}}}"#,
        "rules schema",
    )
    .unwrap();
    // R-12: fixed-arity operators accept exactly their arity, so a generator
    // bug (an extra operand) fails loudly instead of hiding behind the
    // first two arguments.
    let error = validate_dependencies(&form, &rules).unwrap_err();
    assert!(
        error.message.starts_with("RULE_INVALID_EXPRESSION_ARITY"),
        "{error}"
    );
}

#[test]
fn variadic_operators_still_accept_extra_operands() {
    let form = json::parse_object(
        r#"{"schemaVersion":"1.0.0","fields":[{"id":"a","code":"a","type":"number"}]}"#,
        "form schema",
    )
    .unwrap();
    let rules = json::parse_object(
        r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
            "a":{"visibleWhen":{"op":"and","args":[{"lit":true},{"lit":true},{"lit":true}]}}}}"#,
        "rules schema",
    )
    .unwrap();
    // `and`/`or`/`coalesce` fold over their whole list: extra arguments are
    // part of the operator, not a generator bug.
    validate_dependencies(&form, &rules).unwrap();
}

use colander::error::Result;
use colander::schema::*;

// The validator is domain-agnostic, so the tests bring their own schemas.
// `FORM` is the shape a form schema takes: a `oneOf` over per-type field
// definitions.
const FORM: &str = r##"{
    "type":"object",
    "required":["schemaVersion","fields"],
    "properties":{
        "schemaVersion":{"type":"string"},
        "fields":{"type":"array","items":{"$ref":"#/$defs/field"}}
    },
    "$defs":{"field":{
        "oneOf":[
            {"type":"object","required":["id","code","type"],
             "properties":{"id":{"type":"string"},"code":{"type":"string"},
                           "type":{"const":"text"}},
             "additionalProperties":false},
            {"type":"object","required":["id","code","type"],
             "properties":{"id":{"type":"string"},"code":{"type":"string"},
                           "type":{"const":"number"},
                           "multipleOf":{"type":"number"}},
             "additionalProperties":false}
        ]
    }}
}"##;

const RULES: &str = r##"{
    "type":"object",
    "required":["schemaVersion","formSchemaVersion","fields"],
    "properties":{
        "schemaVersion":{"type":"string"},
        "formSchemaVersion":{"type":"string"},
        "fields":{"type":"object","additionalProperties":{
            "type":"object",
            "properties":{"visibleWhen":{"$ref":"#/$defs/expr"}},
            "additionalProperties":false}}
    },
    "$defs":{"expr":{
        "type":"object",
        "required":["op","args"],
        "properties":{
            "op":{"enum":["eq","neq","gt","gte","lt","lte","and","or","not",
                          "empty","coalesce","add","sub","mul","div"]},
            "args":{"type":"array"}},
        "additionalProperties":false
    }}
}"##;

const WORKFLOW: &str = r##"{
    "type":"object",
    "required":["schemaVersion","steps"],
    "properties":{"schemaVersion":{"type":"string"},
                  "steps":{"type":"array"}}
}"##;

fn schemas() -> PublishedSchemas<'static> {
    PublishedSchemas {
        form_schema: FORM,
        ui_schema: FORM,
        rules_schema: RULES,
        workflow_schema: WORKFLOW,
    }
}

fn check_form(instance: &str) -> Result<()> {
    validate_text(FORM, instance, "form schema")
}

#[test]
fn accepts_a_minimal_form_document() {
    check_form(
        r#"{"schemaVersion":"1.0.0","fields":[
            {"id":"notes","code":"notes","type":"text"}]}"#,
    )
    .unwrap();
}

#[test]
fn rejects_unknown_properties_and_missing_required() {
    let error = check_form(
        r#"{"schemaVersion":"1.0.0","fields":[{"id":"a","code":"b","type":"text","wat":1}]}"#,
    )
    .unwrap_err();
    assert!(
        error.message.starts_with("Invalid form schema: "),
        "{error}"
    );

    // Each per-type definition sets `additionalProperties: false`, so a
    // missing `code` and an extra property both surface as a `oneOf` failure.
    let error =
        check_form(r#"{"schemaVersion":"1.0.0","fields":[{"id":"a","type":"text"}]}"#).unwrap_err();
    assert!(error.message.contains("oneOf"), "{error}");

    let error = check_form(r#"{"schemaVersion":"1.0.0"}"#).unwrap_err();
    assert!(error.message.contains("required"), "{error}");
}

#[test]
fn rejects_wrong_types_and_bad_rules_operators() {
    let valid = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "a":{"visibleWhen":{"op":"eq","args":[{"ref":"x"},{"lit":1}]}}}}"#;
    validate_text(RULES, valid, "rules schema").unwrap();

    let bad_op = r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{
        "a":{"visibleWhen":{"op":"nope","args":[]}}}}"#;
    let error = validate_text(RULES, bad_op, "rules schema").unwrap_err();
    assert!(error.message.contains("Invalid rules schema"), "{error}");
}

#[test]
fn validates_the_workflow_document() {
    let error =
        validate_text(WORKFLOW, r#"{"schemaVersion":"1.0.0"}"#, "workflow schema").unwrap_err();
    assert!(
        error.message.starts_with("Invalid workflow schema: "),
        "{error}"
    );
}

#[test]
fn a_broken_schema_definition_is_reported_as_such() {
    let error = validate_text("not json", "{}", "form schema").unwrap_err();
    assert!(
        error
            .message
            .starts_with("Invalid form schema definition: "),
        "{error}"
    );
}

#[test]
fn compositions_use_the_caller_supplied_schemas() {
    let schemas = schemas();
    validate_form_draft(
        r#"{"schemaVersion":"1.0.0","fields":[]}"#,
        Some(r#"{"schemaVersion":"1.0.0","fields":[]}"#),
        Some(r#"{"schemaVersion":"1.0.0","formSchemaVersion":"1.0.0","fields":{}}"#),
        &schemas,
    )
    .unwrap();

    validate_component_draft(r#"{"schemaVersion":"1.0.0","fields":[]}"#, None, &schemas).unwrap();

    // A document that satisfies none of the supplied schemas is rejected
    // by the schema the caller actually passed in.
    let error = validate_component_draft(r#"{"fields":[]}"#, None, &schemas).unwrap_err();
    assert!(error.message.contains("required"), "{error}");
}

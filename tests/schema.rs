use colander::error::Result;
use colander::ffi::schema::colander_validate_schema;
use colander::ffi::session::call_with_text;
use colander::json::{self, Json};
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

// S-3: an unsupported assertion is rejected rather than passing as if it had.
#[test]
fn rejects_an_unsupported_top_level_keyword() {
    let error = validate_text(
        r#"{"type":"object","patternProperties":{"a":{"type":"string"}}}"#,
        r#"{}"#,
        "form schema",
    )
    .unwrap_err();
    assert!(
        error
            .message
            .contains("patternProperties: unsupported keyword"),
        "{error}"
    );
}

// S-3 is structural: the instance never reaches the property, and the nested
// keyword is still caught.
#[test]
fn rejects_an_unsupported_keyword_nested_in_properties() {
    let error = validate_text(
        r#"{"type":"object","properties":{"name":{"type":"string",
            "dependentRequired":{"a":["b"]}}}}"#,
        r#"{}"#,
        "form schema",
    )
    .unwrap_err();
    assert!(
        error
            .message
            .contains("dependentRequired: unsupported keyword"),
        "{error}"
    );
}

#[test]
fn rejects_an_unsupported_keyword_nested_in_items() {
    let error = validate_text(
        r#"{"type":"array","items":{"contains":{"type":"string"}}}"#,
        r#"[]"#,
        "form schema",
    )
    .unwrap_err();
    assert!(
        error.message.contains("contains: unsupported keyword"),
        "{error}"
    );
}

// A `$defs` entry is annotation-only until a `$ref` reaches it; the check
// follows the reference, so a keyword three levels deep is still caught.
#[test]
fn rejects_an_unsupported_keyword_behind_a_reference() {
    let error = validate_text(
        r##"{"$ref":"#/$defs/thing","$defs":{"thing":{"type":"object",
            "properties":{"x":{"prefixItems":[]}}}}}"##,
        r#"{}"#,
        "form schema",
    )
    .unwrap_err();
    assert!(
        error.message.contains("prefixItems: unsupported keyword"),
        "{error}"
    );
}

// S-3: annotation-only keywords stay ignored, whatever their value.
#[test]
fn accepts_annotation_only_keywords() {
    validate_text(
        r#"{"title":"t","description":"d","$comment":"c","examples":[1],
            "deprecated":true,"$defs":{"unused":{"patternProperties":{}}},
            "type":"object"}"#,
        r#"{}"#,
        "form schema",
    )
    .unwrap();
}

// S-4: a keyword whose value has the wrong JSON type is an error.
#[test]
fn rejects_a_wrong_typed_min_length() {
    let error = validate_text(
        r#"{"type":"string","minLength":"3"}"#,
        r#""ab""#,
        "form schema",
    )
    .unwrap_err();
    assert!(
        error
            .message
            .contains("minLength: keyword value must be an integer"),
        "{error}"
    );
}

// S-3 and S-4 apply to `kind:"instance"`, the domain-free entry point.
#[test]
fn kind_instance_rejects_an_unknown_assertion() {
    let envelope = call_with_text(
        r#"{"kind":"instance","schemaJson":"{\"unevaluatedProperties\":false}",
            "instanceJson":"{}"}"#,
        colander_validate_schema,
    );
    let envelope: Json = json::parse(&envelope).expect("envelope is JSON");
    let object = envelope.as_object().unwrap();
    assert_eq!(json::get_bool(object, "ok"), Some(false));
    let message =
        json::get_str(object.get("error").unwrap().as_object().unwrap(), "message").unwrap();
    assert!(
        message.contains("unevaluatedProperties: unsupported keyword"),
        "{message}"
    );
}

#[test]
fn kind_instance_rejects_a_wrong_typed_keyword() {
    let envelope = call_with_text(
        r#"{"kind":"instance","schemaJson":"{\"minLength\":\"3\"}","instanceJson":"\"ab\""}"#,
        colander_validate_schema,
    );
    let envelope: Json = json::parse(&envelope).expect("envelope is JSON");
    let object = envelope.as_object().unwrap();
    assert_eq!(json::get_bool(object, "ok"), Some(false));
    let message =
        json::get_str(object.get("error").unwrap().as_object().unwrap(), "message").unwrap();
    assert!(
        message.contains("minLength: keyword value must be an integer"),
        "{message}"
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

// S-6: look-around is an ECMA-262 construct the old `regex` engine refused to
// compile. The fancy-regex engine compiles it, so the pattern now asserts.
#[test]
fn pattern_uses_the_ecma_262_engine() {
    let schema = r#"{"type":"string","pattern":"^(?=.*[a-z])(?=.*[0-9])[a-z0-9]+$"}"#;
    validate_text(schema, r#""abc123""#, "form schema").unwrap();

    let error = validate_text(schema, r#""abcdef""#, "form schema").unwrap_err();
    assert!(
        error.message.contains("does not match the pattern"),
        "{error}"
    );
}

// S-6: an uncompilable pattern is an error, never a silent non-match.
#[test]
fn rejects_an_uncompilable_pattern() {
    let error = validate_text(
        r#"{"type":"string","pattern":"("}"#,
        r#""anything""#,
        "form schema",
    )
    .unwrap_err();
    assert!(error.message.contains("cannot be compiled"), "{error}");
}

// S-6 is structural: an uncompilable pattern is rejected even when another
// `anyOf` branch would let the instance through.
#[test]
fn rejects_an_uncompilable_pattern_in_an_unreached_branch() {
    let error = validate_text(
        r#"{"anyOf":[{"type":"string","pattern":"("},{"type":"string"}]}"#,
        r#""anything""#,
        "form schema",
    )
    .unwrap_err();
    assert!(error.message.contains("cannot be compiled"), "{error}");
}

// S-7: a truncated failure says that it was truncated.
#[test]
fn a_truncated_failure_says_it_was_truncated() {
    let error = validate_text(
        r#"{"type":"object","required":["a","b","c","d","e","f"]}"#,
        r#"{}"#,
        "form schema",
    )
    .unwrap_err();
    assert!(
        error.message.contains("truncated: 5 of 6 errors shown"),
        "{error}"
    );

    let short = validate_text(
        r#"{"type":"object","required":["a","b"]}"#,
        r#"{}"#,
        "form schema",
    )
    .unwrap_err();
    assert!(!short.message.contains("truncated"), "{short}");
}

// S-5: `format` is asserted for the closed set, and an unknown name is an error.
#[test]
fn asserts_email_and_rejects_a_non_email() {
    let schema = r#"{"type":"string","format":"email"}"#;
    validate_text(schema, r#""a@b.co""#, "form schema").unwrap();

    for bad in [r#""a@b""#, r#""@b.co""#, r#""a b@c.co""#, r#""nota""#] {
        let error = validate_text(schema, bad, "form schema").unwrap_err();
        assert!(
            error.message.contains("string is not a valid email"),
            "{error}"
        );
    }
}

#[test]
fn asserts_rfc3339_dates_and_times() {
    let date = r#"{"type":"string","format":"date"}"#;
    validate_text(date, r#""2024-02-29""#, "form schema").unwrap();
    for bad in [
        r#""2023-02-29""#,
        r#""2024-13-01""#,
        r#""24-01-01""#,
        r#""2024/01/01""#,
    ] {
        let error = validate_text(date, bad, "form schema").unwrap_err();
        assert!(
            error.message.contains("string is not a valid date"),
            "{error}"
        );
    }

    let datetime = r#"{"type":"string","format":"date-time"}"#;
    validate_text(datetime, r#""2024-01-02T15:04:05Z""#, "form schema").unwrap();
    validate_text(datetime, r#""2024-01-02t15:04:05+02:00""#, "form schema").unwrap();
    for bad in [
        r#""2024-01-02 15:04:05""#,
        r#""2024-01-02T25:00:00Z""#,
        r#""2024-01-02""#,
    ] {
        let error = validate_text(datetime, bad, "form schema").unwrap_err();
        assert!(
            error.message.contains("string is not a valid date-time"),
            "{error}"
        );
    }

    let time = r#"{"type":"string","format":"time"}"#;
    validate_text(time, r#""15:04:05Z""#, "form schema").unwrap();
    validate_text(time, r#""15:04:05.123+02:00""#, "form schema").unwrap();
    for bad in [r#""15:04""#, r#""25:00:00Z""#, r#""15:04:05""#] {
        let error = validate_text(time, bad, "form schema").unwrap_err();
        assert!(
            error.message.contains("string is not a valid time"),
            "{error}"
        );
    }
}

#[test]
fn asserts_uuid_ip_hostname_and_uri() {
    let uuid = r#"{"type":"string","format":"uuid"}"#;
    validate_text(
        uuid,
        r#""123e4567-e89b-12d3-a456-426614174000""#,
        "form schema",
    )
    .unwrap();
    for bad in [
        r#""not-a-uuid""#,
        r#""{123e4567-e89b-12d3-a456-426614174000}""#,
    ] {
        let error = validate_text(uuid, bad, "form schema").unwrap_err();
        assert!(
            error.message.contains("string is not a valid uuid"),
            "{error}"
        );
    }

    let ipv4 = r#"{"type":"string","format":"ipv4"}"#;
    validate_text(ipv4, r#""192.168.0.1""#, "form schema").unwrap();
    for bad in [r#""256.1.1.1""#, r#""01.2.3.4""#, r#""1.2.3""#] {
        let error = validate_text(ipv4, bad, "form schema").unwrap_err();
        assert!(
            error.message.contains("string is not a valid ipv4"),
            "{error}"
        );
    }

    let ipv6 = r#"{"type":"string","format":"ipv6"}"#;
    for good in [r#""2001:db8::1""#, r#""::1""#, r#""::ffff:192.168.0.1""#] {
        validate_text(ipv6, good, "form schema").unwrap();
    }
    for bad in [r#""1::2::3""#, r#""12345::""#, r#""fe80::1%eth0""#] {
        let error = validate_text(ipv6, bad, "form schema").unwrap_err();
        assert!(
            error.message.contains("string is not a valid ipv6"),
            "{error}"
        );
    }

    let hostname = r#"{"type":"string","format":"hostname"}"#;
    validate_text(hostname, r#""example.com""#, "form schema").unwrap();
    validate_text(hostname, r#""localhost""#, "form schema").unwrap();
    for bad in [r#""-leading.example""#, r#""empty..label""#] {
        let error = validate_text(hostname, bad, "form schema").unwrap_err();
        assert!(
            error.message.contains("string is not a valid hostname"),
            "{error}"
        );
    }

    let uri = r#"{"type":"string","format":"uri"}"#;
    for good in [
        r#""https://example.com/x""#,
        r#""mailto:a@b.co""#,
        r#""tel:+123""#,
    ] {
        validate_text(uri, good, "form schema").unwrap();
    }
    for bad in [r#""notauri""#, r#""1http:x""#, r#""http:""#] {
        let error = validate_text(uri, bad, "form schema").unwrap_err();
        assert!(
            error.message.contains("string is not a valid uri"),
            "{error}"
        );
    }
}

#[test]
fn rejects_an_unknown_format_name() {
    let error = validate_text(
        r#"{"type":"string","format":"phone"}"#,
        r#""555-1234""#,
        "form schema",
    )
    .unwrap_err();
    assert!(error.message.contains("unknown format"), "{error}");

    // Structural: an unknown name behind an `anyOf` branch fails even when the
    // instance never reaches it.
    let error = validate_text(
        r#"{"anyOf":[{"type":"string","format":"phone"},{"type":"string"}]}"#,
        r#""anything""#,
        "form schema",
    )
    .unwrap_err();
    assert!(error.message.contains("unknown format"), "{error}");
}

#[test]
fn does_not_assert_format_on_a_non_string() {
    // No `type`, so the instance reaches no keyword that cares about strings.
    validate_text(r#"{"format":"email"}"#, r#"5"#, "form schema").unwrap();
    validate_text(r#"{"format":"email"}"#, r#""a@b.co""#, "form schema").unwrap();
}

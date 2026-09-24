//! The closed `format` vocabulary (SPEC S-5).

use colander::schema::validate_text;

// S-5: `format` is asserted for the closed set. Each row is a schema, an
// instance, whether it validates, and the message needle expected on failure.
#[test]
fn asserts_the_closed_format_set() {
    let cases: &[(&str, &str, bool, &str)] = &[
        (
            r#"{"type":"string","format":"email"}"#,
            r#""a@b.co""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"email"}"#,
            r#""a@b""#,
            false,
            "string is not a valid email",
        ),
        (
            r#"{"type":"string","format":"email"}"#,
            r#""@b.co""#,
            false,
            "string is not a valid email",
        ),
        (
            r#"{"type":"string","format":"email"}"#,
            r#""a b@c.co""#,
            false,
            "string is not a valid email",
        ),
        (
            r#"{"type":"string","format":"email"}"#,
            r#""nota""#,
            false,
            "string is not a valid email",
        ),
        (
            r#"{"type":"string","format":"date"}"#,
            r#""2024-02-29""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"date"}"#,
            r#""2023-02-29""#,
            false,
            "string is not a valid date",
        ),
        (
            r#"{"type":"string","format":"date"}"#,
            r#""2024-13-01""#,
            false,
            "string is not a valid date",
        ),
        (
            r#"{"type":"string","format":"date"}"#,
            r#""24-01-01""#,
            false,
            "string is not a valid date",
        ),
        (
            r#"{"type":"string","format":"date"}"#,
            r#""2024/01/01""#,
            false,
            "string is not a valid date",
        ),
        (
            r#"{"type":"string","format":"date-time"}"#,
            r#""2024-01-02T15:04:05Z""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"date-time"}"#,
            r#""2024-01-02t15:04:05+02:00""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"date-time"}"#,
            r#""2024-01-02 15:04:05""#,
            false,
            "string is not a valid date-time",
        ),
        (
            r#"{"type":"string","format":"date-time"}"#,
            r#""2024-01-02T25:00:00Z""#,
            false,
            "string is not a valid date-time",
        ),
        (
            r#"{"type":"string","format":"date-time"}"#,
            r#""2024-01-02""#,
            false,
            "string is not a valid date-time",
        ),
        (
            r#"{"type":"string","format":"time"}"#,
            r#""15:04:05Z""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"time"}"#,
            r#""15:04:05.123+02:00""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"time"}"#,
            r#""15:04""#,
            false,
            "string is not a valid time",
        ),
        (
            r#"{"type":"string","format":"time"}"#,
            r#""25:00:00Z""#,
            false,
            "string is not a valid time",
        ),
        (
            r#"{"type":"string","format":"time"}"#,
            r#""15:04:05""#,
            false,
            "string is not a valid time",
        ),
        (
            r#"{"type":"string","format":"uuid"}"#,
            r#""123e4567-e89b-12d3-a456-426614174000""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"uuid"}"#,
            r#""not-a-uuid""#,
            false,
            "string is not a valid uuid",
        ),
        (
            r#"{"type":"string","format":"uuid"}"#,
            r#""{123e4567-e89b-12d3-a456-426614174000}""#,
            false,
            "string is not a valid uuid",
        ),
        (
            r#"{"type":"string","format":"ipv4"}"#,
            r#""192.168.0.1""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"ipv4"}"#,
            r#""256.1.1.1""#,
            false,
            "string is not a valid ipv4",
        ),
        (
            r#"{"type":"string","format":"ipv4"}"#,
            r#""01.2.3.4""#,
            false,
            "string is not a valid ipv4",
        ),
        (
            r#"{"type":"string","format":"ipv4"}"#,
            r#""1.2.3""#,
            false,
            "string is not a valid ipv4",
        ),
        (
            r#"{"type":"string","format":"ipv6"}"#,
            r#""2001:db8::1""#,
            true,
            "",
        ),
        (r#"{"type":"string","format":"ipv6"}"#, r#""::1""#, true, ""),
        (
            r#"{"type":"string","format":"ipv6"}"#,
            r#""::ffff:192.168.0.1""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"ipv6"}"#,
            r#""1::2::3""#,
            false,
            "string is not a valid ipv6",
        ),
        (
            r#"{"type":"string","format":"ipv6"}"#,
            r#""12345::""#,
            false,
            "string is not a valid ipv6",
        ),
        (
            r#"{"type":"string","format":"ipv6"}"#,
            r#""fe80::1%eth0""#,
            false,
            "string is not a valid ipv6",
        ),
        (
            r#"{"type":"string","format":"hostname"}"#,
            r#""example.com""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"hostname"}"#,
            r#""localhost""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"hostname"}"#,
            r#""-leading.example""#,
            false,
            "string is not a valid hostname",
        ),
        (
            r#"{"type":"string","format":"hostname"}"#,
            r#""empty..label""#,
            false,
            "string is not a valid hostname",
        ),
        (
            r#"{"type":"string","format":"uri"}"#,
            r#""https://example.com/x""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"uri"}"#,
            r#""mailto:a@b.co""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"uri"}"#,
            r#""tel:+123""#,
            true,
            "",
        ),
        (
            r#"{"type":"string","format":"uri"}"#,
            r#""notauri""#,
            false,
            "string is not a valid uri",
        ),
        (
            r#"{"type":"string","format":"uri"}"#,
            r#""1http:x""#,
            false,
            "string is not a valid uri",
        ),
        (
            r#"{"type":"string","format":"uri"}"#,
            r#""http:""#,
            false,
            "string is not a valid uri",
        ),
        (
            r#"{"type":"string","format":"phone"}"#,
            r#""555-1234""#,
            false,
            "unknown format",
        ),
    ];
    for (schema, instance, valid, needle) in cases {
        match validate_text(schema, instance, "form schema") {
            Ok(()) => assert!(*valid, "{schema} on {instance}"),
            Err(error) => {
                assert!(!valid, "{error}");
                assert!(error.message.contains(needle), "{error}");
            }
        }
    }
}

// S-5 is structural: an unknown name behind an `anyOf` branch fails even when
// the instance never reaches it.
#[test]
fn rejects_an_unknown_format_name_in_an_unreached_branch() {
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

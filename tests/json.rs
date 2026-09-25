use colander::json::*;

fn canon(text: &str) -> String {
    canonical(&parse(text).unwrap())
}

#[test]
fn sorts_object_keys_ordinally() {
    assert_eq!(
        canon(r#"{"b":1,"A":2,"a":3,"_":4}"#),
        r#"{"A":2,"_":4,"a":3,"b":1}"#
    );
}

#[test]
fn preserves_raw_number_literals() {
    assert_eq!(
        canon(r#"{"z":123456789012345678901234567890,"y":1.0e-7,"x":1e3,"w":-0.0}"#),
        r#"{"w":-0.0,"x":1e3,"y":1.0e-7,"z":123456789012345678901234567890}"#
    );
    assert_eq!(canon("[1.50,2]"), "[1.50,2]");
    assert_eq!(canon("[1E+3,1e+3,0.0]"), "[1E+3,1e+3,0.0]");
}

#[test]
fn escapes_non_ascii_and_control_characters() {
    assert_eq!(
        canon(r#"{"x":"é<>&'+`\u0001\u007f😀"}"#),
        r#"{"x":"\u00E9\u003C\u003E\u0026\u0027\u002B\u0060\u0001\u007F\uD83D\uDE00"}"#
    );
    assert_eq!(
        canon(r#"{"x":"a\tb\nc\"d\\e/f"}"#),
        r#"{"x":"a\tb\nc\u0022d\\e/f"}"#
    );
}

#[test]
fn parses_surrogate_pairs_and_duplicate_keys() {
    assert_eq!(canon(r#"{"x":"\uD83D\uDE00"}"#), r#"{"x":"\uD83D\uDE00"}"#);
    assert_eq!(
        ordered(&parse(r#"{"a":1,"b":2,"a":3}"#).unwrap()),
        r#"{"a":3,"b":2}"#
    );
}

#[test]
fn formats_doubles_with_shortest_round_trip() {
    assert_eq!(format_double(3.0), "3");
    assert_eq!(format_double(-0.0), "-0");
    assert_eq!(format_double(1.5), "1.5");
    assert_eq!(format_double(1e16), "10000000000000000");
    assert_eq!(format_double(1e17), "1E+17");
    assert_eq!(format_double(1e-4), "0.0001");
    assert_eq!(format_double(1e-5), "1E-05");
    assert_eq!(format_double(5e-324), "5E-324");
    assert_eq!(format_double(1.0 / 3.0), "0.3333333333333333");
    assert_eq!(format_double(0.1 + 0.2), "0.30000000000000004");
    assert_eq!(format_double(-1.25e-3), "-0.00125");
    assert_eq!(format_double(f64::MAX), "1.7976931348623157E+308");
    // The extra digits are the point: this literal names the double that
    // sits just above 22.86, and the shortest round-trip must collapse it.
    #[allow(clippy::excessive_precision)]
    let almost_22_86 = 22.859999999999999_f64;
    assert_eq!(format_double(almost_22_86), "22.86");
}

#[test]
fn rejects_malformed_input() {
    assert!(parse("{").is_err());
    assert!(parse("01").is_err());
    assert!(parse("1.").is_err());
    assert!(parse("[1,]").is_err());
    assert!(parse(r#""\uD83D""#).is_err());
}

#[test]
fn streams_the_same_bytes_as_ordered() {
    let cases = [
        r#"{"a":{"b":[1,2,{"c":null}]},"d":[[],{}]}"#,
        "[1.50,1e3,-0.0]",
        r#""\u0041""#,
        r#""\/""#,
        r#""\uD83D\uDE00""#,
        r#""a\tb\nc\"d\\e/f""#,
        "\"é\"",
        "  {  \"a\" : [ 1 , 2 ] , \"b\" : { }  }  ",
    ];
    for text in cases {
        assert_eq!(
            ordered_stream(text).unwrap(),
            ordered(&parse(text).unwrap()),
            "input {text}"
        );
    }
}

#[test]
fn streams_duplicate_keys_with_the_dom_rule() {
    assert_eq!(
        ordered_stream(r#"{"a":1,"b":2,"a":3}"#).unwrap(),
        r#"{"a":3,"b":2}"#
    );
}

#[test]
fn scopes_duplicate_keys_to_each_object() {
    // The child repeat forces the DOM fallback; the parent's own keys survive it.
    assert_eq!(
        ordered_stream(r#"{"a":{"x":1,"x":2},"x":3}"#).unwrap(),
        r#"{"a":{"x":2},"x":3}"#
    );
    // A child key equal to a parent key is not a duplicate: this one streams.
    assert_eq!(
        ordered_stream(r#"{"a":{"x":1},"x":3}"#).unwrap(),
        r#"{"a":{"x":1},"x":3}"#
    );
}

#[test]
fn streaming_reports_the_same_errors() {
    for text in ["{", "01", "1.", "[1,]", r#""\uD83D""#, r#"{"a":1,}"#, "tru"] {
        let streamed = ordered_stream(text).unwrap_err().message;
        let parsed = parse(text).unwrap_err().message;
        assert_eq!(streamed, parsed, "input {text}");
    }
}

#[test]
fn shared_string_reader_preserves_escapes() {
    assert_eq!(
        parse(r#""\u0041\/é""#).unwrap(),
        Json::String("A/é".to_string())
    );
}

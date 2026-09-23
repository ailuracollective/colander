//! Codec-seam tests: JSON parity, MessagePack behavior, and the generic `run`.

use colander::codec::Codec;
use colander::codec::json::JsonCodec;
use colander::codec::messagepack::MessagePackCodec;
use colander::ffi::envelope::run;
use colander::json::{self, Json, JsonMap};

const NESTED: &str = r#"{"b":[1,2,{"c":null}],"a":{"z":1.50,"y":1e3,"x":true}}"#;

/// The same shape without lossy number spellings, for the round-trip assertion.
const NESTED_STABLE: &str = r#"{"b":[1,2,{"c":null}],"a":{"z":1.5,"y":1000,"x":true}}"#;

fn fixture() -> Json {
    json::parse(NESTED).expect("fixture parses")
}

// ---------------------------------------------------------------------------
// T1: the JSON codec changes nothing
// ---------------------------------------------------------------------------

#[test]
fn json_codec_matches_the_writers_byte_for_byte() {
    let node = fixture();
    let codec = JsonCodec;

    assert_eq!(codec.name(), "json");
    assert_eq!(
        codec.encode_ordered(&node).as_bytes(),
        json::ordered(&node).as_bytes()
    );
    assert_eq!(
        codec.encode_canonical(&node).as_bytes(),
        json::canonical(&node).as_bytes()
    );
}

#[test]
fn json_decode_preserves_the_request_error_text() {
    let codec = JsonCodec;

    // The syntax label is exactly what `parse_object` produced before the seam.
    assert_eq!(
        codec.decode(b"{", "request").unwrap_err().message,
        json::parse_object("{", "request").unwrap_err().message
    );
    // A wrong-typed document is rejected by the shared object check in `run`.
    assert_eq!(
        run(&JsonCodec, b"[]", |_| Ok(Json::Null))
            .unwrap_err()
            .message,
        "Invalid request: expected a JSON object."
    );
    // The UTF-8 failure stays bare, as `read_request` produced it.
    assert_eq!(
        codec.decode(&[0xFF], "request").unwrap_err().message,
        "request is not valid UTF-8"
    );
}

// ---------------------------------------------------------------------------
// T3/T4: MessagePack round-trips and known losses
// ---------------------------------------------------------------------------

#[test]
fn messagepack_round_trips_nested_documents_in_key_order() {
    let node = json::parse(NESTED_STABLE).expect("fixture parses");
    let codec = MessagePackCodec;

    assert_eq!(codec.name(), "messagepack");
    let decoded = codec
        .decode(&codec.encode_ordered(&node), "request")
        .expect("round trip");
    assert_eq!(json::ordered(&decoded), json::ordered(&node));

    let map = decoded.as_object().expect("object");
    assert_eq!(map.keys().collect::<Vec<_>>(), vec!["b", "a"]);

    let nested = map["a"].as_object().expect("nested object");
    assert_eq!(nested.keys().collect::<Vec<_>>(), vec!["z", "y", "x"]);
}

#[test]
fn messagepack_is_lossy_for_number_spelling() {
    let node = json::parse(r#"[1.50,1e3]"#).expect("fixture");
    let codec = MessagePackCodec;

    let decoded = codec
        .decode(&codec.encode_ordered(&node), "request")
        .expect("round trip");
    // Documented loss, not a bug: the literal spelling is gone, so `1.50`
    // returns as the shortest canonical double and `1e3` becomes an integer.
    assert_eq!(json::ordered(&decoded), "[1.5,1000]");
}

#[test]
fn messagepack_keeps_large_unsigned_integers_as_literals() {
    let codec = MessagePackCodec;

    let max_u64 = "18446744073709551615";
    let bytes = codec.encode_ordered(&json::parse(max_u64).expect("fixture"));
    assert_eq!(bytes[0], 0xCF, "above i64::MAX is uint64");
    assert_eq!(
        codec.decode(&bytes, "request").expect("round trip"),
        Json::number_literal(max_u64)
    );

    // Beyond u64 the literal falls back to float64, the encoder's last resort.
    let beyond = "18446744073709551616";
    let bytes = codec.encode_ordered(&json::parse(beyond).expect("fixture"));
    assert_eq!(bytes[0], 0xCB, "beyond u64 is float64");
}

#[test]
fn messagepack_decodes_every_integer_and_float_width() {
    let codec = MessagePackCodec;

    assert_eq!(
        codec.decode(&[0xCC, 0xFF], "request").expect("uint8"),
        Json::integer(255)
    );
    assert_eq!(
        codec
            .decode(&[0xCD, 0x01, 0x00], "request")
            .expect("uint16"),
        Json::integer(256)
    );
    assert_eq!(
        codec
            .decode(&[0xCE, 0x00, 0x01, 0x00, 0x00], "request")
            .expect("uint32"),
        Json::integer(65_536)
    );
    assert_eq!(
        codec.decode(&[0xD0, 0xFF], "request").expect("int8"),
        Json::integer(-1)
    );
    assert_eq!(
        codec.decode(&[0xD1, 0xFF, 0x00], "request").expect("int16"),
        Json::integer(-256)
    );
    assert_eq!(
        codec
            .decode(&[0xD2, 0xFF, 0xFF, 0xFF, 0x00], "request")
            .expect("int32"),
        Json::integer(-256)
    );
    assert_eq!(
        codec
            .decode(
                &[0xD3, 0x7F, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
                "request"
            )
            .expect("int64"),
        Json::integer(i64::MAX)
    );
    // float32 must be widened and re-spelled by `format_double`.
    assert_eq!(
        codec
            .decode(&[0xCA, 0x3F, 0xC0, 0x00, 0x00], "request")
            .expect("float32"),
        Json::double(1.5)
    );
}

#[test]
fn messagepack_canonical_sorts_keys_by_utf8_order() {
    let node = json::parse(r#"{"b":1,"a":2}"#).expect("fixture");
    let bytes = MessagePackCodec.encode_canonical(&node);

    assert_eq!(bytes[0], 0x82, "fixmap of two entries");
    assert_eq!(&bytes[1..3], &[0xA1, b'a'], "sorted key 'a' comes first");
    assert_eq!(&bytes[4..6], &[0xA1, b'b'], "then 'b'");
}

// ---------------------------------------------------------------------------
// T4: the seam runs end to end through the codec-generic entry point
// ---------------------------------------------------------------------------

#[test]
fn the_seam_runs_a_messagepack_request_end_to_end() {
    let codec = MessagePackCodec;
    let request = json::parse(r#"{"value":42,"name":"x"}"#).expect("fixture");
    let bytes = codec.encode_ordered(&request);

    let result = run(&codec, &bytes, |request| {
        let value = json::get_i64(request, "value").expect("value is an integer");
        let mut out = JsonMap::new();
        // Echo a helper that proves the decoded map reached the body.
        out.insert("doubled".to_string(), Json::integer(value * 2));
        out.insert(
            "name".to_string(),
            Json::String(
                json::get_str(request, "name")
                    .unwrap_or_default()
                    .to_string(),
            ),
        );
        Ok(Json::Object(out))
    })
    .expect("body runs");

    assert_eq!(json::ordered(&result), r#"{"doubled":84,"name":"x"}"#);

    // A non-object encoding is rejected by the shared object check.
    let array = codec.encode_ordered(&json::parse("[1,2]").expect("fixture"));
    assert_eq!(
        run(&codec, &array, |_| Ok(Json::Null)).unwrap_err().message,
        "Invalid request: expected a JSON object."
    );
}

// ---------------------------------------------------------------------------
// T4: hostile and truncated input
// ---------------------------------------------------------------------------

#[test]
fn messagepack_rejects_nesting_past_the_depth_limit() {
    let codec = MessagePackCodec;

    // `{"a":{"a":…{"a":null}…}}`, one nesting level per fixmap.
    let mut deep = Vec::new();
    for _ in 0..70 {
        deep.extend_from_slice(&[0x81, 0xA1, b'a']);
    }
    deep.push(0xC0);
    let error = codec.decode(&deep, "request").unwrap_err();
    assert!(
        error.message.contains("nesting depth"),
        "unexpected error: {}",
        error.message
    );

    // At the limit, the same shape decodes.
    let mut shallow = Vec::new();
    for _ in 0..8 {
        shallow.extend_from_slice(&[0x81, 0xA1, b'a']);
    }
    shallow.push(0xC0);
    assert!(codec.decode(&shallow, "request").is_ok());
}

#[test]
fn messagepack_rejects_truncated_input_without_panicking() {
    let codec = MessagePackCodec;

    // A map header promising one entry, with the value missing.
    assert!(codec.decode(&[0x81, 0xA1, b'a'], "request").is_err());
    // A str8 header promising five bytes, with only one present.
    assert!(codec.decode(&[0xD9, 0x05, b'a'], "request").is_err());
    // A bare int32 tag with no payload.
    assert!(codec.decode(&[0xD2], "request").is_err());
    // An empty input has no value at all.
    assert!(codec.decode(&[], "request").is_err());
}

#[test]
fn messagepack_rejects_bin_and_ext_families() {
    let codec = MessagePackCodec;

    for tag in [0xC4_u8, 0xC5, 0xC6] {
        let error = codec.decode(&[tag, 0x00], "request").unwrap_err();
        assert!(error.message.contains("bin"), "tag 0x{tag:02X}");
    }
    for tag in [0xC7_u8, 0xC8, 0xC9, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8] {
        let error = codec.decode(&[tag, 0x00], "request").unwrap_err();
        assert!(error.message.contains("extension"), "tag 0x{tag:02X}");
    }
    assert!(codec.decode(&[0xC1], "request").is_err());
    // Trailing bytes after a complete value are an error too.
    assert!(codec.decode(&[0xC0, 0xC0], "request").is_err());
}

#[test]
fn messagepack_encodes_small_values_with_expected_prefixes() {
    let codec = MessagePackCodec;

    let object = json::parse(r#"{"b":2,"a":1,"c":3}"#).expect("fixture");
    assert_eq!(codec.encode_ordered(&object)[0], 0x83, "fixmap of three");

    let string = json::parse(r#""abc""#).expect("fixture");
    assert_eq!(codec.encode_ordered(&string), vec![0xA3, b'a', b'b', b'c']);

    assert_eq!(
        codec.encode_ordered(&json::parse("1").expect("fixture")),
        vec![0x01]
    );
    assert_eq!(
        codec.encode_ordered(&json::parse("-1").expect("fixture")),
        vec![0xFF]
    );
    assert_eq!(
        codec.encode_ordered(&json::parse("127").expect("fixture")),
        vec![0x7F]
    );
    assert_eq!(
        codec.encode_ordered(&json::parse("-32").expect("fixture")),
        vec![0xE0]
    );
    assert_eq!(
        codec.encode_ordered(&json::parse("128").expect("fixture")),
        vec![0xD1, 0x00, 0x80],
        "128 leaves the fixint range for int16"
    );
}

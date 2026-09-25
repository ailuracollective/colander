use colander::hash::*;

#[test]
fn hashes_the_document_order_payload() {
    // {"b":1,"a":1.50} keeps its document order inside the payload, so the
    // digest differs from the canonical (sorted) rendering.
    let payload = canonical_payload(r#"{"b":1,"a":1.50}"#, Some(r#"{"z":"é"}"#), None).unwrap();
    assert_eq!(
        payload,
        r#"{"form":{"b":1,"a":1.50},"ui":{"z":"\u00E9"},"rules":null}"#
    );
}

#[test]
fn nulls_for_absent_documents() {
    let payload = canonical_payload("{}", None, None).unwrap();
    assert_eq!(payload, r#"{"form":{},"ui":null,"rules":null}"#);
}

#[test]
fn sha256_is_lowercase_hex() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

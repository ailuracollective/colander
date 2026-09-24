//! The panic boundary: no panic reaches the caller, including a panic in the
//! final serialization step (SPEC C-7).

use colander::codec::Codec;
use colander::error::{ColanderError, Result};
use colander::ffi::envelope::{colander_free_string, into_envelope};
use colander::json::Json;

/// A codec whose encoding always panics, so the test drives the boundary
/// rather than hoping the real writer fails.
struct PanicCodec;

impl Codec for PanicCodec {
    type Encoded = String;

    fn name(&self) -> &'static str {
        "panic"
    }

    fn decode(&self, _bytes: &[u8], what: &str) -> Result<Json> {
        Err(ColanderError::new(format!("cannot decode {what}")))
    }

    fn encode_canonical(&self, _node: &Json) -> String {
        panic!("encode_canonical")
    }

    fn encode_ordered(&self, _node: &Json) -> String {
        panic!("encode_ordered")
    }
}

#[test]
fn an_encode_panic_becomes_a_panic_envelope() {
    let pointer = into_envelope(&PanicCodec, Ok(Json::Bool(true)));
    // SAFETY: `into_envelope` hands out a NUL-terminated string the caller owns.
    let text = unsafe { std::ffi::CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned();
    unsafe { colander_free_string(pointer) };
    assert!(text.contains(r#""kind":"panic""#), "{text}");
    assert!(text.contains("response encoding panicked"), "{text}");
}

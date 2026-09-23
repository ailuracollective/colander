//! The codec seam: one trait between [`Json`] and its wire bytes.
//!
//! The core never hardcodes a serialization format. [`Codec`] maps bytes to
//! [`Json`] and back, and every document boundary is generic over it. Two
//! implementations ship here:
//!
//! - [`json::JsonCodec`] delegates to the existing handwritten parser and
//!   writers, so the published JSON behavior stays exactly what it was.
//! - [`messagepack::MessagePackCodec`] is a handwritten MessagePack codec with
//!   no dependency, exercised from Rust because binary cannot cross the
//!   NUL-terminated `char *` ABI.
//!
//! Injection is static: `dispatch` is generic over `C: Codec`, so the compiler
//! monomorphizes and inlines the chosen implementation. There is no vtable and
//! no per-node indirection.

pub mod json;
pub mod messagepack;

use crate::error::Result;
use crate::json::Json;

/// Maps between [`Json`] and its wire representation.
pub trait Codec {
    /// Wire representation: `String` for JSON, `Vec<u8>` for MessagePack.
    ///
    /// The associated type keeps the JSON path allocation-free: the JSON
    /// implementation returns the writer's `String` directly, so
    /// [`CString::new`](std::ffi::CString::new) takes ownership without a copy.
    /// A `Vec<u8>`-only surface would add one copy per response.
    type Encoded: AsRef<[u8]> + Into<Vec<u8>>;

    /// A short name for diagnostics and tests.
    fn name(&self) -> &'static str;

    /// Decode `bytes` into a [`Json`] document, labelling failures with `what`.
    fn decode(&self, bytes: &[u8], what: &str) -> Result<Json>;

    /// Encode `node` with object keys sorted by UTF-8 byte order.
    fn encode_canonical(&self, node: &Json) -> Self::Encoded;

    /// Encode `node` with object keys in document order.
    fn encode_ordered(&self, node: &Json) -> Self::Encoded;
}

//! C ABI: one JSON request string in, one JSON envelope out.
//!
//! Every entry point returns a NUL-terminated UTF-8 string owned by the
//! caller, who must hand it back to [`colander_free_string`]. The envelope is
//! either
//!
//! ```json
//! {"ok":true,"result":{...}}
//! ```
//!
//! or
//!
//! ```json
//! {"ok":false,"error":{"kind":"invalid_request","message":"..."}}
//! ```
//!
//! Panics are caught at the boundary — a Rust panic must never unwind into a
//! C caller.
//!
//! One submodule per core domain, plus the envelope that wraps them all.

pub mod compile;
pub mod describe;
pub mod envelope;
pub mod memory;
pub mod response;
pub mod rules;
pub mod schema;
pub mod session;
pub mod version;

pub use envelope::{ABI_VERSION, ErrorKind};
pub use session::call_with_text;

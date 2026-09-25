//! colander — a portable form runtime, in Rust behind a C ABI.
//!
//! Modules by domain, each split into flat submodules; nothing that a second
//! use has not justified. The crate's vocabulary is forms — fields, repeaters,
//! rules, answers — and the documents it operates on come from the caller.
//!
//! Behaviour is fixed by golden vectors replayed against this crate. Where
//! colander deliberately deviates, the deviation is documented at the site that
//! implements it, so a behaviour change stays an explicit decision.

pub mod codec;
pub mod compile;
pub mod describe;
pub mod error;
pub mod ffi;
pub mod hash;
pub mod index;
pub mod json;
pub mod keys;
pub mod pattern;
pub mod rules;
pub mod schema;
mod semantic;
pub mod semver;
pub mod validate;

pub use error::{ColanderError, Result};

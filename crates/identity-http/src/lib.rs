#![forbid(unsafe_code)]

//! HTTP Identity verification adapter.

mod adapter;

pub use adapter::{IdentityHttpVerifier, IdentityVerifierConfigError};

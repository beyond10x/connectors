//! Versioned credential-free operation protocol.
//!
//! The current API is v0alpha2. Transport boundaries explicitly decode both supported versions
//! and project responses to the requested identity; frozen v0alpha1 artifacts stay unchanged.
pub mod legacy;
pub mod schema;
pub mod wire;
pub use wire::*;

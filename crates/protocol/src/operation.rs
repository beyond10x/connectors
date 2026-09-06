//! Versioned credential-free operation protocol.
//!
//! The default API remains the deployed v0alpha1 projection during provider-first migration.
//! New transports opt into `wire` explicitly; frozen v0alpha1 artifacts are never regenerated.
pub mod legacy;
pub mod schema;
pub mod wire;
pub use legacy::*;

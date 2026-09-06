//! Versioned credential-free operation protocol.
//!
//! The internal API remains v0alpha2. Additive v3 contracts and three-version adapters are
//! explicit modules; all predecessor readers and artifacts stay unchanged.
pub mod legacy;
pub mod schema;
pub mod schema_v3;
pub mod v3;
pub mod versions;
pub mod wire;
pub use wire::*;

#![forbid(unsafe_code)]

//! Inbound hosted and personal-local transport adapters for Connectors application ports.

mod catalog_projection;
pub mod egress;
pub mod hosted;
#[cfg(unix)]
pub mod local;

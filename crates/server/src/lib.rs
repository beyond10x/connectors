#![forbid(unsafe_code)]

//! Inbound hosted and personal-local transport adapters for Connectors application ports.

pub mod hosted;
#[cfg(unix)]
pub mod local;

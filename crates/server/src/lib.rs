#![forbid(unsafe_code)]

//! Inbound hosted and personal-local transport adapters for Connectors application ports.

mod catalog_projection;
pub mod egress;
pub mod hosted;
mod legacy_discovery;
#[cfg(unix)]
pub mod local;
#[cfg(unix)]
mod local_lifecycle;
#[cfg(unix)]
pub mod local_setup;

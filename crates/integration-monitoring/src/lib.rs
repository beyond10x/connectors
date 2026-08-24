#![forbid(unsafe_code)]

//! Monitoring Integration adapter.

mod backend;
mod errors;
mod projection;

pub use backend::{MonitoringBackend, MonitoringError};

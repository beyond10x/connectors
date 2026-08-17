#![forbid(unsafe_code)]

//! Monitoring Integration adapter.

mod backend;
mod errors;

pub use backend::{MonitoringBackend, MonitoringError};

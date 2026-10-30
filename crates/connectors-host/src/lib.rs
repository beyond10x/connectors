pub mod credentials;
pub mod federation;
pub mod http;
pub mod local;
pub mod server;

use connectors_core::{Error, Result};
use serde::de::DeserializeOwned;
use std::path::Path;

pub fn read_config<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let metadata =
        std::fs::metadata(path).map_err(|_| Error::invalid("configuration is not readable"))?;
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return Err(Error::invalid("configuration must be a bounded file"));
    }
    let data = std::fs::read(path).map_err(|_| Error::invalid("configuration is not readable"))?;
    serde_yaml_ng::from_slice(&data)
        .map_err(|_| Error::invalid("configuration does not match the adapter schema"))
}

/// The one log subscriber of every binary in this repository. A log line that
/// cannot be written is dropped: with internal-error logging on, the fmt layer
/// reports a failed write with `eprintln!`, which panics when standard error is
/// a closed pipe and so ends the request that logged, after its dispatch.
pub fn logging() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .log_internal_errors(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .try_init();
}

pub mod schema;

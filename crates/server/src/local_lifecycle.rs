//! Local process control after the Unix transport has authenticated its owner.

use std::path::PathBuf;

use protocol::lifecycle::{self, LifecycleRequest};
use tokio::sync::Notify;

pub(crate) struct Lifecycle {
    pub(crate) shutdown: Notify,
    configuration: Option<String>,
}

impl Lifecycle {
    pub(crate) fn new(configuration: Option<PathBuf>) -> Self {
        Self {
            shutdown: Notify::new(),
            configuration: configuration.map(|path| path.to_string_lossy().into_owned()),
        }
    }

    pub(crate) fn response(&self, frame: &[u8]) -> Option<(Vec<u8>, bool)> {
        if frame.len() > lifecycle::MAX_FRAME_BYTES {
            return None;
        }
        let request: lifecycle::RequestEnvelope = serde_json::from_slice(frame).ok()?;
        if !request.valid() {
            return None;
        }
        let stopping = matches!(request.request, LifecycleRequest::Stop {});
        let response = lifecycle::ResponseEnvelope {
            protocol: lifecycle::CONTRACT.to_owned(),
            request_id: request.request_id,
            response: lifecycle::DaemonStatus {
                process_id: std::process::id(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                configuration: self.configuration.clone(),
                stopping,
            },
        };
        Some((serde_json::to_vec(&response).ok()?, stopping))
    }
}

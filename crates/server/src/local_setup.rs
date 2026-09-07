//! Runtime-owned enrollment port for the authenticated personal-local transport.

use async_trait::async_trait;
pub use protocol::local_setup::SetupError;
use protocol::local_setup::{self, EnrollRequest, LocalSetupRequest};

/// Bound by the runtime to its exact configuration and credential store.
#[async_trait]
pub trait LocalSetupHandler: Send + Sync {
    /// Store or acquire the credential and persist only its non-secret configuration.
    /// The returned value must contain only non-secret outcome metadata.
    async fn enroll(&self, request: EnrollRequest) -> Result<serde_json::Value, SetupError>;
    /// Inspect credential presence through the daemon-owned store, without reading values.
    async fn auth_status(&self) -> Result<serde_json::Value, SetupError> {
        Err(SetupError::Unavailable)
    }
}

pub(crate) async fn handle(
    frame: &[u8],
    handler: Option<&dyn LocalSetupHandler>,
) -> Option<Vec<u8>> {
    if frame.len() > local_setup::MAX_FRAME_BYTES {
        return None;
    }
    let request: local_setup::RequestEnvelope = serde_json::from_slice(frame).ok()?;
    if !request.valid() {
        return None;
    }
    let outcome = match handler {
        Some(handler) => match request.request {
            LocalSetupRequest::Enroll(request) => handler.enroll(request).await,
            LocalSetupRequest::AuthStatus {} => handler.auth_status().await,
        },
        None => Err(SetupError::Unavailable),
    };
    let response = local_setup::ResponseEnvelope {
        protocol: local_setup::CONTRACT.into(),
        request_id: request.request_id,
        result: outcome.as_ref().ok().cloned(),
        error: outcome.err(),
    };
    let bytes = serde_json::to_vec(&response).ok()?;
    (bytes.len() <= local_setup::MAX_FRAME_BYTES).then_some(bytes)
}

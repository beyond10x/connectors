//! Owner-only daemon lifecycle client. No process is signalled by PID.

use protocol::lifecycle::{self, DaemonStatus, LifecycleRequest};

use crate::{request_id, ClientError, LocalClient};

impl LocalClient {
    /// Inspect or gracefully stop the process owning this local control socket.
    pub async fn lifecycle(&self, request: LifecycleRequest) -> Result<DaemonStatus, ClientError> {
        let request_id = request_id();
        let envelope = lifecycle::RequestEnvelope {
            protocol: lifecycle::CONTRACT.to_owned(),
            request_id: request_id.clone(),
            request,
        };
        let response: lifecycle::ResponseEnvelope = self
            .exchange(
                &envelope,
                lifecycle::MAX_FRAME_BYTES,
                lifecycle::MAX_FRAME_BYTES,
            )
            .await?;
        if !response.valid_for(&request_id)
            || response.response.stopping != matches!(request, LifecycleRequest::Stop {})
        {
            return Err(ClientError::InvalidResponse);
        }
        Ok(response.response)
    }
}

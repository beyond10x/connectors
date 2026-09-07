//! Confidential local enrollment framing; no replay or automatic retry.

use protocol::local_setup::{self, EnrollRequest, LocalSetupRequest};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::UnixStream;
use zeroize::Zeroizing;

use crate::{request_id, ClientError, LocalClient};

impl LocalClient {
    /// Send one confidential enrollment request to the running local daemon.
    pub async fn enroll(&self, request: EnrollRequest) -> Result<serde_json::Value, ClientError> {
        self.local_setup(LocalSetupRequest::Enroll(request)).await
    }

    /// Inspect credential presence in the daemon-owned store.
    pub async fn auth_status(&self) -> Result<serde_json::Value, ClientError> {
        self.local_setup(LocalSetupRequest::AuthStatus {}).await
    }

    async fn local_setup(
        &self,
        request: LocalSetupRequest,
    ) -> Result<serde_json::Value, ClientError> {
        let envelope = local_setup::RequestEnvelope {
            protocol: local_setup::CONTRACT.into(),
            request_id: request_id(),
            request,
        };
        if !envelope.valid() {
            return Err(ClientError::InvalidRequest(
                "enrollment input was refused".into(),
            ));
        }
        let mut bytes = Zeroizing::new(serde_json::to_vec(&envelope)?);
        if bytes.len() > local_setup::MAX_FRAME_BYTES {
            return Err(ClientError::InvalidRequest(
                "enrollment request exceeds its bound".into(),
            ));
        }
        bytes.push(b'\n');
        // An acquisition may have reached the provider when the transport fails. Never retry it.
        let response: local_setup::ResponseEnvelope =
            tokio::time::timeout(Duration::from_secs(120), async {
                let mut stream = UnixStream::connect(&self.socket).await?;
                stream.write_all(&bytes).await?;
                stream.shutdown().await?;
                let mut response = Zeroizing::new(String::new());
                BufReader::new(stream)
                    .take((local_setup::MAX_FRAME_BYTES + 1) as u64)
                    .read_line(&mut response)
                    .await?;
                if response.is_empty() || response.len() > local_setup::MAX_FRAME_BYTES {
                    return Err(ClientError::InvalidResponse);
                }
                serde_json::from_str(&response).map_err(ClientError::from)
            })
            .await
            .map_err(|_| {
                ClientError::ConnectionRefused(local_setup::SetupError::OutcomeUnknown.to_string())
            })??;
        if response.protocol != local_setup::CONTRACT || response.request_id != envelope.request_id
        {
            return Err(ClientError::InvalidResponse);
        }
        match (response.result, response.error) {
            (Some(value), None) => Ok(value),
            (None, Some(error)) => Err(ClientError::ConnectionRefused(error.to_string())),
            _ => Err(ClientError::InvalidResponse),
        }
    }
}

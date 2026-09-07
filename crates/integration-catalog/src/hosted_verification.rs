//! Observe only closed outcome metadata from the ordinary verification transport.
//!
//! The catalogue still assembles, admits and executes the exact declared operation. This wrapper
//! neither issues a second request nor retains headers, body, URL or an underlying error string.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use service::{
    EgressHttpRequest, EgressHttpResponse, EgressTransport, EgressTransportError, EgressWebSocket,
    HostedVerificationFailure,
};

pub(super) struct VerificationEgress {
    inner: Arc<dyn EgressTransport>,
    failure: Mutex<Option<HostedVerificationFailure>>,
}

impl VerificationEgress {
    pub(super) fn new(inner: Arc<dyn EgressTransport>) -> Self {
        Self {
            inner,
            failure: Mutex::new(None),
        }
    }

    pub(super) fn failure(&self) -> HostedVerificationFailure {
        self.failure
            .lock()
            .ok()
            .and_then(|failure| *failure)
            .unwrap_or(HostedVerificationFailure::Preparation)
    }
}

#[async_trait]
impl EgressTransport for VerificationEgress {
    async fn execute(
        &self,
        authority_ref: &str,
        request: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError> {
        let result = self.inner.execute(authority_ref, request).await;
        let failure = match &result {
            Ok(response) if !response.is_success() => {
                Some(HostedVerificationFailure::ProviderStatus(response.status))
            }
            Ok(_) => None,
            Err(EgressTransportError::Refused) => {
                Some(HostedVerificationFailure::DestinationRefused)
            }
            Err(EgressTransportError::ResponseTooLarge) => {
                Some(HostedVerificationFailure::ResponseTooLarge)
            }
            Err(EgressTransportError::Transport(failure)) => {
                Some(HostedVerificationFailure::Transport(*failure))
            }
        };
        if let Ok(mut observed) = self.failure.lock() {
            *observed = failure;
        }
        result
    }

    async fn connect_websocket(
        &self,
        _authority_ref: &str,
        _url: String,
        _maximum_message_bytes: usize,
    ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
        Err(EgressTransportError::Refused)
    }
}

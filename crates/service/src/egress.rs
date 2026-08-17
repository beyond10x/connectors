//! Transport-neutral outbound port for credential-bearing Integration adapters.
//!
//! Integrations assemble governed requests and consume bounded responses. Runtime composition
//! injects the only implementation that may resolve DNS or open provider sockets.

use std::collections::BTreeMap;

use async_trait::async_trait;
use connector_resolve::Request;

/// One bounded HTTP exchange. Response headers are an explicit allowlist because returning every
/// provider header would create an accidental credential and cookie projection.
pub struct EgressHttpRequest {
    pub request: Request,
    pub maximum_response_bytes: usize,
    pub response_headers: Vec<String>,
}

/// A bounded response whose body and selected headers stay inside the owning Integration.
pub struct EgressHttpResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl EgressHttpResponse {
    #[must_use]
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}

/// Provider-neutral WebSocket frames needed by supervised Connector channels.
pub enum EgressWebSocketFrame {
    Text(String),
    Ping(Vec<u8>),
    Closed,
    Other,
}

#[async_trait]
pub trait EgressWebSocket: Send + Unpin {
    async fn receive(&mut self) -> Result<EgressWebSocketFrame, EgressTransportError>;
    async fn send_text(&mut self, text: String) -> Result<(), EgressTransportError>;
    async fn send_pong(&mut self, payload: Vec<u8>) -> Result<(), EgressTransportError>;
    async fn close(&mut self) -> Result<(), EgressTransportError>;
}

/// The runtime-owned network capability. An adapter cannot construct or recover a socket from
/// this interface, and every call must carry its admitted Connection or Connect Session identity.
#[async_trait]
pub trait EgressTransport: Send + Sync + 'static {
    async fn execute(
        &self,
        authority_ref: &str,
        request: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError>;

    async fn connect_websocket(
        &self,
        authority_ref: &str,
        url: String,
        maximum_message_bytes: usize,
    ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError>;
}

/// Redaction-safe refusal at the outbound network boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EgressTransportError {
    #[error("Connector egress transport refused the exchange")]
    Refused,
    #[error("Connector egress response exceeded its admitted bound")]
    ResponseTooLarge,
}

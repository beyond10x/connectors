//! Connecting-side authenticated upgrade over a deployment-established TLS stream.

use std::sync::Arc;

use service::authority::{
    AuthorityError, IssuedAuthority, ProofKey, AUTHORIZATION_SCHEME, DPOP_HEADER,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_tungstenite::tungstenite::http::{header, HeaderValue};

use crate::bounded_ws::{BoundedWsTransport, Bounds};
use crate::{media_format, PROFILE};

/// Connecting upgrade failure. Compact credentials are never retained or displayed.
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// Authority cannot produce the exact DPoP presentation.
    #[error("session authority presentation failed: {0}")]
    Authority(#[from] AuthorityError),
    /// The admitted external endpoint cannot form a WebSocket request.
    #[error("admitted application endpoint is invalid")]
    InvalidEndpoint,
    /// A compact value cannot be represented as one HTTP header.
    #[error("session authority presentation is invalid")]
    InvalidPresentation,
    /// The serving peer refused or failed the handshake.
    #[error("application WebSocket handshake failed")]
    Handshake,
    /// The peer did not explicitly select the exact binding profile.
    #[error("application did not select the exact RTVBP voice profile")]
    ProfileRefused,
    /// Locally bounded transport configuration failed.
    #[error("bounded transport could not start: {0}")]
    Transport(String),
}

/// Connect over an already established TLS stream and start the finite RTVBP transport.
///
/// DNS, TCP, proxy routing, and TLS policy remain owned by the selected deployment/network path.
/// This function owns only the exact HTTP upgrade, authority presentation, profile verification,
/// and subsequent bounded semantic transport.
pub async fn connect_authenticated<S>(
    stream: S,
    authority: &IssuedAuthority,
    proof_key: &ProofKey,
    now_epoch_seconds: u64,
    proof_id: impl Into<String>,
    bounds: Bounds,
) -> Result<Arc<BoundedWsTransport>, ConnectError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    if authority.claims().dl_protocol != PROFILE {
        return Err(ConnectError::ProfileRefused);
    }
    let endpoint = &authority.claims().dl_endpoint;
    let proof = proof_key.proof("GET", endpoint, authority, now_epoch_seconds, proof_id)?;
    let mut request = endpoint
        .as_str()
        .into_client_request()
        .map_err(|_| ConnectError::InvalidEndpoint)?;
    request.headers_mut().insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!(
            "{} {}",
            AUTHORIZATION_SCHEME,
            authority.compact().expose_secret()
        ))
        .map_err(|_| ConnectError::InvalidPresentation)?,
    );
    request.headers_mut().insert(
        DPOP_HEADER,
        HeaderValue::from_str(proof.expose_secret())
            .map_err(|_| ConnectError::InvalidPresentation)?,
    );
    request.headers_mut().insert(
        header::SEC_WEBSOCKET_PROTOCOL,
        HeaderValue::from_static(PROFILE),
    );
    let (upgraded, response) = tokio_tungstenite::client_async(request, stream)
        .await
        .map_err(|_| ConnectError::Handshake)?;
    let mut selected = response
        .headers()
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .iter();
    if selected.next().and_then(|value| value.to_str().ok()) != Some(PROFILE)
        || selected.next().is_some()
    {
        return Err(ConnectError::ProfileRefused);
    }
    BoundedWsTransport::start(upgraded, bounds, media_format())
        .map_err(|error| ConnectError::Transport(error.to_string()))
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::SigningKey;
    use futures_util::SinkExt as _;
    use rtvbp::Transport as _;
    use service::authority::{AuthorityIssuer, IssueRequest};
    use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
    use tokio_tungstenite::tungstenite::Message;

    use super::*;

    #[tokio::test]
    #[allow(clippy::result_large_err)]
    async fn exact_authenticated_client_upgrade_starts_bounded_media() {
        let issuer = AuthorityIssuer::new("issuer", "key-1", SigningKey::from_bytes(&[3; 32]));
        let proof_key = ProofKey::from_bytes(&[4; 32]);
        let endpoint = "wss://application.example/voice".to_owned();
        let authority = issuer
            .issue(IssueRequest {
                audience: "application".to_owned(),
                subject: "principal".to_owned(),
                actor: "voice".to_owned(),
                organization: "org".to_owned(),
                deployment: "application".to_owned(),
                connection: "connection".to_owned(),
                grant: "grant".to_owned(),
                resource: "endpoint".to_owned(),
                operation: "call".to_owned(),
                channel_kind: "voice".to_owned(),
                protocol: PROFILE.to_owned(),
                endpoint,
                proof_thumbprint: proof_key.thumbprint(),
                authority_id: "authority".to_owned(),
                issued_at: 100,
                not_before: 100,
                expires_at: 160,
                lease_expires_at: 1_000,
            })
            .unwrap();
        let (client_io, server_io) = tokio::io::duplex(16_384);
        let server = async {
            let mut socket = tokio_tungstenite::accept_hdr_async(
                server_io,
                |request: &Request, mut response: Response| {
                    assert_eq!(request.uri().path(), "/voice");
                    assert!(request
                        .headers()
                        .get(header::AUTHORIZATION)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with(&format!("{AUTHORIZATION_SCHEME} ")));
                    assert!(request.headers().contains_key(DPOP_HEADER));
                    assert_eq!(
                        request.headers().get(header::SEC_WEBSOCKET_PROTOCOL),
                        Some(&HeaderValue::from_static(PROFILE))
                    );
                    response.headers_mut().insert(
                        header::SEC_WEBSOCKET_PROTOCOL,
                        HeaderValue::from_static(PROFILE),
                    );
                    Ok(response)
                },
            )
            .await
            .unwrap();
            socket
                .send(Message::Binary(vec![7; 320].into()))
                .await
                .unwrap();
            socket.close(None).await.unwrap();
        };
        let (transport, ()) = tokio::join!(
            connect_authenticated(
                client_io,
                &authority,
                &proof_key,
                101,
                "proof-1",
                Bounds::voice_v1(),
            ),
            server,
        );
        let transport = transport.unwrap();
        let media = transport.accept_media().await.unwrap();
        assert_eq!(media.read_frame().await.unwrap().data, vec![7; 320]);
    }
}

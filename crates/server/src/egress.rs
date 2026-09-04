//! Connection-bound outbound transport with post-DNS destination enforcement.
//!
//! Integrations provide deployment-selected destination rules. Every request is checked against
//! those rules, resolved exactly once, and the resulting socket addresses are pinned into the
//! transport. Redirects and ambient proxies are disabled, so neither a caller nor a later DNS
//! answer can move a credential-bearing request outside the Connection's aperture.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{SinkExt as _, StreamExt as _};
use reqwest::{Method, RequestBuilder};
use service::{
    EgressByteStream, EgressHttpRequest, EgressHttpResponse, EgressStreamingHttpRequest,
    EgressStreamingHttpResponse, EgressTransport, EgressTransportError, EgressTransportFailure,
    EgressWebSocket, EgressWebSocketFrame,
};
use tokio::net::{lookup_host, TcpStream};
use tokio_tungstenite::tungstenite::handshake::client::Response;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use url::Url;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const STREAM_TOTAL_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const STREAM_HEADER_TIMEOUT: Duration = Duration::from_secs(30);
const STREAM_BODY_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const USER_AGENT: &str = "b10x-connectors/0.1";
const MAX_RESPONSE_HEADERS: usize = 16;
const MAX_RESPONSE_HEADER_BYTES: usize = 8 * 1024;
const MAX_WEBSOCKET_MESSAGE_BYTES: usize = 1024 * 1024;
const MAX_REQUEST_BYTES: usize = 256 * 1024;
const MAX_REQUEST_HEADERS: usize = 64;
const MAX_URL_BYTES: usize = 8 * 1024;
const MAX_STREAM_RESPONSE_BYTES: u64 = 1024 * 1024 * 1024;

/// Address classes admitted after DNS resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressScope {
    /// Public destinations only. Any private, local, reserved, or mixed answer refuses.
    Public,
    /// Operator-selected public or private network destinations. Local, link-local, multicast,
    /// documentation, and otherwise non-routable answers still refuse.
    OperatorNetwork,
}

/// One deployment-selected destination rule. Callers can select neither rules nor address scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestinationRule {
    scheme: String,
    host: HostRule,
    port: u16,
    scope: AddressScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HostRule {
    Exact(String),
    Suffix(String),
}

impl DestinationRule {
    /// Admit one exact origin. Paths, query, fragments, and userinfo are not origin policy.
    pub fn exact_origin(origin: &str, scope: AddressScope) -> Result<Self, EgressError> {
        let url = Url::parse(origin).map_err(|_| EgressError::InvalidRule)?;
        let scheme = url.scheme();
        let host = url.host_str().ok_or(EgressError::InvalidRule)?;
        let port = url
            .port_or_known_default()
            .ok_or(EgressError::InvalidRule)?;
        if !matches!(scheme, "https" | "wss")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(EgressError::InvalidRule);
        }
        Ok(Self {
            scheme: scheme.to_owned(),
            host: HostRule::Exact(host.to_ascii_lowercase()),
            port,
            scope,
        })
    }

    /// Admit provider-selected hosts beneath one exact DNS suffix. This is reserved for protocols
    /// such as Slack Socket Mode where the authenticated provider returns the WebSocket host.
    pub fn dns_suffix(
        scheme: &str,
        suffix: &str,
        port: u16,
        scope: AddressScope,
    ) -> Result<Self, EgressError> {
        let suffix = suffix.to_ascii_lowercase();
        if !matches!(scheme, "https" | "wss") || port == 0 || !valid_dns_suffix(&suffix) {
            return Err(EgressError::InvalidRule);
        }
        Ok(Self {
            scheme: scheme.to_owned(),
            host: HostRule::Suffix(suffix),
            port,
            scope,
        })
    }

    fn matches(&self, url: &Url) -> bool {
        let Some(host) = url.host_str() else {
            return false;
        };
        if url.scheme() != self.scheme || url.port_or_known_default() != Some(self.port) {
            return false;
        }
        let host = host.to_ascii_lowercase();
        match &self.host {
            HostRule::Exact(expected) => &host == expected,
            HostRule::Suffix(suffix) => {
                host.len() > suffix.len() && host.ends_with(suffix.as_str())
            }
        }
    }
}

/// Closed destination policy shared by the HTTP and WebSocket egress paths.
#[derive(Debug, Clone)]
pub struct ConnectionEgress {
    rules: Vec<DestinationRule>,
}

impl ConnectionEgress {
    pub fn new(rules: Vec<DestinationRule>) -> Result<Self, EgressError> {
        if rules.is_empty() {
            return Err(EgressError::InvalidRule);
        }
        Ok(Self { rules })
    }

    /// Resolve every exact rule during startup. Suffix rules are necessarily resolved only after
    /// the authenticated provider supplies a concrete host, then pinned for that connection.
    pub async fn preflight_exact(&self) -> Result<(), EgressError> {
        for rule in &self.rules {
            if let HostRule::Exact(host) = &rule.host {
                let authority = host
                    .parse::<Ipv6Addr>()
                    .map_or_else(|_| host.clone(), |host| format!("[{host}]"));
                let url = Url::parse(&format!("{}://{}:{}", rule.scheme, authority, rule.port))
                    .map_err(|_| EgressError::InvalidRule)?;
                self.resolve(&url).await?;
            }
        }
        Ok(())
    }

    /// Build a request whose DNS result is pinned and whose URL already passed the Connection's
    /// exact destination policy. The returned builder cannot change its target URL.
    async fn request(
        &self,
        authority_ref: &str,
        method: Method,
        url: Url,
        timeout: Duration,
    ) -> Result<RequestBuilder, EgressError> {
        validate_authority_ref(authority_ref)?;
        let destination = self.resolve(&url).await?;
        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(timeout)
            .user_agent(USER_AGENT);
        if url
            .host()
            .is_some_and(|host| matches!(host, url::Host::Domain(_)))
        {
            builder = builder.resolve_to_addrs(&destination.host, &destination.addresses);
        }
        let client = builder.build().map_err(|_| EgressError::Transport)?;
        Ok(client.request(method, url))
    }

    /// Connect a WebSocket to a post-DNS-pinned address while retaining the admitted hostname for
    /// the TLS server name and HTTP Host header.
    async fn open_websocket(
        &self,
        authority_ref: &str,
        url: &Url,
        config: WebSocketConfig,
    ) -> Result<(PinnedWebSocket, Response), EgressError> {
        validate_authority_ref(authority_ref)?;
        if url.scheme() != "wss" {
            return Err(EgressError::DestinationDenied);
        }
        let destination = self.resolve(url).await?;
        for address in destination.addresses {
            let stream =
                match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(address)).await {
                    Ok(Ok(stream)) => stream,
                    Ok(Err(_)) | Err(_) => continue,
                };
            let handshake = tokio_tungstenite::client_async_tls_with_config(
                url.as_str(),
                stream,
                Some(config),
                None,
            );
            if let Ok(Ok(connected)) = tokio::time::timeout(REQUEST_TIMEOUT, handshake).await {
                return Ok(connected);
            }
        }
        Err(EgressError::Transport)
    }

    async fn resolve(&self, url: &Url) -> Result<ResolvedDestination, EgressError> {
        validate_url(url)?;
        let rule = self
            .rules
            .iter()
            .find(|rule| rule.matches(url))
            .ok_or(EgressError::DestinationDenied)?;
        let host = url.host_str().ok_or(EgressError::DestinationDenied)?;
        let port = url
            .port_or_known_default()
            .ok_or(EgressError::DestinationDenied)?;
        let mut addresses = tokio::time::timeout(CONNECT_TIMEOUT, lookup_host((host, port)))
            .await
            .map_err(|_| EgressError::Resolution)?
            .map_err(|_| EgressError::Resolution)?
            .collect::<Vec<_>>();
        addresses.sort_unstable();
        addresses.dedup();
        validate_addresses(rule.scope, &addresses)?;
        Ok(ResolvedDestination {
            host: host.to_owned(),
            addresses,
        })
    }
}

pub type PinnedWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[async_trait]
impl EgressTransport for ConnectionEgress {
    async fn execute(
        &self,
        authority_ref: &str,
        request: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError> {
        if request.maximum_response_bytes == 0
            || request.maximum_response_bytes > protocol::operation::MAX_RESULT_BYTES
            || request.response_headers.len() > MAX_RESPONSE_HEADERS
            || request.request.url.len() > MAX_URL_BYTES
            || request.request.headers.len() > MAX_REQUEST_HEADERS
            || request.request.headers.iter().any(|(name, value)| {
                name.len() > MAX_RESPONSE_HEADER_BYTES || value.len() > MAX_RESPONSE_HEADER_BYTES
            })
            || request
                .request
                .body
                .as_ref()
                .is_some_and(|body| body.len() > MAX_REQUEST_BYTES)
        {
            return Err(EgressTransportError::Refused);
        }
        let method = Method::from_bytes(request.request.method.as_bytes())
            .map_err(|_| EgressTransportError::Refused)?;
        let url = Url::parse(&request.request.url).map_err(|_| EgressTransportError::Refused)?;
        if url.scheme() != "https" {
            return Err(EgressTransportError::Refused);
        }
        let mut outbound = self
            .request(authority_ref, method, url, REQUEST_TIMEOUT)
            .await
            .map_err(|_| EgressTransportError::Refused)?;
        for (name, value) in request.request.headers {
            let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| EgressTransportError::Refused)?;
            let value = reqwest::header::HeaderValue::from_str(&value)
                .map_err(|_| EgressTransportError::Refused)?;
            outbound = outbound.header(name, value);
        }
        if let Some(body) = request.request.body {
            outbound = outbound.body(body);
        }
        let mut response = outbound
            .send()
            .await
            .map_err(|error| EgressTransportError::Transport(classify_send_failure(&error)))?;
        if response
            .content_length()
            .is_some_and(|size| size > request.maximum_response_bytes as u64)
        {
            return Err(EgressTransportError::ResponseTooLarge);
        }
        let mut headers = std::collections::BTreeMap::new();
        for requested in request.response_headers {
            let name = reqwest::header::HeaderName::from_bytes(requested.as_bytes())
                .map_err(|_| EgressTransportError::Refused)?;
            if let Some(value) = response.headers().get(&name) {
                let value = value.to_str().map_err(|_| EgressTransportError::Refused)?;
                if value.len() > MAX_RESPONSE_HEADER_BYTES {
                    return Err(EgressTransportError::Refused);
                }
                headers.insert(name.as_str().to_owned(), value.to_owned());
            }
        }
        let status = response.status().as_u16();
        let mut body = Vec::with_capacity(
            response
                .content_length()
                .unwrap_or(0)
                .min(request.maximum_response_bytes as u64) as usize,
        );
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| EgressTransportError::Transport(EgressTransportFailure::BodyRead))?
        {
            if body
                .len()
                .checked_add(chunk.len())
                .is_none_or(|size| size > request.maximum_response_bytes)
            {
                return Err(EgressTransportError::ResponseTooLarge);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(EgressHttpResponse {
            status,
            headers,
            body,
        })
    }

    async fn execute_stream(
        &self,
        authority_ref: &str,
        request: EgressStreamingHttpRequest,
    ) -> Result<EgressStreamingHttpResponse, EgressTransportError> {
        if request.maximum_response_bytes == 0
            || request.maximum_response_bytes > MAX_STREAM_RESPONSE_BYTES
            || request.response_headers.len() > MAX_RESPONSE_HEADERS
            || request.url.len() > MAX_URL_BYTES
            || request.headers.len() > MAX_REQUEST_HEADERS
            || request.headers.iter().any(|(name, value)| {
                name.len() > MAX_RESPONSE_HEADER_BYTES || value.len() > MAX_RESPONSE_HEADER_BYTES
            })
            || request
                .body
                .as_ref()
                .is_some_and(|body| body.len() > MAX_REQUEST_BYTES)
        {
            return Err(EgressTransportError::Refused);
        }
        let method = Method::from_bytes(request.method.as_bytes())
            .map_err(|_| EgressTransportError::Refused)?;
        let url = Url::parse(&request.url).map_err(|_| EgressTransportError::Refused)?;
        if url.scheme() != "https" {
            return Err(EgressTransportError::Refused);
        }
        let mut outbound = self
            .request(authority_ref, method, url, STREAM_TOTAL_TIMEOUT)
            .await
            .map_err(|_| EgressTransportError::Refused)?;
        for (name, value) in request.headers {
            let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| EgressTransportError::Refused)?;
            let value = reqwest::header::HeaderValue::from_str(&value)
                .map_err(|_| EgressTransportError::Refused)?;
            outbound = outbound.header(name, value);
        }
        if let Some(body) = request.body {
            outbound = outbound.body(body);
        }
        let response = tokio::time::timeout(STREAM_HEADER_TIMEOUT, outbound.send())
            .await
            .map_err(|_| EgressTransportError::Transport(EgressTransportFailure::Timeout))?
            .map_err(|error| EgressTransportError::Transport(classify_send_failure(&error)))?;
        if response
            .content_length()
            .is_some_and(|size| size > request.maximum_response_bytes)
        {
            return Err(EgressTransportError::ResponseTooLarge);
        }
        let status = response.status().as_u16();
        let mut headers = std::collections::BTreeMap::new();
        for requested in request.response_headers {
            let name = reqwest::header::HeaderName::from_bytes(requested.as_bytes())
                .map_err(|_| EgressTransportError::Refused)?;
            if let Some(value) = response.headers().get(&name) {
                let value = value.to_str().map_err(|_| EgressTransportError::Refused)?;
                if value.len() > MAX_RESPONSE_HEADER_BYTES {
                    return Err(EgressTransportError::Refused);
                }
                headers.insert(name.as_str().to_owned(), value.to_owned());
            }
        }
        Ok(EgressStreamingHttpResponse {
            status,
            headers,
            body: Box::new(ReqwestByteStream {
                response,
                observed: 0,
                maximum: request.maximum_response_bytes,
                deadline: tokio::time::Instant::now() + STREAM_TOTAL_TIMEOUT,
            }),
        })
    }

    async fn connect_websocket(
        &self,
        authority_ref: &str,
        url: String,
        maximum_message_bytes: usize,
    ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
        if maximum_message_bytes == 0 || maximum_message_bytes > MAX_WEBSOCKET_MESSAGE_BYTES {
            return Err(EgressTransportError::Refused);
        }
        let url = Url::parse(&url).map_err(|_| EgressTransportError::Refused)?;
        let config = WebSocketConfig::default()
            .max_message_size(Some(maximum_message_bytes))
            .max_frame_size(Some(maximum_message_bytes));
        let (socket, _) = self
            .open_websocket(authority_ref, &url, config)
            .await
            .map_err(|_| EgressTransportError::Refused)?;
        Ok(Box::new(ServerWebSocket { socket }))
    }
}

struct ReqwestByteStream {
    response: reqwest::Response,
    observed: u64,
    maximum: u64,
    deadline: tokio::time::Instant,
}

#[async_trait]
impl EgressByteStream for ReqwestByteStream {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
        let remaining = self
            .deadline
            .checked_duration_since(tokio::time::Instant::now())
            .ok_or(EgressTransportError::Transport(
                EgressTransportFailure::Timeout,
            ))?;
        let chunk = tokio::time::timeout(
            remaining.min(STREAM_BODY_IDLE_TIMEOUT),
            self.response.chunk(),
        )
        .await
        .map_err(|_| EgressTransportError::Transport(EgressTransportFailure::Timeout))?
        .map_err(|_| EgressTransportError::Transport(EgressTransportFailure::BodyRead))?;
        let Some(chunk) = chunk else {
            return Ok(None);
        };
        self.observed = self
            .observed
            .checked_add(chunk.len() as u64)
            .filter(|observed| *observed <= self.maximum)
            .ok_or(EgressTransportError::ResponseTooLarge)?;
        Ok(Some(chunk.to_vec()))
    }
}

struct ServerWebSocket {
    socket: PinnedWebSocket,
}

#[async_trait]
impl EgressWebSocket for ServerWebSocket {
    async fn receive(&mut self) -> Result<EgressWebSocketFrame, EgressTransportError> {
        match self.socket.next().await {
            Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => {
                Ok(EgressWebSocketFrame::Text(text.to_string()))
            }
            Some(Ok(tokio_tungstenite::tungstenite::Message::Ping(payload))) => {
                Ok(EgressWebSocketFrame::Ping(payload.to_vec()))
            }
            Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) | None => {
                Ok(EgressWebSocketFrame::Closed)
            }
            Some(Ok(_)) => Ok(EgressWebSocketFrame::Other),
            Some(Err(_)) => Err(EgressTransportError::Refused),
        }
    }

    async fn send_text(&mut self, text: String) -> Result<(), EgressTransportError> {
        self.socket
            .send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
            .await
            .map_err(|_| EgressTransportError::Refused)
    }

    async fn send_pong(&mut self, payload: Vec<u8>) -> Result<(), EgressTransportError> {
        self.socket
            .send(tokio_tungstenite::tungstenite::Message::Pong(
                payload.into(),
            ))
            .await
            .map_err(|_| EgressTransportError::Refused)
    }

    async fn close(&mut self) -> Result<(), EgressTransportError> {
        self.socket
            .close(None)
            .await
            .map_err(|_| EgressTransportError::Refused)
    }
}

struct ResolvedDestination {
    host: String,
    addresses: Vec<SocketAddr>,
}

/// Classify a failed send into the closed egress vocabulary (S-066). Only the class leaves this
/// function: the error's Display string can embed the full request URL, so it never travels.
/// A TLS failure surfaces inside reqwest's connect phase, so the source chain is inspected for a
/// TLS or certificate marker before the connect class is chosen; the marker match is textual
/// because this transport deliberately depends on no TLS backend type to downcast to.
fn classify_send_failure(error: &reqwest::Error) -> EgressTransportFailure {
    if error.is_timeout() {
        return EgressTransportFailure::Timeout;
    }
    let mut source = std::error::Error::source(error);
    while let Some(current) = source {
        let text = current.to_string().to_ascii_lowercase();
        if text.contains("tls") || text.contains("certificate") || text.contains("handshake") {
            return EgressTransportFailure::Tls;
        }
        source = current.source();
    }
    if error.is_connect() {
        EgressTransportFailure::Connect
    } else {
        EgressTransportFailure::Other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EgressError {
    #[error("egress destination rule is invalid")]
    InvalidRule,
    #[error("egress Connection or Connect Session reference is invalid")]
    InvalidAuthority,
    #[error("egress destination is outside the Connection policy")]
    DestinationDenied,
    #[error("egress destination could not be resolved")]
    Resolution,
    #[error("egress destination resolved outside its admitted address class")]
    AddressDenied,
    #[error("egress transport failed")]
    Transport,
}

fn validate_authority_ref(value: &str) -> Result<(), EgressError> {
    let suffix = value
        .strip_prefix("connection:")
        .or_else(|| value.strip_prefix("connect-session:"));
    if suffix.is_none_or(str::is_empty)
        || value.len() > 512
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(EgressError::InvalidAuthority);
    }
    Ok(())
}

fn validate_url(url: &Url) -> Result<(), EgressError> {
    if !matches!(url.scheme(), "https" | "wss")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(EgressError::DestinationDenied);
    }
    Ok(())
}

fn valid_dns_suffix(value: &str) -> bool {
    value.starts_with('.')
        && value.len() >= 3
        && value[1..].split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

fn validate_addresses(scope: AddressScope, addresses: &[SocketAddr]) -> Result<(), EgressError> {
    if addresses.is_empty()
        || addresses.iter().any(|address| {
            let ip = normalize_ip(address.ip());
            !is_operator_network(ip) || (scope == AddressScope::Public && !is_public(ip))
        })
    {
        return Err(EgressError::AddressDenied);
    }
    Ok(())
}

fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ip) => ip.to_ipv4_mapped().map_or(IpAddr::V6(ip), IpAddr::V4),
        other => other,
    }
}

fn is_operator_network(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            !ip.is_unspecified()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_multicast()
                && ip != Ipv4Addr::BROADCAST
                && !is_v4_documentation(ip)
                && !in_v4(ip, [0, 0, 0, 0], 8)
                && !in_v4(ip, [240, 0, 0, 0], 4)
        }
        IpAddr::V6(ip) => {
            !ip.is_unspecified()
                && !ip.is_loopback()
                && !ip.is_multicast()
                && !is_v6_link_local(ip)
                && !is_v6_documentation(ip)
                && (is_v6_unique_local(ip) || in_v6(ip, 0x2000_u128 << 112, 3))
        }
    }
}

fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            is_operator_network(IpAddr::V4(ip))
                && !ip.is_private()
                && !in_v4(ip, [100, 64, 0, 0], 10)
                && !in_v4(ip, [192, 0, 0, 0], 24)
                && !in_v4(ip, [198, 18, 0, 0], 15)
        }
        IpAddr::V6(ip) => {
            is_operator_network(IpAddr::V6(ip))
                && !is_v6_unique_local(ip)
                && in_v6(ip, 0x2000_u128 << 112, 3)
        }
    }
}

fn in_v4(ip: Ipv4Addr, network: [u8; 4], prefix: u32) -> bool {
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    u32::from(ip) & mask == u32::from(Ipv4Addr::from(network)) & mask
}

fn is_v4_documentation(ip: Ipv4Addr) -> bool {
    in_v4(ip, [192, 0, 2, 0], 24)
        || in_v4(ip, [198, 51, 100, 0], 24)
        || in_v4(ip, [203, 0, 113, 0], 24)
}

fn is_v6_link_local(ip: Ipv6Addr) -> bool {
    ip.segments()[0] & 0xffc0 == 0xfe80
}

fn in_v6(ip: Ipv6Addr, network: u128, prefix: u32) -> bool {
    let mask = if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix)
    };
    u128::from(ip) & mask == network & mask
}

fn is_v6_unique_local(ip: Ipv6Addr) -> bool {
    ip.segments()[0] & 0xfe00 == 0xfc00
}

fn is_v6_documentation(ip: Ipv6Addr) -> bool {
    ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_origin_cannot_be_widened_by_path_host_or_userinfo() {
        let rule = DestinationRule::exact_origin(
            "https://grafana.example.test",
            AddressScope::OperatorNetwork,
        )
        .unwrap();
        assert!(rule.matches(&Url::parse("https://grafana.example.test/api/search").unwrap()));
        assert!(!rule.matches(&Url::parse("https://other.example.test/api/search").unwrap()));
        assert!(DestinationRule::exact_origin(
            "https://user@grafana.example.test",
            AddressScope::OperatorNetwork
        )
        .is_err());
    }

    #[test]
    fn suffix_rule_requires_a_real_child_and_the_exact_scheme_and_port() {
        let rule =
            DestinationRule::dns_suffix("wss", ".slack.com", 443, AddressScope::Public).unwrap();
        assert!(
            rule.matches(&Url::parse("wss://wss-primary.slack.com/link/?ticket=opaque").unwrap())
        );
        assert!(!rule.matches(&Url::parse("wss://slack.com/link/?ticket=opaque").unwrap()));
        assert!(!rule.matches(
            &Url::parse("wss://wss-primary.slack.com.example/link/?ticket=opaque").unwrap()
        ));
        assert!(!rule
            .matches(&Url::parse("https://wss-primary.slack.com/link/?ticket=opaque").unwrap()));
        assert!(
            DestinationRule::dns_suffix("wss", ".bad..example", 443, AddressScope::Public).is_err()
        );
    }

    #[test]
    fn public_dns_refuses_private_local_reserved_and_mixed_answers() {
        let public = SocketAddr::from(([93, 184, 216, 34], 443));
        for denied in [
            SocketAddr::from(([10, 0, 0, 1], 443)),
            SocketAddr::from(([127, 0, 0, 1], 443)),
            SocketAddr::from(([169, 254, 1, 1], 443)),
            SocketAddr::from(([192, 0, 2, 1], 443)),
            SocketAddr::from(([100, 64, 0, 1], 443)),
        ] {
            assert_eq!(
                validate_addresses(AddressScope::Public, &[denied]),
                Err(EgressError::AddressDenied)
            );
            assert_eq!(
                validate_addresses(AddressScope::Public, &[public, denied]),
                Err(EgressError::AddressDenied),
                "mixed answers fail closed"
            );
        }
        assert_eq!(validate_addresses(AddressScope::Public, &[public]), Ok(()));
    }

    #[test]
    fn operator_network_may_admit_private_but_not_process_local_addresses() {
        assert_eq!(
            validate_addresses(
                AddressScope::OperatorNetwork,
                &[SocketAddr::from(([10, 20, 30, 40], 443))]
            ),
            Ok(())
        );
        assert_eq!(
            validate_addresses(
                AddressScope::OperatorNetwork,
                &[SocketAddr::from(([127, 0, 0, 1], 443))]
            ),
            Err(EgressError::AddressDenied)
        );
    }

    #[test]
    fn ipv4_mapped_ipv6_cannot_bypass_address_classification() {
        let mapped = SocketAddr::new(IpAddr::V6("::ffff:127.0.0.1".parse().unwrap()), 443);
        assert_eq!(
            validate_addresses(AddressScope::Public, &[mapped]),
            Err(EgressError::AddressDenied)
        );
        let deprecated_site_local = SocketAddr::new("fec0::1".parse().unwrap(), 443);
        assert_eq!(
            validate_addresses(AddressScope::OperatorNetwork, &[deprecated_site_local]),
            Err(EgressError::AddressDenied)
        );
    }

    #[test]
    fn egress_requires_a_nonempty_ascii_connection_or_session_reference() {
        for denied in [
            "",
            "connection:",
            "connect-session:",
            "grant:provider:read",
            "connection:provider:has space",
            "connection:provider:ümlaut",
        ] {
            assert_eq!(
                validate_authority_ref(denied),
                Err(EgressError::InvalidAuthority)
            );
        }
        assert_eq!(validate_authority_ref("connection:slack:workspace"), Ok(()));
        assert_eq!(validate_authority_ref("connect-session:setup-1"), Ok(()));
    }
}

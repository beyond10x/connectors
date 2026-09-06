//! Session-owned local OAuth instructions and one-use PKCE callback claims.
//! No task is detached: dropping the serving future closes its listener and clears private data.

use connector_oauth::device::DeviceAuthorization;
use connector_oauth::{authorize_url, random_token, AuthorizeParams, Pkce};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::Instant;
use zeroize::{Zeroize as _, Zeroizing};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OAuthTransportError {
    #[error("invalid_policy")]
    InvalidPolicy,
    #[error("port_in_use")]
    PortInUse,
    #[error("host_mismatch")]
    HostMismatch,
    #[error("origin_refused")]
    OriginRefused,
    #[error("state_mismatch")]
    StateMismatch,
    #[error("malformed_callback")]
    MalformedCallback,
    #[error("pkce_binding_mismatch")]
    PkceBindingMismatch,
    #[error("code_exchange_refused")]
    CodeExchangeRefused,
    #[error("capability_refused")]
    CapabilityRefused,
    #[error("expired")]
    Expired,
    #[error("request_limit")]
    RequestLimit,
    #[error("io")]
    Io,
}

/// Configuration already admitted by the deployment/catalog owner.
pub struct PkceEndpointConfig<'a> {
    pub redirect_uri: &'a str,
    pub authorization_origin: &'a str,
    pub authorization_path: &'a str,
    pub client_id: &'a str,
    pub scope: &'a str,
    pub deadline: Instant,
}

/// A claimed callback, never printable. The caller performs exchange through its egress owner.
///
/// ```compile_fail
/// fn must_be_debug<T: std::fmt::Debug>() {}
/// must_be_debug::<connect_session_transport::oauth::OAuthCallback>();
/// ```
pub struct OAuthCallback {
    pub code: Zeroizing<String>,
    pub verifier: Zeroizing<String>,
    pub redirect_uri: String,
}

// One active request and at most 64 accepted connections per authorization. These local
// resource limits supplement the design's byte/time bounds and never trigger a new flow.
const MAX_REQUESTS: usize = 64;
const MAX_HEADERS: usize = 8 * 1024;
const MAX_QUERY: usize = 4 * 1024;
const MAX_CODE: usize = 2 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_SESSION: Duration = Duration::from_secs(600);

struct PendingPkce {
    state: Zeroizing<String>,
    pkce: Pkce,
    challenge: String,
    redirect_uri: String,
}

/// Read-only observation of one admitted receiver's lifetime. This grants no authority and
/// carries no instruction material. A live observation may retire immediately after it is read.
#[derive(Clone)]
pub struct OAuthEndpointLiveness {
    live: Arc<AtomicBool>,
    deadline: Instant,
}

impl OAuthEndpointLiveness {
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.live.load(Ordering::Acquire) && Instant::now() < self.deadline
    }
}

/// Bound local instruction endpoint. Call `receive` immediately under the session owner;
/// cancellation means dropping that future. It owns no detached tasks or exchange transport.
/// Device mode serves instructions until its deadline or caller cancellation, with no callback.
///
/// ```compile_fail
/// fn must_be_debug<T: std::fmt::Debug>() {}
/// must_be_debug::<connect_session_transport::oauth::BoundOAuthEndpoint>();
/// ```
pub struct BoundOAuthEndpoint {
    liveness: OAuthEndpointLiveness,
    listener: Option<TcpListener>,
    authority: String,
    provider_origin: String,
    callback_path: Option<String>,
    capability: Zeroizing<String>,
    instructions: Zeroizing<String>,
    pending: Option<PendingPkce>,
    deadline: Instant,
}
impl BoundOAuthEndpoint {
    pub fn bind_pkce(config: PkceEndpointConfig<'_>) -> Result<Self, OAuthTransportError> {
        validate_deadline(config.deadline)?;
        let (address, callback_path) = validate_redirect_uri(config.redirect_uri)?;
        let origin = url::Url::parse(config.authorization_origin)
            .map_err(|_| OAuthTransportError::InvalidPolicy)?;
        if origin.scheme() != "https"
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.path() != "/"
            || origin.query().is_some()
            || origin.fragment().is_some()
            || !safe_path(config.authorization_path)
            || config.client_id.is_empty()
            || config.client_id.len() > 4_096
            || config.client_id.chars().any(char::is_control)
            || config.scope.len() > 8_192
            || config.scope.chars().any(char::is_control)
        {
            return Err(OAuthTransportError::InvalidPolicy);
        }
        let listener = bind_listener(address)?;
        let state = Zeroizing::new(random_token(32).map_err(|_| OAuthTransportError::Io)?);
        let capability = Zeroizing::new(random_token(32).map_err(|_| OAuthTransportError::Io)?);
        let pkce = Pkce::generate().map_err(|_| OAuthTransportError::Io)?;
        let authorization = Zeroizing::new(
            authorize_url(
                &origin,
                config.authorization_path,
                &AuthorizeParams {
                    client_id: config.client_id,
                    redirect_uri: config.redirect_uri,
                    scope: config.scope,
                    state: &state,
                    code_challenge: Some(pkce.challenge()),
                    extra: &[],
                },
            )
            .map_err(|_| OAuthTransportError::InvalidPolicy)?,
        );
        let instructions = Zeroizing::new(format!(
            "{{\"kind\":\"browser_authorization\",\"authorization_url\":{}}}",
            json_string(&authorization).as_str()
        ));
        let pending = PendingPkce {
            state,
            challenge: pkce.challenge().to_owned(),
            pkce,
            redirect_uri: config.redirect_uri.to_owned(),
        };
        Ok(Self {
            liveness: OAuthEndpointLiveness {
                live: Arc::new(AtomicBool::new(true)),
                deadline: config.deadline,
            },
            listener: Some(listener),
            authority: address.to_string(),
            provider_origin: origin.origin().ascii_serialization(),
            callback_path: Some(callback_path),
            capability,
            instructions,
            pending: Some(pending),
            deadline: config.deadline,
        })
    }

    /// Copy only validated human instructions. The device code stays with the acquisition owner.
    /// The monotonic deadline is capped by the device's remaining caller-clock lifetime.
    pub fn bind_device(
        authorization: &DeviceAuthorization,
        now_unix_ms: u64,
        deadline: Instant,
    ) -> Result<Self, OAuthTransportError> {
        validate_deadline(deadline)?;
        let remaining = authorization
            .deadline_unix_ms()
            .checked_sub(now_unix_ms)
            .filter(|value| *value > 0)
            .ok_or(OAuthTransportError::Expired)?;
        let deadline = deadline.min(
            Instant::now()
                .checked_add(Duration::from_millis(remaining))
                .ok_or(OAuthTransportError::InvalidPolicy)?,
        );
        let listener = bind_listener(SocketAddr::from(([127, 0, 0, 1], 0)))?;
        let authority = listener
            .local_addr()
            .map_err(|_| OAuthTransportError::Io)?
            .to_string();
        let human = authorization.instructions();
        let instructions = Zeroizing::new(format!("{{\"kind\":\"device_authorization\",\"verification_uri\":{},\"user_code\":{},\"verification_uri_complete\":{}}}", json_string(human.verification_uri).as_str(), json_string(human.user_code).as_str(), human.verification_uri_complete.map(json_string).unwrap_or_else(|| Zeroizing::new("null".into())).as_str()));
        Ok(Self {
            liveness: OAuthEndpointLiveness {
                live: Arc::new(AtomicBool::new(true)),
                deadline,
            },
            listener: Some(listener),
            authority,
            provider_origin: String::new(),
            callback_path: None,
            capability: Zeroizing::new(random_token(32).map_err(|_| OAuthTransportError::Io)?),
            instructions,
            pending: None,
            deadline,
        })
    }

    /// Trusted human handoff only. Its exact shape preserves the raw completion URL validator.
    #[must_use]
    pub fn browser_url(&self) -> Zeroizing<String> {
        Zeroizing::new(format!(
            "http://{}/#token={}",
            self.authority,
            self.capability.as_str()
        ))
    }

    /// Observe only this receiver. Clones cannot extend its deadline or revive retirement.
    #[must_use]
    pub fn liveness(&self) -> OAuthEndpointLiveness {
        self.liveness.clone()
    }

    /// Claim matching live state exactly once and close the listener before returning exchange
    /// material. Dropping this future retires all private state, including during a stalled read.
    pub async fn receive(mut self) -> Result<OAuthCallback, OAuthTransportError> {
        self.ensure_live()?;
        tokio::time::timeout_at(self.deadline, async {
            for _ in 0..MAX_REQUESTS {
                self.ensure_live()?;
                let (mut stream, peer) = self
                    .listener
                    .as_ref()
                    .expect("live listener")
                    .accept()
                    .await
                    .map_err(|_| OAuthTransportError::Io)?;
                if peer.ip() != std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST) {
                    continue;
                }
                let result = self.handle(&mut stream).await;
                match result {
                    Ok(Some(claim)) => return Ok(claim),
                    Ok(None) => {}
                    Err(error) => {
                        // A matched callback, including denial, has already retired the listener.
                        let terminal = self.listener.is_none();
                        let _ = reply(
                            &mut stream,
                            "403 Forbidden",
                            "text/plain; charset=utf-8",
                            error.to_string().as_bytes(),
                        )
                        .await;
                        if terminal {
                            return Err(error);
                        }
                    }
                }
            }
            Err(OAuthTransportError::RequestLimit)
        })
        .await
        .map_err(|_| OAuthTransportError::Expired)?
    }

    async fn handle(
        &mut self,
        stream: &mut TcpStream,
    ) -> Result<Option<OAuthCallback>, OAuthTransportError> {
        self.ensure_live()?;
        let bytes = read_request(stream, (Instant::now() + READ_TIMEOUT).min(self.deadline)).await;
        self.ensure_live()?;
        let bytes = bytes?;
        let request = parse_request(&bytes, &self.authority)?;
        if request.target == "/" || request.target == "/instructions" {
            check_origin(request.origin, &format!("http://{}", self.authority))?;
            if request.target == "/" {
                reply(
                    stream,
                    "200 OK",
                    "text/html; charset=utf-8",
                    INSTRUCTION_PAGE.as_bytes(),
                )
                .await?;
            } else {
                if !request.capability.is_some_and(|token| {
                    crate::constant_time_equal(token.as_bytes(), self.capability.as_bytes())
                }) {
                    return Err(OAuthTransportError::CapabilityRefused);
                }
                reply(
                    stream,
                    "200 OK",
                    "application/json",
                    self.instructions.as_bytes(),
                )
                .await?;
            }
            return Ok(None);
        }
        let path = self
            .callback_path
            .as_deref()
            .ok_or(OAuthTransportError::MalformedCallback)?;
        check_origin(request.origin, &self.provider_origin)?;
        let pending = self
            .pending
            .as_ref()
            .ok_or(OAuthTransportError::StateMismatch)?;
        let code = parse_callback(request.target, path, &pending.state);
        if code
            .as_ref()
            .err()
            .is_some_and(|error| *error != OAuthTransportError::CodeExchangeRefused)
        {
            return code.map(|_| None);
        }
        self.ensure_live()?;
        // No await between the matching-state check, one-use take and listener retirement.
        let pending = self.pending.take().expect("checked pending state");
        self.retire();
        let code = code?;
        if pending.challenge != pending.pkce.challenge() {
            return Err(OAuthTransportError::PkceBindingMismatch);
        }
        let claim = OAuthCallback {
            code,
            verifier: pending.pkce.into_verifier(),
            redirect_uri: pending.redirect_uri,
        };
        reply(
            stream,
            "200 OK",
            "text/plain; charset=utf-8",
            b"Authorization received. You may close this tab.\n",
        )
        .await?;
        Ok(Some(claim))
    }

    fn ensure_live(&mut self) -> Result<(), OAuthTransportError> {
        if Instant::now() >= self.deadline {
            self.retire();
            return Err(OAuthTransportError::Expired);
        }
        Ok(())
    }

    fn retire(&mut self) {
        self.liveness.live.store(false, Ordering::Release);
        self.listener.take();
        self.pending.take();
        self.instructions.zeroize();
        self.capability.zeroize();
    }
}

impl Drop for BoundOAuthEndpoint {
    fn drop(&mut self) {
        self.retire();
    }
}

fn json_string(value: &str) -> Zeroizing<String> {
    Zeroizing::new(serde_json::to_string(value).expect("string serialization cannot fail"))
}

fn validate_deadline(deadline: Instant) -> Result<(), OAuthTransportError> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .filter(|value| !value.is_zero())
        .ok_or(OAuthTransportError::Expired)?;
    if remaining > MAX_SESSION {
        return Err(OAuthTransportError::InvalidPolicy);
    }
    Ok(())
}

fn safe_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.contains("//")
        && path.len() <= 1_024
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte))
        && !path
            .split('/')
            .any(|segment| segment == "." || segment == "..")
}

fn validate_redirect_uri(value: &str) -> Result<(SocketAddr, String), OAuthTransportError> {
    let uri = url::Url::parse(value).map_err(|_| OAuthTransportError::InvalidPolicy)?;
    let port = uri
        .port()
        .filter(|port| *port != 0)
        .ok_or(OAuthTransportError::InvalidPolicy)?;
    if uri.scheme() != "http"
        || uri.host_str() != Some("127.0.0.1")
        || !uri.username().is_empty()
        || uri.password().is_some()
        || uri.query().is_some()
        || uri.fragment().is_some()
        || !safe_path(uri.path())
        || matches!(uri.path(), "/" | "/instructions")
        || uri.as_str() != value
    {
        return Err(OAuthTransportError::InvalidPolicy);
    }
    Ok((
        SocketAddr::from(([127, 0, 0, 1], port)),
        uri.path().to_owned(),
    ))
}

fn bind_listener(address: SocketAddr) -> Result<TcpListener, OAuthTransportError> {
    let listener = std::net::TcpListener::bind(address).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AddrInUse {
            OAuthTransportError::PortInUse
        } else {
            OAuthTransportError::Io
        }
    })?;
    listener
        .set_nonblocking(true)
        .map_err(|_| OAuthTransportError::Io)?;
    TcpListener::from_std(listener).map_err(|_| OAuthTransportError::Io)
}

struct Request<'a> {
    target: &'a str,
    origin: Option<&'a str>,
    capability: Option<&'a str>,
}
fn parse_request<'a>(bytes: &'a [u8], authority: &str) -> Result<Request<'a>, OAuthTransportError> {
    let malformed = OAuthTransportError::MalformedCallback;
    if bytes.len() > MAX_HEADERS || !bytes.ends_with(b"\r\n\r\n") {
        return Err(malformed);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| malformed)?;
    let mut lines = text[..text.len() - 4].split("\r\n");
    let mut first = lines.next().ok_or(malformed)?.split(' ');
    let method = first.next().ok_or(malformed)?;
    let target = first.next().ok_or(malformed)?;
    if method != "GET"
        || first.next() != Some("HTTP/1.1")
        || first.next().is_some()
        || !target.starts_with('/')
        || target.starts_with("//")
        || target.contains('#')
        || target.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(malformed);
    }
    let mut host = None;
    let mut origin = None;
    let mut capability = None;
    let mut content_length = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(malformed)?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
        {
            return Err(malformed);
        }
        let value = value.trim_matches([' ', '\t']);
        if value.bytes().any(|byte| byte.is_ascii_control()) {
            return Err(malformed);
        }
        let slot = if name.eq_ignore_ascii_case("host") {
            &mut host
        } else if name.eq_ignore_ascii_case("origin") {
            &mut origin
        } else if name.eq_ignore_ascii_case("x-connect-session") {
            &mut capability
        } else if name.eq_ignore_ascii_case("content-length") {
            &mut content_length
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            return Err(malformed);
        } else {
            continue;
        };
        if slot.replace(value).is_some() {
            return Err(malformed);
        }
    }
    if host != Some(authority) {
        return Err(OAuthTransportError::HostMismatch);
    }
    if content_length.is_some_and(|value| value != "0") {
        return Err(malformed);
    }
    Ok(Request {
        target,
        origin,
        capability,
    })
}

fn check_origin(presented: Option<&str>, expected: &str) -> Result<(), OAuthTransportError> {
    if presented.is_some_and(|origin| origin != expected) {
        return Err(OAuthTransportError::OriginRefused);
    }
    Ok(())
}

fn decode_component(value: &str) -> Result<Zeroizing<String>, OAuthTransportError> {
    let mut decoded = Zeroizing::new(Vec::with_capacity(value.len()));
    let mut bytes = value.bytes();
    while let Some(byte) = bytes.next() {
        decoded.push(match byte {
            b'%' => {
                let high = bytes
                    .next()
                    .and_then(|b| char::from(b).to_digit(16))
                    .ok_or(OAuthTransportError::MalformedCallback)?;
                let low = bytes
                    .next()
                    .and_then(|b| char::from(b).to_digit(16))
                    .ok_or(OAuthTransportError::MalformedCallback)?;
                (high * 16 + low) as u8
            }
            b'+' => b' ',
            byte => byte,
        });
    }
    let value =
        std::str::from_utf8(&decoded).map_err(|_| OAuthTransportError::MalformedCallback)?;
    if value.chars().any(char::is_control) {
        return Err(OAuthTransportError::MalformedCallback);
    }
    Ok(Zeroizing::new(value.to_owned()))
}

fn parse_callback(
    target: &str,
    path: &str,
    expected_state: &str,
) -> Result<Zeroizing<String>, OAuthTransportError> {
    let malformed = OAuthTransportError::MalformedCallback;
    let (requested_path, query) = target.split_once('?').ok_or(malformed)?;
    if requested_path != path || query.len() > MAX_QUERY || query.contains('#') {
        return Err(malformed);
    }
    let mut parameters: Vec<(Zeroizing<String>, Zeroizing<String>)> = Vec::new();
    for field in query.split('&') {
        let (key, value) = field.split_once('=').ok_or(malformed)?;
        let key = decode_component(key)?;
        let value = decode_component(value)?;
        if key.is_empty() || parameters.iter().any(|(existing, _)| **existing == *key) {
            return Err(malformed);
        }
        parameters.push((key, value));
    }
    let get = |name| {
        parameters
            .iter()
            .find(|(key, _)| key.as_str() == name)
            .map(|(_, value)| value.as_str())
    };
    let state = get("state")
        .filter(|value| !value.is_empty())
        .ok_or(malformed)?;
    let code = get("code");
    let error = get("error");
    if state.len() > 128
        || code.is_some() == error.is_some()
        || code.is_some_and(|value| value.is_empty() || value.len() > MAX_CODE)
        || error.is_some_and(str::is_empty)
    {
        return Err(malformed);
    }
    if !crate::constant_time_equal(state.as_bytes(), expected_state.as_bytes()) {
        return Err(OAuthTransportError::StateMismatch);
    }
    if error.is_some() {
        return Err(OAuthTransportError::CodeExchangeRefused);
    }
    Ok(Zeroizing::new(
        code.expect("exactly one code or error").to_owned(),
    ))
}

async fn read_request(
    stream: &mut (impl tokio::io::AsyncRead + Unpin),
    deadline: Instant,
) -> Result<Zeroizing<Vec<u8>>, OAuthTransportError> {
    tokio::time::timeout_at(deadline, async {
        let mut bytes = Zeroizing::new(Vec::with_capacity(1_024));
        let mut chunk = Zeroizing::new([0_u8; 1_024]);
        loop {
            let available = (MAX_HEADERS - bytes.len()).min(chunk.len());
            if available == 0 {
                return Err(OAuthTransportError::MalformedCallback);
            }
            let count = stream
                .read(&mut chunk[..available])
                .await
                .map_err(|_| OAuthTransportError::Io)?;
            if count == 0 {
                return Err(OAuthTransportError::MalformedCallback);
            }
            bytes.extend_from_slice(&chunk[..count]);
            if let Some(end) = bytes.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                if end + 4 != bytes.len() {
                    return Err(OAuthTransportError::MalformedCallback);
                }
                return Ok(bytes);
            }
        }
    })
    .await
    .map_err(|_| OAuthTransportError::MalformedCallback)?
}

async fn reply(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> Result<(), OAuthTransportError> {
    let headers = format!("HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; script-src 'unsafe-inline'; connect-src 'self'; form-action 'none'; base-uri 'none'; frame-ancestors 'none'\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\n\r\n", body.len());
    stream
        .write_all(headers.as_bytes())
        .await
        .map_err(|_| OAuthTransportError::Io)?;
    stream
        .write_all(body)
        .await
        .map_err(|_| OAuthTransportError::Io)?;
    stream.shutdown().await.map_err(|_| OAuthTransportError::Io)
}

const INSTRUCTION_PAGE: &str = r#"<!doctype html><meta charset="utf-8"><title>Connect provider</title><h1>Connect provider</h1><p id="status">Loading instructions.</p><p id="code"></p><a id="continue" rel="noreferrer" hidden>Continue with provider</a><script>const capability=new URLSearchParams(location.hash.slice(1)).get('token');history.replaceState(null,'',location.pathname+'#ready');(async()=>{try{const response=await fetch('/instructions',{headers:{'X-Connect-Session':capability},credentials:'omit',referrerPolicy:'no-referrer'});if(!response.ok)throw new Error();const instruction=await response.json();const link=document.querySelector('#continue');link.href=instruction.authorization_url||instruction.verification_uri_complete||instruction.verification_uri;link.hidden=false;document.querySelector('#code').textContent=instruction.user_code||'';document.querySelector('#status').textContent=instruction.user_code?'Enter this code at the provider.':'Continue to authorize this connection.'}catch{document.querySelector('#status').textContent='Instructions are unavailable.'}})();</script>"#;

#[cfg(test)]
#[path = "oauth_tests.rs"]
mod tests;

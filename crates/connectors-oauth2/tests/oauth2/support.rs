//! A fake authorization server on `127.0.0.1` behind TLS with a per-test self-signed root, and
//! the HTTPS transport the tests give the crate: a rustls client that trusts only that root.
//! Every token and secret here is a fictional fixture value.
use connectors_oauth2::{
    ClientKind, GrantType, Parameter, RedirectKind, Request, RequestEncoding, Response, Rotation,
    ServerDeclaration, Transport, TransportError,
};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName};
use serde_json::Value;
use std::{
    collections::{BTreeMap, VecDeque},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use zeroize::Zeroizing;

pub const CLIENT_ID: &str = "fixture-client-id";
pub const CLIENT_SECRET: &str = "fixture-client-secret-value";
pub const ACCESS_TOKEN: &str = "fixture-access-token-one";
pub const REFRESH_TOKEN: &str = "fixture-refresh-token-one";
pub const ROTATED_REFRESH_TOKEN: &str = "fixture-refresh-token-rotated";

/// One scripted answer of the fake server.
#[derive(Clone)]
pub struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Reply {
    pub fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: serde_json::to_vec(&body).unwrap(),
        }
    }

    pub fn text(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "text/plain".into())],
            body: body.as_bytes().to_vec(),
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// A successful token response.
    pub fn token(access: &str, refresh: Option<&str>, scope: Option<&str>) -> Self {
        let mut body = serde_json::json!({
            "access_token": access,
            "token_type": "Bearer",
            "expires_in": 3600,
        });
        if let Some(refresh) = refresh {
            body["refresh_token"] = refresh.into();
        }
        if let Some(scope) = scope {
            body["scope"] = scope.into();
        }
        Self::json(200, body)
    }

    /// An RFC 6749 §5.2 error response.
    pub fn error(status: u16, code: &str) -> Self {
        Self::json(status, serde_json::json!({"error": code}))
    }
}

/// One request the fake server read.
#[derive(Clone, Debug)]
pub struct Observed {
    pub method: String,
    pub path: String,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

impl Observed {
    /// The request's fields, from a form or a flat JSON object.
    pub fn fields(&self) -> BTreeMap<String, String> {
        match self.content_type.as_deref() {
            Some("application/json") => {
                let value: Value = serde_json::from_slice(&self.body).unwrap();
                value
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(name, value)| (name.clone(), value.as_str().unwrap().to_owned()))
                    .collect()
            }
            Some("application/x-www-form-urlencoded") => url::form_urlencoded::parse(&self.body)
                .into_owned()
                .collect(),
            other => panic!("unexpected request content type {other:?}"),
        }
    }

    /// The field names in the order the request carried them.
    pub fn field_order(&self) -> Vec<String> {
        match self.content_type.as_deref() {
            Some("application/x-www-form-urlencoded") => url::form_urlencoded::parse(&self.body)
                .map(|(name, _)| name.into_owned())
                .collect(),
            _ => panic!("field order is read from forms only"),
        }
    }
}

struct Shared {
    replies: Mutex<VecDeque<Reply>>,
    observed: Mutex<Vec<Observed>>,
    stop: AtomicBool,
}

/// The fake authorization server. Dropping it stops its thread.
pub struct FakeServer {
    port: u16,
    root: CertificateDer<'static>,
    shared: Arc<Shared>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl FakeServer {
    /// Starts a server that answers its requests with `replies`, in order; a request past the
    /// last reply is answered 500.
    pub fn start(replies: Vec<Reply>) -> Self {
        let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let root = certified.cert.der().clone();
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![root.clone()],
            PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der()).into(),
        )
        .unwrap();
        let tls = Arc::new(tls);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let shared = Arc::new(Shared {
            replies: Mutex::new(replies.into()),
            observed: Mutex::new(Vec::new()),
            stop: AtomicBool::new(false),
        });
        let served = shared.clone();
        let thread = std::thread::spawn(move || {
            for stream in listener.incoming() {
                if served.stop.load(Ordering::SeqCst) {
                    return;
                }
                if let Ok(stream) = stream {
                    serve(stream, &tls, &served);
                }
            }
        });
        Self {
            port,
            root,
            shared,
            thread: Some(thread),
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("https://localhost:{}{path}", self.port)
    }

    pub fn transport(&self) -> TlsTransport {
        TlsTransport::trusting(&self.root)
    }

    pub fn observed(&self) -> Vec<Observed> {
        self.shared.observed.lock().unwrap().clone()
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(stream: TcpStream, tls: &Arc<rustls::ServerConfig>, shared: &Shared) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let Ok(connection) = rustls::ServerConnection::new(tls.clone()) else {
        return;
    };
    let mut stream = rustls::StreamOwned::new(connection, stream);
    let Some((head, body)) = read_message(&mut stream, usize::MAX) else {
        return;
    };
    let head = String::from_utf8_lossy(&head).into_owned();
    let mut words = head.split_whitespace();
    let method = words.next().unwrap_or_default().to_owned();
    let target = words.next().unwrap_or_default().to_owned();
    shared.observed.lock().unwrap().push(Observed {
        method,
        path: target,
        content_type: header(&head, "content-type"),
        body,
    });
    let reply = shared
        .replies
        .lock()
        .unwrap()
        .pop_front()
        .unwrap_or_else(|| Reply::text(500, "no scripted reply"));
    let mut out = format!(
        "HTTP/1.1 {} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n",
        reply.status,
        reply.body.len()
    );
    for (name, value) in &reply.headers {
        out.push_str(&format!("{name}: {value}\r\n"));
    }
    out.push_str("\r\n");
    let mut bytes = out.into_bytes();
    bytes.extend_from_slice(&reply.body);
    let _ = stream.write_all(&bytes);
    let _ = stream.flush();
    stream.conn.send_close_notify();
    let _ = stream.flush();
}

/// A header's value from an HTTP head, by case-insensitive name.
fn header(head: &str, wanted: &str) -> Option<String> {
    head.lines().skip(1).find_map(|line| {
        line.split_once(':')
            .filter(|(name, _)| name.trim().eq_ignore_ascii_case(wanted))
            .map(|(_, value)| value.trim().to_owned())
    })
}

/// Reads one HTTP message: its head, then at most `limit + 1` bytes of the body its
/// `Content-Length` announces.
fn read_message(stream: &mut impl Read, limit: usize) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() > 65_536 || stream.read(&mut byte).ok()? == 0 {
            return None;
        }
        head.push(byte[0]);
    }
    let text = String::from_utf8_lossy(&head).into_owned();
    let length: usize = header(&text, "content-length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0_u8; length.min(limit.saturating_add(1))];
    stream.read_exact(&mut body).ok()?;
    Some((head, body))
}

/// The transport the tests hand the crate: HTTPS through rustls, trusting only the fake
/// server's root, one request per connection.
pub struct TlsTransport {
    config: Arc<rustls::ClientConfig>,
}

impl TlsTransport {
    pub fn trusting(root: &CertificateDer<'static>) -> Self {
        let mut roots = rustls::RootCertStore::empty();
        roots.add(root.clone()).unwrap();
        let config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
        Self {
            config: Arc::new(config),
        }
    }
}

impl Transport for TlsTransport {
    fn post(&self, request: &Request<'_>) -> Result<Response, TransportError> {
        let url = url::Url::parse(request.url).map_err(|_| TransportError)?;
        assert_eq!(url.scheme(), "https", "the crate sent a non-HTTPS request");
        let host = url.host_str().ok_or(TransportError)?.to_owned();
        let port = url.port_or_known_default().ok_or(TransportError)?;
        let tcp = TcpStream::connect(("127.0.0.1", port)).map_err(|_| TransportError)?;
        tcp.set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|_| TransportError)?;
        let name = ServerName::try_from(host.clone()).map_err(|_| TransportError)?;
        let connection =
            rustls::ClientConnection::new(self.config.clone(), name).map_err(|_| TransportError)?;
        let mut stream = rustls::StreamOwned::new(connection, tcp);
        let target = match url.query() {
            Some(query) => format!("{}?{query}", url.path()),
            None => url.path().to_owned(),
        };
        let mut message = Zeroizing::new(
            format!(
                "POST {target} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: {}\r\nAccept: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                request.content_type,
                request.body.len()
            )
            .into_bytes(),
        );
        message.extend_from_slice(request.body);
        stream.write_all(&message).map_err(|_| TransportError)?;
        stream.flush().map_err(|_| TransportError)?;
        let (head, body) = read_message(&mut stream, request.limit).ok_or(TransportError)?;
        let head = String::from_utf8_lossy(&head).into_owned();
        let status = head
            .split_whitespace()
            .nth(1)
            .and_then(|status| status.parse().ok())
            .ok_or(TransportError)?;
        Ok(Response {
            status,
            retry_after: header(&head, "retry-after"),
            body: Zeroizing::new(body),
        })
    }
}

/// A transport for flows that never reach the token endpoint.
pub struct NoTransport;

impl Transport for NoTransport {
    fn post(&self, _request: &Request<'_>) -> Result<Response, TransportError> {
        panic!("this test sends no request");
    }
}

/// A complete declaration for the fake server: a public client, form encoding, any-port
/// loopback redirect, both grant types.
pub fn declaration(server: Option<&FakeServer>) -> ServerDeclaration {
    let url = |path: &str| match server {
        Some(server) => server.url(path),
        None => format!("https://localhost:1{path}"),
    };
    ServerDeclaration {
        authorize_url: Some(url("/authorize")),
        token_url: url("/token"),
        device_authorization_url: Some(url("/device")),
        client_id: Some(CLIENT_ID.to_owned()),
        client_kind: ClientKind::Public,
        request_encoding: RequestEncoding::Form,
        refresh_encoding: RequestEncoding::Form,
        redirect_kind: RedirectKind::LoopbackAnyPort,
        redirect_uri: None,
        authorize_parameters: Vec::new(),
        requested_scopes: vec!["fixture.read".to_owned(), "fixture.write".to_owned()],
        required_scopes: vec!["fixture.read".to_owned()],
        rotation: Rotation::Rotating,
        refresh_lead_seconds: 300,
        grant_types: vec![GrantType::AuthorizationCode, GrantType::DeviceCode],
        sources: vec!["https://example.test/oauth2-reference".to_owned()],
        state_in_exchange: false,
    }
}

pub fn parameter(name: &str, value: &str) -> Parameter {
    Parameter {
        name: name.to_owned(),
        value: value.to_owned(),
    }
}

/// The query pairs of a URL, in order.
pub fn query(url: &str) -> Vec<(String, String)> {
    url::Url::parse(url)
        .unwrap()
        .query_pairs()
        .into_owned()
        .collect()
}

/// One query parameter of a URL.
pub fn query_value(url: &str, name: &str) -> String {
    query(url)
        .into_iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
        .unwrap_or_else(|| panic!("{name} missing from the authorize URL"))
}

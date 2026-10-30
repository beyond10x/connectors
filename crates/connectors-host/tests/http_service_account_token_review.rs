//! Security-review conformance probes for the `service-account-token`
//! admission profile (contracts/service/compatibility.md § 2.1, Admission;
//! story:served-instance-admits-a-service-account-token). Each case drives
//! the shipped router against a loopback TokenReview double and checks the
//! contract's own rules: no request reaches the adapter without an exact
//! positive review, the presented token is never logged, echoed or recorded,
//! the review is rooted only in the configured CA and never follows a
//! redirect, and the configuration readers stay closed.
//!
//! Unix only: its fixtures use Unix mode bits.
#![cfg(unix)]
use async_trait::async_trait;
use connectors_core::{Descriptor, Operation, Result, WIRE_VERSION};
use connectors_host::server::{ServiceConfigV3, ServiceState, router_with_admission};
use connectors_sdk::Adapter;
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{self, pki_types::PrivatePkcs8KeyDer},
};

const INSTANCE: &str = "leaf";
const AUDIENCE: &str = "connectors-leaf";
const PRINCIPAL: &str = "system:serviceaccount:agents:example-agent";
/// Distinctive bytes, so any copy in a log, a response or a record is found.
const CALLER_TOKEN: &str = "caller-token-7f3a9c-review-probe";
const INSTANCE_TOKEN: &str = "instance-token-51be0d-review-probe";

struct Fixture {
    calls: Arc<AtomicUsize>,
}
fn operation(id: &str, profile: &str) -> Operation {
    Operation {
        id: id.into(),
        description: format!("{id} fixture"),
        contract: "operations/v1alpha1".into(),
        profile: profile.into(),
        input_schema: json!({"type":"object"}),
        output_schema: json!({"type":"object"}),
    }
}
#[async_trait]
impl Adapter for Fixture {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            version: WIRE_VERSION.into(),
            instance: INSTANCE.into(),
            adapter: "fixture".into(),
            revision: "rev-1".into(),
            configuration_schema: json!({"type":"object"}),
            operations: vec![operation("read", "read"), operation("write", "mutation")],
        }
    }
    async fn invoke(&self, _operation: &str, input: Value) -> Result<Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(input)
    }
}

/// What the double answers: status, extra header lines, body.
type Reply = (u16, Vec<String>, Vec<u8>);
/// Each review request as received: its Authorization value and its body.
type Seen = Vec<(Option<String>, Vec<u8>)>;

/// A loopback TokenReview endpoint over HTTPS, under its own self-signed
/// certificate; it records each request's Authorization and body.
struct Double {
    api: String,
    ca: PathBuf,
    seen: Arc<Mutex<Seen>>,
    reply: Arc<Mutex<Reply>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Double {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Double {
    fn answer(&self, status: u16, headers: &[&str], body: &[u8]) {
        *self.reply.lock().unwrap() = (
            status,
            headers.iter().map(|h| (*h).to_owned()).collect(),
            body.to_vec(),
        );
    }
    fn json(&self, status: u16, body: &Value) {
        self.answer(status, &[], &serde_json::to_vec(body).unwrap());
    }
    fn seen(&self) -> usize {
        self.seen.lock().unwrap().len()
    }
}

fn certificate() -> rcgen::CertifiedKey<rcgen::KeyPair> {
    rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap()
}

async fn double(directory: &Path) -> Double {
    let certificate = certificate();
    let server = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![certificate.cert.der().clone()],
        PrivatePkcs8KeyDer::from(certificate.signing_key.serialize_der()).into(),
    )
    .unwrap();
    let ca = directory.join("cluster-ca.crt");
    std::fs::write(&ca, certificate.cert.pem()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let reply: Arc<Mutex<Reply>> = Arc::new(Mutex::new((
        201,
        Vec::new(),
        serde_json::to_vec(&positive()).unwrap(),
    )));
    let task = tokio::spawn({
        let seen = seen.clone();
        let reply = reply.clone();
        async move {
            let acceptor = TlsAcceptor::from(Arc::new(server));
            while let Ok((stream, _)) = listener.accept().await {
                let (acceptor, seen, reply) = (acceptor.clone(), seen.clone(), reply.clone());
                tokio::spawn(async move {
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        return;
                    };
                    let mut head = Vec::new();
                    while !head.ends_with(b"\r\n\r\n") && head.len() < 32 * 1024 {
                        match stream.read_u8().await {
                            Ok(byte) => head.push(byte),
                            Err(_) => return,
                        }
                    }
                    let head = String::from_utf8_lossy(&head).into_owned();
                    let mut length = 0;
                    let mut authorization = None;
                    for line in head.split("\r\n").skip(1) {
                        if let Some((name, value)) = line.split_once(':') {
                            match name.trim().to_ascii_lowercase().as_str() {
                                "content-length" => length = value.trim().parse().unwrap_or(0),
                                "authorization" => authorization = Some(value.trim().to_owned()),
                                _ => {}
                            }
                        }
                    }
                    let mut body = vec![0; length];
                    if stream.read_exact(&mut body).await.is_err() {
                        return;
                    }
                    seen.lock().unwrap().push((authorization, body));
                    let (status, headers, bytes) = reply.lock().unwrap().clone();
                    let mut answer = format!("HTTP/1.1 {status} Fixture\r\n");
                    for header in headers {
                        answer.push_str(&header);
                        answer.push_str("\r\n");
                    }
                    answer.push_str(&format!(
                        "Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    ));
                    let _ = stream.write_all(answer.as_bytes()).await;
                    let _ = stream.write_all(&bytes).await;
                    let _ = stream.shutdown().await;
                });
            }
        }
    });
    Double {
        api: format!("https://localhost:{port}"),
        ca,
        seen,
        reply,
        task,
    }
}

fn review_with(status: Value) -> Value {
    json!({
        "apiVersion": "authentication.k8s.io/v1",
        "kind": "TokenReview",
        "spec": {"audiences": [AUDIENCE]},
        "status": status,
    })
}

fn positive() -> Value {
    review_with(json!({
        "authenticated": true,
        "audiences": [AUDIENCE],
        "user": {"username": PRINCIPAL},
    }))
}

fn private_file(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
}

fn account_document(api: &str, ca: &Path, credential: &Path) -> Value {
    json!({
        "instance": INSTANCE,
        "listen": "127.0.0.1:0",
        "admission": "service-account-token",
        "audience": AUDIENCE,
        "principal": PRINCIPAL,
        "token_review": {"api": api, "ca_file": ca, "credential_file": credential},
    })
}

/// Read `document` through the hosts' reader.
fn read(root: &Path, document: &Value) -> Result<ServiceConfigV3> {
    let file = root.join("service.yaml");
    std::fs::write(&file, serde_yaml_ng::to_string(document).unwrap()).unwrap();
    connectors_host::read_config(&file)
}

struct Host {
    base: String,
    state: PathBuf,
    calls: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Host {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn host_with(root: &Path, api: &str, ca: &Path) -> Host {
    let credential = root.join("instance-token");
    private_file(&credential, INSTANCE_TOKEN.as_bytes());
    let config = read(root, &account_document(api, ca, &credential)).unwrap();
    let admission = config.admission().unwrap();
    let state = root.join("state");
    std::fs::create_dir(&state).unwrap();
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let adapter = Arc::new(Fixture {
        calls: calls.clone(),
    });
    let service = Arc::new(ServiceState::open(&state, INSTANCE, "fixture", "rev-1").unwrap());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = router_with_admission(adapter, admission, Some(service));
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Host {
        base: format!("http://{address}"),
        state,
        calls,
        task,
    }
}

async fn send(
    host: &Host,
    method: reqwest::Method,
    route: &str,
    authorization: Option<&[u8]>,
    body: Option<&Value>,
) -> (u16, String) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut request = reqwest::Client::new().request(method, format!("{}{route}", host.base));
    if let Some(value) = authorization {
        request = request.header(
            "authorization",
            reqwest::header::HeaderValue::from_bytes(value).unwrap(),
        );
    }
    if let Some(body) = body {
        request = request.body(serde_json::to_vec(body).unwrap());
    }
    let response = request.send().await.unwrap();
    let status = response.status().as_u16();
    let headers = format!("{:?}", response.headers());
    let bytes = response.bytes().await.unwrap();
    (
        status,
        format!("{headers}\n{}", String::from_utf8_lossy(&bytes)),
    )
}

fn invocation(operation: &str, request_id: &str) -> Value {
    json!({"version":"v1alpha2","request_id":request_id,"operation":operation,"revision":"rev-1","input":{"value":1}})
}

fn legacy(operation: &str) -> Value {
    json!({"version": WIRE_VERSION, "request_id":"legacy","operation":operation,"revision":"rev-1","input":{}})
}

/// Every route a caller can name that is not `/healthz`, with its body.
fn routes() -> Vec<(reqwest::Method, &'static str, Option<Value>)> {
    vec![
        (
            reqwest::Method::POST,
            "/v1alpha2/invoke",
            Some(invocation("read", "r")),
        ),
        (
            reqwest::Method::POST,
            "/v1alpha2/invoke",
            Some(invocation("write", "w")),
        ),
        (reqwest::Method::GET, "/v1/describe", None),
        (reqwest::Method::POST, "/v1/invoke", Some(legacy("read"))),
        (reqwest::Method::POST, "/v1/invoke", Some(legacy("write"))),
        (reqwest::Method::GET, "/v1alpha2/describe", None),
    ]
}

/// A log writer the test reads back.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Captured {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

/// Every byte under `directory`, files only.
fn stored(directory: &Path) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut pending = vec![directory.to_owned()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(content) = std::fs::read(&path) {
                bytes.extend_from_slice(&content);
            }
        }
    }
    bytes
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

/// Invariant 1: a malformed, wrong-scheme or empty bearer is refused on every
/// route with no review sent and zero adapter calls; `/healthz` is the one
/// open route and says nothing about the instance.
#[tokio::test]
async fn no_route_admits_a_malformed_bearer_and_healthz_discloses_nothing() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let host = host_with(root.path(), &double.api, &double.ca).await;

    let malformed: Vec<Option<Vec<u8>>> = vec![
        None,
        Some(b"Bearer".to_vec()),
        Some(b"Bearer ".to_vec()),
        Some(format!("bearer {CALLER_TOKEN}").into_bytes()),
        Some(format!("BEARER {CALLER_TOKEN}").into_bytes()),
        Some(format!("Bearer\t{CALLER_TOKEN}").into_bytes()),
        Some(format!("Token {CALLER_TOKEN}").into_bytes()),
        Some(format!("Basic {CALLER_TOKEN}").into_bytes()),
        Some(CALLER_TOKEN.as_bytes().to_vec()),
        Some(b"Bearer \xff\xfe-not-visible-ascii".to_vec()),
    ];
    for authorization in &malformed {
        for (method, route, body) in routes() {
            let (status, answer) = send(
                &host,
                method.clone(),
                route,
                authorization.as_deref(),
                body.as_ref(),
            )
            .await;
            assert!(
                status == 401 || status == 404 || (route == "/v1/invoke" && status == 501),
                "{method} {route} with {authorization:?} answered {status}: {answer}"
            );
            assert!(!answer.contains(CALLER_TOKEN), "{route} echoed: {answer}");
            if route == "/v1alpha2/describe" {
                assert!(
                    !answer.contains("read") && !answer.contains(INSTANCE),
                    "{answer}"
                );
            }
        }
    }
    assert_eq!(double.seen(), 0, "no review is sent for a malformed bearer");
    assert_eq!(host.calls.load(Ordering::SeqCst), 0);

    // `/healthz` is open: it answers 204 with no body and names nothing.
    let (status, answer) = send(&host, reqwest::Method::GET, "/healthz", None, None).await;
    assert_eq!(status, 204);
    for disclosed in [INSTANCE, AUDIENCE, PRINCIPAL, "read", "service-account"] {
        assert!(!answer.contains(disclosed), "{disclosed} in {answer}");
    }
    assert_eq!(double.seen(), 0);
}

/// Invariant 1: every review answer that is not the exact positive one is a
/// refusal on every route, with zero adapter calls; the token and the review's
/// own wording never reach the response.
#[tokio::test]
async fn no_review_answer_but_the_exact_positive_one_admits_on_any_route() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let host = host_with(root.path(), &double.api, &double.ca).await;
    let token = format!("Bearer {CALLER_TOKEN}");

    let positive_bytes = serde_json::to_vec(&positive()).unwrap();
    let mut wrong_version = positive();
    wrong_version["apiVersion"] = json!("authentication.k8s.io/v1beta1");
    let answers: Vec<(&str, u16, Vec<String>, Vec<u8>)> = vec![
        (
            "username in another case",
            201,
            vec![],
            serde_json::to_vec(&review_with(json!({"authenticated": true, "audiences": [AUDIENCE], "user": {"username": PRINCIPAL.to_uppercase()}}))).unwrap(),
        ),
        (
            "username with a trailing space",
            201,
            vec![],
            serde_json::to_vec(&review_with(json!({"authenticated": true, "audiences": [AUDIENCE], "user": {"username": format!("{PRINCIPAL} ")}}))).unwrap(),
        ),
        (
            "username with a trailing newline",
            201,
            vec![],
            serde_json::to_vec(&review_with(json!({"authenticated": true, "audiences": [AUDIENCE], "user": {"username": format!("{PRINCIPAL}\n")}}))).unwrap(),
        ),
        (
            "audience in another case",
            201,
            vec![],
            serde_json::to_vec(&review_with(json!({"authenticated": true, "audiences": [AUDIENCE.to_uppercase()], "user": {"username": PRINCIPAL}}))).unwrap(),
        ),
        (
            "authenticated as a string",
            201,
            vec![],
            serde_json::to_vec(&review_with(json!({"authenticated": "true", "audiences": [AUDIENCE], "user": {"username": PRINCIPAL}}))).unwrap(),
        ),
        (
            "audiences as a string",
            201,
            vec![],
            serde_json::to_vec(&review_with(json!({"authenticated": true, "audiences": AUDIENCE, "user": {"username": PRINCIPAL}}))).unwrap(),
        ),
        (
            "duplicate username, the configured one last",
            201,
            vec![],
            format!(r#"{{"apiVersion":"authentication.k8s.io/v1","kind":"TokenReview","status":{{"authenticated":true,"audiences":["{AUDIENCE}"],"user":{{"username":"system:serviceaccount:agents:other","username":"{PRINCIPAL}"}}}}}}"#).into_bytes(),
        ),
        (
            "duplicate authenticated, true last",
            201,
            vec![],
            format!(r#"{{"apiVersion":"authentication.k8s.io/v1","kind":"TokenReview","status":{{"authenticated":false,"audiences":["{AUDIENCE}"],"user":{{"username":"{PRINCIPAL}"}},"authenticated":true}}}}"#).into_bytes(),
        ),
        (
            "no status",
            201,
            vec![],
            serde_json::to_vec(&json!({"apiVersion": "authentication.k8s.io/v1", "kind": "TokenReview"})).unwrap(),
        ),
        (
            "status null",
            201,
            vec![],
            serde_json::to_vec(&review_with(Value::Null)).unwrap(),
        ),
        ("an array", 201, vec![], b"[]".to_vec()),
        ("a string", 201, vec![], b"\"authenticated\"".to_vec()),
        ("null", 201, vec![], b"null".to_vec()),
        ("a number", 200, vec![], b"42".to_vec()),
        ("an empty 204", 204, vec![], Vec::new()),
        ("another apiVersion", 201, vec![], serde_json::to_vec(&wrong_version).unwrap()),
        (
            "a redirect carrying a positive body",
            302,
            vec![format!("Location: {}/apis/authentication.k8s.io/v1/tokenreviews", double.api)],
            positive_bytes.clone(),
        ),
        ("a 307 carrying a positive body", 307, vec![], positive_bytes.clone()),
        ("a 4xx carrying a positive body", 400, vec![], positive_bytes.clone()),
        ("a 5xx carrying a positive body", 503, vec![], positive_bytes),
    ];
    let mut sent = 0;
    for (what, status, headers, body) in &answers {
        let headers: Vec<&str> = headers.iter().map(String::as_str).collect();
        double.answer(*status, &headers, body);
        for (method, route, request) in routes() {
            if route == "/v1alpha2/describe" {
                continue;
            }
            let (answered, text) = send(
                &host,
                method.clone(),
                route,
                Some(token.as_bytes()),
                request.as_ref(),
            )
            .await;
            assert!(
                matches!(answered, 401 | 503) || (route == "/v1/invoke" && answered == 501),
                "{what}: {method} {route} answered {answered}: {text}"
            );
            assert!(!text.contains(CALLER_TOKEN), "{what}: {text}");
            assert!(!text.contains(INSTANCE_TOKEN), "{what}: {text}");
            if route != "/v1/invoke" {
                sent += 1;
            }
        }
        assert_eq!(
            host.calls.load(Ordering::SeqCst),
            0,
            "{what} reached the adapter"
        );
    }
    // One review per reviewed request: a redirect is never followed.
    assert_eq!(double.seen(), sent);
}

/// Invariant 2: the presented token and the instance's own token appear in no
/// log line, response or stored record, on a refused, a failed and an admitted
/// request alike; the review body is exactly the contract's.
#[tokio::test]
async fn neither_token_reaches_a_log_a_response_or_a_record() {
    let captured = Captured::default();
    let writer = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let host = host_with(root.path(), &double.api, &double.ca).await;
    let token = format!("Bearer {CALLER_TOKEN}");
    let mut texts = Vec::new();

    let refusals = [
        (201, serde_json::to_vec(&review_with(json!({"authenticated": false, "error": format!("token {CALLER_TOKEN} is invalid")}))).unwrap()),
        (201, serde_json::to_vec(&review_with(json!({"authenticated": true, "audiences": ["other"], "user": {"username": PRINCIPAL}}))).unwrap()),
        (401, format!("{{\"message\":\"{INSTANCE_TOKEN} refused\"}}").into_bytes()),
        (500, format!("echo {CALLER_TOKEN}").into_bytes()),
        (201, format!("not json {CALLER_TOKEN}").into_bytes()),
    ];
    for (status, body) in &refusals {
        double.answer(*status, &[], body);
        for (method, route, request) in routes() {
            let (_, text) = send(
                &host,
                method,
                route,
                Some(token.as_bytes()),
                request.as_ref(),
            )
            .await;
            texts.push(text);
        }
    }
    double.json(201, &positive());
    let (status, text) = send(
        &host,
        reqwest::Method::POST,
        "/v1alpha2/invoke",
        Some(token.as_bytes()),
        Some(&invocation("read", "admitted")),
    )
    .await;
    assert_eq!(status, 200, "{text}");
    texts.push(text);
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);

    for text in &texts {
        assert!(
            !text.contains(CALLER_TOKEN),
            "a response echoed the caller token: {text}"
        );
        assert!(
            !text.contains(INSTANCE_TOKEN),
            "a response carried the instance token: {text}"
        );
    }
    let logs = captured.text();
    assert!(
        logs.contains("token review refused") && logs.contains("token review unavailable"),
        "the capture saw the admission's own events:\n{logs}"
    );
    assert!(
        !logs.contains(CALLER_TOKEN),
        "a log line carried the caller token:\n{logs}"
    );
    assert!(
        !logs.contains(INSTANCE_TOKEN),
        "a log line carried the instance token:\n{logs}"
    );
    let records = stored(&host.state);
    assert!(
        !contains(&records, CALLER_TOKEN),
        "a record carried the caller token"
    );
    assert!(
        !contains(&records, INSTANCE_TOKEN),
        "a record carried the instance token"
    );

    // Each review body is exactly the contract's document, and each carries the
    // instance's own token and nothing else in Authorization.
    let seen = double.seen.lock().unwrap().clone();
    assert!(!seen.is_empty());
    for (authorization, body) in seen {
        assert_eq!(
            authorization.as_deref(),
            Some(format!("Bearer {INSTANCE_TOKEN}").as_str())
        );
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            body,
            json!({"apiVersion":"authentication.k8s.io/v1","kind":"TokenReview","spec":{"token":CALLER_TOKEN,"audiences":[AUDIENCE]}})
        );
    }
}

/// Invariants 2 and 3: a cluster API whose certificate the configured CA does
/// not root is `unavailable`, and no request (so neither token) reaches it; a
/// redirect's target never receives the instance's token.
#[tokio::test]
async fn the_review_is_rooted_only_in_the_configured_ca_and_follows_no_redirect() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    // Another CA than the one that signed the double's certificate.
    let other_ca = root.path().join("other-ca.crt");
    std::fs::write(&other_ca, certificate().cert.pem()).unwrap();
    let host = host_with(root.path(), &double.api, &other_ca).await;
    let token = format!("Bearer {CALLER_TOKEN}");
    let (status, text) = send(
        &host,
        reqwest::Method::POST,
        "/v1alpha2/invoke",
        Some(token.as_bytes()),
        Some(&invocation("read", "r")),
    )
    .await;
    assert_eq!(status, 503, "{text}");
    assert_eq!(
        double.seen(),
        0,
        "the unverified endpoint received a request"
    );
    assert_eq!(host.calls.load(Ordering::SeqCst), 0);

    // A redirect to another listener: that listener is never connected to.
    let other_root = tempfile::tempdir().unwrap();
    let redirecting = double_in(other_root.path()).await;
    let target = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_port = target.local_addr().unwrap().port();
    let accepted = Arc::new(AtomicUsize::new(0));
    let counter = accepted.clone();
    let watcher = tokio::spawn(async move {
        while target.accept().await.is_ok() {
            counter.fetch_add(1, Ordering::SeqCst);
        }
    });
    redirecting.answer(
        307,
        &[&format!(
            "Location: https://localhost:{target_port}/apis/authentication.k8s.io/v1/tokenreviews"
        )],
        &serde_json::to_vec(&positive()).unwrap(),
    );
    let host = host_with(other_root.path(), &redirecting.api, &redirecting.ca).await;
    let (status, text) = send(
        &host,
        reqwest::Method::POST,
        "/v1alpha2/invoke",
        Some(token.as_bytes()),
        Some(&invocation("read", "r")),
    )
    .await;
    assert_eq!(status, 503, "{text}");
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(
        accepted.load(Ordering::SeqCst),
        0,
        "the redirect was followed"
    );
    assert_eq!(redirecting.seen(), 1);
    assert_eq!(host.calls.load(Ordering::SeqCst), 0);
    watcher.abort();
}

async fn double_in(directory: &Path) -> Double {
    double(directory).await
}

/// Invariant 3: start refuses an instance credential that is a symbolic link
/// or readable by others; a credential replaced by a link after start is
/// `unavailable` with no review sent.
#[tokio::test]
async fn the_instance_credential_is_never_read_through_a_link_or_from_a_shared_file() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let real = root.path().join("real-token");
    private_file(&real, INSTANCE_TOKEN.as_bytes());
    let link = root.path().join("linked-token");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let shared = root.path().join("shared-token");
    private_file(&shared, INSTANCE_TOKEN.as_bytes());
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o644)).unwrap();
    for credential in [&link, &shared] {
        let config = read(
            root.path(),
            &account_document(&double.api, &double.ca, credential),
        )
        .unwrap();
        let adapter = Arc::new(Fixture {
            calls: Arc::new(AtomicUsize::new(0)),
        });
        let started = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            connectors_host::server::serve(config, adapter),
        )
        .await;
        assert!(
            matches!(started, Ok(Err(_))),
            "start admitted {credential:?}"
        );
    }

    let host = host_with(root.path(), &double.api, &double.ca).await;
    let credential = root.path().join("instance-token");
    std::fs::remove_file(&credential).unwrap();
    std::os::unix::fs::symlink(&real, &credential).unwrap();
    let token = format!("Bearer {CALLER_TOKEN}");
    let (status, text) = send(
        &host,
        reqwest::Method::POST,
        "/v1alpha2/invoke",
        Some(token.as_bytes()),
        Some(&invocation("read", "r")),
    )
    .await;
    assert_eq!(status, 503, "{text}");
    assert_eq!(double.seen(), 0);
    assert_eq!(host.calls.load(Ordering::SeqCst), 0);
}

/// Invariant 4: the v3 reader is closed. An unknown `admission` value, one in
/// another case, an extra key at the top or under `token_review`, and an
/// `http://` endpoint are refused.
#[test]
fn the_v3_reader_refuses_unknown_admission_values_and_extra_keys() {
    let root = tempfile::tempdir().unwrap();
    let ca = root.path().join("ca.crt");
    std::fs::write(&ca, certificate().cert.pem()).unwrap();
    let credential = root.path().join("token");
    private_file(&credential, INSTANCE_TOKEN.as_bytes());
    let base = account_document("https://kubernetes.default.svc", &ca, &credential);
    assert!(read(root.path(), &base).unwrap().admission().is_ok());

    let mut variants = Vec::new();
    for value in [
        "Service-Account-Token",
        "service_account_token",
        "none",
        "oidc",
        "",
    ] {
        let mut document = base.clone();
        document["admission"] = json!(value);
        variants.push((format!("admission {value:?}"), document));
    }
    let mut extra = base.clone();
    extra["groups"] = json!(["system:serviceaccounts"]);
    variants.push(("extra top-level key".into(), extra));
    let mut nested = base.clone();
    nested["token_review"]["insecure_skip_verify"] = json!(true);
    variants.push(("extra key under token_review".into(), nested));
    let mut proxy = base.clone();
    proxy["token_review"]["proxy"] = json!("http://proxy.example:3128");
    variants.push(("proxy under token_review".into(), proxy));
    let mut list = base.clone();
    list["principal"] = json!([PRINCIPAL, "system:serviceaccount:agents:other"]);
    variants.push(("principal list".into(), list));
    let mut audiences = base.clone();
    audiences["audience"] = json!([AUDIENCE, "other"]);
    variants.push(("audience list".into(), audiences));
    for (what, document) in variants {
        let refused = match read(root.path(), &document) {
            Err(_) => true,
            Ok(config) => config.admission().is_err(),
        };
        assert!(refused, "{what} was admitted");
    }

    let mut plaintext = base.clone();
    plaintext["token_review"]["api"] = json!("http://kubernetes.default.svc");
    assert!(read(root.path(), &plaintext).unwrap().admission().is_err());
}

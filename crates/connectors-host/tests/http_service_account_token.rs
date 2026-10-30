//! The `service-account-token` admission profile of the HTTP host
//! (contracts/service/compatibility.md § 2.1, Admission and Legacy binding;
//! story:served-instance-admits-a-service-account-token), against a loopback
//! TokenReview double that records every review it receives.
//!
//! Unix only: its fixtures use Unix mode bits.
#![cfg(unix)]
use async_trait::async_trait;
use connectors_core::{
    Descriptor, Operation, Result, WIRE_VERSION,
    v1alpha2::{AuditStatus, ErrorCode, InvokeResponse, ResponseStatus},
};
use connectors_host::{
    local::audit::{self, Reference},
    server::{
        Admission, CONCURRENT_REVIEWS, ServiceConfig, ServiceConfigV3, ServiceState,
        router_with_admission, router_with_state,
    },
};
use connectors_sdk::{Adapter, Credential, Secret};
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
const OTHER_PRINCIPAL: &str = "system:serviceaccount:agents:other-agent";
/// The projected token the caller presents.
const CALLER_TOKEN: &str = "caller-projected-token";
/// The instance's own token, which authenticates its review to the cluster API.
const INSTANCE_TOKEN: &str = "instance-own-token";
const REVIEW_PATH: &str = "/apis/authentication.k8s.io/v1/tokenreviews";

/// A read and a write (`profile: mutation`); every call is counted.
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

/// One review the double received.
#[derive(Clone, Debug)]
struct Recorded {
    method: String,
    path: String,
    authorization: Option<String>,
    body: Value,
}

/// The loopback TokenReview double: HTTPS under its own self-signed CA. It
/// answers every review with the current `reply` and records the request.
struct Double {
    api: String,
    ca: PathBuf,
    reviews: Arc<Mutex<Vec<Recorded>>>,
    reply: Arc<Mutex<(u16, Vec<u8>)>>,
    task: tokio::task::JoinHandle<()>,
}
impl Double {
    fn answer(&self, status: u16, body: &Value) {
        *self.reply.lock().unwrap() = (status, serde_json::to_vec(body).unwrap());
    }
    fn answer_bytes(&self, status: u16, body: &[u8]) {
        *self.reply.lock().unwrap() = (status, body.to_vec());
    }
    fn reviews(&self) -> Vec<Recorded> {
        self.reviews.lock().unwrap().clone()
    }
}
impl Drop for Double {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn double(directory: &Path) -> Double {
    double_gated(directory, None).await
}

/// With a `gate`, each review is recorded on arrival and answered only once
/// it takes one permit of the gate, so a test can hold reviews in flight.
async fn double_gated(directory: &Path, gate: Option<Arc<tokio::sync::Semaphore>>) -> Double {
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
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
    let reviews = Arc::new(Mutex::new(Vec::new()));
    let reply = Arc::new(Mutex::new((
        201,
        serde_json::to_vec(&review(Some(PRINCIPAL), &[AUDIENCE], true)).unwrap(),
    )));
    let task = tokio::spawn({
        let reviews = reviews.clone();
        let reply = reply.clone();
        async move {
            let acceptor = TlsAcceptor::from(Arc::new(server));
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                tokio::spawn(answer(
                    acceptor.clone(),
                    stream,
                    reviews.clone(),
                    reply.clone(),
                    gate.clone(),
                ));
            }
        }
    });
    Double {
        api: format!("https://localhost:{port}"),
        ca,
        reviews,
        reply,
        task,
    }
}

async fn answer(
    acceptor: TlsAcceptor,
    stream: tokio::net::TcpStream,
    reviews: Arc<Mutex<Vec<Recorded>>>,
    reply: Arc<Mutex<(u16, Vec<u8>)>>,
    gate: Option<Arc<tokio::sync::Semaphore>>,
) {
    let Ok(mut stream) = acceptor.accept(stream).await else {
        return;
    };
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() > 16 * 1024 {
            return;
        }
        match stream.read_u8().await {
            Ok(byte) => head.push(byte),
            Err(_) => return,
        }
    }
    let head = String::from_utf8(head).unwrap();
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next().unwrap().split(' ');
    let method = request_line.next().unwrap().to_owned();
    let path = request_line.next().unwrap().to_owned();
    let mut length = 0;
    let mut authorization = None;
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            match name.trim().to_ascii_lowercase().as_str() {
                "content-length" => length = value.trim().parse().unwrap(),
                "authorization" => authorization = Some(value.trim().to_owned()),
                _ => {}
            }
        }
    }
    let mut body = vec![0; length];
    stream.read_exact(&mut body).await.unwrap();
    reviews.lock().unwrap().push(Recorded {
        method,
        path,
        authorization,
        body: serde_json::from_slice(&body).unwrap(),
    });
    if let Some(gate) = gate {
        gate.acquire().await.unwrap().forget();
    }
    let (status, bytes) = reply.lock().unwrap().clone();
    let head = format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    );
    let _ = stream.write_all(head.as_bytes()).await;
    let _ = stream.write_all(&bytes).await;
    let _ = stream.shutdown().await;
}

/// A TokenReview answer as the cluster API gives it.
fn review(username: Option<&str>, audiences: &[&str], authenticated: bool) -> Value {
    let mut status = json!({"audiences": audiences});
    if authenticated {
        status["authenticated"] = json!(true);
    } else {
        status["error"] = json!("invalid bearer token");
    }
    status["user"] = match username {
        Some(username) => json!({
            "username": username,
            "uid": "00000000-0000-4000-8000-000000000001",
            "groups": ["system:serviceaccounts", "system:serviceaccounts:agents", "system:authenticated"],
        }),
        None => json!({}),
    };
    json!({
        "apiVersion": "authentication.k8s.io/v1",
        "kind": "TokenReview",
        "metadata": {"creationTimestamp": null},
        "spec": {"audiences": [AUDIENCE]},
        "status": status,
    })
}

fn private_file(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
}

/// A `service-account-token` service configuration in a v3 document.
fn account_config(api: &str, ca: &Path, credential: &Path, state: &Path) -> Value {
    json!({
        "instance": INSTANCE,
        "listen": "127.0.0.1:0",
        "state": state,
        "admission": "service-account-token",
        "audience": AUDIENCE,
        "principal": PRINCIPAL,
        "token_review": {"api": api, "ca_file": ca, "credential_file": credential},
    })
}

struct Host {
    base: String,
    path: PathBuf,
    calls: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Host {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Reads the configuration document through the hosts' reader, builds its
/// admission and serves it on loopback.
async fn configured_host(root: &Path, document: &Value) -> Host {
    let file = root.join("service.yaml");
    std::fs::write(&file, serde_yaml_ng::to_string(document).unwrap()).unwrap();
    let config: ServiceConfigV3 = connectors_host::read_config(&file).unwrap();
    let admission = config.admission().unwrap();
    serve(root, admission).await
}

async fn serve(root: &Path, admission: Admission) -> Host {
    let path = root.join("state");
    if !path.exists() {
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let adapter = Arc::new(Fixture {
        calls: calls.clone(),
    });
    let descriptor = adapter.descriptor();
    let state = Arc::new(
        ServiceState::open(&path, INSTANCE, &descriptor.adapter, &descriptor.revision)
            .unwrap()
            .with_standing_grants(Some(standing_grants(root))),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = router_with_admission(adapter, admission, Some(state));
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Host {
        base: format!("http://{address}"),
        path,
        calls,
        task,
    }
}

/// An account host against `double`, with the instance's own token in place.
async fn account_host(root: &Path, double: &Double) -> Host {
    let credential = root.join("instance-token");
    private_file(&credential, INSTANCE_TOKEN.as_bytes());
    configured_host(
        root,
        &account_config(&double.api, &double.ca, &credential, &root.join("state")),
    )
    .await
}

/// A write carries its request ID as its `idempotency_key`
/// (story:served-keyed-writes-under-a-standing-approval).
fn request(operation: &str, request_id: &str) -> Value {
    let mut body = json!({"version":"v1alpha2","request_id":request_id,"operation":operation,"revision":"rev-1","input":{"value":1}});
    if operation == "write" {
        body["idempotency_key"] = json!(request_id);
    }
    body
}

/// A standing grant for the write on the input every request sends, so a
/// keyed write is admitted
/// (story:served-keyed-writes-under-a-standing-approval).
fn standing_grants(root: &Path) -> PathBuf {
    let document = json!({"format": "connectors.standing-grants/1", "revoked": [], "grants": [{
        "id": "g-write", "instance": INSTANCE, "operation": "write",
        "target": [{"pointer": "/value", "value": 1}],
        "not_after_unix_ms": 4_000_000_000_000_i64, "max_per_hour": 10_000}]});
    let path = root.join("grants.json");
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    path
}

async fn send(
    host: &Host,
    method: reqwest::Method,
    route: &str,
    authorization: Option<&str>,
    body: Option<&Value>,
) -> (u16, Vec<u8>) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut request = reqwest::Client::new().request(method, format!("{}{route}", host.base));
    if let Some(value) = authorization {
        request = request.header("authorization", value);
    }
    if let Some(body) = body {
        request = request.body(serde_json::to_vec(body).unwrap());
    }
    let response = request.send().await.unwrap();
    let status = response.status().as_u16();
    (status, response.bytes().await.unwrap().to_vec())
}

async fn invoke(host: &Host, authorization: Option<&str>, body: &Value) -> (u16, InvokeResponse) {
    let (status, bytes) = send(
        host,
        reqwest::Method::POST,
        "/v1alpha2/invoke",
        authorization,
        Some(body),
    )
    .await;
    (status, InvokeResponse::decode(&bytes).unwrap())
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

/// The answer of a refusal before the anchor: no request ID, no record.
fn assert_refused(status: u16, response: &InvokeResponse, http: u16, code: ErrorCode) {
    assert_eq!(status, http);
    assert_eq!(response.status, ResponseStatus::Error);
    assert_eq!(response.error.as_ref().unwrap().code, code);
    assert_eq!(response.request_id, None);
    assert_eq!(response.audit_ref, None);
    assert_eq!(response.audit_status, AuditStatus::Unavailable);
    assert!(response.mutation.is_none());
}

fn anchor(path: &Path, audit_ref: &str) -> audit::Anchor {
    audit::Store::new(path, 100_000)
        .unwrap()
        .observe(&Reference {
            instance: INSTANCE.into(),
            audit_ref: audit_ref.into(),
        })
        .unwrap()
        .expect("the anchored record")
        .anchor
}

/// v1alpha2-service-account-token-admitted.
#[tokio::test]
async fn a_reviewed_token_of_the_configured_principal_is_admitted() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let host = account_host(root.path(), &double).await;

    let (status, response) =
        invoke(&host, Some(&bearer(CALLER_TOKEN)), &request("read", "one")).await;
    assert_eq!(status, 200);
    assert_eq!(response.status, ResponseStatus::Success);
    assert_eq!(response.result, Some(json!({"value":1})));
    assert_eq!(response.audit_status, AuditStatus::Complete);
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    assert_eq!(double.reviews().len(), 1);

    // Nothing is cached: the next request is reviewed again.
    let (status, _) = invoke(&host, Some(&bearer(CALLER_TOKEN)), &request("read", "two")).await;
    assert_eq!(status, 200);
    assert_eq!(double.reviews().len(), 2);
    assert_eq!(host.calls.load(Ordering::SeqCst), 2);

    // The legacy describe admits by the same review: a client describes there.
    let (status, bytes) = send(
        &host,
        reqwest::Method::GET,
        "/v1/describe",
        Some(&bearer(CALLER_TOKEN)),
        None,
    )
    .await;
    assert_eq!(status, 200);
    let descriptor: Descriptor = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(descriptor.revision, "rev-1");
    assert_eq!(double.reviews().len(), 3);

    // The legacy invoke writes no anchor, so this profile does not serve it:
    // `unsupported` before any review and with zero adapter calls.
    let legacy = json!({"version": WIRE_VERSION, "request_id":"legacy","operation":"read","revision":"rev-1","input":{}});
    let (status, bytes) = send(
        &host,
        reqwest::Method::POST,
        "/v1/invoke",
        Some(&bearer(CALLER_TOKEN)),
        Some(&legacy),
    )
    .await;
    assert_eq!(status, 501);
    let error: connectors_core::Error = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(error.code, connectors_core::ErrorCode::Unsupported);
    assert_eq!(double.reviews().len(), 3);
    assert_eq!(host.calls.load(Ordering::SeqCst), 2);
}

/// v1alpha2-service-account-token-admitted: the review names the presented
/// token and exactly the configured audience, and carries the instance's own
/// token, read again at each review.
#[tokio::test]
async fn the_review_names_the_configured_audience() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let host = account_host(root.path(), &double).await;

    let (status, _) = invoke(&host, Some(&bearer(CALLER_TOKEN)), &request("read", "one")).await;
    assert_eq!(status, 200);
    let reviews = double.reviews();
    assert_eq!(reviews.len(), 1);
    let sent = &reviews[0];
    assert_eq!(sent.method, "POST");
    // The fixed review path with no query (the shared client writes an empty
    // one as a bare `?`).
    assert_eq!(
        sent.path.strip_suffix('?').unwrap_or(&sent.path),
        REVIEW_PATH
    );
    assert_eq!(
        sent.authorization.as_deref(),
        Some(bearer(INSTANCE_TOKEN).as_str())
    );
    assert_eq!(
        sent.body,
        json!({
            "apiVersion": "authentication.k8s.io/v1",
            "kind": "TokenReview",
            "spec": {"token": CALLER_TOKEN, "audiences": [AUDIENCE]},
        })
    );

    // The instance's token is a file read at each review: a rotated token is
    // the one the next review carries.
    private_file(
        &root.path().join("instance-token"),
        b"instance-rotated-token",
    );
    let (status, _) = invoke(&host, Some(&bearer(CALLER_TOKEN)), &request("read", "two")).await;
    assert_eq!(status, 200);
    assert_eq!(
        double.reviews()[1].authorization.as_deref(),
        Some("Bearer instance-rotated-token")
    );
}

/// v1alpha2-service-account-token-other-principal: another service account.
#[tokio::test]
async fn a_token_of_another_service_account_is_refused_unauthorized_with_no_adapter_call() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let host = account_host(root.path(), &double).await;

    // Another account, a longer name with the configured one as its prefix,
    // and a user that is not a service account at all.
    let others = [
        OTHER_PRINCIPAL.to_owned(),
        format!("{PRINCIPAL}-2"),
        "system:serviceaccount:other-namespace:example-agent".to_owned(),
        "example-user".to_owned(),
    ];
    for (index, username) in others.iter().enumerate() {
        double.answer(201, &review(Some(username), &[AUDIENCE], true));
        for operation in ["read", "write"] {
            let (status, response) = invoke(
                &host,
                Some(&bearer(CALLER_TOKEN)),
                &request(operation, &format!("{operation}-{index}")),
            )
            .await;
            assert_refused(status, &response, 401, ErrorCode::Unauthorized);
        }
    }
    let (status, _) = send(
        &host,
        reqwest::Method::GET,
        "/v1/describe",
        Some(&bearer(CALLER_TOKEN)),
        None,
    )
    .await;
    assert_eq!(status, 401);
    assert_eq!(double.reviews().len(), others.len() * 2 + 1);
    assert_eq!(host.calls.load(Ordering::SeqCst), 0);
}

/// v1alpha2-service-account-token-other-principal: a token the review does
/// not authenticate.
#[tokio::test]
async fn a_token_the_review_does_not_authenticate_is_refused_with_no_adapter_call() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let host = account_host(root.path(), &double).await;

    // `authenticated` absent (the cluster's answer for an invalid token),
    // explicitly false, and false while still naming the principal.
    let mut explicit = review(None, &[], false);
    explicit["status"]["authenticated"] = json!(false);
    let mut named = review(Some(PRINCIPAL), &[AUDIENCE], false);
    named["status"]["authenticated"] = json!(false);
    for (index, answer) in [review(None, &[], false), explicit, named]
        .iter()
        .enumerate()
    {
        double.answer(201, answer);
        let (status, response) = invoke(
            &host,
            Some(&bearer(CALLER_TOKEN)),
            &request("read", &format!("read-{index}")),
        )
        .await;
        assert_refused(status, &response, 401, ErrorCode::Unauthorized);
    }
    assert_eq!(double.reviews().len(), 3);
    assert_eq!(host.calls.load(Ordering::SeqCst), 0);
}

/// v1alpha2-service-account-token-other-principal: a token authenticated for
/// another audience.
#[tokio::test]
async fn a_review_for_another_audience_is_refused_with_no_adapter_call() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let host = account_host(root.path(), &double).await;

    let mut absent = review(Some(PRINCIPAL), &[], true);
    absent["status"]
        .as_object_mut()
        .unwrap()
        .remove("audiences");
    for (index, answer) in [
        review(Some(PRINCIPAL), &["other-audience"], true),
        review(
            Some(PRINCIPAL),
            &["connectors-leaf-2", "https://kubernetes.default.svc"],
            true,
        ),
        review(Some(PRINCIPAL), &[], true),
        absent,
    ]
    .iter()
    .enumerate()
    {
        double.answer(201, answer);
        let (status, response) = invoke(
            &host,
            Some(&bearer(CALLER_TOKEN)),
            &request("write", &format!("write-{index}")),
        )
        .await;
        assert_refused(status, &response, 401, ErrorCode::Unauthorized);
    }
    assert_eq!(double.reviews().len(), 4);
    assert_eq!(host.calls.load(Ordering::SeqCst), 0);
}

/// v1alpha2-service-account-token-review-unavailable: admission fails closed.
#[tokio::test]
async fn a_failing_or_unreachable_review_is_unavailable_with_no_adapter_call() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let host = account_host(root.path(), &double).await;

    // A failing review: error statuses (the cluster refusing the instance's
    // own token among them), a 2xx that is not a TokenReview, a body that does
    // not decode, and one over the response bound.
    let mut not_review = review(Some(PRINCIPAL), &[AUDIENCE], true);
    not_review["kind"] = json!("SubjectAccessReview");
    let oversized = {
        let mut answer = review(Some(PRINCIPAL), &[AUDIENCE], true);
        answer["metadata"]["annotations"] = json!({"padding": "x".repeat(70 * 1024)});
        serde_json::to_vec(&answer).unwrap()
    };
    let failing: Vec<(u16, Vec<u8>)> = vec![
        (500, b"{}".to_vec()),
        (401, b"{}".to_vec()),
        (
            403,
            serde_json::to_vec(&review(Some(PRINCIPAL), &[AUDIENCE], true)).unwrap(),
        ),
        (201, serde_json::to_vec(&not_review).unwrap()),
        (201, b"not json".to_vec()),
        (201, oversized),
    ];
    for (index, (status, body)) in failing.iter().enumerate() {
        double.answer_bytes(*status, body);
        let (status, response) = invoke(
            &host,
            Some(&bearer(CALLER_TOKEN)),
            &request("read", &format!("read-{index}")),
        )
        .await;
        assert_refused(status, &response, 503, ErrorCode::Unavailable);
    }
    assert_eq!(double.reviews().len(), failing.len());

    // The instance's own token unusable: `unavailable`, never the caller's
    // `unauthorized`, and no review is sent without it.
    let credential = root.path().join("instance-token");
    std::fs::set_permissions(&credential, std::fs::Permissions::from_mode(0o644)).unwrap();
    double.answer(201, &review(Some(PRINCIPAL), &[AUDIENCE], true));
    let (status, response) =
        invoke(&host, Some(&bearer(CALLER_TOKEN)), &request("read", "x")).await;
    assert_refused(status, &response, 503, ErrorCode::Unavailable);
    assert_eq!(double.reviews().len(), failing.len());
    // A failure is not remembered: the next request is reviewed again.
    std::fs::set_permissions(&credential, std::fs::Permissions::from_mode(0o600)).unwrap();
    let (status, _) = invoke(&host, Some(&bearer(CALLER_TOKEN)), &request("read", "y")).await;
    assert_eq!(status, 200);
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);

    // The legacy describe fails closed the same way.
    double.answer_bytes(500, b"{}");
    let (status, _) = send(
        &host,
        reqwest::Method::GET,
        "/v1/describe",
        Some(&bearer(CALLER_TOKEN)),
        None,
    )
    .await;
    assert_eq!(status, 503);

    // An unreachable review endpoint.
    let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    let unreachable_root = tempfile::tempdir().unwrap();
    let credential = unreachable_root.path().join("instance-token");
    private_file(&credential, INSTANCE_TOKEN.as_bytes());
    let unreachable = configured_host(
        unreachable_root.path(),
        &account_config(
            &format!("https://localhost:{port}"),
            &double.ca,
            &credential,
            &unreachable_root.path().join("state"),
        ),
    )
    .await;
    for operation in ["read", "write"] {
        let (status, response) = invoke(
            &unreachable,
            Some(&bearer(CALLER_TOKEN)),
            &request(operation, operation),
        )
        .await;
        assert_refused(status, &response, 503, ErrorCode::Unavailable);
    }
    assert_eq!(unreachable.calls.load(Ordering::SeqCst), 0);
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
}

/// v1alpha2-service-account-token-review-bound: with `CONCURRENT_REVIEWS`
/// reviews in flight, the next request on either admitting route is
/// `unavailable` at once, with no review sent and zero adapter calls; the held
/// reviews then admit, and a later request is reviewed again.
#[tokio::test]
async fn a_request_past_the_review_bound_is_unavailable_with_no_review() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let root = tempfile::tempdir().unwrap();
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    let double = double_gated(root.path(), Some(gate.clone())).await;
    let host = account_host(root.path(), &double).await;

    let held: Vec<_> = (0..CONCURRENT_REVIEWS)
        .map(|index| {
            let url = format!("{}/v1alpha2/invoke", host.base);
            let body = serde_json::to_vec(&request("read", &format!("held-{index}"))).unwrap();
            tokio::spawn(async move {
                let response = reqwest::Client::new()
                    .post(url)
                    .header("authorization", bearer(CALLER_TOKEN))
                    .body(body)
                    .send()
                    .await
                    .unwrap();
                response.status().as_u16()
            })
        })
        .collect();
    // Within the 5-second review deadline, so the held reviews do not expire.
    let started = std::time::Instant::now();
    while double.reviews().len() < CONCURRENT_REVIEWS {
        assert!(
            started.elapsed() < std::time::Duration::from_secs(3),
            "{} of {CONCURRENT_REVIEWS} reviews reached the double",
            double.reviews().len()
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    let (status, response) =
        invoke(&host, Some(&bearer(CALLER_TOKEN)), &request("read", "past")).await;
    assert_refused(status, &response, 503, ErrorCode::Unavailable);
    let (status, _) = send(
        &host,
        reqwest::Method::GET,
        "/v1/describe",
        Some(&bearer(CALLER_TOKEN)),
        None,
    )
    .await;
    assert_eq!(status, 503);
    assert_eq!(double.reviews().len(), CONCURRENT_REVIEWS);
    assert_eq!(host.calls.load(Ordering::SeqCst), 0);

    gate.add_permits(CONCURRENT_REVIEWS);
    for task in held {
        assert_eq!(task.await.unwrap(), 200);
    }
    assert_eq!(host.calls.load(Ordering::SeqCst), CONCURRENT_REVIEWS);
    gate.add_permits(1);
    let (status, _) = invoke(
        &host,
        Some(&bearer(CALLER_TOKEN)),
        &request("read", "after"),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(double.reviews().len(), CONCURRENT_REVIEWS + 1);
}

/// v1alpha2-service-account-token-other-principal: no bearer, no review.
#[tokio::test]
async fn a_missing_bearer_is_unauthorized_before_any_review() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let host = account_host(root.path(), &double).await;

    for authorization in [
        None,
        Some("Bearer "),
        Some("Basic Y2FsbGVyOnRva2Vu"),
        Some(CALLER_TOKEN),
    ] {
        let (status, response) = invoke(&host, authorization, &request("read", "one")).await;
        assert_refused(status, &response, 401, ErrorCode::Unauthorized);
        let (status, _) = send(
            &host,
            reqwest::Method::GET,
            "/v1/describe",
            authorization,
            None,
        )
        .await;
        assert_eq!(status, 401);
    }
    // A token over the bound is refused the same way, before any review.
    let oversized = bearer(&"t".repeat(9 * 1024));
    let (status, response) = invoke(&host, Some(&oversized), &request("read", "two")).await;
    assert_refused(status, &response, 401, ErrorCode::Unauthorized);
    assert!(double.reviews().is_empty());
    assert_eq!(host.calls.load(Ordering::SeqCst), 0);
}

/// The anchor's principal under this profile is the configured service
/// account, for a read and for a write.
#[tokio::test]
async fn the_anchor_principal_is_the_service_account() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let host = account_host(root.path(), &double).await;

    for operation in ["read", "write"] {
        let (status, response) = invoke(
            &host,
            Some(&bearer(CALLER_TOKEN)),
            &request(operation, operation),
        )
        .await;
        assert_eq!(status, 200);
        let anchored = anchor(&host.path, &response.audit_ref.expect("an anchor"));
        assert_eq!(anchored.principal_ref.as_deref(), Some(PRINCIPAL));
        assert_eq!(anchored.instance_id, INSTANCE);
        assert_eq!(anchored.request_id.as_deref(), Some(operation));
    }
    assert_eq!(host.calls.load(Ordering::SeqCst), 2);
}

struct FixedSecret;
#[async_trait]
impl Credential for FixedSecret {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(b"service-token".to_vec()))
    }
}

/// `static-bearer` stays the default: a v2 configuration admits by its bearer,
/// anchors `static-bearer`, serves `/v1/invoke` and sends no review.
#[tokio::test]
async fn static_bearer_admission_is_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let token = root.path().join("token");
    private_file(&token, b"service-token\n");
    let document = json!({
        "instance": INSTANCE,
        "listen": "127.0.0.1:0",
        "service_credential": {"kind": "file", "path": token},
        "state": root.path().join("state"),
    });
    let file = root.path().join("service.yaml");
    std::fs::write(&file, serde_yaml_ng::to_string(&document).unwrap()).unwrap();
    // The v2 reader every existing host uses, into the v3 admission.
    let config: ServiceConfig = connectors_host::read_config(&file).unwrap();
    let config = ServiceConfigV3::from(config);
    let admission = config.admission().unwrap();
    assert!(matches!(admission, Admission::StaticBearer(_)));
    let configured = serve(root.path(), admission).await;

    // The router every existing caller builds, unchanged.
    let other = tempfile::tempdir().unwrap();
    let state_path = other.path().join("state");
    std::fs::create_dir(&state_path).unwrap();
    std::fs::set_permissions(&state_path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let adapter = Arc::new(Fixture {
        calls: calls.clone(),
    });
    let state = Arc::new(ServiceState::open(&state_path, INSTANCE, "fixture", "rev-1").unwrap());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = router_with_state(adapter, Arc::new(FixedSecret), Some(state));
    let legacy_router = Host {
        base: format!("http://{address}"),
        path: state_path,
        calls,
        task: tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        }),
    };

    for host in [&configured, &legacy_router] {
        let (status, response) =
            invoke(host, Some("Bearer service-token"), &request("read", "one")).await;
        assert_eq!(status, 200);
        let anchored = anchor(&host.path, &response.audit_ref.unwrap());
        assert_eq!(anchored.principal_ref.as_deref(), Some("static-bearer"));
        let (status, response) =
            invoke(host, Some("Bearer another-token"), &request("read", "two")).await;
        assert_refused(status, &response, 401, ErrorCode::Unauthorized);
        let (status, response) = invoke(host, None, &request("read", "three")).await;
        assert_refused(status, &response, 401, ErrorCode::Unauthorized);
        let (status, _) = send(
            host,
            reqwest::Method::GET,
            "/v1/describe",
            Some("Bearer service-token"),
            None,
        )
        .await;
        assert_eq!(status, 200);
        let legacy = json!({"version": WIRE_VERSION, "request_id":"legacy","operation":"read","revision":"rev-1","input":{}});
        let (status, _) = send(
            host,
            reqwest::Method::POST,
            "/v1/invoke",
            Some("Bearer service-token"),
            Some(&legacy),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(host.calls.load(Ordering::SeqCst), 2);
    }
}

fn schema(reference: &str) -> Value {
    let mut value = json!({"$ref": reference});
    connectors_host::schema::expand(&mut value).unwrap();
    value
}

/// v1 and v2 documents read unchanged, under their own closed readers and
/// under v3; the admission members exist only under v3.
#[test]
fn a_v2_service_configuration_reads_unchanged() {
    let v1 = json!({"instance":"leaf","listen":"127.0.0.1:0","service_credential":{"kind":"file","path":"token"}});
    let mut v2 = v1.clone();
    v2["state"] = json!("/var/lib/connectors");
    // A real CA, so a refusal below is the rule's and not a missing file's.
    let root = tempfile::tempdir().unwrap();
    let ca = root.path().join("ca.crt");
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    std::fs::write(&ca, certificate.cert.pem()).unwrap();
    let review = json!({"api":"https://kubernetes.default.svc","ca_file":ca,"credential_file":root.path().join("token")});
    let members = [
        ("admission", json!("service-account-token")),
        ("audience", json!(AUDIENCE)),
        ("principal", json!(PRINCIPAL)),
        ("token_review", review.clone()),
    ];

    // The same meaning and the same serialized bytes under v2 and v3, so no
    // configuration or descriptor revision moves.
    for document in [&v1, &v2] {
        let read: ServiceConfig = serde_json::from_value(document.clone()).unwrap();
        assert_eq!(serde_json::to_value(&read).unwrap(), *document);
        let read: ServiceConfigV3 = serde_json::from_value(document.clone()).unwrap();
        assert_eq!(serde_json::to_value(&read).unwrap(), *document);
        assert!(matches!(
            read.admission().unwrap(),
            Admission::StaticBearer(_)
        ));
        let widened = ServiceConfigV3::from(
            serde_json::from_value::<ServiceConfig>(document.clone()).unwrap(),
        );
        assert_eq!(serde_json::to_value(&widened).unwrap(), *document);
    }

    // The v1 and v2 readers stay closed: no admission member under them.
    let (v1_schema, v2_schema, v3_schema) = (
        schema("urn:connectors:config:v1:service"),
        schema("urn:connectors:config:v2:service"),
        schema("urn:connectors:config:v3:service"),
    );
    connectors_sdk::validate(&v2_schema, &v2).unwrap();
    let properties = |schema: &Value| -> Vec<String> {
        let mut names: Vec<String> = schema["allOf"][0]["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        names.sort();
        names
    };
    assert_eq!(
        properties(&v2_schema),
        ["instance", "listen", "service_credential", "state"]
    );
    for (member, value) in &members {
        let mut extended = v2.clone();
        extended[*member] = value.clone();
        assert!(serde_json::from_value::<ServiceConfig>(extended.clone()).is_err());
        assert!(connectors_sdk::validate(&v1_schema, &extended).is_err());
        assert!(connectors_sdk::validate(&v2_schema, &extended).is_err());
    }

    // v3 reads v1, v2 and the account profile, and refuses a mixed document.
    connectors_sdk::validate(&v3_schema, &v1).unwrap();
    connectors_sdk::validate(&v3_schema, &v2).unwrap();
    let mut account =
        json!({"instance":"leaf","listen":"127.0.0.1:0","state":"/var/lib/connectors"});
    for (member, value) in &members {
        account[*member] = value.clone();
    }
    connectors_sdk::validate(&v3_schema, &account).unwrap();
    let read: ServiceConfigV3 = serde_json::from_value(account.clone()).unwrap();
    assert_eq!(serde_json::to_value(&read).unwrap(), account);
    assert!(matches!(
        read.admission().unwrap(),
        Admission::ServiceAccountToken(_)
    ));

    let mut with_bearer = account.clone();
    with_bearer["service_credential"] = v1["service_credential"].clone();
    let mut without_principal = account.clone();
    without_principal
        .as_object_mut()
        .unwrap()
        .remove("principal");
    let mut static_with_audience = v2.clone();
    static_with_audience["audience"] = json!(AUDIENCE);
    let mut explicit_static_with_review = v2.clone();
    explicit_static_with_review["admission"] = json!("static-bearer");
    explicit_static_with_review["token_review"] = review.clone();
    let mut malformed = Vec::new();
    for principal in [
        "system:serviceaccount:agents",
        "system:serviceaccount:agents:",
        "system:serviceaccount::example-agent",
        "system:serviceaccount:Agents:example-agent",
        "system:serviceaccount:agents:example-agent:extra",
        "user:example-agent",
        "system:serviceaccount:agents:-example",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain([format!("system:serviceaccount:agents:{}", "a".repeat(120))])
    {
        let mut document = account.clone();
        document["principal"] = json!(principal);
        malformed.push(document);
    }
    let mut no_audience = account.clone();
    no_audience["audience"] = json!("");
    let mut plaintext = account.clone();
    plaintext["token_review"]["api"] = json!("http://kubernetes.default.svc");
    for refused in [
        with_bearer,
        without_principal,
        static_with_audience,
        explicit_static_with_review,
        no_audience,
    ]
    .iter()
    .chain(&malformed)
    {
        assert!(
            connectors_sdk::validate(&v3_schema, refused).is_err(),
            "the v3 schema refuses {refused}"
        );
        let read: ServiceConfigV3 = serde_json::from_value(refused.clone()).unwrap();
        assert!(read.admission().is_err(), "the reader refuses {refused}");
    }
    // An endpoint without TLS is refused when the admission is built.
    let read: ServiceConfigV3 = serde_json::from_value(plaintext).unwrap();
    assert!(read.admission().is_err());
}

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
        Admission, CONCURRENT_REVIEWS, ServiceAccountPattern, ServiceAccounts, ServiceConfig,
        ServiceConfigV3, ServiceState, router_with_admission, router_with_state,
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
        ServiceState::open(&path, INSTANCE, &descriptor.adapter, &descriptor.revision).unwrap(),
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

fn request(operation: &str, request_id: &str) -> Value {
    json!({"version":"v1alpha2","request_id":request_id,"operation":operation,"revision":"rev-1","input":{"value":1}})
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

/// An account host whose single `principal` is replaced by `accounts`, the
/// `principals` or `service_accounts` member.
async fn account_set_host(root: &Path, double: &Double, accounts: (&str, Value)) -> Host {
    let credential = root.join("instance-token");
    private_file(&credential, INSTANCE_TOKEN.as_bytes());
    let mut document = account_config(&double.api, &double.ca, &credential, &root.join("state"));
    document.as_object_mut().unwrap().remove("principal");
    document[accounts.0] = accounts.1;
    configured_host(root, &document).await
}

/// Each `username` the review names, as admitted (`Some(anchored principal)`)
/// or refused `unauthorized` before any anchor (`None`), for a read and a write.
async fn admits(host: &Host, double: &Double, username: &str, expected: Option<&str>) {
    double.answer(201, &review(Some(username), &[AUDIENCE], true));
    for operation in ["read", "write"] {
        let calls = host.calls.load(Ordering::SeqCst);
        let request_id = format!("{operation}-{}", double.reviews().len());
        let (status, response) = invoke(
            host,
            Some(&bearer(CALLER_TOKEN)),
            &request(operation, &request_id),
        )
        .await;
        match expected {
            Some(principal) => {
                assert_eq!(status, 200, "{username} was refused");
                let anchored = anchor(&host.path, &response.audit_ref.expect("an anchor"));
                assert_eq!(anchored.principal_ref.as_deref(), Some(principal));
                assert_eq!(host.calls.load(Ordering::SeqCst), calls + 1);
            }
            None => {
                assert_refused(status, &response, 401, ErrorCode::Unauthorized);
                assert_eq!(host.calls.load(Ordering::SeqCst), calls, "{username}");
            }
        }
    }
}

/// v1alpha2-service-account-token-listed-accounts: `principals` admits each
/// listed account and no other, and the anchor records the reviewed username.
#[tokio::test]
async fn a_listed_service_account_is_admitted_and_anchored_as_itself() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let host = account_set_host(
        root.path(),
        &double,
        ("principals", json!([PRINCIPAL, OTHER_PRINCIPAL])),
    )
    .await;

    admits(&host, &double, PRINCIPAL, Some(PRINCIPAL)).await;
    admits(&host, &double, OTHER_PRINCIPAL, Some(OTHER_PRINCIPAL)).await;
    for refused in [
        "system:serviceaccount:agents:third-agent".to_owned(),
        format!("{PRINCIPAL}-2"),
        "system:serviceaccount:other-namespace:example-agent".to_owned(),
        "example-user".to_owned(),
    ] {
        admits(&host, &double, &refused, None).await;
    }
    assert_eq!(double.reviews().len(), 12);
}

/// v1alpha2-service-account-token-account-pattern: `service_accounts` admits
/// an account of the literal namespace whose name matches the pattern, `*`
/// standing for any run of characters; the anchor records the reviewed
/// username.
#[tokio::test]
async fn a_namespace_and_name_pattern_admits_each_matching_account() {
    let root = tempfile::tempdir().unwrap();
    let double = double(root.path()).await;
    let host = account_set_host(
        root.path(),
        &double,
        (
            "service_accounts",
            json!({"namespace": "agents", "name": "*-agent"}),
        ),
    )
    .await;

    for admitted in [
        PRINCIPAL,
        OTHER_PRINCIPAL,
        "system:serviceaccount:agents:a.b-agent",
    ] {
        admits(&host, &double, admitted, Some(admitted)).await;
    }
    for refused in [
        // The pattern is anchored at both ends.
        "system:serviceaccount:agents:example-agent-2",
        "system:serviceaccount:agents:coder",
        // `*` matches the name only, and the namespace is literal.
        "system:serviceaccount:agents-2:example-agent",
        "system:serviceaccount:other:example-agent",
        "system:serviceaccount:Agents:example-agent",
        "system:serviceaccounts:agents:example-agent",
        "example-agent",
    ] {
        admits(&host, &double, refused, None).await;
    }
}

/// v1alpha2-service-account-token-account-set-config: exactly one of
/// `principal`, `principals` and `service_accounts` is configured. None, two,
/// an empty or oversized list, an invalid entry and an empty or invalid
/// pattern are refused by the schema and at start, naming the key.
#[test]
fn exactly_one_account_form_is_configured() {
    let root = tempfile::tempdir().unwrap();
    let ca = root.path().join("ca.crt");
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    std::fs::write(&ca, certificate.cert.pem()).unwrap();
    let mut base = account_config(
        "https://kubernetes.default.svc",
        &ca,
        &root.path().join("token"),
        Path::new("/var/lib/connectors"),
    );
    base.as_object_mut().unwrap().remove("principal");
    let v3_schema = schema("urn:connectors:config:v3:service");
    let with = |members: &[(&str, Value)]| {
        let mut document = base.clone();
        for (member, value) in members {
            document[*member] = value.clone();
        }
        document
    };
    let one = ("principal", json!(PRINCIPAL));
    let listed = ("principals", json!([PRINCIPAL, OTHER_PRINCIPAL]));
    let pattern = (
        "service_accounts",
        json!({"namespace": "agents", "name": "*-agent"}),
    );

    for admitted in [
        with(std::slice::from_ref(&one)),
        with(std::slice::from_ref(&listed)),
        with(&[("principals", json!([PRINCIPAL]))]),
        with(&[(
            "principals",
            json!(
                (0..64)
                    .map(|n| format!("system:serviceaccount:agents:agent-{n}"))
                    .collect::<Vec<_>>()
            ),
        )]),
        with(std::slice::from_ref(&pattern)),
        with(&[(
            "service_accounts",
            json!({"namespace": "agents", "name": "*"}),
        )]),
        with(&[(
            "service_accounts",
            json!({"namespace": "agents", "name": "coding-agent"}),
        )]),
    ] {
        connectors_sdk::validate(&v3_schema, &admitted)
            .unwrap_or_else(|_| panic!("the v3 schema admits {admitted}"));
        let read: ServiceConfigV3 = serde_json::from_value(admitted.clone()).unwrap();
        assert_eq!(serde_json::to_value(&read).unwrap(), admitted);
        assert!(
            matches!(read.admission(), Ok(Admission::ServiceAccountToken(_))),
            "the reader admits {admitted}"
        );
    }

    let keys = "exactly one of principal, principals or service_accounts";
    let long_name = format!("system:serviceaccount:agents:{}", "a".repeat(120));
    let refusals: Vec<(Value, &str)> = vec![
        (with(&[]), keys),
        (with(&[one.clone(), listed.clone()]), keys),
        (with(&[one.clone(), pattern.clone()]), keys),
        (with(&[listed.clone(), pattern.clone()]), keys),
        (with(&[one, listed, pattern]), keys),
        (with(&[("principals", json!([]))]), "admission principals"),
        (
            with(&[(
                "principals",
                json!(
                    (0..65)
                        .map(|n| format!("system:serviceaccount:agents:agent-{n}"))
                        .collect::<Vec<_>>()
                ),
            )]),
            "admission principals",
        ),
        (
            with(&[("principals", json!([PRINCIPAL, "user:example-agent"]))]),
            "admission principals",
        ),
        (
            with(&[("principals", json!([PRINCIPAL, long_name]))]),
            "admission principals",
        ),
        (
            with(&[(
                "principals",
                json!(["system:serviceaccount:agents:*-agent"]),
            )]),
            "admission principals",
        ),
        (
            with(&[(
                "service_accounts",
                json!({"namespace": "agents", "name": ""}),
            )]),
            "admission service_accounts",
        ),
        (
            with(&[(
                "service_accounts",
                json!({"namespace": "agents", "name": "agent-?"}),
            )]),
            "admission service_accounts",
        ),
        (
            with(&[(
                "service_accounts",
                json!({"namespace": "agents", "name": "Coding-*"}),
            )]),
            "admission service_accounts",
        ),
        (
            with(&[(
                "service_accounts",
                json!({"namespace": "agents", "name": "a:*"}),
            )]),
            "admission service_accounts",
        ),
        (
            with(&[(
                "service_accounts",
                json!({"namespace": "agents", "name": "a".repeat(129)}),
            )]),
            "admission service_accounts",
        ),
        (
            with(&[("service_accounts", json!({"namespace": "*", "name": "*"}))]),
            "admission service_accounts",
        ),
        (
            with(&[(
                "service_accounts",
                json!({"namespace": "agent*", "name": "*"}),
            )]),
            "admission service_accounts",
        ),
        (
            with(&[("service_accounts", json!({"namespace": "", "name": "*"}))]),
            "admission service_accounts",
        ),
    ];
    // A pattern no account name can match, read with each `*` as one letter:
    // a dot or hyphen at a label's edge, an empty label, or a label over 63.
    let mut refusals = refusals;
    for unmatched in [
        ".".to_owned(),
        "-*".to_owned(),
        "*-".to_owned(),
        "a..b".to_owned(),
        "*.".to_owned(),
        ".*".to_owned(),
        "a-.b".to_owned(),
        "*.-a".to_owned(),
        "*a".repeat(64),
        format!("{}*", "a".repeat(63)),
    ] {
        refusals.push((
            with(&[(
                "service_accounts",
                json!({"namespace": "agents", "name": unmatched}),
            )]),
            "admission service_accounts",
        ));
    }
    for (refused, key) in &refusals {
        assert!(
            connectors_sdk::validate(&v3_schema, refused).is_err(),
            "the v3 schema refuses {refused}"
        );
        let read: ServiceConfigV3 = serde_json::from_value(refused.clone()).unwrap();
        let error = read
            .admission()
            .err()
            .unwrap_or_else(|| panic!("the reader refuses {refused}"));
        assert!(
            error.message.contains(key),
            "{refused}: {:?} does not name {key}",
            error.message
        );
    }
    // The account's 128 bytes are shared by the namespace and the name, so
    // a name one byte past what the namespace leaves matches no account. The
    // schema bounds each member alone; start checks their sum.
    let namespace = "n".repeat(63);
    let budget = 128 - "system:serviceaccount:".len() - namespace.len() - 1;
    for (name, admitted) in [
        (format!("{}*", "a".repeat(budget - 1)), true),
        ("a".repeat(budget), true),
        (format!("{}*", "a".repeat(budget)), false),
        ("a".repeat(budget + 1), false),
    ] {
        let document = with(&[(
            "service_accounts",
            json!({"namespace": namespace, "name": name}),
        )]);
        let read: ServiceConfigV3 = serde_json::from_value(document.clone()).unwrap();
        match read.admission() {
            Ok(_) => assert!(admitted, "the reader admits {document}"),
            Err(error) => {
                assert!(!admitted, "the reader refuses {document}");
                assert!(error.message.contains("admission service_accounts"));
            }
        }
    }

    // `service_accounts` is closed, and neither set member is a
    // `static-bearer` member.
    for refused in [
        with(&[(
            "service_accounts",
            json!({"namespace": "agents", "name": "*", "groups": ["x"]}),
        )]),
        with(&[("service_accounts", json!({"name": "*"}))]),
        with(&[("principals", json!(PRINCIPAL))]),
    ] {
        assert!(connectors_sdk::validate(&v3_schema, &refused).is_err());
        assert!(serde_json::from_value::<ServiceConfigV3>(refused).is_err());
    }
    let token = root.path().join("token");
    for member in [
        ("principals", json!([PRINCIPAL])),
        (
            "service_accounts",
            json!({"namespace": "agents", "name": "*"}),
        ),
    ] {
        let mut document = json!({"instance":"leaf","listen":"127.0.0.1:0","service_credential":{"kind":"file","path":token}});
        document[member.0] = member.1;
        assert!(connectors_sdk::validate(&v3_schema, &document).is_err());
        let read: ServiceConfigV3 = serde_json::from_value(document.clone()).unwrap();
        assert!(read.admission().is_err(), "{document}");
    }
}

/// A started admission describes the accounts it was configured with, and
/// `principal()` answers what its documentation says for each form.
#[test]
fn a_started_admission_describes_its_configured_accounts() {
    let root = tempfile::tempdir().unwrap();
    let ca = root.path().join("ca.crt");
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    std::fs::write(&ca, certificate.cert.pem()).unwrap();
    let mut base = account_config(
        "https://kubernetes.default.svc",
        &ca,
        &root.path().join("token"),
        Path::new("/var/lib/connectors"),
    );
    base.as_object_mut().unwrap().remove("principal");
    let started = |member: &str, value: Value| {
        let mut document = base.clone();
        document[member] = value;
        let read: ServiceConfigV3 = serde_json::from_value(document).unwrap();
        match read.admission() {
            Ok(Admission::ServiceAccountToken(admission)) => admission,
            _ => panic!("{member} was not admitted"),
        }
    };
    let pattern = ServiceAccountPattern {
        namespace: "agents".into(),
        name: "*-agent".into(),
    };
    for (member, value, accounts, principal) in [
        (
            "principal",
            json!(PRINCIPAL),
            ServiceAccounts::Listed(vec![PRINCIPAL.into()]),
            PRINCIPAL,
        ),
        (
            "principals",
            json!([OTHER_PRINCIPAL, PRINCIPAL]),
            ServiceAccounts::Listed(vec![OTHER_PRINCIPAL.into(), PRINCIPAL.into()]),
            OTHER_PRINCIPAL,
        ),
        (
            "service_accounts",
            serde_json::to_value(&pattern).unwrap(),
            ServiceAccounts::Pattern(pattern.clone()),
            "system:serviceaccount:agents:*-agent",
        ),
    ] {
        let admission = started(member, value);
        assert_eq!(admission.accounts(), &accounts, "{member}");
        assert_eq!(admission.principal(), principal, "{member}");
    }
}

/// The class of the null-member finding: every v3 member whose presence the
/// schema's `required`, `oneOf` or `not` reads (`service_credential`,
/// `audience`, `principal`, `principals`, `service_accounts`,
/// `token_review`), written as `null` under either profile, alone or beside
/// its valid value's siblings, is refused by the schema and by the reader
/// alike.
#[test]
fn a_null_presence_member_is_refused_by_the_schema_and_the_reader() {
    let root = tempfile::tempdir().unwrap();
    let ca = root.path().join("ca.crt");
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    std::fs::write(&ca, certificate.cert.pem()).unwrap();
    let token = root.path().join("token");
    let account = account_config(
        "https://kubernetes.default.svc",
        &ca,
        &token,
        Path::new("/var/lib/connectors"),
    );
    let bearer = json!({"instance":"leaf","listen":"127.0.0.1:0","service_credential":{"kind":"file","path":token}});
    let v3_schema = schema("urn:connectors:config:v3:service");
    let members = [
        "service_credential",
        "audience",
        "principal",
        "principals",
        "service_accounts",
        "token_review",
    ];
    let mut documents = Vec::new();
    for base in [&account, &bearer] {
        for member in members {
            // Replacing the member's value, or adding it, as null.
            let mut document = base.clone();
            document[member] = Value::Null;
            documents.push(document);
        }
    }
    for document in &documents {
        assert!(
            connectors_sdk::validate(&v3_schema, document).is_err(),
            "the v3 schema admits {document}"
        );
        let refused = match serde_json::from_value::<ServiceConfigV3>(document.clone()) {
            Err(_) => true,
            Ok(read) => read.admission().is_err(),
        };
        assert!(refused, "the reader admits {document}");
    }
}

/// Adversary (D275 UA): the v3 schema and the reader agree on every account
/// form, including a member present as `null`. A YAML document writes one as
/// an empty value (`principals:`), which `read_config` reads as absent while
/// the schema's `required` counts it as present.
#[test]
fn the_schema_and_the_reader_agree_on_a_null_account_member() {
    let root = tempfile::tempdir().unwrap();
    let ca = root.path().join("ca.crt");
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    std::fs::write(&ca, certificate.cert.pem()).unwrap();
    let mut base = account_config(
        "https://kubernetes.default.svc",
        &ca,
        &root.path().join("token"),
        Path::new("/var/lib/connectors"),
    );
    base.as_object_mut().unwrap().remove("principal");
    let v3_schema = schema("urn:connectors:config:v3:service");
    let with = |members: &[(&str, Value)]| {
        let mut document = base.clone();
        for (member, value) in members {
            document[*member] = value.clone();
        }
        document
    };
    let token = root.path().join("token");
    let bearer = |member: &str| {
        let mut document = json!({"instance":"leaf","listen":"127.0.0.1:0","service_credential":{"kind":"file","path":token}});
        document[member] = Value::Null;
        document
    };
    let pattern = json!({"namespace": "agents", "name": "*-agent"});
    let documents = [
        with(&[("principals", Value::Null)]),
        with(&[("service_accounts", Value::Null)]),
        with(&[
            ("principal", Value::Null),
            ("principals", json!([PRINCIPAL])),
        ]),
        with(&[
            ("principal", Value::Null),
            ("service_accounts", pattern.clone()),
        ]),
        with(&[("principals", Value::Null), ("principal", json!(PRINCIPAL))]),
        with(&[
            ("service_accounts", Value::Null),
            ("principal", json!(PRINCIPAL)),
        ]),
        with(&[("principals", Value::Null), ("service_accounts", pattern)]),
        bearer("principals"),
        bearer("service_accounts"),
    ];
    let mut disagreements = Vec::new();
    for document in &documents {
        let schema_admits = connectors_sdk::validate(&v3_schema, document).is_ok();
        let reader_admits = serde_json::from_value::<ServiceConfigV3>(document.clone())
            .map(|read| read.admission().is_ok())
            .unwrap_or(false);
        if schema_admits != reader_admits {
            disagreements.push(format!(
                "schema {} / reader {}: {document}",
                if schema_admits { "admits" } else { "refuses" },
                if reader_admits { "admits" } else { "refuses" },
            ));
        }
    }
    assert!(
        disagreements.is_empty(),
        "the v3 schema and the reader disagree on {} documents:\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
}

/// Adversary (D275 UA): the schema and the reader agree on a namespace, a
/// name pattern and a listed account that end in a newline or hold a
/// non-ASCII letter, which a `$`-anchored pattern or a char-counted
/// `maxLength` could let through on one side only.
#[test]
fn the_schema_and_the_reader_agree_on_trailing_and_unicode_account_values() {
    let root = tempfile::tempdir().unwrap();
    let ca = root.path().join("ca.crt");
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    std::fs::write(&ca, certificate.cert.pem()).unwrap();
    let mut base = account_config(
        "https://kubernetes.default.svc",
        &ca,
        &root.path().join("token"),
        Path::new("/var/lib/connectors"),
    );
    base.as_object_mut().unwrap().remove("principal");
    let v3_schema = schema("urn:connectors:config:v3:service");
    let with = |member: &str, value: Value| {
        let mut document = base.clone();
        document[member] = value;
        document
    };
    let documents = [
        with(
            "service_accounts",
            json!({"namespace": "agents\n", "name": "*"}),
        ),
        with(
            "service_accounts",
            json!({"namespace": "agents", "name": "*\n"}),
        ),
        with(
            "service_accounts",
            json!({"namespace": "agents", "name": "agent-\u{e4}*"}),
        ),
        with(
            "service_accounts",
            json!({"namespace": "\u{e4}gents", "name": "*"}),
        ),
        with("principals", json!([format!("{PRINCIPAL}\n")])),
        with("principals", json!([format!("{PRINCIPAL}\u{e4}")])),
    ];
    for document in &documents {
        assert!(
            connectors_sdk::validate(&v3_schema, document).is_err(),
            "the v3 schema admits {document}"
        );
        let read: ServiceConfigV3 = serde_json::from_value(document.clone()).unwrap();
        assert!(read.admission().is_err(), "the reader admits {document}");
    }
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

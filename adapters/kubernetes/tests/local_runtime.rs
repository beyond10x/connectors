//! Deterministic private-runtime journey against a local TLS fixture cluster.
//! No real Kubernetes cluster is contacted and no real credential is used.
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure},
};
use connectors_sdk::Secret;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::oneshot,
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{self, pki_types::PrivatePkcs8KeyDer},
};

#[path = "local_runtime/cli_journey.rs"]
mod cli_journey;

/// Fictional fixture material only.
const TOKEN_ONE: &str = "fixture-kubernetes-token-one";
const TOKEN_TWO: &str = "fixture-kubernetes-token-two";
const USER_ONE: &str = "system:serviceaccount:fixture:reader-one";
const USER_TWO: &str = "system:serviceaccount:fixture:reader-two";
/// A cluster may configure the anonymous principal's name. The group is what
/// stays constant, so the fixture can rename the principal and keep the group.
const RENAMED_ANONYMOUS: &str = "unauthenticated-user";

struct Cluster {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    ca: PathBuf,
    pem: String,
    config: PathBuf,
    calls: Arc<Mutex<Vec<String>>>,
    methods: Arc<Mutex<Vec<String>>>,
    /// 0 off, 1 the well-known `system:anonymous`, 2 a renamed principal that
    /// keeps only the `system:unauthenticated` group.
    anonymous: Arc<std::sync::atomic::AtomicU8>,
    /// Non-zero forces this status on the review endpoint only.
    review_status: Arc<std::sync::atomic::AtomicU16>,
    pause: Arc<std::sync::atomic::AtomicBool>,
}

fn pods(count: usize, continuation: &str) -> Value {
    let items: Vec<Value> = (0..count)
        .map(|index| json!({"metadata":{"uid":format!("pod-uid-{index}"),"name":format!("pod-{index}"),"resourceVersion":"41"}}))
        .collect();
    json!({"metadata":{"resourceVersion":"41","continue":continuation},"items":items})
}

/// A Service with no ready backends: Kubernetes serialises both collections as
/// an explicit null. Observed on v1.31.5. It must read as no observations.
fn empty_slice() -> Value {
    json!({"metadata":{"resourceVersion":"44","continue":""},"items":[{
        "metadata":{"uid":"slice-uid-empty","name":"api-empty","resourceVersion":"44"},
        "ports": null,
        "endpoints": null
    }]})
}

fn slices() -> Value {
    json!({"metadata":{"resourceVersion":"42","continue":""},"items":[{
        "metadata":{"uid":"slice-uid-1","name":"api-abc","labels":{"kubernetes.io/service-name":"api"},"resourceVersion":"42"},
        "ports":[{"port":8080,"protocol":"TCP","appProtocol":"http"}],
        "endpoints":[{"addresses":["10.0.0.7"],"conditions":{"ready":true}}]
    }]})
}

fn nodes() -> Value {
    json!({"metadata":{"resourceVersion":"43","continue":""},"items":[{
        "metadata":{"uid":"node-uid-1","name":"node-1","resourceVersion":"43"},
        "status":{"addresses":[{"type":"InternalIP","address":"10.0.0.2"}],"conditions":[{"type":"Ready","status":"True"}]}
    }]})
}

impl Cluster {
    fn new(discover_hosts: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("private");
        filesystem::directory(&directory, true, true).unwrap();
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let pem = cert.cert.pem();
        let ca = directory.join("ca.pem");
        private(&ca, pem.as_bytes());
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert.cert.der().clone()],
            PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()).into(),
        )
        .unwrap();
        let (address_tx, address_rx) = std::sync::mpsc::channel();
        let (stop, mut stopped) = oneshot::channel();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let observed = calls.clone();
        let methods = Arc::new(Mutex::new(Vec::new()));
        let observed_methods = methods.clone();
        let anonymous = Arc::new(std::sync::atomic::AtomicU8::new(0));
        let anonymous_mode = anonymous.clone();
        let review_status = Arc::new(std::sync::atomic::AtomicU16::new(0));
        let forced_review = review_status.clone();
        let pause = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let paused = pause.clone();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                address_tx.send(listener.local_addr().unwrap()).unwrap();
                let acceptor = TlsAcceptor::from(Arc::new(tls));
                loop {
                    let stream = tokio::select! {_=&mut stopped=>break,value=listener.accept()=>value.unwrap().0};
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        continue;
                    };
                    let mut header = Vec::new();
                    loop {
                        if header.ends_with(b"\r\n\r\n") {
                            break;
                        }
                        assert!(header.len() < 8192);
                        match stream.read_u8().await {
                            Ok(byte) => header.push(byte),
                            Err(_) => break,
                        }
                    }
                    let request = String::from_utf8(header).unwrap();
                    let method = request.split_whitespace().next().unwrap_or_default().to_owned();
                    observed_methods.lock().unwrap().push(method.clone());
                    let path = request.split_whitespace().nth(1).unwrap_or_default().to_owned();
                    let route = path.split('?').next().unwrap().to_owned();
                    let content_length = request
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                                .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    assert!(content_length <= 4096);
                    let mut body = vec![0; content_length];
                    stream.read_exact(&mut body).await.unwrap();
                    let credential = request.lines().find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                            .map(|(_, value)| value.trim().to_owned())
                    });
                    // Only fictional fixture material is accepted. Raw headers
                    // are never retained in observations or diagnostics.
                    let user = match credential.as_deref() {
                        Some(value) if value == format!("Bearer {TOKEN_ONE}") => Some(USER_ONE),
                        Some(value) if value == format!("Bearer {TOKEN_TWO}") => Some(USER_TWO),
                        _ => None,
                    };
                    observed.lock().unwrap().push(path.clone());
                    if paused.load(std::sync::atomic::Ordering::SeqCst) {
                        tokio::select! {_=&mut stopped=>break,_=tokio::time::sleep(Duration::from_secs(2))=>{}}
                    }
                    let (status, payload) = if route
                        == "/apis/authentication.k8s.io/v1/selfsubjectreviews"
                    {
                        assert_eq!(method, "POST");
                        let input: Value = serde_json::from_slice(&body).unwrap();
                        assert_eq!(
                            input,
                            json!({"apiVersion":"authentication.k8s.io/v1","kind":"SelfSubjectReview"})
                        );
                        let forced = forced_review.load(std::sync::atomic::Ordering::SeqCst);
                        if forced != 0 {
                            (forced, json!({"kind":"Status","code":forced}))
                        } else {
                            let (named, groups) =
                                match anonymous_mode.load(std::sync::atomic::Ordering::SeqCst) {
                                    1 => (Some("system:anonymous"), vec!["system:unauthenticated"]),
                                    2 => (
                                        Some(RENAMED_ANONYMOUS),
                                        vec!["system:unauthenticated"],
                                    ),
                                    _ => (user, vec!["system:authenticated"]),
                                };
                            match named {
                                None => (401, json!({"kind":"Status","code":401})),
                                Some(name) => (
                                    201,
                                    json!({"apiVersion":"authentication.k8s.io/v1","kind":"SelfSubjectReview",
                                        "status":{"userInfo":{"username":name,"uid":"fixture-uid","groups":groups}}}),
                                ),
                            }
                        }
                    } else if user.is_none() {
                        (401, json!({"kind":"Status","code":401}))
                    } else if route == "/api/v1/namespaces/fixture/pods" {
                        let second = path.contains("continue=");
                        (200, pods(1, if second { "" } else { "next-page-token" }))
                    } else if route == "/apis/discovery.k8s.io/v1/namespaces/backendless/endpointslices" {
                        (200, empty_slice())
                    } else if route == "/apis/discovery.k8s.io/v1/namespaces/fixture/endpointslices" {
                        (200, slices())
                    } else if route == "/api/v1/nodes" {
                        (200, nodes())
                    } else {
                        (404, json!({"kind":"Status","code":404}))
                    };
                    let body = serde_json::to_vec(&payload).unwrap();
                    let header = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(header.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let config = directory.join("kubernetes.json");
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format":"connectors-kubernetes-local/1",
                "instance":"fixture-kubernetes",
                "api_base":format!("https://localhost:{}/", address.port()),
                "ca_file":ca,
                "namespaces":["backendless","fixture"],
                "resource_kinds":["pods","endpointslices"],
                "discover_hosts":discover_hosts
            }))
            .unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            ca,
            pem,
            config,
            calls,
            methods,
            anonymous,
            review_status,
            pause,
        }
    }
    fn selection(&self) -> Adapter {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-kubernetes"))
            .arg("--local-config")
            .arg(&self.config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "bootstrap inspection failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        bootstrap.validate().unwrap();
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-kubernetes"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture-kubernetes".into(),
            adapter_id: "kubernetes".into(),
            configuration_revision: bootstrap.configuration_revision,
            protocol: "v1alpha1".into(),
            startup: Startup::OnDemand,
            restart: Restart::Never,
            executable: Executable {
                sha256: hex::encode(Sha256::digest(fs::read(&binary).unwrap())),
                path: binary,
                args: vec![
                    "--local-config".into(),
                    self.config.to_str().unwrap().into(),
                ],
            },
        }
    }
    fn count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
    /// Observed paths without their query string. The shared HTTP binding
    /// appends an empty query to a request that carries no parameters.
    fn routes(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|path| path.split('?').next().unwrap_or_default().to_owned())
            .collect()
    }
}
impl Drop for Cluster {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}
fn private(path: &std::path::Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn token(one: bool) -> Secret {
    Secret(serde_json::to_vec(&json!({"token": if one { TOKEN_ONE } else { TOKEN_TWO }})).unwrap())
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
fn invoke(
    child: &mut Child,
    operation: &str,
    partition: &str,
    secret: &Secret,
    input: Value,
) -> Result<Value, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child
        .invoke(
            operation,
            &revision,
            partition,
            secret,
            &serde_json::to_vec(&input).unwrap(),
            deadline(),
        )
        .map(|b| serde_json::from_slice(&b).unwrap())
}

#[test]
fn private_kubernetes_validates_through_selfsubjectreview_and_reads_scoped_resources() {
    let cluster = Cluster::new(false);
    let selection = cluster.selection();
    assert_eq!(cluster.count(), 0);
    let mut child = Child::spawn(&selection).unwrap();
    assert_eq!(cluster.count(), 0);
    // Changing the path after bootstrap cannot change the captured TLS roots.
    private(
        &cluster.ca,
        rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .unwrap()
            .cert
            .pem()
            .as_bytes(),
    );
    let baseline = child
        .validate("kubernetes.token", &token(true), deadline())
        .unwrap_or_else(|failure| {
            panic!(
                "validation {failure:?}; observed paths {:?}",
                cluster.calls.lock().unwrap()
            )
        });
    assert_eq!(baseline.identity.kind, "kubernetes.user");
    assert_eq!(baseline.identity.subject, USER_ONE);
    // Kubernetes issues no scope grant, and no credential expiry was observed.
    assert!(baseline.granted_scopes.is_none());
    assert!(baseline.credential_expires_at_ms.is_none());
    assert_eq!(
        cluster.routes(),
        ["/apis/authentication.k8s.io/v1/selfsubjectreviews"]
    );
    assert_eq!(cluster.methods.lock().unwrap().as_slice(), ["POST"]);
    assert_eq!(
        child
            .validate("kubernetes.token", &token(false), deadline())
            .unwrap()
            .identity
            .subject,
        USER_TWO
    );
    let before = cluster.count();
    assert!(matches!(
        child.validate("unknown", &token(true), deadline()),
        Err(Failure::Unsupported)
    ));
    assert!(matches!(
        child.validate(
            "kubernetes.token",
            &Secret(br#"{"token":"fictional","token":"duplicate"}"#.to_vec()),
            deadline()
        ),
        Err(Failure::InvalidInput)
    ));
    assert_eq!(cluster.count(), before);

    // A namespace outside the configured scope refuses before provider work.
    assert_eq!(
        invoke(
            &mut child,
            "resources.list",
            "one",
            &token(true),
            json!({"namespace":"kube-system","kind":"pods","limit":10})
        ),
        Err(Failure::Forbidden)
    );
    // So does a resource kind the configuration did not select.
    assert_eq!(
        invoke(
            &mut child,
            "resources.list",
            "one",
            &token(true),
            json!({"namespace":"fixture","kind":"services","limit":10})
        ),
        Err(Failure::Forbidden)
    );
    // hosts.discover is not advertised when the configuration disables it.
    assert!(
        !child
            .bootstrap()
            .descriptor()
            .unwrap()
            .operations
            .iter()
            .any(|o| o.id == "hosts.discover")
    );
    assert_eq!(cluster.count(), before);

    let first = invoke(
        &mut child,
        "resources.list",
        "one",
        &token(true),
        json!({"namespace":"fixture","kind":"pods","limit":1}),
    )
    .unwrap();
    assert_eq!(first["items"][0]["metadata"]["name"], "pod-0");
    assert_eq!(first["complete"], false);
    let cursor = first["next_cursor"].as_str().unwrap().to_owned();
    let page = json!({"namespace":"fixture","kind":"pods","limit":1,"cursor":cursor});
    let before = cluster.count();
    // A continuation issued under one connection is unreadable under another.
    assert_eq!(
        invoke(
            &mut child,
            "resources.list",
            "two",
            &token(false),
            page.clone()
        ),
        Err(Failure::StaleCursor)
    );
    assert_eq!(cluster.count(), before);
    assert_eq!(
        invoke(&mut child, "resources.list", "one", &token(true), page).unwrap()["complete"],
        true
    );
    let endpoints = invoke(
        &mut child,
        "endpoints.discover",
        "one",
        &token(true),
        json!({"namespace":"fixture","limit":10}),
    )
    .unwrap();
    assert_eq!(endpoints["items"][0]["address"], "10.0.0.7");
    assert_eq!(endpoints["items"][0]["port"], 8080);
    assert_eq!(endpoints["items"][0]["service"], "api");
    assert_eq!(endpoints["complete"], true);
    assert_eq!(
        invoke(
            &mut child,
            "hosts.discover",
            "one",
            &token(true),
            json!({"limit":10})
        ),
        Err(Failure::NotFound)
    );

    let incarnation = child.incarnation().to_owned();
    assert_eq!(
        child.stop("stale-incarnation"),
        Err(Failure::IncarnationMismatch)
    );
    child.stop(&incarnation).unwrap();
    assert!(matches!(
        child.validate("kubernetes.token", &token(true), deadline()),
        Err(Failure::Unavailable)
    ));
    // The independently computed revision detects changed native configuration.
    assert!(matches!(
        Child::spawn(&selection),
        Err(Failure::ReadinessMismatch)
    ));
    private(&cluster.ca, cluster.pem.as_bytes());
    let mut restored = Child::spawn(&selection).unwrap();
    assert_eq!(
        restored
            .validate("kubernetes.token", &token(true), deadline())
            .unwrap()
            .identity
            .subject,
        USER_ONE
    );
}

#[test]
fn configured_host_discovery_is_advertised_and_reads_nodes() {
    let cluster = Cluster::new(true);
    let mut child = Child::spawn(&cluster.selection()).unwrap();
    assert!(
        child
            .bootstrap()
            .descriptor()
            .unwrap()
            .operations
            .iter()
            .any(|o| o.id == "hosts.discover")
    );
    let hosts = invoke(
        &mut child,
        "hosts.discover",
        "one",
        &token(true),
        json!({"limit":10}),
    )
    .unwrap();
    assert_eq!(hosts["items"][0]["name"], "node-1");
    assert_eq!(hosts["complete"], true);
}

#[test]
fn an_accepted_anonymous_request_is_refused_rather_than_saved_as_an_identity() {
    let cluster = Cluster::new(false);
    let mut child = Child::spawn(&cluster.selection()).unwrap();
    // 1 is the well-known name; 2 renames the principal and keeps only the
    // group, which is what a cluster configuring anonymous auth produces. Both
    // must refuse: keying on the username alone would save an unauthenticated
    // request as an ordinary `kubernetes.user` identity.
    for mode in [1_u8, 2] {
        cluster
            .anonymous
            .store(mode, std::sync::atomic::Ordering::SeqCst);
        assert!(
            matches!(
                child.validate("kubernetes.token", &token(true), deadline()),
                Err(Failure::InvalidCredential)
            ),
            "anonymous mode {mode} was not refused"
        );
    }
}

#[test]
fn review_status_codes_map_to_distinct_failures_without_a_business_read() {
    let cluster = Cluster::new(false);
    let mut child = Child::spawn(&cluster.selection()).unwrap();
    // 403 is a cluster whose RBAC forbids the review itself; it is separated
    // from an unusable credential and from a transient cluster failure.
    for (status, expected) in [
        (403_u16, Failure::Forbidden),
        (429, Failure::Unavailable),
        (500, Failure::Unavailable),
        (503, Failure::Unavailable),
        (599, Failure::Unavailable),
        // Outside every interpreted range: a response, not a classification.
        (418, Failure::Protocol),
        (302, Failure::Protocol),
    ] {
        cluster
            .review_status
            .store(status, std::sync::atomic::Ordering::SeqCst);
        let result = child.validate("kubernetes.token", &token(true), deadline());
        assert!(
            matches!(&result, Err(failure) if *failure == expected),
            "status {status} gave {:?}, expected {expected:?}",
            result.err()
        );
    }
    cluster
        .review_status
        .store(0, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        child
            .validate("kubernetes.token", &token(true), deadline())
            .unwrap()
            .identity
            .subject,
        USER_ONE
    );
    // Every observed call was the review probe; no business read was attempted.
    assert!(
        cluster
            .routes()
            .iter()
            .all(|route| route == "/apis/authentication.k8s.io/v1/selfsubjectreviews"),
        "unexpected routes {:?}",
        cluster.routes()
    );
}

#[test]
fn an_unaccepted_credential_refuses_before_any_business_read() {
    let cluster = Cluster::new(false);
    let mut child = Child::spawn(&cluster.selection()).unwrap();
    assert!(matches!(
        child.validate(
            "kubernetes.token",
            &Secret(br#"{"token":"fixture-unaccepted"}"#.to_vec()),
            deadline()
        ),
        Err(Failure::InvalidCredential)
    ));
    assert_eq!(
        cluster.routes(),
        ["/apis/authentication.k8s.io/v1/selfsubjectreviews"]
    );
}

#[test]
fn bootstrap_mismatches_and_changed_artifacts_refuse_before_provider_work() {
    let cluster = Cluster::new(false);
    let selection = cluster.selection();
    let mut wrong = selection.clone();
    wrong.instance_id = "wrong-instance".into();
    assert!(matches!(
        Child::spawn(&wrong),
        Err(Failure::ReadinessMismatch)
    ));
    let mut wrong = selection.clone();
    wrong.configuration_revision = "wrong-revision".into();
    assert!(matches!(
        Child::spawn(&wrong),
        Err(Failure::ReadinessMismatch)
    ));
    let mut wrong = selection.clone();
    wrong.executable.sha256 = "0".repeat(64);
    assert!(matches!(
        Child::spawn(&wrong),
        Err(Failure::InvalidConfiguration)
    ));
    let script = cluster.root.path().join("private/script");
    private(&script, b"#!/bin/sh\nexit 0\n");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    wrong.executable.path = script;
    wrong.executable.sha256 = hex::encode(Sha256::digest(b"#!/bin/sh\nexit 0\n"));
    assert!(matches!(
        Child::spawn(&wrong),
        Err(Failure::InvalidConfiguration)
    ));
    assert_eq!(cluster.count(), 0);
}

#[test]
fn lost_validation_response_closes_the_owned_channel_without_replay() {
    let cluster = Cluster::new(false);
    let mut child = Child::spawn(&cluster.selection()).unwrap();
    cluster
        .pause
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let result = child.validate(
        "kubernetes.token",
        &token(true),
        connectors_sdk::now_ms() + 1000,
    );
    // The peer may hit its same original deadline and close before the parent's
    // socket timeout. Both observations must close ownership and prohibit replay.
    assert!(
        matches!(result, Err(Failure::Timeout | Failure::Unavailable)),
        "failure {:?}",
        result.err()
    );
    assert_eq!(cluster.count(), 1);
    assert!(matches!(
        child.validate("kubernetes.token", &token(true), deadline()),
        Err(Failure::Unavailable)
    ));
    assert_eq!(cluster.count(), 1);
}

#[test]
fn a_backendless_endpointslice_reads_as_no_observations_rather_than_malformed() {
    let cluster = Cluster::new(false);
    let mut child = Child::spawn(&cluster.selection()).unwrap();
    // The `backendless` namespace returns null for both collections, which is
    // what a real Service with no ready backends produces.
    let page = invoke(
        &mut child,
        "endpoints.discover",
        "one",
        &token(true),
        json!({"namespace":"backendless","limit":10}),
    )
    .unwrap_or_else(|failure| {
        panic!("a null collection must not be a protocol error: {failure:?}")
    });
    assert_eq!(page["items"].as_array().map(|i| i.len()), Some(0));
    assert_eq!(page["complete"], true);
}

//! The Prometheus executable through the host's private runtime, against a local TLS
//! fixture serving the recorded answers in `tests/fixtures` (Prometheus HTTP
//! API shapes). No real Prometheus is contacted and no real credential is used.
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Effect, Failure},
};
use connectors_sdk::Secret;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU16, Ordering},
    },
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
const TOKEN_ONE: &str = "fixture-prometheus-token-one";
const TOKEN_TWO: &str = "fixture-prometheus-token-two";
const INSTANCE: &str = "fixture-prometheus";
const START: u64 = 1_788_822_000;
const END: u64 = 1_788_825_600;
const PROMQL: &str = r#"sum by (status) (rate(http_requests_total[5m]))"#;
const PROBE: &str = "/api/v1/status/buildinfo";

fn fixture(name: &str) -> Vec<u8> {
    fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// One received request: method, target with its query, and authorization.
#[derive(Clone, Debug)]
struct Call {
    method: String,
    target: String,
    authorization: String,
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    config: PathBuf,
    calls: Arc<Mutex<Vec<Call>>>,
    /// Non-zero forces this status on the identity probe only.
    probe_status: Arc<AtomicU16>,
}

/// The answer to one request: the recorded fixture its route selects. Prometheus is
/// served below `prefix` (`/` for a direct Prometheus); a request outside it is not found.
fn answer(
    method: &str,
    target: &str,
    authorization: &str,
    probe: u16,
    prefix: &str,
) -> (u16, Vec<u8>) {
    if authorization != format!("Bearer {TOKEN_ONE}")
        && authorization != format!("Bearer {TOKEN_TWO}")
    {
        return (401, b"authentication failure\n".to_vec());
    }
    let Some(target) = target.strip_prefix(prefix).map(|rest| format!("/{rest}")) else {
        return (404, b"404 page not found\n".to_vec());
    };
    let route = target.split('?').next().unwrap_or_default();
    match (method, route) {
        ("GET", PROBE) if probe != 0 => (probe, b"{}".to_vec()),
        ("GET", PROBE) => (200, fixture("buildinfo.json")),
        ("GET", "/api/v1/query") => (200, fixture("query_vector.json")),
        ("GET", "/api/v1/query_range") => (200, fixture("query_range_matrix.json")),
        ("GET", "/api/v1/rules") => (200, fixture("rules.json")),
        _ => (404, b"404 page not found\n".to_vec()),
    }
}

impl Provider {
    fn new() -> Self {
        Self::under("/")
    }
    /// A Prometheus whose HTTP API is served below `prefix`, as Grafana's data-source proxy
    /// serves it; `base_url` carries the prefix.
    fn under(prefix: &'static str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("private");
        filesystem::directory(&directory, true, true).unwrap();
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = directory.join("ca.pem");
        private(&ca, cert.cert.pem().as_bytes());
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
        let probe_status = Arc::new(AtomicU16::new(0));
        let forced = probe_status.clone();
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
                    while !header.ends_with(b"\r\n\r\n") {
                        assert!(header.len() < 16 * 1024, "fixture header bound");
                        match stream.read_u8().await {
                            Ok(byte) => header.push(byte),
                            Err(_) => break,
                        }
                    }
                    let request = String::from_utf8(header).unwrap();
                    let mut line = request.split_whitespace();
                    let method = line.next().unwrap_or_default().to_owned();
                    let target = line.next().unwrap_or_default().to_owned();
                    let authorization = request
                        .lines()
                        .filter_map(|l| l.split_once(':'))
                        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                        .map(|(_, value)| value.trim().to_owned())
                        .unwrap_or_default();
                    let (status, body) =
                        answer(
                        &method,
                        &target,
                        &authorization,
                        forced.load(Ordering::SeqCst),
                        prefix,
                    );
                    observed.lock().unwrap().push(Call {
                        method,
                        target,
                        authorization,
                    });
                    let head = format!(
                        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
                    let _ = stream.shutdown().await;
                }
            });
        });
        let port = address_rx.recv().unwrap().port();
        let config = directory.join("prometheus.json");
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format": "connectors-prometheus-local/1",
                "instance": INSTANCE,
                "base_url": format!("https://localhost:{port}{prefix}"),
                "ca_file": ca,
                "query_scope": {"allowed_matchers": []},
            }))
            .unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            config,
            calls,
            probe_status,
        }
    }
    fn selection(&self) -> Adapter {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-prometheus"))
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
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-prometheus"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: None,
            permissions: Default::default(),
            instance_id: INSTANCE.into(),
            adapter_id: "prometheus".into(),
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
    /// Observed paths without their query string.
    fn routes(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.target.split('?').next().unwrap_or_default().to_owned())
            .collect()
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}
fn private(path: &std::path::Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn token(value: &str) -> Secret {
    Secret(serde_json::to_vec(&json!({ "token": value })).unwrap())
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
fn invoke(child: &mut Child, operation: &str, input: Value) -> Result<Value, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child
        .invoke(
            operation,
            &revision,
            "one",
            &token(TOKEN_ONE),
            &serde_json::to_vec(&input).unwrap(),
            deadline(),
        )
        .map(|b| serde_json::from_slice(&b).unwrap())
}

#[test]
fn the_bootstrap_advertises_the_bearer_profile_and_three_reads() {
    let provider = Provider::new();
    let child = Child::spawn(&provider.selection()).unwrap();
    let bootstrap = child.bootstrap();
    assert_eq!(bootstrap.adapter, "prometheus");
    assert_eq!(bootstrap.instance, INSTANCE);
    let profiles: Vec<&str> = bootstrap.profiles.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(profiles, ["prometheus.bearer"]);
    let mut operations: Vec<(&str, bool)> = bootstrap
        .requirements
        .iter()
        .map(|r| (r.operation.as_str(), r.effect == Effect::Read))
        .collect();
    operations.sort();
    assert_eq!(
        operations,
        [
            ("rules.list", true),
            ("series.query", true),
            ("series.query_range", true)
        ]
    );
    // Spawning and describing perform no provider work.
    assert_eq!(provider.count(), 0);
}

#[test]
fn a_bearer_token_is_proved_by_the_buildinfo_probe_and_named_by_the_configured_instance() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline = child
        .validate("prometheus.bearer", &token(TOKEN_ONE), deadline())
        .unwrap_or_else(|f| {
            panic!(
                "validation {f:?}; calls {:?}",
                provider.calls.lock().unwrap()
            )
        });
    assert_eq!(baseline.identity.kind, "prometheus.connection");
    assert_eq!(baseline.identity.subject, INSTANCE);
    assert!(baseline.granted_scopes.is_none());
    assert!(baseline.credential_expires_at_ms.is_none());
    assert_eq!(baseline.valid_until_ms - baseline.collected_at_ms, 60_000);
    assert_eq!(provider.routes(), [PROBE]);
    {
        let calls = provider.calls.lock().unwrap();
        assert_eq!(calls[0].method, "GET");
        assert_eq!(calls[0].authorization, format!("Bearer {TOKEN_ONE}"));
    }
    // Another accepted token is the same identity: the subject is the
    // configured connection, not a Prometheus account.
    assert_eq!(
        child
            .validate("prometheus.bearer", &token(TOKEN_TWO), deadline())
            .unwrap()
            .identity
            .subject,
        INSTANCE
    );
    // An unaccepted token is an invalid credential, with no business read.
    assert!(matches!(
        child.validate(
            "prometheus.bearer",
            &token("fixture-unaccepted"),
            deadline()
        ),
        Err(Failure::InvalidCredential)
    ));
    for (status, expected) in [
        (403, Failure::InvalidCredential),
        (429, Failure::Unavailable),
        (503, Failure::Unavailable),
        (404, Failure::Protocol),
    ] {
        provider.probe_status.store(status, Ordering::SeqCst);
        let result = child.validate("prometheus.bearer", &token(TOKEN_ONE), deadline());
        assert!(
            matches!(&result, Err(failure) if *failure == expected),
            "status {status}: {:?}",
            result.err()
        );
    }
    provider.probe_status.store(0, Ordering::SeqCst);
    assert!(provider.routes().iter().all(|r| r == PROBE));
    // An undeclared profile and a malformed entry refuse before provider work.
    let before = provider.count();
    assert!(matches!(
        child.validate("prometheus.anonymous", &token(TOKEN_ONE), deadline()),
        Err(Failure::Unsupported)
    ));
    for entry in [
        br#"{"token":""}"#.as_slice(),
        br#"{"token":"a","token":"b"}"#,
        br#"{"api_key":"fixture"}"#,
        b"not json",
    ] {
        assert!(matches!(
            child.validate("prometheus.bearer", &Secret(entry.to_vec()), deadline()),
            Err(Failure::InvalidInput)
        ));
    }
    assert_eq!(provider.count(), before);
}

/// Every read once on one child; the routes it sends, below `prefix`.
fn each_read(provider: &Provider, prefix: &str) {
    let mut child = Child::spawn(&provider.selection()).unwrap();
    child
        .validate("prometheus.bearer", &token(TOKEN_ONE), deadline())
        .unwrap();

    let instant = invoke(
        &mut child,
        "series.query",
        json!({"query": "up", "time_unix_s": END}),
    )
    .unwrap();
    assert_eq!(instant["result_type"], "vector", "{instant}");
    assert_eq!(
        instant["series"][1]["samples"][0],
        json!([1788825600.25, "NaN"])
    );
    assert_eq!(instant["provenance"]["instance"], INSTANCE);

    let range = invoke(
        &mut child,
        "series.query_range",
        json!({"query": PROMQL, "start_unix_s": START, "end_unix_s": END, "step_s": 60}),
    )
    .unwrap();
    assert_eq!(range["result_type"], "matrix");
    assert_eq!(range["series"][0]["labels"], json!({"status": "200"}));
    assert_eq!(
        range["series"][0]["samples"][2],
        json!([1788822120, "+Inf"])
    );
    assert_eq!(range["complete"], true);

    let rules = invoke(&mut child, "rules.list", json!({"kind": "alerting"})).unwrap();
    assert_eq!(rules["items"][0]["name"], "ApiDown");
    assert_eq!(rules["items"][0]["state"], "firing");
    assert_eq!(rules["items"].as_array().unwrap().len(), 2);

    let base = prefix.trim_end_matches('/');
    assert_eq!(
        provider.routes(),
        [
            format!("{base}{PROBE}"),
            format!("{base}/api/v1/query"),
            format!("{base}/api/v1/query_range"),
            format!("{base}/api/v1/rules"),
        ]
    );
    assert!(
        provider
            .calls
            .lock()
            .unwrap()
            .iter()
            .all(|c| c.method == "GET" && c.authorization == format!("Bearer {TOKEN_ONE}"))
    );
    // An input the profile does not admit refuses before provider work.
    let before = provider.count();
    assert_eq!(
        invoke(
            &mut child,
            "series.query_range",
            json!({"query": PROMQL, "start_unix_s": START, "end_unix_s": END, "step_s": 1}),
        ),
        Err(Failure::InvalidInput)
    );
    assert_eq!(provider.count(), before);
}

#[test]
fn each_read_answers_through_the_private_runtime_from_the_recorded_answers() {
    each_read(&Provider::new(), "/");
}

#[test]
fn a_changed_configuration_refuses_the_cached_selection_before_provider_work() {
    let provider = Provider::new();
    let selection = provider.selection();
    let mut wrong = selection.clone();
    wrong.instance_id = "wrong-instance".into();
    assert!(matches!(
        Child::spawn(&wrong),
        Err(Failure::ReadinessMismatch)
    ));
    // Replacing the trusted CA changes the configuration revision.
    let ca = provider.root.path().join("private/ca.pem");
    private(
        &ca,
        rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .unwrap()
            .cert
            .pem()
            .as_bytes(),
    );
    assert!(matches!(
        Child::spawn(&selection),
        Err(Failure::ReadinessMismatch)
    ));
    assert_eq!(provider.count(), 0);
}

/// Prometheus through Grafana (`grafana.prometheus.query`, `.range`, `.rules`):
/// Grafana's data-source proxy serves a Prometheus data source's HTTP API below
/// `/api/datasources/proxy/uid/<uid>/` and authenticates the Grafana
/// service-account token itself. A Prometheus connection whose `base_url` is that
/// proxy path sends its probe and every read below the prefix, with the token as
/// its bearer.
#[test]
fn a_connection_through_the_grafana_datasource_proxy_reads_below_the_proxy_prefix() {
    const PROXY: &str = "/api/datasources/proxy/uid/P1809F7CD0C75ACF3/";
    each_read(&Provider::under(PROXY), PROXY);
}

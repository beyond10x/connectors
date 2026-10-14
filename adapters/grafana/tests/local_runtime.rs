//! The Grafana executable through the host's private runtime, against a local TLS fixture
//! serving the recorded answer in `tests/fixtures` (Grafana `GET /api/datasources` shape).
//! No real Grafana is contacted and no real credential is used.
#![cfg(unix)]
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
const TOKEN_ONE: &str = "fixture-grafana-token-one";
const TOKEN_TWO: &str = "fixture-grafana-token-two";
const INSTANCE: &str = "fixture-grafana";

fn fixture(name: &str) -> Vec<u8> {
    fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// One received request: method, target with its query, authorization, the stack header
/// (empty when absent) and the body.
#[derive(Clone, Debug)]
struct Call {
    method: String,
    target: String,
    authorization: String,
    stack: String,
    body: Vec<u8>,
}

/// The IRM API URL's path on the fixture server.
const IRM_PREFIX: &str = "/oncall/";
const QUERY_INCIDENTS: &str =
    "/api/plugins/grafana-irm-app/resources/api/v1/IncidentsService.QueryIncidents";

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    config: PathBuf,
    calls: Arc<Mutex<Vec<Call>>>,
    /// Non-zero forces this status on `GET /api/datasources`.
    probe_status: Arc<AtomicU16>,
    /// Non-zero forces this status on the IRM probe.
    irm_probe_status: Arc<AtomicU16>,
}

/// The answer to one request: the recorded fixture its route selects.
fn answer(
    method: &str,
    target: &str,
    authorization: &str,
    probe: u16,
    irm_probe: u16,
) -> (u16, Vec<u8>) {
    if authorization != format!("Bearer {TOKEN_ONE}")
        && authorization != format!("Bearer {TOKEN_TWO}")
    {
        return (401, br#"{"message":"Unauthorized"}"#.to_vec());
    }
    let (route, query) = target.split_once('?').unwrap_or((target, ""));
    let alert_groups = format!("{IRM_PREFIX}api/v1/alert_groups/");
    match (method, route) {
        ("GET", "/api/datasources") if probe != 0 => (probe, b"{}".to_vec()),
        ("GET", "/api/datasources") => (200, fixture("datasources.json")),
        // The IRM probe: the fixed one-second window, one record per page.
        ("GET", r) if r == alert_groups && query.ends_with("&perpage=1") => {
            if irm_probe != 0 {
                (irm_probe, b"{}".to_vec())
            } else {
                (
                    200,
                    br#"{"count":0,"next":null,"previous":null,"results":[]}"#.to_vec(),
                )
            }
        }
        ("GET", r) if r == alert_groups => (200, fixture("alert_groups_page2.json")),
        ("POST", QUERY_INCIDENTS) => (200, fixture("incidents_page2.json")),
        _ => (404, br#"{"message":"Not found"}"#.to_vec()),
    }
}

impl Provider {
    /// A stack with IRM: the configuration names `irm_api_url` on the same fixture server.
    fn new() -> Self {
        Self::with_irm(true)
    }
    fn with_irm(irm: bool) -> Self {
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
        let irm_probe_status = Arc::new(AtomicU16::new(0));
        let irm_forced = irm_probe_status.clone();
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
                    let named = |wanted: &str| {
                        request
                            .lines()
                            .filter_map(|l| l.split_once(':'))
                            .find(|(name, _)| name.eq_ignore_ascii_case(wanted))
                            .map(|(_, value)| value.trim().to_owned())
                            .unwrap_or_default()
                    };
                    let authorization = named("authorization");
                    let stack = named("x-grafana-url");
                    let length: usize = named("content-length").parse().unwrap_or(0);
                    assert!(length <= 64 * 1024, "fixture body bound");
                    let mut body = vec![0; length];
                    if stream.read_exact(&mut body).await.is_err() {
                        continue;
                    }
                    let (status, answered) = answer(
                        &method,
                        &target,
                        &authorization,
                        forced.load(Ordering::SeqCst),
                        irm_forced.load(Ordering::SeqCst),
                    );
                    observed.lock().unwrap().push(Call {
                        method,
                        target,
                        authorization,
                        stack,
                        body,
                    });
                    let head = format!(
                        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answered.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&answered).await;
                    let _ = stream.shutdown().await;
                }
            });
        });
        let port = address_rx.recv().unwrap().port();
        let config = directory.join("grafana.json");
        let mut document = json!({
            "format": "connectors-grafana-local/1",
            "instance": INSTANCE,
            "base_url": format!("https://localhost:{port}/"),
            "ca_file": ca,
        });
        if irm {
            document["irm_api_url"] = json!(format!("https://localhost:{port}{IRM_PREFIX}"));
        }
        private(&config, &serde_json::to_vec(&document).unwrap());
        Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            config,
            calls,
            probe_status,
            irm_probe_status,
        }
    }
    fn selection(&self) -> Adapter {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-grafana"))
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
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-grafana"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: None,
            permissions: Default::default(),
            instance_id: INSTANCE.into(),
            adapter_id: "grafana".into(),
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
    /// The configured stack URL as the IRM API is told it: without its final `/`.
    fn stack_url(&self) -> String {
        let document: Value = serde_json::from_slice(&fs::read(&self.config).unwrap()).unwrap();
        document["base_url"]
            .as_str()
            .unwrap()
            .trim_end_matches('/')
            .to_owned()
    }
    fn count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
    /// Observed targets.
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
fn the_bootstrap_advertises_both_service_account_profiles_and_only_reads() {
    // With and without the IRM API URL the surface is the same; only what answers differs.
    for irm in [true, false] {
        let provider = Provider::with_irm(irm);
        let child = Child::spawn(&provider.selection()).unwrap();
        let bootstrap = child.bootstrap();
        assert_eq!(bootstrap.adapter, "grafana");
        assert_eq!(bootstrap.instance, INSTANCE);
        let profiles: Vec<&str> = bootstrap.profiles.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            profiles,
            ["grafana.service_account", "grafana.irm_service_account"]
        );
        let operations: Vec<(&str, &str, bool)> = bootstrap
            .requirements
            .iter()
            .map(|r| {
                (
                    r.operation.as_str(),
                    r.profile.as_str(),
                    r.effect == Effect::Read,
                )
            })
            .collect();
        assert_eq!(
            operations,
            [
                ("datasources.list", "grafana.service_account", true),
                ("irm.alert_groups.list", "grafana.irm_service_account", true),
                ("irm.alert_group.get", "grafana.irm_service_account", true),
                ("irm.incidents.list", "grafana.irm_service_account", true),
                ("irm.incident.get", "grafana.irm_service_account", true),
            ]
        );
        // Spawning and describing perform no provider work.
        assert_eq!(provider.count(), 0);
    }
}

#[test]
fn a_service_account_token_is_proved_by_the_datasources_probe_and_named_by_the_configured_instance()
{
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline = child
        .validate("grafana.service_account", &token(TOKEN_ONE), deadline())
        .unwrap_or_else(|f| {
            panic!(
                "validation {f:?}; calls {:?}",
                provider.calls.lock().unwrap()
            )
        });
    assert_eq!(baseline.identity.kind, "grafana.connection");
    assert_eq!(baseline.identity.subject, INSTANCE);
    assert!(baseline.granted_scopes.is_none());
    assert!(baseline.credential_expires_at_ms.is_none());
    assert_eq!(baseline.valid_until_ms - baseline.collected_at_ms, 60_000);
    assert_eq!(provider.routes(), ["/api/datasources"]);
    {
        let calls = provider.calls.lock().unwrap();
        assert_eq!(calls[0].method, "GET");
        assert_eq!(calls[0].authorization, format!("Bearer {TOKEN_ONE}"));
    }
    // Another accepted token is the same identity: the subject is the configured
    // connection, not a Grafana account.
    assert_eq!(
        child
            .validate("grafana.service_account", &token(TOKEN_TWO), deadline())
            .unwrap()
            .identity
            .subject,
        INSTANCE
    );
    // An unaccepted token is an invalid credential.
    assert!(matches!(
        child.validate(
            "grafana.service_account",
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
        let result = child.validate("grafana.service_account", &token(TOKEN_ONE), deadline());
        assert!(
            matches!(&result, Err(failure) if *failure == expected),
            "status {status}: {:?}",
            result.err()
        );
    }
    provider.probe_status.store(0, Ordering::SeqCst);
    assert!(provider.routes().iter().all(|r| r == "/api/datasources"));
    // An undeclared profile and a malformed entry refuse before provider work.
    let before = provider.count();
    assert!(matches!(
        child.validate("grafana.anonymous", &token(TOKEN_ONE), deadline()),
        Err(Failure::Unsupported)
    ));
    for entry in [
        br#"{"token":""}"#.as_slice(),
        br#"{"token":"a","token":"b"}"#,
        br#"{"api_key":"fixture"}"#,
        b"not json",
    ] {
        assert!(matches!(
            child.validate(
                "grafana.service_account",
                &Secret(entry.to_vec()),
                deadline()
            ),
            Err(Failure::InvalidInput)
        ));
    }
    assert_eq!(provider.count(), before);
}

#[test]
fn the_datasource_list_answers_through_the_private_runtime_from_the_recorded_answer() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();

    let listed = invoke(&mut child, "datasources.list", json!({})).unwrap();
    assert_eq!(listed["provenance"]["instance"], INSTANCE);
    assert_eq!(listed["provenance"]["resource"], "grafana:datasources");
    assert_eq!(listed["complete"], true);
    assert!(listed["next_cursor"].is_null());
    assert_eq!(
        listed["items"][0],
        json!({"uid": "P8E80F9AEF21F6940", "name": "Loki", "type": "loki",
               "access": "proxy", "is_default": false})
    );
    assert_eq!(listed["items"].as_array().unwrap().len(), 3);
    assert!(!listed.to_string().contains("monitoring.svc"));

    assert_eq!(provider.routes(), ["/api/datasources"]);
    assert!(
        provider
            .calls
            .lock()
            .unwrap()
            .iter()
            .all(|c| c.method == "GET" && c.authorization == format!("Bearer {TOKEN_ONE}"))
    );
    // An input the operation does not admit refuses before provider work.
    let before = provider.count();
    assert_eq!(
        invoke(&mut child, "datasources.list", json!({"type": "loki"})),
        Err(Failure::InvalidInput)
    );
    assert_eq!(provider.count(), before);
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

/// story:grafana-irm-history-reads: a connection made under `grafana.irm_service_account`
/// is proved by the IRM probe, and reads alert groups and incidents through the private
/// runtime from the recorded answers.
#[test]
fn a_saved_irm_connection_reads_alert_groups_and_incidents_through_the_private_runtime() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline = child
        .validate("grafana.irm_service_account", &token(TOKEN_ONE), deadline())
        .unwrap_or_else(|f| {
            panic!(
                "validation {f:?}; calls {:?}",
                provider.calls.lock().unwrap()
            )
        });
    assert_eq!(baseline.identity.kind, "grafana.connection");
    assert_eq!(baseline.identity.subject, INSTANCE);
    assert_eq!(baseline.valid_until_ms - baseline.collected_at_ms, 60_000);
    let stack = provider.stack_url();
    {
        let calls = provider.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].method, "GET");
        assert_eq!(
            calls[0].target,
            "/oncall/api/v1/alert_groups/?started_at=2000-01-01T00%3A00%3A00_2000-01-01T00%3A00%3A01&perpage=1"
        );
        assert_eq!(calls[0].authorization, format!("Bearer {TOKEN_ONE}"));
        assert_eq!(calls[0].stack, stack);
    }
    for (status, expected) in [
        (401, Failure::InvalidCredential),
        (403, Failure::InvalidCredential),
        (429, Failure::Unavailable),
        (503, Failure::Unavailable),
        (404, Failure::Protocol),
    ] {
        provider.irm_probe_status.store(status, Ordering::SeqCst);
        let result = child.validate("grafana.irm_service_account", &token(TOKEN_ONE), deadline());
        assert!(
            matches!(&result, Err(failure) if *failure == expected),
            "status {status}: {:?}",
            result.err()
        );
    }
    provider.irm_probe_status.store(0, Ordering::SeqCst);

    let before = provider.count();
    let groups = invoke(
        &mut child,
        "irm.alert_groups.list",
        json!({"since": "2026-01-01T00:00:00Z", "until": "2026-01-02T00:00:00Z"}),
    )
    .unwrap();
    assert_eq!(groups["provenance"]["instance"], INSTANCE);
    assert_eq!(groups["provenance"]["resource"], "grafana:irm.alert_groups");
    assert_eq!(groups["complete"], true);
    assert_eq!(groups["items"][0]["id"], "I9LPV30QXJ5KM");
    assert!(!groups.to_string().contains("fixture-private"));
    let incidents = invoke(
        &mut child,
        "irm.incidents.list",
        json!({"since": "2026-01-01T00:00:00Z", "until": "2026-01-02T00:00:00Z"}),
    )
    .unwrap();
    assert_eq!(incidents["provenance"]["resource"], "grafana:irm.incidents");
    assert_eq!(incidents["items"][0]["incident_id"], "incident-103");
    assert_eq!(
        incidents["links"],
        json!([{"incident_id": "incident-103", "alert_group_id": "I9LPV30QXJ5KM"}])
    );
    let calls = provider.calls.lock().unwrap();
    let read = &calls[before..];
    assert_eq!(read.len(), 2);
    assert_eq!(read[0].method, "GET");
    assert!(
        read[0]
            .target
            .starts_with("/oncall/api/v1/alert_groups/?started_at=")
    );
    assert_eq!(read[0].stack, stack);
    assert_eq!(read[1].method, "POST");
    assert_eq!(read[1].target, format!("{QUERY_INCIDENTS}?"));
    // The incident service is the stack itself; the stack header belongs to the IRM API.
    assert_eq!(read[1].stack, "");
    let body: Value = serde_json::from_slice(&read[1].body).unwrap();
    assert_eq!(body["query"]["dateFrom"], "2026-01-01T00:00:00Z");
    assert!(
        read.iter()
            .all(|c| c.authorization == format!("Bearer {TOKEN_ONE}"))
    );
}

#[test]
fn without_an_irm_api_url_the_irm_profile_and_reads_are_unavailable_before_provider_work() {
    let provider = Provider::with_irm(false);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    assert!(matches!(
        child.validate("grafana.irm_service_account", &token(TOKEN_ONE), deadline()),
        Err(Failure::Unavailable)
    ));
    for (operation, input) in [
        (
            "irm.alert_groups.list",
            json!({"since": "2026-01-01T00:00:00Z", "until": "2026-01-02T00:00:00Z"}),
        ),
        ("irm.alert_group.get", json!({"alert_group_id": "I1"})),
        (
            "irm.incidents.list",
            json!({"since": "2026-01-01T00:00:00Z", "until": "2026-01-02T00:00:00Z"}),
        ),
        ("irm.incident.get", json!({"incident_id": "incident-1"})),
    ] {
        assert_eq!(
            invoke(&mut child, operation, input),
            Err(Failure::Unavailable),
            "{operation}"
        );
    }
    assert_eq!(provider.count(), 0);
    // The data-source read is unchanged.
    invoke(&mut child, "datasources.list", json!({})).unwrap();
    assert_eq!(provider.routes(), ["/api/datasources"]);
}

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

#[path = "local_runtime/ci_provider.rs"]
mod ci_provider;
#[path = "local_runtime/cli_journey.rs"]
mod cli_journey;
#[path = "../../../crates/connectors-host/tests/fixtures/clock/server.rs"]
mod clock_fixture;
#[path = "local_runtime/mr_provider.rs"]
mod mr_provider;

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    ca: PathBuf,
    pem: String,
    config: PathBuf,
    calls: Arc<Mutex<Vec<String>>>,
    methods: Arc<Mutex<Vec<String>>>,
    pause: Arc<std::sync::atomic::AtomicBool>,
    response_status: Arc<std::sync::atomic::AtomicU16>,
    merge_mode: Arc<std::sync::atomic::AtomicU8>,
    merge_effects: Arc<std::sync::atomic::AtomicUsize>,
    /// Incremented once a held merge response is actually waiting for its
    /// release. An observer that waits for this cannot release a hold the
    /// handler has not entered yet.
    merge_held: Arc<std::sync::atomic::AtomicUsize>,
}
impl Provider {
    fn new() -> Self {
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
        let pause = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let paused = pause.clone();
        let response_status = Arc::new(std::sync::atomic::AtomicU16::new(0));
        let forced_status = response_status.clone();
        let merge_mode = Arc::new(std::sync::atomic::AtomicU8::new(0));
        let merge_behavior = merge_mode.clone();
        let merge_effects = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let effects = merge_effects.clone();
        let merge_held = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let held = merge_held.clone();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                address_tx.send(listener.local_addr().unwrap()).unwrap();
                let acceptor=TlsAcceptor::from(Arc::new(tls));
                loop {
                    let stream=tokio::select! {_=&mut stopped=>break,value=listener.accept()=>value.unwrap().0};
                    let Ok(mut stream)=acceptor.accept(stream).await else {continue;};
                    let mut header=Vec::new();
                    loop {
                        if header.ends_with(b"\r\n\r\n") {break;}
                        assert!(header.len()<8192);
                        match stream.read_u8().await {Ok(byte)=>header.push(byte),Err(_)=>break}
                    }
                    let request=String::from_utf8(header).unwrap();
                    // Latch the merge behavior before this request becomes observable.
                    // Recording the method first would let an observer release a hold
                    // between that record and the branch below, so the handler would
                    // answer the released mode instead of the held one.
                    let merge=merge_behavior.load(std::sync::atomic::Ordering::SeqCst);
                    observed_methods.lock().unwrap().push(request.split_whitespace().next().unwrap_or_default().to_owned());
                    let path=request.split_whitespace().nth(1).unwrap_or_default().to_owned();
                    let route=path.split('?').next().unwrap();
                    let content_length=request.lines().find_map(|line|line.split_once(':').filter(|(name,_)|name.eq_ignore_ascii_case("content-length")).map(|(_,value)|value.trim().parse::<usize>().unwrap())).unwrap_or(0);
                    assert!(content_length <= 4096);
                    let mut body = vec![0; content_length];
                    stream.read_exact(&mut body).await.unwrap();
                    let credential=request.lines().find_map(|line|line.split_once(':').filter(|(name,_)|name.eq_ignore_ascii_case("private-token")).map(|(_,value)|value.trim()));
                    // Only fictional fixture material is accepted. Do not retain
                    // raw headers in observations or failure diagnostics.
                    let valid=matches!(credential,Some("fixture-pat-one"|"fixture-pat-two"));
                    let user=if credential==Some("fixture-pat-two") {43} else {42};
                    observed.lock().unwrap().push(path.clone());
                    if route == "/api/v4/projects/org%2Fproject/merge_requests/4" {
                        while merge_behavior.load(std::sync::atomic::Ordering::SeqCst) == 4 {
                            tokio::select! {_=&mut stopped=>return,_=tokio::time::sleep(Duration::from_millis(10))=>{}}
                        }
                    }
                    if paused.load(std::sync::atomic::Ordering::SeqCst) {
                        tokio::select! {_=&mut stopped=>break,_=tokio::time::sleep(Duration::from_secs(2))=>{}}
                    }
                    let forced = forced_status.load(std::sync::atomic::Ordering::SeqCst);
                    if valid && request.starts_with("PUT ") && route=="/api/v4/projects/org%2Fproject/merge_requests/4/merge" {
                        let input: Value = serde_json::from_slice(&body).unwrap();
                        assert_eq!(input, json!({"sha":"0123456789abcdef0123456789abcdef01234567","auto_merge":false,"should_remove_source_branch":false}));
                        let mode = merge;
                        let (status, body) = if mode == 2 || mode == 6 {
                            if mode == 6 {
                                // Hold a known refusal until the test has made host
                                // settlement fail, then answer the same refusal.
                                held.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                while merge_behavior.load(std::sync::atomic::Ordering::SeqCst) == 6 {
                                    tokio::select! {_=&mut stopped=>return,_=tokio::time::sleep(Duration::from_millis(10))=>{}}
                                }
                            }
                            (409, b"head changed".to_vec())
                        } else {
                            effects.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            if mode == 1 { continue; } // effect occurred; response lost
                            if mode == 3 {
                                // Hold the response until the test kills the exact owner.
                                // Drop the stream afterward; never produce a late result.
                                while merge_behavior.load(std::sync::atomic::Ordering::SeqCst) == 3 {
                                    tokio::select! {_=&mut stopped=>return,_=tokio::time::sleep(Duration::from_millis(10))=>{}}
                                }
                                continue;
                            }
                            if mode == 5 {
                                // The effect already happened. Hold its known response
                                // until the test has made host settlement fail.
                                held.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                while merge_behavior.load(std::sync::atomic::Ordering::SeqCst) == 5 {
                                    tokio::select! {_=&mut stopped=>return,_=tokio::time::sleep(Duration::from_millis(10))=>{}}
                                }
                            }
                            let (_, body, _) = mr_provider::reply("/api/v4/projects/org%2Fproject/merge_requests/4", "", &[]).unwrap();
                            let mut item: Value = serde_json::from_slice(&body).unwrap();
                            item["state"] = json!("merged");
                            (200, serde_json::to_vec(&item).unwrap())
                        };
                        let header=format!("HTTP/1.1 {status} fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());
                        let _=stream.write_all(header.as_bytes()).await;
                        let _=stream.write_all(&body).await;
                        continue;
                    }
                    let (status,body,extra)=if forced != 0 {(forced,json!({"error":"fixture override"}),"")}
                        else if !valid {(401,json!({"error":"fixture refusal"}),"")}
                        else if route=="/api/v4/user" {(200,json!({"id":user,"state":"active"}),"")}
                        else if route=="/api/v4/personal_access_tokens/self" {(200,json!({"id":99,"user_id":user,"active":true,"revoked":false,"scopes":["api"],"expires_at":null}),"")}
                        else if path.contains("/issues?") && path.contains("page=2") {(200,json!([]),"x-next-page: \r\n")}
                        else if path.contains("/issues?") {(200,json!([{"id":1,"title":"fixture"}]),"x-next-page: 2\r\n")}
                        else if path.contains("/repository/files/") {(200,json!({"content":"Zml4dHVyZQ==","encoding":"base64","size":7,"last_commit_id":"fixture-commit"}),"")}
                        else {(200,json!({"id":7,"name":"fixture-project"}),"")};
                    let body=serde_json::to_vec(&body).unwrap();
                    let (status,body,extra)=if valid && forced==0 {
                        let calls=observed.lock().unwrap();
                        mr_provider::reply(route,&path,&calls).or_else(||ci_provider::reply(route,&path,&calls)).unwrap_or((status,body,extra))
                    } else { (status,body,extra) };
                    let header=format!("HTTP/1.1 {status} fixture\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n",body.len());
                    let _=stream.write_all(header.as_bytes()).await;
                    let _=stream.write_all(&body).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let config = directory.join("gitlab.json");
        private(&config,&serde_json::to_vec(&json!({"format":"connectors-gitlab-local/1","instance":"fixture-gitlab","api_base":format!("https://localhost:{}/api/v4",address.port()),"ca_file":ca,"allowed_projects":["org/project"]})).unwrap());
        Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            ca,
            pem,
            config,
            calls,
            methods,
            pause,
            response_status,
            merge_mode,
            merge_effects,
            merge_held,
        }
    }
    fn selection(&self) -> Adapter {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-gitlab"))
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
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-gitlab"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture-gitlab".into(),
            adapter_id: "gitlab".into(),
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
fn token(one: bool) -> Secret {
    Secret(if one {
        br#"{"token":"fixture-pat-one"}"#.to_vec()
    } else {
        br#"{"token":"fixture-pat-two"}"#.to_vec()
    })
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
fn private_gitlab_validates_and_reads_with_exact_caps_and_cursor_partitions() {
    let provider = Provider::new();
    let selection = provider.selection();
    assert_eq!(provider.count(), 0);
    let mut child = Child::spawn(&selection).unwrap();
    assert_eq!(provider.count(), 0);
    // Changing the path after bootstrap cannot change the captured TLS roots.
    private(
        &provider.ca,
        rcgen::generate_simple_self_signed(vec!["localhost".into()])
            .unwrap()
            .cert
            .pem()
            .as_bytes(),
    );
    let baseline = child
        .validate("gitlab.pat", &token(true), deadline())
        .unwrap_or_else(|failure| {
            panic!(
                "validation {failure:?}; observed paths {:?}",
                provider.calls.lock().unwrap()
            )
        });
    assert_eq!(baseline.identity.subject, "42");
    assert!(baseline.granted_scopes.unwrap().contains("read_api"));
    assert_eq!(
        child
            .validate("gitlab.pat", &token(false), deadline())
            .unwrap()
            .identity
            .subject,
        "43"
    );
    let before = provider.count();
    assert!(matches!(
        child.validate("unknown", &token(true), deadline()),
        Err(Failure::Unsupported)
    ));
    assert!(matches!(
        child.validate(
            "gitlab.pat",
            &Secret(br#"{"token":"fictional","token":"duplicate"}"#.to_vec()),
            deadline()
        ),
        Err(Failure::InvalidInput)
    ));
    assert_eq!(provider.count(), before);
    assert_eq!(
        invoke(
            &mut child,
            "project.get",
            "one",
            &token(true),
            json!({"project":"outside"})
        ),
        Err(Failure::Forbidden)
    );
    assert_eq!(provider.count(), before);
    assert_eq!(
        invoke(
            &mut child,
            "project.get",
            "one",
            &token(true),
            json!({"project":"org/project"})
        )
        .unwrap()["item"]["id"],
        7
    );
    let first = invoke(
        &mut child,
        "issues.list",
        "one",
        &token(true),
        json!({"project":"org/project","limit":1}),
    )
    .unwrap();
    let cursor = first["next_cursor"].as_str().unwrap();
    let page = json!({"project":"org/project","limit":1,"cursor":cursor});
    let before = provider.count();
    assert_eq!(
        invoke(
            &mut child,
            "issues.list",
            "two",
            &token(false),
            page.clone()
        ),
        Err(Failure::StaleCursor)
    );
    assert_eq!(provider.count(), before);
    assert_eq!(
        invoke(&mut child, "issues.list", "one", &token(true), page).unwrap()["complete"],
        true
    );
    assert_eq!(
        invoke(
            &mut child,
            "file.get",
            "one",
            &token(true),
            json!({"project":"org/project","path":"README.md","ref":"main"})
        )
        .unwrap()["item"]["content"],
        "Zml4dHVyZQ=="
    );
    assert_eq!(
        child.stop("stale-incarnation"),
        Err(Failure::IncarnationMismatch)
    );
    assert_eq!(
        child
            .validate("gitlab.pat", &token(true), deadline())
            .unwrap()
            .identity
            .subject,
        "42"
    );
    let incarnation = child.incarnation().to_owned();
    child.stop(&incarnation).unwrap();
    assert!(matches!(
        child.validate("gitlab.pat", &token(true), deadline()),
        Err(Failure::Unavailable)
    ));
    // The independently computed revision detects changed native configuration.
    assert!(matches!(
        Child::spawn(&selection),
        Err(Failure::ReadinessMismatch)
    ));
    private(&provider.ca, provider.pem.as_bytes());
    let mut restored = Child::spawn(&selection).unwrap();
    assert_eq!(
        restored
            .validate("gitlab.pat", &token(true), deadline())
            .unwrap()
            .identity
            .subject,
        "42"
    );
}

#[test]
fn bootstrap_mismatches_and_changed_artifacts_refuse_before_provider_work() {
    let provider = Provider::new();
    let selection = provider.selection();
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
    let script = provider.root.path().join("private/script");
    private(&script, b"#!/bin/sh\nexit 0\n");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    wrong.executable.path = script;
    wrong.executable.sha256 = hex::encode(Sha256::digest(b"#!/bin/sh\nexit 0\n"));
    assert!(matches!(
        Child::spawn(&wrong),
        Err(Failure::InvalidConfiguration)
    ));
    assert_eq!(provider.count(), 0);
}

#[test]
fn lost_validation_response_closes_the_owned_channel_without_replay() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    provider
        .pause
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let result = child.validate("gitlab.pat", &token(true), connectors_sdk::now_ms() + 1000);
    // The peer may hit its same original deadline and close before the parent's
    // socket timeout. Both observations must close ownership and prohibit replay.
    assert!(
        matches!(result, Err(Failure::Timeout | Failure::Unavailable)),
        "failure {:?}",
        result.err()
    );
    assert_eq!(provider.count(), 1);
    assert!(matches!(
        child.validate("gitlab.pat", &token(true), deadline()),
        Err(Failure::Unavailable)
    ));
    assert_eq!(provider.count(), 1);
}

// ---------------------------------------------------------------------------
// Adversary pass 1, unit 1. Test-only additions; no production source touched.
// ---------------------------------------------------------------------------

/// The failed-settlement journey is the declared delivery of the **Attempt
/// terminal settlement** row of `docs/gitlab-write-failure-matrix.md`. Nothing
/// compares the shipped journey against that row, so this does: the row must
/// not still be written as production CLI evidence that has yet to be produced.
#[test]
fn the_failure_matrix_row_stops_demanding_the_settlement_journey_that_exists() {
    let matrix = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/gitlab-write-failure-matrix.md"
    ))
    .unwrap();
    let journeys = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/local_runtime/guarded_merge.rs"
    ))
    .unwrap();
    assert!(
        journeys.contains(
            "fn gitlab_cli_failed_settlement_keeps_the_known_effect_and_recovers_conservatively"
        ),
        "the journey this row is measured against is not in the fixture"
    );
    let row = matrix
        .lines()
        .find(|line| line.starts_with("| Attempt terminal settlement |"))
        .expect("the matrix must carry an Attempt terminal settlement row");
    let cells: Vec<&str> = row.split('|').map(str::trim).collect();
    assert!(
        !cells[4].starts_with("Fail settlement after a known native Applied/Refused response"),
        "the shipped CLI journey exists, and the row still lists it as remaining \
         production CLI evidence: {}",
        cells[4]
    );
}

/// The mode 8/9 settlement journey releases its held provider response the
/// moment one PUT appears in `methods` and the effect count matches. For the
/// refused half (`merge_mode` 6) the effect count is zero before the request
/// exists, so the whole of that condition is `methods` — which is recorded at
/// the end of the request header, before the body is read and before the merge
/// branch latches `merge_mode`. The release is therefore able to overtake the
/// hold, and the fixture answers with the released mode instead of the held
/// one: 200 applied plus a real provider effect where the journey asserts a
/// refusal and zero effects.
#[test]
fn a_held_refusal_survives_the_release_the_settlement_journey_performs() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use std::sync::atomic::Ordering::SeqCst;
    let provider = Provider::new();
    provider.merge_mode.store(6, SeqCst);
    let configuration: Value =
        serde_json::from_slice(&fs::read(&provider.config).unwrap()).unwrap();
    let port: u16 = configuration["api_base"]
        .as_str()
        .unwrap()
        .trim_end_matches("/api/v4")
        .rsplit(':')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(rustls::pki_types::CertificateDer::from(
            STANDARD
                .decode(
                    provider
                        .pem
                        .lines()
                        .filter(|line| !line.starts_with("-----"))
                        .collect::<String>(),
                )
                .unwrap(),
        ))
        .unwrap();
    let client = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    let body = br#"{"sha":"0123456789abcdef0123456789abcdef01234567","auto_merge":false,"should_remove_source_branch":false}"#;
    let head = format!(
        "PUT /api/v4/projects/org%2Fproject/merge_requests/4/merge HTTP/1.1\r\nHost: localhost\r\nPrivate-Token: fixture-pat-one\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let response = runtime.block_on(async {
        let socket = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let mut stream = tokio_rustls::TlsConnector::from(Arc::new(client))
            .connect(
                rustls::pki_types::ServerName::try_from("localhost").unwrap(),
                socket,
            )
            .await
            .unwrap();
        stream.write_all(head.as_bytes()).await.unwrap();
        stream.flush().await.unwrap();
        // Verbatim the break condition of the mode 8/9 branch, for mode 9.
        let until = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            let sent = provider
                .methods
                .lock()
                .unwrap()
                .iter()
                .filter(|method| method.as_str() == "PUT")
                .count();
            if sent == 1 && provider.merge_effects.load(SeqCst) == 0 {
                break;
            }
            assert!(
                std::time::Instant::now() < until,
                "fixture did not record the merge"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        // What the journey does next: inject the metadata fault, then release.
        provider.merge_mode.store(0, SeqCst);
        stream.write_all(body).await.unwrap();
        stream.flush().await.unwrap();
        let mut response = Vec::new();
        // The fixture closes without close_notify; an unclean EOF is expected.
        let _ = stream.read_to_end(&mut response).await;
        response
    });
    let response = String::from_utf8_lossy(&response);
    assert!(
        response.starts_with("HTTP/1.1 409"),
        "the held refusal was answered with the released mode instead: {response}"
    );
    assert_eq!(
        provider.merge_effects.load(SeqCst),
        0,
        "the refused half of the settlement journey recorded a provider merge effect"
    );
}

// ---------------------------------------------------------------------------
// Adversary pass 2, unit 1. Test-only addition; no production source touched.
// ---------------------------------------------------------------------------

/// Journey mode 10 revokes the connection after a known Applied provider effect
/// and checks that disclosure is refused with no second effect. That is not the
/// **Attempt terminal settlement** row it was added under; it is a connection
/// race after the native response, which is what the **Final result disclosure**
/// row lists as work still to be done. One row was rewritten when the evidence
/// landed and this one was not, so its two cells now understate what is covered
/// and overstate what remains.
#[test]
fn the_disclosure_row_records_the_post_effect_revocation_journey_that_exists() {
    let matrix = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/gitlab-write-failure-matrix.md"
    ))
    .unwrap();
    let journeys = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/local_runtime/guarded_merge.rs"
    ))
    .unwrap();
    if !journeys.contains("Final disclosure admission is checked after settlement") {
        // No such evidence in the fixture; the row is entitled to keep asking.
        return;
    }
    let row = matrix
        .lines()
        .find(|line| line.starts_with("| Final result disclosure |"))
        .expect("the matrix must carry a Final result disclosure row");
    let cells: Vec<&str> = row.split('|').map(str::trim).collect();
    assert_ne!(
        cells[4],
        "Complete policy/key/connection races after native response and during final \
         audit handling, with effect counts and safe response checks.",
        "guarded_merge.rs now runs a connection race after the native response with \
         effect counts and a safe response check, and this row's remaining column is \
         byte-unchanged; its existing-evidence column still names only preflight \
         revocation and background recovery: {}",
        cells[3]
    );
}

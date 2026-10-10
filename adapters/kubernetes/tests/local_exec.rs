//! `pods.exec` end to end through the local private runtime: the composition
//! loads with `pod_exec: true`, lists the mutation only on the
//! `connectors-private/2` write exchange, and runs it through
//! `prepare_write`/commit over the host's upgraded-stream write capability
//! against a TLS fixture that answers the exec subresource's WebSocket
//! upgrade with recorded `v5.channel.k8s.io` messages. No cluster is
//! contacted and no real credential is used.
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Effect, Failure, PrivateProtocol, WriteEffect, WriteResult},
};
use connectors_sdk::Secret;
use futures_util::SinkExt as _;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::oneshot;
use tokio_rustls::{
    TlsAcceptor,
    rustls::{self, pki_types::PrivatePkcs8KeyDer},
};
use tokio_tungstenite::tungstenite::{
    Message,
    handshake::server::{Callback, ErrorResponse, Request, Response},
};

/// Records each exec handshake (URI, authorization, offered subprotocols) and
/// selects `v5.channel.k8s.io`. The framing library fixes the callback's error
/// type.
struct Handshake(Arc<Mutex<Vec<(String, String, String)>>>);

impl Callback for Handshake {
    fn on_request(
        self,
        request: &Request,
        mut response: Response,
    ) -> Result<Response, ErrorResponse> {
        let header = |name: &str| {
            request
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_owned()
        };
        self.0.lock().unwrap().push((
            request.uri().to_string(),
            header("authorization"),
            header("sec-websocket-protocol"),
        ));
        response.headers_mut().insert(
            "sec-websocket-protocol",
            "v5.channel.k8s.io".parse().unwrap(),
        );
        Ok(response)
    }
}

/// Fictional fixture material only.
const TOKEN: &str = "fixture-kubernetes-exec-token";
const SUCCESS: u8 = 0;
const EXIT_THREE: u8 = 1;
const LOST: u8 = 2;

/// The recorded exec subresource: every connection is one upgrade request,
/// answered according to `mode`.
struct Cluster {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    mode: Arc<AtomicU8>,
    connections: Arc<AtomicUsize>,
    /// The request target and authorization of each upgrade the fixture saw.
    upgrades: Arc<Mutex<Vec<(String, String, String)>>>,
}
impl Cluster {
    fn new(pod_exec: bool) -> Self {
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
        let mode = Arc::new(AtomicU8::new(SUCCESS));
        let connections = Arc::new(AtomicUsize::new(0));
        let upgrades = Arc::new(Mutex::new(Vec::new()));
        let (address_tx, address_rx) = std::sync::mpsc::channel();
        let (stop, mut stopped) = oneshot::channel();
        let thread = {
            let mode = mode.clone();
            let connections = connections.clone();
            let upgrades = upgrades.clone();
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                runtime.block_on(async {
                    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                    address_tx.send(listener.local_addr().unwrap()).unwrap();
                    let acceptor = TlsAcceptor::from(Arc::new(tls));
                    loop {
                        let stream = tokio::select! {
                            _ = &mut stopped => break,
                            value = listener.accept() => value.unwrap().0,
                        };
                        connections.fetch_add(1, Ordering::SeqCst);
                        let Ok(stream) = acceptor.accept(stream).await else {
                            continue;
                        };
                        let seen = upgrades.clone();
                        let Ok(mut socket) =
                            tokio_tungstenite::accept_hdr_async(stream, Handshake(seen)).await
                        else {
                            continue;
                        };
                        let frame = |channel: u8, data: &[u8]| {
                            let mut message = vec![channel];
                            message.extend_from_slice(data);
                            Message::Binary(message.into())
                        };
                        match mode.load(Ordering::SeqCst) {
                            SUCCESS => {
                                let _ = socket.send(frame(1, b"fixture-pod\n")).await;
                                let _ = socket.send(frame(2, b"note\n")).await;
                                let _ = socket
                                    .send(frame(3, br#"{"metadata":{},"status":"Success"}"#))
                                    .await;
                                let _ = socket.close(None).await;
                            }
                            EXIT_THREE => {
                                let status = json!({"metadata":{},"status":"Failure",
                                    "message":"command terminated with non-zero exit code: exit code 3",
                                    "reason":"NonZeroExitCode",
                                    "details":{"causes":[{"reason":"ExitCode","message":"3"}]}});
                                let _ = socket.send(frame(2, b"failed\n")).await;
                                let _ = socket
                                    .send(frame(3, status.to_string().as_bytes()))
                                    .await;
                                let _ = socket.close(None).await;
                            }
                            _ => {
                                // Output, then the connection ends with no
                                // status and no close frame.
                                let _ = socket.send(frame(1, b"partial")).await;
                                let _ = socket.flush().await;
                                drop(socket);
                            }
                        }
                    }
                });
            })
        };
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let config = directory.join("kubernetes.json");
        let mut document = json!({
            "format":"connectors-kubernetes-local/1",
            "instance":"fixture-kubernetes",
            "api_base":format!("https://localhost:{}/", address.port()),
            "ca_file":ca,
            "namespaces":["fixture"],
            "resource_kinds":["pods"],
        });
        if pod_exec {
            document["pod_exec"] = json!(true);
        }
        private(&config, &serde_json::to_vec(&document).unwrap());
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config,
            mode,
            connections,
            upgrades,
        }
    }
    fn bootstrap(&self) -> Bootstrap {
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
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        bootstrap.validate().unwrap();
        bootstrap
    }
    fn selection(&self, protocol: Option<PrivateProtocol>) -> Adapter {
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-kubernetes"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: protocol,
            permissions: Default::default(),
            instance_id: "fixture-kubernetes".into(),
            adapter_id: "kubernetes".into(),
            configuration_revision: self.bootstrap().configuration_revision,
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
    fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }
}
impl Drop for Cluster {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn secret() -> Secret {
    Secret(serde_json::to_vec(&json!({"token": TOKEN})).unwrap())
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
fn input(namespace: &str) -> Value {
    json!({"namespace":namespace,"pod":"api-0","container":"api",
        "command":["printenv","HOSTNAME"],"timeout_seconds":10,"max_output_bytes":1024})
}
/// Prepare and commit one exec, as the host's approval coordinator does once
/// it holds a verified, spent approval. A refusal in prepare is the `Err`.
fn exec(child: &mut Child, input: &Value) -> Result<WriteResult, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let prepared = child.prepare_write(
        "pods.exec",
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        deadline(),
    )?;
    Ok(prepared.commit())
}

/// `pod_exec: true` now loads. The read exchange (`--print-local-bootstrap`,
/// protocol one) never lists `pods.exec`; the write exchange lists it as the
/// one write requirement, under a separately revised descriptor.
#[test]
fn local_pod_exec_loads_and_is_listed_only_on_the_write_exchange() {
    let cluster = Cluster::new(true);
    let unconfigured = Cluster::new(false).bootstrap();
    let read = cluster.bootstrap();
    assert_ne!(
        read.configuration_revision, unconfigured.configuration_revision,
        "pod_exec is part of the admitted configuration"
    );
    assert!(
        read.descriptor()
            .unwrap()
            .operations
            .iter()
            .all(|o| o.id != "pods.exec")
    );
    let child = Child::spawn(&cluster.selection(Some(PrivateProtocol::V2))).unwrap();
    let bootstrap = child.bootstrap();
    let descriptor = bootstrap.descriptor().unwrap();
    assert!(descriptor.operations.iter().any(|o| o.id == "pods.exec"));
    assert_ne!(descriptor.revision, read.descriptor().unwrap().revision);
    let writes: Vec<_> = bootstrap
        .requirements
        .iter()
        .filter(|r| r.effect == Effect::Write)
        .map(|r| r.operation.as_str())
        .collect();
    assert_eq!(writes, ["pods.exec"]);
    assert_eq!(cluster.connections(), 0);
}

/// A committed exec upgrades once, carries the argument vector as `command`
/// terms with no shell and the credential, and settles from the status: exit
/// 0 with bounded output, and a non-zero exit as a completed attempt.
#[test]
fn local_pod_exec_runs_through_the_approved_write_exchange() {
    let cluster = Cluster::new(true);
    let mut child = Child::spawn(&cluster.selection(Some(PrivateProtocol::V2))).unwrap();
    let result = exec(&mut child, &input("fixture")).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let value = result.result.unwrap();
    assert_eq!(value["exit_code"], 0);
    assert_eq!(value["stdout"], "fixture-pod\n");
    assert_eq!(value["stderr"], "note\n");
    assert_eq!(value["stdout_truncated"], false);
    assert_eq!(value["stderr_truncated"], false);
    assert_eq!(value["pod"], "api-0");
    {
        let upgrades = cluster.upgrades.lock().unwrap();
        assert_eq!(upgrades.len(), 1);
        let (target, authorization, offered) = &upgrades[0];
        assert_eq!(
            target,
            "/api/v1/namespaces/fixture/pods/api-0/exec?container=api&command=printenv&command=HOSTNAME&stdin=false&stdout=true&stderr=true&tty=false"
        );
        assert_eq!(authorization, &format!("Bearer {TOKEN}"));
        assert_eq!(offered, "v5.channel.k8s.io, v4.channel.k8s.io");
    }

    cluster.mode.store(EXIT_THREE, Ordering::SeqCst);
    let result = exec(&mut child, &input("fixture")).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let value = result.result.unwrap();
    assert_eq!(value["exit_code"], 3);
    assert_eq!(value["stderr"], "failed\n");
    assert_eq!(cluster.upgrades.lock().unwrap().len(), 2);
}

/// A stream lost after the server switched protocols, before any status, is
/// an unknown outcome: the command may have run. The upgrade is never resent.
#[test]
fn local_pod_exec_lost_stream_is_an_unknown_outcome() {
    let cluster = Cluster::new(true);
    cluster.mode.store(LOST, Ordering::SeqCst);
    let mut child = Child::spawn(&cluster.selection(Some(PrivateProtocol::V2))).unwrap();
    let result = exec(&mut child, &input("fixture")).unwrap();
    assert_eq!(result.effect, WriteEffect::Unknown, "{:?}", result.result);
    assert!(result.result.is_err());
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(cluster.upgrades.lock().unwrap().len(), 1);
    assert_eq!(cluster.connections(), 1);
}

/// No upgraded stream is opened outside a committed write: not through the
/// read exchange, not by a preparation that is cancelled, not for a namespace
/// outside the configured set, and not on protocol one at all.
#[test]
fn local_pod_exec_opens_no_stream_outside_a_committed_write() {
    let cluster = Cluster::new(true);
    let mut child = Child::spawn(&cluster.selection(Some(PrivateProtocol::V2))).unwrap();
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let body = serde_json::to_vec(&input("fixture")).unwrap();

    let read = child.invoke("pods.exec", &revision, "one", &secret(), &body, deadline());
    assert_eq!(read.unwrap_err(), Failure::Unsupported);

    let prepared = child
        .prepare_write("pods.exec", &revision, "one", &secret(), &body, deadline())
        .unwrap();
    prepared.cancel().unwrap();

    let outside = serde_json::to_vec(&input("kube-system")).unwrap();
    let refused = child.prepare_write(
        "pods.exec",
        &revision,
        "one",
        &secret(),
        &outside,
        deadline(),
    );
    assert_eq!(refused.err(), Some(Failure::Forbidden));

    let mut one = Child::spawn(&cluster.selection(None)).unwrap();
    let revision = one.bootstrap().descriptor().unwrap().revision;
    let refused = one.prepare_write("pods.exec", &revision, "one", &secret(), &body, deadline());
    assert_eq!(refused.err(), Some(Failure::Unsupported));

    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(cluster.connections(), 0);
    assert!(cluster.upgrades.lock().unwrap().is_empty());
}

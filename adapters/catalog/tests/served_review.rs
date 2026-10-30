//! Security-review conformance probes for the served catalog binary
//! (story:served-instance-admits-a-service-account-token): no catalog write is
//! reachable by any operation id, the mounted credential is refused under the
//! `mounted_secret` checks with no provider request, the binary admits by
//! TokenReview under `service-account-token`, and a read still answers when
//! the binary's standard error is a closed pipe.
//!
//! The binary runs in the jail of `connectors_custody::mounted::fixture`, as
//! in `served.rs`. Every token is fictional and only compared.
#![cfg(target_os = "linux")]
use connectors_custody::mounted::fixture::{Jail, Map};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::{
    fs,
    io::{BufRead as _, BufReader, Write as _},
    os::unix::fs::{PermissionsExt as _, symlink},
    path::Path,
    process::{Child, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    sync::oneshot,
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{self, pki_types::PrivatePkcs8KeyDer},
};

const SERVICE_TOKEN: &str = "fixture-service-bearer";
const TOKEN: &str = "fixture-pat-one";
const SOURCE: &str = "/srv/secret";
const MOUNTED: &str = "/run/connectors/credential";
const BINARY: &str = "/bin-test/connectors-catalog-provider";
const CONFIG: &str = "/srv/served.json";
const AUDIENCE: &str = "connectors-gitlab";
const PRINCIPAL: &str = "system:serviceaccount:agents:example-agent";
const CALLER_TOKEN: &str = "caller-projected-token";
const INSTANCE_TOKEN: &str = "instance-own-token";

/// One HTTPS listener on loopback, answering each request with `reply(method,
/// route, headers)` and recording it.
struct Listener {
    port: u16,
    pem: String,
    seen: Arc<Mutex<Vec<(String, String)>>>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

type Reply = fn(&str, &str, &str) -> (u16, Value);

impl Listener {
    fn new(reply: Reply) -> Self {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let pem = cert.cert.pem();
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
        let (address_tx, address_rx) = mpsc::channel();
        let (stop, mut stopped) = oneshot::channel();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let observed = seen.clone();
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
                    let stream = tokio::select! {
                        _ = &mut stopped => break,
                        accepted = listener.accept() => accepted.unwrap().0,
                    };
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        continue;
                    };
                    let mut head = Vec::new();
                    while !head.ends_with(b"\r\n\r\n") && head.len() < 16384 {
                        match stream.read_u8().await {
                            Ok(byte) => head.push(byte),
                            Err(_) => break,
                        }
                    }
                    let head = String::from_utf8_lossy(&head).into_owned();
                    let length = head
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                                .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    let mut body = vec![0; length];
                    let _ = stream.read_exact(&mut body).await;
                    let mut words = head.split_whitespace();
                    let method = words.next().unwrap_or_default().to_owned();
                    let route = words
                        .next()
                        .unwrap_or_default()
                        .split('?')
                        .next()
                        .unwrap_or_default()
                        .to_owned();
                    let (status, answer) = reply(&method, &route, &head);
                    observed.lock().unwrap().push((method, route));
                    let answer = serde_json::to_vec(&answer).unwrap();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answer.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&answer).await;
                    let _ = stream.shutdown().await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        Self {
            port: address.port(),
            pem,
            seen,
            stop: Some(stop),
            thread: Some(thread),
        }
    }

    fn endpoint(&self) -> String {
        format!("https://localhost:{}", self.port)
    }

    fn seen(&self) -> Vec<(String, String)> {
        self.seen.lock().unwrap().clone()
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self.stop.take().map(|stop| stop.send(()));
        let _ = self.thread.take().map(std::thread::JoinHandle::join);
    }
}

/// A GitLab-shaped provider: `GET /api/v4/projects/7` answers one project for
/// the fixture token; anything else is 404 or 401.
fn provider() -> Listener {
    Listener::new(|method, route, head| {
        let known = head.lines().any(|line| {
            line.split_once(':').is_some_and(|(name, value)| {
                name.eq_ignore_ascii_case("private-token") && value.trim() == TOKEN
            })
        });
        if !known {
            (401, json!({"message": "401 Unauthorized"}))
        } else if method == "GET" && route == "/api/v4/projects/7" {
            (200, json!({"id": 7, "name": "fixture"}))
        } else {
            (404, json!({"message": "404 Not Found"}))
        }
    })
}

/// A TokenReview endpoint that authenticates every review carrying the
/// instance's own token as `PRINCIPAL` for `AUDIENCE`.
fn cluster() -> Listener {
    Listener::new(|_, _, head| {
        if !head.contains(INSTANCE_TOKEN) {
            return (401, json!({"kind": "Status", "code": 401}));
        }
        (
            201,
            json!({
                "apiVersion": "authentication.k8s.io/v1",
                "kind": "TokenReview",
                "status": {"authenticated": true, "audiences": [AUDIENCE], "user": {"username": PRINCIPAL}},
            }),
        )
    })
}

/// A TokenReview endpoint that authenticates nothing.
fn refusing_cluster() -> Listener {
    Listener::new(|_, _, _| {
        (
            201,
            json!({
                "apiVersion": "authentication.k8s.io/v1",
                "kind": "TokenReview",
                "status": {"error": "invalid bearer token"},
            }),
        )
    })
}

struct Served {
    jail: Jail,
    alias: String,
    profile: String,
    child: Option<Child>,
    base: String,
    bearer: String,
}

impl Served {
    fn new(provider: &Listener, instance: &str, profile: &str, catalog: Value) -> Self {
        Self::with_service(provider, instance, profile, catalog, None)
    }

    /// `cluster`, when given, selects `service-account-token` against it.
    fn with_service(
        provider: &Listener,
        instance: &str,
        profile: &str,
        catalog: Value,
        cluster: Option<&Listener>,
    ) -> Self {
        let mut jail = Jail::new();
        jail.directory("/srv", 0o755);
        jail.directory("/srv/state", 0o700);
        jail.directory("/srv/tmp", 0o700);
        jail.directory(SOURCE, 0o755);
        jail.directory(MOUNTED, 0o755);
        jail.read_only(SOURCE, MOUNTED);
        jail.install(
            Path::new(env!("CARGO_BIN_EXE_connectors-catalog-provider")),
            BINARY,
        );
        let private = |inside: &str, bytes: &[u8], mode: u32| {
            let path = jail.path(inside);
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        };
        private("/srv/ca.pem", provider.pem.as_bytes(), 0o644);
        private("/srv/service-token", SERVICE_TOKEN.as_bytes(), 0o600);
        let (service, bearer) = match cluster {
            None => (
                json!({
                    "instance": instance,
                    "listen": "127.0.0.1:0",
                    "service_credential": {"kind": "file", "path": "/srv/service-token"},
                    "state": "/srv/state",
                }),
                SERVICE_TOKEN.to_owned(),
            ),
            Some(cluster) => {
                private("/srv/cluster-ca.pem", cluster.pem.as_bytes(), 0o644);
                private("/srv/instance-token", INSTANCE_TOKEN.as_bytes(), 0o600);
                (
                    json!({
                        "instance": instance,
                        "listen": "127.0.0.1:0",
                        "state": "/srv/state",
                        "admission": "service-account-token",
                        "audience": AUDIENCE,
                        "principal": PRINCIPAL,
                        "token_review": {
                            "api": cluster.endpoint(),
                            "ca_file": "/srv/cluster-ca.pem",
                            "credential_file": "/srv/instance-token",
                        },
                    }),
                    CALLER_TOKEN.to_owned(),
                )
            }
        };
        let config = json!({
            "service": service,
            "catalog": catalog,
            "credential": {"binding": "mounted_secret", "directory": MOUNTED},
        });
        private(CONFIG, &serde_json::to_vec(&config).unwrap(), 0o600);
        Self {
            jail,
            alias: instance.to_owned(),
            profile: profile.to_owned(),
            child: None,
            base: String::new(),
            bearer,
        }
    }

    fn path(&self, inside: &str) -> std::path::PathBuf {
        self.jail.path(inside)
    }

    /// A new timestamped directory holding `items` (alias, profile, bytes,
    /// mode), then `..data` moved onto it, as the kubelet's atomic writer does.
    fn deliver_items(&self, stamp: &str, items: &[(&str, &str, Item)]) {
        let source = self.path(SOURCE);
        let stamp_dir = source.join(stamp);
        fs::create_dir_all(&stamp_dir).unwrap();
        fs::set_permissions(&stamp_dir, fs::Permissions::from_mode(0o755)).unwrap();
        for (alias, profile, item) in items {
            let directory = stamp_dir.join(alias);
            fs::create_dir_all(&directory).unwrap();
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).unwrap();
            let path = directory.join(profile);
            match item {
                Item::File(bytes, mode) => {
                    fs::write(&path, bytes).unwrap();
                    fs::set_permissions(&path, fs::Permissions::from_mode(*mode)).unwrap();
                }
                Item::Link(target) => symlink(target, &path).unwrap(),
            }
            if source.join(alias).symlink_metadata().is_err() {
                symlink(Path::new("..data").join(alias), source.join(alias)).unwrap();
            }
        }
        if source.join(&self.alias).symlink_metadata().is_err() {
            symlink(
                Path::new("..data").join(&self.alias),
                source.join(&self.alias),
            )
            .unwrap();
        }
        symlink(stamp, source.join("..data_tmp")).unwrap();
        fs::rename(source.join("..data_tmp"), source.join("..data")).unwrap();
    }

    fn deliver(&self, stamp: &str, item: Item) {
        let (alias, profile) = (self.alias.clone(), self.profile.clone());
        self.deliver_items(stamp, &[(&alias, &profile, item)]);
    }

    /// Start the binary and wait for its listening line. With `close_stderr`
    /// the test then closes the read end of the binary's standard error, as
    /// a supervisor that exits does; otherwise it drains it for the binary's
    /// whole life.
    fn start(&mut self, close_stderr: bool) -> Result<(), String> {
        let mut attempts = 0;
        let mut child = loop {
            let spawned = self
                .jail
                .command(BINARY, Map::Own)
                .args(["--config", CONFIG])
                .env_clear()
                .env("SSL_CERT_FILE", "/srv/ca.pem")
                .env("TMPDIR", "/srv/tmp")
                .env("NO_COLOR", "1")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn();
            match spawned {
                Ok(child) => break child,
                Err(error) if error.raw_os_error() == Some(libc::ETXTBSY) && attempts < 100 => {
                    attempts += 1;
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(error) => panic!("{}", Jail::spawn_error(&error)),
            }
        };
        let stderr = child.stderr.take().unwrap();
        let (lines_tx, lines_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => return,
                    Ok(_) => {
                        let listening = line.contains("adapter service listening");
                        let _ = lines_tx.send(line.clone());
                        if listening && close_stderr {
                            // Dropping the reader closes the pipe's read end.
                            return;
                        }
                    }
                }
            }
        });
        let mut log = String::new();
        let started = std::time::Instant::now();
        while started.elapsed() < Duration::from_secs(60) {
            match lines_rx.recv_timeout(Duration::from_millis(200)) {
                Ok(line) => {
                    log.push_str(&line);
                    if line.contains("adapter service listening") {
                        let listen = line
                            .split_whitespace()
                            .find_map(|field| field.strip_prefix("listen="))
                            .expect("the listening line names its address");
                        self.base = format!("http://{listen}");
                        self.child = Some(child);
                        // Let the reader thread drop the pipe first.
                        std::thread::sleep(Duration::from_millis(100));
                        return Ok(());
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let status = child.wait().unwrap();
                    return Err(format!("exited {status}: {log}"));
                }
            }
        }
        let _ = child.kill();
        let _ = child.wait();
        Err(format!("did not listen within 60 s: {log}"))
    }

    async fn send_as(
        &self,
        bearer: &str,
        method: reqwest::Method,
        route: &str,
        body: Option<&Value>,
    ) -> Result<(u16, Value), String> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut request = reqwest::Client::new()
            .request(method, format!("{}{route}", self.base))
            .bearer_auth(bearer)
            .timeout(Duration::from_secs(30));
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().await.map_err(|error| format!("{error:?}"))?;
        let status = response.status().as_u16();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| format!("{error:?}"))?;
        Ok((
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        ))
    }

    async fn send(
        &self,
        method: reqwest::Method,
        route: &str,
        body: Option<&Value>,
    ) -> (u16, Value) {
        self.send_as(&self.bearer, method, route, body)
            .await
            .expect("the served host answers")
    }

    async fn revision(&self) -> String {
        let (status, descriptor) = self.send(reqwest::Method::GET, "/v1/describe", None).await;
        assert_eq!(status, 200, "{descriptor}");
        descriptor["revision"].as_str().unwrap().to_owned()
    }

    fn invocation(revision: &str, request_id: &str, operation: &str, input: Value) -> Value {
        json!({
            "version": "v1alpha2",
            "request_id": request_id,
            "operation": operation,
            "revision": revision,
            "input": input,
        })
    }

    async fn invoke(
        &self,
        revision: &str,
        request_id: &str,
        operation: &str,
        input: Value,
    ) -> (u16, Value) {
        let body = Self::invocation(revision, request_id, operation, input);
        self.send(reqwest::Method::POST, "/v1alpha2/invoke", Some(&body))
            .await
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

enum Item {
    File(Vec<u8>, u32),
    Link(String),
}

/// One delivery: (alias, profile, item) for each item it holds.
type Delivery<'a> = Vec<(&'a str, &'a str, Item)>;

fn document(token: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"token": token})).unwrap()
}

fn gitlab(provider: &Listener) -> Value {
    json!({"connector": "gitlab", "instance": "fixture", "endpoint": provider.endpoint()})
}

fn started(provider: &Listener, close_stderr: bool) -> Served {
    let mut served = Served::new(provider, "gitlab.fixture", "gitlab.pat", gitlab(provider));
    served.deliver(
        "..2026_10_30_00_00_00.000000001",
        Item::File(document(TOKEN), 0o400),
    );
    served
        .start(close_stderr)
        .expect("the served binary starts");
    served
}

/// Invariant 7: an admitted read the provider answers is answered 200 even
/// when the binary's standard error is a closed pipe. Measured: the
/// `operation completed` event after dispatch
/// (crates/connectors-host/src/server/v1alpha2.rs:241) cannot be written, the
/// fmt layer's internal-error `eprintln!` panics, the execution task dies after
/// the provider answered and before the final observation, and the host
/// answers 500 `internal`.
#[tokio::test(flavor = "multi_thread")]
async fn a_read_is_answered_while_the_served_binary_stderr_is_closed() {
    let provider = provider();
    let served = started(&provider, true);
    let revision = served.revision().await;
    let (status, answer) = served
        .invoke(&revision, "read-closed", "project.get", json!({"id": "7"}))
        .await;
    let reached = provider.seen().len();
    assert_eq!(
        status, 200,
        "a read the provider answered ({reached} provider request) was answered {status}: {answer}"
    );
    assert_eq!(answer["audit_status"], "complete", "{answer}");
}

/// Invariant 7, admission side: under `service-account-token`, a request the
/// review refuses is answered 401 `unauthorized` with no provider request even
/// when standard error is closed. Measured: the refusal's `token review
/// refused` event panics in the request handler, so the connection closes with
/// no answer; the provider is still never reached (fail closed).
#[tokio::test(flavor = "multi_thread")]
async fn a_refused_review_is_answered_unauthorized_while_stderr_is_closed() {
    let provider = provider();
    let cluster = refusing_cluster();
    let mut served = Served::with_service(
        &provider,
        "gitlab.fixture",
        "gitlab.pat",
        gitlab(&provider),
        Some(&cluster),
    );
    served.deliver(
        "..2026_10_30_00_00_00.000000001",
        Item::File(document(TOKEN), 0o400),
    );
    served.start(true).expect("the served binary starts");
    let body = Served::invocation("rev", "refused", "project.get", json!({"id": "7"}));
    let answered = served
        .send_as(
            CALLER_TOKEN,
            reqwest::Method::POST,
            "/v1alpha2/invoke",
            Some(&body),
        )
        .await;
    // Fail closed: nothing reached the provider whatever the answer was.
    assert!(provider.seen().is_empty(), "{:?}", provider.seen());
    assert_eq!(cluster.seen().len(), 1);
    let (status, answer) = answered.expect("the refusal is answered");
    assert_eq!(status, 401, "{answer}");
    assert_eq!(answer["error"]["code"], "unauthorized", "{answer}");
}

/// The served binary under `service-account-token`, end to end: a reviewed
/// token reads; the describe route admits by the same review; `/v1/invoke` is
/// `unsupported` with no review; a refused review never reaches the provider.
#[tokio::test(flavor = "multi_thread")]
async fn the_served_binary_admits_by_token_review() {
    let provider = provider();
    let cluster = cluster();
    let mut served = Served::with_service(
        &provider,
        "gitlab.fixture",
        "gitlab.pat",
        gitlab(&provider),
        Some(&cluster),
    );
    served.deliver(
        "..2026_10_30_00_00_00.000000001",
        Item::File(document(TOKEN), 0o400),
    );
    served.start(false).expect("the served binary starts");
    let revision = served.revision().await;
    assert_eq!(cluster.seen().len(), 1);
    let (status, answer) = served
        .invoke(&revision, "read-1", "project.get", json!({"id": "7"}))
        .await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(cluster.seen().len(), 2);
    assert_eq!(provider.seen().len(), 1);
    let legacy = json!({"version": "v1alpha1", "request_id": "l", "operation": "project.get", "revision": revision, "input": {"id": "7"}});
    let (status, _) = served
        .send(reqwest::Method::POST, "/v1/invoke", Some(&legacy))
        .await;
    assert_eq!(status, 501);
    assert_eq!(cluster.seen().len(), 2);
    assert_eq!(provider.seen().len(), 1);

    let refusing = refusing_cluster();
    let mut other = Served::with_service(
        &provider,
        "gitlab.fixture",
        "gitlab.pat",
        gitlab(&provider),
        Some(&refusing),
    );
    other.deliver(
        "..2026_10_30_00_00_00.000000001",
        Item::File(document(TOKEN), 0o400),
    );
    other.start(false).expect("the served binary starts");
    let (status, answer) = other
        .invoke(&revision, "read-2", "project.get", json!({"id": "7"}))
        .await;
    assert_eq!(status, 401, "{answer}");
    let (status, _) = other.send(reqwest::Method::GET, "/v1/describe", None).await;
    assert_eq!(status, 401);
    assert_eq!(
        provider.seen().len(),
        1,
        "a refused review reached the provider"
    );
}

/// Invariant 6: no operation id reaches a catalog write: the write's own id,
/// its case and encoding variants, ids the description does not list, on
/// `/v1alpha2/invoke` and `/v1/invoke`.
#[tokio::test(flavor = "multi_thread")]
async fn no_operation_id_reaches_a_catalog_write() {
    let provider = provider();
    let served = started(&provider, false);
    let revision = served.revision().await;
    let ids = [
        "merge_request.create",
        "merge_request.merge",
        "MERGE_REQUEST.CREATE",
        "Merge_Request.Create",
        "merge_request%2Ecreate",
        "merge_request.create ",
        "merge_request.create\u{0}",
        "merge_request．create",
        "project.get/../merge_request.create",
        "project.GET",
        "PROJECT.GET",
    ];
    let input =
        json!({"id": "7", "source_branch": "a", "target_branch": "b", "title": "t", "sha": "0"});
    for (index, id) in ids.iter().enumerate() {
        let (status, answer) = served
            .invoke(&revision, &format!("w-{index}"), id, input.clone())
            .await;
        assert_ne!(status, 200, "{id:?}: {answer}");
        let legacy = json!({"version": "v1alpha1", "request_id": format!("l-{index}"), "operation": id, "revision": revision, "input": input});
        let (status, answer) = served
            .send(reqwest::Method::POST, "/v1/invoke", Some(&legacy))
            .await;
        assert_ne!(status, 200, "{id:?}: {answer}");
    }
    assert!(
        provider.seen().is_empty(),
        "a request reached the provider: {:?}",
        provider.seen()
    );
}

const OPENAPI: &str = r#"{"openapi":"3.0.3","info":{"title":"Fixture forge","version":"1"},
"servers":[{"url":"https://forge.example.com/api/v4"}],
"paths":{
"/user":{"get":{"operationId":"getApiV4User","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"}}}}}}}}},
"/personal_access_tokens/self":{"get":{"operationId":"getApiV4PersonalAccessTokensSelf","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object"}}}}}}},
"/projects/{id}":{"get":{"operationId":"getApiV4ProjectsId","parameters":[{"name":"id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"},"name":{"type":"string"}}}}}}}}},
"/projects/{id}/issues":{"post":{"operationId":"postApiV4ProjectsIdIssues","parameters":[{"name":"id","in":"path","required":true,"schema":{"type":"string"}}],"requestBody":{"content":{"application/json":{"schema":{"type":"object","properties":{"title":{"type":"string"}}}}}},"responses":{"201":{"description":"created","content":{"application/json":{"schema":{"type":"object"}}}}}}}
}}"#;

/// Invariant 6: a definition that declares a write operation as a read, or
/// leaves a write's effect undeclared, does not put it in reach: the binary
/// refuses to start, or describes it not and no request reaches the provider.
#[tokio::test(flavor = "multi_thread")]
async fn a_write_declared_as_a_read_is_never_dispatched() {
    let mut starts = 0;
    for issue in [
        // Control: the same fixture with the write declared as a write starts.
        "  issue.create:\n    op: postApiV4ProjectsIdIssues\n    effect: write\n    body: {title: string!}\n",
        "  issue.create:\n    op: postApiV4ProjectsIdIssues\n    effect: read\n    body: {title: string!}\n",
        "  issue.create:\n    op: postApiV4ProjectsIdIssues\n    effect: read\n",
        "  issue.create:\n    op: postApiV4ProjectsIdIssues\n    body: {title: string!}\n",
        "  issue.create: {op: postApiV4ProjectsIdIssues, description: Create an issue}\n",
    ] {
        let provider = Listener::new(|_, _, _| (201, json!({"id": 1})));
        let mut served = Served::new(
            &provider,
            "forge.fixture",
            "forge.pat",
            json!({
                "definition": "/srv/defs/connectors/forge.yaml",
                "definitions_root": "/srv/defs",
                "instance": "fixture",
                "endpoint": provider.endpoint(),
            }),
        );
        let defs = served.path("/srv/defs");
        fs::create_dir_all(defs.join("upstream/forge")).unwrap();
        fs::create_dir_all(defs.join("connectors")).unwrap();
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gzip.write_all(OPENAPI.as_bytes()).unwrap();
        fs::write(
            defs.join("upstream/forge/openapi.json.gz"),
            gzip.finish().unwrap(),
        )
        .unwrap();
        let sha256 = hex::encode(Sha256::digest(OPENAPI.as_bytes()));
        let definition = format!(
            "schema: urn:beyond10x:connectors-definition:1\n\
             source:\n  openapi: upstream/forge/openapi.json.gz\n  sha256: {sha256}\n\
             base_url: https://{{hostname}}/api/v4\n\
             config: {{hostname: {{type: string, default: forge.example.com}}}}\n\
             auth:\n  forge.pat:\n    scheme: header\n    header: PRIVATE-TOKEN\n\
             \x20   label: Fixture personal access token\n\
             \x20   identity: {{path: user, kind: forge.user, subject: /id}}\n\
             \x20   scopes: {{path: personal_access_tokens/self, pointer: /scopes, minimum: [api]}}\n\
             operations:\n\
             \x20 project.get: {{op: getApiV4ProjectsId, description: Read one project}}\n{issue}"
        );
        fs::write(defs.join("connectors/forge.yaml"), definition).unwrap();
        served.deliver(
            "..2026_10_30_00_00_00.000000001",
            Item::File(document(TOKEN), 0o400),
        );
        if let Err(refusal) = served.start(false) {
            eprintln!("start refused for {issue:?}: {refusal}");
            continue;
        }
        starts += 1;
        let revision = served.revision().await;
        let (_, descriptor) = served
            .send(reqwest::Method::GET, "/v1/describe", None)
            .await;
        let listed: Vec<&str> = descriptor["operations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|operation| operation["id"].as_str().unwrap())
            .collect();
        assert!(!listed.contains(&"issue.create"), "{issue}: {listed:?}");
        let (status, answer) = served
            .invoke(
                &revision,
                "w",
                "issue.create",
                json!({"id": "7", "title": "t"}),
            )
            .await;
        assert_ne!(status, 200, "{issue}: {answer}");
        assert!(
            !provider.seen().iter().any(|(method, _)| method != "GET"),
            "{issue}: a non-GET request reached the provider: {:?}",
            provider.seen()
        );
    }
    assert!(starts >= 1, "the control fixture starts");
}

/// Invariant 5: each mounted item the checks must refuse is `unavailable`
/// with no provider request: a link out of the volume, an item readable or
/// writable by others, one over the size bound, and items only for another
/// instance or another profile. A good item delivered after them is read.
#[tokio::test(flavor = "multi_thread")]
async fn a_refused_mounted_item_never_reaches_the_provider() {
    let provider = provider();
    let served = started(&provider, false);
    let revision = served.revision().await;
    let (status, answer) = served
        .invoke(&revision, "first", "project.get", json!({"id": "7"}))
        .await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(provider.seen().len(), 1);

    let outside = served.path("/srv/escape");
    fs::write(&outside, document(TOKEN)).unwrap();
    fs::set_permissions(&outside, fs::Permissions::from_mode(0o400)).unwrap();
    let oversized = {
        let mut bytes = document(TOKEN);
        bytes.extend(std::iter::repeat_n(b' ', 64 * 1024 + 1));
        bytes
    };
    let mut stamp = 2;
    let mut next = || {
        stamp += 1;
        format!("..2026_10_30_00_00_00.{stamp:09}")
    };
    let cases: Vec<(&str, Delivery)> = vec![
        (
            "a link out of the volume",
            vec![(
                "gitlab.fixture",
                "gitlab.pat",
                Item::Link("/srv/escape".into()),
            )],
        ),
        (
            "a relative link out of the volume",
            vec![(
                "gitlab.fixture",
                "gitlab.pat",
                Item::Link("../../../../escape".into()),
            )],
        ),
        (
            "world-readable",
            vec![(
                "gitlab.fixture",
                "gitlab.pat",
                Item::File(document(TOKEN), 0o444),
            )],
        ),
        (
            "group-writable",
            vec![(
                "gitlab.fixture",
                "gitlab.pat",
                Item::File(document(TOKEN), 0o420),
            )],
        ),
        (
            "world-writable",
            vec![(
                "gitlab.fixture",
                "gitlab.pat",
                Item::File(document(TOKEN), 0o402),
            )],
        ),
        (
            "over the size bound",
            vec![("gitlab.fixture", "gitlab.pat", Item::File(oversized, 0o400))],
        ),
        (
            "empty",
            vec![(
                "gitlab.fixture",
                "gitlab.pat",
                Item::File(Vec::new(), 0o400),
            )],
        ),
        (
            "only for another instance",
            vec![(
                "gitlab.other",
                "gitlab.pat",
                Item::File(document(TOKEN), 0o400),
            )],
        ),
        (
            "only for another profile",
            vec![(
                "gitlab.fixture",
                "gitlab.oauth",
                Item::File(document(TOKEN), 0o400),
            )],
        ),
    ];
    for (what, items) in &cases {
        served.deliver_items(&next(), items);
        let (status, answer) = served
            .invoke(
                &revision,
                &format!("r-{}", what.replace(' ', "-")),
                "project.get",
                json!({"id": "7"}),
            )
            .await;
        assert_eq!(status, 503, "{what}: {answer}");
        assert_eq!(answer["error"]["code"], "unavailable", "{what}: {answer}");
        assert!(!answer.to_string().contains(TOKEN), "{what}: {answer}");
        assert_eq!(provider.seen().len(), 1, "{what} reached the provider");
    }
    served.deliver(&next(), Item::File(document(TOKEN), 0o400));
    let (status, answer) = served
        .invoke(&revision, "last", "project.get", json!({"id": "7"}))
        .await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(provider.seen().len(), 2);
}

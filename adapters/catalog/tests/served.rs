//! story:served-instance-admits-a-service-account-token, its catalog part: a
//! catalog instance runs on the served host (`connectors-catalog-provider
//! --config`), describes and reads, reads its credential document from a
//! mounted file at every request under the `mounted_secret` checks, and
//! dispatches no write.
//!
//! The binary runs in a jail (`connectors_custody::mounted::fixture`): its own
//! user and mount namespaces, the credential source bound read-only as a
//! Kubernetes Secret volume presents it. The network is the host's, so the
//! test reaches the served host on loopback and the served host reaches a
//! disposable HTTPS provider on loopback, whose root reaches the binary only
//! through `SSL_CERT_FILE`. Every token is fictional and only compared.
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

/// The bearer the served host admits under `static-bearer`.
const SERVICE_TOKEN: &str = "fixture-service-bearer";
/// The provider credentials the fixture accepts in `PRIVATE-TOKEN`.
const TOKEN: &str = "fixture-pat-one";
const ROTATED: &str = "fixture-pat-two";
/// Where the test lays the Secret volume out, and where the jail mounts it
/// read-only for the served binary.
const SOURCE: &str = "/srv/secret";
const MOUNTED: &str = "/run/connectors/credential";
const BINARY: &str = "/bin-test/connectors-catalog-provider";
const CONFIG: &str = "/srv/served.json";
/// The standing grant file a host that serves writes reads at every write.
const GRANTS: &str = "/srv/grants.json";
/// The provider's answers to `PUT /repos/example-org/service/pulls/<n>/merge`,
/// by pull request: GitHub's documented refusals of a merge ("Merge a pull
/// request": 405 when the merge cannot be performed, 409 when the head moved
/// since the given `sha`, 422 when validation fails, as for a branch that must
/// be brought up to date first) and an ordinary server error. Each refusal
/// body has GitHub's shape, `message` and `documentation_url`; none is a
/// recorded answer.
const MERGE_ANSWERS: [(&str, u16, &str); 4] = [
    (
        "/api/v4/repos/example-org/service/pulls/1/merge",
        405,
        r#"{"message":"Pull Request is not mergeable","documentation_url":"https://docs.github.com/rest/pulls/pulls#merge-a-pull-request"}"#,
    ),
    (
        "/api/v4/repos/example-org/service/pulls/2/merge",
        409,
        r#"{"message":"Head branch was modified. Review and try the merge again.","documentation_url":"https://docs.github.com/rest/pulls/pulls#merge-a-pull-request"}"#,
    ),
    (
        "/api/v4/repos/example-org/service/pulls/3/merge",
        422,
        r#"{"message":"Head branch is not up to date with the base branch","documentation_url":"https://docs.github.com/rest/pulls/pulls#merge-a-pull-request"}"#,
    ),
    (
        "/api/v4/repos/example-org/service/pulls/5/merge",
        502,
        r#"{"message":"Server Error"}"#,
    ),
];
/// The merge whose connection the provider cuts without an answer.
const MERGE_CUT: &str = "/api/v4/repos/example-org/service/pulls/4/merge";

/// One request the provider answered: method, path (without the query) and the
/// `PRIVATE-TOKEN` it carried.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Seen {
    method: String,
    route: String,
    token: Option<String>,
}

/// A GitLab-shaped API on loopback: `GET /api/v4/projects/<id>` answers one
/// project for either fixture token and 401 otherwise; anything else is 404.
struct Provider {
    port: u16,
    pem: String,
    seen: Arc<Mutex<Vec<Seen>>>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Provider {
    fn new() -> Self {
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
                    let mut words = head.split_whitespace();
                    let method = words.next().unwrap_or_default().to_owned();
                    let target = words.next().unwrap_or_default().to_owned();
                    let token = head.lines().find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("private-token"))
                            .map(|(_, value)| value.trim().to_owned())
                    });
                    // A write's body is read whole before the answer, so
                    // the connection closes with nothing left unread.
                    let length: usize = head
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                                .and_then(|(_, value)| value.trim().parse().ok())
                        })
                        .unwrap_or(0);
                    let mut sent = vec![0; length.min(65_536)];
                    if stream.read_exact(&mut sent).await.is_err() {
                        continue;
                    }
                    let route = target.split('?').next().unwrap_or_default().to_owned();
                    let known = matches!(token.as_deref(), Some(TOKEN | ROTATED));
                    // A merge the provider never answers: the connection is
                    // cut after the request was read whole.
                    if known && method == "PUT" && route == MERGE_CUT {
                        observed.lock().unwrap().push(Seen {
                            method,
                            route,
                            token,
                        });
                        drop(stream);
                        continue;
                    }
                    let merge = MERGE_ANSWERS
                        .iter()
                        .find(|(merged, _, _)| method == "PUT" && route == *merged);
                    let (status, body) = if !known {
                        (401, json!({"message": "401 Unauthorized"}))
                    } else if method == "GET" && route == "/api/v4/projects/7" {
                        (200, json!({"id": 7, "name": "fixture"}))
                    } else if method == "POST" && route == "/api/v4/projects/7/issues" {
                        // GitLab's created issue (`APIEntitiesIssue`).
                        let sent: Value = serde_json::from_slice(&sent).unwrap_or(Value::Null);
                        (
                            201,
                            json!({"id": 501, "iid": 3, "project_id": 7, "title": sent["title"],
                                   "state": "opened"}),
                        )
                    } else if let Some((_, status, body)) = merge {
                        (*status, serde_json::from_str(body).unwrap())
                    } else {
                        (404, json!({"message": "404 Not Found"}))
                    };
                    observed.lock().unwrap().push(Seen {
                        method,
                        route,
                        token,
                    });
                    let body = serde_json::to_vec(&body).unwrap();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
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

    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.stop.take().map(|stop| stop.send(()));
        let _ = self.thread.take().map(std::thread::JoinHandle::join);
    }
}

/// The jail the served binary runs in: the binary, its configuration, its
/// service bearer and state, the provider's root and the credential source.
struct Served {
    jail: Jail,
    /// The mounted item's alias: the instance id.
    alias: String,
    profile: String,
    child: Option<Child>,
    base: String,
    log: Arc<Mutex<String>>,
}

impl Served {
    /// Lay out the jail for `instance` (its id, `<connector>.<name>`) with
    /// `catalog` as the configuration's `catalog` member, and deliver
    /// `document` as the item `<instance>/<profile>`.
    fn new(provider: &Provider, instance: &str, profile: &str, catalog: Value) -> Self {
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
        let config = json!({
            "service": {
                "instance": instance,
                "listen": "127.0.0.1:0",
                "service_credential": {"kind": "file", "path": "/srv/service-token"},
                "state": "/srv/state",
            },
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
            log: Arc::default(),
        }
    }

    /// The host path of a path inside the jail.
    fn path(&self, inside: &str) -> std::path::PathBuf {
        self.jail.path(inside)
    }

    /// Lay the item out as the kubelet's atomic writer does: the bytes in a
    /// new timestamped directory, mode `mode`, then `..data` moved onto it.
    fn deliver(&self, stamp: &str, document: &[u8], mode: u32) {
        let source = self.path(SOURCE);
        let item = source.join(stamp).join(&self.alias).join(&self.profile);
        fs::create_dir_all(item.parent().unwrap()).unwrap();
        for directory in [source.join(stamp), source.join(stamp).join(&self.alias)] {
            fs::set_permissions(directory, fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::write(&item, document).unwrap();
        fs::set_permissions(&item, fs::Permissions::from_mode(mode)).unwrap();
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

    /// A stamp directory with the alias but no item, then `..data` onto it:
    /// the Secret no longer holds the key.
    fn withdraw(&self, stamp: &str) {
        let source = self.path(SOURCE);
        let alias = source.join(stamp).join(&self.alias);
        fs::create_dir_all(&alias).unwrap();
        for directory in [source.join(stamp), alias] {
            fs::set_permissions(directory, fs::Permissions::from_mode(0o755)).unwrap();
        }
        symlink(stamp, source.join("..data_tmp")).unwrap();
        fs::rename(source.join("..data_tmp"), source.join("..data")).unwrap();
    }

    /// Start the served binary and wait for its listening line; answers its
    /// stderr so far when it exits instead.
    fn start(&mut self) -> Result<(), String> {
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
                // A copy another thread's child still holds open for writing.
                Err(error) if error.raw_os_error() == Some(libc::ETXTBSY) && attempts < 100 => {
                    attempts += 1;
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(error) => panic!("{}", Jail::spawn_error(&error)),
            }
        };
        // Drained for the process's whole life, so it never writes into a
        // closed pipe; kept for the assertions' messages.
        let stderr = child.stderr.take().unwrap();
        let (lines_tx, lines_rx) = mpsc::channel();
        let log = self.log.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let Ok(line) = line else { break };
                log.lock().unwrap().push_str(&format!("{line}\n"));
                let _ = lines_tx.send(line);
            }
        });
        let started = std::time::Instant::now();
        while started.elapsed() < Duration::from_secs(60) {
            match lines_rx.recv_timeout(Duration::from_millis(200)) {
                Ok(line) => {
                    if line.contains("adapter service listening") {
                        let listen = line
                            .split_whitespace()
                            .find_map(|field| field.strip_prefix("listen="))
                            .expect("the listening line names its address");
                        self.base = format!("http://{listen}");
                        self.child = Some(child);
                        return Ok(());
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let status = child.wait().unwrap();
                    return Err(format!("exited {status}: {}", self.log()));
                }
            }
        }
        let _ = child.kill();
        let _ = child.wait();
        panic!(
            "the served binary did not listen within 60 s: {}",
            self.log()
        );
    }

    /// What the served binary wrote to standard error so far.
    fn log(&self) -> String {
        self.log.lock().unwrap().clone()
    }

    /// [`Served::new`] whose host reads `grants` as its standing grant file
    /// (`service.standing_grants`), laid out private in the jail.
    fn granted(
        provider: &Provider,
        instance: &str,
        profile: &str,
        catalog: Value,
        grants: &Value,
    ) -> Self {
        let served = Self::new(provider, instance, profile, catalog);
        let write = |inside: &str, bytes: &[u8]| {
            let path = served.path(inside);
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        };
        write(GRANTS, &serde_json::to_vec(grants).unwrap());
        let mut config: Value =
            serde_json::from_slice(&fs::read(served.path(CONFIG)).unwrap()).unwrap();
        config["service"]["standing_grants"] = json!(GRANTS);
        write(CONFIG, &serde_json::to_vec(&config).unwrap());
        served
    }

    fn started(provider: &Provider, instance: &str, profile: &str, catalog: Value) -> Self {
        let mut served = Self::new(provider, instance, profile, catalog);
        served.deliver("..2026_10_30_00_00_00.000000001", &document(TOKEN), 0o400);
        served.start().expect("the served binary starts");
        served
    }

    async fn send(
        &self,
        method: reqwest::Method,
        route: &str,
        body: Option<&Value>,
    ) -> (u16, Value) {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut request = reqwest::Client::new()
            .request(method, format!("{}{route}", self.base))
            .bearer_auth(SERVICE_TOKEN)
            .timeout(Duration::from_secs(30));
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().await.expect("the served host answers");
        let status = response.status().as_u16();
        let bytes = response.bytes().await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn describe(&self) -> Value {
        let (status, descriptor) = self.send(reqwest::Method::GET, "/v1/describe", None).await;
        assert_eq!(status, 200, "{descriptor}");
        descriptor
    }

    /// `POST /v1alpha2/invoke` of `operation` at the described revision.
    async fn invoke(
        &self,
        revision: &str,
        request_id: &str,
        operation: &str,
        input: Value,
    ) -> (u16, Value) {
        let body = json!({
            "version": "v1alpha2",
            "request_id": request_id,
            "operation": operation,
            "revision": revision,
            "input": input,
        });
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

/// The `gitlab.pat` credential document: `{"token": ...}`.
fn document(token: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"token": token})).unwrap()
}

fn operations(descriptor: &Value) -> Vec<String> {
    descriptor["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|operation| operation["id"].as_str().unwrap().to_owned())
        .collect()
}

fn gitlab(provider: &Provider) -> Value {
    json!({"connector": "gitlab", "instance": "fixture", "endpoint": provider.endpoint()})
}

/// A shipped connector's instance on the served host describes its reads and
/// no write, answers a read through the provider with the mounted credential,
/// and refuses a write before any provider request.
#[tokio::test(flavor = "multi_thread")]
async fn a_served_catalog_instance_describes_and_reads() {
    let provider = Provider::new();
    let served = Served::started(&provider, "gitlab.fixture", "gitlab.pat", gitlab(&provider));
    let descriptor = served.describe().await;
    assert_eq!(descriptor["instance"], "gitlab.fixture");
    assert_eq!(descriptor["adapter"], "catalog");
    let listed = operations(&descriptor);
    assert!(listed.contains(&"project.get".to_owned()), "{listed:?}");
    // story:served-instance-admits-a-service-account-token: the served host
    // advertises no catalog write.
    assert!(
        !listed.contains(&"merge_request.create".to_owned()),
        "{listed:?}"
    );
    for operation in &descriptor["operations"].as_array().unwrap()[..] {
        assert_ne!(operation["profile"], "mutation", "{operation}");
    }
    let revision = descriptor["revision"].as_str().unwrap();

    let (status, answer) = served
        .invoke(revision, "read-1", "project.get", json!({"id": "7"}))
        .await;
    assert_eq!(status, 200, "{answer}\n{}", served.log());
    assert_eq!(answer["result"]["body"]["name"], "fixture", "{answer}");
    assert_eq!(
        provider.seen(),
        vec![Seen {
            method: "GET".into(),
            route: "/api/v4/projects/7".into(),
            token: Some(TOKEN.into()),
        }]
    );

    // A write the shipped connector declares is not dispatched: the host
    // finds no such operation and answers before any anchor or request.
    let (status, answer) = served
        .invoke(
            revision,
            "write-1",
            "merge_request.create",
            json!({"id": "7", "source_branch": "a", "target_branch": "b", "title": "t"}),
        )
        .await;
    assert_eq!(status, 404, "{answer}\n{}", served.log());
    assert_eq!(answer["error"]["code"], "not_found", "{answer}");
    let legacy = json!({"version": "v1alpha1", "request_id": "write-2", "operation": "merge_request.create", "revision": revision, "input": {}});
    let (status, answer) = served
        .send(reqwest::Method::POST, "/v1/invoke", Some(&legacy))
        .await;
    assert_ne!(status, 200, "{answer}");
    assert_eq!(provider.seen().len(), 1, "no write reached the provider");
}

/// story:served-keyed-writes-under-a-standing-approval, its catalog part: with
/// a standing grant file the served instance describes its writes, and a
/// keyed `issue.create` the grant covers reaches the provider once with the
/// mounted credential. The same key and input again is a replay with no
/// second request; a write to a project the grant does not name, and a write
/// without a key, are refused before any request.
#[tokio::test(flavor = "multi_thread")]
async fn a_served_catalog_write_under_a_standing_grant_is_sent_once() {
    let provider = Provider::new();
    let grants = json!({
        "format": "connectors.standing-grants/1",
        "grants": [{
            "id": "g-fixture-issues", "instance": "gitlab.fixture", "operation": "issue.create",
            "target": [{"pointer": "/id", "value": "7"}],
            "not_after_unix_ms": 4_000_000_000_000_i64, "max_per_hour": 5
        }],
        "revoked": []
    });
    let mut served = Served::granted(
        &provider,
        "gitlab.fixture",
        "gitlab.pat",
        gitlab(&provider),
        &grants,
    );
    served.deliver("..2026_10_30_00_00_00.000000001", &document(TOKEN), 0o400);
    served.start().expect("the served binary starts");
    let descriptor = served.describe().await;
    let create = descriptor["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|operation| operation["id"] == "issue.create")
        .unwrap_or_else(|| panic!("issue.create is not described: {descriptor}"));
    assert_eq!(create["profile"], "mutation");
    let revision = descriptor["revision"].as_str().unwrap();

    let keyed = |request_id: &str, key: Option<&str>, project: &str| {
        let mut body = json!({
            "version": "v1alpha2", "request_id": request_id, "operation": "issue.create",
            "revision": revision, "input": {"id": project, "body": {"title": "Fixture issue"}},
        });
        if let Some(key) = key {
            body["idempotency_key"] = json!(key);
        }
        body
    };
    let route = "/v1alpha2/invoke";
    let (status, answer) = served
        .send(
            reqwest::Method::POST,
            route,
            Some(&keyed("w-1", Some("k-1"), "7")),
        )
        .await;
    assert_eq!(status, 200, "{answer}\n{}", served.log());
    assert_eq!(
        answer["result"]["body"]["title"], "Fixture issue",
        "{answer}"
    );
    assert_eq!(answer["mutation"]["classification"], "applied", "{answer}");
    assert_eq!(answer["mutation"]["replayed"], false, "{answer}");
    let writes = || {
        provider
            .seen()
            .into_iter()
            .filter(|seen| seen.method == "POST")
            .collect::<Vec<_>>()
    };
    assert_eq!(
        writes(),
        vec![Seen {
            method: "POST".into(),
            route: "/api/v4/projects/7/issues".into(),
            token: Some(TOKEN.into()),
        }]
    );

    let (status, replay) = served
        .send(
            reqwest::Method::POST,
            route,
            Some(&keyed("w-1-again", Some("k-1"), "7")),
        )
        .await;
    assert_eq!(status, 200, "{replay}");
    assert_eq!(replay["mutation"]["replayed"], true, "{replay}");
    assert_eq!(replay["mutation"]["attempt"], answer["mutation"]["attempt"]);
    assert_eq!(replay["result"], answer["result"]);

    for (body, status_wanted, code) in [
        (keyed("w-2", Some("k-2"), "8"), 403, "approval_required"),
        (keyed("w-3", None, "7"), 400, "invalid_input"),
    ] {
        let (status, refused) = served.send(reqwest::Method::POST, route, Some(&body)).await;
        assert_eq!(status, status_wanted, "{refused}");
        assert_eq!(refused["error"]["code"], code, "{refused}");
        assert!(refused["mutation"].is_null(), "{refused}");
    }
    // The legacy route reads no grant, so on this host it runs no write.
    let legacy = json!({"version": "v1alpha1", "request_id": "w-4", "operation": "issue.create",
        "revision": revision, "input": {"id": "7", "body": {"title": "Fixture issue"}}});
    let (status, refused) = served
        .send(reqwest::Method::POST, "/v1/invoke", Some(&legacy))
        .await;
    assert_eq!(status, 501, "{refused}");
    assert_eq!(refused["error"]["code"], "unsupported", "{refused}");
    assert_eq!(writes().len(), 1, "one write reached the provider");
}

/// The credential is the mounted item's bytes at each request: a rotation is
/// followed by the next read, and an item the checks refuse or the volume no
/// longer holds is `unavailable` with no provider request.
#[tokio::test(flavor = "multi_thread")]
async fn a_served_catalog_instance_reads_its_credential_from_the_mounted_file() {
    let provider = Provider::new();
    let served = Served::started(&provider, "gitlab.fixture", "gitlab.pat", gitlab(&provider));
    let descriptor = served.describe().await;
    let revision = descriptor["revision"].as_str().unwrap();
    let invoker = &served;
    let read =
        move |id: &'static str| invoker.invoke(revision, id, "project.get", json!({"id": "7"}));

    let (status, answer) = read("first").await;
    assert_eq!(status, 200, "{answer}\n{}", served.log());
    served.deliver("..2026_10_30_00_00_00.000000002", &document(ROTATED), 0o400);
    let (status, answer) = read("rotated").await;
    assert_eq!(status, 200, "{answer}\n{}", served.log());
    let tokens: Vec<_> = provider.seen().into_iter().map(|seen| seen.token).collect();
    assert_eq!(tokens, vec![Some(TOKEN.into()), Some(ROTATED.into())]);

    // Check 5: an item with a write bit is refused, and nothing is sent.
    served.deliver("..2026_10_30_00_00_00.000000003", &document(TOKEN), 0o600);
    let (status, answer) = read("writable").await;
    assert_eq!(status, 503, "{answer}\n{}", served.log());
    assert_eq!(answer["error"]["code"], "unavailable", "{answer}");
    // The volume no longer holds the key.
    served.withdraw("..2026_10_30_00_00_00.000000004");
    let (status, answer) = read("withdrawn").await;
    assert_eq!(status, 503, "{answer}\n{}", served.log());
    assert_eq!(answer["error"]["code"], "unavailable", "{answer}");
    let text = answer.to_string();
    assert!(!text.contains(MOUNTED) && !text.contains(TOKEN), "{text}");
    assert_eq!(
        provider.seen().len(),
        2,
        "no request without an admitted credential"
    );

    // Delivered again, it is read again.
    served.deliver("..2026_10_30_00_00_00.000000005", &document(ROTATED), 0o400);
    let (status, answer) = read("again").await;
    assert_eq!(status, 200, "{answer}\n{}", served.log());
    assert_eq!(provider.seen().len(), 3);
}

/// The vendored source a definition pins: two reads and one write.
const OPENAPI: &str = r#"{"openapi":"3.0.3","info":{"title":"Fixture forge","version":"1"},
"servers":[{"url":"https://forge.example.com/api/v4"}],
"paths":{
"/user":{"get":{"operationId":"getApiV4User","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"}}}}}}}}},
"/personal_access_tokens/self":{"get":{"operationId":"getApiV4PersonalAccessTokensSelf","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object"}}}}}}},
"/projects/{id}":{"get":{"operationId":"getApiV4ProjectsId","parameters":[{"name":"id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"},"name":{"type":"string"}}}}}}}}},
"/projects/{id}/issues":{"get":{"operationId":"getApiV4ProjectsIdIssues","parameters":[{"name":"id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"array","items":{"type":"object"}}}}}}},
"post":{"operationId":"postApiV4ProjectsIdIssues","parameters":[{"name":"id","in":"path","required":true,"schema":{"type":"string"}}],"requestBody":{"content":{"application/json":{"schema":{"type":"object","properties":{"title":{"type":"string"}}}}}},"responses":{"201":{"description":"created","content":{"application/json":{"schema":{"type":"object"}}}}}}}
}}"#;

/// A token-profile definition as an operator writes it, with two reads and
/// one write.
fn forge_definition(source_sha256: &str) -> String {
    format!(
        "schema: urn:beyond10x:connectors-definition:1\n\
         source:\n  openapi: upstream/forge/openapi.json.gz\n  sha256: {source_sha256}\n\
         base_url: https://{{hostname}}/api/v4\n\
         config: {{hostname: {{type: string, default: forge.example.com}}}}\n\
         auth:\n  forge.pat:\n    scheme: header\n    header: PRIVATE-TOKEN\n\
         \x20   label: Fixture personal access token\n\
         \x20   identity: {{path: user, kind: forge.user, subject: /id}}\n\
         \x20   scopes: {{path: personal_access_tokens/self, pointer: /scopes, minimum: [api]}}\n\
         operations:\n\
         \x20 project.get: {{op: getApiV4ProjectsId, description: Read one project}}\n\
         \x20 issues.list: {{op: getApiV4ProjectsIdIssues, description: List a project's issues}}\n\
         \x20 issue.create:\n    op: postApiV4ProjectsIdIssues\n    effect: write\n    body: {{title: string!}}\n"
    )
}

/// An instance of a definition file on the served host describes the
/// definition's reads and none of its writes, and reads through it.
#[tokio::test(flavor = "multi_thread")]
async fn a_served_definition_instance_describes_its_operations() {
    let provider = Provider::new();
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
    fs::write(
        defs.join("connectors/forge.yaml"),
        forge_definition(&sha256),
    )
    .unwrap();
    served.deliver("..2026_10_30_00_00_00.000000001", &document(TOKEN), 0o400);
    served
        .start()
        .expect("the served binary starts on a definition");

    let descriptor = served.describe().await;
    assert_eq!(descriptor["instance"], "forge.fixture");
    let mut listed = operations(&descriptor);
    listed.sort();
    assert_eq!(
        listed,
        vec!["issues.list".to_owned(), "project.get".to_owned()]
    );
    let revision = descriptor["revision"].as_str().unwrap();
    let (status, answer) = served
        .invoke(revision, "read-1", "project.get", json!({"id": "7"}))
        .await;
    assert_eq!(status, 200, "{answer}\n{}", served.log());
    assert_eq!(answer["result"]["body"]["name"], "fixture", "{answer}");
    let (status, answer) = served
        .invoke(
            revision,
            "write-1",
            "issue.create",
            json!({"id": "7", "title": "t"}),
        )
        .await;
    assert_eq!(status, 404, "{answer}\n{}", served.log());
    assert_eq!(provider.seen().len(), 1, "no write reached the provider");
}

/// The vendored source of a forge whose repository read and write take
/// `owner` and `repo`, beside a search and the user read, which take neither.
const REPOSITORY_OPENAPI: &str = r#"{"openapi":"3.0.3","info":{"title":"Fixture repository forge","version":"1"},
"servers":[{"url":"https://forge.example.com/api/v4"}],
"paths":{
"/user":{"get":{"operationId":"getUser","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"}}}}}}}}},
"/personal_access_tokens/self":{"get":{"operationId":"getTokenSelf","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object"}}}}}}},
"/search/repositories":{"get":{"operationId":"searchRepositories","parameters":[{"name":"q","in":"query","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object"}}}}}}},
"/repos/{owner}/{repo}":{"get":{"operationId":"getRepository","parameters":[{"name":"owner","in":"path","required":true,"schema":{"type":"string"}},{"name":"repo","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object"}}}}}}},
"/repos/{owner}/{repo}/issues":{"post":{"operationId":"createIssue","parameters":[{"name":"owner","in":"path","required":true,"schema":{"type":"string"}},{"name":"repo","in":"path","required":true,"schema":{"type":"string"}}],"requestBody":{"content":{"application/json":{"schema":{"type":"object","properties":{"title":{"type":"string"}}}}}},"responses":{"201":{"description":"created","content":{"application/json":{"schema":{"type":"object"}}}}}}}
}}"#;

/// Lay out, in `served`'s jail, a definition of [`REPOSITORY_OPENAPI`] with
/// a repository read, a search, the user read and a repository write.
fn define_repository_forge(served: &Served) {
    define_forge(
        served,
        REPOSITORY_OPENAPI,
        "\x20 repo.get: {op: getRepository, description: Read one repository}\n\
         \x20 repos.search: {op: searchRepositories, description: Search repositories}\n\
         \x20 user.get: {op: getUser, description: Read the user}\n\
         \x20 issue.create:\n    op: createIssue\n    effect: write\n    body: {title: string!}\n",
    );
}

/// Lay out, in `served`'s jail, a token-profile definition of `openapi`
/// whose `operations` member is `operations`.
fn define_forge(served: &Served, openapi: &str, operations: &str) {
    let defs = served.path("/srv/defs");
    fs::create_dir_all(defs.join("upstream/forge")).unwrap();
    fs::create_dir_all(defs.join("connectors")).unwrap();
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(openapi.as_bytes()).unwrap();
    fs::write(
        defs.join("upstream/forge/openapi.json.gz"),
        gzip.finish().unwrap(),
    )
    .unwrap();
    let sha256 = hex::encode(Sha256::digest(openapi.as_bytes()));
    let definition = format!(
        "schema: urn:beyond10x:connectors-definition:1\n\
         source:\n  openapi: upstream/forge/openapi.json.gz\n  sha256: {sha256}\n\
         base_url: https://{{hostname}}/api/v4\n\
         config: {{hostname: {{type: string, default: forge.example.com}}}}\n\
         auth:\n  forge.pat:\n    scheme: header\n    header: PRIVATE-TOKEN\n\
         \x20   label: Fixture personal access token\n\
         \x20   identity: {{path: user, kind: forge.user, subject: /id}}\n\
         \x20   scopes: {{path: personal_access_tokens/self, pointer: /scopes, minimum: [api]}}\n\
         operations:\n{operations}"
    );
    fs::write(defs.join("connectors/forge.yaml"), definition).unwrap();
}

/// The `catalog` member of a [`define_repository_forge`] instance, with
/// `repositories` as `catalog.repositories` unless it is null.
fn repository_catalog(provider: &Provider, repositories: Value) -> Value {
    let mut catalog = json!({
        "definition": "/srv/defs/connectors/forge.yaml",
        "definitions_root": "/srv/defs",
        "instance": "fixture",
        "endpoint": provider.endpoint(),
    });
    if !repositories.is_null() {
        catalog["repositories"] = repositories;
    }
    catalog
}

/// story:catalog-reads-stay-inside-the-instance-repositories: with
/// `catalog.repositories` the served instance describes only the operations
/// that take `owner` and `repo`, sends a read or a write only for a listed
/// repository (case and `.git` aside), and answers any other repository as it
/// answers a missing one, `not_found`, with no request to the provider.
#[tokio::test(flavor = "multi_thread")]
async fn a_served_repository_list_bounds_reads_and_writes_to_its_repositories() {
    let provider = Provider::new();
    let grant = |id: &str, owner: &str| {
        json!({
            "id": id, "instance": "forge.fixture", "operation": "issue.create",
            "target": [{"pointer": "/owner", "value": owner}],
            "not_after_unix_ms": 4_000_000_000_000_i64, "max_per_hour": 5
        })
    };
    let grants = json!({
        "format": "connectors.standing-grants/1",
        "grants": [grant("g-listed", "example-org"), grant("g-other", "other-org")],
        "revoked": []
    });
    let mut served = Served::granted(
        &provider,
        "forge.fixture",
        "forge.pat",
        repository_catalog(&provider, json!(["Example-Org/Service.git"])),
        &grants,
    );
    define_repository_forge(&served);
    served.deliver("..2026_10_30_00_00_00.000000001", &document(TOKEN), 0o400);
    served
        .start()
        .expect("the served binary starts with a repository list");

    let descriptor = served.describe().await;
    let mut listed = operations(&descriptor);
    listed.sort();
    assert_eq!(
        listed,
        vec!["issue.create".to_owned(), "repo.get".to_owned()],
        "only the operations that take owner and repo are described"
    );
    let revision = descriptor["revision"].as_str().unwrap();

    // A listed repository is read through the provider, which has no such
    // repository: the answer of a missing repository.
    let (status, missing) = served
        .invoke(
            revision,
            "read-listed",
            "repo.get",
            json!({"owner": "example-org", "repo": "service"}),
        )
        .await;
    assert_eq!(status, 404, "{missing}\n{}", served.log());
    assert_eq!(missing["error"]["code"], "not_found", "{missing}");
    assert_eq!(
        provider.seen(),
        vec![Seen {
            method: "GET".into(),
            route: "/api/v4/repos/example-org/service".into(),
            token: Some(TOKEN.into()),
        }]
    );

    // Any other repository: the same answer, and nothing is sent.
    for (id, input) in [
        (
            "other-owner",
            json!({"owner": "other-org", "repo": "service"}),
        ),
        (
            "other-repo",
            json!({"owner": "example-org", "repo": "other"}),
        ),
        (
            "traversal",
            json!({"owner": "example-org", "repo": "service/../../other-org/service"}),
        ),
    ] {
        let (status, answer) = served.invoke(revision, id, "repo.get", input).await;
        assert_eq!(status, 404, "{id}: {answer}\n{}", served.log());
        assert_eq!(answer["error"], missing["error"], "{id}: {answer}");
    }
    // Operations without both members are not described, so not served.
    for (id, operation, input) in [
        ("search", "repos.search", json!({"q": "service"})),
        ("user", "user.get", json!({})),
    ] {
        let (status, answer) = served.invoke(revision, id, operation, input).await;
        assert_eq!(status, 404, "{id}: {answer}");
        assert_eq!(answer["error"]["code"], "not_found", "{id}: {answer}");
    }
    assert_eq!(
        provider.seen().len(),
        1,
        "no other read reached the provider"
    );

    // A write takes the same check: one the grant covers, to a repository
    // the list does not name, is `not_found` before the host records it, with
    // no request.
    let keyed = |request_id: &str, key: &str, owner: &str, repo: &str| {
        json!({
            "version": "v1alpha2", "request_id": request_id, "operation": "issue.create",
            "revision": revision, "idempotency_key": key,
            "input": {"owner": owner, "repo": repo, "body": {"title": "Fixture issue"}},
        })
    };
    let route = "/v1alpha2/invoke";
    // The key is not used up: the same key again is the same refusal, not a
    // replay of a recorded attempt.
    for request_id in ["w-other", "w-other-again"] {
        let (status, refused) = served
            .send(
                reqwest::Method::POST,
                route,
                Some(&keyed(request_id, "k-other", "other-org", "service")),
            )
            .await;
        assert_eq!(status, 404, "{request_id}: {refused}\n{}", served.log());
        assert_eq!(
            refused["error"], missing["error"],
            "{request_id}: {refused}"
        );
        assert!(refused["mutation"].is_null(), "{request_id}: {refused}");
        assert!(refused["audit_ref"].is_null(), "{request_id}: {refused}");
    }
    let writes = || {
        provider
            .seen()
            .into_iter()
            .filter(|seen| seen.method == "POST")
            .collect::<Vec<_>>()
    };
    assert_eq!(
        writes(),
        vec![],
        "no write to an unlisted repository is sent"
    );
    // To the listed repository it is sent.
    let (_, answer) = served
        .send(
            reqwest::Method::POST,
            route,
            Some(&keyed("w-listed", "k-listed", "example-org", "Service")),
        )
        .await;
    assert_eq!(
        writes(),
        vec![Seen {
            method: "POST".into(),
            route: "/api/v4/repos/example-org/Service/issues".into(),
            token: Some(TOKEN.into()),
        }],
        "{answer}\n{}",
        served.log()
    );
}

/// A `catalog.repositories` that is null, empty or invalid refuses start;
/// without the key every operation is described and any repository is read.
#[tokio::test(flavor = "multi_thread")]
async fn a_served_repository_list_that_is_empty_or_invalid_refuses_start() {
    let provider = Provider::new();
    for repositories in [
        Value::Null,
        json!([]),
        json!(["example-org"]),
        json!(["example-org/"]),
        json!(["example-org/service/extra"]),
        json!(["../service"]),
    ] {
        let mut served = Served::new(&provider, "forge.fixture", "forge.pat", {
            let mut catalog = repository_catalog(&provider, Value::Null);
            catalog["repositories"] = repositories.clone();
            catalog
        });
        define_repository_forge(&served);
        served.deliver("..2026_10_30_00_00_00.000000001", &document(TOKEN), 0o400);
        let Err(log) = served.start() else {
            panic!("{repositories} must refuse start");
        };
        assert!(log.contains("catalog: "), "{repositories}: {log}");
        assert!(log.contains("repositor"), "{repositories}: {log}");
    }

    let mut served = Served::new(
        &provider,
        "forge.fixture",
        "forge.pat",
        repository_catalog(&provider, Value::Null),
    );
    define_repository_forge(&served);
    served.deliver("..2026_10_30_00_00_00.000000001", &document(TOKEN), 0o400);
    served
        .start()
        .expect("the served binary starts without a list");
    let descriptor = served.describe().await;
    let mut listed = operations(&descriptor);
    listed.sort();
    assert_eq!(listed, vec!["repo.get", "repos.search", "user.get"]);
    let revision = descriptor["revision"].as_str().unwrap();
    let (status, answer) = served
        .invoke(
            revision,
            "read-any",
            "repo.get",
            json!({"owner": "other-org", "repo": "service"}),
        )
        .await;
    assert_eq!(status, 404, "{answer}\n{}", served.log());
    assert_eq!(
        provider.seen(),
        vec![Seen {
            method: "GET".into(),
            route: "/api/v4/repos/other-org/service".into(),
            token: Some(TOKEN.into()),
        }]
    );
}

/// Adversary (story:catalog-reads-stay-inside-the-instance-repositories):
/// a configuration that names `catalog.repositories` but gives it no list
/// (`repositories:` with nothing after it in YAML, or a template that
/// renders an empty value as `null`) is not a list, so it must refuse start
/// as `git.repositories: null` does; it must never serve every repository
/// the credential reaches.
#[tokio::test(flavor = "multi_thread")]
async fn adversary_a_null_repository_list_refuses_start_and_never_fails_open() {
    let provider = Provider::new();
    let mut catalog = repository_catalog(&provider, Value::Null);
    catalog["repositories"] = Value::Null;
    let mut served = Served::new(&provider, "forge.fixture", "forge.pat", catalog);
    define_repository_forge(&served);
    served.deliver("..2026_10_30_00_00_00.000000001", &document(TOKEN), 0o400);
    if served.start().is_ok() {
        let descriptor = served.describe().await;
        let revision = descriptor["revision"].as_str().unwrap().to_owned();
        let (status, answer) = served
            .invoke(
                &revision,
                "read-unlisted",
                "repo.get",
                json!({"owner": "other-org", "repo": "service"}),
            )
            .await;
        panic!(
            "`catalog.repositories: null` started and serves every repository: \
             described {:?}; repo.get other-org/service answered {status} {answer}; \
             the provider saw {:?}",
            operations(&descriptor),
            provider.seen(),
        );
    }
}

/// Adversary: names compare without case and a trailing `.git` is dropped
/// (docs/running-services.md, Catalog instances), so the entry
/// `Example-Org/Service.GIT` names the repository `example-org/service`, and
/// a read of it reaches the provider.
#[tokio::test(flavor = "multi_thread")]
async fn adversary_an_uppercase_git_suffix_in_the_list_still_names_the_repository() {
    let provider = Provider::new();
    let mut served = Served::new(
        &provider,
        "forge.fixture",
        "forge.pat",
        repository_catalog(&provider, json!(["Example-Org/Service.GIT"])),
    );
    define_repository_forge(&served);
    served.deliver("..2026_10_30_00_00_00.000000001", &document(TOKEN), 0o400);
    served
        .start()
        .expect("the served binary starts with the list");
    let descriptor = served.describe().await;
    let revision = descriptor["revision"].as_str().unwrap().to_owned();
    let (status, answer) = served
        .invoke(
            &revision,
            "read-listed",
            "repo.get",
            json!({"owner": "example-org", "repo": "service"}),
        )
        .await;
    assert_eq!(status, 404, "{answer}\n{}", served.log());
    assert_eq!(
        provider.seen(),
        vec![Seen {
            method: "GET".into(),
            route: "/api/v4/repos/example-org/service".into(),
            token: Some(TOKEN.into()),
        }],
        "the listed repository is read: {answer}"
    );
}

/// The vendored source of a forge with GitHub's "Merge a pull request":
/// `PUT /repos/{owner}/{repo}/pulls/{pull_number}/merge`.
const MERGE_OPENAPI: &str = r#"{"openapi":"3.0.3","info":{"title":"Fixture merging forge","version":"1"},
"servers":[{"url":"https://forge.example.com/api/v4"}],
"paths":{
"/user":{"get":{"operationId":"getUser","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"}}}}}}}}},
"/personal_access_tokens/self":{"get":{"operationId":"getTokenSelf","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object"}}}}}}},
"/repos/{owner}/{repo}/pulls/{pull_number}/merge":{"put":{"operationId":"mergePullRequest","parameters":[{"name":"owner","in":"path","required":true,"schema":{"type":"string"}},{"name":"repo","in":"path","required":true,"schema":{"type":"string"}},{"name":"pull_number","in":"path","required":true,"schema":{"type":"integer"}}],"requestBody":{"content":{"application/json":{"schema":{"type":"object","properties":{"merge_method":{"type":"string"}}}}}},"responses":{"200":{"description":"merged","content":{"application/json":{"schema":{"type":"object","properties":{"merged":{"type":"boolean"},"sha":{"type":"string"},"message":{"type":"string"}}}}}}}}}
}}"#;

/// story:served-merge-refusal-answers-refused: a served `pull_request.merge`
/// the provider definitely refuses (405, 409, 422 with GitHub's bodies)
/// answers that refusal's code with the provider's message, the mutation
/// classified `refused`, and a replay of the same key answers the same
/// refusal with no second request. A cut connection and a 5xx say nothing
/// about the merge: `outcome_unknown`, classified `unknown`.
#[tokio::test(flavor = "multi_thread")]
async fn a_served_merge_the_provider_refuses_answers_refused_with_its_message() {
    let provider = Provider::new();
    let grants = json!({
        "format": "connectors.standing-grants/1",
        "grants": [{
            "id": "g-merges", "instance": "forge.fixture", "operation": "pull_request.merge",
            "target": [{"pointer": "/owner", "value": "example-org"}],
            "not_after_unix_ms": 4_000_000_000_000_i64, "max_per_hour": 20
        }],
        "revoked": []
    });
    let mut served = Served::granted(
        &provider,
        "forge.fixture",
        "forge.pat",
        repository_catalog(&provider, Value::Null),
        &grants,
    );
    define_forge(
        &served,
        MERGE_OPENAPI,
        "\x20 user.get: {op: getUser, description: Read the user}\n\
         \x20 pull_request.merge:\n    op: mergePullRequest\n    effect: write\n    body: {merge_method: string}\n",
    );
    served.deliver("..2026_10_30_00_00_00.000000001", &document(TOKEN), 0o400);
    served.start().expect("the served binary starts");
    let descriptor = served.describe().await;
    let revision = descriptor["revision"].as_str().unwrap().to_owned();
    let merge = |request_id: &str, key: &str, pull: u64| {
        json!({
            "version": "v1alpha2", "request_id": request_id, "operation": "pull_request.merge",
            "revision": revision, "idempotency_key": key,
            "input": {"owner": "example-org", "repo": "service", "pull_number": pull,
                      "body": {"merge_method": "squash"}},
        })
    };
    let route = "/v1alpha2/invoke";
    let puts = || {
        provider
            .seen()
            .into_iter()
            .filter(|seen| seen.method == "PUT")
            .count()
    };

    for (pull, status_wanted, code, message) in [
        (1, 403, "forbidden", "Pull Request is not mergeable"),
        (
            2,
            400,
            "invalid_input",
            "Head branch was modified. Review and try the merge again.",
        ),
        (
            3,
            400,
            "invalid_input",
            "Head branch is not up to date with the base branch",
        ),
    ] {
        let before = puts();
        let request_id = format!("merge-{pull}");
        let key = format!("k-merge-{pull}");
        let (status, refused) = served
            .send(
                reqwest::Method::POST,
                route,
                Some(&merge(&request_id, &key, pull)),
            )
            .await;
        assert_eq!(status, status_wanted, "{pull}: {refused}\n{}", served.log());
        assert_eq!(refused["status"], "error", "{pull}: {refused}");
        assert_eq!(refused["error"]["code"], code, "{pull}: {refused}");
        let text = refused["error"]["message"].as_str().unwrap_or_default();
        assert!(
            text.contains(message),
            "{pull}: the provider's message is carried: {refused}"
        );
        assert_eq!(
            refused["mutation"]["classification"], "refused",
            "{pull}: {refused}"
        );
        assert_eq!(refused["mutation"]["replayed"], false, "{pull}: {refused}");
        assert_eq!(
            refused["mutation"]["original_request_id"], request_id,
            "{pull}: {refused}"
        );
        assert!(
            refused["mutation"]["attempt"]["id"].is_string(),
            "{pull}: {refused}"
        );
        assert_eq!(puts(), before + 1, "{pull}: one merge request was sent");

        // The same key and input again: the original refusal, replayed.
        let (status, replay) = served
            .send(
                reqwest::Method::POST,
                route,
                Some(&merge(&format!("{request_id}-again"), &key, pull)),
            )
            .await;
        assert_eq!(status, status_wanted, "{pull}: {replay}\n{}", served.log());
        assert_eq!(replay["error"], refused["error"], "{pull}: {replay}");
        assert_eq!(
            replay["mutation"]["classification"], "refused",
            "{pull}: {replay}"
        );
        assert_eq!(replay["mutation"]["replayed"], true, "{pull}: {replay}");
        assert_eq!(
            replay["mutation"]["attempt"], refused["mutation"]["attempt"],
            "{pull}: {replay}"
        );
        assert_eq!(puts(), before + 1, "{pull}: a replay sends nothing");
    }

    // A cut connection and a server error say nothing about the merge.
    for pull in [4, 5] {
        let before = puts();
        let (status, unknown) = served
            .send(
                reqwest::Method::POST,
                route,
                Some(&merge(
                    &format!("merge-{pull}"),
                    &format!("k-merge-{pull}"),
                    pull,
                )),
            )
            .await;
        assert_eq!(status, 502, "{pull}: {unknown}\n{}", served.log());
        assert_eq!(
            unknown["error"]["code"], "outcome_unknown",
            "{pull}: {unknown}"
        );
        assert_eq!(
            unknown["mutation"]["classification"], "unknown",
            "{pull}: {unknown}"
        );
        assert_eq!(
            puts(),
            before + 1,
            "{pull}: one merge request, never resent"
        );
    }
}

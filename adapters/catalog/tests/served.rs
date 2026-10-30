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
                    let route = target.split('?').next().unwrap_or_default().to_owned();
                    let known = matches!(token.as_deref(), Some(TOKEN | ROTATED));
                    let (status, body) = if !known {
                        (401, json!({"message": "401 Unauthorized"}))
                    } else if method == "GET" && route == "/api/v4/projects/7" {
                        (200, json!({"id": 7, "name": "fixture"}))
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

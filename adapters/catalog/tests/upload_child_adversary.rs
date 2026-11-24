//! Adversarial pass on the upload the provider child sends
//! (story:a-file-upload-through-the-local-cli-is-sent-with-its-ports).
//!
//! The child commits every write through `PreparedWrite::execute_created`,
//! which now sends an upload's three requests inside the one commit deadline.
//! These cases drive the shipped `file.upload` through prepare and commit
//! against a disposable HTTPS fixture that is the Slack Web API below `/api`
//! and the upload host under `/upload/v1/`, on one loopback listener reached
//! as `localhost` (the origin the connection admits) and as `127.0.0.1` (an
//! origin the upload reaches and the connection does not admit).
//!
//! What each case measures:
//!
//! - the commit deadline ending while the bytes are unanswered, which is
//!   unknown with nothing shared, as the guide says of the local CLI;
//! - the commit deadline ending during the share, which must stay unknown and
//!   be sent once;
//! - an admitted upload host with `credential: false`, which must be sent no
//!   bearer header;
//! - an upload URL on an origin the connection does not admit, refused with no
//!   byte sent;
//! - a `401` of an upload host that was sent no credential, refused with
//!   nothing shared;
//! - a prepared upload sent without its slot and byte ports, by the engine's
//!   two other entry points.
//!
//! Every id, name and token is synthetic. No live credential and no network.
#![cfg(unix)]
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::ErrorCode;
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure, PrivateProtocol, WriteEffect, WriteResult},
};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, Secret, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
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

/// A fictional bot token and the header it travels as.
const TOKEN: &str = "fixture-slack-bot-token";
const HEADER: &str = "Bearer fixture-slack-bot-token";
/// The file id the fixture issues with each slot.
const ISSUED: &str = "F0FIXTUREADV";
/// Bytes that are not text, so a re-encoding would change them.
const FILE_BYTES: &[u8] = b"%PDF-1.7\n\xff\xfe\x00\x01 adversary upload\n";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// What the upload host does with a slot's bytes.
#[derive(Clone, Copy)]
enum Bytes {
    /// Answers `200` in text, as Slack does.
    Taken,
    /// Reads the request and never answers.
    Unanswered,
    /// Answers this status in text.
    Status(u16),
}

#[derive(Clone, Copy)]
struct Options {
    /// The `credential` member of the connection's one admitted host.
    credential: bool,
    bytes: Bytes,
    /// The completing write is read and never answered.
    share_unanswered: bool,
    /// The slot issues its URL on `127.0.0.1`, which the upload reaches and
    /// the connection does not admit.
    elsewhere: bool,
}
impl Options {
    fn plain(credential: bool) -> Self {
        Self {
            credential,
            bytes: Bytes::Taken,
            share_unanswered: false,
            elsewhere: false,
        }
    }
}

/// One request the fixture read.
#[derive(Clone, Debug)]
struct Seen {
    method: String,
    route: String,
    query: String,
    authorization: Option<String>,
    content_type: Option<String>,
    length: usize,
    sha256: String,
    json: Value,
}

struct Slack {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    seen: Arc<Mutex<Vec<Seen>>>,
}

async fn answer(
    acceptor: TlsAcceptor,
    stream: tokio::net::TcpStream,
    port: u16,
    options: Options,
    seen: Arc<Mutex<Vec<Seen>>>,
) {
    let Ok(mut stream) = acceptor.accept(stream).await else {
        return;
    };
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() >= 8192 {
            return;
        }
        match stream.read_u8().await {
            Ok(byte) => header.push(byte),
            Err(_) => return,
        }
    }
    let request = String::from_utf8_lossy(&header).into_owned();
    let method = request
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned();
    let path = request
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_owned();
    let value = |wanted: &str| {
        request.lines().find_map(|line| {
            line.split_once(':')
                .filter(|(name, _)| name.eq_ignore_ascii_case(wanted))
                .map(|(_, value)| value.trim().to_owned())
        })
    };
    let length: usize = value("content-length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    if length > 65_536 {
        return;
    }
    let mut body = vec![0; length];
    if stream.read_exact(&mut body).await.is_err() {
        return;
    }
    let (route, query) = path.split_once('?').unwrap_or((path.as_str(), ""));
    seen.lock().unwrap().push(Seen {
        method: method.clone(),
        route: route.to_owned(),
        query: query.to_owned(),
        authorization: value("authorization"),
        content_type: value("content-type"),
        length,
        sha256: hex::encode(Sha256::digest(&body)),
        json: serde_json::from_slice(&body).unwrap_or(Value::Null),
    });
    let json_answer = |value: Value| (200, "application/json", serde_json::to_vec(&value).unwrap());
    let (status, kind, bytes): (u16, &str, Vec<u8>) = match (method.as_str(), route) {
        ("GET", "/api/auth.test") => json_answer(json!({
            "ok": true, "url": "https://fixture.example.test/", "team": "Fixture Team",
            "user": "fixture-bot", "team_id": "T0FIXTURE01", "user_id": "U0FIXTURE01",
            "bot_id": "B0FIXTURE01", "is_enterprise_install": false})),
        ("POST", "/api/files.getUploadURLExternal") => {
            let host = if options.elsewhere {
                "127.0.0.1"
            } else {
                "localhost"
            };
            json_answer(json!({
                "ok": true, "file_id": ISSUED,
                "upload_url": format!("https://{host}:{port}/upload/v1/{ISSUED}")}))
        }
        ("POST", upload) if upload.starts_with("/upload/v1/") => match options.bytes {
            Bytes::Taken => (200, "text/plain", format!("OK - {length}").into_bytes()),
            Bytes::Status(status) => (status, "text/plain", b"fixture refusal".to_vec()),
            Bytes::Unanswered => {
                // Held open until the fixture stops.
                std::future::pending::<()>().await;
                return;
            }
        },
        ("POST", "/api/files.completeUploadExternal") => {
            if options.share_unanswered {
                std::future::pending::<()>().await;
                return;
            }
            let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            json_answer(json!({
                "ok": true,
                "files": [{"id": body["files"][0]["id"], "title": body["files"][0]["title"]}]}))
        }
        _ => (
            404,
            "application/json",
            serde_json::to_vec(&json!({"ok": false, "error": "no fixture"})).unwrap(),
        ),
    };
    let header = format!(
        "HTTP/1.1 {status} fixture\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        bytes.len()
    );
    let _ = stream.write_all(header.as_bytes()).await;
    let _ = stream.write_all(&bytes).await;
}

fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

impl Slack {
    fn start(options: Options) -> Self {
        let root_directory = tempfile::tempdir().unwrap();
        let directory = root_directory.path().join("private");
        filesystem::directory(&directory, true, true).unwrap();
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into(), "127.0.0.1".into()])
            .unwrap();
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
        let (port_tx, port_rx) = std::sync::mpsc::channel();
        let (stop, mut stopped) = oneshot::channel();
        let seen: Arc<Mutex<Vec<Seen>>> = Arc::default();
        let observed = seen.clone();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                let port = listener.local_addr().unwrap().port();
                port_tx.send(port).unwrap();
                let acceptor = TlsAcceptor::from(Arc::new(tls));
                loop {
                    let stream = tokio::select! {
                        _ = &mut stopped => break,
                        value = listener.accept() => value.unwrap().0,
                    };
                    tokio::spawn(answer(
                        acceptor.clone(),
                        stream,
                        port,
                        options,
                        observed.clone(),
                    ));
                }
            });
        });
        let port = port_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let origin = format!("https://localhost:{port}");
        let other = format!("https://127.0.0.1:{port}");
        // The shipped selection, its upload reaching the fixture's two origins
        // in place of `files.slack.com`, which the fixture cannot serve.
        let mut operations: Value = serde_json::from_slice(
            &fs::read(root().join("providers/slack/operations.json")).unwrap(),
        )
        .unwrap();
        let mut reaching = 0;
        for selection in operations["operations"].as_array_mut().unwrap() {
            if selection["id"] == "file.upload" {
                selection["binary"]["hosts"] = json!([origin, other]);
                reaching += 1;
            }
        }
        assert_eq!(reaching, 1, "the shipped `file.upload`");
        let operations_file = directory.join("operations.json");
        private(&operations_file, &serde_json::to_vec(&operations).unwrap());
        let config = directory.join("catalog.json");
        let document = json!({
            "format": "connectors-catalog-local/2",
            "instance": "fixture-slack",
            "provider": "slack",
            "bundle_directory": root().join("generated/bundles").canonicalize().unwrap(),
            "api_base": format!("{origin}/api"),
            "ca_file": ca,
            "hosts": [{"origin": origin, "credential": options.credential}],
            "auth": {
                "profile": "slack.bot",
                "header": "Authorization",
                "bearer": true,
                "label": "Slack bot token",
                "identity": {"path": "auth.test", "kind": "slack.user",
                             "subject_pointer": "/user_id"}
            },
            "operations_file": operations_file,
        });
        private(&config, &serde_json::to_vec(&document).unwrap());
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root_directory,
            config,
            seen,
        }
    }

    /// The provider child's launch on private protocol two, which carries writes.
    fn adapter(&self) -> Adapter {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
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
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            standing_grants: None,
            credential_revision: None,
            private_protocol: Some(PrivateProtocol::V2),
            permissions: Default::default(),
            instance_id: "fixture-slack".into(),
            adapter_id: "catalog".into(),
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

    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }

    /// The routes of the POSTs the fixture read, in order.
    fn posts(&self) -> Vec<String> {
        self.seen()
            .into_iter()
            .filter(|seen| seen.method == "POST")
            .map(|seen| seen.route)
            .collect()
    }
}
impl Drop for Slack {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn upload_input() -> Value {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    json!({"body": {"channel_id": "C0FIXTURE01", "thread_ts": "1780000100.000100",
                    "filename": "fixture-1.pdf", "title": "Fixture file",
                    "initial_comment": "fixture file uploaded"},
           "content": {"base64": STANDARD.encode(FILE_BYTES),
                       "sha256": hex::encode(Sha256::digest(FILE_BYTES)),
                       "media_type": "application/pdf"}})
}

/// Prepare and commit one `file.upload` on this child, as the local owner
/// does once it holds an approval, with `within_ms` as the one deadline of
/// both. A refusal in prepare is the `Err`.
fn upload(child: &mut Child, within_ms: u64) -> Result<WriteResult, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let prepared = child.prepare_write(
        "file.upload",
        &revision,
        "one",
        &Secret(serde_json::to_vec(&json!({"token": TOKEN})).unwrap()),
        &serde_json::to_vec(&upload_input()).unwrap(),
        connectors_sdk::now_ms() + within_ms,
    )?;
    Ok(prepared.commit())
}

const SLOT: &str = "/api/files.getUploadURLExternal";
const SHARE: &str = "/api/files.completeUploadExternal";
fn bytes_route() -> String {
    format!("/upload/v1/{ISSUED}")
}

/// docs/catalog-slack.md, "The share": a refusal or failure of the first two
/// requests is `refused`, with one case that differs through the local CLI.
/// Through the provider child the three requests run inside one commit
/// deadline, and that deadline ending while the byte request is unanswered
/// drops the whole commit as unknown: the owner records `outcome_unknown` with
/// `timeout` for an upload whose completing write was never sent. The guide
/// names that case; this holds it, and that nothing is shared.
#[test]
fn a_commit_deadline_ending_while_the_bytes_are_unanswered_is_unknown_and_shares_nothing() {
    let slack = Slack::start(Options {
        bytes: Bytes::Unanswered,
        ..Options::plain(true)
    });
    let mut child = Child::spawn(&slack.adapter()).unwrap();
    let result = upload(&mut child, 4_000)
        .unwrap_or_else(|failure| panic!("refused in prepare: {failure:?}"));
    // Nothing was shared: the completing write never left.
    assert_eq!(slack.posts(), [SLOT, bytes_route().as_str()]);
    assert_eq!(result.effect, WriteEffect::Unknown, "{result:?}");
    assert!(matches!(result.result, Err(Failure::Timeout)), "{result:?}");
}

/// The other side of the same deadline: once the completing write is sent,
/// its end is unknown, never refused and never applied, and it is sent once.
#[test]
fn a_commit_deadline_ending_during_the_share_is_unknown_and_shared_once() {
    let slack = Slack::start(Options {
        share_unanswered: true,
        ..Options::plain(true)
    });
    let mut child = Child::spawn(&slack.adapter()).unwrap();
    let result = upload(&mut child, 4_000)
        .unwrap_or_else(|failure| panic!("refused in prepare: {failure:?}"));
    assert_eq!(result.effect, WriteEffect::Unknown, "{result:?}");
    assert_eq!(slack.posts(), [SLOT, bytes_route().as_str(), SHARE]);
}

/// An upload host admitted with `credential: false` takes the bytes without
/// the connection's bearer header; the slot and the share, on the API base,
/// carry it. The slot's query is the file's name and length and nothing else.
#[test]
fn an_upload_host_admitted_without_the_credential_is_sent_the_bytes_and_no_bearer() {
    let slack = Slack::start(Options::plain(false));
    let mut child = Child::spawn(&slack.adapter()).unwrap();
    let result = upload(&mut child, 30_000)
        .unwrap_or_else(|failure| panic!("refused in prepare: {failure:?}"));
    assert_eq!(result.effect, WriteEffect::Applied, "{result:?}");
    let seen = slack.seen();
    let routes: Vec<&str> = seen.iter().map(|seen| seen.route.as_str()).collect();
    assert_eq!(
        routes,
        ["/api/auth.test", SLOT, bytes_route().as_str(), SHARE],
        "{seen:?}"
    );
    assert_eq!(
        seen[1].query,
        format!("filename=fixture-1.pdf&length={}", FILE_BYTES.len())
    );
    assert_eq!(seen[1].length, 0, "the slot carries no body");
    assert_eq!(seen[2].length, FILE_BYTES.len());
    assert_eq!(seen[2].sha256, hex::encode(Sha256::digest(FILE_BYTES)));
    assert_eq!(seen[2].content_type.as_deref(), Some("application/pdf"));
    assert_eq!(
        seen[2].authorization, None,
        "the upload host was sent the credential"
    );
    for api in [&seen[0], &seen[1], &seen[3]] {
        assert_eq!(api.authorization.as_deref(), Some(HEADER), "{}", api.route);
    }
    assert_eq!(
        seen[3].json,
        json!({"files": [{"id": ISSUED, "title": "Fixture file"}],
               "channel_id": "C0FIXTURE01", "thread_ts": "1780000100.000100",
               "initial_comment": "fixture file uploaded"})
    );
}

/// docs/catalog-slack.md: a connection that does not admit the upload URL's
/// host "refuses every upload as `forbidden` after
/// `files.getUploadURLExternal`, with no byte sent". Here the URL is on an
/// origin the upload reaches and the connection does not admit.
#[test]
fn an_upload_url_on_an_origin_the_connection_does_not_admit_is_refused_with_no_byte_sent() {
    let slack = Slack::start(Options {
        elsewhere: true,
        ..Options::plain(true)
    });
    let mut child = Child::spawn(&slack.adapter()).unwrap();
    let result = upload(&mut child, 30_000)
        .unwrap_or_else(|failure| panic!("refused in prepare: {failure:?}"));
    assert_eq!(result.effect, WriteEffect::Refused, "{result:?}");
    assert!(
        matches!(result.result, Err(Failure::Forbidden)),
        "{result:?}"
    );
    assert_eq!(slack.posts(), [SLOT]);
}

/// An upload host admitted with `credential: false` is sent no credential, and
/// its `401` refuses the upload with nothing shared. The code it is refused
/// with today is `invalid_credential` (`unshared_refusal` maps every `401` to
/// `unauthorized`), which says more than the answer does; the review of this
/// change records that as a finding, and this case holds only what a consumer
/// relies on: refused, no share, no credential sent to that host.
#[test]
fn a_401_of_an_upload_host_that_was_sent_no_credential_refuses_and_shares_nothing() {
    let slack = Slack::start(Options {
        bytes: Bytes::Status(401),
        ..Options::plain(false)
    });
    let mut child = Child::spawn(&slack.adapter()).unwrap();
    let result = upload(&mut child, 30_000)
        .unwrap_or_else(|failure| panic!("refused in prepare: {failure:?}"));
    assert_eq!(result.effect, WriteEffect::Refused, "{result:?}");
    assert_eq!(slack.posts(), [SLOT, bytes_route().as_str()]);
    let seen = slack.seen();
    assert_eq!(seen[2].authorization, None, "{seen:?}");
}

/// Answers every read `ok`, as `auth.test` answers an accepted token.
struct Accepted;
#[async_trait::async_trait]
impl AuthenticatedHttp for Accepted {
    async fn get(&self, _: &[&str], _: &[(&str, String)]) -> connectors_core::Result<HttpResponse> {
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: serde_json::to_vec(&json!({"ok": true, "user_id": "U0FIXTURE01"})).unwrap(),
        })
    }
}

/// A write port that records what it is asked to send and answers nothing.
struct Recording(Arc<Mutex<Vec<String>>>);
#[async_trait::async_trait]
impl AuthenticatedWrite for Recording {
    async fn send_json(
        self: Box<Self>,
        _: WriteMethod,
        segments: &[&str],
        _: &[(&str, String)],
        _: &Value,
    ) -> connectors_core::Result<HttpResponse> {
        self.0.lock().unwrap().push(segments.join("/"));
        Err(connectors_core::Error::unavailable())
    }
    async fn send_bytes(
        self: Box<Self>,
        _: WriteMethod,
        segments: &[&str],
        _: &[(&str, String)],
        _: &str,
        _: &[u8],
    ) -> connectors_core::Result<HttpResponse> {
        self.0.lock().unwrap().push(segments.join("/"));
        Err(connectors_core::Error::unavailable())
    }
}

/// The story's acceptance 3: "a prepared upload is still refused when it is
/// sent without its ports". The provider child no longer reaches that guard,
/// and no other test did: the completing write alone would share a file id
/// whose slot was never asked and whose bytes were never sent.
#[tokio::test]
async fn a_prepared_upload_sent_without_its_ports_is_refused_with_nothing_sent() {
    let file: Value =
        serde_json::from_slice(&fs::read(root().join("providers/slack/operations.json")).unwrap())
            .unwrap();
    let shipped: Vec<Selection> = serde_json::from_value(file["operations"].clone()).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), "slack").unwrap();
    let engine = Engine::new(&bundle, "/api", &shipped).unwrap();
    let sent: Arc<Mutex<Vec<String>>> = Arc::default();
    let prepare = || async {
        let prepared = engine
            .prepare(&Accepted, "fixture-slack", "file.upload", upload_input())
            .await
            .unwrap_or_else(|error| panic!("refused in prepare: {}", error.message));
        assert!(prepared.uploads());
        prepared
    };
    let refused = |error: connectors_core::Error| {
        assert_eq!(error.code, ErrorCode::Forbidden, "{}", error.message);
        assert!(
            error.message.contains("slot and byte ports"),
            "{}",
            error.message
        );
    };
    match prepare()
        .await
        .execute_holding(Box::new(Recording(sent.clone())), None)
        .await
    {
        WriteOutcome::Refused(error) => refused(error),
        _ => panic!("`execute_holding` sent an upload without its ports"),
    }
    match prepare()
        .await
        .execute(Box::new(Recording(sent.clone())))
        .await
    {
        WriteOutcome::Refused(error) => refused(error),
        _ => panic!("`execute` sent an upload without its ports"),
    }
    assert!(
        sent.lock().unwrap().is_empty(),
        "{:?}",
        sent.lock().unwrap()
    );
}

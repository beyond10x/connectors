//! Adversarial cases for story:slack-single-user-lookups and
//! story:parity-slack-file-delete: `users.info`, `users.lookupByEmail` and the
//! guarded `files.delete`, run through the provider child against a disposable
//! HTTPS fixture whose answers each case writes. Every id, address and token is
//! synthetic. No live credential and no network.
#![cfg(unix)]
use connectors_catalog_provider::Selection;
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure, PrivateProtocol, Refusal, WriteEffect, WriteResult},
};
use connectors_sdk::Secret;
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

/// A fictional bot token.
const TOKEN: &str = "fixture-slack-bot-token";
const HEADER: &str = "Bearer fixture-slack-bot-token";
const PROFILE: &str = "slack.bot";

/// One request the fixture read.
#[derive(Clone, Debug)]
struct Exchange {
    method: String,
    path: String,
    route: String,
    query: String,
    authorization: Option<String>,
    token_header: bool,
    body: Value,
}

/// What the fixture answers: JSON with a status, raw bytes labelled JSON, or
/// nothing at all, the connection closed after the request was read.
#[derive(Clone, Debug)]
enum Reply {
    Json(u16, Value),
    Raw(u16, Vec<u8>),
    Drop,
}

type Responder = Arc<dyn Fn(&Exchange) -> Reply + Send + Sync>;
type Exchanges = Arc<Mutex<Vec<Exchange>>>;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn root_path(relative: &str) -> PathBuf {
    root().join(relative).canonicalize().unwrap()
}
fn shipped() -> Vec<Selection> {
    let file: Value =
        serde_json::from_slice(&fs::read(root().join("providers/slack/operations.json")).unwrap())
            .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn guide() -> String {
    fs::read_to_string(root().join("../../docs/catalog-slack.md")).unwrap()
}
/// The guide's bot configuration example.
fn documented_config() -> Value {
    let guide = guide();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| {
            body.contains("\"provider\": \"slack\"")
                && body.contains(&format!("\"profile\": \"{PROFILE}\""))
        })
        .expect("the documented bot configuration");
    serde_json::from_str::<Value>(example).unwrap()
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn identity() -> Value {
    json!({"ok": true, "url": "https://fixture.example.test/", "team": "Fixture Team",
           "user": "fixture-bot", "team_id": "T0FIXTURE01", "user_id": "U0FIXTURE01",
           "bot_id": "B0FIXTURE01", "is_enterprise_install": false})
}
fn user(n: u64) -> Value {
    json!({"id": format!("U0FIXTURE{n:02}"), "name": format!("fixture-user-{n}"),
           "deleted": false, "is_bot": false, "is_app_user": false,
           "profile": {"real_name": format!("Fixture User {n}")},
           "team_id": "T0FIXTURE01", "updated": 1_780_000_000u64 + n})
}

/// The raw `key=value` pairs of a query, undecoded.
fn pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key.to_owned(), value.to_owned())
        })
        .collect()
}
/// A query component as a form decoder reads it, as Slack's does: `+` is a
/// space and `%XX` a byte.
fn form_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' => match text
                .get(i + 1..i + 3)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            {
                Some(byte) => {
                    out.push(byte);
                    i += 3;
                }
                None => {
                    out.push(b'%');
                    i += 1;
                }
            },
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
/// The one value of `key` in a query, decoded, or `None` when the query
/// carries it other than exactly once.
fn one(query: &str, key: &str) -> Option<String> {
    let found: Vec<String> = pairs(query)
        .into_iter()
        .filter(|(k, _)| form_decode(k) == key)
        .map(|(_, v)| form_decode(&v))
        .collect();
    (found.len() == 1).then(|| found[0].clone())
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    exchanges: Exchanges,
    protocol: Option<PrivateProtocol>,
}
impl Provider {
    fn reading(responder: Responder) -> Self {
        Self::start(None, responder)
    }
    fn writing(responder: Responder) -> Self {
        Self::start(Some(PrivateProtocol::V2), responder)
    }
    fn start(protocol: Option<PrivateProtocol>, responder: Responder) -> Self {
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
        let exchanges: Exchanges = Arc::default();
        let recorded = exchanges.clone();
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
                        value = listener.accept() => value.unwrap().0,
                    };
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        continue;
                    };
                    let mut header = Vec::new();
                    while !header.ends_with(b"\r\n\r\n") {
                        assert!(header.len() < 8192);
                        match stream.read_u8().await {
                            Ok(byte) => header.push(byte),
                            Err(_) => break,
                        }
                    }
                    let request = String::from_utf8_lossy(&header).into_owned();
                    let header_value = |wanted: &str| {
                        request.lines().find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case(wanted))
                                .map(|(_, value)| value.trim().to_owned())
                        })
                    };
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
                    let (route, query) = path.split_once('?').unwrap_or((&path, ""));
                    let length: usize = header_value("content-length")
                        .map(|value| value.parse().unwrap())
                        .unwrap_or(0);
                    assert!(length <= 65_536);
                    let mut sent = vec![0; length];
                    if stream.read_exact(&mut sent).await.is_err() {
                        continue;
                    }
                    let exchange = Exchange {
                        method,
                        route: route.to_owned(),
                        query: query.to_owned(),
                        path: path.clone(),
                        authorization: header_value("authorization"),
                        token_header: header_value("token").is_some(),
                        body: if sent.is_empty() {
                            Value::Null
                        } else {
                            serde_json::from_slice(&sent).unwrap_or(Value::Null)
                        },
                    };
                    recorded.lock().unwrap().push(exchange.clone());
                    let reply = if exchange.authorization.as_deref() != Some(HEADER) {
                        Reply::Json(200, json!({"ok": false, "error": "invalid_auth"}))
                    } else if exchange.route == "/api/auth.test" {
                        Reply::Json(200, identity())
                    } else {
                        responder(&exchange)
                    };
                    let (status, bytes) = match reply {
                        Reply::Json(status, value) => (status, serde_json::to_vec(&value).unwrap()),
                        Reply::Raw(status, bytes) => (status, bytes),
                        Reply::Drop => continue,
                    };
                    let header = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    );
                    let _ = stream.write_all(header.as_bytes()).await;
                    let _ = stream.write_all(&bytes).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let config = directory.join("catalog.json");
        let mut document = documented_config();
        document["instance"] = json!("fixture-slack");
        document["bundle_directory"] = json!(root_path("generated/bundles"));
        document["api_base"] = json!(format!("https://localhost:{}/api", address.port()));
        document["ca_file"] = json!(ca);
        document["operations_file"] = json!(root_path("providers/slack/operations.json"));
        private(&config, &serde_json::to_vec(&document).unwrap());
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config,
            exchanges,
            protocol,
        }
    }
    fn selection(&self) -> Adapter {
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
            credential_revision: None,
            private_protocol: self.protocol,
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
    fn exchanges(&self) -> Vec<Exchange> {
        self.exchanges.lock().unwrap().clone()
    }
    fn to(&self, route: &str) -> Vec<Exchange> {
        self.exchanges()
            .into_iter()
            .filter(|exchange| exchange.route == route)
            .collect()
    }
    fn posts(&self) -> Vec<Exchange> {
        self.exchanges()
            .into_iter()
            .filter(|exchange| exchange.method == "POST")
            .collect()
    }
    /// The token travels in the `Authorization` header alone: never a `token`
    /// header, query pair or body member, and never its text anywhere else.
    fn assert_token_in_the_header_alone(&self) {
        for exchange in self.exchanges() {
            assert_eq!(
                exchange.authorization.as_deref(),
                Some(HEADER),
                "{}",
                exchange.path
            );
            assert!(!exchange.token_header, "{}", exchange.path);
            assert!(!exchange.path.contains(TOKEN), "{}", exchange.path);
            assert!(
                pairs(&exchange.query)
                    .iter()
                    .all(|(key, _)| !form_decode(key).eq_ignore_ascii_case("token")),
                "{}",
                exchange.path
            );
            assert!(
                !exchange.body.to_string().contains(TOKEN),
                "{}",
                exchange.body
            );
        }
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn secret() -> Secret {
    Secret(serde_json::to_vec(&json!({"token": TOKEN})).unwrap())
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
fn attempt(child: &mut Child, operation: &str, input: &Value) -> Result<Vec<u8>, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child.invoke(
        operation,
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        deadline(),
    )
}
fn explained(child: &mut Child, operation: &str, input: &Value) -> Result<Vec<u8>, Refusal> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child.invoke_explained(
        operation,
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        deadline(),
    )
}
fn write(child: &mut Child, operation: &str, input: &Value) -> Result<WriteResult, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let prepared = child.prepare_write(
        operation,
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        deadline(),
    )?;
    Ok(prepared.commit())
}
fn delete(file: &str) -> Value {
    json!({"body": {"file": file}})
}

/// `files.info` answering the file it was asked for, by its one decoded
/// `file` value, and `files.delete` answering `ok` true.
fn echoing(exchange: &Exchange) -> Reply {
    match exchange.route.as_str() {
        "/api/files.info" => match one(&exchange.query, "file") {
            Some(id) => Reply::Json(
                200,
                json!({"ok": true, "file": {"id": id, "name": "fixture.pdf"}, "comments": []}),
            ),
            None => Reply::Json(200, json!({"ok": false, "error": "file_not_found"})),
        },
        "/api/files.delete" => Reply::Json(200, json!({"ok": true})),
        _ => Reply::Json(404, json!({"ok": false, "error": "no fixture"})),
    }
}

/// A responder serving `route` whatever the test last put in `slot`, and
/// [`echoing`] otherwise.
fn scripted(route: &'static str, slot: Arc<Mutex<Reply>>) -> Responder {
    Arc::new(move |exchange: &Exchange| {
        if exchange.route == route {
            slot.lock().unwrap().clone()
        } else {
            echoing(exchange)
        }
    })
}

// ── files.delete: the file read before the write ─────────────────────────────

/// The guard pins `/file/id` to `body.file`: an answer naming another file,
/// a look-alike of it, or no file id at all stops the delete with nothing
/// posted.
#[test]
fn a_delete_is_never_sent_when_files_info_names_another_file_or_none() {
    let slot = Arc::new(Mutex::new(Reply::Drop));
    let provider = Provider::writing(scripted("/api/files.info", slot.clone()));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let answers = [
        json!({"ok": true, "file": {"id": "F0FIXTURE02"}}),
        json!({"ok": true, "file": {"id": "f0fixture01"}}),
        json!({"ok": true, "file": {"id": "F0FIXTURE01 "}}),
        json!({"ok": true, "file": {"id": " F0FIXTURE01"}}),
        json!({"ok": true, "file": {"id": ""}}),
        json!({"ok": true, "file": {"id": null}}),
        json!({"ok": true, "file": {"id": ["F0FIXTURE01"]}}),
        json!({"ok": true, "file": {"name": "F0FIXTURE01"}}),
        json!({"ok": true, "file": null}),
        json!({"ok": true, "id": "F0FIXTURE01"}),
        json!({"ok": true, "files": [{"id": "F0FIXTURE01"}]}),
    ];
    for answer in &answers {
        *slot.lock().unwrap() = Reply::Json(200, answer.clone());
        let outcome = write(&mut child, "files.delete", &delete("F0FIXTURE01"));
        assert!(
            outcome.is_err(),
            "{answer}: {:?}",
            outcome.map(|result| result.effect)
        );
        assert!(provider.posts().is_empty(), "{answer}: a delete was sent");
    }
    assert_eq!(provider.to("/api/files.info").len(), answers.len());
    provider.assert_token_in_the_header_alone();
}

/// Any `files.info` answer but `200` with the JSON boolean `ok: true` stops
/// the delete: an `ok: false` with an error the delete does not list, none,
/// a look-alike `ok`, a body that is not JSON, a status that is not `200`, or
/// no answer at all.
#[test]
fn a_delete_is_never_sent_when_files_info_answers_anything_but_ok_true() {
    let slot = Arc::new(Mutex::new(Reply::Drop));
    let provider = Provider::writing(scripted("/api/files.info", slot.clone()));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let file = json!({"id": "F0FIXTURE01", "name": "fixture.pdf"});
    let replies = [
        Reply::Json(200, json!({"ok": false, "error": "fixture_unlisted"})),
        Reply::Json(200, json!({"ok": false, "error": "ratelimited"})),
        Reply::Json(200, json!({"ok": false, "error": "invalid_auth"})),
        Reply::Json(200, json!({"ok": false, "error": "file_deleted"})),
        Reply::Json(200, json!({"ok": false, "file": file})),
        Reply::Json(200, json!({"ok": "true", "file": file})),
        Reply::Json(200, json!({"ok": 1, "file": file})),
        Reply::Json(200, json!({"ok": null, "file": file})),
        Reply::Json(200, json!({"file": file})),
        Reply::Json(200, json!([{"ok": true, "file": file}])),
        Reply::Raw(200, b"ok true F0FIXTURE01".to_vec()),
        Reply::Json(404, json!({"ok": true, "file": file})),
        Reply::Json(500, json!({"ok": true, "file": file})),
        Reply::Json(403, json!({"ok": true, "file": file})),
        Reply::Drop,
    ];
    for reply in &replies {
        *slot.lock().unwrap() = reply.clone();
        let outcome = write(&mut child, "files.delete", &delete("F0FIXTURE01"));
        assert!(
            outcome.is_err(),
            "{reply:?}: {:?}",
            outcome.map(|result| result.effect)
        );
        assert!(provider.posts().is_empty(), "{reply:?}: a delete was sent");
    }
    provider.assert_token_in_the_header_alone();
}

/// Only `200` with the JSON boolean `ok: true` is applied. Every other
/// acknowledgement is refused (a declared name) or unknown, and each delete
/// was posted exactly once.
#[test]
fn no_acknowledgement_but_ok_true_is_applied_and_each_is_sent_once() {
    let slot = Arc::new(Mutex::new(Reply::Drop));
    let provider = Provider::writing(scripted("/api/files.delete", slot.clone()));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let cases = [
        (
            Reply::Json(200, json!({"ok": false, "error": "file_not_found"})),
            Some(WriteEffect::Refused),
        ),
        (
            Reply::Json(200, json!({"ok": "true", "error": "cant_delete_file"})),
            None,
        ),
        (Reply::Json(200, json!({"ok": false})), None),
        (
            Reply::Json(200, json!({"ok": false, "error": "no_permission"})),
            None,
        ),
        (
            Reply::Json(200, json!({"ok": false, "error": "invalid_auth"})),
            None,
        ),
        (
            Reply::Json(200, json!({"ok": false, "error": "fixture_unlisted"})),
            None,
        ),
        (
            Reply::Json(200, json!({"ok": false, "error": ["file_deleted"]})),
            None,
        ),
        (Reply::Json(200, json!({"ok": "true"})), None),
        (Reply::Json(200, json!({"ok": 1})), None),
        (Reply::Json(200, json!({"ok": null})), None),
        (Reply::Json(200, json!({})), None),
        (Reply::Json(200, json!([{"ok": true}])), None),
        (Reply::Raw(200, b"ok".to_vec()), None),
        (Reply::Raw(200, Vec::new()), None),
        (Reply::Json(429, json!({"ok": true})), None),
        (Reply::Json(500, json!({"ok": true})), None),
        (Reply::Drop, Some(WriteEffect::Unknown)),
    ];
    for (n, (reply, effect)) in cases.iter().enumerate() {
        *slot.lock().unwrap() = reply.clone();
        let file = format!("F0FIXTURE{:02}", 10 + n);
        let result = write(&mut child, "files.delete", &delete(&file))
            .unwrap_or_else(|failure| panic!("{reply:?} refused in prepare: {failure:?}"));
        assert_ne!(result.effect, WriteEffect::Applied, "{reply:?}");
        assert!(result.result.is_err(), "{reply:?}");
        if let Some(effect) = effect {
            assert_eq!(result.effect, *effect, "{reply:?}");
        }
        let posts = provider.posts();
        assert_eq!(posts.len(), n + 1, "{reply:?} sent once");
        assert_eq!(posts[n].body, json!({"file": file}), "{reply:?}");
    }
    provider.assert_token_in_the_header_alone();
}

/// A file id holding characters that would change a query (`&`, `=`, `%`,
/// `+`, `#`, `;`, a space, a NUL, non-ASCII) reaches `files.info` as one
/// `file` value equal to it, and the delete body as itself.
#[test]
fn a_file_id_that_could_change_the_request_is_read_and_deleted_as_itself() {
    let provider = Provider::writing(Arc::new(echoing));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let ids = [
        "F0FIX&file=F0FIXTURE02",
        "F0FIX%26file%3DF0FIXTURE02",
        "F0FIX+1",
        "F0FIX 1",
        "F0FIX#token=x",
        "F0FIX;file=F0FIXTURE02",
        "F0FIX?file=F0FIXTURE02",
        "F0FIX\u{0}",
        "F0F\u{130}XTURE",
        "F0FIX/../users.list",
    ];
    for (n, id) in ids.iter().enumerate() {
        let result = write(&mut child, "files.delete", &delete(id))
            .unwrap_or_else(|failure| panic!("{id:?} refused in prepare: {failure:?}"));
        assert_eq!(result.effect, WriteEffect::Applied, "{id:?}");
        let reads = provider.to("/api/files.info");
        let read = &reads[n];
        let sent = pairs(&read.query);
        assert_eq!(sent.len(), 1, "{id:?}: {}", read.path);
        assert_eq!(
            one(&read.query, "file").as_deref(),
            Some(*id),
            "{}",
            read.path
        );
        let posts = provider.posts();
        assert_eq!(posts[n].route, "/api/files.delete", "{id:?}");
        assert!(posts[n].query.is_empty(), "{id:?}: {}", posts[n].path);
        assert_eq!(posts[n].body, json!({"file": id}), "{id:?}");
    }
    provider.assert_token_in_the_header_alone();
}

/// The body is closed to one `file` string, and no credential is admitted
/// under any key or case, beside the body or in it: each is refused before
/// any request, the two reads included.
#[test]
fn files_delete_admits_one_file_string_and_no_credential_anywhere() {
    let provider = Provider::writing(Arc::new(echoing));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let file = "F0FIXTURE01";
    let inputs = [
        json!({"body": {"file": file}, "file": "F0FIXTURE02"}),
        json!({"body": {"file": file}, "token": TOKEN}),
        json!({"body": {"file": file}, "query:token": TOKEN}),
        json!({"body": {"file": file}, "header:token": TOKEN}),
        json!({"body": {"file": file}, "query:file": "F0FIXTURE02"}),
        json!({"body": {"file": file, "Token": TOKEN}}),
        json!({"body": {"file": file, "TOKEN": TOKEN}}),
        json!({"body": {"file": file, "file ": "F0FIXTURE02"}}),
        json!({"body": {"file": file, "files": ["F0FIXTURE02"]}}),
        json!({"body": {"file": [file]}}),
        json!({"body": {"file": {"id": file}}}),
        json!({"body": {"file": true}}),
        json!({"body": {"file": 1}}),
        json!({"body": {"file": null}}),
        json!({"body": file}),
        json!({"body": null}),
        json!({"body": [{"file": file}]}),
        json!({"file": file}),
        json!({}),
    ];
    for input in &inputs {
        let outcome = write(&mut child, "files.delete", input);
        assert!(
            outcome.is_err(),
            "{input}: {:?}",
            outcome.map(|result| result.effect)
        );
        assert!(
            provider.exchanges().is_empty(),
            "{input}: {:?}",
            provider.exchanges()
        );
    }
}

// ── users.info and users.lookupByEmail ───────────────────────────────────────

/// Slack's `200` with anything but the JSON boolean `ok: true` is never
/// returned as data: an `ok: false` naming an error outside the envelope, or
/// none, is `internal`; a look-alike `ok` or none is refused too. Each sent
/// once.
#[test]
fn a_one_user_read_answered_outside_the_envelope_is_never_data() {
    let slot = Arc::new(Mutex::new(Reply::Drop));
    let reply = slot.clone();
    let provider = Provider::reading(Arc::new(move |exchange: &Exchange| {
        match exchange.route.as_str() {
            "/api/users.info" | "/api/users.lookupByEmail" => reply.lock().unwrap().clone(),
            _ => Reply::Json(404, json!({"ok": false, "error": "no fixture"})),
        }
    }));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let cases = [
        (
            json!({"ok": false, "error": "fixture_unlisted", "user": user(11)}),
            Some(Failure::ProviderInternal),
        ),
        (
            json!({"ok": false, "user": user(11)}),
            Some(Failure::ProviderInternal),
        ),
        (
            json!({"ok": false, "error": 7, "user": user(11)}),
            Some(Failure::ProviderInternal),
        ),
        (
            json!({"ok": false, "error": {"name": "user_not_found"}}),
            Some(Failure::ProviderInternal),
        ),
        (json!({"ok": "true", "user": user(11)}), None),
        (json!({"ok": "false", "user": user(11)}), None),
        (json!({"ok": 1, "user": user(11)}), None),
        (json!({"ok": null, "user": user(11)}), None),
        (json!({"user": user(11)}), None),
        (json!([{"ok": true, "user": user(11)}]), None),
    ];
    let mut sent = 0;
    for (operation, input) in [
        ("users.info", json!({"user": "U0FIXTURE11"})),
        (
            "users.lookupByEmail",
            json!({"email": "fixture-user-11@example.test"}),
        ),
    ] {
        for (answer, failure) in &cases {
            *slot.lock().unwrap() = Reply::Json(200, answer.clone());
            let outcome = explained(&mut child, operation, &input);
            sent += 1;
            let refusal = match outcome {
                Ok(bytes) => panic!(
                    "`{operation}` {answer} answered as data: {}",
                    String::from_utf8_lossy(&bytes)
                ),
                Err(refusal) => refusal,
            };
            if let Some(failure) = failure {
                assert_eq!(refusal.failure, *failure, "`{operation}` {answer}");
            }
            assert_eq!(provider.exchanges().len(), sent, "`{operation}` sent once");
        }
    }
    provider.assert_token_in_the_header_alone();
}

/// The envelope names its three new errors exactly: a near spelling is
/// `internal`, on the two new reads and on the reads that had the envelope
/// before.
#[test]
fn the_new_envelope_names_match_exactly_and_nothing_near_them() {
    let slot = Arc::new(Mutex::new(Reply::Drop));
    let reply = slot.clone();
    let provider = Provider::reading(Arc::new(move |_: &Exchange| reply.lock().unwrap().clone()));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let reads = [
        ("users.info", json!({"user": "U0FIXTURE11"})),
        (
            "users.lookupByEmail",
            json!({"email": "fixture-user-11@example.test"}),
        ),
        ("files.info", json!({"file": "F0FIXTURE01"})),
        ("users.list", json!({})),
        ("team.info", json!({})),
    ];
    for near in [
        "user_not_visible ",
        "USER_NOT_VISIBLE",
        "users_not_found_",
        "user_not_founds",
        "enterprise_is_restricted.",
        "Enterprise_is_restricted",
    ] {
        *slot.lock().unwrap() = Reply::Json(200, json!({"ok": false, "error": near}));
        for (operation, input) in &reads {
            let refusal = explained(&mut child, operation, input).expect_err(near);
            assert_eq!(
                refusal.failure,
                Failure::ProviderInternal,
                "`{operation}` `{near}`"
            );
        }
    }
}

/// An address holding `+`, `%`, a space, `&`, `=` or non-ASCII reaches Slack
/// as one `email` value that a form decoder reads back as the address itself.
#[test]
fn an_email_reaches_slack_as_itself_whatever_it_holds() {
    let provider = Provider::reading(Arc::new(|exchange: &Exchange| {
        match exchange.route.as_str() {
            "/api/users.lookupByEmail" => Reply::Json(200, json!({"ok": true, "user": user(11)})),
            _ => Reply::Json(404, json!({"ok": false, "error": "no fixture"})),
        }
    }));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let emails = [
        "fixture+tag@example.test",
        "fixture%40other@example.test",
        "fixture%2Btag@example.test",
        "fixture user@example.test",
        "fixture&token=x@example.test",
        "fixture&email=other@example.test",
        "fixture=1@example.test",
        "\"fixture\"@example.test",
        "f\u{ef}xture@example.test",
        "fixture#x@example.test",
    ];
    for (n, email) in emails.iter().enumerate() {
        let body: Value = serde_json::from_slice(
            &attempt(&mut child, "users.lookupByEmail", &json!({"email": email}))
                .unwrap_or_else(|failure| panic!("{email:?}: {failure:?}")),
        )
        .unwrap();
        assert_eq!(body["body"]["user"]["id"], "U0FIXTURE11", "{email:?}");
        let exchange = &provider.to("/api/users.lookupByEmail")[n];
        assert_eq!(
            pairs(&exchange.query).len(),
            1,
            "{email:?}: {}",
            exchange.path
        );
        assert_eq!(
            one(&exchange.query, "email").as_deref(),
            Some(*email),
            "{email:?} sent as {}",
            exchange.path
        );
    }
    provider.assert_token_in_the_header_alone();
}

/// A missing `user` or `email`, one that is not one scalar, the other read's
/// key, an undeclared key or the credential under any key is refused before
/// any request.
#[test]
fn a_missing_or_unusable_user_or_email_sends_nothing() {
    let provider = Provider::reading(Arc::new(|_: &Exchange| {
        Reply::Json(200, json!({"ok": true, "user": user(11)}))
    }));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let cases = [
        ("users.info", json!({})),
        ("users.info", json!({"user": null})),
        (
            "users.info",
            json!({"user": ["U0FIXTURE11", "U0FIXTURE12"]}),
        ),
        ("users.info", json!({"user": {"id": "U0FIXTURE11"}})),
        (
            "users.info",
            json!({"email": "fixture-user-11@example.test"}),
        ),
        (
            "users.info",
            json!({"user": "U0FIXTURE11", "query:token": TOKEN}),
        ),
        (
            "users.info",
            json!({"user": "U0FIXTURE11", "header:token": TOKEN}),
        ),
        ("users.info", json!({"user": "U0FIXTURE11", "Token": TOKEN})),
        ("users.info", json!({"user": "U0FIXTURE11", "cursor": "x"})),
        ("users.lookupByEmail", json!({})),
        ("users.lookupByEmail", json!({"email": null})),
        (
            "users.lookupByEmail",
            json!({"email": ["a@example.test", "b@example.test"]}),
        ),
        ("users.lookupByEmail", json!({"user": "U0FIXTURE11"})),
        (
            "users.lookupByEmail",
            json!({"email": "fixture-user-11@example.test", "query:token": TOKEN}),
        ),
        (
            "users.lookupByEmail",
            json!({"email": "fixture-user-11@example.test", "TOKEN": TOKEN}),
        ),
    ];
    for (operation, input) in &cases {
        let outcome = attempt(&mut child, operation, input);
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "`{operation}` {input}: {outcome:?}"
        );
    }
    assert!(
        provider.exchanges().is_empty(),
        "{:?}",
        provider.exchanges()
    );
}

/// An empty `user` or `email` names nobody: the read is refused before any
/// request rather than asking Slack for no one.
#[test]
#[ignore = "story:required-parameter-refuses-empty: the engine sends a required query parameter given as an empty string (adapters/catalog/src/lib.rs:1259)"]
fn an_empty_user_or_email_sends_nothing() {
    let provider = Provider::reading(Arc::new(|_: &Exchange| {
        Reply::Json(200, json!({"ok": false, "error": "user_not_found"}))
    }));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input) in [
        ("users.info", json!({"user": ""})),
        ("users.lookupByEmail", json!({"email": ""})),
    ] {
        let outcome = attempt(&mut child, operation, &input);
        assert!(
            matches!(outcome, Err(Failure::InvalidInput)),
            "`{operation}` {input}: {outcome:?}"
        );
    }
    assert!(
        provider.exchanges().is_empty(),
        "sent: {:?}",
        provider
            .exchanges()
            .iter()
            .map(|exchange| exchange.path.clone())
            .collect::<Vec<_>>()
    );
}

// ── The guide against the shipped set ────────────────────────────────────────

/// The bot set ships `users.info` and `users.lookupByEmail`; the guide must
/// not tell a reader that neither connection selects them.
#[test]
fn the_guide_does_not_deny_selecting_the_one_user_reads() {
    let shipped = shipped();
    for id in ["users.info", "users.lookupByEmail"] {
        assert!(shipped.iter().any(|selection| selection.id == id), "{id}");
    }
    let guide = guide().split_whitespace().collect::<Vec<_>>().join(" ");
    let denial = "(`users.info`, `users.lookupByEmail`) is not selected on either connection";
    assert!(
        !guide.contains(denial),
        "docs/catalog-slack.md still says: {denial}"
    );
}

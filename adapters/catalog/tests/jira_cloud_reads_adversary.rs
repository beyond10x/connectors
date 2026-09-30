//! Adversarial cases for story:catalog-jira-cloud-reads.
//!
//! Drives the shipped Jira selection set and its guide against the pinned
//! platform REST v3 document: the guide's end condition for `issues.search`
//! against what the pinned document says the last page carries, the exact wire
//! encoding of reserved characters in JQL and in a continuation token, that an
//! undeclared parameter is refused before any request, and that the basic
//! credential appears in no argv, no environment and no failure of the child.
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child},
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

const ACCOUNT: &str = "adversary@example.test";
const TOKEN: &str = "adversary-jira-token-7Qx";
/// `printf '%s' 'adversary@example.test:adversary-jira-token-7Qx' | base64 -w0`
const HEADER: &str = "Basic YWR2ZXJzYXJ5QGV4YW1wbGUudGVzdDphZHZlcnNhcnktamlyYS10b2tlbi03UXg=";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

// ---------------------------------------------------------------------------
// The guide against the pinned document
// ---------------------------------------------------------------------------

/// The pinned document's own response schema says the last page's token "will
/// be null", while its parameter text says the field is "not included". The
/// guide's end condition names only absence, so a reader who tests for the
/// key's presence walks on after a last page that carries `"nextPageToken":
/// null`.
#[test]
fn guide_end_condition_for_search_covers_the_null_token_the_pinned_document_names() {
    let source: Value = serde_json::from_slice(
        &fs::read(root().join("../atlassian/upstream/jira-platform-v3.json")).unwrap(),
    )
    .unwrap();
    let described =
        source["components"]["schemas"]["SearchAndReconcileResults"]["properties"]["nextPageToken"]
            ["description"]
            .as_str()
            .unwrap();
    assert!(
        described.contains("this token will be null"),
        "anchor: the pinned document no longer says the last token is null"
    );
    let guide = fs::read_to_string(root().join("../../docs/catalog-jira.md")).unwrap();
    let row = guide
        .lines()
        .find(|l| l.starts_with('|') && l.contains("`issues.search`"))
        .unwrap();
    let paragraph = guide
        .lines()
        .find(|l| l.starts_with("- **`issues.search`**"))
        .map(|first| {
            guide[guide.find(first).unwrap()..]
                .split("\n- **")
                .next()
                .unwrap()
                .to_owned()
        })
        .unwrap();
    assert!(
        row.contains("null") || paragraph.contains("null"),
        "the guide's `issues.search` end condition is only `no nextPageToken`; the pinned \
         document says the last page's token is null. row: {row}"
    );
}

// ---------------------------------------------------------------------------
// A fixture that accepts only HEADER and records each request line
// ---------------------------------------------------------------------------

type Requests = Arc<Mutex<Vec<(String, Option<String>)>>>;

struct Fixture {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    requests: Requests,
}

fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

impl Fixture {
    fn new() -> Self {
        let root_dir = tempfile::tempdir().unwrap();
        let directory = root_dir.path().join("private");
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
        let requests: Requests = Arc::new(Mutex::new(Vec::new()));
        let observed = requests.clone();
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
                        assert!(header.len() < 8192);
                        match stream.read_u8().await {
                            Ok(byte) => header.push(byte),
                            Err(_) => break,
                        }
                    }
                    let request = String::from_utf8(header).unwrap();
                    let path = request
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or_default()
                        .to_owned();
                    let authorization = request.lines().skip(1).find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                            .map(|(_, value)| value.trim().to_owned())
                    });
                    let (status, body) = if authorization.as_deref() != Some(HEADER) {
                        (401, json!({"errorMessages": ["fixture refusal"]}))
                    } else if path.starts_with("/rest/api/3/myself") {
                        (200, json!({"accountId": "fixture-account-id", "active": true}))
                    } else {
                        (200, json!({"issues": [], "isLast": true}))
                    };
                    observed.lock().unwrap().push((path, authorization));
                    let body = serde_json::to_vec(&body).unwrap();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let config = directory.join("catalog.json");
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format": "connectors-catalog-local/2",
                "instance": "adversary-jira",
                "provider": "jira",
                "bundle_directory": root().join("generated/bundles").canonicalize().unwrap(),
                "api_base": format!("https://localhost:{}/rest/api/3", address.port()),
                "ca_file": ca,
                "auth": {
                    "profile": "atlassian.basic",
                    "scheme": "basic",
                    "header": "Authorization",
                    "bearer": false,
                    "account_label": "Account email",
                    "label": "API token",
                    "identity": {"path": "myself", "kind": "atlassian.account", "subject_pointer": "/accountId"}
                },
                "operations_file": root().join("providers/jira/operations.json").canonicalize().unwrap(),
            }))
            .unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root_dir,
            config,
            requests,
        }
    }
    fn adapter(&self) -> Adapter {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(&self.config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(output.status.success());
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "adversary-jira".into(),
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
    fn requests(&self) -> Vec<(String, Option<String>)> {
        self.requests.lock().unwrap().clone()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn secret(account: &str, token: &str) -> Secret {
    Secret(serde_json::to_vec(&json!({"account": account, "token": token})).unwrap())
}

fn call(
    child: &mut Child,
    operation: &str,
    credential: &Secret,
    input: Value,
) -> Result<Value, String> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child
        .invoke(
            operation,
            &revision,
            "one",
            credential,
            &serde_json::to_vec(&input).unwrap(),
            connectors_sdk::now_ms() + 30_000,
        )
        .map(|bytes| serde_json::from_slice(&bytes).unwrap())
        .map_err(|failure| format!("{failure:?}"))
}

// ---------------------------------------------------------------------------
// Wire encoding
// ---------------------------------------------------------------------------

#[test]
fn reserved_characters_in_jql_and_token_reach_the_wire_form_encoded() {
    let fixture = Fixture::new();
    let mut child = Child::spawn(&fixture.adapter()).unwrap();
    let credential = secret(ACCOUNT, TOKEN);
    call(
        &mut child,
        "issues.search",
        &credential,
        json!({"jql": "summary ~ \"C++ & 100%\" AND updated >= \"2026-09-01 00:00\"",
               "nextPageToken": "Ab+/c=="}),
    )
    .unwrap();
    let requests = fixture.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].0,
        "/rest/api/3/search/jql?jql=summary+%7E+%22C%2B%2B+%26+100%25%22+AND+updated+%3E%3D+\
         %222026-09-01+00%3A00%22&nextPageToken=Ab%2B%2Fc%3D%3D"
    );
    assert_eq!(requests[0].1.as_deref(), Some(HEADER));
}

#[test]
fn an_undeclared_parameter_is_refused_before_any_request() {
    let fixture = Fixture::new();
    let mut child = Child::spawn(&fixture.adapter()).unwrap();
    let credential = secret(ACCOUNT, TOKEN);
    // `startAt` belongs to the deprecated search, not to /search/jql.
    let refused = call(
        &mut child,
        "issues.search",
        &credential,
        json!({"jql": "project = FIX", "startAt": 50}),
    );
    assert!(refused.is_err(), "undeclared `startAt` was accepted");
    let refused = call(
        &mut child,
        "issue.changelog",
        &credential,
        json!({"issueIdOrKey": "FIX-1", "orderBy": "created"}),
    );
    assert!(refused.is_err(), "undeclared `orderBy` was accepted");
    assert!(fixture.requests().is_empty());
    // The same call with a declared parameter does reach the fixture, so the
    // refusals above are not a dead child.
    call(
        &mut child,
        "issue.changelog",
        &credential,
        json!({"issueIdOrKey": "FIX-1", "maxResults": 1}),
    )
    .unwrap();
    assert_eq!(
        fixture.requests()[0].0,
        "/rest/api/3/issue/FIX-1/changelog?maxResults=1"
    );
}

// ---------------------------------------------------------------------------
// The credential stays out of argv, environ and failures
// ---------------------------------------------------------------------------

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle.as_bytes())
}

#[test]
fn the_basic_credential_is_in_no_argv_environ_or_failure() {
    let fixture = Fixture::new();
    let mut child = Child::spawn(&fixture.adapter()).unwrap();
    let credential = secret(ACCOUNT, TOKEN);
    // A successful call first, so the child has held the header.
    call(
        &mut child,
        "issue.comments",
        &credential,
        json!({"issueIdOrKey": "FIX-1"}),
    )
    .unwrap();
    assert_eq!(fixture.requests()[0].1.as_deref(), Some(HEADER));

    let encoded = HEADER.trim_start_matches("Basic ");
    let config = fixture.config.to_str().unwrap().to_owned();
    let mut inspected = 0;
    for entry in fs::read_dir("/proc").unwrap().flatten() {
        let Ok(cmdline) = fs::read(entry.path().join("cmdline")) else {
            continue;
        };
        if !contains(&cmdline, &config) {
            continue;
        }
        let environ = fs::read(entry.path().join("environ")).unwrap_or_default();
        for needle in [ACCOUNT, TOKEN, encoded] {
            assert!(!contains(&cmdline, needle), "argv carries `{needle}`");
            assert!(!contains(&environ, needle), "environ carries `{needle}`");
        }
        inspected += 1;
    }
    assert!(inspected >= 1, "the provider child was not found in /proc");
    let config_bytes = fs::read(&fixture.config).unwrap();
    for needle in [ACCOUNT, TOKEN, encoded] {
        assert!(!contains(&config_bytes, needle));
    }

    // A refused credential and a malformed one fail without echoing either.
    let wrong = call(
        &mut child,
        "issue.comments",
        &secret(ACCOUNT, "wrong-token-9Zp"),
        json!({"issueIdOrKey": "FIX-1"}),
    );
    let colon = call(
        &mut child,
        "issue.comments",
        &secret("a:b@example.test", TOKEN),
        json!({"issueIdOrKey": "FIX-1"}),
    );
    for outcome in [&wrong, &colon] {
        let text = format!("{outcome:?}");
        for needle in [
            ACCOUNT,
            TOKEN,
            encoded,
            "wrong-token-9Zp",
            "a:b@example.test",
        ] {
            assert!(
                !text.contains(needle),
                "an outcome echoes `{needle}`: {text}"
            );
        }
    }
    assert!(colon.is_err(), "an account with a colon was accepted");
    // The success and the refused token reached the wire; the colon account did not.
    assert_eq!(
        fixture.requests().len(),
        2,
        "the colon account reached the wire"
    );
}

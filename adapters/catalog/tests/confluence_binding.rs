//! One Confluence instance whose operations bind, at validation, to the REST
//! API its credential can read (story:confluence-token-kind-binding, option a:
//! the host carries the binding).
//!
//! The instance's configuration declares REST v2 (`confluence`) first and REST
//! v1 (`confluence-v1`) as its alternative, each with its own bundle, selection
//! set and access probe, and names the answer that moves validation on to the
//! next binding: a `401` whose admitted reason holds `scope does not match`,
//! the answer Atlassian gives a scoped token on an API its scopes do not cover.
//! Nothing in the engine names Confluence; the pairing is configuration.
//!
//! Each case runs through the provider child against a disposable HTTPS
//! fixture whose access answers the test sets: the exact requests are
//! asserted, so a fallback that was not taken is a request that was not sent.
//! No live credential and no network.
#![cfg(unix)]
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Baseline, Bootstrap, Child, Failure, Refusal},
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

/// The fictional basic material the Jira and Confluence tests use:
/// `Basic base64("reader@example.test:fixture-jira-token")`.
const ACCOUNT: &str = "reader@example.test";
const TOKEN: &str = "fixture-jira-token";
const HEADER: &str = "Basic cmVhZGVyQGV4YW1wbGUudGVzdDpmaXh0dXJlLWppcmEtdG9rZW4=";
const PROFILE: &str = "atlassian.basic";
const INSTANCE: &str = "fixture-confluence";
/// The requests each probe sends.
const IDENTITY: &str = "/wiki/rest/api/user/current";
const V2_ACCESS: &str = "/wiki/api/v2/pages?limit=1";
const V1_ACCESS: &str = "/wiki/rest/api/space?limit=1";
/// The reads one page under each API.
const V2_PAGE: &str = "/wiki/api/v2/pages/1001?body-format=storage";
const V1_PAGE: &str = "/wiki/rest/api/content/1001?expand=body.storage";
/// The v2 and v1 ids the shipped selection sets serve.
const V2_IDS: [&str; 4] = ["page.comments", "page.get", "pages.changed", "space.pages"];
const V1_IDS: [&str; 7] = [
    "content.search",
    "page.comments",
    "page.get",
    "pages.changed",
    "space.get",
    "space.pages",
    "spaces.list",
];

/// How an access probe is answered.
#[derive(Clone, Copy, Debug)]
enum Access {
    /// `200`, the read the binding needs.
    Answered,
    /// Atlassian's answer to a scoped token outside its scopes.
    ScopeRefused,
    /// A `401` that is not about scope.
    Refused,
    /// A `403` naming why.
    Forbidden,
}
impl Access {
    fn answer(self, body: Value) -> (u16, Value) {
        match self {
            Access::Answered => (200, body),
            Access::ScopeRefused => (
                401,
                json!({"code": 401, "message": "Unauthorized; scope does not match"}),
            ),
            Access::Refused => (401, json!({"code": 401, "message": "Unauthorized"})),
            Access::Forbidden => (403, json!({"message": "fixture forbidden"})),
        }
    }
}
#[derive(Clone, Copy, Debug)]
struct Mode {
    v2: Access,
    v1: Access,
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn root_path(relative: &str) -> PathBuf {
    root().join(relative).canonicalize().unwrap()
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
/// The shipped `auth` profile of `provider`.
fn shipped_auth(provider: &str) -> Value {
    let declaration: Value = serde_json::from_slice(
        &fs::read(
            root()
                .join("providers")
                .join(provider)
                .join("connector.json"),
        )
        .unwrap(),
    )
    .unwrap();
    declaration["profiles"][0].clone()
}
fn v1_page() -> Value {
    json!({"id": "1001", "type": "page", "status": "current", "title": "fixture page 1001",
           "space": {"key": "DOCS", "name": "fixture space DOCS"},
           "version": {"number": 4, "when": "2026-09-20T10:00:00.000Z"},
           "body": {"storage": {"value": "<p>v1 page 1001</p>", "representation": "storage"}},
           "_links": {"webui": "/spaces/DOCS/pages/1001"}})
}
fn v2_page() -> Value {
    json!({"id": "1001", "status": "current", "title": "fixture page 1001",
           "spaceId": "65538", "parentId": "1000", "parentType": "page",
           "authorId": "fixture-author", "createdAt": "2026-08-01T08:00:00.000Z",
           "version": {"number": 3, "message": "", "minorEdit": false,
                       "authorId": "fixture-author", "createdAt": "2026-09-20T10:00:00.000Z"},
           "body": {"storage": {"representation": "storage", "value": "<p>v2 page 1001</p>"}},
           "_links": {"webui": "/spaces/FIX/pages/1001"}})
}
fn served(path: &str, mode: Mode) -> (u16, Value) {
    let (route, _) = path.split_once('?').unwrap_or((path, ""));
    match path {
        V2_ACCESS => mode
            .v2
            .answer(json!({"results": [], "_links": {"base": "https://site.example.test/wiki"}})),
        V1_ACCESS => mode
            .v1
            .answer(json!({"results": [], "start": 0, "limit": 1, "size": 0,
                   "_links": {"base": "https://site.example.test/wiki", "context": "/wiki"}})),
        V2_PAGE => mode.v2.answer(v2_page()),
        V1_PAGE => mode.v1.answer(v1_page()),
        _ if route == IDENTITY => (
            200,
            json!({"type": "known", "accountId": "fixture-account-id"}),
        ),
        _ => (404, json!({"message": "no fixture"})),
    }
}

/// The instance's native configuration: v2 first, v1 its alternative.
fn configuration(port: u16, ca: &Path) -> Value {
    json!({
        "format": "connectors-catalog-local/2",
        "instance": INSTANCE,
        "provider": "confluence",
        "bundle_directory": root_path("generated/bundles"),
        "api_base": format!("https://localhost:{port}/wiki"),
        "ca_file": ca,
        "auth": shipped_auth("confluence"),
        "operations_file": root_path("providers/confluence/operations.json"),
        "bindings": {
            "id": "v2",
            "next": {"status": 401, "reason": "scope does not match"},
            "alternatives": [{
                "id": "v1",
                "provider": "confluence-v1",
                "operations_file": root_path("providers/confluence-v1/operations.json"),
                "access": {"path": "rest/api/space", "query": {"limit": "1"}}
            }]
        }
    })
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    config: PathBuf,
    ca: PathBuf,
    port: u16,
    requests: Arc<Mutex<Vec<String>>>,
    mode: Arc<Mutex<Mode>>,
}
impl Provider {
    fn new(mode: Mode) -> Self {
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
        let requests = Arc::new(Mutex::new(Vec::new()));
        let mode = Arc::new(Mutex::new(mode));
        let (observed, current) = (requests.clone(), mode.clone());
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
                    let authorization = request.lines().find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                            .map(|(_, value)| value.trim().to_owned())
                    });
                    // Only the fictional header is accepted: anything else is
                    // the bad password, refused on every route.
                    let (status, body) = if authorization.as_deref() != Some(HEADER) {
                        (401, json!({"message": "fixture refusal"}))
                    } else {
                        served(&path, *current.lock().unwrap())
                    };
                    observed.lock().unwrap().push(path);
                    let body = serde_json::to_vec(&body).unwrap();
                    let header = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(header.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let config = directory.join("catalog.json");
        private(
            &config,
            &serde_json::to_vec(&configuration(address.port(), &ca)).unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            config,
            ca,
            port: address.port(),
            requests,
            mode,
        }
    }
    fn set(&self, mode: Mode) {
        *self.mode.lock().unwrap() = mode;
    }
    /// The requests sent since the last call.
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.requests.lock().unwrap())
    }
    fn bootstrap_of(config: &Path) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap()
    }
    fn bootstrap(&self) -> Bootstrap {
        let output = Self::bootstrap_of(&self.config);
        assert!(
            output.status.success(),
            "bootstrap inspection failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        bootstrap.validate().unwrap();
        bootstrap
    }
    fn selection(&self) -> Adapter {
        let bootstrap = self.bootstrap();
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            credential_revision: None,
            private_protocol: None,
            permissions: Default::default(),
            instance_id: INSTANCE.into(),
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
    /// `edit` applied to this instance's configuration, written beside it.
    fn variant(&self, edit: impl FnOnce(&mut Value)) -> PathBuf {
        let mut config = configuration(self.port, &self.ca);
        edit(&mut config);
        let path = self.root.path().join("private/variant.json");
        private(&path, &serde_json::to_vec(&config).unwrap());
        path
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn secret() -> Secret {
    Secret(serde_json::to_vec(&json!({"account": ACCOUNT, "token": TOKEN})).unwrap())
}
fn wrong_secret() -> Secret {
    Secret(
        serde_json::to_vec(&json!({"account": ACCOUNT, "token": "fixture-bad-password"})).unwrap(),
    )
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
fn validate(child: &mut Child, secret: &Secret) -> Result<Baseline, Refusal> {
    child.validate_explained(PROFILE, secret, deadline())
}
/// The revision of `binding`'s description of the child's instance.
fn revision(child: &Child, binding: &str) -> String {
    child
        .bootstrap()
        .bound(Some(binding))
        .unwrap()
        .descriptor()
        .unwrap()
        .revision
}
fn invoke(
    child: &mut Child,
    binding: &str,
    operation: &str,
    input: &Value,
) -> Result<Value, Refusal> {
    let revision = revision(child, binding);
    child
        .invoke_bound(
            Some(binding),
            operation,
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(input).unwrap(),
            deadline(),
        )
        .map(|output: Vec<u8>| serde_json::from_slice(&output).unwrap())
}
fn ids(bootstrap: &Bootstrap) -> Vec<String> {
    let mut ids: Vec<String> = bootstrap
        .requirements
        .iter()
        .map(|r| r.operation.clone())
        .collect();
    ids.sort();
    ids
}

const V2_SCOPE_REFUSED: Mode = Mode {
    v2: Access::ScopeRefused,
    v1: Access::Answered,
};
const V2_ANSWERED: Mode = Mode {
    v2: Access::Answered,
    v1: Access::Answered,
};

/// The bootstrap describes the first binding as before and carries each
/// alternative's own description and requirements beside it.
#[test]
fn the_bootstrap_carries_each_binding_with_its_own_operations() {
    let provider = Provider::new(V2_ANSWERED);
    let bootstrap = provider.bootstrap();
    let bindings = bootstrap.bindings.as_ref().expect("bindings declared");
    assert_eq!(bindings.id, "v2");
    assert_eq!(
        bindings
            .alternatives
            .iter()
            .map(|b| b.id.as_str())
            .collect::<Vec<_>>(),
        ["v1"]
    );
    assert_eq!(ids(&bootstrap), V2_IDS);
    assert_eq!(ids(&bootstrap.bound(None).unwrap()), V2_IDS);
    assert_eq!(ids(&bootstrap.bound(Some("v2")).unwrap()), V2_IDS);
    let v1 = bootstrap.bound(Some("v1")).unwrap();
    assert_eq!(ids(&v1), V1_IDS);
    assert!(v1.bindings.is_none(), "a bound view names no alternatives");
    v1.validate().unwrap();
    // Each binding is its own description: a revision of one is stale for the other.
    assert_ne!(
        v1.descriptor().unwrap().revision,
        bootstrap.descriptor().unwrap().revision
    );
    assert_eq!(v1.configuration_revision, bootstrap.configuration_revision);
    assert!(matches!(
        bootstrap.bound(Some("v3")),
        Err(Failure::StaleDescription)
    ));
}

/// The `bindings` member both guides show is the one these cases run, its
/// placeholder path standing for this checkout.
#[test]
fn the_guides_show_the_bindings_these_cases_run() {
    let tested = configuration(1, Path::new("ca.pem"))["bindings"].clone();
    let checkout = root().join("../..").canonicalize().unwrap();
    for page in ["catalog-confluence.md", "local-catalog-provider.md"] {
        let text = fs::read_to_string(root().join("../../docs").join(page)).unwrap();
        let shown: Vec<Value> = text
            .split("```json\n")
            .skip(1)
            .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
            .filter(|body| body.contains("\"bindings\""))
            .map(|body| {
                let body = body.replace("/absolute/path", checkout.to_str().unwrap());
                let parsed: Value = serde_json::from_str(&body)
                    .or_else(|_| serde_json::from_str(&format!("{{{body}}}")))
                    .unwrap();
                parsed["bindings"].clone()
            })
            .collect();
        assert_eq!(shown, std::slice::from_ref(&tested), "{page}");
    }
}

/// Without `bindings` the bootstrap has no such member, so existing
/// configurations print the bytes they printed before.
#[test]
fn a_configuration_without_bindings_prints_no_bindings() {
    let provider = Provider::new(V2_ANSWERED);
    let path = provider.variant(|config| {
        config.as_object_mut().unwrap().remove("bindings");
    });
    let output = Provider::bootstrap_of(&path);
    assert!(output.status.success());
    let printed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(printed.get("bindings").is_none(), "{printed}");
    let with = provider.bootstrap();
    assert_ne!(
        printed["configuration_revision"], with.configuration_revision,
        "the bindings enter the configuration revision"
    );
}

/// A configuration whose bindings could not be followed is refused at load.
#[test]
fn unusable_bindings_are_refused_at_load() {
    let provider = Provider::new(V2_ANSWERED);
    type Edit = Box<dyn Fn(&mut Value)>;
    let edits: [(&str, Edit); 6] = [
        (
            "no first access probe",
            Box::new(|c| {
                c["auth"].as_object_mut().unwrap().remove("access");
            }),
        ),
        (
            "an alternative named as the first",
            Box::new(|c| c["bindings"]["alternatives"][0]["id"] = json!("v2")),
        ),
        (
            "a next answer after the last binding",
            Box::new(|c| {
                c["bindings"]["alternatives"][0]["next"] = json!({"status": 401, "reason": "x"})
            }),
        ),
        (
            "no next answer before an alternative",
            Box::new(|c| {
                c["bindings"].as_object_mut().unwrap().remove("next");
            }),
        ),
        (
            "an alternative with no access probe",
            Box::new(|c| {
                c["bindings"]["alternatives"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("access");
            }),
        ),
        (
            "a next answer that is not a refusal",
            Box::new(|c| c["bindings"]["next"]["status"] = json!(200)),
        ),
    ];
    for (what, edit) in edits {
        let path = provider.variant(edit);
        let output = Provider::bootstrap_of(&path);
        assert!(!output.status.success(), "{what} was admitted");
    }
}

/// Acceptance 1: refused for scope on v2, answered on v1, the connection
/// binds to v1 and reads the v1 page.
#[test]
fn scope_refused_on_v2_and_answered_on_v1_binds_to_v1() {
    let provider = Provider::new(V2_SCOPE_REFUSED);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    provider.take();
    let baseline = validate(&mut child, &secret()).unwrap();
    assert_eq!(baseline.binding.as_deref(), Some("v1"));
    assert_eq!(baseline.identity.subject, "fixture-account-id");
    assert_eq!(
        provider.take(),
        [format!("{IDENTITY}?"), V2_ACCESS.into(), V1_ACCESS.into()]
    );
    let page = invoke(
        &mut child,
        "v1",
        "page.get",
        &json!({"id": "1001", "expand": "body.storage"}),
    )
    .unwrap();
    assert_eq!(provider.take(), [V1_PAGE]);
    assert_eq!(page["status"], 200);
    assert_eq!(page["body"], v1_page());
    assert_eq!(page["provenance"]["instance"], INSTANCE);
}

/// Acceptance 2: answered on v2, the connection binds to v2 and nothing is
/// sent to v1.
#[test]
fn answered_on_v2_binds_to_v2_without_a_v1_request() {
    let provider = Provider::new(V2_ANSWERED);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    provider.take();
    let baseline = validate(&mut child, &secret()).unwrap();
    assert_eq!(baseline.binding.as_deref(), Some("v2"));
    assert_eq!(provider.take(), [format!("{IDENTITY}?"), V2_ACCESS.into()]);
    let page = invoke(
        &mut child,
        "v2",
        "page.get",
        &json!({"id": "1001", "body-format": "storage"}),
    )
    .unwrap();
    assert_eq!(page["body"], v2_page());
    let sent = provider.take();
    assert_eq!(sent, [V2_PAGE]);
    assert!(!sent.iter().any(|p| p.starts_with("/wiki/rest/api/space")));
}

/// Acceptance 3: refused by both, nothing binds, and the refusal names both
/// answers.
#[test]
fn refused_by_both_does_not_connect_and_names_both_answers() {
    let provider = Provider::new(Mode {
        v2: Access::ScopeRefused,
        v1: Access::Forbidden,
    });
    let mut child = Child::spawn(&provider.selection()).unwrap();
    provider.take();
    let refusal = validate(&mut child, &secret()).err().expect("refused");
    assert_eq!(refusal.failure, Failure::InsufficientScope);
    let reason = refusal.reason.expect("a reason naming both answers");
    assert!(
        reason.contains("binding v2 answered 401 (Unauthorized; scope does not match)"),
        "{reason}"
    );
    assert!(
        reason.contains("binding v1 answered 403 (fixture forbidden)"),
        "{reason}"
    );
    assert_eq!(
        provider.take(),
        [format!("{IDENTITY}?"), V2_ACCESS.into(), V1_ACCESS.into()]
    );
}

/// Acceptance 4: a refusal that is not about scope moves to no other binding,
/// whether the password is wrong or v2 refuses the read for another reason.
#[test]
fn a_refusal_that_is_not_about_scope_does_not_fall_back() {
    let provider = Provider::new(V2_SCOPE_REFUSED);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    provider.take();
    let refusal = validate(&mut child, &wrong_secret())
        .err()
        .expect("refused");
    assert_eq!(refusal.failure, Failure::InvalidCredential);
    assert_eq!(refusal.reason, None);
    assert_eq!(provider.take(), [format!("{IDENTITY}?")]);

    provider.set(Mode {
        v2: Access::Refused,
        v1: Access::Answered,
    });
    let refusal = validate(&mut child, &secret()).err().expect("refused");
    assert_eq!(refusal.failure, Failure::InsufficientScope);
    assert_eq!(
        refusal.reason.as_deref(),
        Some("binding v2 answered 401 (Unauthorized)")
    );
    assert_eq!(provider.take(), [format!("{IDENTITY}?"), V2_ACCESS.into()]);
}

/// Acceptance 5: each validation probes again from the first binding, so one
/// that sees the other API answering binds to it.
#[test]
fn a_revalidation_that_sees_the_other_api_answering_rebinds() {
    let provider = Provider::new(V2_SCOPE_REFUSED);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let first = validate(&mut child, &secret()).unwrap();
    assert_eq!(first.binding.as_deref(), Some("v1"));
    provider.set(V2_ANSWERED);
    provider.take();
    let second = validate(&mut child, &secret()).unwrap();
    assert_eq!(second.binding.as_deref(), Some("v2"));
    assert_eq!(provider.take(), [format!("{IDENTITY}?"), V2_ACCESS.into()]);
    provider.set(V2_SCOPE_REFUSED);
    let third = validate(&mut child, &secret()).unwrap();
    assert_eq!(third.binding.as_deref(), Some("v1"));
}

/// Acceptance 6: an id the bound binding does not serve is refused by name
/// before anything is sent, and a revision of another binding is stale.
#[test]
fn an_id_the_bound_binding_lacks_is_a_named_refusal() {
    let provider = Provider::new(V2_ANSWERED);
    let mut child = Child::spawn(&provider.selection()).unwrap();
    validate(&mut child, &secret()).unwrap();
    provider.take();
    let refusal = invoke(
        &mut child,
        "v2",
        "content.search",
        &json!({"cql": "type = page"}),
    )
    .expect_err("refused");
    assert_eq!(refusal.failure, Failure::NotFound);
    // The same id under the binding that serves it is admitted.
    assert!(
        child
            .bootstrap()
            .bound(Some("v1"))
            .unwrap()
            .requirements
            .iter()
            .any(|r| r.operation == "content.search")
    );
    // A revision of one binding does not admit a read under another.
    let v1 = revision(&child, "v1");
    let refusal = child
        .invoke_bound(
            Some("v2"),
            "page.get",
            &v1,
            "one",
            &secret(),
            &serde_json::to_vec(&json!({"id": "1001"})).unwrap(),
            deadline(),
        )
        .expect_err("refused");
    assert_eq!(refusal.failure, Failure::StaleDescription);
    // A binding the instance does not declare is not a description at all.
    let refusal = child
        .invoke_bound(
            Some("v3"),
            "page.get",
            &v1,
            "one",
            &secret(),
            &serde_json::to_vec(&json!({"id": "1001"})).unwrap(),
            deadline(),
        )
        .expect_err("refused");
    assert_eq!(refusal.failure, Failure::StaleDescription);
    assert!(provider.take().is_empty(), "a refused read sent a request");
    assert!(child.running().unwrap());
}

/// The production CLI, its owner and a disposable Secret Service, for the
/// journey below; the same fixtures `local_runtime`'s CLI journeys use.
mod journey {
    use super::*;
    use connectors_host::local::{config::Paths, keyring, owner};
    use std::{
        io::Write,
        process::{Child as Process, Output, Stdio},
        time::Instant,
    };

    struct Owned(Process);
    impl Drop for Owned {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn quiet(command: &mut Command) -> &mut Command {
        command
            .env_clear()
            .env("PATH", "/usr/bin")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
    }

    pub(super) struct Custody {
        _daemon: Owned,
        _bus: Owned,
        pub(super) socket: PathBuf,
    }
    impl Custody {
        pub(super) fn new(parent: &Path) -> Self {
            let directory = parent.join("custody");
            for path in ["", "home", "data/keyrings", "runtime"] {
                filesystem::directory(&directory.join(path), true, true).unwrap();
            }
            let socket = directory.join("bus");
            let bus = Owned(
                quiet(Command::new("/usr/bin/dbus-daemon").args([
                    "--session",
                    "--nofork",
                    "--nopidfile",
                ]))
                .arg(format!("--address=unix:path={}", socket.display()))
                .stdin(Stdio::null())
                .spawn()
                .unwrap(),
            );
            let until = Instant::now() + Duration::from_secs(5);
            while std::os::unix::net::UnixStream::connect(&socket).is_err() {
                assert!(Instant::now() < until, "fixture bus did not start");
                std::thread::sleep(Duration::from_millis(20));
            }
            let runtime = directory.join("runtime");
            let mut daemon = Owned(
                quiet(Command::new("/usr/bin/gnome-keyring-daemon").args([
                    "--foreground",
                    "--components=secrets",
                    "--unlock",
                    "--control-directory",
                ]))
                .arg(&runtime)
                .env("HOME", directory.join("home"))
                .env("XDG_DATA_HOME", directory.join("data"))
                .env("XDG_RUNTIME_DIR", &runtime)
                .env(
                    "DBUS_SESSION_BUS_ADDRESS",
                    format!("unix:path={}", socket.display()),
                )
                .stdin(Stdio::piped())
                .spawn()
                .unwrap(),
            );
            daemon
                .0
                .stdin
                .take()
                .unwrap()
                .write_all(b"fictional-catalog-fixture-keyring-password")
                .unwrap();
            let until = Instant::now() + Duration::from_secs(10);
            while !keyring::custody::available_at(Some(&socket)) {
                assert!(Instant::now() < until, "fixture custody unavailable");
                assert!(daemon.0.try_wait().unwrap().is_none());
                std::thread::sleep(Duration::from_millis(30));
            }
            Self {
                _daemon: daemon,
                _bus: bus,
                socket,
            }
        }
    }

    pub(super) struct Cli {
        binary: PathBuf,
        pub(super) paths: Paths,
    }
    impl Cli {
        pub(super) fn new(root: &Path) -> Self {
            Self {
                binary: std::env::var_os("CONNECTORS_TEST_CLI")
                    .expect("built production CLI required")
                    .into(),
                paths: Paths {
                    config: root.join("cli/config.toml"),
                    state: root.join("cli/state"),
                },
            }
        }
        pub(super) fn run(&self, args: &[&str]) -> Output {
            Command::new(&self.binary)
                .env_clear()
                // A test build's owner exits once this process has ended
                // (contracts/cli/v1alpha1/owner.md).
                .env(
                    "CONNECTORS_TEST_OWNER_PARENT",
                    std::process::id().to_string(),
                )
                .args(["--output", "json", "--config"])
                .arg(&self.paths.config)
                .arg("--state-dir")
                .arg(&self.paths.state)
                .args(args)
                .stdin(Stdio::null())
                .output()
                .unwrap()
        }
    }
    impl Drop for Cli {
        fn drop(&mut self) {
            if let Ok(client) = owner::Client::connect(&self.paths, false) {
                let host = client.host_incarnation.clone();
                let _ = client.shutdown(&host);
                let until = Instant::now() + Duration::from_secs(5);
                while self.paths.state.join("owner.sock").exists() && Instant::now() < until {
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }

    #[track_caller]
    pub(super) fn success(output: Output) -> Value {
        assert!(
            output.status.success(),
            "CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        for stream in [&output.stdout, &output.stderr] {
            let text = String::from_utf8_lossy(stream);
            assert!(!text.contains(TOKEN) && !text.contains(HEADER));
        }
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["ok"], true);
        value["result"].clone()
    }

    #[track_caller]
    pub(super) fn refusal(output: Output, code: &str) -> Value {
        assert!(!output.status.success());
        let text = String::from_utf8_lossy(&output.stderr);
        assert!(!text.contains(TOKEN) && !text.contains(HEADER));
        let value: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(value["error"]["data"]["code"], code, "{value}");
        value["error"]["data"].clone()
    }
}

/// The pinned instance table the guide writes for this instance
/// (docs/catalog-confluence.md, "One instance for either API"): every
/// operation either binding serves is permitted.
fn instance_table(provider: &Provider, custody: &journey::Custody) -> String {
    let adapter = provider.selection();
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let mut operations: Vec<&str> = V2_IDS.iter().chain(&V1_IDS).copied().collect();
    operations.sort();
    operations.dedup();
    format!(
        "format = \"connectors-local/5\"\nowner_uid = {}\nsecret_service_socket = {}\n\n[connectors.confluence]\nenabled = true\n\n[connectors.confluence.instances.either]\nalias = \"confluence-either\"\ninstance_id = {}\nadapter_id = \"catalog\"\nconfiguration_revision = {}\nprotocol = \"v1alpha1\"\nprivate_protocol = \"connectors-private/2\"\nstartup = \"on-demand\"\nrestart = \"never\"\n\n[connectors.confluence.instances.either.permissions]\nprofiles = [{}]\noperations = {}\n\n[connectors.confluence.instances.either.executable]\npath = {}\nsha256 = {}\nargs = {}\n",
        filesystem::uid(),
        q(custody.socket.to_str().unwrap()),
        q(&adapter.instance_id),
        q(&adapter.configuration_revision),
        q(PROFILE),
        serde_json::to_string(&operations).unwrap(),
        q(adapter.executable.path.to_str().unwrap()),
        q(&adapter.executable.sha256),
        serde_json::to_string(&adapter.executable.args).unwrap()
    )
}

/// Acceptance 1 and 5 through the production CLI: a token v2 refuses for
/// scope connects bound to v1, `operations list` and `describe` with
/// `--connection` name v1's operations, `page.get` returns the v1 page through
/// `connectors operations invoke`, and a revalidation that sees v2 answering
/// rebinds the connection and says so.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_v1_bound_connection_reads_the_v1_page_through_the_cli_and_rebinds_on_revalidation() {
    use journey::{refusal, success};
    let provider = Provider::new(V2_SCOPE_REFUSED);
    let custody = journey::Custody::new(provider.root.path());
    let cli = journey::Cli::new(provider.root.path());
    success(cli.run(&["setup", "init"]));
    success(cli.run(&["setup", "check"]));
    private(
        &cli.paths.config,
        instance_table(&provider, &custody).as_bytes(),
    );
    connectors_host::local::config::Config::read(&cli.paths.config).unwrap();
    let selector = ["--connector", "confluence", "--instance", "either"];
    let with = |args: &[&str], tail: &[&str]| -> Vec<String> {
        args.iter()
            .chain(&selector)
            .chain(tail)
            .map(|s| (*s).to_owned())
            .collect()
    };
    let run = |args: Vec<String>| cli.run(&args.iter().map(String::as_str).collect::<Vec<_>>());
    let listed = |output: Value| -> Vec<String> {
        let mut ids: Vec<String> = output["operations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["id"].as_str().unwrap().to_owned())
            .collect();
        ids.sort();
        ids
    };

    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &secret().0);
    provider.take();
    let connected = success(run(with(
        &["connections", "connect"],
        &[
            "--profile",
            PROFILE,
            "--credential-file",
            credential.to_str().unwrap(),
        ],
    )))["connection"]
        .clone();
    assert_eq!(connected["summary"]["state"], "ready");
    assert_eq!(connected["binding"], "v1", "{connected}");
    assert!(
        provider
            .take()
            .ends_with(&[V2_ACCESS.into(), V1_ACCESS.into()])
    );
    let connection = connected["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let revision = connected["summary"]["revision"]
        .as_str()
        .unwrap()
        .to_owned();

    // Without a connection the instance is described by its first binding;
    // with one, by the binding that connection reads through.
    assert_eq!(
        listed(success(run(with(&["operations", "list"], &[])))),
        V2_IDS
    );
    let bound = with(&["operations", "list"], &["--connection", &connection]);
    assert_eq!(listed(success(run(bound))), V1_IDS);
    let describe = |operation: &str, connection: Option<&str>| {
        let mut tail = vec!["--operation", operation];
        if let Some(connection) = connection {
            tail.extend(["--connection", connection]);
        }
        let description = success(run(with(&["operations", "describe"], &tail)));
        (
            description["schema"].as_str().unwrap().to_owned(),
            description["revision"].as_str().unwrap().to_owned(),
        )
    };
    let invoke = |operation: &str, (schema, revision): &(String, String), input: Value| {
        run(with(
            &["operations", "invoke"],
            &[
                "--connection",
                &connection,
                "--operation",
                operation,
                "--schema",
                schema,
                "--revision",
                revision,
                "--input-json",
                &input.to_string(),
            ],
        ))
    };
    let first = describe("page.get", None);
    let v1 = describe("page.get", Some(&connection));
    assert_ne!(first.1, v1.1, "each binding is its own description");

    let page = success(invoke(
        "page.get",
        &v1,
        json!({"id": "1001", "expand": "body.storage"}),
    ));
    assert_eq!(page["result"]["status"], 200);
    assert_eq!(page["result"]["body"], v1_page());
    assert_eq!(provider.take(), [V1_PAGE]);
    // An id only v1 serves is described under v1, and the first binding's
    // description does not admit a read through the v1-bound connection.
    describe("spaces.list", Some(&connection));
    refusal(
        invoke("page.get", &first, json!({"id": "1001"})),
        "stale_description",
    );
    assert!(provider.take().is_empty());

    // v2 now answers: a revalidation probes from the first binding again.
    provider.set(V2_ANSWERED);
    let revalidated = success(run(with(
        &["connections", "revalidate"],
        &[
            "--connection",
            &connection,
            "--expected-revision",
            &revision,
        ],
    )));
    assert_eq!(revalidated["rebound"], json!({"from": "v1", "to": "v2"}));
    assert_eq!(revalidated["connection"]["binding"], "v2");
    assert_eq!(revalidated["connection"]["summary"]["state"], "ready");
    let bound = with(&["operations", "list"], &["--connection", &connection]);
    assert_eq!(listed(success(run(bound))), V2_IDS);
    provider.take();
    refusal(
        invoke("page.get", &v1, json!({"id": "1001"})),
        "stale_description",
    );
    let v2 = describe("page.get", Some(&connection));
    assert_eq!(v2.1, first.1);
    let page = success(invoke(
        "page.get",
        &v2,
        json!({"id": "1001", "body-format": "storage"}),
    ));
    assert_eq!(page["result"]["body"], v2_page());
    assert_eq!(provider.take(), [V2_PAGE]);
}

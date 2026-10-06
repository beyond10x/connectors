//! story:forge-issue-create: `issue.create` in the shipped GitLab selection
//! set, `postApiV4ProjectsIdIssues` (`POST /api/v4/projects/{id}/issues`), an
//! unguarded write admitted only as a required-approval mutation.
//!
//! The write runs through a private-protocol-two provider child against a
//! disposable HTTPS GitLab: prepare and commit, the transport the host's
//! approval coordinator drives once it holds a verified, spent approval. The
//! approval binds the operation and the whole input by digest, so a proof for
//! one issue verifies for exactly that input. Without an approval there is no
//! route: the read transport refuses the id on protocol two, and protocol one
//! declares no write at all. The fixture records each request's method, route
//! and body, so "no request was sent" is a count of what it saw. The fixture
//! token is fictional and only ever compared, never printed.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::local::{
    approvals,
    config::{Adapter, Executable, Restart, Startup},
    filesystem, mutations,
    runtime::{self, Bootstrap, Child, Failure, PrivateProtocol, WriteEffect, WriteResult},
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

const INSTANCE: &str = "fixture-gitlab";
const PROFILE: &str = "gitlab.pat";
/// The fictional token the fixture accepts in `PRIVATE-TOKEN`.
const TOKEN: &str = "fixture-pat-one";
/// The issues route of the fixture project `org/project`, its path id
/// encoded as one segment.
const ISSUES: &str = "/api/v4/projects/org%2Fproject/issues";

/// Method, request target and body of each fixture request.
type Requests = Arc<Mutex<Vec<(String, String, Vec<u8>)>>>;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value =
        serde_json::from_slice(&fs::read(root().join("providers/gitlab/operations.json")).unwrap())
            .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], "gitlab");
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn engine() -> Engine {
    let bundle = bundle::load(&root().join("generated/bundles"), "gitlab").unwrap();
    Engine::new(&bundle, "/api/v4", &shipped()).unwrap()
}

/// The issue GitLab answers for a created `body`, as the pinned source's
/// `APIEntitiesIssue` shapes it.
fn created(body: &Value) -> Value {
    json!({"id": 501, "iid": 7, "project_id": 7, "title": body["title"],
           "description": body["description"], "labels": body["labels"],
           "state": "opened", "web_url": "https://gitlab.example.test/org/project/-/issues/7"})
}
fn issue_body() -> Value {
    json!({"title": "Fixture issue", "description": "Filed through the forge selection",
           "labels": "fixture,triage"})
}
fn issue_input() -> Value {
    json!({"id": "org/project", "body": issue_body()})
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    requests: Requests,
    protocol: Option<PrivateProtocol>,
}
impl Provider {
    fn new(protocol: Option<PrivateProtocol>) -> Self {
        let root = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
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
                    let mut head = Vec::new();
                    while !head.ends_with(b"\r\n\r\n") {
                        assert!(head.len() < 8192);
                        match stream.read_u8().await {
                            Ok(byte) => head.push(byte),
                            Err(_) => break,
                        }
                    }
                    let head = String::from_utf8(head).unwrap();
                    let header = |wanted: &str| {
                        head.lines().find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case(wanted))
                                .map(|(_, value)| value.trim().to_owned())
                        })
                    };
                    let mut words = head.split_whitespace();
                    let method = words.next().unwrap_or_default().to_owned();
                    let target = words.next().unwrap_or_default().to_owned();
                    let length: usize = header("content-length")
                        .map(|value| value.parse().unwrap())
                        .unwrap_or(0);
                    assert!(length <= 65_536);
                    let mut body = vec![0; length];
                    if stream.read_exact(&mut body).await.is_err() {
                        continue;
                    }
                    let route = target.split('?').next().unwrap_or_default().to_owned();
                    let (status, answer) = if header("private-token").as_deref() != Some(TOKEN) {
                        (401, json!({"message": "401 Unauthorized"}))
                    } else if method == "POST" && route == ISSUES {
                        match serde_json::from_slice::<Value>(&body) {
                            Ok(input) if input["title"].is_string() => (201, created(&input)),
                            _ => (400, json!({"error": "title is missing"})),
                        }
                    } else {
                        (404, json!({"message": "404 Not Found"}))
                    };
                    observed.lock().unwrap().push((method, target, body));
                    let answer = serde_json::to_vec(&answer).unwrap();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answer.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&answer).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let config = directory.join("catalog.json");
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format": "connectors-catalog-local/2",
                "instance": INSTANCE,
                "provider": "gitlab",
                "bundle_directory": root_path("generated/bundles"),
                "api_base": format!("https://localhost:{}/api/v4", address.port()),
                "ca_file": ca,
                "auth": {
                    "profile": PROFILE,
                    "header": "PRIVATE-TOKEN",
                    "bearer": false,
                    "label": "GitLab personal access token",
                    "identity": {"path": "user", "kind": "gitlab.user", "subject_pointer": "/id"},
                    "scopes": {"path": "personal_access_tokens/self", "pointer": "/scopes"},
                    "minimum_scopes": ["api"]
                },
                "operations_file": root_path("providers/gitlab/operations.json"),
            }))
            .unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config,
            requests,
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
            private_protocol: self.protocol,
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
    /// Every request the fixture saw, as method, target and body parsed as
    /// JSON (`null` when it carried none).
    fn requests(&self) -> Vec<(String, String, Value)> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|(method, target, body)| {
                let body = if body.is_empty() {
                    Value::Null
                } else {
                    serde_json::from_slice(body).unwrap()
                };
                (method.clone(), target.clone(), body)
            })
            .collect()
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}
fn root_path(relative: &str) -> PathBuf {
    root().join(relative).canonicalize().unwrap()
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn secret() -> Secret {
    Secret(format!(r#"{{"token":"{TOKEN}"}}"#).into_bytes())
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
/// One call on the read transport, which carries no approval.
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
/// Prepare and commit one write on this child: the transport the host's
/// approval coordinator drives once it holds a verified, spent approval. A
/// refusal in prepare is the `Err`.
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

/// `issue.create` is a shipped write of `postApiV4ProjectsIdIssues` with no
/// guard: its inputs are the path `id` and a `body` object, both required,
/// and nothing a guard would read. On private protocol two the bootstrap
/// lists it as a write requirement of the token profile, which is what lets
/// an approval policy name it and what makes the host require an approval;
/// protocol one lists no write at all.
#[test]
fn issue_create_is_a_declared_unguarded_write_that_requires_approval() {
    let selection = shipped()
        .into_iter()
        .find(|s| s.id == "issue.create")
        .expect("`issue.create` is not shipped");
    assert_eq!(selection.operation_id, "postApiV4ProjectsIdIssues");
    assert_eq!(selection.effect, Effect::Write);
    assert!(selection.guard.is_none(), "a create has nothing to pin");

    let engine = engine();
    assert_eq!(engine.effect("issue.create"), Some(Effect::Write));
    assert!(
        engine
            .declarations(&[Effect::Read])
            .iter()
            .all(|o| o.id != "issue.create"),
        "`issue.create` is declared a read"
    );
    let declaration = engine
        .declarations(&[Effect::Write])
        .into_iter()
        .find(|o| o.id == "issue.create")
        .expect("`issue.create` is not a declared write");
    let schema = &declaration.input_schema;
    let mut names: Vec<&str> = schema["properties"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    names.sort();
    assert_eq!(names, ["body", "id"]);
    assert_eq!(schema["required"], json!(["body", "id"]));
    assert_eq!(schema["additionalProperties"], json!(false));
    assert_eq!(schema["properties"]["body"]["type"], "object");

    let two = Provider::new(Some(PrivateProtocol::V2));
    let child = Child::spawn(&two.selection()).unwrap();
    let requirement = child
        .bootstrap()
        .requirements
        .iter()
        .find(|r| r.operation == "issue.create")
        .expect("`issue.create` is not a protocol-two requirement");
    assert!(
        requirement.effect == runtime::Effect::Write,
        "`issue.create` is not a write requirement"
    );
    assert_eq!(requirement.profile, PROFILE);

    let one = Provider::new(None);
    let child = Child::spawn(&one.selection()).unwrap();
    assert!(
        child
            .bootstrap()
            .requirements
            .iter()
            .all(|r| r.operation != "issue.create"),
        "protocol one lists `issue.create`"
    );
    assert!(two.requests().is_empty() && one.requests().is_empty());
}

/// Approved, the write sends exactly one request — `POST` on the project's
/// issues route with the body as supplied and the token — and no preflight,
/// and returns GitLab's 201 answer as applied.
#[test]
fn an_approved_issue_create_sends_exactly_the_declared_request_and_body() {
    let provider = Provider::new(Some(PrivateProtocol::V2));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = write(&mut child, "issue.create", &issue_input())
        .unwrap_or_else(|failure| panic!("`issue.create` refused in prepare: {failure:?}"));
    assert_eq!(result.effect, WriteEffect::Applied);
    let result = result
        .result
        .unwrap_or_else(|failure| panic!("`issue.create` result: {failure:?}"));
    assert_eq!(
        provider.requests(),
        [(
            "POST".to_owned(),
            // The transport writes an empty query when no query parameter is
            // bound, so the wire request ends in `?`.
            format!("{ISSUES}?"),
            issue_body()
        )]
    );
    assert_eq!(result["status"], 201);
    assert_eq!(result["body"], created(&issue_body()));
    assert_eq!(result["provenance"]["instance"], INSTANCE);
}

/// Without an approval there is no route to the request. On protocol two the
/// read transport, which carries none, refuses `issue.create` as unsupported;
/// on protocol one it is not an operation and prepare is unsupported. Nothing
/// reaches GitLab.
#[test]
fn issue_create_without_an_approval_is_refused_before_any_request() {
    let two = Provider::new(Some(PrivateProtocol::V2));
    let mut child = Child::spawn(&two.selection()).unwrap();
    let outcome = attempt(&mut child, "issue.create", &issue_input());
    assert!(
        matches!(outcome, Err(Failure::Unsupported)),
        "`issue.create` on the protocol-two read transport: {outcome:?}"
    );
    let one = Provider::new(None);
    let mut child = Child::spawn(&one.selection()).unwrap();
    let outcome = attempt(&mut child, "issue.create", &issue_input());
    assert!(
        matches!(outcome, Err(Failure::NotFound)),
        "`issue.create` on protocol one: {outcome:?}"
    );
    let outcome = write(&mut child, "issue.create", &issue_input());
    assert!(
        matches!(outcome, Err(Failure::Unsupported)),
        "`issue.create` prepared on protocol one: {:?}",
        outcome.map(|result| result.effect)
    );
    assert!(two.requests().is_empty(), "protocol two sent a request");
    assert!(one.requests().is_empty(), "protocol one sent a request");
}

/// Public RFC 8032 §7.1 test material, never a deployment signing key.
const APPROVAL_SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const APPROVAL_PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const APPROVAL_NOW: i64 = 1_789_056_000_000;

struct FixedClock;
impl mutations::Clock for FixedClock {
    fn now(&self) -> mutations::Result<mutations::ClockInterval> {
        Ok(mutations::ClockInterval {
            lower_unix_ms: APPROVAL_NOW,
            upper_unix_ms: APPROVAL_NOW + 2000,
        })
    }
}
/// A key admission that accepts every subject under its key id, so a refusal
/// can only come from the proof itself.
struct Key(approvals::ConfiguredApprovalKey);
impl approvals::CurrentAdmission for &Key {
    fn key(&self) -> &approvals::ConfiguredApprovalKey {
        &self.0
    }
}
impl approvals::ReceiverPolicy for Key {
    type Guard<'a>
        = &'a Key
    where
        Self: 'a;
    fn admit<'a>(&'a self, _: &approvals::Subject, kid: &str) -> approvals::Result<&'a Key> {
        if kid == self.0.kid {
            Ok(self)
        } else {
            Err(approvals::Failure::Refused)
        }
    }
}
impl approvals::IssuancePolicy for Key {
    type Guard<'a>
        = &'a Key
    where
        Self: 'a;
    fn authorize<'a>(
        &'a self,
        subject: &approvals::Subject,
        kid: &str,
    ) -> approvals::Result<&'a Key> {
        approvals::ReceiverPolicy::admit(self, subject, kid)
    }
}

/// The approval subject of a GitLab write, as the owner resolves it: the
/// operation's declared contract and profile, and the digest of the whole
/// input (`crates/connectors-host/src/local/owner/approval_issuance.rs`).
fn subject(operation: &str, input: &Value) -> approvals::Subject {
    let declaration = engine()
        .declarations(&[Effect::Write])
        .into_iter()
        .find(|o| o.id == operation)
        .unwrap();
    approvals::Subject {
        format: "connectors.approval-subject/v1".into(),
        target: approvals::Target {
            instance: INSTANCE.into(),
            operation: operation.into(),
            connection: "fixture-connection".into(),
            connection_revision: "fixture-connection-revision".into(),
            contract: declaration.contract,
            profile: declaration.profile,
            descriptor_revision: "fixture-descriptor-revision".into(),
            configuration_revision: "fixture-configuration-revision".into(),
        },
        authority: approvals::Authority {
            scope: approvals::Scope {
                tenant: None,
                realm: None,
                caller: "fixture-caller".into(),
                executor: None,
            },
            current_authority: None,
            executor: None,
        },
        origin: approvals::Origin {
            kind: approvals::OriginKind::Direct,
            authority_ref: INSTANCE.into(),
        },
        route: None,
        canonicalization: "adapter-v1-canonical-json".into(),
        input_sha256: connectors_core::digest(input),
        approval_mode: "required".into(),
    }
}

/// An approval issued for one issue verifies for exactly that input and is
/// refused for any other — a changed title, description or label, an added
/// body member, another project — and for another write over the same input.
#[test]
fn an_issue_create_approval_for_a_different_input_is_refused() {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let key = Key(approvals::ConfiguredApprovalKey {
        issuer: "fixture-issuer".into(),
        audience: "fixture-audience".into(),
        kid: "fixture-key".into(),
        public_key: URL_SAFE_NO_PAD.encode(hex::decode(APPROVAL_PUBLIC).unwrap()),
        not_before_unix_ms: 0,
        not_after_unix_ms: APPROVAL_NOW + 1_000_000,
        revoked: false,
    });
    let signer = approvals::Signer::from_seed(
        Secret(hex::decode(APPROVAL_SEED).unwrap()),
        "fixture-key".into(),
    )
    .unwrap();
    let change = |pointer: &str, value: Value| {
        let mut changed = issue_input();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        changed.pointer_mut(parent).unwrap()[key] = value;
        changed
    };
    let approved = issue_input();
    let proof = signer
        .issue(&subject("issue.create", &approved), &key, &FixedClock)
        .unwrap();
    assert!(
        approvals::verify(
            &proof,
            &subject("issue.create", &approved),
            &key,
            &FixedClock
        )
        .is_ok(),
        "`issue.create` refused its own approval"
    );
    for other in [
        change("/body/title", json!("Another issue")),
        change("/body/description", json!("changed")),
        change("/body/labels", json!("fixture")),
        change("/body/confidential", json!(true)),
        change("/id", json!("org/other")),
    ] {
        assert_ne!(other, approved);
        let outcome =
            approvals::verify(&proof, &subject("issue.create", &other), &key, &FixedClock);
        assert!(
            matches!(outcome, Err(approvals::Failure::Refused)),
            "`issue.create` accepted its approval for {other}"
        );
    }
    let outcome = approvals::verify(
        &proof,
        &subject("merge_request.create", &approved),
        &key,
        &FixedClock,
    );
    assert!(
        matches!(outcome, Err(approvals::Failure::Refused)),
        "the `issue.create` approval verified for `merge_request.create`"
    );
}

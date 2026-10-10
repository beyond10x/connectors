//! story:parity-gitlab-tags-releases: GitLab tags and releases, selected from
//! the pinned OpenAPI document (`adapters/gitlab/upstream/openapi_v3.yaml`).
//!
//! - `tag.get`, `getApiV4ProjectsIdRepositoryTagsTagName`: one tag.
//! - `tag.create`, `postApiV4ProjectsIdRepositoryTags`: guarded on `body.ref`
//!   resolving to the pinned commit `sha`; proven by the answer naming the tag
//!   at that commit.
//! - `tag.delete`, `deleteApiV4ProjectsIdRepositoryTagsTagName`: guarded on the
//!   tag existing at the pinned commit; proven by the same read after the
//!   delete answering 404 (`postflight.absent`).
//! - `release.get`, `getApiV4ProjectsIdReleasesTagName`: one release.
//! - `release.links`, `getApiV4ProjectsIdReleasesTagNameAssetsLinks`: one
//!   release's asset links, one page per call.
//! - `release.create`, `postApiV4ProjectsIdReleases`: guarded on the tag
//!   existing at the pinned commit; proven by the answer naming the release of
//!   that tag at that commit. The body is closed without `ref` and
//!   `tag_message`, so it releases an existing tag and never creates one.
//! - `release.update`, `putApiV4ProjectsIdReleasesTagName`: guarded on the
//!   release of that tag at the pinned commit, before and after.
//!
//! Reads run through the engine with a recording transport and through a
//! provider child; writes through a private-protocol-two provider child against
//! a disposable HTTPS GitLab, prepared and committed as the host's approval
//! coordinator drives it. The fixture answers in the pinned document's shapes
//! (`APIEntitiesTag`, `APIEntitiesRelease`, `APIEntitiesReleasesLink`,
//! `APIEntitiesCommit`, the fields the guards read) with synthetic ids, keeps
//! its tags and releases, and records each request, so "no request was sent"
//! is a count of what it saw.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure, PrivateProtocol, WriteEffect, WriteResult},
};
use connectors_sdk::{AuthenticatedHttp, HttpResponse, Secret};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, VecDeque},
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

const BASE: &str = "/api/v4";
const INSTANCE: &str = "fixture-gitlab";
/// The fictional token the fixture accepts in `PRIVATE-TOKEN`.
const TOKEN: &str = "fixture-pat-one";
/// The fixture project `org/project`, its path id encoded as one segment.
const PROJECT: &str = "/api/v4/projects/org%2Fproject";

const TAG_GET: &str = "tag.get";
const TAG_CREATE: &str = "tag.create";
const TAG_DELETE: &str = "tag.delete";
const RELEASE_GET: &str = "release.get";
const RELEASE_LINKS: &str = "release.links";
const RELEASE_CREATE: &str = "release.create";
const RELEASE_UPDATE: &str = "release.update";

/// The head of `main`, and every fixture tag's commit.
const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";
/// Another commit.
const OTHER: &str = "89abcdef0123456789abcdef0123456789abcdef";

/// Tagged at `HEAD` and released.
const RELEASED: &str = "v1.0.0";
/// Tagged at `HEAD`, no release yet.
const BARE: &str = "v1.1.0";
/// Tagged at `HEAD`, a name with a slash, sent as one path segment.
const SLASHED: &str = "release/v2";
/// GitLab answers its delete 204 and keeps it, as a GitLab that ignored the
/// delete would.
const STICKY: &str = "sticky";
/// GitLab answers its delete 204, and the read after it 500.
const FLAKY: &str = "flaky";
/// GitLab answers its delete 204, and the read after it 403.
const DENIED: &str = "denied";
/// GitLab answers its delete 204, and the read after it 410.
const GONE: &str = "gone";
/// A branch the fixture reports at `HEAD` but creates a tag from at `OTHER`, as
/// a branch moved between the preflight read and the create would.
const RACING: &str = "racing";

/// Method, request target and body of each fixture request.
type Requests = Arc<Mutex<Vec<(String, String, Vec<u8>)>>>;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value =
        serde_json::from_slice(&fs::read(root().join("providers/gitlab/operations.json")).unwrap())
            .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn bundle() -> connectors_catalog::bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "gitlab").unwrap()
}
fn engine() -> Engine {
    Engine::new(&bundle(), BASE, &shipped()).unwrap()
}

/// `APIEntitiesCommit`, the fields read here.
fn commit(id: &str) -> Value {
    json!({"id": id, "short_id": &id[..8], "title": "fixture commit", "parent_ids": []})
}
/// `APIEntitiesTag`.
fn tag(name: &str, at: &str, released: bool) -> Value {
    json!({"name": name, "message": null, "target": at, "commit": commit(at),
           "release": if released { json!({"tag_name": name, "description": "notes"}) } else { Value::Null },
           "protected": false, "created_at": null})
}
/// `APIEntitiesReleasesLink`.
fn link(id: u64, name: &str) -> Value {
    json!({"id": id, "name": name, "url": format!("https://example.invalid/{name}"),
           "direct_asset_url": format!("https://example.invalid/{name}"), "link_type": "other"})
}
/// `APIEntitiesRelease`.
fn release(tag_name: &str, at: &str, name: &str, description: &str, links: &[Value]) -> Value {
    json!({"name": name, "tag_name": tag_name, "description": description,
           "created_at": "2026-10-01T00:00:00.000Z", "released_at": "2026-10-01T00:00:00.000Z",
           "upcoming_release": false, "commit": commit(at),
           "assets": {"count": links.len(), "sources": [], "links": links}})
}

#[derive(Default)]
struct Fixture {
    /// Each tag's commit, by name.
    tags: BTreeMap<String, String>,
    /// Each release's name, description and links, by tag name.
    releases: BTreeMap<String, (String, String, Vec<Value>)>,
    /// Tags whose read after a delete answers the status beside them.
    failing: Vec<(String, u16)>,
}
impl Fixture {
    fn new() -> Self {
        let mut fixture = Self::default();
        for name in [RELEASED, BARE, SLASHED, STICKY, FLAKY, DENIED, GONE] {
            fixture.tags.insert(name.into(), HEAD.into());
        }
        fixture.releases.insert(
            RELEASED.into(),
            (
                "Version 1".into(),
                "notes".into(),
                vec![link(1, "linux-x86_64"), link(2, "checksums")],
            ),
        );
        fixture
    }
    fn resolve(reference: &str) -> Option<&'static str> {
        match reference {
            "main" | RACING | HEAD => Some(HEAD),
            OTHER => Some(OTHER),
            _ => None,
        }
    }
    fn tag(&self, name: &str) -> Value {
        tag(name, &self.tags[name], self.releases.contains_key(name))
    }
    fn release(&self, name: &str) -> Value {
        let (title, description, links) = &self.releases[name];
        release(name, &self.tags[name], title, description, links)
    }
    fn answer(&mut self, method: &str, route: &str, body: &[u8]) -> (u16, Value) {
        let Some(rest) = route.strip_prefix(&format!("{PROJECT}/")) else {
            return (404, json!({"message": "404 Project Not Found"}));
        };
        let decoded = rest.replace("%2F", "/");
        let input: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
        let not_found = || (404, json!({"message": "404 Not found"}));
        if let Some(reference) = decoded.strip_prefix("repository/commits/") {
            return match (method, Self::resolve(reference)) {
                ("GET", Some(id)) => (200, commit(id)),
                _ => not_found(),
            };
        }
        if decoded == "repository/tags" && method == "POST" {
            let name = input["tag_name"].as_str().unwrap_or_default().to_owned();
            if self.tags.contains_key(&name) {
                return (
                    400,
                    json!({"message": format!("Tag {name} already exists")}),
                );
            }
            let reference = input["ref"].as_str().unwrap_or_default();
            let Some(at) = Self::resolve(reference) else {
                return (400, json!({"message": "Target does not exist"}));
            };
            let at = if reference == RACING { OTHER } else { at };
            self.tags.insert(name.clone(), at.into());
            return (201, self.tag(&name));
        }
        if let Some(name) = decoded.strip_prefix("repository/tags/") {
            if let Some((_, status)) = self.failing.iter().find(|(failing, _)| failing == name) {
                return (*status, json!({"message": format!("{status}")}));
            }
            if !self.tags.contains_key(name) {
                return not_found();
            }
            return match method {
                "GET" => (200, self.tag(name)),
                "DELETE" => {
                    match name {
                        STICKY => {}
                        FLAKY => self.failing.push((name.to_owned(), 500)),
                        DENIED => self.failing.push((name.to_owned(), 403)),
                        GONE => self.failing.push((name.to_owned(), 410)),
                        _ => {
                            self.tags.remove(name);
                        }
                    }
                    (204, Value::Null)
                }
                _ => not_found(),
            };
        }
        if decoded == "releases" && method == "POST" {
            let name = input["tag_name"].as_str().unwrap_or_default().to_owned();
            if self.releases.contains_key(&name) {
                return (409, json!({"message": "Release already exists"}));
            }
            if !self.tags.contains_key(&name) {
                return (422, json!({"message": "Ref is not specified"}));
            }
            self.releases.insert(
                name.clone(),
                (
                    input["name"].as_str().unwrap_or(&name).to_owned(),
                    input["description"].as_str().unwrap_or_default().to_owned(),
                    Vec::new(),
                ),
            );
            return (201, self.release(&name));
        }
        if let Some(rest) = decoded.strip_prefix("releases/") {
            let (name, links) = match rest.strip_suffix("/assets/links") {
                Some(name) => (name, true),
                None => (rest, false),
            };
            if !self.releases.contains_key(name) {
                return not_found();
            }
            return match (method, links) {
                ("GET", true) => (200, Value::Array(self.releases[name].2.clone())),
                ("GET", false) => (200, self.release(name)),
                ("PUT", false) => {
                    let entry = self.releases.get_mut(name).unwrap();
                    if let Some(title) = input["name"].as_str() {
                        entry.0 = title.to_owned();
                    }
                    if let Some(description) = input["description"].as_str() {
                        entry.1 = description.to_owned();
                    }
                    (200, self.release(name))
                }
                _ => not_found(),
            };
        }
        not_found()
    }
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    requests: Requests,
}
impl Provider {
    fn new() -> Self {
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
                let mut fixture = Fixture::new();
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
                    } else {
                        fixture.answer(&method, &route, &body)
                    };
                    observed.lock().unwrap().push((method, target, body));
                    // A 204 carries no body, as GitLab's delete answers.
                    let answer = if status == 204 {
                        Vec::new()
                    } else {
                        serde_json::to_vec(&answer).unwrap()
                    };
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
                    "profile": "gitlab.pat",
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
            private_protocol: Some(PrivateProtocol::V2),
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
fn revision(child: &Child) -> String {
    child.bootstrap().descriptor().unwrap().revision
}
/// Prepare and commit one write on this child: the transport the host's
/// approval coordinator drives once it holds a verified, spent approval. A
/// refusal in prepare is the `Err`.
fn write(child: &mut Child, operation: &str, input: &Value) -> Result<WriteResult, Failure> {
    let revision = revision(child);
    let prepared = child.prepare_write(
        operation,
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        connectors_sdk::now_ms() + 30_000,
    )?;
    Ok(prepared.commit())
}
/// One read on this child, its output parsed.
fn read(child: &mut Child, operation: &str, input: &Value) -> Result<Value, Failure> {
    let revision = revision(child);
    child
        .invoke(
            operation,
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(input).unwrap(),
            connectors_sdk::now_ms() + 30_000,
        )
        .map(|bytes| serde_json::from_slice(&bytes).unwrap())
}
fn declared(operation: &str, effect: Effect) -> connectors_core::Operation {
    engine()
        .declarations(&[effect])
        .into_iter()
        .find(|o| o.id == operation)
        .unwrap_or_else(|| panic!("`{operation}` is not a declared {effect:?}"))
}
fn request(method: &str, path: &str, body: Value) -> (String, String, Value) {
    (method.to_owned(), format!("{PROJECT}/{path}?"), body)
}
fn encoded(name: &str) -> String {
    name.replace('/', "%2F")
}
fn get_tag(name: &str) -> (String, String, Value) {
    request(
        "GET",
        &format!("repository/tags/{}", encoded(name)),
        Value::Null,
    )
}
fn get_release(name: &str) -> (String, String, Value) {
    request("GET", &format!("releases/{}", encoded(name)), Value::Null)
}

/// The seven selections are shipped as reviewed: three reads, and four writes
/// each guarded before and proven after, with the bodies the pinned request
/// schemas give them closed and typed.
#[test]
fn the_tag_and_release_operations_are_shipped_as_reviewed() {
    let selections = shipped();
    let selection = |id: &str| {
        let mut value =
            serde_json::to_value(selections.iter().find(|s| s.id == id).unwrap()).unwrap();
        value.as_object_mut().unwrap().remove("description");
        value
    };
    assert_eq!(
        selection(TAG_GET),
        json!({"id": TAG_GET, "operation_id": "getApiV4ProjectsIdRepositoryTagsTagName",
               "effect": "read", "guard": null, "response": null})
    );
    assert_eq!(
        selection(RELEASE_GET),
        json!({"id": RELEASE_GET, "operation_id": "getApiV4ProjectsIdReleasesTagName",
               "effect": "read", "guard": null, "response": null})
    );
    assert_eq!(
        selection(RELEASE_LINKS),
        json!({"id": RELEASE_LINKS, "operation_id": "getApiV4ProjectsIdReleasesTagNameAssetsLinks",
               "effect": "read", "guard": null, "response": null,
               "bounds": {"per_page": {"minimum": 1, "maximum": 100}}})
    );
    assert_eq!(
        selection(TAG_CREATE),
        json!({"id": TAG_CREATE, "operation_id": "postApiV4ProjectsIdRepositoryTags",
               "effect": "write", "response": null,
               "body_keys": ["tag_name", "ref", "message"],
               "body_types": {"tag_name": "string", "ref": "string", "message": "string"},
               "guard": {
                   "preflight": {"operation_id": "getApiV4ProjectsIdRepositoryCommitsSha",
                                 "values": {"id": "id", "sha": "body.ref"},
                                 "checks": [{"pointer": "/id", "expect": {"input": "sha"}}]},
                   "postflight": {"checks": [{"pointer": "/name", "expect": {"input": "body.tag_name"}},
                                             {"pointer": "/commit/id", "expect": {"input": "sha"}}]}}})
    );
    assert_eq!(
        selection(TAG_DELETE),
        json!({"id": TAG_DELETE, "operation_id": "deleteApiV4ProjectsIdRepositoryTagsTagName",
               "effect": "write", "response": null,
               "guard": {
                   "preflight": {"operation_id": "getApiV4ProjectsIdRepositoryTagsTagName",
                                 "values": {"id": "id", "tag_name": "tag_name"},
                                 "checks": [{"pointer": "/name", "expect": {"input": "tag_name"}},
                                            {"pointer": "/commit/id", "expect": {"input": "sha"}}]},
                   "postflight": {"checks": [],
                                  "read": {"operation_id": "getApiV4ProjectsIdRepositoryTagsTagName",
                                           "values": {"id": "id", "tag_name": "tag_name"}},
                                  "absent": true}}})
    );
    assert_eq!(
        selection(RELEASE_CREATE),
        json!({"id": RELEASE_CREATE, "operation_id": "postApiV4ProjectsIdReleases",
               "effect": "write", "response": null,
               "body_keys": ["tag_name", "name", "description", "released_at", "milestones", "milestone_ids", "assets"],
               "body_types": {"tag_name": "string", "name": "string", "description": "string", "released_at": "string"},
               "guard": {
                   "preflight": {"operation_id": "getApiV4ProjectsIdRepositoryTagsTagName",
                                 "values": {"id": "id", "tag_name": "body.tag_name"},
                                 "checks": [{"pointer": "/commit/id", "expect": {"input": "sha"}}]},
                   "postflight": {"checks": [{"pointer": "/tag_name", "expect": {"input": "body.tag_name"}},
                                             {"pointer": "/commit/id", "expect": {"input": "sha"}}]}}})
    );
    assert_eq!(
        selection(RELEASE_UPDATE),
        json!({"id": RELEASE_UPDATE, "operation_id": "putApiV4ProjectsIdReleasesTagName",
               "effect": "write", "response": null,
               "body_keys": ["name", "description", "released_at", "milestones", "milestone_ids"],
               "body_types": {"name": "string", "description": "string", "released_at": "string"},
               "guard": {
                   "preflight": {"operation_id": "getApiV4ProjectsIdReleasesTagName",
                                 "values": {"id": "id", "tag_name": "tag_name"},
                                 "checks": [{"pointer": "/tag_name", "expect": {"input": "tag_name"}},
                                            {"pointer": "/commit/id", "expect": {"input": "sha"}}]},
                   "postflight": {"checks": [{"pointer": "/tag_name", "expect": {"input": "tag_name"}},
                                             {"pointer": "/commit/id", "expect": {"input": "sha"}}]}}})
    );
    // Each write declares the pinned commit `sha` it is guarded on, and the
    // release create does not admit `ref` or `tag_message`, which would create
    // a tag.
    for write in [TAG_CREATE, TAG_DELETE, RELEASE_CREATE, RELEASE_UPDATE] {
        let declaration = declared(write, Effect::Write);
        assert!(
            declaration.input_schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("sha")),
            "{write}: {}",
            declaration.input_schema
        );
    }
    let create = declared(RELEASE_CREATE, Effect::Write);
    let body = &create.input_schema["properties"]["body"];
    assert_eq!(body["additionalProperties"], false, "{body}");
    assert!(body["properties"].get("ref").is_none(), "{body}");
    assert!(body["properties"].get("tag_message").is_none(), "{body}");
    for read in [TAG_GET, RELEASE_GET, RELEASE_LINKS] {
        declared(read, Effect::Read);
    }
}

type Call = (Vec<String>, Vec<(String, String)>);
/// A recording read transport that answers each read in turn.
struct Reads {
    responses: Mutex<VecDeque<Value>>,
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(
        &self,
        path: &[&str],
        query: &[(&str, String)],
    ) -> connectors_core::Result<HttpResponse> {
        self.calls.lock().unwrap().push((
            path.iter().map(|s| s.to_string()).collect(),
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        let body = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider read");
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }
}

/// Through the engine: each read sends one GET to the route the pinned
/// document declares, the tag name as one path segment, and returns GitLab's
/// body unchanged; the links list is one page, `per_page` within 1 to 100.
#[tokio::test]
async fn the_reads_send_their_route_and_return_the_body_unchanged() {
    let engine = engine();
    let links = json!([link(1, "linux-x86_64"), link(2, "checksums")]);
    let cases = [
        (
            TAG_GET,
            json!({"id": "org/project", "tag_name": SLASHED}),
            tag(SLASHED, HEAD, false),
            vec!["projects", "org/project", "repository", "tags", SLASHED],
            vec![],
        ),
        (
            RELEASE_GET,
            json!({"id": "org/project", "tag_name": RELEASED, "include_html_description": true}),
            release(RELEASED, HEAD, "Version 1", "notes", &[]),
            vec!["projects", "org/project", "releases", RELEASED],
            vec![("include_html_description", "true")],
        ),
        (
            RELEASE_LINKS,
            json!({"id": "org/project", "tag_name": RELEASED, "per_page": 100, "page": 2}),
            links.clone(),
            vec![
                "projects",
                "org/project",
                "releases",
                RELEASED,
                "assets",
                "links",
            ],
            vec![("page", "2"), ("per_page", "100")],
        ),
    ];
    for (operation, input, answer, path, query) in cases {
        let http = Reads {
            responses: Mutex::new(VecDeque::from([answer.clone()])),
            calls: Mutex::new(Vec::new()),
        };
        let output = engine
            .read(&http, "one", operation, input)
            .await
            .unwrap_or_else(|error| panic!("{operation}: {error:?}"));
        assert_eq!(output["status"], 200, "{operation}");
        assert_eq!(output["body"], answer, "{operation}");
        let calls = http.calls.lock().unwrap();
        assert_eq!(calls.len(), 1, "{operation}");
        let mut sent = calls[0].1.clone();
        sent.sort();
        assert_eq!(calls[0].0, path, "{operation}");
        assert_eq!(
            sent,
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<Vec<_>>(),
            "{operation}"
        );
    }
    // A page size outside 1 to 100, or one that is not an integer, is refused before a request.
    for per_page in [json!(0), json!(101), json!("x")] {
        let http = Reads {
            responses: Mutex::new(VecDeque::new()),
            calls: Mutex::new(Vec::new()),
        };
        let refusal = engine
            .read(
                &http,
                INSTANCE,
                RELEASE_LINKS,
                json!({"id": "org/project", "tag_name": RELEASED, "per_page": per_page}),
            )
            .await
            .err()
            .unwrap_or_else(|| panic!("per_page {per_page} admitted"));
        assert_eq!(refusal.code, connectors_core::ErrorCode::InvalidInput);
        assert!(http.calls.lock().unwrap().is_empty());
    }
}

/// Through the provider process: each read answers GitLab's body, and a tag or
/// release GitLab does not find is `not_found`.
#[test]
fn the_reads_answer_through_the_provider_process() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let output = read(
        &mut child,
        TAG_GET,
        &json!({"id": "org/project", "tag_name": SLASHED}),
    )
    .unwrap();
    assert_eq!(output["body"], tag(SLASHED, HEAD, false));
    let output = read(
        &mut child,
        RELEASE_GET,
        &json!({"id": "org/project", "tag_name": RELEASED}),
    )
    .unwrap();
    assert_eq!(output["body"]["tag_name"], RELEASED);
    assert_eq!(output["body"]["commit"]["id"], HEAD);
    let output = read(
        &mut child,
        RELEASE_LINKS,
        &json!({"id": "org/project", "tag_name": RELEASED, "per_page": 20}),
    )
    .unwrap();
    assert_eq!(
        output["body"],
        json!([link(1, "linux-x86_64"), link(2, "checksums")])
    );
    for (operation, name) in [(TAG_GET, "v9"), (RELEASE_GET, BARE), (RELEASE_LINKS, BARE)] {
        assert_eq!(
            read(
                &mut child,
                operation,
                &json!({"id": "org/project", "tag_name": name})
            ),
            Err(Failure::ProviderNotFound),
            "{operation} {name}"
        );
    }
    assert_eq!(
        provider.requests(),
        [
            get_tag(SLASHED),
            get_release(RELEASED),
            (
                "GET".to_owned(),
                format!("{PROJECT}/releases/{RELEASED}/assets/links?per_page=20"),
                Value::Null
            ),
            get_tag("v9"),
            get_release(BARE),
            request("GET", &format!("releases/{BARE}/assets/links"), Value::Null),
        ]
    );
}

fn tag_create_input(name: &str, reference: &str, sha: &str) -> Value {
    json!({"id": "org/project", "sha": sha,
           "body": {"tag_name": name, "ref": reference, "message": "fixture tag"}})
}
fn tag_delete_input(name: &str, sha: &str) -> Value {
    json!({"id": "org/project", "tag_name": name, "sha": sha})
}
fn release_create_input(name: &str, sha: &str) -> Value {
    json!({"id": "org/project", "sha": sha,
           "body": {"tag_name": name, "name": "Version 1.1", "description": "changes"}})
}
fn release_update_input(name: &str, sha: &str) -> Value {
    json!({"id": "org/project", "tag_name": name, "sha": sha,
           "body": {"description": "corrected notes"}})
}

/// Approved, `tag.create` reads the commit `body.ref` names, sends one POST,
/// and is applied when GitLab answers the tag at the pinned commit. A ref
/// moved between the read and the create, so that GitLab tags another commit,
/// leaves the effect unknown.
#[test]
fn tag_create_pins_the_commit_and_proves_the_tag_at_it() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = write(
        &mut child,
        TAG_CREATE,
        &tag_create_input("v2.0.0", "main", HEAD),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 201);
    assert_eq!(output["body"]["name"], "v2.0.0");
    assert_eq!(output["body"]["commit"]["id"], HEAD);
    assert_eq!(
        provider.requests(),
        [
            request("GET", "repository/commits/main", Value::Null),
            request(
                "POST",
                "repository/tags",
                json!({"tag_name": "v2.0.0", "ref": "main", "message": "fixture tag"})
            ),
        ]
    );
    let before = provider.requests().len();
    let result = write(
        &mut child,
        TAG_CREATE,
        &tag_create_input("v2.1.0", RACING, HEAD),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Unknown);
    assert_eq!(result.result, Err(Failure::Protocol));
    assert_eq!(provider.requests().len(), before + 2);
}

/// Approved, `tag.delete` reads the tag, sends one DELETE, reads the tag again
/// and is applied only when that read answers 404: GitLab's 204 carries no
/// body, so the read after is the proof. A tag still found after the delete,
/// or a read after it that answers anything but 404 (500, 403, 410), leaves the
/// effect unknown.
#[test]
fn tag_delete_proves_the_tag_before_and_its_absence_after() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let delete = |name: &str| {
        request(
            "DELETE",
            &format!("repository/tags/{}", encoded(name)),
            Value::Null,
        )
    };
    let result = write(&mut child, TAG_DELETE, &tag_delete_input(SLASHED, HEAD)).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 204);
    assert_eq!(output["body"], Value::Null);
    assert_eq!(
        provider.requests(),
        [get_tag(SLASHED), delete(SLASHED), get_tag(SLASHED)]
    );
    for name in [STICKY, FLAKY, DENIED, GONE] {
        let before = provider.requests().len();
        let result = write(&mut child, TAG_DELETE, &tag_delete_input(name, HEAD)).unwrap();
        assert_eq!(result.effect, WriteEffect::Unknown, "{name}");
        assert_eq!(result.result, Err(Failure::Protocol), "{name}");
        assert_eq!(
            provider.requests()[before..],
            [get_tag(name), delete(name), get_tag(name)],
            "{name}"
        );
    }
}

/// Approved, `release.create` reads the tag, sends one POST and is applied when
/// GitLab answers the release of that tag at the pinned commit; `release.update`
/// reads the release, sends one PUT and is applied when GitLab answers it still
/// at that tag and commit.
#[test]
fn release_create_and_update_pin_the_tag_commit() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = write(
        &mut child,
        RELEASE_CREATE,
        &release_create_input(BARE, HEAD),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 201);
    assert_eq!(output["body"]["tag_name"], BARE);
    assert_eq!(output["body"]["name"], "Version 1.1");
    assert_eq!(
        provider.requests(),
        [
            get_tag(BARE),
            request(
                "POST",
                "releases",
                json!({"tag_name": BARE, "name": "Version 1.1", "description": "changes"})
            ),
        ]
    );
    let before = provider.requests().len();
    let result = write(
        &mut child,
        RELEASE_UPDATE,
        &release_update_input(RELEASED, HEAD),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 200);
    assert_eq!(output["body"]["description"], "corrected notes");
    assert_eq!(
        provider.requests()[before..],
        [
            get_release(RELEASED),
            request(
                "PUT",
                &format!("releases/{RELEASED}"),
                json!({"description": "corrected notes"})
            ),
        ]
    );
    // A second create of the same release is GitLab's definite 409.
    let before = provider.requests().len();
    let result = write(
        &mut child,
        RELEASE_CREATE,
        &release_create_input(BARE, HEAD),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Refused);
    assert_eq!(provider.requests().len(), before + 2);
}

/// Each guard refuses before its write: a ref at another commit or one GitLab
/// does not find, a tag at another commit or one it does not find, a release
/// at another commit or one it does not find. Each sends only its read.
#[test]
fn each_preflight_mismatch_is_refused_before_the_write() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let commit_read = |reference: &str| {
        request(
            "GET",
            &format!("repository/commits/{reference}"),
            Value::Null,
        )
    };
    for (operation, input, read) in [
        (
            TAG_CREATE,
            tag_create_input("v3", "main", OTHER),
            commit_read("main"),
        ),
        (
            TAG_CREATE,
            tag_create_input("v3", "gone", HEAD),
            commit_read("gone"),
        ),
        (TAG_DELETE, tag_delete_input(BARE, OTHER), get_tag(BARE)),
        (TAG_DELETE, tag_delete_input("v9", HEAD), get_tag("v9")),
        (
            RELEASE_CREATE,
            release_create_input(BARE, OTHER),
            get_tag(BARE),
        ),
        (
            RELEASE_CREATE,
            release_create_input("v9", HEAD),
            get_tag("v9"),
        ),
        (
            RELEASE_UPDATE,
            release_update_input(RELEASED, OTHER),
            get_release(RELEASED),
        ),
        (
            RELEASE_UPDATE,
            release_update_input(BARE, HEAD),
            get_release(BARE),
        ),
    ] {
        let before = provider.requests().len();
        let outcome = write(&mut child, operation, &input).map(|result| result.effect);
        assert_eq!(outcome, Err(Failure::Forbidden), "{operation} {input}");
        assert_eq!(provider.requests()[before..], [read], "{operation} {input}");
    }
}

/// A body outside the pinned request schema's keys, or one without the pinned
/// `sha`, is refused before any request: `ref` and `tag_message` on a release
/// create, which would create a tag; an unknown key on each write; a
/// non-string tag name.
#[test]
fn a_body_outside_the_closed_keys_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut cases = Vec::new();
    for (key, value) in [
        ("ref", json!("main")),
        ("tag_message", json!("annotated")),
        ("legacy_catalog_publish", json!(true)),
    ] {
        let mut input = release_create_input("v4", HEAD);
        input["body"][key] = value;
        cases.push((RELEASE_CREATE, input));
    }
    let mut input = tag_create_input("v4", "main", HEAD);
    input["body"]["release_description"] = json!("old api");
    cases.push((TAG_CREATE, input));
    let mut input = tag_create_input("v4", "main", HEAD);
    input["body"]["tag_name"] = json!(4);
    cases.push((TAG_CREATE, input));
    let mut input = release_update_input(RELEASED, HEAD);
    input["body"]["tag_name"] = json!("v5");
    cases.push((RELEASE_UPDATE, input));
    let mut input = tag_create_input("v4", "main", HEAD);
    input.as_object_mut().unwrap().remove("sha");
    cases.push((TAG_CREATE, input));
    let mut input = tag_delete_input(BARE, HEAD);
    input.as_object_mut().unwrap().remove("sha");
    cases.push((TAG_DELETE, input));
    for (operation, input) in cases {
        let outcome = write(&mut child, operation, &input).map(|result| result.effect);
        assert_eq!(outcome, Err(Failure::InvalidInput), "{operation} {input}");
    }
    assert!(provider.requests().is_empty(), "{:?}", provider.requests());
}

/// `postflight.absent` is refused when the selection loads unless it is `true`,
/// beside a `read`, with no `checks` and no `any_of`; a postflight `read`
/// without checks is still refused unless it is `absent`.
#[test]
fn a_malformed_absent_postflight_is_refused_when_the_selection_loads() {
    let bundle = bundle();
    let base = shipped();
    let with = |change: &dyn Fn(&mut Value)| -> Result<Engine, String> {
        let mut selections: Vec<Value> = base
            .iter()
            .map(|s| serde_json::to_value(s).unwrap())
            .collect();
        let selection = selections
            .iter_mut()
            .find(|s| s["id"] == TAG_DELETE)
            .unwrap();
        change(&mut selection["guard"]["postflight"]);
        let selections: Vec<Selection> =
            serde_json::from_value(Value::Array(selections)).map_err(|e| e.to_string())?;
        Engine::new(&bundle, BASE, &selections).map_err(|e| e.to_string())
    };
    assert!(with(&|_| {}).is_ok());
    let check = json!({"pointer": "/name", "expect": {"input": "tag_name"}});
    for (name, change) in [
        (
            "absent false",
            Box::new(|p: &mut Value| p["absent"] = json!(false)) as Box<dyn Fn(&mut Value)>,
        ),
        (
            "absent null",
            Box::new(|p: &mut Value| p["absent"] = Value::Null),
        ),
        (
            "absent without a read",
            Box::new(|p: &mut Value| {
                p.as_object_mut().unwrap().remove("read");
            }),
        ),
        (
            "absent with a check",
            Box::new({
                let check = check.clone();
                move |p: &mut Value| p["checks"] = json!([check.clone()])
            }),
        ),
        (
            "absent with an any_of",
            Box::new({
                let check = check.clone();
                move |p: &mut Value| p["any_of"] = json!([check.clone(), check.clone()])
            }),
        ),
        (
            "a read without checks and without absent",
            Box::new(|p: &mut Value| {
                p.as_object_mut().unwrap().remove("absent");
            }),
        ),
    ] {
        assert!(with(&*change).is_err(), "{name} loaded");
    }
}

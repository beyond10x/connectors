//! story:parity-gitlab-ci-writes and story:parity-gitlab-project-writes:
//! GitLab pipeline, environment, commit, file, branch and project operations,
//! selected from the pinned OpenAPI document
//! (`adapters/gitlab/upstream/openapi_v3.yaml`).
//!
//! - `pipeline.retry`, `postApiV4ProjectsIdPipelinesPipelineIdRetry`, and
//!   `pipeline.cancel`, `postApiV4ProjectsIdPipelinesPipelineIdCancel`: guarded
//!   on the pipeline existing for the pinned commit `sha`; proven by the answer
//!   naming the same pipeline at that commit.
//! - `pipeline.create`, `postApiV4ProjectsIdPipeline`: guarded on `body.ref`
//!   resolving to `sha`; proven by the answer naming a pipeline for that ref at
//!   that commit.
//! - `environments.list`, `getApiV4ProjectsIdEnvironments`: one page.
//! - `commit.create`, `postApiV4ProjectsIdRepositoryCommits`: guarded on the
//!   branch head being `sha`; proven by the new commit's first parent being
//!   `sha`. The body admits no `start_branch`, `start_sha`, `start_project` or
//!   `force`.
//! - `file.update`, `putApiV4ProjectsIdRepositoryFilesFilePath`: guarded on the
//!   branch head being `sha`; GitLab answers only the file path and branch, so
//!   the proof is a read of the branch's commit after the write, whose first
//!   parent must be `sha`.
//! - `branch.create`, `postApiV4ProjectsIdRepositoryBranches`: guarded on
//!   `body.ref` resolving to `sha`; proven by the answer naming the branch at it.
//! - `branch.delete`, `deleteApiV4ProjectsIdRepositoryBranchesBranch`: guarded
//!   on the branch existing at `sha`; proven by the same read after the delete
//!   answering 404 (`postflight.absent`).
//! - `project.create`, `postApiV4Projects`: guarded on the namespace read by its
//!   path `namespace` having the id `body.namespace_id`; proven by the answer
//!   naming the project `body.path` in that namespace.
//!
//! The pinned document declares the bodies of `commit.create`, `file.update`
//! and `project.create` only as `multipart/form-data`; GitLab's reference
//! documents them as JSON, and the cited amendments in
//! `adapters/gitlab/upstream/openapi_v3.amendments.json` correct the bundle's
//! request media type to `application/json`, the body the engine sends.
//!
//! Reads run through the engine with a recording transport and through a
//! provider child; writes through a private-protocol-two provider child against
//! a disposable HTTPS GitLab, prepared and committed as the host's approval
//! coordinator drives it. The fixture answers in the pinned document's shapes
//! (`APIEntitiesCiPipeline`, `APIEntitiesEnvironment`, `APIEntitiesCommitDetail`,
//! `APIEntitiesBranch`, `APIEntitiesProject` and the namespace, the fields the
//! guards read) with synthetic ids, keeps its state, and records each request
//! with its content type, so "no request was sent" is a count of what it saw.
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
    collections::{BTreeMap, BTreeSet, VecDeque},
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

const PIPELINE_RETRY: &str = "pipeline.retry";
const PIPELINE_CANCEL: &str = "pipeline.cancel";
const PIPELINE_CREATE: &str = "pipeline.create";
const ENVIRONMENTS_LIST: &str = "environments.list";
const COMMIT_CREATE: &str = "commit.create";
const FILE_UPDATE: &str = "file.update";
const BRANCH_CREATE: &str = "branch.create";
const BRANCH_DELETE: &str = "branch.delete";
const PROJECT_CREATE: &str = "project.create";

/// The head of every fixture branch, and the commit of pipelines 18, 19 and 21.
const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";
/// Another commit, that of pipeline 20.
const OTHER: &str = "89abcdef0123456789abcdef0123456789abcdef";

/// Failed at `HEAD`: a retry runs it again.
const FAILED: u64 = 18;
/// Running at `HEAD`: a cancel stops it.
const RUNNING: u64 = 19;
/// Failed at `OTHER`.
const ELSEWHERE: u64 = 20;
/// Read at `HEAD`, but GitLab's answer to a retry or cancel names `OTHER`, as
/// an answer for another commit would.
const DRIFTING: u64 = 21;

/// A branch, a ref and a project path the fixture reports at `HEAD` but writes
/// onto `OTHER`, as a branch moved between the preflight read and the write
/// would.
const RACING: &str = "racing";
/// GitLab answers its delete 204 and keeps it, as a GitLab that ignored the
/// delete would.
const STICKY: &str = "sticky";
/// After a write, GitLab answers the read of this branch (and of its commit)
/// 500.
const FLAKY: &str = "flaky";
/// After a write, the read answers 403.
const DENIED: &str = "denied";
/// After a write, the read answers 410.
const GONE: &str = "gone";

/// The namespace `org`, id 42, and its subgroup `org/team`, id 43.
const ORG: u64 = 42;
const TEAM: u64 = 43;

/// Method, request target, content type and body of each fixture request.
type Requests = Arc<Mutex<Vec<(String, String, Option<String>, Vec<u8>)>>>;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped_file() -> Value {
    serde_json::from_slice(&fs::read(root().join("providers/gitlab/operations.json")).unwrap())
        .unwrap()
}
fn shipped() -> Vec<Selection> {
    serde_json::from_value(shipped_file()["operations"].clone()).unwrap()
}
fn bundle() -> connectors_catalog::bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "gitlab").unwrap()
}
fn engine() -> Engine {
    Engine::new(&bundle(), BASE, &shipped()).unwrap()
}

/// `APIEntitiesCommitDetail`, the fields read here.
fn commit(id: &str, parent: &str) -> Value {
    json!({"id": id, "short_id": &id[..8], "title": "fixture commit",
           "parent_ids": [parent], "message": "fixture commit"})
}
/// `APIEntitiesBranch`.
fn branch(name: &str, head: &str, parent: &str) -> Value {
    json!({"name": name, "commit": commit(head, parent), "merged": false, "protected": false,
           "default": name == "main", "developers_can_push": false, "developers_can_merge": false,
           "can_push": true, "web_url": format!("https://example.invalid/-/tree/{name}")})
}
/// `APIEntitiesCiPipeline`.
fn pipeline(id: u64, reference: &str, sha: &str, status: &str) -> Value {
    json!({"id": id, "iid": id, "project_id": 7, "sha": sha, "ref": reference, "status": status,
           "source": "push", "created_at": "2026-10-01T00:00:00.000Z",
           "updated_at": "2026-10-01T00:00:00.000Z",
           "web_url": format!("https://example.invalid/-/pipelines/{id}")})
}
/// `APIEntitiesEnvironment`.
fn environment(id: u64, name: &str, state: &str) -> Value {
    json!({"id": id, "name": name, "slug": name.replace('/', "-"), "external_url": null,
           "state": state, "tier": "production", "created_at": "2026-10-01T00:00:00.000Z",
           "updated_at": "2026-10-01T00:00:00.000Z"})
}
/// The namespace read's answer.
fn namespace(id: u64, full_path: &str) -> Value {
    json!({"id": id, "name": full_path.rsplit('/').next().unwrap(),
           "path": full_path.rsplit('/').next().unwrap(), "kind": "group",
           "full_path": full_path, "parent_id": if id == TEAM { json!(ORG) } else { Value::Null },
           "web_url": format!("https://example.invalid/groups/{full_path}")})
}

#[derive(Default)]
struct Fixture {
    /// Each branch's head, by name.
    branches: BTreeMap<String, String>,
    /// Each known commit's first parent, by id.
    commits: BTreeMap<String, String>,
    /// Each pipeline's ref, commit and status, by id.
    pipelines: BTreeMap<u64, (String, String, String)>,
    /// Each project, as namespace id and path.
    projects: BTreeSet<(u64, String)>,
    /// Decoded project-relative routes whose read answers the status beside
    /// them.
    failing: Vec<(String, u16)>,
    /// The next synthetic id.
    next: u64,
}
impl Fixture {
    fn new() -> Self {
        let mut fixture = Self {
            next: 100,
            ..Self::default()
        };
        fixture.commits.insert(HEAD.into(), OTHER.into());
        fixture.commits.insert(OTHER.into(), OTHER.into());
        for name in ["main", "feature", RACING, STICKY, FLAKY, DENIED, GONE] {
            fixture.branches.insert(name.into(), HEAD.into());
        }
        for (id, sha, status) in [
            (FAILED, HEAD, "failed"),
            (RUNNING, HEAD, "running"),
            (ELSEWHERE, OTHER, "failed"),
            (DRIFTING, HEAD, "failed"),
        ] {
            fixture
                .pipelines
                .insert(id, ("main".into(), sha.into(), status.into()));
        }
        fixture.projects.insert((TEAM, "existing".into()));
        fixture
    }
    fn id(&mut self) -> u64 {
        self.next += 1;
        self.next
    }
    fn new_commit(&mut self, parent: &str) -> String {
        let id = format!("{:040x}", self.id());
        self.commits.insert(id.clone(), parent.into());
        id
    }
    /// A branch name or a known commit id, to its commit.
    fn resolve(&self, reference: &str) -> Option<String> {
        self.branches.get(reference).cloned().or_else(|| {
            self.commits
                .get_key_value(reference)
                .map(|(id, _)| id.clone())
        })
    }
    fn commit(&self, id: &str) -> Value {
        commit(id, &self.commits[id])
    }
    fn branch(&self, name: &str) -> Value {
        let head = &self.branches[name];
        branch(name, head, &self.commits[head])
    }
    fn pipeline(&self, id: u64) -> Value {
        let (reference, sha, status) = &self.pipelines[&id];
        pipeline(id, reference, sha, status)
    }
    /// Commits onto `name` as GitLab would: the new commit's parent is the
    /// branch head, or `OTHER` on the racing branch.
    fn commit_onto(&mut self, name: &str) -> String {
        let parent = if name == RACING {
            OTHER.to_owned()
        } else {
            self.branches[name].clone()
        };
        let id = self.new_commit(&parent);
        self.branches.insert(name.into(), id.clone());
        if name == FLAKY || name == DENIED || name == GONE {
            let status = match name {
                FLAKY => 500,
                DENIED => 403,
                _ => 410,
            };
            self.failing
                .push((format!("repository/commits/{name}"), status));
        }
        id
    }
    fn answer(&mut self, method: &str, route: &str, body: &[u8]) -> (u16, Value) {
        let input: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
        let not_found = || (404, json!({"message": "404 Not found"}));
        if let Some(id) = route.strip_prefix("/api/v4/namespaces/") {
            return match (method, id.replace("%2F", "/").as_str()) {
                ("GET", "org" | "42") => (200, namespace(ORG, "org")),
                ("GET", "org/team" | "43") => (200, namespace(TEAM, "org/team")),
                _ => (404, json!({"message": "404 Namespace Not Found"})),
            };
        }
        if route == "/api/v4/projects" && method == "POST" {
            let path = input["path"].as_str().unwrap_or_default().to_owned();
            let Some(namespace_id) = input["namespace_id"].as_u64() else {
                return (400, json!({"message": "namespace_id is invalid"}));
            };
            let full_path = match namespace_id {
                ORG => "org",
                TEAM => "org/team",
                _ => return (404, json!({"message": "404 Namespace Not Found"})),
            };
            if !self.projects.insert((namespace_id, path.clone())) {
                return (
                    400,
                    json!({"message": {"path": ["has already been taken"]}}),
                );
            }
            // The racing path lands in `org` whatever was asked, as a request
            // GitLab placed elsewhere would.
            let (namespace_id, full_path) = if path == RACING {
                (ORG, "org")
            } else {
                (namespace_id, full_path)
            };
            let id = self.id();
            return (
                201,
                json!({"id": id, "name": input["name"].as_str().unwrap_or(&path), "path": path,
                       "path_with_namespace": format!("{full_path}/{path}"),
                       "description": input["description"], "visibility": input["visibility"].as_str().unwrap_or("private"),
                       "default_branch": input["default_branch"].as_str().unwrap_or("main"),
                       "namespace": {"id": namespace_id, "full_path": full_path, "kind": "group"}}),
            );
        }
        let Some(rest) = route.strip_prefix(&format!("{PROJECT}/")) else {
            return (404, json!({"message": "404 Project Not Found"}));
        };
        let decoded = rest.replace("%2F", "/");
        if let Some((_, status)) = self.failing.iter().find(|(failing, _)| *failing == decoded) {
            return (*status, json!({"message": format!("{status}")}));
        }
        if decoded == "environments" && method == "GET" {
            return (
                200,
                json!([
                    environment(1, "production", "available"),
                    environment(2, "review/feature", "stopped")
                ]),
            );
        }
        if decoded == "pipeline" && method == "POST" {
            let reference = input["ref"].as_str().unwrap_or_default().to_owned();
            let Some(sha) = self.resolve(&reference) else {
                return (400, json!({"message": {"base": ["Reference not found"]}}));
            };
            let sha = if reference == RACING {
                OTHER.to_owned()
            } else {
                sha
            };
            let id = self.id();
            self.pipelines
                .insert(id, (reference, sha, "created".into()));
            return (201, self.pipeline(id));
        }
        if let Some(rest) = decoded.strip_prefix("pipelines/") {
            let (id, action) = match rest.split_once('/') {
                Some((id, action)) => (id, Some(action)),
                None => (rest, None),
            };
            let Some(id) = id
                .parse::<u64>()
                .ok()
                .filter(|id| self.pipelines.contains_key(id))
            else {
                return not_found();
            };
            let status = match (method, action) {
                ("GET", None) => return (200, self.pipeline(id)),
                ("POST", Some("retry")) => "running",
                ("POST", Some("cancel")) => "canceled",
                _ => return not_found(),
            };
            let entry = self.pipelines.get_mut(&id).unwrap();
            // GitLab retries only failed or canceled jobs, and cancels only a
            // pipeline that has not finished.
            let changes = match status {
                "running" => entry.2 == "failed" || entry.2 == "canceled",
                _ => entry.2 == "running" || entry.2 == "pending" || entry.2 == "created",
            };
            if changes {
                entry.2 = status.into();
            }
            let mut answer = self.pipeline(id);
            if id == DRIFTING {
                answer["sha"] = json!(OTHER);
            }
            return (201, answer);
        }
        if let Some(reference) = decoded.strip_prefix("repository/commits/") {
            return match (method, self.resolve(reference)) {
                ("GET", Some(id)) => (200, self.commit(&id)),
                _ => not_found(),
            };
        }
        if decoded == "repository/commits" && method == "POST" {
            let name = input["branch"].as_str().unwrap_or_default().to_owned();
            if !self.branches.contains_key(&name) {
                return (
                    400,
                    json!({"message": "You can only create or edit files when you are on a branch"}),
                );
            }
            let id = self.commit_onto(&name);
            return (201, self.commit(&id));
        }
        if let Some(path) = decoded.strip_prefix("repository/files/") {
            if method != "PUT" {
                return not_found();
            }
            let name = input["branch"].as_str().unwrap_or_default().to_owned();
            if !self.branches.contains_key(&name) || path == "missing.txt" {
                return (
                    400,
                    json!({"message": "A file with this name doesn't exist"}),
                );
            }
            self.commit_onto(&name);
            return (200, json!({"file_path": path, "branch": name}));
        }
        if decoded == "repository/branches" && method == "POST" {
            let name = input["branch"].as_str().unwrap_or_default().to_owned();
            if self.branches.contains_key(&name) {
                return (400, json!({"message": "Branch already exists"}));
            }
            let reference = input["ref"].as_str().unwrap_or_default();
            let Some(at) = self.resolve(reference) else {
                return (400, json!({"message": "Invalid reference name"}));
            };
            let at = if reference == RACING {
                OTHER.to_owned()
            } else {
                at
            };
            self.branches.insert(name.clone(), at);
            return (201, self.branch(&name));
        }
        if let Some(name) = decoded.strip_prefix("repository/branches/") {
            if !self.branches.contains_key(name) {
                return (404, json!({"message": "404 Branch Not Found"}));
            }
            return match method {
                "GET" => (200, self.branch(name)),
                "DELETE" => {
                    match name {
                        STICKY => {}
                        FLAKY => self.failing.push((decoded.clone(), 500)),
                        DENIED => self.failing.push((decoded.clone(), 403)),
                        GONE => self.failing.push((decoded.clone(), 410)),
                        _ => {
                            self.branches.remove(name);
                        }
                    }
                    (204, Value::Null)
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
                    observed
                        .lock()
                        .unwrap()
                        .push((method, target, header("content-type"), body));
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
            .map(|(method, target, _, body)| {
                let body = if body.is_empty() {
                    Value::Null
                } else {
                    serde_json::from_slice(body).unwrap()
                };
                (method.clone(), target.clone(), body)
            })
            .collect()
    }
    /// The content type of every request that carried a body, by method and
    /// target.
    fn content_types(&self) -> Vec<(String, String, Option<String>)> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, _, _, body)| !body.is_empty())
            .map(|(method, target, kind, _)| (method.clone(), target.clone(), kind.clone()))
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
fn get_pipeline(id: u64) -> (String, String, Value) {
    request("GET", &format!("pipelines/{id}"), Value::Null)
}
fn get_branch(name: &str) -> (String, String, Value) {
    request("GET", &format!("repository/branches/{name}"), Value::Null)
}
fn get_commit(reference: &str) -> (String, String, Value) {
    request(
        "GET",
        &format!("repository/commits/{reference}"),
        Value::Null,
    )
}
fn get_namespace(path: &str) -> (String, String, Value) {
    (
        "GET".to_owned(),
        format!("/api/v4/namespaces/{}?", path.replace('/', "%2F")),
        Value::Null,
    )
}

fn pipeline_input(id: u64, sha: &str) -> Value {
    json!({"id": "org/project", "pipeline_id": id, "sha": sha})
}
fn pipeline_create_input(reference: &str, sha: &str) -> Value {
    json!({"id": "org/project", "sha": sha,
           "body": {"ref": reference, "variables": [{"key": "DEPLOY", "value": "true", "variable_type": "env_var"}]}})
}
fn commit_create_input(branch: &str, sha: &str) -> Value {
    json!({"id": "org/project", "sha": sha,
           "body": {"branch": branch, "commit_message": "fixture change",
                    "actions": [{"action": "create", "file_path": "docs/new.md", "content": "new"},
                                {"action": "update", "file_path": "README.md", "content": "changed"}]}})
}
fn file_update_input(path: &str, branch: &str, sha: &str) -> Value {
    json!({"id": "org/project", "file_path": path, "sha": sha,
           "body": {"branch": branch, "content": "changed", "commit_message": "update file"}})
}
fn branch_create_input(name: &str, reference: &str, sha: &str) -> Value {
    json!({"id": "org/project", "sha": sha, "body": {"branch": name, "ref": reference}})
}
fn branch_delete_input(name: &str, sha: &str) -> Value {
    json!({"id": "org/project", "branch": name, "sha": sha})
}
fn project_create_input(namespace: &str, namespace_id: u64, path: &str) -> Value {
    json!({"namespace": namespace,
           "body": {"name": "New project", "path": path, "namespace_id": namespace_id,
                    "visibility": "private", "initialize_with_readme": true}})
}

/// The selections, in the shipped file, without their descriptions.
fn selection(id: &str) -> Value {
    let file = shipped_file();
    let mut value = file["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == id)
        .unwrap_or_else(|| panic!("`{id}` is not shipped"))
        .clone();
    value.as_object_mut().unwrap().remove("description");
    value
}

/// The nine selections are shipped as reviewed: one read, and eight writes each
/// guarded before and proven after, their bodies closed to the keys GitLab's
/// reference gives them, without the keys that would create a branch, rewrite
/// history, or import or template a project.
#[test]
fn the_ci_and_project_operations_are_shipped_as_reviewed() {
    let pipeline_guard = json!({
        "preflight": {"operation_id": "getApiV4ProjectsIdPipelinesPipelineId",
                      "values": {"id": "id", "pipeline_id": "pipeline_id"},
                      "checks": [{"pointer": "/id", "expect": {"input": "pipeline_id"}},
                                 {"pointer": "/sha", "expect": {"input": "sha"}}]},
        "postflight": {"checks": [{"pointer": "/id", "expect": {"input": "pipeline_id"}},
                                  {"pointer": "/sha", "expect": {"input": "sha"}}]}});
    assert_eq!(
        selection(PIPELINE_RETRY),
        json!({"id": PIPELINE_RETRY, "operation_id": "postApiV4ProjectsIdPipelinesPipelineIdRetry",
               "effect": "write", "guard": pipeline_guard})
    );
    assert_eq!(
        selection(PIPELINE_CANCEL),
        json!({"id": PIPELINE_CANCEL, "operation_id": "postApiV4ProjectsIdPipelinesPipelineIdCancel",
               "effect": "write", "guard": pipeline_guard})
    );
    assert_eq!(
        selection(PIPELINE_CREATE),
        json!({"id": PIPELINE_CREATE, "operation_id": "postApiV4ProjectsIdPipeline",
               "effect": "write", "body_keys": ["ref", "variables", "inputs"],
               "body_types": {"ref": "string"},
               "guard": {
                   "preflight": {"operation_id": "getApiV4ProjectsIdRepositoryCommitsSha",
                                 "values": {"id": "id", "sha": "body.ref"},
                                 "checks": [{"pointer": "/id", "expect": {"input": "sha"}}]},
                   "postflight": {"checks": [{"pointer": "/ref", "expect": {"input": "body.ref"}},
                                             {"pointer": "/sha", "expect": {"input": "sha"}}]}}})
    );
    assert_eq!(
        selection(ENVIRONMENTS_LIST),
        json!({"id": ENVIRONMENTS_LIST, "operation_id": "getApiV4ProjectsIdEnvironments",
               "effect": "read", "bounds": {"per_page": {"minimum": 1, "maximum": 100}}})
    );
    let branch_head = json!({"operation_id": "getApiV4ProjectsIdRepositoryBranchesBranch",
                             "values": {"id": "id", "branch": "body.branch"},
                             "checks": [{"pointer": "/commit/id", "expect": {"input": "sha"}}]});
    let first_parent = json!([{"pointer": "/parent_ids/0", "expect": {"input": "sha"}}]);
    assert_eq!(
        selection(COMMIT_CREATE),
        json!({"id": COMMIT_CREATE, "operation_id": "postApiV4ProjectsIdRepositoryCommits",
               "effect": "write",
               "body_keys": ["branch", "commit_message", "actions", "author_email", "author_name", "stats"],
               "body_types": {"branch": "string", "commit_message": "string", "author_email": "string",
                              "author_name": "string", "stats": "boolean"},
               "body_required": ["commit_message", "actions"],
               "guard": {"preflight": branch_head, "postflight": {"checks": first_parent}}})
    );
    assert_eq!(
        selection(FILE_UPDATE),
        json!({"id": FILE_UPDATE, "operation_id": "putApiV4ProjectsIdRepositoryFilesFilePath",
               "effect": "write",
               "body_keys": ["branch", "content", "commit_message", "encoding", "author_email",
                             "author_name", "last_commit_id"],
               "body_types": {"branch": "string", "content": "string", "commit_message": "string",
                              "encoding": "string", "author_email": "string", "author_name": "string",
                              "last_commit_id": "string"},
               "body_required": ["content", "commit_message"],
               "guard": {"preflight": branch_head,
                         "postflight": {"checks": first_parent,
                                        "read": {"operation_id": "getApiV4ProjectsIdRepositoryCommitsSha",
                                                 "values": {"id": "id", "sha": "body.branch"}}}}})
    );
    assert_eq!(
        selection(BRANCH_CREATE),
        json!({"id": BRANCH_CREATE, "operation_id": "postApiV4ProjectsIdRepositoryBranches",
               "effect": "write", "body_keys": ["branch", "ref"],
               "body_types": {"branch": "string", "ref": "string"},
               "guard": {
                   "preflight": {"operation_id": "getApiV4ProjectsIdRepositoryCommitsSha",
                                 "values": {"id": "id", "sha": "body.ref"},
                                 "checks": [{"pointer": "/id", "expect": {"input": "sha"}}]},
                   "postflight": {"checks": [{"pointer": "/name", "expect": {"input": "body.branch"}},
                                             {"pointer": "/commit/id", "expect": {"input": "sha"}}]}}})
    );
    assert_eq!(
        selection(BRANCH_DELETE),
        json!({"id": BRANCH_DELETE, "operation_id": "deleteApiV4ProjectsIdRepositoryBranchesBranch",
               "effect": "write",
               "guard": {
                   "preflight": {"operation_id": "getApiV4ProjectsIdRepositoryBranchesBranch",
                                 "values": {"id": "id", "branch": "branch"},
                                 "checks": [{"pointer": "/name", "expect": {"input": "branch"}},
                                            {"pointer": "/commit/id", "expect": {"input": "sha"}}]},
                   "postflight": {"checks": [],
                                  "read": {"operation_id": "getApiV4ProjectsIdRepositoryBranchesBranch",
                                           "values": {"id": "id", "branch": "branch"}},
                                  "absent": true}}})
    );
    assert_eq!(
        selection(PROJECT_CREATE),
        json!({"id": PROJECT_CREATE, "operation_id": "postApiV4Projects", "effect": "write",
               "body_keys": ["name", "path", "namespace_id", "description", "visibility",
                             "default_branch", "initialize_with_readme"],
               "body_types": {"name": "string", "path": "string", "namespace_id": "integer",
                              "description": "string", "visibility": "string",
                              "default_branch": "string", "initialize_with_readme": "boolean"},
               "guard": {
                   "preflight": {"operation_id": "getApiV4NamespacesId",
                                 "values": {"id": "namespace"},
                                 "checks": [{"pointer": "/id", "expect": {"input": "body.namespace_id"}},
                                            {"pointer": "/full_path", "expect": {"input": "namespace"}}]},
                   "postflight": {"checks": [{"pointer": "/path", "expect": {"input": "body.path"}},
                                             {"pointer": "/namespace/id", "expect": {"input": "body.namespace_id"}}]}}})
    );
    // Each write but the project create declares the pinned commit `sha` it is
    // guarded on; the project create declares the namespace path it resolves.
    for (write, pinned) in [
        (PIPELINE_RETRY, "sha"),
        (PIPELINE_CANCEL, "sha"),
        (PIPELINE_CREATE, "sha"),
        (COMMIT_CREATE, "sha"),
        (FILE_UPDATE, "sha"),
        (BRANCH_CREATE, "sha"),
        (BRANCH_DELETE, "sha"),
        (PROJECT_CREATE, "namespace"),
    ] {
        let declaration = declared(write, Effect::Write);
        assert!(
            declaration.input_schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!(pinned)),
            "{write}: {}",
            declaration.input_schema
        );
    }
    for (write, absent) in [
        (
            COMMIT_CREATE,
            &["start_branch", "start_sha", "start_project", "force"][..],
        ),
        (FILE_UPDATE, &["start_branch"][..]),
        (
            PROJECT_CREATE,
            &[
                "import_url",
                "template_name",
                "template_project_id",
                "mirror",
            ][..],
        ),
    ] {
        let declaration = declared(write, Effect::Write);
        let body = &declaration.input_schema["properties"]["body"];
        assert_eq!(body["additionalProperties"], false, "{write}: {body}");
        for key in absent {
            assert!(
                body["properties"].get(key).is_none(),
                "{write} admits {key}: {body}"
            );
        }
    }
    declared(ENVIRONMENTS_LIST, Effect::Read);
}

/// The pinned document declares the commit create, file update and project
/// create bodies only as `multipart/form-data`; the cited amendments make the
/// bundle declare `application/json`, the body GitLab's reference documents and
/// the engine sends, and the bundle's source record counts all four amendments.
#[test]
fn the_json_bodied_writes_declare_json_in_the_bundle() {
    let bundle = bundle();
    for operation_id in [
        "postApiV4ProjectsIdRepositoryCommits",
        "putApiV4ProjectsIdRepositoryFilesFilePath",
        "postApiV4Projects",
    ] {
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap();
        assert_eq!(
            operation.request_media_types,
            ["application/json"],
            "{operation_id}"
        );
    }
    let record = bundle.source.amendments.as_ref().expect("amendment record");
    assert_eq!(record.file_name, "openapi_v3.amendments.json");
    assert_eq!(record.count, 4);
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

/// Through the engine: `environments.list` sends one GET to the route the
/// pinned document declares, with its filters and one page, and returns
/// GitLab's body unchanged; a page size outside 1 to 100, or one that is not an
/// integer, is refused before a request.
#[tokio::test]
async fn environments_list_sends_its_route_and_returns_the_body_unchanged() {
    let engine = engine();
    let answer = json!([environment(1, "production", "available")]);
    let http = Reads {
        responses: Mutex::new(VecDeque::from([answer.clone()])),
        calls: Mutex::new(Vec::new()),
    };
    let output = engine
        .read(
            &http,
            "one",
            ENVIRONMENTS_LIST,
            json!({"id": "org/project", "states": "available", "search": "prod",
                   "per_page": 20, "page": 1}),
        )
        .await
        .unwrap();
    assert_eq!(output["status"], 200);
    assert_eq!(output["body"], answer);
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, ["projects", "org/project", "environments"]);
    let mut sent = calls[0].1.clone();
    sent.sort();
    assert_eq!(
        sent,
        [
            ("page".to_owned(), "1".to_owned()),
            ("per_page".to_owned(), "20".to_owned()),
            ("search".to_owned(), "prod".to_owned()),
            ("states".to_owned(), "available".to_owned()),
        ]
    );
    for per_page in [json!(0), json!(101), json!("x")] {
        let http = Reads {
            responses: Mutex::new(VecDeque::new()),
            calls: Mutex::new(Vec::new()),
        };
        let refusal = engine
            .read(
                &http,
                INSTANCE,
                ENVIRONMENTS_LIST,
                json!({"id": "org/project", "per_page": per_page}),
            )
            .await
            .err()
            .unwrap_or_else(|| panic!("per_page {per_page} admitted"));
        assert_eq!(refusal.code, connectors_core::ErrorCode::InvalidInput);
        assert!(http.calls.lock().unwrap().is_empty());
    }
}

/// Through the provider process: `environments.list` answers GitLab's body,
/// and a project GitLab does not find is `not_found`.
#[test]
fn environments_list_answers_through_the_provider_process() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let output = read(
        &mut child,
        ENVIRONMENTS_LIST,
        &json!({"id": "org/project", "per_page": 20}),
    )
    .unwrap();
    assert_eq!(
        output["body"],
        json!([
            environment(1, "production", "available"),
            environment(2, "review/feature", "stopped")
        ])
    );
    assert_eq!(
        read(&mut child, ENVIRONMENTS_LIST, &json!({"id": "org/missing"})),
        Err(Failure::ProviderNotFound)
    );
    assert_eq!(
        provider.requests(),
        [
            (
                "GET".to_owned(),
                format!("{PROJECT}/environments?per_page=20"),
                Value::Null
            ),
            (
                "GET".to_owned(),
                "/api/v4/projects/org%2Fmissing/environments?".to_owned(),
                Value::Null
            ),
        ]
    );
}

/// Approved, `pipeline.retry` and `pipeline.cancel` read the pipeline, send one
/// POST with no body and are applied when GitLab answers the same pipeline at
/// the pinned commit; an answer naming another commit leaves the effect
/// unknown.
#[test]
fn pipeline_retry_and_cancel_pin_the_pipeline_commit() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = write(&mut child, PIPELINE_RETRY, &pipeline_input(FAILED, HEAD)).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 201);
    assert_eq!(output["body"]["id"], FAILED);
    assert_eq!(output["body"]["status"], "running");
    let before = provider.requests().len();
    let result = write(&mut child, PIPELINE_CANCEL, &pipeline_input(RUNNING, HEAD)).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    assert_eq!(result.result.unwrap()["body"]["status"], "canceled");
    assert_eq!(
        provider.requests(),
        [
            get_pipeline(FAILED),
            request("POST", &format!("pipelines/{FAILED}/retry"), Value::Null),
            get_pipeline(RUNNING),
            request("POST", &format!("pipelines/{RUNNING}/cancel"), Value::Null),
        ]
    );
    assert_eq!(before, 2);
    for operation in [PIPELINE_RETRY, PIPELINE_CANCEL] {
        let before = provider.requests().len();
        let result = write(&mut child, operation, &pipeline_input(DRIFTING, HEAD)).unwrap();
        assert_eq!(result.effect, WriteEffect::Unknown, "{operation}");
        assert_eq!(result.result, Err(Failure::Protocol), "{operation}");
        assert_eq!(provider.requests().len(), before + 2, "{operation}");
    }
}

/// Approved, `pipeline.create` reads the commit `body.ref` names, sends one
/// POST with the variables and is applied when GitLab answers a pipeline for
/// that ref at the pinned commit; a ref moved between the read and the create
/// leaves the effect unknown.
#[test]
fn pipeline_create_pins_the_ref_commit() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let input = pipeline_create_input("main", HEAD);
    let result = write(&mut child, PIPELINE_CREATE, &input).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 201);
    assert_eq!(output["body"]["ref"], "main");
    assert_eq!(output["body"]["sha"], HEAD);
    assert_eq!(
        provider.requests(),
        [
            get_commit("main"),
            request("POST", "pipeline", input["body"].clone()),
        ]
    );
    let before = provider.requests().len();
    let result = write(
        &mut child,
        PIPELINE_CREATE,
        &pipeline_create_input(RACING, HEAD),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Unknown);
    assert_eq!(result.result, Err(Failure::Protocol));
    assert_eq!(provider.requests().len(), before + 2);
}

/// Approved, `commit.create` reads the branch, posts the actions as one JSON
/// document and is applied when the new commit's first parent is the pinned
/// head; a branch moved between the read and the commit leaves the effect
/// unknown, and the same input sent again is refused by the guard, the head
/// having moved.
#[test]
fn commit_create_lands_on_the_pinned_head_as_json() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let input = commit_create_input("feature", HEAD);
    let result = write(&mut child, COMMIT_CREATE, &input).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 201);
    assert_eq!(output["body"]["parent_ids"], json!([HEAD]));
    assert_eq!(
        provider.requests(),
        [
            get_branch("feature"),
            request("POST", "repository/commits", input["body"].clone()),
        ]
    );
    assert_eq!(
        provider.content_types(),
        [(
            "POST".to_owned(),
            format!("{PROJECT}/repository/commits?"),
            Some("application/json".to_owned())
        )]
    );
    let before = provider.requests().len();
    assert_eq!(
        write(&mut child, COMMIT_CREATE, &input).map(|result| result.effect),
        Err(Failure::Forbidden)
    );
    assert_eq!(provider.requests()[before..], [get_branch("feature")]);
    let before = provider.requests().len();
    let result = write(
        &mut child,
        COMMIT_CREATE,
        &commit_create_input(RACING, HEAD),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Unknown);
    assert_eq!(result.result, Err(Failure::Protocol));
    assert_eq!(provider.requests().len(), before + 2);
}

/// Approved, `file.update` reads the branch, sends one JSON PUT, then reads the
/// commit the branch names, and is applied only when that commit's first
/// parent is the pinned head: GitLab's answer names only the file and branch.
/// A branch moved between the first read and the write, or a read after the
/// write that answers 500, 403 or 410, leaves the effect unknown; a file GitLab
/// cannot update is its definite 400.
#[test]
fn file_update_proves_its_commit_on_the_pinned_head_by_a_read_after() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let input = file_update_input("docs/guide.md", "feature", HEAD);
    let result = write(&mut child, FILE_UPDATE, &input).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 200);
    assert_eq!(
        output["body"],
        json!({"file_path": "docs/guide.md", "branch": "feature"})
    );
    assert_eq!(
        provider.requests(),
        [
            get_branch("feature"),
            request(
                "PUT",
                "repository/files/docs%2Fguide.md",
                input["body"].clone()
            ),
            get_commit("feature"),
        ]
    );
    assert_eq!(
        provider.content_types(),
        [(
            "PUT".to_owned(),
            format!("{PROJECT}/repository/files/docs%2Fguide.md?"),
            Some("application/json".to_owned())
        )]
    );
    for name in [RACING, FLAKY, DENIED, GONE] {
        let before = provider.requests().len();
        let result = write(
            &mut child,
            FILE_UPDATE,
            &file_update_input("docs/guide.md", name, HEAD),
        )
        .unwrap();
        assert_eq!(result.effect, WriteEffect::Unknown, "{name}");
        assert_eq!(result.result, Err(Failure::Protocol), "{name}");
        assert_eq!(
            provider.requests()[before..],
            [
                get_branch(name),
                request(
                    "PUT",
                    "repository/files/docs%2Fguide.md",
                    file_update_input("docs/guide.md", name, HEAD)["body"].clone()
                ),
                get_commit(name),
            ],
            "{name}"
        );
    }
    let before = provider.requests().len();
    let result = write(
        &mut child,
        FILE_UPDATE,
        &file_update_input("missing.txt", "main", HEAD),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Refused);
    assert_eq!(provider.requests().len(), before + 2);
}

/// Approved, `branch.create` reads the commit `body.ref` names, sends one POST
/// and is applied when GitLab answers the branch at the pinned commit; a ref
/// moved between the read and the create leaves the effect unknown, and a
/// branch that already exists is GitLab's definite 400.
#[test]
fn branch_create_pins_the_ref_commit_and_proves_the_branch_at_it() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let result = write(
        &mut child,
        BRANCH_CREATE,
        &branch_create_input("topic", "main", HEAD),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 201);
    assert_eq!(output["body"]["name"], "topic");
    assert_eq!(output["body"]["commit"]["id"], HEAD);
    assert_eq!(
        provider.requests(),
        [
            get_commit("main"),
            request(
                "POST",
                "repository/branches",
                json!({"branch": "topic", "ref": "main"})
            ),
        ]
    );
    let before = provider.requests().len();
    let result = write(
        &mut child,
        BRANCH_CREATE,
        &branch_create_input("topic-2", RACING, HEAD),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Unknown);
    assert_eq!(result.result, Err(Failure::Protocol));
    assert_eq!(provider.requests().len(), before + 2);
    let before = provider.requests().len();
    let result = write(
        &mut child,
        BRANCH_CREATE,
        &branch_create_input("feature", "main", HEAD),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Refused);
    assert_eq!(provider.requests().len(), before + 2);
}

/// Approved, `branch.delete` reads the branch, sends one DELETE, reads the
/// branch again and is applied only when that read answers 404. A branch still
/// found after the delete, or a read after it that answers anything but 404
/// (500, 403, 410), leaves the effect unknown.
#[test]
fn branch_delete_proves_the_branch_before_and_its_absence_after() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let delete = |name: &str| {
        request(
            "DELETE",
            &format!("repository/branches/{name}"),
            Value::Null,
        )
    };
    let result = write(
        &mut child,
        BRANCH_DELETE,
        &branch_delete_input("feature", HEAD),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 204);
    assert_eq!(output["body"], Value::Null);
    assert_eq!(
        provider.requests(),
        [
            get_branch("feature"),
            delete("feature"),
            get_branch("feature")
        ]
    );
    for name in [STICKY, FLAKY, DENIED, GONE] {
        let before = provider.requests().len();
        let result = write(&mut child, BRANCH_DELETE, &branch_delete_input(name, HEAD)).unwrap();
        assert_eq!(result.effect, WriteEffect::Unknown, "{name}");
        assert_eq!(result.result, Err(Failure::Protocol), "{name}");
        assert_eq!(
            provider.requests()[before..],
            [get_branch(name), delete(name), get_branch(name)],
            "{name}"
        );
    }
}

/// Approved, `project.create` reads the namespace by its path, refuses unless
/// it has the given id, posts one JSON body and is applied when GitLab answers
/// the project at that path in that namespace. An answer placing it in another
/// namespace leaves the effect unknown; a path already taken is GitLab's
/// definite 400.
#[test]
fn project_create_resolves_the_namespace_by_path_and_proves_the_project_in_it() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let input = project_create_input("org/team", TEAM, "service");
    let result = write(&mut child, PROJECT_CREATE, &input).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied, "{:?}", result.result);
    let output = result.result.unwrap();
    assert_eq!(output["status"], 201);
    assert_eq!(output["body"]["path_with_namespace"], "org/team/service");
    assert_eq!(
        provider.requests(),
        [
            get_namespace("org/team"),
            (
                "POST".to_owned(),
                "/api/v4/projects?".to_owned(),
                input["body"].clone()
            ),
        ]
    );
    assert_eq!(
        provider.content_types(),
        [(
            "POST".to_owned(),
            "/api/v4/projects?".to_owned(),
            Some("application/json".to_owned())
        )]
    );
    let before = provider.requests().len();
    let result = write(
        &mut child,
        PROJECT_CREATE,
        &project_create_input("org/team", TEAM, RACING),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Unknown);
    assert_eq!(result.result, Err(Failure::Protocol));
    assert_eq!(provider.requests().len(), before + 2);
    let before = provider.requests().len();
    let result = write(
        &mut child,
        PROJECT_CREATE,
        &project_create_input("org/team", TEAM, "existing"),
    )
    .unwrap();
    assert_eq!(result.effect, WriteEffect::Refused);
    assert_eq!(provider.requests().len(), before + 2);
}

/// Each guard refuses before its write, sending only its read: a pipeline for
/// another commit or one GitLab does not find; a ref at another commit or one
/// it does not find; a branch whose head is another commit or one it does not
/// find; a branch to delete at another commit or absent; a namespace path whose
/// id is not the given one, given as its id rather than its path, or absent.
#[test]
fn each_preflight_mismatch_is_refused_before_the_write() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, read) in [
        (
            PIPELINE_RETRY,
            pipeline_input(ELSEWHERE, HEAD),
            get_pipeline(ELSEWHERE),
        ),
        (PIPELINE_RETRY, pipeline_input(99, HEAD), get_pipeline(99)),
        (
            PIPELINE_CANCEL,
            pipeline_input(RUNNING, OTHER),
            get_pipeline(RUNNING),
        ),
        (PIPELINE_CANCEL, pipeline_input(99, HEAD), get_pipeline(99)),
        (
            PIPELINE_CREATE,
            pipeline_create_input("main", OTHER),
            get_commit("main"),
        ),
        (
            PIPELINE_CREATE,
            pipeline_create_input("nowhere", HEAD),
            get_commit("nowhere"),
        ),
        (
            COMMIT_CREATE,
            commit_create_input("main", OTHER),
            get_branch("main"),
        ),
        (
            COMMIT_CREATE,
            commit_create_input("nowhere", HEAD),
            get_branch("nowhere"),
        ),
        (
            FILE_UPDATE,
            file_update_input("README.md", "main", OTHER),
            get_branch("main"),
        ),
        (
            FILE_UPDATE,
            file_update_input("README.md", "nowhere", HEAD),
            get_branch("nowhere"),
        ),
        (
            BRANCH_CREATE,
            branch_create_input("topic", "main", OTHER),
            get_commit("main"),
        ),
        (
            BRANCH_CREATE,
            branch_create_input("topic", "nowhere", HEAD),
            get_commit("nowhere"),
        ),
        (
            BRANCH_DELETE,
            branch_delete_input("main", OTHER),
            get_branch("main"),
        ),
        (
            BRANCH_DELETE,
            branch_delete_input("nowhere", HEAD),
            get_branch("nowhere"),
        ),
        (
            PROJECT_CREATE,
            project_create_input("org", TEAM, "service"),
            get_namespace("org"),
        ),
        (
            PROJECT_CREATE,
            project_create_input("43", TEAM, "service"),
            get_namespace("43"),
        ),
        (
            PROJECT_CREATE,
            project_create_input("elsewhere", TEAM, "service"),
            get_namespace("elsewhere"),
        ),
    ] {
        let before = provider.requests().len();
        let outcome = write(&mut child, operation, &input).map(|result| result.effect);
        assert_eq!(outcome, Err(Failure::Forbidden), "{operation} {input}");
        assert_eq!(provider.requests()[before..], [read], "{operation} {input}");
    }
}

/// A body outside the closed keys, a key of the wrong type, or an input
/// without what a guard pins is refused before any request: `start_branch`,
/// `start_sha`, `start_project` or `force` on a commit, which would create a
/// branch or rewrite history; `start_branch` on a file update; `import_url` or
/// a template on a project create; an unknown key on the pipeline and branch
/// creates; a string `namespace_id`; a missing `sha` or `namespace`; a commit
/// without actions; a file update without content.
#[test]
fn a_body_outside_the_closed_keys_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut cases = Vec::new();
    for (key, value) in [
        ("start_branch", json!("main")),
        ("start_sha", json!(HEAD)),
        ("start_project", json!("org/other")),
        ("force", json!(true)),
    ] {
        let mut input = commit_create_input("main", HEAD);
        input["body"][key] = value;
        cases.push((COMMIT_CREATE, input));
    }
    let mut input = commit_create_input("main", HEAD);
    input["body"].as_object_mut().unwrap().remove("actions");
    cases.push((COMMIT_CREATE, input));
    let mut input = file_update_input("README.md", "main", HEAD);
    input["body"]["start_branch"] = json!("main");
    cases.push((FILE_UPDATE, input));
    let mut input = file_update_input("README.md", "main", HEAD);
    input["body"].as_object_mut().unwrap().remove("content");
    cases.push((FILE_UPDATE, input));
    for (key, value) in [
        ("import_url", json!("https://example.invalid/repo.git")),
        ("template_name", json!("rails")),
        ("mirror", json!(true)),
    ] {
        let mut input = project_create_input("org/team", TEAM, "service");
        input["body"][key] = value;
        cases.push((PROJECT_CREATE, input));
    }
    let mut input = project_create_input("org/team", TEAM, "service");
    input["body"]["namespace_id"] = json!("43");
    cases.push((PROJECT_CREATE, input));
    let mut input = project_create_input("org/team", TEAM, "service");
    input.as_object_mut().unwrap().remove("namespace");
    cases.push((PROJECT_CREATE, input));
    let mut input = pipeline_create_input("main", HEAD);
    input["body"]["source"] = json!("web");
    cases.push((PIPELINE_CREATE, input));
    let mut input = branch_create_input("topic", "main", HEAD);
    input["body"]["protected"] = json!(true);
    cases.push((BRANCH_CREATE, input));
    for (operation, mut input) in [
        (PIPELINE_RETRY, pipeline_input(FAILED, HEAD)),
        (PIPELINE_CANCEL, pipeline_input(RUNNING, HEAD)),
        (PIPELINE_CREATE, pipeline_create_input("main", HEAD)),
        (COMMIT_CREATE, commit_create_input("main", HEAD)),
        (FILE_UPDATE, file_update_input("README.md", "main", HEAD)),
        (BRANCH_CREATE, branch_create_input("topic", "main", HEAD)),
        (BRANCH_DELETE, branch_delete_input("feature", HEAD)),
    ] {
        input.as_object_mut().unwrap().remove("sha");
        cases.push((operation, input));
    }
    for (operation, input) in cases {
        let outcome = write(&mut child, operation, &input).map(|result| result.effect);
        assert_eq!(outcome, Err(Failure::InvalidInput), "{operation} {input}");
    }
    assert!(provider.requests().is_empty(), "{:?}", provider.requests());
}

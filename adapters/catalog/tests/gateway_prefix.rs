//! An `api_base` behind an API gateway: `request_prefix` names the leading part
//! of the `api_base` path that the gateway adds in front of the pinned
//! document's paths, such as `/ex/jira/<cloud id>`. The document's base-path
//! check applies to what follows the prefix, every request (the identity read
//! included) goes to the full `api_base`, and a configuration without the
//! prefix loads to the same bootstrap as before the field existed.
//!
//! Each gateway case runs the configuration example the product guide shows,
//! against a disposable HTTPS fixture. No live credential and no network.
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
    process::{Command, Output},
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

/// The gateway origin the guides write; the fixture replaces it.
const GATEWAY: &str = "https://api.atlassian.com";
/// The placeholder cloud id the guides write.
const CLOUD: &str = "your-cloud-id";
const TOKEN: &str = "fixture-gateway-token";
const HEADER: &str = "Bearer fixture-gateway-token";

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

/// Every JSON block of a guide that parses as a whole configuration for
/// `provider`, in document order.
fn documented(guide: &str, provider: &str) -> Vec<Value> {
    let guide = fs::read_to_string(root().join("../../docs").join(guide)).unwrap();
    guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .filter_map(|body| serde_json::from_str::<Value>(body).ok())
        .filter(|config| config["provider"] == provider)
        .collect()
}
/// The site form: the guide's first configuration for `provider`.
fn site_form(guide: &str, provider: &str) -> Value {
    let config = documented(guide, provider)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("{guide} documents no {provider} configuration"));
    assert!(config.get("request_prefix").is_none());
    config
}
/// The bearer (OAuth) gateway form: the guide's configuration that states a
/// `request_prefix` and a bearer token. The basic gateway form a
/// service-account token uses is driven by `gateway_prefix_adversary.rs`.
fn gateway_form(guide: &str, provider: &str) -> Value {
    documented(guide, provider)
        .into_iter()
        .find(|config| config.get("request_prefix").is_some() && config["auth"]["bearer"] == true)
        .unwrap_or_else(|| panic!("{guide} documents no gateway form for {provider}"))
}

/// A configuration made loadable here: the repository's bundles and selection
/// set, and no CA file unless the caller adds one.
fn localized(mut config: Value, provider: &str) -> Value {
    config["bundle_directory"] = json!(root_path("generated/bundles"));
    config["operations_file"] = json!(root_path(&format!("providers/{provider}/operations.json")));
    config.as_object_mut().unwrap().remove("ca_file");
    config
}

fn print_bootstrap(config: &Value) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let private_directory = directory.path().join("private");
    filesystem::directory(&private_directory, true, true).unwrap();
    let path = private_directory.join("catalog.json");
    private(&path, &serde_json::to_vec(config).unwrap());
    Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(&path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap()
}
fn refusal(config: &Value) -> String {
    let output = print_bootstrap(config);
    assert!(
        !output.status.success(),
        "loaded: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).trim().to_owned()
}

/// The bootstrap each guide's site-form example printed before `request_prefix`
/// existed (base `94d7d25e3`), as the SHA-256 of `--print-local-bootstrap`'s
/// stdout, with its configuration revision. The GitLab example is loaded
/// without its `ca_file`, whose digest would depend on a generated CA. A
/// rebuilt bundle changes these, and `bundle_drift.rs` pins the bundles. They
/// were re-pinned when the bundles were rebuilt to record repeated query
/// parameters, and again when a repeated parameter's declared schema was typed
/// like its elements. The GitLab example loads the shipped selection set, so
/// its pair was re-pinned again when that set gained `commits.list` and
/// `repository.compare`, again when it gained `deployments.list`, again
/// when `commits.list` withheld `pagination` and `page_token`, again when it
/// gained `issue.create`, and again when it declared the merge request feed
/// (`feed.containers`, `feed.items`), which the configuration revision carries,
/// again when it gained the merge-request note, discussion read, reply and
/// resolve, and again when those writes typed and required their body keys
/// and the resolve guard compared the discussion id, and again when it gained
/// `repository.tree`, `commit.get`, `commit.diff` and `branches.list`, and
/// again when it gained the fixed-body variants `merge_request.auto_merge` and
/// `merge_request.reopen`, and again when it gained the merge-request list
/// reads `merge_request.diffs` and `merge_request.discussions`, and again when
/// it gained `search.blobs`, whose cited path correction also rebuilt the
/// GitLab bundle (its digest is in the revision;
/// `gitlab_commit_reads_adversary.rs` reconstructs the bundle before it),
/// and again when it gained `tag.get`, `tag.create`, `tag.delete`,
/// `release.get`, `release.links`, `release.create` and `release.update`,
/// and again when it gained `pipeline.retry`, `pipeline.cancel`,
/// `pipeline.create`, `environments.list`, `commit.create`, `file.update`,
/// `branch.create`, `branch.delete` and `project.create`, whose cited media
/// type corrections also rebuilt the GitLab bundle, and again when
/// `file.update` proved its write by reading the branch, not the revision
/// `commits/{branch}` that a same-named tag wins.
/// The Jira example loads its shipped selection set too, so its pair was
/// re-pinned when that set gained `issue.get`, `issue.create_meta` and
/// `users.search`, again when it gained `issue.transitions`, and again when it
/// gained the guarded write `issue.transition.run`, and again when it gained
/// the binary read `attachment.content`.
const UNPREFIXED: [(&str, &str, &str, &str); 3] = [
    (
        "local-catalog-provider.md",
        "gitlab",
        "f1e56d54ab3acdbedb01b9683bc7ea002e92e20714ed89168a3b782a838e720d",
        "222bce75c9f8307b93515556b95694e198ef9e0929005c5357443c63ec2438d4",
    ),
    (
        "catalog-jira.md",
        "jira",
        "0f1fb08e6a17ed2d4cf8f60cd1b322378d831a78ccaf273b0a019402f1922fff",
        "d7a86083ea26c4f5d94e392d3310ae45cd8289167d69e38f6b8a8ece0c0c3348",
    ),
    // The Confluence guide declares an `access` probe since #102, which is part of
    // its configuration and so of its revision.
    (
        "catalog-confluence.md",
        "confluence",
        "b6620549f33372b55a07ec1e1abc582583e2aff7f14a9434e73fcc4272a2cc0a",
        "86a936572eaa670f31dfb80ea503d568c7e3732384bbe46207047796a5599e28",
    ),
];

#[test]
fn a_configuration_without_a_prefix_prints_the_bootstrap_it_printed_before() {
    let mut observed = Vec::new();
    for (guide, provider, _, _) in UNPREFIXED {
        let output = print_bootstrap(&localized(site_form(guide, provider), provider));
        assert!(
            output.status.success(),
            "{guide}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bootstrap: Value = serde_json::from_slice(&output.stdout).unwrap();
        observed.push((
            guide,
            hex::encode(Sha256::digest(&output.stdout)),
            bootstrap["configuration_revision"]
                .as_str()
                .unwrap()
                .to_owned(),
        ));
    }
    for ((guide, _, digest, revision), (_, seen_digest, seen_revision)) in
        UNPREFIXED.iter().zip(&observed)
    {
        assert_eq!(
            (seen_digest.as_str(), seen_revision.as_str()),
            (*digest, *revision),
            "{guide}; observed {observed:?}"
        );
    }
}

/// Route and `Authorization` header of each fixture request.
type Requests = Arc<Mutex<Vec<(String, Option<String>)>>>;

/// A TLS fixture that answers every request with the same JSON body, so the
/// identity read and each product read succeed, and records what it was sent.
struct Fixture {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    root: tempfile::TempDir,
    ca: PathBuf,
    port: u16,
    requests: Requests,
}
impl Fixture {
    fn new() -> Self {
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
                    let authorization = request.lines().find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                            .map(|(_, value)| value.trim().to_owned())
                    });
                    let (status, body) = if authorization.as_deref() == Some(HEADER) {
                        (200, json!({"accountId": "fixture-account-id"}))
                    } else {
                        (401, json!({"errorMessages": ["fixture refusal"]}))
                    };
                    observed.lock().unwrap().push((path, authorization));
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
        Self {
            stop: Some(stop),
            thread: Some(thread),
            root,
            ca,
            port: address.port(),
            requests,
        }
    }
    /// The guide's gateway configuration with the gateway origin replaced by
    /// this fixture's.
    fn configure(&self, guide: &str, provider: &str) -> Value {
        let mut config = localized(gateway_form(guide, provider), provider);
        let api_base = config["api_base"].as_str().unwrap().to_owned();
        let rest = api_base
            .strip_prefix(GATEWAY)
            .unwrap_or_else(|| panic!("{guide}: the gateway form is not on {GATEWAY}"));
        config["api_base"] = json!(format!("https://localhost:{}{rest}", self.port));
        config["ca_file"] = json!(self.ca);
        config
    }
    fn adapter(&self, config: &Value) -> Adapter {
        let path = self.root.path().join("private/catalog.json");
        private(&path, &serde_json::to_vec(config).unwrap());
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(&path)
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
        assert_eq!(
            bootstrap.provider_authority,
            format!("{}/", config["api_base"].as_str().unwrap()),
            "the authority is the full gateway base"
        );
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: None,
            permissions: Default::default(),
            instance_id: bootstrap.instance.clone(),
            adapter_id: "catalog".into(),
            configuration_revision: bootstrap.configuration_revision,
            protocol: "v1alpha1".into(),
            startup: Startup::OnDemand,
            restart: Restart::Never,
            executable: Executable {
                sha256: hex::encode(Sha256::digest(fs::read(&binary).unwrap())),
                path: binary,
                args: vec!["--local-config".into(), path.to_str().unwrap().into()],
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

fn secret() -> Secret {
    Secret(serde_json::to_vec(&json!({"token": TOKEN})).unwrap())
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}

/// Validate the documented profile and run one read through the gateway form;
/// return the paths the fixture saw, each with the bearer header asserted.
fn through_gateway(guide: &str, provider: &str, operation: &str, input: Value) -> Vec<String> {
    let fixture = Fixture::new();
    let config = fixture.configure(guide, provider);
    let profile = config["auth"]["profile"].as_str().unwrap().to_owned();
    let mut child = Child::spawn(&fixture.adapter(&config)).unwrap();
    let baseline = child
        .validate(&profile, &secret(), deadline())
        .unwrap_or_else(|failure| {
            panic!(
                "{guide}: validation {failure:?}; saw {:?}",
                fixture.requests()
            )
        });
    assert_eq!(baseline.identity.subject, "fixture-account-id");
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let output = child
        .invoke(
            operation,
            &revision,
            "one",
            &secret(),
            &serde_json::to_vec(&input).unwrap(),
            deadline(),
        )
        .unwrap_or_else(|failure| panic!("{guide}: `{operation}` {failure:?}"));
    let result: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result["status"], 200, "{guide}: `{operation}`");
    let requests = fixture.requests();
    for (path, authorization) in &requests {
        assert_eq!(authorization.as_deref(), Some(HEADER), "{guide}: {path}");
    }
    requests.into_iter().map(|(path, _)| path).collect()
}

#[test]
fn a_jira_read_and_its_identity_read_go_through_the_gateway_prefix() {
    let paths = through_gateway(
        "catalog-jira.md",
        "jira",
        "issue.comments",
        json!({"issueIdOrKey": "FIX-1", "startAt": 0, "maxResults": 2}),
    );
    assert_eq!(
        paths,
        [
            // The host transport ends a query-less GET with an empty `?`.
            format!("/ex/jira/{CLOUD}/rest/api/3/myself?"),
            format!("/ex/jira/{CLOUD}/rest/api/3/issue/FIX-1/comment?startAt=0&maxResults=2"),
        ]
    );
}

#[test]
fn a_confluence_read_and_its_identity_read_go_through_the_gateway_prefix() {
    let paths = through_gateway(
        "catalog-confluence.md",
        "confluence",
        "page.get",
        json!({"id": "1001", "body-format": "storage"}),
    );
    assert_eq!(
        paths,
        [
            // The host transport ends a query-less GET with an empty `?`.
            format!("/ex/confluence/{CLOUD}/wiki/rest/api/user/current?"),
            format!("/ex/confluence/{CLOUD}/wiki/api/v2/pages/1001?body-format=storage"),
        ]
    );
}

/// The documented gateway form with `api_base` and `request_prefix` replaced.
fn jira_gateway(api_base: &str, prefix: Option<&str>) -> Value {
    let mut config = localized(gateway_form("catalog-jira.md", "jira"), "jira");
    config["api_base"] = json!(api_base);
    match prefix {
        Some(prefix) => config["request_prefix"] = json!(prefix),
        None => {
            config.as_object_mut().unwrap().remove("request_prefix");
        }
    }
    config
}

#[test]
fn the_documented_gateway_forms_load_and_differ_from_the_site_form() {
    for (guide, provider) in [
        ("catalog-jira.md", "jira"),
        ("catalog-confluence.md", "confluence"),
    ] {
        let gateway = localized(gateway_form(guide, provider), provider);
        let output = print_bootstrap(&gateway);
        assert!(
            output.status.success(),
            "{guide}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bootstrap: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            bootstrap["provider_authority"],
            format!("{}/", gateway["api_base"].as_str().unwrap())
        );
        let site = print_bootstrap(&localized(site_form(guide, provider), provider));
        let site: Value = serde_json::from_slice(&site.stdout).unwrap();
        assert_ne!(
            bootstrap["configuration_revision"],
            site["configuration_revision"]
        );
    }
}

#[test]
fn without_a_prefix_the_gateway_base_is_still_outside_the_document_base() {
    let base = format!("{GATEWAY}/ex/jira/{CLOUD}/rest/api/3");
    assert!(
        print_bootstrap(&jira_gateway(&base, Some(&format!("/ex/jira/{CLOUD}"))))
            .status
            .success()
    );
    refusal(&jira_gateway(&base, None));
}

#[test]
fn a_document_path_outside_the_base_after_the_prefix_is_still_refused() {
    // After `/ex/jira/<id>` the base is `/rest/api/2`; every selected Jira
    // operation sits under `/rest/api/3`.
    let base = format!("{GATEWAY}/ex/jira/{CLOUD}/rest/api/2");
    refusal(&jira_gateway(&base, Some(&format!("/ex/jira/{CLOUD}"))));
}

#[test]
fn a_prefix_that_is_not_plain_leading_path_text_is_invalid_configuration() {
    let base = format!("{GATEWAY}/ex/jira/{CLOUD}/rest/api/3");
    for prefix in [
        "",
        "/",
        "ex/jira/your-cloud-id",
        "/ex/jira/your-cloud-id/",
        "/ex//jira/your-cloud-id",
        "/ex/jira/../jira/your-cloud-id",
        "/ex/./jira/your-cloud-id",
        "/ex/jira/your-cloud-id?x=1",
        "/ex/jira/your-cloud-id#top",
        "/ex%2Fjira/your-cloud-id",
        "/ex/jira/your%2dcloud-id",
        "/ex/jira/your cloud-id",
        "/ex\\jira/your-cloud-id",
        // Plain text, but not the leading part of the `api_base` path.
        "/ex/jira/another-cloud-id",
        "/ex/confluence/your-cloud-id",
        "/jira/your-cloud-id",
        "/ex/jira/your-cloud-id/rest/api/3/issue",
    ] {
        assert_eq!(
            refusal(&jira_gateway(&base, Some(prefix))),
            "InvalidConfiguration",
            "prefix {prefix:?}"
        );
    }
}

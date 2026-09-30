//! Adversary cases for `request_prefix` (story catalog-api-base-gateway-prefix).
//!
//! The coordinator connected Jira live through the Atlassian gateway with a
//! service-account API token over HTTP basic (`atlassian.basic`), and a search
//! returned issues. The guides show only a bearer gateway form and say a
//! service-account token is sent as `Bearer`. These cases drive the guides and
//! the implementation against that, plus the revision and parsing edges.
use base64::Engine as _;
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

const GATEWAY: &str = "https://api.atlassian.com";
const ACCOUNT: &str = "robot@serviceaccount.atlassian.com";
const TOKEN: &str = "fixture-service-account-token";

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
fn loaded(config: &Value) -> Value {
    let output = print_bootstrap(config);
    assert!(
        output.status.success(),
        "refused: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn refused(config: &Value) -> bool {
    !print_bootstrap(config).status.success()
}

type Requests = Arc<Mutex<Vec<(String, Option<String>)>>>;

/// A TLS fixture that answers every request 200 with an `accountId` and records
/// each request's route and `Authorization` header.
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
                    observed.lock().unwrap().push((path, authorization));
                    let body = serde_json::to_vec(&json!({"accountId": "fixture-account-id"}))
                        .unwrap();
                    let header = format!(
                        "HTTP/1.1 200 fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
    fn origin(&self) -> String {
        format!("https://localhost:{}", self.port)
    }
    fn adapter(&self, config: &Value) -> Adapter {
        let path = self.root.path().join("private/catalog.json");
        private(&path, &serde_json::to_vec(config).unwrap());
        let bootstrap: Bootstrap = serde_json::from_value(loaded(config)).unwrap();
        bootstrap.validate().unwrap();
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

fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}

/// Validate `config`'s profile with `secret` and run one read; return every
/// request the fixture saw.
fn run(
    fixture: &Fixture,
    config: &Value,
    secret: &Value,
    operation: &str,
    input: Value,
) -> Vec<(String, Option<String>)> {
    let profile = config["auth"]["profile"].as_str().unwrap().to_owned();
    let secret = Secret(serde_json::to_vec(secret).unwrap());
    let mut child = Child::spawn(&fixture.adapter(config)).unwrap();
    child
        .validate(&profile, &secret, deadline())
        .unwrap_or_else(|failure| panic!("validation {failure:?}; saw {:?}", fixture.requests()));
    let revision = child.bootstrap().descriptor().unwrap().revision;
    let output = child
        .invoke(
            operation,
            &revision,
            "one",
            &secret,
            &serde_json::to_vec(&input).unwrap(),
            deadline(),
        )
        .unwrap_or_else(|failure| panic!("`{operation}` {failure:?}"));
    let result: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(result["status"], 200);
    fixture.requests()
}

/// The live path the coordinator verified (service-account API token, basic,
/// through the gateway) must be a documented configuration, not only a bearer
/// form that the guide itself says was never checked. Each guide must show a
/// gateway configuration with `scheme: basic`, and that configuration must send
/// `Basic base64(account:token)` to the prefixed paths.
#[test]
fn each_guide_documents_the_basic_gateway_form_a_service_account_token_uses() {
    let basic = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("{ACCOUNT}:{TOKEN}"))
    );
    for (guide, provider, operation, input, expected) in [
        (
            "catalog-jira.md",
            "jira",
            "issue.comments",
            json!({"issueIdOrKey": "FIX-1", "startAt": 0, "maxResults": 2}),
            [
                "/ex/jira/your-cloud-id/rest/api/3/myself?",
                "/ex/jira/your-cloud-id/rest/api/3/issue/FIX-1/comment?startAt=0&maxResults=2",
            ],
        ),
        (
            "catalog-confluence.md",
            "confluence",
            "page.get",
            json!({"id": "1001", "body-format": "storage"}),
            [
                "/ex/confluence/your-cloud-id/wiki/rest/api/user/current?",
                "/ex/confluence/your-cloud-id/wiki/api/v2/pages/1001?body-format=storage",
            ],
        ),
    ] {
        let documented = documented(guide, provider)
            .into_iter()
            .find(|config| {
                config.get("request_prefix").is_some() && config["auth"]["scheme"] == "basic"
            })
            .unwrap_or_else(|| {
                panic!(
                    "{guide} shows no gateway configuration with `scheme: basic`; its only \
                     gateway form is {:?}, and a service-account API token was verified live \
                     over basic",
                    documented(guide, provider)
                        .into_iter()
                        .filter(|c| c.get("request_prefix").is_some())
                        .map(|c| c["auth"]["profile"].clone())
                        .collect::<Vec<_>>()
                )
            });
        let fixture = Fixture::new();
        let mut config = localized(documented, provider);
        let rest = config["api_base"]
            .as_str()
            .unwrap()
            .strip_prefix(GATEWAY)
            .unwrap()
            .to_owned();
        config["api_base"] = json!(format!("{}{rest}", fixture.origin()));
        config["ca_file"] = json!(fixture.ca);
        let seen = run(
            &fixture,
            &config,
            &json!({"account": ACCOUNT, "token": TOKEN}),
            operation,
            input,
        );
        assert_eq!(
            seen,
            expected
                .iter()
                .map(|path| (path.to_string(), Some(basic.clone())))
                .collect::<Vec<_>>(),
            "{guide}"
        );
    }
}

/// The prefix changes where requests go even when `api_base` and the
/// selections are unchanged, so it must be part of the configuration revision.
/// The shipped test compares a gateway form with a site form, which differ in
/// `api_base` already, so it stays green if the prefix is dropped from the
/// revision input. Here only the prefix differs.
#[test]
fn two_valid_prefixes_on_one_api_base_give_two_revisions() {
    let site = documented("catalog-jira.md", "jira")
        .into_iter()
        .next()
        .unwrap();
    let mut config = localized(site, "jira");
    config["api_base"] = json!("https://gateway.example/rest/api/3/rest/api/3");
    config["request_prefix"] = json!("/rest/api/3");
    let shorter = loaded(&config);
    config["request_prefix"] = json!("/rest/api/3/rest/api/3");
    let whole = loaded(&config);
    assert_eq!(shorter["provider_authority"], whole["provider_authority"]);
    assert_ne!(
        shorter["configuration_revision"], whole["configuration_revision"],
        "two prefixes that send the same selection to different paths share a revision"
    );
}

/// The acceptance statement's own form: `api_base` is the gateway root and the
/// document's paths are appended in full. The identity read then names the
/// document path itself.
#[test]
fn the_acceptance_form_with_the_gateway_root_as_api_base_sends_the_full_document_paths() {
    let fixture = Fixture::new();
    let site = documented("catalog-jira.md", "jira")
        .into_iter()
        .next()
        .unwrap();
    let mut config = localized(site, "jira");
    config["api_base"] = json!(format!("{}/ex/jira/cloud-1", fixture.origin()));
    config["request_prefix"] = json!("/ex/jira/cloud-1");
    config["auth"]["identity"]["path"] = json!("rest/api/3/myself");
    config["ca_file"] = json!(fixture.ca);
    let seen = run(
        &fixture,
        &config,
        &json!({"account": ACCOUNT, "token": TOKEN}),
        "issue.changelog",
        json!({"issueIdOrKey": "FIX-2", "startAt": 0, "maxResults": 1}),
    );
    let paths = seen.into_iter().map(|(path, _)| path).collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            "/ex/jira/cloud-1/rest/api/3/myself?",
            "/ex/jira/cloud-1/rest/api/3/issue/FIX-2/changelog?startAt=0&maxResults=1",
        ]
    );
}

/// Parsing edges the shipped list does not cover.
#[test]
fn prefix_parsing_edges() {
    let site = documented("catalog-jira.md", "jira")
        .into_iter()
        .next()
        .unwrap();
    let with = |api_base: &str, prefix: &str| {
        let mut config = localized(site.clone(), "jira");
        config["api_base"] = json!(api_base);
        config["request_prefix"] = json!(prefix);
        config
    };
    // A trailing slash on api_base is the same base.
    let plain = loaded(&with(
        "https://g.example/ex/jira/c/rest/api/3",
        "/ex/jira/c",
    ));
    let slashed = loaded(&with(
        "https://g.example/ex/jira/c/rest/api/3/",
        "/ex/jira/c",
    ));
    assert_eq!(
        plain["configuration_revision"],
        slashed["configuration_revision"]
    );
    // Case differs from the api_base path: refused, not folded.
    assert!(refused(&with(
        "https://g.example/ex/jira/c/rest/api/3",
        "/EX/jira/c"
    )));
    assert!(refused(&with(
        "https://g.example/ex/JIRA/c/rest/api/3",
        "/ex/jira/c"
    )));
    // Unicode in the prefix, and in the api_base path (percent-encoded there).
    assert!(refused(&with(
        "https://g.example/ex/jira/c/rest/api/3",
        "/ex/jira/ç"
    )));
    assert!(refused(&with(
        "https://g.example/ex/jira/ç/rest/api/3",
        "/ex/jira/ç"
    )));
    assert!(refused(&with(
        "https://g.example/ex/jira/%C3%A7/rest/api/3",
        "/ex/jira/%C3%A7"
    )));
    // An encoded character in the api_base path that decodes to the prefix.
    assert!(refused(&with(
        "https://g.example/ex/jira/c%2D1/rest/api/3",
        "/ex/jira/c-1"
    )));
    // An api_base path with an empty segment.
    assert!(refused(&with(
        "https://g.example/ex//jira/c/rest/api/3",
        "/ex/jira/c"
    )));
    // Dot segments in api_base are normalised before the prefix is matched,
    // so the prefix is matched against where requests really go.
    let dotted = loaded(&with(
        "https://g.example/ex/jira/x/../c/rest/api/3",
        "/ex/jira/c",
    ));
    assert_eq!(
        dotted["provider_authority"],
        "https://g.example/ex/jira/c/rest/api/3/"
    );
    assert!(refused(&with(
        "https://g.example/ex/jira/x/../c/rest/api/3",
        "/ex/jira/x"
    )));
    // A prefix that is a segment's text prefix but not a whole segment.
    assert!(refused(&with(
        "https://g.example/ex/jira/c/rest/api/3",
        "/ex/jir"
    )));
    // Over 1024 bytes of otherwise valid segments.
    let long = format!("/{}", "a".repeat(1024));
    assert!(refused(&with(
        &format!("https://g.example{long}/rest/api/3"),
        &long
    )));
    // A `null` prefix is no prefix: same revision as omitting it.
    let mut null = localized(site.clone(), "jira");
    null["request_prefix"] = Value::Null;
    assert_eq!(
        loaded(&null)["configuration_revision"],
        loaded(&localized(site.clone(), "jira"))["configuration_revision"]
    );
}

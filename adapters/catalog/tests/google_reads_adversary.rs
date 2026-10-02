//! Adversary cases against the Google Drive and Google Slides read selections.
//! Each case asserts a claim the story, the guide or an existing test makes,
//! against the code that is supposed to keep it. Fixture secrets are
//! fictional and only ever compared, never printed.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure},
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

const BASE: &str = "/drive/v3";
const PROVIDER: &str = "google-drive";
const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive.readonly";
const CLIENT_ID: &str = "fixture-client-id.apps.example.test";
const CLIENT_SECRET: &str = "fixture-client-secret-adversary";
const REFRESH_TOKEN: &str = "fixture-refresh-token-adversary";
const ACCESS_TOKEN: &str = "fixture-access-token-adversary";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped(provider: &str) -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join(format!("providers/{provider}/operations.json"))).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn committed_bundle(provider: &str) -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), provider).unwrap()
}

/// `google_drive.rs::a_selection_the_projection_lacks_is_refused_at_load`
/// once substituted `drive.files.download`, which the pinned document and the
/// committed bundle both carry (POST), so the load was refused for the method,
/// not the absence the test is named for. It now substitutes
/// `drive.files.search`, which neither carries. Asserting the refusal it
/// claims for that id:
#[test]
fn the_drive_absent_selection_case_is_refused_for_its_absence() {
    let bundle = committed_bundle(PROVIDER);
    let mut selections = shipped(PROVIDER);
    selections[0].operation_id = "drive.files.search".into();
    let refusal = Engine::new(&bundle, BASE, &selections)
        .err()
        .expect("refused at load");
    assert!(
        refusal.message.contains("bundle carries no operation"),
        "refused for another reason: {}",
        refusal.message
    );
}

/// Drive refuses `about.get` unless `fields` is set (the story's own note, the
/// selection's description and the projected operation description). An agent
/// reading `connectors operations describe` sees the input schema, so the
/// selection declares `fields` required and the schema says so; the input `{}`
/// is refused before a request is spent on a certain refusal.
#[test]
fn about_get_declares_fields_required() {
    let bundle = committed_bundle(PROVIDER);
    let engine = Engine::new(&bundle, BASE, &shipped(PROVIDER)).unwrap();
    let about = engine
        .declarations(&[Effect::Read])
        .into_iter()
        .find(|o| o.id == "about.get")
        .unwrap();
    assert_eq!(about.input_schema["required"], json!(["fields"]));
    let description = &about.description;
    assert!(
        description.contains("Drive requires fields"),
        "the selection's description no longer warns that Drive requires `fields`"
    );
}

type Requests = Arc<Mutex<Vec<(String, String, Option<String>)>>>;

/// Answers keyed by route only: exports at and just over the response limit,
/// a non-UTF-8 export, and nothing else (404).
fn answer(target: &str) -> Option<(&'static str, Vec<u8>)> {
    let (route, _) = target.split_once('?').unwrap_or((target, ""));
    let limit = connectors_core::RESPONSE_LIMIT;
    match route {
        "/drive/v3/files/fixture-at-limit/export" => Some(("text/plain", vec![b'x'; limit])),
        "/drive/v3/files/fixture-over-limit/export" => Some(("text/plain", vec![b'x'; limit + 1])),
        "/drive/v3/files/fixture-binary/export" => {
            Some(("application/pdf", vec![0x25, 0x50, 0xff, 0xfe, 0x00, 0x81]))
        }
        _ => None,
    }
}

fn id_token() -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let part = |value: &Value| URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap());
    format!(
        "{}.{}.{}",
        part(&json!({"alg": "RS256", "typ": "JWT", "kid": "fixture"})),
        part(
            &json!({"iss": "https://accounts.google.com", "aud": CLIENT_ID, "azp": CLIENT_ID,
                     "sub": "110000000000000000002", "iat": 1, "exp": 4_000_000_000_u64})
        ),
        URL_SAFE_NO_PAD.encode(b"fixture-signature")
    )
}

fn documented_config() -> Value {
    let guide = fs::read_to_string(root().join("../../docs/catalog-google-drive.md")).unwrap();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains(&format!("\"provider\": \"{PROVIDER}\"")))
        .unwrap();
    serde_json::from_str::<Value>(example).unwrap()
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    requests: Requests,
}

async fn write_fixture_response(
    stream: &mut (impl tokio::io::AsyncWrite + Unpin),
    head: &[u8],
    body: &[u8],
) {
    // The over-limit client may close early; its refusal is asserted by the caller.
    let _ = stream.write_all(head).await;
    let _ = stream.write_all(body).await;
    // write_all may accept plaintext while TLS ciphertext is still buffered.
    let _ = stream.flush().await;
}

/// Real TLS over a bounded byte stream makes pending ciphertext deterministic.
/// This checks the fixture writer, independently of the production-child size test.
#[tokio::test]
async fn tls_fixture_delivers_the_complete_response_under_backpressure() {
    tokio::time::timeout(Duration::from_secs(5), async {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let mut server = rustls::ServerConfig::builder_with_provider(Arc::new(
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
        // Neither side drives application reads until both handshakes finish.
        server.send_tls13_tickets = 0;
        let mut roots = rustls::RootCertStore::empty();
        roots.add(cert.cert.der().clone()).unwrap();
        let client = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
        let acceptor = TlsAcceptor::from(Arc::new(server));
        let connector = tokio_rustls::TlsConnector::from(Arc::new(client));
        let (client_io, server_io) = tokio::io::duplex(64);
        let (server, client) = tokio::join!(
            acceptor.accept(server_io),
            connector.connect("localhost".try_into().unwrap(), client_io),
        );
        let mut server = server.unwrap();
        let mut client = client.unwrap();
        let body = vec![b'x'; 1024];
        let head = b"HTTP/1.1 200 fixture\r\nContent-Length: 1024\r\nConnection: close\r\n\r\n";
        let expected = [head.as_slice(), body.as_slice()].concat();
        let send = async move {
            write_fixture_response(&mut server, head, &body).await;
        };
        let receive = async move {
            let mut received = vec![0; expected.len()];
            client
                .read_exact(&mut received)
                .await
                .expect("the fixture must deliver every declared response byte before drop");
            assert_eq!(received, expected);
        };
        tokio::join!(send, receive);
    })
    .await
    .expect("the bounded TLS fixture must finish");
}

impl Provider {
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
                    let authorization = header("authorization");
                    let (status, content_type, answer) = if method == "POST" && target == "/token"
                    {
                        let form = String::from_utf8(body).unwrap();
                        let mut fields: Vec<&str> = form.split('&').collect();
                        fields.sort();
                        let mut expected = [
                            "grant_type=refresh_token".to_owned(),
                            format!("client_id={CLIENT_ID}"),
                            format!("client_secret={CLIENT_SECRET}"),
                            format!("refresh_token={REFRESH_TOKEN}"),
                        ];
                        expected.sort();
                        if fields == expected {
                            let grant = json!({
                                "access_token": ACCESS_TOKEN, "token_type": "Bearer",
                                "expires_in": 3600,
                                "scope": format!("openid {DRIVE_SCOPE}"),
                                "id_token": id_token()});
                            (200, "application/json", serde_json::to_vec(&grant).unwrap())
                        } else {
                            let refusal = json!({"error": "invalid_grant"});
                            (400, "application/json", serde_json::to_vec(&refusal).unwrap())
                        }
                    } else if authorization.as_deref() != Some(&*format!("Bearer {ACCESS_TOKEN}"))
                    {
                        let refusal = json!({"error": {"code": 401}});
                        (401, "application/json", serde_json::to_vec(&refusal).unwrap())
                    } else {
                        match (method.as_str(), answer(&target)) {
                            ("GET", Some((content_type, body))) => (200, content_type, body),
                            _ => {
                                let missing = json!({"error": {"code": 404}});
                                (404, "application/json", serde_json::to_vec(&missing).unwrap())
                            }
                        }
                    };
                    observed
                        .lock()
                        .unwrap()
                        .push((method, target, authorization));
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answer.len()
                    );
                    write_fixture_response(&mut stream, head.as_bytes(), &answer).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let origin = format!("{}{}", concat!("https", "://localhost:"), address.port());
        let mut config = documented_config();
        config["instance"] = json!("fixture-google-drive-adversary");
        config["bundle_directory"] = json!(root_path("generated/bundles"));
        config["operations_file"] = json!(root_path("providers/google-drive/operations.json"));
        config["api_base"] = json!(format!("{origin}/drive/v3"));
        config["ca_file"] = json!(ca);
        config["auth"]["token_url"] = json!(format!("{origin}/token"));
        config["auth"]["token_ca_file"] = json!(ca);
        let path = directory.join("catalog.json");
        private(&path, &serde_json::to_vec(&config).unwrap());
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config: path,
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
        assert!(output.status.success());
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        bootstrap.validate().unwrap();
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture-google-drive-adversary".into(),
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
    fn api_targets(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, target, _)| target != "/token")
            .map(|(_, target, _)| target.clone())
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
    Secret(
        serde_json::to_vec(&json!({
            "client_id": CLIENT_ID,
            "client_secret": CLIENT_SECRET,
            "refresh_token": REFRESH_TOKEN,
        }))
        .unwrap(),
    )
}
fn attempt(child: &mut Child, operation: &str, input: &Value) -> Result<Vec<u8>, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child.invoke(
        operation,
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        connectors_sdk::now_ms() + 30_000,
    )
}

/// A Drive `fileId` carrying `/`, `?` or `#` stays one escaped segment on both
/// path-bearing reads; the route (before the query) is compared.
#[test]
fn drive_file_id_never_splits_or_leaks_into_the_query() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, route) in [
        (
            "files.get",
            json!({"fileId": "fixture/doc"}),
            "/drive/v3/files/fixture%2Fdoc",
        ),
        (
            "files.get",
            json!({"fileId": "fixture?doc#x"}),
            "/drive/v3/files/fixture%3Fdoc%23x",
        ),
        (
            "files.export",
            json!({"fileId": "fixture/doc", "mimeType": "text/plain"}),
            "/drive/v3/files/fixture%2Fdoc/export",
        ),
    ] {
        let before = provider.api_targets().len();
        let _ = attempt(&mut child, operation, &input);
        let targets = provider.api_targets();
        assert_eq!(targets.len(), before + 1, "`{operation}` {input}");
        let sent = targets[before]
            .split_once('?')
            .map_or(&*targets[before], |(r, _)| r);
        assert_eq!(sent, route, "`{operation}` {input}");
    }
}

/// The guide: "an answer over 4 MiB (`connectors_core::RESPONSE_LIMIT`) is
/// refused as `capacity`", and the selection: "at most the engine's 4 MiB
/// response limit". An export of exactly the limit is returned as text; one
/// byte more is refused as capacity.
#[test]
fn files_export_at_and_over_the_response_limit() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let at = attempt(
        &mut child,
        "files.export",
        &json!({"fileId": "fixture-at-limit", "mimeType": "text/plain"}),
    );
    let at: Value = serde_json::from_slice(
        &at.unwrap_or_else(|failure| panic!("an export of exactly 4 MiB failed: {failure:?}")),
    )
    .unwrap();
    assert_eq!(
        at["body"].as_str().map(str::len),
        Some(connectors_core::RESPONSE_LIMIT)
    );
    let over = attempt(
        &mut child,
        "files.export",
        &json!({"fileId": "fixture-over-limit", "mimeType": "text/plain"}),
    );
    assert!(
        matches!(over, Err(Failure::Capacity | Failure::ProviderCapacity)),
        "{over:?}"
    );
}

/// The guide: "bytes that are not UTF-8 are refused as `upstream_protocol`".
#[test]
fn files_export_of_non_utf8_bytes_is_refused_as_upstream_protocol() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let outcome = attempt(
        &mut child,
        "files.export",
        &json!({"fileId": "fixture-binary", "mimeType": "application/pdf"}),
    );
    assert!(matches!(outcome, Err(Failure::Protocol)), "{outcome:?}");
}

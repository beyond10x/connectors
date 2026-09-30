//! Google Slides presentation and page reads through the catalog provider.
//!
//! The shipped selection set is pinned by id and Discovery method id, resolves
//! against the committed bundle compiled from the projection of the pinned
//! Slides v1 Discovery document, and is cited row by row in
//! `docs/catalog-google-slides.md`. Each read runs through the provider child
//! against a disposable HTTPS fixture that serves the Slides routes and an
//! OAuth token route on one host: the child exchanges the fixture refresh entry
//! for an access token, and the exact request (path, query,
//! `Authorization: Bearer …`) and the returned body are asserted. A path value
//! carrying `/` stays one escaped segment. The profile is the guide's own,
//! pointed at the fixture token route. The fixture secrets are fictional and
//! only ever compared, never printed. No live credential and no network.
use connectors_catalog::{bundle, discovery};
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

/// The path the configured `api_base` carries: none. The projection's one
/// server is `https://slides.googleapis.com`, and each operation path carries
/// its own `/v1`.
const BASE: &str = "";
const PROVIDER: &str = "google-slides";
/// The bundle's auth profile, and the profile of the guide's configuration.
const PROFILE: &str = "google.oauth";
const SLIDES_SCOPE: &str = "https://www.googleapis.com/auth/presentations.readonly";
/// Fictional OAuth material. The access token is what the fixture token route
/// issues and the only bearer the fixture Slides routes accept.
const CLIENT_ID: &str = "fixture-client-id.apps.example.test";
const CLIENT_SECRET: &str = "fixture-client-secret-slides";
const REFRESH_TOKEN: &str = "fixture-refresh-token-slides";
const ACCESS_TOKEN: &str = "fixture-access-token-slides";
/// The pinned Discovery document, relative to this crate.
const UPSTREAM: &str = "../google/upstream/slides/slides-api.json";
/// The committed projection, relative to this crate.
const PROJECTED: &str = "../google/generated/slides.openapi.json";

/// The shipped ids, their Discovery method id and the path the bundle records.
/// A renamed, dropped or added id fails here.
const SHIPPED: [(&str, &str, &str); 3] = [
    (
        "presentations.get",
        "slides.presentations.get",
        "/v1/presentations/{presentationId}",
    ),
    (
        "presentations.pages.get",
        "slides.presentations.pages.get",
        "/v1/presentations/{presentationId}/pages/{pageObjectId}",
    ),
    (
        "presentations.pages.getThumbnail",
        "slides.presentations.pages.getThumbnail",
        "/v1/presentations/{presentationId}/pages/{pageObjectId}/thumbnail",
    ),
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers/google-slides/operations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], PROVIDER);
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn pinned() -> Value {
    serde_json::from_slice(&fs::read(root().join(UPSTREAM)).unwrap()).unwrap()
}
fn committed_bundle() -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), PROVIDER).unwrap()
}

#[test]
fn shipped_slides_selections_are_exactly_the_three_reads() {
    let selections = shipped();
    let bundle = committed_bundle();
    assert_eq!(bundle.auth_profile, PROFILE);
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let mut declared: Vec<String> = engine
        .declarations(&[Effect::Read, Effect::Write])
        .into_iter()
        .map(|o| o.id)
        .collect();
    declared.sort();
    let expected: Vec<&str> = SHIPPED.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(declared, expected);
    assert!(engine.declarations(&[Effect::Write]).is_empty());
    for (id, operation_id, path) in SHIPPED {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        // The selection id is the Discovery id without its API-name prefix.
        assert_eq!(
            Some(id),
            operation_id.strip_prefix("slides."),
            "`{id}` is not `{operation_id}` without `slides.`"
        );
        assert_eq!(selection.effect, Effect::Read, "`{id}`");
        assert_eq!(engine.effect(id), Some(Effect::Read), "`{id}`");
        // Every response is JSON, as the projection declares; none is read as
        // text and none is bounded.
        assert_eq!(selection.response, None, "`{id}`");
        assert!(selection.bounds.is_empty(), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the projection lacks `{operation_id}`"));
        assert_eq!(operation.method, "get", "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
    }
}

#[test]
fn a_selection_the_projection_lacks_is_refused_at_load() {
    let bundle = committed_bundle();
    let mut selections = shipped();
    // Slides has no list method; presentations are listed through Drive. The
    // id is absent from the bundle, so the load is refused for its absence and
    // for nothing else.
    let absent = "slides.presentations.list";
    assert!(
        bundle
            .inventory
            .operations
            .iter()
            .all(|o| o.operation_id.as_deref() != Some(absent)),
        "the bundle carries `{absent}`"
    );
    selections[0].operation_id = absent.into();
    let refusal = Engine::new(&bundle, BASE, &selections)
        .err()
        .expect("refused at load");
    assert_eq!(
        refusal.message,
        format!("bundle carries no operation `{absent}`")
    );
}

/// The bundle is the projection of the pinned document, and records it; the
/// reserved expansion of `presentations.get` is recorded as rewritten.
#[test]
fn the_bundle_is_derived_from_the_pinned_discovery_document() {
    let bytes = fs::read(root().join(UPSTREAM)).unwrap();
    let digest = hex::encode(Sha256::digest(&bytes));
    let projection = discovery::project(&bytes).unwrap();
    let committed = fs::read(root().join(PROJECTED)).unwrap();
    assert!(
        committed == projection.openapi,
        "committed projection drifted from the pinned Discovery document"
    );
    assert!(
        projection
            .record
            .rewritten_paths
            .iter()
            .any(|r| r.operation_id == "slides.presentations.get"),
        "the `{{+presentationId}}` rewrite is not recorded"
    );
    let bundle = committed_bundle();
    assert_eq!(bundle.source.file_name, "slides.openapi.json");
    assert_eq!(
        bundle.source.source_sha256,
        hex::encode(Sha256::digest(&committed))
    );
    let derivation = bundle.source.derivation.expect("the bundle's derivation");
    assert_eq!(derivation.from_file, "slides-api.json");
    assert_eq!(derivation.from_sha256, digest);
    assert_eq!(derivation.from_bytes, bytes.len());
    assert_eq!(derivation.format, discovery::FORMAT);
    assert_eq!(derivation.projector, discovery::PROJECTOR);
    assert_eq!(json!(derivation.discovery_revision), pinned()["revision"]);
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    assert!(index.find(PROVIDER).is_some());
}

fn guide() -> String {
    fs::read_to_string(root().join("../../docs/catalog-google-slides.md")).unwrap()
}

#[test]
fn guide_cites_each_operation_the_response_cap_and_fields() {
    let guide = guide();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    for (id, operation_id, path) in SHIPPED {
        let cited = rows.iter().any(|row| {
            row.starts_with(&format!("| `{id}` "))
                && row.contains(&format!("`{operation_id}`"))
                && row.contains(&format!("`GET {path}`"))
        });
        assert!(cited, "no row cites `{id}` as `{operation_id}` `{path}`");
    }
    // The response cap, and `fields` as the way to stay under it.
    assert!(guide.contains("4 MiB"), "the guide does not state the cap");
    assert!(guide.contains("`connectors_core::RESPONSE_LIMIT`"));
    assert!(
        guide.contains("`fields`"),
        "the guide does not name `fields`"
    );
    assert!(guide.contains("`contentUrl`"));
}

/// The guide's configuration example for this provider.
fn documented_config() -> Value {
    let guide = guide();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains(&format!("\"provider\": \"{PROVIDER}\"")))
        .expect("the documented google-slides configuration");
    serde_json::from_str::<Value>(example).unwrap()
}

/// The guide configures the bundle's profile as an `oauth2_refresh` profile
/// against Google's token endpoint, with the Slides read-only scope, and the
/// projection's server as the API base.
#[test]
fn guide_documents_the_oauth_refresh_configuration() {
    let config = documented_config();
    assert_eq!(config["api_base"], "https://slides.googleapis.com");
    let auth = &config["auth"];
    assert_eq!(auth["profile"], PROFILE);
    assert_eq!(auth["scheme"], "oauth2_refresh");
    assert_eq!(auth["header"], "Authorization");
    assert_eq!(auth["bearer"], true);
    assert_eq!(auth["token_url"], "https://oauth2.googleapis.com/token");
    assert_eq!(
        auth["authorize_url"],
        "https://accounts.google.com/o/oauth2/auth"
    );
    assert_eq!(auth["identity"]["source"], "id_token");
    assert_eq!(auth["minimum_scopes"], json!([SLIDES_SCOPE]));
    assert!(
        auth["requested_scopes"]
            .as_array()
            .unwrap()
            .contains(&json!(SLIDES_SCOPE))
    );
    assert!(auth.get("token_ca_file").is_none());
    // Every shipped read accepts that scope, per the pinned document.
    let document = pinned();
    let presentations = &document["resources"]["presentations"];
    for method in [
        &presentations["methods"]["get"],
        &presentations["resources"]["pages"]["methods"]["get"],
        &presentations["resources"]["pages"]["methods"]["getThumbnail"],
    ] {
        assert!(
            method["scopes"]
                .as_array()
                .unwrap()
                .contains(&json!(SLIDES_SCOPE)),
            "`{}`",
            method["id"]
        );
    }
}

/// Method, route with query, and `Authorization` header of each fixture request.
type Requests = Arc<Mutex<Vec<(String, String, Option<String>)>>>;

const DECK: &str = "fixture-deck-1";
fn presentation() -> Value {
    json!({"presentationId": DECK, "title": "Fixture deck",
           "slides": [{"objectId": "fixture-slide-1"}, {"objectId": "fixture-slide-2"}]})
}
fn page() -> Value {
    json!({"objectId": "fixture-slide-1", "pageType": "SLIDE",
           "pageElements": [{"objectId": "fixture-shape-1",
                             "shape": {"shapeType": "TEXT_BOX",
                                       "text": {"textElements": [
                                           {"textRun": {"content": "Fixture title\n"}}]}}}],
           "revisionId": "fixture-revision"})
}
fn thumbnail() -> Value {
    json!({"contentUrl": "https://thumbnails.example.test/fixture-slide-1.png",
           "width": 200, "height": 112})
}

/// The recorded Slides answers the fixture serves, keyed by route: a content
/// type and a body. `None` for anything else, which the fixture answers 404.
fn answer(target: &str) -> Option<(&'static str, Vec<u8>)> {
    let (route, _) = target.split_once('?').unwrap_or((target, ""));
    let json_body = |value: Value| Some(("application/json", serde_json::to_vec(&value).unwrap()));
    match route {
        "/v1/presentations/fixture-deck-1" => json_body(presentation()),
        "/v1/presentations/fixture-deck-1/pages/fixture-slide-1" => json_body(page()),
        "/v1/presentations/fixture-deck-1/pages/fixture-slide-1/thumbnail" => {
            json_body(thumbnail())
        }
        _ => None,
    }
}
/// The recorded JSON body of `target`.
fn recorded(target: &str) -> Value {
    serde_json::from_slice(&answer(target).unwrap().1).unwrap()
}

/// An unsigned JWT with `claims` as its payload, as the token answer's
/// `id_token`.
fn id_token() -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let part = |value: &Value| URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap());
    format!(
        "{}.{}.{}",
        part(&json!({"alg": "RS256", "typ": "JWT", "kid": "fixture"})),
        part(
            &json!({"iss": "https://accounts.google.com", "aud": CLIENT_ID, "azp": CLIENT_ID,
                     "sub": "110000000000000000001", "iat": 1, "exp": 4_000_000_000_u64})
        ),
        URL_SAFE_NO_PAD.encode(b"fixture-signature")
    )
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
                        // Only the fixture entry is exchanged; the form is
                        // compared, never printed.
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
                                "scope": format!("openid {SLIDES_SCOPE}"),
                                "id_token": id_token()});
                            (200, "application/json", serde_json::to_vec(&grant).unwrap())
                        } else {
                            let refusal = json!({"error": "invalid_grant"});
                            (400, "application/json", serde_json::to_vec(&refusal).unwrap())
                        }
                    } else if authorization.as_deref() != Some(&*format!("Bearer {ACCESS_TOKEN}"))
                    {
                        let refusal = json!({"error": {"code": 401, "message": "fixture refusal"}});
                        (401, "application/json", serde_json::to_vec(&refusal).unwrap())
                    } else {
                        match (method.as_str(), answer(&target)) {
                            ("GET", Some((content_type, body))) => (200, content_type, body),
                            _ => {
                                let missing = json!({"error": {"code": 404, "message": "no fixture"}});
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
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&answer).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        // The guide's configuration, with the fixture as API host and token
        // host, trusted through its own CA.
        let mut config = documented_config();
        config["instance"] = json!("fixture-google-slides");
        config["bundle_directory"] = json!(root_path("generated/bundles"));
        config["operations_file"] = json!(root_path("providers/google-slides/operations.json"));
        config["api_base"] = json!(format!("https://localhost:{}", address.port()));
        config["ca_file"] = json!(ca);
        config["auth"]["token_url"] = json!(format!("https://localhost:{}/token", address.port()));
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
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture-google-slides".into(),
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
    fn requests(&self) -> Vec<(String, String, Option<String>)> {
        self.requests.lock().unwrap().clone()
    }
    /// The Slides requests (everything but the token route), as route with query.
    fn api_targets(&self) -> Vec<String> {
        self.requests()
            .into_iter()
            .filter(|(_, target, _)| target != "/token")
            .map(|(_, target, _)| target)
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
fn invoke(child: &mut Child, operation: &str, input: Value) -> Value {
    let output = attempt(child, operation, &input)
        .unwrap_or_else(|failure| panic!("`{operation}` failed: {failure:?}"));
    serde_json::from_slice(&output).unwrap()
}

/// Each read's input and the exact request the fixture must observe.
fn first_requests() -> [(&'static str, Value, &'static str); 3] {
    [
        (
            "presentations.get",
            json!({"presentationId": DECK, "fields": "presentationId,title,slides.objectId"}),
            "/v1/presentations/fixture-deck-1?fields=presentationId%2Ctitle%2Cslides.objectId",
        ),
        (
            "presentations.pages.get",
            json!({"presentationId": DECK, "pageObjectId": "fixture-slide-1"}),
            // The transport writes an empty query when no query parameter is
            // bound, so the wire request ends in `?`.
            "/v1/presentations/fixture-deck-1/pages/fixture-slide-1?",
        ),
        (
            "presentations.pages.getThumbnail",
            json!({"presentationId": DECK, "pageObjectId": "fixture-slide-1",
                   "thumbnailProperties.mimeType": "PNG",
                   "thumbnailProperties.thumbnailSize": "SMALL"}),
            "/v1/presentations/fixture-deck-1/pages/fixture-slide-1/thumbnail?thumbnailProperties.mimeType=PNG&thumbnailProperties.thumbnailSize=SMALL",
        ),
    ]
}

#[test]
fn each_read_sends_the_declared_request_with_the_exchanged_bearer_and_returns_the_recorded_body() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, expected) in first_requests() {
        let before = provider.api_targets().len();
        let result = invoke(&mut child, operation, input);
        let targets = provider.api_targets();
        assert_eq!(targets.len(), before + 1, "`{operation}` requests");
        assert_eq!(targets[before], expected, "`{operation}` request");
        let (method, _, authorization) = provider
            .requests()
            .into_iter()
            .rfind(|(_, target, _)| target == expected)
            .unwrap();
        assert_eq!(method, "GET", "`{operation}` method");
        assert!(
            authorization.as_deref() == Some(&*format!("Bearer {ACCESS_TOKEN}")),
            "`{operation}` does not carry the exchanged bearer"
        );
        assert_eq!(result["status"], 200, "`{operation}` status");
        assert_eq!(result["provenance"]["instance"], "fixture-google-slides");
        // The engine re-serialises the body, so the recorded answer is compared
        // as JSON, not as bytes.
        assert_eq!(result["body"], recorded(expected), "`{operation}` body");
    }
    // The thumbnail read returned the image's URL and fetched nothing from it:
    // every request went to the fixture's token route or its Slides routes.
    let requests = provider.requests();
    assert!(
        requests
            .iter()
            .all(|(_, target, _)| target == "/token" || target.starts_with("/v1/presentations/")),
        "an unexpected request was sent"
    );
    // The bearer came from the token route: one exchange, a form POST with no
    // credential header, before any Slides request.
    let token: Vec<_> = requests.iter().filter(|(_, t, _)| t == "/token").collect();
    assert_eq!(token.len(), 1, "token exchanges");
    assert_eq!(token[0].0, "POST");
    assert!(
        token[0].2.is_none(),
        "credential header on the token request"
    );
    assert_eq!(requests[0].1, "/token", "the exchange comes first");
    assert_eq!(requests.len(), 4);
}

/// A path value carrying `/` is escaped into one segment and never split into
/// two, for every path parameter of every shipped read — including the
/// `{+presentationId}` reserved expansion, which the projection narrows to a
/// plain `{presentationId}`.
#[test]
fn presentations_get_refuses_slash_in_id() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let cases = [
        (
            "presentations.get",
            json!({"presentationId": "fixture/deck"}),
            "/v1/presentations/fixture%2Fdeck?",
        ),
        (
            "presentations.pages.get",
            json!({"presentationId": "fixture/deck", "pageObjectId": "fixture-slide-1"}),
            "/v1/presentations/fixture%2Fdeck/pages/fixture-slide-1?",
        ),
        (
            "presentations.pages.get",
            json!({"presentationId": DECK, "pageObjectId": "fixture/slide"}),
            "/v1/presentations/fixture-deck-1/pages/fixture%2Fslide?",
        ),
        (
            "presentations.pages.getThumbnail",
            json!({"presentationId": "fixture/deck", "pageObjectId": "fixture-slide-1"}),
            "/v1/presentations/fixture%2Fdeck/pages/fixture-slide-1/thumbnail?",
        ),
        (
            "presentations.pages.getThumbnail",
            json!({"presentationId": DECK, "pageObjectId": "fixture/slide"}),
            "/v1/presentations/fixture-deck-1/pages/fixture%2Fslide/thumbnail?",
        ),
    ];
    // Every path parameter of every shipped read is covered above.
    let bundle = committed_bundle();
    for selection in shipped() {
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(selection.operation_id.as_str()))
            .unwrap();
        for parameter in operation
            .parameters
            .iter()
            .filter(|p| p.location == connectors_catalog::inventory::Location::Path)
        {
            assert!(
                cases.iter().any(|(id, input, _)| *id == selection.id
                    && input[&parameter.name].as_str().unwrap().contains('/')),
                "`{}` `{}` has no slash case",
                selection.id,
                parameter.name
            );
        }
    }
    for (operation, input, expected) in cases {
        let before = provider.api_targets().len();
        // The fixture has no recorded answer for these ids; only the request
        // it observed is under test.
        let _ = attempt(&mut child, operation, &input);
        let targets = provider.api_targets();
        assert_eq!(targets.len(), before + 1, "`{operation}` requests");
        assert_eq!(targets[before], expected, "`{operation}` request");
    }
}

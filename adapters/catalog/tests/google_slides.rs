//! Google Slides presentation and page reads, and the two guarded writes,
//! through the catalog provider.
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
//!
//! Each write runs through the same child over private protocol two: the
//! host's prepare and commit, the declared preflight, and the exact POST body.
//! The approval binding is exercised with the host's own approval signer and
//! verifier against a subject built from the child's descriptor; the signing
//! key is the public RFC 8032 section 7.1 test vector, never a deployment key.
use connectors_catalog::{bundle, discovery};
use connectors_catalog_provider::{Check, Effect, Engine, Expectation, Selection};
use connectors_host::local::{
    approvals,
    config::{Adapter, Executable, Restart, Startup},
    filesystem, mutations,
    runtime::{Bootstrap, Child, Failure, PrivateProtocol, WriteEffect},
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
/// The Slides scope both writes need, per the pinned document.
const WRITE_SCOPE: &str = "https://www.googleapis.com/auth/presentations";
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
/// The shipped writes, their Discovery method id and the path the bundle
/// records. Both are POSTs with a JSON body.
const WRITES: [(&str, &str, &str); 2] = [
    (
        "presentations.create",
        "slides.presentations.create",
        "/v1/presentations",
    ),
    (
        "presentations.batchUpdate",
        "slides.presentations.batchUpdate",
        "/v1/presentations/{presentationId}:batchUpdate",
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
fn shipped_slides_selections_are_the_three_reads_and_two_writes() {
    let selections = shipped();
    let bundle = committed_bundle();
    assert_eq!(bundle.auth_profile, PROFILE);
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let ids = |effect: Effect| {
        let mut ids: Vec<String> = engine
            .declarations(&[effect])
            .into_iter()
            .map(|o| o.id)
            .collect();
        ids.sort();
        ids
    };
    let expected: Vec<&str> = SHIPPED.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(ids(Effect::Read), expected);
    let mut expected: Vec<&str> = WRITES.iter().map(|(id, _, _)| *id).collect();
    expected.sort();
    assert_eq!(ids(Effect::Write), expected);
    assert_eq!(selections.len(), SHIPPED.len() + WRITES.len());
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
    let declarations = engine.declarations(&[Effect::Write]);
    for (id, operation_id, path) in WRITES {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(Some(id), operation_id.strip_prefix("slides."), "`{id}`");
        assert_eq!(selection.effect, Effect::Write, "`{id}`");
        assert_eq!(engine.effect(id), Some(Effect::Write), "`{id}`");
        assert_eq!(selection.response, None, "`{id}`");
        assert!(selection.bounds.is_empty(), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the projection lacks `{operation_id}`"));
        assert_eq!(operation.method, "post", "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
        assert_eq!(
            operation.request_media_types,
            ["application/json"],
            "`{id}`"
        );
        // A write is a required-approval mutation, and its whole input —
        // including the body the approval digest covers — is closed.
        let declaration = declarations.iter().find(|o| o.id == id).unwrap();
        assert_eq!(declaration.profile, "mutation", "`{id}`");
        let schema = &declaration.input_schema;
        assert_eq!(schema["additionalProperties"], false, "`{id}`");
        assert!(
            schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("body")),
            "`{id}` does not require its body"
        );
    }
    // Nothing exists to compare before a create: it carries no guard.
    let create = selections
        .iter()
        .find(|s| s.id == "presentations.create")
        .unwrap();
    assert_eq!(create.guard, None);
    // The update is guarded on the presentation's revision: one read of the
    // presentation before the write, compared with the revision the body pins
    // in `writeControl.requiredRevisionId`, which Google also checks
    // atomically. The acknowledgement names the same presentation.
    let guard = selections
        .iter()
        .find(|s| s.id == "presentations.batchUpdate")
        .unwrap()
        .guard
        .as_ref()
        .expect("`presentations.batchUpdate` is guarded");
    assert_eq!(guard.preflight.operation_id, "slides.presentations.get");
    assert_eq!(
        guard.preflight.values,
        [("presentationId".to_owned(), "presentationId".to_owned())].into()
    );
    assert_eq!(
        guard.preflight.checks,
        [Check {
            pointer: "/revisionId".into(),
            expect: Expectation::Input("body.writeControl.requiredRevisionId".into()),
        }]
    );
    assert_eq!(
        guard.postflight.checks,
        [Check {
            pointer: "/presentationId".into(),
            expect: Expectation::Input("presentationId".into()),
        }]
    );
    // The pinned document names the fields the guard reads and compares.
    let document = pinned();
    let schemas = &document["schemas"];
    assert_eq!(
        schemas["Presentation"]["properties"]["revisionId"]["type"],
        "string"
    );
    assert_eq!(
        schemas["BatchUpdatePresentationRequest"]["properties"]["writeControl"]["$ref"],
        "WriteControl"
    );
    assert_eq!(
        schemas["WriteControl"]["properties"]["requiredRevisionId"]["type"],
        "string"
    );
    assert_eq!(
        schemas["BatchUpdatePresentationResponse"]["properties"]["presentationId"]["type"],
        "string"
    );
    // Both writes accept the Slides write scope, and neither accepts the
    // read-only one; the write scope covers every shipped read too.
    let presentations = &document["resources"]["presentations"];
    let methods = &presentations["methods"];
    for method in [&methods["create"], &methods["batchUpdate"]] {
        let scopes = method["scopes"].as_array().unwrap();
        assert!(scopes.contains(&json!(WRITE_SCOPE)), "`{}`", method["id"]);
        assert!(!scopes.contains(&json!(SLIDES_SCOPE)), "`{}`", method["id"]);
    }
    let pages = &presentations["resources"]["pages"]["methods"];
    for method in [&methods["get"], &pages["get"], &pages["getThumbnail"]] {
        let scopes = method["scopes"].as_array().unwrap();
        assert!(scopes.contains(&json!(WRITE_SCOPE)), "`{}`", method["id"]);
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
fn guide_cites_each_operation_the_write_scope_the_response_cap_and_fields() {
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
    for (id, operation_id, path) in WRITES {
        let cited = rows.iter().any(|row| {
            row.starts_with(&format!("| `{id}` "))
                && row.contains(&format!("`{operation_id}`"))
                && row.contains(&format!("`POST {path}`"))
        });
        assert!(
            cited,
            "no row cites `{id}` as `{operation_id}` `POST {path}`"
        );
    }
    // The write scope, the pinned revision the guard compares, and the route to
    // a write-capable connection: a separate instance, connected afresh. Repair
    // cannot add a scope, and the guide must not say it does.
    assert!(guide.contains(&format!("`{WRITE_SCOPE}`")));
    assert!(guide.contains("`writeControl.requiredRevisionId`"));
    let flat = guide.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("Writes use a separate instance"));
    assert!(guide.contains("`google-slides-write`"));
    assert!(flat.contains("`connections connect` that instance"));
    assert!(flat.contains("`connections repair` cannot add a scope"));
    assert!(
        !flat.contains("with `connections repair`"),
        "the guide still routes the write scope through `connections repair`"
    );
    // `fields` on `batchUpdate` must keep what the postflight compares, and a
    // view-only preflight's code is explained.
    assert!(flat.contains("must therefore keep `presentationId`"));
    assert!(flat.contains("fails its preflight as `upstream_protocol`"));
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
        "https://accounts.google.com/o/oauth2/v2/auth"
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
/// The fixture deck's current revision, which its read answers.
const REVISION: &str = "fixture-revision-2";
fn presentation() -> Value {
    json!({"presentationId": DECK, "title": "Fixture deck", "revisionId": REVISION,
           "slides": [{"objectId": "fixture-slide-1"}, {"objectId": "fixture-slide-2"}]})
}
/// The recorded answers to the two writes, keyed by route.
fn posted(target: &str) -> Option<Value> {
    let (route, _) = target.split_once('?').unwrap_or((target, ""));
    match route {
        "/v1/presentations" => Some(json!({"presentationId": "fixture-deck-new",
                                           "title": "Fixture new deck",
                                           "revisionId": "fixture-revision-new"})),
        "/v1/presentations/fixture%2Fdeck:batchUpdate" => {
            Some(json!({"presentationId": "fixture/deck", "replies": [{}]}))
        }
        "/v1/presentations/fixture-deck-1:batchUpdate" => Some(json!({
            "presentationId": DECK, "replies": [{}],
            "writeControl": {"requiredRevisionId": "fixture-revision-3"}})),
        _ => None,
    }
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
        // The deck whose id carries `/`, as the slash case of the write reads it.
        "/v1/presentations/fixture%2Fdeck" => {
            json_body(json!({"presentationId": "fixture/deck", "revisionId": REVISION}))
        }
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
    bodies: Bodies,
}
/// Route with query and body of each POST that reached a Slides route.
type Bodies = Arc<Mutex<Vec<(String, Vec<u8>)>>>;
impl Provider {
    fn new() -> Self {
        Self::with(|_| {})
    }
    /// The guide's configuration against the fixture, changed by `adjust`.
    fn with(adjust: impl FnOnce(&mut Value)) -> Self {
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
        let bodies: Bodies = Arc::new(Mutex::new(Vec::new()));
        let recorded_bodies = bodies.clone();
        let thread = std::thread::spawn(move || {
            let bodies = recorded_bodies;
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
                        if method == "POST" {
                            bodies.lock().unwrap().push((target.clone(), body));
                        }
                        let found = match method.as_str() {
                            "GET" => answer(&target),
                            "POST" => posted(&target).map(|value| {
                                ("application/json", serde_json::to_vec(&value).unwrap())
                            }),
                            _ => None,
                        };
                        match found {
                            Some((content_type, body)) => (200, content_type, body),
                            None => {
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
        adjust(&mut config);
        let path = directory.join("catalog.json");
        private(&path, &serde_json::to_vec(&config).unwrap());
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config: path,
            requests,
            bodies,
        }
    }
    /// The same selection over private protocol two, which carries writes.
    fn write_selection(&self) -> Adapter {
        Adapter {
            private_protocol: Some(PrivateProtocol::V2),
            ..self.selection()
        }
    }
    fn bodies(&self) -> Vec<(String, Value)> {
        self.bodies
            .lock()
            .unwrap()
            .iter()
            .map(|(target, body)| (target.clone(), serde_json::from_slice(body).unwrap()))
            .collect()
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
/// two, for every path parameter of every shipped read and write — including the
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
    // The one write with a path parameter: its preflight read and its POST
    // each carry the slash escaped in one segment, `:batchUpdate` unescaped.
    let [_, (write_id, deck)] = write_inputs();
    let mut slashed = deck.clone();
    slashed["presentationId"] = json!("fixture/deck");
    let write_cases = [(write_id, slashed)];
    // Every path parameter of every shipped read and write is covered.
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
            let slash = |id: &str, input: &Value| {
                id == selection.id && input[&parameter.name].as_str().unwrap().contains('/')
            };
            assert!(
                cases.iter().any(|(id, input, _)| slash(id, input))
                    || write_cases.iter().any(|(id, input)| slash(id, input)),
                "`{}` `{}` has no slash case",
                selection.id,
                parameter.name
            );
        }
    }
    for (operation, input, expected) in cases {
        let before = provider.api_targets().len();
        // Only the request the fixture observed is under test.
        let _ = attempt(&mut child, operation, &input);
        let targets = provider.api_targets();
        assert_eq!(targets.len(), before + 1, "`{operation}` requests");
        assert_eq!(targets[before], expected, "`{operation}` request");
    }
    let mut writer = Child::spawn(&provider.write_selection()).unwrap();
    for (operation, input) in write_cases {
        let before = provider.api_targets().len();
        let result = write(&mut writer, operation, &input).unwrap();
        assert_eq!(result.effect, WriteEffect::Applied, "`{operation}`");
        assert_eq!(
            provider.api_targets()[before..],
            [
                "/v1/presentations/fixture%2Fdeck?",
                "/v1/presentations/fixture%2Fdeck:batchUpdate?",
            ],
            "`{operation}` requests"
        );
    }
}

/// Each write's approved input: `create` with metadata only, `batchUpdate`
/// pinned to the fixture deck's current revision.
fn write_inputs() -> [(&'static str, Value); 2] {
    [
        (
            "presentations.create",
            json!({"body": {"title": "Fixture new deck"}}),
        ),
        (
            "presentations.batchUpdate",
            json!({"presentationId": DECK, "body": {
                "requests": [{"createSlide": {"objectId": "fixture-slide-3"}}],
                "writeControl": {"requiredRevisionId": REVISION}}}),
        ),
    ]
}
/// The same input with one value inside its body changed.
fn other_body(input: &Value) -> Value {
    let mut other = input.clone();
    match other["body"].get_mut("title") {
        Some(title) => *title = json!("Another deck"),
        None => other["body"]["requests"][0]["createSlide"]["objectId"] = json!("fixture-slide-4"),
    }
    other
}
/// Prepare through the host's write exchange and, if that succeeds, commit.
fn write(
    child: &mut Child,
    operation: &str,
    input: &Value,
) -> Result<connectors_host::local::runtime::WriteResult, Failure> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
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

/// A write offered on the dispatch path that carries no approval is refused
/// by the host before the child is asked for anything: no token exchange and
/// no Slides request. Over private protocol one a write is not even
/// described; over private protocol two it is a `mutation`, which only the
/// host's prepare/commit exchange runs, and the owner enters that exchange only
/// with a verified proof (`owner/mutation/execution.rs`). An absent or empty
/// proof document is refused where the owner decodes it.
#[test]
fn each_write_without_an_approval_is_refused_before_any_request() {
    let provider = Provider::new();
    let mut v1 = Child::spawn(&provider.selection()).unwrap();
    let mut v2 = Child::spawn(&provider.write_selection()).unwrap();
    let described = v2.bootstrap().descriptor().unwrap();
    for (operation, input) in write_inputs() {
        assert!(
            v1.bootstrap()
                .descriptor()
                .unwrap()
                .operation(operation)
                .is_err(),
            "`{operation}` described over private protocol one"
        );
        assert!(
            matches!(attempt(&mut v1, operation, &input), Err(Failure::NotFound)),
            "`{operation}` over private protocol one"
        );
        assert_eq!(
            described.operation(operation).unwrap().profile,
            "mutation",
            "`{operation}`"
        );
        assert!(
            matches!(
                attempt(&mut v2, operation, &input),
                Err(Failure::Unsupported)
            ),
            "`{operation}` dispatched without an approval"
        );
    }
    for document in [b"".as_slice(), b"{}"] {
        assert!(matches!(
            approvals::Evidence::from_document(Secret(document.to_vec())),
            Err(approvals::Failure::Refused)
        ));
    }
    assert!(provider.requests().is_empty(), "a request was sent");
}

/// The public RFC 8032 section 7.1 test 1 key pair: test material only.
const APPROVAL_SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const APPROVAL_PUBLIC: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
struct Now;
impl mutations::Clock for Now {
    fn now(&self) -> mutations::Result<mutations::ClockInterval> {
        let now = connectors_sdk::now_ms() as i64;
        Ok(mutations::ClockInterval {
            lower_unix_ms: now,
            upper_unix_ms: now + 1000,
        })
    }
}
struct ApprovalKey(approvals::ConfiguredApprovalKey);
impl approvals::CurrentAdmission for &ApprovalKey {
    fn key(&self) -> &approvals::ConfiguredApprovalKey {
        &self.0
    }
}
/// Admits by key id only, so any refusal is the proof's own subject binding.
impl approvals::ReceiverPolicy for ApprovalKey {
    type Guard<'a> = &'a ApprovalKey;
    fn admit<'a>(&'a self, _: &approvals::Subject, kid: &str) -> approvals::Result<&'a Self> {
        if kid == self.0.kid {
            Ok(self)
        } else {
            Err(approvals::Failure::Refused)
        }
    }
}
impl approvals::IssuancePolicy for ApprovalKey {
    type Guard<'a> = &'a ApprovalKey;
    fn authorize<'a>(
        &'a self,
        subject: &approvals::Subject,
        kid: &str,
    ) -> approvals::Result<&'a Self> {
        approvals::ReceiverPolicy::admit(self, subject, kid)
    }
}
fn approval_key() -> ApprovalKey {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    ApprovalKey(approvals::ConfiguredApprovalKey {
        issuer: "fixture-issuer".into(),
        audience: "approval:fixture-google-slides".into(),
        kid: "fixture-key".into(),
        public_key: URL_SAFE_NO_PAD.encode(hex::decode(APPROVAL_PUBLIC).unwrap()),
        not_before_unix_ms: 0,
        not_after_unix_ms: 4_000_000_000_000,
        revoked: false,
    })
}
/// The approval subject for `input` to `operation` on this child, with the
/// input digest the owner's issuance computes (`connectors_core::digest`).
fn subject(child: &Child, operation: &str, input: &Value) -> approvals::Subject {
    let bootstrap = child.bootstrap();
    let descriptor = bootstrap.descriptor().unwrap();
    let declared = descriptor.operation(operation).unwrap();
    approvals::Subject {
        format: "connectors.approval-subject/v1".into(),
        target: approvals::Target {
            instance: bootstrap.instance.clone(),
            operation: operation.into(),
            connection: "fixture-connection".into(),
            connection_revision: "fixture-connection-revision".into(),
            contract: declared.contract.clone(),
            profile: declared.profile.clone(),
            descriptor_revision: descriptor.revision.clone(),
            configuration_revision: bootstrap.configuration_revision.clone(),
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
            authority_ref: bootstrap.instance.clone(),
        },
        route: None,
        canonicalization: "adapter-v1-canonical-json".into(),
        input_sha256: connectors_core::digest(input),
        approval_mode: "required".into(),
    }
}

/// An approval binds the whole input, body included: a proof issued for one
/// body does not verify for another, so that write is refused before the
/// child prepares it and nothing is sent. The proof verifies for the body it
/// was issued for, and that write sends exactly that body.
#[test]
fn a_write_approved_for_a_different_body_is_refused() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.write_selection()).unwrap();
    let key = approval_key();
    let signer = approvals::Signer::from_seed(
        Secret(hex::decode(APPROVAL_SEED).unwrap()),
        "fixture-key".into(),
    )
    .unwrap();
    for (operation, approved) in write_inputs() {
        let proof = signer
            .issue(&subject(&child, operation, &approved), &key, &Now)
            .unwrap();
        let changed = other_body(&approved);
        assert_ne!(changed["body"], approved["body"]);
        let before = provider.requests().len();
        assert!(
            matches!(
                approvals::verify(&proof, &subject(&child, operation, &changed), &key, &Now),
                Err(approvals::Failure::Refused)
            ),
            "`{operation}` approval verified for a different body"
        );
        assert_eq!(provider.requests().len(), before, "`{operation}` sent");
        approvals::verify(&proof, &subject(&child, operation, &approved), &key, &Now)
            .unwrap_or_else(|failure| panic!("`{operation}` approval refused: {failure:?}"));
        let result = write(&mut child, operation, &approved).unwrap();
        assert_eq!(result.effect, WriteEffect::Applied, "`{operation}`");
        let bodies = provider.bodies();
        assert_eq!(bodies.last().unwrap().1, approved["body"], "`{operation}`");
    }
    // One POST per approved body, and none for a body that was not approved.
    assert_eq!(provider.bodies().len(), 2);
}

/// `batchUpdate` whose pinned revision is not the deck's current one fails
/// the preflight read: one GET of the presentation, then a refusal, and no
/// POST. A body that pins no revision is refused before any Slides request.
/// The current revision passes the preflight, and the one POST carries the
/// body unchanged to the `:batchUpdate` route.
#[test]
fn batch_update_with_a_stale_required_revision_fails_the_preflight_and_sends_no_write() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.write_selection()).unwrap();
    let operation = "presentations.batchUpdate";
    let [_, (_, current)] = write_inputs();
    let mut stale = current.clone();
    stale["body"]["writeControl"]["requiredRevisionId"] = json!("fixture-revision-1");
    assert!(
        matches!(
            write(&mut child, operation, &stale),
            Err(Failure::Forbidden)
        ),
        "a stale revision was not refused in preflight"
    );
    assert_eq!(
        provider.api_targets(),
        ["/v1/presentations/fixture-deck-1?"]
    );
    assert!(provider.bodies().is_empty(), "a write was sent");

    let mut unpinned = current.clone();
    unpinned["body"]
        .as_object_mut()
        .unwrap()
        .remove("writeControl");
    assert!(
        matches!(
            write(&mut child, operation, &unpinned),
            Err(Failure::InvalidInput)
        ),
        "a body that pins no revision was not refused"
    );
    assert_eq!(provider.api_targets().len(), 1, "an unpinned write read");
    assert!(provider.bodies().is_empty(), "a write was sent");

    let result = write(&mut child, operation, &current).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied);
    let value = result.result.unwrap();
    assert_eq!(value["status"], 200);
    assert_eq!(
        value["body"],
        posted("/v1/presentations/fixture-deck-1:batchUpdate").unwrap()
    );
    assert_eq!(
        provider.api_targets(),
        [
            "/v1/presentations/fixture-deck-1?",
            "/v1/presentations/fixture-deck-1?",
            "/v1/presentations/fixture-deck-1:batchUpdate?",
        ]
    );
    assert_eq!(
        provider.bodies(),
        [(
            "/v1/presentations/fixture-deck-1:batchUpdate?".to_owned(),
            current["body"].clone()
        )]
    );
    let (method, _, authorization) = provider.requests().pop().unwrap();
    assert_eq!(method, "POST");
    assert!(authorization.as_deref() == Some(&*format!("Bearer {ACCESS_TOKEN}")));
}

/// `create` has no preflight: one POST to `/v1/presentations` with the
/// metadata body unchanged, and the created presentation returned.
#[test]
fn create_sends_one_post_with_its_body_and_no_preflight() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.write_selection()).unwrap();
    let [(operation, input), _] = write_inputs();
    let result = write(&mut child, operation, &input).unwrap();
    assert_eq!(result.effect, WriteEffect::Applied);
    let value = result.result.unwrap();
    assert_eq!(value["body"], posted("/v1/presentations").unwrap());
    assert_eq!(provider.api_targets(), ["/v1/presentations?"]);
    assert_eq!(
        provider.bodies(),
        [("/v1/presentations?".to_owned(), input["body"].clone())]
    );
}

/// An instance configured for writes names the write scope in
/// `minimum_scopes`; a refresh token granted only the read-only scope then
/// refuses validation as insufficient scope. The same grant validates the
/// guide's read-only configuration.
#[test]
fn slides_write_config_requires_presentations_scope() {
    let deadline = || connectors_sdk::now_ms() + 30_000;
    let writes = Provider::with(|config| {
        config["auth"]["minimum_scopes"] = json!([WRITE_SCOPE]);
        config["auth"]["requested_scopes"] = json!(["openid", WRITE_SCOPE]);
    });
    let mut child = Child::spawn(&writes.selection()).unwrap();
    assert!(
        matches!(
            child.validate(PROFILE, &secret(), deadline()),
            Err(Failure::InsufficientScope)
        ),
        "a read-only grant validated a write configuration"
    );
    assert_eq!(writes.requests().len(), 1, "one token exchange");
    assert_eq!(writes.requests()[0].1, "/token");

    let reads = Provider::new();
    let mut child = Child::spawn(&reads.selection()).unwrap();
    child.validate(PROFILE, &secret(), deadline()).unwrap();
}

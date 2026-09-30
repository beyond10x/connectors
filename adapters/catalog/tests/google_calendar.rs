//! Google Calendar reads — the calendar list and events — through the catalog
//! provider.
//!
//! The shipped selection set is pinned by id and Discovery method id, resolves
//! against the committed bundle compiled from the projection of the pinned
//! Calendar v3 Discovery document, and is cited row by row in
//! `docs/catalog-google-calendar.md`. Each read runs through the provider child
//! against a disposable HTTPS fixture that serves the Calendar routes and an
//! OAuth token route on one host: the child exchanges the fixture refresh entry
//! for an access token, and the exact request (path, query,
//! `Authorization: Bearer …`) and the returned body are asserted. Both lists
//! walk two pages to the page carrying `nextSyncToken`, and an expired sync
//! token's `410` reaches the caller as `not_found`. The profile is the guide's
//! own, pointed at the fixture token route. The fixture secrets are fictional
//! and only ever compared, never printed. No live credential and no network.
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

/// The path the configured `api_base` carries: the projection's one server,
/// `https://www.googleapis.com/calendar/v3`, below its host.
const BASE: &str = "/calendar/v3";
const PROVIDER: &str = "google-calendar";
/// The bundle's auth profile, and the profile of the guide's configuration.
const PROFILE: &str = "google.oauth";
const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar.readonly";
/// Fictional OAuth material. The access token is what the fixture token route
/// issues and the only bearer the fixture Calendar routes accept.
const CLIENT_ID: &str = "fixture-client-id.apps.example.test";
const CLIENT_SECRET: &str = "fixture-client-secret-calendar";
const REFRESH_TOKEN: &str = "fixture-refresh-token-calendar";
const ACCESS_TOKEN: &str = "fixture-access-token-calendar";
/// The pinned Discovery document, relative to this crate.
const UPSTREAM: &str = "../google/upstream/calendar/calendar-api.json";
/// The committed projection, relative to this crate.
const PROJECTED: &str = "../google/generated/calendar.openapi.json";
/// The page-size ceiling the pinned `events.list` states only in the text of
/// its `maxResults` description ("can never be larger than 2500 events").
const EVENTS_MAX_RESULTS: u64 = 2500;

/// The shipped ids, their Discovery method id and the path the bundle records
/// (the Discovery path below the server path `/calendar/v3`). A renamed,
/// dropped or added id fails here.
const SHIPPED: [(&str, &str, &str); 3] = [
    (
        "calendarList.list",
        "calendar.calendarList.list",
        "/calendar/v3/users/me/calendarList",
    ),
    (
        "events.get",
        "calendar.events.get",
        "/calendar/v3/calendars/{calendarId}/events/{eventId}",
    ),
    (
        "events.list",
        "calendar.events.list",
        "/calendar/v3/calendars/{calendarId}/events",
    ),
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers/google-calendar/operations.json")).unwrap(),
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
fn engine() -> Engine {
    Engine::new(&committed_bundle(), BASE, &shipped()).unwrap()
}

#[test]
fn shipped_calendar_selections_are_exactly_the_three_reads() {
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
            operation_id.strip_prefix("calendar."),
            "`{id}` is not `{operation_id}` without `calendar.`"
        );
        assert_eq!(selection.effect, Effect::Read, "`{id}`");
        assert_eq!(selection.response, None, "`{id}` reads JSON");
        assert_eq!(engine.effect(id), Some(Effect::Read), "`{id}`");
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
    // Calendar has no `events.search` method: search is `events.list` with `q`.
    let absent = "calendar.events.search";
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

/// The bundle is the projection of the pinned document, and records it.
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
    let bundle = committed_bundle();
    assert_eq!(bundle.source.file_name, "calendar.openapi.json");
    assert_eq!(
        bundle.source.source_sha256,
        hex::encode(Sha256::digest(&committed))
    );
    let derivation = bundle.source.derivation.expect("the bundle's derivation");
    assert_eq!(derivation.from_file, "calendar-api.json");
    assert_eq!(derivation.from_sha256, digest);
    assert_eq!(derivation.from_bytes, bytes.len());
    assert_eq!(derivation.format, discovery::FORMAT);
    assert_eq!(derivation.projector, discovery::PROJECTOR);
    assert_eq!(json!(derivation.discovery_revision), pinned()["revision"]);
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    assert!(index.find(PROVIDER).is_some());
}

/// `events.list` bounds `maxResults` at 1–2500: the pinned document's own
/// `minimum` and the ceiling its description states. No other selection
/// carries a bound.
#[test]
fn only_events_list_bounds_max_results_at_the_documented_range() {
    let document = pinned();
    let max_results =
        &document["resources"]["events"]["methods"]["list"]["parameters"]["maxResults"];
    assert_eq!(max_results["minimum"], "1");
    assert!(max_results.get("maximum").is_none());
    assert!(
        max_results["description"]
            .as_str()
            .unwrap()
            .contains(&format!("never be larger than {EVENTS_MAX_RESULTS} events"))
    );
    for selection in shipped() {
        let written = serde_json::to_value(&selection).unwrap();
        let expected = if selection.id == "events.list" {
            json!({"maxResults": {"minimum": 1, "maximum": EVENTS_MAX_RESULTS}})
        } else {
            Value::Null
        };
        assert_eq!(written["bounds"], expected, "`{}`", selection.id);
    }
}

/// `eventTypes` is repeated in the pinned document, so the declared input
/// schema takes an array of string elements.
#[test]
fn events_list_declares_event_types_repeated() {
    let declaration = engine()
        .declarations(&[Effect::Read])
        .into_iter()
        .find(|o| o.id == "events.list")
        .unwrap();
    let event_types = &declaration.input_schema["properties"]["eventTypes"];
    assert!(
        event_types["type"]
            .as_array()
            .is_some_and(|types| types.contains(&json!("array"))),
        "{event_types}"
    );
    assert!(
        event_types["items"]["type"]
            .as_array()
            .is_some_and(|types| types.contains(&json!("string"))),
        "{event_types}"
    );
}

fn guide() -> String {
    fs::read_to_string(root().join("../../docs/catalog-google-calendar.md")).unwrap()
}

#[test]
fn guide_cites_each_operation_its_paging_and_its_deltas() {
    let guide = guide();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    for (id, operation_id, path, paging, end, deltas) in [
        (
            "calendarList.list",
            "calendar.calendarList.list",
            "/calendar/v3/users/me/calendarList",
            "`pageToken`, `maxResults`",
            "`nextPageToken` absent",
            "`syncToken`",
        ),
        (
            "events.list",
            "calendar.events.list",
            "/calendar/v3/calendars/{calendarId}/events",
            "`pageToken`, `maxResults`",
            "`nextPageToken` absent",
            "`syncToken`",
        ),
        (
            "events.get",
            "calendar.events.get",
            "/calendar/v3/calendars/{calendarId}/events/{eventId}",
            "single item",
            "n/a",
            "`events.list`",
        ),
    ] {
        let cited = rows.iter().any(|row| {
            row.starts_with(&format!("| `{id}` "))
                && row.contains(&format!("`{operation_id}`"))
                && row.contains(&format!("`GET {path}`"))
        });
        assert!(cited, "no row cites `{id}` as `{operation_id}` `{path}`");
        let paged = rows.iter().any(|row| {
            row.starts_with(&format!("| `{id}` "))
                && row.contains(paging)
                && row.contains(end)
                && row.contains(deltas)
        });
        assert!(paged, "no row states the paging of `{id}`");
    }
}

/// The guide states the time window, the sync-token cycle and the reset rule:
/// the parameters that cannot accompany a `syncToken`, and that a `410`
/// (`fullSyncRequired`) means discarding the token for a full walk without it.
#[test]
fn guide_documents_time_windows_sync_tokens_and_the_full_sync_reset() {
    let guide = guide();
    for term in [
        "`timeMin`",
        "`timeMax`",
        "`updatedMin`",
        "`singleEvents`",
        "`eventTypes`",
        "`syncToken`",
        "`nextSyncToken`",
        "`410`",
        "`fullSyncRequired`",
        "`not_found`",
        "`rate_limited`",
        "2500",
    ] {
        assert!(guide.contains(term), "the guide does not state {term}");
    }
    // Discovery names the parameters that cannot accompany `syncToken`.
    let sync = pinned()["resources"]["events"]["methods"]["list"]["parameters"]["syncToken"]
        ["description"]
        .as_str()
        .unwrap()
        .to_owned();
    for excluded in [
        "iCalUID",
        "orderBy",
        "privateExtendedProperty",
        "q",
        "sharedExtendedProperty",
        "timeMin",
        "timeMax",
        "updatedMin",
    ] {
        assert!(sync.contains(&format!("- {excluded}")), "{excluded}");
        assert!(
            guide.contains(&format!("`{excluded}`")),
            "the guide does not name `{excluded}` as excluded beside `syncToken`"
        );
    }
}

/// The guide's configuration example for this provider.
fn documented_config() -> Value {
    let guide = guide();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains(&format!("\"provider\": \"{PROVIDER}\"")))
        .expect("the documented google-calendar configuration");
    serde_json::from_str::<Value>(example).unwrap()
}

/// The guide configures the bundle's profile as an `oauth2_refresh` profile
/// against Google's token endpoint, with the Calendar read-only scope, and the
/// projection's server as the API base.
#[test]
fn guide_documents_the_oauth_refresh_configuration() {
    let config = documented_config();
    assert_eq!(config["api_base"], "https://www.googleapis.com/calendar/v3");
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
    assert_eq!(auth["minimum_scopes"], json!([CALENDAR_SCOPE]));
    assert!(
        auth["requested_scopes"]
            .as_array()
            .unwrap()
            .contains(&json!(CALENDAR_SCOPE))
    );
    assert!(auth.get("token_ca_file").is_none());
    // Every shipped read accepts that scope, per the pinned document.
    let document = pinned();
    for (resource, method) in [
        ("calendarList", "list"),
        ("events", "list"),
        ("events", "get"),
    ] {
        let method = &document["resources"][resource]["methods"][method];
        assert!(
            method["scopes"]
                .as_array()
                .unwrap()
                .contains(&json!(CALENDAR_SCOPE)),
            "`{}`",
            method["id"]
        );
    }
}

/// Method, route with query, and `Authorization` header of each fixture request.
type Requests = Arc<Mutex<Vec<(String, String, Option<String>)>>>;

fn calendar_entry(id: &str, summary: &str) -> Value {
    json!({"kind": "calendar#calendarListEntry", "id": id, "summary": summary,
           "accessRole": "reader", "timeZone": "Europe/Berlin"})
}
fn event(id: &str, summary: &str) -> Value {
    json!({"kind": "calendar#event", "id": id, "status": "confirmed", "summary": summary,
           "eventType": "default",
           "start": {"dateTime": "2026-09-21T10:00:00+02:00"},
           "end": {"dateTime": "2026-09-21T11:00:00+02:00"},
           "updated": "2026-09-20T08:00:00.000Z"})
}
fn events_page(items: Vec<Value>, token: (&str, &str)) -> Value {
    let mut page = json!({"kind": "calendar#events", "summary": "Fixture calendar",
                          "timeZone": "Europe/Berlin", "items": items});
    page[token.0] = json!(token.1);
    page
}

/// The body of Google's answer to an expired sync token.
fn full_sync_required() -> Value {
    json!({"error": {"code": 410, "message": "Sync token is no longer valid, a full sync is required.",
                     "errors": [{"domain": "calendar", "reason": "fullSyncRequired",
                                 "message": "Sync token is no longer valid, a full sync is required."}]}})
}

/// The recorded Calendar answers the fixture serves, keyed by route and paging
/// position: a status and a JSON body. `None` for anything else, which the
/// fixture answers 404.
fn answer(target: &str) -> Option<(u16, Value)> {
    let (route, query) = target.split_once('?').unwrap_or((target, ""));
    let has = |pair: &str| query.split('&').any(|p| p == pair);
    let ok = |value: Value| Some((200, value));
    match route {
        "/calendar/v3/users/me/calendarList" if has("pageToken=fixture-calendars-page-2") => {
            ok(json!({"kind": "calendar#calendarList",
                      "nextSyncToken": "fixture-calendars-sync-1",
                      "items": [calendar_entry("team@group.example.test", "Team")]}))
        }
        "/calendar/v3/users/me/calendarList" if has("maxResults=1") => {
            ok(json!({"kind": "calendar#calendarList",
                      "nextPageToken": "fixture-calendars-page-2",
                      "items": [calendar_entry("reader@example.test", "Reader")]}))
        }
        "/calendar/v3/users/me/calendarList" => ok(json!({"kind": "calendar#calendarList",
            "nextSyncToken": "fixture-calendars-sync-1", "items": []})),
        "/calendar/v3/calendars/primary/events" if has("syncToken=fixture-expired-sync") => {
            Some((410, full_sync_required()))
        }
        "/calendar/v3/calendars/primary/events" if has("syncToken=fixture-events-sync-1") => {
            ok(events_page(
                vec![event("fixture-event-4", "moved")],
                ("nextSyncToken", "fixture-events-sync-2"),
            ))
        }
        "/calendar/v3/calendars/primary/events" if has("pageToken=fixture-events-page-2") => {
            ok(events_page(
                vec![event("fixture-event-3", "third")],
                ("nextSyncToken", "fixture-events-sync-1"),
            ))
        }
        "/calendar/v3/calendars/primary/events" if has("maxResults=2") => ok(events_page(
            vec![
                event("fixture-event-1", "first"),
                event("fixture-event-2", "second"),
            ],
            ("nextPageToken", "fixture-events-page-2"),
        )),
        // Any other page size: one last page.
        "/calendar/v3/calendars/primary/events" => ok(events_page(
            vec![],
            ("nextSyncToken", "fixture-events-sync-1"),
        )),
        "/calendar/v3/calendars/primary/events/fixture-event-1" => {
            ok(event("fixture-event-1", "first"))
        }
        _ => None,
    }
}
/// The recorded JSON body of `target`.
fn recorded(target: &str) -> Value {
    answer(target).unwrap().1
}

/// An unsigned JWT as the token answer's `id_token`.
fn id_token() -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let part = |value: &Value| URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap());
    format!(
        "{}.{}.{}",
        part(&json!({"alg": "RS256", "typ": "JWT", "kid": "fixture"})),
        part(
            &json!({"iss": "https://accounts.google.com", "aud": CLIENT_ID, "azp": CLIENT_ID,
                     "sub": "110000000000000000003", "iat": 1, "exp": 4_000_000_000_u64})
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
                    let (status, answer) = if method == "POST" && target == "/token" {
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
                            (
                                200,
                                json!({
                                    "access_token": ACCESS_TOKEN, "token_type": "Bearer",
                                    "expires_in": 3600,
                                    "scope": format!("openid {CALENDAR_SCOPE}"),
                                    "id_token": id_token()}),
                            )
                        } else {
                            (400, json!({"error": "invalid_grant"}))
                        }
                    } else if authorization.as_deref() != Some(&*format!("Bearer {ACCESS_TOKEN}"))
                    {
                        (401, json!({"error": {"code": 401, "message": "fixture refusal"}}))
                    } else {
                        match (method.as_str(), answer(&target)) {
                            ("GET", Some(answer)) => answer,
                            _ => (404, json!({"error": {"code": 404, "message": "no fixture"}})),
                        }
                    };
                    observed
                        .lock()
                        .unwrap()
                        .push((method, target, authorization));
                    let answer = serde_json::to_vec(&answer).unwrap();
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
        config["instance"] = json!("fixture-google-calendar");
        config["bundle_directory"] = json!(root_path("generated/bundles"));
        config["operations_file"] = json!(root_path("providers/google-calendar/operations.json"));
        config["api_base"] = json!(format!("https://localhost:{}/calendar/v3", address.port()));
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
            instance_id: "fixture-google-calendar".into(),
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
    /// The Calendar requests (everything but the token route), as route with
    /// query.
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
            "calendarList.list",
            json!({"maxResults": 1}),
            "/calendar/v3/users/me/calendarList?maxResults=1",
        ),
        (
            "events.list",
            json!({"calendarId": "primary", "maxResults": 2, "singleEvents": true,
                   "timeMin": "2026-09-21T00:00:00Z", "timeMax": "2026-09-28T00:00:00Z"}),
            "/calendar/v3/calendars/primary/events?maxResults=2&singleEvents=true\
             &timeMax=2026-09-28T00%3A00%3A00Z&timeMin=2026-09-21T00%3A00%3A00Z",
        ),
        (
            "events.get",
            json!({"calendarId": "primary", "eventId": "fixture-event-1"}),
            "/calendar/v3/calendars/primary/events/fixture-event-1?",
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
        assert_eq!(result["provenance"]["instance"], "fixture-google-calendar");
        // The engine re-serialises the body, so the recorded answer is
        // compared as JSON, not as bytes.
        assert_eq!(result["body"], recorded(expected), "`{operation}` body");
    }
    // The bearer came from the token route: one exchange, before any Calendar
    // request; the cached token served every read after it.
    let requests = provider.requests();
    let token: Vec<_> = requests.iter().filter(|(_, t, _)| t == "/token").collect();
    assert_eq!(token.len(), 1, "token exchanges");
    assert_eq!(token[0].0, "POST");
    assert!(
        token[0].2.is_none(),
        "credential header on the token request"
    );
    assert_eq!(requests[0].1, "/token", "the exchange comes first");
}

/// `eventTypes` is repeated: two types are two `eventTypes` pairs, in the
/// order given.
#[test]
fn events_list_repeats_event_types() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    invoke(
        &mut child,
        "events.list",
        json!({"calendarId": "primary", "eventTypes": ["default", "focusTime"]}),
    );
    assert_eq!(
        provider.api_targets(),
        ["/calendar/v3/calendars/primary/events?eventTypes=default&eventTypes=focusTime"]
    );
}

/// The guide: the engine does not check an `eventTypes` value against the
/// pinned document's enumeration (`connectors_catalog::inventory::Parameter`
/// records no `enum`); an unknown type is sent, and Google answers it.
#[test]
fn events_list_sends_an_event_type_outside_the_enumeration() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    invoke(
        &mut child,
        "events.list",
        json!({"calendarId": "primary", "eventTypes": ["default", "fixture-unknown"]}),
    );
    assert_eq!(
        provider.api_targets(),
        ["/calendar/v3/calendars/primary/events?eventTypes=default&eventTypes=fixture-unknown"]
    );
}

/// Walk one list from `input` following `nextPageToken`, and return the ids
/// seen, the page count and the `nextSyncToken` of the last page.
fn walk(child: &mut Child, operation: &str, mut input: Value) -> (Vec<String>, usize, String) {
    let mut ids = Vec::new();
    let mut pages = 0;
    loop {
        let body = invoke(child, operation, input.clone())["body"].clone();
        pages += 1;
        for item in body["items"].as_array().unwrap() {
            ids.push(item["id"].as_str().unwrap().to_owned());
        }
        match body.get("nextPageToken").and_then(Value::as_str) {
            Some(token) => {
                assert!(body.get("nextSyncToken").is_none());
                input["pageToken"] = json!(token);
            }
            None => {
                let sync = body["nextSyncToken"]
                    .as_str()
                    .expect("the last page carries nextSyncToken");
                return (ids, pages, sync.to_owned());
            }
        }
        assert!(pages < 3, "`{operation}` did not stop");
    }
}

#[test]
fn calendar_list_walks_two_pages_to_next_sync_token() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let (ids, pages, sync) = walk(&mut child, "calendarList.list", json!({"maxResults": 1}));
    assert_eq!(pages, 2);
    assert_eq!(sync, "fixture-calendars-sync-1");
    assert_eq!(ids, ["reader@example.test", "team@group.example.test"]);
    assert_eq!(
        provider.api_targets(),
        [
            "/calendar/v3/users/me/calendarList?maxResults=1",
            "/calendar/v3/users/me/calendarList?maxResults=1&pageToken=fixture-calendars-page-2",
        ]
    );
}

/// The full walk ends on the page carrying `nextSyncToken`; sending it as
/// `syncToken` reads only what changed since, with the next token.
#[test]
fn events_list_walks_two_pages_to_next_sync_token_and_reads_the_delta() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let (ids, pages, sync) = walk(
        &mut child,
        "events.list",
        json!({"calendarId": "primary", "maxResults": 2, "singleEvents": true}),
    );
    assert_eq!(pages, 2);
    assert_eq!(sync, "fixture-events-sync-1");
    assert_eq!(
        ids,
        ["fixture-event-1", "fixture-event-2", "fixture-event-3"]
    );
    let (delta, pages, next) = walk(
        &mut child,
        "events.list",
        json!({"calendarId": "primary", "singleEvents": true, "syncToken": sync}),
    );
    assert_eq!((delta, pages), (vec!["fixture-event-4".to_owned()], 1));
    assert_eq!(next, "fixture-events-sync-2");
    assert_eq!(
        provider.api_targets(),
        [
            "/calendar/v3/calendars/primary/events?maxResults=2&singleEvents=true",
            "/calendar/v3/calendars/primary/events?maxResults=2&pageToken=fixture-events-page-2\
             &singleEvents=true",
            "/calendar/v3/calendars/primary/events?singleEvents=true&syncToken=fixture-events-sync-1",
        ]
    );
}

/// An expired sync token is Google's `410` `fullSyncRequired`; it reaches the
/// caller as `not_found`, the refusal the guide's reset rule starts from.
#[test]
fn an_expired_sync_token_is_refused_as_not_found() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let outcome = attempt(
        &mut child,
        "events.list",
        &json!({"calendarId": "primary", "syncToken": "fixture-expired-sync"}),
    );
    assert!(
        matches!(outcome, Err(Failure::ProviderNotFound)),
        "{outcome:?}"
    );
    assert_eq!(
        provider.api_targets(),
        ["/calendar/v3/calendars/primary/events?syncToken=fixture-expired-sync"]
    );
}

/// `maxResults` 0 and 2501 are refused as `invalid_input` with nothing sent,
/// as numbers and as strings; 1 and 2500 are sent.
#[test]
fn events_list_max_results_bounds() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let first = json!({"calendarId": "primary", "maxResults": 2});
    // Exchange the token first, so "nothing sent" counts every request.
    invoke(&mut child, "events.list", first.clone());
    let mut escaped = Vec::new();
    for size in [json!(0), json!(2501), json!("0"), json!("2501")] {
        let mut input = first.clone();
        input["maxResults"] = size.clone();
        let before = provider.requests().len();
        let outcome = attempt(&mut child, "events.list", &input);
        let sent = provider.requests().len() != before;
        if sent || !matches!(outcome, Err(Failure::InvalidInput)) {
            escaped.push(format!("maxResults={size} sent={sent} {outcome:?}"));
        }
    }
    assert!(
        escaped.is_empty(),
        "not refused before any request: {escaped:#?}"
    );
    for size in [1, EVENTS_MAX_RESULTS] {
        let mut input = first.clone();
        input["maxResults"] = json!(size);
        let before = provider.api_targets().len();
        invoke(&mut child, "events.list", input);
        let targets = provider.api_targets();
        assert_eq!(targets.len(), before + 1, "{size}");
        assert_eq!(
            targets[before],
            format!("/calendar/v3/calendars/primary/events?maxResults={size}"),
        );
    }
}

//! Second adversary pass against the Google Calendar and Gmail read selections,
//! aimed at the corrections of the first: the Gmail resync order, the `fields`
//! warnings, the Calendar reset rule's re-check and the new
//! `users.messages.attachments.get` selection. Each case reads the text an
//! agent reads (the guide, or the description `operations describe` returns)
//! against the pinned Discovery document, and where the claim is about bytes,
//! drives the provider child against a disposable HTTPS fixture. Fixture
//! secrets are fictional and only ever compared, never printed. No network.
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

const PROVIDER: &str = "google-gmail";
const GMAIL_SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";
const CLIENT_ID: &str = "fixture-client-id.apps.example.test";
const CLIENT_SECRET: &str = "fixture-client-secret-gmail-adv2";
const REFRESH_TOKEN: &str = "fixture-refresh-token-gmail-adv2";
const ACCESS_TOKEN: &str = "fixture-access-token-gmail-adv2";
/// The largest part `size` this pass reads successfully, and one it does not:
/// both under the 4 MiB the descriptions name.
const FITS: usize = 3 * 1024 * 1024 - 1024;
const UNDER_NAMED_LIMIT: usize = 3 * 1024 * 1024 + 512 * 1024;
const _: () = assert!(UNDER_NAMED_LIMIT < connectors_core::RESPONSE_LIMIT);

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn guide(name: &str) -> String {
    fs::read_to_string(root().join(format!("../../docs/{name}"))).unwrap()
}
/// The paragraph of `text` that starts with `lead`; a list item (a `lead`
/// starting with `- `) ends at the next item as well.
fn paragraph(text: &str, lead: &str) -> String {
    let start = text.find(lead).unwrap_or_else(|| panic!("no `{lead}`"));
    let rest = &text[start..];
    let mut end = rest.find("\n\n").unwrap_or(rest.len());
    if lead.starts_with("- ") {
        end = end.min(rest.find("\n- ").unwrap_or(rest.len()));
    }
    rest[..end].to_owned()
}
fn description(provider: &str, id: &str) -> String {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join(format!("providers/{provider}/operations.json"))).unwrap(),
    )
    .unwrap();
    file["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == id)
        .unwrap_or_else(|| panic!("no selection `{id}`"))["description"]
        .as_str()
        .unwrap()
        .to_owned()
}
fn pinned_calendar() -> Value {
    serde_json::from_slice(
        &fs::read(root().join("../google/upstream/calendar/calendar-api.json")).unwrap(),
    )
    .unwrap()
}
fn pinned_gmail() -> Value {
    serde_json::from_slice(
        &fs::read(root().join("../google/upstream/gmail/gmail-api.json")).unwrap(),
    )
    .unwrap()
}
/// Every `items(...)` group in the `fields` examples of `text`.
fn item_groups(text: &str) -> Vec<String> {
    let mut groups = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("items(") {
        let tail = &rest[start + "items(".len()..];
        let end = tail.find(')').unwrap_or(tail.len());
        groups.push(tail[..end].to_owned());
        rest = &tail[end..];
    }
    groups
}

/// The pinned `MessagePartBody.size` is the "Number of bytes for the message
/// part data (encoding notwithstanding)", and `data` is that part "as a
/// base64url encoded string" — four bytes on the wire for every three of
/// `size`. The `users.messages.attachments.get` description and the guide's
/// "Large messages and attachments" route both bound the read by "the
/// engine's 4 MiB response limit" and nothing else, so an agent holding a
/// `full` message compares a part's `size` with 4 MiB. A part of 3.5 MiB is
/// under it and is refused as `capacity`: the ceiling on `size` is about
/// 3 MiB, and a part above it cannot be read through this provider at all.
/// Measured first (a 3 MiB − 1 KiB part is read, a 3.5 MiB part is refused);
/// then the texts an agent reads must state the ceiling on `size`.
#[test]
fn an_attachment_under_the_named_limit_is_refused_and_the_texts_state_the_real_ceiling() {
    let pinned = pinned_gmail();
    let size = pinned["schemas"]["MessagePartBody"]["properties"]["size"]["description"]
        .as_str()
        .unwrap();
    assert!(size.contains("encoding notwithstanding"), "{size}");

    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let read = |child: &mut Child, id: &str| {
        attempt(
            child,
            "users.messages.attachments.get",
            &json!({"userId": "me", "messageId": "fixture-message-1", "id": id}),
        )
    };
    let fits = read(&mut child, "fixture-attachment-fits")
        .unwrap_or_else(|failure| panic!("a {FITS}-byte part failed: {failure:?}"));
    let fits: Value = serde_json::from_slice(&fits).unwrap();
    assert_eq!(fits["body"]["size"], json!(FITS));
    let refused = read(&mut child, "fixture-attachment-under-named-limit");
    assert!(
        matches!(refused, Err(Failure::Capacity | Failure::ProviderCapacity)),
        "a {UNDER_NAMED_LIMIT}-byte part: {refused:?}"
    );

    let route = paragraph(
        &guide("catalog-google-gmail.md"),
        "- **Large messages and attachments.**",
    );
    let mut silent = Vec::new();
    for (source, text) in [
        (
            "users.messages.attachments.get description",
            description(PROVIDER, "users.messages.attachments.get"),
        ),
        (
            "users.messages.get description",
            description(PROVIDER, "users.messages.get"),
        ),
        (
            "docs/catalog-google-gmail.md Large messages and attachments",
            route,
        ),
    ] {
        if !text.contains("3 MiB") {
            silent.push(format!("{source}: {text}"));
        }
    }
    assert!(
        silent.is_empty(),
        "a part of {UNDER_NAMED_LIMIT} bytes (`size`, under the 4 MiB named) is refused as \
         capacity because its base64url `data` is 4/3 larger; these texts bound the read only \
         by 4 MiB and never state that a part over about 3 MiB cannot be read: {silent:#?}"
    );
}

/// The corrected reset rule: "first re-check the calendar with
/// `calendarList.list`: if it is gone, drop its mirror", and the `events.list`
/// description: "re-check calendarId with calendarList.list". The pinned
/// document says `primary` is a keyword ("If you want to access the primary
/// calendar of the currently logged in user, use the \"primary\" keyword"), and
/// the guide itself says a `calendarId` is "a calendar's id from
/// `calendarList.list`, or `primary`": no calendar list entry has the id
/// `primary`. And `calendarList.list` leaves hidden entries out unless asked
/// (`showHidden`: "The default is False"). So on the commonest sync — the
/// `primary` calendar the description recommends — and on any hidden
/// calendar, a sync-token expiry followed by the literal re-check finds no
/// entry, and the guide says to drop the mirror instead of walking again.
#[test]
fn the_calendar_reset_recheck_does_not_misjudge_primary_or_hidden_calendars() {
    let pinned = pinned_calendar();
    let calendar_id = pinned["resources"]["events"]["methods"]["list"]["parameters"]["calendarId"]
        ["description"]
        .as_str()
        .unwrap();
    assert!(
        calendar_id.contains("use the \"primary\" keyword"),
        "{calendar_id}"
    );
    let show_hidden = pinned["resources"]["calendarList"]["methods"]["list"]["parameters"]
        ["showHidden"]["description"]
        .as_str()
        .unwrap();
    assert!(
        show_hidden.contains("The default is False"),
        "{show_hidden}"
    );

    let rule = paragraph(&guide("catalog-google-calendar.md"), "**Reset rule.**");
    let events = description("google-calendar", "events.list");
    let mut misjudged = Vec::new();
    for (source, text) in [
        ("docs/catalog-google-calendar.md reset rule", rule.as_str()),
        ("events.list description", events.as_str()),
    ] {
        // The reset instruction only: from the first `not_found` on.
        let start = text
            .find("not_found")
            .unwrap_or_else(|| panic!("{source}: no not_found"));
        let reset = &text[start..];
        if reset.contains("calendarList.list")
            && !(reset.contains("primary") && reset.contains("showHidden"))
        {
            misjudged.push(format!("{source}: {reset}"));
        }
    }
    assert!(
        misjudged.is_empty(),
        "the re-check looks the calendarId up in calendarList.list, which never lists \
         `primary` by that id and omits hidden calendars by default, so an expired token on \
         either is judged a gone calendar: {misjudged:#?}"
    );
}

/// The `fields` examples the correction added are the model an agent copies
/// for a sync walk: `items(id,summary,start,end)` for `events.list`,
/// `items(id,summary)` for `calendarList.list` and the guide. A delta carries
/// deletions as items, and the pinned document marks them only by field:
/// `Event.status` "cancelled" ("Clients should remove their locally synced
/// copies ... Deleted events are only guaranteed to have the id field
/// populated"), `CalendarListEntry.deleted`. A delta narrowed by those
/// examples returns a deleted event as a bare `{"id": ...}` the agent upserts,
/// so the mirror keeps every deleted event.
#[test]
fn the_sync_fields_examples_keep_the_deletion_markers() {
    let pinned = pinned_calendar();
    let status = pinned["schemas"]["Event"]["properties"]["status"]["description"]
        .as_str()
        .unwrap();
    assert!(
        status.contains("Clients should remove their locally synced copies"),
        "{status}"
    );
    assert!(
        pinned["schemas"]["CalendarListEntry"]["properties"]["deleted"].is_object(),
        "CalendarListEntry.deleted"
    );
    let fields = paragraph(
        &guide("catalog-google-calendar.md"),
        "- **`fields` and the end conditions.**",
    );
    // The paragraph gives one example per list; each keeps its own list's marker.
    let (events_example, calendar_example) = fields
        .split_once("`events.list`, and")
        .map(|(events, calendar)| (events.to_owned(), calendar.to_owned()))
        .expect("one fields example for events.list, then one for calendarList.list");
    let mut dropped = Vec::new();
    for (source, text, marker) in [
        (
            "events.list description",
            description("google-calendar", "events.list"),
            "status",
        ),
        (
            "calendarList.list description",
            description("google-calendar", "calendarList.list"),
            "deleted",
        ),
        (
            "docs/catalog-google-calendar.md fields (events)",
            events_example,
            "status",
        ),
        (
            "docs/catalog-google-calendar.md fields (calendar list)",
            calendar_example,
            "deleted",
        ),
    ] {
        let groups = item_groups(&text);
        assert!(!groups.is_empty(), "{source}: no items(...) example");
        for group in groups {
            if !group.split(',').any(|field| field.trim() == marker) {
                dropped.push(format!("{source}: items({group}) without {marker}"));
            }
        }
    }
    assert!(
        dropped.is_empty(),
        "the fields examples for a sync walk drop the field a delta marks a deletion with: \
         {dropped:#?}"
    );
}

/// The pinned `syncToken`: "it is not allowed to set showDeleted to False"
/// (`events.list`) and "not allowed to set showDeleted neither showHidden to
/// False" (`calendarList.list`). The first pass put this in the guide only.
/// The descriptions `operations describe` returns still omit it, and the
/// `calendarList.list` description tells the caller to send the token "with
/// the same other parameters", so a walk that set either to `false` has a
/// delta Google refuses.
#[test]
fn the_sync_token_descriptions_name_the_show_deleted_restriction() {
    let pinned = pinned_calendar();
    let list = pinned["resources"]["calendarList"]["methods"]["list"]["parameters"]["syncToken"]
        ["description"]
        .as_str()
        .unwrap();
    assert!(
        list.contains("not allowed to set showDeleted neither showHidden to False"),
        "{list}"
    );
    let mut missing = Vec::new();
    for (id, terms) in [
        ("events.list", &["showDeleted"][..]),
        ("calendarList.list", &["showDeleted", "showHidden"][..]),
    ] {
        let text = description("google-calendar", id);
        for term in terms {
            if !text.contains(term) {
                missing.push(format!("{id}: {term}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "the sync-token descriptions omit the parameters that cannot be false beside \
         syncToken: {missing:?}"
    );
}

/// The correction fixed the order in the reset rule ("first read a new
/// baseline historyId from users.getProfile, then do a full sync") because a
/// baseline read after a walk loses the changes made during it. The first
/// sync is the same walk and the same race, and the guide's instruction for it
/// — "Take a baseline once: `users.getProfile` returns the mailbox's current
/// `historyId`" — and the `users.getProfile` description order nothing
/// against the first walk.
#[test]
fn the_first_gmail_baseline_is_ordered_before_the_first_walk() {
    let deltas = paragraph(&guide("catalog-google-gmail.md"), "Take a baseline once");
    let profile = description(PROVIDER, "users.getProfile");
    let ordered = |text: &str| text.contains("before") && text.contains("walk");
    assert!(
        ordered(&deltas) || ordered(&profile),
        "neither the first-baseline paragraph nor the users.getProfile description orders the \
         baseline before the first full walk: {deltas:?} / {profile:?}"
    );
}

// --- The provider child against a disposable HTTPS fixture. ---

type Requests = Arc<Mutex<Vec<(String, String, Option<String>)>>>;

/// A `users.messages.attachments.get` answer for a part of `size` bytes, as
/// Gmail sends it: `size` and the base64url `data`.
fn attachment(size: usize) -> Vec<u8> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    serde_json::to_vec(&json!({"size": size, "data": URL_SAFE_NO_PAD.encode(vec![0_u8; size])}))
        .unwrap()
}
fn answer(target: &str) -> Option<Vec<u8>> {
    let (route, _) = target.split_once('?').unwrap_or((target, ""));
    match route {
        "/gmail/v1/users/me/messages/fixture-message-1/attachments/fixture-attachment-fits" => {
            Some(attachment(FITS))
        }
        "/gmail/v1/users/me/messages/fixture-message-1/attachments/fixture-attachment-under-named-limit" => {
            Some(attachment(UNDER_NAMED_LIMIT))
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
                     "sub": "110000000000000000009", "iat": 1, "exp": 4_000_000_000_u64})
        ),
        URL_SAFE_NO_PAD.encode(b"fixture-signature")
    )
}
fn documented_config() -> Value {
    let guide = guide("catalog-google-gmail.md");
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
                                "scope": format!("openid {GMAIL_SCOPE}"),
                                "id_token": id_token()});
                            (200, serde_json::to_vec(&grant).unwrap())
                        } else {
                            let refusal = json!({"error": "invalid_grant"});
                            (400, serde_json::to_vec(&refusal).unwrap())
                        }
                    } else if authorization.as_deref() != Some(&*format!("Bearer {ACCESS_TOKEN}"))
                    {
                        (401, serde_json::to_vec(&json!({"error": {"code": 401}})).unwrap())
                    } else {
                        match (method.as_str(), answer(&target)) {
                            ("GET", Some(body)) => (200, body),
                            _ => (
                                404,
                                serde_json::to_vec(&json!({"error": {"code": 404}})).unwrap(),
                            ),
                        }
                    };
                    observed
                        .lock()
                        .unwrap()
                        .push((method, target, authorization));
                    let head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answer.len()
                    );
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&answer).await;
                    // A multi-MiB answer can still sit in the TLS session when the
                    // stream drops; flush it and send close_notify, or the client
                    // reads a truncated body as `Unavailable`.
                    let _ = stream.flush().await;
                    let _ = stream.shutdown().await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let origin = format!("{}{}", concat!("https", "://localhost:"), address.port());
        let mut config = documented_config();
        config["instance"] = json!("fixture-google-gmail-adv2");
        config["bundle_directory"] = json!(root_path("generated/bundles"));
        config["operations_file"] = json!(root_path("providers/google-gmail/operations.json"));
        config["api_base"] = json!(origin);
        config["ca_file"] = json!(ca);
        config["auth"]["token_url"] = json!(format!("{origin}/token"));
        config["auth"]["token_ca_file"] = json!(ca);
        let path = directory.join("catalog.json");
        private(&path, &serde_json::to_vec(&config).unwrap());
        drop(requests);
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config: path,
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
            instance_id: "fixture-google-gmail-adv2".into(),
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

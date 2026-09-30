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
//!
//! The three event writes run through the same child over private protocol
//! two, under the guide's write instance: the host's prepare and commit, the
//! declared `events.get` preflight that compares the pinned `etag`, and the
//! exact request and body. The approval binding is exercised with the host's
//! own approval signer and verifier against a subject built from the child's
//! descriptor; the signing key is the public RFC 8032 section 7.1 test vector,
//! never a deployment key.
use connectors_catalog::{bundle, discovery};
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::local::{
    approvals,
    config::{Adapter, Executable, Restart, Startup},
    filesystem, mutations,
    runtime::{Bootstrap, Child, Failure, PrivateProtocol, WriteEffect, WriteResult},
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
/// The scope the three event writes need, per the pinned document.
const WRITE_SCOPE: &str = "https://www.googleapis.com/auth/calendar.events";
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
/// The ceiling the pinned `calendarList.list` states the same way ("can never be
/// larger than 250 entries").
const CALENDARS_MAX_RESULTS: u64 = 250;

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
/// The shipped writes: id, Discovery method id, method and the path the bundle
/// records.
const WRITES: [(&str, &str, &str, &str); 3] = [
    (
        "events.delete",
        "calendar.events.delete",
        "delete",
        "/calendar/v3/calendars/{calendarId}/events/{eventId}",
    ),
    (
        "events.insert",
        "calendar.events.insert",
        "post",
        "/calendar/v3/calendars/{calendarId}/events",
    ),
    (
        "events.patch",
        "calendar.events.patch",
        "patch",
        "/calendar/v3/calendars/{calendarId}/events/{eventId}",
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
fn shipped_calendar_selections_are_the_three_reads_and_three_writes() {
    let selections = shipped();
    let bundle = committed_bundle();
    assert_eq!(bundle.auth_profile, PROFILE);
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let ids = |effects: &[Effect]| {
        let mut declared: Vec<String> = engine
            .declarations(effects)
            .into_iter()
            .map(|o| o.id)
            .collect();
        declared.sort();
        declared
    };
    let reads: Vec<&str> = SHIPPED.iter().map(|(id, _, _)| *id).collect();
    let writes: Vec<&str> = WRITES.iter().map(|(id, _, _, _)| *id).collect();
    assert_eq!(ids(&[Effect::Read]), reads);
    assert_eq!(ids(&[Effect::Write]), writes);
    assert_eq!(selections.len(), reads.len() + writes.len());
    for (id, operation_id, method, path) in WRITES {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(Some(id), operation_id.strip_prefix("calendar."), "`{id}`");
        assert_eq!(selection.effect, Effect::Write, "`{id}`");
        assert_eq!(engine.effect(id), Some(Effect::Write), "`{id}`");
        assert_eq!(selection.response, None, "`{id}`");
        assert!(selection.bounds.is_empty(), "`{id}` bounds");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the projection lacks `{operation_id}`"));
        assert_eq!(operation.method, method, "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
        // A create has nothing to compare before it exists; patch and delete
        // read the event first and require it to be confirmed and its `etag`
        // to be the pinned one.
        let guard = serde_json::to_value(&selection.guard).unwrap();
        let expected = if id == "events.insert" {
            Value::Null
        } else {
            json!({
                "preflight": {"operation_id": "calendar.events.get",
                              "values": {"calendarId": "calendarId", "eventId": "eventId"},
                              "checks": [{"pointer": "/status", "expect": {"literal": "confirmed"}},
                                         {"pointer": "/etag", "expect": {"input": "etag"}}]},
                "postflight": {"checks": []}})
        };
        assert_eq!(guard, expected, "`{id}` guard");
        // The pin is a declared, required input; Google is never sent it.
        let declaration = engine
            .declarations(&[Effect::Write])
            .into_iter()
            .find(|o| o.id == id)
            .unwrap();
        assert_eq!(declaration.profile, "mutation", "`{id}`");
        let required = &declaration.input_schema["required"];
        assert_eq!(
            required.as_array().unwrap().contains(&json!("etag")),
            id != "events.insert",
            "`{id}` requires {required}"
        );
        // Every write names whom Google emails: `sendUpdates` is required.
        assert_eq!(selection.required, ["sendUpdates"], "`{id}` required");
        assert!(
            required.as_array().unwrap().contains(&json!("sendUpdates")),
            "`{id}` requires {required}"
        );
        assert!(
            operation.parameters.iter().all(|p| p.name != "etag"),
            "`{operation_id}` declares an `etag` parameter"
        );
    }
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

/// Both lists bound `maxResults`: the pinned document's own `minimum` and the
/// ceiling its description states, 2500 for `events.list` and 250 for
/// `calendarList.list`. `events.get` carries no bound.
#[test]
fn both_lists_bound_max_results_at_the_documented_range() {
    let document = pinned();
    for (resource, noun, maximum) in [
        ("events", "events", EVENTS_MAX_RESULTS),
        ("calendarList", "entries", CALENDARS_MAX_RESULTS),
    ] {
        let max_results =
            &document["resources"][resource]["methods"]["list"]["parameters"]["maxResults"];
        assert_eq!(max_results["minimum"], "1", "{resource}");
        assert!(max_results.get("maximum").is_none(), "{resource}");
        assert!(
            max_results["description"]
                .as_str()
                .unwrap()
                .contains(&format!("never be larger than {maximum} {noun}")),
            "{resource}"
        );
    }
    for selection in shipped() {
        let written = serde_json::to_value(&selection).unwrap();
        let expected = match selection.id.as_str() {
            "events.list" => json!({"maxResults": {"minimum": 1, "maximum": EVENTS_MAX_RESULTS}}),
            "calendarList.list" => {
                json!({"maxResults": {"minimum": 1, "maximum": CALENDARS_MAX_RESULTS}})
            }
            _ => Value::Null,
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
            "`pageToken`, `maxResults` (1–250)",
            "`nextPageToken` absent",
            "`syncToken`",
        ),
        (
            "events.list",
            "calendar.events.list",
            "/calendar/v3/calendars/{calendarId}/events",
            "`pageToken`, `maxResults` (1–2500)",
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

/// The pinned document: beside `syncToken`, "All other query parameters should
/// be the same as for the initial synchronization to avoid undefined
/// behavior". The `events.list` description an agent reads says a delta
/// repeats the full walk's other parameters, and does not offer a windowed
/// full walk followed by an unwindowed delta.
#[test]
fn events_list_description_keeps_the_full_walk_parameters_for_the_delta() {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join("providers/google-calendar/operations.json")).unwrap(),
    )
    .unwrap();
    let description = file["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["id"] == "events.list")
        .unwrap()["description"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        description.contains("repeats every other parameter of the full walk"),
        "{description}"
    );
    assert!(
        description.contains("a walk meant for sync tokens sends none of"),
        "{description}"
    );
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
        "**`fields` and the end conditions.**",
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

/// The guide's configuration example for this provider: the read-only one, or
/// the write instance's, which also asks for the write scope.
fn documented(write: bool) -> Value {
    let guide = guide();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| {
            body.contains(&format!("\"provider\": \"{PROVIDER}\""))
                && body.contains(WRITE_SCOPE) == write
        })
        .expect("the documented google-calendar configuration");
    serde_json::from_str::<Value>(example).unwrap()
}
fn documented_config() -> Value {
    documented(false)
}

/// Each write is cited with its request and its guard; the write scope, the
/// pinned `etag` and the route to a write-capable connection are stated: a
/// separate instance, connected afresh. Repair cannot add a scope, and the
/// guide must not say it does.
#[test]
fn guide_cites_each_write_its_guard_and_the_write_scope() {
    let guide = guide();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    for (id, operation_id, method, path) in WRITES {
        let request = format!("`{} {path}`", method.to_uppercase());
        let guard = if id == "events.insert" {
            "none"
        } else {
            "`/status` is `confirmed` and `/etag` equals the input `etag`"
        };
        let cited = rows.iter().any(|row| {
            row.starts_with(&format!("| `{id}` "))
                && row.contains(&format!("`{operation_id}`"))
                && row.contains(&request)
                && row.contains(guard)
        });
        assert!(cited, "no row cites `{id}` as {request} with guard {guard}");
    }
    assert!(guide.contains(&format!("`{WRITE_SCOPE}`")));
    let flat = guide.split_whitespace().collect::<Vec<_>>().join(" ");
    for phrase in [
        "Writes use a separate instance",
        "`google-calendar-write`",
        "`connections connect` that instance with the Google client file",
        "`connections repair` cannot add a scope",
        "approval binds the whole input by its digest",
        "`sendUpdates`",
    ] {
        assert!(flat.contains(phrase), "the guide does not say {phrase}");
    }
    assert!(
        !flat.contains("through `connections repair`")
            && !flat.contains("with `connections repair`"),
        "the guide routes the write scope through `connections repair`"
    );
}

/// The write configuration is a separate instance, `google-calendar-write`:
/// the read one with its own instance id and the write scope added to both
/// `minimum_scopes` and `requested_scopes`, and nothing else changed.
#[test]
fn guide_documents_the_write_configuration_with_the_calendar_events_scope() {
    let read = documented_config();
    let write = documented(true);
    assert_eq!(read["instance"], "google-calendar");
    assert_eq!(write["instance"], "google-calendar-write");
    assert_eq!(
        write["auth"]["minimum_scopes"],
        json!([CALENDAR_SCOPE, WRITE_SCOPE])
    );
    assert_eq!(
        write["auth"]["requested_scopes"],
        json!(["openid", CALENDAR_SCOPE, WRITE_SCOPE])
    );
    let strip = |mut config: Value| {
        config.as_object_mut().unwrap().remove("instance");
        config["auth"]
            .as_object_mut()
            .unwrap()
            .retain(|key, _| key != "minimum_scopes" && key != "requested_scopes");
        config
    };
    assert_eq!(strip(write), strip(read));
    // Every write, and the reads its preflight makes, accept the write scope,
    // per the pinned document; the read-only scope accepts no write.
    let events = &pinned()["resources"]["events"]["methods"];
    for method in ["insert", "patch", "delete", "get"] {
        let scopes = events[method]["scopes"].as_array().unwrap();
        assert!(scopes.contains(&json!(WRITE_SCOPE)), "`{method}`");
        assert_eq!(
            scopes.contains(&json!(CALENDAR_SCOPE)),
            method == "get",
            "`{method}`"
        );
    }
}

/// Each write's description names the effects that reach people other than
/// the caller, and says the approval binds the whole input by digest.
#[test]
fn write_descriptions_name_their_effects_and_the_digest_binding() {
    let selections = shipped();
    let description = |id: &str| {
        selections
            .iter()
            .find(|s| s.id == id)
            .and_then(|s| s.description.clone())
            .unwrap_or_default()
    };
    let mut missing = Vec::new();
    for (id, terms) in [
        (
            "events.insert",
            &[
                "sendUpdates",
                "attendees",
                "conferenceDataVersion",
                "conferenceData.createRequest",
                "guestsCanModify",
                "guestsCanInviteOthers",
                "guestsCanSeeOtherGuests",
                "visibility",
                "autoDeclineMode",
                "recurrence",
                "a second event",
                "by digest",
            ][..],
        ),
        (
            "events.patch",
            &[
                "etag",
                "sendUpdates",
                "attendees",
                "replaces",
                "conferenceDataVersion",
                "guestsCanModify",
                "visibility",
                "every instance",
                "maxAttendees",
                "attendeesOmitted",
                "confirmed",
                "tentative",
                "by digest",
            ][..],
        ),
        (
            "events.delete",
            &[
                "etag",
                "sendUpdates",
                "every instance",
                "confirmed",
                "tentative",
                "by digest",
            ][..],
        ),
    ] {
        let text = description(id);
        // Every write names the three `sendUpdates` values and says the
        // provider does not check them, so the approver must.
        let checked = [
            "must be all, externalOnly or none",
            "the provider does not check the value",
        ];
        for term in terms.iter().chain(&checked) {
            if !text.contains(term) {
                missing.push(format!("{id}: {term}"));
            }
        }
    }
    // Insert and patch answer a truncated guest list when sent with
    // `maxAttendees`, and a patch body copied from a truncated read is
    // described as the pinned document describes it, not as a certainty.
    for (id, term) in [
        ("events.insert", "maxAttendees"),
        (
            "events.patch",
            "limits an update to the participant's response",
        ),
        ("events.patch", "not verified"),
    ] {
        if !description(id).contains(term) {
            missing.push(format!("{id}: {term}"));
        }
    }
    assert!(missing.is_empty(), "descriptions omit {missing:#?}");
}

/// The guide says what the engine does not check about `sendUpdates`, how a
/// rate-limited write is classified, and which operations truncate a guest
/// list.
#[test]
fn guide_states_what_the_engine_leaves_to_the_approver_and_the_caller() {
    let guide = guide();
    let flat = guide.split_whitespace().collect::<Vec<_>>().join(" ");
    for phrase in [
        // `sendUpdates` is required, but its value is not checked.
        "The issuer checks that it is one of `all`, `externalOnly` or `none` before approving.",
        // A write answered `429` is `unknown`, as the provider guide says.
        "A write that Google answers `429` is reported `unknown`",
        // Insert and patch take `maxAttendees` like the reads.
        "`events.get`, `events.list`, `events.insert` and `events.patch` take `maxAttendees`",
        // The pinned document's words, not an unverified certainty.
        "\"can be used to only update the participant's response\"",
    ] {
        assert!(flat.contains(phrase), "the guide does not say: {phrase}");
    }
    for stale in [
        "an approved input always says whom Google emails",
        "and the patch then removes every other guest",
    ] {
        assert!(!flat.contains(stale), "the guide still says: {stale}");
    }
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
    // The `auth_uri` of the installed-app client file Google issues.
    assert_eq!(
        auth["authorize_url"],
        "https://accounts.google.com/o/oauth2/auth"
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
/// The fixture events' current `etag`, which every event read answers. Google's
/// event ETags are quoted strings, and the quotes are part of the value.
const ETAG: &str = "\"fixture-etag-2\"";
/// An earlier `etag` of the same event: a pin taken before a change.
const STALE_ETAG: &str = "\"fixture-etag-1\"";
/// The `etag` Google gives an event a write created or changed.
const WRITTEN_ETAG: &str = "\"fixture-etag-3\"";
fn event(id: &str, summary: &str) -> Value {
    json!({"kind": "calendar#event", "etag": ETAG, "id": id, "status": "confirmed",
           "summary": summary, "eventType": "default",
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
        // A deleted event, which the pinned document says `events.get` always
        // returns, and a tentative one; each still carries the current `etag`.
        "/calendar/v3/calendars/primary/events/fixture-event-cancelled" => {
            let mut deleted = event("fixture-event-cancelled", "deleted");
            deleted["status"] = json!("cancelled");
            ok(deleted)
        }
        "/calendar/v3/calendars/primary/events/fixture-event-tentative" => {
            let mut tentative = event("fixture-event-tentative", "tentative");
            tentative["status"] = json!("tentative");
            ok(tentative)
        }
        // An event Google no longer has: `410 Gone`, reason `deleted`.
        "/calendar/v3/calendars/primary/events/fixture-event-gone" => Some((
            410,
            json!({"error": {"code": 410, "message": "Resource has been deleted",
                             "errors": [{"domain": "global", "reason": "deleted",
                                         "message": "Resource has been deleted"}]}}),
        )),
        _ => None,
    }
}

/// The recorded answer to a write: a status and, except for a delete's
/// `204 No Content`, the event Google returns. An insert answers the event it
/// created from the body, a patch the stored event with the body applied, each
/// with a new `etag`. `None` for anything else, which the fixture answers 404.
fn written(method: &str, target: &str, body: &[u8]) -> Option<(u16, Option<Value>)> {
    let (route, _) = target.split_once('?').unwrap_or((target, ""));
    let apply = |mut event: Value| {
        let supplied: Value = serde_json::from_slice(body).unwrap();
        for (key, value) in supplied.as_object().unwrap() {
            event[key] = value.clone();
        }
        event["etag"] = json!(WRITTEN_ETAG);
        event
    };
    match (method, route) {
        ("POST", "/calendar/v3/calendars/primary/events") => Some((
            200,
            Some(apply(
                json!({"kind": "calendar#event", "id": "fixture-event-new",
                              "status": "confirmed", "eventType": "default"}),
            )),
        )),
        ("PATCH", "/calendar/v3/calendars/primary/events/fixture-event-1") => {
            Some((200, Some(apply(event("fixture-event-1", "first")))))
        }
        ("DELETE", "/calendar/v3/calendars/primary/events/fixture-event-1") => Some((204, None)),
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

/// Method, route with query, and body of each Calendar request that was not a
/// GET.
type Bodies = Arc<Mutex<Vec<(String, String, Vec<u8>)>>>;

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    requests: Requests,
    bodies: Bodies,
    protocol: Option<PrivateProtocol>,
}
/// The grant of the guide's read-only consent.
fn read_grant() -> String {
    format!("openid {CALENDAR_SCOPE}")
}
/// The grant of the write instance's consent: the read scope and the write
/// scope.
fn write_grant() -> String {
    format!("openid {CALENDAR_SCOPE} {WRITE_SCOPE}")
}
impl Provider {
    /// The guide's read-only configuration, over private protocol one.
    fn new() -> Self {
        Self::with(documented_config(), read_grant(), None)
    }
    /// The guide's write instance, over private protocol two, whose token
    /// route grants `grant`.
    fn writes(grant: &str) -> Self {
        Self::with(
            documented(true),
            grant.to_owned(),
            Some(PrivateProtocol::V2),
        )
    }
    /// `config` pointed at the fixture, whose token route grants `grant`.
    fn with(mut config: Value, grant: String, protocol: Option<PrivateProtocol>) -> Self {
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
        let carried = bodies.clone();
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
                    let (status, answer): (u16, Option<Value>) = if method == "POST"
                        && target == "/token"
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
                            (
                                200,
                                Some(json!({
                                    "access_token": ACCESS_TOKEN, "token_type": "Bearer",
                                    "expires_in": 3600,
                                    "scope": grant.clone(),
                                    "id_token": id_token()})),
                            )
                        } else {
                            (400, Some(json!({"error": "invalid_grant"})))
                        }
                    } else if authorization.as_deref() != Some(&*format!("Bearer {ACCESS_TOKEN}"))
                    {
                        let refusal = json!({"error": {"code": 401, "message": "fixture refusal"}});
                        (401, Some(refusal))
                    } else {
                        let found = if method == "GET" {
                            answer(&target).map(|(status, value)| (status, Some(value)))
                        } else {
                            let found = written(&method, &target, &body);
                            carried
                                .lock()
                                .unwrap()
                                .push((method.clone(), target.clone(), body));
                            found
                        };
                        found.unwrap_or_else(|| {
                            let missing = json!({"error": {"code": 404, "message": "no fixture"}});
                            (404, Some(missing))
                        })
                    };
                    observed
                        .lock()
                        .unwrap()
                        .push((method, target, authorization));
                    let answer = answer
                        .map(|value| serde_json::to_vec(&value).unwrap())
                        .unwrap_or_default();
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
            bodies,
            protocol,
        }
    }
    /// Method, route with query and JSON body (`null` for none) of each
    /// Calendar request that was not a GET.
    fn bodies(&self) -> Vec<(String, String, Value)> {
        self.bodies
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
    /// Method and route with query of each Calendar request.
    fn api_calls(&self) -> Vec<(String, String)> {
        self.requests()
            .into_iter()
            .filter(|(_, target, _)| target != "/token")
            .map(|(method, target, _)| (method, target))
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
            private_protocol: self.protocol,
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

/// `maxResults` 0 and one over `maximum` are refused as `invalid_input` with
/// nothing sent, as numbers and as strings; 1 and `maximum` are sent.
fn max_results_bounds(operation: &str, first: Value, route: &str, maximum: u64) {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    // Exchange the token first, so "nothing sent" counts every request.
    invoke(&mut child, operation, first.clone());
    let over = maximum + 1;
    let mut escaped = Vec::new();
    for size in [json!(0), json!(over), json!("0"), json!(over.to_string())] {
        let mut input = first.clone();
        input["maxResults"] = size.clone();
        let before = provider.requests().len();
        let outcome = attempt(&mut child, operation, &input);
        let sent = provider.requests().len() != before;
        if sent || !matches!(outcome, Err(Failure::InvalidInput)) {
            escaped.push(format!(
                "`{operation}` maxResults={size} sent={sent} {outcome:?}"
            ));
        }
    }
    assert!(
        escaped.is_empty(),
        "not refused before any request: {escaped:#?}"
    );
    for size in [1, maximum] {
        let mut input = first.clone();
        input["maxResults"] = json!(size);
        let before = provider.api_targets().len();
        invoke(&mut child, operation, input);
        let targets = provider.api_targets();
        assert_eq!(targets.len(), before + 1, "`{operation}` {size}");
        assert_eq!(targets[before], format!("{route}?maxResults={size}"));
    }
}

#[test]
fn events_list_max_results_bounds() {
    max_results_bounds(
        "events.list",
        json!({"calendarId": "primary", "maxResults": 2}),
        "/calendar/v3/calendars/primary/events",
        EVENTS_MAX_RESULTS,
    );
}

#[test]
fn calendar_list_max_results_bounds() {
    max_results_bounds(
        "calendarList.list",
        json!({"maxResults": 1}),
        "/calendar/v3/users/me/calendarList",
        CALENDARS_MAX_RESULTS,
    );
}

/// A new event with one guest, and the input that creates it on `primary`
/// without notifying anyone.
fn review() -> Value {
    json!({"summary": "Fixture review",
           "start": {"dateTime": "2026-10-05T10:00:00+02:00"},
           "end": {"dateTime": "2026-10-05T11:00:00+02:00"},
           "attendees": [{"email": "guest@example.test"}]})
}
fn moved() -> Value {
    json!({"summary": "first, moved", "start": {"dateTime": "2026-09-22T10:00:00+02:00"},
           "end": {"dateTime": "2026-09-22T11:00:00+02:00"}})
}
fn insert_input() -> Value {
    json!({"calendarId": "primary", "sendUpdates": "none", "body": review()})
}
fn patch_input(etag: &str) -> Value {
    json!({"calendarId": "primary", "eventId": "fixture-event-1", "etag": etag,
           "sendUpdates": "none", "body": moved()})
}
fn delete_input(etag: &str) -> Value {
    json!({"calendarId": "primary", "eventId": "fixture-event-1", "etag": etag,
           "sendUpdates": "none"})
}
/// Each write's input at the current `etag`.
fn write_inputs() -> [(&'static str, Value); 3] {
    [
        ("events.insert", insert_input()),
        ("events.patch", patch_input(ETAG)),
        ("events.delete", delete_input(ETAG)),
    ]
}

/// Prepare (the declared preflight included) and commit one write on this
/// child: the transport the host's approval coordinator drives once it holds a
/// verified, spent approval. A refusal in prepare is the `Err`.
fn write(child: &mut Child, operation: &str, input: &Value) -> Result<WriteResult, Failure> {
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
#[track_caller]
fn applied(result: Result<WriteResult, Failure>, operation: &str) -> Value {
    let result =
        result.unwrap_or_else(|failure| panic!("`{operation}` refused in prepare: {failure:?}"));
    assert_eq!(result.effect, WriteEffect::Applied, "`{operation}`");
    result
        .result
        .unwrap_or_else(|failure| panic!("`{operation}` result: {failure:?}"))
}
fn calls(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(method, target)| ((*method).to_owned(), (*target).to_owned()))
        .collect()
}
const EVENT_1: &str = "/calendar/v3/calendars/primary/events/fixture-event-1";

/// Each write sends exactly its request — patch and delete after their one
/// preflight read — with the body as supplied, `sendUpdates` in the query, the
/// pinned `etag` nowhere, and the exchanged bearer; Google's answer is returned
/// as applied, a delete's empty `204` as a `null` body.
#[test]
fn each_write_sends_exactly_its_request_and_body_and_returns_the_answer() {
    let provider = Provider::writes(&write_grant());
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let preflight = (
        "GET",
        "/calendar/v3/calendars/primary/events/fixture-event-1?",
    );
    for (operation, input, expected, body) in [
        (
            "events.insert",
            insert_input(),
            calls(&[(
                "POST",
                "/calendar/v3/calendars/primary/events?sendUpdates=none",
            )]),
            review(),
        ),
        (
            "events.patch",
            patch_input(ETAG),
            calls(&[
                preflight,
                (
                    "PATCH",
                    "/calendar/v3/calendars/primary/events/fixture-event-1?sendUpdates=none",
                ),
            ]),
            moved(),
        ),
        (
            "events.delete",
            delete_input(ETAG),
            calls(&[
                preflight,
                (
                    "DELETE",
                    "/calendar/v3/calendars/primary/events/fixture-event-1?sendUpdates=none",
                ),
            ]),
            Value::Null,
        ),
    ] {
        let before = provider.api_calls().len();
        let carried = provider.bodies().len();
        let result = applied(write(&mut child, operation, &input), operation);
        assert_eq!(
            provider.api_calls()[before..],
            expected[..],
            "`{operation}` requests"
        );
        let (method, target) = expected.last().unwrap().clone();
        assert_eq!(
            provider.bodies()[carried..],
            [(method.clone(), target.clone(), body.clone())],
            "`{operation}` body"
        );
        let sent = if body.is_null() {
            Vec::new()
        } else {
            serde_json::to_vec(&body).unwrap()
        };
        let (status, answer) = written(&method, &target, &sent).unwrap();
        assert_eq!(result["status"], status, "`{operation}` status");
        assert_eq!(
            result["body"],
            answer.unwrap_or(Value::Null),
            "`{operation}` answer"
        );
        assert_eq!(result["provenance"]["instance"], "fixture-google-calendar");
    }
    let bearer = format!("Bearer {ACCESS_TOKEN}");
    let requests = provider.requests();
    assert!(
        requests
            .iter()
            .filter(|(_, target, _)| target != "/token")
            .all(
                |(_, target, authorization)| authorization.as_deref() == Some(bearer.as_str())
                    && !target.contains("etag")
            ),
        "a Calendar request without the exchanged bearer, or carrying the pin"
    );
}

/// A pinned `etag` other than the event's current one fails the preflight for
/// patch and delete: the one `events.get` is sent and no write follows. The
/// same inputs at the current `etag` then write, so the refusal was the pin.
#[test]
fn a_stale_etag_fails_the_preflight_and_sends_no_write() {
    let provider = Provider::writes(&write_grant());
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, stale) in [
        ("events.patch", patch_input(STALE_ETAG)),
        ("events.delete", delete_input(STALE_ETAG)),
        // The same value without Google's quotes is another value.
        ("events.patch", patch_input(ETAG.trim_matches('"'))),
    ] {
        let before = provider.api_calls().len();
        let outcome = write(&mut child, operation, &stale);
        assert!(
            matches!(outcome, Err(Failure::Forbidden)),
            "`{operation}` at {}: {:?}",
            stale["etag"],
            outcome.map(|result| result.effect)
        );
        assert_eq!(
            provider.api_calls()[before..],
            calls(&[("GET", &format!("{EVENT_1}?"))])[..],
            "`{operation}`"
        );
    }
    assert!(provider.bodies().is_empty(), "a write was sent");
    applied(
        write(&mut child, "events.patch", &patch_input(ETAG)),
        "events.patch",
    );
    applied(
        write(&mut child, "events.delete", &delete_input(ETAG)),
        "events.delete",
    );
    assert_eq!(provider.bodies().len(), 2);
}

/// Patch and delete without a pin, or with a pin that is not a scalar, are
/// refused as invalid input before any Calendar request.
#[test]
fn patch_and_delete_without_an_etag_are_refused_before_any_request() {
    let provider = Provider::writes(&write_grant());
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input) in [
        ("events.patch", patch_input(ETAG)),
        ("events.delete", delete_input(ETAG)),
    ] {
        let mut unpinned = input.clone();
        unpinned.as_object_mut().unwrap().remove("etag");
        let mut object = input.clone();
        object["etag"] = json!({"value": ETAG});
        let mut null = input.clone();
        null["etag"] = Value::Null;
        for (what, input) in [("no etag", unpinned), ("an object", object), ("null", null)] {
            let outcome = write(&mut child, operation, &input);
            assert!(
                matches!(outcome, Err(Failure::InvalidInput)),
                "`{operation}` with {what}: {:?}",
                outcome.map(|result| result.effect)
            );
        }
    }
    assert!(
        provider.api_calls().is_empty(),
        "a Calendar request was sent"
    );
}

/// An event the preflight cannot find — Google's `404`, or `410` for one it
/// deleted — is a refusal before any write: the one `events.get` and nothing
/// after it.
#[test]
fn a_write_to_an_event_google_does_not_have_sends_no_write() {
    let provider = Provider::writes(&write_grant());
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for event_id in ["fixture-event-unknown", "fixture-event-gone"] {
        for (operation, mut input) in [
            ("events.patch", patch_input(ETAG)),
            ("events.delete", delete_input(ETAG)),
        ] {
            input["eventId"] = json!(event_id);
            let before = provider.api_calls().len();
            let outcome = write(&mut child, operation, &input);
            assert!(
                outcome.is_err(),
                "`{operation}` of {event_id}: {:?}",
                outcome.map(|result| result.effect)
            );
            let route = format!("/calendar/v3/calendars/primary/events/{event_id}?");
            assert_eq!(
                provider.api_calls()[before..],
                calls(&[("GET", &route)])[..],
                "`{operation}` of {event_id}"
            );
        }
    }
    assert!(provider.bodies().is_empty(), "a write was sent");
}

/// An event whose `status` is not `confirmed` — a deleted event, answered as
/// `cancelled` with an `etag` that equals the pin, or a `tentative` one — fails
/// the preflight's literal check: the one `events.get` and no write.
#[test]
fn a_write_to_an_event_that_is_not_confirmed_sends_no_write() {
    let provider = Provider::writes(&write_grant());
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for event_id in ["fixture-event-cancelled", "fixture-event-tentative"] {
        for (operation, mut input) in [
            ("events.patch", patch_input(ETAG)),
            ("events.delete", delete_input(ETAG)),
        ] {
            input["eventId"] = json!(event_id);
            let before = provider.api_calls().len();
            let outcome = write(&mut child, operation, &input);
            assert!(
                matches!(outcome, Err(Failure::Forbidden)),
                "`{operation}` of {event_id}: {:?}",
                outcome.map(|result| result.effect)
            );
            let route = format!("/calendar/v3/calendars/primary/events/{event_id}?");
            assert_eq!(
                provider.api_calls()[before..],
                calls(&[("GET", &route)])[..],
                "`{operation}` of {event_id}"
            );
        }
    }
    assert!(provider.bodies().is_empty(), "a write was sent");
}

/// A write offered on the dispatch path, which carries no approval, is refused
/// by the host before the child is asked for anything: no token exchange and
/// no Calendar request. Over private protocol one a write is not even
/// described and cannot be prepared; over private protocol two it is a
/// `mutation`, which only the host's prepare/commit exchange runs, and the
/// owner enters that exchange only with a verified proof. An absent or empty
/// proof document is refused where the owner decodes it.
#[test]
fn each_write_without_an_approval_is_refused_before_any_request() {
    let two = Provider::writes(&write_grant());
    let mut v2 = Child::spawn(&two.selection()).unwrap();
    let described = v2.bootstrap().descriptor().unwrap();
    let one = Provider::with(documented(true), write_grant(), None);
    let mut v1 = Child::spawn(&one.selection()).unwrap();
    for (operation, input) in write_inputs() {
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
        let outcome = write(&mut v1, operation, &input);
        assert!(
            matches!(outcome, Err(Failure::Unsupported)),
            "`{operation}` prepared over private protocol one: {:?}",
            outcome.map(|result| result.effect)
        );
    }
    for document in [b"".as_slice(), b"{}"] {
        assert!(matches!(
            approvals::Evidence::from_document(Secret(document.to_vec())),
            Err(approvals::Failure::Refused)
        ));
    }
    assert!(two.requests().is_empty(), "protocol two sent a request");
    assert!(one.requests().is_empty(), "protocol one sent a request");
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
        audience: "approval:fixture-google-calendar".into(),
        kid: "fixture-key".into(),
        public_key: URL_SAFE_NO_PAD.encode(hex::decode(APPROVAL_PUBLIC).unwrap()),
        not_before_unix_ms: 0,
        not_after_unix_ms: 4_000_000_000_000,
        revoked: false,
    })
}
fn signer() -> approvals::Signer {
    approvals::Signer::from_seed(
        Secret(hex::decode(APPROVAL_SEED).unwrap()),
        "fixture-key".into(),
    )
    .unwrap()
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
/// Issue a proof for `approved`, and require that it does not verify for
/// `changed` — so that write is refused before the child prepares it and
/// nothing is sent — and that it verifies for `approved`, whose write then
/// sends exactly the approved request.
fn approved_only(
    provider: &Provider,
    child: &mut Child,
    operation: &str,
    approved: &Value,
    changed: &Value,
) -> Value {
    let key = approval_key();
    assert_ne!(approved, changed);
    let proof = signer()
        .issue(&subject(child, operation, approved), &key, &Now)
        .unwrap();
    let before = provider.requests().len();
    assert!(
        matches!(
            approvals::verify(&proof, &subject(child, operation, changed), &key, &Now),
            Err(approvals::Failure::Refused)
        ),
        "`{operation}` approval verified for {changed}"
    );
    assert_eq!(provider.requests().len(), before, "`{operation}` sent");
    approvals::verify(&proof, &subject(child, operation, approved), &key, &Now)
        .unwrap_or_else(|failure| panic!("`{operation}` approval refused: {failure:?}"));
    applied(write(child, operation, approved), operation)
}

/// An approval binds the whole input: a proof issued for one body, one target
/// event or one pin does not verify for another.
#[test]
fn a_write_approved_for_a_different_input_is_refused() {
    let provider = Provider::writes(&write_grant());
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut other_body = insert_input();
    other_body["body"]["attendees"] = json!([{"email": "someone-else@example.test"}]);
    approved_only(
        &provider,
        &mut child,
        "events.insert",
        &insert_input(),
        &other_body,
    );
    let mut other_event = patch_input(ETAG);
    other_event["eventId"] = json!("fixture-event-2");
    approved_only(
        &provider,
        &mut child,
        "events.patch",
        &patch_input(ETAG),
        &other_event,
    );
    approved_only(
        &provider,
        &mut child,
        "events.delete",
        &delete_input(ETAG),
        &delete_input(STALE_ETAG),
    );
    // One write per approved input, and none for an input that was not.
    assert_eq!(provider.bodies().len(), 3);
}

/// The approval binds `sendUpdates`: a proof issued for an input with
/// `sendUpdates: none` does not verify for the same event with
/// `sendUpdates: all`, and no request is sent for it. The approved input's
/// write carries `sendUpdates=none`.
#[test]
fn approval_binds_send_updates() {
    let provider = Provider::writes(&write_grant());
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, quiet) in write_inputs() {
        assert_eq!(quiet["sendUpdates"], "none");
        let mut loud = quiet.clone();
        loud["sendUpdates"] = json!("all");
        let before = provider.bodies().len();
        approved_only(&provider, &mut child, operation, &quiet, &loud);
        let sent = provider.bodies();
        assert_eq!(sent.len(), before + 1, "`{operation}` writes");
        assert!(
            sent[before].1.ends_with("?sendUpdates=none"),
            "`{operation}` sent {}",
            sent[before].1
        );
    }
    assert!(
        provider
            .api_calls()
            .iter()
            .all(|(_, target)| !target.contains("sendUpdates=all")),
        "a request carried sendUpdates=all"
    );
}

/// The write configuration asks for `calendar.events`, so a consent that
/// granted only the read-only scope fails validation; one that granted both
/// validates.
#[test]
fn calendar_write_config_requires_calendar_events_scope() {
    let deadline = || connectors_sdk::now_ms() + 30_000;
    let provider = Provider::writes(&read_grant());
    let mut child = Child::spawn(&provider.selection()).unwrap();
    assert!(
        matches!(
            child.validate(PROFILE, &secret(), deadline()),
            Err(Failure::InsufficientScope)
        ),
        "a read-only grant validated the write configuration"
    );
    assert_eq!(
        provider.requests().len(),
        1,
        "one token exchange and nothing else"
    );
    let provider = Provider::writes(&write_grant());
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let validated = child
        .validate(PROFILE, &secret(), deadline())
        .unwrap_or_else(|failure| panic!("validation {failure:?}"));
    assert_eq!(
        validated.granted_scopes.unwrap(),
        [
            "openid".to_owned(),
            CALENDAR_SCOPE.to_owned(),
            WRITE_SCOPE.to_owned()
        ]
        .into()
    );
}

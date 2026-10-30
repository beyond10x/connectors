//! Adversary cases for keyed writes under a standing grant
//! (contracts/service/compatibility.md § 2.1, Keyed writes under a standing
//! grant; ess/domains/standing_approvals.yaml;
//! story:standing-write-approval-for-unattended-agents, served placement).
//!
//! The scope check and the keyed reservation, driven from the contract and the
//! story rather than from the implementation's own tests.
#![cfg(unix)]
use async_trait::async_trait;
use connectors_core::{
    Descriptor, Operation, Result, WIRE_VERSION,
    v1alpha2::{EffectKnowledge, ErrorCode, InvokeResponse},
};
use connectors_host::{
    local::standing::{self, Grant, Taken, Uncovered, Unreadable, Uses, WINDOW_MS},
    server::{ServiceState, router_with_state},
};
use connectors_sdk::{Adapter, Credential, Secret};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

const INSTANCE: &str = "leaf";
const TOKEN: &str = "service-token";
const LATER: i64 = 2_000_000_000_000;

struct FixedSecret;
#[async_trait]
impl Credential for FixedSecret {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(TOKEN.as_bytes().to_vec()))
    }
}

struct Fixture {
    requests: Arc<AtomicUsize>,
}
fn operation(id: &str, profile: &str) -> Operation {
    Operation {
        id: id.into(),
        description: format!("{id} fixture"),
        contract: "operations/v1alpha1".into(),
        profile: profile.into(),
        input_schema: json!({"type":"object"}),
        output_schema: json!({"type":"object"}),
    }
}
#[async_trait]
impl Adapter for Fixture {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            version: WIRE_VERSION.into(),
            instance: INSTANCE.into(),
            adapter: "fixture".into(),
            revision: "rev-1".into(),
            configuration_schema: json!({"type":"object"}),
            operations: vec![
                operation("issue.read", "read"),
                operation("issue.create", "mutation"),
                operation("issue.close", "mutation"),
            ],
        }
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        let number = self.requests.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(json!({"operation": operation, "number": number, "input": input}))
    }
}

struct Host {
    endpoint: String,
    task: tokio::task::JoinHandle<()>,
    state: Arc<ServiceState>,
    requests: Arc<AtomicUsize>,
    _root: tempfile::TempDir,
}
impl Drop for Host {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Host {
    fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}

/// A host with standing grants read from `grants` (a path under `root`).
async fn start(root: tempfile::TempDir, grants: PathBuf) -> Host {
    let path = root.path().join("state");
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let adapter = Arc::new(Fixture {
        requests: requests.clone(),
    });
    let state = Arc::new(
        ServiceState::open(&path, INSTANCE, "fixture", "rev-1")
            .unwrap()
            .with_standing_grants(Some(grants))
            .unwrap(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = router_with_state(adapter, Arc::new(FixedSecret), Some(state.clone()));
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Host {
        endpoint: format!("http://{address}/v1alpha2/invoke"),
        task,
        state,
        requests,
        _root: root,
    }
}

fn grant(id: &str, max_per_hour: u32) -> Value {
    json!({
        "id": id, "instance": INSTANCE, "operation": "issue.create",
        "target": [{"pointer": "/owner", "value": "acme"}, {"pointer": "/repo", "value": "widgets"}],
        "not_after_unix_ms": LATER, "max_per_hour": max_per_hour
    })
}
fn grant_file(grants: Value, revoked: Value) -> Value {
    json!({"format": "connectors.standing-grants/1", "grants": grants, "revoked": revoked})
}
fn put(path: &Path, document: &Value) {
    let staged = path.with_extension("new");
    std::fs::write(&staged, serde_json::to_vec(document).unwrap()).unwrap();
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::rename(&staged, path).unwrap();
}
fn write(request_id: &str, key: Option<&str>, operation: &str, input: Value) -> Value {
    let mut body = json!({"version": "v1alpha2", "request_id": request_id,
        "operation": operation, "revision": "rev-1", "input": input});
    if let Some(key) = key {
        body["idempotency_key"] = json!(key);
    }
    body
}
fn widgets(title: &str) -> Value {
    json!({"owner": "acme", "repo": "widgets", "title": title})
}
async fn invoke(endpoint: &str, body: &Value) -> (u16, InvokeResponse) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let response = reqwest::Client::new()
        .post(endpoint)
        .bearer_auth(TOKEN)
        .body(serde_json::to_vec(body).unwrap())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.bytes().await.unwrap();
    (status, InvokeResponse::decode_keyed(&bytes).unwrap())
}
fn code(response: &InvokeResponse) -> Option<ErrorCode> {
    response.error.as_ref().map(|e| e.code)
}

/// The kubelet's atomic-writer layout of a ConfigMap or Secret volume:
/// `<dir>/..<ts>/grants.json` (a regular file), `<dir>/..data -> ..<ts>` and
/// `<dir>/grants.json -> ..data/grants.json`. An update writes a new `..<ts2>`
/// and swaps `..data`; the top-level name stays a symbolic link throughout.
#[cfg(target_os = "linux")]
fn kubelet_publish(dir: &Path, generation: &str, document: &Value) {
    let data = dir.join(format!("..{generation}"));
    std::fs::create_dir(&data).unwrap();
    std::fs::write(
        data.join("grants.json"),
        serde_json::to_vec(document).unwrap(),
    )
    .unwrap();
    std::fs::set_permissions(
        data.join("grants.json"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    let staged = dir.join("..data_tmp");
    std::os::unix::fs::symlink(format!("..{generation}"), &staged).unwrap();
    std::fs::rename(&staged, dir.join("..data")).unwrap();
    if std::fs::symlink_metadata(dir.join("grants.json")).is_err() {
        std::os::unix::fs::symlink("..data/grants.json", dir.join("grants.json")).unwrap();
    }
}

/// Story (Decided, served placement): "the grant is part of the instance's
/// mounted configuration, a file the operator writes and the host reads on
/// every call. A removed grant, or one listed as revoked, is a revocation from
/// the next call on, with no restart." The served instance runs in the
/// cluster's credential namespace (design D243 §4), where mounted
/// configuration is a ConfigMap or Secret volume. Its live-updating layout
/// makes the configured path a symbolic link; a `subPath` mount avoids the
/// link but is never updated, so revocation would need a restart.
///
/// story:served-keyed-writes-under-a-standing-approval (C3 fix round,
/// decision 1): the volume layout is admitted where the `mounted_secret`
/// binding runs, Linux; another Unix refuses any link at the name.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn adversary_a_grant_file_mounted_as_a_kubernetes_volume_admits_and_revokes_live() {
    let root = tempfile::tempdir().unwrap();
    let volume = root.path().join("grants");
    std::fs::create_dir(&volume).unwrap();
    kubelet_publish(
        &volume,
        "2026_10_30_08_00_00.000000001",
        &grant_file(json!([grant("g-widgets", 10)]), json!([])),
    );
    let host = start(root, volume.join("grants.json")).await;

    let (status, admitted) = invoke(
        &host.endpoint,
        &write("w-1", Some("key-1"), "issue.create", widgets("first")),
    )
    .await;
    assert_eq!(
        (status, code(&admitted)),
        (200, None),
        "a covered write under a volume-mounted grant file was refused: {admitted:?}"
    );
    assert_eq!(host.requests(), 1);

    // The operator revokes the grant; the kubelet swaps `..data`.
    kubelet_publish(
        &volume,
        "2026_10_30_08_05_00.000000002",
        &grant_file(json!([grant("g-widgets", 10)]), json!(["g-widgets"])),
    );
    let (status, refused) = invoke(
        &host.endpoint,
        &write("w-2", Some("key-2"), "issue.create", widgets("second")),
    )
    .await;
    assert_eq!(
        (status, code(&refused)),
        (403, Some(ErrorCode::ApprovalRequired))
    );
    assert_eq!(host.requests(), 1);
}

/// docs/running-services.md: "Sending the same key and input again never
/// sends again: it answers the original attempt's outcome, marked
/// `replayed: true`." The existing concurrency case runs under
/// `max_per_hour: 100`, so it never reaches the cap. Under `max_per_hour: 1`
/// the original takes the only slot before it reserves the key (after the
/// anchor), so a concurrent duplicate in that window meets `rate_limited` for
/// a write that was admitted and applied, with `retry_after_seconds` near an
/// hour.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn adversary_a_concurrent_duplicate_under_a_cap_of_one_is_a_replay_not_rate_limited() {
    let root = tempfile::tempdir().unwrap();
    let grants = root.path().join("grants.json");
    let host = Arc::new(start(root, grants.clone()).await);
    let mut answers = Vec::new();
    for round in 0..6 {
        // A fresh grant id per round: a fresh hour of one slot.
        put(
            &grants,
            &grant_file(json!([grant(&format!("g-round-{round}"), 1)]), json!([])),
        );
        let before = host.requests();
        let mut calls = Vec::new();
        for index in 0..8 {
            let endpoint = host.endpoint.clone();
            let key = format!("round-{round}");
            calls.push(tokio::spawn(async move {
                invoke(
                    &endpoint,
                    &write(
                        &format!("w-{round}-{index}"),
                        Some(&key),
                        "issue.create",
                        widgets(&format!("round {round}")),
                    ),
                )
                .await
            }));
        }
        for call in calls {
            let (status, response) = call.await.unwrap();
            answers.push((
                round,
                status,
                code(&response),
                response.error.as_ref().and_then(|e| e.retry_after_seconds),
                response.mutation.as_ref().map(|m| m.replayed),
            ));
        }
        assert_eq!(
            host.requests(),
            before + 1,
            "round {round}: one key, one request"
        );
    }
    // story:served-keyed-writes-under-a-standing-approval (C3 fix round,
    // decision 4): at most four replays wait on a pending original; a
    // further one is answered at once as a replay of it, `outcome_unknown`.
    // Either way a duplicate is a replay, never `rate_limited`.
    let wrong: Vec<_> = answers
        .iter()
        .filter(|(_, status, code, _, replayed)| {
            !(*status == 200
                || (*status == 502
                    && *code == Some(ErrorCode::OutcomeUnknown)
                    && *replayed == Some(true)))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} duplicates of an applied keyed write were not answered as replays \
         (round, status, code, retry_after_seconds, replayed): {wrong:?}",
        wrong.len(),
        answers.len()
    );
}

/// Key edge cases (compatibility § 2.1: 1 to 256 UTF-8 bytes, opaque).
#[tokio::test]
async fn adversary_key_edges_follow_the_contract() {
    let root = tempfile::tempdir().unwrap();
    let grants = root.path().join("grants.json");
    let host = start(root, grants.clone()).await;
    put(
        &grants,
        &grant_file(json!([grant("g-widgets", 100)]), json!([])),
    );
    let e = &host.endpoint;

    // Empty and 257-byte keys: invalid_input with the request ID.
    for key in [String::new(), "k".repeat(257), "é".repeat(129)] {
        let (status, refused) =
            invoke(e, &write("w-bad", Some(&key), "issue.create", widgets("x"))).await;
        assert_eq!(
            (status, code(&refused)),
            (400, Some(ErrorCode::InvalidInput))
        );
        assert_eq!(refused.request_id.as_deref(), Some("w-bad"));
    }
    // A 256-byte non-ASCII key with a NUL and whitespace is admitted as is.
    let odd = format!(" \u{0}{}", "é".repeat(127));
    assert_eq!(odd.len(), 256);
    let (status, first) = invoke(
        e,
        &write("w-odd", Some(&odd), "issue.create", widgets("odd")),
    )
    .await;
    assert_eq!(status, 200, "{first:?}");
    // Its trimmed form is another key, not a replay.
    let (status, other) = invoke(
        e,
        &write(
            "w-odd-trim",
            Some(odd.trim()),
            "issue.create",
            widgets("odd"),
        ),
    )
    .await;
    assert_eq!(status, 200, "{other:?}");
    assert!(!other.mutation.unwrap().replayed);
    assert_eq!(host.requests(), 2);

    // Key order and whitespace in the input are the same request: a replay.
    let body = br#"{"version":"v1alpha2","request_id":"w-reorder","operation":"issue.create",
        "revision":"rev-1","idempotency_key":"order",
        "input":{"title":"t","repo":"widgets","owner":"acme"}}"#;
    let reordered = br#"{ "input" : { "owner":"acme" , "repo":"widgets","title":"t" },
        "idempotency_key":"order","revision":"rev-1","operation":"issue.create",
        "request_id":"w-reorder-2","version":"v1alpha2" }"#;
    for raw in [&body[..], &reordered[..]] {
        let response = reqwest::Client::new()
            .post(e)
            .bearer_auth(TOKEN)
            .body(raw.to_vec())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 200);
    }
    assert_eq!(host.requests(), 3, "a reordered input was sent again");

    // Duplicate members in the input are not a valid envelope.
    let duplicate = br#"{"version":"v1alpha2","request_id":"w-dup","operation":"issue.create",
        "revision":"rev-1","idempotency_key":"dup",
        "input":{"owner":"acme","repo":"widgets","repo":"other"}}"#;
    let response = reqwest::Client::new()
        .post(e)
        .bearer_auth(TOKEN)
        .body(duplicate.to_vec())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 400);
    assert_eq!(host.requests(), 3);

    // The same key on another operation is a conflict, not a replay.
    put(
        &grants,
        &grant_file(
            json!([grant("g-widgets", 100), {
                "id": "g-close", "instance": INSTANCE, "operation": "issue.close",
                "target": [{"pointer": "/owner", "value": "acme"}],
                "not_after_unix_ms": LATER, "max_per_hour": 100
            }]),
            json!([]),
        ),
    );
    let (status, conflict) = invoke(
        e,
        &write("w-close", Some("order"), "issue.close", widgets("t")),
    )
    .await;
    assert_eq!(
        (status, code(&conflict)),
        (409, Some(ErrorCode::IdempotencyConflict))
    );

    // A key refused before the anchor is not spent: once covered, it runs.
    let (status, refused) = invoke(
        e,
        &write(
            "w-out",
            Some("later"),
            "issue.create",
            json!({"owner": "acme", "repo": "x"}),
        ),
    )
    .await;
    assert_eq!(
        (status, code(&refused)),
        (403, Some(ErrorCode::ApprovalRequired))
    );
    // Two covering grants from here on.
    put(
        &grants,
        &grant_file(
            json!([grant("g-first", 100), grant("g-widgets", 100)]),
            json!([]),
        ),
    );
    let (status, admitted) = invoke(
        e,
        &write("w-in", Some("later"), "issue.create", widgets("later")),
    )
    .await;
    assert_eq!(status, 200, "{admitted:?}");
    assert_eq!(host.requests(), 4);

    // Two covering grants: the first in file order with room is recorded.
    let attempt = admitted.mutation.unwrap().attempt.unwrap();
    assert_eq!(
        host.state.grant_of(attempt.id.as_str()).unwrap().as_deref(),
        Some("g-first")
    );

    // Revoked after an applied write: the replay still answers the original.
    put(
        &grants,
        &grant_file(
            json!([grant("g-first", 100), grant("g-widgets", 100)]),
            json!(["g-first", "g-widgets"]),
        ),
    );
    let (status, replay) = invoke(
        e,
        &write(
            "w-in-again",
            Some("later"),
            "issue.create",
            widgets("later"),
        ),
    )
    .await;
    assert_eq!(status, 200);
    let mutation = replay.mutation.unwrap();
    assert!(mutation.replayed);
    assert_eq!(mutation.classification, EffectKnowledge::Applied);
    assert_eq!(host.requests(), 4);
}

fn document(grants: Value) -> Vec<u8> {
    serde_json::to_vec(&grant_file(grants, json!([]))).unwrap()
}
fn with(target: Value) -> Value {
    json!([{"id": "g", "instance": INSTANCE, "operation": "issue.create",
        "target": target, "not_after_unix_ms": LATER, "max_per_hour": 1}])
}

/// JSON pointer escapes, nesting, array indexes, type confusion, Unicode
/// normalization and case: only the exact member with the exact JSON value
/// is inside a grant.
#[test]
fn adversary_target_members_match_only_the_exact_member_and_value() {
    let covers = |target: Value, input: Value| {
        standing::parse(&document(with(target)))
            .unwrap()
            .covering(INSTANCE, "issue.create", &input, 1_000)
            .map(|grants| grants.len())
    };
    let missing = Err(Uncovered::MissingMember);
    let other = Err(Uncovered::OtherTarget);
    // `~1` is `/`, `~0` is `~`, `~01` is the key `~1`.
    let slash = json!([{"pointer": "/a~1b", "value": "x"}]);
    assert_eq!(covers(slash.clone(), json!({"a/b": "x"})), Ok(1));
    assert_eq!(covers(slash, json!({"a": {"b": "x"}})), missing);
    assert_eq!(
        covers(
            json!([{"pointer": "/a~0b", "value": "x"}]),
            json!({"a~b": "x"})
        ),
        Ok(1)
    );
    assert_eq!(
        covers(
            json!([{"pointer": "/~01", "value": "x"}]),
            json!({"~1": "x"})
        ),
        Ok(1)
    );
    assert_eq!(
        covers(
            json!([{"pointer": "/~01", "value": "x"}]),
            json!({"/": "x"})
        ),
        missing
    );
    // `/` is the member named "".
    assert_eq!(
        covers(json!([{"pointer": "/", "value": "x"}]), json!({"": "x"})),
        Ok(1)
    );
    assert_eq!(
        covers(json!([{"pointer": "/", "value": "x"}]), json!({"x": "x"})),
        missing
    );
    // Nested and array members.
    let nested = json!([{"pointer": "/repo/owner", "value": "acme"}]);
    assert_eq!(
        covers(nested.clone(), json!({"repo": {"owner": "acme"}})),
        Ok(1)
    );
    assert_eq!(covers(nested, json!({"repo": {"Owner": "acme"}})), missing);
    let first = json!([{"pointer": "/ids/0", "value": 7}]);
    assert_eq!(covers(first.clone(), json!({"ids": [7, 8]})), Ok(1));
    assert_eq!(covers(first.clone(), json!({"ids": [8, 7]})), other);
    assert_eq!(
        covers(
            json!([{"pointer": "/ids/00", "value": 7}]),
            json!({"ids": [7]})
        ),
        missing
    );
    assert_eq!(
        covers(
            json!([{"pointer": "/ids/-", "value": 7}]),
            json!({"ids": [7]})
        ),
        missing
    );
    assert_eq!(covers(first, json!({"ids": {"0": 7}})), Ok(1));
    // Type confusion.
    let one = json!([{"pointer": "/n", "value": 1}]);
    assert_eq!(covers(one.clone(), json!({"n": "1"})), other);
    assert_eq!(covers(one.clone(), json!({"n": 1.0})), other);
    assert_eq!(covers(one.clone(), json!({"n": true})), other);
    assert_eq!(covers(one, json!({"n": [1]})), other);
    assert_eq!(
        covers(
            json!([{"pointer": "/b", "value": true}]),
            json!({"b": "true"})
        ),
        other
    );
    let null = json!([{"pointer": "/v", "value": null}]);
    assert_eq!(covers(null.clone(), json!({"v": null})), Ok(1));
    assert_eq!(covers(null, json!({})), missing);
    // Unicode normalization and case are other values.
    let nfc = json!([{"pointer": "/owner", "value": "caf\u{e9}"}]);
    assert_eq!(covers(nfc.clone(), json!({"owner": "cafe\u{301}"})), other);
    assert_eq!(covers(nfc, json!({"owner": "CAF\u{c9}"})), other);
    // An object or array value is compared whole.
    let object = json!([{"pointer": "/repo", "value": {"owner": "acme", "name": "w"}}]);
    assert_eq!(
        covers(
            object.clone(),
            json!({"repo": {"name": "w", "owner": "acme"}})
        ),
        Ok(1)
    );
    assert_eq!(
        covers(
            object,
            json!({"repo": {"owner": "acme", "name": "w", "fork": true}})
        ),
        other
    );
    assert_eq!(
        covers(
            json!([{"pointer": "/l", "value": ["a", "b"]}]),
            json!({"l": ["b", "a"]})
        ),
        other
    );
    // Another instance, another operation, case variants of either.
    let grants = standing::parse(&document(with(json!([{"pointer": "/o", "value": 1}])))).unwrap();
    let input = json!({"o": 1});
    for (instance, operation) in [
        ("Leaf", "issue.create"),
        ("leaf ", "issue.create"),
        ("other", "issue.create"),
        (INSTANCE, "Issue.Create"),
        (INSTANCE, "issue.close"),
    ] {
        assert!(grants.covering(instance, operation, &input, 1_000).is_err());
    }
}

/// The file reader is closed and every bound refuses the whole file.
#[test]
fn adversary_a_grant_file_outside_its_bounds_is_malformed() {
    let base = |change: &dyn Fn(&mut Value)| {
        let mut one = with(json!([{"pointer": "/o", "value": 1}]));
        change(&mut one[0]);
        standing::parse(&document(one)).err()
    };
    let malformed = Some(Unreadable::Malformed);
    for value in [
        json!(0),
        json!(-1),
        json!(10_001),
        json!(4_294_967_297_u64),
        json!(1.0),
        json!("1"),
    ] {
        assert_eq!(
            base(&|g| g["max_per_hour"] = value.clone()),
            malformed,
            "max {value}"
        );
    }
    for value in [
        json!(-1),
        json!(9_007_199_254_740_992_i64),
        json!(1.5e12),
        json!("2030-01-01T00:00:00+02:00"),
        json!(null),
    ] {
        assert_eq!(
            base(&|g| g["not_after_unix_ms"] = value.clone()),
            malformed,
            "time {value}"
        );
    }
    assert_eq!(
        base(&|g| drop(g.as_object_mut().unwrap().remove("not_after_unix_ms"))),
        malformed
    );
    assert_eq!(base(&|g| g["extra"] = json!(1)), malformed);
    assert_eq!(base(&|g| g["target"][0]["extra"] = json!(1)), malformed);
    assert_eq!(base(&|g| g["target"][0]["pointer"] = json!("")), malformed);
    assert_eq!(base(&|g| g["target"][0]["pointer"] = json!("o")), malformed);
    assert_eq!(
        base(&|g| g["target"][0]["pointer"] = json!("/a~")),
        malformed
    );
    assert_eq!(base(&|g| g["instance"] = json!("leaf ")), malformed);
    // Duplicate members inside a grant and inside a target value.
    for raw in [
        br#"{"format":"connectors.standing-grants/1","grants":[{"id":"g","id":"h","instance":"leaf","operation":"issue.create","target":[{"pointer":"/o","value":1}],"not_after_unix_ms":1,"max_per_hour":1}]}"#.as_slice(),
        br#"{"format":"connectors.standing-grants/1","grants":[{"id":"g","instance":"leaf","operation":"issue.create","target":[{"pointer":"/o","value":{"a":1,"a":2}}],"not_after_unix_ms":1,"max_per_hour":1}]}"#.as_slice(),
        br#"{"format":"connectors.standing-grants/1","grants":[],"revoked":["g"],"revoked":[]}"#.as_slice(),
    ] {
        assert_eq!(standing::parse(raw).err(), malformed);
    }
    // A revoked id the file does not list is accepted and changes nothing.
    assert!(
        standing::parse(
            br#"{"format":"connectors.standing-grants/1","grants":[],"revoked":["gone"]}"#
        )
        .is_ok()
    );
}

/// `not_after` is exclusive to the millisecond.
#[test]
fn adversary_not_after_is_exclusive() {
    let mut one = with(json!([{"pointer": "/o", "value": 1}]));
    one[0]["not_after_unix_ms"] = json!(5_000);
    let grants = standing::parse(&document(one)).unwrap();
    let input = json!({"o": 1});
    assert!(
        grants
            .covering(INSTANCE, "issue.create", &input, 4_999)
            .is_ok()
    );
    assert_eq!(
        grants
            .covering(INSTANCE, "issue.create", &input, 5_000)
            .err(),
        Some(Uncovered::Expired)
    );
    assert_eq!(
        grants
            .covering(INSTANCE, "issue.create", &input, 5_001)
            .err(),
        Some(Uncovered::Expired)
    );
}

/// The last slot is taken once, by threads racing on one store, and the
/// window frees a use exactly one hour and one millisecond after it.
#[test]
fn adversary_the_last_slot_is_taken_once_and_the_window_is_one_hour() {
    let root = tempfile::tempdir().unwrap();
    let grant: Grant =
        serde_json::from_value(with(json!([{"pointer": "/o", "value": 1}]))[0].clone()).unwrap();
    let grant = Arc::new(grant);
    let directory = Arc::new(root.path().to_path_buf());
    Uses::open(&directory).unwrap();
    let threads: Vec<_> = (0..16)
        .map(|_| {
            let grant = grant.clone();
            let directory = directory.clone();
            std::thread::spawn(move || {
                let uses = Uses::open(&directory).unwrap();
                matches!(
                    uses.take(INSTANCE, &[grant.as_ref()], 1_000_000).unwrap(),
                    Taken::Slot(_)
                )
            })
        })
        .collect();
    let slots = threads
        .into_iter()
        .map(|t| t.join().unwrap())
        .filter(|taken| *taken)
        .count();
    assert_eq!(slots, 1);
    let uses = Uses::open(&directory).unwrap();
    assert_eq!(
        uses.take(INSTANCE, &[grant.as_ref()], 1_000_000 + WINDOW_MS)
            .unwrap(),
        Taken::AtCap {
            retry_after_seconds: 1
        }
    );
    assert!(matches!(
        uses.take(INSTANCE, &[grant.as_ref()], 1_000_000 + WINDOW_MS + 1)
            .unwrap(),
        Taken::Slot(_)
    ));
}

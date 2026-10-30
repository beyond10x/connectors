//! Security review of keyed writes under a standing grant
//! (story:served-keyed-writes-under-a-standing-approval; contracts/service/
//! compatibility.md § 2.1, Keyed writes under a standing grant).
//!
//! Conformance checks of the unit's own contract that its suite does not
//! make: a replay after a restart that interrupted the original, the service
//! slots replays hold while they wait, a FIFO or directory at the grant path,
//! the principal a key's replay is bound to, and the custody of the use store.
//!
//! Unix only: the fixtures use Unix mode bits and a FIFO.
#![cfg(unix)]
use async_trait::async_trait;
use connectors_core::{
    Descriptor, Operation, Result, WIRE_VERSION,
    v1alpha2::{
        BaseErrorCode, CauseStage, EffectKnowledge, ErrorCode, InvokeResponse, ResponseStatus,
    },
};
use connectors_host::local::service::{
    GrantCheck, KeyLookup, KeyedPreparation, STATIC_BEARER_PRINCIPAL, Write,
};
use connectors_host::server::{ServiceState, router_with_state};
use connectors_sdk::{Adapter, Credential, Secret};
use serde_json::{Value, json};
use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

const INSTANCE: &str = "leaf";
const TOKEN: &str = "service-token";
const LATER: i64 = 2_000_000_000_000;
/// The host's service slots (`server.rs`, `Semaphore::new(32)`).
const SLOTS: usize = 32;

struct FixedSecret;
#[async_trait]
impl Credential for FixedSecret {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(TOKEN.as_bytes().to_vec()))
    }
}

/// `issue.read` (`profile: read`) and `issue.create` (`profile: mutation`);
/// every call is one counted provider request.
struct Fixture(Arc<AtomicUsize>);
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
            ],
        }
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        let number = self.0.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(json!({"operation": operation, "number": number, "input": input}))
    }
}

fn widgets(title: &str) -> Value {
    json!({"owner": "acme", "repo": "widgets", "title": title})
}

fn grant_document() -> Value {
    json!({"format": "connectors.standing-grants/1", "revoked": [], "grants": [{
        "id": "g-widgets", "instance": INSTANCE, "operation": "issue.create",
        "target": [{"pointer": "/owner", "value": "acme"}, {"pointer": "/repo", "value": "widgets"}],
        "not_after_unix_ms": LATER, "max_per_hour": 10_000
    }]})
}

/// A root with an owner-only `state` directory and an owner-only covering
/// grant file `grants.json`.
fn root() -> Arc<tempfile::TempDir> {
    let root = Arc::new(tempfile::tempdir().unwrap());
    let state = root.path().join("state");
    std::fs::create_dir(&state).unwrap();
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700)).unwrap();
    let grants = root.path().join("grants.json");
    std::fs::write(&grants, serde_json::to_vec(&grant_document()).unwrap()).unwrap();
    std::fs::set_permissions(&grants, std::fs::Permissions::from_mode(0o600)).unwrap();
    root
}

fn open_state(root: &Path, grants: PathBuf) -> ServiceState {
    ServiceState::open(&root.join("state"), INSTANCE, "fixture", "rev-1")
        .unwrap()
        .with_standing_grants(Some(grants))
        .unwrap()
}

struct Host {
    base: String,
    task: tokio::task::JoinHandle<()>,
    state: Arc<ServiceState>,
    requests: Arc<AtomicUsize>,
}
impl Drop for Host {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// A host process on `root`'s state directory, its grant file at `grants`.
async fn serve(root: &Path, grants: PathBuf) -> Host {
    let requests = Arc::new(AtomicUsize::new(0));
    let state = Arc::new(open_state(root, grants));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = router_with_state(
        Arc::new(Fixture(requests.clone())),
        Arc::new(FixedSecret),
        Some(state.clone()),
    );
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Host {
        base: format!("http://{address}"),
        task,
        state,
        requests,
    }
}

fn body(operation: &str, request_id: &str, key: Option<&str>, input: Value) -> Value {
    let mut body = json!({"version": "v1alpha2", "request_id": request_id,
        "operation": operation, "revision": "rev-1", "input": input});
    if let Some(key) = key {
        body["idempotency_key"] = json!(key);
    }
    body
}

async fn invoke(host: &Host, body: &Value) -> (u16, InvokeResponse) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let response = reqwest::Client::new()
        .post(format!("{}/v1alpha2/invoke", host.base))
        .bearer_auth(TOKEN)
        .body(serde_json::to_vec(body).unwrap())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.bytes().await.unwrap();
    (status, InvokeResponse::decode_keyed(&bytes).unwrap())
}

/// The keyed write the host would record for `principal`.
fn write<'a>(request_id: &'a str, principal: &'a str, input: &'a Value) -> Write<'a> {
    Write {
        request_id,
        principal,
        operation: "issue.create",
        contract: "operations/v1alpha1",
        profile: "mutation",
        descriptor_revision: "rev-1",
        input,
    }
}

/// A host process that admitted the write keyed `key`, prepared its attempt
/// and opened its dispatch, then stopped before any settlement: the state a
/// crash or a killed pod leaves (compatibility § 2.1, "A pending reservation
/// found after a restart").
fn interrupted_write(root: &Path, key: &str, input: &Value) {
    let state = open_state(root, root.join("grants.json"));
    let write = write("w-original", STATIC_BEARER_PRINCIPAL, input);
    let GrantCheck::Admitted(taken) = state.check_grant(&write) else {
        panic!("the fixture grant does not cover the write");
    };
    let Ok(KeyedPreparation::Prepared(prepared)) = state.prepare_keyed(&write, key, &taken) else {
        panic!("the keyed attempt was not prepared");
    };
    state
        .open_attempt(prepared)
        .expect("the prepared attempt did not open");
}

/// Operations § 4 and compatibility § 2.1: the replay of an original a crash
/// interrupted is awaited at most 25 seconds and answered `outcome_unknown`,
/// `replayed: true`; it is never dispatched again.
#[tokio::test]
async fn review_a_replay_of_a_write_interrupted_by_a_restart_is_outcome_unknown_and_never_sent() {
    let root = root();
    let input = widgets("first");
    interrupted_write(root.path(), "key-crash", &input);
    let host = serve(root.path(), root.path().join("grants.json")).await;
    let started = Instant::now();
    let (status, response) = invoke(
        &host,
        &body("issue.create", "w-after-restart", Some("key-crash"), input),
    )
    .await;
    let waited = started.elapsed();
    assert_eq!(
        response.error.as_ref().map(|e| e.code),
        Some(ErrorCode::OutcomeUnknown),
        "{response:?}"
    );
    assert_eq!(status, 502, "{response:?}");
    let mutation = response
        .mutation
        .expect("a replay names the original attempt");
    assert!(mutation.replayed);
    assert_eq!(mutation.classification, EffectKnowledge::Unknown);
    assert_eq!(mutation.original_request_id.as_deref(), Some("w-original"));
    assert!(
        waited <= Duration::from_secs(27),
        "the replay waited {waited:?}"
    );
    // story:served-keyed-writes-under-a-standing-approval (C3 fix round,
    // decision 4): no process runs the original, so nothing waits for it.
    assert!(
        waited < Duration::from_secs(5),
        "the replay waited {waited:?}"
    );
    assert_eq!(
        mutation.cause.map(|cause| (cause.code, cause.stage)),
        Some((BaseErrorCode::Unavailable, CauseStage::Observation))
    );
    assert_eq!(
        host.requests.load(Ordering::SeqCst),
        0,
        "the interrupted write was sent again"
    );
    assert_eq!(host.state.attempt_count().unwrap(), 1);
}

/// The brief's invariant 4: a waiter cannot be used to pin the host's
/// resources. A replay of a pending key holds one of the host's 32 service
/// slots for the whole 25-second wait, and keeps holding it after its caller
/// hung up (the execution is a host task the request only awaits). After a
/// restart left one key pending, an admitted caller that sends 32 replays and
/// drops each connection after one second leaves no slot for any other
/// caller's read for 25 seconds, with no provider involved and no connection
/// held.
#[tokio::test]
async fn review_replays_waiting_on_a_pending_key_leave_a_slot_for_another_callers_read() {
    let root = root();
    let input = widgets("first");
    interrupted_write(root.path(), "key-crash", &input);
    let host = Arc::new(serve(root.path(), root.path().join("grants.json")).await);
    let _ = rustls::crypto::ring::default_provider().install_default();
    let impatient = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();
    let mut hung_up = Vec::new();
    for index in 0..SLOTS {
        let client = impatient.clone();
        let url = format!("{}/v1alpha2/invoke", host.base);
        let body = body(
            "issue.create",
            &format!("w-replay-{index}"),
            Some("key-crash"),
            input.clone(),
        );
        hung_up.push(tokio::spawn(async move {
            let response = client
                .post(url)
                .bearer_auth(TOKEN)
                .body(serde_json::to_vec(&body).unwrap())
                .send()
                .await
                .ok()?;
            let status = response.status().as_u16();
            let bytes = response.bytes().await.ok()?;
            Some((status, InvokeResponse::decode_keyed(&bytes).unwrap()))
        }));
    }
    // story:served-keyed-writes-under-a-standing-approval (C3 fix round,
    // decision 4): a pending original no process of this host runs is
    // answered `outcome_unknown` at once, with no wait. A replay still
    // records its own anchor and final observation, and the 32 callers'
    // records are written one after another, so a caller late in that line
    // may still hang up after its second; none waits for the original.
    let mut answered = 0;
    for call in hung_up {
        let Some((status, response)) = call.await.unwrap() else {
            continue;
        };
        answered += 1;
        assert_eq!(
            (status, response.error.as_ref().map(|e| e.code)),
            (502, Some(ErrorCode::OutcomeUnknown)),
            "{response:?}"
        );
        let mutation = response.mutation.expect("a replay names the original");
        assert!(mutation.replayed);
        assert_eq!(mutation.classification, EffectKnowledge::Unknown);
    }
    assert!(answered > 0, "every replay waited past one second");
    assert_eq!(host.requests.load(Ordering::SeqCst), 0);
    // Every replaying caller has hung up; none holds a connection now.
    let (status, response) = invoke(
        &host,
        &body("issue.read", "r-other-caller", None, json!({"q": 1})),
    )
    .await;
    assert_eq!(
        (
            status,
            response.status,
            response.error.as_ref().map(|e| e.code)
        ),
        (200, ResponseStatus::Success, None),
        "detached replay waiters hold every service slot: {response:?}"
    );
}

/// Invariant 2: a FIFO or a directory at the grant path is refused
/// `unavailable` at once, with no provider request; opening the FIFO must not
/// block the write until a writer appears.
#[tokio::test]
async fn review_a_fifo_or_a_directory_at_the_grant_path_is_refused_unavailable_at_once() {
    let root = root();
    let fifo = root.path().join("grants.fifo");
    let name = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: a valid NUL-terminated path.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let directory = root.path().join("grants.dir");
    std::fs::create_dir(&directory).unwrap();
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    for (case, path) in [("fifo", fifo), ("directory", directory)] {
        let host = serve(root.path(), path).await;
        let answered = tokio::time::timeout(
            Duration::from_secs(5),
            invoke(
                &host,
                &body(
                    "issue.create",
                    "w-1",
                    Some(&format!("key-{case}")),
                    widgets("t"),
                ),
            ),
        )
        .await;
        let Ok((status, response)) = answered else {
            panic!("{case}: the grant read blocked the write");
        };
        assert_eq!(status, 503, "{case}: {response:?}");
        assert_eq!(
            response.error.as_ref().map(|e| e.code),
            Some(ErrorCode::Unavailable)
        );
        assert!(response.mutation.is_none(), "{case}");
        assert_eq!(host.requests.load(Ordering::SeqCst), 0, "{case}");
        assert_eq!(host.state.attempt_count().unwrap(), 0, "{case}");
    }
}

/// Invariant 3: a retained result is replayed only in the namespace of the
/// principal that wrote it. Another principal presenting the same key and
/// input finds no reservation (a new write of its own, under its own grant
/// check), never the first principal's stored result.
#[tokio::test]
async fn review_a_key_replays_only_in_the_namespace_of_the_principal_that_wrote_it() {
    let root = root();
    let state = open_state(root.path(), root.path().join("grants.json"));
    let input = widgets("first");
    let owner = "system:serviceaccount:agents:writer";
    let other = "system:serviceaccount:agents:reader";
    let first = write("w-1", owner, &input);
    let GrantCheck::Admitted(taken) = state.check_grant(&first) else {
        panic!("the fixture grant does not cover the write");
    };
    let Ok(KeyedPreparation::Prepared(prepared)) =
        state.prepare_keyed(&first, "shared-key", &taken)
    else {
        panic!("the keyed attempt was not prepared");
    };
    let attempt = state.open_attempt(prepared).unwrap();
    let result = json!({"created": "issue-7", "token_like": "provider-returned-value"});
    assert!(state.settle_keyed(attempt, Some(&result), None));

    let Ok(KeyLookup::Replay(own)) = state.lookup_key(&first, "shared-key") else {
        panic!("the writer's own key did not replay");
    };
    assert_eq!(
        own.result.as_ref().and_then(|r| r.get("result")),
        Some(&result)
    );
    let probe = write("w-2", other, &input);
    assert!(
        matches!(state.lookup_key(&probe, "shared-key"), Ok(KeyLookup::Miss)),
        "another principal's lookup reached the writer's reservation"
    );
}

/// Invariant 3: the state directory's stores are owner-only files. The
/// metadata authority refuses a store that is not; the use store
/// `standing-grants.sqlite3` should be held to the same custody.
#[tokio::test]
async fn review_the_use_store_is_an_owner_only_file_like_the_metadata_authority() {
    let root = root();
    let state = open_state(root.path(), root.path().join("grants.json"));
    let input = widgets("first");
    let GrantCheck::Admitted(_) = state.check_grant(&write("w-1", STATIC_BEARER_PRINCIPAL, &input))
    else {
        panic!("the fixture grant does not cover the write");
    };
    let directory = root.path().join("state");
    let uses = std::fs::metadata(directory.join("standing-grants.sqlite3")).unwrap();
    assert_eq!(uses.uid(), unsafe { libc::geteuid() });
    assert_eq!(
        uses.mode() & 0o077,
        0,
        "standing-grants.sqlite3 is mode {:o}",
        uses.mode() & 0o777
    );
}

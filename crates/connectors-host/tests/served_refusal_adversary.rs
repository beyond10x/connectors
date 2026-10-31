//! Adversary cases for story:served-merge-refusal-answers-refused, host part:
//! a keyed write on `POST /v1alpha2/invoke` whose adapter answers the
//! provider's definite refusal through `Adapter::invoke_write_at`.
//!
//! Driven against compatibility § 2.1 ("Keyed writes under a standing grant",
//! "`mutation`") and § 5 (`Error` is `{code,message,retry_after_seconds?}`
//! with a safe message of at most 512 UTF-8 bytes; a replay of a Failed
//! original answers its original code and message with `refused`).
//!
//! Unix only: the grant file and state directory use Unix mode bits.
#![cfg(unix)]
use async_trait::async_trait;
use connectors_core::{
    Descriptor, Operation, Result, WIRE_VERSION,
    v1alpha2::{EffectKnowledge, ErrorCode, InvokeResponse, MESSAGE_LIMIT, ResponseStatus},
};
use connectors_host::{
    local::{
        audit::{self, Reference},
        mutations,
    },
    server::{ServiceState, router_with_state},
};
use connectors_sdk::{Adapter, Credential, Secret, WriteOutcome};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

const INSTANCE: &str = "leaf";
const TOKEN: &str = "service-token";
const LATER: i64 = 2_000_000_000_000;
/// The message a well-behaved refusal carries.
const REFUSAL: &str = "provider refused the write: Pull Request is not mergeable";

struct FixedSecret;
#[async_trait]
impl Credential for FixedSecret {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(TOKEN.as_bytes().to_vec()))
    }
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
fn descriptor() -> Descriptor {
    Descriptor {
        version: WIRE_VERSION.into(),
        instance: INSTANCE.into(),
        adapter: "fixture".into(),
        revision: "rev-1".into(),
        configuration_schema: json!({"type":"object"}),
        operations: vec![operation("merge", "mutation")],
    }
}

/// `merge` is one provider request, counted before it waits for a shared
/// hold of `gate`. Its input's `mode` picks the provider's answer:
/// `refuse` (the well-behaved refusal), `long` (a 600-byte message), `empty`
/// (an empty message), `rate` (a refused rate limit naming a delay), and
/// anything else applies.
struct Refusing {
    requests: Arc<AtomicUsize>,
    gate: Arc<RwLock<()>>,
}
#[async_trait]
impl Adapter for Refusing {
    fn descriptor(&self) -> Descriptor {
        descriptor()
    }
    async fn invoke(&self, _operation: &str, input: Value) -> Result<Value> {
        self.requests.fetch_add(1, Ordering::SeqCst);
        Ok(input)
    }
    async fn invoke_write_at(
        &self,
        _revision: &str,
        _operation: &str,
        input: Value,
    ) -> WriteOutcome<Value> {
        self.requests.fetch_add(1, Ordering::SeqCst);
        let _shared = self.gate.read().await;
        let refused = |code, message: String| {
            WriteOutcome::Refused(connectors_core::Error::new(code, message).answered())
        };
        match input["mode"].as_str().unwrap_or_default() {
            "refuse" => refused(connectors_core::ErrorCode::Forbidden, REFUSAL.into()),
            "long" => refused(
                connectors_core::ErrorCode::InvalidInput,
                format!("provider refused the write: {}", "no ".repeat(200)),
            ),
            "empty" => refused(connectors_core::ErrorCode::Forbidden, String::new()),
            "rate" => {
                let mut error = connectors_core::Error::new(
                    connectors_core::ErrorCode::RateLimited,
                    "provider refused the request",
                )
                .answered();
                error.retry_after_seconds = Some(30);
                WriteOutcome::Refused(error)
            }
            _ => WriteOutcome::Applied(Ok(input)),
        }
    }
}

/// An adapter that keeps the default `invoke_write_at`: every write error is
/// an error whose effect it does not know.
struct Defaulted {
    requests: Arc<AtomicUsize>,
}
#[async_trait]
impl Adapter for Defaulted {
    fn descriptor(&self) -> Descriptor {
        descriptor()
    }
    async fn invoke(&self, _operation: &str, _input: Value) -> Result<Value> {
        self.requests.fetch_add(1, Ordering::SeqCst);
        Err(connectors_core::Error::new(connectors_core::ErrorCode::Forbidden, REFUSAL).answered())
    }
}

struct Host {
    endpoint: String,
    task: tokio::task::JoinHandle<()>,
    state: Arc<ServiceState>,
    path: PathBuf,
    requests: Arc<AtomicUsize>,
    gate: Arc<RwLock<()>>,
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

/// A keyed host whose grant covers `merge` on owner `acme`, `max_per_hour`
/// writes an hour, with the default port when `defaulted`.
async fn host(defaulted: bool, max_per_hour: u32) -> Host {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let grants = root.path().join("grants.json");
    let document = json!({
        "format": "connectors.standing-grants/1",
        "grants": [{
            "id": "g-merge", "instance": INSTANCE, "operation": "merge",
            "target": [{"pointer": "/owner", "value": "acme"}],
            "not_after_unix_ms": LATER, "max_per_hour": max_per_hour
        }],
        "revoked": []
    });
    std::fs::write(&grants, serde_json::to_vec(&document).unwrap()).unwrap();
    std::fs::set_permissions(&grants, std::fs::Permissions::from_mode(0o644)).unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(RwLock::new(()));
    let adapter: Arc<dyn Adapter> = if defaulted {
        Arc::new(Defaulted {
            requests: requests.clone(),
        })
    } else {
        Arc::new(Refusing {
            requests: requests.clone(),
            gate: gate.clone(),
        })
    };
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
        path,
        requests,
        gate,
        _root: root,
    }
}

/// One keyed request: the HTTP status and the raw body, undecoded.
async fn send(endpoint: &str, body: &Value) -> (u16, Vec<u8>) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let response = reqwest::Client::new()
        .post(endpoint)
        .bearer_auth(TOKEN)
        .body(serde_json::to_vec(body).unwrap())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    (status, response.bytes().await.unwrap().to_vec())
}

/// One keyed request through the binding's own reader.
async fn invoke(host: &Host, body: &Value) -> (u16, InvokeResponse) {
    let (status, bytes) = send(&host.endpoint, body).await;
    let decoded = InvokeResponse::decode_keyed(&bytes).unwrap_or_else(|error| {
        panic!(
            "the host's answer is not a valid keyed Response ({error:?}): {}",
            String::from_utf8_lossy(&bytes)
        )
    });
    (status, decoded)
}

fn merge(request_id: &str, key: &str, mode: &str) -> Value {
    json!({
        "version": "v1alpha2", "request_id": request_id, "operation": "merge",
        "revision": "rev-1", "idempotency_key": key,
        "input": {"owner": "acme", "repo": "widgets", "mode": mode}
    })
}

fn final_outcome(host: &Host, audit_ref: &str) -> audit::Outcome {
    audit::Store::new(&host.path, 100_000)
        .unwrap()
        .observe(&Reference {
            instance: INSTANCE.into(),
            audit_ref: audit_ref.into(),
        })
        .unwrap()
        .expect("the anchored record")
        .final_observation
        .expect("the final observation")
        .outcome
}

async fn until(what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !done() {
        assert!(Instant::now() < deadline, "{what}");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// Compatibility § 5: "Error remains `{code,message,retry_after_seconds?}`
/// with a safe message of at most 512 UTF-8 bytes". The host is the
/// boundary: an adapter's refusal message is answered as it came, so a
/// refusal over the bound gives an answer the binding's own reader refuses.
#[tokio::test]
async fn adversary_a_refused_write_answer_keeps_the_message_bound() {
    let host = host(false, 100).await;
    let (status, bytes) = send(&host.endpoint, &merge("long-1", "k-long", "long")).await;
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status, 400, "{body}");
    assert_eq!(body["mutation"]["classification"], "refused", "{body}");
    let message = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        message.len() <= MESSAGE_LIMIT,
        "the refusal's message is {} bytes, over the {MESSAGE_LIMIT}-byte bound",
        message.len()
    );
    assert!(
        InvokeResponse::decode_keyed(&bytes).is_ok(),
        "the binding's own reader refuses the host's answer"
    );
}

/// Compatibility § 2.1, Replay: "a Failed one, which the provider definitely
/// refused, its original code and message with `refused`". An empty refusal
/// message is answered `forbidden` the first time, and its replay changes
/// the code (and the HTTP status) to the fixed `invalid_input`.
#[tokio::test]
async fn adversary_a_replayed_refusal_answers_the_original_code() {
    let host = host(false, 100).await;
    let (status, original) = invoke(&host, &merge("empty-1", "k-empty", "empty")).await;
    assert_eq!(
        original.mutation.as_ref().map(|m| m.classification),
        Some(EffectKnowledge::Refused)
    );
    let (replay_status, replay) = invoke(&host, &merge("empty-2", "k-empty", "empty")).await;
    assert!(replay.mutation.as_ref().is_some_and(|m| m.replayed));
    assert_eq!(host.requests(), 1, "a replay sends nothing");
    assert_eq!(
        replay.error.as_ref().map(|e| e.code),
        original.error.as_ref().map(|e| e.code),
        "the replay answers another code than the original refusal"
    );
    assert_eq!(replay_status, status);
}

/// The adapter's refused rate limit names when to try again; the answer
/// drops it, so the caller of a refused write learns `rate_limited` and no
/// delay, though the adapter knew one (compatibility § 5, `Error`).
#[tokio::test]
async fn adversary_a_refused_rate_limit_keeps_its_retry_after() {
    let host = host(false, 100).await;
    let (status, response) = invoke(&host, &merge("rate-1", "k-rate", "rate")).await;
    assert_eq!(status, 429, "{response:?}");
    assert_eq!(
        response.mutation.as_ref().map(|m| m.classification),
        Some(EffectKnowledge::Refused)
    );
    let error = response.error.expect("the refusal");
    assert_eq!(error.code, ErrorCode::RateLimited);
    assert_eq!(
        error.retry_after_seconds,
        Some(30),
        "the provider's delay is lost"
    );
}

/// A concurrent request with the same key while the refused original is in
/// flight waits for it and answers the same refusal, replayed, with one
/// provider request; the attempt is Failed and both anchors record refused.
#[tokio::test]
async fn adversary_a_concurrent_duplicate_of_a_refused_write_answers_the_refusal() {
    let host = Arc::new(host(false, 100).await);
    let hold = host.gate.clone().write_owned().await;
    let first = {
        let host = host.clone();
        tokio::spawn(async move { invoke(&host, &merge("dup-1", "k-dup", "refuse")).await })
    };
    until("the original reaches the provider", || host.requests() == 1).await;
    let second = {
        let host = host.clone();
        tokio::spawn(async move { invoke(&host, &merge("dup-2", "k-dup", "refuse")).await })
    };
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!second.is_finished(), "the duplicate waits on the original");
    drop(hold);
    let (status, original) = first.await.unwrap();
    let (replay_status, replay) = second.await.unwrap();
    assert_eq!(status, 403, "{original:?}");
    assert_eq!(replay_status, 403, "{replay:?}");
    let error = original.error.clone().expect("the refusal");
    assert_eq!(error.code, ErrorCode::Forbidden);
    assert_eq!(error.message, REFUSAL);
    assert_eq!(replay.error, original.error);
    let (mutation, replayed) = (original.mutation.unwrap(), replay.mutation.unwrap());
    assert_eq!(mutation.classification, EffectKnowledge::Refused);
    assert_eq!(replayed.classification, EffectKnowledge::Refused);
    assert!(!mutation.replayed && replayed.replayed);
    assert_eq!(replayed.attempt, mutation.attempt);
    assert_eq!(replayed.original_request_id.as_deref(), Some("dup-1"));
    assert_eq!(host.requests(), 1, "the duplicate sent nothing");
    let attempt = host
        .state
        .attempt(mutation.attempt.unwrap().id.as_str())
        .unwrap();
    assert_eq!(attempt.state, mutations::State::Failed);
    assert_eq!(
        final_outcome(&host, original.audit_ref.as_deref().unwrap()),
        audit::Outcome::Refused
    );
    assert_eq!(
        final_outcome(&host, replay.audit_ref.as_deref().unwrap()),
        audit::Outcome::Refused
    );
}

/// The key of a refused write is spent: other input under it is
/// `idempotency_conflict` with no request, and the caller retries the fixed
/// write under a new key, which is a new attempt (and a new grant use).
#[tokio::test]
async fn adversary_a_refused_key_is_spent_and_a_new_key_is_a_new_attempt() {
    let host = host(false, 2).await;
    let (_, refused) = invoke(&host, &merge("spent-1", "k-spent", "refuse")).await;
    assert_eq!(
        refused.mutation.as_ref().map(|m| m.classification),
        Some(EffectKnowledge::Refused)
    );
    let (status, conflict) = invoke(&host, &merge("spent-2", "k-spent", "apply")).await;
    assert_eq!(status, 409, "{conflict:?}");
    assert_eq!(
        conflict.error.as_ref().map(|e| e.code),
        Some(ErrorCode::IdempotencyConflict)
    );
    assert!(conflict.mutation.is_none());
    assert_eq!(host.requests(), 1);
    let (status, applied) = invoke(&host, &merge("spent-3", "k-fresh", "apply")).await;
    assert_eq!(status, 200, "{applied:?}");
    assert_eq!(applied.status, ResponseStatus::Success);
    let mutation = applied.mutation.unwrap();
    assert_eq!(mutation.classification, EffectKnowledge::Applied);
    assert!(!mutation.replayed);
    assert_eq!(host.requests(), 2);
    // The refused write counted a use: the hour's two are spent.
    let (status, limited) = invoke(&host, &merge("spent-4", "k-third", "apply")).await;
    assert_eq!(status, 429, "{limited:?}");
    assert_eq!(host.requests(), 2);
}

/// An adapter that keeps the default port cannot say refused: its error
/// after dispatch stays `outcome_unknown` / `unknown`, and its replay too.
#[tokio::test]
async fn adversary_an_adapter_on_the_default_port_stays_unknown() {
    let host = host(true, 100).await;
    let (status, response) = invoke(&host, &merge("default-1", "k-default", "refuse")).await;
    assert_eq!(status, 502, "{response:?}");
    assert_eq!(
        response.error.as_ref().map(|e| e.code),
        Some(ErrorCode::OutcomeUnknown)
    );
    let mutation = response.mutation.unwrap();
    assert_eq!(mutation.classification, EffectKnowledge::Unknown);
    let (status, replay) = invoke(&host, &merge("default-2", "k-default", "refuse")).await;
    assert_eq!(status, 502, "{replay:?}");
    assert_eq!(
        replay.mutation.map(|m| (m.classification, m.replayed)),
        Some((EffectKnowledge::Unknown, true))
    );
    assert_eq!(host.requests(), 1);
}

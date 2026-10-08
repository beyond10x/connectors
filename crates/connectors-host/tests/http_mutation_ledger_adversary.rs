//! Adversarial cases for the HTTP host's mutation attempt ledger on
//! `POST /v1alpha2/invoke` (contracts/service/compatibility.md § 2.1,
//! contracts/service/audit.md § 1-3, ess/domains/mutations.yaml "The HTTP
//! host's attempt ledger", ess/domains/execution_audit.yaml "Final-observation
//! recovery").
use async_trait::async_trait;
use connectors_core::{
    Descriptor, Operation, Result, WIRE_VERSION,
    v1alpha2::{
        CauseStage, EffectKnowledge, ErrorCode as WireCode, InvokeResponse, ResponseStatus,
    },
};
use connectors_host::{
    local::{
        audit::{self, Reference},
        mutations,
    },
    server::{ServiceState, router_with_state},
};
use connectors_sdk::{Adapter, Credential, Secret};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

const INSTANCE: &str = "leaf";
const TOKEN: &str = "service-token";

struct FixedSecret;
#[async_trait]
impl Credential for FixedSecret {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(TOKEN.as_bytes().to_vec()))
    }
}

/// `write` applies and echoes; `panicking-write` is dispatched and the adapter
/// panics before it answers (a provider-response parsing bug, an `unwrap` on a
/// provider body): the answer of a dispatched write is lost.
struct Fixture {
    calls: Arc<AtomicUsize>,
}
fn operation(id: &str) -> Operation {
    Operation {
        id: id.into(),
        description: format!("{id} fixture"),
        contract: "operations/v1alpha1".into(),
        profile: "mutation".into(),
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
            operations: vec![operation("write"), operation("panicking-write")],
        }
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if operation == "panicking-write" {
            panic!("provider body did not parse");
        }
        Ok(input)
    }
}

fn private(state: &Path, usable: bool) {
    let mode = if usable { 0o700 } else { 0o750 };
    std::fs::set_permissions(state, std::fs::Permissions::from_mode(mode)).unwrap();
}

struct Host {
    endpoint: String,
    task: tokio::task::JoinHandle<()>,
    state: Arc<ServiceState>,
    path: PathBuf,
    calls: Arc<AtomicUsize>,
    _root: tempfile::TempDir,
}
async fn host() -> Host {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    std::fs::create_dir(&path).unwrap();
    private(&path, true);
    let calls = Arc::new(AtomicUsize::new(0));
    let adapter = Arc::new(Fixture {
        calls: calls.clone(),
    });
    let state = Arc::new(ServiceState::open(&path, INSTANCE, "fixture", "rev-1").unwrap());
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
        calls,
        _root: root,
    }
}
fn request(operation: &str, request_id: &str) -> Value {
    json!({"version":"v1alpha2","request_id":request_id,"operation":operation,"revision":"rev-1","input":{"value":1}})
}
async fn invoke(host: &Host, body: Value) -> (u16, InvokeResponse) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap()
        .post(&host.endpoint)
        .bearer_auth(TOKEN)
        .body(serde_json::to_vec(&body).unwrap())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.bytes().await.unwrap();
    (status, InvokeResponse::decode(&bytes).unwrap())
}
fn audit_record(path: &Path, audit_ref: &str) -> audit::Record {
    audit::Store::new(path, 100_000)
        .unwrap()
        .observe(&Reference {
            instance: INSTANCE.into(),
            audit_ref: audit_ref.into(),
        })
        .unwrap()
        .expect("the anchored record")
}

// compatibility.md § 2.1 `mutation`: "From that record on, every response
// carries it" and "A dispatched write whose answer is lost is
// `error`/`outcome_unknown`/`unknown` with its attempt and is not retried";
// mutations.yaml: the dispatch, its settlement and the final observation run
// in a host task. An adapter that panics after dispatch loses its answer. The
// caller must learn the outcome is unknown and which attempt it was; an
// `internal` error with no attempt reads as a safe-to-retry failure of a write
// that may have been applied.
#[tokio::test]
async fn a_dispatched_write_whose_adapter_panics_is_outcome_unknown_with_its_attempt() {
    let host = host().await;
    let (status, response) = invoke(&host, request("panicking-write", "panicked-1")).await;
    assert_eq!(
        host.calls.load(Ordering::SeqCst),
        1,
        "the write was dispatched"
    );
    assert_eq!(
        host.state.attempt_count().unwrap(),
        1,
        "its attempt was recorded"
    );
    let code = response.error.as_ref().map(|error| error.code);
    assert!(
        response.mutation.is_some(),
        "a dispatched write with a lost answer answered HTTP {status} {code:?} with no mutation, request_id {:?}, audit_ref {:?}: its recorded attempt is unnamed",
        response.request_id,
        response.audit_ref
    );
    assert_eq!(code, Some(WireCode::OutcomeUnknown));
    let mutation = response.mutation.unwrap();
    assert_eq!(mutation.classification, EffectKnowledge::Unknown);
    let attempt = mutation.attempt.expect("the recorded attempt");
    let record = host.state.attempt(attempt.id.as_str()).unwrap();
    assert_eq!(record.state, mutations::State::Indeterminate);
    host.task.abort();
}

// compatibility.md scenario `v1alpha2-invoke-write-link-failed-not-attempted`:
// "the attempt link fails after the write's attempt is prepared → HTTP 503
// `unavailable`, `mutation.classification: not_attempted` with the attempt, the
// attempt aborted, zero adapter calls; the anchor's final observation records
// the refusal" (audit.md § 1 rule 6). The execution_audit FinalOutcome has a
// `refused` variant, and the local owner records `not_attempted` as `refused`
// (src/local/owner/mutation.rs).
//
// No hook can fail only the link from outside the crate, so a helper thread
// makes the state directory group-readable (every metadata open then fails)
// in short windows while writes run one after another, until a write's
// anchor and attempt were recorded and its link was refused. Afterwards the
// directory is usable again and the running host recovers any retained final
// observation by itself.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_failed_link_is_not_attempted_and_the_final_observation_records_the_refusal() {
    let host = host().await;
    let stop = Arc::new(AtomicBool::new(false));
    // A request that fails the test still stops the flipper.
    struct Stop(Arc<AtomicBool>);
    impl Drop for Stop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let _stop = Stop(stop.clone());
    let flipper = {
        let stop = stop.clone();
        let path = host.path.clone();
        std::thread::spawn(move || {
            // Fixed-seed LCG: windows of 0-2.9 ms closed, 0-5.9 ms open.
            let mut seed: u64 = 0x5eed_1ed6_e7a1_1ed0;
            let mut next = |bound: u64| {
                seed = seed
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                (seed >> 33) % bound
            };
            while !stop.load(Ordering::SeqCst) {
                private(&path, false);
                std::thread::sleep(Duration::from_micros(next(3_000)));
                private(&path, true);
                std::thread::sleep(Duration::from_micros(next(6_000)));
            }
            private(&path, true);
        })
    };

    let mut dispatched = 0;
    let mut found = None;
    // A hard bound besides the count: the search gives up (and the test
    // fails below) after two minutes, whatever each request costs.
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    for index in 0..1_500 {
        if std::time::Instant::now() > deadline {
            break;
        }
        let (status, response) = invoke(&host, request("write", &format!("w-{index}"))).await;
        let Some(mutation) = &response.mutation else {
            continue;
        };
        match mutation.classification {
            EffectKnowledge::Applied | EffectKnowledge::Unknown => dispatched += 1,
            EffectKnowledge::NotAttempted
                if mutation.cause.as_ref().map(|cause| cause.stage) == Some(CauseStage::Audit) =>
            {
                found = Some((status, response));
                break;
            }
            _ => {}
        }
    }
    stop.store(true, Ordering::SeqCst);
    flipper.join().unwrap();
    private(&host.path, true);
    let (status, response) =
        found.expect("no write met a refused link in 1,500 tries or two minutes");

    assert_eq!(status, 503);
    assert_eq!(response.status, ResponseStatus::Error);
    assert_eq!(response.error.as_ref().unwrap().code, WireCode::Unavailable);
    let mutation = response.mutation.clone().unwrap();
    let attempt = mutation.attempt.expect("the prepared attempt");
    assert_eq!(
        mutation.original_request_id.as_deref(),
        Some(response.request_id.as_deref().unwrap())
    );
    assert_eq!(
        host.calls.load(Ordering::SeqCst),
        dispatched,
        "the not-attempted write reached the adapter"
    );
    let record = host.state.attempt(attempt.id.as_str()).unwrap();
    assert!(
        matches!(
            record.state,
            mutations::State::Aborted | mutations::State::Prepared
        ),
        "a not-attempted write's attempt is {:?}",
        record.state
    );

    // The final observation, acknowledged in line or recovered by the host.
    let audit_ref = response.audit_ref.clone().expect("the anchor's reference");
    let mut observation = None;
    for _ in 0..50 {
        observation = audit_record(&host.path, &audit_ref).final_observation;
        if observation.is_some() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let observation = observation.expect("the final observation");
    assert_eq!(
        observation.outcome,
        audit::Outcome::Refused,
        "the anchor of a not-attempted write records {:?} (code {:?}), not the refusal (audit_status {:?})",
        observation.outcome,
        observation.code,
        response.audit_status,
    );
    host.task.abort();
}

// execution_audit.yaml, Final-observation recovery: "Bound: at most 10,000
// retained observations per host, plus the invocations already admitted (at
// most 32 in flight). While the retained set is full the host refuses every
// new v1alpha2 invocation before its anchor". The running host calls
// `recover_observations` every second whenever anything is retained; that
// call takes the whole retained set out of the gate's view while it retries
// it, so a full set against a store that is still unavailable opens
// admission for the length of every recovery pass.
#[test]
fn a_recovery_pass_over_a_full_retained_set_keeps_admission_closed() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    std::fs::create_dir(&path).unwrap();
    private(&path, true);
    let state = Arc::new(ServiceState::open(&path, INSTANCE, "fixture", "rev-1").unwrap());
    private(&path, false);
    let reference = |index: usize| Reference {
        instance: INSTANCE.into(),
        audit_ref: format!("00000000-0000-4000-8000-{index:012}"),
    };
    for index in 0..10_000 {
        assert!(!state.finish(&reference(index), audit::Outcome::Success, None));
    }
    assert_eq!(state.pending_observations(), 10_000);
    assert!(!state.accepting(), "a full retained set admits nothing");

    let recovery = {
        let state = state.clone();
        std::thread::spawn(move || state.recover_observations())
    };
    // What the route does for each request while recovery runs: admit when
    // `accepting()`, and a store still unavailable retains its observation.
    let mut admitted = 0;
    while !recovery.is_finished() {
        if state.accepting() && admitted < 100 {
            admitted += 1;
            let added = reference(10_000 + admitted);
            assert!(!state.finish(&added, audit::Outcome::Success, None));
        }
    }
    let remaining = recovery.join().unwrap();
    assert_eq!(
        admitted, 0,
        "a recovery pass over a full retained set of an unavailable store admitted {admitted} new invocations; {remaining} observations are now retained against a bound of 10,000 + 32"
    );
    assert!(state.pending_observations() <= 10_032);
}

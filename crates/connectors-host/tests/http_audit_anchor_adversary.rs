//! Adversarial cases for the HTTP host's execution-audit anchor on
//! `POST /v1alpha2/invoke` (contracts/service/audit.md § 2-3,
//! contracts/service/v1alpha2/semantics.md § 5-6).
use async_trait::async_trait;
use connectors_core::{
    Descriptor, Operation, Result, WIRE_VERSION,
    v1alpha2::{AuditStatus, InvokeResponse},
};
use connectors_host::{
    local::audit::{self, Reference},
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

/// Set when the adapter's in-flight `invoke` future is dropped before it
/// answers: the host stopped driving a dispatched provider call.
struct Abandoned(Arc<AtomicBool>, bool);
impl Drop for Abandoned {
    fn drop(&mut self) {
        if !self.1 {
            self.0.store(true, Ordering::SeqCst);
        }
    }
}

/// `slow-write` takes 400 ms to answer; `bad-write` applies and then answers
/// a body its declared output schema does not admit.
struct Fixture {
    started: Arc<AtomicUsize>,
    abandoned: Arc<AtomicBool>,
}
fn operation(id: &str, profile: &str, output: Value) -> Operation {
    Operation {
        id: id.into(),
        description: format!("{id} fixture"),
        contract: "operations/v1alpha1".into(),
        profile: profile.into(),
        input_schema: json!({"type":"object"}),
        output_schema: output,
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
                operation("slow-write", "mutation", json!({"type":"object"})),
                operation(
                    "bad-write",
                    "mutation",
                    json!({"type":"object","required":["id"]}),
                ),
            ],
        }
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        self.started.fetch_add(1, Ordering::SeqCst);
        let mut guard = Abandoned(self.abandoned.clone(), false);
        if operation == "slow-write" {
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
        guard.1 = true;
        Ok(input)
    }
}

struct Host {
    endpoint: String,
    task: tokio::task::JoinHandle<()>,
    state: Arc<ServiceState>,
    path: PathBuf,
    started: Arc<AtomicUsize>,
    abandoned: Arc<AtomicBool>,
    _root: tempfile::TempDir,
}
async fn host() -> Host {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let started = Arc::new(AtomicUsize::new(0));
    let abandoned = Arc::new(AtomicBool::new(false));
    let adapter = Arc::new(Fixture {
        started: started.clone(),
        abandoned: abandoned.clone(),
    });
    let descriptor = adapter.descriptor();
    let state = Arc::new(
        ServiceState::open(&path, INSTANCE, &descriptor.adapter, &descriptor.revision).unwrap(),
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
        started,
        abandoned,
        _root: root,
    }
}
fn request(operation: &str, request_id: &str) -> Value {
    json!({"version":"v1alpha2","request_id":request_id,"operation":operation,"revision":"rev-1","input":{"value":1}})
}
fn observe(path: &Path, audit_ref: &str) -> Option<audit::Record> {
    audit::Store::new(path, 100_000)
        .unwrap()
        .observe(&Reference {
            instance: INSTANCE.into(),
            audit_ref: audit_ref.into(),
        })
        .unwrap()
}

// semantics.md § 5: "Audit admission and completion must be bounded inside the
// execution/delivery policy while preserving actual effect knowledge when a
// reply cannot be sent"; audit.md § 3: the owner retains the exact observation
// across acknowledgement uncertainty. A caller that hangs up while its write is
// dispatched must not abandon the provider call midway, which also skips the
// final observation and leaves nothing retained for recovery.
#[tokio::test]
async fn a_write_whose_caller_hangs_up_keeps_its_final_observation() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let host = host().await;
    let sent = reqwest::Client::new()
        .post(&host.endpoint)
        .bearer_auth(TOKEN)
        .timeout(Duration::from_millis(150))
        .body(serde_json::to_vec(&request("slow-write", "hung-up")).unwrap())
        .send()
        .await;
    assert!(sent.is_err(), "the caller gave up before the answer");
    // Well past the adapter's own 400 ms.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(
        host.started.load(Ordering::SeqCst),
        1,
        "the write was dispatched"
    );
    let abandoned = host.abandoned.load(Ordering::SeqCst);
    let retained = host.state.pending_observations();
    assert!(
        !abandoned,
        "the host dropped a dispatched write mid-call when its caller hung up: no final observation was attempted, retained for recovery={retained}"
    );
    host.task.abort();
}

// semantics.md § 6: "an invalid response after a sent mutation yields
// uncertainty, not safe retry permission" and "a possible dispatch remains
// unknown". The adapter applied the write and answered; only the host's output
// check failed, so the audit's final observation cannot say the write ended in
// `error`.
#[tokio::test]
async fn a_write_whose_answer_fails_its_output_schema_is_observed_unknown() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let host = host().await;
    let bytes = reqwest::Client::new()
        .post(&host.endpoint)
        .bearer_auth(TOKEN)
        .body(serde_json::to_vec(&request("bad-write", "invalid-answer")).unwrap())
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let response = InvokeResponse::decode(&bytes).unwrap();
    assert_eq!(
        host.started.load(Ordering::SeqCst),
        1,
        "the write was dispatched"
    );
    assert_eq!(response.audit_status, AuditStatus::Complete);
    let audit_ref = response.audit_ref.expect("an acknowledged reference");
    let observation = observe(&host.path, &audit_ref)
        .expect("the anchored record")
        .final_observation
        .expect("the final observation");
    assert_eq!(
        observation.outcome,
        audit::Outcome::Unknown,
        "a dispatched write with an invalid answer was observed as {:?} code {:?}",
        observation.outcome,
        observation.code
    );
    host.task.abort();
}

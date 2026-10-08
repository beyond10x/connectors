//! The HTTP host's execution-audit anchor on `POST /v1alpha2/invoke`
//! (contracts/service/audit.md § 1-4, contracts/service/compatibility.md § 2.1).
use async_trait::async_trait;
use connectors_core::{
    Descriptor, Operation, Result, WIRE_VERSION,
    v1alpha2::{AuditStatus, ErrorCode, InvokeResponse, ResponseStatus},
};
use connectors_host::{
    local::audit::{self, Reference},
    server::{ServiceState, router, router_with_state},
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

/// A read (`profile: read`) and a write (`profile: mutation`, which an
/// `external_write` operation must declare). `break_store` makes the next
/// dispatch take the state directory away, so the final observation fails.
struct Fixture {
    calls: Arc<AtomicUsize>,
    state: PathBuf,
    break_store: Arc<AtomicBool>,
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
            operations: vec![operation("read", "read"), operation("write", "mutation")],
        }
    }
    async fn invoke(&self, _operation: &str, input: Value) -> Result<Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.break_store.swap(false, Ordering::SeqCst) {
            unusable(&self.state);
        }
        Ok(input)
    }
}

/// The metadata owner refuses a state directory that is not private, so a
/// group-readable mode makes every audit write fail without deleting anything.
fn unusable(state: &Path) {
    std::fs::set_permissions(state, std::fs::Permissions::from_mode(0o750)).unwrap();
}
fn usable(state: &Path) {
    std::fs::set_permissions(state, std::fs::Permissions::from_mode(0o700)).unwrap();
}

struct Host {
    endpoint: String,
    task: tokio::task::JoinHandle<()>,
    state: Arc<ServiceState>,
    path: PathBuf,
    calls: Arc<AtomicUsize>,
    break_store: Arc<AtomicBool>,
    _root: tempfile::TempDir,
}
async fn host() -> Host {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    std::fs::create_dir(&path).unwrap();
    usable(&path);
    let calls = Arc::new(AtomicUsize::new(0));
    let break_store = Arc::new(AtomicBool::new(false));
    let adapter = Arc::new(Fixture {
        calls: calls.clone(),
        state: path.clone(),
        break_store: break_store.clone(),
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
        calls,
        break_store,
        _root: root,
    }
}
async fn invoke(endpoint: &str, body: Value) -> (u16, InvokeResponse) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let response = reqwest::Client::new()
        .post(endpoint)
        .bearer_auth(TOKEN)
        .body(serde_json::to_vec(&body).unwrap())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.bytes().await.unwrap();
    (status, InvokeResponse::decode(&bytes).unwrap())
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

#[tokio::test]
async fn audited_write_answers_complete_with_a_ref_that_reads_back_its_anchor() {
    let host = host().await;
    for (operation, access) in [
        ("write", audit::Access::Write),
        ("read", audit::Access::Read),
    ] {
        let (status, response) = invoke(&host.endpoint, request(operation, operation)).await;
        assert_eq!(status, 200, "{operation}");
        assert_eq!(response.status, ResponseStatus::Success);
        assert_eq!(response.result, Some(json!({"value":1})));
        assert_eq!(response.audit_status, AuditStatus::Complete);
        assert!(response.source_audit.is_none());
        let audit_ref = response.audit_ref.expect("an acknowledged reference");
        let record = observe(&host.path, &audit_ref).expect("the anchored record");
        let anchor = &record.anchor;
        assert_eq!(anchor.instance_id, INSTANCE);
        assert_eq!(anchor.kind, audit::Kind::AdmittedExecution);
        assert_eq!(anchor.activity, Some(audit::Activity::Invoke));
        assert_eq!(anchor.access, Some(access));
        assert_eq!(anchor.hop, audit::Hop::Execution);
        assert_eq!(anchor.stage, audit::Stage::Admission);
        assert_eq!(anchor.request_id.as_deref(), Some(operation));
        assert_eq!(anchor.principal_ref.as_deref(), Some("static-bearer"));
        assert_eq!(anchor.operation_id.as_deref(), Some(operation));
        assert_eq!(anchor.descriptor_revision.as_deref(), Some("rev-1"));
        assert_eq!(anchor.connection_ref, None);
        let observed = record.final_observation.expect("the final observation");
        assert_eq!(observed.outcome, audit::Outcome::Success);
        assert_eq!(observed.code, None);
    }
    assert_eq!(host.calls.load(Ordering::SeqCst), 2);
    host.task.abort();
}

#[tokio::test]
async fn an_anchor_that_cannot_be_written_refuses_before_any_adapter_call() {
    let host = host().await;
    unusable(&host.path);
    for operation in ["write", "read"] {
        let (status, response) = invoke(&host.endpoint, request(operation, "refused")).await;
        assert_eq!(status, 503, "{operation}");
        assert_eq!(response.status, ResponseStatus::Error);
        assert_eq!(response.error.unwrap().code, ErrorCode::Unavailable);
        assert_eq!(response.request_id.as_deref(), Some("refused"));
        assert_eq!(response.audit_ref, None);
        assert_eq!(response.audit_status, AuditStatus::Unavailable);
        assert!(response.mutation.is_none());
        assert!(response.result.is_none());
    }
    assert_eq!(host.calls.load(Ordering::SeqCst), 0);
    // The store coming back is what admits the next write, not the refusal.
    usable(&host.path);
    let (status, response) = invoke(&host.endpoint, request("write", "after")).await;
    assert_eq!(status, 200);
    assert_eq!(response.audit_status, AuditStatus::Complete);
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    host.task.abort();
}

#[tokio::test]
async fn a_lost_final_observation_answers_incomplete_and_recovers_once() {
    let host = host().await;
    host.break_store.store(true, Ordering::SeqCst);
    let (status, response) = invoke(&host.endpoint, request("write", "lost")).await;
    // The known result is kept; only the audit observation is unconfirmed.
    assert_eq!(status, 200);
    assert_eq!(response.status, ResponseStatus::Success);
    assert_eq!(response.result, Some(json!({"value":1})));
    assert_eq!(response.audit_status, AuditStatus::Incomplete);
    let audit_ref = response.audit_ref.expect("the anchor's reference");
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.state.pending_observations(), 1);
    usable(&host.path);
    let anchored = observe(&host.path, &audit_ref).unwrap();
    assert_eq!(anchored.final_observation, None);
    assert_eq!(anchored.anchor.access, Some(audit::Access::Write));

    assert_eq!(host.state.recover_observations(), 0);
    let recovered = observe(&host.path, &audit_ref).unwrap();
    let observation = recovered.final_observation.clone().expect("recovered");
    assert_eq!(observation.outcome, audit::Outcome::Success);
    assert_eq!(recovered.anchor, anchored.anchor);
    // Recovery again appends nothing and never redispatches.
    assert_eq!(host.state.recover_observations(), 0);
    assert_eq!(observe(&host.path, &audit_ref).unwrap(), recovered);
    assert_eq!(host.calls.load(Ordering::SeqCst), 1);
    host.task.abort();
}

#[tokio::test]
async fn a_host_without_state_answers_unavailable_before_decoding() {
    let calls = Arc::new(AtomicUsize::new(0));
    let adapter = Arc::new(Fixture {
        calls: calls.clone(),
        state: PathBuf::from("/nonexistent"),
        break_store: Arc::new(AtomicBool::new(false)),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, router(adapter, Arc::new(FixedSecret)))
            .await
            .unwrap();
    });
    let endpoint = format!("http://{address}/v1alpha2/invoke");
    for body in [request("write", "no-state"), json!("not an envelope")] {
        let (status, response) = invoke(&endpoint, body).await;
        assert_eq!(status, 503);
        assert_eq!(response.error.unwrap().code, ErrorCode::Unavailable);
        assert_eq!(response.request_id, None);
        assert_eq!(response.audit_ref, None);
        assert_eq!(response.audit_status, AuditStatus::Unavailable);
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    task.abort();
}

#[test]
fn host_start_records_its_instance_once_and_refuses_another_adapter() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    std::fs::create_dir(&path).unwrap();
    usable(&path);
    drop(ServiceState::open(&path, INSTANCE, "fixture", "rev-1").unwrap());
    drop(ServiceState::open(&path, INSTANCE, "fixture", "rev-1").unwrap());
    // A new configuration revision of the same adapter moves the instance.
    drop(ServiceState::open(&path, INSTANCE, "fixture", "rev-2").unwrap());
    assert!(ServiceState::open(&path, INSTANCE, "other", "rev-2").is_err());
    drop(ServiceState::open(&path, INSTANCE, "fixture", "rev-2").unwrap());
}

#[test]
fn service_configuration_v2_adds_state_and_v1_stays_closed() {
    let config = json!({"instance":"leaf","listen":"127.0.0.1:0","service_credential":{"kind":"file","path":"token"}});
    let mut stated = config.clone();
    stated["state"] = json!("/var/lib/connectors");
    let schema = |reference: &str| {
        let mut value = json!({"$ref": reference});
        connectors_host::schema::expand(&mut value).unwrap();
        value
    };
    let v1 = schema("urn:connectors:config:v1:service");
    let v2 = schema("urn:connectors:config:v2:service");
    connectors_sdk::validate(&v1, &config).unwrap();
    assert!(connectors_sdk::validate(&v1, &stated).is_err());
    connectors_sdk::validate(&v2, &config).unwrap();
    connectors_sdk::validate(&v2, &stated).unwrap();
}

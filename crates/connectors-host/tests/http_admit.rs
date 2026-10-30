//! `Adapter::admit` on `POST /v1alpha2/invoke`: a refusal the adapter answers
//! from the validated input alone, before the host anchors the call
//! (story:catalog-reads-stay-inside-the-instance-repositories). A refused
//! call leaves no anchor, no attempt and no key; an adapter with the default
//! `admit` is dispatched as before.
//!
//! Unix only: its fixtures use Unix mode bits.
#![cfg(unix)]
use async_trait::async_trait;
use connectors_core::{
    Descriptor, Error, Operation, Result, WIRE_VERSION,
    v1alpha2::{AuditStatus, ErrorCode, InvokeResponse, ResponseStatus},
};
use connectors_host::server::{ServiceState, router_with_state};
use connectors_sdk::{Adapter, Credential, Secret};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

const INSTANCE: &str = "leaf";
const TOKEN: &str = "service-token";
/// 2033-05-18: after every case.
const LATER: i64 = 2_000_000_000_000;

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
        operations: vec![
            operation("item.read", "read"),
            operation("item.create", "mutation"),
        ],
    }
}

/// Counts its calls; while `refusing`, `admit` refuses every call
/// `not_found`.
struct Refusing {
    calls: Arc<AtomicUsize>,
    admitted: Arc<AtomicUsize>,
    refusing: Arc<AtomicBool>,
}
#[async_trait]
impl Adapter for Refusing {
    fn descriptor(&self) -> Descriptor {
        descriptor()
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(json!({"operation": operation, "input": input}))
    }
    fn admit(&self, _operation: &str, _input: &Value) -> Result<()> {
        self.admitted.fetch_add(1, Ordering::SeqCst);
        if self.refusing.load(Ordering::SeqCst) {
            return Err(Error::new(
                connectors_core::ErrorCode::NotFound,
                "fixture: not listed",
            ));
        }
        Ok(())
    }
}

/// The default `admit`.
struct Plain {
    calls: Arc<AtomicUsize>,
}
#[async_trait]
impl Adapter for Plain {
    fn descriptor(&self) -> Descriptor {
        descriptor()
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(json!({"operation": operation, "input": input}))
    }
}

struct Host {
    endpoint: String,
    task: tokio::task::JoinHandle<()>,
    state: Arc<ServiceState>,
    _root: tempfile::TempDir,
}
impl Drop for Host {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// A keyed host serving `adapter`, with one grant for every `item.create`.
async fn host(adapter: Arc<dyn Adapter>) -> Host {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let grants = root.path().join("grants.json");
    let document = json!({
        "format": "connectors.standing-grants/1",
        "grants": [{
            "id": "g-items", "instance": INSTANCE, "operation": "item.create",
            "target": [{"pointer": "/owner", "value": "example-org"}],
            "not_after_unix_ms": LATER, "max_per_hour": 10
        }],
        "revoked": []
    });
    std::fs::write(&grants, serde_json::to_vec(&document).unwrap()).unwrap();
    std::fs::set_permissions(&grants, std::fs::Permissions::from_mode(0o644)).unwrap();
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
        _root: root,
    }
}

async fn invoke(host: &Host, body: &Value) -> (u16, InvokeResponse) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let response = reqwest::Client::new()
        .post(&host.endpoint)
        .bearer_auth(TOKEN)
        .body(serde_json::to_vec(body).unwrap())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let bytes = response.bytes().await.unwrap();
    (status, InvokeResponse::decode_keyed(&bytes).unwrap())
}

fn request(operation: &str, request_id: &str, key: Option<&str>) -> Value {
    let mut body = json!({
        "version": "v1alpha2", "request_id": request_id, "operation": operation,
        "revision": "rev-1", "input": {"owner": "example-org", "repo": "service"},
    });
    if let Some(key) = key {
        body["idempotency_key"] = json!(key);
    }
    body
}

/// A refusal in `admit`: `not_found` with the adapter's message, no anchor
/// (`audit_ref` null, audit `unavailable`), no `mutation`, no attempt, no
/// adapter call and no key used: the same key is refused again, not
/// replayed, and once `admit` lets it through it is a new write.
#[tokio::test]
async fn a_refusal_in_admit_leaves_no_record_and_no_key() {
    let calls = Arc::new(AtomicUsize::new(0));
    let admitted = Arc::new(AtomicUsize::new(0));
    let refusing = Arc::new(AtomicBool::new(true));
    let host = host(Arc::new(Refusing {
        calls: calls.clone(),
        admitted: admitted.clone(),
        refusing: refusing.clone(),
    }))
    .await;
    for body in [
        request("item.create", "w-1", Some("k-1")),
        request("item.create", "w-1-again", Some("k-1")),
        request("item.read", "r-1", None),
    ] {
        let (status, response) = invoke(&host, &body).await;
        assert_eq!(status, 404, "{body}: {response:?}");
        assert_eq!(response.status, ResponseStatus::Error);
        let error = response.error.as_ref().unwrap();
        assert_eq!(error.code, ErrorCode::NotFound, "{body}");
        assert_eq!(error.message, "fixture: not listed", "{body}");
        assert_eq!(response.audit_status, AuditStatus::Unavailable, "{body}");
        assert!(response.audit_ref.is_none(), "{body}: {response:?}");
        assert!(response.mutation.is_none(), "{body}: {response:?}");
    }
    assert_eq!(admitted.load(Ordering::SeqCst), 3);
    assert_eq!(host.state.attempt_count().unwrap(), 0);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "no refused call reached the adapter"
    );

    // The key was never reserved: admitted now, it is a new write.
    refusing.store(false, Ordering::SeqCst);
    let (status, response) = invoke(&host, &request("item.create", "w-2", Some("k-1"))).await;
    assert_eq!(status, 200, "{response:?}");
    let mutation = response.mutation.as_ref().unwrap();
    assert!(!mutation.replayed, "{response:?}");
    assert!(response.audit_ref.is_some(), "{response:?}");
    assert_eq!(host.state.attempt_count().unwrap(), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

/// The default `admit` refuses nothing: a read and a keyed write are anchored
/// and dispatched as before, and the same key again is a replay.
#[tokio::test]
async fn the_default_admit_dispatches_as_before() {
    let calls = Arc::new(AtomicUsize::new(0));
    let host = host(Arc::new(Plain {
        calls: calls.clone(),
    }))
    .await;
    let (status, read) = invoke(&host, &request("item.read", "r-1", None)).await;
    assert_eq!(status, 200, "{read:?}");
    assert!(
        read.audit_ref.is_some() && read.mutation.is_none(),
        "{read:?}"
    );
    let (status, write) = invoke(&host, &request("item.create", "w-1", Some("k-1"))).await;
    assert_eq!(status, 200, "{write:?}");
    assert!(!write.mutation.as_ref().unwrap().replayed, "{write:?}");
    let (status, replay) = invoke(&host, &request("item.create", "w-1-again", Some("k-1"))).await;
    assert_eq!(status, 200, "{replay:?}");
    assert!(replay.mutation.as_ref().unwrap().replayed, "{replay:?}");
    assert_eq!(host.state.attempt_count().unwrap(), 1);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

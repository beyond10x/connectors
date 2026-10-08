//! A real HTTP host with a local metadata authority, serving `/v1/*` and
//! `POST /v1alpha2/invoke` over a fixture adapter.
#![allow(dead_code)]
use async_trait::async_trait;
use connectors_core::{Descriptor, Operation, Result, WIRE_VERSION};
use connectors_host::server::{ServiceState, router_with_state};
use connectors_sdk::{Adapter, Credential, Secret};
use serde_json::{Value, json};
use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

pub const TOKEN: &str = "service-token";

struct FixedSecret;
#[async_trait]
impl Credential for FixedSecret {
    async fn resolve(&self) -> Result<Secret> {
        Ok(Secret(TOKEN.as_bytes().to_vec()))
    }
}

/// `write` (profile `mutation`, so `external_write`) applies and echoes its
/// input; `read` echoes it. Counts every dispatch.
pub struct Fixture(pub Arc<AtomicUsize>);
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
            instance: "leaf".into(),
            adapter: "fixture".into(),
            revision: "rev-1".into(),
            configuration_schema: json!({"type":"object"}),
            operations: vec![operation("write", "mutation"), operation("read", "read")],
        }
    }
    async fn invoke(&self, _operation: &str, input: Value) -> Result<Value> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(input)
    }
}

pub struct Host {
    pub endpoint: String,
    pub state: Arc<ServiceState>,
    pub path: PathBuf,
    pub calls: Arc<AtomicUsize>,
    pub task: tokio::task::JoinHandle<()>,
    _root: tempfile::TempDir,
}

impl Host {
    pub async fn start() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state");
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let state = Arc::new(ServiceState::open(&path, "leaf", "fixture", "rev-1").unwrap());
        let app = router_with_state(
            Arc::new(Fixture(calls.clone())),
            Arc::new(FixedSecret),
            Some(state.clone()),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Self {
            endpoint: format!("http://{address}/"),
            state,
            path,
            calls,
            task,
            _root: root,
        }
    }
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.task.abort();
    }
}

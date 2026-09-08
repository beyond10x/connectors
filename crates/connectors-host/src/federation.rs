use crate::{credentials::CredentialRef, server::ServiceConfig};
use async_trait::async_trait;
use connectors_client::Endpoint;
use connectors_core::{Descriptor, Error, ErrorCode, Result, WIRE_VERSION, digest};
use connectors_sdk::{Adapter, Credential};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownstreamConfig {
    pub name: String,
    pub endpoint: String,
    pub credential: CredentialRef,
    #[serde(default)]
    pub allow_plaintext: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FederationConfig {
    pub service: ServiceConfig,
    pub downstreams: Vec<DownstreamConfig>,
}
struct Leaf {
    endpoint: Endpoint,
    descriptor: Descriptor,
    config: DownstreamConfig,
}
struct Snapshot {
    descriptor: Descriptor,
    leaves: BTreeMap<String, Arc<Leaf>>,
    routes: BTreeMap<String, (Arc<Leaf>, String)>,
}
pub struct Federation {
    config: FederationConfig,
    snapshot: RwLock<Arc<Snapshot>>,
    refresh_lock: tokio::sync::Mutex<()>,
}
impl Federation {
    pub async fn connect(config: &FederationConfig) -> Result<Self> {
        if !connectors_core::valid_id(&config.service.instance) {
            return Err(Error::invalid("invalid federation instance identity"));
        }
        if config.downstreams.is_empty() || config.downstreams.len() > 32 {
            return Err(Error::invalid(
                "configure between one and 32 downstream services",
            ));
        }
        let mut leaves = BTreeMap::new();
        for downstream in &config.downstreams {
            if !connectors_core::valid_id(&downstream.name) || downstream.name.contains("__") {
                return Err(Error::invalid("invalid downstream route name"));
            }
            if leaves.contains_key(&downstream.name) {
                return Err(Error::invalid("duplicate route name"));
            }
            let endpoint = Endpoint::new(&downstream.endpoint, downstream.allow_plaintext)?;
            let descriptor = endpoint
                .with_token(token(&downstream.credential).await?)?
                .describe()
                .await?;
            leaves.insert(
                downstream.name.clone(),
                Arc::new(Leaf {
                    endpoint,
                    descriptor,
                    config: downstream.clone(),
                }),
            );
        }
        let snapshot = snapshot(config, leaves)?;
        Ok(Self {
            config: config.clone(),
            snapshot: RwLock::new(Arc::new(snapshot)),
            refresh_lock: tokio::sync::Mutex::new(()),
        })
    }
    fn current(&self) -> Arc<Snapshot> {
        self.snapshot
            .read()
            .expect("snapshot lock poisoned")
            .clone()
    }
    async fn refresh(&self, stale: &Leaf) -> Result<()> {
        let _guard = self.refresh_lock.lock().await;
        let current = self.current();
        if current.leaves[&stale.config.name].descriptor.revision != stale.descriptor.revision {
            return Ok(());
        }
        let descriptor = stale
            .endpoint
            .with_token(token(&stale.config.credential).await?)?
            .describe()
            .await?;
        let mut leaves = current.leaves.clone();
        leaves.insert(
            stale.config.name.clone(),
            Arc::new(Leaf {
                descriptor,
                endpoint: stale.endpoint.clone(),
                config: stale.config.clone(),
            }),
        );
        let next = snapshot(&self.config, leaves)?;
        *self.snapshot.write().expect("snapshot lock poisoned") = Arc::new(next);
        Ok(())
    }
}
async fn token(credential: &CredentialRef) -> Result<String> {
    String::from_utf8(credential.resolve().await?.0)
        .map_err(|_| Error::invalid("invalid downstream credential"))
}
fn snapshot(config: &FederationConfig, leaves: BTreeMap<String, Arc<Leaf>>) -> Result<Snapshot> {
    let mut routes = BTreeMap::new();
    let mut operations = Vec::new();
    let mut sources = BTreeMap::new();
    for (name, leaf) in &leaves {
        if leaf.descriptor.instance == config.service.instance
            || leaf.descriptor.adapter == "federation"
        {
            return Err(Error::new(
                ErrorCode::Unsupported,
                "this profile supports a single federation hop",
            ));
        }
        sources.insert(name, &leaf.descriptor.revision);
        for operation in &leaf.descriptor.operations {
            let mut projected = operation.clone();
            projected.id = format!("{name}__{}", operation.id);
            if !connectors_core::valid_id(&projected.id) || routes.contains_key(&projected.id) {
                return Err(Error::invalid(
                    "invalid or duplicate federated operation identity",
                ));
            }
            routes.insert(projected.id.clone(), (leaf.clone(), operation.id.clone()));
            operations.push(projected);
        }
    }
    let descriptor = Descriptor {
        version: WIRE_VERSION.into(),
        instance: config.service.instance.clone(),
        adapter: "federation".into(),
        revision: digest(&json!({"config":config,"sources":sources})),
        operations,
        configuration_schema: json!({"type":"object"}),
    };
    Ok(Snapshot {
        descriptor,
        leaves,
        routes,
    })
}
#[async_trait]
impl Adapter for Federation {
    fn descriptor(&self) -> Descriptor {
        self.current().descriptor.clone()
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        self.invoke_at(&self.current().descriptor.revision, operation, input)
            .await
    }
    async fn invoke_at(&self, revision: &str, operation: &str, input: Value) -> Result<Value> {
        let current = self.current();
        if revision != current.descriptor.revision {
            return Err(Error::new(
                ErrorCode::StaleDescription,
                "refresh the descriptor before resubmitting",
            ));
        }
        let (leaf, downstream_operation) = current
            .routes
            .get(operation)
            .ok_or_else(|| Error::new(ErrorCode::NotFound, "unknown route"))?;
        let result = leaf
            .endpoint
            .with_token(token(&leaf.config.credential).await?)?
            .invoke(&leaf.descriptor, downstream_operation, input)
            .await;
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::StaleDescription)
        {
            // Publish a complete validated snapshot; never replay an invocation.
            self.refresh(leaf).await?;
        }
        result
    }
}

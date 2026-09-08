use crate::{credentials::CredentialRef, server::ServiceConfig};
use async_trait::async_trait;
use connectors_client::Client;
use connectors_core::{Descriptor, Error, ErrorCode, Result, WIRE_VERSION, digest};
use connectors_sdk::{Adapter, Credential};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

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

pub struct Federation {
    descriptor: Descriptor,
    routes: BTreeMap<String, (Client, Descriptor, String, CredentialRef)>,
}
impl Federation {
    pub async fn connect(config: &FederationConfig) -> Result<Self> {
        if !connectors_core::valid_id(&config.service.instance) {
            return Err(Error::invalid("invalid federation instance identity"));
        }
        let mut routes = BTreeMap::new();
        let mut operations = Vec::new();
        let mut sources = BTreeMap::new();
        if config.downstreams.is_empty() || config.downstreams.len() > 32 {
            return Err(Error::invalid(
                "configure between one and 32 downstream services",
            ));
        }
        for downstream in &config.downstreams {
            if !connectors_core::valid_id(&downstream.name) || downstream.name.contains("__") {
                return Err(Error::invalid("invalid downstream route name"));
            }
            let token = String::from_utf8(downstream.credential.resolve().await?.0)
                .map_err(|_| Error::invalid("invalid downstream credential"))?;
            let client = Client::new(&downstream.endpoint, token, downstream.allow_plaintext)?;
            let descriptor = client.describe().await?;
            if descriptor.instance == config.service.instance || descriptor.adapter == "federation"
            {
                return Err(Error::new(
                    ErrorCode::Unsupported,
                    "this profile supports a single federation hop",
                ));
            }
            if sources
                .insert(downstream.name.clone(), descriptor.revision.clone())
                .is_some()
            {
                return Err(Error::invalid("duplicate route name"));
            }
            for operation in &descriptor.operations {
                let mut projected = operation.clone();
                projected.id = format!("{}__{}", downstream.name, operation.id);
                if !connectors_core::valid_id(&projected.id) {
                    return Err(Error::invalid(
                        "federated operation identity exceeds bounds",
                    ));
                }
                routes.insert(
                    projected.id.clone(),
                    (
                        client.clone(),
                        descriptor.clone(),
                        operation.id.clone(),
                        downstream.credential.clone(),
                    ),
                );
                operations.push(projected);
            }
        }
        let revision = digest(&json!({"config":config,"sources":sources}));
        let descriptor = Descriptor {
            version: WIRE_VERSION.into(),
            instance: config.service.instance.clone(),
            adapter: "federation".into(),
            revision,
            operations,
            configuration_schema: json!({"type":"object"}),
        };
        Ok(Self { descriptor, routes })
    }
}
#[async_trait]
impl Adapter for Federation {
    fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        let (client, descriptor, downstream_operation, credential) = self
            .routes
            .get(operation)
            .ok_or_else(|| Error::new(ErrorCode::NotFound, "unknown route"))?;
        let token = String::from_utf8(credential.resolve().await?.0)
            .map_err(|_| Error::invalid("invalid downstream credential"))?;
        client
            .with_token(token)?
            .invoke(descriptor, downstream_operation, input)
            .await
    }
}

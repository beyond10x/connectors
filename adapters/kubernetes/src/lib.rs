use async_trait::async_trait;
use connectors_contracts::{EndpointObservation, Page};
use connectors_core::{Descriptor, Error, ErrorCode, Result};
use connectors_sdk::{
    Adapter, AuthenticatedHttp, Cursors, decode, encode, instance_descriptor, provenance,
    upstream_json,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

pub mod auth;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub namespaces: Vec<String>,
    pub resource_kinds: Vec<String>,
    #[serde(default)]
    pub discover_hosts: bool,
}
pub struct Kubernetes {
    http: Arc<dyn AuthenticatedHttp>,
    config: Config,
    descriptor: Descriptor,
    cursors: Arc<Cursors>,
    partition: Option<String>,
}

impl Kubernetes {
    pub fn new(
        instance: &str,
        config: Config,
        effective_configuration: Value,
        http: Arc<dyn AuthenticatedHttp>,
    ) -> Result<Self> {
        if config.namespaces.is_empty()
            || config.namespaces.iter().any(|n| {
                n.is_empty()
                    || n.len() > 63
                    || !n
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            })
            || config.resource_kinds.iter().any(|k| {
                !matches!(
                    k.as_str(),
                    "pods" | "services" | "deployments" | "endpointslices"
                )
            })
        {
            return Err(Error::invalid("invalid namespace or resource-kind scope"));
        }
        let mut descriptor = instance_descriptor(
            include_str!("../generated/descriptor.json"),
            instance,
            &effective_configuration,
        )?;
        connectors_sdk::verify_handlers(
            &descriptor,
            &["resources.list", "endpoints.discover", "hosts.discover"],
        )?;
        if !config.discover_hosts {
            descriptor.operations.retain(|o| o.id != "hosts.discover");
        }
        Ok(Self {
            http,
            config,
            descriptor,
            cursors: Arc::new(Cursors::default()),
            partition: None,
        })
    }
    /// A new immutable HTTP capability for one host-admitted use. Cursor state
    /// stays adapter-owned and is partitioned by an opaque authenticated binding,
    /// so a continuation issued under one connection is not readable under
    /// another. This value grants no credential lookup or dispatch authority.
    pub fn with_authenticated_http(
        &self,
        http: Arc<dyn AuthenticatedHttp>,
        partition: &str,
    ) -> Result<Self> {
        if !connectors_core::valid_id(partition) {
            return Err(Error::invalid("invalid cursor partition"));
        }
        Ok(Self {
            http,
            config: self.config.clone(),
            descriptor: self.descriptor.clone(),
            cursors: self.cursors.clone(),
            partition: Some(partition.to_owned()),
        })
    }
    fn namespace(&self, namespace: &str) -> Result<()> {
        if !self.config.namespaces.iter().any(|n| n == namespace) {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "namespace is outside configured scope",
            ));
        }
        Ok(())
    }
    async fn list(
        &self,
        operation: &str,
        namespace: &str,
        kind: &str,
        limit: u16,
        cursor: Option<&str>,
    ) -> Result<Page> {
        if !(1..=100).contains(&limit) {
            return Err(Error::invalid("limit must be between one and 100"));
        }
        if kind != "nodes" {
            self.namespace(namespace)?;
        }
        let mut context = json!({"instance":self.descriptor.instance,"revision":self.descriptor.revision,"operation":operation,"namespace":namespace,"kind":kind,"limit":limit});
        if let Some(partition) = &self.partition {
            context["partition"] = json!(partition);
        }
        let mut query = vec![("limit", limit.to_string())];
        if let Some(cursor) = cursor {
            query.push(("continue", self.cursors.read(&context, cursor)?));
        }
        let segments = match kind {
            "nodes" => vec!["api", "v1", "nodes"],
            "endpointslices" => vec![
                "apis",
                "discovery.k8s.io",
                "v1",
                "namespaces",
                namespace,
                "endpointslices",
            ],
            "deployments" => vec!["apis", "apps", "v1", "namespaces", namespace, "deployments"],
            "pods" | "services" => vec!["api", "v1", "namespaces", namespace, kind],
            _ => {
                return Err(Error::new(
                    ErrorCode::Unsupported,
                    "resource kind not supported",
                ));
            }
        };
        let response = self.http.get(&segments, &query).await?;
        let value = upstream_json(&response)?;
        let items = value["items"]
            .as_array()
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::UpstreamProtocol,
                    "invalid Kubernetes list response",
                )
            })?
            .clone();
        if items.len() > limit as usize {
            return Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "Kubernetes exceeded requested page size",
            ));
        }
        let continuation = value["metadata"]["continue"].as_str().unwrap_or_default();
        let next_cursor = if continuation.is_empty() {
            None
        } else {
            Some(self.cursors.issue(&context, continuation.into())?)
        };
        let revision = value["metadata"]["resourceVersion"]
            .as_str()
            .map(str::to_owned);
        Ok(Page {
            items,
            complete: next_cursor.is_none(),
            next_cursor,
            provenance: provenance(
                &self.descriptor.instance,
                format!("{namespace}/{kind}"),
                revision,
            ),
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resources {
    namespace: String,
    kind: String,
    limit: u16,
    #[serde(default)]
    cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Endpoints {
    namespace: String,
    limit: u16,
    #[serde(default)]
    cursor: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Hosts {
    limit: u16,
    #[serde(default)]
    cursor: Option<String>,
}

#[async_trait]
impl Adapter for Kubernetes {
    fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        match operation {
            "resources.list" => {
                let args: Resources = decode(input)?;
                if !self.config.resource_kinds.contains(&args.kind) {
                    return Err(Error::new(
                        ErrorCode::Forbidden,
                        "resource kind is outside configured scope",
                    ));
                }
                encode(
                    self.list(
                        operation,
                        &args.namespace,
                        &args.kind,
                        args.limit,
                        args.cursor.as_deref(),
                    )
                    .await?,
                )
            }
            "endpoints.discover" => {
                let args: Endpoints = decode(input)?;
                let page = self
                    .list(
                        operation,
                        &args.namespace,
                        "endpointslices",
                        args.limit,
                        args.cursor.as_deref(),
                    )
                    .await?;
                let mut endpoints = Vec::new();
                for slice in &page.items {
                    let uid = slice["metadata"]["uid"].as_str().ok_or_else(|| {
                        Error::new(ErrorCode::UpstreamProtocol, "EndpointSlice lacks identity")
                    })?;
                    let service = slice["metadata"]["labels"]["kubernetes.io/service-name"]
                        .as_str()
                        .map(str::to_owned);
                    let ports = slice["ports"].as_array().ok_or_else(|| {
                        Error::new(ErrorCode::UpstreamProtocol, "EndpointSlice lacks ports")
                    })?;
                    let entries = slice["endpoints"].as_array().ok_or_else(|| {
                        Error::new(ErrorCode::UpstreamProtocol, "EndpointSlice lacks endpoints")
                    })?;
                    for entry in entries {
                        let addresses = entry["addresses"].as_array().ok_or_else(|| {
                            Error::new(ErrorCode::UpstreamProtocol, "endpoint lacks addresses")
                        })?;
                        for address in addresses {
                            let address = address.as_str().ok_or_else(|| {
                                Error::new(ErrorCode::UpstreamProtocol, "invalid endpoint address")
                            })?;
                            for port in ports {
                                let Some(number) = port["port"].as_u64() else {
                                    continue;
                                };
                                let number = u16::try_from(number).map_err(|_| {
                                    Error::new(ErrorCode::UpstreamProtocol, "invalid endpoint port")
                                })?;
                                if number == 0 {
                                    return Err(Error::new(
                                        ErrorCode::UpstreamProtocol,
                                        "invalid endpoint port",
                                    ));
                                }
                                if endpoints.len() >= 4096 {
                                    return Err(Error::new(
                                        ErrorCode::Capacity,
                                        "endpoint expansion exceeds page budget; request fewer slices",
                                    ));
                                }
                                endpoints.push(EndpointObservation {
                                    id: format!(
                                        "{uid}/{address}/{number}/{}",
                                        port["protocol"].as_str().unwrap_or("TCP")
                                    ),
                                    namespace: args.namespace.clone(),
                                    service: service.clone(),
                                    address: address.into(),
                                    port: number,
                                    transport: port["protocol"].as_str().unwrap_or("TCP").into(),
                                    application_protocol: port["appProtocol"]
                                        .as_str()
                                        .map(str::to_owned),
                                    ready: entry["conditions"]["ready"].as_bool(),
                                    source_uid: uid.into(),
                                    reachability: "source_cluster_network".into(),
                                });
                            }
                        }
                    }
                }
                encode(Page {
                    items: endpoints,
                    next_cursor: page.next_cursor,
                    complete: page.complete,
                    provenance: page.provenance,
                })
            }
            "hosts.discover" => {
                if !self.config.discover_hosts {
                    return Err(Error::new(
                        ErrorCode::Forbidden,
                        "host discovery is disabled",
                    ));
                }
                let args: Hosts = decode(input)?;
                let mut page = self
                    .list(operation, "", "nodes", args.limit, args.cursor.as_deref())
                    .await?;
                page.items=page.items.into_iter().map(|node|json!({"id":node["metadata"]["uid"],"name":node["metadata"]["name"],"addresses":node["status"]["addresses"],"conditions":node["status"]["conditions"],"source_revision":node["metadata"]["resourceVersion"]})).collect();
                encode(page)
            }
            _ => Err(Error::new(
                ErrorCode::NotFound,
                "operation is not implemented",
            )),
        }
    }
}

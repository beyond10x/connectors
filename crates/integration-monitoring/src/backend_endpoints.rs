//! Grafana datasource observations participate in the shared endpoint API and retain mediation.

use super::*;

const MAX_INVENTORY_BYTES: usize = 8 * 1024 * 1024;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct InventorySnapshot {
    version: u8,
    state: MonitoringState,
}
use protocol::endpoint::{
    Endpoint, EndpointBinding, EndpointError, EndpointErrorCode, EndpointRequest, EndpointResult,
    EndpointState, EndpointTransport,
};

impl MonitoringInner {
    fn inventory_key(&self) -> String {
        format!(
            "monitoring.inventory:{:x}",
            Sha256::digest(
                format!(
                    "{}\0{}\0{}",
                    self.owner.tenant_id(),
                    self.owner.actor_subject(),
                    self.policy.canonical_origin()
                )
                .as_bytes()
            )
        )
    }

    pub(super) fn persist_inventory(&self, state: &MonitoringState) -> Result<(), MonitoringError> {
        let Some(store) = &self.inventory_state else {
            return Ok(());
        };
        let body = serde_json::to_vec(&InventorySnapshot {
            version: 1,
            state: state.clone(),
        })
        .map_err(|_| MonitoringError::new("inventory-state"))?;
        store
            .replace(&self.inventory_key(), &body, MAX_INVENTORY_BYTES)
            .map_err(|_| MonitoringError::new("inventory-state"))
    }

    pub(super) fn restore_inventory(&self) -> Result<(), MonitoringError> {
        let Some(store) = &self.inventory_state else {
            return Ok(());
        };
        let Some(body) = store
            .read(&self.inventory_key(), MAX_INVENTORY_BYTES)
            .map_err(|_| MonitoringError::new("inventory-state"))?
        else {
            return Ok(());
        };
        let mut snapshot: InventorySnapshot =
            serde_json::from_slice(&body).map_err(|_| MonitoringError::new("inventory-state"))?;
        if snapshot.version != 1
            || snapshot.state.observations.len() > 5000
            || snapshot.state.children.len() > 5000
        {
            return Err(MonitoringError::new("inventory-state"));
        }
        let parent = snapshot.state.parent.as_ref();
        if parent.is_some_and(|parent| {
            !safe_reference(&parent.connection_ref) || !valid_title(&parent.label)
        }) || snapshot
            .state
            .observations
            .iter()
            .any(|(key, observation)| {
                key != &observation.observation_ref
                    || parent.is_none_or(|parent| {
                        observation.source_connection_ref != parent.connection_ref
                    })
                    || !safe_datasource_uid(&observation.resource_binding)
                    || !valid_title(&observation.title)
                    || !valid_observed_type(&observation.observed_type)
                    || !self.endpoint(observation).validate()
            })
        {
            return Err(MonitoringError::new("inventory-state"));
        }
        // Grants come from this process's current configuration. Recreate children lazily rather
        // than restoring an obsolete grant reference from a previous policy generation.
        snapshot.state.children.clear();
        for observation in snapshot.state.observations.values_mut() {
            observation.connection_ref = None;
            observation.target_provider =
                target_provider(&observation.observed_type).map(str::to_owned);
        }
        *lock(&self.state) = snapshot.state;
        Ok(())
    }
    pub(super) fn owns_endpoint(&self, request: &EndpointRequest) -> bool {
        match request {
            EndpointRequest::List(request) => request
                .source_ref
                .as_deref()
                .is_none_or(|source| self.is_parent(source)),
            EndpointRequest::Refresh(request) => request
                .source_ref
                .as_deref()
                .is_none_or(|source| self.is_parent(source)),
            EndpointRequest::Show(request) => {
                self.endpoint_observation(&request.endpoint_ref).is_some()
            }
            EndpointRequest::Bind(request) => {
                self.endpoint_observation(&request.endpoint_ref).is_some()
            }
        }
    }

    pub(super) async fn handle_endpoint(
        &self,
        context: &PrincipalContext,
        request: EndpointRequest,
    ) -> Result<EndpointResult, EndpointError> {
        if !self.same_authority_partition(context) || !self.admits(context) {
            return Err(endpoint_refusal());
        }
        match request {
            EndpointRequest::List(request) => {
                let query = request.query.to_ascii_lowercase();
                let mut endpoints: Vec<_> = lock(&self.state)
                    .observations
                    .values()
                    .filter(|observation| {
                        request
                            .source_ref
                            .as_ref()
                            .is_none_or(|source| source == &observation.source_connection_ref)
                    })
                    .filter(|observation| {
                        query.is_empty()
                            || observation.title.to_ascii_lowercase().contains(&query)
                            || observation
                                .resource_binding
                                .to_ascii_lowercase()
                                .contains(&query)
                            || observation
                                .observed_type
                                .to_ascii_lowercase()
                                .contains(&query)
                    })
                    .map(|observation| self.endpoint(observation))
                    .collect();
                endpoints.sort_by(|left, right| left.endpoint_ref.cmp(&right.endpoint_ref));
                endpoints.retain(|endpoint| {
                    request
                        .cursor
                        .as_deref()
                        .is_none_or(|cursor| endpoint.endpoint_ref.as_str() > cursor)
                });
                let next_cursor = if endpoints.len() > usize::from(request.limit) {
                    endpoints.truncate(usize::from(request.limit));
                    endpoints
                        .last()
                        .map(|endpoint| endpoint.endpoint_ref.clone())
                } else {
                    None
                };
                Ok(EndpointResult::List {
                    endpoints,
                    next_cursor,
                    warnings: Vec::new(),
                })
            }
            EndpointRequest::Show(request) => {
                let observation = self
                    .endpoint_observation(&request.endpoint_ref)
                    .ok_or_else(endpoint_not_found)?;
                Ok(EndpointResult::Show {
                    endpoint: self.endpoint(&observation),
                })
            }
            EndpointRequest::Refresh(request) => {
                self.require_endpoint_management(context)?;
                if request
                    .source_ref
                    .as_deref()
                    .is_some_and(|source| !self.is_parent(source))
                {
                    return Err(endpoint_not_found());
                }
                self.refresh_endpoint_inventory()
                    .await
                    .map_err(endpoint_operation_error)?;
                Ok(EndpointResult::Refresh {
                    endpoints: lock(&self.state).observations.len(),
                    warnings: Vec::new(),
                })
            }
            EndpointRequest::Bind(request) => {
                self.require_endpoint_management(context)?;
                let observation = self
                    .endpoint_observation(&request.endpoint_ref)
                    .ok_or_else(endpoint_not_found)?;
                // Grafana's reviewed datasource and credential are the route owner. An endpoint
                // binding must not turn this mediated source into arbitrary direct egress.
                if observation.target_provider.as_ref() != Some(&request.binding.provider)
                    || request.binding.base_path.is_some()
                    || request.binding.credential.is_some()
                    || request.binding.direct_address.is_some()
                    || request.binding.database.is_some()
                    || request.binding.tls.is_some()
                    || request.binding.scheme.is_some()
                {
                    return Err(EndpointError::new(EndpointErrorCode::InvalidInput,
                        "Grafana datasource bindings retain their recognized provider and Grafana-owned route", false));
                }
                Ok(EndpointResult::Bind {
                    endpoint: self.endpoint(&observation),
                })
            }
        }
    }

    pub(super) async fn resolve_endpoint(
        &self,
        context: &PrincipalContext,
        endpoint_ref: &str,
        operation_ref: &str,
    ) -> Result<String, OperationError> {
        self.check_operation_context(context)?;
        self.require_operation_access(context)?;
        let observation = self
            .endpoint_observation(endpoint_ref)
            .ok_or_else(operation_not_found)?;
        if !supported_operation(operation_ref) {
            return Err(operation_not_found());
        }
        let provider = provider_for_operation(operation_ref);
        if observation.target_provider.as_deref() != Some(provider) {
            return Err(operation_not_granted());
        }
        // Source authentication is necessary to validate current inventory. Target credentials
        // remain inside Grafana; no target request or tunnel starts during normalization.
        self.refresh_endpoint_inventory().await?;
        let fresh = self
            .endpoint_observation(endpoint_ref)
            .ok_or_else(operation_not_found)?;
        if !fresh.active
            || fresh.resource_binding != observation.resource_binding
            || fresh.target_provider != observation.target_provider
        {
            return Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "the Grafana endpoint has changed; obtain a fresh endpoint description",
                false,
            ));
        }
        // Existing materialization verifies the independent target grant and mediated child.
        self.materialize(&fresh.observation_ref)
            .map(|connection| connection.summary.connection_ref)
            .map_err(|_| operation_not_granted())
    }

    fn endpoint_observation(&self, endpoint_ref: &str) -> Option<StoredObservation> {
        lock(&self.state)
            .observations
            .values()
            .find(|observation| self.endpoint(observation).endpoint_ref == endpoint_ref)
            .cloned()
    }

    fn endpoint(&self, observation: &StoredObservation) -> Endpoint {
        let uid_sha256 = if observation.resource_binding.is_empty() {
            self.hosted_targets
                .as_ref()
                .and_then(|targets| {
                    targets.iter().find(|target| {
                        observation.connection_ref.as_ref() == Some(&target.connection_ref)
                    })
                })
                .map(|target| target.uid_sha256.clone())
                .unwrap_or_else(|| {
                    format!(
                        "{:x}",
                        Sha256::digest(observation.observation_ref.as_bytes())
                    )
                })
        } else {
            format!(
                "{:x}",
                Sha256::digest(observation.resource_binding.as_bytes())
            )
        };
        let endpoint_ref = format!(
            "endpoint:grafana:{}",
            digest_prefix(
                format!("{}\0{uid_sha256}", observation.source_connection_ref).as_bytes()
            )
        );
        let state = if !observation.active {
            EndpointState::Stale
        } else if observation.target_provider.is_none() {
            EndpointState::UnknownProvider
        } else if observation.connection_ref.is_some()
            || observation
                .target_provider
                .as_deref()
                .is_some_and(|provider| self.policy.target_grant(provider).is_some())
        {
            EndpointState::Ready
        } else {
            EndpointState::Denied
        };
        Endpoint {
            endpoint_ref,
            source_ref: observation.source_connection_ref.clone(),
            namespace: None,
            resource_kind: "GrafanaDatasource".into(),
            resource_name: if observation.resource_binding.is_empty() {
                observation.observation_ref.clone()
            } else {
                observation.resource_binding.clone()
            },
            resource_uid: format!("sha256:{uid_sha256}"),
            port_name: None,
            port: None,
            transport: EndpointTransport::Tcp,
            interface: "http".into(),
            provider: observation.target_provider.clone(),
            state,
            binding: observation
                .target_provider
                .as_ref()
                .map(|provider| EndpointBinding {
                    provider: provider.clone(),
                    base_path: None,
                    credential: None,
                    direct_address: None,
                    database: None,
                    tls: None,
                    scheme: None,
                }),
        }
    }

    fn require_endpoint_management(&self, context: &PrincipalContext) -> Result<(), EndpointError> {
        let admitted = match &self.access {
            MonitoringAccess::ExactOwner => context == &self.owner,
            MonitoringAccess::HostedGroups {
                tenant_id,
                operator_groups,
                ..
            } => {
                context.tenant_id() == tenant_id
                    && !operator_groups.is_disjoint(context.verified_groups())
            }
        };
        admitted.then_some(()).ok_or_else(endpoint_refusal)
    }

    pub(super) async fn refresh_endpoint_inventory(&self) -> Result<(), OperationError> {
        if self.hosted_targets.is_some() {
            return self.reconcile_hosted_targets().await;
        }
        let parent = lock(&self.state)
            .parent
            .clone()
            .ok_or_else(operation_not_granted)?;
        let token = self
            .load_credential(GRAFANA_DATASOURCES_LIST, ROUTE_DIRECT)
            .await?;
        let operation =
            operation_document(GRAFANA_DATASOURCES_LIST).ok_or_else(operation_unavailable)?;
        let output = self
            .execute_direct(
                &self.owner,
                &parent,
                &token,
                operation,
                serde_json::json!({}),
            )
            .await?;
        self.reconcile_observations(&parent.connection_ref, &output)
    }
}

fn safe_reference(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn endpoint_not_found() -> EndpointError {
    EndpointError::new(
        EndpointErrorCode::NotFound,
        "the Grafana endpoint is unavailable",
        false,
    )
}
fn endpoint_refusal() -> EndpointError {
    EndpointError::new(
        EndpointErrorCode::NotGranted,
        "Grafana endpoint access is not admitted",
        false,
    )
}
fn endpoint_operation_error(_: OperationError) -> EndpointError {
    EndpointError::new(
        EndpointErrorCode::Unavailable,
        "Grafana inventory refresh is unavailable",
        true,
    )
}

//! One endpoint API and operation backend shared by kubeconfig and service-account placements.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use connector_secrets::{CredentialRef, MemoryStore, Secret, SecretStore};
use domain::endpoint::{Endpoint, EndpointCredentialReference, EndpointState, EndpointTls};
use integration_catalog::endpoint as catalog_endpoint;
use protocol::connection;
use protocol::endpoint::{EndpointError, EndpointErrorCode, EndpointRequest, EndpointResult};
use protocol::operation::{
    self, InvocationResult, OperationError, OperationErrorCode, OperationRequest, OperationResult,
};
use service::{ConnectorBackend, EgressTransport, PrincipalContext};
use sha2::{Digest, Sha256};

use super::{
    EndpointRouteLease, EndpointSourceError, KubernetesEndpointSource, ResolvedEndpointCredentials,
};

/// Runtime composition supplies the network aperture. Integrations do not create an HTTP client.
pub trait EndpointEgressFactory: Send + Sync + 'static {
    fn transport(
        &self,
        connection_ref: &str,
        route: &EndpointRouteLease,
    ) -> Result<Arc<dyn EgressTransport>, EndpointSourceError>;
}

/// Authenticated actor policy, independently intersected with source/provider grants.
pub enum EndpointPrincipalPolicy {
    Local(Arc<PrincipalContext>),
    Hosted {
        tenant: String,
        namespace_groups: BTreeMap<String, BTreeSet<String>>,
        operator_groups: BTreeSet<String>,
    },
}

impl EndpointPrincipalPolicy {
    fn owns(&self, context: &PrincipalContext) -> bool {
        match self {
            Self::Local(owner) => owner.as_ref() == context,
            Self::Hosted { tenant, .. } => tenant == context.tenant_id(),
        }
    }

    fn manages(&self, context: &PrincipalContext) -> bool {
        self.owns(context)
            && match self {
                Self::Local(_) => true,
                Self::Hosted {
                    operator_groups, ..
                } => !operator_groups.is_disjoint(context.verified_groups()),
            }
    }

    fn reads(&self, context: &PrincipalContext, endpoint: &Endpoint) -> bool {
        if !self.owns(context) {
            return false;
        }
        match self {
            Self::Local(_) => true,
            Self::Hosted {
                namespace_groups, ..
            } => {
                let mut namespaces = Vec::new();
                if let Some(namespace) = endpoint.namespace.as_deref() {
                    namespaces.push(namespace);
                }
                if let Some(EndpointCredentialReference::KubernetesSecret { namespace, .. }) =
                    endpoint
                        .binding
                        .as_ref()
                        .and_then(|binding| binding.credential.as_ref())
                {
                    namespaces.push(namespace);
                }
                !namespaces.is_empty()
                    && namespaces.into_iter().all(|namespace| {
                        namespace_groups.get(namespace).is_some_and(|groups| {
                            self.manages(context) || !groups.is_disjoint(context.verified_groups())
                        })
                    })
            }
        }
    }
}

/// Generic endpoint projection with invocation routed through existing catalog/SQL owners.
pub struct KubernetesEndpointBackend {
    source: Arc<KubernetesEndpointSource>,
    policy: EndpointPrincipalPolicy,
    egress: Arc<dyn EndpointEgressFactory>,
    reconciliation: Option<tokio::task::JoinHandle<()>>,
    initial_refresh: Option<tokio::task::JoinHandle<()>>,
}

impl KubernetesEndpointBackend {
    pub fn new(
        source: Arc<KubernetesEndpointSource>,
        policy: EndpointPrincipalPolicy,
        egress: Arc<dyn EndpointEgressFactory>,
    ) -> Self {
        let weak = Arc::downgrade(&source);
        let reconciliation = tokio::runtime::Handle::try_current().ok().map(|runtime| {
            runtime.spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                interval.tick().await;
                loop {
                    interval.tick().await;
                    let Some(source) = weak.upgrade() else {
                        break;
                    };
                    let _ = source.refresh().await;
                }
            })
        });
        Self {
            source,
            policy,
            egress,
            reconciliation,
            initial_refresh: None,
        }
    }

    /// Restore metadata immediately and validate the cluster in a task owned by this backend.
    pub fn refresh_on_start(mut self) -> Self {
        let weak = Arc::downgrade(&self.source);
        self.initial_refresh = Some(tokio::spawn(async move {
            if let Some(source) = weak.upgrade() {
                let _ = source.refresh().await;
            }
        }));
        self
    }

    pub fn source(&self) -> &Arc<KubernetesEndpointSource> {
        &self.source
    }

    fn endpoints(&self, context: &PrincipalContext) -> Result<Vec<Endpoint>, OperationError> {
        if !self.policy.owns(context) {
            return Err(operation_refused());
        }
        Ok(self
            .source
            .list()
            .map_err(operation_error)?
            .into_iter()
            .filter(|endpoint| self.policy.reads(context, endpoint))
            .collect())
    }

    fn callable(&self, context: &PrincipalContext) -> Result<Vec<Endpoint>, OperationError> {
        Ok(self
            .endpoints(context)?
            .into_iter()
            .filter(|endpoint| {
                endpoint.provider.is_some()
                    && !matches!(
                        endpoint.state,
                        EndpointState::Stale
                            | EndpointState::Denied
                            | EndpointState::UnknownProvider
                            | EndpointState::UnsupportedProtocol
                    )
            })
            .collect())
    }

    fn by_connection(
        &self,
        context: &PrincipalContext,
        reference: &str,
    ) -> Result<Endpoint, OperationError> {
        self.callable(context)?
            .into_iter()
            .find(|endpoint| connection_ref(endpoint) == reference)
            .ok_or_else(operation_refused)
    }

    fn description_ref(&self, context: &PrincipalContext, operation_ref: &str) -> String {
        let binding = catalog::operation(catalog::OperationKey::id(operation_ref))
            .and_then(|operation| self.source.binding_digest(operation.provider).ok())
            .unwrap_or_default();
        format!(
            "description:endpoint:{}",
            hex::encode(Sha256::digest(format!(
                "{}\0{}\0{}\0{}",
                context.authority_snapshot_sha256(),
                self.source.source_ref(),
                operation_ref,
                binding,
            )))
        )
    }

    fn connections_for(
        &self,
        context: &PrincipalContext,
        provider: &str,
    ) -> Result<Vec<operation::ConnectionSummary>, OperationError> {
        Ok(self
            .callable(context)?
            .iter()
            .filter(|endpoint| endpoint.provider.as_deref() == Some(provider))
            .map(operation_connection)
            .collect())
    }

    async fn invoke(
        &self,
        context: &PrincipalContext,
        request: operation::InvokeRequest,
    ) -> Result<InvocationResult, OperationError> {
        let endpoint = self.by_connection(context, &request.connection_ref)?;
        if request.description_ref != self.description_ref(context, &request.operation_ref) {
            return Err(operation_refused());
        }
        let endpoint = self
            .source
            .validate(&endpoint.endpoint_ref)
            .await
            .map_err(operation_error)?;
        if !self.policy.reads(context, &endpoint) {
            return Err(operation_refused());
        }
        let provider = endpoint.provider.as_deref().ok_or_else(operation_refused)?;
        if matches!(provider, "postgresql" | "mysql") {
            let engine = if provider == "mysql" {
                driver_sql::SqlEngine::MySql
            } else {
                driver_sql::SqlEngine::Postgres
            };
            let admitted =
                driver_sql::admit_operation(engine, &request.operation_ref, request.input)
                    .map_err(|_| {
                        OperationError::new(
                            OperationErrorCode::InvalidInput,
                            "SQL input did not satisfy the admitted read operation",
                            false,
                        )
                    })?;
            let credentials = self
                .source
                .resolve_credentials(&endpoint)
                .await
                .map_err(operation_error)?;
            let route = self
                .source
                .open_route(&endpoint, &credentials)
                .await
                .map_err(operation_error)?;
            let binding = endpoint.binding.as_ref().ok_or_else(operation_refused)?;
            let database = binding
                .database
                .as_deref()
                .or_else(|| credentials.expose("database"))
                .filter(|value| !value.is_empty())
                .ok_or_else(|| operation_error(EndpointSourceError::MissingCredentials))?;
            let user = credentials
                .expose("username")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| operation_error(EndpointSourceError::MissingCredentials))?;
            let password = credentials
                .expose("password")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| operation_error(EndpointSourceError::MissingCredentials))?;
            let credential = match binding.credential.as_ref() {
                Some(EndpointCredentialReference::KubernetesSecret {
                    namespace,
                    name,
                    keys,
                }) => driver_sql::credentials::CredentialReference::KubernetesSecret {
                    namespace: namespace.clone(),
                    name: name.clone(),
                    key: keys
                        .get("password")
                        .cloned()
                        .ok_or_else(operation_refused)?,
                },
                _ => return Err(operation_error(EndpointSourceError::MissingCredentials)),
            };
            let tls = match binding.tls {
                Some(EndpointTls::Disabled) => driver_sql::SqlTls::Disabled,
                _ => credentials
                    .expose("ca")
                    .map(|ca| driver_sql::SqlTls::WithCa(ca.as_bytes().to_vec()))
                    .unwrap_or(driver_sql::SqlTls::Required),
            };
            let config = driver_sql::SqlConnectionConfig {
                engine,
                host: route
                    .logical_url
                    .host_str()
                    .ok_or_else(operation_refused)?
                    .to_owned(),
                port: route.logical_url.port().unwrap_or(if provider == "mysql" {
                    3306
                } else {
                    5432
                }),
                database: database.to_owned(),
                user: user.to_owned(),
                credential: credential.clone(),
                statement_timeout_ms: driver_sql::DEFAULT_STATEMENT_TIMEOUT_MS,
                connect_address: route.connect_address,
                tls,
            };
            let secret = InvocationPassword {
                reference: credential,
                password,
            };
            let output = admitted.execute(&config, &secret).await.map_err(|_| {
                OperationError::new(
                    OperationErrorCode::Unavailable,
                    "the admitted SQL endpoint could not complete the read",
                    false,
                )
            })?;
            return Ok(InvocationResult {
                operation_ref: request.operation_ref.clone(),
                output,
                connector_audit_ref: audit(
                    context,
                    &request.operation_ref,
                    &request.connection_ref,
                ),
                execution_ref: None,
            });
        }
        let admitted =
            catalog_endpoint::admit_http(provider, &request.operation_ref, request.input)?;
        let credentials = self
            .source
            .resolve_credentials(&endpoint)
            .await
            .map_err(operation_error)?;
        let route = self
            .source
            .open_route(&endpoint, &credentials)
            .await
            .map_err(operation_error)?;
        let (config, secrets) = http_credentials(context, provider, &credentials).await?;
        let egress = self
            .egress
            .transport(&request.connection_ref, &route)
            .map_err(operation_error)?;
        admitted
            .execute(
                context.tenant_id(),
                &request.connection_ref,
                route.logical_url.as_str(),
                &config,
                &secrets,
                egress.as_ref(),
            )
            .await
    }
}

impl Drop for KubernetesEndpointBackend {
    fn drop(&mut self) {
        if let Some(task) = &self.reconciliation {
            task.abort();
        }
        if let Some(task) = &self.initial_refresh {
            task.abort();
        }
    }
}

#[async_trait]
impl ConnectorBackend for KubernetesEndpointBackend {
    async fn ready(&self) -> Result<(), service::BackendReadinessError> {
        Ok(())
    }

    fn capabilities(&self) -> service::BackendCapabilities {
        service::BackendCapabilities {
            connections: true,
            ..service::BackendCapabilities::OPERATIONS
        }
    }

    fn owns_endpoint(&self, request: &EndpointRequest) -> bool {
        match request {
            EndpointRequest::List(request) => request
                .source_ref
                .as_deref()
                .is_none_or(|source| source == self.source.source_ref()),
            EndpointRequest::Refresh(request) => request
                .source_ref
                .as_deref()
                .is_none_or(|source| source == self.source.source_ref()),
            EndpointRequest::Show(request) => self.source.show(&request.endpoint_ref).is_ok(),
            EndpointRequest::Bind(request) => self.source.show(&request.endpoint_ref).is_ok(),
        }
    }

    async fn handle_endpoint(
        &self,
        context: &PrincipalContext,
        request: EndpointRequest,
    ) -> Result<EndpointResult, EndpointError> {
        if !self.policy.owns(context) {
            return Err(endpoint_error(EndpointSourceError::Denied));
        }
        match request {
            EndpointRequest::List(request) => {
                if request
                    .source_ref
                    .as_deref()
                    .is_some_and(|source| source != self.source.source_ref())
                {
                    return Err(endpoint_error(EndpointSourceError::Stale));
                }
                let query = request.query.to_ascii_lowercase();
                let endpoints = self
                    .source
                    .list()
                    .map_err(endpoint_error)?
                    .into_iter()
                    .filter(|endpoint| {
                        self.policy.reads(context, endpoint)
                            && format!(
                                "{} {} {} {}",
                                endpoint.resource_name,
                                endpoint.namespace.as_deref().unwrap_or(""),
                                endpoint.provider.as_deref().unwrap_or(""),
                                endpoint.interface
                            )
                            .to_ascii_lowercase()
                            .contains(&query)
                    })
                    .collect::<Vec<_>>();
                let digest = hex::encode(Sha256::digest(
                    serde_json::to_vec(&endpoints)
                        .map_err(|_| endpoint_error(EndpointSourceError::Unavailable))?,
                ));
                let authority = context.authority_snapshot_sha256();
                let cursor_prefix = hex::encode(Sha256::digest(format!(
                    "{}\0{}\0{}\0{}",
                    self.source.source_ref(),
                    authority,
                    query,
                    digest
                )));
                let offset = if let Some(cursor) = request.cursor {
                    let (prefix, offset) = cursor
                        .rsplit_once(':')
                        .ok_or_else(|| endpoint_error(EndpointSourceError::InvalidBinding))?;
                    if prefix != cursor_prefix {
                        return Err(EndpointError::new(
                            EndpointErrorCode::Conflict,
                            "endpoint inventory changed; restart listing",
                            false,
                        ));
                    }
                    offset
                        .parse::<usize>()
                        .map_err(|_| endpoint_error(EndpointSourceError::InvalidBinding))?
                } else {
                    0
                };
                if offset > endpoints.len() {
                    return Err(endpoint_error(EndpointSourceError::InvalidBinding));
                }
                let end = offset
                    .saturating_add(usize::from(request.limit))
                    .min(endpoints.len());
                let mut warnings = self.source.warnings().map_err(endpoint_error)?;
                if !self.policy.manages(context) && !warnings.is_empty() {
                    warnings = vec!["Endpoint source refresh is incomplete".to_owned()];
                }
                Ok(EndpointResult::List {
                    endpoints: endpoints[offset..end].to_vec(),
                    next_cursor: (end < endpoints.len()).then(|| format!("{cursor_prefix}:{end}")),
                    warnings,
                })
            }
            EndpointRequest::Show(request) => {
                let endpoint = self
                    .source
                    .show(&request.endpoint_ref)
                    .map_err(endpoint_error)?;
                if !self.policy.reads(context, &endpoint) {
                    return Err(endpoint_error(EndpointSourceError::Denied));
                }
                Ok(EndpointResult::Show { endpoint })
            }
            EndpointRequest::Bind(request) => {
                if !self.policy.manages(context) {
                    return Err(endpoint_error(EndpointSourceError::Denied));
                }
                Ok(EndpointResult::Bind {
                    endpoint: self
                        .source
                        .bind(&request.endpoint_ref, request.binding)
                        .map_err(endpoint_error)?,
                })
            }
            EndpointRequest::Refresh(request) => {
                if !self.policy.manages(context)
                    || request
                        .source_ref
                        .as_deref()
                        .is_some_and(|source| source != self.source.source_ref())
                {
                    return Err(endpoint_error(EndpointSourceError::Denied));
                }
                let scan = self.source.refresh().await.map_err(endpoint_error)?;
                Ok(EndpointResult::Refresh {
                    endpoints: scan.endpoints.len(),
                    warnings: scan.warnings.into_iter().take(100).collect(),
                })
            }
        }
    }

    async fn resolve_endpoint(
        &self,
        context: &PrincipalContext,
        reference: &str,
        operation_ref: &str,
    ) -> Result<String, OperationError> {
        let endpoint = self.source.show(reference).map_err(operation_error)?;
        if !self.policy.reads(context, &endpoint) {
            return Err(operation_refused());
        }
        let operation = catalog::operation(catalog::OperationKey::id(operation_ref))
            .ok_or_else(operation_refused)?;
        if endpoint.provider.as_deref() != Some(operation.provider)
            || !catalog_endpoint::read_admitted(operation)
        {
            return Err(operation_refused());
        }
        let endpoint = self
            .source
            .validate(reference)
            .await
            .map_err(operation_error)?;
        Ok(connection_ref(&endpoint))
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Search(_) => true,
            OperationRequest::Describe(request) => catalog::operation(catalog::OperationKey::id(
                &request.operation_ref,
            ))
            .is_some_and(|operation| {
                self.source.list().is_ok_and(|endpoints| {
                    endpoints
                        .iter()
                        .any(|endpoint| endpoint.provider.as_deref() == Some(operation.provider))
                })
            }),
            OperationRequest::Invoke(request) => catalog::operation(catalog::OperationKey::id(
                &request.operation_ref,
            ))
            .is_some_and(|operation| {
                self.source.list().is_ok_and(|endpoints| {
                    endpoints.iter().any(|endpoint| {
                        connection_ref(endpoint) == request.connection_ref
                            && endpoint.provider.as_deref() == Some(operation.provider)
                    })
                })
            }),
            _ => false,
        }
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        match request {
            OperationRequest::Search(request) => {
                let mut operations = Vec::new();
                let providers: BTreeSet<_> = self
                    .callable(context)?
                    .iter()
                    .filter_map(|endpoint| endpoint.provider.clone())
                    .collect();
                for provider in providers {
                    if let Some(provider) = catalog::provider(catalog::ProviderKey::id(&provider)) {
                        for operation in provider.operations.iter().filter(|operation| {
                            catalog_endpoint::read_admitted(operation)
                                && format!(
                                    "{} {} {}",
                                    operation.provider, operation.id, operation.description
                                )
                                .to_ascii_lowercase()
                                .contains(&request.query.to_ascii_lowercase())
                        }) {
                            operations.push(catalog_endpoint::summary(
                                operation,
                                self.connections_for(context, provider.id)?,
                            ));
                        }
                    }
                }
                operations.truncate(usize::from(request.limit));
                Ok(OperationResult::Search { operations })
            }
            OperationRequest::Describe(request) => {
                let operation =
                    catalog::operation(catalog::OperationKey::id(&request.operation_ref))
                        .ok_or_else(operation_refused)?;
                let connections = self.connections_for(context, operation.provider)?;
                if connections.is_empty() {
                    return Err(operation_refused());
                }
                Ok(OperationResult::Describe(catalog_endpoint::describe(
                    operation.provider,
                    operation.id,
                    connections,
                    self.description_ref(context, operation.id),
                )?))
            }
            OperationRequest::Invoke(request) => self
                .invoke(context, request)
                .await
                .map(OperationResult::Invoke),
            _ => Err(operation_refused()),
        }
    }

    fn owns_connection(&self, request: &connection::ConnectionRequest) -> bool {
        match request {
            connection::ConnectionRequest::Search(_) => true,
            connection::ConnectionRequest::Describe(request) => {
                self.source.list().is_ok_and(|endpoints| {
                    endpoints
                        .iter()
                        .any(|endpoint| connection_ref(endpoint) == request.connection_ref)
                })
            }
            _ => false,
        }
    }

    async fn handle_connection(
        &self,
        context: &PrincipalContext,
        request: connection::ConnectionRequest,
    ) -> Result<connection::ConnectionResult, connection::ConnectionError> {
        let refused = || {
            connection::ConnectionError::new(
                connection::ConnectionErrorCode::NotGranted,
                "endpoint Connection is not admitted",
                false,
            )
        };
        match request {
            connection::ConnectionRequest::Search(request) => {
                let connections = self
                    .callable(context)
                    .map_err(|_| refused())?
                    .iter()
                    .filter(|endpoint| {
                        endpoint
                            .resource_name
                            .to_ascii_lowercase()
                            .contains(&request.query.to_ascii_lowercase())
                    })
                    .take(usize::from(request.limit))
                    .map(connection_summary)
                    .collect();
                Ok(connection::ConnectionResult::Search { connections })
            }
            connection::ConnectionRequest::Describe(request) => {
                let endpoint = self
                    .by_connection(context, &request.connection_ref)
                    .map_err(|_| refused())?;
                Ok(connection::ConnectionResult::Describe(
                    connection::ConnectionDescription {
                        summary: connection_summary(&endpoint),
                        channels: Vec::new(),
                    },
                ))
            }
            _ => Err(refused()),
        }
    }
}

fn connection_ref(endpoint: &Endpoint) -> String {
    format!(
        "connection:endpoint:{}",
        hex::encode(Sha256::digest(endpoint.endpoint_ref.as_bytes()))
    )
}

fn operation_connection(endpoint: &Endpoint) -> operation::ConnectionSummary {
    operation::ConnectionSummary {
        connection_ref: connection_ref(endpoint),
        label: endpoint.resource_name.clone(),
        provider: endpoint.provider.clone().unwrap_or_default(),
        audiences: Vec::new(),
        purpose: None,
    }
}

fn connection_summary(endpoint: &Endpoint) -> connection::ConnectionSummary {
    connection::ConnectionSummary {
        connection_ref: connection_ref(endpoint),
        integration_ref: endpoint.provider.clone().unwrap_or_default(),
        label: endpoint.resource_name.clone(),
        state: connection::ConnectionState::Authorized,
        initiation: vec![connection::ConnectionInitiator::Platform],
        route: connection::ConnectionRoute::Direct,
        scope: None,
        actor: None,
        auth_profile: None,
    }
}

async fn http_credentials(
    context: &PrincipalContext,
    provider_id: &str,
    values: &ResolvedEndpointCredentials,
) -> Result<(integration_catalog::DeclaredConfig, MemoryStore), OperationError> {
    let provider =
        catalog::provider(catalog::ProviderKey::id(provider_id)).ok_or_else(operation_refused)?;
    let authority = provider.authority.ok_or_else(operation_refused)?;
    let mut config = integration_catalog::DeclaredConfig::new(BTreeMap::new(), true);
    let secrets = MemoryStore::new();
    for credential in provider.auth {
        let value = values
            .expose(credential.name)
            .or_else(|| values.expose(credential.leaf));
        if let Some(value) = value {
            let reference =
                CredentialRef::new(context.tenant_id(), authority, "default", credential.leaf)
                    .map_err(|_| operation_refused())?;
            secrets
                .put(&reference, &Secret::new(value))
                .await
                .map_err(|_| operation_refused())?;
        }
        if let Some(username) = values
            .expose(&format!("username.{}", credential.name))
            .or_else(|| values.expose("username"))
        {
            config = config.with_username(credential.name, username);
        }
    }
    Ok((config, secrets))
}

struct InvocationPassword<'a> {
    reference: driver_sql::credentials::CredentialReference,
    password: &'a str,
}
impl driver_sql::credentials::CredentialSource for InvocationPassword<'_> {
    fn resolve(
        &self,
        reference: &driver_sql::credentials::CredentialReference,
    ) -> Result<driver_sql::credentials::ResolvedSecret, driver_sql::credentials::CredentialError>
    {
        if reference != &self.reference {
            return Err(
                driver_sql::credentials::CredentialError::SourceUnavailable {
                    reference: reference.describe(),
                    detail: "credential is not part of this admitted invocation".to_owned(),
                },
            );
        }
        Ok(driver_sql::credentials::ResolvedSecret::new(
            self.password.to_owned(),
        ))
    }
}

fn audit(context: &PrincipalContext, operation: &str, connection: &str) -> String {
    format!(
        "audit:endpoint:{}",
        hex::encode(Sha256::digest(format!(
            "{}\0{operation}\0{connection}",
            context.authority_snapshot_sha256()
        )))
    )
}

pub(super) fn endpoint_error(error: EndpointSourceError) -> EndpointError {
    let code = match error {
        EndpointSourceError::Denied => EndpointErrorCode::NotGranted,
        EndpointSourceError::Stale => EndpointErrorCode::NotFound,
        EndpointSourceError::InvalidBinding => EndpointErrorCode::InvalidInput,
        _ => EndpointErrorCode::Unavailable,
    };
    EndpointError::new(
        code,
        error.to_string(),
        matches!(error, EndpointSourceError::Unavailable),
    )
}

fn operation_error(error: EndpointSourceError) -> OperationError {
    OperationError::new(
        match error {
            EndpointSourceError::Denied => OperationErrorCode::NotGranted,
            EndpointSourceError::Stale => OperationErrorCode::StaleAuthority,
            EndpointSourceError::InvalidBinding => OperationErrorCode::InvalidInput,
            _ => OperationErrorCode::Unavailable,
        },
        error.to_string(),
        matches!(error, EndpointSourceError::Unavailable),
    )
}

fn operation_refused() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotGranted,
        "endpoint operation is not admitted by current source policy",
        false,
    )
}

#[cfg(test)]
mod tests;

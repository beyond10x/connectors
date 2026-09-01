//! Deterministic composition of generated service factories into the exact backend registry.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use protocol::connection::{ConnectionError, ConnectionRequest, ConnectionResult};
use protocol::datasource::{DatasourceError, DatasourceRequest, DatasourceResult};
use protocol::event::{EventError, EventRequest, EventResult};
use protocol::operation::{
    DescribeRequest, OperationError, OperationErrorCode, OperationRequest, OperationResult,
};
use service::{
    BackendCapabilities, BackendReadinessError, ConnectSessionAccess, ConnectorBackend,
    ConnectorServiceFactory, HostedCompletionError, HostedCompletionPage,
    HostedCompletionSubmission, PrincipalContext, ServiceDeployment, ServiceManifest,
    ServiceOperation,
};

use crate::BackendRegistry;

/// Closed refusal vocabulary for generated-service registration and deployment composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ServiceBundleError {
    #[error("a generated service manifest is invalid")]
    InvalidManifest,
    #[error("a generated service identity was registered more than once")]
    ServiceIdentityCollision,
    #[error("an operation identity was contributed by more than one service")]
    OperationIdentityCollision,
    #[error("a deployment overlay names no registered service")]
    UnknownService,
    #[error("a service received more than one deployment overlay")]
    DeploymentCollision,
    #[error("a deployment provider identity is invalid")]
    InvalidProviderIdentity,
    #[error("a permanent provider reference was assigned more than once")]
    ProviderIdentityCollision,
    #[error("a permanent provider authority was assigned more than once")]
    ProviderAuthorityCollision,
    #[error("a deployment operation policy or resource reference is invalid")]
    InvalidOperationDeployment,
    #[error("a deployment overlay does not name the manifest's exact operation set")]
    OperationOverlayMismatch,
    #[error("a generated backend dispatch set does not match its operation catalog")]
    CatalogDispatchMismatch,
    #[error("a generated backend does not claim an operation it declares")]
    BackendOwnershipMismatch,
    #[error("a generated service factory refused its deployment overlay")]
    FactoryBind,
}

struct RegisteredFactory {
    factory: Arc<dyn ConnectorServiceFactory>,
    manifest: ServiceManifest,
}

/// Builder for a closed generated-service bundle.
///
/// Registration validates identities and catalog shape, but is intentionally inert. Only
/// [`deploy`](Self::deploy) marks a factory for binding, and a registered factory without an
/// overlay contributes no provider, no backend and no authority to the completed bundle.
#[derive(Default)]
pub struct ServiceBundleBuilder {
    factories: BTreeMap<String, RegisteredFactory>,
    operation_owners: BTreeMap<String, String>,
    deployments: BTreeMap<String, ServiceDeployment>,
}

impl ServiceBundleBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an owned factory without activating it.
    pub fn register<F>(&mut self, factory: F) -> Result<&mut Self, ServiceBundleError>
    where
        F: ConnectorServiceFactory,
    {
        self.register_shared(Arc::new(factory))
    }

    /// Register a shared or dynamically selected factory without activating it.
    pub fn register_shared(
        &mut self,
        factory: Arc<dyn ConnectorServiceFactory>,
    ) -> Result<&mut Self, ServiceBundleError> {
        let manifest = normalize_manifest(factory.manifest())?;
        if self.factories.contains_key(&manifest.service_ref) {
            return Err(ServiceBundleError::ServiceIdentityCollision);
        }
        if manifest
            .operations
            .iter()
            .any(|operation| self.operation_owners.contains_key(&operation.operation_ref))
        {
            return Err(ServiceBundleError::OperationIdentityCollision);
        }
        for operation in &manifest.operations {
            self.operation_owners.insert(
                operation.operation_ref.clone(),
                manifest.service_ref.clone(),
            );
        }
        self.factories.insert(
            manifest.service_ref.clone(),
            RegisteredFactory { factory, manifest },
        );
        Ok(self)
    }

    /// Attach the complete deployment overlay which activates one registered factory.
    pub fn deploy(
        &mut self,
        deployment: ServiceDeployment,
    ) -> Result<&mut Self, ServiceBundleError> {
        let registered = self
            .factories
            .get(&deployment.service_ref)
            .ok_or(ServiceBundleError::UnknownService)?;
        if self.deployments.contains_key(&deployment.service_ref) {
            return Err(ServiceBundleError::DeploymentCollision);
        }
        validate_deployment(&deployment, &registered.manifest)?;
        for other in self.deployments.values() {
            if other.provider.provider_ref == deployment.provider.provider_ref {
                return Err(ServiceBundleError::ProviderIdentityCollision);
            }
            if other.provider.authority == deployment.provider.authority {
                return Err(ServiceBundleError::ProviderAuthorityCollision);
            }
        }
        self.deployments
            .insert(deployment.service_ref.clone(), deployment);
        Ok(self)
    }

    /// Bind every explicitly deployed factory in deterministic provider order.
    pub async fn build(mut self) -> Result<ServiceBundle, ServiceBundleError> {
        let mut active = self
            .deployments
            .into_values()
            .map(|deployment| {
                let registered = self
                    .factories
                    .remove(&deployment.service_ref)
                    .expect("deploy validates service registration");
                (registered, deployment)
            })
            .collect::<Vec<_>>();
        active.sort_by(|left, right| {
            left.1
                .provider
                .provider_ref
                .cmp(&right.1.provider.provider_ref)
                .then_with(|| {
                    left.0
                        .manifest
                        .service_ref
                        .cmp(&right.0.manifest.service_ref)
                })
        });

        let mut services = Vec::with_capacity(active.len());
        let mut backends = Vec::<Arc<dyn ConnectorBackend>>::with_capacity(active.len());
        for (registered, deployment) in active {
            let expected = registered
                .manifest
                .operations
                .iter()
                .map(|operation| operation.operation_ref.clone())
                .collect::<BTreeSet<_>>();
            let dispatch = registered
                .factory
                .bind(&deployment)
                .await
                .map_err(|_| ServiceBundleError::FactoryBind)?;
            if dispatch.operation_refs() != &expected {
                return Err(ServiceBundleError::CatalogDispatchMismatch);
            }
            let (backend, _) = dispatch.into_parts();
            if !backend.capabilities().operations
                || expected.iter().any(|operation_ref| {
                    !backend.owns_operation(&OperationRequest::Describe(DescribeRequest {
                        operation_ref: operation_ref.clone(),
                    }))
                })
            {
                return Err(ServiceBundleError::BackendOwnershipMismatch);
            }

            let operations = registered
                .manifest
                .operations
                .iter()
                .cloned()
                .map(|operation| {
                    let policy = deployment
                        .operations
                        .get(&operation.operation_ref)
                        .expect("deploy validates the exact operation set")
                        .clone();
                    (operation.operation_ref.clone(), (operation, policy))
                })
                .collect();
            backends.push(Arc::new(DeployedServiceBackend {
                inner: backend,
                operations,
            }));
            services.push(DeployedService {
                manifest: registered.manifest,
                deployment,
            });
        }
        Ok(ServiceBundle { services, backends })
    }
}

/// One bound service and the exact reviewed configuration that activated it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployedService {
    pub manifest: ServiceManifest,
    pub deployment: ServiceDeployment,
}

/// Deterministically ordered generated-service contribution ready for runtime composition.
pub struct ServiceBundle {
    services: Vec<DeployedService>,
    backends: Vec<Arc<dyn ConnectorBackend>>,
}

impl ServiceBundle {
    #[must_use]
    pub fn services(&self) -> &[DeployedService] {
        &self.services
    }

    /// Clone the cheap backend handles so this bundle can join other runtime-owned backends.
    #[must_use]
    pub fn backends(&self) -> Vec<Arc<dyn ConnectorBackend>> {
        self.backends.clone()
    }

    /// Consume the bundle into handles suitable for extending an existing composition.
    #[must_use]
    pub fn into_backends(self) -> Vec<Arc<dyn ConnectorBackend>> {
        self.backends
    }

    /// Build the ordinary exact-ownership registry when this bundle is the complete composition.
    #[must_use]
    pub fn registry(&self) -> BackendRegistry {
        BackendRegistry::new(self.backends())
    }

    /// Consume this bundle into the ordinary exact-ownership registry.
    #[must_use]
    pub fn into_registry(self) -> BackendRegistry {
        BackendRegistry::new(self.into_backends())
    }
}

fn normalize_manifest(
    mut manifest: ServiceManifest,
) -> Result<ServiceManifest, ServiceBundleError> {
    if !valid_ref(&manifest.service_ref)
        || !valid_text(&manifest.provider.display_name, 256)
        || !valid_text(&manifest.provider.description, 4096)
        || manifest.operations.is_empty()
        || manifest.operations.iter().any(|operation| {
            !valid_ref(&operation.operation_ref)
                || !valid_text(&operation.title, 256)
                || !valid_text(&operation.description, 4096)
        })
    {
        return Err(ServiceBundleError::InvalidManifest);
    }
    manifest
        .operations
        .sort_by(|left, right| left.operation_ref.cmp(&right.operation_ref));
    if manifest
        .operations
        .windows(2)
        .any(|pair| pair[0].operation_ref == pair[1].operation_ref)
    {
        return Err(ServiceBundleError::InvalidManifest);
    }
    Ok(manifest)
}

fn validate_deployment(
    deployment: &ServiceDeployment,
    manifest: &ServiceManifest,
) -> Result<(), ServiceBundleError> {
    if !valid_provider_ref(&deployment.provider.provider_ref)
        || !valid_authority(&deployment.provider.authority)
    {
        return Err(ServiceBundleError::InvalidProviderIdentity);
    }
    let expected = manifest
        .operations
        .iter()
        .map(|operation| operation.operation_ref.as_str())
        .collect::<BTreeSet<_>>();
    let actual = deployment
        .operations
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(ServiceBundleError::OperationOverlayMismatch);
    }
    if deployment.operations.values().any(|operation| {
        invalid_bindings(&operation.endpoint_bindings)
            || invalid_bindings(&operation.credential_bindings)
            || operation
                .grant_refs
                .iter()
                .any(|reference| !valid_ref(reference) || reference.contains(['*', '?']))
    }) {
        return Err(ServiceBundleError::InvalidOperationDeployment);
    }
    Ok(())
}

fn invalid_bindings(bindings: &BTreeMap<String, String>) -> bool {
    bindings
        .iter()
        .any(|(name, reference)| !valid_ref(name) || !valid_ref(reference))
}

fn valid_provider_ref(value: &str) -> bool {
    valid_ref(value) && !value.contains(['/', '?', '#'])
}

fn valid_authority(value: &str) -> bool {
    value.len() <= 253
        && value.split('.').count() >= 2
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        })
}

fn valid_ref(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && !value.chars().any(char::is_control)
        && value.trim() == value
}

struct DeployedServiceBackend {
    inner: Arc<dyn ConnectorBackend>,
    operations: BTreeMap<String, (ServiceOperation, service::OperationDeployment)>,
}

impl DeployedServiceBackend {
    fn owns_declared_operation(&self, request: &OperationRequest) -> bool {
        let operation_ref = match request {
            OperationRequest::Describe(request) => Some(request.operation_ref.as_str()),
            OperationRequest::Invoke(request) => Some(request.operation_ref.as_str()),
            _ => None,
        };
        operation_ref.is_none_or(|reference| self.operations.contains_key(reference))
            && self.inner.owns_operation(request)
    }

    fn protocol(message: &'static str) -> OperationError {
        OperationError::new(OperationErrorCode::Protocol, message, false)
    }
}

#[async_trait]
impl ConnectorBackend for DeployedServiceBackend {
    async fn ready(&self) -> Result<(), BackendReadinessError> {
        self.inner.ready().await
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.inner.capabilities()
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        self.owns_declared_operation(request)
    }

    fn owns_connection(&self, request: &ConnectionRequest) -> bool {
        self.inner.owns_connection(request)
    }

    fn connect_session_access(
        &self,
        request: &protocol::connection::ConnectSessionCreateRequest,
    ) -> ConnectSessionAccess {
        self.inner.connect_session_access(request)
    }

    fn setup_profiles(&self, provider_ref: &str) -> Vec<protocol::catalog::SetupProfileSummary> {
        self.inner.setup_profiles(provider_ref)
    }

    fn owns_event(&self, request: &EventRequest) -> bool {
        self.inner.owns_event(request)
    }

    fn owns_datasource(&self, request: &DatasourceRequest) -> bool {
        self.inner.owns_datasource(request)
    }

    fn owns_hosted_completion(&self, connect_session_ref: &str) -> bool {
        self.inner.owns_hosted_completion(connect_session_ref)
    }

    fn hosted_completion_page(
        &self,
        connect_session_ref: &str,
    ) -> Result<HostedCompletionPage, HostedCompletionError> {
        self.inner.hosted_completion_page(connect_session_ref)
    }

    async fn complete_hosted_session(
        &self,
        connect_session_ref: &str,
        capability: &str,
        submission: HostedCompletionSubmission,
    ) -> Result<(), HostedCompletionError> {
        self.inner
            .complete_hosted_session(connect_session_ref, capability, submission)
            .await
    }

    fn owns_hosted_oauth_state(&self, integration_ref: &str, state: &str) -> bool {
        self.inner.owns_hosted_oauth_state(integration_ref, state)
    }

    async fn complete_hosted_oauth(
        &self,
        integration_ref: &str,
        state: &str,
        code: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), HostedCompletionError> {
        self.inner
            .complete_hosted_oauth(integration_ref, state, code, error)
            .await
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        match request {
            OperationRequest::Search(search) => {
                let result = self
                    .inner
                    .handle(context, OperationRequest::Search(search))
                    .await?;
                let OperationResult::Search { operations } = result else {
                    return Err(Self::protocol(
                        "generated service search returned a wrong result",
                    ));
                };
                let mut visible = BTreeMap::new();
                for mut summary in operations {
                    let Some((catalog, deployment)) = self.operations.get(&summary.operation_ref)
                    else {
                        continue;
                    };
                    if !deployment.expose {
                        continue;
                    }
                    summary.title.clone_from(&catalog.title);
                    summary.effect = catalog.effect;
                    summary.approval = deployment.approval;
                    if visible
                        .insert(summary.operation_ref.clone(), summary)
                        .is_some()
                    {
                        return Err(Self::protocol(
                            "generated service search returned a duplicate operation",
                        ));
                    }
                }
                Ok(OperationResult::Search {
                    operations: visible.into_values().collect(),
                })
            }
            OperationRequest::Describe(describe) => {
                let Some((catalog, deployment)) = self.operations.get(&describe.operation_ref)
                else {
                    return Err(Self::protocol(
                        "generated service described an undeclared operation",
                    ));
                };
                let result = self
                    .inner
                    .handle(context, OperationRequest::Describe(describe.clone()))
                    .await?;
                let OperationResult::Describe(mut description) = result else {
                    return Err(Self::protocol(
                        "generated service describe returned a wrong result",
                    ));
                };
                if description.operation_ref != describe.operation_ref {
                    return Err(Self::protocol(
                        "generated service describe returned a wrong operation identity",
                    ));
                }
                description.title.clone_from(&catalog.title);
                description.description.clone_from(&catalog.description);
                description.input_schema.clone_from(&catalog.input_schema);
                description.output_schema.clone_from(&catalog.output_schema);
                description.effect = catalog.effect;
                description.approval = deployment.approval;
                Ok(OperationResult::Describe(description))
            }
            other => self.inner.handle(context, other).await,
        }
    }

    async fn handle_connection(
        &self,
        context: &PrincipalContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.inner.handle_connection(context, request).await
    }

    async fn handle_event(
        &self,
        context: &PrincipalContext,
        request: EventRequest,
    ) -> Result<EventResult, EventError> {
        self.inner.handle_event(context, request).await
    }

    async fn handle_datasource(
        &self,
        context: &PrincipalContext,
        request: DatasourceRequest,
    ) -> Result<DatasourceResult, DatasourceError> {
        self.inner.handle_datasource(context, request).await
    }

    async fn shutdown(&self) {
        self.inner.shutdown().await;
    }
}

#[cfg(test)]
mod tests {
    use protocol::operation::{
        ApprovalPosture, EffectClass, OperationDescription, OperationSummary, SearchRequest,
    };
    use serde_json::json;
    use service::{
        DeploymentRisk, OperationDeployment, ProviderIdentity, ServiceDispatch,
        ServiceFactoryBindError, ServiceProviderMetadata,
    };

    use super::*;

    const FIRST_OPERATION: &str = "devcenter.todo.list.create";
    const SECOND_OPERATION: &str = "devcenter.todo.item.create";

    struct SyntheticFactory {
        manifest: ServiceManifest,
        dispatch: BTreeSet<String>,
        claims_catalog: bool,
        refuses_bind: bool,
    }

    impl SyntheticFactory {
        fn new(service_ref: &str, operations: &[&str]) -> Self {
            let operations = operations
                .iter()
                .map(|operation_ref| ServiceOperation {
                    operation_ref: (*operation_ref).to_owned(),
                    title: format!("Operation {operation_ref}"),
                    description: format!("Runs {operation_ref}"),
                    input_schema: json!({"type": "object"}),
                    output_schema: json!({"type": "object"}),
                    effect: EffectClass::Mutating,
                })
                .collect::<Vec<_>>();
            Self {
                dispatch: operations
                    .iter()
                    .map(|operation| operation.operation_ref.clone())
                    .collect(),
                manifest: ServiceManifest {
                    service_ref: service_ref.to_owned(),
                    provider: ServiceProviderMetadata {
                        display_name: format!("Provider {service_ref}"),
                        description: "Synthetic generated service".to_owned(),
                    },
                    operations,
                },
                claims_catalog: true,
                refuses_bind: false,
            }
        }
    }

    #[async_trait]
    impl ConnectorServiceFactory for SyntheticFactory {
        fn manifest(&self) -> ServiceManifest {
            self.manifest.clone()
        }

        async fn bind(
            &self,
            _deployment: &ServiceDeployment,
        ) -> Result<ServiceDispatch, ServiceFactoryBindError> {
            if self.refuses_bind {
                return Err(ServiceFactoryBindError);
            }
            Ok(ServiceDispatch::new(
                Arc::new(SyntheticBackend {
                    operations: self.dispatch.clone(),
                    claims_catalog: self.claims_catalog,
                }),
                self.dispatch.iter().cloned(),
            ))
        }
    }

    struct SyntheticBackend {
        operations: BTreeSet<String>,
        claims_catalog: bool,
    }

    #[async_trait]
    impl ConnectorBackend for SyntheticBackend {
        async fn ready(&self) -> Result<(), BackendReadinessError> {
            Ok(())
        }

        fn owns_operation(&self, request: &OperationRequest) -> bool {
            if !self.claims_catalog {
                return false;
            }
            match request {
                OperationRequest::Describe(request) => {
                    self.operations.contains(&request.operation_ref)
                }
                OperationRequest::Invoke(request) => {
                    self.operations.contains(&request.operation_ref)
                }
                _ => false,
            }
        }

        async fn handle(
            &self,
            _context: &PrincipalContext,
            request: OperationRequest,
        ) -> Result<OperationResult, OperationError> {
            match request {
                OperationRequest::Search(_) => Ok(OperationResult::Search {
                    operations: self
                        .operations
                        .iter()
                        .map(|operation_ref| OperationSummary {
                            operation_ref: operation_ref.clone(),
                            title: "backend title is replaced".to_owned(),
                            effect: EffectClass::ReadOnly,
                            approval: ApprovalPosture::NotRequired,
                            connections: Vec::new(),
                        })
                        .collect(),
                }),
                OperationRequest::Describe(request) => {
                    Ok(OperationResult::Describe(OperationDescription {
                        operation_ref: request.operation_ref,
                        title: "backend title is replaced".to_owned(),
                        description: "backend description is replaced".to_owned(),
                        input_schema: json!(true),
                        output_schema: json!(true),
                        effect: EffectClass::ReadOnly,
                        approval: ApprovalPosture::NotRequired,
                        connections: Vec::new(),
                        description_ref: "description:synthetic".to_owned(),
                    }))
                }
                _ => Err(OperationError::new(
                    OperationErrorCode::NotFound,
                    "synthetic operation not found",
                    false,
                )),
            }
        }
    }

    fn operation_policy(expose: bool) -> OperationDeployment {
        OperationDeployment {
            expose,
            risk: DeploymentRisk::Medium,
            approval: ApprovalPosture::Required,
            endpoint_bindings: BTreeMap::from([(
                "service".to_owned(),
                "endpoint:devcenter".to_owned(),
            )]),
            credential_bindings: BTreeMap::from([(
                "service".to_owned(),
                "credential:devcenter-service".to_owned(),
            )]),
            grant_refs: BTreeSet::from(["grant:devcenter-todo".to_owned()]),
        }
    }

    fn deployment(
        service_ref: &str,
        provider_ref: &str,
        authority: &str,
        operations: &[(&str, bool)],
    ) -> ServiceDeployment {
        ServiceDeployment {
            service_ref: service_ref.to_owned(),
            provider: ProviderIdentity {
                provider_ref: provider_ref.to_owned(),
                authority: authority.to_owned(),
            },
            operations: operations
                .iter()
                .map(|(operation, expose)| ((*operation).to_owned(), operation_policy(*expose)))
                .collect(),
        }
    }

    fn context() -> PrincipalContext {
        PrincipalContext::hosted(
            "tenant-dev".to_owned(),
            "person:test".to_owned(),
            "person:test".to_owned(),
            None,
            "snapshot:test".to_owned(),
            "a".repeat(64),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn registration_is_inert_until_an_explicit_overlay_is_present() {
        let mut builder = ServiceBundleBuilder::new();
        builder
            .register(SyntheticFactory::new("service:todo", &[FIRST_OPERATION]))
            .unwrap();
        let bundle = builder.build().await.unwrap();
        assert!(bundle.services().is_empty());
        assert!(bundle.backends().is_empty());

        let result = bundle
            .registry()
            .handle(
                &context(),
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: FIRST_OPERATION.to_owned(),
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(result.code, OperationErrorCode::NotFound);
    }

    #[tokio::test]
    async fn bundle_order_and_policy_projection_are_deterministic() {
        async fn build(reverse: bool) -> ServiceBundle {
            let todo = SyntheticFactory::new("service:todo", &[FIRST_OPERATION]);
            let usage = SyntheticFactory::new("service:usage", &[SECOND_OPERATION]);
            let mut builder = ServiceBundleBuilder::new();
            if reverse {
                builder.register(usage).unwrap().register(todo).unwrap();
            } else {
                builder.register(todo).unwrap().register(usage).unwrap();
            }
            let todo = deployment(
                "service:todo",
                "provider:todo",
                "dev.b10x.todo",
                &[(FIRST_OPERATION, false)],
            );
            let usage = deployment(
                "service:usage",
                "provider:usage",
                "dev.b10x.usage",
                &[(SECOND_OPERATION, true)],
            );
            if reverse {
                builder.deploy(usage).unwrap().deploy(todo).unwrap();
            } else {
                builder.deploy(todo).unwrap().deploy(usage).unwrap();
            }
            builder.build().await.unwrap()
        }

        let first = build(false).await;
        let second = build(true).await;
        let identities = |bundle: &ServiceBundle| {
            bundle
                .services()
                .iter()
                .map(|service| service.deployment.provider.provider_ref.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(identities(&first), identities(&second));
        assert_eq!(
            identities(&first),
            vec!["provider:todo".to_owned(), "provider:usage".to_owned()]
        );

        let result = first
            .registry()
            .handle(
                &context(),
                OperationRequest::Search(SearchRequest {
                    query: String::new(),
                    limit: 10,
                }),
            )
            .await
            .unwrap();
        let OperationResult::Search { operations } = result else {
            panic!("search result");
        };
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].operation_ref, SECOND_OPERATION);
        assert_eq!(operations[0].effect, EffectClass::Mutating);
        assert_eq!(operations[0].approval, ApprovalPosture::Required);
    }

    #[test]
    fn identity_and_operation_collisions_are_refused() {
        let mut service_collision = ServiceBundleBuilder::new();
        service_collision
            .register(SyntheticFactory::new("service:a", &[FIRST_OPERATION]))
            .unwrap();
        assert_eq!(
            service_collision
                .register(SyntheticFactory::new("service:a", &[SECOND_OPERATION]))
                .err(),
            Some(ServiceBundleError::ServiceIdentityCollision)
        );

        let mut operation_collision = ServiceBundleBuilder::new();
        operation_collision
            .register(SyntheticFactory::new("service:a", &[FIRST_OPERATION]))
            .unwrap();
        assert_eq!(
            operation_collision
                .register(SyntheticFactory::new("service:b", &[FIRST_OPERATION]))
                .err(),
            Some(ServiceBundleError::OperationIdentityCollision)
        );

        let mut provider_collision = ServiceBundleBuilder::new();
        provider_collision
            .register(SyntheticFactory::new("service:a", &[FIRST_OPERATION]))
            .unwrap()
            .register(SyntheticFactory::new("service:b", &[SECOND_OPERATION]))
            .unwrap()
            .deploy(deployment(
                "service:a",
                "provider:shared",
                "dev.b10x.a",
                &[(FIRST_OPERATION, true)],
            ))
            .unwrap();
        assert_eq!(
            provider_collision
                .deploy(deployment(
                    "service:b",
                    "provider:shared",
                    "dev.b10x.b",
                    &[(SECOND_OPERATION, true)],
                ))
                .err(),
            Some(ServiceBundleError::ProviderIdentityCollision)
        );

        let mut authority_collision = ServiceBundleBuilder::new();
        authority_collision
            .register(SyntheticFactory::new("service:a", &[FIRST_OPERATION]))
            .unwrap()
            .register(SyntheticFactory::new("service:b", &[SECOND_OPERATION]))
            .unwrap()
            .deploy(deployment(
                "service:a",
                "provider:a",
                "dev.b10x.shared",
                &[(FIRST_OPERATION, true)],
            ))
            .unwrap();
        assert_eq!(
            authority_collision
                .deploy(deployment(
                    "service:b",
                    "provider:b",
                    "dev.b10x.shared",
                    &[(SECOND_OPERATION, true)],
                ))
                .err(),
            Some(ServiceBundleError::ProviderAuthorityCollision)
        );
    }

    #[tokio::test]
    async fn catalog_dispatch_and_backend_ownership_mismatches_are_refused() {
        let mut dispatch_mismatch = SyntheticFactory::new("service:todo", &[FIRST_OPERATION]);
        dispatch_mismatch.dispatch.clear();
        let mut builder = ServiceBundleBuilder::new();
        builder
            .register(dispatch_mismatch)
            .unwrap()
            .deploy(deployment(
                "service:todo",
                "provider:todo",
                "dev.b10x.todo",
                &[(FIRST_OPERATION, true)],
            ))
            .unwrap();
        assert!(matches!(
            builder.build().await,
            Err(ServiceBundleError::CatalogDispatchMismatch)
        ));

        let mut ownership_mismatch = SyntheticFactory::new("service:todo", &[FIRST_OPERATION]);
        ownership_mismatch.claims_catalog = false;
        let mut builder = ServiceBundleBuilder::new();
        builder
            .register(ownership_mismatch)
            .unwrap()
            .deploy(deployment(
                "service:todo",
                "provider:todo",
                "dev.b10x.todo",
                &[(FIRST_OPERATION, true)],
            ))
            .unwrap();
        assert!(matches!(
            builder.build().await,
            Err(ServiceBundleError::BackendOwnershipMismatch)
        ));

        let mut factory_refusal = SyntheticFactory::new("service:todo", &[FIRST_OPERATION]);
        factory_refusal.refuses_bind = true;
        let mut builder = ServiceBundleBuilder::new();
        builder
            .register(factory_refusal)
            .unwrap()
            .deploy(deployment(
                "service:todo",
                "provider:todo",
                "dev.b10x.todo",
                &[(FIRST_OPERATION, true)],
            ))
            .unwrap();
        assert!(matches!(
            builder.build().await,
            Err(ServiceBundleError::FactoryBind)
        ));
    }

    #[test]
    fn malformed_manifests_and_deployments_are_refused() {
        let mut malformed = SyntheticFactory::new("service:todo", &[FIRST_OPERATION]);
        malformed.manifest.provider.display_name = " ".to_owned();
        assert_eq!(
            ServiceBundleBuilder::new().register(malformed).err(),
            Some(ServiceBundleError::InvalidManifest)
        );

        let duplicate_operation =
            SyntheticFactory::new("service:todo", &[FIRST_OPERATION, FIRST_OPERATION]);
        assert_eq!(
            ServiceBundleBuilder::new()
                .register(duplicate_operation)
                .err(),
            Some(ServiceBundleError::InvalidManifest)
        );

        let mut unknown = ServiceBundleBuilder::new();
        assert_eq!(
            unknown
                .deploy(deployment(
                    "service:missing",
                    "provider:todo",
                    "dev.b10x.todo",
                    &[(FIRST_OPERATION, true)],
                ))
                .err(),
            Some(ServiceBundleError::UnknownService)
        );

        let mut builder = ServiceBundleBuilder::new();
        builder
            .register(SyntheticFactory::new(
                "service:todo",
                &[FIRST_OPERATION, SECOND_OPERATION],
            ))
            .unwrap();
        assert_eq!(
            builder
                .deploy(deployment(
                    "service:todo",
                    "provider:todo",
                    "dev.b10x.todo",
                    &[(FIRST_OPERATION, true)],
                ))
                .err(),
            Some(ServiceBundleError::OperationOverlayMismatch)
        );

        let mut builder = ServiceBundleBuilder::new();
        builder
            .register(SyntheticFactory::new("service:todo", &[FIRST_OPERATION]))
            .unwrap();
        let invalid_identity = deployment(
            "service:todo",
            "provider:todo",
            "Dev.B10x.Todo",
            &[(FIRST_OPERATION, true)],
        );
        assert_eq!(
            builder.deploy(invalid_identity).err(),
            Some(ServiceBundleError::InvalidProviderIdentity)
        );

        let mut invalid_resource = deployment(
            "service:todo",
            "provider:todo",
            "dev.b10x.todo",
            &[(FIRST_OPERATION, true)],
        );
        invalid_resource
            .operations
            .get_mut(FIRST_OPERATION)
            .unwrap()
            .grant_refs
            .insert("grant:*".to_owned());
        assert_eq!(
            builder.deploy(invalid_resource).err(),
            Some(ServiceBundleError::InvalidOperationDeployment)
        );

        let valid = deployment(
            "service:todo",
            "provider:todo",
            "dev.b10x.todo",
            &[(FIRST_OPERATION, true)],
        );
        builder.deploy(valid.clone()).unwrap();
        assert_eq!(
            builder.deploy(valid).err(),
            Some(ServiceBundleError::DeploymentCollision)
        );
    }
}

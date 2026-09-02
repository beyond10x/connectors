//! Generator-facing contract for contributing one service to a Connector composition.
//!
//! A factory declaration is inert. It describes a provider-shaped catalog, but it cannot expose
//! an operation, resolve an endpoint or credential, or grant authority until a composition passes
//! an explicit [`ServiceDeployment`] to [`ConnectorServiceFactory::bind`].

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::ConnectorBackend;

/// The reviewed deployment risk vocabulary consumed by Connector grants.
pub use domain::GrantRisk as DeploymentRisk;
/// The approval posture published for one deployed operation.
pub use protocol::operation::ApprovalPosture as DeploymentApproval;
/// The protocol effect published by one generated operation.
pub use protocol::operation::EffectClass as OperationEffect;

/// Product-neutral metadata generated for the provider represented by one service.
///
/// Provider identity is deliberately absent. A deployment assigns the permanent reference and
/// reverse-DNS authority through [`ProviderIdentity`]; generated code cannot silently choose them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceProviderMetadata {
    pub display_name: String,
    pub description: String,
}

/// One stable operation in a generated service catalog.
///
/// Risk, approval and exposure are deployment policy and therefore live in
/// [`OperationDeployment`]. Schemas and effect remain authored service facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceOperation {
    pub operation_ref: String,
    pub title: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub effect: OperationEffect,
}

/// Complete, deterministic catalog contribution emitted by a service generator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceManifest {
    /// Stable identity of the generated service definition, independent of deployment identity.
    pub service_ref: String,
    pub provider: ServiceProviderMetadata,
    pub operations: Vec<ServiceOperation>,
}

/// Permanent provider identity assigned by reviewed deployment configuration.
///
/// Once published, neither field may be repointed. Bundle construction rejects collisions across
/// every bound factory before any backend is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderIdentity {
    pub provider_ref: String,
    pub authority: String,
    /// Deployment-owned credential-free Connection through which this service is invoked.
    pub connection_ref: String,
}

/// Deployment policy and value-free resource bindings for one operation.
///
/// Endpoint values and credential bytes do not belong here. The maps bind names emitted by the
/// service definition to opaque deployment-owned references. `grant_refs` assigns stable records
/// owned by this generated-service deployment; hosted composition derives their closed operation
/// sets and merges them into the tenant's durable authority state only when the bundle activates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationDeployment {
    pub expose: bool,
    pub risk: DeploymentRisk,
    pub approval: DeploymentApproval,
    pub endpoint_bindings: BTreeMap<String, String>,
    pub credential_bindings: BTreeMap<String, String>,
    pub grant_refs: BTreeSet<String>,
}

/// Explicit activation overlay for one previously registered service factory.
///
/// The operation map must name the manifest's exact operation set. Omitting an entry cannot make a
/// policy decision by silence, and adding an unknown entry cannot create a route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDeployment {
    pub service_ref: String,
    pub provider: ProviderIdentity,
    pub operations: BTreeMap<String, OperationDeployment>,
}

/// Value-free refusal returned when a factory cannot bind reviewed deployment configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a Connector service factory could not bind its deployment overlay")]
pub struct ServiceFactoryBindError;

/// One backend produced by a factory and the exact operation catalog it dispatches.
///
/// Keeping the dispatch set separate from the manifest is intentional: the composition root can
/// mechanically refuse generator/runtime drift rather than trusting two generated artifacts to
/// agree because they came from the same build.
pub struct ServiceDispatch {
    backend: Arc<dyn ConnectorBackend>,
    operation_refs: BTreeSet<String>,
}

impl ServiceDispatch {
    #[must_use]
    pub fn new(
        backend: Arc<dyn ConnectorBackend>,
        operation_refs: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            backend,
            operation_refs: operation_refs.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn operation_refs(&self) -> &BTreeSet<String> {
        &self.operation_refs
    }

    #[must_use]
    pub fn into_parts(self) -> (Arc<dyn ConnectorBackend>, BTreeSet<String>) {
        (self.backend, self.operation_refs)
    }
}

/// Generator-targetable factory for one provider-shaped service contribution.
///
/// `manifest` must be pure and stable. `bind` is called only for a service carrying an explicit
/// deployment overlay; merely registering the factory never constructs an active backend.
#[async_trait]
pub trait ConnectorServiceFactory: Send + Sync + 'static {
    fn manifest(&self) -> ServiceManifest;

    async fn bind(
        &self,
        deployment: &ServiceDeployment,
    ) -> Result<ServiceDispatch, ServiceFactoryBindError>;
}

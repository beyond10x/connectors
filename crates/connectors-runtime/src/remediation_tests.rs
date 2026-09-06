//! Actual registry routing: predicates select owners; refusals never trigger fallback dispatch.
use super::*;
use service::{
    CredentialReadiness, RemediationError, RemediationMetadata, RemediationRoute, RemediationTarget,
};
use std::sync::atomic::{AtomicU64, Ordering};

struct RemediationBackend {
    owns: bool,
    readiness_calls: AtomicU64,
}
#[async_trait]
impl ConnectorBackend for RemediationBackend {
    async fn ready(&self) -> Result<(), BackendReadinessError> {
        Ok(())
    }
    async fn handle(
        &self,
        _: &PrincipalContext,
        _: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        panic!("readiness must not dispatch an ordinary operation")
    }
    fn owns_remediation(&self, route: RemediationRoute<'_>) -> bool {
        self.owns
            && matches!(route, RemediationRoute::Target(target) if target.operation_ref == "slack-conversations-history" && target.connection_ref == "connection:configured")
    }
    fn remediation_metadata<'a>(
        &'a self,
        _: &PrincipalContext,
        _: RemediationTarget<'_>,
    ) -> Result<RemediationMetadata<'a>, RemediationError> {
        Err(RemediationError::Refused)
    }
    async fn credential_readiness(
        &self,
        _: &PrincipalContext,
        _: RemediationTarget<'_>,
    ) -> CredentialReadiness {
        self.readiness_calls.fetch_add(1, Ordering::SeqCst);
        CredentialReadiness::MissingCredential
    }
}
fn principal() -> PrincipalContext {
    PrincipalContext::local(&protocol::operation::OwnerContext {
        tenant_id: "local".into(),
        agent_id: "fixture".into(),
        agent_revision: 1,
        authority_snapshot_id: "snapshot:fixture".into(),
        authority_snapshot_sha256: "a".repeat(64),
    })
    .unwrap()
}
fn target() -> RemediationTarget<'static> {
    RemediationTarget {
        operation_ref: "slack-conversations-history",
        connection_ref: "connection:configured",
    }
}
#[tokio::test]
async fn remediation_registry_uses_one_exact_owner_without_operation_dispatch() {
    let backend = Arc::new(RemediationBackend {
        owns: true,
        readiness_calls: AtomicU64::new(0),
    });
    let registry = BackendRegistry::new(vec![backend.clone()]);
    assert!(registry.owns_remediation(RemediationRoute::Target(target())));
    assert_eq!(
        registry.credential_readiness(&principal(), target()).await,
        CredentialReadiness::MissingCredential
    );
    assert_eq!(backend.readiness_calls.load(Ordering::SeqCst), 1);
    assert!(matches!(
        registry.remediation_metadata(&principal(), target()),
        Err(RemediationError::Refused)
    ));
}
#[tokio::test]
async fn remediation_registry_ambiguity_and_absence_do_not_probe_any_owner() {
    let first = Arc::new(RemediationBackend {
        owns: true,
        readiness_calls: AtomicU64::new(0),
    });
    let second = Arc::new(RemediationBackend {
        owns: true,
        readiness_calls: AtomicU64::new(0),
    });
    let ambiguous = BackendRegistry::new(vec![first.clone(), second.clone()]);
    assert_eq!(
        ambiguous.credential_readiness(&principal(), target()).await,
        CredentialReadiness::DependencyUnavailable
    );
    assert!(matches!(
        ambiguous.remediation_metadata(&principal(), target()),
        Err(RemediationError::Unavailable)
    ));
    let absent = BackendRegistry::new(vec![first.clone()]);
    let unknown = RemediationTarget {
        operation_ref: "unknown",
        connection_ref: "connection:configured",
    };
    assert_eq!(
        absent.credential_readiness(&principal(), unknown).await,
        CredentialReadiness::Unsupported
    );
    assert!(matches!(
        absent.remediation_metadata(&principal(), unknown),
        Err(RemediationError::Refused)
    ));
    assert_eq!(first.readiness_calls.load(Ordering::SeqCst), 0);
    assert_eq!(second.readiness_calls.load(Ordering::SeqCst), 0);
}

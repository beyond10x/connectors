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

struct AuthAdversaryOptionalRoute {
    operation: bool,
    connection: bool,
    route: bool,
    unavailable: bool,
    metadata_calls: AtomicU64,
}
#[async_trait]
impl ConnectorBackend for AuthAdversaryOptionalRoute {
    async fn ready(&self) -> Result<(), BackendReadinessError> {
        Ok(())
    }
    fn owns_operation(&self, _: &OperationRequest) -> bool {
        self.operation
    }
    fn owns_connection(&self, _: &ConnectionRequest) -> bool {
        self.connection
    }
    fn owns_remediation(&self, _: RemediationRoute<'_>) -> bool {
        self.route
    }
    fn remediation_metadata<'a>(
        &'a self,
        _: &PrincipalContext,
        _: RemediationTarget<'_>,
    ) -> Result<RemediationMetadata<'a>, RemediationError> {
        self.metadata_calls.fetch_add(1, Ordering::SeqCst);
        Err(if self.route {
            if self.unavailable {
                RemediationError::Unavailable
            } else {
                RemediationError::Refused
            }
        } else {
            RemediationError::Unsupported
        })
    }
    async fn handle(
        &self,
        _: &PrincipalContext,
        _: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        panic!("metadata selection must not execute an operation")
    }
}
#[tokio::test]
async fn auth_adversary_registry_never_combines_split_owners_or_falls_through_claimed_errors() {
    let mut observations = Vec::new();
    for case in [
        "ordinary",
        "operation-only",
        "split",
        "ambiguous-ordinary",
        "claimed-refused",
        "claimed-unavailable",
    ] {
        let first = Arc::new(AuthAdversaryOptionalRoute {
            operation: true,
            connection: !matches!(case, "split" | "operation-only"),
            route: case.starts_with("claimed"),
            unavailable: case == "claimed-unavailable",
            metadata_calls: 0.into(),
        });
        let second = Arc::new(AuthAdversaryOptionalRoute {
            operation: case == "ambiguous-ordinary",
            connection: true,
            route: false,
            unavailable: false,
            metadata_calls: 0.into(),
        });
        let registry = BackendRegistry::new(if matches!(case, "split" | "ambiguous-ordinary") {
            vec![first.clone(), second.clone()]
        } else {
            vec![first.clone()]
        });
        let result = registry.remediation_metadata(&principal(), target());
        let classification = match result {
            Err(RemediationError::Refused) => "refused",
            Err(RemediationError::Unavailable) => "unavailable",
            Err(RemediationError::Unsupported) => "unsupported",
            _ => "unexpected",
        };
        observations.push((
            case,
            classification,
            first.metadata_calls.load(Ordering::SeqCst),
            second.metadata_calls.load(Ordering::SeqCst),
        ));
    }
    eprintln!("actual registry owner/error observations: {observations:?}");
    assert_eq!(
        observations,
        [
            ("ordinary", "unsupported", 1, 0),
            ("operation-only", "unsupported", 1, 0),
            ("split", "refused", 0, 0),
            ("ambiguous-ordinary", "unavailable", 0, 0),
            ("claimed-refused", "refused", 1, 0),
            ("claimed-unavailable", "unavailable", 1, 0),
        ]
    );
}

struct AuthAdversary2SessionOwner {
    session: &'static str,
    result: service::RemediationError,
    calls: AtomicU64,
}
#[async_trait]
impl ConnectorBackend for AuthAdversary2SessionOwner {
    async fn ready(&self) -> Result<(), BackendReadinessError> {
        Ok(())
    }
    fn owns_remediation(&self, route: service::RemediationRoute<'_>) -> bool {
        matches!(route, service::RemediationRoute::Session(reference) if reference == self.session)
    }
    async fn handle_remediation(
        &self,
        _: &PrincipalContext,
        request: service::RemediationRequest,
        _: Arc<dyn service::RemediationAuthority>,
    ) -> Result<service::RemediationResult, service::RemediationError> {
        assert!(matches!(
            request,
            service::RemediationRequest::Status(_) | service::RemediationRequest::Acknowledge(_)
        ));
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(self.result)
    }
    async fn handle(
        &self,
        _: &PrincipalContext,
        _: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        panic!("session routing must not dispatch an operation")
    }
}
struct AuthAdversary2NoAdmission;
impl service::RemediationAuthority for AuthAdversary2NoAdmission {
    fn recheck(
        &self,
        _: &PrincipalContext,
        _: &service::RemediationBinding,
        _: u64,
    ) -> Result<(), service::RemediationError> {
        panic!("the registry must preserve its selected owner's refusal")
    }
}
#[tokio::test]
async fn auth_adversary2_session_routing_refuses_ambiguity_and_never_falls_back_after_claim() {
    use protocol::connection_v2::{RemediationAcknowledgeRequest, RemediationStatusRequest};
    for scenario in [
        "single-refused",
        "single-unavailable",
        "single-unsupported",
        "ambiguous",
        "unknown",
    ] {
        for acknowledge in [false, true] {
            let expected = match scenario {
                "single-unavailable" | "ambiguous" => RemediationError::Unavailable,
                "single-unsupported" => RemediationError::Unsupported,
                _ => RemediationError::Refused,
            };
            let first = Arc::new(AuthAdversary2SessionOwner {
                session: "session:owned",
                result: expected,
                calls: 0.into(),
            });
            let second = Arc::new(AuthAdversary2SessionOwner {
                session: if scenario == "ambiguous" {
                    "session:owned"
                } else {
                    "session:decoy"
                },
                result: RemediationError::Unavailable,
                calls: 0.into(),
            });
            let registry = BackendRegistry::new(vec![first.clone(), second.clone()]);
            let reference = if scenario == "unknown" {
                "session:unknown"
            } else {
                "session:owned"
            };
            let request = if acknowledge {
                service::RemediationRequest::Acknowledge(RemediationAcknowledgeRequest {
                    connect_session_ref: reference.into(),
                    operation_ref: target().operation_ref.into(),
                    connection_ref: target().connection_ref.into(),
                })
            } else {
                service::RemediationRequest::Status(RemediationStatusRequest {
                    connect_session_ref: reference.into(),
                })
            };
            let result = registry
                .handle_remediation(&principal(), request, Arc::new(AuthAdversary2NoAdmission))
                .await;
            let Err(actual) = result else {
                panic!("refusal was promoted to readiness")
            };
            assert_eq!(actual, expected);
            let calls = first.calls.load(Ordering::SeqCst);
            assert_eq!(calls, u64::from(scenario.starts_with("single-")));
            assert_eq!(second.calls.load(Ordering::SeqCst), 0);
            eprintln!(
                "session route {scenario}, acknowledge={acknowledge}: {actual:?}, calls={calls}"
            );
        }
    }
}

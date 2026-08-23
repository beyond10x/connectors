use std::collections::BTreeSet;

use crate::{ConnectionAuthority, GrantDecision, RouteAdapter};

/// Closed built-in driver identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DriverId {
    HttpV1,
    SipV1,
    AudioV1,
    CdpV1,
}

impl DriverId {
    /// Stable catalog token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HttpV1 => "http_v1",
            Self::SipV1 => "sip_v1",
            Self::AudioV1 => "audio_v1",
            Self::CdpV1 => "cdp_v1",
        }
    }
}

/// Lifecycle shape independently of its driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interaction {
    Unary,
    Stream,
    Subscription,
    LeasedSession,
    SessionEstablishment,
}

/// Selected placement for this admitted operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    ConnectorsDeployment,
    SubstrateWorkload,
    FederatedSatellite,
}

/// Closed implementation form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Implementation {
    BuiltIn,
}

/// Host authority that must exist before credentials are materialized or a driver runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    PublicNetwork,
    PrivateNetwork,
    UnixSocket,
    FileSecret,
    Process,
    Container,
    Device,
}

/// Reviewed catalog facts carried into dispatch without rereading a declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationFacts {
    pub provider: String,
    pub operation: String,
    pub service: String,
    pub interaction: Interaction,
    pub placement: Placement,
    pub implementation: Implementation,
    pub required_capabilities: BTreeSet<Capability>,
    pub permission_subjects: Vec<String>,
}

/// Evidence that authentication and the exact Connector Grant were admitted before planning.
///
/// Fields are private so downstream code cannot reinterpret a tenant, principal, or grant after
/// the plan is built, and both constructors name the authority they stand on: a hosted
/// invocation is admitted by a [`GrantDecision`] proof, a personal placement by the local-owner
/// assertion. There is no constructor over bare caller-supplied claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedOperation {
    provider: String,
    operation: String,
    organization: String,
    principal: String,
    grant: String,
    connection: ConnectionAuthority,
}

impl AdmittedOperation {
    /// Admission proven by a Grant evaluation. [`GrantDecision`] has no public constructor, so
    /// reaching this call means `GrantEvaluator::evaluate` produced the decision.
    ///
    /// **An expired decision refuses at use**: the caller states its `now` and a decision past
    /// its bound expiry answers the same neutral refusal every other refusal is.
    ///
    /// # Errors
    ///
    /// [`crate::GrantRefusal::Refused`] when the decision has expired.
    pub fn from_decision(
        decision: GrantDecision,
        now: std::time::SystemTime,
    ) -> Result<Self, crate::GrantRefusal> {
        let parts = decision.into_parts(now)?;
        Ok(Self {
            provider: parts.provider,
            operation: parts.operation,
            organization: parts.organization,
            principal: parts.principal,
            grant: parts.grant,
            connection: parts.connection,
        })
    }

    /// Admission asserted, not evaluated: the caller vouches that the requesting principal IS
    /// the deployment owner — a peer already authenticated on the owner-only Unix socket, or
    /// in-process composition inside a personal placement the owner started.
    ///
    /// No Grant evaluation runs on this path. Hosted request handling must never construct
    /// admission through it; a hosted invocation is admitted only by a [`GrantDecision`].
    pub fn for_local_owner(
        provider: impl Into<String>,
        operation: impl Into<String>,
        organization: impl Into<String>,
        principal: impl Into<String>,
        grant: impl Into<String>,
        connection: ConnectionAuthority,
    ) -> Self {
        Self {
            provider: provider.into(),
            operation: operation.into(),
            organization: organization.into(),
            principal: principal.into(),
            grant: grant.into(),
            connection,
        }
    }

    pub fn provider(&self) -> &str {
        &self.provider
    }

    pub fn operation(&self) -> &str {
        &self.operation
    }

    pub fn organization(&self) -> &str {
        &self.organization
    }

    pub fn principal(&self) -> &str {
        &self.principal
    }

    pub fn grant(&self) -> &str {
        &self.grant
    }

    pub fn connection(&self) -> &str {
        self.connection.id()
    }

    pub fn connection_authority(&self) -> &ConnectionAuthority {
        &self.connection
    }
}

/// An inert HTTP template. It contains no credential and opens no socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpPlan {
    pub method: String,
    pub url_template: String,
}

/// An inert target-Provider HTTP operation routed through a parent Connection.
///
/// It deliberately carries a target-relative path rather than the target Provider's direct origin,
/// so a mediated route cannot silently fall back to direct egress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediatedHttpPlan {
    pub method: String,
    pub target_path_template: String,
    pub parent_connection: String,
    pub resource_binding: String,
    pub adapter: RouteAdapter,
}

/// An inert SIP establishment plan. Destination policy and media/listener apertures are selected
/// deployment facts; no caller-provided URI or socket appears here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SipPlan {
    pub connection: String,
}

/// An inert local-audio plan. The synthesizer, voice model and audio sink are deployment-selected
/// facts resolved after admission; no caller-provided path, device or executable appears here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioPlan {
    pub connection: String,
}

/// An inert browser plan. The executable, the dedicated profile directory and the artifact
/// directory are deployment-selected facts resolved after admission; no caller-provided path,
/// executable or address appears here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserPlan {
    pub connection: String,
}

/// Exactly one closed driver plan. HTTP, SIP, audio and browser fields cannot coexist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolPlan {
    HttpV1(HttpPlan),
    MediatedHttpV1(MediatedHttpPlan),
    SipV1(SipPlan),
    AudioV1(AudioPlan),
    CdpV1(BrowserPlan),
}

impl ProtocolPlan {
    pub const fn driver(&self) -> DriverId {
        match self {
            Self::HttpV1(_) => DriverId::HttpV1,
            Self::MediatedHttpV1(_) => DriverId::HttpV1,
            Self::SipV1(_) => DriverId::SipV1,
            Self::AudioV1(_) => DriverId::AudioV1,
            Self::CdpV1(_) => DriverId::CdpV1,
        }
    }
}

/// Complete zero-I/O unit handed to the one server dispatch composition point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZeroIoPlan {
    facts: OperationFacts,
    admission: AdmittedOperation,
    protocol: ProtocolPlan,
}

impl ZeroIoPlan {
    pub fn new(
        facts: OperationFacts,
        admission: AdmittedOperation,
        protocol: ProtocolPlan,
    ) -> Self {
        Self {
            facts,
            admission,
            protocol,
        }
    }

    pub fn facts(&self) -> &OperationFacts {
        &self.facts
    }

    pub fn admission(&self) -> &AdmittedOperation {
        &self.admission
    }

    pub fn protocol(&self) -> &ProtocolPlan {
        &self.protocol
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet as Set;
    use std::sync::Arc;
    use std::time::SystemTime;

    use connector_state::MemoryState;

    use super::*;
    use crate::{
        Grant, GrantAction, GrantEvaluator, GrantFacts, GrantIdempotency, GrantRequest, GrantRisk,
        GrantSelector, GrantSet, InitiationPolicy,
    };

    #[test]
    fn a_grant_decision_is_the_hosted_admission_path() {
        let connection =
            ConnectionAuthority::new("connection:grafana:ops", InitiationPolicy::platform_only())
                .expect("valid connection reference");
        // The only production route to a decision is the evaluator over a bound store; there
        // is no test builder to keep honest separately.
        let store = MemoryState::new();
        GrantSet {
            revision: 12,
            grants: vec![Grant {
                grant: "grant:observability-read".to_owned(),
                provider: "grafana".to_owned(),
                connection: "connection:grafana:ops".to_owned(),
                selector: Some(GrantSelector {
                    risk_ceiling: GrantRisk::Low,
                    effects: Set::from([crate::GrantEffect::Read]),
                    idempotency: Set::from([GrantIdempotency::Idempotent]),
                }),
                allow: Set::new(),
                deny: Set::new(),
                inbound_events: Set::new(),
            }],
        }
        .write(&store, "tenant:acme")
        .expect("seed");
        let now = SystemTime::now();
        let decision = GrantEvaluator::bound(Arc::new(store))
            .evaluate(
                &GrantRequest {
                    issuer: "https://identity.example".to_owned(),
                    tenant: "tenant:acme".to_owned(),
                    subject: "principal:svc-observer".to_owned(),
                    actor: None,
                    provider: "grafana".to_owned(),
                    connection: connection.clone(),
                    catalog_generation: "generation:1".to_owned(),
                    description_ref: "description:grafana:1".to_owned(),
                    input_digest: "sha256:abcd".to_owned(),
                    action: GrantAction::Invoke {
                        operation: "grafana/datasource.query".to_owned(),
                        facts: GrantFacts {
                            risk: GrantRisk::Low,
                            effects: Set::from([crate::GrantEffect::Read]),
                            idempotency: GrantIdempotency::Idempotent,
                        },
                    },
                },
                now,
            )
            .expect("the seeded grant admits");
        let admitted =
            AdmittedOperation::from_decision(decision, now).expect("a live decision admits");
        assert_eq!(admitted.provider(), "grafana");
        assert_eq!(admitted.operation(), "grafana/datasource.query");
        assert_eq!(admitted.organization(), "tenant:acme");
        assert_eq!(admitted.principal(), "principal:svc-observer");
        assert_eq!(admitted.grant(), "grant:observability-read");
        assert_eq!(admitted.connection_authority(), &connection);
    }

    #[test]
    fn the_local_owner_path_carries_the_same_evidence_shape() {
        let connection =
            ConnectionAuthority::new("connection:local", InitiationPolicy::platform_only())
                .expect("valid connection reference");
        let admitted = AdmittedOperation::for_local_owner(
            "sip",
            "sip/dial",
            "tenant:local",
            "principal:owner",
            "grant:local-owner",
            connection,
        );
        assert_eq!(admitted.provider(), "sip");
        assert_eq!(admitted.connection(), "connection:local");
    }
}

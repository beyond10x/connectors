use std::collections::BTreeSet;

use crate::{ConnectionAuthority, RouteAdapter};

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
/// the plan is built. The service layer constructs this from its admission result and checks the
/// catalog identities again while planning.
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
    /// Construct admission evidence from an already successful grant decision.
    pub fn from_grant_decision(
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

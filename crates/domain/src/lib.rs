#![forbid(unsafe_code)]

//! Protocol-neutral Connectors domain types. Planning is data and performs no I/O; Grant
//! evaluation and the approval gate are the deliberate exceptions, and their only I/O is the
//! S-041 state port.

mod approval;
pub mod audio;
mod connection;
mod discovery;
mod evaluator;
mod grant;
mod plan;
pub mod voice;

pub use approval::{
    ApprovalAuditKind, ApprovalError, ApprovalGate, ApprovalInvocation, ApprovalOutcome,
    ApprovalRecord, ApprovalRedemption, RecoveredPresentation, APPROVAL_AUDIT_STATE_KEY,
};
pub use connection::{
    ConnectionAuthority, ConnectionAuthorityError, ConnectionInitiator, ConnectionRoute,
    InitiationPolicy, RouteAdapter,
};
pub use discovery::{
    ConnectionCandidate, ConnectionCandidateSource, DiscoveryError, DiscoveryObservation,
};
pub use evaluator::conformance as grant_conformance;
pub use evaluator::{
    GrantAction, GrantDecision, GrantEvaluator, GrantRefusal, GrantRequest, GrantRuling,
};
pub use grant::{
    Grant, GrantEffect, GrantFacts, GrantIdempotency, GrantRisk, GrantSelector, GrantSet,
    GRANTS_CELL_BOUND,
};
pub use plan::{
    AdmittedOperation, AudioPlan, BrowserPlan, Capability, DriverId, HttpPlan, Implementation,
    Interaction, MediatedHttpPlan, OperationFacts, Placement, ProtocolPlan, SipPlan, SqlPlan,
    ZeroIoPlan,
};

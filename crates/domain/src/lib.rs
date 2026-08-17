#![forbid(unsafe_code)]

//! Protocol-neutral Connectors domain types. Planning is data and performs no I/O.

pub mod audio;
mod connection;
mod discovery;
mod plan;
pub mod voice;

pub use connection::{
    ConnectionAuthority, ConnectionAuthorityError, ConnectionInitiator, ConnectionRoute,
    InitiationPolicy, RouteAdapter,
};
pub use discovery::{
    ConnectionCandidate, ConnectionCandidateSource, DiscoveryError, DiscoveryObservation,
};
pub use plan::{
    AdmittedOperation, AudioPlan, BrowserPlan, Capability, DriverId, HttpPlan, Implementation,
    Interaction, MediatedHttpPlan, OperationFacts, Placement, ProtocolPlan, SipPlan, ZeroIoPlan,
};

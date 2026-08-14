#![forbid(unsafe_code)]

//! Protocol-neutral Connectors domain types. Planning is data and performs no I/O.

mod connection;
mod discovery;
mod plan;
pub mod voice;

pub use connection::{
    ConnectionAuthority, ConnectionAuthorityError, ConnectionInitiator, ConnectionRoute,
    InitiationPolicy, RouteAdapter,
};
pub use discovery::{ConnectionCandidate, DiscoveryError, DiscoveryObservation};
pub use plan::{
    AdmittedOperation, Capability, DriverId, HttpPlan, Implementation, Interaction,
    MediatedHttpPlan, OperationFacts, Placement, ProtocolPlan, SipPlan, ZeroIoPlan,
};

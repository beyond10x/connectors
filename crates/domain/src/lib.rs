#![forbid(unsafe_code)]

//! Protocol-neutral Connectors domain types. Planning is data and performs no I/O.

mod connection;
mod plan;
pub mod voice;

pub use connection::{
    ConnectionAuthority, ConnectionAuthorityError, ConnectionInitiator, InitiationPolicy,
};
pub use plan::{
    AdmittedOperation, Capability, DriverId, HttpPlan, Implementation, Interaction, OperationFacts,
    Placement, ProtocolPlan, SipPlan, ZeroIoPlan,
};

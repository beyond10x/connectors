#![forbid(unsafe_code)]

//! Protocol-neutral Connectors domain types. Planning is data and performs no I/O.

mod plan;
pub mod voice;

pub use plan::{
    AdmittedOperation, Capability, DriverId, HttpPlan, Implementation, Interaction, OperationFacts,
    Placement, ProtocolPlan, SipPlan, ZeroIoPlan,
};

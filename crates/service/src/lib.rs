#![forbid(unsafe_code)]

//! Pure Connectors use cases.

mod planning;

pub use planning::{plan_operation, PlanError, PlanningEnvironment};

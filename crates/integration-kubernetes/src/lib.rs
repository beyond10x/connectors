#![forbid(unsafe_code)]

//! Kubernetes Integration adapters.

mod databases;
pub mod endpoints;
mod hosted;
mod local;
mod local_services;
mod local_workloads;
mod workloads;

pub use hosted::{KubernetesBackendError, KubernetesStatusBackend};
pub use local::{
    local_contexts, KubernetesLocalBackend, KubernetesLocalError, LocalContextSummary,
};

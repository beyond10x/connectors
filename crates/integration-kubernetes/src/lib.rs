#![forbid(unsafe_code)]

//! Kubernetes Integration adapters.

mod hosted;
mod local;
mod local_services;
mod local_workloads;
mod workloads;

pub use hosted::{KubernetesBackendError, KubernetesStatusBackend};
pub use local::{KubernetesLocalBackend, KubernetesLocalError};

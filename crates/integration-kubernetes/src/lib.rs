#![forbid(unsafe_code)]

//! Kubernetes Integration adapters.

mod hosted;
mod local;

pub use hosted::{KubernetesBackendError, KubernetesStatusBackend};
pub use local::{KubernetesLocalBackend, KubernetesLocalError};

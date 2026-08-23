#![forbid(unsafe_code)]

//! Reusable daemon composition below the command-line surface.

mod claims;
mod composition;
mod registry;

pub use claims::{ClaimError, EventReplyClaims};
pub use composition::{
    acquire_argocd_token, argocd_acquisition, default_config_path, default_state_root,
    validate_state_root, HostedRuntime, PersonalCredentialStores, PersonalRuntime, RuntimeError,
};
pub use connectors_config::{ConfigError, PersonalConfig};
/// Re-exported so the CLI can describe an acquisition without depending on the adapter directly.
pub use integration_catalog::argocd;
pub use registry::BackendRegistry;

#![forbid(unsafe_code)]

//! Reusable daemon composition below the command-line surface.

mod composition;
mod registry;

pub use composition::{
    default_config_path, default_state_root, validate_state_root, HostedRuntime,
    PersonalCredentialStores, PersonalRuntime, RuntimeError,
};
pub use connectors_config::{ConfigError, PersonalConfig};
pub use registry::BackendRegistry;

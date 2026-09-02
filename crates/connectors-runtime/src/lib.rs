#![forbid(unsafe_code)]

//! Reusable daemon composition below the command-line surface.

mod claims;
mod composition;
mod registry;
mod service_bundle;

pub use claims::{ClaimError, EventReplyClaims};
pub use composition::{
    acquire_argocd_token, argocd_acquisition, default_config_path, default_state_root,
    validate_state_root, HostedRuntime, PersonalCredentialStores, PersonalRuntime, RuntimeError,
};
pub use connectors_config::{ConfigError, PersonalConfig};
/// Re-exported so the CLI can describe an acquisition without depending on the adapter directly.
pub use integration_catalog::argocd;
/// Reviewed outbound MCP service profiles and the factory that binds them into a service bundle.
pub mod mcp {
    pub use integration_mcp::{
        McpBearerBinding, McpIntegrationError, McpRuntimeBinding, McpServiceFactory,
        McpServiceProfile, ReviewedOperation, ReviewedProvider, PROFILE_CONTRACT,
    };
}
pub use registry::BackendRegistry;
pub use service_bundle::{
    DeployedService, ServiceBundle, ServiceBundleBuilder, ServiceBundleError,
};

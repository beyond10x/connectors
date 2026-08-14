#![forbid(unsafe_code)]

//! Personal-local daemon composition and the first catalog-backed operation backend.

mod config;
mod monitoring_backend;
mod runtime;
mod sip_backend;
mod slack_backend;

pub use config::{
    AuthorityConfig, ConfigError, GrafanaIntegrationConfig, InitiationConfig, PersonalConfig,
    PersonalVoiceConfig, SlackIntegrationConfig,
};
pub use monitoring_backend::{MonitoringBackend, MonitoringError};
pub use runtime::{load_authority_issuer, RuntimeLauncher};
pub use sip_backend::{
    LaunchError, LaunchedSession, RefusingBackend, SessionLauncher, SipOperationBackend,
};
pub use slack_backend::{SlackBackend, SlackError};

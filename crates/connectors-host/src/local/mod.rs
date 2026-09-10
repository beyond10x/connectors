//! Linux local configuration and metadata infrastructure.
//!
//! These owners do not grant provider access. Custody qualification and the
//! connection/supervisor coordinators must be bound before credential acquisition.
pub mod config;
pub mod filesystem;
pub mod keyring;
pub mod metadata;
pub mod owner;
pub mod protected;
pub mod registry;
pub mod runtime;

/// Closed, credential-free local failures. Never include OS, parser or DB text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidConfiguration,
    ConfigurationExists,
    MetadataUnavailable,
    OutcomeUnknown,
}

pub type Result<T> = std::result::Result<T, Failure>;

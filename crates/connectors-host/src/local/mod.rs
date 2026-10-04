//! Linux local configuration and metadata infrastructure.
//!
//! These owners do not grant provider access. Custody qualification and the
//! connection/supervisor coordinators must be bound before credential acquisition.
#[cfg(test)]
mod adversary_w2_tests;
#[cfg(test)]
mod adversary_w3_tests;
pub mod approval_keys;
pub mod approval_policy;
pub mod approvals;
pub mod audit;
pub mod clock;
pub mod config;
pub mod filesystem;
pub mod keyring;
pub mod metadata;
pub mod mutations;
pub mod oauth;
#[cfg(test)]
mod oauth_adversary_pass2_tests;
#[cfg(test)]
mod oauth_adversary_tests;
pub mod operation_curation;
pub mod owner;
pub mod protected;
pub mod registry;
pub mod runtime;
#[cfg(test)]
mod security_replay_tests;
mod unix;

/// Closed, credential-free local failures. Never include OS, parser or DB text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidConfiguration,
    ConfigurationExists,
    MetadataUnavailable,
    /// An ER write was definitely not committed because its recorded revision moved.
    ConcurrentRevision,
    OutcomeUnknown,
}

pub type Result<T> = std::result::Result<T, Failure>;

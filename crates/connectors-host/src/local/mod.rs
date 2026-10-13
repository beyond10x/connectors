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
pub mod owner;
pub mod protected;
pub mod registry;
pub mod runtime;
#[cfg(test)]
mod security_replay_tests;
pub mod service;
#[cfg(test)]
pub(crate) mod test_executable;
#[cfg(test)]
mod test_streams;

/// Closed, credential-free local failures. Never include OS, parser or DB text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    InvalidConfiguration,
    ConfigurationExists,
    MetadataUnavailable,
    /// An ER write was definitely not committed because its recorded revision moved.
    ConcurrentRevision,
    OutcomeUnknown,
    /// The store was written by a newer binary whose record model differs from
    /// this one's: its writer marker's release, or `None` when a record carries a
    /// field this binary's model does not declare and no newer marker names the
    /// writer. Public code `metadata_schema_newer`; nothing was read or written.
    MetadataSchemaNewer(Option<Release>),
}

/// A release version, `major.minor.patch`, ordered numerically.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Release {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Release {
    /// This binary's release version.
    ///
    /// # Panics
    /// Never: the workspace version is a `major.minor.patch` release, which a
    /// unit test pins.
    #[must_use]
    pub fn current() -> Self {
        Self::parse(runtime::state::BUILD_VERSION).expect("the build version is a release version")
    }

    /// `major.minor.patch` in decimal, nothing else.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.split('.');
        let mut next = || {
            parts
                .next()
                .filter(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
                .and_then(|part| part.parse().ok())
        };
        let release = Self {
            major: next()?,
            minor: next()?,
            patch: next()?,
        };
        parts.next().is_none().then_some(release)
    }
}

impl std::fmt::Display for Release {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub type Result<T> = std::result::Result<T, Failure>;

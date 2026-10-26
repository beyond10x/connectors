//! Linux local configuration and metadata infrastructure.
//!
//! These owners do not grant provider access. Custody qualification and the
//! connection/supervisor coordinators must be bound before credential acquisition.
pub mod access_token;
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
pub mod core_dumps;
pub mod definitions;
pub mod filesystem;
pub mod keyring;
pub mod metadata;
pub mod mutations;
pub mod oauth;
pub mod oauth2_grant;
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
pub mod service;
#[cfg(test)]
pub(crate) mod test_executable;
#[cfg(test)]
mod test_streams;
/// The owner socket's nonblocking connect; Linux only, where an `AF_UNIX`
/// connect to a full accept queue would otherwise wait unbounded.
#[cfg(target_os = "linux")]
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
    /// The store was written by a newer binary whose record model differs from
    /// this one's: its writer marker's release, or `None` when a record carries a
    /// field this binary's model does not declare and no newer marker names the
    /// writer. Public code `metadata_schema_newer`; nothing was read or written.
    MetadataSchemaNewer(Option<Release>),
}

/// The longest pre-release a [`Release`] holds, in bytes. A longer one is a
/// version this binary does not order.
pub const PRE_RELEASE_BOUND: usize = 40;

/// A release version, SemVer 2.0.0 `major.minor.patch[-pre-release][+build]`,
/// ordered by SemVer precedence: numerically by its core, a pre-release below
/// its release, pre-releases identifier by identifier. Build metadata takes no
/// part in precedence and is not kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Release {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// The pre-release's bytes, zero after `pre_len`, so that equality is
    /// equal precedence.
    pre: [u8; PRE_RELEASE_BOUND],
    pre_len: u8,
}

impl Release {
    /// The release `major.minor.patch`, without a pre-release.
    #[must_use]
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
            pre: [0; PRE_RELEASE_BOUND],
            pre_len: 0,
        }
    }

    /// This binary's release version.
    ///
    /// # Panics
    /// Never: the workspace version is a SemVer release, which a unit test pins.
    #[must_use]
    pub fn current() -> Self {
        Self::parse(runtime::state::BUILD_VERSION).expect("the build version is a release version")
    }

    /// A SemVer 2.0.0 version: `major.minor.patch` in decimal, an optional
    /// pre-release of at most [`PRE_RELEASE_BOUND`] bytes whose identifiers are
    /// non-empty ASCII alphanumerics and hyphens (a numeric one without a
    /// leading zero), and optional build metadata of the same characters.
    /// Anything else is `None`: a version this binary cannot order.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let identifier = |part: &str| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        };
        let (text, build) = match text.split_once('+') {
            Some((text, build)) => (text, Some(build)),
            None => (text, None),
        };
        if build.is_some_and(|build| !build.split('.').all(identifier)) {
            return None;
        }
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let mut parts = core.split('.');
        let mut next = || {
            parts
                .next()
                .filter(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
                .and_then(|part| part.parse().ok())
        };
        let mut release = Self::new(next()?, next()?, next()?);
        if parts.next().is_some() {
            return None;
        }
        if let Some(pre) = pre {
            let numeric_with_leading_zero = |part: &str| {
                part.len() > 1 && part.starts_with('0') && part.bytes().all(|b| b.is_ascii_digit())
            };
            if pre.len() > PRE_RELEASE_BOUND
                || !pre
                    .split('.')
                    .all(|part| identifier(part) && !numeric_with_leading_zero(part))
            {
                return None;
            }
            release.pre[..pre.len()].copy_from_slice(pre.as_bytes());
            release.pre_len = u8::try_from(pre.len()).ok()?;
        }
        Some(release)
    }

    /// The pre-release, if this is one.
    #[must_use]
    pub fn pre_release(&self) -> Option<&str> {
        let pre = &self.pre[..usize::from(self.pre_len)];
        (!pre.is_empty()).then(|| std::str::from_utf8(pre).unwrap_or_default())
    }
}

impl Ord for Release {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        // SemVer §11.4.1: numeric identifiers carry no leading zero, so the
        // longer is the larger, whatever its size.
        let identifier = |left: &str, right: &str| {
            let numeric = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
            match (numeric(left), numeric(right)) {
                (true, true) => left.len().cmp(&right.len()).then_with(|| left.cmp(right)),
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (false, false) => left.cmp(right),
            }
        };
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.pre_release(), other.pre_release()) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(left), Some(right)) => {
                    let (mut left, mut right) = (left.split('.'), right.split('.'));
                    loop {
                        match (left.next(), right.next()) {
                            (None, None) => break Ordering::Equal,
                            (None, Some(_)) => break Ordering::Less,
                            (Some(_), None) => break Ordering::Greater,
                            (Some(left), Some(right)) => match identifier(left, right) {
                                Ordering::Equal => {}
                                order => break order,
                            },
                        }
                    }
                }
            })
    }
}

impl PartialOrd for Release {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::fmt::Display for Release {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)?;
        match self.pre_release() {
            Some(pre) => write!(formatter, "-{pre}"),
            None => Ok(()),
        }
    }
}

pub type Result<T> = std::result::Result<T, Failure>;

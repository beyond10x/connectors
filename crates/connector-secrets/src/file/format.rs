//! File-format identities shared by the store and its compiled compatibility report.

/// The first line of every file this store writes.
pub(super) const HEADER: &str = "# codewandler-connector-secrets file store, v1";

/// The prefix a version line begins with, and the version this store speaks.
///
/// Split out from [`HEADER`] so the version is **checked** rather than merely written. A header a
/// reader skips as a comment is decoration: a future `v2` — one that encrypted the values, say, or
/// changed the separator — would be loaded as `v1`, and the failure would be a wrong answer rather
/// than a refusal. `v2` bytes read as `v1` is exactly the case a credential store must not guess at.
pub(super) const VERSION_PREFIX: &str = "# codewandler-connector-secrets file store, v";
pub(super) const VERSION: &str = "1";

/// The file-format versions this store can read and write, without opening a store.
///
/// The first is the initial format, retained by ordinary writes to a legacy store. A
/// prepared-transaction write switches that store to the second format; later writes retain it.
#[must_use]
pub const fn supported_format_versions() -> [&'static str; 2] {
    [VERSION, super::prepared::VERSION]
}

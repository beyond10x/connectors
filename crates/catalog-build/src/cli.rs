//! What a caller asked for, as data.
//!
//! **This module knows nothing about argv.** Parsing belongs to the binary — `crates/catalog-cli`,
//! with clap's derive API — and what crosses the boundary is this struct. The predecessor put a
//! hand-rolled parser here, and the cost was that the surface's rules ("`--select` is `scaffold`
//! only") lived in `match` guards and in prose beside them rather than in a type a `--help` could
//! be generated from.
//!
//! Keeping the *shape* here rather than in the binary is deliberate: [`crate::run`] takes an
//! [`Invocation`], so an integration test drives the library through exactly the value `main`
//! builds, without going through a command line to do it.

use std::path::PathBuf;

/// What to do, and to what.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Invocation {
    /// The verb.
    pub command: Command,
    /// The repository root; `None` means the current directory.
    pub root: Option<PathBuf>,
    /// Restrict the run to one provider.
    pub provider: Option<String>,
    /// Restrict the run to one whole service of each provider it covers — a service name or a
    /// rendered gid. Normally paired with `provider`, since a service is a level *within* a
    /// provider; a provider in the run that has no such service is a loud error naming what it has.
    pub service: Option<String>,
    /// `scaffold` only: which operations to select, in the order they were written.
    ///
    /// Each entry is one `[[patch.select]]` statement, spelled
    /// `<service>:<path_prefix>:<METHOD,METHOD>`; see [`crate::scaffold`] for why selection is an
    /// argument rather than something the helper infers.
    pub selects: Vec<String>,
    /// `scaffold` only: report the document against the connector as it stands, rather than
    /// emitting TOML.
    pub diff: bool,
}

/// A verb of the `catalog` surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Command {
    /// Compile committed inputs into committed artifacts.
    #[default]
    Build,
    /// Show what a rebuild would change, without writing.
    Diff,
    /// Recompute every hash in `connectors.lock` and refuse on drift.
    Check,
    /// Write the provider TOML that references a vendored document, to stdout.
    Scaffold,
}

//! `catalog`: compile the reviewed connector declarations into the catalog artifacts.
//!
//! The binary is a thin shell over this library so that the whole command surface is reachable from
//! an integration test — what `tests/` exercises is exactly what `main` runs. Argument parsing is
//! the binary's (clap, derive API); what crosses the boundary is [`cli::Invocation`], so a test
//! drives this library through exactly the value `main` builds.
//!
//! # The shape of a build
//!
//! ```text
//! providers/<name>.toml ──┐
//!                         ├─► seam::load ─► Connector ─► document::render ─► catalog/<name>.catalog.json
//! specs/<name>/<ver>.json ┘                                                  crates/catalog-reader/catalog.pack
//!                                                                            connectors.lock
//!                                                                            web/public/catalog.json
//! ```
//!
//! One lowering, two projections. [`document`] renders the canonical per-provider document — the
//! reviewed artifact — [`pack`] compiles every document into the one distributable file, and
//! [`site`] projects the same IR into the public explorer's JSON. Nothing renders code.
//!
//! [`discovery`] finds the inputs, [`seam`] loads them, [`pipeline`] compares the result against
//! the committed tree, and only then does [`artifact`] write. `diff` stops one step earlier and
//! renders instead ([`diff`]).
//!
//! # What this crate does not do, and why
//!
//! It is the predecessor's `connector-cli` **minus every emitter**. Gone with the Flux engine
//! (design 02 §2): the `.flux` module and `.connector.toml` manifest per service, the per-operation
//! `.flux` renderings, the generated Rust catalogue tables and their index, the README snippet's
//! SVG/PNG rasterisation, and the native-plugin migration check against a Flux checkout. What is
//! left is the lowering, the artifacts a consumer reads, and the fences that hold them honest.
//!
//! # Invariants this crate holds
//!
//! - **Hermetic and offline.** `build` and `diff` read committed bytes and never contact a vendor;
//!   [`net`] is the single door, and `tests/main/no_network.rs` proves a build never reaches it.
//! - **Deterministic.** Equal inputs produce byte-identical artifacts, so a rebuild over unchanged
//!   inputs writes nothing at all.
//! - **All-or-nothing.** Every provider is compiled before any file is written.
//! - **Explicit.** Generation is a command a human runs and reviews as a diff — never a `build.rs`.
//! - **Engine-free.** No `codewandler-flux-*` crate appears anywhere in the resolved graph, which
//!   `tests/main/engine_free.rs` asserts over the whole workspace rather than over this sentence.

// The canonical-document JSON Schema (`document::schema`) is one `json!` literal, and its nesting
// is deeper than the macro's default expansion budget. Raising the limit is the sanctioned answer
// (the error message's own suggestion); splitting the schema across several literals would trade a
// compiler constant for a document a reader can no longer see whole.
#![recursion_limit = "256"]

pub mod artifact;
pub mod cli;
pub mod diff;
pub mod discovery;
pub mod document;
mod inbound;
pub mod net;
pub mod pack;
pub mod pipeline;
pub mod scaffold;
pub mod seam;
pub mod site;
pub mod status;
pub mod surface;
pub mod workspace;

use std::io::Write;

use anyhow::{bail, Context, Result};

use crate::cli::{Command, Invocation};
use crate::workspace::Workspace;

/// Execute a parsed invocation, writing user-facing output to `out`.
pub fn run(invocation: &Invocation, out: &mut impl Write) -> Result<()> {
    match invocation.command {
        Command::Build => build(invocation, out),
        Command::Diff => show_diff(invocation, out),
        Command::Scaffold => scaffold(invocation, out),
        Command::Check => not_yet_implemented(),
    }
}

/// Compile every provider and write what changed.
///
/// **A build refuses outright when a committed file under an artifact root is claimed by no plan**,
/// before writing anything at all. Two reasons it lands here rather than after the writes: a
/// refusal that had already written half the run would leave the tree carrying both a partial build
/// and the orphan, and the all-or-nothing property this module already holds is the one a reader
/// will assume.
fn build(invocation: &Invocation, out: &mut impl Write) -> Result<()> {
    let workspace = workspace_for(invocation)?;
    let plan = pipeline::plan_selected(
        &workspace,
        invocation.provider.as_deref(),
        invocation.service.as_deref(),
    )?;

    report_diagnostics(&plan, out)?;
    refuse_orphans(&workspace, &plan)?;

    if plan.is_up_to_date() {
        writeln!(
            out,
            "{} up to date; nothing written",
            summarize(plan.providers.len(), plan.artifacts.len())
        )?;
        return Ok(());
    }

    let written = pipeline::apply(&plan)?;
    for path in &written {
        writeln!(out, "wrote {}", workspace.display_path(path).display())?;
    }
    writeln!(
        out,
        "{}; {} written",
        summarize(plan.providers.len(), plan.artifacts.len()),
        written.len()
    )?;
    Ok(())
}

/// Write the provider TOML that references a vendored document.
///
/// **To `out` and nowhere else.** This is the one verb whose whole safety argument is that it
/// produces text rather than a file: the author diffs and pastes, so a bad run costs nothing and
/// the reviewed artifact stays a human's. Nothing here opens a file for writing, and the emitted
/// TOML is not an artifact — it is not hashed, not in `connectors.lock`, and `diff` says nothing
/// about it.
fn scaffold(invocation: &Invocation, out: &mut impl Write) -> Result<()> {
    let workspace = workspace_for(invocation)?;
    let Some(provider) = invocation.provider.as_deref() else {
        bail!("`catalog scaffold` needs a provider to scaffold: `catalog scaffold <PROVIDER>`");
    };

    let rendered = if invocation.diff {
        scaffold::render_diff(&workspace, provider)?
    } else {
        scaffold::render(&workspace, provider, &invocation.selects)?
    };
    write!(out, "{rendered}")?;
    Ok(())
}

/// Show what a build would change. Writes nothing — see [`pipeline::plan`].
fn show_diff(invocation: &Invocation, out: &mut impl Write) -> Result<()> {
    let workspace = workspace_for(invocation)?;
    let plan = pipeline::plan_selected(
        &workspace,
        invocation.provider.as_deref(),
        invocation.service.as_deref(),
    )?;
    report_diagnostics(&plan, out)?;
    write!(out, "{}", diff::render(&workspace, &plan))?;
    // After the render, not before: `diff` is a preview, and the preview is worth having even in the
    // run that fails. The non-zero exit is the point — an orphan is drift in exactly the sense
    // `connectors.lock` exists to catch, so CI must fail on it rather than print a warning nobody
    // reads.
    refuse_orphans(&workspace, &plan)
}

/// Stop, naming every committed file under an artifact root that the plan does not claim.
///
/// Silent on a scoped run, because [`pipeline::plan_selected`] leaves the list empty there: a
/// `--provider` run compiled a subset and every other provider's artifacts would read as unclaimed.
fn refuse_orphans(workspace: &Workspace, plan: &pipeline::Plan) -> Result<()> {
    if plan.orphans.is_empty() {
        return Ok(());
    }
    bail!("{}", diff::orphan_refusal(workspace, &plan.orphans))
}

/// Say what the vendored spec documents got wrong, before saying what the build did.
///
/// **Reported, never fatal.** A real vendor OpenAPI document is incomplete or wrong somewhere, and
/// ingest skips the endpoint rather than failing the connector; this is the line that keeps that
/// from being silent. It writes nothing at all when there is nothing to say, which is every
/// hand-authored connector.
fn report_diagnostics(plan: &pipeline::Plan, out: &mut impl Write) -> Result<()> {
    for diagnostic in &plan.diagnostics {
        writeln!(out, "spec: {diagnostic}")?;
    }
    Ok(())
}

fn summarize(providers: usize, artifacts: usize) -> String {
    let provider_noun = if providers == 1 {
        "provider"
    } else {
        "providers"
    };
    let artifact_noun = if artifacts == 1 {
        "artifact"
    } else {
        "artifacts"
    };
    format!("{providers} {provider_noun}, {artifacts} {artifact_noun}")
}

fn workspace_for(invocation: &Invocation) -> Result<Workspace> {
    let root = match &invocation.root {
        Some(root) => root.clone(),
        None => std::env::current_dir().context("cannot determine the current directory")?,
    };
    Ok(Workspace::new(root))
}

/// `check` is declared and unbuilt, and it fails loudly rather than exiting zero.
///
/// Deliberately an error rather than a no-op: a command that exits zero without doing anything is
/// how a CI pipeline comes to believe it is checking something it is not. The verifier
/// `connectors.lock` never had is design 02 §2's day-one change and lands with its own story;
/// until it does, this refusal is the honest answer.
fn not_yet_implemented() -> Result<()> {
    bail!(
        "`catalog check` is not yet implemented. It is the day-one lockfile verifier of design 02 \
         §2: recompute every hash in `connectors.lock` and exit non-zero on drift. Until it lands, \
         `catalog diff` reports drift in the artifacts (but not in the lockfile's own record of \
         them)."
    )
}

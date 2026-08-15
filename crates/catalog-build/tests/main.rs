//! **One integration-test binary for this crate.**
//!
//! Every `.rs` file directly under `tests/` is its own crate, which Cargo links into its own
//! executable carrying the entire dependency graph — the predecessor measured 179 files, 792
//! executables and 30 GB in `target/debug/deps`, and a disk exhaustion that failed a release cut.
//! The files under `tests/main/` are therefore modules of this single test target. Run one of them
//! with `cargo test -p catalog-build --test main <module>::`.
//!
//! The `#[path]` attribute on every declaration is load-bearing: this file is a crate root, and a
//! crate root resolves a bare `mod x;` in its **own** directory (`tests/`) — the same rule that
//! makes `tests/common/mod.rs` reachable — never in `tests/main/`.
//!
//! # Two kinds of file, and only two
//!
//! - **[`catalog_invariants`]** — every catalogue invariant, iterated over the whole catalogue.
//!   There is no per-provider test file here and there will not be one: a rule about connectors is
//!   stated once and parameterised, so the next connector is covered the moment it exists.
//! - **The workspace fences** — [`architecture_fence`], [`dependency_fence`], [`engine_free`],
//!   [`msrv_fence`], [`json_governance`], [`no_network`]. Each is about the *workspace*, not about
//!   the catalogue, and each is its own argument.

mod common;

#[path = "main/architecture_fence.rs"]
mod architecture_fence;
#[path = "main/catalog_invariants.rs"]
mod catalog_invariants;
#[path = "main/dependency_fence.rs"]
mod dependency_fence;
#[path = "main/engine_free.rs"]
mod engine_free;
#[path = "main/json_governance.rs"]
mod json_governance;
#[path = "main/msrv_fence.rs"]
mod msrv_fence;
#[path = "main/no_network.rs"]
mod no_network;

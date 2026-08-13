//! **One integration-test binary for this crate.**
//!
//! Every `.rs` file directly under `tests/` is its own crate, which Cargo links into its own
//! executable carrying the entire dependency graph. The files under `tests/main/` are therefore
//! modules of this single test target. Run one of them with
//! `cargo test -p catalog --test main <module>::`.
//!
//! The `#[path]` attribute on every declaration is load-bearing: this file is a crate root, and a
//! crate root resolves a bare `mod x;` in its **own** directory (`tests/`), never in `tests/main/`.

#[path = "main/consumer_api.rs"]
mod consumer_api;
#[path = "main/pack_table.rs"]
mod pack_table;

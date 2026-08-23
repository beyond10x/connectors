#![forbid(unsafe_code)]

//! The operator-facing surface, below the command line and above the protocols.
//!
//! # Why this is not in the CLI
//!
//! `crates/connectors-cli` is a **thin frontend** — clap definitions, dispatch, and the one
//! process-level decision only a binary can make. That is not a style preference: it is asserted by
//! `catalog-build/tests/main/architecture_fence.rs`, which caps the CLI's production line count and
//! pins its exact dependency list, because a prose boundary that can silently regain a forbidden
//! dependency is not a boundary. Writing a configuration, diagnosing an installation and reading
//! the catalogue are *behaviour*, and the fence's own instruction for behaviour is to move it
//! behind its owned package. This is that package.
//!
//! Being a library rather than a `main.rs` also makes each piece testable without a process, which
//! is why `init` can prove that what it writes is what the daemon reads back.
//!
//! # What lives here
//!
//! - [`init`] — write a configuration nobody should have had to author by hand.
//! - [`doctor`] — report what is configured, what is running, and what cannot work.
//! - [`enrol`] — connect any catalogued provider: prompt for what the catalogue cannot answer.
//! - [`providers`] — what the embedded catalogue can reach, as a measured fact.
//! - [`auth`] — which providers are connected, answered without reading a credential.
//! - [`connect`] — the guided provider flows, returning their outcome as data so `-o json` works.
//! - [`envelope`] — reducing a transport envelope to its payload, or to a refusal that exits non-zero.
//! - [`input`] — where one operation's caller input comes from.
//! - [`output`] — how any of those reach a terminal, a pipe, or a parser.
//!
//! Composition stays in `connectors-runtime` and wire framing in `connectors-client`; this package
//! composes no adapter and frames no request. [`connect`] does drive `connectors-client`, because a
//! guided flow is a conversation with the local Connector and the conversation is the behaviour
//! being packaged.

pub mod auth;
pub mod connect;
pub mod doctor;
pub mod enrol;
pub mod envelope;
pub mod init;
pub mod input;
pub mod output;
pub mod providers;

pub use output::{emit, emit_error, payload, Format, OutputError};

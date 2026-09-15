// generated from connectors v2
// model digest b46e08fdaea05ec1169f3d75212c80578b59fd21a94fe1d523f21714bddb663c
// contract digest d01ebb2ef8f96762b184433c1df72babf3565039aef3600d49dc1cd99b931c9b
// do not edit: regenerate with `cargo xtask synth --target clap`


//! What is owed: one method per command the tree places.
//!
//! A method receives `clap::ArgMatches` rather than the command's declared input type. The
//! Rust target already emits every input as a type, and a fourth rendering of the type layer
//! would be a fourth thing to keep in step — so this target emits the grammar and leaves the
//! types where they are. `TARGET.md` states that as a weakening rather than leaving it to be
//! discovered.

/// What a command does, once somebody decides.
pub trait Handler {
}

/// A handler that decides nothing, and says which obligation is owed.
///
/// The honest empty state: the emitted binary parses, completes and refuses. A refusal that
/// names the command is the one a reader learns the plan from.
pub struct Unimplemented;

impl Handler for Unimplemented {
}

// generated from connectors v1
// model digest 92d00f77341a93351ebb20ce24412c86947ef6a36dfe66fa4cebae0cab13e13a
// contract digest a721680ab8e341cf3170805a2fa8ab211d3965b1d68dc7ef001bb77709e339ae
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

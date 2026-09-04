// generated from connectors v1
// model digest 5849d9d17106bb6c8f9f7331f9c1fcf8d8addd0506851273b950db073eacb81b
// contract digest ec905bc6d63ad79ca30a539c0a27a4d8c83bd41b4d0909594faa9ae6f34ab91e
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

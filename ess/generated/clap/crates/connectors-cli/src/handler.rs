// generated from connectors v1
// model digest ffd4f3b4f5db4b1e759db7bf19723cfe0ae0347d8bcb7d4fdc456841c33359f7
// contract digest dcd14c9765ebca6a92f7ca68d9b70b74f7381b6a373e6a037444bc8944118799
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

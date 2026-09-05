// generated from connectors v1
// model digest 3dd38aa4aa8dfa6484eef894f4ef64554a34471416cb728ac32b8d147f5d15fa
// contract digest ff324d008163ada73d593ae301c90ed7c76016fae2fd068827f423f4603d8b6c
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

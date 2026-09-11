// generated from gitlab_writes v1
// model digest 60e62b0197192fde0bab0673274f72e3236cac4e4c6b290f72b847ab42c0254f
// contract digest bf080db26d82ecf93e6e97307361cd6a49bbf2ef3cb125dd19f3a4605b5a35c2
// do not edit: regenerate with `ess synthesize`

//! Semantic types synthesised from the `gitlab_writes` specification, v1.
//!
//! Typed local write values. Consumer authority and native effect interpretation remain implementation obligations.
//!
//! Generated, not written: the specification is the source of truth, and the door to changing
//! anything here is `ess synthesize`. What is deliberately absent — behaviour, queries,
//! escalations — is listed with reasons in the `PLAN.md` beside this workspace, and every entry
//! there is owed through a typed seam in an `obligations` module here.

// `deny`, not the source workspace's lint set: this crate must hold on its own, and an undocumented
// public item here is an emitter defect worth failing the gate over.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod obligation;
pub mod primitives;
pub mod requests;

// generated from connectors v1
// model digest 0f1b92b6b86a69778081be2aca23de79337294bbb219d2bd7f19aad1f6523d26
// contract digest b9b32b755ebf5ba2e2d690e05122218781ab050a55aa96d08f5ab2d6164aff5e
// do not edit: regenerate with `ess synthesize`

//! Semantic types synthesised from the `connectors` specification, v1.
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
pub mod sessions;

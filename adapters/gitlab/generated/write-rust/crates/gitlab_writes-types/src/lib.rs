// generated from gitlab_writes v1
// model digest d71a3b2285e5b421c6f596c260c21fda824356846d5876221beafc5cf7d18f9a
// contract digest 0d52075ceb7e8f55a5f1b8672be27768f0e663364671db383cfdd5fe5f27c313
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

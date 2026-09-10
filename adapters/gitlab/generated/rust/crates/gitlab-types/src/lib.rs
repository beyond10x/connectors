// generated from gitlab v1
// model digest b7b2cb3c4d5e331acc466cbffc8cb5f0365f08bc1fa22c9b39b3dd9e02fa5202
// contract digest e998b4b4fd35be0dc69280a8a0eec358930cb7071586963ad07fe9edfc0b828c
// do not edit: regenerate with `ess synthesize`

//! Semantic types synthesised from the `gitlab` specification, v1.
//!
//! Typed adapter request inputs. Connectors owns operation/result and provider-binding semantics.
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

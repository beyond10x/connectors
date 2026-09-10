// generated from gitlab v1
// model digest ede8787ae4303e8248e5bd6de3675b12866d725713b94852f37db003d4982392
// contract digest 9020464eeb141c4af3f7cd1c704975f1a25aa17da6a44f4b61ff3a5c25400334
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

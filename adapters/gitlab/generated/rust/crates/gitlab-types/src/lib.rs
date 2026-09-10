// generated from gitlab v1
// model digest b437c9021e38b1a4dca0876effaedcbf8c5f6ce1fcb3f7212e936fd6f9356d07
// contract digest 004c3aa38f241afa38c5b5988b23bf2cc5194cd63287264f15043685f8a77bf7
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

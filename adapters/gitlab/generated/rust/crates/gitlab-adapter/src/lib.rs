// generated from gitlab v1
// model digest 5464a959b36fe5673bd570eecf3f661540dd261b60c0fde2fd37f4a5bf2d97f3
// contract digest c3d3a9b408c90c2a8b03daf08984379733603f2c84f483640772f01ea8aea190
// do not edit: regenerate with `ess synthesize`

//! gitlab-adapter — the `gitlab-adapter` component of `gitlab` v1.
//!
//! The component's outer surface exactly as the specification declares it: accepted commands as
//! handlers, declared views as queries, published events as a typed outbox. The behaviour behind
//! every handler is an implementation obligation — see the `PLAN.md` beside this workspace — and
//! until one is satisfied, its stub answers with a typed refusal naming what is owed.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event this component declares it publishes, on its way to the system's transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishedEvent {}

/// gitlab-adapter — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct GitlabAdapter<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> GitlabAdapter<B> {
    /// A new port over the given obligation implementations.
    pub fn new(behaviors: B) -> Self {
        Self {
            behaviors,
            outbox: Vec::new(),
        }
    }

    /// Hands over everything published since the last drain, in publication order.
    ///
    /// The system's transport calls this; anything else reading it is taking events the transport
    /// will then never deliver.
    pub fn drain_outbox(&mut self) -> Vec<PublishedEvent> {
        core::mem::take(&mut self.outbox)
    }
}

// generated from gitlab_writes v1
// model digest 60e62b0197192fde0bab0673274f72e3236cac4e4c6b290f72b847ab42c0254f
// contract digest bf080db26d82ecf93e6e97307361cd6a49bbf2ef3cb125dd19f3a4605b5a35c2
// do not edit: regenerate with `ess synthesize`

//! gitlab-writes-adapter — the `gitlab-writes-adapter` component of `gitlab_writes` v1.
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

/// gitlab-writes-adapter — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct GitlabWritesAdapter<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> GitlabWritesAdapter<B> {
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

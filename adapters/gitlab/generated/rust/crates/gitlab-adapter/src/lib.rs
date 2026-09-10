// generated from gitlab v1
// model digest b7b2cb3c4d5e331acc466cbffc8cb5f0365f08bc1fa22c9b39b3dd9e02fa5202
// contract digest e998b4b4fd35be0dc69280a8a0eec358930cb7071586963ad07fe9edfc0b828c
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

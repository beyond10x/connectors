// generated from gitlab_writes v1
// model digest d71a3b2285e5b421c6f596c260c21fda824356846d5876221beafc5cf7d18f9a
// contract digest 0d52075ceb7e8f55a5f1b8672be27768f0e663364671db383cfdd5fe5f27c313
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

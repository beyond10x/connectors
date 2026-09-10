// generated from gitlab v1
// model digest ede8787ae4303e8248e5bd6de3675b12866d725713b94852f37db003d4982392
// contract digest 9020464eeb141c4af3f7cd1c704975f1a25aa17da6a44f4b61ff3a5c25400334
// do not edit: regenerate with `ess synthesize`

//! The `gitlab` system, v1: its components assembled, its bindings wired, and its one transport.
//!
//! The transport is derived from the specification, not chosen: `at_least_once` is the only
//! delivery guarantee the model declares, so published events land on an append-only log and a
//! pump delivers each to every binding that reacts to it. The log is the system's observable
//! record, and so is the record of what each binding invoked. What no specification determines
//! — how an escalation event is filled, behaviour behind the ports — stays an obligation; see
//! the `PLAN.md` beside this workspace.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event on the system's log: everything any component publishes, and everything a binding
/// escalates into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemEvent {}

impl From<gitlab_adapter::PublishedEvent> for SystemEvent {
    fn from(event: gitlab_adapter::PublishedEvent) -> Self {
        match event {}
    }
}

/// The `gitlab` system: every component behind its port, and the transport between them.
///
/// The component fields are public because commands enter the system through a component's own
/// port; the log and its delivery cursor are not, because publishing happens by pumping, not by
/// writing history directly.
pub struct System<GitlabAdapterBehaviors> {
    /// The `gitlab-adapter` component.
    pub gitlab_adapter: gitlab_adapter::GitlabAdapter<GitlabAdapterBehaviors>,
    published: Vec<SystemEvent>,
    cursor: usize,
}

impl<GitlabAdapterBehaviors> System<GitlabAdapterBehaviors> {
    /// Assembles the system from its components.
    pub fn new(gitlab_adapter: gitlab_adapter::GitlabAdapter<GitlabAdapterBehaviors>) -> Self {
        Self {
            gitlab_adapter,
            published: Vec::new(),
            cursor: 0,
        }
    }

    /// Everything published so far, in publication order — the system's observable record.
    pub fn published(&self) -> &[SystemEvent] {
        &self.published
    }
}

impl<GitlabAdapterBehaviors> System<GitlabAdapterBehaviors> {
    /// Delivers until quiescent: collects every component's outbox onto the log, then delivers
    /// each logged event to every binding that reacts to it — at least once each, which is the
    /// guarantee the specification declares.
    ///
    /// `Err` carries the first unmet obligation that delivery could not route around; the log
    /// keeps everything already published. A specification whose bindings feed each other
    /// without end will not quiesce, and this pump will not pretend otherwise.
    pub fn pump(&mut self) -> Result<(), gitlab_types::obligation::UnmetObligation> {
        loop {
            self.collect();
            if self.cursor == self.published.len() {
                return Ok(());
            }
            self.cursor += 1;
        }
    }

    /// Moves every component's outbox onto the log, in component order.
    fn collect(&mut self) {
        for event in self.gitlab_adapter.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
    }
}

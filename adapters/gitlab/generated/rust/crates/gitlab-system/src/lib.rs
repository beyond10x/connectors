// generated from gitlab v1
// model digest 5464a959b36fe5673bd570eecf3f661540dd261b60c0fde2fd37f4a5bf2d97f3
// contract digest c3d3a9b408c90c2a8b03daf08984379733603f2c84f483640772f01ea8aea190
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

//! What this Connector publishes, and under which name.
//!
//! The operation table below is the one place that decides how an operation is called on the
//! wire, so the projections that hand a name to a caller — search and describe — live here with
//! it rather than beside dispatch.

use protocol::audio::{
    SPEECH_SPEAK_OPERATION, SPEECH_SPEAK_TOOL_REF, SPEECH_STATUS_OPERATION, SPEECH_STATUS_TOOL_REF,
};
use protocol::browser::{
    BROWSER_CLOSE_OPERATION, BROWSER_CLOSE_TOOL_REF, BROWSER_GOTO_OPERATION, BROWSER_GOTO_TOOL_REF,
    BROWSER_OPEN_OPERATION, BROWSER_OPEN_TOOL_REF, BROWSER_SCREENSHOT_OPERATION,
    BROWSER_SCREENSHOT_TOOL_REF, BROWSER_SNAPSHOT_OPERATION, BROWSER_SNAPSHOT_TOOL_REF,
};
use protocol::operation::{
    DescribeRequest, OperationDescription, OperationError, OperationResult, OperationSummary,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

pub(super) const MODULE_REQUEST_TYPE: &str = "b10x.module-request.v1+jws";
pub(super) const MODULE_AUTHORIZATION_SCHEME: &str = "DLModule ";
pub(super) const MODULE_REQUEST_TTL_SECONDS: u64 = 30;
pub(super) const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const HTTP_TOTAL_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) const WORK_EVENT_CHANNEL: &str = "event-channel:b10x:work";
pub(super) const WORK_EVENT_BINDING: &str = "work.module-events.v1";
pub(super) const PLANNER_EVENT_CHANNEL: &str = "event-channel:b10x:planner";
pub(super) const PLANNER_EVENT_BINDING: &str = "planner.module-events.v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkOwnerEventPage {
    pub(super) events: Vec<WorkOwnerEvent>,
    pub(super) next_cursor: Option<String>,
    pub(super) has_more: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WorkOwnerEvent {
    pub(super) protocol: String,
    pub(super) id: String,
    pub(super) module: String,
    pub(super) key: String,
    pub(super) schema_version: u16,
    pub(super) occurred_at: String,
    pub(super) cursor: String,
    pub(super) data: Value,
}

/// One resolved operation: its catalog contract, the name this Connector dispatches and audits
/// under, and its title.
pub(super) struct ResolvedOperation<'a> {
    pub(super) contract: &'a connector_resolve::document::Operation,
    pub(super) canonical: &'static str,
    pub(super) title: &'static str,
}

/// Each row is `(catalog id, published operation reference, title)`.
///
/// Every operation is addressable under two names — the catalog id (`colab-workspace-list`) and
/// the dotted operation reference (`colab.workspaces.list`) — and `policy.rs` adds a third for
/// the module-global ids. All of them keep resolving, so existing callers are unaffected. Only
/// the second column is ever published, so someone reading the surface meets each operation once,
/// under one name, instead of believing there are two operations that behave identically.
#[rustfmt::skip]
pub(super) const OPERATIONS: [(&str, &str, &str); 72] = [
    (SPEECH_SPEAK_OPERATION, SPEECH_SPEAK_TOOL_REF, "Speak on local audio"),
    (SPEECH_STATUS_OPERATION, SPEECH_STATUS_TOOL_REF, "Inspect local speech readiness"),
    (BROWSER_OPEN_OPERATION, BROWSER_OPEN_TOOL_REF, "Open a dedicated browser"),
    (BROWSER_GOTO_OPERATION, BROWSER_GOTO_TOOL_REF, "Navigate the dedicated browser"),
    (BROWSER_SNAPSHOT_OPERATION, BROWSER_SNAPSHOT_TOOL_REF, "Read the browser page structure"),
    (BROWSER_SCREENSHOT_OPERATION, BROWSER_SCREENSHOT_TOOL_REF, "Capture a browser screenshot"),
    (BROWSER_CLOSE_OPERATION, BROWSER_CLOSE_TOOL_REF, "Close the dedicated browser"),
    ("knowledge-query", "knowledge.query", "Query visible Ontology claims"),
    ("knowledge-explain", "knowledge.explain", "Explain one Ontology claim"),
    ("knowledge-snapshot", "knowledge.snapshot", "Create an Ontology context snapshot"),
    ("ontology-branch-list", "ontology.branches.list", "List Ontology branches"),
    ("ontology-branch-create", "ontology.branches.create", "Create an Ontology branch"),
    ("ontology-branch-schema-extend", "ontology.branches.schema.extend", "Extend an Ontology branch schema"),
    ("ontology-claim-assert", "ontology.claims.assert", "Assert an Ontology claim"),
    ("ontology-claim-retract", "ontology.claims.retract", "Retract an Ontology claim"),
    ("ontology-pack-install", "ontology.packs.install", "Install an Ontology pack"),
    ("ontology-proposal-create", "ontology.proposals.create", "Create an Ontology proposal"),
    ("ontology-proposal-get", "ontology.proposals.get", "Get an Ontology proposal"),
    ("ontology-proposal-evaluate", "ontology.proposals.evaluate", "Evaluate an Ontology proposal"),
    ("ontology-proposal-approval-record", "ontology.proposals.approval.record", "Record an Ontology proposal approval"),
    ("ontology-proposal-promote", "ontology.proposals.promote", "Promote an Ontology proposal"),
    ("ontology-schema-register", "ontology.schemas.register", "Register an Ontology schema"),
    ("work-request-create", "work.requests.create", "Create a Work request"),
    ("work-request-get", "work.requests.get", "Get a Work request"),
    ("work-request-list", "work.requests.list", "List Work requests"),
    ("work-task-create", "work.tasks.create", "Create a Work task"),
    ("work-task-get", "work.tasks.get", "Get a Work task"),
    ("work-task-list", "work.tasks.list", "List Work tasks"),
    ("work-task-status-update", "work.tasks.status.update", "Update Work task status"),
    ("planner-project-register", "planner.projects.register", "Register a Planner project"),
    ("planner-project-list", "planner.projects.list", "List Planner projects"),
    ("planner-board-get", "planner.board.get", "Read a project board"),
    ("planner-search", "planner.search", "Search one project"),
    ("planner-entity-create", "planner.entities.create", "Create a planning entity"),
    ("planner-entity-get", "planner.entities.get", "Read a planning entity"),
    ("planner-entity-list", "planner.entities.list", "List planning entities"),
    ("planner-entity-update", "planner.entities.update", "Update a planning entity"),
    ("planner-entity-delete", "planner.entities.delete", "Delete a planning entity"),
    ("planner-entity-restore", "planner.entities.restore", "Restore a planning entity"),
    ("planner-story-transition-explain", "planner.stories.transitions.explain", "Explain a story transition"),
    ("planner-story-gate-record", "planner.stories.gates.record", "Record story gate evidence"),
    ("planner-story-transition", "planner.stories.transition", "Transition a story"),
    ("planner-decision-transition", "planner.decisions.transition", "Transition a decision"),
    ("planner-sync-preview", "planner.sync.preview", "Preview repository synchronization"),
    ("planner-sync-apply", "planner.sync.apply", "Apply repository synchronization"),
    ("planner-sync-confirm", "planner.sync.confirm", "Confirm repository synchronization"),
    ("planner-sync-conflict-list", "planner.sync.conflicts.list", "List synchronization conflicts"),
    ("planner-sync-conflict-resolve", "planner.sync.conflicts.resolve", "Resolve a synchronization conflict"),
    ("planner-search-all", "planner.search.all", "Search every Planner project"),
    ("planner-sync-session-begin", "planner.sync.sessions.begin", "Begin a repository sync session"),
    ("planner-sync-session-upload", "planner.sync.sessions.upload", "Upload repository sync documents"),
    ("planner-sync-session-preview", "planner.sync.sessions.preview", "Preview a repository sync session"),
    ("planner-sync-session-apply", "planner.sync.sessions.apply", "Apply a repository sync session"),
    ("planner-sync-session-confirm", "planner.sync.sessions.confirm", "Confirm a repository sync session"),
    ("workspaces-list", "workspaces.list", "List visible logical workspaces"),
    ("workspaces-create", "workspaces.create", "Create a logical workspace"),
    ("workspaces-get", "workspaces.get", "Get a logical workspace"),
    ("workspaces-delete", "workspaces.delete", "Destroy a logical workspace"),
    ("workspaces-checkouts-list", "workspaces.checkouts.list", "List workspace checkouts"),
    ("workspaces-checkouts-create", "workspaces.checkouts.create", "Create a physical checkout"),
    ("workspaces-checkouts-delete", "workspaces.checkouts.delete", "Destroy a physical checkout"),
    ("workspace-file-read", "workspace.files.read", "Read a digest-bound checkout file"),
    ("workspace-tree-list", "workspace.files.list", "List a bounded checkout tree"),
    ("workspace-file-replace", "workspace.files.replace", "Compare-and-set replace a checkout file"),
    ("workspace-file-edit", "workspace.files.edit", "Compare-and-set edit checkout text"),
    ("workspace-file-patch", "workspace.files.patch", "Compare-and-set patch checkout lines"),
    ("workspace-exec-start", "workspace.exec.start", "Run a bounded command in a checkout"),
    ("colab-room-create", "colab.rooms.create", "Create a conversation room"),
    ("colab-workspace-list", "colab.workspaces.list", "List conversation workspace attachments"),
    ("colab-workspace-attach", "colab.workspaces.attach", "Attach a checkout to a conversation"),
    ("colab-workspace-current-set", "colab.workspaces.current.set", "Select the current conversation checkout"),
    ("colab-workspace-detach", "colab.workspaces.detach", "Detach a checkout from a conversation"),
];

impl super::B10xBackend {
    /// Resolve one operation by any of its names.
    pub(super) fn operation(&self, operation_ref: &str) -> Option<ResolvedOperation<'_>> {
        let (canonical, _, title) = super::operation_row(operation_ref)?;
        self.configured(canonical)
            .then(|| self.document.operation(canonical))
            .flatten()
            .map(|contract| ResolvedOperation {
                contract,
                canonical,
                title,
            })
    }

    /// One row per operation, never one row per name: the rows are keyed by catalog id, and every
    /// summary is named by `published_ref`. Searching the catalog id still finds the operation.
    pub(super) fn search(&self, query: &str) -> Vec<OperationSummary> {
        let needle = query.to_ascii_lowercase();
        super::all_operation_rows()
            .filter_map(|(canonical, published_ref, title)| {
                let operation = self
                    .configured(canonical)
                    .then(|| self.document.operation(canonical))
                    .flatten()?;
                let haystack = format!(
                    "{canonical} {published_ref} {title} {}",
                    operation.contract_description()
                )
                .to_ascii_lowercase();
                (needle.is_empty() || haystack.contains(&needle)).then(|| OperationSummary {
                    operation_ref: published_ref.to_owned(),
                    title: title.to_owned(),
                    effect: super::effect(operation.effects()),
                    approval: super::approval(canonical, operation.effects()),
                    connections: vec![self.connection()],
                })
            })
            .collect()
    }

    /// Describe is a lookup by a name the caller already holds, so it answers under that name.
    ///
    /// This is deliberately not the published name. The Agent's Connector client refuses a
    /// description whose `operation_ref` differs from the one it asked about
    /// (`runtime/agent/crates/agent-connectors-client/src/lib.rs:2027`), and Zwirn asks for the
    /// catalog ids directly (`products/zwirn/crates/agent-app/src/remote_workspaces.rs:23-33`).
    /// Renaming the answer here would fail every one of those describes as a protocol error and
    /// take the workspace and module-widget surfaces with it. The single-name rule therefore
    /// belongs to `search`, which is where a name is discovered rather than presented.
    pub(super) fn describe(
        &self,
        context: &service::PrincipalContext,
        request: DescribeRequest,
    ) -> Result<OperationResult, OperationError> {
        let ResolvedOperation {
            contract: operation,
            canonical,
            title,
        } = self
            .operation(&request.operation_ref)
            .ok_or_else(super::not_found)?;
        Ok(OperationResult::Describe(OperationDescription {
            operation_ref: request.operation_ref,
            title: title.to_owned(),
            description: operation.contract_description().to_owned(),
            input_schema: operation.input_schema().clone(),
            output_schema: super::response_schema(&self.catalog, canonical)?,
            effect: super::effect(operation.effects()),
            approval: super::approval(canonical, operation.effects()),
            connections: vec![self.connection()],
            description_ref: self.description_ref(context, canonical),
        }))
    }
}

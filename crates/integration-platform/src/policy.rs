use connector_resolve::document::{HostEffect, Operation};
use protocol::audio::SPEECH_SPEAK_OPERATION;
use protocol::browser::BROWSER_SCREENSHOT_OPERATION;
use protocol::operation::{ApprovalPosture, EffectClass, OperationError, OperationErrorCode};
use serde_json::Value;

use super::{unavailable, OPERATIONS};

pub(super) fn all_operation_rows(
) -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
    OPERATIONS.into_iter()
}

pub(super) fn operation_row(value: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let value = module_global_alias(value).unwrap_or(value);
    all_operation_rows()
        .find(|(canonical, operation_ref, _)| value == *canonical || value == *operation_ref)
}

fn module_global_alias(value: &str) -> Option<&'static str> {
    Some(match value {
        "work/request.create" => "work-request-create",
        "work/request.get" => "work-request-get",
        "work/request.list" => "work-request-list",
        "work/task.create" => "work-task-create",
        "work/task.get" => "work-task-get",
        "work/task.list" => "work-task-list",
        "work/task.status.update" => "work-task-status-update",
        "planner/project.register" => "planner-project-register",
        "planner/project.list" => "planner-project-list",
        "planner/board.get" => "planner-board-get",
        "planner/search.project" => "planner-search",
        "planner/entity.create" => "planner-entity-create",
        "planner/entity.get" => "planner-entity-get",
        "planner/entity.list" => "planner-entity-list",
        "planner/entity.update" => "planner-entity-update",
        "planner/entity.delete" => "planner-entity-delete",
        "planner/entity.restore" => "planner-entity-restore",
        "planner/story.transition.explain" => "planner-story-transition-explain",
        "planner/story.gate.record" => "planner-story-gate-record",
        "planner/story.transition" => "planner-story-transition",
        "planner/decision.transition" => "planner-decision-transition",
        "planner/sync.preview" => "planner-sync-preview",
        "planner/sync.apply" => "planner-sync-apply",
        "planner/sync.confirm" => "planner-sync-confirm",
        "planner/sync.conflict.list" => "planner-sync-conflict-list",
        "planner/sync.conflict.resolve" => "planner-sync-conflict-resolve",
        "planner/search.all" => "planner-search-all",
        "planner/sync.session.begin" => "planner-sync-session-begin",
        "planner/sync.session.upload" => "planner-sync-session-upload",
        "planner/sync.session.preview" => "planner-sync-session-preview",
        "planner/sync.session.apply" => "planner-sync-session-apply",
        "planner/sync.session.confirm" => "planner-sync-session-confirm",
        "ontology/schema.register" => "ontology-schema-register",
        "ontology/branch.list" => "ontology-branch-list",
        "ontology/branch.create" => "ontology-branch-create",
        "ontology/branch.schema.extend" => "ontology-branch-schema-extend",
        "ontology/claim.assert" => "ontology-claim-assert",
        "ontology/claim.retract" => "ontology-claim-retract",
        "ontology/claim.explain" => "knowledge-explain",
        "ontology/claim.query" => "knowledge-query",
        "ontology/snapshot.create" => "knowledge-snapshot",
        "ontology/pack.install" => "ontology-pack-install",
        "ontology/proposal.create" => "ontology-proposal-create",
        "ontology/proposal.get" => "ontology-proposal-get",
        "ontology/proposal.evaluate" => "ontology-proposal-evaluate",
        "ontology/proposal.approval.record" => "ontology-proposal-approval-record",
        "ontology/proposal.promote" => "ontology-proposal-promote",
        "workspaces/workspace.list" => "workspaces-list",
        "workspaces/workspace.create" => "workspaces-create",
        "workspaces/workspace.get" => "workspaces-get",
        "workspaces/workspace.delete" => "workspaces-delete",
        "workspaces/checkout.list" => "workspaces-checkouts-list",
        "workspaces/checkout.create" => "workspaces-checkouts-create",
        "workspaces/checkout.delete" => "workspaces-checkouts-delete",
        "workspaces/file.read" => "workspace-file-read",
        "workspaces/file.list" => "workspace-tree-list",
        "workspaces/file.replace" => "workspace-file-replace",
        "workspaces/file.edit" => "workspace-file-edit",
        "workspaces/file.patch" => "workspace-file-patch",
        "workspaces/exec.start" => "workspace-exec-start",
        "colab/room.create" => "colab-room-create",
        "colab/workspace.list" => "colab-workspace-list",
        "colab/workspace.attach" => "colab-workspace-attach",
        "colab/workspace.current.set" => "colab-workspace-current-set",
        "colab/workspace.detach" => "colab-workspace-detach",
        _ => return None,
    })
}

pub(super) fn module_operation(canonical: &str) -> Option<&'static str> {
    Some(match canonical {
        "work-request-create" => "work/request.create",
        "work-request-get" => "work/request.get",
        "work-request-list" => "work/request.list",
        "work-task-create" => "work/task.create",
        "work-task-get" => "work/task.get",
        "work-task-list" => "work/task.list",
        "work-task-status-update" => "work/task.status.update",
        "planner-project-register" => "planner/project.register",
        "planner-project-list" => "planner/project.list",
        "planner-board-get" => "planner/board.get",
        "planner-search" => "planner/search.project",
        "planner-entity-create" => "planner/entity.create",
        "planner-entity-get" => "planner/entity.get",
        "planner-entity-list" => "planner/entity.list",
        "planner-entity-update" => "planner/entity.update",
        "planner-entity-delete" => "planner/entity.delete",
        "planner-entity-restore" => "planner/entity.restore",
        "planner-story-transition-explain" => "planner/story.transition.explain",
        "planner-story-gate-record" => "planner/story.gate.record",
        "planner-story-transition" => "planner/story.transition",
        "planner-decision-transition" => "planner/decision.transition",
        "planner-sync-preview" => "planner/sync.preview",
        "planner-sync-apply" => "planner/sync.apply",
        "planner-sync-confirm" => "planner/sync.confirm",
        "planner-sync-conflict-list" => "planner/sync.conflict.list",
        "planner-sync-conflict-resolve" => "planner/sync.conflict.resolve",
        "planner-search-all" => "planner/search.all",
        "planner-sync-session-begin" => "planner/sync.session.begin",
        "planner-sync-session-upload" => "planner/sync.session.upload",
        "planner-sync-session-preview" => "planner/sync.session.preview",
        "planner-sync-session-apply" => "planner/sync.session.apply",
        "planner-sync-session-confirm" => "planner/sync.session.confirm",
        "ontology-schema-register" => "ontology/schema.register",
        "ontology-branch-list" => "ontology/branch.list",
        "ontology-branch-create" => "ontology/branch.create",
        "ontology-branch-schema-extend" => "ontology/branch.schema.extend",
        "ontology-claim-assert" => "ontology/claim.assert",
        "ontology-claim-retract" => "ontology/claim.retract",
        "knowledge-explain" => "ontology/claim.explain",
        "knowledge-query" => "ontology/claim.query",
        "knowledge-snapshot" => "ontology/snapshot.create",
        "ontology-pack-install" => "ontology/pack.install",
        "ontology-proposal-create" => "ontology/proposal.create",
        "ontology-proposal-get" => "ontology/proposal.get",
        "ontology-proposal-evaluate" => "ontology/proposal.evaluate",
        "ontology-proposal-approval-record" => "ontology/proposal.approval.record",
        "ontology-proposal-promote" => "ontology/proposal.promote",
        "workspaces-list" => "workspaces/workspace.list",
        "workspaces-create" => "workspaces/workspace.create",
        "workspaces-get" => "workspaces/workspace.get",
        "workspaces-delete" => "workspaces/workspace.delete",
        "workspaces-checkouts-list" => "workspaces/checkout.list",
        "workspaces-checkouts-create" => "workspaces/checkout.create",
        "workspaces-checkouts-delete" => "workspaces/checkout.delete",
        "workspace-file-read" => "workspaces/file.read",
        "workspace-tree-list" => "workspaces/file.list",
        "workspace-file-replace" => "workspaces/file.replace",
        "workspace-file-edit" => "workspaces/file.edit",
        "workspace-file-patch" => "workspaces/file.patch",
        "workspace-exec-start" => "workspaces/exec.start",
        "colab-room-create" => "colab/room.create",
        "colab-workspace-list" => "colab/workspace.list",
        "colab-workspace-attach" => "colab/workspace.attach",
        "colab-workspace-current-set" => "colab/workspace.current.set",
        "colab-workspace-detach" => "colab/workspace.detach",
        _ => return None,
    })
}

pub(super) fn response_schema(catalog: &Value, canonical: &str) -> Result<Value, OperationError> {
    let operation = catalog["operations"]
        .as_array()
        .and_then(|operations| {
            operations
                .iter()
                .find(|operation| operation.get("id").and_then(Value::as_str) == Some(canonical))
        })
        .ok_or_else(unavailable)?;
    let schema = operation
        .as_object()
        .and_then(|fields| fields.get("response_schema"))
        .cloned()
        .or_else(|| {
            (canonical.starts_with("workspaces-")
                || canonical.starts_with("workspace-")
                || canonical.starts_with("colab-"))
            .then(|| serde_json::json!({"type": "object"}))
        })
        .or_else(|| {
            matches!(
                canonical,
                "ontology-proposal-create"
                    | "ontology-proposal-approval-record"
                    | "ontology-schema-register"
            )
            .then(|| serde_json::json!({"type": "null"}))
        })
        .ok_or_else(unavailable)?;
    // The catalog intentionally represents an exact JSON null response as `null`, while the
    // Operation protocol publishes JSON Schema. Preserve that constraint instead of mistaking it
    // for an absent schema.
    Ok(if schema.is_null() {
        serde_json::json!({"type": "null"})
    } else {
        schema
    })
}

pub(super) fn effect(effects: &[HostEffect]) -> EffectClass {
    if effects.contains(&HostEffect::Write) {
        EffectClass::Mutating
    } else {
        EffectClass::ReadOnly
    }
}

pub(super) fn approval(canonical: &str, effects: &[HostEffect]) -> ApprovalPosture {
    if effects.contains(&HostEffect::Write)
        || matches!(
            canonical,
            SPEECH_SPEAK_OPERATION | BROWSER_SCREENSHOT_OPERATION
        )
    {
        ApprovalPosture::Required
    } else {
        ApprovalPosture::NotRequired
    }
}

pub(super) fn post_dispatch_error(operation: &Operation, error: OperationError) -> OperationError {
    if operation.effects().contains(&HostEffect::Write) {
        OperationError::new(
            OperationErrorCode::OutcomeUnknown,
            "the Connector dispatched an effect but could not verify its outcome",
            false,
        )
    } else {
        error
    }
}

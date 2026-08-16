use connector_resolve::document::{HostEffect, Operation};
use protocol::audio::SPEECH_SPEAK_OPERATION;
use protocol::browser::BROWSER_SCREENSHOT_OPERATION;
use protocol::operation::{ApprovalPosture, EffectClass, OperationError, OperationErrorCode};
use serde_json::Value;

use super::{invalid, unavailable, OPERATIONS};

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
        _ => return None,
    })
}

pub(super) fn response_schema(catalog: &Value, canonical: &str) -> Result<Value, OperationError> {
    let operation = catalog["operations"]
        .as_array()
        .and_then(|operations| {
            operations.iter().find(|operation| {
                operation.get("id").and_then(Value::as_str) == Some(canonical)
            })
        })
        .ok_or_else(unavailable)?;
    let schema = operation
        .as_object()
        .and_then(|fields| fields.get("response_schema"))
        .cloned()
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

pub(super) fn approval(canonical: &str) -> ApprovalPosture {
    if matches!(
        canonical,
        SPEECH_SPEAK_OPERATION
            | BROWSER_SCREENSHOT_OPERATION
            | "work-request-create"
            | "work-task-create"
            | "work-task-status-update"
            | "ontology-branch-create"
            | "ontology-branch-schema-extend"
            | "ontology-claim-assert"
            | "ontology-claim-retract"
            | "ontology-pack-install"
            | "ontology-proposal-create"
            | "ontology-proposal-evaluate"
            | "ontology-proposal-approval-record"
            | "ontology-proposal-promote"
            | "ontology-schema-register"
    ) {
        ApprovalPosture::Required
    } else {
        ApprovalPosture::NotRequired
    }
}

pub(super) fn check_approval(
    canonical: &str,
    evidence: Option<&str>,
) -> Result<(), OperationError> {
    match (approval(canonical), evidence) {
        (ApprovalPosture::Required, None) => Err(OperationError::new(
            OperationErrorCode::ApprovalRequired,
            "external approval evidence is required",
            false,
        )),
        // An opaque caller-supplied reference is not proof. Until the Connector process has a
        // verifier that binds issuer, principal, invocation digest, decision, and freshness, every
        // presented value is refused. In particular, no deployment-config string acts as a shared
        // approval password.
        (ApprovalPosture::Required, Some(_)) => Err(OperationError::new(
            OperationErrorCode::ApprovalDenied,
            "receiver-verifiable invocation approval is not configured",
            false,
        )),
        (ApprovalPosture::NotRequired, Some(_)) => Err(invalid()),
        _ => Ok(()),
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

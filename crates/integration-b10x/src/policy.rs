use connector_resolve::document::{HostEffect, Operation};
use protocol::audio::SPEECH_SPEAK_OPERATION;
use protocol::browser::BROWSER_SCREENSHOT_OPERATION;
use protocol::operation::{ApprovalPosture, EffectClass, OperationError, OperationErrorCode};
use serde_json::Value;

use super::{invalid, unavailable, OPERATIONS, REMAINING_WORK_OPERATIONS};

pub(super) fn all_operation_rows(
) -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
    OPERATIONS.into_iter().chain(REMAINING_WORK_OPERATIONS)
}

pub(super) fn operation_row(value: &str) -> Option<(&'static str, &'static str, &'static str)> {
    all_operation_rows()
        .find(|(canonical, operation_ref, _)| value == *canonical || value == *operation_ref)
}

pub(super) fn response_schema(catalog: &Value, canonical: &str) -> Result<Value, OperationError> {
    catalog["operations"]
        .as_array()
        .and_then(|operations| {
            operations
                .iter()
                .find(|operation| operation["id"] == canonical)
        })
        .and_then(|operation| operation.get("response_schema"))
        .cloned()
        .ok_or_else(unavailable)
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

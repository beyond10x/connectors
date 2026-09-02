//! Human-decided exact-input approval issuance.

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse as _, Json, Response};
use domain::ApprovalRecord;
use protocol::approval::{IssuedApproval, RequestEnvelope};
use protocol::operation::{ApprovalPosture, OperationError, OperationErrorCode};

use super::admission::{enforcement_refusal_response, redescribe};
use super::enforcement::canonical_input_digest;
use super::{
    bearer, error, operation_failure, HostedState, IdentityVerificationError, CONNECTORS_AUDIENCE,
};

/// Issue one bounded, one-time proof after re-checking the human's exact pending invocation.
pub(super) async fn issue(
    State(state): State<HostedState>,
    headers: HeaderMap,
    Json(request): Json<RequestEnvelope>,
) -> Response {
    if let Err(error) = request.validate() {
        return operation_failure(&request.request_id, error, StatusCode::BAD_REQUEST);
    }
    let Some(credential) = bearer(&headers) else {
        return error(StatusCode::UNAUTHORIZED, "identity-access-token-required");
    };
    let principal = match state.verifier.verify(credential, CONNECTORS_AUDIENCE).await {
        Ok(principal) => principal,
        Err(IdentityVerificationError::Refused) => {
            return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused");
        }
        Err(IdentityVerificationError::Unavailable) => {
            return error(StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable");
        }
    };
    if principal.tenant_id != request.context.tenant_id
        || !principal.allows("connectors.approvals.issue")
        || principal.actor_subject != principal.subject
    {
        return operation_failure(
            &request.request_id,
            OperationError::new(
                OperationErrorCode::NotGranted,
                "the verified human authority does not admit approval issuance",
                false,
            ),
            StatusCode::FORBIDDEN,
        );
    }
    let owner = match principal.principal_context(&request.request_id) {
        Ok(owner) => owner,
        Err(_) => return error(StatusCode::UNAUTHORIZED, "identity-access-token-refused"),
    };
    let described = match redescribe(
        &state,
        &owner,
        &request.request_id,
        &request.request.operation_ref,
    )
    .await
    {
        Ok(description) => description,
        Err(response) => return *response,
    };
    let exact_connection = described
        .connections
        .iter()
        .any(|connection| connection.connection_ref == request.request.connection_ref);
    if described.description_ref != request.request.description_ref
        || described.approval != ApprovalPosture::Required
        || !exact_connection
    {
        return operation_failure(
            &request.request_id,
            OperationError::new(
                OperationErrorCode::StaleAuthority,
                "the approval request does not match the current Connector description",
                false,
            ),
            StatusCode::CONFLICT,
        );
    }
    let mut random = [0_u8; 32];
    if getrandom::fill(&mut random).is_err() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "approval-authority-unavailable",
        );
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let expires_at_seconds = now.saturating_add(request.request.ttl_seconds);
    let reference = format!("approval:{}", hex::encode(random));
    let record = ApprovalRecord {
        reference: reference.clone(),
        issuer: principal.issuer,
        subject: principal.subject,
        operation: request.request.operation_ref,
        connection: request.request.connection_ref,
        input_digest: canonical_input_digest(&request.request.input),
        expires_at_seconds,
    };
    if let Err(refusal) = state.authority.issue(&record) {
        return enforcement_refusal_response(&request.request_id, refusal);
    }
    let mut response = (
        StatusCode::CREATED,
        Json(IssuedApproval {
            protocol: protocol::approval::CONTRACT.to_owned(),
            request_id: request.request_id,
            approval_evidence_ref: reference,
            expires_at_seconds,
        }),
    )
        .into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        header::PRAGMA,
        axum::http::HeaderValue::from_static("no-cache"),
    );
    response
}

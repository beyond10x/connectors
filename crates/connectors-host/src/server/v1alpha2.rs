//! `POST /v1alpha2/invoke` (contracts/service/compatibility.md § 2.1): the
//! minimal route that carries the execution-audit anchor. Every admitted
//! invocation, read or write, is anchored and acknowledged before the adapter
//! is called, and its one final observation is recorded after (audit.md § 1-3).
//! The mutation attempt ledger and the `mutation` member are not served here.
use super::{Service, admitted};
use crate::local::{audit, service::Admitted};
use axum::{
    Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response as HttpResponse},
};
use connectors_core::{
    RESPONSE_LIMIT,
    v1alpha2::{
        AuditStatus, Error, ErrorCode, InvokeRequest, InvokeResponse, Refusal, ResponseStatus,
        Version,
    },
};
use connectors_sdk::validate;
use serde_json::Value;
use std::time::{Duration, Instant};

/// An `external_write` operation declares `profile: mutation`, and a read
/// profile cannot carry it (contracts/operations/v1alpha1/semantics.md § 2,
/// Effect declaration rule); the legacy Descriptor carries no `effects`.
const MUTATION_PROFILE: &str = "mutation";
const DEADLINE: Duration = Duration::from_secs(20);

pub(super) async fn invoke(
    State(service): State<Service>,
    headers: HeaderMap,
    body: std::result::Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> HttpResponse {
    // A host without `state` answers before decoding (compatibility.md § 2.1).
    let Some(state) = service.state.clone() else {
        return refused(
            None,
            ErrorCode::Unavailable,
            "service state is not configured",
        );
    };
    if let Err(error) = admitted(&service, &headers).await {
        return refused(None, code(error.code), &error.message);
    }
    let Ok(bytes) = body else {
        return refused(None, ErrorCode::Capacity, "request exceeds byte limit");
    };
    let request = match InvokeRequest::decode(&bytes) {
        Ok(request) => request,
        Err(refusal) => return answer(refusal.into_response()),
    };
    if !connectors_core::valid_id(&request.request_id)
        || !connectors_core::valid_id(&request.operation)
    {
        return refused(None, ErrorCode::InvalidInput, "invalid request identifier");
    }
    let request_id = request.request_id.clone();
    let refuse = |code, message: &str| refused(Some(request_id.clone()), code, message);
    let descriptor = service.adapter.descriptor();
    if request.revision != descriptor.revision {
        return refuse(
            ErrorCode::StaleDescription,
            "refresh the descriptor before resubmitting",
        );
    }
    let operation = match descriptor.operation(&request.operation) {
        Ok(operation) => operation.clone(),
        Err(error) => return refuse(code(error.code), &error.message),
    };
    if let Err(error) = validate(&operation.input_schema, &request.input) {
        return refuse(code(error.code), &error.message);
    }
    let Ok(_permit) = service.slots.clone().try_acquire_owned() else {
        return refuse(ErrorCode::Capacity, "service concurrency limit reached");
    };
    let access = if operation.profile == MUTATION_PROFILE {
        audit::Access::Write
    } else {
        audit::Access::Read
    };

    // The anchor is acknowledged before any adapter call; without it nothing
    // is dispatched and no record is referenced (audit.md § 2).
    let anchored = {
        let state = state.clone();
        let request_id = request_id.clone();
        let operation = request.operation.clone();
        let revision = descriptor.revision.clone();
        tokio::task::spawn_blocking(move || {
            state.anchor(&Admitted {
                request_id: &request_id,
                operation: &operation,
                descriptor_revision: &revision,
                access,
            })
        })
        .await
    };
    let Ok(Ok(reference)) = anchored else {
        return refuse(ErrorCode::Unavailable, "execution audit is unavailable");
    };

    let started = Instant::now();
    let result = dispatch(&service, &request, &operation).await;
    tracing::info!(request_id = ?request.request_id, operation = ?request.operation, elapsed_ms = started.elapsed().as_millis(), success = result.is_ok(), "operation completed");

    let (outcome, observed_code) = match &result {
        Ok(_) => (audit::Outcome::Success, None),
        // A write whose answer did not arrive may have taken effect.
        Err(error) if access == audit::Access::Write && error.code == ErrorCode::Timeout => {
            (audit::Outcome::Unknown, Some("timeout".to_owned()))
        }
        Err(error) => (audit::Outcome::Error, Some(code_text(error.code))),
    };
    let complete = {
        let state = state.clone();
        let reference = reference.clone();
        tokio::task::spawn_blocking(move || state.finish(&reference, outcome, observed_code))
            .await
            .unwrap_or(false)
    };
    let (status, result, error) = match result {
        Ok(value) => (ResponseStatus::Success, Some(value), None),
        Err(error) => (ResponseStatus::Error, None, Some(error)),
    };
    answer(InvokeResponse {
        version: Version::V1alpha2,
        request_id: Some(request_id),
        status,
        result,
        error,
        audit_ref: Some(reference.audit_ref),
        audit_status: if complete {
            AuditStatus::Complete
        } else {
            AuditStatus::Incomplete
        },
        source_audit: None,
        mutation: None,
    })
}

async fn dispatch(
    service: &Service,
    request: &InvokeRequest,
    operation: &connectors_core::Operation,
) -> std::result::Result<Value, Error> {
    let result = tokio::time::timeout(
        DEADLINE,
        service
            .adapter
            .invoke_at(&request.revision, &request.operation, request.input.clone()),
    )
    .await
    .map_err(|_| Error::new(ErrorCode::Timeout, "operation deadline exceeded"))?
    .map_err(|error| Error::new(code(error.code), error.message))?;
    validate(&operation.output_schema, &result).map_err(|_| {
        Error::new(
            ErrorCode::UpstreamProtocol,
            "result does not match the declared output schema",
        )
    })?;
    if serde_json::to_vec(&result)
        .map_err(|_| Error::new(ErrorCode::Internal, "internal error"))?
        .len()
        > RESPONSE_LIMIT - 1024
    {
        return Err(Error::new(ErrorCode::Capacity, "result exceeds byte limit"));
    }
    Ok(result)
}

/// The 13 base codes keep their names on this binding.
fn code(code: connectors_core::ErrorCode) -> ErrorCode {
    use connectors_core::ErrorCode as Base;
    match code {
        Base::InvalidInput => ErrorCode::InvalidInput,
        Base::Unsupported => ErrorCode::Unsupported,
        Base::Unauthorized => ErrorCode::Unauthorized,
        Base::Forbidden => ErrorCode::Forbidden,
        Base::NotFound => ErrorCode::NotFound,
        Base::StaleDescription => ErrorCode::StaleDescription,
        Base::StaleCursor => ErrorCode::StaleCursor,
        Base::RateLimited => ErrorCode::RateLimited,
        Base::Unavailable => ErrorCode::Unavailable,
        Base::Capacity => ErrorCode::Capacity,
        Base::Timeout => ErrorCode::Timeout,
        Base::UpstreamProtocol => ErrorCode::UpstreamProtocol,
        Base::Internal => ErrorCode::Internal,
    }
}
fn code_text(code: ErrorCode) -> String {
    serde_json::to_value(code)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "internal".to_owned())
}

/// The extended HTTP mapping (compatibility.md § 5).
fn http_status(code: ErrorCode) -> StatusCode {
    use ErrorCode::*;
    match code {
        InvalidInput | Unsupported => StatusCode::BAD_REQUEST,
        Unauthorized => StatusCode::UNAUTHORIZED,
        Forbidden | ApprovalRequired | ApprovalRefused | NotGranted | RouteRefused
        | InsufficientScope => StatusCode::FORBIDDEN,
        NotFound => StatusCode::NOT_FOUND,
        StaleDescription | StaleCursor | ApprovalReplayed | IdempotencyConflict
        | StaleAuthority | ConnectionNotReady | SessionNotReady | OfferRejected => {
            StatusCode::CONFLICT
        }
        SessionLost | OfferExpired | LeaseExpired | Revoked => StatusCode::GONE,
        Capacity => StatusCode::PAYLOAD_TOO_LARGE,
        RateLimited => StatusCode::TOO_MANY_REQUESTS,
        Internal => StatusCode::INTERNAL_SERVER_ERROR,
        UpstreamProtocol | OutcomeUnknown => StatusCode::BAD_GATEWAY,
        Unavailable | RouteUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        Timeout => StatusCode::GATEWAY_TIMEOUT,
    }
}

/// A refusal before the anchor: no record exists, so `audit_ref` is null and
/// `audit_status` is `unavailable`.
fn refused(request_id: Option<String>, code: ErrorCode, message: &str) -> HttpResponse {
    let mut message = message.to_owned();
    if message.len() > connectors_core::v1alpha2::MESSAGE_LIMIT {
        message = "request refused".to_owned();
    }
    answer(
        Refusal {
            request_id,
            error: Error::new(code, message),
        }
        .into_response(),
    )
}

fn answer(response: InvokeResponse) -> HttpResponse {
    let status = response
        .error
        .as_ref()
        .map(|error| http_status(error.code))
        .unwrap_or(StatusCode::OK);
    (status, Json(response)).into_response()
}

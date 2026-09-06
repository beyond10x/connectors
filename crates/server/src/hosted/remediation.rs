//! Non-consuming current admission before the ordinary approval/dispatch boundary.
use super::*;
use protocol::operation::{v3, DescribeRequest, InvokeRequest, OperationResult};
use serde_json::Value;
use service::{
    CredentialReadiness, PrincipalContext, RemediationError, RemediationMetadata, RemediationTarget,
};

struct NoExternalSchemas;
impl jsonschema::Retrieve for NoExternalSchemas {
    fn retrieve(
        &self,
        _: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err("external schema retrieval is unavailable".into())
    }
}

/// Bind internal metadata to this receiver's canonical catalog and exact requested Connection.
/// A backend String is not authority to publish a reference. Personal OAuth's existing session
/// owner uses this exact catalog provider identity as integration_ref; no reference is inferred
/// from an arbitrary label, Connection string or provider response.
pub(crate) fn validated_metadata(
    metadata: &RemediationMetadata<'_>,
    target: RemediationTarget<'_>,
) -> Result<Value, RemediationError> {
    let canonical =
        catalog::reader::operation(target.operation_ref).ok_or(RemediationError::Refused)?;
    if metadata.operation.id() != canonical.id()
        || metadata.operation.record() != canonical.record()
        || metadata.operation.provider() != canonical.provider()
        || metadata.connection.id() != target.connection_ref
        || metadata.integration_ref != canonical.provider()
        || metadata.catalog_generation != catalog::reader::embedded().digest()
    {
        return Err(RemediationError::Refused);
    }
    let record: Value =
        serde_json::from_str(canonical.record()).map_err(|_| RemediationError::Unavailable)?;
    if !record["auth"].as_array().is_some_and(|mechanisms| {
        mechanisms.iter().any(|mechanism| {
            mechanism.as_array().is_some_and(|credentials| {
                credentials.len() == 1
                    && credentials[0].as_str() == Some(metadata.auth_profile.as_str())
            })
        })
    }) {
        return Err(RemediationError::Refused);
    }
    Ok(record)
}

pub(crate) fn validate_input(record: &Value, input: &Value) -> Result<(), RemediationError> {
    let schema = record
        .get("contract")
        .and_then(|contract| contract.get("input_schema"))
        .ok_or(RemediationError::Unavailable)?;
    let validator = jsonschema::options()
        .with_retriever(NoExternalSchemas)
        .build(schema)
        .map_err(|_| RemediationError::Unavailable)?;
    if validator.is_valid(input) {
        Ok(())
    } else {
        Err(RemediationError::InvalidInput)
    }
}

pub(crate) fn authentication(
    metadata: &RemediationMetadata<'_>,
    need: CredentialReadiness,
) -> Option<v3::AuthenticationRequired> {
    let need = match need {
        CredentialReadiness::MissingCredential => v3::AuthenticationNeed::AuthorizeConfigured,
        CredentialReadiness::CredentialDegraded => v3::AuthenticationNeed::ReauthorizeExisting,
        _ => return None,
    };
    Some(v3::AuthenticationRequired {
        operation_ref: metadata.operation.id().into(),
        connection_ref: metadata.connection.id().into(),
        integration_ref: metadata.integration_ref.clone(),
        auth_profile: metadata.auth_profile.clone(),
        need,
        attempt: v3::AuthenticationAttemptState::NotAttempted,
        next_action: v3::AuthenticationNextAction::StartTrustedRemediation,
    })
}

pub(crate) fn operation_error(error: RemediationError) -> v3::OperationError {
    let code = match error {
        RemediationError::Refused => v3::OperationErrorCode::NotGranted,
        RemediationError::InvalidInput => v3::OperationErrorCode::InvalidInput,
        RemediationError::Conflict => v3::OperationErrorCode::StaleAuthority,
        RemediationError::Unsupported | RemediationError::Unavailable => {
            v3::OperationErrorCode::Unavailable
        }
    };
    v3::OperationError::new(code, error.to_string(), false)
}

fn refusal(request_id: &str, error: RemediationError) -> Response {
    let status = match error {
        RemediationError::Refused => {
            return enforcement_refusal_response(
                request_id,
                enforcement::EnforcementRefusal::NotAdmitted,
            )
        }
        RemediationError::InvalidInput => StatusCode::BAD_REQUEST,
        RemediationError::Conflict => StatusCode::CONFLICT,
        RemediationError::Unsupported | RemediationError::Unavailable => {
            StatusCode::SERVICE_UNAVAILABLE
        }
    };
    (
        status,
        Json(v3::ResponseEnvelope::failure(
            request_id,
            operation_error(error),
        )),
    )
        .into_response()
}

pub(super) async fn operation_preflight(
    state: &HostedState,
    principal: &HostedPrincipal,
    owner: &PrincipalContext,
    request_id: &str,
    invoke: &InvokeRequest,
) -> Option<Response> {
    let target = RemediationTarget {
        operation_ref: &invoke.operation_ref,
        connection_ref: &invoke.connection_ref,
    };
    let resolved = state.backend.remediation_metadata(owner, target);
    // Old/default backends preserve ordinary admission and do not claim remediation support.
    if matches!(&resolved, Err(RemediationError::Unsupported)) {
        return None;
    }
    if let Err(error) = state.authority.require_remediation_store() {
        return Some(enforcement_refusal_response(request_id, error));
    }
    let metadata = match resolved {
        Ok(value) => value,
        Err(error) => return Some(refusal(request_id, error)),
    };
    let record = match validated_metadata(&metadata, target) {
        Ok(value) => value,
        Err(error) => return Some(refusal(request_id, error)),
    };
    if let Err(error) = state.authority.admit_remediation(
        principal,
        &metadata,
        &record,
        &invoke.description_ref,
        &canonical_input_digest(&invoke.input),
    ) {
        return Some(enforcement_refusal_response(request_id, error));
    }
    if let Err(error) = validate_input(&record, &invoke.input) {
        return Some(refusal(request_id, error));
    }
    // Ordinary invoke still needs its current real description. This is never fabricated for
    // Created bindings; those use the separately admitted trusted Start path.
    let description = match state
        .backend
        .handle(
            owner,
            OperationRequest::Describe(DescribeRequest {
                operation_ref: invoke.operation_ref.clone(),
            }),
        )
        .await
    {
        Ok(OperationResult::Describe(description)) => description,
        Err(error)
            if matches!(
                error.code,
                OperationErrorCode::NotFound | OperationErrorCode::NotGranted
            ) =>
        {
            return Some(refusal(request_id, RemediationError::Refused))
        }
        _ => return Some(refusal(request_id, RemediationError::Unavailable)),
    };
    if description.operation_ref != invoke.operation_ref
        || description.description_ref != invoke.description_ref
        || description.input_schema != record["contract"]["input_schema"]
        || !description.connections.iter().any(|connection| {
            connection.connection_ref == invoke.connection_ref
                && connection.provider == metadata.operation.provider()
                && connection.purpose.as_deref() == Some(metadata.auth_profile.as_str())
        })
    {
        return Some(refusal(request_id, RemediationError::Conflict));
    }
    let readiness = state.backend.credential_readiness(owner, target).await;
    if readiness == CredentialReadiness::DependencyUnavailable {
        return Some(refusal(request_id, RemediationError::Unavailable));
    }
    let auth = authentication(&metadata, readiness)?;
    let response = v3::ResponseEnvelope::failure(
        request_id,
        v3::OperationError::authentication_required(auth),
    );
    if response.validate().is_err() {
        return Some(refusal(request_id, RemediationError::Unavailable));
    }
    Some((StatusCode::CONFLICT, Json(response)).into_response())
}

/// Hosted bound acquisition has no production current-publication capability. Still evaluate
/// the real operation grant and acquisition management boundary before reporting Unsupported;
/// neither the personal factory nor a synthetic test backend supplies hosted authority.
pub(super) fn hosted_bound_refusal(
    state: &HostedState,
    principal: &HostedPrincipal,
    owner: &PrincipalContext,
    request: &protocol::connection_v2::ConnectionRequest,
) -> RemediationError {
    use protocol::connection_v2::ConnectionRequest;
    if state.authority.require_remediation_store().is_err() {
        return RemediationError::Unavailable;
    }
    let ConnectionRequest::RemediationStart(start) = request else {
        // No hosted bound owner can currently resolve a stored operation/grant for these
        // session references. Never substitute an unbound v1 session or disclose its status.
        return RemediationError::Refused;
    };
    let target = RemediationTarget {
        operation_ref: &start.operation_ref,
        connection_ref: &start.connection_ref,
    };
    let metadata = match state.backend.remediation_metadata(owner, target) {
        Ok(metadata) => metadata,
        Err(error) => return error,
    };
    let record = match validated_metadata(&metadata, target) {
        Ok(record) => record,
        Err(error) => return error,
    };
    // Deterministic receiver metadata identity, deliberately not a callable description lease.
    let metadata_digest = canonical_input_digest(&serde_json::json!({
        "domain": "remediation-metadata", "operation": record,
        "catalog_generation": metadata.catalog_generation,
        "connection_ref": metadata.connection.id(),
        "initiation": metadata.connection.initiation().iter().collect::<Vec<_>>(),
        "integration_ref": metadata.integration_ref, "auth_profile": metadata.auth_profile,
    }));
    if let Err(error) = state.authority.admit_remediation(
        principal,
        &metadata,
        &record,
        &format!("remediation-metadata:{metadata_digest}"),
        &canonical_input_digest(&start.input),
    ) {
        return match error {
            enforcement::EnforcementRefusal::NotAdmitted => RemediationError::Refused,
            enforcement::EnforcementRefusal::Unavailable => RemediationError::Unavailable,
        };
    }
    let acquisition = protocol::connection::ConnectSessionCreateRequest {
        integration_ref: metadata.integration_ref.clone(),
        auth_profile: Some(metadata.auth_profile.clone()),
        label: "Authentication remediation".into(),
    };
    let self_service = state.backend.connect_session_access(&acquisition)
        == service::ConnectSessionAccess::SelfService;
    if !principal.allows(if self_service {
        "connectors.connections.self"
    } else {
        "connectors.connections.manage"
    }) || (!self_service && !state.policy.admits_operator(principal))
    {
        return RemediationError::Refused;
    }
    if let Err(error) = validate_input(&record, &start.input) {
        return error;
    }
    RemediationError::Unsupported
}

//! One endpoint-to-Connection normalization seam, shared by local and hosted transports.

use crate::{ConnectorBackend, PrincipalContext};
use protocol::operation::{self, v4, OperationError, OperationErrorCode, OperationResult};

/// A normalized request still requires all existing operation admission and freshness checks.
pub struct NormalizedEndpointOperation {
    /// Existing internal request dispatched through the ordinary authorization owner.
    pub request: operation::OperationRequest,
    /// A target-aware description returns only this independently admitted Connection.
    pub description_connection: Option<String>,
}

/// Revalidate endpoint identity and policy before normal operation admission, without credentials.
pub async fn normalize_endpoint_operation<B: ConnectorBackend + ?Sized>(
    backend: &B,
    context: &PrincipalContext,
    request: v4::OperationRequest,
) -> Result<NormalizedEndpointOperation, v4::OperationError> {
    request.validate_target()?;
    let (operation, connection, endpoint, describe) = match &request {
        v4::OperationRequest::Describe(value) => (
            Some(value.operation_ref.as_str()),
            value.connection_ref.as_deref(),
            value.endpoint_ref.as_deref(),
            true,
        ),
        v4::OperationRequest::Invoke(value) => (
            Some(value.operation_ref.as_str()),
            value.connection_ref.as_deref(),
            value.endpoint_ref.as_deref(),
            false,
        ),
        _ => (None, None, None, false),
    };
    let resolved = match endpoint {
        Some(endpoint) => Some(
            backend
                .resolve_endpoint(context, endpoint, operation.expect("target has operation"))
                .await?,
        ),
        None => connection.map(str::to_owned),
    };
    if resolved.as_deref().is_some_and(|value| {
        value.is_empty() || value.len() > 512 || !value.bytes().all(|byte| byte.is_ascii_graphic())
    }) {
        return Err(OperationError::new(
            OperationErrorCode::Protocol,
            "endpoint owner returned an invalid Connection reference",
            false,
        )
        .into());
    }
    Ok(NormalizedEndpointOperation {
        request: request.into_internal(resolved.as_deref())?,
        description_connection: describe.then_some(resolved).flatten(),
    })
}

/// Restrict a description after the existing backend policy selected its admitted Connections.
pub fn constrain_endpoint_description(
    result: OperationResult,
    connection: Option<&str>,
) -> Result<OperationResult, OperationError> {
    let Some(connection) = connection else {
        return Ok(result);
    };
    let OperationResult::Describe(mut description) = result else {
        return Err(OperationError::new(
            OperationErrorCode::Protocol,
            "target description returned an unrelated result",
            false,
        ));
    };
    description
        .connections
        .retain(|value| value.connection_ref == connection);
    if description.connections.len() != 1 {
        return Err(OperationError::new(
            OperationErrorCode::NotGranted,
            "the operation is not admitted for this endpoint",
            false,
        ));
    }
    Ok(OperationResult::Describe(description))
}

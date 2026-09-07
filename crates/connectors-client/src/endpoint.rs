//! Explicit endpoint inventory and v4 operation clients; neither negotiates nor retries versions.

use super::*;
use protocol::{endpoint, operation::v4};

impl LocalClient {
    /// Discover or manage non-secret endpoint bindings through the owner-only daemon socket.
    pub async fn endpoint(
        &self,
        context: &operation::OwnerContext,
        request: endpoint::EndpointRequest,
    ) -> Result<endpoint::ResponseEnvelope, ClientError> {
        let envelope = endpoint_envelope(context, request)?;
        let bytes = serde_json::to_vec(&envelope)?;
        let bytes = self
            .versioned_exchange(bytes, endpoint::MAX_RESULT_BYTES)
            .await?;
        endpoint_response(&bytes, &envelope)
    }

    /// Invoke using one Connection or endpoint reference under the exact v4 contract.
    pub async fn operation_v4(
        &self,
        context: &operation::OwnerContext,
        request: v4::OperationRequest,
    ) -> Result<v4::ResponseEnvelope, ClientError> {
        let envelope = operation_envelope(context, request)?;
        let bytes = self
            .versioned_exchange(serde_json::to_vec(&envelope)?, operation::MAX_RESULT_BYTES)
            .await?;
        operation_response(&bytes, &envelope)
    }
}

impl HostedClient {
    /// Use the same endpoint API against an authenticated hosted server.
    pub async fn endpoint(
        &self,
        bearer: &str,
        context: &operation::OwnerContext,
        request: endpoint::EndpointRequest,
    ) -> Result<endpoint::ResponseEnvelope, ClientError> {
        require_bearer(bearer)?;
        let envelope = endpoint_envelope(context, request)?;
        let (_, bytes) = self
            .versioned_exchange(
                &endpoint(&self.base, "endpoints"),
                bearer,
                serde_json::to_vec(&envelope)?,
                endpoint::MAX_RESULT_BYTES,
                false,
            )
            .await?;
        endpoint_response(&bytes, &envelope)
    }

    /// Select operation v4 exactly; authentication outcomes do not cause operation resends.
    pub async fn operation_v4(
        &self,
        bearer: &str,
        context: &operation::OwnerContext,
        request: v4::OperationRequest,
    ) -> Result<v4::ResponseEnvelope, ClientError> {
        require_bearer(bearer)?;
        let envelope = operation_envelope(context, request)?;
        let (status, bytes) = self
            .versioned_exchange(
                &self.operations,
                bearer,
                serde_json::to_vec(&envelope)?,
                operation::MAX_RESULT_BYTES,
                true,
            )
            .await?;
        let response = operation_response(&bytes, &envelope)?;
        let authentication = response
            .error
            .as_ref()
            .is_some_and(|error| error.code == v4::OperationErrorCode::AuthenticationRequired);
        if (status == reqwest::StatusCode::CONFLICT) != authentication {
            return Err(ClientError::InvalidResponse);
        }
        Ok(response)
    }
}

fn endpoint_envelope(
    context: &operation::OwnerContext,
    request: endpoint::EndpointRequest,
) -> Result<endpoint::RequestEnvelope, ClientError> {
    let envelope = endpoint::RequestEnvelope {
        protocol: endpoint::CONTRACT.into(),
        request_id: request_id(),
        context: context.clone(),
        request,
    };
    envelope
        .validate()
        .map_err(|error| ClientError::InvalidRequest(error.to_string()))?;
    Ok(envelope)
}

fn endpoint_response(
    bytes: &[u8],
    request: &endpoint::RequestEnvelope,
) -> Result<endpoint::ResponseEnvelope, ClientError> {
    let response: endpoint::ResponseEnvelope =
        serde_json::from_slice(bytes).map_err(|_| ClientError::InvalidResponse)?;
    if response.validate().is_err()
        || response.request_id != request.request_id
        || !response.matches_request(&request.request)
    {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

fn operation_envelope(
    context: &operation::OwnerContext,
    request: v4::OperationRequest,
) -> Result<v4::RequestEnvelope, ClientError> {
    let envelope = v4::RequestEnvelope {
        protocol: v4::CONTRACT.into(),
        request_id: request_id(),
        context: context.clone(),
        request,
    };
    envelope
        .validate()
        .map_err(|error| ClientError::InvalidRequest(error.to_string()))?;
    Ok(envelope)
}

fn operation_response(
    bytes: &[u8],
    request: &v4::RequestEnvelope,
) -> Result<v4::ResponseEnvelope, ClientError> {
    let response: v4::ResponseEnvelope =
        serde_json::from_slice(bytes).map_err(|_| ClientError::InvalidResponse)?;
    if response.validate().is_err() || response.request_id != request.request_id {
        return Err(ClientError::InvalidResponse);
    }
    if let Some(auth) = response
        .error
        .as_ref()
        .and_then(|error| error.authentication.as_ref())
    {
        let v4::OperationRequest::Invoke(invoke) = &request.request else {
            return Err(ClientError::InvalidResponse);
        };
        if auth.operation_ref != invoke.operation_ref
            || invoke
                .connection_ref
                .as_ref()
                .is_some_and(|connection| &auth.connection_ref != connection)
        {
            return Err(ClientError::InvalidResponse);
        }
    }
    if let Some(result) = &response.response {
        let matches = match (result, &request.request) {
            (
                operation::OperationResult::Search { operations },
                v4::OperationRequest::Search(request),
            ) => operations.len() <= usize::from(request.limit),
            (
                operation::OperationResult::Describe(result),
                v4::OperationRequest::Describe(request),
            ) => {
                result.operation_ref == request.operation_ref
                    && request.connection_ref.as_ref().is_none_or(|reference| {
                        result.connections.len() == 1
                            && result.connections[0].connection_ref == *reference
                    })
                    && (request.endpoint_ref.is_none() || result.connections.len() == 1)
            }
            (operation::OperationResult::Invoke(result), v4::OperationRequest::Invoke(request)) => {
                result.operation_ref == request.operation_ref
            }
            (
                operation::OperationResult::SessionStatus(result),
                v4::OperationRequest::SessionStatus(request),
            ) => result.execution_ref == request.execution_ref,
            (
                operation::OperationResult::SessionTerminate(result),
                v4::OperationRequest::SessionTerminate(request),
            ) => result.execution_ref == request.execution_ref,
            (
                operation::OperationResult::SessionReconcile(result),
                v4::OperationRequest::SessionReconcile(request),
            ) => result.execution_ref == request.execution_ref,
            (
                operation::OperationResult::SessionSignal(result),
                v4::OperationRequest::SessionSignal(request),
            ) => result.execution_ref == request.execution_ref,
            _ => false,
        };
        if !matches {
            return Err(ClientError::InvalidResponse);
        }
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn owner() -> operation::OwnerContext {
        operation::OwnerContext {
            tenant_id: "tenant".into(),
            agent_id: "agent".into(),
            agent_revision: 1,
            authority_snapshot_id: "snapshot".into(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    #[test]
    fn endpoint_client_refuses_a_valid_response_to_another_method() {
        let request = endpoint_envelope(
            &owner(),
            endpoint::EndpointRequest::List(endpoint::ListRequest {
                source_ref: None,
                query: String::new(),
                limit: 10,
                cursor: None,
            }),
        )
        .unwrap();
        let response = endpoint::ResponseEnvelope::success(
            &request.request_id,
            endpoint::EndpointResult::Refresh {
                endpoints: 0,
                warnings: Vec::new(),
            },
        );
        assert!(response.validate().is_ok());
        assert!(matches!(
            endpoint_response(&serde_json::to_vec(&response).unwrap(), &request),
            Err(ClientError::InvalidResponse)
        ));
    }

    #[test]
    fn endpoint_operation_client_refuses_wrong_operation_and_predecessor_identity() {
        let request = operation_envelope(
            &owner(),
            v4::OperationRequest::Invoke(v4::InvokeRequest {
                operation_ref: "loki-query-range".into(),
                endpoint_ref: Some("endpoint:loki".into()),
                connection_ref: None,
                description_ref: "description:loki".into(),
                input: serde_json::json!({}),
                approval_evidence_ref: None,
            }),
        )
        .unwrap();
        let result = operation::OperationResult::Invoke(operation::InvocationResult {
            operation_ref: "unrelated-operation".into(),
            output: serde_json::json!({}),
            connector_audit_ref: "audit:one".into(),
            execution_ref: None,
        });
        let response = v4::ResponseEnvelope::success(&request.request_id, result);
        assert!(response.validate().is_ok());
        assert!(matches!(
            operation_response(&serde_json::to_vec(&response).unwrap(), &request),
            Err(ClientError::InvalidResponse)
        ));
        let predecessor = response.into_v3();
        assert!(matches!(
            operation_response(&serde_json::to_vec(&predecessor).unwrap(), &request),
            Err(ClientError::InvalidResponse)
        ));
    }
}

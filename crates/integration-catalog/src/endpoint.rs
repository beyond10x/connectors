//! Canonical catalog execution for a target resolved by the shared endpoint plane.
//!
//! The owning backend checks caller and source grants first. Admission here then checks the
//! provider's declared read ceiling and schema before any credential store or transport is used.

use std::collections::BTreeMap;

use catalog::{HostEffect, OperationDirection, Risk};
use connector_secrets::SecretStore;
use protocol::operation::{
    ConnectionSummary, InvocationResult, OperationDescription, OperationError, OperationErrorCode,
    OperationSummary,
};
use service::{EgressHttpRequest, EgressTransport};

use crate::{
    approval_posture, audit_ref, effect_class, incremental_reads, refusal, DeclaredConfig,
    MAX_INPUT_BYTES,
};

/// Read policy derives from catalog traits and remains valid when operations are added.
pub fn read_admitted(operation: &catalog::Operation) -> bool {
    operation.direction == OperationDirection::Read
        && operation.risk != Risk::Destructive
        && operation
            .effects
            .iter()
            .all(|effect| matches!(effect, HostEffect::Read | HostEffect::Network))
}

/// Shared description projection for dynamically resolved HTTP and SQL targets.
pub fn describe(
    provider: &str,
    operation_ref: &str,
    connections: Vec<ConnectionSummary>,
    description_ref: String,
) -> Result<OperationDescription, OperationError> {
    let operation = admitted_operation(provider, operation_ref)?;
    Ok(OperationDescription {
        rate_advice: service::operation_rate_advice(operation),
        operation_ref: operation.id.to_owned(),
        title: operation.id.to_owned(),
        description: operation.description.to_owned(),
        input_schema: incremental_reads::input_schema(operation_ref).unwrap_or_else(|| {
            serde_json::from_str(operation.input_schema).expect("generated catalog schema")
        }),
        output_schema: operation
            .output_schema
            .map(|schema| serde_json::from_str(schema).expect("generated catalog schema"))
            .unwrap_or(serde_json::Value::Null),
        effect: effect_class(operation),
        approval: approval_posture(operation),
        connections,
        description_ref,
    })
}

pub fn summary(
    operation: &catalog::Operation,
    connections: Vec<ConnectionSummary>,
) -> OperationSummary {
    OperationSummary {
        operation_ref: operation.id.to_owned(),
        title: operation.id.to_owned(),
        effect: effect_class(operation),
        approval: approval_posture(operation),
        connections,
    }
}

fn admitted_operation(
    provider: &str,
    operation_ref: &str,
) -> Result<&'static catalog::Operation, OperationError> {
    let operation =
        catalog::operation(catalog::OperationKey::id(operation_ref)).ok_or_else(|| {
            refusal(
                OperationErrorCode::NotFound,
                "the operation is absent from the installed catalog",
            )
        })?;
    if operation.provider != provider || !read_admitted(operation) {
        return Err(refusal(
            OperationErrorCode::NotGranted,
            "endpoint policy admits this provider's read operations only",
        ));
    }
    Ok(operation)
}

/// An HTTP operation whose provider, read traits, and input were validated before secret I/O.
pub struct AdmittedHttpOperation {
    operation: &'static catalog::Operation,
    input: serde_json::Value,
}

/// Validate the catalog contract without resolving credentials or constructing a route.
pub fn admit_http(
    provider: &str,
    operation_ref: &str,
    input: serde_json::Value,
) -> Result<AdmittedHttpOperation, OperationError> {
    let operation = admitted_operation(provider, operation_ref)?;
    if serde_json::to_vec(&input).map_or(true, |bytes| bytes.len() > MAX_INPUT_BYTES) {
        return Err(refusal(
            OperationErrorCode::InvalidInput,
            "operation input exceeds the admitted bound",
        ));
    }
    let schema = incremental_reads::input_schema(operation_ref).unwrap_or_else(|| {
        serde_json::from_str(operation.input_schema).expect("generated catalog schema")
    });
    if !jsonschema::is_valid(&schema, &input) {
        return Err(refusal(
            OperationErrorCode::InvalidInput,
            "operation input does not satisfy its declared schema",
        ));
    }
    incremental_reads::validate_input(operation_ref, &input)?;
    let document = connector_resolve::document::provider(provider).ok_or_else(|| {
        refusal(
            OperationErrorCode::Unavailable,
            "provider request templates are unavailable",
        )
    })?;
    let declared = document.operation(operation_ref).ok_or_else(|| {
        refusal(
            OperationErrorCode::NotFound,
            "the operation has no request template",
        )
    })?;
    if declared.protocol_driver() != connector_resolve::document::ProtocolDriver::HttpV1 {
        return Err(refusal(
            OperationErrorCode::Unavailable,
            "endpoint protocol requires a different installed driver",
        ));
    }
    Ok(AdmittedHttpOperation { operation, input })
}

impl AdmittedHttpOperation {
    /// Use the same declared credential assembler, request resolver, pagination and bounds as
    /// configured catalog Connections; only the admitted origin comes from discovery.
    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        self,
        tenant: &str,
        connection_ref: &str,
        base_url: &str,
        config: &DeclaredConfig,
        secrets: &dyn SecretStore,
        egress: &dyn EgressTransport,
    ) -> Result<InvocationResult, OperationError> {
        let operation = self.operation;
        let provider =
            catalog::provider(catalog::ProviderKey::id(operation.provider)).ok_or_else(|| {
                refusal(
                    OperationErrorCode::Unavailable,
                    "the provider is unavailable",
                )
            })?;
        let document = connector_resolve::document::provider(provider.id).ok_or_else(|| {
            refusal(
                OperationErrorCode::Unavailable,
                "the provider request template is unavailable",
            )
        })?;
        let declared = document.operation(operation.id).ok_or_else(|| {
            refusal(
                OperationErrorCode::NotFound,
                "the operation has no request template",
            )
        })?;
        let assembly = connector_resolve::assemble_credentials(
            operation, provider, tenant, None, secrets, config,
        )
        .await
        .map_err(|_| {
            refusal(
                OperationErrorCode::NotGranted,
                "the bound Secret does not satisfy the operation's declared credentials",
            )
        })?;
        let mut plan = connector_resolve::resolve(
            declared,
            base_url,
            &self.input,
            &BTreeMap::new(),
            &assembly.credentials,
        )
        .map_err(|_| {
            refusal(
                OperationErrorCode::InvalidInput,
                "input does not satisfy the declared request",
            )
        })?;
        incremental_reads::prepare_request(operation.id, &self.input, &mut plan.request)?;
        let request_url = plan.request.url.clone();
        let response = egress
            .execute(
                connection_ref,
                EgressHttpRequest {
                    request: plan.request,
                    maximum_response_bytes: protocol::operation::MAX_RESULT_BYTES,
                    response_headers: incremental_reads::response_headers(operation.id),
                },
            )
            .await
            .map_err(|error| match error {
                service::EgressTransportError::ResponseTooLarge => refusal(
                    OperationErrorCode::ResultTooLarge,
                    "endpoint response exceeded its admitted bound",
                ),
                _ => refusal(
                    OperationErrorCode::Unavailable,
                    "the admitted endpoint route could not complete the request",
                ),
            })?;
        if response.status == 429 {
            return Err(OperationError::rate_limited(
                "endpoint rate limit was reached",
                service::retry_after_seconds(response.header("retry-after")),
            ));
        }
        if !response.is_success() {
            return Err(refusal(
                OperationErrorCode::Unavailable,
                format!("endpoint returned HTTP {}", response.status),
            ));
        }
        let output = if incremental_reads::handles(operation.id) {
            incremental_reads::project(operation.id, &self.input, &request_url, response)?
        } else {
            serde_json::from_slice(&response.body).unwrap_or_else(|_| {
                serde_json::Value::String(String::from_utf8_lossy(&response.body).into_owned())
            })
        };
        // Providers can echo a supplied token in successful JSON too. Refuse such output rather
        // than changing the provider's declared schema with arbitrary replacement strings.
        let serialized = serde_json::to_string(&output).map_err(|_| {
            refusal(
                OperationErrorCode::Unavailable,
                "endpoint output is unavailable",
            )
        })?;
        if assembly.redactions.iter().any(|redaction| {
            !redaction.expose().is_empty() && serialized.contains(redaction.expose())
        }) {
            return Err(refusal(
                OperationErrorCode::Unavailable,
                "endpoint output contains credential material",
            ));
        }
        Ok(InvocationResult {
            operation_ref: operation.id.to_owned(),
            output,
            connector_audit_ref: audit_ref(operation.id, connection_ref),
            execution_ref: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_admission_refuses_writes_other_providers_and_invalid_input_without_io() {
        let read = catalog::provider(catalog::ProviderKey::id("asterisk"))
            .unwrap()
            .operations
            .iter()
            .find(|operation| read_admitted(operation))
            .unwrap();
        assert!(admit_http("loki", read.id, serde_json::json!({})).is_err());
        assert!(admit_http("asterisk", read.id, serde_json::json!(["invalid"])).is_err());
        for operation in catalog::provider(catalog::ProviderKey::id("asterisk"))
            .unwrap()
            .operations
        {
            if !read_admitted(operation) {
                assert!(admit_http("asterisk", operation.id, serde_json::json!({})).is_err());
            }
        }
    }
}

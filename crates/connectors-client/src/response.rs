//! Correlation and closed-envelope validation for Connector client responses.

use protocol::{catalog, connection, datasource, event, operation};

use crate::ClientError;

pub(crate) fn validate_versioned_operation_response(
    bytes: &[u8],
    expected_version: operation::versions::Version,
    request_id: &str,
    request: &operation::OperationRequest,
) -> Result<operation::v3::ResponseEnvelope, ClientError> {
    let (version, response) =
        operation::versions::decode_response(bytes).map_err(|_| ClientError::InvalidResponse)?;
    if version != expected_version || response.request_id != request_id {
        return Err(ClientError::InvalidResponse);
    }
    if let Some(auth) = response
        .error
        .as_ref()
        .and_then(|error| error.authentication.as_ref())
    {
        let operation::OperationRequest::Invoke(invoke) = request else {
            return Err(ClientError::InvalidResponse);
        };
        if auth.operation_ref != invoke.operation_ref
            || auth.connection_ref != invoke.connection_ref
        {
            return Err(ClientError::InvalidResponse);
        }
    }
    Ok(response)
}

pub(crate) fn validate_connection_v2_response(
    bytes: &[u8],
    request_id: &str,
) -> Result<protocol::connection_v2::ResponseEnvelope, ClientError> {
    let (version, response) = protocol::connection_v2::decode_response(bytes)
        .map_err(|_| ClientError::InvalidResponse)?;
    if version != protocol::connection_v2::Version::V0Alpha2 || response.request_id != request_id {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

pub(crate) fn validate_catalog_response(
    response: catalog::ResponseEnvelope,
    request_id: &str,
) -> Result<catalog::ResponseEnvelope, ClientError> {
    if response.request_id != request_id || response.validate().is_err() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

pub(crate) fn validate_operation_response(
    response: operation::ResponseEnvelope,
    request_id: &str,
) -> Result<operation::ResponseEnvelope, ClientError> {
    if response.request_id != request_id || response.validate().is_err() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

pub(crate) fn validate_connection_response(
    response: connection::ResponseEnvelope,
    request_id: &str,
) -> Result<connection::ResponseEnvelope, ClientError> {
    if response.request_id != request_id || response.validate().is_err() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

pub(crate) fn validate_event_response(
    response: event::ResponseEnvelope,
    request_id: &str,
) -> Result<event::ResponseEnvelope, ClientError> {
    if response.request_id != request_id || response.validate().is_err() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

pub(crate) fn validate_datasource_response(
    response: datasource::ResponseEnvelope,
    request_id: &str,
) -> Result<datasource::ResponseEnvelope, ClientError> {
    if response.request_id != request_id || response.validate().is_err() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HostedClient, LocalClient};
    use serde_json::{json, Value};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _};

    fn owner() -> operation::OwnerContext {
        operation::OwnerContext {
            tenant_id: "fixture".to_owned(),
            agent_id: "fixture".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "fixture".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    fn invoke() -> operation::OperationRequest {
        operation::OperationRequest::Invoke(operation::InvokeRequest {
            operation_ref: "fixture.write".to_owned(),
            connection_ref: "connection:fixture".to_owned(),
            description_ref: "description:fixture".to_owned(),
            input: json!({}),
            approval_evidence_ref: None,
        })
    }

    fn reply(case: &str, request: &Value) -> String {
        assert_eq!(request["protocol"], operation::CONTRACT);
        assert_eq!(request["protocol"], "b10x.connector-operation.v0alpha2");
        assert_eq!(request["request"]["method"], "invoke");
        if case == "malformed" {
            return "{".to_owned();
        }
        let mut response = json!({"protocol":operation::CONTRACT,"request_id":request["request_id"],
            "status":"error","error":{"code":"rate_limited","message":"provider refusal","retriable":true,"retry_after_seconds":30}});
        match case {
            "v1" => response["protocol"] = json!(operation::legacy::CONTRACT),
            "unknown-version" => response["protocol"] = json!("b10x.connector-operation.v99"),
            "wrong-correlation" => response["request_id"] = json!("another-request"),
            "rate" => {}
            _ => unreachable!(),
        }
        response.to_string()
    }

    fn check(case: &str, result: Result<operation::ResponseEnvelope, ClientError>) {
        if case == "rate" {
            let error = result.unwrap().error.unwrap();
            assert_eq!(error.code, operation::OperationErrorCode::RateLimited);
            assert!(error.retriable);
            assert_eq!(error.retry_after_seconds, Some(30));
        } else {
            assert!(result.is_err(), "{case} must not be accepted");
        }
    }

    #[tokio::test]
    async fn rate_stage2_local_client_never_resends_after_any_received_refusal_or_invalid_reply() {
        for case in [
            "rate",
            "v1",
            "unknown-version",
            "wrong-correlation",
            "malformed",
        ] {
            let root = tempfile::tempdir().unwrap();
            let socket = root.path().join("fixture.sock");
            let listener = tokio::net::UnixListener::bind(&socket).unwrap();
            let serving = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut line = String::new();
                tokio::io::BufReader::new(&mut stream)
                    .read_line(&mut line)
                    .await
                    .unwrap();
                let response = reply(case, &serde_json::from_str(&line).unwrap());
                stream
                    .write_all(format!("{response}\n").as_bytes())
                    .await
                    .unwrap();
                drop(stream);
                assert!(tokio::time::timeout(
                    std::time::Duration::from_millis(50),
                    listener.accept()
                )
                .await
                .is_err());
            });
            check(
                case,
                LocalClient::new(&socket)
                    .operation_v2(&owner(), invoke())
                    .await,
            );
            serving.await.unwrap();
        }
    }

    #[tokio::test]
    async fn rate_stage2_hosted_client_never_resends_after_any_received_refusal_or_invalid_reply() {
        for case in [
            "rate",
            "v1",
            "unknown-version",
            "wrong-correlation",
            "malformed",
        ] {
            let calls = Arc::new(AtomicUsize::new(0));
            let observed = calls.clone();
            let app = axum::Router::new().route(
                "/operations",
                axum::routing::post(move |body: axum::body::Bytes| {
                    let observed = observed.clone();
                    async move {
                        observed.fetch_add(1, Ordering::SeqCst);
                        reply(case, &serde_json::from_slice(&body).unwrap())
                    }
                }),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url =
                url::Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
            let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
            let client = HostedClient::from_parts(url, reqwest::Client::new());
            check(
                case,
                client
                    .operation_v2("fixture-session", &owner(), invoke())
                    .await,
            );
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            serving.abort();
        }
    }
}

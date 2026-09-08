use crate::credentials::CredentialRef;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response as HttpResponse},
    routing::{get, post},
};
use connectors_core::{
    Descriptor, Error, ErrorCode, Invocation, Outcome, REQUEST_LIMIT, RESPONSE_LIMIT, Response,
    Result, WIRE_VERSION,
};
use connectors_sdk::{Adapter, Credential, validate};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::Instant};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServiceConfig {
    #[schemars(length(min = 1, max = 128), regex(pattern = "^[A-Za-z0-9_.-]+$"))]
    pub instance: String,
    #[schemars(with = "String", length(min = 1, max = 512))]
    pub listen: SocketAddr,
    pub service_credential: CredentialRef,
}

#[derive(Clone)]
struct Service {
    adapter: Arc<dyn Adapter>,
    credential: Arc<dyn Credential>,
    slots: Arc<Semaphore>,
}

pub fn router(adapter: Arc<dyn Adapter>, credential: Arc<dyn Credential>) -> Router {
    Router::new()
        .route("/healthz", get(|| async { StatusCode::NO_CONTENT }))
        .route("/v1/describe", get(describe))
        .route("/v1/invoke", post(invoke))
        .layer(DefaultBodyLimit::max(REQUEST_LIMIT))
        .with_state(Service {
            adapter,
            credential,
            slots: Arc::new(Semaphore::new(32)),
        })
}

async fn admitted(service: &Service, headers: &HeaderMap) -> Result<()> {
    let presented = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| Error::new(ErrorCode::Unauthorized, "service authentication required"))?;
    let expected = service.credential.resolve().await?;
    if !bool::from(expected.0.as_slice().ct_eq(presented.as_bytes())) {
        return Err(Error::new(
            ErrorCode::Unauthorized,
            "service authentication failed",
        ));
    }
    Ok(())
}

async fn describe(State(service): State<Service>, headers: HeaderMap) -> HttpResponse {
    if let Err(error) = admitted(&service, &headers).await {
        return error_http(error);
    }
    Json(service.adapter.descriptor()).into_response()
}

async fn invoke(
    State(service): State<Service>,
    headers: HeaderMap,
    body: std::result::Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> HttpResponse {
    if let Err(error) = admitted(&service, &headers).await {
        return error_http(error);
    }
    let bytes = match body {
        Ok(v) => v,
        Err(_) => {
            return error_http(Error::new(
                ErrorCode::Capacity,
                "request exceeds byte limit",
            ));
        }
    };
    let invocation: Invocation = match connectors_core::read_json(&bytes) {
        Ok(v) => v,
        Err(_) => return error_http(Error::invalid("invalid invocation envelope")),
    };
    let started = Instant::now();
    let result = execute(&service, &invocation).await;
    tracing::info!(request_id = ?invocation.request_id, operation = ?invocation.operation, elapsed_ms = started.elapsed().as_millis(), success = result.is_ok(), "operation completed");
    let status = result.as_ref().err().map(status).unwrap_or(StatusCode::OK);
    let response = Response {
        version: WIRE_VERSION.into(),
        request_id: invocation.request_id,
        outcome: match result {
            Ok(result) => Outcome::Success { result },
            Err(error) => Outcome::Error { error },
        },
    };
    (status, Json(response)).into_response()
}

async fn execute(service: &Service, request: &Invocation) -> Result<serde_json::Value> {
    if request.version != WIRE_VERSION {
        return Err(Error::new(
            ErrorCode::Unsupported,
            "unsupported request version",
        ));
    }
    if !connectors_core::valid_id(&request.request_id)
        || !connectors_core::valid_id(&request.operation)
    {
        return Err(Error::invalid("invalid request identifier"));
    }
    let descriptor: Descriptor = service.adapter.descriptor();
    if request.revision != descriptor.revision {
        return Err(Error::new(
            ErrorCode::StaleDescription,
            "refresh the descriptor before resubmitting",
        ));
    }
    let operation = descriptor.operation(&request.operation)?;
    validate(&operation.input_schema, &request.input)?;
    let _permit = service
        .slots
        .try_acquire()
        .map_err(|_| Error::new(ErrorCode::Capacity, "service concurrency limit reached"))?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        service
            .adapter
            .invoke_at(&request.revision, &request.operation, request.input.clone()),
    )
    .await
    .map_err(|_| Error::new(ErrorCode::Timeout, "operation deadline exceeded"))??;
    validate(&operation.output_schema, &result).map_err(|_| {
        Error::new(
            ErrorCode::UpstreamProtocol,
            "result does not match the declared output schema",
        )
    })?;
    if serde_json::to_vec(&result)
        .map_err(|_| Error::internal())?
        .len()
        > RESPONSE_LIMIT - 1024
    {
        return Err(Error::new(ErrorCode::Capacity, "result exceeds byte limit"));
    }
    Ok(result)
}

fn status(error: &Error) -> StatusCode {
    match error.code {
        ErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
        ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
        ErrorCode::Forbidden => StatusCode::FORBIDDEN,
        ErrorCode::NotFound => StatusCode::NOT_FOUND,
        ErrorCode::StaleDescription | ErrorCode::StaleCursor => StatusCode::CONFLICT,
        ErrorCode::Unsupported => StatusCode::NOT_IMPLEMENTED,
        ErrorCode::Capacity | ErrorCode::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        ErrorCode::Timeout => StatusCode::GATEWAY_TIMEOUT,
        ErrorCode::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        ErrorCode::UpstreamProtocol => StatusCode::BAD_GATEWAY,
        ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
fn error_http(error: Error) -> HttpResponse {
    (status(&error), Json(error)).into_response()
}

pub async fn serve(config: ServiceConfig, adapter: Arc<dyn Adapter>) -> Result<()> {
    config.service_credential.resolve().await?;
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .map_err(|_| Error::invalid("service listener cannot be bound"))?;
    tracing::info!(instance=%config.instance, listen=%listener.local_addr().map_err(|_|Error::internal())?, "adapter service listening");
    axum::serve(
        listener,
        router(adapter, Arc::new(config.service_credential)),
    )
    .with_graceful_shutdown(async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! { _=tokio::signal::ctrl_c()=>{}, _=terminate.recv()=>{} }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    })
    .await
    .map_err(|_| Error::internal())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    struct BoundedAdapter {
        entered: Arc<Semaphore>,
        result: Option<String>,
    }
    #[async_trait::async_trait]
    impl Adapter for BoundedAdapter {
        fn descriptor(&self) -> Descriptor {
            Descriptor {
                version: WIRE_VERSION.into(),
                instance: "bounds".into(),
                adapter: "fixture".into(),
                revision: "one".into(),
                configuration_schema: json!({"type":"object"}),
                operations: vec![connectors_core::Operation {
                    id: "read".into(),
                    description: "bounded read".into(),
                    contract: "operations/v1alpha1".into(),
                    profile: "read".into(),
                    input_schema: json!({"type":"object"}),
                    output_schema: json!({"type":"string"}),
                }],
            }
        }
        async fn invoke(&self, _: &str, _: Value) -> Result<Value> {
            self.entered.add_permits(1);
            match &self.result {
                Some(value) => Ok(json!(value)),
                None => std::future::pending().await,
            }
        }
    }
    fn fixture(result: Option<String>) -> (Service, Arc<Semaphore>) {
        let entered = Arc::new(Semaphore::new(0));
        (
            Service {
                adapter: Arc::new(BoundedAdapter {
                    entered: entered.clone(),
                    result,
                }),
                credential: Arc::new(CredentialRef::Environment {
                    name: "UNUSED".into(),
                }),
                slots: Arc::new(Semaphore::new(32)),
            },
            entered,
        )
    }
    fn invocation() -> Invocation {
        Invocation {
            version: WIRE_VERSION.into(),
            request_id: "bounds".into(),
            operation: "read".into(),
            revision: "one".into(),
            input: json!({}),
        }
    }
    #[tokio::test(start_paused = true)]
    async fn service_deadline_cancels_the_adapter_and_releases_capacity() {
        let (service, entered) = fixture(None);
        let start = tokio::time::Instant::now();
        assert_eq!(
            execute(&service, &invocation()).await.unwrap_err().code,
            ErrorCode::Timeout
        );
        assert_eq!(start.elapsed(), std::time::Duration::from_secs(20));
        assert_eq!(entered.available_permits(), 1);
        assert_eq!(service.slots.available_permits(), 32);
    }
    #[tokio::test]
    async fn thirty_two_inflight_calls_refuse_the_next_until_one_is_dropped() {
        let (service, entered) = fixture(None);
        let spawn = || {
            let service = service.clone();
            tokio::spawn(async move { execute(&service, &invocation()).await })
        };
        let mut calls = (0..32).map(|_| spawn()).collect::<Vec<_>>();
        entered.acquire_many(32).await.unwrap().forget();
        assert_eq!(
            execute(&service, &invocation()).await.unwrap_err().code,
            ErrorCode::Capacity
        );
        assert_eq!(entered.available_permits(), 0);
        let first = calls.remove(0);
        first.abort();
        assert!(first.await.unwrap_err().is_cancelled());
        calls.push(spawn());
        entered.acquire().await.unwrap().forget();
        for call in calls {
            call.abort();
            let _ = call.await;
        }
        assert_eq!(service.slots.available_permits(), 32);
    }
    #[tokio::test]
    async fn oversized_adapter_result_is_refused_after_schema_validation() {
        let (service, _) = fixture(Some("x".repeat(RESPONSE_LIMIT)));
        assert_eq!(
            execute(&service, &invocation()).await.unwrap_err().code,
            ErrorCode::Capacity
        );
        let (service, _) = fixture(Some("small".into()));
        assert_eq!(
            execute(&service, &invocation()).await.unwrap(),
            json!("small")
        );
    }
}

//! Identity-authenticated Git fetch control and internal-only Smart HTTP bytes.

use std::sync::Arc;
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::extract::{DefaultBodyLimit, Path, RawQuery, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse as _, Json, Response};
use axum::routing::{get, post};
use axum::Router;
use futures_util::stream;
use protocol::git_fetch::{CreatedSession, RequestEnvelope};
use serde::Serialize;
use service::{
    GitFetchAccess, GitFetchBroker, GitFetchControlError, GitFetchDataError, GitFetchExchange,
    GitFetchService,
};
use zeroize::Zeroizing;

use super::{bearer, IdentityVerificationError, IdentityVerifier, CONNECTORS_AUDIENCE};

const SOURCE_AUTHORIZATION: &str = "x-b10x-git-source-authorization";
const MAX_UPLOAD_PACK_REQUEST_BYTES: usize = 256 * 1024;
const MAX_REQUEST_HEADER_BYTES: usize = 16 * 1024;
const MAX_REQUEST_HEADERS: usize = 64;
const BODY_READ_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
struct ControlState {
    verifier: Arc<dyn IdentityVerifier>,
    broker: Arc<dyn GitFetchBroker>,
}

#[derive(Clone)]
struct DataState {
    broker: Arc<dyn GitFetchBroker>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ErrorBody {
    error: &'static str,
}

/// Build the public control route relative to the configured hosted API base.
pub fn git_fetch_control_router(
    verifier: Arc<dyn IdentityVerifier>,
    broker: Arc<dyn GitFetchBroker>,
) -> Router {
    Router::new()
        .route(
            "/git-fetch-sessions",
            post(create).layer(DefaultBodyLimit::max(protocol::git_fetch::MAX_FRAME_BYTES)),
        )
        .with_state(ControlState { verifier, broker })
}

/// Build the internal byte plane at its absolute, non-public prefix.
pub fn git_fetch_internal_router(broker: Arc<dyn GitFetchBroker>) -> Router {
    Router::new()
        .route(
            "/internal/git-fetch/{session_ref}/{repository}/info/refs",
            get(discover),
        )
        .route(
            "/internal/git-fetch/{session_ref}/{repository}/git-upload-pack",
            post(upload_pack).layer(DefaultBodyLimit::max(MAX_UPLOAD_PACK_REQUEST_BYTES)),
        )
        .with_state(DataState { broker })
}

async fn create(State(state): State<ControlState>, request: Request) -> Response {
    let headers = request.headers();
    if !headers_within_bound(headers)
        || headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            != Some("application/json")
    {
        return refusal(StatusCode::BAD_REQUEST, "git-fetch-request-invalid");
    }
    let Some(credential) = bearer(headers) else {
        return refusal(StatusCode::UNAUTHORIZED, "identity-access-token-required");
    };
    let principal = match state.verifier.verify(credential, CONNECTORS_AUDIENCE).await {
        Ok(principal) => principal,
        Err(IdentityVerificationError::Refused) => {
            return refusal(StatusCode::UNAUTHORIZED, "identity-access-token-refused");
        }
        Err(IdentityVerificationError::Unavailable) => {
            return refusal(StatusCode::SERVICE_UNAVAILABLE, "identity-unavailable");
        }
    };
    if !principal.allows("connectors.catalog.read") {
        return refusal(StatusCode::FORBIDDEN, "git-fetch-not-granted");
    }
    let body = match tokio::time::timeout(
        BODY_READ_TIMEOUT,
        to_bytes(request.into_body(), protocol::git_fetch::MAX_FRAME_BYTES),
    )
    .await
    {
        Ok(Ok(body)) => body,
        Ok(Err(_)) => return refusal(StatusCode::PAYLOAD_TOO_LARGE, "git-fetch-request-invalid"),
        Err(_) => return refusal(StatusCode::REQUEST_TIMEOUT, "git-fetch-request-invalid"),
    };
    let request: RequestEnvelope = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(_) => return refusal(StatusCode::BAD_REQUEST, "git-fetch-request-invalid"),
    };
    if !request.is_valid() {
        return refusal(StatusCode::BAD_REQUEST, "git-fetch-request-invalid");
    }
    if principal.tenant_id != request.context.tenant_id {
        return refusal(StatusCode::FORBIDDEN, "git-fetch-not-granted");
    }
    let context = match principal.principal_context(&request.request_id) {
        Ok(context) => context,
        Err(_) => {
            return refusal(StatusCode::UNAUTHORIZED, "identity-access-token-refused");
        }
    };
    match state.broker.create(&context, request.request).await {
        Ok(grant) => {
            let source_authorization = grant.expose_at_control_boundary().to_owned();
            confidential_json(
                StatusCode::CREATED,
                &CreatedSession {
                    protocol: protocol::git_fetch::CONTRACT.to_owned(),
                    request_id: request.request_id,
                    session_ref: grant.session_ref,
                    source: grant.source,
                    locator: grant.locator,
                    reference: grant.reference,
                    expected_commit: grant.expected_commit,
                    depth: grant.depth,
                    expires_at_unix_ms: grant.expires_at_unix_ms,
                    source_authorization,
                },
            )
        }
        Err(GitFetchControlError::Invalid) => {
            refusal(StatusCode::BAD_REQUEST, "git-fetch-request-invalid")
        }
        Err(GitFetchControlError::NotGranted) => {
            refusal(StatusCode::FORBIDDEN, "git-fetch-not-granted")
        }
        Err(GitFetchControlError::Conflict) => {
            refusal(StatusCode::CONFLICT, "git-fetch-replay-conflict")
        }
        Err(GitFetchControlError::Unavailable) => {
            refusal(StatusCode::SERVICE_UNAVAILABLE, "git-fetch-unavailable")
        }
    }
}

async fn discover(
    State(state): State<DataState>,
    Path((session_ref, repository)): Path<(String, String)>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> Response {
    if !headers_within_bound(&headers) || query.as_deref() != Some("service=git-upload-pack") {
        return refusal(StatusCode::NOT_FOUND, "git-fetch-refused");
    }
    let access = match source_access(
        session_ref,
        repository,
        &headers,
        GitFetchService::Discovery,
    ) {
        Ok(access) => access,
        Err(()) => return refusal(StatusCode::NOT_FOUND, "git-fetch-refused"),
    };
    exchange(&state, access, None).await
}

async fn upload_pack(
    State(state): State<DataState>,
    Path((session_ref, repository)): Path<(String, String)>,
    request: Request,
) -> Response {
    if !headers_within_bound(request.headers())
        || request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            != Some("application/x-git-upload-pack-request")
    {
        return refusal(StatusCode::NOT_FOUND, "git-fetch-refused");
    }
    let access = match source_access(
        session_ref,
        repository,
        request.headers(),
        GitFetchService::UploadPack,
    ) {
        Ok(access) => access,
        Err(()) => return refusal(StatusCode::NOT_FOUND, "git-fetch-refused"),
    };
    if state.broker.authorize(&access).is_err() {
        return refusal(StatusCode::NOT_FOUND, "git-fetch-refused");
    }
    let body = match tokio::time::timeout(
        BODY_READ_TIMEOUT,
        to_bytes(request.into_body(), MAX_UPLOAD_PACK_REQUEST_BYTES),
    )
    .await
    {
        Ok(Ok(body)) if !body.is_empty() => body,
        Ok(Ok(_)) => return refusal(StatusCode::NOT_FOUND, "git-fetch-refused"),
        Ok(Err(_)) => return refusal(StatusCode::PAYLOAD_TOO_LARGE, "git-fetch-refused"),
        Err(_) => return refusal(StatusCode::REQUEST_TIMEOUT, "git-fetch-refused"),
    };
    exchange(&state, access, Some(body.to_vec())).await
}

async fn exchange(state: &DataState, access: GitFetchAccess, body: Option<Vec<u8>>) -> Response {
    if state.broker.authorize(&access).is_err() {
        return refusal(StatusCode::NOT_FOUND, "git-fetch-refused");
    }
    let service = access.service;
    let response = state
        .broker
        .exchange(GitFetchExchange {
            session_ref: access.session_ref,
            repository: access.repository,
            source_authorization: access.source_authorization,
            service,
            git_protocol: access.git_protocol,
            body,
        })
        .await;
    let response = match response {
        Ok(response) => response,
        Err(GitFetchDataError::Refused) => {
            return refusal(StatusCode::NOT_FOUND, "git-fetch-refused");
        }
        Err(GitFetchDataError::Unavailable) => {
            return refusal(StatusCode::BAD_GATEWAY, "git-fetch-unavailable");
        }
    };
    let expected_content_type = match service {
        GitFetchService::Discovery => "application/x-git-upload-pack-advertisement",
        GitFetchService::UploadPack => "application/x-git-upload-pack-result",
    };
    if response.status != 200 || response.content_type != expected_content_type {
        return refusal(StatusCode::BAD_GATEWAY, "git-fetch-unavailable");
    }
    let body = stream::unfold(response.body, |mut stream| async move {
        match stream.next_chunk().await {
            Ok(Some(bytes)) => Some((Ok::<_, std::io::Error>(bytes), stream)),
            Ok(None) => None,
            Err(_) => Some((
                Err(std::io::Error::other("git fetch stream refused")),
                stream,
            )),
        }
    });
    let mut response = Response::new(Body::from_stream(body));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(expected_content_type),
    );
    no_store(&mut response);
    response
}

fn source_access(
    session_ref: String,
    repository: String,
    headers: &HeaderMap,
    service: GitFetchService,
) -> Result<GitFetchAccess, ()> {
    let protocols = headers.get_all("git-protocol").iter().collect::<Vec<_>>();
    let git_protocol = match protocols.as_slice() {
        [] => None,
        [value] if value.as_bytes() == b"version=2" => Some("version=2".to_owned()),
        _ => return Err(()),
    };
    if headers.get_all(SOURCE_AUTHORIZATION).iter().count() != 1 {
        return Err(());
    }
    let authorization = headers
        .get(SOURCE_AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            (32..=256).contains(&value.len())
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        });
    let authorization = authorization.ok_or(())?;
    Ok(GitFetchAccess {
        session_ref,
        repository,
        source_authorization: Zeroizing::new(authorization.to_owned()),
        service,
        git_protocol,
    })
}

fn headers_within_bound(headers: &HeaderMap) -> bool {
    headers.len() <= MAX_REQUEST_HEADERS
        && headers
            .iter()
            .try_fold(0_usize, |observed, (name, value)| {
                observed
                    .checked_add(name.as_str().len())?
                    .checked_add(value.as_bytes().len())
            })
            .is_some_and(|observed| observed <= MAX_REQUEST_HEADER_BYTES)
}

fn refusal(status: StatusCode, code: &'static str) -> Response {
    let mut response = (status, Json(ErrorBody { error: code })).into_response();
    no_store(&mut response);
    response
}

fn confidential_json<T: Serialize>(status: StatusCode, value: &T) -> Response {
    let mut response = (status, Json(value)).into_response();
    no_store(&mut response);
    response
}

fn no_store(response: &mut Response) {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
}

#[cfg(test)]
mod tests {
    use axum::body::Bytes;
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicBool, Ordering};

    use async_trait::async_trait;
    use axum::body::to_bytes;
    use axum::http::Request;
    use service::{
        EgressByteStream, EgressTransportError, GitFetchExchangeResponse, GitFetchGrant,
    };
    use tower::ServiceExt as _;

    use super::*;
    use crate::hosted::HostedPrincipal;

    struct Verifier;

    #[async_trait]
    impl IdentityVerifier for Verifier {
        async fn ready(&self) -> Result<(), IdentityVerificationError> {
            Ok(())
        }

        async fn verify(
            &self,
            _credential: &str,
            _audience: &str,
        ) -> Result<HostedPrincipal, IdentityVerificationError> {
            Ok(HostedPrincipal {
                issuer: "https://identity.example.test".to_owned(),
                tenant_id: "tenant-one".to_owned(),
                subject: "person:owner".to_owned(),
                actor_subject: "person:owner".to_owned(),
                email: Some("owner@example.test".to_owned()),
                token_id: "token-one".to_owned(),
                scopes: BTreeSet::from(["connectors.catalog.read".to_owned()]),
                groups: BTreeSet::new(),
                authority_snapshot_sha256: "a".repeat(64),
                deployment_id: None,
            })
        }
    }

    struct RefusingVerifier;

    #[async_trait]
    impl IdentityVerifier for RefusingVerifier {
        async fn ready(&self) -> Result<(), IdentityVerificationError> {
            Ok(())
        }

        async fn verify(
            &self,
            _credential: &str,
            _audience: &str,
        ) -> Result<HostedPrincipal, IdentityVerificationError> {
            Err(IdentityVerificationError::Refused)
        }
    }

    struct OneChunk(Option<Vec<u8>>);

    #[async_trait]
    impl EgressByteStream for OneChunk {
        async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
            Ok(self.0.take())
        }
    }

    struct Broker;

    #[async_trait]
    impl GitFetchBroker for Broker {
        async fn create(
            &self,
            _context: &service::PrincipalContext,
            request: protocol::git_fetch::CreateRequest,
        ) -> Result<GitFetchGrant, GitFetchControlError> {
            Ok(GitFetchGrant::admitted(
                "git-fetch:one".to_owned(),
                "gitlab".to_owned(),
                "https://connectors.internal/internal/git-fetch/git-fetch:one/repository.git"
                    .to_owned(),
                &request,
                2_000_000_000_000,
                Zeroizing::new("x".repeat(43)),
            ))
        }

        fn authorize(&self, request: &GitFetchAccess) -> Result<(), GitFetchDataError> {
            if request.source_authorization.as_str() == "x".repeat(43) {
                Ok(())
            } else {
                Err(GitFetchDataError::Refused)
            }
        }

        async fn exchange(
            &self,
            request: GitFetchExchange,
        ) -> Result<GitFetchExchangeResponse, GitFetchDataError> {
            if request.source_authorization.as_str() != "x".repeat(43) {
                return Err(GitFetchDataError::Refused);
            }
            Ok(GitFetchExchangeResponse {
                status: 200,
                content_type: "application/x-git-upload-pack-advertisement".to_owned(),
                body: Box::new(OneChunk(Some(b"git-packet".to_vec()))),
            })
        }
    }

    fn request_envelope() -> RequestEnvelope {
        RequestEnvelope {
            protocol: protocol::git_fetch::CONTRACT.to_owned(),
            request_id: "request-one".to_owned(),
            context: protocol::operation::OwnerContext {
                tenant_id: "tenant-one".to_owned(),
                agent_id: "workspace".to_owned(),
                agent_revision: 1,
                authority_snapshot_id: "snapshot-one".to_owned(),
                authority_snapshot_sha256: "b".repeat(64),
            },
            request: protocol::git_fetch::CreateRequest {
                idempotency_key: "coding-session-one".to_owned(),
                connection_ref: "connection:gitlab:one".to_owned(),
                project_id: 42,
                reference: "trunk".to_owned(),
                expected_commit: "c".repeat(40),
                depth: 10,
            },
        }
    }

    #[tokio::test]
    async fn control_and_internal_routes_are_separate_and_non_cacheable() {
        let broker: Arc<dyn GitFetchBroker> = Arc::new(Broker);
        let control = git_fetch_control_router(Arc::new(Verifier), broker.clone());
        let response = control
            .clone()
            .oneshot(
                Request::post("/git-fetch-sessions")
                    .header(header::AUTHORIZATION, "Bearer identity-token")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&request_envelope()).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let created: CreatedSession = serde_json::from_slice(
            &to_bytes(
                response.into_body(),
                protocol::git_fetch::MAX_RESPONSE_BYTES,
            )
            .await
            .unwrap(),
        )
        .unwrap();
        assert!(created.is_valid("request-one"));
        assert_eq!(
            control
                .oneshot(
                    Request::get(
                        "/internal/git-fetch/git-fetch:one/repository.git/info/refs?service=git-upload-pack",
                    )
                    .body(Body::empty())
                    .unwrap(),
                )
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );

        let internal = git_fetch_internal_router(broker);
        let refused = internal
            .clone()
            .oneshot(
                Request::get(
                    "/internal/git-fetch/git-fetch:one/repository.git/info/refs?service=git-upload-pack",
                )
                .header(header::AUTHORIZATION, "Bearer identity-token")
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::NOT_FOUND);
        assert_eq!(refused.headers()[header::CACHE_CONTROL], "no-store");

        let accepted = internal
            .oneshot(
                Request::get(
                    "/internal/git-fetch/git-fetch:one/repository.git/info/refs?service=git-upload-pack",
                )
                .header(SOURCE_AUTHORIZATION, "x".repeat(43))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(accepted.status(), StatusCode::OK);
        assert_eq!(accepted.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(
            to_bytes(accepted.into_body(), 128).await.unwrap(),
            Bytes::from_static(b"git-packet")
        );
    }

    #[tokio::test]
    async fn rejected_control_identity_is_decided_before_the_request_body_is_polled() {
        let polled = Arc::new(AtomicBool::new(false));
        let observed = polled.clone();
        let body = Body::from_stream(stream::once(async move {
            observed.store(true, Ordering::SeqCst);
            Ok::<_, std::io::Error>(Bytes::from_static(b"not-json"))
        }));
        let response = git_fetch_control_router(Arc::new(RefusingVerifier), Arc::new(Broker))
            .oneshot(
                Request::post("/git-fetch-sessions")
                    .header(header::AUTHORIZATION, "Bearer refused")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(!polled.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn rejected_source_authority_is_decided_before_body_or_broker_exchange() {
        let polled = Arc::new(AtomicBool::new(false));
        let observed = polled.clone();
        let body = Body::from_stream(stream::once(async move {
            observed.store(true, Ordering::SeqCst);
            Ok::<_, std::io::Error>(Bytes::from_static(b"request"))
        }));
        let internal = git_fetch_internal_router(Arc::new(Broker));
        let refused = internal
            .clone()
            .oneshot(
                Request::post("/internal/git-fetch/git-fetch:one/repository.git/git-upload-pack")
                    .header(
                        header::CONTENT_TYPE,
                        "application/x-git-upload-pack-request",
                    )
                    .header(SOURCE_AUTHORIZATION, "wrong-authority-token-value-00000000")
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::NOT_FOUND);
        assert!(!polled.load(Ordering::SeqCst));

        let v2 = internal
            .oneshot(
                Request::get(
                    "/internal/git-fetch/git-fetch:one/repository.git/info/refs?service=git-upload-pack",
                )
                .header(SOURCE_AUTHORIZATION, "x".repeat(43))
                .header("git-protocol", "version=2")
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(v2.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn ambiguous_protocol_or_source_headers_are_refused_before_reading_the_body() {
        for protocols in [
            vec!["version=0"],
            vec!["version=1"],
            vec!["version=2:agent=other"],
            vec!["version=2, version=2"],
            vec!["version=2", "version=2"],
            vec!["version=2", "version=1"],
        ] {
            let polled = Arc::new(AtomicBool::new(false));
            let observed = polled.clone();
            let body = Body::from_stream(stream::once(async move {
                observed.store(true, Ordering::SeqCst);
                Ok::<_, std::io::Error>(Bytes::from_static(b"request"))
            }));
            let mut request =
                Request::post("/internal/git-fetch/git-fetch:one/repository.git/git-upload-pack")
                    .header(
                        header::CONTENT_TYPE,
                        "application/x-git-upload-pack-request",
                    )
                    .header(SOURCE_AUTHORIZATION, "x".repeat(43));
            for protocol in protocols {
                request = request.header("git-protocol", protocol);
            }
            let response = git_fetch_internal_router(Arc::new(Broker))
                .oneshot(request.body(body).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert!(!polled.load(Ordering::SeqCst));
        }
        let response = git_fetch_internal_router(Arc::new(Broker)).oneshot(
            Request::get("/internal/git-fetch/git-fetch:one/repository.git/info/refs?service=git-upload-pack")
                .header(SOURCE_AUTHORIZATION, "x".repeat(43))
                .header(SOURCE_AUTHORIZATION, "x".repeat(43))
                .body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

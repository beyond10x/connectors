//! Hosted client for one exact, short-lived Git fetch source capability.

use protocol::{git_fetch, operation};
use serde::de::DeserializeOwned;
use zeroize::Zeroizing;

use crate::{request_id, require_bearer, ClientError, GitFetchSession, HostedClient};

impl HostedClient {
    /// Create a bounded read-only Git source capability for one exact provider revision.
    pub async fn create_git_fetch_session(
        &self,
        bearer: &str,
        context: &operation::OwnerContext,
        request: git_fetch::CreateRequest,
    ) -> Result<GitFetchSession, ClientError> {
        require_bearer(bearer)?;
        let request_id = request_id();
        let envelope = git_fetch::RequestEnvelope {
            protocol: git_fetch::CONTRACT.to_owned(),
            request_id: request_id.clone(),
            context: context.clone(),
            request,
        };
        if !envelope.is_valid() {
            return Err(ClientError::InvalidRequest(
                "Git fetch session request is invalid".to_owned(),
            ));
        }
        let body = serde_json::to_vec(&envelope)?;
        let created: git_fetch::CreatedSession = self.git_fetch_exchange(bearer, body).await?;
        if !created.is_valid(&request_id)
            || created.reference != envelope.request.reference
            || created.expected_commit != envelope.request.expected_commit
            || created.depth != envelope.request.depth
        {
            return Err(ClientError::InvalidResponse);
        }
        Ok(GitFetchSession::from_wire(created))
    }

    async fn git_fetch_exchange<R: DeserializeOwned>(
        &self,
        bearer: &str,
        body: Vec<u8>,
    ) -> Result<R, ClientError> {
        let mut response = self
            .http
            .post(self.git_fetch_sessions.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .bearer_auth(bearer)
            .body(body)
            .send()
            .await
            .map_err(|_| ClientError::HostedUnavailable)?;
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => return Err(ClientError::HostedAuthentication),
            reqwest::StatusCode::FORBIDDEN => return Err(ClientError::HostedNotGranted),
            status if status.is_client_error() => {
                return Err(ClientError::GitFetchRefused(status.as_u16()));
            }
            status if !status.is_success() => return Err(ClientError::HostedUnavailable),
            _ => {}
        }
        if response.headers().get(reqwest::header::CACHE_CONTROL)
            != Some(&reqwest::header::HeaderValue::from_static("no-store"))
            || response.headers().get(reqwest::header::PRAGMA)
                != Some(&reqwest::header::HeaderValue::from_static("no-cache"))
        {
            return Err(ClientError::CacheableCredentialResponse);
        }
        if response
            .content_length()
            .is_some_and(|length| length > git_fetch::MAX_RESPONSE_BYTES as u64)
        {
            return Err(ClientError::InvalidResponse);
        }
        let mut bytes = Zeroizing::new(Vec::new());
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| ClientError::HostedUnavailable)?
        {
            if bytes.len() + chunk.len() > git_fetch::MAX_RESPONSE_BYTES {
                return Err(ClientError::InvalidResponse);
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.is_empty() {
            return Err(ClientError::InvalidResponse);
        }
        Ok(serde_json::from_slice(&bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Bytes;
    use axum::extract::State;
    use axum::http::HeaderMap;
    use axum::response::{IntoResponse as _, Json};
    use axum::routing::post;
    use axum::Router;
    use tokio::net::TcpListener;

    use super::*;

    fn context() -> operation::OwnerContext {
        operation::OwnerContext {
            tenant_id: "tenant-one".to_owned(),
            agent_id: "workspace".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "snapshot-one".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    #[tokio::test]
    async fn response_is_bound_to_the_request_and_source_authority_is_redacted() {
        async fn git_fetch(
            State(substitute): State<bool>,
            headers: HeaderMap,
            body: Bytes,
        ) -> axum::response::Response {
            assert_eq!(headers[reqwest::header::AUTHORIZATION], "Bearer session-1");
            let request: git_fetch::RequestEnvelope = serde_json::from_slice(&body).unwrap();
            assert!(request.is_valid());
            let reference = if substitute {
                "substituted"
            } else {
                request.request.reference.as_str()
            };
            (
                [
                    (reqwest::header::CACHE_CONTROL, "no-store"),
                    (reqwest::header::PRAGMA, "no-cache"),
                ],
                Json(serde_json::json!({
                    "protocol": git_fetch::CONTRACT,
                    "request_id": request.request_id,
                    "session_ref": "git-fetch:one",
                    "source": "gitlab",
                    "locator": "https://connectors.internal/internal/git-fetch/git-fetch:one/repository.git",
                    "reference": reference,
                    "expected_commit": request.request.expected_commit,
                    "depth": request.request.depth,
                    "expires_at_unix_ms": 2_000_000_000_000_u64,
                    "source_authorization": "synthetic-one-use-source-capability"
                })),
            )
                .into_response()
        }

        async fn serve(substitute: bool) -> (HostedClient, tokio::task::JoinHandle<()>) {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let app = Router::new()
                .route("/api/connectors/v1/git-fetch-sessions", post(git_fetch))
                .with_state(substitute);
            let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
            (
                HostedClient::new(&format!("http://{address}/api/connectors/v1")).unwrap(),
                serving,
            )
        }

        let request = git_fetch::CreateRequest {
            idempotency_key: "coding-session-one".to_owned(),
            connection_ref: "connection:gitlab:one".to_owned(),
            project_id: 42,
            reference: "trunk".to_owned(),
            expected_commit: "d".repeat(40),
            depth: 10,
        };
        let (client, serving) = serve(false).await;
        let session = client
            .create_git_fetch_session("session-1", &context(), request.clone())
            .await
            .unwrap();
        assert_eq!(session.reference, "trunk");
        assert_eq!(
            session.expose_at_source_boundary(),
            "synthetic-one-use-source-capability"
        );
        assert!(!format!("{session:?}").contains("synthetic-one-use"));
        serving.abort();

        let (client, serving) = serve(true).await;
        assert!(matches!(
            client
                .create_git_fetch_session("session-1", &context(), request)
                .await,
            Err(ClientError::InvalidResponse)
        ));
        serving.abort();
    }
}

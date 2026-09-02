//! Typed and bounded hosted Integration administration.

use std::time::Duration;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;
use zeroize::Zeroizing;

use crate::model::AdminCredentialRequest;
use crate::{
    endpoint, request_id, require_bearer, AdminAuthMetadata, AdminCredentialState,
    AdminCredentialWrite, AdminLoginMetadata, AdminStatus, ClientError, HostedClient,
};

const ADMIN_RESPONSE_BYTES: usize = 64 * 1024;
const IDENTITY_RESPONSE_BYTES: usize = 16 * 1024;

/// Typed client for the exact Identity authority selected by a hosted Connectors instance.
#[derive(Clone)]
pub struct AdminIdentityClient {
    origin: Url,
    audience: String,
    scope: String,
    http: reqwest::Client,
}

impl AdminIdentityClient {
    pub fn new(target: &AdminAuthMetadata) -> Result<Self, ClientError> {
        let origin =
            Url::parse(&target.identity_origin).map_err(|_| ClientError::InvalidResponse)?;
        let loopback = origin.scheme() == "http" && origin.host_str() == Some("127.0.0.1");
        if !(origin.scheme() == "https" || loopback)
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || (origin.path() != "/" && !origin.path().is_empty())
            || origin.query().is_some()
            || origin.fragment().is_some()
            || target.audience.is_empty()
            || target.scope.is_empty()
        {
            return Err(ClientError::InvalidResponse);
        }
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|_| ClientError::HostedUnavailable)?;
        Ok(Self {
            origin,
            audience: target.audience.clone(),
            scope: target.scope.clone(),
            http,
        })
    }

    /// Reads and validates Identity's public CLI PKCE discovery document.
    pub async fn login_metadata(&self) -> Result<AdminLoginMetadata, ClientError> {
        let endpoint = self
            .origin
            .join(".well-known/identity-cli-login")
            .map_err(|_| ClientError::InvalidResponse)?;
        let mut response = self
            .http
            .get(endpoint)
            .send()
            .await
            .map_err(|_| ClientError::HostedUnavailable)?;
        if !response.status().is_success() {
            return Err(ClientError::AdminAuthenticationRefused(
                response.status().as_u16(),
            ));
        }
        let metadata: AdminLoginMetadata =
            decode_bounded(&mut response, IDENTITY_RESPONSE_BYTES).await?;
        let issuer = Url::parse(&metadata.issuer).map_err(|_| ClientError::InvalidResponse)?;
        let authorization = Url::parse(&metadata.authorization_endpoint)
            .map_err(|_| ClientError::InvalidResponse)?;
        let token =
            Url::parse(&metadata.token_endpoint).map_err(|_| ClientError::InvalidResponse)?;
        let access = Url::parse(&metadata.access_token_endpoint)
            .map_err(|_| ClientError::InvalidResponse)?;
        if issuer != self.origin
            || !same_origin(&issuer, &authorization)
            || !same_origin(&issuer, &token)
            || !same_origin(&issuer, &access)
            || metadata.cli_client_id.is_empty()
            || !metadata
                .response_types_supported
                .iter()
                .any(|value| value == "code")
            || !metadata
                .grant_types_supported
                .iter()
                .any(|value| value == "authorization_code")
            || !metadata
                .code_challenge_methods_supported
                .iter()
                .any(|value| value == "S256")
        {
            return Err(ClientError::InvalidResponse);
        }
        Ok(metadata)
    }

    /// Exchanges the one-use browser result and verifier for an exact Connectors access token.
    pub async fn exchange_access_token(
        &self,
        metadata: &AdminLoginMetadata,
        code: &Zeroizing<String>,
        redirect_uri: &str,
        verifier: &Zeroizing<String>,
    ) -> Result<Zeroizing<String>, ClientError> {
        let token_endpoint =
            Url::parse(&metadata.token_endpoint).map_err(|_| ClientError::InvalidResponse)?;
        if !same_origin(&self.origin, &token_endpoint) {
            return Err(ClientError::InvalidResponse);
        }
        let response = self
            .http
            .post(token_endpoint)
            .form(&CodeExchange {
                grant_type: "authorization_code",
                client_id: &metadata.cli_client_id,
                code: code.as_str(),
                redirect_uri,
                code_verifier: verifier.as_str(),
            })
            .send()
            .await
            .map_err(|_| ClientError::HostedUnavailable)?;
        let session: SessionResponse = identity_response(response).await?;
        if session.email.as_deref().is_some_and(str::is_empty)
            || session.session_type != "opaque_server_session"
            || session.session.is_empty()
            || session.expires_in <= 0
            || session.tenant_id.is_empty()
            || session.subject.is_empty()
        {
            return Err(ClientError::InvalidResponse);
        }
        let session = Zeroizing::new(session.session);
        let access_endpoint = Url::parse(&metadata.access_token_endpoint)
            .map_err(|_| ClientError::InvalidResponse)?;
        if !same_origin(&self.origin, &access_endpoint) {
            return Err(ClientError::InvalidResponse);
        }
        let body = serde_json::to_vec(&AccessRequest {
            audience: &self.audience,
            scope: &self.scope,
        })?;
        let response = self
            .http
            .post(access_endpoint)
            .bearer_auth(session.as_str())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .map_err(|_| ClientError::HostedUnavailable)?;
        let access: AccessResponse = identity_response(response).await?;
        if access.token_type != "Bearer"
            || access.access_token.is_empty()
            || access.expires_in <= 0
            || access.audience != self.audience
            || access.scope != self.scope
        {
            return Err(ClientError::InvalidResponse);
        }
        Ok(Zeroizing::new(access.access_token))
    }
}

#[derive(Serialize)]
struct CodeExchange<'a> {
    grant_type: &'a str,
    client_id: &'a str,
    code: &'a str,
    redirect_uri: &'a str,
    code_verifier: &'a str,
}

#[derive(Serialize)]
struct AccessRequest<'a> {
    audience: &'a str,
    scope: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionResponse {
    session: String,
    session_type: String,
    expires_in: i64,
    tenant_id: String,
    subject: String,
    email: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccessResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    audience: String,
    scope: String,
}

async fn identity_response<R: DeserializeOwned>(
    mut response: reqwest::Response,
) -> Result<R, ClientError> {
    if !response.status().is_success() {
        return Err(ClientError::AdminAuthenticationRefused(
            response.status().as_u16(),
        ));
    }
    require_no_store(&response)?;
    decode_bounded(&mut response, IDENTITY_RESPONSE_BYTES).await
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
        && right.username().is_empty()
        && right.password().is_none()
        && right.fragment().is_none()
}

impl HostedClient {
    /// Reads the public Identity login facts selected by this hosted Connector deployment.
    pub async fn admin_auth_metadata(&self) -> Result<AdminAuthMetadata, ClientError> {
        let mut response = self
            .http
            .get(self.admin_auth_metadata.clone())
            .send()
            .await
            .map_err(|_| ClientError::HostedUnavailable)?;
        if !response.status().is_success() {
            return Err(ClientError::HostedUnavailable);
        }
        decode_bounded(&mut response, ADMIN_RESPONSE_BYTES).await
    }

    /// Reports readiness of every activated hosted Integration without reading credential bytes.
    pub async fn admin_integrations_status(
        &self,
        identity_bearer: &str,
    ) -> Result<AdminStatus, ClientError> {
        self.admin_exchange(
            reqwest::Method::GET,
            self.admin_integrations.clone(),
            identity_bearer,
            None,
        )
        .await
    }

    /// Writes one named operator credential directly to Connector custody.
    pub async fn set_admin_credential(
        &self,
        identity_bearer: &str,
        integration_ref: &str,
        credential: &str,
        value: &Zeroizing<String>,
        replace: bool,
    ) -> Result<AdminCredentialWrite, ClientError> {
        require_admin_segment(integration_ref)?;
        require_admin_segment(credential)?;
        let endpoint = endpoint(
            &self.base,
            &format!("admin/integrations/{integration_ref}/credentials/{credential}"),
        );
        let request_id = request_id();
        let body = serde_json::to_vec(&AdminCredentialRequest {
            request_id: &request_id,
            value: value.as_str(),
            replace,
        })?;
        let written: AdminCredentialWrite = self
            .admin_exchange(reqwest::Method::PUT, endpoint, identity_bearer, Some(body))
            .await?;
        if written.request_id != request_id
            || written.integration_ref != integration_ref
            || written.credential != credential
            || written.state != AdminCredentialState::Present
        {
            return Err(ClientError::InvalidResponse);
        }
        Ok(written)
    }

    async fn admin_exchange<R: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        endpoint: Url,
        bearer: &str,
        body: Option<Vec<u8>>,
    ) -> Result<R, ClientError> {
        require_bearer(bearer)?;
        let mut request = self.http.request(method, endpoint).bearer_auth(bearer);
        if let Some(body) = body {
            if body.len() > 12 * 1024 {
                return Err(ClientError::InvalidRequest(
                    "administrative credential request exceeds the protocol bound".to_owned(),
                ));
            }
            request = request
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(body);
        }
        let mut response = request
            .send()
            .await
            .map_err(|_| ClientError::HostedUnavailable)?;
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
                return Err(ClientError::HostedNotGranted);
            }
            status if status.is_client_error() => {
                return Err(ClientError::AdminRefused(status.as_u16()));
            }
            status if !status.is_success() => return Err(ClientError::HostedUnavailable),
            _ => {}
        }
        require_no_store(&response)?;
        decode_bounded(&mut response, ADMIN_RESPONSE_BYTES).await
    }
}

fn require_admin_segment(value: &str) -> Result<(), ClientError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ClientError::InvalidRequest(
            "administrative resource name is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn require_no_store(response: &reqwest::Response) -> Result<(), ClientError> {
    if response.headers().get(reqwest::header::CACHE_CONTROL)
        != Some(&reqwest::header::HeaderValue::from_static("no-store"))
        || response.headers().get(reqwest::header::PRAGMA)
            != Some(&reqwest::header::HeaderValue::from_static("no-cache"))
    {
        return Err(ClientError::CacheableCredentialResponse);
    }
    Ok(())
}

async fn decode_bounded<R: DeserializeOwned>(
    response: &mut reqwest::Response,
    response_bound: usize,
) -> Result<R, ClientError> {
    if response
        .content_length()
        .is_some_and(|length| length > response_bound as u64)
    {
        return Err(ClientError::InvalidResponse);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ClientError::HostedUnavailable)?
    {
        if bytes.len() + chunk.len() > response_bound {
            return Err(ClientError::InvalidResponse);
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod tests {
    use axum::body::Bytes;
    use axum::extract::{Path, State};
    use axum::http::HeaderMap;
    use axum::response::IntoResponse as _;
    use axum::routing::{get, post, put};
    use axum::{Json, Router};
    use tokio::net::TcpListener;

    use super::*;

    fn confidential(value: serde_json::Value) -> axum::response::Response {
        (
            [
                (reqwest::header::CACHE_CONTROL, "no-store"),
                (reqwest::header::PRAGMA, "no-cache"),
            ],
            Json(value),
        )
            .into_response()
    }

    #[tokio::test]
    async fn named_resources_are_typed_and_the_value_is_not_exposed() {
        async fn metadata() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "identity_origin": "https://identity.example.test",
                "audience": "urn:b10x:connectors",
                "scope": "connectors.integrations.manage"
            }))
        }
        async fn status(headers: HeaderMap) -> axum::response::Response {
            assert_eq!(headers[reqwest::header::AUTHORIZATION], "Bearer access");
            confidential(serde_json::json!({"integrations": [], "ready": true}))
        }
        async fn write(
            Path((integration, credential)): Path<(String, String)>,
            body: Bytes,
        ) -> axum::response::Response {
            let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(request["value"], "secret-marker");
            confidential(serde_json::json!({
                "request_id": request["request_id"],
                "integration_ref": integration,
                "credential": credential,
                "state": "present",
                "replaced": false
            }))
        }
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new()
            .route("/api/connectors/v1/admin/auth-metadata", get(metadata))
            .route("/api/connectors/v1/admin/integrations", get(status))
            .route(
                "/api/connectors/v1/admin/integrations/{integration}/credentials/{credential}",
                put(write),
            );
        let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let base = Url::parse(&format!("http://{address}/api/connectors/v1")).unwrap();
        let client = HostedClient::from_parts(base, reqwest::Client::new());
        assert_eq!(
            client.admin_auth_metadata().await.unwrap().audience,
            "urn:b10x:connectors"
        );
        assert!(
            client
                .admin_integrations_status("access")
                .await
                .unwrap()
                .ready
        );
        let marker = Zeroizing::new("secret-marker".to_owned());
        let written = client
            .set_admin_credential("access", "gitlab", "oauth_client_secret", &marker, false)
            .await
            .unwrap();
        assert_eq!(written.state, AdminCredentialState::Present);
        assert!(!format!("{written:?}").contains(marker.as_str()));
        serving.abort();
    }

    #[tokio::test]
    async fn identity_pkce_exchange_returns_only_the_exact_access_credential() {
        async fn discovery(State(origin): State<String>) -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "issuer": origin,
                "authorization_endpoint": format!("{origin}/oauth/authorize"),
                "token_endpoint": format!("{origin}/oauth/token"),
                "access_token_endpoint": format!("{origin}/v1/access-token"),
                "cli_client_id": "connectors-cli",
                "response_types_supported": ["code"],
                "grant_types_supported": ["authorization_code"],
                "code_challenge_methods_supported": ["S256"]
            }))
        }
        async fn token(body: Bytes) -> axum::response::Response {
            let body = std::str::from_utf8(&body).unwrap();
            assert!(body.contains("grant_type=authorization_code"));
            assert!(body.contains("client_id=connectors-cli"));
            assert!(body.contains("code=one-use-code"));
            assert!(body.contains("code_verifier=pkce-verifier"));
            confidential(serde_json::json!({
                "session": "identity-session",
                "session_type": "opaque_server_session",
                "expires_in": 3600,
                "tenant_id": "tenant-one",
                "subject": "person-one",
                "email": "person@example.test"
            }))
        }
        async fn access(headers: HeaderMap, body: Bytes) -> axum::response::Response {
            assert_eq!(
                headers[reqwest::header::AUTHORIZATION],
                "Bearer identity-session"
            );
            let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(body["audience"], "urn:b10x:connectors");
            assert_eq!(body["scope"], "connectors.integrations.manage");
            confidential(serde_json::json!({
                "access_token": "identity-access",
                "token_type": "Bearer",
                "expires_in": 300,
                "audience": "urn:b10x:connectors",
                "scope": "connectors.integrations.manage"
            }))
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let origin = format!("http://{address}");
        let app = Router::new()
            .route("/.well-known/identity-cli-login", get(discovery))
            .route("/oauth/token", post(token))
            .route("/v1/access-token", post(access))
            .with_state(origin.clone());
        let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let identity = AdminIdentityClient::new(&AdminAuthMetadata {
            identity_origin: origin,
            audience: "urn:b10x:connectors".to_owned(),
            scope: "connectors.integrations.manage".to_owned(),
        })
        .unwrap();
        let metadata = identity.login_metadata().await.unwrap();
        let access = identity
            .exchange_access_token(
                &metadata,
                &Zeroizing::new("one-use-code".to_owned()),
                "http://127.0.0.1:12345/callback",
                &Zeroizing::new("pkce-verifier".to_owned()),
            )
            .await
            .unwrap();
        assert_eq!(access.as_str(), "identity-access");
        serving.abort();
    }
}

#![forbid(unsafe_code)]

//! Connector-owned custody for a user subscription credential and the narrow lease that lets an
//! explicitly bound Harness attempt spend it. No API in this crate lists or exports stored values.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use connector_secrets::{CredentialRef, Secret, SecretStore, StoreError};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use zeroize::{Zeroize, Zeroizing};

const AUTHORITY: &str = "com.anthropic.claude-code";
const CREDENTIAL: &str = "subscription_token";
const DEFAULT_SERVICE: &str = "default";
const MAX_CREDENTIAL_BYTES: usize = 16 * 1024;
const MAX_ATTEMPT_BYTES: usize = 256;
const MAX_LEASES: usize = 10_000;
const MAX_LEASE_USES: u16 = 1_024;
const MAX_OAUTH_RESPONSE_BYTES: usize = 64 * 1024;
const OAUTH_PENDING_MILLIS: u64 = 10 * 60 * 1_000;
const REFRESH_SKEW_SECONDS: u64 = 5 * 60;
const OAUTH_RECORD_VERSION: u8 = 1;
const REQUIRED_SCOPE: &str = "user:inference";

const CLAUDE_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const CLAUDE_AUTHORIZE_ENDPOINT: &str = "https://claude.com/cai/oauth/authorize";
const CLAUDE_TOKEN_ENDPOINT: &str = "https://platform.claude.com/v1/oauth/token";
const CLAUDE_REDIRECT_URI: &str = "https://platform.claude.com/oauth/code/callback";
const CLAUDE_SCOPES: &str = "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload";

#[derive(Debug, thiserror::Error)]
pub enum CustodyError {
    #[error("the subscription credential is malformed")]
    InvalidCredential,
    #[error("the attempt binding is malformed")]
    InvalidAttempt,
    #[error("subscription credential custody is unavailable")]
    Unavailable,
    #[error("no subscription credential is connected")]
    NotConnected,
    #[error("the subscription credential lease was refused")]
    LeaseRefused,
    #[error("the subscription OAuth flow was refused")]
    OauthRefused,
}

/// Public, non-secret provider configuration for Claude's subscription OAuth client.
#[derive(Debug, Clone)]
pub struct ClaudeOAuthConfig {
    client_id: String,
    authorize_endpoint: Url,
    token_endpoint: Url,
    redirect_uri: String,
    scopes: String,
}

impl ClaudeOAuthConfig {
    /// The public-client contract used by the installed Claude Code client.
    pub fn official() -> Result<Self, CustodyError> {
        Self::new(
            CLAUDE_CLIENT_ID,
            CLAUDE_AUTHORIZE_ENDPOINT,
            CLAUDE_TOKEN_ENDPOINT,
            CLAUDE_REDIRECT_URI,
            CLAUDE_SCOPES,
        )
    }

    /// Builds an exact provider contract. Plain HTTP is accepted only for loopback tests.
    pub fn new(
        client_id: &str,
        authorize_endpoint: &str,
        token_endpoint: &str,
        redirect_uri: &str,
        scopes: &str,
    ) -> Result<Self, CustodyError> {
        let authorize_endpoint = provider_url(authorize_endpoint)?;
        let token_endpoint = provider_url(token_endpoint)?;
        let redirect = Url::parse(redirect_uri).map_err(|_| CustodyError::OauthRefused)?;
        if !valid_public_value(client_id, 512)
            || !valid_scope_set(scopes)
            || !valid_provider_url(&redirect)
        {
            return Err(CustodyError::OauthRefused);
        }
        Ok(Self {
            client_id: client_id.to_owned(),
            authorize_endpoint,
            token_endpoint,
            redirect_uri: redirect.into(),
            scopes: scopes.to_owned(),
        })
    }
}

/// Non-secret start result returned to an authenticated browser client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthStart {
    pub authorization_url: String,
    pub flow_id: String,
    pub expires_at: u64,
}

struct PendingOAuth {
    tenant_id: String,
    subject: String,
    state: String,
    verifier: Zeroizing<String>,
}

struct ClaudeOAuth {
    config: ClaudeOAuthConfig,
    http: reqwest::Client,
    pending: Mutex<connector_oauth::PendingStates<PendingOAuth>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredOAuthCredential {
    version: u8,
    access_token: String,
    refresh_token: String,
    expires_at_unix_ms: u64,
    refresh_token_expires_at_unix_ms: Option<u64>,
    scopes: Vec<String>,
}

impl Drop for StoredOAuthCredential {
    fn drop(&mut self) {
        self.access_token.zeroize();
        self.refresh_token.zeroize();
    }
}

#[derive(Deserialize)]
struct ProviderTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    #[serde(default)]
    refresh_token_expires_in: Option<u64>,
    #[serde(default)]
    scope: String,
}

#[derive(Serialize)]
struct AuthorizationCodeExchange<'a> {
    grant_type: &'static str,
    code: &'a str,
    redirect_uri: &'a str,
    client_id: &'a str,
    code_verifier: &'a str,
    state: &'a str,
}

#[derive(Serialize)]
struct RefreshExchange<'a> {
    grant_type: &'static str,
    refresh_token: &'a str,
    client_id: &'a str,
    scope: &'a str,
}

/// The only secret returned to an HTTP adapter: a short-lived lease capability, never the stored
/// provider credential.
pub struct LeaseCapability {
    pub lease_id: String,
    token: Zeroizing<String>,
    pub expires_at: u64,
}

impl LeaseCapability {
    #[must_use]
    pub fn expose_at_transport_boundary(&self) -> &str {
        self.token.as_str()
    }
}

impl std::fmt::Debug for LeaseCapability {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LeaseCapability")
            .field("lease_id", &self.lease_id)
            .field("token", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

struct Lease {
    token_sha256: [u8; 32],
    credential_ref: CredentialRef,
    owner_subject: String,
    attempt_id: String,
    expires_at: u64,
    remaining_uses: u16,
}

/// One in-process lease authority over a durable Connector-owned secret store. Restarting loses
/// leases and therefore revokes them; it never loses the underlying connection.
#[derive(Clone)]
pub struct SubscriptionCustody {
    store: Arc<dyn SecretStore>,
    leases: Arc<Mutex<BTreeMap<String, Lease>>>,
    oauth: Option<Arc<ClaudeOAuth>>,
}

impl std::fmt::Debug for SubscriptionCustody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SubscriptionCustody")
            .field("store", &"SecretStore")
            .field("leases", &"[REDACTED]")
            .field("oauth", &self.oauth.as_ref().map(|_| "configured"))
            .finish()
    }
}

impl SubscriptionCustody {
    #[must_use]
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        Self {
            store,
            leases: Arc::new(Mutex::new(BTreeMap::new())),
            oauth: None,
        }
    }

    /// Enables Claude's public-client OAuth flow and refresh lifecycle for this custody.
    pub fn with_claude_oauth(
        store: Arc<dyn SecretStore>,
        config: ClaudeOAuthConfig,
    ) -> Result<Self, CustodyError> {
        let builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30));
        let builder = if config.token_endpoint.scheme() == "https" {
            builder.https_only(true)
        } else {
            builder
        };
        let http = builder.build().map_err(|_| CustodyError::Unavailable)?;
        Ok(Self {
            store,
            leases: Arc::new(Mutex::new(BTreeMap::new())),
            oauth: Some(Arc::new(ClaudeOAuth {
                config,
                http,
                pending: Mutex::new(connector_oauth::PendingStates::new(
                    connector_oauth::DEFAULT_PENDING_CAPACITY,
                )),
            })),
        })
    }

    /// Replaces the credential for one verified tenant subject.
    pub async fn connect(
        &self,
        tenant_id: &str,
        subject: &str,
        credential: Zeroizing<String>,
    ) -> Result<(), CustodyError> {
        if credential.len() < 16
            || credential.len() > MAX_CREDENTIAL_BYTES
            || credential.chars().any(char::is_whitespace)
        {
            return Err(CustodyError::InvalidCredential);
        }
        let reference = credential_ref(tenant_id, subject)?;
        let mut leases = self.leases.lock().await;
        self.store
            .put_owned(&reference, subject, &Secret::new(credential.as_str()))
            .await
            .map_err(|_| CustodyError::Unavailable)?;
        leases.retain(|_, lease| lease.credential_ref != reference);
        Ok(())
    }

    /// Starts a single-use Claude OAuth authorization for one verified tenant subject.
    pub async fn start_oauth(
        &self,
        tenant_id: &str,
        subject: &str,
    ) -> Result<OAuthStart, CustodyError> {
        let oauth = self.oauth.as_ref().ok_or(CustodyError::OauthRefused)?;
        let pkce = connector_oauth::Pkce::generate().map_err(|_| CustodyError::Unavailable)?;
        let state = connector_oauth::random_token(32).map_err(|_| CustodyError::Unavailable)?;
        let flow_id = connector_oauth::random_token(32).map_err(|_| CustodyError::Unavailable)?;
        let now = now_millis()?;
        let expires_at_unix_ms = now
            .checked_add(OAUTH_PENDING_MILLIS)
            .ok_or(CustodyError::Unavailable)?;
        let path = oauth.config.authorize_endpoint.path().to_owned();
        let authorization_url = connector_oauth::authorize_url(
            &oauth.config.authorize_endpoint,
            &path,
            &connector_oauth::AuthorizeParams {
                client_id: &oauth.config.client_id,
                redirect_uri: &oauth.config.redirect_uri,
                scope: &oauth.config.scopes,
                state: &state,
                code_challenge: Some(pkce.challenge()),
                extra: &[("code", "true")],
            },
        )
        .map_err(|_| CustodyError::OauthRefused)?;
        oauth
            .pending
            .lock()
            .await
            .insert(
                flow_id.clone(),
                PendingOAuth {
                    tenant_id: tenant_id.to_owned(),
                    subject: subject.to_owned(),
                    state,
                    verifier: pkce.into_verifier(),
                },
                expires_at_unix_ms,
                now,
            )
            .map_err(|_| CustodyError::Unavailable)?;
        Ok(OAuthStart {
            authorization_url,
            flow_id,
            expires_at: expires_at_unix_ms / 1_000,
        })
    }

    /// Completes one pending authorization from the provider's `code#state` manual result,
    /// validates its flow binding and token response, and replaces custody.
    pub async fn complete_oauth(
        &self,
        tenant_id: &str,
        subject: &str,
        flow_id: &str,
        code_and_state: &str,
    ) -> Result<(), CustodyError> {
        if !valid_public_value(flow_id, 512)
            || code_and_state.is_empty()
            || code_and_state.len() > 8 * 1024
            || code_and_state.chars().any(char::is_whitespace)
        {
            return Err(CustodyError::OauthRefused);
        }
        let oauth = self.oauth.as_ref().ok_or(CustodyError::OauthRefused)?;
        let pending = oauth
            .pending
            .lock()
            .await
            .take(flow_id, now_millis()?)
            .ok_or(CustodyError::OauthRefused)?
            .payload;
        if pending.tenant_id != tenant_id || pending.subject != subject {
            return Err(CustodyError::OauthRefused);
        }
        // Claude's manual callback renders one opaque value as `authorization_code#state`.
        // Treat the returned state as part of the provider response: bind it to this pending
        // browser flow, then send only the authorization-code component to the token endpoint.
        let (authorization_code, returned_state) = code_and_state
            .split_once('#')
            .ok_or(CustodyError::OauthRefused)?;
        if authorization_code.is_empty()
            || returned_state.is_empty()
            || returned_state.contains('#')
            || returned_state != pending.state
        {
            return Err(CustodyError::OauthRefused);
        }
        let response = oauth
            .exchange_code(authorization_code, &pending.verifier, &pending.state)
            .await?;
        let record = oauth_record(response, None, now_millis()?)?;
        let reference = credential_ref(tenant_id, subject)?;
        let serialized =
            Zeroizing::new(serde_json::to_string(&record).map_err(|_| CustodyError::Unavailable)?);
        if serialized.len() > MAX_CREDENTIAL_BYTES {
            return Err(CustodyError::OauthRefused);
        }
        let mut leases = self.leases.lock().await;
        self.store
            .put_owned(&reference, subject, &Secret::new(serialized.as_str()))
            .await
            .map_err(|_| CustodyError::Unavailable)?;
        leases.retain(|_, lease| lease.credential_ref != reference);
        Ok(())
    }

    /// Reports presence without returning or hashing the provider credential.
    pub async fn connected(&self, tenant_id: &str, subject: &str) -> Result<bool, CustodyError> {
        let reference = credential_ref(tenant_id, subject)?;
        self.store
            .exists(&reference)
            .await
            .map_err(|_| CustodyError::Unavailable)
    }

    /// Revokes the connection and every currently live lease for that credential address.
    pub async fn disconnect(&self, tenant_id: &str, subject: &str) -> Result<(), CustodyError> {
        let reference = credential_ref(tenant_id, subject)?;
        let mut leases = self.leases.lock().await;
        self.store
            .delete(&reference)
            .await
            .map_err(|_| CustodyError::Unavailable)?;
        leases.retain(|_, lease| lease.credential_ref != reference);
        Ok(())
    }

    /// Creates a capability bound to one attempt, a duration, and a finite number of wire calls.
    pub async fn lease(
        &self,
        tenant_id: &str,
        subject: &str,
        attempt_id: &str,
        ttl: Duration,
        maximum_uses: u16,
    ) -> Result<LeaseCapability, CustodyError> {
        validate_attempt(attempt_id)?;
        if ttl.is_zero()
            || ttl > Duration::from_secs(60 * 60)
            || maximum_uses == 0
            || maximum_uses > MAX_LEASE_USES
        {
            return Err(CustodyError::LeaseRefused);
        }
        let reference = credential_ref(tenant_id, subject)?;
        let mut leases = self.leases.lock().await;
        match self.store.get(&reference).await {
            Ok(_) => {}
            Err(StoreError::NotFound { .. }) => return Err(CustodyError::NotConnected),
            Err(_) => return Err(CustodyError::Unavailable),
        }
        let lease_id = connector_oauth::random_token(18).map_err(|_| CustodyError::Unavailable)?;
        let token = connector_oauth::random_token(32).map_err(|_| CustodyError::Unavailable)?;
        let current_time = now()?;
        let expires_at = current_time
            .checked_add(ttl.as_secs())
            .ok_or(CustodyError::Unavailable)?;
        leases.retain(|_, lease| lease.expires_at > current_time);
        if leases.len() >= MAX_LEASES {
            return Err(CustodyError::Unavailable);
        }
        leases.insert(
            lease_id.clone(),
            Lease {
                token_sha256: Sha256::digest(token.as_bytes()).into(),
                credential_ref: reference,
                owner_subject: subject.to_owned(),
                attempt_id: attempt_id.to_owned(),
                expires_at,
                remaining_uses: maximum_uses,
            },
        );
        Ok(LeaseCapability {
            lease_id,
            token: Zeroizing::new(token),
            expires_at,
        })
    }

    /// Redeems one use and returns the provider credential directly to the Harness bearer source.
    pub async fn redeem(
        &self,
        lease_id: &str,
        lease_token: &str,
        attempt_id: &str,
    ) -> Result<Secret, CustodyError> {
        validate_attempt(attempt_id)?;
        let mut leases = self.leases.lock().await;
        let (reference, owner_subject) = {
            let lease = leases.get_mut(lease_id).ok_or(CustodyError::LeaseRefused)?;
            let candidate: [u8; 32] = Sha256::digest(lease_token.as_bytes()).into();
            if !constant_time_equal(&lease.token_sha256, &candidate)
                || lease.attempt_id != attempt_id
                || lease.expires_at <= now()?
                || lease.remaining_uses == 0
            {
                return Err(CustodyError::LeaseRefused);
            }
            lease.remaining_uses -= 1;
            (lease.credential_ref.clone(), lease.owner_subject.clone())
        };
        let stored = self
            .store
            .get(&reference)
            .await
            .map_err(|error| match error {
                StoreError::NotFound { .. } => CustodyError::NotConnected,
                _ => CustodyError::Unavailable,
            })?;
        if !stored.expose_secret().starts_with('{') {
            return Ok(stored);
        }
        let record: StoredOAuthCredential =
            serde_json::from_str(stored.expose_secret()).map_err(|_| CustodyError::Unavailable)?;
        if record.version != OAUTH_RECORD_VERSION {
            return Err(CustodyError::Unavailable);
        }
        let current_time = now_millis()?;
        if !connector_oauth::refresh_due(
            record.expires_at_unix_ms,
            Some(current_time),
            REFRESH_SKEW_SECONDS,
        ) {
            return Ok(Secret::new(&record.access_token));
        }
        let oauth = self.oauth.as_ref().ok_or(CustodyError::Unavailable)?;
        if record
            .refresh_token_expires_at_unix_ms
            .is_some_and(|expires_at| expires_at <= current_time)
        {
            return Err(CustodyError::NotConnected);
        }
        let response = oauth.exchange_refresh(&record.refresh_token).await?;
        let refreshed = oauth_record(response, Some(&record), current_time)?;
        let serialized = Zeroizing::new(
            serde_json::to_string(&refreshed).map_err(|_| CustodyError::Unavailable)?,
        );
        self.store
            .put_owned(
                &reference,
                &owner_subject,
                &Secret::new(serialized.as_str()),
            )
            .await
            .map_err(|_| CustodyError::Unavailable)?;
        Ok(Secret::new(&refreshed.access_token))
    }
}

impl ClaudeOAuth {
    async fn exchange_code(
        &self,
        code: &str,
        verifier: &str,
        state: &str,
    ) -> Result<ProviderTokenResponse, CustodyError> {
        self.exchange(&AuthorizationCodeExchange {
            grant_type: "authorization_code",
            code,
            redirect_uri: &self.config.redirect_uri,
            client_id: &self.config.client_id,
            code_verifier: verifier,
            state,
        })
        .await
    }

    async fn exchange_refresh(
        &self,
        refresh_token: &str,
    ) -> Result<ProviderTokenResponse, CustodyError> {
        self.exchange(&RefreshExchange {
            grant_type: "refresh_token",
            refresh_token,
            client_id: &self.config.client_id,
            scope: &self.config.scopes,
        })
        .await
    }

    async fn exchange(
        &self,
        request: &impl Serialize,
    ) -> Result<ProviderTokenResponse, CustodyError> {
        let response = self
            .http
            .post(self.config.token_endpoint.clone())
            .json(request)
            .send()
            .await
            .map_err(|_| CustodyError::Unavailable)?;
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|length| length > MAX_OAUTH_RESPONSE_BYTES as u64)
        {
            return Err(CustodyError::OauthRefused);
        }
        let body = response
            .bytes()
            .await
            .map_err(|_| CustodyError::Unavailable)?;
        if body.len() > MAX_OAUTH_RESPONSE_BYTES {
            return Err(CustodyError::OauthRefused);
        }
        serde_json::from_slice(&body).map_err(|_| CustodyError::OauthRefused)
    }
}

fn oauth_record(
    mut response: ProviderTokenResponse,
    previous: Option<&StoredOAuthCredential>,
    now_unix_ms: u64,
) -> Result<StoredOAuthCredential, CustodyError> {
    let mut access_token = Zeroizing::new(std::mem::take(&mut response.access_token));
    let mut refresh_token = Zeroizing::new(
        response
            .refresh_token
            .take()
            .or_else(|| previous.map(|record| record.refresh_token.clone()))
            .ok_or(CustodyError::OauthRefused)?,
    );
    let scopes = if response.scope.trim().is_empty() {
        previous
            .map(|record| record.scopes.clone())
            .ok_or(CustodyError::OauthRefused)?
    } else {
        canonical_scopes(&response.scope)?
    };
    if response.expires_in == 0
        || !valid_secret(access_token.as_str())
        || !valid_secret(refresh_token.as_str())
        || !scopes.iter().any(|scope| scope == REQUIRED_SCOPE)
    {
        return Err(CustodyError::OauthRefused);
    }
    let expires_at_unix_ms = now_unix_ms
        .checked_add(response.expires_in.saturating_mul(1_000))
        .ok_or(CustodyError::Unavailable)?;
    let refresh_token_expires_at_unix_ms = match response.refresh_token_expires_in {
        Some(lifetime) => Some(
            now_unix_ms
                .checked_add(lifetime.saturating_mul(1_000))
                .ok_or(CustodyError::Unavailable)?,
        ),
        None => previous.and_then(|record| record.refresh_token_expires_at_unix_ms),
    };
    Ok(StoredOAuthCredential {
        version: OAUTH_RECORD_VERSION,
        access_token: std::mem::take(&mut *access_token),
        refresh_token: std::mem::take(&mut *refresh_token),
        expires_at_unix_ms,
        refresh_token_expires_at_unix_ms,
        scopes,
    })
}

fn canonical_scopes(value: &str) -> Result<Vec<String>, CustodyError> {
    if value.is_empty()
        || value.len() > 8 * 1024
        || value.split_whitespace().collect::<Vec<_>>().join(" ") != value
    {
        return Err(CustodyError::OauthRefused);
    }
    let mut scopes = value
        .split_ascii_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if scopes.iter().any(|scope| !valid_public_value(scope, 256)) {
        return Err(CustodyError::OauthRefused);
    }
    scopes.sort();
    scopes.dedup();
    Ok(scopes)
}

fn provider_url(value: &str) -> Result<Url, CustodyError> {
    let url = Url::parse(value).map_err(|_| CustodyError::OauthRefused)?;
    if valid_provider_url(&url) {
        Ok(url)
    } else {
        Err(CustodyError::OauthRefused)
    }
}

fn valid_provider_url(url: &Url) -> bool {
    let loopback_http = url.scheme() == "http"
        && url
            .host_str()
            .is_some_and(|host| host == "127.0.0.1" || host == "localhost");
    (url.scheme() == "https" || loopback_http)
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
}

fn valid_scope_set(value: &str) -> bool {
    canonical_scopes(value).is_ok()
}

fn valid_public_value(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && !matches!(byte, b'"' | b'\\'))
}

fn valid_secret(value: &str) -> bool {
    (16..=MAX_CREDENTIAL_BYTES).contains(&value.len())
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn credential_ref(tenant_id: &str, subject: &str) -> Result<CredentialRef, CustodyError> {
    let digest = hex::encode(Sha256::digest(subject.as_bytes()));
    let instance = format!(
        "{}-{}-{}-{}-{}",
        &digest[0..8],
        &digest[8..12],
        &digest[12..16],
        &digest[16..20],
        &digest[20..32]
    );
    CredentialRef::for_instance(tenant_id, AUTHORITY, &instance, DEFAULT_SERVICE, CREDENTIAL)
        .map_err(|_| CustodyError::Unavailable)
}

fn validate_attempt(attempt_id: &str) -> Result<(), CustodyError> {
    if attempt_id.is_empty()
        || attempt_id.len() > MAX_ATTEMPT_BYTES
        || !attempt_id.bytes().all(|byte| byte.is_ascii_graphic())
    {
        Err(CustodyError::InvalidAttempt)
    } else {
        Ok(())
    }
}

fn now() -> Result<u64, CustodyError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| CustodyError::Unavailable)
}

fn now_millis() -> Result<u64, CustodyError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .map_err(|_| CustodyError::Unavailable)
}

fn constant_time_equal(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::extract::State;
    use axum::routing::post;
    use axum::{Json, Router};
    use connector_secrets::MemoryStore;
    use serde_json::{json, Value};

    use super::*;

    #[tokio::test]
    async fn custody_never_exports_but_an_exact_attempt_lease_can_redeem() {
        let custody = SubscriptionCustody::new(Arc::new(MemoryStore::new()));
        custody
            .connect(
                "tenant-one",
                "human-alice",
                Zeroizing::new("synthetic-subscription-token".to_owned()),
            )
            .await
            .unwrap();
        assert!(custody
            .connected("tenant-one", "human-alice")
            .await
            .unwrap());
        let capability = custody
            .lease(
                "tenant-one",
                "human-alice",
                "attempt-one",
                Duration::from_secs(60),
                1,
            )
            .await
            .unwrap();
        assert!(custody
            .redeem(
                &capability.lease_id,
                capability.expose_at_transport_boundary(),
                "wrong-attempt"
            )
            .await
            .is_err());
        let secret = custody
            .redeem(
                &capability.lease_id,
                capability.expose_at_transport_boundary(),
                "attempt-one",
            )
            .await
            .unwrap();
        assert_eq!(secret.expose_secret(), "synthetic-subscription-token");
        assert!(custody
            .redeem(
                &capability.lease_id,
                capability.expose_at_transport_boundary(),
                "attempt-one"
            )
            .await
            .is_err());
        assert!(!format!("{capability:?}").contains("synthetic"));
    }

    #[tokio::test]
    async fn replacing_a_credential_revokes_every_lease_over_the_old_generation() {
        let custody = SubscriptionCustody::new(Arc::new(MemoryStore::new()));
        custody
            .connect(
                "tenant-one",
                "human-alice",
                Zeroizing::new("synthetic-subscription-token-one".to_owned()),
            )
            .await
            .unwrap();
        let capability = custody
            .lease(
                "tenant-one",
                "human-alice",
                "attempt-one",
                Duration::from_secs(60),
                2,
            )
            .await
            .unwrap();
        custody
            .connect(
                "tenant-one",
                "human-alice",
                Zeroizing::new("synthetic-subscription-token-two".to_owned()),
            )
            .await
            .unwrap();
        assert!(custody
            .redeem(
                &capability.lease_id,
                capability.expose_at_transport_boundary(),
                "attempt-one"
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn disconnect_revokes_live_leases_and_removes_presence() {
        let custody = SubscriptionCustody::new(Arc::new(MemoryStore::new()));
        custody
            .connect(
                "tenant-one",
                "human-alice",
                Zeroizing::new("synthetic-subscription-token".to_owned()),
            )
            .await
            .unwrap();
        let capability = custody
            .lease(
                "tenant-one",
                "human-alice",
                "attempt-one",
                Duration::from_secs(60),
                2,
            )
            .await
            .unwrap();
        custody
            .disconnect("tenant-one", "human-alice")
            .await
            .unwrap();
        assert!(!custody
            .connected("tenant-one", "human-alice")
            .await
            .unwrap());
        assert!(custody
            .redeem(
                &capability.lease_id,
                capability.expose_at_transport_boundary(),
                "attempt-one"
            )
            .await
            .is_err());
    }

    async fn oauth_token(
        State(exchanges): State<Arc<AtomicUsize>>,
        Json(request): Json<Value>,
    ) -> Json<Value> {
        let exchange = exchanges.fetch_add(1, Ordering::SeqCst);
        match request["grant_type"].as_str() {
            Some("authorization_code") => {
                assert_eq!(request["code"], "one-use-provider-code");
                assert_eq!(request["client_id"], "public-client");
                assert!(request["redirect_uri"]
                    .as_str()
                    .unwrap()
                    .ends_with("/callback"));
                assert!(request["code_verifier"].as_str().unwrap().len() >= 43);
                assert!(request["state"].as_str().unwrap().len() >= 43);
                assert_eq!(exchange, 0);
                Json(json!({
                    "access_token":"synthetic-access-token-one",
                    "refresh_token":"synthetic-refresh-token-one",
                    "expires_in":1,
                    "refresh_token_expires_in":3600,
                    "scope":"user:profile user:inference",
                    "account":{"ignored":"provider-extension"}
                }))
            }
            Some("refresh_token") => {
                assert_eq!(request["refresh_token"], "synthetic-refresh-token-one");
                assert_eq!(exchange, 1);
                Json(json!({
                    "access_token":"synthetic-access-token-two",
                    "expires_in":3600
                }))
            }
            other => panic!("unexpected grant type: {other:?}"),
        }
    }

    #[tokio::test]
    async fn pkce_completion_stays_in_custody_and_refreshes_at_redemption() {
        let exchanges = Arc::new(AtomicUsize::new(0));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server_exchanges = exchanges.clone();
        tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new()
                    .route("/token", post(oauth_token))
                    .with_state(server_exchanges),
            )
            .await
            .unwrap();
        });
        let origin = format!("http://127.0.0.1:{}", address.port());
        let config = ClaudeOAuthConfig::new(
            "public-client",
            &format!("{origin}/authorize"),
            &format!("{origin}/token"),
            &format!("{origin}/callback"),
            "user:profile user:inference",
        )
        .unwrap();
        let custody =
            SubscriptionCustody::with_claude_oauth(Arc::new(MemoryStore::new()), config).unwrap();

        let start = custody
            .start_oauth("tenant-one", "human-alice")
            .await
            .unwrap();
        let authorization = Url::parse(&start.authorization_url).unwrap();
        let query = authorization.query_pairs().collect::<BTreeMap<_, _>>();
        assert_eq!(query.get("client_id").unwrap(), "public-client");
        assert_eq!(query.get("code").unwrap(), "true");
        assert_eq!(query.get("code_challenge_method").unwrap(), "S256");
        assert!(!query.contains_key("code_verifier"));
        let returned_code = format!("one-use-provider-code#{}", query.get("state").unwrap());

        custody
            .complete_oauth("tenant-one", "human-alice", &start.flow_id, &returned_code)
            .await
            .unwrap();
        assert!(custody
            .complete_oauth("tenant-one", "human-alice", &start.flow_id, &returned_code,)
            .await
            .is_err());
        assert!(custody
            .connected("tenant-one", "human-alice")
            .await
            .unwrap());

        let lease = custody
            .lease(
                "tenant-one",
                "human-alice",
                "attempt-one",
                Duration::from_secs(60),
                1,
            )
            .await
            .unwrap();
        let redeemed = custody
            .redeem(
                &lease.lease_id,
                lease.expose_at_transport_boundary(),
                "attempt-one",
            )
            .await
            .unwrap();
        assert_eq!(redeemed.expose_secret(), "synthetic-access-token-two");
        assert_eq!(exchanges.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn an_initial_oauth_response_without_scopes_is_still_refused() {
        let response: ProviderTokenResponse = serde_json::from_value(json!({
            "access_token":"synthetic-access-token-one",
            "refresh_token":"synthetic-refresh-token-one",
            "expires_in":3600
        }))
        .expect("a scope-omitting provider response has a closed default");

        assert!(matches!(
            oauth_record(response, None, 1_700_000_000_000),
            Err(CustodyError::OauthRefused)
        ));
    }

    #[tokio::test]
    async fn pkce_completion_refuses_a_missing_or_mismatched_returned_state_before_exchange() {
        let exchanges = Arc::new(AtomicUsize::new(0));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server_exchanges = exchanges.clone();
        tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new()
                    .route("/token", post(oauth_token))
                    .with_state(server_exchanges),
            )
            .await
            .unwrap();
        });
        let origin = format!("http://127.0.0.1:{}", address.port());
        let config = ClaudeOAuthConfig::new(
            "public-client",
            &format!("{origin}/authorize"),
            &format!("{origin}/token"),
            &format!("{origin}/callback"),
            "user:profile user:inference",
        )
        .unwrap();
        let custody =
            SubscriptionCustody::with_claude_oauth(Arc::new(MemoryStore::new()), config).unwrap();

        let missing_state = custody
            .start_oauth("tenant-one", "human-alice")
            .await
            .unwrap();
        assert!(custody
            .complete_oauth(
                "tenant-one",
                "human-alice",
                &missing_state.flow_id,
                "one-use-provider-code",
            )
            .await
            .is_err());

        let mismatched_state = custody
            .start_oauth("tenant-one", "human-alice")
            .await
            .unwrap();
        assert!(custody
            .complete_oauth(
                "tenant-one",
                "human-alice",
                &mismatched_state.flow_id,
                "one-use-provider-code#wrong-state",
            )
            .await
            .is_err());
        assert_eq!(exchanges.load(Ordering::SeqCst), 0);
    }
}

//! Zero-configuration native login and short-lived hosted Connector authority.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use identity_client::{AccessToken, IdentityClient, LoginMetadata, SessionExchange};
use protocol::{endpoint as connection, event, operation};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use tokio::io::{
    AsyncBufReadExt as _, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, BufReader,
};
use tokio::net::TcpListener;
use url::Url;
use zeroize::Zeroizing;

use crate::{ClientError, HostedClient};

const DISCOVERY_PROTOCOL: &str = "b10x.connectors-client-discovery.v1";
const CONNECTORS_AUDIENCE: &str = "urn:b10x:connectors";
const CATALOG_SCOPE: &str = "connectors.catalog.read";
const CONNECTION_MANAGE_SCOPE: &str = "connectors.endpoints.manage";
const CONNECTION_SELF_SCOPE: &str = "connectors.endpoints.self";
const EVENT_READ_SCOPE: &str = "connectors.events.read";
const EVENT_SELF_SCOPE: &str = "connectors.events.self";
const INVOKE_SCOPE: &str = "connectors.invoke";
const KEYRING_SERVICE: &str = "dev.b10x.connectors.identity-session";
/// The session metadata version this client reads and writes.
pub const METADATA_VERSION: u32 = 1;
const MAX_DISCOVERY_BYTES: usize = 16 * 1024;
const MAX_CALLBACK_BYTES: usize = 16 * 1024;
const MAX_MCP_FRAME_BYTES: usize = operation::MAX_FRAME_BYTES;
const REFRESH_MARGIN_SECONDS: u64 = 30;

/// Options for one browser Authorization Code + S256 PKCE login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginOptions {
    pub connectors_base: String,
    pub no_browser: bool,
    pub timeout: Duration,
}

impl LoginOptions {
    #[must_use]
    pub fn interactive(connectors_base: impl Into<String>) -> Self {
        Self {
            connectors_base: connectors_base.into(),
            no_browser: false,
            timeout: Duration::from_secs(300),
        }
    }
}

/// Non-secret local account selection. The opaque session is stored separately in the OS keyring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionMetadata {
    pub connectors_base: String,
    pub identity_origin: String,
    pub tenant_id: String,
    pub subject: String,
    pub email: Option<String>,
    pub obtained_at: u64,
    pub idle_expires_at: u64,
}

impl SessionMetadata {
    #[must_use]
    pub fn display_identity(&self) -> &str {
        self.email.as_deref().unwrap_or(&self.subject)
    }

    fn owner_context(&self) -> operation::OwnerContext {
        let digest = Sha256::digest(
            format!(
                "{}\n{}\n{}\n{}",
                self.connectors_base, self.identity_origin, self.tenant_id, self.subject
            )
            .as_bytes(),
        );
        operation::OwnerContext {
            tenant_id: self.tenant_id.clone(),
            agent_id: "connectors-cli".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "identity-session".to_owned(),
            authority_snapshot_sha256: hex(digest),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MetadataFile {
    version: u32,
    active_base: Option<String>,
    sessions: Vec<SessionMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectorsDiscovery {
    protocol: String,
    identity_origin: String,
    identity_audience: String,
}

/// Credential-safe login and refresh failures.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("hosted Connectors discovery is invalid or unavailable")]
    Discovery,
    #[error("Identity login metadata is invalid or unavailable")]
    LoginMetadata,
    #[error("Identity browser authorization could not be started")]
    Browser,
    #[error("the browser login callback was invalid or timed out")]
    Callback,
    #[error("Identity refused the authorization-code exchange")]
    CodeExchange,
    #[error("the operating-system keyring could not store or read the Identity session")]
    Keyring,
    #[error("the local non-secret Identity session metadata is invalid or unavailable")]
    State,
    #[error("there is no active hosted Connectors login; run `connectors session login <URL>`")]
    NoActiveLogin,
    #[error("Identity could not issue a short-lived Connector access token")]
    AccessToken,
}

/// A hosted Connector request with automatic exact-scope access-token renewal failed.
#[derive(Debug, thiserror::Error)]
pub enum AuthenticatedHostedError {
    #[error(transparent)]
    Identity(#[from] IdentityError),
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error("the local MCP stdio frame exceeded its named byte bound")]
    McpFrameBound,
    #[error("local MCP stdio input or output failed")]
    McpIo,
}

trait SecretStore: Send + Sync {
    fn save(&self, account: &str, secret: &str) -> Result<(), IdentityError>;
    fn load(&self, account: &str) -> Result<Zeroizing<String>, IdentityError>;
    fn delete(&self, account: &str) -> Result<(), IdentityError>;
}

struct OsKeyring;

impl SecretStore for OsKeyring {
    fn save(&self, account: &str, secret: &str) -> Result<(), IdentityError> {
        keyring::Entry::new(KEYRING_SERVICE, account)
            .map_err(|_| IdentityError::Keyring)?
            .set_password(secret)
            .map_err(|_| IdentityError::Keyring)
    }

    fn load(&self, account: &str) -> Result<Zeroizing<String>, IdentityError> {
        keyring::Entry::new(KEYRING_SERVICE, account)
            .map_err(|_| IdentityError::Keyring)?
            .get_password()
            .map(Zeroizing::new)
            .map_err(|_| IdentityError::Keyring)
    }

    fn delete(&self, account: &str) -> Result<(), IdentityError> {
        match keyring::Entry::new(KEYRING_SERVICE, account)
            .map_err(|_| IdentityError::Keyring)?
            .delete_credential()
        {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(IdentityError::Keyring),
        }
    }
}

/// Complete browser login and select this hosted Connector deployment for later commands.
pub async fn login(options: &LoginOptions) -> Result<SessionMetadata, IdentityError> {
    let state_path = metadata_path()?;
    let store: Arc<dyn SecretStore> = Arc::new(OsKeyring);
    login_with(options, &state_path, store, present_authorization).await
}

async fn login_with<F>(
    options: &LoginOptions,
    state_path: &Path,
    store: Arc<dyn SecretStore>,
    present: F,
) -> Result<SessionMetadata, IdentityError>
where
    F: FnOnce(&Url, bool) -> Result<(), IdentityError>,
{
    let base = validated_base(&options.connectors_base)?;
    let discovery = fetch_discovery(&base).await?;
    let identity = IdentityClient::new(&discovery.identity_origin, CONNECTORS_AUDIENCE)
        .map_err(|_| IdentityError::LoginMetadata)?;
    let metadata = identity
        .login_metadata()
        .await
        .map_err(|_| IdentityError::LoginMetadata)?;
    validate_login_metadata(&discovery, &metadata)?;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|_| IdentityError::Callback)?;
    let port = listener
        .local_addr()
        .map_err(|_| IdentityError::Callback)?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let state = random_token(32)?;
    let nonce = random_token(32)?;
    let verifier = Zeroizing::new(random_token(64)?);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let authorization = authorization_url(&metadata, &redirect_uri, &state, &nonce, &challenge)?;
    present(&authorization, options.no_browser)?;
    let code = wait_for_callback(listener, &state, options.timeout).await?;
    let exchanged = identity
        .exchange_code(
            &metadata.cli_client_id,
            &code,
            &redirect_uri,
            verifier.as_str(),
        )
        .await
        .map_err(|_| IdentityError::CodeExchange)?;
    persist_login(base, discovery, exchanged, state_path, store)
}

fn persist_login(
    base: Url,
    discovery: ConnectorsDiscovery,
    exchange: SessionExchange,
    state_path: &Path,
    store: Arc<dyn SecretStore>,
) -> Result<SessionMetadata, IdentityError> {
    if exchange.expires_in <= 0 || exchange.expires_in > 24 * 60 * 60 {
        return Err(IdentityError::CodeExchange);
    }
    let now = unix_time()?;
    let expires = u64::try_from(exchange.expires_in).map_err(|_| IdentityError::CodeExchange)?;
    let session = SessionMetadata {
        connectors_base: base.as_str().trim_end_matches('/').to_owned(),
        identity_origin: discovery.identity_origin,
        tenant_id: exchange.tenant_id,
        subject: exchange.subject,
        email: exchange.email,
        obtained_at: now,
        idle_expires_at: now.saturating_add(expires),
    };
    validate_session(&session)?;
    let account = keyring_account(&session);
    store.save(&account, exchange.credential.expose_at_cookie_boundary())?;
    let mut file = read_metadata(state_path)?.unwrap_or_else(empty_metadata);
    file.sessions
        .retain(|candidate| candidate.connectors_base != session.connectors_base);
    file.sessions.push(session.clone());
    file.active_base = Some(session.connectors_base.clone());
    if let Err(error) = write_metadata(state_path, &file) {
        let _ = store.delete(&account);
        return Err(error);
    }
    Ok(session)
}

/// Read the selected non-secret account without opening the keyring.
pub fn active_session_metadata() -> Result<Option<SessionMetadata>, IdentityError> {
    let path = metadata_path()?;
    active_session_at(&path)
}

fn active_session_at(path: &Path) -> Result<Option<SessionMetadata>, IdentityError> {
    let Some(file) = read_metadata(path)? else {
        return Ok(None);
    };
    let session = file.active_base.as_deref().and_then(|active| {
        file.sessions
            .iter()
            .find(|session| session.connectors_base == active)
    });
    session.cloned().map_or(Ok(None), |session| {
        validate_session(&session)?;
        Ok(Some(session))
    })
}

/// Remove the selected session from both the OS keyring and non-secret state.
pub fn logout() -> Result<Option<SessionMetadata>, IdentityError> {
    let path = metadata_path()?;
    let Some(session) = active_session_at(&path)? else {
        return Ok(None);
    };
    let store = OsKeyring;
    store.delete(&keyring_account(&session))?;
    let mut file = read_metadata(&path)?.ok_or(IdentityError::State)?;
    file.sessions
        .retain(|candidate| candidate.connectors_base != session.connectors_base);
    file.active_base = file
        .sessions
        .last()
        .map(|candidate| candidate.connectors_base.clone());
    write_metadata(&path, &file)?;
    Ok(Some(session))
}

/// Hosted client bound to the selected account and renewing access authority by exact scope.
pub struct AuthenticatedHostedClient {
    hosted: HostedClient,
    tokens: IdentityAccessTokenSource,
    context: operation::OwnerContext,
}

impl AuthenticatedHostedClient {
    /// Discover or manage endpoints using exact catalog or connection-management token scope.
    pub async fn endpoint(
        &self,
        request: protocol::endpoint_inventory::EndpointInventoryRequest,
    ) -> Result<protocol::endpoint_inventory::ResponseEnvelope, AuthenticatedHostedError> {
        let scope = match &request {
            protocol::endpoint_inventory::EndpointInventoryRequest::List(_)
            | protocol::endpoint_inventory::EndpointInventoryRequest::Show(_) => CATALOG_SCOPE,
            protocol::endpoint_inventory::EndpointInventoryRequest::Refresh(_)
            | protocol::endpoint_inventory::EndpointInventoryRequest::Bind(_) => CONNECTION_MANAGE_SCOPE,
        };
        let token = self.tokens.access_token(scope).await?;
        match self
            .hosted
            .endpoint(&token, &self.context, request.clone())
            .await
        {
            Err(ClientError::HostedAuthentication) => {
                self.tokens.invalidate(scope)?;
                let token = self.tokens.access_token(scope).await?;
                Ok(self.hosted.endpoint(&token, &self.context, request).await?)
            }
            result => Ok(result?),
        }
    }

    /// Select the endpoint-aware v5 contract with no version negotiation or invocation retry.
    pub async fn operation_v5(
        &self,
        request: operation::v5::OperationRequest,
    ) -> Result<operation::v5::ResponseEnvelope, AuthenticatedHostedError> {
        let scope = match &request {
            operation::v5::OperationRequest::Search(_)
            | operation::v5::OperationRequest::Describe(_) => CATALOG_SCOPE,
            _ => INVOKE_SCOPE,
        };
        let token = self.tokens.access_token(scope).await?;
        match self
            .hosted
            .operation_v5(&token, &self.context, request.clone())
            .await
        {
            Err(ClientError::HostedAuthentication) => {
                self.tokens.invalidate(scope)?;
                let token = self.tokens.access_token(scope).await?;
                Ok(self
                    .hosted
                    .operation_v5(&token, &self.context, request)
                    .await?)
            }
            result => Ok(result?),
        }
    }

    /// Open the selected hosted deployment. No network access occurs until a request is sent.
    pub fn active() -> Result<Self, IdentityError> {
        let session = active_session_metadata()?.ok_or(IdentityError::NoActiveLogin)?;
        Self::from_session(session, Arc::new(OsKeyring))
    }

    fn from_session(
        session: SessionMetadata,
        store: Arc<dyn SecretStore>,
    ) -> Result<Self, IdentityError> {
        validate_session(&session)?;
        let hosted =
            HostedClient::new(&session.connectors_base).map_err(|_| IdentityError::State)?;
        let context = session.owner_context();
        let tokens = IdentityAccessTokenSource::new(session, store)?;
        Ok(Self {
            hosted,
            tokens,
            context,
        })
    }

    /// The coordinated default retains typed authentication needs without renewing on HTTP 409.
    pub async fn operation(
        &self,
        request: operation::OperationRequest,
    ) -> Result<operation::v3::ResponseEnvelope, AuthenticatedHostedError> {
        self.operation_versioned(operation::versions::Version::V0Alpha3, request)
            .await
    }

    /// Select v2 explicitly; no version negotiation is performed.
    pub async fn operation_v2(
        &self,
        request: operation::OperationRequest,
    ) -> Result<operation::ResponseEnvelope, AuthenticatedHostedError> {
        Ok(self
            .operation_versioned(operation::versions::Version::V0Alpha2, request)
            .await?
            .into_v2())
    }

    pub async fn operation_versioned(
        &self,
        version: operation::versions::Version,
        request: operation::OperationRequest,
    ) -> Result<operation::v3::ResponseEnvelope, AuthenticatedHostedError> {
        let scope = match request {
            operation::OperationRequest::Search(_) | operation::OperationRequest::Describe(_) => {
                CATALOG_SCOPE
            }
            _ => INVOKE_SCOPE,
        };
        let token = self.tokens.access_token(scope).await?;
        match self
            .hosted
            .operation_versioned(version, &token, &self.context, request.clone())
            .await
        {
            Err(ClientError::HostedAuthentication) => {
                self.tokens.invalidate(scope)?;
                let token = self.tokens.access_token(scope).await?;
                Ok(self
                    .hosted
                    .operation_versioned(version, &token, &self.context, request)
                    .await?)
            }
            result => Ok(result?),
        }
    }

    pub async fn connection(
        &self,
        request: connection::EndpointRequest,
    ) -> Result<connection::ResponseEnvelope, AuthenticatedHostedError> {
        let scope = connection_scope(&request);
        let token = self.tokens.access_token(scope).await?;
        match self
            .hosted
            .connection(&token, &self.context, request.clone())
            .await
        {
            Err(ClientError::HostedAuthentication) => {
                self.tokens.invalidate(scope)?;
                let token = self.tokens.access_token(scope).await?;
                Ok(self
                    .hosted
                    .connection(&token, &self.context, request)
                    .await?)
            }
            result => Ok(result?),
        }
    }

    /// Create one Git fetch session with automatic catalog-scope token renewal.
    pub async fn create_git_fetch_session(
        &self,
        request: protocol::git_fetch::CreateRequest,
    ) -> Result<crate::GitFetchSession, AuthenticatedHostedError> {
        let token = self.tokens.access_token(CATALOG_SCOPE).await?;
        match self
            .hosted
            .create_git_fetch_session(&token, &self.context, request.clone())
            .await
        {
            Err(ClientError::HostedAuthentication) => {
                self.tokens.invalidate(CATALOG_SCOPE)?;
                let token = self.tokens.access_token(CATALOG_SCOPE).await?;
                Ok(self
                    .hosted
                    .create_git_fetch_session(&token, &self.context, request)
                    .await?)
            }
            result => Ok(result?),
        }
    }

    /// Explicit endpoint subscription lifecycle; authenticated refusals are renewed only before
    /// dispatch, and transport failures never retry a stream-opening request.
    pub async fn event_v2(
        &self,
        request: event::v2::EventRequest,
    ) -> Result<event::v2::ResponseEnvelope, AuthenticatedHostedError> {
        let scope = request
            .legacy_read()
            .as_ref()
            .map_or(EVENT_READ_SCOPE, event_scope);
        let token = self.tokens.access_token(scope).await?;
        match self
            .hosted
            .event_v2(&token, &self.context, request.clone())
            .await
        {
            Err(ClientError::HostedAuthentication) => {
                self.tokens.invalidate(scope)?;
                let token = self.tokens.access_token(scope).await?;
                Ok(self.hosted.event_v2(&token, &self.context, request).await?)
            }
            result => Ok(result?),
        }
    }

    pub async fn event(
        &self,
        request: event::EventRequest,
    ) -> Result<event::ResponseEnvelope, AuthenticatedHostedError> {
        let scope = event_scope(&request);
        let token = self.tokens.access_token(scope).await?;
        match self
            .hosted
            .event(&token, &self.context, request.clone())
            .await
        {
            Err(ClientError::HostedAuthentication) => {
                self.tokens.invalidate(scope)?;
                let token = self.tokens.access_token(scope).await?;
                Ok(self.hosted.event(&token, &self.context, request).await?)
            }
            result => Ok(result?),
        }
    }

    async fn mcp(
        &self,
        request: &[u8],
        scope: &'static str,
    ) -> Result<Option<Vec<u8>>, AuthenticatedHostedError> {
        let token = self.tokens.access_token(scope).await?;
        match self.hosted.mcp_exchange(&token, request).await {
            Err(ClientError::HostedAuthentication) => {
                self.tokens.invalidate(scope)?;
                let token = self.tokens.access_token(scope).await?;
                Ok(self.hosted.mcp_exchange(&token, request).await?)
            }
            result => Ok(result?),
        }
    }
}

struct IdentityAccessTokenSource {
    session: SessionMetadata,
    identity: IdentityClient,
    store: Arc<dyn SecretStore>,
    cached: Mutex<BTreeMap<&'static str, CachedToken>>,
    clock: Arc<dyn Fn() -> Result<u64, IdentityError> + Send + Sync>,
}

struct CachedToken {
    value: Zeroizing<String>,
    expires_at: u64,
}

impl IdentityAccessTokenSource {
    fn new(session: SessionMetadata, store: Arc<dyn SecretStore>) -> Result<Self, IdentityError> {
        let identity = IdentityClient::new(&session.identity_origin, CONNECTORS_AUDIENCE)
            .map_err(|_| IdentityError::State)?;
        Ok(Self {
            session,
            identity,
            store,
            cached: Mutex::new(BTreeMap::new()),
            clock: Arc::new(unix_time),
        })
    }

    async fn access_token(&self, scope: &'static str) -> Result<Zeroizing<String>, IdentityError> {
        if !matches!(
            scope,
            CATALOG_SCOPE
                | CONNECTION_MANAGE_SCOPE
                | CONNECTION_SELF_SCOPE
                | EVENT_READ_SCOPE
                | EVENT_SELF_SCOPE
                | INVOKE_SCOPE
        ) {
            return Err(IdentityError::AccessToken);
        }
        let now = (self.clock)()?;
        if let Some(token) = self
            .cached
            .lock()
            .map_err(|_| IdentityError::AccessToken)?
            .get(scope)
            .filter(|token| token.expires_at.saturating_sub(REFRESH_MARGIN_SECONDS) > now)
        {
            return Ok(Zeroizing::new(token.value.to_string()));
        }
        let session = self.store.load(&keyring_account(&self.session))?;
        let authorization = Zeroizing::new(format!("Bearer {}", session.as_str()));
        let token = self
            .identity
            .issue_access_token(&authorization, CONNECTORS_AUDIENCE, scope)
            .await
            .map_err(|_| IdentityError::AccessToken)?;
        self.cache_access_token(scope, now, token)
    }

    fn cache_access_token(
        &self,
        scope: &'static str,
        now: u64,
        token: AccessToken,
    ) -> Result<Zeroizing<String>, IdentityError> {
        if token.expires_in <= 0
            || token.expires_in > 300
            || token.audience != CONNECTORS_AUDIENCE
            || token.scope != scope
        {
            return Err(IdentityError::AccessToken);
        }
        let lifetime = u64::try_from(token.expires_in).map_err(|_| IdentityError::AccessToken)?;
        let value = Zeroizing::new(
            token
                .credential
                .expose_at_authorization_boundary()
                .to_owned(),
        );
        if value.is_empty() || value.len() > 512 || !value.is_ascii() {
            return Err(IdentityError::AccessToken);
        }
        self.cached
            .lock()
            .map_err(|_| IdentityError::AccessToken)?
            .insert(
                scope,
                CachedToken {
                    value: Zeroizing::new(value.to_string()),
                    expires_at: now.saturating_add(lifetime),
                },
            );
        Ok(value)
    }

    fn invalidate(&self, scope: &'static str) -> Result<(), IdentityError> {
        self.cached
            .lock()
            .map_err(|_| IdentityError::AccessToken)?
            .remove(scope);
        Ok(())
    }
}

/// Run the selected hosted Connector's MCP surface as a local newline-delimited stdio server.
pub async fn run_mcp_bridge() -> Result<(), AuthenticatedHostedError> {
    let client = AuthenticatedHostedClient::active()?;
    bridge(client, tokio::io::stdin(), tokio::io::stdout()).await
}

async fn bridge<R, W>(
    client: AuthenticatedHostedClient,
    input: R,
    mut output: W,
) -> Result<(), AuthenticatedHostedError>
where
    R: tokio::io::AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut input = BufReader::new(input);
    loop {
        let mut frame = Vec::new();
        let read = input
            .read_until(b'\n', &mut frame)
            .await
            .map_err(|_| AuthenticatedHostedError::McpIo)?;
        if read == 0 {
            return Ok(());
        }
        if frame.len() > MAX_MCP_FRAME_BYTES {
            return Err(AuthenticatedHostedError::McpFrameBound);
        }
        while frame
            .last()
            .is_some_and(|byte| matches!(byte, b'\n' | b'\r'))
        {
            frame.pop();
        }
        if frame.is_empty() {
            continue;
        }
        let scope = mcp_scope(&frame);
        if let Some(response) = client.mcp(&frame, scope).await? {
            output
                .write_all(&response)
                .await
                .map_err(|_| AuthenticatedHostedError::McpIo)?;
            output
                .write_all(b"\n")
                .await
                .map_err(|_| AuthenticatedHostedError::McpIo)?;
            output
                .flush()
                .await
                .map_err(|_| AuthenticatedHostedError::McpIo)?;
        }
    }
}

fn mcp_scope(frame: &[u8]) -> &'static str {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(frame) else {
        return CATALOG_SCOPE;
    };
    let invokes = value.get("method").and_then(serde_json::Value::as_str) == Some("tools/call")
        && value
            .pointer("/params/name")
            .and_then(serde_json::Value::as_str)
            == Some("tool_invoke");
    if invokes {
        INVOKE_SCOPE
    } else {
        CATALOG_SCOPE
    }
}

fn connection_scope(request: &connection::EndpointRequest) -> &'static str {
    match request {
        connection::EndpointRequest::CandidateActivate(_)
        | connection::EndpointRequest::Materialize(_) => CONNECTION_MANAGE_SCOPE,
        connection::EndpointRequest::ConnectSessionCreate(_) => CONNECTION_SELF_SCOPE,
        _ => CATALOG_SCOPE,
    }
}

fn event_scope(request: &event::EventRequest) -> &'static str {
    let self_service = match request {
        event::EventRequest::Search(request) => request.query == "slack",
        event::EventRequest::Receive(request) => {
            request.channel_ref.starts_with("event-channel:slack:")
        }
        event::EventRequest::Replay(request) => request.event_ref.starts_with("event:slack:"),
    };
    if self_service {
        EVENT_SELF_SCOPE
    } else {
        EVENT_READ_SCOPE
    }
}

async fn fetch_discovery(base: &Url) -> Result<ConnectorsDiscovery, IdentityError> {
    let endpoint = base
        .join(".well-known/connectors-client")
        .map_err(|_| IdentityError::Discovery)?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| IdentityError::Discovery)?;
    let mut response = client
        .get(endpoint)
        .send()
        .await
        .map_err(|_| IdentityError::Discovery)?;
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|length| length > MAX_DISCOVERY_BYTES as u64)
    {
        return Err(IdentityError::Discovery);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| IdentityError::Discovery)?
    {
        if bytes.len() + chunk.len() > MAX_DISCOVERY_BYTES {
            return Err(IdentityError::Discovery);
        }
        bytes.extend_from_slice(&chunk);
    }
    let discovery: ConnectorsDiscovery =
        serde_json::from_slice(&bytes).map_err(|_| IdentityError::Discovery)?;
    if discovery.protocol != DISCOVERY_PROTOCOL
        || discovery.identity_audience != CONNECTORS_AUDIENCE
    {
        return Err(IdentityError::Discovery);
    }
    Ok(discovery)
}

fn validated_base(value: &str) -> Result<Url, IdentityError> {
    HostedClient::new(value).map_err(|_| IdentityError::Discovery)?;
    let mut base = Url::parse(value).map_err(|_| IdentityError::Discovery)?;
    if !base.path().ends_with('/') {
        base.set_path(&format!("{}/", base.path()));
    }
    Ok(base)
}

fn validate_login_metadata(
    discovery: &ConnectorsDiscovery,
    metadata: &LoginMetadata,
) -> Result<(), IdentityError> {
    let origin =
        Url::parse(&discovery.identity_origin).map_err(|_| IdentityError::LoginMetadata)?;
    if metadata.issuer.trim_end_matches('/') != discovery.identity_origin.trim_end_matches('/')
        || !same_origin(&origin, &metadata.authorization_endpoint)
        || !same_origin(&origin, &metadata.token_endpoint)
        || !same_origin(&origin, &metadata.access_token_endpoint)
        || metadata.response_types_supported != ["code"]
        || metadata.grant_types_supported != ["authorization_code"]
        || metadata.code_challenge_methods_supported != ["S256"]
        || metadata.cli_client_id.is_empty()
    {
        return Err(IdentityError::LoginMetadata);
    }
    Ok(())
}

fn same_origin(origin: &Url, endpoint: &str) -> bool {
    Url::parse(endpoint).is_ok_and(|endpoint| {
        origin.scheme() == endpoint.scheme()
            && origin.host_str() == endpoint.host_str()
            && origin.port_or_known_default() == endpoint.port_or_known_default()
            && endpoint.username().is_empty()
            && endpoint.password().is_none()
            && endpoint.fragment().is_none()
    })
}

fn authorization_url(
    metadata: &LoginMetadata,
    redirect_uri: &str,
    state: &str,
    nonce: &str,
    challenge: &str,
) -> Result<Url, IdentityError> {
    let mut url =
        Url::parse(&metadata.authorization_endpoint).map_err(|_| IdentityError::LoginMetadata)?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &metadata.cli_client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", "openid profile email")
        .append_pair("state", state)
        .append_pair("nonce", nonce)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(url)
}

fn present_authorization(url: &Url, no_browser: bool) -> Result<(), IdentityError> {
    if no_browser {
        eprintln!("Open this URL to sign in:\n{url}");
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(url.as_str());
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url.as_str()]);
        command
    };
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(url.as_str());
        command
    };
    command
        .spawn()
        .map(|_| ())
        .map_err(|_| IdentityError::Browser)
}

async fn wait_for_callback(
    listener: TcpListener,
    expected_state: &str,
    timeout: Duration,
) -> Result<Zeroizing<String>, IdentityError> {
    let (mut stream, address) = tokio::time::timeout(timeout, listener.accept())
        .await
        .map_err(|_| IdentityError::Callback)?
        .map_err(|_| IdentityError::Callback)?;
    if !address.ip().is_loopback() {
        return Err(IdentityError::Callback);
    }
    let mut request = Vec::new();
    let mut chunk = [0_u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|_| IdentityError::Callback)?;
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);
        if request.len() > MAX_CALLBACK_BYTES {
            return Err(IdentityError::Callback);
        }
    }
    let line = std::str::from_utf8(&request)
        .map_err(|_| IdentityError::Callback)?
        .lines()
        .next()
        .ok_or(IdentityError::Callback)?;
    let mut parts = line.split_ascii_whitespace();
    if parts.next() != Some("GET") {
        return Err(IdentityError::Callback);
    }
    let target = parts.next().ok_or(IdentityError::Callback)?;
    if parts.next() != Some("HTTP/1.1") || parts.next().is_some() {
        return Err(IdentityError::Callback);
    }
    let callback =
        Url::parse(&format!("http://127.0.0.1{target}")).map_err(|_| IdentityError::Callback)?;
    if callback.path() != "/callback" || callback.fragment().is_some() {
        return Err(IdentityError::Callback);
    }
    let mut returned_state = None;
    let mut code = None;
    for (name, value) in callback.query_pairs() {
        let slot = match name.as_ref() {
            "state" => &mut returned_state,
            "code" => &mut code,
            "error" => return Err(IdentityError::Callback),
            _ => continue,
        };
        if slot.replace(value.into_owned()).is_some() {
            return Err(IdentityError::Callback);
        }
    }
    if returned_state.as_deref() != Some(expected_state) {
        return Err(IdentityError::Callback);
    }
    let code = code.ok_or(IdentityError::Callback)?;
    if code.is_empty() || code.len() > 4096 || !code.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(IdentityError::Callback);
    }
    stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: 30\r\nConnection: close\r\n\r\nSigned in. You can close this.")
        .await
        .map_err(|_| IdentityError::Callback)?;
    Ok(Zeroizing::new(code))
}

fn validate_session(session: &SessionMetadata) -> Result<(), IdentityError> {
    let base = validated_base(&session.connectors_base)?;
    let origin = Url::parse(&session.identity_origin).map_err(|_| IdentityError::State)?;
    if base.as_str().trim_end_matches('/') != session.connectors_base
        || !matches!(origin.scheme(), "https" | "http")
        || origin.host_str().is_none()
        || !origin.username().is_empty()
        || origin.password().is_some()
        || (origin.path() != "/" && !origin.path().is_empty())
        || origin.query().is_some()
        || origin.fragment().is_some()
        || session.tenant_id.is_empty()
        || session.subject.is_empty()
        || session.idle_expires_at <= session.obtained_at
    {
        return Err(IdentityError::State);
    }
    Ok(())
}

fn metadata_path() -> Result<PathBuf, IdentityError> {
    if let Some(root) = std::env::var_os("XDG_STATE_HOME") {
        return Ok(PathBuf::from(root).join("b10x/connectors/identity-sessions.json"));
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/state/b10x/connectors/identity-sessions.json"))
        .ok_or(IdentityError::State)
}

fn empty_metadata() -> MetadataFile {
    MetadataFile {
        version: METADATA_VERSION,
        active_base: None,
        sessions: Vec::new(),
    }
}

fn read_metadata(path: &Path) -> Result<Option<MetadataFile>, IdentityError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(IdentityError::State),
    };
    if bytes.len() > 1024 * 1024 {
        return Err(IdentityError::State);
    }
    let file: MetadataFile = serde_json::from_slice(&bytes).map_err(|_| IdentityError::State)?;
    if file.version != METADATA_VERSION
        || file.sessions.len() > 32
        || file.active_base.as_ref().is_some_and(|active| {
            !file
                .sessions
                .iter()
                .any(|session| &session.connectors_base == active)
        })
    {
        return Err(IdentityError::State);
    }
    for session in &file.sessions {
        validate_session(session)?;
    }
    Ok(Some(file))
}

fn write_metadata(path: &Path, file: &MetadataFile) -> Result<(), IdentityError> {
    let parent = path.parent().ok_or(IdentityError::State)?;
    fs::create_dir_all(parent).map_err(|_| IdentityError::State)?;
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
        .map_err(|_| IdentityError::State)?;
    let temporary = parent.join(format!(".identity-sessions.{}.tmp", std::process::id()));
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|_| IdentityError::State)?;
    let written = serde_json::to_writer_pretty(&mut output, file)
        .map_err(|_| IdentityError::State)
        .and_then(|()| output.write_all(b"\n").map_err(|_| IdentityError::State))
        .and_then(|()| output.sync_all().map_err(|_| IdentityError::State));
    if let Err(error) = written {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    fs::rename(&temporary, path).map_err(|_| IdentityError::State)
}

fn keyring_account(session: &SessionMetadata) -> String {
    let input = format!("{}\n{}", session.connectors_base, session.identity_origin);
    format!(
        "v1-{}",
        URL_SAFE_NO_PAD.encode(Sha256::digest(input.as_bytes()))
    )
}

fn random_token(bytes: usize) -> Result<String, IdentityError> {
    let mut random = vec![0_u8; bytes];
    getrandom::fill(&mut random).map_err(|_| IdentityError::Browser)?;
    Ok(URL_SAFE_NO_PAD.encode(random))
}

fn unix_time() -> Result<u64, IdentityError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| IdentityError::State)
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .fold(String::new(), |mut output, byte| {
            use std::fmt::Write as _;
            let _ = write!(output, "{byte:02x}");
            output
        })
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;

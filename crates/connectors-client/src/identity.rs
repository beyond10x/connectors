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
use protocol::{connection, event, operation};
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
const CONNECTION_MANAGE_SCOPE: &str = "connectors.connections.manage";
const CONNECTION_SELF_SCOPE: &str = "connectors.connections.self";
const EVENT_READ_SCOPE: &str = "connectors.events.read";
const EVENT_SELF_SCOPE: &str = "connectors.events.self";
const INVOKE_SCOPE: &str = "connectors.invoke";
const KEYRING_SERVICE: &str = "dev.b10x.connectors.identity-session";
const METADATA_VERSION: u32 = 1;
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

    pub async fn operation(
        &self,
        request: operation::OperationRequest,
    ) -> Result<operation::ResponseEnvelope, AuthenticatedHostedError> {
        let scope = match request {
            operation::OperationRequest::Search(_) | operation::OperationRequest::Describe(_) => {
                CATALOG_SCOPE
            }
            _ => INVOKE_SCOPE,
        };
        let token = self.tokens.access_token(scope).await?;
        match self
            .hosted
            .operation(&token, &self.context, request.clone())
            .await
        {
            Err(ClientError::HostedAuthentication) => {
                self.tokens.invalidate(scope)?;
                let token = self.tokens.access_token(scope).await?;
                Ok(self
                    .hosted
                    .operation(&token, &self.context, request)
                    .await?)
            }
            result => Ok(result?),
        }
    }

    pub async fn connection(
        &self,
        request: connection::ConnectionRequest,
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

fn connection_scope(request: &connection::ConnectionRequest) -> &'static str {
    match request {
        connection::ConnectionRequest::CandidateActivate(_)
        | connection::ConnectionRequest::Materialize(_) => CONNECTION_MANAGE_SCOPE,
        connection::ConnectionRequest::ConnectSessionCreate(_) => CONNECTION_SELF_SCOPE,
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
mod tests {
    use super::*;
    use std::net::TcpStream;
    use std::sync::atomic::{AtomicU64, Ordering};

    use axum::body::Bytes;
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode};
    use axum::response::IntoResponse as _;
    use axum::routing::{get, post};
    use axum::{Json, Router};

    #[derive(Default)]
    struct MemoryStore(Mutex<BTreeMap<String, String>>);

    impl SecretStore for MemoryStore {
        fn save(&self, account: &str, secret: &str) -> Result<(), IdentityError> {
            self.0
                .lock()
                .map_err(|_| IdentityError::Keyring)?
                .insert(account.to_owned(), secret.to_owned());
            Ok(())
        }

        fn load(&self, account: &str) -> Result<Zeroizing<String>, IdentityError> {
            self.0
                .lock()
                .map_err(|_| IdentityError::Keyring)?
                .get(account)
                .cloned()
                .map(Zeroizing::new)
                .ok_or(IdentityError::Keyring)
        }

        fn delete(&self, account: &str) -> Result<(), IdentityError> {
            self.0
                .lock()
                .map_err(|_| IdentityError::Keyring)?
                .remove(account);
            Ok(())
        }
    }

    struct FakeState {
        origin: String,
        connectors_base: String,
        access_issues: Mutex<BTreeMap<String, u64>>,
        mcp_bearers: Mutex<Vec<String>>,
    }

    async fn discovery(State(state): State<Arc<FakeState>>) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "protocol": DISCOVERY_PROTOCOL,
            "identity_origin": state.origin,
            "identity_audience": CONNECTORS_AUDIENCE,
        }))
    }

    async fn login_metadata(State(state): State<Arc<FakeState>>) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "issuer": state.origin,
            "authorization_endpoint": format!("{}/authorize", state.origin),
            "token_endpoint": format!("{}/oauth/token", state.origin),
            "access_token_endpoint": format!("{}/v1/access-token", state.origin),
            "cli_client_id": "connectors-test-client",
            "response_types_supported": ["code"],
            "grant_types_supported": ["authorization_code"],
            "code_challenge_methods_supported": ["S256"],
        }))
    }

    async fn exchange(body: Bytes) -> impl axum::response::IntoResponse {
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("grant_type=authorization_code"));
        assert!(body.contains("code=controlled-code"));
        (
            [("cache-control", "no-store"), ("pragma", "no-cache")],
            Json(serde_json::json!({
                "session": "controlled-opaque-identity-session",
                "session_type": "opaque_server_session",
                "expires_in": 86400,
                "tenant_id": "tenant-test",
                "subject": "person:test",
                "email": "test@example.test",
            })),
        )
    }

    async fn access_token(
        State(state): State<Arc<FakeState>>,
        headers: HeaderMap,
        Json(request): Json<serde_json::Value>,
    ) -> impl axum::response::IntoResponse {
        assert_eq!(
            headers.get("authorization").unwrap(),
            "Bearer controlled-opaque-identity-session"
        );
        assert_eq!(request["audience"], CONNECTORS_AUDIENCE);
        let scope = request["scope"].as_str().unwrap().to_owned();
        let issue = {
            let mut issues = state.access_issues.lock().unwrap();
            let issue = issues.entry(scope.clone()).or_default();
            *issue += 1;
            *issue
        };
        (
            [("cache-control", "no-store"), ("pragma", "no-cache")],
            Json(serde_json::json!({
                "access_token": format!("access-{scope}-{issue}"),
                "token_type": "Bearer",
                "expires_in": 300,
                "audience": CONNECTORS_AUDIENCE,
                "scope": scope,
            })),
        )
    }

    async fn mcp(
        State(state): State<Arc<FakeState>>,
        headers: HeaderMap,
        Json(request): Json<serde_json::Value>,
    ) -> axum::response::Response {
        state.mcp_bearers.lock().unwrap().push(
            headers
                .get("authorization")
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
        );
        let id = request["id"].clone();
        let result = if request["method"] == "tools/list" {
            serde_json::json!({"tools": []})
        } else {
            serde_json::json!({"content": [], "isError": false})
        };
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result,
            })),
        )
            .into_response()
    }

    struct FakeIdentity {
        state: Arc<FakeState>,
        task: tokio::task::JoinHandle<()>,
    }

    impl FakeIdentity {
        async fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let origin = format!("http://{}", listener.local_addr().unwrap());
            let state = Arc::new(FakeState {
                connectors_base: format!("{origin}/connectors/v1"),
                origin,
                access_issues: Mutex::new(BTreeMap::new()),
                mcp_bearers: Mutex::new(Vec::new()),
            });
            let application = Router::new()
                .route(
                    "/connectors/v1/.well-known/connectors-client",
                    get(discovery),
                )
                .route("/.well-known/identity-cli-login", get(login_metadata))
                .route("/oauth/token", post(exchange))
                .route("/v1/access-token", post(access_token))
                .route("/connectors/v1/mcp", post(mcp))
                .with_state(state.clone());
            let task = tokio::spawn(async move {
                axum::serve(listener, application).await.unwrap();
            });
            Self { state, task }
        }
    }

    impl Drop for FakeIdentity {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    fn complete_browser(url: &Url, _no_browser: bool) -> Result<(), IdentityError> {
        let query: BTreeMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query.get("response_type").map(String::as_str), Some("code"));
        assert_eq!(
            query.get("code_challenge_method").map(String::as_str),
            Some("S256")
        );
        assert!(query.get("nonce").is_some_and(|nonce| !nonce.is_empty()));
        let redirect = query.get("redirect_uri").unwrap().clone();
        let state = query.get("state").unwrap().clone();
        std::thread::spawn(move || {
            let callback = Url::parse(&redirect).unwrap();
            let mut stream = TcpStream::connect(("127.0.0.1", callback.port().unwrap())).unwrap();
            write!(
                stream,
                "GET /callback?code=controlled-code&state={state} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            let mut answer = String::new();
            std::io::Read::read_to_string(&mut stream, &mut answer).unwrap();
            assert!(answer.contains("200 OK"));
        });
        Ok(())
    }

    #[test]
    fn mcp_invocation_uses_only_the_invoke_scope() {
        assert_eq!(
            mcp_scope(br#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#),
            CATALOG_SCOPE
        );
        assert_eq!(
            mcp_scope(br#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tool_invoke","arguments":{}}}"#),
            INVOKE_SCOPE
        );
        assert_eq!(
            mcp_scope(br#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"tool_search","arguments":{}}}"#),
            CATALOG_SCOPE
        );
    }

    #[test]
    fn hosted_request_families_select_the_smallest_available_scope() {
        assert_eq!(
            connection_scope(&connection::ConnectionRequest::Search(
                connection::SearchRequest {
                    query: String::new(),
                    limit: 1,
                },
            )),
            CATALOG_SCOPE
        );
        assert_eq!(
            connection_scope(&connection::ConnectionRequest::ConnectSessionCreate(
                connection::ConnectSessionCreateRequest {
                    integration_ref: "slack".to_owned(),
                    label: "mine".to_owned(),
                    auth_profile: None,
                },
            )),
            CONNECTION_SELF_SCOPE
        );
        assert_eq!(
            event_scope(&event::EventRequest::Search(event::SearchRequest {
                query: "slack".to_owned(),
                limit: 1,
            })),
            EVENT_SELF_SCOPE
        );
        assert_eq!(
            event_scope(&event::EventRequest::Search(event::SearchRequest {
                query: String::new(),
                limit: 1,
            })),
            EVENT_READ_SCOPE
        );
    }

    #[test]
    fn keyring_account_contains_no_endpoint_or_principal() {
        let session = SessionMetadata {
            connectors_base: "https://connectors.example.test/api/connectors/v1".to_owned(),
            identity_origin: "https://identity.example.test".to_owned(),
            tenant_id: "tenant-test".to_owned(),
            subject: "person:test".to_owned(),
            email: Some("test@example.test".to_owned()),
            obtained_at: 1,
            idle_expires_at: 2,
        };
        let account = keyring_account(&session);
        assert!(account.starts_with("v1-"));
        assert!(!account.contains("example"));
        assert!(!account.contains("person"));
    }

    #[tokio::test]
    async fn login_separates_the_session_and_refreshes_exact_scope_tokens() {
        let fake = FakeIdentity::start().await;
        let temporary = tempfile::tempdir().unwrap();
        let metadata = temporary.path().join("identity-sessions.json");
        let store = Arc::new(MemoryStore::default());
        let session = login_with(
            &LoginOptions {
                connectors_base: fake.state.connectors_base.clone(),
                no_browser: true,
                timeout: Duration::from_secs(5),
            },
            &metadata,
            store.clone(),
            complete_browser,
        )
        .await
        .unwrap();

        let state_bytes = fs::read_to_string(&metadata).unwrap();
        assert!(!state_bytes.contains("controlled-opaque-identity-session"));
        assert_eq!(active_session_at(&metadata).unwrap(), Some(session.clone()));
        assert_eq!(store.0.lock().unwrap().len(), 1);

        let now = Arc::new(AtomicU64::new(1_000));
        let mut source = IdentityAccessTokenSource::new(session.clone(), store.clone()).unwrap();
        let clock = now.clone();
        source.clock = Arc::new(move || Ok(clock.load(Ordering::SeqCst)));
        assert_eq!(
            source.access_token(CATALOG_SCOPE).await.unwrap().as_str(),
            "access-connectors.catalog.read-1"
        );
        assert_eq!(
            source.access_token(CATALOG_SCOPE).await.unwrap().as_str(),
            "access-connectors.catalog.read-1",
            "a live token is reused"
        );
        now.store(1_271, Ordering::SeqCst);
        assert_eq!(
            source.access_token(CATALOG_SCOPE).await.unwrap().as_str(),
            "access-connectors.catalog.read-2",
            "the token is replaced inside its thirty-second refresh margin"
        );
        assert_eq!(
            source.access_token(INVOKE_SCOPE).await.unwrap().as_str(),
            "access-connectors.invoke-1",
            "a different family receives a separate exact-scope token"
        );
        assert_eq!(
            fake.state.access_issues.lock().unwrap().clone(),
            BTreeMap::from([(CATALOG_SCOPE.to_owned(), 2), (INVOKE_SCOPE.to_owned(), 1)])
        );

        let client = AuthenticatedHostedClient::from_session(session, store).unwrap();
        let (mut input_writer, input_reader) = tokio::io::duplex(4096);
        let (output_writer, mut output_reader) = tokio::io::duplex(4096);
        let bridge = tokio::spawn(bridge(client, input_reader, output_writer));
        input_writer
            .write_all(
                br#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tool_invoke","arguments":{}}}
"#,
            )
            .await
            .unwrap();
        input_writer.shutdown().await.unwrap();
        let mut output = String::new();
        output_reader.read_to_string(&mut output).await.unwrap();
        bridge.await.unwrap().unwrap();
        assert_eq!(output.lines().count(), 2);
        assert_eq!(
            fake.state.mcp_bearers.lock().unwrap().as_slice(),
            [
                "Bearer access-connectors.catalog.read-3",
                "Bearer access-connectors.invoke-2",
            ]
        );
    }
}

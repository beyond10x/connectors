#![forbid(unsafe_code)]
//! Governed outbound MCP as a generated Connector service.
//!
//! The remote server supplies protocol bytes, never authority. A reviewed [`McpServiceProfile`]
//! freezes its complete tool snapshot and assigns local operation identities, prose, and effects.
//! The ordinary service deployment overlay then supplies exposure, risk, approval, grants, and
//! opaque endpoint/credential bindings. Every HTTP exchange crosses the injected
//! [`service::EgressTransport`], and bearer material is fetched from the injected
//! [`connector_secrets::SecretStore`] for that exchange only.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use b10x_mcp_client::{connect_http_with_client, Connection};
use b10x_mcp_types::{ClientError, HttpTransportConfig, Limits, ToolCall, ToolSnapshot};
use connector_address::CredentialRef;
use connector_resolve::Request;
use connector_secrets::SecretStore;
use futures_util::stream::{self, BoxStream};
use futures_util::StreamExt as _;
use http::{HeaderName, HeaderValue};
use protocol::operation::{
    ApprovalPosture, ConnectionSummary, DescribeRequest, EffectClass, InvocationResult,
    InvokeRequest, OperationDescription, OperationError, OperationErrorCode, OperationRequest,
    OperationResult, OperationSummary, SearchRequest,
};
use rmcp::model::{ClientJsonRpcMessage, ServerJsonRpcMessage};
use rmcp::transport::streamable_http_client::{
    AuthRequiredError, InsufficientScopeError, SseError, StreamableHttpClient, StreamableHttpError,
    StreamableHttpPostResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use service::{
    BackendCapabilities, BackendReadinessError, ConnectorBackend, ConnectorServiceFactory,
    EgressHttpRequest, EgressTransport, PrincipalContext, ServiceDeployment, ServiceDispatch,
    ServiceFactoryBindError, ServiceManifest, ServiceOperation, ServiceProviderMetadata,
};
use sha2::{Digest as _, Sha256};
use sse_stream::Sse;
use tokio::sync::RwLock;

/// Immutable profile contract understood by this adapter.
pub const PROFILE_CONTRACT: &str = "b10x.connector-mcp-service.v1";
const ENDPOINT_BINDING: &str = "mcp_connection";
const BEARER_BINDING: &str = "bearer";
const MAX_PROFILE_OPERATIONS: usize = 512;

/// Provider-owned display facts copied into the reviewed local profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedProvider {
    pub display_name: String,
    pub description: String,
}

/// Local authority assigned to one exact remote MCP tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedOperation {
    pub remote_tool: String,
    pub operation_ref: String,
    pub title: String,
    pub description: String,
    pub effect: EffectClass,
}

/// Complete reviewed service definition for one frozen MCP server snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpServiceProfile {
    pub contract: String,
    pub service_ref: String,
    pub connection_ref: String,
    pub connection_label: String,
    pub provider: ReviewedProvider,
    pub snapshot: ToolSnapshot,
    pub operations: Vec<ReviewedOperation>,
}

/// Runtime values and opaque deployment references for one profile.
#[derive(Debug, Clone)]
pub struct McpRuntimeBinding {
    pub endpoint: String,
    pub endpoint_binding_ref: String,
    pub bearer: Option<McpBearerBinding>,
}

/// Existing Connector secret address selected for bearer authentication.
#[derive(Debug, Clone)]
pub struct McpBearerBinding {
    pub deployment_ref: String,
    pub credential_ref: CredentialRef,
}

/// Value-free preparation refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum McpIntegrationError {
    #[error("the reviewed MCP service profile is invalid")]
    InvalidProfile,
    #[error("the live MCP tool snapshot differs from the reviewed profile")]
    SnapshotMismatch,
    #[error("the MCP service dependency is unavailable")]
    Unavailable,
}

/// A prepared generated-service factory. Merely preparing it grants nothing; the service bundle
/// still requires a complete [`ServiceDeployment`] before this backend becomes reachable.
pub struct McpServiceFactory {
    profile: McpServiceProfile,
    binding: McpRuntimeBinding,
    connection: Arc<RwLock<Connection>>,
    secrets: Arc<dyn SecretStore>,
}

impl McpServiceFactory {
    /// Connect through Connector-owned egress, discover tools, and require the exact reviewed
    /// snapshot before returning an inert factory.
    pub async fn prepare(
        profile: McpServiceProfile,
        binding: McpRuntimeBinding,
        secrets: Arc<dyn SecretStore>,
        egress: Arc<dyn EgressTransport>,
    ) -> Result<Self, McpIntegrationError> {
        validate_profile(&profile, &binding)?;
        let limits = connector_limits();
        let client = ConnectorHttpClient {
            endpoint: binding.endpoint.clone(),
            authority_ref: binding.endpoint_binding_ref.clone(),
            credential_ref: binding
                .bearer
                .as_ref()
                .map(|bearer| bearer.credential_ref.clone()),
            secrets: Arc::clone(&secrets),
            egress,
        };
        let connection = connect_http_with_client(
            profile.snapshot.connection.clone(),
            &HttpTransportConfig {
                url: binding.endpoint.clone(),
                headers: BTreeMap::new(),
            },
            None,
            limits,
            client,
        )
        .await
        .map_err(map_prepare_error)?;
        if connection.snapshot() != &profile.snapshot {
            return Err(McpIntegrationError::SnapshotMismatch);
        }
        Ok(Self {
            profile,
            binding,
            connection: Arc::new(RwLock::new(connection)),
            secrets,
        })
    }
}

#[async_trait]
impl ConnectorServiceFactory for McpServiceFactory {
    fn manifest(&self) -> ServiceManifest {
        ServiceManifest {
            service_ref: self.profile.service_ref.clone(),
            provider: ServiceProviderMetadata {
                display_name: self.profile.provider.display_name.clone(),
                description: self.profile.provider.description.clone(),
            },
            operations: self
                .profile
                .operations
                .iter()
                .map(|reviewed| {
                    let tool = self
                        .profile
                        .snapshot
                        .tool(&reviewed.remote_tool)
                        .expect("profile validation covers every reviewed tool");
                    ServiceOperation {
                        operation_ref: reviewed.operation_ref.clone(),
                        title: reviewed.title.clone(),
                        description: reviewed.description.clone(),
                        input_schema: tool.input_schema.clone(),
                        output_schema: json!({"type": "object"}),
                        effect: reviewed.effect,
                    }
                })
                .collect(),
        }
    }

    async fn bind(
        &self,
        deployment: &ServiceDeployment,
    ) -> Result<ServiceDispatch, ServiceFactoryBindError> {
        if deployment.service_ref != self.profile.service_ref
            || deployment.operations.len() != self.profile.operations.len()
            || deployment.operations.values().any(|operation| {
                operation.endpoint_bindings
                    != BTreeMap::from([(
                        ENDPOINT_BINDING.to_owned(),
                        self.binding.endpoint_binding_ref.clone(),
                    )])
                    || operation.credential_bindings != expected_credential_bindings(&self.binding)
                    || operation.grant_refs.is_empty()
            })
        {
            return Err(ServiceFactoryBindError);
        }
        let operation_refs = self
            .profile
            .operations
            .iter()
            .map(|operation| operation.operation_ref.clone())
            .collect::<BTreeSet<_>>();
        let backend = McpBackend {
            profile: self.profile.clone(),
            deployment: deployment.clone(),
            connection: Arc::clone(&self.connection),
            secrets: Arc::clone(&self.secrets),
        };
        Ok(ServiceDispatch::new(Arc::new(backend), operation_refs))
    }
}

fn expected_credential_bindings(binding: &McpRuntimeBinding) -> BTreeMap<String, String> {
    binding
        .bearer
        .as_ref()
        .map_or_else(BTreeMap::new, |bearer| {
            BTreeMap::from([(BEARER_BINDING.to_owned(), bearer.deployment_ref.clone())])
        })
}

struct McpBackend {
    profile: McpServiceProfile,
    deployment: ServiceDeployment,
    connection: Arc<RwLock<Connection>>,
    secrets: Arc<dyn SecretStore>,
}

impl McpBackend {
    fn reviewed(&self, operation_ref: &str) -> Option<&ReviewedOperation> {
        self.profile
            .operations
            .iter()
            .find(|operation| operation.operation_ref == operation_ref)
    }

    fn connection_summary(&self) -> ConnectionSummary {
        ConnectionSummary {
            connection_ref: self.profile.connection_ref.clone(),
            label: self.profile.connection_label.clone(),
            provider: self.deployment.provider.provider_ref.clone(),
            audiences: Vec::new(),
            purpose: Some("Use this reviewed MCP service connection".to_owned()),
        }
    }

    fn description_ref(&self, context: &PrincipalContext, operation_ref: &str) -> String {
        let mut digest = Sha256::new();
        digest.update(self.profile.snapshot.sha256.as_bytes());
        digest.update(b"\0");
        digest.update(context.stable_authority_seed());
        digest.update(b"\0");
        digest.update(operation_ref.as_bytes());
        digest.update(b"\0");
        digest.update(self.profile.connection_ref.as_bytes());
        format!("description-sha256-{:x}", digest.finalize())
    }

    fn search(&self, request: &SearchRequest) -> OperationResult {
        let query = request.query.to_ascii_lowercase();
        let operations = self
            .profile
            .operations
            .iter()
            .filter(|operation| {
                query.is_empty()
                    || operation
                        .operation_ref
                        .to_ascii_lowercase()
                        .contains(&query)
                    || operation.title.to_ascii_lowercase().contains(&query)
                    || operation.description.to_ascii_lowercase().contains(&query)
            })
            .take(usize::from(request.limit))
            .map(|operation| OperationSummary {
                operation_ref: operation.operation_ref.clone(),
                title: operation.title.clone(),
                effect: operation.effect,
                approval: ApprovalPosture::Required,
                connections: vec![self.connection_summary()],
            })
            .collect();
        OperationResult::Search { operations }
    }

    fn describe(
        &self,
        context: &PrincipalContext,
        request: DescribeRequest,
    ) -> Result<OperationResult, OperationError> {
        let reviewed = self
            .reviewed(&request.operation_ref)
            .ok_or_else(operation_not_found)?;
        let tool = self
            .profile
            .snapshot
            .tool(&reviewed.remote_tool)
            .ok_or_else(operation_not_found)?;
        Ok(OperationResult::Describe(OperationDescription {
            operation_ref: reviewed.operation_ref.clone(),
            title: reviewed.title.clone(),
            description: reviewed.description.clone(),
            input_schema: tool.input_schema.clone(),
            output_schema: json!({"type": "object"}),
            effect: reviewed.effect,
            approval: ApprovalPosture::Required,
            connections: vec![self.connection_summary()],
            description_ref: self.description_ref(context, &reviewed.operation_ref),
        }))
    }

    async fn invoke(
        &self,
        context: &PrincipalContext,
        request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        let reviewed = self
            .reviewed(&request.operation_ref)
            .ok_or_else(operation_not_found)?;
        if request.connection_ref != self.profile.connection_ref {
            return Err(operation_not_granted());
        }
        if request.description_ref != self.description_ref(context, &request.operation_ref) {
            return Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "operation description lease is stale",
                false,
            ));
        }
        let tool = self
            .profile
            .snapshot
            .tool(&reviewed.remote_tool)
            .ok_or_else(operation_not_found)?;
        let validator = jsonschema::validator_for(&tool.input_schema).map_err(|_| {
            OperationError::new(
                OperationErrorCode::Unavailable,
                "the reviewed operation schema is unavailable",
                false,
            )
        })?;
        if !validator.is_valid(&request.input) {
            return Err(OperationError::new(
                OperationErrorCode::InvalidInput,
                "operation input does not satisfy the reviewed schema",
                false,
            ));
        }
        let result = self
            .connection
            .read()
            .await
            .call(
                &ToolCall {
                    name: reviewed.remote_tool.clone(),
                    arguments: request.input,
                },
                None,
            )
            .await
            .map_err(map_call_error)?;
        if result.is_error {
            return Err(OperationError::new(
                OperationErrorCode::Protocol,
                "the remote MCP tool reported failure",
                false,
            ));
        }
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(|_| operation_unavailable())?;
        Ok(OperationResult::Invoke(InvocationResult {
            operation_ref: reviewed.operation_ref.clone(),
            output: result.raw,
            connector_audit_ref: format!("audit:mcp:{}", hex::encode(random)),
            execution_ref: None,
        }))
    }
}

#[async_trait]
impl ConnectorBackend for McpBackend {
    async fn ready(&self) -> Result<(), BackendReadinessError> {
        self.secrets
            .ready()
            .await
            .map_err(|_| BackendReadinessError)
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::OPERATIONS
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Search(_) => true,
            OperationRequest::Describe(request) => self.reviewed(&request.operation_ref).is_some(),
            OperationRequest::Invoke(request) => {
                request.connection_ref == self.profile.connection_ref
                    && self.reviewed(&request.operation_ref).is_some()
            }
            _ => false,
        }
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        match request {
            OperationRequest::Search(request) => Ok(self.search(&request)),
            OperationRequest::Describe(request) => self.describe(context, request),
            OperationRequest::Invoke(request) => self.invoke(context, request).await,
            _ => Err(operation_not_found()),
        }
    }

    async fn shutdown(&self) {
        let _ = self.connection.write().await.close().await;
    }
}

fn validate_profile(
    profile: &McpServiceProfile,
    binding: &McpRuntimeBinding,
) -> Result<(), McpIntegrationError> {
    let endpoint =
        url::Url::parse(&binding.endpoint).map_err(|_| McpIntegrationError::InvalidProfile)?;
    if profile.contract != PROFILE_CONTRACT
        || profile.operations.is_empty()
        || profile.operations.len() > MAX_PROFILE_OPERATIONS
        || endpoint.scheme() != "https"
        || endpoint.host_str().is_none()
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.fragment().is_some()
        || !valid_ref(&binding.endpoint_binding_ref)
        || binding.bearer.as_ref().is_some_and(|bearer| {
            !valid_ref(&bearer.deployment_ref) || bearer.credential_ref.tenant().is_empty()
        })
    {
        return Err(McpIntegrationError::InvalidProfile);
    }
    let reconstructed = ToolSnapshot::new(
        profile.snapshot.connection.clone(),
        profile.snapshot.protocol_version.clone(),
        profile.snapshot.tools.clone(),
        connector_limits(),
    )
    .map_err(|_| McpIntegrationError::InvalidProfile)?;
    if reconstructed != profile.snapshot {
        return Err(McpIntegrationError::InvalidProfile);
    }
    let remote = profile
        .operations
        .iter()
        .map(|operation| operation.remote_tool.as_str())
        .collect::<BTreeSet<_>>();
    let local = profile
        .operations
        .iter()
        .map(|operation| operation.operation_ref.as_str())
        .collect::<BTreeSet<_>>();
    let snapshot = profile
        .snapshot
        .tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect::<BTreeSet<_>>();
    if remote.len() != profile.operations.len()
        || local.len() != profile.operations.len()
        || remote != snapshot
        || !valid_ref(&profile.service_ref)
        || !valid_ref(&profile.connection_ref)
        || !valid_text(&profile.connection_label, 256)
        || !valid_text(&profile.provider.display_name, 256)
        || !valid_text(&profile.provider.description, 4096)
        || profile.operations.iter().any(|operation| {
            !valid_ref(&operation.operation_ref)
                || !valid_text(&operation.title, 256)
                || !valid_text(&operation.description, 4096)
        })
    {
        return Err(McpIntegrationError::InvalidProfile);
    }
    Ok(())
}

fn connector_limits() -> Limits {
    Limits {
        max_frame_bytes: protocol::operation::MAX_RESULT_BYTES,
        max_tools: MAX_PROFILE_OPERATIONS,
        max_tool_descriptor_bytes: 64 * 1024,
        max_arguments_bytes: 60 * 1024,
        max_result_bytes: protocol::operation::MAX_RESULT_BYTES,
        max_pages: 128,
        request_timeout: Duration::from_secs(30),
    }
}

fn valid_ref(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn map_prepare_error(error: ClientError) -> McpIntegrationError {
    match error {
        ClientError::Configuration(_) | ClientError::Protocol(_) | ClientError::Bound { .. } => {
            McpIntegrationError::SnapshotMismatch
        }
        _ => McpIntegrationError::Unavailable,
    }
}

fn map_call_error(error: ClientError) -> OperationError {
    match error {
        ClientError::Bound { .. } => OperationError::new(
            OperationErrorCode::ResultTooLarge,
            "the remote MCP result exceeded the admitted bound",
            false,
        ),
        ClientError::AuthorizationRequired { .. } => operation_not_granted(),
        ClientError::Configuration(_) | ClientError::Protocol(_) => OperationError::new(
            OperationErrorCode::Protocol,
            "the remote MCP exchange violated the reviewed protocol",
            false,
        ),
        _ => operation_unavailable(),
    }
}

fn operation_not_found() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotFound,
        "operation was not found",
        false,
    )
}

fn operation_not_granted() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotGranted,
        "operation is not granted through this Connection",
        false,
    )
}

fn operation_unavailable() -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        "the remote MCP service is unavailable",
        true,
    )
}

#[derive(Clone)]
struct ConnectorHttpClient {
    endpoint: String,
    authority_ref: String,
    credential_ref: Option<CredentialRef>,
    secrets: Arc<dyn SecretStore>,
    egress: Arc<dyn EgressTransport>,
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("Connector MCP egress refused")]
struct ConnectorHttpError;

impl ConnectorHttpClient {
    async fn execute(
        &self,
        method: &str,
        uri: &str,
        mut headers: BTreeMap<String, String>,
        body: Option<String>,
    ) -> Result<service::EgressHttpResponse, StreamableHttpError<ConnectorHttpError>> {
        if uri != self.endpoint {
            return Err(StreamableHttpError::Client(ConnectorHttpError));
        }
        if let Some(reference) = &self.credential_ref {
            let secret = self
                .secrets
                .get(reference)
                .await
                .map_err(|_| StreamableHttpError::Client(ConnectorHttpError))?;
            headers.insert(
                "authorization".to_owned(),
                format!("Bearer {}", secret.expose_secret()),
            );
        }
        self.egress
            .execute(
                &self.authority_ref,
                EgressHttpRequest {
                    request: Request {
                        method: method.to_owned(),
                        url: uri.to_owned(),
                        headers,
                        body,
                    },
                    maximum_response_bytes: protocol::operation::MAX_RESULT_BYTES,
                    response_headers: vec![
                        "content-type".to_owned(),
                        "content-length".to_owned(),
                        "mcp-session-id".to_owned(),
                        "www-authenticate".to_owned(),
                    ],
                },
            )
            .await
            .map_err(|_| StreamableHttpError::Client(ConnectorHttpError))
    }
}

impl StreamableHttpClient for ConnectorHttpClient {
    type Error = ConnectorHttpError;

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_token: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        self.post_message_with_max_sse_event_size(
            uri,
            message,
            session_id,
            auth_token,
            custom_headers,
            protocol::operation::MAX_RESULT_BYTES,
        )
        .await
    }

    async fn post_message_with_max_sse_event_size(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_token: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
        max_sse_event_size: usize,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        let mut headers = request_headers(custom_headers)?;
        headers.insert(
            "accept".to_owned(),
            "text/event-stream, application/json".to_owned(),
        );
        headers.insert("content-type".to_owned(), "application/json".to_owned());
        if let Some(session_id) = &session_id {
            headers.insert("mcp-session-id".to_owned(), session_id.to_string());
        }
        if let Some(auth_token) = auth_token {
            if self.credential_ref.is_some() {
                return Err(StreamableHttpError::Client(ConnectorHttpError));
            }
            headers.insert("authorization".to_owned(), format!("Bearer {auth_token}"));
        }
        let body = serde_json::to_string(&message)?;
        let response = self.execute("POST", &uri, headers, Some(body)).await?;
        classify_auth(&response)?;
        let session = response.header("mcp-session-id").map(str::to_owned);
        if matches!(response.status, 202 | 204)
            || (response.status == 200 && response.body.is_empty())
        {
            return Ok(StreamableHttpPostResponse::Accepted);
        }
        if response.status == 404 && session_id.is_some() {
            return Err(StreamableHttpError::SessionExpired);
        }
        if !(200..300).contains(&response.status) {
            if content_type(&response) == Some("application/json") {
                if let Ok(message) = serde_json::from_slice::<ServerJsonRpcMessage>(&response.body)
                {
                    return Ok(StreamableHttpPostResponse::Json(message, session));
                }
            }
            if is_discover(&message) && session_id.is_none() {
                return legacy_discover_refusal(&message);
            }
            return Err(StreamableHttpError::UnexpectedServerResponse(
                "MCP endpoint returned an unsuccessful status".into(),
            ));
        }
        match content_type(&response) {
            Some("application/json") => {
                let message = serde_json::from_slice(&response.body)?;
                Ok(StreamableHttpPostResponse::Json(message, session))
            }
            Some("text/event-stream") => {
                let events = parse_sse(&response.body, max_sse_event_size)?;
                Ok(StreamableHttpPostResponse::Sse(
                    stream::iter(events.into_iter().map(Ok)).boxed(),
                    session,
                ))
            }
            _ => Err(StreamableHttpError::UnexpectedContentType(
                response.header("content-type").map(str::to_owned),
            )),
        }
    }

    async fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        auth_token: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<(), StreamableHttpError<Self::Error>> {
        let mut headers = request_headers(custom_headers)?;
        headers.insert("mcp-session-id".to_owned(), session_id.to_string());
        if let Some(auth_token) = auth_token {
            if self.credential_ref.is_some() {
                return Err(StreamableHttpError::Client(ConnectorHttpError));
            }
            headers.insert("authorization".to_owned(), format!("Bearer {auth_token}"));
        }
        let response = self.execute("DELETE", &uri, headers, None).await?;
        classify_auth(&response)?;
        if response.status == 405 || (200..300).contains(&response.status) {
            Ok(())
        } else if response.status == 404 {
            Err(StreamableHttpError::SessionExpired)
        } else {
            Err(StreamableHttpError::UnexpectedServerResponse(
                "MCP endpoint refused session deletion".into(),
            ))
        }
    }

    async fn get_stream(
        &self,
        _uri: Arc<str>,
        _session_id: Option<Arc<str>>,
        _last_event_id: Option<String>,
        _auth_token: Option<String>,
        _custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<BoxStream<'static, Result<Sse, SseError>>, StreamableHttpError<Self::Error>> {
        // EgressTransport deliberately exposes bounded exchanges, not an ambient long-lived
        // response body. Tools-only request/response remains supported; server-pushed notifications
        // do not create a second socket capability.
        Err(StreamableHttpError::ServerDoesNotSupportSse)
    }
}

fn request_headers(
    custom: HashMap<HeaderName, HeaderValue>,
) -> Result<BTreeMap<String, String>, StreamableHttpError<ConnectorHttpError>> {
    let mut headers = BTreeMap::new();
    for (name, value) in custom {
        if ["accept", "mcp-session-id", "last-event-id", "authorization"]
            .iter()
            .any(|reserved| name.as_str().eq_ignore_ascii_case(reserved))
        {
            return Err(StreamableHttpError::ReservedHeaderConflict(
                name.to_string(),
            ));
        }
        let value = value
            .to_str()
            .map_err(|_| StreamableHttpError::Client(ConnectorHttpError))?;
        headers.insert(name.as_str().to_owned(), value.to_owned());
    }
    Ok(headers)
}

fn classify_auth(
    response: &service::EgressHttpResponse,
) -> Result<(), StreamableHttpError<ConnectorHttpError>> {
    if response.status == 401 {
        let challenge = response
            .header("www-authenticate")
            .unwrap_or("Bearer")
            .to_owned();
        return Err(StreamableHttpError::AuthRequired(AuthRequiredError::new(
            challenge,
        )));
    }
    if response.status == 403 {
        let challenge = response
            .header("www-authenticate")
            .unwrap_or("Bearer")
            .to_owned();
        let scope = challenge_scope(&challenge);
        return Err(StreamableHttpError::InsufficientScope(
            InsufficientScopeError::new(challenge, scope),
        ));
    }
    Ok(())
}

fn challenge_scope(challenge: &str) -> Option<String> {
    let lower = challenge.to_ascii_lowercase();
    let start = lower.find("scope=")? + "scope=".len();
    let value = &challenge[start..];
    if let Some(value) = value.strip_prefix('"') {
        value.find('"').map(|end| value[..end].to_owned())
    } else {
        let end = value
            .find(|character: char| character == ',' || character.is_whitespace())
            .unwrap_or(value.len());
        (end > 0).then(|| value[..end].to_owned())
    }
}

fn content_type(response: &service::EgressHttpResponse) -> Option<&str> {
    response
        .header("content-type")
        .and_then(|value| value.split(';').next())
        .map(str::trim)
}

fn is_discover(message: &ClientJsonRpcMessage) -> bool {
    serde_json::to_value(message)
        .ok()
        .and_then(|value| {
            value
                .get("method")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .is_some_and(|method| method == "server/discover")
}

fn legacy_discover_refusal(
    message: &ClientJsonRpcMessage,
) -> Result<StreamableHttpPostResponse, StreamableHttpError<ConnectorHttpError>> {
    let value = serde_json::to_value(message)?;
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    let response = json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": -32600, "message": "server/discover is unsupported"}
    });
    Ok(StreamableHttpPostResponse::Json(
        serde_json::from_value(response)?,
        None,
    ))
}

fn parse_sse(
    body: &[u8],
    maximum_event_bytes: usize,
) -> Result<Vec<Sse>, StreamableHttpError<ConnectorHttpError>> {
    let text = std::str::from_utf8(body)
        .map_err(|_| StreamableHttpError::UnexpectedServerResponse("invalid SSE text".into()))?;
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut events = Vec::new();
    for block in normalized.split("\n\n").filter(|block| !block.is_empty()) {
        if block.len() > maximum_event_bytes {
            return Err(StreamableHttpError::UnexpectedServerResponse(
                "SSE event exceeded the admitted bound".into(),
            ));
        }
        let mut event = Sse::default();
        let mut data = Vec::new();
        for line in block.lines() {
            if line.starts_with(':') {
                continue;
            }
            let (field, value) = line.split_once(':').map_or((line, ""), |(field, value)| {
                (field, value.strip_prefix(' ').unwrap_or(value))
            });
            match field {
                "event" => event.event = Some(value.to_owned()),
                "data" => data.push(value),
                "id" if !value.contains('\0') => event.id = Some(value.to_owned()),
                "retry" => event.retry = value.parse().ok(),
                _ => {}
            }
        }
        if !data.is_empty() {
            event.data = Some(data.join("\n"));
        }
        events.push(event);
    }
    Ok(events)
}

#[cfg(test)]
mod tests;

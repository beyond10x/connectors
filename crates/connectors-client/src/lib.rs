#![forbid(unsafe_code)]

//! Typed, bounded clients and provider-neutral workflows for Connector control protocols.

use std::fs;
use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub use protocol::{approval, catalog, datasource, endpoint, git_fetch, operation};
use protocol::{connection, event};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::UnixStream;
use url::Url;
use zeroize::Zeroizing;

mod admin;
#[path = "endpoint.rs"]
mod endpoint_client;
mod git_fetch_client;
mod hosted_catalog;
mod identity;
mod lifecycle;
mod local_setup;
mod model;
mod personal_oauth;
mod remediation;
mod response;

pub use admin::AdminIdentityClient;
pub use identity::{
    active_session_metadata, login, logout, run_mcp_bridge, AuthenticatedHostedClient,
    AuthenticatedHostedError, IdentityError, LoginOptions, SessionMetadata,
    METADATA_VERSION as SESSION_METADATA_VERSION,
};
pub use model::{
    AdminAuthMetadata, AdminConfigurationField, AdminCredentialState, AdminCredentialStatus,
    AdminCredentialWrite, AdminIntegrationStatus, AdminLoginMetadata, AdminStatus,
    CandidateActivationOutcome, ClientError, GitFetchSession, MaterializationOutcome,
    PendingConnection, PendingPersonalOAuth, PendingRemediation, PersonalOAuthInstructions,
    RedeemedSubscription, SubscriptionLease, SubscriptionOAuthStart, SubscriptionStatus,
};
use model::{
    CompleteSubscriptionOAuthRequest, ConnectSubscriptionRequest, CreateSubscriptionLeaseRequest,
    RedeemSubscriptionLeaseRequest, RedeemedSubscriptionResponse, SubscriptionLeaseResponse,
};
use response::{
    validate_connection_response, validate_datasource_response, validate_event_response,
    validate_operation_response,
};

const COMPLETION_RESPONSE_BYTES: usize = 1024;
const IDENTITY_BEARER_BYTES: usize = 512;
const SUBSCRIPTION_RESPONSE_BYTES: usize = 20 * 1024;

/// Client for the owner-permissioned personal-local Unix-socket binding.
#[derive(Debug, Clone)]
pub struct LocalClient {
    socket: PathBuf,
}

impl LocalClient {
    /// Selects one explicit local Connector control socket.
    #[must_use]
    pub fn new(socket: impl Into<PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }

    /// Sends one operation request and validates its correlated response.
    /// Sends one operation request using the coordinated v3 identity, with no fallback.
    pub async fn operation(
        &self,
        context: &operation::OwnerContext,
        request: operation::OperationRequest,
    ) -> Result<operation::v3::ResponseEnvelope, ClientError> {
        self.operation_v3(context, request).await
    }

    /// Explicit predecessor selection preserves the complete v2 client contract.
    pub async fn operation_v2(
        &self,
        context: &operation::OwnerContext,
        request: operation::OperationRequest,
    ) -> Result<operation::ResponseEnvelope, ClientError> {
        let request_id = request_id();
        let envelope = operation::RequestEnvelope {
            protocol: operation::CONTRACT.to_owned(),
            request_id: request_id.clone(),
            context: context.clone(),
            request,
        };
        envelope
            .validate()
            .map_err(|error| ClientError::InvalidRequest(error.to_string()))?;
        let response = self
            .exchange(
                &envelope,
                operation::MAX_FRAME_BYTES,
                operation::MAX_RESULT_BYTES,
            )
            .await?;
        validate_operation_response(response, &request_id)
    }

    /// Sends one Connection request and validates its correlated response.
    pub async fn connection(
        &self,
        context: &operation::OwnerContext,
        request: connection::ConnectionRequest,
    ) -> Result<connection::ResponseEnvelope, ClientError> {
        let request_id = request_id();
        let envelope = connection::RequestEnvelope {
            protocol: connection::CONTRACT.to_owned(),
            request_id: request_id.clone(),
            context: context.clone(),
            request,
        };
        envelope
            .validate()
            .map_err(|error| ClientError::InvalidRequest(error.to_string()))?;
        let response = self
            .exchange(
                &envelope,
                connection::MAX_FRAME_BYTES,
                connection::MAX_RESPONSE_BYTES,
            )
            .await?;
        validate_connection_response(response, &request_id)
    }

    /// Begin a provider credential flow and return its validated one-use endpoint.
    pub async fn begin_connect_session(
        &self,
        context: &operation::OwnerContext,
        integration_ref: String,
        label: String,
    ) -> Result<PendingConnection, ClientError> {
        let result = self
            .connection_result(
                context,
                connection::ConnectionRequest::ConnectSessionCreate(
                    connection::ConnectSessionCreateRequest {
                        integration_ref,
                        label,
                        auth_profile: None,
                    },
                ),
            )
            .await?;
        let connection::ConnectionResult::ConnectSessionCreate(created) = result else {
            return Err(ClientError::InvalidResponse);
        };
        if created.state != connection::ConnectSessionState::Pending {
            return Err(ClientError::InvalidResponse);
        }
        Ok(PendingConnection {
            session_ref: created.connect_session_ref,
            completion_endpoint: created
                .completion_endpoint
                .map(PathBuf::from)
                .ok_or(ClientError::InvalidResponse)?,
        })
    }

    /// Confirm a submitted Connect Session and wait briefly for its Connection to become callable.
    pub async fn finish_connect_session(
        &self,
        context: &operation::OwnerContext,
        session_ref: String,
    ) -> Result<connection::ConnectionDescription, ClientError> {
        let result = self
            .connection_result(
                context,
                connection::ConnectionRequest::ConnectSessionStatus(
                    connection::ConnectSessionStatusRequest {
                        connect_session_ref: session_ref,
                    },
                ),
            )
            .await?;
        let connection::ConnectionResult::ConnectSessionStatus(completed) = result else {
            return Err(ClientError::InvalidResponse);
        };
        if completed.state != connection::ConnectSessionState::Completed {
            return Err(ClientError::CompletionRefused);
        }
        let connection_ref = completed
            .connection_ref
            .ok_or(ClientError::InvalidResponse)?;
        for _ in 0..20 {
            let result = self
                .connection_result(
                    context,
                    connection::ConnectionRequest::Describe(connection::DescribeRequest {
                        connection_ref: connection_ref.clone(),
                    }),
                )
                .await?;
            let connection::ConnectionResult::Describe(description) = result else {
                return Err(ClientError::InvalidResponse);
            };
            if description.summary.state == connection::ConnectionState::Callable {
                return Ok(description);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Err(ClientError::InvalidResponse)
    }

    /// Select and activate one exact Integration candidate, then return its initial observations.
    pub async fn activate_candidate(
        &self,
        context: &operation::OwnerContext,
        integration_ref: String,
        label: Option<String>,
        exact_title: Option<String>,
    ) -> Result<CandidateActivationOutcome, ClientError> {
        let result = self
            .connection_result(
                context,
                connection::ConnectionRequest::CandidateSearch(
                    connection::CandidateSearchRequest {
                        integration_ref,
                        query: exact_title.clone().unwrap_or_default(),
                        limit: connection::MAX_SEARCH_RESULTS,
                    },
                ),
            )
            .await?;
        let connection::ConnectionResult::CandidateSearch { candidates } = result else {
            return Err(ClientError::InvalidResponse);
        };
        if exact_title.is_none() && candidates.len() != 1 {
            return Ok(CandidateActivationOutcome::SelectionRequired(candidates));
        }
        let candidate = if let Some(exact) = exact_title {
            candidates
                .into_iter()
                .find(|candidate| candidate.title == exact)
                .ok_or_else(|| {
                    ClientError::ConnectionRefused(
                        "the exact candidate title was not detected".to_owned(),
                    )
                })?
        } else {
            candidates
                .into_iter()
                .next()
                .ok_or(ClientError::InvalidResponse)?
        };
        let result = self
            .connection_result(
                context,
                connection::ConnectionRequest::CandidateActivate(
                    connection::CandidateActivateRequest {
                        candidate_ref: candidate.candidate_ref,
                        label: label.unwrap_or(candidate.title),
                    },
                ),
            )
            .await?;
        let connection::ConnectionResult::CandidateActivate(connection) = result else {
            return Err(ClientError::InvalidResponse);
        };
        let observations = self
            .observations(context, connection.summary.connection_ref.clone())
            .await?;
        Ok(CandidateActivationOutcome::Connected {
            connection,
            observations,
        })
    }

    /// Read the current bounded discovery observations for a source Connection.
    pub async fn observations(
        &self,
        context: &operation::OwnerContext,
        source_connection_ref: String,
    ) -> Result<Vec<connection::DiscoveryObservationSummary>, ClientError> {
        let result = self
            .connection_result(
                context,
                connection::ConnectionRequest::ObservationSearch(
                    connection::ObservationSearchRequest {
                        source_connection_ref,
                        query: String::new(),
                        limit: connection::MAX_SEARCH_RESULTS,
                    },
                ),
            )
            .await?;
        let connection::ConnectionResult::ObservationSearch { observations } = result else {
            return Err(ClientError::InvalidResponse);
        };
        Ok(observations)
    }

    /// Materialize one previously returned discovery observation.
    pub async fn materialize(
        &self,
        context: &operation::OwnerContext,
        observation_ref: String,
    ) -> Result<connection::ConnectionDescription, ClientError> {
        let result = self
            .connection_result(
                context,
                connection::ConnectionRequest::Materialize(connection::MaterializeRequest {
                    observation_ref,
                }),
            )
            .await?;
        let connection::ConnectionResult::Materialize(connection) = result else {
            return Err(ClientError::InvalidResponse);
        };
        Ok(connection)
    }

    /// Materialize every supported observation, preserving typed Grant refusals as a count.
    pub async fn materialize_admitted(
        &self,
        context: &operation::OwnerContext,
        observations: Vec<connection::DiscoveryObservationSummary>,
    ) -> Result<MaterializationOutcome, ClientError> {
        let mut outcome = MaterializationOutcome {
            connections: Vec::new(),
            unsupported: 0,
            not_granted: 0,
        };
        for observation in observations {
            if observation.target_provider_ref.is_none()
                || observation.state == connection::DiscoveryObservationState::Unsupported
            {
                outcome.unsupported += 1;
                continue;
            }
            let response = self
                .connection(
                    context,
                    connection::ConnectionRequest::Materialize(connection::MaterializeRequest {
                        observation_ref: observation.observation_ref,
                    }),
                )
                .await?;
            match (response.status, response.response, response.error) {
                (
                    connection::ResponseStatus::Ok,
                    Some(connection::ConnectionResult::Materialize(description)),
                    None,
                ) => outcome.connections.push(description.summary),
                (
                    connection::ResponseStatus::Error,
                    None,
                    Some(connection::ConnectionError {
                        code: connection::ConnectionErrorCode::NotGranted,
                        ..
                    }),
                ) => outcome.not_granted += 1,
                (connection::ResponseStatus::Error, None, Some(error)) => {
                    return Err(ClientError::ConnectionRefused(error.to_string()));
                }
                _ => return Err(ClientError::InvalidResponse),
            }
        }
        Ok(outcome)
    }

    async fn connection_result(
        &self,
        context: &operation::OwnerContext,
        request: connection::ConnectionRequest,
    ) -> Result<connection::ConnectionResult, ClientError> {
        let response = self.connection(context, request).await?;
        match response.status {
            connection::ResponseStatus::Ok => response.response.ok_or(ClientError::InvalidResponse),
            connection::ResponseStatus::Error => Err(ClientError::ConnectionRefused(
                response
                    .error
                    .ok_or(ClientError::InvalidResponse)?
                    .to_string(),
            )),
        }
    }

    /// Sends one event request and validates its correlated response.
    pub async fn event(
        &self,
        context: &operation::OwnerContext,
        request: event::EventRequest,
    ) -> Result<event::ResponseEnvelope, ClientError> {
        let request_id = request_id();
        let envelope = event::RequestEnvelope {
            protocol: event::CONTRACT.to_owned(),
            request_id: request_id.clone(),
            context: context.clone(),
            request,
        };
        envelope
            .validate()
            .map_err(|error| ClientError::InvalidRequest(error.to_string()))?;
        let response = self
            .exchange(&envelope, event::MAX_FRAME_BYTES, event::MAX_RESPONSE_BYTES)
            .await?;
        validate_event_response(response, &request_id)
    }

    async fn exchange<T: Serialize, R: DeserializeOwned>(
        &self,
        envelope: &T,
        request_bound: usize,
        response_bound: usize,
    ) -> Result<R, ClientError> {
        let mut bytes = serde_json::to_vec(envelope)?;
        if bytes.len() > request_bound {
            return Err(ClientError::InvalidRequest(
                "request frame exceeds the protocol bound".to_owned(),
            ));
        }
        bytes.push(b'\n');
        let mut stream = UnixStream::connect(&self.socket).await?;
        stream.write_all(&bytes).await?;
        stream.shutdown().await?;
        let mut response = String::new();
        BufReader::new(stream)
            .take((response_bound + 1) as u64)
            .read_line(&mut response)
            .await?;
        if response.is_empty() || response.len() > response_bound {
            return Err(ClientError::InvalidResponse);
        }
        Ok(serde_json::from_str(&response)?)
    }
}

/// Client for the Identity-authenticated hosted HTTP binding.
#[derive(Clone)]
pub struct HostedClient {
    http: reqwest::Client,
    base: Url,
    catalog: Url,
    operations: Url,
    connections: Url,
    events: Url,
    datasources: Url,
    git_fetch_sessions: Url,
    approvals: Url,
    subscription_credential: Url,
    subscription_leases: Url,
    subscription_oauth_start: Url,
    subscription_oauth_complete: Url,
    mcp: Url,
    admin_auth_metadata: Url,
    admin_integrations: Url,
}

impl HostedClient {
    /// Creates a bounded client for an exact API base such as
    /// `https://connectors.example/api/connectors/v1`.
    pub fn new(base: &str) -> Result<Self, ClientError> {
        let base = validated_hosted_base(base)?;
        let builder = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(35));
        let builder = if base.scheme() == "https" {
            builder.https_only(true)
        } else {
            builder
        };
        let http = builder
            .build()
            .map_err(|_| ClientError::HostedUnavailable)?;
        Ok(Self::from_parts(base, http))
    }

    /// Sends one hosted operation request with an ephemeral Identity bearer.
    /// Sends one hosted operation using the coordinated v3 identity, with no fallback.
    pub async fn operation(
        &self,
        bearer: &str,
        context: &operation::OwnerContext,
        request: operation::OperationRequest,
    ) -> Result<operation::v3::ResponseEnvelope, ClientError> {
        self.operation_v3(bearer, context, request).await
    }

    /// Explicit predecessor selection preserves the complete v2 client contract.
    pub async fn operation_v2(
        &self,
        bearer: &str,
        context: &operation::OwnerContext,
        request: operation::OperationRequest,
    ) -> Result<operation::ResponseEnvelope, ClientError> {
        require_bearer(bearer)?;
        let request_id = request_id();
        let envelope = operation::RequestEnvelope {
            protocol: operation::CONTRACT.to_owned(),
            request_id: request_id.clone(),
            context: context.clone(),
            request,
        };
        envelope
            .validate()
            .map_err(|error| ClientError::InvalidRequest(error.to_string()))?;
        let response = self
            .exchange(
                &self.operations,
                bearer,
                &envelope,
                operation::MAX_FRAME_BYTES,
                operation::MAX_RESULT_BYTES,
            )
            .await?;
        validate_operation_response(response, &request_id)
    }

    /// Ask hosted Connectors to issue one human-approved, exact-input, one-time proof.
    pub async fn issue_approval(
        &self,
        bearer: &str,
        context: &operation::OwnerContext,
        request: approval::IssueRequest,
    ) -> Result<approval::IssuedApproval, ClientError> {
        require_bearer(bearer)?;
        let request_id = request_id();
        let envelope = approval::RequestEnvelope {
            protocol: approval::CONTRACT.to_owned(),
            request_id: request_id.clone(),
            context: context.clone(),
            request,
        };
        envelope
            .validate()
            .map_err(|error| ClientError::InvalidRequest(error.to_string()))?;
        let body = serde_json::to_vec(&envelope)?;
        let issued: approval::IssuedApproval = self
            .confidential_exchange(
                reqwest::Method::POST,
                self.approvals.clone(),
                bearer,
                Some(body),
                approval::MAX_FRAME_BYTES,
                approval::MAX_RESPONSE_BYTES,
            )
            .await?;
        if !issued.validate(&request_id) {
            return Err(ClientError::InvalidResponse);
        }
        Ok(issued)
    }

    /// Sends one hosted Connection request with an ephemeral Identity bearer.
    pub async fn connection(
        &self,
        bearer: &str,
        context: &operation::OwnerContext,
        request: connection::ConnectionRequest,
    ) -> Result<connection::ResponseEnvelope, ClientError> {
        require_bearer(bearer)?;
        let request_id = request_id();
        let envelope = connection::RequestEnvelope {
            protocol: connection::CONTRACT.to_owned(),
            request_id: request_id.clone(),
            context: context.clone(),
            request,
        };
        envelope
            .validate()
            .map_err(|error| ClientError::InvalidRequest(error.to_string()))?;
        let response = self
            .exchange(
                &self.connections,
                bearer,
                &envelope,
                connection::MAX_FRAME_BYTES,
                connection::MAX_RESPONSE_BYTES,
            )
            .await?;
        validate_connection_response(response, &request_id)
    }

    /// Sends one hosted Event request with an ephemeral Identity bearer.
    pub async fn event(
        &self,
        bearer: &str,
        context: &operation::OwnerContext,
        request: event::EventRequest,
    ) -> Result<event::ResponseEnvelope, ClientError> {
        require_bearer(bearer)?;
        let request_id = request_id();
        let envelope = event::RequestEnvelope {
            protocol: event::CONTRACT.to_owned(),
            request_id: request_id.clone(),
            context: context.clone(),
            request,
        };
        envelope
            .validate()
            .map_err(|error| ClientError::InvalidRequest(error.to_string()))?;
        let response = self
            .exchange(
                &self.events,
                bearer,
                &envelope,
                event::MAX_FRAME_BYTES,
                event::MAX_RESPONSE_BYTES,
            )
            .await?;
        validate_event_response(response, &request_id)
    }

    /// Sends one hosted, read-only datasource request with an ephemeral Identity bearer.
    pub async fn datasource(
        &self,
        bearer: &str,
        context: &operation::OwnerContext,
        request: datasource::DatasourceRequest,
    ) -> Result<datasource::ResponseEnvelope, ClientError> {
        require_bearer(bearer)?;
        let request_id = request_id();
        let envelope = datasource::RequestEnvelope {
            protocol: datasource::CONTRACT.to_owned(),
            request_id: request_id.clone(),
            context: context.clone(),
            request,
        };
        envelope
            .validate()
            .map_err(|error| ClientError::InvalidRequest(error.to_string()))?;
        let response = self
            .exchange(
                &self.datasources,
                bearer,
                &envelope,
                datasource::MAX_FRAME_BYTES,
                datasource::MAX_RESULT_BYTES,
            )
            .await?;
        validate_datasource_response(response, &request_id)
    }

    /// Reports whether the authenticated person has connected a Claude Code subscription. The
    /// answer contains no credential material.
    pub async fn claude_code_subscription_status(
        &self,
        identity_bearer: &str,
    ) -> Result<SubscriptionStatus, ClientError> {
        require_bearer(identity_bearer)?;
        self.subscription_exchange(
            reqwest::Method::GET,
            self.subscription_credential.clone(),
            identity_bearer,
            None,
        )
        .await
    }

    /// Replaces the authenticated person's Claude Code subscription credential. The value is
    /// consumed by this request and is never retained by the client.
    pub async fn connect_claude_code_subscription(
        &self,
        identity_bearer: &str,
        credential: Zeroizing<String>,
    ) -> Result<SubscriptionStatus, ClientError> {
        require_bearer(identity_bearer)?;
        let body = serde_json::to_vec(&ConnectSubscriptionRequest {
            credential: credential.as_str(),
        })?;
        self.subscription_exchange(
            reqwest::Method::PUT,
            self.subscription_credential.clone(),
            identity_bearer,
            Some(body),
        )
        .await
    }

    /// Starts Claude's public-client PKCE flow without exposing the verifier to this client.
    pub async fn start_claude_code_subscription_oauth(
        &self,
        identity_bearer: &str,
    ) -> Result<SubscriptionOAuthStart, ClientError> {
        require_bearer(identity_bearer)?;
        self.subscription_exchange(
            reqwest::Method::POST,
            self.subscription_oauth_start.clone(),
            identity_bearer,
            None,
        )
        .await
    }

    /// Completes one pending PKCE flow. The provider's complete `code#state` manual result is
    /// wiped on drop; Connectors validates the state and exchanges only the code component.
    pub async fn complete_claude_code_subscription_oauth(
        &self,
        identity_bearer: &str,
        flow_id: &str,
        code: Zeroizing<String>,
    ) -> Result<SubscriptionStatus, ClientError> {
        require_bearer(identity_bearer)?;
        let body = serde_json::to_vec(&CompleteSubscriptionOAuthRequest {
            flow_id,
            code: code.as_str(),
        })?;
        self.subscription_exchange(
            reqwest::Method::POST,
            self.subscription_oauth_complete.clone(),
            identity_bearer,
            Some(body),
        )
        .await
    }

    /// Deletes the authenticated person's Claude Code subscription credential and revokes every
    /// live in-process lease over it.
    pub async fn disconnect_claude_code_subscription(
        &self,
        identity_bearer: &str,
    ) -> Result<SubscriptionStatus, ClientError> {
        require_bearer(identity_bearer)?;
        self.subscription_exchange(
            reqwest::Method::DELETE,
            self.subscription_credential.clone(),
            identity_bearer,
            None,
        )
        .await
    }

    /// Creates one finite-use credential capability bound to an exact Harness attempt.
    pub async fn lease_claude_code_subscription(
        &self,
        identity_bearer: &str,
        attempt_id: &str,
        ttl: Duration,
        maximum_uses: u16,
    ) -> Result<SubscriptionLease, ClientError> {
        require_bearer(identity_bearer)?;
        let body = serde_json::to_vec(&CreateSubscriptionLeaseRequest {
            attempt_id,
            ttl_seconds: ttl.as_secs(),
            maximum_uses,
        })?;
        let response: SubscriptionLeaseResponse = self
            .subscription_exchange(
                reqwest::Method::POST,
                self.subscription_leases.clone(),
                identity_bearer,
                Some(body),
            )
            .await?;
        Ok(SubscriptionLease {
            lease_id: response.lease_id,
            token: Zeroizing::new(response.lease_token),
            expires_at: response.expires_at,
        })
    }

    /// Consumes one use of an attempt capability and returns the provider credential directly to
    /// the caller's provider adapter. The returned allocation is wiped on drop.
    pub async fn redeem_claude_code_subscription(
        &self,
        lease: &SubscriptionLease,
        attempt_id: &str,
    ) -> Result<RedeemedSubscription, ClientError> {
        require_lease_id(&lease.lease_id)?;
        let endpoint = endpoint(
            &self.base,
            &format!("subscription-leases/{}/redeem", lease.lease_id),
        );
        let body = serde_json::to_vec(&RedeemSubscriptionLeaseRequest { attempt_id })?;
        let response: RedeemedSubscriptionResponse = self
            .subscription_exchange(
                reqwest::Method::POST,
                endpoint,
                lease.expose_at_redemption_boundary(),
                Some(body),
            )
            .await?;
        if response.kind != "oauth" {
            return Err(ClientError::InvalidResponse);
        }
        Ok(RedeemedSubscription {
            credential: Zeroizing::new(response.credential),
            kind: response.kind,
        })
    }

    fn from_parts(base: Url, http: reqwest::Client) -> Self {
        Self {
            base: base.clone(),
            catalog: endpoint(&base, "catalog"),
            operations: endpoint(&base, "operations"),
            connections: endpoint(&base, "connections"),
            events: endpoint(&base, "events"),
            datasources: endpoint(&base, "datasources"),
            git_fetch_sessions: endpoint(&base, "git-fetch-sessions"),
            approvals: endpoint(&base, "approvals"),
            subscription_credential: endpoint(&base, "subscription-credentials/claude-code"),
            subscription_leases: endpoint(&base, "subscription-credentials/claude-code/leases"),
            subscription_oauth_start: endpoint(
                &base,
                "subscription-credentials/claude-code/oauth/start",
            ),
            subscription_oauth_complete: endpoint(
                &base,
                "subscription-credentials/claude-code/oauth/complete",
            ),
            mcp: endpoint(&base, "mcp"),
            admin_auth_metadata: endpoint(&base, "admin/auth-metadata"),
            admin_integrations: endpoint(&base, "admin/integrations"),
            http,
        }
    }

    async fn mcp_exchange(
        &self,
        bearer: &str,
        body: &[u8],
    ) -> Result<Option<Vec<u8>>, ClientError> {
        require_bearer(bearer)?;
        if body.is_empty() || body.len() > operation::MAX_FRAME_BYTES {
            return Err(ClientError::InvalidRequest(
                "MCP frame exceeds the protocol bound".to_owned(),
            ));
        }
        let mut response = self
            .http
            .post(self.mcp.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .bearer_auth(bearer)
            .body(body.to_vec())
            .send()
            .await
            .map_err(|_| ClientError::HostedUnavailable)?;
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => return Err(ClientError::HostedAuthentication),
            reqwest::StatusCode::FORBIDDEN => return Err(ClientError::HostedNotGranted),
            reqwest::StatusCode::ACCEPTED => return Ok(None),
            status if !status.is_success() => return Err(ClientError::HostedUnavailable),
            _ => {}
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| ClientError::HostedUnavailable)?
        {
            if bytes.len() + chunk.len() > operation::MAX_RESULT_BYTES {
                return Err(ClientError::InvalidResponse);
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.is_empty() {
            return Err(ClientError::InvalidResponse);
        }
        serde_json::from_slice::<serde_json::Value>(&bytes)?;
        Ok(Some(bytes))
    }

    async fn subscription_exchange<R: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        endpoint: Url,
        bearer: &str,
        body: Option<Vec<u8>>,
    ) -> Result<R, ClientError> {
        self.confidential_exchange(
            method,
            endpoint,
            bearer,
            body,
            SUBSCRIPTION_RESPONSE_BYTES,
            SUBSCRIPTION_RESPONSE_BYTES,
        )
        .await
    }

    async fn confidential_exchange<R: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        endpoint: Url,
        bearer: &str,
        body: Option<Vec<u8>>,
        request_bound: usize,
        response_bound: usize,
    ) -> Result<R, ClientError> {
        require_bearer(bearer)?;
        let mut request = self.http.request(method, endpoint).bearer_auth(bearer);
        if let Some(body) = body {
            if body.len() > request_bound {
                return Err(ClientError::InvalidRequest(
                    "confidential request exceeds the protocol bound".to_owned(),
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
            reqwest::StatusCode::UNAUTHORIZED => return Err(ClientError::HostedAuthentication),
            reqwest::StatusCode::FORBIDDEN => return Err(ClientError::HostedNotGranted),
            status if status.is_client_error() => {
                return Err(ClientError::SubscriptionRefused(status.as_u16()));
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

    async fn exchange<T: Serialize, R: DeserializeOwned>(
        &self,
        endpoint: &Url,
        bearer: &str,
        envelope: &T,
        request_bound: usize,
        response_bound: usize,
    ) -> Result<R, ClientError> {
        let body = serde_json::to_vec(envelope)?;
        if body.len() > request_bound {
            return Err(ClientError::InvalidRequest(
                "request frame exceeds the protocol bound".to_owned(),
            ));
        }
        let mut response = self
            .http
            .post(endpoint.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .bearer_auth(bearer)
            .body(body)
            .send()
            .await
            .map_err(|_| ClientError::HostedUnavailable)?;
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => return Err(ClientError::HostedAuthentication),
            reqwest::StatusCode::FORBIDDEN => return Err(ClientError::HostedNotGranted),
            status if !status.is_success() => return Err(ClientError::HostedUnavailable),
            _ => {}
        }
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
}

/// A completion socket whose location, ownership, and mode were checked before prompting.
#[derive(Debug)]
pub struct CompletionEndpoint {
    path: PathBuf,
}

impl CompletionEndpoint {
    /// Validates one Connector-issued endpoint beneath `<state-root>/connect-sessions`.
    pub fn validate(state_root: &Path, endpoint: &Path) -> Result<Self, ClientError> {
        if !endpoint.is_absolute()
            || endpoint.parent() != Some(state_root.join("connect-sessions").as_path())
        {
            return Err(ClientError::UnsafeCompletionEndpoint);
        }
        let parent = fs::symlink_metadata(endpoint.parent().expect("checked parent"))
            .map_err(|_| ClientError::UnsafeCompletionEndpoint)?;
        let metadata =
            fs::symlink_metadata(endpoint).map_err(|_| ClientError::UnsafeCompletionEndpoint)?;
        let owner = rustix::process::geteuid().as_raw();
        if !parent.file_type().is_dir()
            || parent.file_type().is_symlink()
            || parent.uid() != owner
            || parent.permissions().mode() & 0o077 != 0
            || !metadata.file_type().is_socket()
            || metadata.file_type().is_symlink()
            || metadata.uid() != owner
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(ClientError::UnsafeCompletionEndpoint);
        }
        Ok(Self {
            path: endpoint.to_owned(),
        })
    }

    /// Submits one credential and requires the bounded one-use acknowledgement.
    pub async fn submit(&self, credential: &[u8]) -> Result<(), ClientError> {
        let mut stream = UnixStream::connect(&self.path).await?;
        stream.write_all(credential).await?;
        stream.write_all(b"\n").await?;
        stream.shutdown().await?;
        let mut response = String::new();
        BufReader::new(stream)
            .take((COMPLETION_RESPONSE_BYTES + 1) as u64)
            .read_line(&mut response)
            .await?;
        if response.is_empty() || response.len() > COMPLETION_RESPONSE_BYTES {
            return Err(ClientError::InvalidResponse);
        }
        let response: CompletionAcknowledgement = serde_json::from_str(&response)?;
        if !response.accepted {
            return Err(ClientError::CompletionRefused);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CompletionAcknowledgement {
    accepted: bool,
}

fn validated_hosted_base(base: &str) -> Result<Url, ClientError> {
    let base = Url::parse(base).map_err(|_| ClientError::InvalidHostedBase)?;
    let internal_http = base.scheme() == "http"
        && base.host_str().is_some_and(|host| {
            host == "127.0.0.1" || host == "localhost" || host.ends_with(".svc.cluster.local")
        });
    if !(base.scheme() == "https" || internal_http)
        || base.host_str().is_none()
        || !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
        || !base.path().starts_with('/')
        || base.path().contains("//")
        || (base.path() != "/" && base.path().ends_with('/'))
    {
        return Err(ClientError::InvalidHostedBase);
    }
    Ok(base)
}

fn endpoint(base: &Url, leaf: &str) -> Url {
    let mut endpoint = base.clone();
    let prefix = base.path().trim_end_matches('/');
    endpoint.set_path(&format!("{prefix}/{leaf}"));
    endpoint
}

fn require_bearer(bearer: &str) -> Result<(), ClientError> {
    if bearer.is_empty()
        || bearer.len() > IDENTITY_BEARER_BYTES
        || !bearer.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(ClientError::InvalidIdentityBearer);
    }
    Ok(())
}

fn require_lease_id(lease_id: &str) -> Result<(), ClientError> {
    if lease_id.is_empty()
        || lease_id.len() > 256
        || !lease_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ClientError::InvalidResponse);
    }
    Ok(())
}

fn request_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("client-{}-{timestamp}", std::process::id())
}

#[cfg(test)]
mod tests;

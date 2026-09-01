#![forbid(unsafe_code)]

//! Typed, bounded clients and provider-neutral workflows for Connector control protocols.
//!
//! The raw clients own wire framing and response validation. Reusable helpers compose only generic
//! protocol transitions such as candidate activation and Connect Session completion. This crate
//! does not choose an owner context, Grant, operation, provider policy, or Identity credential,
//! and it never persists a credential.

use std::fs;
use std::io;
use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use protocol::{connection, event, operation};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::UnixStream;
use url::Url;
use zeroize::Zeroizing;

const COMPLETION_RESPONSE_BYTES: usize = 1024;
const IDENTITY_BEARER_BYTES: usize = 512;
const SUBSCRIPTION_RESPONSE_BYTES: usize = 20 * 1024;

/// A transport, framing, or protocol validation failure.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The caller supplied a request that violates its protocol contract.
    #[error("Connector request was invalid: {0}")]
    InvalidRequest(String),
    /// The peer returned an invalid, uncorrelated, empty, or oversized response.
    #[error("Connector returned an invalid response")]
    InvalidResponse,
    /// The configured hosted API base is not one explicit HTTPS base URL.
    #[error("hosted Connector base must be one explicit HTTPS URL")]
    InvalidHostedBase,
    /// The supplied Identity bearer cannot be represented by the bounded hosted binding.
    #[error("hosted Connector Identity bearer is invalid")]
    InvalidIdentityBearer,
    /// The hosted Connector refused the presented Identity authority.
    #[error("hosted Connector Identity authority was refused")]
    HostedNotGranted,
    /// The hosted Connector was unavailable or returned a non-contract HTTP result.
    #[error("hosted Connector is unavailable")]
    HostedUnavailable,
    /// A response carrying a credential or capability omitted the mandatory cache refusal.
    #[error("hosted Connector returned a cacheable credential response")]
    CacheableCredentialResponse,
    /// The Connect Session endpoint is not the expected owner-only Unix socket.
    #[error(
        "the Connect Session completion endpoint is not an owner-only socket under this state root"
    )]
    UnsafeCompletionEndpoint,
    /// The Connector rejected the submitted Connect Session credential.
    #[error("the Connector refused the submitted credential")]
    CompletionRefused,
    /// The Connector returned a typed Connection-domain refusal.
    #[error("the Connector refused the connection request: {0}")]
    ConnectionRefused(String),
    /// A local transport operation failed.
    #[error("local Connector transport failed: {0}")]
    Io(#[from] io::Error),
    /// A frame could not be encoded or decoded.
    #[error("Connector protocol JSON was malformed: {0}")]
    Json(#[from] serde_json::Error),
}

/// Client for the owner-permissioned personal-local Unix-socket binding.
#[derive(Debug, Clone)]
pub struct LocalClient {
    socket: PathBuf,
}

/// Value-free result of beginning a Connector-owned credential acquisition session.
pub struct PendingConnection {
    pub session_ref: String,
    pub completion_endpoint: PathBuf,
}

/// Result of a generic candidate-selection and activation workflow.
pub enum CandidateActivationOutcome {
    SelectionRequired(Vec<connection::ConnectionCandidateSummary>),
    Connected {
        connection: connection::ConnectionDescription,
        observations: Vec<connection::DiscoveryObservationSummary>,
    },
}

/// Value-free result of materializing the recognized observations admitted by the Connector.
pub struct MaterializationOutcome {
    pub connections: Vec<connection::ConnectionSummary>,
    pub unsupported: usize,
    pub not_granted: usize,
}

/// Presence-only state for a user-bound subscription credential.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionStatus {
    pub provider: String,
    pub connected: bool,
}

/// A short-lived, finite-use capability bound to one Harness attempt. Diagnostics never reveal
/// the bearer value.
pub struct SubscriptionLease {
    pub lease_id: String,
    token: Zeroizing<String>,
    pub expires_at: u64,
}

impl SubscriptionLease {
    #[must_use]
    pub fn expose_at_redemption_boundary(&self) -> &str {
        self.token.as_str()
    }
}

impl std::fmt::Debug for SubscriptionLease {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SubscriptionLease")
            .field("lease_id", &self.lease_id)
            .field("token", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// One provider credential returned at the exact Harness bearer boundary. Its allocation is wiped
/// on drop and its diagnostics are redacted.
pub struct RedeemedSubscription {
    credential: Zeroizing<String>,
    pub kind: String,
}

impl RedeemedSubscription {
    #[must_use]
    pub fn expose_at_provider_boundary(&self) -> &str {
        self.credential.as_str()
    }
}

impl std::fmt::Debug for RedeemedSubscription {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RedeemedSubscription")
            .field("credential", &"[REDACTED]")
            .field("kind", &self.kind)
            .finish()
    }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ConnectSubscriptionRequest<'a> {
    credential: &'a str,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct CreateSubscriptionLeaseRequest<'a> {
    attempt_id: &'a str,
    ttl_seconds: u64,
    maximum_uses: u16,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SubscriptionLeaseResponse {
    lease_id: String,
    lease_token: String,
    expires_at: u64,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct RedeemSubscriptionLeaseRequest<'a> {
    attempt_id: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RedeemedSubscriptionResponse {
    credential: String,
    kind: String,
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
    pub async fn operation(
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

/// Client for the Identity-authenticated hosted HTTPS binding.
#[derive(Clone)]
pub struct HostedClient {
    http: reqwest::Client,
    base: Url,
    operations: Url,
    connections: Url,
    events: Url,
    subscription_credential: Url,
    subscription_leases: Url,
}

impl HostedClient {
    /// Creates a bounded client for an exact API base such as
    /// `https://connectors.example/api/connectors/v1`.
    pub fn new(base: &str) -> Result<Self, ClientError> {
        let base = validated_hosted_base(base)?;
        let http = reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(35))
            .build()
            .map_err(|_| ClientError::HostedUnavailable)?;
        Ok(Self::from_parts(base, http))
    }

    /// Sends one hosted operation request with an ephemeral Identity bearer.
    pub async fn operation(
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
            operations: endpoint(&base, "operations"),
            connections: endpoint(&base, "connections"),
            events: endpoint(&base, "events"),
            subscription_credential: endpoint(&base, "subscription-credentials/claude-code"),
            subscription_leases: endpoint(&base, "subscription-credentials/claude-code/leases"),
            http,
        }
    }

    async fn subscription_exchange<R: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        endpoint: Url,
        bearer: &str,
        body: Option<Vec<u8>>,
    ) -> Result<R, ClientError> {
        require_bearer(bearer)?;
        let mut request = self.http.request(method, endpoint).bearer_auth(bearer);
        if let Some(body) = body {
            if body.len() > SUBSCRIPTION_RESPONSE_BYTES {
                return Err(ClientError::InvalidRequest(
                    "subscription request exceeds the protocol bound".to_owned(),
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
            .is_some_and(|length| length > SUBSCRIPTION_RESPONSE_BYTES as u64)
        {
            return Err(ClientError::InvalidResponse);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| ClientError::HostedUnavailable)?
        {
            if bytes.len() + chunk.len() > SUBSCRIPTION_RESPONSE_BYTES {
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
            reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
                return Err(ClientError::HostedNotGranted);
            }
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

fn validate_operation_response(
    response: operation::ResponseEnvelope,
    request_id: &str,
) -> Result<operation::ResponseEnvelope, ClientError> {
    if response.request_id != request_id || response.validate().is_err() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

fn validate_connection_response(
    response: connection::ResponseEnvelope,
    request_id: &str,
) -> Result<connection::ResponseEnvelope, ClientError> {
    if response.request_id != request_id || response.validate().is_err() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

fn validate_event_response(
    response: event::ResponseEnvelope,
    request_id: &str,
) -> Result<event::ResponseEnvelope, ClientError> {
    if response.request_id != request_id || response.validate().is_err() {
        return Err(ClientError::InvalidResponse);
    }
    Ok(response)
}

fn validated_hosted_base(base: &str) -> Result<Url, ClientError> {
    let base = Url::parse(base).map_err(|_| ClientError::InvalidHostedBase)?;
    if base.scheme() != "https"
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
mod tests {
    use axum::body::Bytes;
    use axum::extract::{Path as AxumPath, State};
    use axum::http::HeaderMap;
    use axum::response::IntoResponse as _;
    use axum::routing::{get, post};
    use axum::{Json, Router};
    use protocol::operation::{
        OperationRequest, OperationResult, OwnerContext, ResponseEnvelope, ResponseStatus,
        SearchRequest,
    };
    use tempfile::tempdir;
    use tokio::io::BufReader;
    use tokio::net::{TcpListener, UnixListener};

    use super::*;

    fn context() -> OwnerContext {
        OwnerContext {
            tenant_id: "tenant-1".to_owned(),
            agent_id: "agent-1".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "snapshot-1".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    #[tokio::test]
    async fn local_client_frames_and_correlates_an_operation() {
        let root = tempdir().unwrap();
        let socket = root.path().join("connectors.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let serving = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut line = String::new();
            BufReader::new(&mut stream)
                .read_line(&mut line)
                .await
                .unwrap();
            let request: operation::RequestEnvelope = serde_json::from_str(&line).unwrap();
            request.validate().unwrap();
            assert_eq!(request.protocol, operation::CONTRACT);
            let response = ResponseEnvelope::success(
                request.request_id,
                OperationResult::Search {
                    operations: Vec::new(),
                },
            );
            let mut bytes = serde_json::to_vec(&response).unwrap();
            bytes.push(b'\n');
            stream.write_all(&bytes).await.unwrap();
        });
        let response = LocalClient::new(&socket)
            .operation(
                &context(),
                OperationRequest::Search(SearchRequest {
                    query: "status".to_owned(),
                    limit: 1,
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status, ResponseStatus::Ok);
        serving.await.unwrap();
    }

    #[tokio::test]
    async fn completion_endpoint_is_validated_before_secret_submission() {
        let root = tempdir().unwrap();
        let sessions = root.path().join("connect-sessions");
        fs::create_dir(&sessions).unwrap();
        fs::set_permissions(&sessions, fs::Permissions::from_mode(0o700)).unwrap();
        let socket = sessions.join("complete.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
        let endpoint = CompletionEndpoint::validate(root.path(), &socket).unwrap();
        let serving = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut credential = String::new();
            BufReader::new(&mut stream)
                .read_line(&mut credential)
                .await
                .unwrap();
            assert_eq!(credential, "secret-value\n");
            stream.write_all(b"{\"accepted\":true}\n").await.unwrap();
        });
        endpoint.submit(b"secret-value").await.unwrap();
        serving.await.unwrap();

        let outside = root.path().join("outside.sock");
        let _outside_listener = UnixListener::bind(&outside).unwrap();
        assert!(matches!(
            CompletionEndpoint::validate(root.path(), &outside),
            Err(ClientError::UnsafeCompletionEndpoint)
        ));
    }

    #[test]
    fn hosted_client_requires_one_explicit_https_base() {
        assert!(HostedClient::new("https://connectors.example/api/connectors/v1").is_ok());
        assert!(matches!(
            HostedClient::new("http://connectors.example/api/connectors/v1"),
            Err(ClientError::InvalidHostedBase)
        ));
        assert!(matches!(
            HostedClient::new("https://user@connectors.example/api/connectors/v1"),
            Err(ClientError::InvalidHostedBase)
        ));
    }

    #[tokio::test]
    async fn hosted_client_posts_the_same_typed_operation_frame() {
        async fn operation_handler(
            State(expected): State<OwnerContext>,
            headers: HeaderMap,
            body: Bytes,
        ) -> Bytes {
            assert_eq!(
                headers.get(reqwest::header::AUTHORIZATION).unwrap(),
                "Bearer session-1"
            );
            let request: operation::RequestEnvelope = serde_json::from_slice(&body).unwrap();
            request.validate().unwrap();
            assert_eq!(request.context, expected);
            Bytes::from(
                serde_json::to_vec(&ResponseEnvelope::success(
                    request.request_id,
                    OperationResult::Search {
                        operations: Vec::new(),
                    },
                ))
                .unwrap(),
            )
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new()
            .route("/api/connectors/v1/operations", post(operation_handler))
            .with_state(context());
        let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let base = Url::parse(&format!("http://{address}/api/connectors/v1")).unwrap();
        let client = HostedClient::from_parts(base, reqwest::Client::new());
        let response = client
            .operation(
                "session-1",
                &context(),
                OperationRequest::Search(SearchRequest {
                    query: String::new(),
                    limit: 1,
                }),
            )
            .await
            .unwrap();
        assert_eq!(response.status, ResponseStatus::Ok);
        serving.abort();
    }

    #[tokio::test]
    async fn hosted_subscription_client_redacts_and_redeems_one_attempt_capability() {
        async fn lease(headers: HeaderMap, body: Bytes) -> axum::response::Response {
            assert_eq!(
                headers[reqwest::header::AUTHORIZATION],
                "Bearer identity-access"
            );
            let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(request["attempt_id"], "attempt-one");
            (
                [
                    (reqwest::header::CACHE_CONTROL, "no-store"),
                    (reqwest::header::PRAGMA, "no-cache"),
                ],
                Json(serde_json::json!({
                    "lease_id": "lease-one",
                    "lease_token": "lease-capability-value",
                    "expires_at": 4_000_000_000_u64
                })),
            )
                .into_response()
        }

        async fn redeem(
            AxumPath(lease_id): AxumPath<String>,
            headers: HeaderMap,
            body: Bytes,
        ) -> axum::response::Response {
            assert_eq!(lease_id, "lease-one");
            assert_eq!(
                headers[reqwest::header::AUTHORIZATION],
                "Bearer lease-capability-value"
            );
            let request: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(request["attempt_id"], "attempt-one");
            (
                [
                    (reqwest::header::CACHE_CONTROL, "no-store"),
                    (reqwest::header::PRAGMA, "no-cache"),
                ],
                Json(serde_json::json!({
                    "credential": "synthetic-provider-credential",
                    "kind": "oauth"
                })),
            )
                .into_response()
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new()
            .route(
                "/api/connectors/v1/subscription-credentials/claude-code/leases",
                post(lease),
            )
            .route(
                "/api/connectors/v1/subscription-leases/{lease_id}/redeem",
                post(redeem),
            );
        let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let base = Url::parse(&format!("http://{address}/api/connectors/v1")).unwrap();
        let client = HostedClient::from_parts(base, reqwest::Client::new());
        let lease = client
            .lease_claude_code_subscription(
                "identity-access",
                "attempt-one",
                Duration::from_secs(60),
                1,
            )
            .await
            .unwrap();
        assert!(!format!("{lease:?}").contains("capability-value"));
        let redeemed = client
            .redeem_claude_code_subscription(&lease, "attempt-one")
            .await
            .unwrap();
        assert_eq!(
            redeemed.expose_at_provider_boundary(),
            "synthetic-provider-credential"
        );
        assert!(!format!("{redeemed:?}").contains("synthetic-provider"));
        serving.abort();
    }

    #[tokio::test]
    async fn hosted_subscription_client_refuses_a_cacheable_credential_boundary() {
        async fn status() -> Json<serde_json::Value> {
            Json(serde_json::json!({"provider":"claude-code","connected":false}))
        }
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new().route(
            "/api/connectors/v1/subscription-credentials/claude-code",
            get(status),
        );
        let serving = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let base = Url::parse(&format!("http://{address}/api/connectors/v1")).unwrap();
        let client = HostedClient::from_parts(base, reqwest::Client::new());
        assert!(matches!(
            client
                .claude_code_subscription_status("identity-access")
                .await,
            Err(ClientError::CacheableCredentialResponse)
        ));
        serving.abort();
    }
}

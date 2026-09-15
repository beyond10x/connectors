//! Explicit versioned transport and trusted, bounded authentication remediation.

use super::*;
use protocol::{connection_v2, operation::versions::Version};

impl LocalClient {
    /// Select one exact operation identity. This never negotiates or resends a request.
    pub async fn operation_versioned(
        &self,
        version: Version,
        context: &operation::OwnerContext,
        request: operation::OperationRequest,
    ) -> Result<operation::v3::ResponseEnvelope, ClientError> {
        let request_id = request_id();
        let bytes = operation_request(version, context, &request_id, request.clone())?;
        let bytes = self
            .versioned_exchange(bytes, operation::MAX_RESULT_BYTES)
            .await?;
        response::validate_versioned_operation_response(&bytes, version, &request_id, &request)
    }

    /// Use the additive operation contract explicitly, without changing older callers' selection.
    pub async fn operation_v3(
        &self,
        context: &operation::OwnerContext,
        request: operation::OperationRequest,
    ) -> Result<operation::v3::ResponseEnvelope, ClientError> {
        self.operation_versioned(Version::V0Alpha3, context, request)
            .await
    }

    /// Send one trusted Connection v2 command. Private responses must not be publicly rendered.
    pub async fn connection_v2(
        &self,
        context: &operation::OwnerContext,
        request: connection_v2::ConnectionRequest,
    ) -> Result<connection_v2::ResponseEnvelope, ClientError> {
        let request_id = request_id();
        let bytes = connection_request(context, &request_id, request)?;
        let bytes = self
            .versioned_exchange(bytes, connection_v2::MAX_RESPONSE_BYTES)
            .await?;
        response::validate_connection_v2_response(&bytes, &request_id)
    }

    async fn versioned_exchange(
        &self,
        mut bytes: Vec<u8>,
        response_bound: usize,
    ) -> Result<Vec<u8>, ClientError> {
        bytes.push(b'\n');
        tokio::time::timeout(Duration::from_secs(35), async {
            let mut stream = UnixStream::connect(&self.socket).await?;
            stream.write_all(&bytes).await?;
            stream.shutdown().await?;
            let mut response = Vec::new();
            BufReader::new(stream)
                .take((response_bound + 1) as u64)
                .read_until(b'\n', &mut response)
                .await?;
            if response.is_empty() || response.len() > response_bound {
                return Err(ClientError::InvalidResponse);
            }
            Ok(response)
        })
        .await
        .map_err(|_| ClientError::InvalidResponse)?
    }
}

impl HostedClient {
    /// Select one exact operation identity and accept only its correlated original-byte response.
    /// A v3 HTTP 409 is a typed pre-dispatch refusal, never an Identity renewal signal.
    pub async fn operation_versioned(
        &self,
        version: Version,
        bearer: &str,
        context: &operation::OwnerContext,
        request: operation::OperationRequest,
    ) -> Result<operation::v3::ResponseEnvelope, ClientError> {
        require_bearer(bearer)?;
        let request_id = request_id();
        let bytes = operation_request(version, context, &request_id, request.clone())?;
        let (status, bytes) = self
            .versioned_exchange(
                &self.operations,
                bearer,
                bytes,
                operation::MAX_RESULT_BYTES,
                true,
            )
            .await?;
        let reply = response::validate_versioned_operation_response(
            &bytes,
            version,
            &request_id,
            &request,
        )?;
        let authentication = reply.error.as_ref().is_some_and(|error| {
            error.code == operation::v3::OperationErrorCode::AuthenticationRequired
        });
        if (status == reqwest::StatusCode::CONFLICT) != authentication {
            return Err(ClientError::InvalidResponse);
        }
        Ok(reply)
    }

    /// Use operation v3 explicitly. No automatic fallback or operation retry is performed.
    pub async fn operation_v3(
        &self,
        bearer: &str,
        context: &operation::OwnerContext,
        request: operation::OperationRequest,
    ) -> Result<operation::v3::ResponseEnvelope, ClientError> {
        self.operation_versioned(Version::V0Alpha3, bearer, context, request)
            .await
    }

    /// Send one trusted Connection v2 command. This transport does not confer management authority.
    pub async fn connection_v2(
        &self,
        bearer: &str,
        context: &operation::OwnerContext,
        request: connection_v2::ConnectionRequest,
    ) -> Result<connection_v2::ResponseEnvelope, ClientError> {
        require_bearer(bearer)?;
        let request_id = request_id();
        let bytes = connection_request(context, &request_id, request)?;
        let (_, bytes) = self
            .versioned_exchange(
                &self.connections,
                bearer,
                bytes,
                connection_v2::MAX_RESPONSE_BYTES,
                false,
            )
            .await?;
        response::validate_connection_v2_response(&bytes, &request_id)
    }

    async fn versioned_exchange(
        &self,
        endpoint: &Url,
        bearer: &str,
        bytes: Vec<u8>,
        response_bound: usize,
        operation_conflict: bool,
    ) -> Result<(reqwest::StatusCode, Vec<u8>), ClientError> {
        let mut response = self
            .http
            .post(endpoint.clone())
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .bearer_auth(bearer)
            .body(bytes)
            .send()
            .await
            .map_err(|_| ClientError::HostedUnavailable)?;
        let status = response.status();
        match status {
            reqwest::StatusCode::UNAUTHORIZED => return Err(ClientError::HostedAuthentication),
            reqwest::StatusCode::FORBIDDEN => return Err(ClientError::HostedNotGranted),
            reqwest::StatusCode::CONFLICT if operation_conflict => {}
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
            if bytes.len().saturating_add(chunk.len()) > response_bound {
                return Err(ClientError::InvalidResponse);
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.is_empty() {
            return Err(ClientError::InvalidResponse);
        }
        Ok((status, bytes))
    }
}

fn operation_request(
    version: Version,
    context: &operation::OwnerContext,
    request_id: &str,
    request: operation::OperationRequest,
) -> Result<Vec<u8>, ClientError> {
    version
        .encode_request(operation::RequestEnvelope {
            protocol: operation::CONTRACT.into(),
            request_id: request_id.into(),
            context: context.clone(),
            request,
        })
        .map_err(|_| ClientError::InvalidRequest("invalid operation request".into()))
}

fn connection_request(
    context: &operation::OwnerContext,
    request_id: &str,
    request: connection_v2::ConnectionRequest,
) -> Result<Vec<u8>, ClientError> {
    let envelope = connection_v2::RequestEnvelope {
        protocol: connection_v2::CONTRACT.into(),
        request_id: request_id.into(),
        context: context.clone(),
        request,
    };
    envelope
        .validate()
        .map_err(|_| ClientError::RemediationRefused)?;
    serde_json::to_vec(&envelope).map_err(|_| ClientError::RemediationRefused)
}

impl LocalClient {
    /// Start remediation for one explicit operation/Connection and the caller's bounded input.
    /// Integration/profile expectations come from trusted configuration and only check the reply;
    /// the receiver derives its own binding and evaluates all current authority.
    pub async fn begin_remediation(
        &self,
        context: &operation::OwnerContext,
        request: connection_v2::RemediationStartRequest,
        integration_ref: &str,
        auth_profile: &str,
    ) -> Result<PendingRemediation, ClientError> {
        let operation_ref = request.operation_ref.clone();
        let connection_ref = request.connection_ref.clone();
        let input = request.input.clone();
        let response = self
            .connection_v2(
                context,
                connection_v2::ConnectionRequest::RemediationStart(request),
            )
            .await
            .map_err(|_| ClientError::RemediationRefused)?;
        let Some(connection_v2::ConnectionResult::RemediationStart(status)) = response.response
        else {
            return Err(ClientError::RemediationRefused);
        };
        let now = personal_oauth::oauth_now().map_err(|_| ClientError::RemediationRefused)?;
        if status.operation_ref != operation_ref
            || status.connection_ref != connection_ref
            || status.integration_ref != integration_ref
            || status.auth_profile != auth_profile
            || status.expires_at_unix_ms <= now
            || status.expires_at_unix_ms - now > 600_000
            || status.session.completion_endpoint.is_some()
            || !matches!(
                status.resume_state,
                connection_v2::RemediationResumeState::Pending
                    | connection_v2::RemediationResumeState::Ready
            )
        {
            return Err(ClientError::RemediationRefused);
        }
        let browser_url = status.session.browser_completion_url.map(Zeroizing::new);
        if status.resume_state == connection_v2::RemediationResumeState::Pending {
            personal_oauth::oauth_instruction_endpoint(
                browser_url
                    .as_deref()
                    .map(String::as_str)
                    .ok_or(ClientError::RemediationRefused)?,
            )
            .map_err(|_| ClientError::RemediationRefused)?;
        }
        let deadline =
            tokio::time::Instant::now() + Duration::from_millis(status.expires_at_unix_ms - now);
        Ok(PendingRemediation {
            receiver: self.socket.clone(),
            owner: context.clone(),
            operation_ref,
            connection_ref,
            integration_ref: integration_ref.into(),
            auth_profile: auth_profile.into(),
            input,
            session_ref: status.connect_session_ref,
            need: status.need,
            expires_at_unix_ms: status.expires_at_unix_ms,
            deadline,
            browser_url,
            ready: status.resume_state == connection_v2::RemediationResumeState::Ready,
        })
    }

    /// Fetch the same protected OAuth instruction format used by ordinary personal acquisition.
    /// A ready session needs no instruction endpoint and produces no synthetic human URL.
    pub async fn remediation_instructions(
        &self,
        pending: &PendingRemediation,
        expected_origin: &str,
    ) -> Result<Option<PersonalOAuthInstructions>, ClientError> {
        pending.live()?;
        if pending.receiver != self.socket {
            return Err(ClientError::RemediationRefused);
        }
        if pending.ready {
            return Ok(None);
        }
        let url = pending
            .browser_url
            .as_ref()
            .ok_or(ClientError::RemediationRefused)?;
        personal_oauth::fetch_personal_oauth_instructions(
            url,
            pending.expires_at_unix_ms,
            pending.deadline,
            expected_origin,
        )
        .await
        .map(Some)
        .map_err(|_| ClientError::RemediationRefused)
    }

    /// Poll to Ready, acknowledge once, and validate fresh descriptions against the captured input.
    /// This consumes the local handoff even if an acknowledgement response is lost. It never invokes.
    pub async fn finish_remediation(
        &self,
        context: &operation::OwnerContext,
        pending: PendingRemediation,
    ) -> Result<(), ClientError> {
        if pending.receiver != self.socket || pending.owner != *context {
            return Err(ClientError::RemediationRefused);
        }
        tokio::time::timeout_at(
            pending.deadline,
            self.finish_remediation_before_deadline(context, &pending),
        )
        .await
        .map_err(|_| ClientError::RemediationRefused)?
        .map_err(|_| ClientError::RemediationRefused)
    }

    async fn finish_remediation_before_deadline(
        &self,
        context: &operation::OwnerContext,
        pending: &PendingRemediation,
    ) -> Result<(), ClientError> {
        let mut ready = pending.ready;
        while !ready {
            pending.live()?;
            let response = self
                .connection_v2(
                    context,
                    connection_v2::ConnectionRequest::RemediationStatus(
                        connection_v2::RemediationStatusRequest {
                            connect_session_ref: pending.session_ref.clone(),
                        },
                    ),
                )
                .await?;
            pending.live()?;
            let Some(connection_v2::ConnectionResult::RemediationStatus(status)) =
                response.response
            else {
                return Err(ClientError::RemediationRefused);
            };
            if !pending.matches(&status) {
                return Err(ClientError::RemediationRefused);
            }
            match status.resume_state {
                connection_v2::RemediationResumeState::Ready => ready = true,
                connection_v2::RemediationResumeState::Pending => {
                    tokio::time::sleep(Duration::from_secs(1)).await
                }
                _ => return Err(ClientError::RemediationRefused),
            }
        }
        pending.live()?;
        let response = self
            .connection_v2(
                context,
                connection_v2::ConnectionRequest::RemediationAcknowledge(
                    connection_v2::RemediationAcknowledgeRequest {
                        connect_session_ref: pending.session_ref.clone(),
                        operation_ref: pending.operation_ref.clone(),
                        connection_ref: pending.connection_ref.clone(),
                    },
                ),
            )
            .await?;
        let Some(connection_v2::ConnectionResult::RemediationAcknowledge(ack)) = response.response
        else {
            return Err(ClientError::RemediationRefused);
        };
        if ack.connect_session_ref != pending.session_ref
            || ack.operation_ref != pending.operation_ref
            || ack.connection_ref != pending.connection_ref
        {
            return Err(ClientError::RemediationRefused);
        }
        pending.live()?;
        let response = self
            .connection_v2(
                context,
                connection_v2::ConnectionRequest::Describe(connection_v2::DescribeRequest {
                    connection_ref: pending.connection_ref.clone(),
                }),
            )
            .await?;
        let Some(connection_v2::ConnectionResult::Describe(description)) = response.response else {
            return Err(ClientError::RemediationRefused);
        };
        if description.summary.connection_ref != pending.connection_ref
            || description.summary.integration_ref != pending.integration_ref
            || description.summary.auth_profile.as_deref() != Some(pending.auth_profile.as_str())
            || description.summary.state != connection::ConnectionState::Callable
        {
            return Err(ClientError::RemediationRefused);
        }
        pending.live()?;
        let response = self
            .operation_v3(
                context,
                operation::OperationRequest::Describe(operation::DescribeRequest {
                    operation_ref: pending.operation_ref.clone(),
                }),
            )
            .await?;
        let Some(operation::OperationResult::Describe(description)) = response.response else {
            return Err(ClientError::RemediationRefused);
        };
        if description.operation_ref != pending.operation_ref
            || !description.connections.iter().any(|binding| {
                binding.connection_ref == pending.connection_ref
                    && binding.provider == pending.integration_ref
                    && binding
                        .purpose
                        .as_deref()
                        .is_none_or(|purpose| purpose == pending.auth_profile)
            })
        {
            return Err(ClientError::RemediationRefused);
        }
        let validator = jsonschema::options()
            .with_retriever(NoSchemaRetrieval)
            .build(&description.input_schema)
            .map_err(|_| ClientError::RemediationRefused)?;
        if !validator.is_valid(&pending.input) {
            return Err(ClientError::RemediationRefused);
        }
        pending.live()?;
        Ok(())
    }
}

struct NoSchemaRetrieval;
impl jsonschema::Retrieve for NoSchemaRetrieval {
    fn retrieve(
        &self,
        _: &jsonschema::Uri<String>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        Err("external schema retrieval is unavailable".into())
    }
}

impl PendingRemediation {
    fn live(&self) -> Result<(), ClientError> {
        if tokio::time::Instant::now() >= self.deadline
            || personal_oauth::oauth_now().map_err(|_| ClientError::RemediationRefused)?
                >= self.expires_at_unix_ms
        {
            return Err(ClientError::RemediationRefused);
        }
        Ok(())
    }
    fn matches(&self, status: &connection_v2::BoundRemediationStatus) -> bool {
        status.connect_session_ref == self.session_ref
            && status.operation_ref == self.operation_ref
            && status.connection_ref == self.connection_ref
            && status.integration_ref == self.integration_ref
            && status.auth_profile == self.auth_profile
            && status.need == self.need
            && status.expires_at_unix_ms == self.expires_at_unix_ms
            && status.session.completion_endpoint.is_none()
            && (status.resume_state != connection_v2::RemediationResumeState::Ready
                || status.session.connection_ref.as_deref() == Some(self.connection_ref.as_str()))
    }
}

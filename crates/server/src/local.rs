//! Owner-authenticated Unix-socket binding for the local Connector contracts.

use std::fs::{File, OpenOptions};
use std::future::Future;
use std::io;
use std::os::unix::fs::{
    FileTypeExt as _, MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use fs2::FileExt as _;
use protocol::operation::{RequestEnvelope, ResponseEnvelope, MAX_FRAME_BYTES};
use serde::Deserialize;
use service::{ConnectorBackend, PrincipalContext};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinSet;

const MAX_LOCAL_CLIENTS: usize = 64;
const FRAME_READ_DEADLINE: Duration = Duration::from_secs(5);
const BACKEND_SHUTDOWN_DEADLINE: Duration = Duration::from_secs(15);

/// Bound personal-local daemon. Binding completes before this value is returned, so callers can
/// publish readiness without racing the accept loop.
pub struct LocalOperationDaemon<B: ?Sized> {
    listener: UnixListener,
    socket_path: PathBuf,
    owner_uid: u32,
    backend: Arc<B>,
    _ownership: LocalStateOwnership,
}

/// Exclusive owner-only state capability. Acquire before constructing any backend or journal.
pub struct LocalStateOwnership {
    socket_path: PathBuf,
    owner_uid: u32,
    _state_lock: File,
}

impl LocalStateOwnership {
    pub fn acquire(socket_path: impl Into<PathBuf>) -> Result<Self, LocalDaemonError> {
        let socket_path = socket_path.into();
        let parent = socket_path
            .parent()
            .ok_or(LocalDaemonError::MissingParent)?;
        let owner_uid = rustix::process::geteuid().as_raw();
        prepare_parent(parent, owner_uid)?;
        let state_lock = acquire_state_lock(parent, owner_uid)?;
        validate_owned_socket(&socket_path, owner_uid)?;
        Ok(Self {
            socket_path,
            owner_uid,
            _state_lock: state_lock,
        })
    }

    /// An ephemeral runtime requires absence even after acquiring the state lock. A socket that
    /// appeared after the caller's probe is never removed or treated as permission to retry.
    pub fn require_absent_socket(&self) -> Result<(), LocalDaemonError> {
        match std::fs::symlink_metadata(&self.socket_path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Ok(_) => Err(LocalDaemonError::AlreadyRunning),
            Err(error) => Err(error.into()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LocalDaemonError {
    #[error("local Connector socket path has no parent directory")]
    MissingParent,
    #[error("local Connector state directory is not an owner-only real directory")]
    UnsafeStateDirectory,
    #[error("existing local Connector socket path is not the owner's Unix socket")]
    UnsafeExistingSocket,
    #[error("local Connector state lock is unsafe")]
    UnsafeStateLock,
    #[error("another Connector process owns this state root; use its daemon or wait for its command to finish")]
    AlreadyRunning,
    #[error("local Connector socket I/O failed: {0}")]
    Io(#[from] io::Error),
}

impl<B: ConnectorBackend + ?Sized> LocalOperationDaemon<B> {
    /// Bind an owner-only Unix socket, refusing symlinks, foreign ownership, broad permissions, and
    /// an existing non-socket object.
    pub async fn bind(
        socket_path: impl Into<PathBuf>,
        backend: Arc<B>,
    ) -> Result<Self, LocalDaemonError> {
        Self::bind_owned(LocalStateOwnership::acquire(socket_path)?, backend).await
    }

    /// Bind using ownership acquired by runtime composition before adapter initialization.
    pub async fn bind_owned(
        ownership: LocalStateOwnership,
        backend: Arc<B>,
    ) -> Result<Self, LocalDaemonError> {
        let socket_path = ownership.socket_path.clone();
        let owner_uid = ownership.owner_uid;
        remove_owned_stale_socket(&socket_path, owner_uid)?;
        let listener = UnixListener::bind(&socket_path)?;
        std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))?;
        Ok(Self {
            listener,
            socket_path,
            owner_uid,
            backend,
            _ownership: ownership,
        })
    }

    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Serve until shutdown, then abort and join every incomplete local client before removing the
    /// exact socket path.
    pub async fn serve_until<F>(self, shutdown: F) -> Result<(), LocalDaemonError>
    where
        F: Future<Output = ()>,
    {
        let mut clients = JoinSet::new();
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                () = &mut shutdown => break,
                accepted = self.listener.accept() => {
                    let (stream, _) = accepted?;
                    if clients.len() >= MAX_LOCAL_CLIENTS {
                        drop(stream);
                        continue;
                    }
                    let backend = Arc::clone(&self.backend);
                    let owner_uid = self.owner_uid;
                    clients.spawn(async move {
                        let _ = serve_client(stream, owner_uid, backend).await;
                    });
                }
                Some(_) = clients.join_next(), if !clients.is_empty() => {}
            }
        }
        clients.abort_all();
        while clients.join_next().await.is_some() {}
        let _ = tokio::time::timeout(BACKEND_SHUTDOWN_DEADLINE, self.backend.shutdown()).await;
        drop(self.listener);
        remove_owned_stale_socket(&self.socket_path, self.owner_uid)?;
        Ok(())
    }
}

/// One bounded request using exactly the daemon's framing, principal and dispatch owners.
pub struct LocalOneShot<B: ?Sized> {
    backend: Arc<B>,
    _ownership: LocalStateOwnership,
}

/// Local presentation provenance, never accepted from a wire or backend error message.
#[derive(Debug)]
pub enum OneShotOperationV3Outcome {
    Reply(protocol::operation::v3::ResponseEnvelope),
    RequiresDaemon(protocol::operation::v3::ResponseEnvelope),
}
impl OneShotOperationV3Outcome {
    /// Preserve the existing envelope-only API without changing its wire response.
    pub fn into_response(self) -> protocol::operation::v3::ResponseEnvelope {
        match self {
            Self::Reply(response) | Self::RequiresDaemon(response) => response,
        }
    }
}

impl<B: ConnectorBackend + ?Sized> LocalOneShot<B> {
    pub fn new(ownership: LocalStateOwnership, backend: Arc<B>) -> Result<Self, LocalDaemonError> {
        ownership.require_absent_socket()?;
        Ok(Self {
            backend,
            _ownership: ownership,
        })
    }

    pub async fn operation(
        self,
        request: RequestEnvelope,
    ) -> Result<ResponseEnvelope, LocalDaemonError> {
        let result = async {
            if let Err(error) = validate_one_shot_operation(&request) {
                return Ok(ResponseEnvelope::failure(&request.request_id, error));
            }
            if let protocol::operation::OperationRequest::Invoke(invoke) = &request.request {
                if !self.backend.supports_ephemeral_invocation(invoke) {
                    return Ok(ResponseEnvelope::failure(
                        &request.request_id,
                        daemon_required("operation invoke"),
                    ));
                }
            }
            let frame = serde_json::to_vec(&request).map_err(io::Error::other)?;
            let bytes = dispatch_frame(&frame, protocol::operation::CONTRACT, self.backend.clone())
                .await?
                .ok_or_else(|| io::Error::other("invalid one-shot operation frame"))?;
            serde_json::from_slice(&bytes).map_err(|error| io::Error::other(error).into())
        }
        .await;
        self.backend.shutdown().await;
        result
    }

    /// One explicit v3 operation, retaining the ordinary backend port and shutdown ownership.
    pub async fn operation_v3(
        self,
        request: protocol::operation::v3::RequestEnvelope,
    ) -> Result<protocol::operation::v3::ResponseEnvelope, LocalDaemonError> {
        self.operation_v3_outcome(request)
            .await
            .map(OneShotOperationV3Outcome::into_response)
    }

    /// Retain only receiver-owned daemon requirements for trusted local presentation.
    pub async fn operation_v3_outcome(
        self,
        request: protocol::operation::v3::RequestEnvelope,
    ) -> Result<OneShotOperationV3Outcome, LocalDaemonError> {
        use protocol::operation::v3;

        let result = async {
            if let Some(outcome) = preflight_one_shot_operation_v3(&request) {
                return Ok(outcome);
            }
            if let protocol::operation::OperationRequest::Invoke(invoke) = &request.request {
                if !self.backend.supports_ephemeral_invocation(invoke) {
                    return Ok(OneShotOperationV3Outcome::RequiresDaemon(
                        v3::ResponseEnvelope::failure(
                            &request.request_id,
                            daemon_required("operation invoke").into(),
                        ),
                    ));
                }
            }
            let frame = serde_json::to_vec(&request).map_err(io::Error::other)?;
            let bytes = dispatch_frame(&frame, v3::CONTRACT, self.backend.clone())
                .await?
                .ok_or_else(|| io::Error::other("invalid one-shot operation frame"))?;
            let (version, response) = protocol::operation::versions::decode_response(&bytes)
                .map_err(|_| io::Error::other("invalid one-shot operation response"))?;
            if version != protocol::operation::versions::Version::V0Alpha3
                || response.request_id != request.request_id
            {
                return Err(io::Error::other("uncorrelated one-shot operation response").into());
            }
            Ok(OneShotOperationV3Outcome::Reply(response))
        }
        .await;
        self.backend.shutdown().await;
        result
    }

    pub async fn connection(
        self,
        request: protocol::connection::RequestEnvelope,
    ) -> Result<protocol::connection::ResponseEnvelope, LocalDaemonError> {
        let result = async {
            if let Err(error) = validate_one_shot_connection(&request) {
                return Ok(protocol::connection::ResponseEnvelope::failure(
                    &request.request_id,
                    error,
                ));
            }
            let frame = serde_json::to_vec(&request).map_err(io::Error::other)?;
            let bytes =
                dispatch_frame(&frame, protocol::connection::CONTRACT, self.backend.clone())
                    .await?
                    .ok_or_else(|| io::Error::other("invalid one-shot connection frame"))?;
            serde_json::from_slice(&bytes).map_err(|error| io::Error::other(error).into())
        }
        .await;
        self.backend.shutdown().await;
        result
    }
}

fn daemon_required(method: &str) -> protocol::operation::OperationError {
    protocol::operation::OperationError::new(protocol::operation::OperationErrorCode::Unavailable,
        format!("{method} requires a persistent daemon for this operation; run `connectors serve local` with the same --config and --state-root"), false)
}

/// Check before runtime composition, so known persistent commands cannot initialize adapters.
pub fn validate_one_shot_operation(
    request: &RequestEnvelope,
) -> Result<(), protocol::operation::OperationError> {
    request.validate()?;
    persistent_operation_refusal(&request.request).map_or(Ok(()), Err)
}

fn persistent_operation_refusal(
    request: &protocol::operation::OperationRequest,
) -> Option<protocol::operation::OperationError> {
    use protocol::operation::OperationRequest;
    match request {
        OperationRequest::Search(_)
        | OperationRequest::Describe(_)
        | OperationRequest::Invoke(_) => None,
        OperationRequest::SessionStatus(_)
        | OperationRequest::SessionTerminate(_)
        | OperationRequest::SessionReconcile(_)
        | OperationRequest::SessionSignal(_) => Some(daemon_required("operation session control")),
    }
}

/// Validate before composition and identify only the shared persistent-command decision.
pub fn preflight_one_shot_operation_v3(
    request: &protocol::operation::v3::RequestEnvelope,
) -> Option<OneShotOperationV3Outcome> {
    use protocol::operation::v3::ResponseEnvelope;
    if let Err(error) = request.validate() {
        return Some(OneShotOperationV3Outcome::Reply(ResponseEnvelope::failure(
            &request.request_id,
            error,
        )));
    }
    if let Err(error) = request.clone().into_v2().validate() {
        return Some(OneShotOperationV3Outcome::Reply(ResponseEnvelope::failure(
            &request.request_id,
            error.into(),
        )));
    }
    persistent_operation_refusal(&request.request).map(|error| {
        OneShotOperationV3Outcome::RequiresDaemon(ResponseEnvelope::failure(
            &request.request_id,
            error.into(),
        ))
    })
}

/// Connection metadata is bounded. Activation, materialization and credential acquisition require
/// the daemon until each adapter can prove custody and continuation survive process exit.
pub fn validate_one_shot_connection(
    request: &protocol::connection::RequestEnvelope,
) -> Result<(), protocol::connection::ConnectionError> {
    use protocol::connection::{ConnectionError, ConnectionErrorCode, ConnectionRequest};
    request.validate()?;
    match &request.request {
        ConnectionRequest::Search(_) | ConnectionRequest::Describe(_) | ConnectionRequest::CandidateSearch(_)
        | ConnectionRequest::ObservationSearch(_) => Ok(()),
        ConnectionRequest::CandidateActivate(_) | ConnectionRequest::Materialize(_)
        | ConnectionRequest::ConnectSessionCreate(_) | ConnectionRequest::ConnectSessionStatus(_) =>
            Err(ConnectionError::new(ConnectionErrorCode::Unavailable,
                "connection activation, materialization and connect sessions require a persistent daemon; run `connectors serve local` with the same --config and --state-root", false)),
    }
}

fn local_now() -> Result<u64, service::RemediationError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .ok_or(service::RemediationError::Unavailable)
}

fn private_binding(
    context: &PrincipalContext,
    metadata: &service::RemediationMetadata<'_>,
    input: &serde_json::Value,
    need: protocol::operation::v3::AuthenticationNeed,
    admission: &service::RemediationAdmission,
) -> service::RemediationBinding {
    use sha2::{Digest as _, Sha256};
    service::RemediationBinding {
        operation_ref: metadata.operation.id().into(),
        connection_ref: metadata.connection.id().into(),
        integration_ref: metadata.integration_ref.clone(),
        auth_profile: metadata.auth_profile.clone(),
        need,
        canonical_input_sha256: crate::hosted::canonical_input_digest(input),
        stable_authority_sha256: hex::encode(Sha256::digest(context.stable_authority_seed())),
        grant_ref: admission.grant_ref.clone(),
        grant_revision: admission.grant_revision,
        admission_policy_sha256: admission.admission_policy_sha256.clone(),
        expires_at_unix_ms: admission.expires_at_unix_ms,
    }
}

async fn local_auth_preflight<B: ConnectorBackend + ?Sized>(
    backend: &B,
    context: &PrincipalContext,
    invoke: &protocol::operation::InvokeRequest,
) -> Result<
    Option<protocol::operation::v3::AuthenticationRequired>,
    protocol::operation::v3::OperationError,
> {
    use crate::hosted::remediation::{
        authentication, operation_error, validate_input, validated_metadata,
    };
    use service::{CredentialReadiness, RemediationError, RemediationTarget};
    let target = RemediationTarget {
        operation_ref: &invoke.operation_ref,
        connection_ref: &invoke.connection_ref,
    };
    if !backend.owns_remediation(service::RemediationRoute::Target(target)) {
        return Ok(None);
    }
    let metadata = match backend.remediation_metadata(context, target) {
        Ok(value) => value,
        Err(RemediationError::Unsupported) => return Ok(None),
        Err(error) => return Err(operation_error(error)),
    };
    let record = validated_metadata(&metadata, target).map_err(operation_error)?;
    validate_input(&record, &invoke.input).map_err(operation_error)?;
    let admission = backend
        .personal_remediation_admission(context, target)
        .map_err(operation_error)?;
    let description = match backend
        .handle(
            context,
            protocol::operation::OperationRequest::Describe(protocol::operation::DescribeRequest {
                operation_ref: invoke.operation_ref.clone(),
            }),
        )
        .await
    {
        Ok(protocol::operation::OperationResult::Describe(value)) => value,
        Err(error) if error.code == protocol::operation::OperationErrorCode::StaleAuthority => {
            return Err(error.into())
        }
        _ => return Err(operation_error(RemediationError::Refused)),
    };
    if description.operation_ref != invoke.operation_ref
        || description.description_ref != invoke.description_ref
        || description.input_schema != record["contract"]["input_schema"]
        || !description.connections.iter().any(|connection| {
            connection.connection_ref == invoke.connection_ref
                && connection.provider == metadata.operation.provider()
                && connection.purpose.as_deref() == Some(metadata.auth_profile.as_str())
        })
    {
        return Err(operation_error(RemediationError::Conflict));
    }
    let readiness = backend.credential_readiness(context, target).await;
    if readiness == CredentialReadiness::DependencyUnavailable {
        return Err(operation_error(RemediationError::Unavailable));
    }
    let Some(auth) = authentication(&metadata, readiness) else {
        return Ok(None);
    };
    let binding = private_binding(context, &metadata, &invoke.input, auth.need, &admission);
    admission
        .authority
        .recheck(context, &binding, local_now().map_err(operation_error)?)
        .map_err(operation_error)?;
    Ok(Some(auth))
}

fn connection_remediation_error(
    error: service::RemediationError,
) -> protocol::connection::ConnectionError {
    use protocol::connection::{ConnectionError, ConnectionErrorCode};
    let code = match error {
        service::RemediationError::Refused => ConnectionErrorCode::NotGranted,
        service::RemediationError::InvalidInput => ConnectionErrorCode::InvalidInput,
        service::RemediationError::Conflict => ConnectionErrorCode::Conflict,
        _ => ConnectionErrorCode::Unavailable,
    };
    ConnectionError::new(code, error.to_string(), false)
}

// Status and acknowledgement resolve the existing session's retained current-authority
// capability at its owner. This guard cannot grant a fresh Start or replace that capability.
struct NoNewAdmission;
impl service::RemediationAuthority for NoNewAdmission {
    fn recheck(
        &self,
        _: &PrincipalContext,
        _: &service::RemediationBinding,
        _: u64,
    ) -> Result<(), service::RemediationError> {
        Err(service::RemediationError::Unavailable)
    }
}

async fn local_connection<B: ConnectorBackend + ?Sized>(
    backend: &B,
    context: &PrincipalContext,
    request: protocol::connection_v2::RequestEnvelope,
) -> Result<protocol::connection_v2::ConnectionResult, protocol::connection::ConnectionError> {
    use crate::hosted::remediation::{authentication, validate_input, validated_metadata};
    use protocol::connection_v2::{ConnectionRequest as Request, ConnectionResult as Result};
    use service::{RemediationError, RemediationRequest, RemediationResult, RemediationTarget};
    if let Ok(ordinary) = request.clone().into_v1() {
        return backend
            .handle_connection(context, ordinary.request)
            .await
            .map(Into::into);
    }
    let expected = match &request.request {
        Request::RemediationStart(value) => (
            0_u8,
            None,
            Some((value.operation_ref.clone(), value.connection_ref.clone())),
        ),
        Request::RemediationStatus(value) => (1, Some(value.connect_session_ref.clone()), None),
        Request::RemediationAcknowledge(value) => (
            2,
            Some(value.connect_session_ref.clone()),
            Some((value.operation_ref.clone(), value.connection_ref.clone())),
        ),
        _ => return Err(connection_remediation_error(RemediationError::InvalidInput)),
    };
    let (request, authority, start) = match request.request {
        Request::RemediationStart(start) => {
            let target = RemediationTarget {
                operation_ref: &start.operation_ref,
                connection_ref: &start.connection_ref,
            };
            let metadata = backend
                .remediation_metadata(context, target)
                .map_err(connection_remediation_error)?;
            let record =
                validated_metadata(&metadata, target).map_err(connection_remediation_error)?;
            validate_input(&record, &start.input).map_err(connection_remediation_error)?;
            let admission = backend
                .personal_remediation_admission(context, target)
                .map_err(connection_remediation_error)?;
            let readiness = backend.credential_readiness(context, target).await;
            let auth = authentication(&metadata, readiness).ok_or_else(|| {
                connection_remediation_error(
                    if readiness == service::CredentialReadiness::DependencyUnavailable {
                        RemediationError::Unavailable
                    } else {
                        RemediationError::Conflict
                    },
                )
            })?;
            let binding = private_binding(context, &metadata, &start.input, auth.need, &admission);
            admission
                .authority
                .recheck(
                    context,
                    &binding,
                    local_now().map_err(connection_remediation_error)?,
                )
                .map_err(connection_remediation_error)?;
            (
                RemediationRequest::Start(Box::new(binding)),
                admission.authority,
                true,
            )
        }
        Request::RemediationStatus(value) => (
            RemediationRequest::Status(value),
            Arc::new(NoNewAdmission) as Arc<dyn service::RemediationAuthority>,
            false,
        ),
        Request::RemediationAcknowledge(value) => (
            RemediationRequest::Acknowledge(value),
            Arc::new(NoNewAdmission) as Arc<dyn service::RemediationAuthority>,
            false,
        ),
        _ => return Err(connection_remediation_error(RemediationError::InvalidInput)),
    };
    match backend
        .handle_remediation(context, request, authority)
        .await
        .map_err(connection_remediation_error)?
    {
        RemediationResult::Status(status)
            if (start && expected.0 == 0 || !start && expected.0 == 1)
                && expected
                    .1
                    .as_deref()
                    .is_none_or(|reference| reference == status.connect_session_ref)
                && expected.2.as_ref().is_none_or(|(operation, connection)| {
                    operation == &status.operation_ref && connection == &status.connection_ref
                }) =>
        {
            if start {
                Ok(Result::RemediationStart(*status))
            } else {
                Ok(Result::RemediationStatus(*status))
            }
        }
        RemediationResult::Acknowledged(value)
            if expected.0 == 2
                && expected.1.as_deref() == Some(value.connect_session_ref.as_str())
                && expected.2.as_ref().is_some_and(|(operation, connection)| {
                    operation == &value.operation_ref && connection == &value.connection_ref
                }) =>
        {
            Ok(Result::RemediationAcknowledge(value))
        }
        _ => Err(connection_remediation_error(RemediationError::Unavailable)),
    }
}

async fn serve_client<B: ConnectorBackend + ?Sized>(
    mut stream: UnixStream,
    owner_uid: u32,
    backend: Arc<B>,
) -> Result<(), LocalDaemonError> {
    let credential = stream.peer_cred()?;
    if credential.uid() != owner_uid {
        return Ok(());
    }
    let frame = {
        let mut reader = BufReader::new(&mut stream);
        match tokio::time::timeout(FRAME_READ_DEADLINE, read_frame(&mut reader)).await {
            Ok(frame) => frame?,
            Err(_) => return Ok(()),
        }
    };
    let Some(frame) = frame else {
        return Ok(());
    };
    let probe: ProtocolProbe = match serde_json::from_slice(&frame) {
        Ok(probe) => probe,
        Err(_) => return Ok(()),
    };
    let Some(mut bytes) = dispatch_frame(&frame, &probe.protocol, backend).await? else {
        return Ok(());
    };
    bytes.push(b'\n');
    stream.write_all(&bytes).await?;
    stream.shutdown().await?;
    Ok(())
}

#[derive(Deserialize)]
struct ProtocolProbe {
    protocol: String,
}

async fn dispatch_frame<B: ConnectorBackend + ?Sized>(
    frame: &[u8],
    protocol_name: &str,
    backend: Arc<B>,
) -> Result<Option<Vec<u8>>, LocalDaemonError> {
    let bytes = match protocol_name {
        protocol::endpoint::CONTRACT => {
            if frame.len() > protocol::endpoint::MAX_FRAME_BYTES {
                return Ok(None);
            }
            let request: protocol::endpoint::RequestEnvelope = match serde_json::from_slice(frame) {
                Ok(value) => value,
                Err(_) => return Ok(None),
            };
            if request.validate().is_err() {
                return Ok(None);
            }
            let context = match PrincipalContext::local(&request.context) {
                Ok(value) => value,
                Err(_) => return Ok(None),
            };
            let response = match backend.handle_endpoint(&context, request.request).await {
                Ok(result) => {
                    protocol::endpoint::ResponseEnvelope::success(&request.request_id, result)
                }
                Err(error) => {
                    protocol::endpoint::ResponseEnvelope::failure(&request.request_id, error)
                }
            };
            let response = match response.validate() {
                Ok(()) => response,
                Err(error) => {
                    protocol::endpoint::ResponseEnvelope::failure(request.request_id, error)
                }
            };
            serde_json::to_vec(&response).map_err(io::Error::other)?
        }
        protocol::operation::v4::CONTRACT => {
            let request: protocol::operation::v4::RequestEnvelope =
                match serde_json::from_slice(frame) {
                    Ok(value) => value,
                    Err(_) => return Ok(None),
                };
            if request.validate().is_err() {
                return Ok(None);
            }
            let context = match PrincipalContext::local(&request.context) {
                Ok(value) => value,
                Err(_) => return Ok(None),
            };
            let request_id = request.request_id;
            let response =
                match service::normalize_endpoint_operation(&*backend, &context, request.request)
                    .await
                {
                    Err(error) => {
                        protocol::operation::v4::ResponseEnvelope::failure(&request_id, error)
                    }
                    Ok(normalized) => {
                        let preflight =
                            if let protocol::operation::OperationRequest::Invoke(invoke) =
                                &normalized.request
                            {
                                local_auth_preflight(&*backend, &context, invoke).await
                            } else {
                                Ok(None)
                            };
                        match preflight {
                            Ok(Some(auth)) => protocol::operation::v4::ResponseEnvelope::failure(
                                &request_id,
                                protocol::operation::v3::OperationError::authentication_required(
                                    auth,
                                ),
                            ),
                            Err(error) => protocol::operation::v4::ResponseEnvelope::failure(
                                &request_id,
                                error,
                            ),
                            Ok(None) => match backend
                                .handle(&context, normalized.request)
                                .await
                                .and_then(|result| {
                                    service::constrain_endpoint_description(
                                        result,
                                        normalized.description_connection.as_deref(),
                                    )
                                }) {
                                Ok(result) => protocol::operation::v4::ResponseEnvelope::success(
                                    &request_id,
                                    result,
                                ),
                                Err(error) => protocol::operation::v4::ResponseEnvelope::failure(
                                    &request_id,
                                    error.into(),
                                ),
                            },
                        }
                    }
                };
            let response = match response.validate() {
                Ok(()) => response,
                Err(error) => protocol::operation::v4::ResponseEnvelope::failure(request_id, error),
            };
            serde_json::to_vec(&response).map_err(io::Error::other)?
        }
        protocol::operation::legacy::CONTRACT
        | protocol::operation::wire::CONTRACT
        | protocol::operation::v3::CONTRACT => {
            let (version, request) = match protocol::operation::versions::decode_request(frame) {
                Ok(request) => request,
                Err(_) => return Ok(None),
            };
            let context = match PrincipalContext::local(&request.context) {
                Ok(context) => context,
                Err(_) => return Ok(None),
            };
            let request_id = request.request_id;
            let preflight =
                if let protocol::operation::OperationRequest::Invoke(invoke) = &request.request {
                    local_auth_preflight(&*backend, &context, invoke).await
                } else {
                    Ok(None)
                };
            let response = match preflight {
                Ok(Some(auth)) => protocol::operation::v3::ResponseEnvelope::failure(
                    &request_id,
                    protocol::operation::v3::OperationError::authentication_required(auth),
                ),
                Err(error) => {
                    protocol::operation::v3::ResponseEnvelope::failure(&request_id, error)
                }
                Ok(None) => match backend.handle(&context, request.request).await {
                    Ok(response) => {
                        protocol::operation::v3::ResponseEnvelope::success(&request_id, response)
                    }
                    Err(error) => protocol::operation::v3::ResponseEnvelope::failure(
                        &request_id,
                        error.into(),
                    ),
                },
            };
            let response = match response.validate() {
                Ok(()) => response,
                Err(error) => protocol::operation::v3::ResponseEnvelope::failure(request_id, error),
            };
            version
                .encode_response(response)
                .map_err(io::Error::other)?
        }
        protocol::connection::CONTRACT | protocol::connection_v2::CONTRACT => {
            let (version, request) = match protocol::connection_v2::decode_request(frame) {
                Ok(value) => value,
                Err(_) => return Ok(None),
            };
            let context = match PrincipalContext::local(&request.context) {
                Ok(context) => context,
                Err(_) => return Ok(None),
            };
            let request_id = request.request_id.clone();
            let response = match local_connection(&*backend, &context, request).await {
                Ok(result) => {
                    protocol::connection_v2::ResponseEnvelope::success(&request_id, result)
                }
                Err(error) => {
                    protocol::connection_v2::ResponseEnvelope::failure(&request_id, error)
                }
            };
            let response = match response.validate() {
                Ok(()) => response,
                Err(_) => protocol::connection_v2::ResponseEnvelope::failure(
                    &request_id,
                    connection_remediation_error(service::RemediationError::Unavailable),
                ),
            };
            version
                .encode_response(response)
                .map_err(io::Error::other)?
        }
        protocol::catalog::CONTRACT => {
            let request: protocol::catalog::RequestEnvelope = match serde_json::from_slice(frame) {
                Ok(request) => request,
                Err(_) => return Ok(None),
            };
            if request.validate().is_err() || PrincipalContext::local(&request.context).is_err() {
                return Ok(None);
            }
            let request_id = request.request_id;
            let response = match crate::catalog_projection::handle(request.request, &*backend) {
                Ok(result) => protocol::catalog::ResponseEnvelope::success(&request_id, result),
                Err(error) => protocol::catalog::ResponseEnvelope::failure(&request_id, error),
            };
            let response = match response.validate() {
                Ok(()) => response,
                Err(error) => protocol::catalog::ResponseEnvelope::failure(request_id, error),
            };
            serde_json::to_vec(&response).map_err(io::Error::other)?
        }
        protocol::event::CONTRACT => {
            let request: protocol::event::RequestEnvelope = match serde_json::from_slice(frame) {
                Ok(request) => request,
                Err(_) => return Ok(None),
            };
            if request.validate().is_err() {
                return Ok(None);
            }
            let context = match PrincipalContext::local(&request.context) {
                Ok(context) => context,
                Err(_) => return Ok(None),
            };
            let request_id = request.request_id;
            let response = match backend.handle_event(&context, request.request).await {
                Ok(response) => protocol::event::ResponseEnvelope::success(&request_id, response),
                Err(error) => protocol::event::ResponseEnvelope::failure(&request_id, error),
            };
            let response = match response.validate() {
                Ok(()) => response,
                Err(error) => protocol::event::ResponseEnvelope::failure(request_id, error),
            };
            serde_json::to_vec(&response).map_err(io::Error::other)?
        }
        protocol::datasource::CONTRACT => {
            let request: protocol::datasource::RequestEnvelope = match serde_json::from_slice(frame)
            {
                Ok(request) => request,
                Err(_) => return Ok(None),
            };
            if request.validate().is_err() {
                return Ok(None);
            }
            let context = match PrincipalContext::local(&request.context) {
                Ok(context) => context,
                Err(_) => return Ok(None),
            };
            let request_id = request.request_id;
            let response = match backend.handle_datasource(&context, request.request).await {
                Ok(response) => {
                    protocol::datasource::ResponseEnvelope::success(&request_id, response)
                }
                Err(error) => protocol::datasource::ResponseEnvelope::failure(&request_id, error),
            };
            let response = match response.validate() {
                Ok(()) => response,
                Err(error) => protocol::datasource::ResponseEnvelope::failure(request_id, error),
            };
            serde_json::to_vec(&response).map_err(io::Error::other)?
        }
        _ => return Ok(None),
    };
    Ok(Some(bytes))
}

async fn read_frame<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
) -> io::Result<Option<Vec<u8>>> {
    let mut frame = Vec::with_capacity(4096);
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "unterminated operation frame",
                ))
            };
        }
        if let Some(newline) = available.iter().position(|byte| *byte == b'\n') {
            if frame.len() + newline > MAX_FRAME_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "operation frame exceeds bound",
                ));
            }
            frame.extend_from_slice(&available[..newline]);
            reader.consume(newline + 1);
            return Ok(Some(frame));
        }
        if frame.len() + available.len() > MAX_FRAME_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "operation frame exceeds bound",
            ));
        }
        let consumed = available.len();
        frame.extend_from_slice(available);
        reader.consume(consumed);
    }
}

fn acquire_state_lock(parent: &Path, owner_uid: u32) -> Result<File, LocalDaemonError> {
    let path = parent.join(".connectors.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map_err(|_| LocalDaemonError::UnsafeStateLock)?;
    let metadata = file
        .metadata()
        .map_err(|_| LocalDaemonError::UnsafeStateLock)?;
    if !metadata.file_type().is_file()
        || metadata.uid() != owner_uid
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(LocalDaemonError::UnsafeStateLock);
    }
    file.try_lock_exclusive()
        .map_err(|error| match error.kind() {
            io::ErrorKind::WouldBlock => LocalDaemonError::AlreadyRunning,
            _ => LocalDaemonError::UnsafeStateLock,
        })?;
    Ok(file)
}

fn prepare_parent(parent: &Path, owner_uid: u32) -> Result<(), LocalDaemonError> {
    if !parent.exists() {
        std::fs::create_dir_all(parent)?;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
    }
    let metadata = std::fs::symlink_metadata(parent)?;
    if !metadata.file_type().is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != owner_uid
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(LocalDaemonError::UnsafeStateDirectory);
    }
    Ok(())
}

fn remove_owned_stale_socket(path: &Path, owner_uid: u32) -> Result<(), LocalDaemonError> {
    if validate_owned_socket(path, owner_uid)? {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn validate_owned_socket(path: &Path, owner_uid: u32) -> Result<bool, LocalDaemonError> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    if !metadata.file_type().is_socket()
        || metadata.file_type().is_symlink()
        || metadata.uid() != owner_uid
    {
        return Err(LocalDaemonError::UnsafeExistingSocket);
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    use async_trait::async_trait;
    use protocol::connection::{ConnectionError, ConnectionRequest, ConnectionResult};
    use protocol::event::{EventError, EventRequest, EventResult};
    use protocol::operation::{
        ApprovalPosture, EffectClass, OperationError, OperationRequest, OperationResult,
        OperationSummary, OwnerContext, ResponseStatus, SearchRequest,
    };
    use tokio::sync::oneshot;

    use super::*;

    static SEQUENCE: AtomicU64 = AtomicU64::new(1);

    #[derive(Default)]
    struct SyntheticBackend {
        shutdown: AtomicBool,
        operation_calls: AtomicU64,
        connection_called: AtomicBool,
        event_called: AtomicBool,
    }

    #[async_trait]
    impl ConnectorBackend for SyntheticBackend {
        async fn ready(&self) -> Result<(), service::BackendReadinessError> {
            // This process-local transport test backend has no configured dependency.
            Ok(())
        }

        async fn handle(
            &self,
            _context: &PrincipalContext,
            request: OperationRequest,
        ) -> Result<OperationResult, OperationError> {
            self.operation_calls.fetch_add(1, Ordering::SeqCst);
            assert!(matches!(
                request,
                OperationRequest::Search(SearchRequest { .. })
            ));
            Ok(OperationResult::Search {
                operations: vec![OperationSummary {
                    operation_ref: "sip.dial".to_owned(),
                    title: "Dial SIP".to_owned(),
                    effect: EffectClass::Mutating,
                    approval: ApprovalPosture::Required,
                    connections: Vec::new(),
                }],
            })
        }

        async fn handle_connection(
            &self,
            _context: &PrincipalContext,
            request: ConnectionRequest,
        ) -> Result<ConnectionResult, ConnectionError> {
            assert!(matches!(request, ConnectionRequest::Search(_)));
            self.connection_called.store(true, Ordering::Release);
            Ok(ConnectionResult::Search {
                connections: Vec::new(),
            })
        }

        async fn handle_event(
            &self,
            _context: &PrincipalContext,
            request: EventRequest,
        ) -> Result<EventResult, EventError> {
            assert!(matches!(request, EventRequest::Search(_)));
            self.event_called.store(true, Ordering::Release);
            Ok(EventResult::Search {
                channels: Vec::new(),
            })
        }

        async fn shutdown(&self) {
            self.shutdown.store(true, Ordering::Release);
        }
    }

    fn temporary_socket() -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "b10x-local-operation-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        (root.join("connectors.sock"), root)
    }

    fn context() -> OwnerContext {
        OwnerContext {
            tenant_id: "tenant-local".to_owned(),
            agent_id: "agent-dev".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "authority-1".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    #[tokio::test]
    async fn auth_one_shot_v3_serves_a_real_result_and_joins_shutdown() {
        let (socket, root) = temporary_socket();
        let backend = Arc::new(SyntheticBackend::default());
        let request = protocol::operation::v3::RequestEnvelope {
            protocol: protocol::operation::v3::CONTRACT.into(),
            request_id: "one-shot-v3-search".into(),
            context: context(),
            request: OperationRequest::Search(SearchRequest {
                query: "sip".into(),
                limit: 10,
            }),
        };
        let response = LocalOneShot::new(
            LocalStateOwnership::acquire(&socket).unwrap(),
            backend.clone(),
        )
        .unwrap()
        .operation_v3(request)
        .await
        .unwrap();
        response.validate().unwrap();
        assert_eq!(response.protocol, protocol::operation::v3::CONTRACT);
        assert_eq!(response.request_id, "one-shot-v3-search");
        let Some(OperationResult::Search { operations }) = response.response else {
            panic!("the real one-shot backend result must survive the v3 transport");
        };
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].operation_ref, "sip.dial");
        assert_eq!(backend.operation_calls.load(Ordering::SeqCst), 1);
        assert!(backend.shutdown.load(Ordering::Acquire));
        assert!(!socket.exists());
        std::fs::remove_file(root.join(".connectors.lock")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[tokio::test]
    async fn auth_one_shot_v3_refuses_before_backend_work_and_joins_shutdown() {
        use protocol::operation::v3::OperationErrorCode as Code;

        for (protocol, request, expected) in [
            (
                protocol::operation::v3::CONTRACT,
                OperationRequest::SessionStatus(protocol::operation::SessionRequest {
                    execution_ref: "execution:fixture".into(),
                }),
                Code::Unavailable,
            ),
            (
                "b10x.connector-operation.v0alpha99",
                OperationRequest::Search(SearchRequest {
                    query: "sip".into(),
                    limit: 10,
                }),
                Code::Protocol,
            ),
        ] {
            let (socket, root) = temporary_socket();
            let backend = Arc::new(SyntheticBackend::default());
            let response = LocalOneShot::new(
                LocalStateOwnership::acquire(&socket).unwrap(),
                backend.clone(),
            )
            .unwrap()
            .operation_v3(protocol::operation::v3::RequestEnvelope {
                protocol: protocol.into(),
                request_id: "one-shot-v3-refusal".into(),
                context: context(),
                request,
            })
            .await
            .unwrap();
            response.validate().unwrap();
            assert_eq!(response.protocol, protocol::operation::v3::CONTRACT);
            assert_eq!(response.request_id, "one-shot-v3-refusal");
            assert!(response.response.is_none());
            let error = response.error.unwrap();
            assert_eq!(error.code, expected);
            assert!(!error.retriable);
            assert!(error.authentication.is_none());
            assert_eq!(backend.operation_calls.load(Ordering::SeqCst), 0);
            assert!(backend.shutdown.load(Ordering::Acquire));
            assert!(!socket.exists());
            std::fs::remove_file(root.join(".connectors.lock")).unwrap();
            std::fs::remove_dir(root).unwrap();
        }
    }

    struct AuthenticationBackend {
        shutdown: AtomicBool,
        readiness: std::sync::atomic::AtomicU64,
        dispatch: std::sync::atomic::AtomicU64,
    }
    struct FixtureAdmission;
    impl service::RemediationAuthority for FixtureAdmission {
        fn recheck(
            &self,
            _: &PrincipalContext,
            binding: &service::RemediationBinding,
            _: u64,
        ) -> Result<(), service::RemediationError> {
            if binding.grant_ref == "grant:local-fixture"
                && binding.connection_ref == "connection:local-fixture"
            {
                Ok(())
            } else {
                Err(service::RemediationError::Refused)
            }
        }
    }
    #[async_trait::async_trait]
    impl ConnectorBackend for AuthenticationBackend {
        fn owns_remediation(&self, route: service::RemediationRoute<'_>) -> bool {
            matches!(route, service::RemediationRoute::Target(target)
                if target.operation_ref == "slack-conversations-history"
                    && target.connection_ref == "connection:local-fixture")
        }
        async fn ready(&self) -> Result<(), service::BackendReadinessError> {
            Ok(())
        }
        fn supports_ephemeral_invocation(&self, _: &protocol::operation::InvokeRequest) -> bool {
            true
        }
        fn remediation_metadata<'a>(
            &'a self,
            _: &PrincipalContext,
            target: service::RemediationTarget<'_>,
        ) -> Result<service::RemediationMetadata<'a>, service::RemediationError> {
            if target.operation_ref != "slack-conversations-history"
                || target.connection_ref != "connection:local-fixture"
            {
                return Err(service::RemediationError::Refused);
            }
            Ok(service::RemediationMetadata {
                operation: catalog::reader::operation(target.operation_ref).unwrap(),
                connection: domain::ConnectionAuthority::new(
                    target.connection_ref,
                    domain::InitiationPolicy::platform_only(),
                )
                .unwrap(),
                integration_ref: "slack".into(),
                auth_profile: "slack.bot_token".into(),
                catalog_generation: catalog::reader::embedded().digest().into(),
            })
        }
        fn personal_remediation_admission(
            &self,
            _: &PrincipalContext,
            _: service::RemediationTarget<'_>,
        ) -> Result<service::RemediationAdmission, service::RemediationError> {
            // Explicit synthetic policy; production obtains these facts from its actual OAuth owner.
            Ok(service::RemediationAdmission {
                grant_ref: "grant:local-fixture".into(),
                grant_revision: None,
                admission_policy_sha256: "f".repeat(64),
                expires_at_unix_ms: u64::MAX,
                authority: Arc::new(FixtureAdmission),
            })
        }
        async fn credential_readiness(
            &self,
            _: &PrincipalContext,
            _: service::RemediationTarget<'_>,
        ) -> service::CredentialReadiness {
            self.readiness.fetch_add(1, Ordering::SeqCst);
            service::CredentialReadiness::MissingCredential
        }
        async fn handle(
            &self,
            _: &PrincipalContext,
            request: protocol::operation::OperationRequest,
        ) -> Result<protocol::operation::OperationResult, protocol::operation::OperationError>
        {
            if matches!(request, protocol::operation::OperationRequest::Invoke(_)) {
                self.dispatch.fetch_add(1, Ordering::SeqCst);
                return Err(protocol::operation::OperationError::new(
                    protocol::operation::OperationErrorCode::Unavailable,
                    "fixture dispatch refusal",
                    false,
                ));
            }
            assert!(matches!(
                request,
                protocol::operation::OperationRequest::Describe(_)
            ));
            let operation =
                catalog::operation(catalog::OperationKey::id("slack-conversations-history"))
                    .unwrap();
            Ok(protocol::operation::OperationResult::Describe(
                protocol::operation::OperationDescription {
                    rate_advice: None,
                    operation_ref: operation.id.into(),
                    title: "fixture".into(),
                    description: "fixture".into(),
                    input_schema: serde_json::from_str(operation.input_schema).unwrap(),
                    output_schema: serde_json::json!({}),
                    effect: protocol::operation::EffectClass::ReadOnly,
                    approval: protocol::operation::ApprovalPosture::NotRequired,
                    connections: vec![protocol::operation::ConnectionSummary {
                        connection_ref: "connection:local-fixture".into(),
                        label: "fixture".into(),
                        provider: "slack".into(),
                        audiences: vec![],
                        purpose: Some("slack.bot_token".into()),
                    }],
                    description_ref: "description:local-fixture".into(),
                },
            ))
        }
        async fn shutdown(&self) {
            self.shutdown.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn auth_one_shot_v3_need_precedes_dispatch_and_joins_shutdown() {
        let (socket, root) = temporary_socket();
        let backend = Arc::new(AuthenticationBackend {
            shutdown: false.into(),
            readiness: 0.into(),
            dispatch: 0.into(),
        });
        let ownership = LocalStateOwnership::acquire(&socket).unwrap();
        let request = protocol::operation::v3::RequestEnvelope {
            protocol: protocol::operation::v3::CONTRACT.into(),
            request_id: "auth:one-shot".into(),
            context: context(),
            request: protocol::operation::OperationRequest::Invoke(
                protocol::operation::InvokeRequest {
                    operation_ref: "slack-conversations-history".into(),
                    connection_ref: "connection:local-fixture".into(),
                    description_ref: "description:local-fixture".into(),
                    input: serde_json::json!({"channel":"C123"}),
                    approval_evidence_ref: None,
                },
            ),
        };
        let response = LocalOneShot::new(ownership, backend.clone())
            .unwrap()
            .operation_v3(request)
            .await
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
        assert!(backend.shutdown.load(Ordering::SeqCst));
        assert!(!socket.exists());
        assert_eq!(backend.dispatch.load(Ordering::SeqCst), 0);
        let error = response.error.expect("authentication refusal");
        assert_eq!(
            error.code,
            protocol::operation::v3::OperationErrorCode::AuthenticationRequired
        );
        assert_eq!(
            error.authentication.unwrap().attempt,
            protocol::operation::v3::AuthenticationAttemptState::NotAttempted
        );
        assert_eq!(backend.readiness.load(Ordering::SeqCst), 1);
    }

    struct RateRefusalBackend {
        calls: AtomicU64,
    }

    #[async_trait]
    impl ConnectorBackend for RateRefusalBackend {
        async fn ready(&self) -> Result<(), service::BackendReadinessError> {
            Ok(())
        }
        async fn handle(
            &self,
            _context: &PrincipalContext,
            _request: OperationRequest,
        ) -> Result<OperationResult, OperationError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Err(serde_json::from_value(serde_json::json!({
                "code":"rate_limited", "message":"provider refused this request",
                "retriable":true, "retry_after_seconds":30
            }))
            .expect("the current internal error vocabulary carries definite rate refusal"))
        }
    }

    #[tokio::test]
    async fn rate_stage2_actual_socket_serves_both_versions_without_resending() {
        let (socket, root) = temporary_socket();
        let backend = Arc::new(RateRefusalBackend {
            calls: AtomicU64::new(0),
        });
        let daemon = LocalOperationDaemon::bind(&socket, Arc::clone(&backend))
            .await
            .unwrap();
        let (stop, stopped) = oneshot::channel();
        let serving = tokio::spawn(async move {
            daemon
                .serve_until(async {
                    let _ = stopped.await;
                })
                .await
                .unwrap();
        });
        let mut responses = Vec::new();
        for version in [
            "b10x.connector-operation.v0alpha2",
            "b10x.connector-operation.v0alpha1",
            "b10x.connector-operation.v0alpha99",
        ] {
            let frame = serde_json::json!({"protocol":version,"request_id":"rate-local","context":context(),
                "request":{"method":"invoke","params":{"operation_ref":"fixture.read","connection_ref":"connection:fixture","description_ref":"description:fixture","input":{}}}});
            let mut stream = UnixStream::connect(&socket).await.unwrap();
            let mut bytes = serde_json::to_vec(&frame).unwrap();
            bytes.push(b'\n');
            stream.write_all(&bytes).await.unwrap();
            let mut response = String::new();
            BufReader::new(stream)
                .read_line(&mut response)
                .await
                .unwrap();
            responses.push(response);
        }
        stop.send(()).unwrap();
        serving.await.unwrap();
        std::fs::remove_file(root.join(".connectors.lock")).unwrap();
        std::fs::remove_dir(root).unwrap();
        assert!(
            !responses[0].is_empty(),
            "the v2 socket request needs a correlated response"
        );
        let current: serde_json::Value = serde_json::from_str(&responses[0]).unwrap();
        assert_eq!(current["protocol"], "b10x.connector-operation.v0alpha2");
        assert_eq!(current["request_id"], "rate-local");
        assert_eq!(current["error"]["code"], "rate_limited");
        assert_eq!(current["error"]["retry_after_seconds"], 30);
        let old: serde_json::Value = serde_json::from_str(&responses[1]).unwrap();
        assert_eq!(old["protocol"], "b10x.connector-operation.v0alpha1");
        assert_eq!(old["error"]["code"], "unavailable");
        assert!(old["error"].get("retry_after_seconds").is_none());
        assert!(responses[2].is_empty());
        assert_eq!(backend.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn owner_socket_serves_one_strict_bounded_operation_frame() {
        let (socket, root) = temporary_socket();
        let backend = Arc::new(SyntheticBackend::default());
        let daemon = LocalOperationDaemon::bind(&socket, Arc::clone(&backend))
            .await
            .unwrap();
        assert_eq!(
            std::fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let serving = tokio::spawn(async move {
            daemon
                .serve_until(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        let request = RequestEnvelope {
            protocol: protocol::operation::CONTRACT.to_owned(),
            request_id: "request-1".to_owned(),
            context: context(),
            request: OperationRequest::Search(SearchRequest {
                query: "sip".to_owned(),
                limit: 10,
            }),
        };
        let mut stream = UnixStream::connect(&socket).await.unwrap();
        let mut bytes = serde_json::to_vec(&request).unwrap();
        bytes.push(b'\n');
        stream.write_all(&bytes).await.unwrap();
        let mut response = String::new();
        BufReader::new(stream)
            .read_line(&mut response)
            .await
            .unwrap();
        let response: ResponseEnvelope = serde_json::from_str(&response).unwrap();
        assert_eq!(response.status, ResponseStatus::Ok);
        response.validate().unwrap();

        shutdown_tx.send(()).unwrap();
        serving.await.unwrap();
        assert!(backend.shutdown.load(Ordering::Acquire));
        assert!(!socket.exists());
        std::fs::remove_file(root.join(".connectors.lock")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[tokio::test]
    async fn one_socket_dispatches_the_value_free_connection_and_event_contracts() {
        let backend = Arc::new(SyntheticBackend::default());
        let connection = protocol::connection::RequestEnvelope {
            protocol: protocol::connection::CONTRACT.to_owned(),
            request_id: "connection-request-1".to_owned(),
            context: context(),
            request: ConnectionRequest::Search(protocol::connection::SearchRequest {
                query: String::new(),
                limit: 10,
            }),
        };
        let bytes = serde_json::to_vec(&connection).unwrap();
        let response = dispatch_frame(&bytes, protocol::connection::CONTRACT, Arc::clone(&backend))
            .await
            .unwrap()
            .unwrap();
        let response: protocol::connection::ResponseEnvelope =
            serde_json::from_slice(&response).unwrap();
        response.validate().unwrap();

        let event = protocol::event::RequestEnvelope {
            protocol: protocol::event::CONTRACT.to_owned(),
            request_id: "event-request-1".to_owned(),
            context: context(),
            request: EventRequest::Search(protocol::event::SearchRequest {
                query: String::new(),
                limit: 10,
            }),
        };
        let bytes = serde_json::to_vec(&event).unwrap();
        let response = dispatch_frame(&bytes, protocol::event::CONTRACT, Arc::clone(&backend))
            .await
            .unwrap()
            .unwrap();
        let response: protocol::event::ResponseEnvelope =
            serde_json::from_slice(&response).unwrap();
        response.validate().unwrap();

        assert!(backend.connection_called.load(Ordering::Acquire));
        assert!(backend.event_called.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn a_second_daemon_cannot_unlink_the_live_daemons_socket() {
        let (socket, root) = temporary_socket();
        let first = LocalOperationDaemon::bind(&socket, Arc::new(SyntheticBackend::default()))
            .await
            .unwrap();
        assert!(matches!(
            LocalOperationDaemon::bind(&socket, Arc::new(SyntheticBackend::default())).await,
            Err(LocalDaemonError::AlreadyRunning)
        ));
        assert!(socket.exists());

        drop(first);
        let replacement =
            LocalOperationDaemon::bind(&socket, Arc::new(SyntheticBackend::default()))
                .await
                .unwrap();
        drop(replacement);
        std::fs::remove_file(&socket).unwrap();
        std::fs::remove_file(root.join(".connectors.lock")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[tokio::test]
    async fn a_broad_state_directory_refuses_without_repair() {
        let (socket, root) = temporary_socket();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(
            LocalOperationDaemon::bind(socket, Arc::new(SyntheticBackend::default())).await,
            Err(LocalDaemonError::UnsafeStateDirectory)
        ));
        assert_eq!(
            std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o755
        );
        std::fs::remove_dir(root).unwrap();
    }
}

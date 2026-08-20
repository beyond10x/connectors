//! Catalog-backed generic operation projection for the admitted SIP voice runtime.

use std::collections::{BTreeMap, BTreeSet};
use std::net::ToSocketAddrs as _;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use connector_resolve::document::{Document, HostEffect, ProtocolDriver};
use domain::{
    voice::TerminationReason, AdmittedOperation, Capability, ConnectionAuthority, DriverId,
};
use hosted_state::PostgresState;
use protocol::operation::{
    ApprovalPosture, ConnectionSummary, DescribeRequest, EffectClass, InvocationResult,
    InvokeRequest, OperationDescription, OperationError, OperationErrorCode, OperationRequest,
    OperationResult, OperationSummary, RequestedSessionTermination, SessionRequest, SessionState,
    SessionStatus, SessionTerminateRequest, SessionTermination,
};
use protocol::sip::{
    SipDialEstablished, SipDialInput, SIP_DIAL_OPERATION, SIP_DIAL_PROVIDER, SIP_DIAL_TOOL_REF,
};
use service::{
    admit_sip_dial, admit_voice_dial, AdmittedSipPlan, AdmittedVoicePlan, FixedHostResolution,
};
use service::{
    plan_operation, BackendCapabilities, BackendReadinessError, ConnectorBackend,
    PlanningEnvironment, PrincipalContext,
};
use sha2::{Digest as _, Sha256};
use tokio::sync::{watch, Semaphore};
use voice_runtime::VoiceSessionControl;

use connectors_config::PersonalVoiceConfig;

const B10X_DOCUMENT: &str = include_str!("../../../catalog/b10x.catalog.json");
const MAX_LIVE_SESSIONS: usize = 64;
const MAX_SESSION_RECORDS: usize = 1024;
const MAX_AUDIT_BYTES: u64 = 16 * 1024 * 1024;
const AUDIT_STATE_KEY: &str = "sip.audit";

/// Redaction-safe failure before an established session handle exists.
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("voice session launch failed: {code}")]
pub struct LaunchError {
    code: &'static str,
}

impl LaunchError {
    #[must_use]
    pub const fn new(code: &'static str) -> Self {
        Self { code }
    }
}

/// Established session custody returned by the isolated runtime launcher.
pub struct LaunchedSession {
    pub receipt: SipDialEstablished,
    pub control: VoiceSessionControl,
    pub completion: watch::Receiver<Option<SessionTermination>>,
}

/// Socket-owning runtime seam. Tests can prove operation semantics without opening SIP/RTVBP.
#[async_trait]
pub trait SessionLauncher: Send + Sync + 'static {
    /// Check mandatory launch dependencies without opening a provider connection or reading a
    /// credential value.
    async fn ready(&self) -> Result<(), LaunchError>;

    async fn launch(&self, admitted: AdmittedVoicePlan) -> Result<LaunchedSession, LaunchError>;

    /// Establish a **raw SIP call**: no application channel, no session authority.
    ///
    /// Defaulted to a refusal so an implementation that only carries calls onward to an application
    /// keeps compiling untouched, and one that only terminates them at the edge implements this and
    /// refuses [`launch`](Self::launch). Two bindings of one neutral session contract, neither
    /// pretending to be the other.
    async fn launch_sip(&self, _admitted: AdmittedSipPlan) -> Result<LaunchedSession, LaunchError> {
        Err(LaunchError::new("raw-sip-unsupported"))
    }
}

/// Configured implementation of the generic operation contract for exactly `sip.dial`.
pub struct SipOperationBackend<L> {
    config: PersonalVoiceConfig,
    principal: PrincipalContext,
    launcher: Arc<L>,
    document: Document,
    output_schema: serde_json::Value,
    catalog_sha256: String,
    deployment_sha256: String,
    routes: service::SipDialRouteTable,
    sessions: Mutex<BTreeMap<String, SessionRecord>>,
    audit: Arc<AuditJournal>,
    live_capacity: Arc<Semaphore>,
}

struct SessionRecord {
    operation_ref: String,
    connection_ref: String,
    audit_ref: String,
    control: VoiceSessionControl,
    completion: watch::Receiver<Option<SessionTermination>>,
    terminating: bool,
}

struct AuditJournal {
    path: PathBuf,
    hosted_state: Option<PostgresState>,
    writer: Mutex<()>,
}

#[derive(serde::Serialize)]
struct AuditEvent<'a> {
    audit_ref: &'a str,
    execution_ref: &'a str,
    operation_ref: &'a str,
    connection_ref: &'a str,
    tenant_id: &'a str,
    agent_id: &'a str,
    action: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    termination: Option<SessionTermination>,
}

impl AuditJournal {
    fn append(&self, event: AuditEvent<'_>) -> Result<(), std::io::Error> {
        let _guard = lock(&self.writer);
        let mut line = serde_json::to_vec(&serde_json::json!({
            "at_unix_seconds": SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(std::io::Error::other)?
                .as_secs(),
            "event": event,
        }))
        .map_err(std::io::Error::other)?;
        line.push(b'\n');
        if let Some(hosted_state) = &self.hosted_state {
            return hosted_state
                .append(AUDIT_STATE_KEY, &line, MAX_AUDIT_BYTES as usize)
                .map(|_| ())
                .map_err(|error| match error {
                    hosted_state::StateError::Capacity => std::io::Error::new(
                        std::io::ErrorKind::FileTooLarge,
                        "connector audit bound reached",
                    ),
                    _ => std::io::Error::other(error),
                });
        }
        let parent = self.path.parent().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "audit path has no parent")
        })?;
        let parent_metadata = fs::symlink_metadata(parent)?;
        if !parent_metadata.file_type().is_dir()
            || parent_metadata.uid() != rustix::process::geteuid().as_raw()
            || parent_metadata.permissions().mode() & 0o077 != 0
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "unsafe audit state root",
            ));
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(&self.path)?;
        let metadata = file.metadata()?;
        if !metadata.file_type().is_file()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "unsafe audit file",
            ));
        }
        if metadata
            .len()
            .checked_add(line.len() as u64)
            .is_none_or(|length| length > MAX_AUDIT_BYTES)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::FileTooLarge,
                "connector audit bound reached",
            ));
        }
        file.write_all(&line)?;
        file.sync_data()
    }
}

impl<L: SessionLauncher> SipOperationBackend<L> {
    /// Construct only after the canonical member and all deployment routes validate.
    pub fn new(
        config: PersonalVoiceConfig,
        launcher: Arc<L>,
        state_root: &Path,
    ) -> Result<Self, OperationError> {
        Self::new_inner(config, launcher, state_root, None)
    }

    /// Construct hosted SIP audit state against the service-owned PostgreSQL database.
    pub fn new_postgres(
        config: PersonalVoiceConfig,
        launcher: Arc<L>,
        state_root: &Path,
        hosted_state: PostgresState,
    ) -> Result<Self, OperationError> {
        Self::new_inner(config, launcher, state_root, Some(hosted_state))
    }

    fn new_inner(
        config: PersonalVoiceConfig,
        launcher: Arc<L>,
        state_root: &Path,
        hosted_state: Option<PostgresState>,
    ) -> Result<Self, OperationError> {
        let principal = config.principal_context().map_err(|_| unavailable())?;
        let document = Document::parse(B10X_DOCUMENT).map_err(|_| unavailable())?;
        let operation = document
            .operation(SIP_DIAL_OPERATION)
            .ok_or_else(unavailable)?;
        if document.connector != SIP_DIAL_PROVIDER
            || !operation.expose
            || operation.protocol_driver() != ProtocolDriver::SipV1
        {
            return Err(unavailable());
        }
        let output_schema = response_schema(B10X_DOCUMENT)?;
        let catalog_sha256 = format!("{:x}", Sha256::digest(B10X_DOCUMENT.as_bytes()));
        let deployment_sha256 = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&config).map_err(|_| unavailable())?)
        );
        let routes = config.sip_routes().map_err(|_| unavailable())?;
        Ok(Self {
            config,
            principal,
            launcher,
            document,
            output_schema,
            catalog_sha256,
            deployment_sha256,
            routes,
            sessions: Mutex::new(BTreeMap::new()),
            audit: Arc::new(AuditJournal {
                path: state_root.join("connector-audit.jsonl"),
                hosted_state,
                writer: Mutex::new(()),
            }),
            live_capacity: Arc::new(Semaphore::new(MAX_LIVE_SESSIONS)),
        })
    }

    fn check_context(&self, actual: &PrincipalContext) -> Result<(), OperationError> {
        if actual == &self.principal {
            Ok(())
        } else {
            Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
    }

    fn summary(&self) -> OperationSummary {
        let operation = self
            .document
            .operation(SIP_DIAL_OPERATION)
            .expect("validated canonical operation");
        OperationSummary {
            operation_ref: SIP_DIAL_TOOL_REF.to_owned(),
            title: "Dial a SIP voice session".to_owned(),
            effect: effect(operation.effects()),
            approval: ApprovalPosture::Required,
            connections: vec![self.connection()],
        }
    }

    fn connection(&self) -> ConnectionSummary {
        ConnectionSummary {
            connection_ref: self.config.connection.connection_ref.clone(),
            label: self.config.connection.label.clone(),
            provider: SIP_DIAL_PROVIDER.to_owned(),
            audiences: catalog::provider(catalog::ProviderKey::id(SIP_DIAL_PROVIDER))
                .map(|provider| {
                    provider
                        .audiences
                        .iter()
                        .map(|audience| audience.as_str().to_owned())
                        .collect()
                })
                .unwrap_or_default(),
            purpose: None,
        }
    }

    fn description_ref(&self, context: &PrincipalContext) -> String {
        let mut digest = Sha256::new();
        digest.update(self.catalog_sha256.as_bytes());
        digest.update(b"\0");
        digest.update(self.deployment_sha256.as_bytes());
        digest.update(b"\0");
        digest.update(context.stable_authority_seed());
        digest.update(b"\0");
        digest.update(self.config.connection.connection_ref.as_bytes());
        digest.update(b"\0");
        digest.update(self.config.connection.grant_ref.as_bytes());
        digest.update(b"\0");
        digest.update(self.config.connection.approval_evidence_ref.as_bytes());
        format!("description-sha256-{:x}", digest.finalize())
    }

    fn describe(
        &self,
        context: &PrincipalContext,
        request: DescribeRequest,
    ) -> Result<OperationResult, OperationError> {
        require_operation(&request.operation_ref)?;
        let operation = self
            .document
            .operation(SIP_DIAL_OPERATION)
            .expect("validated canonical operation");
        Ok(OperationResult::Describe(OperationDescription {
            operation_ref: SIP_DIAL_TOOL_REF.to_owned(),
            title: "Dial a SIP voice session".to_owned(),
            description: operation.contract_description().to_owned(),
            input_schema: operation.input_schema().clone(),
            output_schema: self.output_schema.clone(),
            effect: effect(operation.effects()),
            approval: ApprovalPosture::Required,
            connections: vec![self.connection()],
            description_ref: self.description_ref(context),
        }))
    }

    async fn invoke(
        &self,
        context: &PrincipalContext,
        request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        require_operation(&request.operation_ref)?;
        if request.connection_ref != self.config.connection.connection_ref {
            return Err(not_granted());
        }
        if request.description_ref != self.description_ref(context) {
            return Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "operation description lease is stale",
                false,
            ));
        }
        match request.approval_evidence_ref.as_deref() {
            None => {
                return Err(OperationError::new(
                    OperationErrorCode::ApprovalRequired,
                    "external approval evidence is required",
                    false,
                ))
            }
            Some(actual) if actual != self.config.connection.approval_evidence_ref => {
                return Err(OperationError::new(
                    OperationErrorCode::ApprovalDenied,
                    "external approval evidence is not current",
                    false,
                ))
            }
            Some(_) => {}
        }
        let input: SipDialInput = serde_json::from_value(request.input).map_err(|_| invalid())?;
        input.validate().map_err(|_| invalid())?;
        // The trunk actually selected, which is the default when the caller named none. Looking it
        // up from the caller's field would leave an untargeted dial authorized against nothing.
        let selected = self
            .routes
            .selected_alias(&input)
            .map_err(|_| not_granted())?;
        let permission_subject = self
            .config
            .permission_subject(&selected)
            .ok_or_else(not_granted)?;
        let operation = self
            .document
            .operation(SIP_DIAL_OPERATION)
            .expect("validated canonical operation");
        let connection = ConnectionAuthority::new(
            &self.config.connection.connection_ref,
            self.config.initiation_policy(),
        )
        .map_err(|_| not_granted())?;
        let admission = AdmittedOperation::from_grant_decision(
            SIP_DIAL_PROVIDER,
            SIP_DIAL_OPERATION,
            context.tenant_id(),
            context.actor_subject(),
            &self.config.connection.grant_ref,
            connection,
        );
        let plan = plan_operation(
            SIP_DIAL_PROVIDER,
            operation,
            admission,
            &PlanningEnvironment {
                available_drivers: BTreeSet::from([DriverId::SipV1]),
                available_route_adapters: BTreeSet::new(),
                capabilities: BTreeSet::from([Capability::PrivateNetwork]),
                permission_subjects: vec![permission_subject.to_owned()],
            },
        )
        .map_err(|_| not_granted())?;
        // **Raw SIP and SIP-plus-application-channel are two admissions, and both already existed.**
        // `admit_sip_dial` produces socket-opening evidence for the call itself; `admit_voice_dial`
        // additionally admits the application channel a call is carried onward to. Calling the
        // second unconditionally meant a Connection could not be admitted without an application
        // route, so raw SIP was unreachable — not because either protocol required the other, but
        // because this line only knew one of them.
        //
        // The launch is dispatched here rather than behind one plan type because the two carry
        // different evidence, and a type that could hold either would let a launcher receive
        // application evidence it must not act on.
        // **Resolve a named trunk here, off the reactor, before admission.**
        //
        // Two properties are being held at once. `getaddrinfo` blocks, and a telephony path that
        // blocks the runtime thread stalls every other call sharing it — so the lookup runs on a
        // blocking task. And the answer is handed *into* admission rather than used after it, so
        // the address DNS chose is aperture-checked like any other; resolving afterwards would let
        // anyone able to answer for the name redirect the call.
        let resolver = self.resolve_pending_host(&input).await?;

        let launched = match self.config.application_route() {
            Some(route) => {
                let admitted = admit_voice_dial(&plan, &input, &self.routes, route, &resolver)
                    .map_err(|_| not_granted())?;
                let live_permit = Arc::clone(&self.live_capacity)
                    .try_acquire_owned()
                    .map_err(|_| unavailable())?;
                self.prune_terminal_records()?;
                (self.launcher.launch(admitted).await, live_permit)
            }
            None => {
                let admitted = admit_sip_dial(&plan, &input, &self.routes, &resolver)
                    .map_err(|_| not_granted())?;
                let live_permit = Arc::clone(&self.live_capacity)
                    .try_acquire_owned()
                    .map_err(|_| unavailable())?;
                self.prune_terminal_records()?;
                (self.launcher.launch_sip(admitted).await, live_permit)
            }
        };
        let (launched, live_permit) = launched;
        let launched = launched.map_err(|_| unavailable())?;
        let execution_ref = opaque_ref("execution")?;
        let audit_ref = opaque_ref("audit")?;
        let output = match serde_json::to_value(&launched.receipt) {
            Ok(output) => output,
            Err(_) => {
                launched
                    .control
                    .terminate(TerminationReason::AuthorityRevoked);
                return Err(unavailable());
            }
        };
        if self
            .audit
            .append(AuditEvent {
                audit_ref: &audit_ref,
                execution_ref: &execution_ref,
                operation_ref: SIP_DIAL_TOOL_REF,
                connection_ref: &self.config.connection.connection_ref,
                tenant_id: context.tenant_id(),
                agent_id: context.actor_subject(),
                action: "session_established",
                termination: None,
            })
            .is_err()
        {
            launched
                .control
                .terminate(TerminationReason::AuthorityRevoked);
            return Err(unavailable());
        }
        let mut terminal = launched.completion.clone();
        let terminal_audit = Arc::clone(&self.audit);
        let terminal_audit_ref = audit_ref.clone();
        let terminal_execution_ref = execution_ref.clone();
        let terminal_connection_ref = self.config.connection.connection_ref.clone();
        let terminal_tenant_id = context.tenant_id().to_owned();
        let terminal_agent_id = context.actor_subject().to_owned();
        tokio::spawn(async move {
            let _live_permit = live_permit;
            loop {
                if let Some(termination) = *terminal.borrow_and_update() {
                    let _ = terminal_audit.append(AuditEvent {
                        audit_ref: &terminal_audit_ref,
                        execution_ref: &terminal_execution_ref,
                        operation_ref: SIP_DIAL_TOOL_REF,
                        connection_ref: &terminal_connection_ref,
                        tenant_id: &terminal_tenant_id,
                        agent_id: &terminal_agent_id,
                        action: "session_terminated",
                        termination: Some(termination),
                    });
                    break;
                }
                if terminal.changed().await.is_err() {
                    break;
                }
            }
        });
        lock(&self.sessions).insert(
            execution_ref.clone(),
            SessionRecord {
                operation_ref: SIP_DIAL_TOOL_REF.to_owned(),
                connection_ref: self.config.connection.connection_ref.clone(),
                audit_ref: audit_ref.clone(),
                control: launched.control,
                completion: launched.completion,
                terminating: false,
            },
        );
        Ok(OperationResult::Invoke(InvocationResult {
            operation_ref: SIP_DIAL_TOOL_REF.to_owned(),
            output,
            connector_audit_ref: audit_ref,
            execution_ref: Some(execution_ref),
        }))
    }

    /// Resolve this dial's trunk name, if it has one, without blocking the runtime thread.
    ///
    /// A trunk configured as a literal address resolves nothing and never leaves this function.
    async fn resolve_pending_host(
        &self,
        input: &SipDialInput,
    ) -> Result<FixedHostResolution, OperationError> {
        let Some((host, port)) = self
            .routes
            .pending_host(input)
            .map_err(|_| not_granted())?
        else {
            return Ok(FixedHostResolution(Vec::new()));
        };
        let resolved = tokio::task::spawn_blocking(move || {
            // The system resolver, in the order it answered. A DNS load balancer's ordering is
            // its own decision and is honoured rather than sorted or shuffled here.
            (host.as_str(), port)
                .to_socket_addrs()
                .map(Iterator::collect::<Vec<_>>)
                .unwrap_or_default()
        })
        .await
        .map_err(|_| unavailable())?;
        Ok(FixedHostResolution(resolved))
    }

    fn prune_terminal_records(&self) -> Result<(), OperationError> {
        let mut sessions = lock(&self.sessions);
        if sessions.len() < MAX_SESSION_RECORDS {
            return Ok(());
        }
        let remove = sessions
            .iter()
            .filter(|(_, record)| record.completion.borrow().is_some())
            .map(|(execution_ref, _)| execution_ref.clone())
            .take(sessions.len() - MAX_SESSION_RECORDS + 1)
            .collect::<Vec<_>>();
        for execution_ref in remove {
            sessions.remove(&execution_ref);
        }
        if sessions.len() >= MAX_SESSION_RECORDS {
            Err(unavailable())
        } else {
            Ok(())
        }
    }

    fn session_status(&self, request: SessionRequest) -> Result<SessionStatus, OperationError> {
        let sessions = lock(&self.sessions);
        let record = sessions.get(&request.execution_ref).ok_or_else(not_found)?;
        Ok(status(&request.execution_ref, record))
    }

    fn session_terminate(
        &self,
        request: SessionTerminateRequest,
    ) -> Result<SessionStatus, OperationError> {
        let mut sessions = lock(&self.sessions);
        let record = sessions
            .get_mut(&request.execution_ref)
            .ok_or_else(not_found)?;
        if record.completion.borrow().is_none() {
            self.audit
                .append(AuditEvent {
                    audit_ref: &record.audit_ref,
                    execution_ref: &request.execution_ref,
                    operation_ref: &record.operation_ref,
                    connection_ref: &record.connection_ref,
                    tenant_id: &self.config.owner.tenant_id,
                    agent_id: &self.config.owner.agent_id,
                    action: "termination_requested",
                    termination: Some(requested_termination(request.reason)),
                })
                .map_err(|_| unavailable())?;
            if record.control.terminate(termination_reason(request.reason)) {
                record.terminating = true;
            }
        }
        Ok(status(&request.execution_ref, record))
    }
}

#[async_trait]
impl<L: SessionLauncher> ConnectorBackend for SipOperationBackend<L> {
    async fn ready(&self) -> Result<(), BackendReadinessError> {
        self.launcher
            .ready()
            .await
            .map_err(|_| BackendReadinessError)
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::OPERATIONS
    }

    fn owns_operation(&self, request: &OperationRequest) -> bool {
        match request {
            OperationRequest::Describe(request) => request.operation_ref == SIP_DIAL_TOOL_REF,
            OperationRequest::Invoke(request) => {
                request.operation_ref == SIP_DIAL_TOOL_REF
                    && request.connection_ref == self.config.connection.connection_ref
            }
            OperationRequest::SessionStatus(request)
            | OperationRequest::SessionReconcile(request) => {
                lock(&self.sessions).contains_key(&request.execution_ref)
            }
            OperationRequest::SessionTerminate(request) => {
                lock(&self.sessions).contains_key(&request.execution_ref)
            }
            OperationRequest::Search(_) => false,
        }
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.check_context(context)?;
        match request {
            OperationRequest::Search(request) => {
                let needle = request.query.to_ascii_lowercase();
                let summary = self.summary();
                let haystack = format!(
                    "{} {} sip voice dial call peer pbx",
                    summary.operation_ref, summary.title
                )
                .to_ascii_lowercase();
                let operations = if needle.is_empty() || haystack.contains(&needle) {
                    vec![summary]
                } else {
                    Vec::new()
                };
                Ok(OperationResult::Search { operations })
            }
            OperationRequest::Describe(request) => self.describe(context, request),
            OperationRequest::Invoke(request) => self.invoke(context, request).await,
            OperationRequest::SessionStatus(request) => Ok(OperationResult::SessionStatus(
                self.session_status(request)?,
            )),
            OperationRequest::SessionTerminate(request) => Ok(OperationResult::SessionTerminate(
                self.session_terminate(request)?,
            )),
            OperationRequest::SessionReconcile(request) => {
                let status = self.session_status(request).map_err(|error| {
                    if error.code == OperationErrorCode::NotFound {
                        OperationError::new(
                            OperationErrorCode::OutcomeUnknown,
                            "session outcome is unknown to this daemon generation",
                            false,
                        )
                    } else {
                        error
                    }
                })?;
                Ok(OperationResult::SessionReconcile(status))
            }
        }
    }

    async fn shutdown(&self) {
        let sessions = {
            let sessions = lock(&self.sessions);
            sessions
                .iter()
                .map(|(execution_ref, record)| {
                    (
                        execution_ref.clone(),
                        record.operation_ref.clone(),
                        record.connection_ref.clone(),
                        record.audit_ref.clone(),
                        record.control.clone(),
                        record.completion.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        for (execution_ref, operation_ref, connection_ref, audit_ref, control, _) in &sessions {
            if control.terminate(TerminationReason::AuthorityRevoked) {
                let _ = self.audit.append(AuditEvent {
                    audit_ref,
                    execution_ref,
                    operation_ref,
                    connection_ref,
                    tenant_id: &self.config.owner.tenant_id,
                    agent_id: &self.config.owner.agent_id,
                    action: "daemon_shutdown_requested",
                    termination: Some(SessionTermination::Revoked),
                });
            }
        }
        for (_, _, _, _, _, mut completion) in sessions {
            loop {
                if completion.borrow_and_update().is_some() {
                    break;
                }
                if completion.changed().await.is_err() {
                    break;
                }
            }
        }
    }
}

fn response_schema(document: &str) -> Result<serde_json::Value, OperationError> {
    let value: serde_json::Value = serde_json::from_str(document).map_err(|_| unavailable())?;
    value["operations"]
        .as_array()
        .and_then(|operations| {
            operations
                .iter()
                .find(|operation| operation["id"] == SIP_DIAL_OPERATION)
        })
        .and_then(|operation| operation.get("response_schema"))
        .cloned()
        .ok_or_else(unavailable)
}

fn effect(effects: &[HostEffect]) -> EffectClass {
    if effects.contains(&HostEffect::Write) {
        EffectClass::Mutating
    } else {
        EffectClass::ReadOnly
    }
}

fn status(execution_ref: &str, record: &SessionRecord) -> SessionStatus {
    let termination = *record.completion.borrow();
    let state = if termination.is_some() {
        SessionState::Terminated
    } else if record.terminating {
        SessionState::Terminating
    } else {
        SessionState::Established
    };
    SessionStatus {
        execution_ref: execution_ref.to_owned(),
        operation_ref: record.operation_ref.clone(),
        connection_ref: record.connection_ref.clone(),
        state,
        termination,
        connector_audit_ref: record.audit_ref.clone(),
    }
}

fn termination_reason(reason: RequestedSessionTermination) -> TerminationReason {
    match reason {
        RequestedSessionTermination::Completed => TerminationReason::Completed,
        RequestedSessionTermination::Cancelled => TerminationReason::Cancelled,
        RequestedSessionTermination::Revoked => TerminationReason::AuthorityRevoked,
    }
}

fn requested_termination(reason: RequestedSessionTermination) -> SessionTermination {
    match reason {
        RequestedSessionTermination::Completed => SessionTermination::Completed,
        RequestedSessionTermination::Cancelled => SessionTermination::Cancelled,
        RequestedSessionTermination::Revoked => SessionTermination::Revoked,
    }
}

fn require_operation(actual: &str) -> Result<(), OperationError> {
    if actual == SIP_DIAL_TOOL_REF {
        Ok(())
    } else {
        Err(not_found())
    }
}

fn opaque_ref(prefix: &str) -> Result<String, OperationError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| unavailable())?;
    Ok(format!("{prefix}-{}", hex::encode(bytes)))
}

fn unavailable() -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        "connector runtime is unavailable",
        true,
    )
}

fn not_found() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotFound,
        "operation or session was not found",
        false,
    )
}

fn not_granted() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotGranted,
        "operation is not granted for this Connection",
        false,
    )
}

fn invalid() -> OperationError {
    OperationError::new(
        OperationErrorCode::InvalidInput,
        "operation input is invalid",
        false,
    )
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;
    use std::sync::atomic::{AtomicBool, Ordering};

    use protocol::operation::{
        DescribeRequest, InvokeRequest, OperationRequest, OperationResult, SearchRequest,
        SessionRequest, SessionState, SessionTerminateRequest,
    };
    use protocol::sip::SipDialState;
    use tokio::sync::watch;

    use super::*;
    use connectors_config::InitiationConfig;

    #[derive(Default)]
    struct FakeLauncher {
        unavailable: AtomicBool,
        routes: Mutex<Vec<String>>,
        completion: Mutex<Option<watch::Sender<Option<SessionTermination>>>>,
    }

    #[async_trait]
    impl SessionLauncher for FakeLauncher {
        async fn ready(&self) -> Result<(), LaunchError> {
            if self.unavailable.load(Ordering::Acquire) {
                Err(LaunchError::new("test_dependency_unavailable"))
            } else {
                Ok(())
            }
        }

        async fn launch(
            &self,
            admitted: AdmittedVoicePlan,
        ) -> Result<LaunchedSession, LaunchError> {
            lock(&self.routes).push(admitted.sip().route().to_uri.clone());
            let (sender, completion) = watch::channel(None);
            *lock(&self.completion) = Some(sender);
            Ok(LaunchedSession {
                receipt: SipDialEstablished {
                    call: "call-1".to_owned(),
                    session: "session-1".to_owned(),
                    channel: Some("channel-1".to_owned()),
                    state: SipDialState::Established,
                },
                control: VoiceSessionControl::new(),
                completion,
            })
        }
    }

    fn config() -> PersonalVoiceConfig {
        toml::from_str(
            r#"
[owner]
tenant_id = "tenant-1"
agent_id = "agent-1"
agent_revision = 7
authority_snapshot_id = "snapshot-7"
authority_snapshot_sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

[connection]
connection_ref = "connection-asterisk-dev"
label = "Asterisk development cluster"
grant_ref = "grant-sip-dial-1"
initiation = "b10x"
approval_evidence_ref = "approval-sip-dial-1"

[authority]
issuer = "https://connectors.example"
key_id = "voice-key-1"
signing_key_file = "/run/user/1000/b10x-voice.key"

[application]
actor = "connectors-voice"
audience = "application-voice"
deployment = "application-dev"
resource = "voice-channel"
endpoint = "wss://application.example/voice"
authority_lifetime_seconds = 30
session_lease_seconds = 60
connect_address = "127.0.0.1:7443"
tls_server_name = "application.example"

[[sip.targets]]
alias = "asterisk-dev"
permission_subject = "loopback:127.0.0.1"
signaling_bind = "127.0.0.1:0"
sent_by = "127.0.0.1"
target = "127.0.0.1:5060"
signaling_transport = "udp"
to_uri = "sip:callee@127.0.0.1:5060"
from_uri = "sip:caller@127.0.0.1"
media_advertised = "127.0.0.1"
media_bind = "127.0.0.1"
dial_timeout_seconds = 5
network_mode = "loopback"
signaling_apertures = [{ address = "127.0.0.1", first_port = 1, last_port = 65535 }]
media_apertures = [{ address = "127.0.0.1", first_port = 1, last_port = 65535 }]
"#,
        )
        .unwrap()
    }

    fn state_root() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        root
    }

    #[tokio::test]
    async fn readiness_delegates_to_the_mandatory_launcher_probe_without_launching() {
        let root = state_root();
        let launcher = Arc::new(FakeLauncher::default());
        let backend =
            SipOperationBackend::new(config(), Arc::clone(&launcher), root.path()).unwrap();

        backend.ready().await.unwrap();
        launcher.unavailable.store(true, Ordering::Release);
        assert_eq!(backend.ready().await, Err(BackendReadinessError));
        assert!(lock(&launcher.routes).is_empty());
    }

    async fn description_ref(
        backend: &SipOperationBackend<FakeLauncher>,
        context: &PrincipalContext,
    ) -> String {
        match backend
            .handle(
                context,
                OperationRequest::Describe(DescribeRequest {
                    operation_ref: SIP_DIAL_TOOL_REF.to_owned(),
                }),
            )
            .await
            .unwrap()
        {
            OperationResult::Describe(description) => {
                assert_eq!(description.effect, EffectClass::Mutating);
                assert_eq!(description.approval, ApprovalPosture::Required);
                // **Neither input is required, and that is the contract.** The Connection names
                // the trunk (or declares a default), and the trunk names its own endpoint, so a
                // caller that supplies nothing still dials something admitted. This asserted
                // `required[0] == "target"` while the projection marked every parameter mandatory
                // regardless of what the provider declared.
                assert_eq!(
                    description.input_schema["required"].as_array().map(Vec::len),
                    Some(0)
                );
                let properties = description.input_schema["properties"]
                    .as_object()
                    .expect("the input schema declares properties");
                assert!(properties.contains_key("target"));
                assert!(
                    properties.contains_key("number"),
                    "the dialled number must reach the model-facing surface"
                );
                assert_eq!(description.output_schema["required"][0], "call");
                description.description_ref
            }
            other => panic!("unexpected result: {other:?}"),
        }
    }

    fn invoke(description_ref: String, approval: Option<&str>, target: &str) -> OperationRequest {
        OperationRequest::Invoke(InvokeRequest {
            operation_ref: SIP_DIAL_TOOL_REF.to_owned(),
            connection_ref: "connection-asterisk-dev".to_owned(),
            description_ref,
            input: serde_json::json!({"target": target}),
            approval_evidence_ref: approval.map(str::to_owned),
        })
    }

    #[tokio::test]
    async fn catalog_projection_invocation_session_control_and_audit_share_one_path() {
        let root = state_root();
        let launcher = Arc::new(FakeLauncher::default());
        let backend =
            SipOperationBackend::new(config(), Arc::clone(&launcher), root.path()).unwrap();
        let context = backend.principal.clone();

        let search = backend
            .handle(
                &context,
                OperationRequest::Search(SearchRequest {
                    query: "voice".to_owned(),
                    limit: 10,
                }),
            )
            .await
            .unwrap();
        let OperationResult::Search { operations } = search else {
            panic!("search result expected")
        };
        assert_eq!(operations[0].operation_ref, SIP_DIAL_TOOL_REF);
        let lease = description_ref(&backend, &context).await;

        let missing_approval = backend
            .handle(&context, invoke(lease.clone(), None, "asterisk-dev"))
            .await
            .unwrap_err();
        assert_eq!(missing_approval.code, OperationErrorCode::ApprovalRequired);
        let injected_destination = backend
            .handle(
                &context,
                invoke(
                    lease.clone(),
                    Some("approval-sip-dial-1"),
                    "sip:callee@127.0.0.1:5060",
                ),
            )
            .await
            .unwrap_err();
        assert_eq!(injected_destination.code, OperationErrorCode::InvalidInput);

        let result = backend
            .handle(
                &context,
                invoke(lease, Some("approval-sip-dial-1"), "asterisk-dev"),
            )
            .await
            .unwrap();
        let OperationResult::Invoke(invocation) = result else {
            panic!("invoke result expected")
        };
        assert_eq!(invocation.output["state"], "established");
        assert_eq!(
            lock(&launcher.routes).as_slice(),
            ["sip:callee@127.0.0.1:5060"]
        );
        let execution_ref = invocation.execution_ref.unwrap();

        let status = backend
            .handle(
                &context,
                OperationRequest::SessionStatus(SessionRequest {
                    execution_ref: execution_ref.clone(),
                }),
            )
            .await
            .unwrap();
        let OperationResult::SessionStatus(status) = status else {
            panic!("status expected")
        };
        assert_eq!(status.state, SessionState::Established);

        let terminating = backend
            .handle(
                &context,
                OperationRequest::SessionTerminate(SessionTerminateRequest {
                    execution_ref: execution_ref.clone(),
                    reason: RequestedSessionTermination::Cancelled,
                }),
            )
            .await
            .unwrap();
        let OperationResult::SessionTerminate(terminating) = terminating else {
            panic!("terminate result expected")
        };
        assert_eq!(terminating.state, SessionState::Terminating);
        lock(&launcher.completion)
            .as_ref()
            .unwrap()
            .send_replace(Some(SessionTermination::Cancelled));
        tokio::task::yield_now().await;
        let terminal = backend
            .handle(
                &context,
                OperationRequest::SessionStatus(SessionRequest {
                    execution_ref: execution_ref.clone(),
                }),
            )
            .await
            .unwrap();
        let OperationResult::SessionStatus(terminal) = terminal else {
            panic!("terminal status expected")
        };
        assert_eq!(terminal.state, SessionState::Terminated);
        assert_eq!(terminal.termination, Some(SessionTermination::Cancelled));

        for _ in 0..10 {
            let audit = std::fs::read_to_string(root.path().join("connector-audit.jsonl")).unwrap();
            if audit.contains("session_terminated") {
                assert!(audit.contains("session_established"));
                assert!(audit.contains("termination_requested"));
                assert!(!audit.contains("sip:callee"));
                return;
            }
            tokio::task::yield_now().await;
        }
        panic!("terminal audit event was not written");
    }

    #[tokio::test]
    async fn stale_owner_provider_only_unknown_alias_and_restart_reconciliation_refuse() {
        let root = state_root();
        let launcher = Arc::new(FakeLauncher::default());
        let mut configured = config();
        configured.connection.initiation = InitiationConfig::Provider;
        let backend =
            SipOperationBackend::new(configured, Arc::clone(&launcher), root.path()).unwrap();
        let context = backend.principal.clone();
        let lease = description_ref(&backend, &context).await;
        let refused = backend
            .handle(
                &context,
                invoke(lease, Some("approval-sip-dial-1"), "asterisk-dev"),
            )
            .await
            .unwrap_err();
        assert_eq!(refused.code, OperationErrorCode::NotGranted);
        assert!(lock(&launcher.routes).is_empty());

        let alias_launcher = Arc::new(FakeLauncher::default());
        let alias_backend =
            SipOperationBackend::new(config(), Arc::clone(&alias_launcher), root.path()).unwrap();
        let alias_context = alias_backend.principal.clone();
        let alias_lease = description_ref(&alias_backend, &alias_context).await;
        let mut changed_route = config();
        changed_route
            .application
            .as_mut()
            .expect("this fixture configures an application channel")
            .resource = "different-voice-channel".to_owned();
        let changed_backend = SipOperationBackend::new(
            changed_route,
            Arc::new(FakeLauncher::default()),
            root.path(),
        )
        .unwrap();
        assert_ne!(
            alias_lease,
            description_ref(&changed_backend, &alias_context).await,
            "description leases must bind deployment-selected routing"
        );
        let unknown_alias = alias_backend
            .handle(
                &alias_context,
                invoke(alias_lease, Some("approval-sip-dial-1"), "unconfigured-pbx"),
            )
            .await
            .unwrap_err();
        assert_eq!(unknown_alias.code, OperationErrorCode::NotGranted);
        assert!(lock(&alias_launcher.routes).is_empty());

        let mut stale_owner = backend.config.owner_context();
        stale_owner.agent_revision += 1;
        let stale = PrincipalContext::local(&stale_owner).unwrap();
        let stale = backend
            .handle(
                &stale,
                OperationRequest::Search(SearchRequest {
                    query: String::new(),
                    limit: 1,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(stale.code, OperationErrorCode::StaleAuthority);

        let reconciled = backend
            .handle(
                &context,
                OperationRequest::SessionReconcile(SessionRequest {
                    execution_ref: "execution-from-an-older-process".to_owned(),
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(reconciled.code, OperationErrorCode::OutcomeUnknown);
    }
}

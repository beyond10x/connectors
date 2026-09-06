//! Personal and hosted Slack composition share one recovery and supervision owner.

use super::*;

impl SlackBackend {
    /// Open owner-only state, recover any decided credential transaction, and supervise every
    /// callable Slack Connection. The configuration contains policy only; no ambient secret source
    /// is consulted.
    pub async fn open(
        owner: PrincipalContext,
        policy: SlackIntegrationConfig,
        state_root: &Path,
        credential_store: Arc<dyn PreparedSecretStore>,
        egress: Arc<dyn EgressTransport>,
    ) -> Result<Self, SlackError> {
        Self::open_inner(SlackOpenContext {
            admission: PrincipalAdmission::Exact(Box::new(owner)),
            completion_mode: CompletionMode::Local,
            policy,
            state_root,
            credential_store,
            egress,
            hosted_state: None,
            supervision_enabled: true,
        })
        .await
    }

    /// Open the hosted Slack Integration for one Identity tenant. Connect Sessions complete over
    /// the exact public Connector origin and every credential is committed through `credential_store`.
    pub async fn open_hosted(
        tenant_id: String,
        public_origin: url::Url,
        policy: SlackIntegrationConfig,
        state_root: &Path,
        credential_store: Arc<dyn PreparedSecretStore>,
        hosted_state: Arc<dyn StateStore>,
        egress: Arc<dyn EgressTransport>,
    ) -> Result<Self, SlackError> {
        Self::open_inner(SlackOpenContext {
            admission: PrincipalAdmission::Tenant(tenant_id),
            completion_mode: CompletionMode::Hosted { public_origin },
            policy,
            state_root,
            credential_store,
            egress,
            hosted_state: Some(hosted_state),
            supervision_enabled: true,
        })
        .await
    }

    /// Open the same durable personal state for bounded commands without starting Socket Mode.
    pub async fn open_without_supervision(
        owner: PrincipalContext,
        policy: SlackIntegrationConfig,
        state_root: &Path,
        credential_store: Arc<dyn PreparedSecretStore>,
        egress: Arc<dyn EgressTransport>,
    ) -> Result<Self, SlackError> {
        Self::open_inner(SlackOpenContext {
            admission: PrincipalAdmission::Exact(Box::new(owner)),
            completion_mode: CompletionMode::Local,
            policy,
            state_root,
            credential_store,
            egress,
            hosted_state: None,
            supervision_enabled: false,
        })
        .await
    }

    #[cfg(test)]
    pub(super) async fn open_with_supervision(
        owner: PrincipalContext,
        policy: SlackIntegrationConfig,
        state_root: &Path,
        credential_store: Arc<dyn PreparedSecretStore>,
        supervision_enabled: bool,
    ) -> Result<Self, SlackError> {
        Self::open_inner(SlackOpenContext {
            admission: PrincipalAdmission::Exact(Box::new(owner)),
            completion_mode: CompletionMode::Local,
            policy,
            state_root,
            credential_store,
            egress: test_egress(),
            hosted_state: None,
            supervision_enabled,
        })
        .await
    }

    pub(super) async fn open_inner(context: SlackOpenContext<'_>) -> Result<Self, SlackError> {
        let SlackOpenContext {
            admission,
            completion_mode,
            policy,
            state_root,
            credential_store,
            egress,
            hosted_state,
            supervision_enabled,
        } = context;
        let metadata = read_state(state_root, hosted_state.as_deref())?;
        let event_store = Arc::new(EventStore::open(
            state_root.join("events.jsonl"),
            hosted_state.clone(),
        )?);
        let (shutdown, _) = watch::channel(false);
        let inner = Arc::new(SlackInner {
            admission,
            completion_mode,
            policy,
            state_root: state_root.to_path_buf(),
            hosted_state: hosted_state.clone(),
            credential_store,
            metadata: Mutex::new(metadata),
            sessions: Mutex::new(
                ConnectSessionLifecycle::new(INTEGRATION_REF, MAX_CONNECT_SESSIONS)
                    .map_err(|_| SlackError::new("connect-session-lifecycle"))?,
            ),
            session_owners: Mutex::new(BTreeMap::new()),
            hosted_sessions: Mutex::new(BTreeMap::new()),
            oauth_states: Mutex::new(PendingStates::new(DEFAULT_PENDING_CAPACITY)),
            hosted_completion_lock: tokio::sync::Mutex::new(()),
            event_store,
            audit: AuditJournal::new(state_root.join("slack-operation-audit.jsonl"), hosted_state),
            channel_states: Mutex::new(BTreeMap::new()),
            supervisors_started: Mutex::new(std::collections::BTreeSet::new()),
            shutdown,
            tasks: Mutex::new(Vec::new()),
            egress,
            supervision_enabled,
        });
        inner.recover_pending().await?;
        inner.ensure_org_connection().await?;
        inner.ensure_declared_instances().await?;
        for connection in lock(&inner.metadata).connections.clone() {
            if inner.connection_is_admitted(&connection) {
                inner.start_supervisor(connection);
            }
        }
        Ok(Self { inner })
    }
}

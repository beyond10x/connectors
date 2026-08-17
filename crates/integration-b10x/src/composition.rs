use hosted_state::PostgresState;

use super::*;

impl B10xBackend {
    /// Compose a personal-local backend pinned to one exact Agent authority snapshot.
    pub fn personal(
        config: B10xIntegrationConfig,
        principal: PrincipalContext,
        state_root: &Path,
    ) -> Result<Self, B10xIntegrationError> {
        Self::new(
            config,
            PrincipalAdmission::Exact(principal),
            state_root,
            None,
        )
    }

    /// Compose a hosted backend for Identity-verified principals in one tenant.
    pub fn hosted(
        config: B10xIntegrationConfig,
        tenant_ids: Vec<String>,
        state_root: &Path,
    ) -> Result<Self, B10xIntegrationError> {
        Self::hosted_inner(config, tenant_ids, state_root, None)
    }

    /// Compose hosted B10x state against the service-owned PostgreSQL database.
    pub fn hosted_postgres(
        config: B10xIntegrationConfig,
        tenant_ids: Vec<String>,
        state_root: &Path,
        hosted_state: PostgresState,
    ) -> Result<Self, B10xIntegrationError> {
        Self::hosted_inner(config, tenant_ids, state_root, Some(hosted_state))
    }

    fn hosted_inner(
        config: B10xIntegrationConfig,
        tenant_ids: Vec<String>,
        state_root: &Path,
        hosted_state: Option<PostgresState>,
    ) -> Result<Self, B10xIntegrationError> {
        if tenant_ids.is_empty() {
            return Err(B10xIntegrationError::InvalidConfiguration);
        }
        Self::new(
            config,
            PrincipalAdmission::Tenants(tenant_ids.into_iter().collect()),
            state_root,
            hosted_state,
        )
    }

    fn new(
        config: B10xIntegrationConfig,
        admission: PrincipalAdmission,
        state_root: &Path,
        hosted_state: Option<PostgresState>,
    ) -> Result<Self, B10xIntegrationError> {
        let legacy_event_tenant = match &admission {
            PrincipalAdmission::Exact(principal) => Some(principal.tenant_id().to_owned()),
            PrincipalAdmission::Tenants(tenants) if tenants.len() == 1 => {
                tenants.iter().next().cloned()
            }
            PrincipalAdmission::Tenants(_) => None,
        };
        let module_signer = ModuleSigner::load(&config)?;
        let document = Document::parse(DOCUMENT)
            .map_err(|_| B10xIntegrationError::InvalidConfiguration)?;
        let catalog: Value = serde_json::from_str(DOCUMENT)
            .map_err(|_| B10xIntegrationError::InvalidConfiguration)?;
        if document.connector != PROVIDER {
            return Err(B10xIntegrationError::InvalidConfiguration);
        }
        let client = http_client(HTTP_CONNECT_TIMEOUT, HTTP_TOTAL_TIMEOUT)?;
        let deployment_sha256 = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&config)
                    .map_err(|_| B10xIntegrationError::InvalidConfiguration)?,
            )
        );
        let work_events = ModuleEventStore::open(
            "work",
            state_root.join("b10x-work-events.json"),
            legacy_event_tenant.as_deref(),
            hosted_state.clone(),
            "b10x.work-events",
        )
        .map_err(|()| B10xIntegrationError::InvalidConfiguration)?;
        let planner_events = ModuleEventStore::open(
            "planner",
            state_root.join("b10x-planner-events.json"),
            None,
            hosted_state.clone(),
            "b10x.planner-events",
        )
        .map_err(|()| B10xIntegrationError::InvalidConfiguration)?;
        Ok(Self {
            audio: config.audio_route().map(|_| Arc::new(Mutex::new(None))),
            browser: config.browser_route().map(|_| Arc::new(Mutex::new(None))),
            config,
            admission,
            document,
            catalog,
            client,
            http_total_timeout: HTTP_TOTAL_TIMEOUT,
            catalog_sha256: format!("{:x}", Sha256::digest(DOCUMENT.as_bytes())),
            deployment_sha256,
            audit: AuditJournal::new(
                state_root.join("b10x-operation-audit.jsonl"),
                hosted_state,
            ),
            work_events,
            planner_events,
            module_signer,
        })
    }
}

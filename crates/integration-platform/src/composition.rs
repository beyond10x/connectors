use std::path::Path;
use std::sync::Arc;

use connector_state::StateStore;

use super::*;

impl PlatformBackend {
    /// Compose a personal-local backend pinned to one exact Agent authority snapshot.
    pub fn personal(
        config: PlatformIntegrationConfig,
        principal: PrincipalContext,
        state_root: &Path,
    ) -> Result<Self, PlatformIntegrationError> {
        Self::new(
            config,
            PrincipalAdmission::Exact(Box::new(principal)),
            state_root,
            None,
        )
    }

    /// Compose a hosted backend for Identity-verified principals in one tenant.
    pub fn hosted(
        config: PlatformIntegrationConfig,
        tenant_ids: Vec<String>,
        state_root: &Path,
    ) -> Result<Self, PlatformIntegrationError> {
        Self::hosted_inner(config, tenant_ids, state_root, None)
    }

    /// Compose hosted platform state against the durable store the deployment bound.
    ///
    /// This used to name PostgreSQL, and naming it meant a person running the product on their own
    /// machine had to keep a database server alive to hold an audit journal and an event cursor.
    /// Which backend holds them is the deployment's choice; nothing below can see it.
    pub fn hosted_with_state(
        config: PlatformIntegrationConfig,
        tenant_ids: Vec<String>,
        state_root: &Path,
        hosted_state: Arc<dyn StateStore>,
    ) -> Result<Self, PlatformIntegrationError> {
        Self::hosted_inner(config, tenant_ids, state_root, Some(hosted_state))
    }

    fn hosted_inner(
        config: PlatformIntegrationConfig,
        tenant_ids: Vec<String>,
        state_root: &Path,
        hosted_state: Option<Arc<dyn StateStore>>,
    ) -> Result<Self, PlatformIntegrationError> {
        if tenant_ids.is_empty() {
            return Err(PlatformIntegrationError::InvalidConfiguration);
        }
        Self::new(
            config,
            PrincipalAdmission::Tenants(tenant_ids.into_iter().collect()),
            state_root,
            hosted_state,
        )
    }

    fn new(
        config: PlatformIntegrationConfig,
        admission: PrincipalAdmission,
        state_root: &Path,
        hosted_state: Option<Arc<dyn StateStore>>,
    ) -> Result<Self, PlatformIntegrationError> {
        for socket in config.module_sockets.values() {
            transport::validate_module_socket(socket)?;
        }
        let legacy_event_tenant = match &admission {
            PrincipalAdmission::Exact(principal) => Some(principal.tenant_id().to_owned()),
            PrincipalAdmission::Tenants(tenants) if tenants.len() == 1 => {
                tenants.iter().next().cloned()
            }
            PrincipalAdmission::Tenants(_) => None,
        };
        let module_signer = ModuleSigner::load(&config)?;
        let document = Document::parse(DOCUMENT)
            .map_err(|_| PlatformIntegrationError::InvalidConfiguration)?;
        let catalog: Value = serde_json::from_str(DOCUMENT)
            .map_err(|_| PlatformIntegrationError::InvalidConfiguration)?;
        if document.connector != PROVIDER {
            return Err(PlatformIntegrationError::InvalidConfiguration);
        }
        let client = http_client(HTTP_CONNECT_TIMEOUT, HTTP_TOTAL_TIMEOUT)?;
        let deployment_sha256 = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&config)
                    .map_err(|_| PlatformIntegrationError::InvalidConfiguration)?,
            )
        );
        let work_events = ModuleEventStore::open(
            "work",
            state_root.join("b10x-work-events.json"),
            legacy_event_tenant.as_deref(),
            hosted_state.clone(),
            "b10x.work-events",
        )
        .map_err(|()| PlatformIntegrationError::InvalidConfiguration)?;
        let planner_events = ModuleEventStore::open(
            "planner",
            state_root.join("b10x-planner-events.json"),
            None,
            hosted_state.clone(),
            "b10x.planner-events",
        )
        .map_err(|()| PlatformIntegrationError::InvalidConfiguration)?;
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
            audit: AuditJournal::new(state_root.join("b10x-operation-audit.jsonl"), hosted_state),
            work_events,
            planner_events,
            module_signer,
        })
    }
}

//! Local endpoint setup and passive kubeconfig metadata.

use super::*;

/// Passive kubeconfig setup metadata. No token, cluster URL, or helper command is projected.
#[derive(Debug, Clone)]
pub struct LocalContextSummary {
    pub name: String,
    pub namespace: Option<String>,
    pub current: bool,
    pub requires_exec_auth: bool,
}

/// List safe context metadata without constructing a client, running a helper, or contacting a cluster.
pub fn local_contexts() -> Result<Vec<LocalContextSummary>, KubernetesLocalError> {
    let kubeconfig = Kubeconfig::read().map_err(|_| KubernetesLocalError::Kubeconfig)?;
    let mut contexts = kubeconfig
        .contexts
        .iter()
        .filter_map(|entry| {
            let context = entry.context.as_ref()?;
            binding_for_context(&kubeconfig, &entry.name)?;
            Some(LocalContextSummary {
                name: entry.name.clone(),
                namespace: context.namespace.clone(),
                current: kubeconfig.current_context.as_deref() == Some(&entry.name),
                requires_exec_auth: context_uses_credential_plugin(&kubeconfig, &entry.name),
            })
        })
        .collect::<Vec<_>>();
    contexts.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(contexts)
}

impl KubernetesLocalBackend {
    /// Compose the durable endpoint plane with the runtime's existing metadata store.
    pub fn open_with_state(
        owner: PrincipalContext,
        policy: KubernetesIntegrationConfig,
        state_root: &Path,
        state: Arc<dyn connector_state::StateStore>,
    ) -> Result<Self, KubernetesLocalError> {
        let mut backend = Self::open(owner, policy, state_root)?;
        backend.endpoint_state = Some(state);
        Ok(backend)
    }

    /// Inject runtime-owned provider execution before source activation.
    pub fn with_endpoint_backend_factory(
        mut self,
        factory: Arc<dyn EndpointBackendFactory>,
    ) -> Self {
        self.endpoint_factory = Some(factory);
        self
    }

    /// Reattach the explicitly selected source during daemon startup, including its stored
    /// credential-helper consent. Deterministic source identity restores endpoint references.
    pub async fn restore_selected_context(&self) -> Result<(), KubernetesLocalError> {
        let Some(selected) = self.policy.selected_context.as_deref() else {
            return Ok(());
        };
        let candidate = self
            .candidates
            .values()
            .find(|candidate| candidate.context_name == selected)
            .ok_or(KubernetesLocalError::EndpointSource)?;
        let kubeconfig = Kubeconfig::read().map_err(|_| KubernetesLocalError::EndpointSource)?;
        let fresh = binding_for_context(&kubeconfig, selected)
            .ok_or(KubernetesLocalError::EndpointSource)?;
        if fresh.evidence_material != candidate.evidence_material
            || context_uses_credential_plugin(&kubeconfig, selected) && !self.policy.allow_exec_auth
        {
            return Err(KubernetesLocalError::EndpointSource);
        }
        // Config construction reads trusted local files only. Helpers and API access belong to
        // the owned refresh/invocation task, so an offline cluster cannot stall daemon startup.
        let mut config = Config::from_custom_kubeconfig(
            kubeconfig,
            &KubeConfigOptions {
                context: Some(selected.to_owned()),
                ..KubeConfigOptions::default()
            },
        )
        .await
        .map_err(|_| KubernetesLocalError::EndpointSource)?;
        config.proxy_url = None;
        config.connect_timeout = Some(std::time::Duration::from_secs(5));
        let client = Client::try_from(config).map_err(|_| KubernetesLocalError::EndpointSource)?;
        let connection_ref = opaque_ref(
            "connection:kubernetes:",
            &format!(
                "{}\0{}",
                candidate.summary.candidate_ref, candidate.evidence_material
            ),
        );
        let store = self
            .endpoint_state
            .as_ref()
            .ok_or(KubernetesLocalError::EndpointSource)?;
        let factory = self
            .endpoint_factory
            .as_ref()
            .ok_or(KubernetesLocalError::EndpointSource)?;
        let source = Arc::new(
            KubernetesEndpointSource::new(
                client.clone(),
                connection_ref.clone(),
                self.policy.namespaces.iter().cloned().collect(),
                self.policy.all_namespaces,
                self.policy.target_grants.clone(),
                store.clone(),
                EndpointPlacement::Local,
            )
            .map_err(|_| KubernetesLocalError::EndpointSource)?,
        );
        let backend = factory.build(
            source,
            EndpointPrincipalPolicy::Local(Arc::new(self.owner.clone())),
        );
        let description = ConnectionDescription {
            summary: ConnectionSummary {
                connection_ref: connection_ref.clone(),
                integration_ref: KUBERNETES.to_owned(),
                label: selected.to_owned(),
                state: ConnectionState::Authorized,
                initiation: initiation(self.policy.initiation),
                route: ConnectionRoute::Direct,
                scope: None,
                actor: None,
                auth_profile: None,
            },
            channels: Vec::new(),
        };
        let mut state = lock(&self.state);
        state.candidate_connections.insert(
            candidate.summary.candidate_ref.clone(),
            connection_ref.clone(),
        );
        state.clients.insert(connection_ref.clone(), client);
        state.connections.insert(connection_ref, description);
        *lock(&self.endpoint_backend) = Some(backend);
        Ok(())
    }

    pub(super) fn endpoint_backend(&self) -> Option<Arc<dyn ConnectorBackend>> {
        lock(&self.endpoint_backend).clone()
    }
}

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

    /// Inject the runtime-owned HTTP/WebSocket route aperture before source activation.
    pub fn with_endpoint_egress(mut self, egress: Arc<dyn EndpointEgressFactory>) -> Self {
        self.endpoint_egress = Some(egress);
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
        self.activate(CandidateActivateRequest {
            candidate_ref: candidate.summary.candidate_ref.clone(),
            label: selected.to_owned(),
        })
        .await
        .map_err(|_| KubernetesLocalError::EndpointSource)?;
        Ok(())
    }

    pub(super) fn endpoint_backend(&self) -> Option<Arc<KubernetesEndpointBackend>> {
        lock(&self.endpoint_backend).clone()
    }
}

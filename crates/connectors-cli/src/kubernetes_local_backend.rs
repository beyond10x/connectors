//! Trusted personal-local kubeconfig discovery and bounded Kubernetes monitoring discovery.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use k8s_openapi::api::authentication::v1::SelfSubjectReview;
use k8s_openapi::api::authorization::v1::{
    ResourceAttributes, SelfSubjectAccessReview, SelfSubjectAccessReviewSpec,
};
use k8s_openapi::api::core::v1::Service;
use kube::api::{ListParams, PostParams};
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::{Api, Client, Config};
use protocol::connection::{
    CandidateActivateRequest, CandidateSearchRequest, ConnectionCandidateState,
    ConnectionCandidateSummary, ConnectionDescription, ConnectionError, ConnectionErrorCode,
    ConnectionInitiator, ConnectionRequest, ConnectionResult, ConnectionRoute, ConnectionState,
    ConnectionSummary, DiscoveryObservationState, DiscoveryObservationSummary,
    ObservationSearchRequest,
};
use protocol::event::{EventError, EventRequest, EventResult};
use protocol::operation::{OperationError, OperationRequest, OperationResult, OwnerContext};
use server::local::OperationBackend;
use sha2::{Digest as _, Sha256};

use crate::{InitiationConfig, KubernetesIntegrationConfig};

const KUBERNETES: &str = "kubernetes";
const DISCOVERY_REF: &str = "discovery:kubernetes-service-v1";

/// Setup refusal for the personal-local Kubernetes backend. Details deliberately do not include
/// kubeconfig contents, credential helpers, endpoints, or paths.
#[derive(Debug, thiserror::Error)]
pub enum KubernetesLocalError {
    #[error("standard kubeconfig contexts could not be discovered")]
    Kubeconfig,
}

#[derive(Debug, Clone)]
struct CandidateBinding {
    summary: ConnectionCandidateSummary,
    context_name: String,
    evidence_material: String,
}

#[derive(Debug, Default)]
struct KubernetesState {
    connections: BTreeMap<String, ConnectionDescription>,
    candidate_connections: BTreeMap<String, String>,
    observations: BTreeMap<String, Vec<DiscoveryObservationSummary>>,
}

/// Personal-local backend which passively detects kubeconfig contexts, then contacts a cluster
/// only after one opaque candidate is explicitly activated.
pub struct KubernetesLocalBackend {
    operation: Arc<dyn OperationBackend>,
    owner: OwnerContext,
    policy: KubernetesIntegrationConfig,
    candidates: BTreeMap<String, CandidateBinding>,
    state: Mutex<KubernetesState>,
    activation: tokio::sync::Mutex<()>,
}

impl KubernetesLocalBackend {
    /// Read context metadata from the standard merged kubeconfig. No cluster request or auth exec
    /// occurs here. Credential-bearing fields remain private to this trusted Connector process.
    pub fn open(
        owner: OwnerContext,
        policy: KubernetesIntegrationConfig,
        _state_root: &Path,
        operation: Arc<dyn OperationBackend>,
    ) -> Result<Self, KubernetesLocalError> {
        let kubeconfig = Kubeconfig::read().map_err(|_| KubernetesLocalError::Kubeconfig)?;
        let candidates = candidates(&kubeconfig);
        Ok(Self {
            operation,
            owner,
            policy,
            candidates,
            state: Mutex::new(KubernetesState::default()),
            activation: tokio::sync::Mutex::new(()),
        })
    }

    /// Number of activated Kubernetes source Connections in this daemon generation.
    #[must_use]
    pub fn connection_count(&self) -> usize {
        lock(&self.state).connections.len()
    }

    /// Number of passively detected context candidates.
    #[must_use]
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    fn check_context(&self, context: &OwnerContext) -> Result<(), ConnectionError> {
        if context != &self.owner {
            return Err(ConnectionError::new(
                ConnectionErrorCode::StaleAuthority,
                "owner context does not match this Connector generation",
                false,
            ));
        }
        Ok(())
    }

    fn search_candidates(
        &self,
        request: &CandidateSearchRequest,
    ) -> Vec<ConnectionCandidateSummary> {
        let query = request.query.to_ascii_lowercase();
        let state = lock(&self.state);
        self.candidates
            .values()
            .filter(|candidate| {
                candidate
                    .summary
                    .title
                    .to_ascii_lowercase()
                    .contains(&query)
            })
            .map(|candidate| {
                let mut summary = candidate.summary.clone();
                if let Some(connection_ref) = state
                    .candidate_connections
                    .get(&candidate.summary.candidate_ref)
                {
                    summary.state = ConnectionCandidateState::Activated;
                    summary.connection_ref = Some(connection_ref.clone());
                }
                summary
            })
            .take(usize::from(request.limit))
            .collect()
    }

    fn search_connections(&self, query: &str) -> Vec<ConnectionSummary> {
        let query = query.to_ascii_lowercase();
        lock(&self.state)
            .connections
            .values()
            .map(|connection| connection.summary.clone())
            .filter(|connection| connection.label.to_ascii_lowercase().contains(&query))
            .collect()
    }

    async fn activate(
        &self,
        request: CandidateActivateRequest,
    ) -> Result<ConnectionDescription, ConnectionError> {
        let candidate = self
            .candidates
            .get(&request.candidate_ref)
            .cloned()
            .ok_or_else(connection_not_found)?;
        // Activation can invoke an external credential helper. Serialize and re-check so repeated
        // calls cannot run that effect more than once for the same candidate in this generation.
        let _activation = self.activation.lock().await;
        if let Some(connection_ref) = lock(&self.state)
            .candidate_connections
            .get(&request.candidate_ref)
            .cloned()
        {
            return lock(&self.state)
                .connections
                .get(&connection_ref)
                .cloned()
                .ok_or_else(connection_protocol);
        }

        // Re-read at the explicit activation boundary so a stale passive candidate cannot silently
        // bind to a different cluster or identity.
        let kubeconfig = Kubeconfig::read().map_err(|_| connection_unavailable())?;
        let fresh = binding_for_context(&kubeconfig, &candidate.context_name)
            .ok_or_else(connection_not_found)?;
        if fresh.evidence_material != candidate.evidence_material {
            return Err(ConnectionError::new(
                ConnectionErrorCode::StaleAuthority,
                "kubeconfig context changed after it was detected",
                false,
            ));
        }
        if context_uses_credential_plugin(&kubeconfig, &candidate.context_name)
            && !self.policy.allow_exec_auth
        {
            return Err(ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "the selected context uses a credential plugin; allow_exec_auth is required",
                false,
            ));
        }

        let options = KubeConfigOptions {
            context: Some(candidate.context_name.clone()),
            ..KubeConfigOptions::default()
        };
        let mut config = Config::from_custom_kubeconfig(kubeconfig, &options)
            .await
            .map_err(|_| connection_unavailable())?;
        // The kubeconfig-selected API server is the only admitted destination in this slice.
        // Ambient or kubeconfig proxy routing requires its own reviewed route contract.
        config.proxy_url = None;
        let client = Client::try_from(config).map_err(|_| connection_unavailable())?;
        verify_identity(client.clone()).await?;
        let services = discover_services(client, &self.policy).await?;

        let connection_ref = opaque_ref(
            "connection:kubernetes:",
            &format!(
                "{}\0{}",
                candidate.summary.candidate_ref, candidate.evidence_material
            ),
        );
        let description = ConnectionDescription {
            summary: ConnectionSummary {
                connection_ref: connection_ref.clone(),
                integration_ref: KUBERNETES.to_owned(),
                label: request.label,
                state: ConnectionState::Authorized,
                initiation: initiation(self.policy.initiation),
                route: ConnectionRoute::Direct,
            },
            channels: Vec::new(),
        };
        let observations = normalize_services(&connection_ref, services);
        let mut state = lock(&self.state);
        state
            .candidate_connections
            .insert(request.candidate_ref, connection_ref.clone());
        state
            .observations
            .insert(connection_ref.clone(), observations);
        state
            .connections
            .insert(connection_ref, description.clone());
        Ok(description)
    }

    fn observations(
        &self,
        request: &ObservationSearchRequest,
    ) -> Option<Vec<DiscoveryObservationSummary>> {
        let query = request.query.to_ascii_lowercase();
        lock(&self.state)
            .observations
            .get(&request.source_connection_ref)
            .map(|observations| {
                observations
                    .iter()
                    .filter(|observation| observation.title.to_ascii_lowercase().contains(&query))
                    .take(usize::from(request.limit))
                    .cloned()
                    .collect()
            })
    }

    fn has_observation(&self, observation_ref: &str) -> bool {
        lock(&self.state)
            .observations
            .values()
            .flatten()
            .any(|observation| observation.observation_ref == observation_ref)
    }
}

#[async_trait]
impl OperationBackend for KubernetesLocalBackend {
    async fn handle(
        &self,
        context: &OwnerContext,
        request: OperationRequest,
    ) -> Result<OperationResult, OperationError> {
        self.operation.handle(context, request).await
    }

    async fn handle_connection(
        &self,
        context: &OwnerContext,
        request: ConnectionRequest,
    ) -> Result<ConnectionResult, ConnectionError> {
        self.check_context(context)?;
        match request {
            ConnectionRequest::CandidateSearch(request)
                if request.integration_ref == KUBERNETES =>
            {
                Ok(ConnectionResult::CandidateSearch {
                    candidates: self.search_candidates(&request),
                })
            }
            ConnectionRequest::CandidateActivate(request)
                if self.candidates.contains_key(&request.candidate_ref) =>
            {
                self.activate(request)
                    .await
                    .map(ConnectionResult::CandidateActivate)
            }
            ConnectionRequest::Search(request) => {
                let mut connections = match self
                    .operation
                    .handle_connection(context, ConnectionRequest::Search(request.clone()))
                    .await
                {
                    Ok(ConnectionResult::Search { connections }) => connections,
                    Err(error)
                        if matches!(
                            error.code,
                            ConnectionErrorCode::NotFound | ConnectionErrorCode::Unavailable
                        ) =>
                    {
                        Vec::new()
                    }
                    Ok(_) => return Err(connection_protocol()),
                    Err(error) => return Err(error),
                };
                connections.extend(self.search_connections(&request.query));
                connections.sort_by(|left, right| left.connection_ref.cmp(&right.connection_ref));
                connections.dedup_by(|left, right| left.connection_ref == right.connection_ref);
                connections.truncate(usize::from(request.limit));
                Ok(ConnectionResult::Search { connections })
            }
            ConnectionRequest::Describe(request) => {
                let description = {
                    lock(&self.state)
                        .connections
                        .get(&request.connection_ref)
                        .cloned()
                };
                if let Some(description) = description {
                    Ok(ConnectionResult::Describe(description))
                } else {
                    self.operation
                        .handle_connection(context, ConnectionRequest::Describe(request))
                        .await
                }
            }
            ConnectionRequest::ObservationSearch(request) => {
                if let Some(observations) = self.observations(&request) {
                    Ok(ConnectionResult::ObservationSearch { observations })
                } else {
                    self.operation
                        .handle_connection(context, ConnectionRequest::ObservationSearch(request))
                        .await
                }
            }
            ConnectionRequest::Materialize(request)
                if self.has_observation(&request.observation_ref) =>
            {
                Err(ConnectionError::new(
                    ConnectionErrorCode::Unavailable,
                    "the Kubernetes Service route adapter is not installed in this release",
                    false,
                ))
            }
            other => self.operation.handle_connection(context, other).await,
        }
    }

    async fn handle_event(
        &self,
        context: &OwnerContext,
        request: EventRequest,
    ) -> Result<EventResult, EventError> {
        self.operation.handle_event(context, request).await
    }

    async fn shutdown(&self) {
        self.operation.shutdown().await;
    }
}

fn candidates(kubeconfig: &Kubeconfig) -> BTreeMap<String, CandidateBinding> {
    let mut names = kubeconfig
        .contexts
        .iter()
        .map(|context| context.name.as_str())
        .collect::<Vec<_>>();
    names.sort_unstable();
    names
        .into_iter()
        .filter_map(|name| binding_for_context(kubeconfig, name))
        .map(|binding| (binding.summary.candidate_ref.clone(), binding))
        .collect()
}

fn binding_for_context(kubeconfig: &Kubeconfig, context_name: &str) -> Option<CandidateBinding> {
    if context_name.trim().is_empty()
        || context_name.len() > 256
        || context_name.chars().any(char::is_control)
    {
        return None;
    }
    let context = kubeconfig
        .contexts
        .iter()
        .find(|candidate| candidate.name == context_name)?
        .context
        .as_ref()?;
    let cluster = kubeconfig
        .clusters
        .iter()
        .find(|candidate| candidate.name == context.cluster)?
        .cluster
        .as_ref()?;
    let server = cluster.server.as_deref()?;
    let origin = url::Url::parse(server).ok()?;
    if origin.scheme() != "https"
        || origin.host_str().is_none()
        || !origin.username().is_empty()
        || origin.password().is_some()
        || !(origin.path().is_empty() || origin.path() == "/")
        || origin.query().is_some()
        || origin.fragment().is_some()
        || cluster.insecure_skip_tls_verify == Some(true)
    {
        return None;
    }
    let user = context.user.as_deref().unwrap_or("");
    let namespace = context.namespace.as_deref().unwrap_or("default");
    let auth = kubeconfig
        .auth_infos
        .iter()
        .find(|candidate| candidate.name == user)
        .and_then(|candidate| candidate.auth_info.as_ref());
    let auth_material = auth.map_or_else(String::new, |auth| {
        let mut groups = auth.impersonate_groups.clone().unwrap_or_default();
        groups.sort();
        format!(
            "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
            auth.username.as_deref().unwrap_or(""),
            auth.token_file.as_deref().unwrap_or(""),
            auth.client_certificate.as_deref().unwrap_or(""),
            auth.client_key.as_deref().unwrap_or(""),
            auth.token.is_some(),
            auth.password.is_some(),
            auth.client_key_data.is_some(),
            auth.impersonate.as_deref().unwrap_or(""),
            auth.impersonate_uid.as_deref().unwrap_or(""),
            groups.join(","),
            auth.auth_provider
                .as_ref()
                .map_or("", |provider| provider.name.as_str()),
            auth.exec
                .as_ref()
                .and_then(|exec| exec.command.as_deref())
                .unwrap_or(""),
            auth.exec.is_some(),
        )
    });
    let evidence_material = digest(&format!(
        "{context_name}\0{}\0{user}\0{namespace}\0{server}\0{}\0{}\0{}\0{}\0{auth_material}",
        context.cluster,
        cluster.certificate_authority.as_deref().unwrap_or(""),
        cluster
            .certificate_authority_data
            .as_deref()
            .map_or_else(String::new, digest),
        cluster.tls_server_name.as_deref().unwrap_or(""),
        cluster.disable_compression.unwrap_or(false),
    ));
    let evidence_sha256 = evidence_material.clone();
    Some(CandidateBinding {
        summary: ConnectionCandidateSummary {
            candidate_ref: opaque_ref("candidate:kubernetes:", &evidence_material),
            integration_ref: KUBERNETES.to_owned(),
            title: context_name.to_owned(),
            state: ConnectionCandidateState::Detected,
            evidence_sha256,
            connection_ref: None,
        },
        context_name: context_name.to_owned(),
        evidence_material,
    })
}

fn context_uses_credential_plugin(kubeconfig: &Kubeconfig, context_name: &str) -> bool {
    let Some(context) = kubeconfig
        .contexts
        .iter()
        .find(|candidate| candidate.name == context_name)
        .and_then(|candidate| candidate.context.as_ref())
    else {
        return false;
    };
    context.user.as_ref().is_some_and(|user| {
        kubeconfig
            .auth_infos
            .iter()
            .find(|candidate| &candidate.name == user)
            .and_then(|candidate| candidate.auth_info.as_ref())
            .is_some_and(|auth| auth.exec.is_some() || auth.auth_provider.is_some())
    })
}

async fn verify_identity(client: Client) -> Result<(), ConnectionError> {
    let reviews: Api<SelfSubjectReview> = Api::all(client);
    let reviewed = reviews
        .create(&PostParams::default(), &SelfSubjectReview::default())
        .await
        .map_err(|_| connection_unavailable())?;
    let username = reviewed
        .status
        .and_then(|status| status.user_info)
        .and_then(|identity| identity.username);
    if username.as_deref().is_none_or(str::is_empty) {
        return Err(connection_unavailable());
    }
    Ok(())
}

async fn discover_services(
    client: Client,
    policy: &KubernetesIntegrationConfig,
) -> Result<Vec<Service>, ConnectionError> {
    let limit = u32::from(policy.resource_limit);
    if policy.namespaces.is_empty() {
        if !can_list_services(client.clone(), None).await? {
            return Err(ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "the selected Kubernetes identity cannot list Services cluster-wide",
                false,
            ));
        }
        let services: Api<Service> = Api::all(client);
        let mut items = services
            .list(&ListParams::default().limit(limit))
            .await
            .map(|list| list.items)
            .map_err(|_| connection_unavailable())?;
        items.truncate(usize::from(policy.resource_limit));
        return Ok(items);
    }

    let mut remaining = usize::from(policy.resource_limit);
    let mut discovered = Vec::new();
    for namespace in &policy.namespaces {
        if remaining == 0 {
            break;
        }
        if !can_list_services(client.clone(), Some(namespace)).await? {
            continue;
        }
        let services: Api<Service> = Api::namespaced(client.clone(), namespace);
        let mut items = services
            .list(&ListParams::default().limit(remaining as u32))
            .await
            .map_err(|_| connection_unavailable())?
            .items;
        items.truncate(remaining);
        remaining -= items.len();
        discovered.extend(items);
    }
    Ok(discovered)
}

async fn can_list_services(
    client: Client,
    namespace: Option<&str>,
) -> Result<bool, ConnectionError> {
    let reviews: Api<SelfSubjectAccessReview> = Api::all(client);
    let review = SelfSubjectAccessReview {
        spec: SelfSubjectAccessReviewSpec {
            resource_attributes: Some(ResourceAttributes {
                group: Some(String::new()),
                namespace: namespace.map(str::to_owned),
                resource: Some("services".to_owned()),
                verb: Some("list".to_owned()),
                version: Some("v1".to_owned()),
                ..ResourceAttributes::default()
            }),
            ..SelfSubjectAccessReviewSpec::default()
        },
        ..SelfSubjectAccessReview::default()
    };
    let reviewed = reviews
        .create(&PostParams::default(), &review)
        .await
        .map_err(|_| connection_unavailable())?;
    Ok(reviewed.status.is_some_and(|status| status.allowed))
}

fn normalize_services(
    source_connection_ref: &str,
    services: Vec<Service>,
) -> Vec<DiscoveryObservationSummary> {
    let mut seen = BTreeSet::new();
    let mut observations = services
        .into_iter()
        .filter_map(|service| {
            let provider = recognize_service(&service)?;
            let namespace = service.metadata.namespace.as_deref()?;
            let name = service.metadata.name.as_deref()?;
            let uid = service.metadata.uid.as_deref().unwrap_or("");
            let binding =
                format!("{source_connection_ref}\0{namespace}\0{name}\0{uid}\0{provider}");
            if !seen.insert(binding.clone()) {
                return None;
            }
            let title = format!("{namespace}/{name} ({provider})");
            Some(DiscoveryObservationSummary {
                observation_ref: opaque_ref("observation:kubernetes:", &binding),
                discovery_ref: DISCOVERY_REF.to_owned(),
                source_connection_ref: source_connection_ref.to_owned(),
                observed_type: "kubernetes_service".to_owned(),
                title,
                state: DiscoveryObservationState::Observed,
                evidence_generation: 1,
                evidence_sha256: digest(&binding),
                target_provider_ref: Some(provider.to_owned()),
                connection_ref: None,
            })
        })
        .collect::<Vec<_>>();
    observations.sort_by(|left, right| left.title.cmp(&right.title));
    observations
}

fn recognize_service(service: &Service) -> Option<&'static str> {
    let name = service
        .metadata
        .name
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    let identity_labels = service
        .metadata
        .labels
        .as_ref()
        .into_iter()
        .flat_map(|labels| {
            ["app.kubernetes.io/name", "app", "k8s-app", "name"]
                .into_iter()
                .filter_map(|key| labels.get(key))
                .map(|value| value.to_ascii_lowercase())
        });
    let haystack = std::iter::once(name)
        .chain(identity_labels)
        .collect::<Vec<_>>()
        .join(" ");
    if haystack.contains("grafana") {
        Some("grafana")
    } else if haystack.contains("alertmanager") {
        Some("alertmanager")
    } else if haystack.contains("loki") {
        Some("loki")
    } else if haystack.contains("prometheus") {
        Some("prometheus")
    } else {
        None
    }
}

fn initiation(config: InitiationConfig) -> Vec<ConnectionInitiator> {
    match config {
        InitiationConfig::B10x => vec![ConnectionInitiator::B10x],
        InitiationConfig::Provider => vec![ConnectionInitiator::Provider],
        InitiationConfig::Both => vec![
            ConnectionInitiator::B10x,
            ConnectionInitiator::Provider,
        ],
    }
}

fn opaque_ref(prefix: &str, value: &str) -> String {
    format!("{prefix}{}", &digest(value)[..32])
}

fn digest(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn connection_not_found() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::NotFound,
        "connection candidate was not found",
        false,
    )
}

fn connection_unavailable() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Unavailable,
        "the selected Kubernetes context could not be verified",
        true,
    )
}

fn connection_protocol() -> ConnectionError {
    ConnectionError::new(
        ConnectionErrorCode::Protocol,
        "the Connection backend returned an invalid response",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passive_candidates_expose_only_context_label_and_opaque_evidence() {
        let kubeconfig = Kubeconfig::from_yaml(
            r#"
apiVersion: v1
kind: Config
clusters:
- name: dev
  cluster:
    server: https://10.0.0.1:6443
contexts:
- name: dev-cluster
  context:
    cluster: dev
    user: alice
users:
- name: alice
  user:
    token: secret-token
"#,
        )
        .unwrap();
        let candidates = candidates(&kubeconfig);
        let candidate = candidates.values().next().unwrap();
        let encoded = serde_json::to_string(&candidate.summary).unwrap();
        assert_eq!(candidate.summary.title, "dev-cluster");
        assert!(!encoded.contains("secret-token"));
        assert!(!encoded.contains("10.0.0.1"));
        assert!(!encoded.contains("alice"));
    }

    #[test]
    fn monitoring_service_recognition_is_curated() {
        let service = |name: &str| Service {
            metadata: kube::core::ObjectMeta {
                name: Some(name.to_owned()),
                namespace: Some("monitoring".to_owned()),
                ..Default::default()
            },
            ..Service::default()
        };
        assert_eq!(
            recognize_service(&service("infra-grafana")),
            Some("grafana")
        );
        assert_eq!(
            recognize_service(&service("kube-prometheus")),
            Some("prometheus")
        );
        assert_eq!(recognize_service(&service("postgres")), None);

        let unrelated = Service {
            metadata: kube::core::ObjectMeta {
                name: Some("database".to_owned()),
                namespace: Some("monitoring".to_owned()),
                labels: Some(BTreeMap::from([
                    ("app.kubernetes.io/name".to_owned(), "postgres".to_owned()),
                    ("managed-by".to_owned(), "grafana-operator".to_owned()),
                ])),
                ..Default::default()
            },
            ..Service::default()
        };
        assert_eq!(recognize_service(&unrelated), None);
    }

    #[test]
    fn insecure_api_server_contexts_are_not_candidates() {
        let kubeconfig = Kubeconfig::from_yaml(
            r#"
apiVersion: v1
kind: Config
clusters:
- name: dev
  cluster:
    server: http://cluster.example
contexts:
- name: dev-cluster
  context:
    cluster: dev
"#,
        )
        .unwrap();
        assert!(candidates(&kubeconfig).is_empty());
    }
}

//! Shared Kubernetes endpoint inventory, independent of local or hosted authentication.
//!
//! Discovery reads Service metadata only. Credentials and native Pod forwarding have separate
//! methods so the invocation owner can keep both behind its current operation admission.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use connector_state::StateStore;
use domain::endpoint::{
    Endpoint, EndpointBinding, EndpointCredentialReference, EndpointState, EndpointTransport,
};
use k8s_openapi::api::core::v1::{Service, ServicePort};
use kube::{api::ListParams, Api, Client};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

mod routes;
pub use routes::{EndpointRouteLease, ResolvedEndpointCredentials};
mod composition;
mod crossplane;
pub use composition::{EndpointBackendFactory, EndpointPrincipalPolicy};

const MAX_STATE_BYTES: usize = 16 * 1024 * 1024;
const PAGE_SIZE: u32 = 256;
const MAX_PAGES: usize = 4096;

pub(super) async fn api_call<T>(
    future: impl std::future::Future<Output = Result<T, kube::Error>>,
) -> Result<T, EndpointSourceError> {
    tokio::time::timeout(std::time::Duration::from_secs(15), future)
        .await
        .map_err(|_| EndpointSourceError::Unavailable)?
        .map_err(routes::source_error)
}

/// How an admitted endpoint is reached from this Connector process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointPlacement {
    /// Pod streams are carried over the source's Kubernetes API connection.
    Local,
    /// Cluster DNS is reachable from the hosted Connector's approved network.
    Hosted,
}

/// Fixed caller-safe error classes. Kubernetes response bodies never escape this boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EndpointSourceError {
    #[error("endpoint source policy refused access")]
    Denied,
    #[error("endpoint resource has changed or disappeared; refresh discovery")]
    Stale,
    #[error("endpoint binding is invalid")]
    InvalidBinding,
    #[error("endpoint requires a named credential binding")]
    MissingCredentials,
    #[error("endpoint requires an explicit reachable direct route")]
    UnavailableRoute,
    #[error("endpoint source is unavailable")]
    Unavailable,
    #[error("endpoint inventory exceeded its bound")]
    Capacity,
}

/// A scan carries its completeness so denied or failed namespaces never erase old evidence.
#[derive(Debug, Clone)]
pub struct EndpointScan {
    pub endpoints: Vec<Endpoint>,
    pub complete: bool,
    pub warnings: Vec<String>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Image {
    endpoints: BTreeMap<String, Endpoint>,
    bindings: BTreeMap<String, EndpointBinding>,
    #[serde(default)]
    warnings: Vec<String>,
}

/// One configured cluster source, with durable credential-free inventory and operator bindings.
pub struct KubernetesEndpointSource {
    client: Client,
    source_ref: String,
    namespaces: BTreeSet<String>,
    all_namespaces: bool,
    target_grants: BTreeMap<String, String>,
    placement: EndpointPlacement,
    store: Arc<dyn StateStore>,
    key: String,
    image: Mutex<Image>,
    unverified: Mutex<BTreeSet<String>>,
    refresh_lock: tokio::sync::Mutex<()>,
}

impl KubernetesEndpointSource {
    /// Attach an already authenticated cluster client. This performs no Kubernetes I/O.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client: Client,
        source_ref: String,
        namespaces: BTreeSet<String>,
        all_namespaces: bool,
        target_grants: BTreeMap<String, String>,
        store: Arc<dyn StateStore>,
        placement: EndpointPlacement,
    ) -> Result<Self, EndpointSourceError> {
        if source_ref.is_empty() || namespaces.iter().any(|name| !dns_name(name)) {
            return Err(EndpointSourceError::InvalidBinding);
        }
        let key = format!(
            "kubernetes.endpoints.{}",
            hex::encode(Sha256::digest(source_ref.as_bytes()))
        );
        let image: Image = store
            .read(&key, MAX_STATE_BYTES)
            .map_err(|_| EndpointSourceError::Unavailable)?
            .map(|bytes| {
                serde_json::from_slice(&bytes).map_err(|_| EndpointSourceError::Unavailable)
            })
            .transpose()?
            .unwrap_or_default();
        if image
            .endpoints
            .values()
            .any(|endpoint| !endpoint.validate() || endpoint.source_ref != source_ref)
            || image.bindings.values().any(|binding| !binding.validate())
        {
            return Err(EndpointSourceError::Unavailable);
        }
        let unverified = image.endpoints.keys().cloned().collect();
        Ok(Self {
            client,
            source_ref,
            namespaces,
            all_namespaces,
            target_grants,
            placement,
            store,
            key,
            image: Mutex::new(image),
            unverified: Mutex::new(unverified),
            refresh_lock: tokio::sync::Mutex::new(()),
        })
    }

    pub fn source_ref(&self) -> &str {
        &self.source_ref
    }

    /// The runtime-provided metadata port, shared with bounded endpoint event receipts.
    pub fn metadata_store(&self) -> Arc<dyn StateStore> {
        self.store.clone()
    }

    pub fn admits_namespace(&self, namespace: &str) -> bool {
        dns_name(namespace) && (self.all_namespaces || self.namespaces.contains(namespace))
    }

    /// Provider policy is separate from visibility in the Service inventory.
    pub fn grant_for(&self, provider: &str) -> Option<&str> {
        self.target_grants
            .get(provider)
            .filter(|grant| !grant.is_empty())
            .map(String::as_str)
    }

    /// Current durable inventory, without contacting the cluster or resolving a Secret.
    pub fn list(&self) -> Result<Vec<Endpoint>, EndpointSourceError> {
        let image = self
            .image
            .lock()
            .map_err(|_| EndpointSourceError::Unavailable)?;
        let unverified = self
            .unverified
            .lock()
            .map_err(|_| EndpointSourceError::Unavailable)?;
        Ok(image
            .endpoints
            .values()
            .cloned()
            .map(|mut endpoint| {
                self.apply_policy(&mut endpoint);
                if unverified.contains(&endpoint.endpoint_ref) {
                    endpoint.state = EndpointState::Stale;
                }
                endpoint
            })
            .collect())
    }

    /// Last refresh diagnostics contain no provider response body or Secret values.
    pub fn warnings(&self) -> Result<Vec<String>, EndpointSourceError> {
        let mut warnings = self
            .image
            .lock()
            .map_err(|_| EndpointSourceError::Unavailable)?
            .warnings
            .clone();
        if !self
            .unverified
            .lock()
            .map_err(|_| EndpointSourceError::Unavailable)?
            .is_empty()
        {
            warnings.push(
                "Persisted endpoints await validation against the current cluster".to_owned(),
            );
        }
        warnings.truncate(100);
        Ok(warnings)
    }

    /// Credential-free authority projection used by the runtime to expire description leases.
    pub fn binding_digest(&self, provider: &str) -> Result<String, EndpointSourceError> {
        let image = self
            .image
            .lock()
            .map_err(|_| EndpointSourceError::Unavailable)?;
        let endpoints = image
            .endpoints
            .values()
            .filter(|endpoint| endpoint.provider.as_deref() == Some(provider))
            .collect::<Vec<_>>();
        let bytes = serde_json::to_vec(&(
            endpoints,
            &self.namespaces,
            self.all_namespaces,
            self.grant_for(provider),
        ))
        .map_err(|_| EndpointSourceError::Unavailable)?;
        Ok(hex::encode(Sha256::digest(bytes)))
    }

    pub(super) fn validation_candidate(
        &self,
        reference: &str,
    ) -> Result<Endpoint, EndpointSourceError> {
        let mut endpoint = self
            .image
            .lock()
            .map_err(|_| EndpointSourceError::Unavailable)?
            .endpoints
            .get(reference)
            .cloned()
            .ok_or(EndpointSourceError::Stale)?;
        if endpoint.state == EndpointState::Stale {
            endpoint.state = EndpointState::Ready;
        }
        self.apply_policy(&mut endpoint);
        Ok(endpoint)
    }

    pub(super) fn mark_validated(&self, endpoint: &Endpoint) -> Result<(), EndpointSourceError> {
        let mut image = self
            .image
            .lock()
            .map_err(|_| EndpointSourceError::Unavailable)?;
        let current = image
            .endpoints
            .get_mut(&endpoint.endpoint_ref)
            .ok_or(EndpointSourceError::Stale)?;
        if current.binding != endpoint.binding {
            return Err(EndpointSourceError::Stale);
        }
        current.state = endpoint.state;
        self.unverified
            .lock()
            .map_err(|_| EndpointSourceError::Unavailable)?
            .remove(&endpoint.endpoint_ref);
        Ok(())
    }

    pub fn show(&self, reference: &str) -> Result<Endpoint, EndpointSourceError> {
        self.list()?
            .into_iter()
            .find(|endpoint| endpoint.endpoint_ref == reference)
            .ok_or(EndpointSourceError::Stale)
    }

    /// Persist one explicit interpretation; cluster metadata never writes this map.
    pub fn bind(
        &self,
        reference: &str,
        binding: EndpointBinding,
    ) -> Result<Endpoint, EndpointSourceError> {
        self.validate_binding(&binding)?;
        let mut image = self
            .image
            .lock()
            .map_err(|_| EndpointSourceError::Unavailable)?;
        let mut next = image.clone();
        let endpoint = next
            .endpoints
            .get_mut(reference)
            .ok_or(EndpointSourceError::Stale)?;
        endpoint.provider = Some(binding.provider.clone());
        endpoint.binding = Some(binding.clone());
        endpoint.state = EndpointState::Ready;
        self.apply_policy(endpoint);
        let endpoint = endpoint.clone();
        next.bindings.insert(reference.to_owned(), binding);
        self.persist(&next)?;
        *image = next;
        Ok(endpoint)
    }

    /// Walk every Service page in each admitted namespace. Missing pages remain explicitly partial.
    pub async fn refresh(&self) -> Result<EndpointScan, EndpointSourceError> {
        let _refresh = self.refresh_lock.lock().await;
        let scopes = if self.all_namespaces {
            vec![None]
        } else {
            self.namespaces
                .iter()
                .map(|namespace| Some(namespace.as_str()))
                .collect()
        };
        let mut scan = EndpointScan {
            endpoints: Vec::new(),
            complete: true,
            warnings: Vec::new(),
        };
        for namespace in scopes {
            let api: Api<Service> = match namespace {
                Some(namespace) => Api::namespaced(self.client.clone(), namespace),
                None => Api::all(self.client.clone()),
            };
            let mut cursor = None;
            let mut seen = BTreeSet::new();
            for page in 0..MAX_PAGES {
                let mut params = ListParams::default().limit(PAGE_SIZE);
                if let Some(cursor) = cursor.as_deref() {
                    params = params.continue_token(cursor);
                }
                match api_call(api.list(&params)).await {
                    Ok(list) => {
                        if list.items.len() > PAGE_SIZE as usize
                            || list.items.iter().any(|service| {
                                service
                                    .metadata
                                    .namespace
                                    .as_deref()
                                    .is_none_or(|value| !self.admits_namespace(value))
                                    || namespace.is_some_and(|value| {
                                        service.metadata.namespace.as_deref() != Some(value)
                                    })
                            })
                        {
                            return Err(EndpointSourceError::Unavailable);
                        }
                        for service in list.items {
                            scan.endpoints
                                .extend(project_service(&self.source_ref, &service));
                        }
                        cursor = list.metadata.continue_.filter(|value| !value.is_empty());
                        match &cursor {
                            None => break,
                            Some(value) if !seen.insert(value.clone()) || page + 1 == MAX_PAGES => {
                                scan.complete = false;
                                scan.warnings
                                    .push("Service pagination did not complete".to_owned());
                                break;
                            }
                            _ => {}
                        }
                    }
                    Err(error) => {
                        scan.complete = false;
                        scan.warnings.push(format!(
                            "Service discovery {} for namespace {}",
                            if error == EndpointSourceError::Denied {
                                "denied"
                            } else {
                                "unavailable"
                            },
                            namespace.unwrap_or("*")
                        ));
                        break;
                    }
                }
            }
        }
        self.discover_crossplane(&mut scan).await;
        self.reconcile(scan)
    }

    fn reconcile(&self, mut scan: EndpointScan) -> Result<EndpointScan, EndpointSourceError> {
        let mut image = self
            .image
            .lock()
            .map_err(|_| EndpointSourceError::Unavailable)?;
        let mut next = image.clone();
        let mut observed = BTreeSet::new();
        for endpoint in &mut scan.endpoints {
            if let Some(binding) = next.bindings.get(&endpoint.endpoint_ref) {
                endpoint.binding = Some(binding.clone());
                endpoint.provider = Some(binding.provider.clone());
            }
            self.apply_policy(endpoint);
            observed.insert(endpoint.endpoint_ref.clone());
            next.endpoints
                .insert(endpoint.endpoint_ref.clone(), endpoint.clone());
        }
        if scan.complete {
            for (reference, endpoint) in &mut next.endpoints {
                if !observed.contains(reference) {
                    endpoint.state = EndpointState::Stale;
                }
            }
        }
        next.warnings = scan.warnings.iter().take(100).cloned().collect();
        self.persist(&next)?;
        self.unverified
            .lock()
            .map_err(|_| EndpointSourceError::Unavailable)?
            .retain(|reference| !observed.contains(reference));
        *image = next;
        Ok(scan)
    }

    fn persist(&self, image: &Image) -> Result<(), EndpointSourceError> {
        let bytes = serde_json::to_vec(image).map_err(|_| EndpointSourceError::Unavailable)?;
        self.store
            .replace(&self.key, &bytes, MAX_STATE_BYTES)
            .map_err(|error| match error {
                connector_state::StateError::Capacity => EndpointSourceError::Capacity,
                _ => EndpointSourceError::Unavailable,
            })
    }

    fn validate_binding(&self, binding: &EndpointBinding) -> Result<(), EndpointSourceError> {
        if !binding.validate()
            || catalog::provider(catalog::ProviderKey::id(&binding.provider)).is_none()
        {
            return Err(EndpointSourceError::InvalidBinding);
        }
        if let Some(EndpointCredentialReference::KubernetesSecret {
            namespace,
            name,
            keys,
        }) = &binding.credential
        {
            if !self.admits_namespace(namespace)
                || !dns_name(name)
                || keys.values().any(|key| {
                    key.is_empty()
                        || key.len() > 253
                        || !key
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
                })
            {
                return Err(EndpointSourceError::Denied);
            }
        }
        if let Some(address) = &binding.direct_address {
            routes::validate_address(address)?;
        }
        Ok(())
    }

    fn apply_policy(&self, endpoint: &mut Endpoint) {
        if endpoint.state == EndpointState::Stale {
            return;
        }
        if endpoint
            .namespace
            .as_deref()
            .is_some_and(|namespace| !self.admits_namespace(namespace))
        {
            endpoint.state = EndpointState::Denied;
            return;
        }
        if endpoint.transport != EndpointTransport::Tcp || endpoint.interface == "ami" {
            endpoint.state = EndpointState::UnsupportedProtocol;
            return;
        }
        let Some(provider) = endpoint
            .provider
            .as_deref()
            .and_then(|id| catalog::provider(catalog::ProviderKey::id(id)))
        else {
            endpoint.state = EndpointState::UnknownProvider;
            return;
        };
        if self.grant_for(provider.id).is_none() {
            endpoint.state = EndpointState::Denied;
            return;
        }
        if endpoint
            .binding
            .as_ref()
            .is_some_and(|binding| self.validate_binding(binding).is_err())
        {
            endpoint.state = EndpointState::Denied;
            return;
        }
        if (!provider.auth.is_empty() || matches!(provider.id, "mysql" | "postgresql"))
            && endpoint
                .binding
                .as_ref()
                .and_then(|binding| binding.credential.as_ref())
                .is_none()
        {
            endpoint.state = EndpointState::MissingCredentials;
            return;
        }
        if endpoint.state != EndpointState::UnavailableRoute {
            endpoint.state = EndpointState::Ready;
        }
    }
}

#[derive(Deserialize)]
struct Profiles {
    profile: Vec<Profile>,
}

#[derive(Deserialize)]
struct Profile {
    provider: Option<String>,
    interface: String,
    apps: Vec<String>,
    ports: Vec<u16>,
    port_names: Vec<String>,
    base_path: Option<String>,
}

fn profiles() -> &'static [Profile] {
    static PROFILES: std::sync::OnceLock<Vec<Profile>> = std::sync::OnceLock::new();
    PROFILES.get_or_init(|| {
        toml::from_str::<Profiles>(include_str!("../profiles.toml"))
            .expect("shipped endpoint profiles are valid")
            .profile
    })
}

/// Pure all-port inventory projection. Unknown and unsupported interfaces remain visible.
pub fn project_service(source_ref: &str, service: &Service) -> Vec<Endpoint> {
    let (Some(namespace), Some(name), Some(uid), Some(spec)) = (
        service.metadata.namespace.as_deref(),
        service.metadata.name.as_deref(),
        service.metadata.uid.as_deref(),
        service.spec.as_ref(),
    ) else {
        return Vec::new();
    };
    if !dns_name(namespace) || !dns_name(name) || uid.is_empty() {
        return Vec::new();
    }
    spec.ports.iter().flatten().filter_map(|port| {
        let number = u16::try_from(port.port).ok().filter(|port| *port != 0)?;
        let candidates: Vec<_> = profiles().iter().filter(|profile| profile_matches(profile, service, port)).collect();
        let profile = (candidates.len() == 1).then(|| candidates[0]);
        let transport = match port.protocol.as_deref().unwrap_or("TCP") {
            "TCP" => EndpointTransport::Tcp, "UDP" => EndpointTransport::Udp, "SCTP" => EndpointTransport::Sctp, _ => return None,
        };
        let interface = profile.map(|profile| profile.interface.clone()).unwrap_or_else(|| {
            match port.app_protocol.as_deref() { Some("http") => "http", Some("https") => "https", _ => "unknown" }.to_owned()
        });
        let binding = profile.and_then(|profile| profile.provider.as_ref().map(|provider| EndpointBinding {
            provider: provider.clone(), base_path: profile.base_path.clone(), credential: None, direct_address: None, database: None, tls: None, scheme: None,
        }));
        let identity = format!("{source_ref}\0{namespace}\0{name}\0{uid}\0{}\0{number}\0{transport:?}\0{interface}", port.name.as_deref().unwrap_or(""));
        let state = if transport != EndpointTransport::Tcp || interface == "ami" { EndpointState::UnsupportedProtocol }
            else if profile.is_none() { EndpointState::UnknownProvider }
            else if spec.selector.as_ref().is_none_or(BTreeMap::is_empty) { EndpointState::UnavailableRoute }
            else { EndpointState::Ready };
        Some(Endpoint { endpoint_ref: format!("endpoint:kubernetes:{}", hex::encode(Sha256::digest(identity.as_bytes()))),
            source_ref: source_ref.to_owned(), namespace: Some(namespace.to_owned()), resource_kind: "Service".to_owned(),
            resource_name: name.to_owned(), resource_uid: uid.to_owned(), port_name: port.name.clone(), port: Some(number),
            transport, interface, provider: profile.and_then(|profile| profile.provider.clone()), state, binding })
    }).collect()
}

fn profile_matches(profile: &Profile, service: &Service, port: &ServicePort) -> bool {
    if port.protocol.as_deref().unwrap_or("TCP") != "TCP" {
        return false;
    }
    let named = port
        .name
        .as_ref()
        .is_some_and(|name| profile.port_names.contains(name));
    let identity = service
        .metadata
        .name
        .as_ref()
        .is_some_and(|name| profile.apps.contains(name))
        || service.metadata.labels.as_ref().is_some_and(|labels| {
            ["app.kubernetes.io/name", "app", "k8s-app"]
                .iter()
                .filter_map(|key| labels.get(*key))
                .any(|value| profile.apps.contains(value))
        });
    named || identity && u16::try_from(port.port).is_ok_and(|port| profile.ports.contains(&port))
}

pub(super) fn dns_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.split('.').all(|part| {
            !part.is_empty()
                && part.len() <= 63
                && part.as_bytes()[0].is_ascii_alphanumeric()
                && part.as_bytes()[part.len() - 1].is_ascii_alphanumeric()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

#[cfg(test)]
mod tests;

//! Per-invocation credential and route resolution after admission, never during inventory.

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use domain::endpoint::{Endpoint, EndpointCredentialReference, EndpointState, EndpointTransport};
use k8s_openapi::api::core::v1::{Pod, Secret, Service, ServicePort};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::{api::ListParams, Api};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use url::Url;
use zeroize::{Zeroize, Zeroizing};

use super::{project_service, EndpointPlacement, EndpointSourceError, KubernetesEndpointSource};

/// Secret fields live only for one admitted invocation and redact themselves in Debug.
#[derive(Default)]
pub struct ResolvedEndpointCredentials(BTreeMap<String, Zeroizing<String>>);

impl std::fmt::Debug for ResolvedEndpointCredentials {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ResolvedEndpointCredentials(<redacted>)")
    }
}

impl ResolvedEndpointCredentials {
    /// Deliberate exposure at the protocol driver's credential boundary.
    pub fn expose(&self, field: &str) -> Option<&str> {
        self.0.get(field).map(|value| value.as_str())
    }
}

/// A short-lived route. Dropping it aborts both the bridge and Kubernetes forwarding tasks.
pub struct EndpointRouteLease {
    /// Logical origin, retained for Host headers and TLS hostname verification.
    pub logical_url: Url,
    /// A private, single-use bridge when the driver cannot consume the native stream directly.
    pub connect_address: Option<SocketAddr>,
    bridge: Option<JoinHandle<()>>,
    forwarder: Option<kube::api::Portforwarder>,
}

impl std::fmt::Debug for EndpointRouteLease {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EndpointRouteLease")
            .field("logical_url", &self.logical_url)
            .field("forwarded", &self.forwarder.is_some())
            .finish()
    }
}

impl Drop for EndpointRouteLease {
    fn drop(&mut self) {
        if let Some(task) = &self.bridge {
            task.abort();
        }
        if let Some(forwarder) = &self.forwarder {
            forwarder.abort();
        }
    }
}

impl KubernetesEndpointSource {
    /// Revalidate immutable resource identity and current policy. Does not read a credential.
    pub async fn validate(&self, reference: &str) -> Result<Endpoint, EndpointSourceError> {
        let endpoint = self.show(reference)?;
        if endpoint.source_ref != self.source_ref {
            return Err(EndpointSourceError::Denied);
        }
        match endpoint.state {
            EndpointState::Denied => return Err(EndpointSourceError::Denied),
            EndpointState::Stale => return Err(EndpointSourceError::Stale),
            EndpointState::UnknownProvider | EndpointState::UnsupportedProtocol => {
                return Err(EndpointSourceError::InvalidBinding)
            }
            _ => {}
        }
        if endpoint.resource_kind != "Service" {
            return self.validate_crossplane(endpoint).await;
        }
        let namespace = endpoint
            .namespace
            .as_deref()
            .ok_or(EndpointSourceError::Stale)?;
        if !self.admits_namespace(namespace) {
            return Err(EndpointSourceError::Denied);
        }
        let api: Api<Service> = Api::namespaced(self.client.clone(), namespace);
        let current = api
            .get(&endpoint.resource_name)
            .await
            .map_err(source_error)?;
        if !project_service(&self.source_ref, &current)
            .iter()
            .any(|current| current.endpoint_ref == endpoint.endpoint_ref)
        {
            return Err(EndpointSourceError::Stale);
        }
        Ok(endpoint)
    }

    /// Read only the bound Secret, after the invocation owner has admitted the operation.
    pub async fn resolve_credentials(
        &self,
        endpoint: &Endpoint,
    ) -> Result<ResolvedEndpointCredentials, EndpointSourceError> {
        if endpoint.source_ref != self.source_ref
            || self.show(&endpoint.endpoint_ref)?.binding != endpoint.binding
        {
            return Err(EndpointSourceError::Stale);
        }
        let Some(binding) = endpoint.binding.as_ref() else {
            return Ok(ResolvedEndpointCredentials::default());
        };
        self.validate_binding(binding)?;
        let Some(EndpointCredentialReference::KubernetesSecret {
            namespace,
            name,
            keys,
        }) = &binding.credential
        else {
            return Ok(ResolvedEndpointCredentials::default());
        };
        let api: Api<Secret> = Api::namespaced(self.client.clone(), namespace);
        let mut secret = api
            .get(name)
            .await
            .map_err(|error| match source_error(error) {
                EndpointSourceError::Stale => EndpointSourceError::MissingCredentials,
                other => other,
            })?;
        if secret.metadata.name.as_deref() != Some(name)
            || secret.metadata.namespace.as_deref() != Some(namespace)
        {
            return Err(EndpointSourceError::Unavailable);
        }
        let mut values = BTreeMap::new();
        let mut data = secret.data.take().unwrap_or_default();
        let result = keys.iter().try_for_each(|(field, key)| {
            let value = data
                .get(key)
                .ok_or(EndpointSourceError::MissingCredentials)?;
            if value.0.is_empty() || value.0.len() > 64 * 1024 {
                return Err(EndpointSourceError::MissingCredentials);
            }
            let value = std::str::from_utf8(&value.0)
                .map_err(|_| EndpointSourceError::MissingCredentials)?;
            values.insert(field.clone(), Zeroizing::new(value.to_owned()));
            Ok(())
        });
        for value in data.values_mut() {
            value.0.zeroize();
        }
        if let Some(values) = &mut secret.string_data {
            for value in values.values_mut() {
                value.zeroize();
            }
        }
        result?;
        Ok(ResolvedEndpointCredentials(values))
    }

    /// Resolve an approved route. The returned guard must outlive the entire driver call.
    pub async fn open_route(
        &self,
        endpoint: &Endpoint,
        credentials: &ResolvedEndpointCredentials,
    ) -> Result<EndpointRouteLease, EndpointSourceError> {
        // Recheck even when the caller performed a prior metadata-only resolution: description
        // leases and delayed invocation must not extend a resource's lifetime.
        let current = self.validate(&endpoint.endpoint_ref).await?;
        if current.binding != endpoint.binding || current.resource_uid != endpoint.resource_uid {
            return Err(EndpointSourceError::Stale);
        }
        if endpoint.transport != EndpointTransport::Tcp {
            return Err(EndpointSourceError::UnavailableRoute);
        }
        let explicit = endpoint
            .binding
            .as_ref()
            .and_then(|binding| binding.direct_address.as_deref());
        if let Some(address) = explicit {
            let mut logical_url = validate_address(address)?;
            if let Some(path) = endpoint
                .binding
                .as_ref()
                .and_then(|binding| binding.base_path.as_deref())
            {
                logical_url.set_path(path);
            }
            return Ok(EndpointRouteLease {
                logical_url,
                connect_address: None,
                bridge: None,
                forwarder: None,
            });
        }
        if endpoint.resource_kind != "Service" {
            // A Secret's host is discovery evidence, not permission to reach arbitrary external
            // networks. The operator supplies a direct_address before this route can be opened.
            let _ = credentials;
            return Err(EndpointSourceError::UnavailableRoute);
        }
        let namespace = endpoint
            .namespace
            .as_deref()
            .ok_or(EndpointSourceError::Stale)?;
        let port = endpoint.port.ok_or(EndpointSourceError::Stale)?;
        let api: Api<Service> = Api::namespaced(self.client.clone(), namespace);
        let service = api
            .get(&endpoint.resource_name)
            .await
            .map_err(source_error)?;
        if service.metadata.uid.as_deref() != Some(&endpoint.resource_uid) {
            return Err(EndpointSourceError::Stale);
        }
        let spec = service.spec.as_ref().ok_or(EndpointSourceError::Stale)?;
        let service_port = spec
            .ports
            .iter()
            .flatten()
            .find(|candidate| {
                candidate.port == i32::from(port)
                    && candidate.name == endpoint.port_name
                    && candidate.protocol.as_deref().unwrap_or("TCP") == "TCP"
            })
            .ok_or(EndpointSourceError::Stale)?;
        if spec.selector.as_ref().is_none_or(BTreeMap::is_empty) {
            return Err(EndpointSourceError::UnavailableRoute);
        }
        let scheme = match endpoint.binding.as_ref().and_then(|binding| binding.scheme) {
            Some(domain::endpoint::EndpointScheme::Http) => "http",
            Some(domain::endpoint::EndpointScheme::Https) => "https",
            None => match endpoint.interface.as_str() {
                "https" => "https",
                "mysql" => "mysql",
                "postgresql" => "postgresql",
                _ => "http",
            },
        };
        let mut logical_url = validate_address(&format!(
            "{scheme}://{}.{}.svc:{port}",
            endpoint.resource_name, namespace
        ))?;
        if let Some(path) = endpoint
            .binding
            .as_ref()
            .and_then(|binding| binding.base_path.as_deref())
        {
            logical_url.set_path(path);
        }
        if self.placement == EndpointPlacement::Hosted {
            return Ok(EndpointRouteLease {
                logical_url,
                connect_address: None,
                bridge: None,
                forwarder: None,
            });
        }
        let labels = spec
            .selector
            .as_ref()
            .ok_or(EndpointSourceError::UnavailableRoute)?;
        let selector = labels
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(",");
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), namespace);
        let mut cursor = None;
        let mut selected = None;
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..super::MAX_PAGES {
            let mut params = ListParams::default()
                .labels(&selector)
                .limit(super::PAGE_SIZE);
            if let Some(token) = cursor.as_deref() {
                params = params.continue_token(token);
            }
            let page = pods.list(&params).await.map_err(source_error)?;
            if page.items.len() > super::PAGE_SIZE as usize {
                return Err(EndpointSourceError::Capacity);
            }
            for pod in page.items {
                if let Some(target) = pod_target_port(&pod, namespace, labels, service_port) {
                    selected = Some((pod, target));
                    break;
                }
            }
            if selected.is_some() {
                break;
            }
            cursor = page.metadata.continue_.filter(|value| !value.is_empty());
            match &cursor {
                None => break,
                Some(value) if !seen.insert(value.clone()) => {
                    return Err(EndpointSourceError::Unavailable)
                }
                _ => {}
            }
        }
        let (pod, target_port) = selected.ok_or(EndpointSourceError::UnavailableRoute)?;
        let name = pod
            .metadata
            .name
            .as_deref()
            .ok_or(EndpointSourceError::UnavailableRoute)?;
        let mut forwarder = pods
            .portforward(name, &[target_port])
            .await
            .map_err(source_error)?;
        let mut stream = match forwarder.take_stream(target_port) {
            Some(stream) => stream,
            None => {
                forwarder.abort();
                return Err(EndpointSourceError::UnavailableRoute);
            }
        };
        let listener =
            match TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)).await {
                Ok(listener) => listener,
                Err(_) => {
                    forwarder.abort();
                    return Err(EndpointSourceError::Unavailable);
                }
            };
        let address = listener
            .local_addr()
            .map_err(|_| EndpointSourceError::Unavailable)?;
        let bridge = tokio::spawn(async move {
            let accepted = tokio::time::timeout(Duration::from_secs(10), listener.accept()).await;
            // One driver connection owns this forward. No reusable local proxy survives it.
            drop(listener);
            if let Ok(Ok((mut socket, _))) = accepted {
                let _ = tokio::io::copy_bidirectional(&mut socket, &mut stream).await;
            }
        });
        Ok(EndpointRouteLease {
            logical_url,
            connect_address: Some(address),
            bridge: Some(bridge),
            forwarder: Some(forwarder),
        })
    }
}

pub(super) fn validate_address(address: &str) -> Result<Url, EndpointSourceError> {
    let url = Url::parse(address).map_err(|_| EndpointSourceError::InvalidBinding)?;
    if !matches!(url.scheme(), "http" | "https" | "mysql" | "postgresql")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(EndpointSourceError::InvalidBinding);
    }
    Ok(url)
}

pub(super) fn source_error(error: kube::Error) -> EndpointSourceError {
    match error {
        kube::Error::Api(response) if response.code == 401 || response.code == 403 => {
            EndpointSourceError::Denied
        }
        kube::Error::Api(response) if response.code == 404 => EndpointSourceError::Stale,
        _ => EndpointSourceError::Unavailable,
    }
}

fn pod_target_port(
    pod: &Pod,
    namespace: &str,
    labels: &BTreeMap<String, String>,
    service_port: &ServicePort,
) -> Option<u16> {
    if pod.metadata.namespace.as_deref() != Some(namespace)
        || pod.metadata.uid.as_deref().is_none_or(str::is_empty)
        || pod.metadata.name.as_deref().is_none_or(str::is_empty)
        || pod.metadata.deletion_timestamp.is_some()
        || !pod.metadata.labels.as_ref().is_some_and(|actual| {
            labels
                .iter()
                .all(|(key, value)| actual.get(key) == Some(value))
        })
        || !pod.status.as_ref().is_some_and(|status| {
            status.phase.as_deref() == Some("Running")
                && status
                    .conditions
                    .iter()
                    .flatten()
                    .any(|condition| condition.type_ == "Ready" && condition.status == "True")
        })
    {
        return None;
    }
    let port = match &service_port.target_port {
        Some(IntOrString::Int(port)) => *port,
        None => service_port.port,
        Some(IntOrString::String(name)) => {
            let ports: std::collections::BTreeSet<_> = pod
                .spec
                .as_ref()?
                .containers
                .iter()
                .flat_map(|container| container.ports.iter().flatten())
                .filter(|port| {
                    port.name.as_deref() == Some(name)
                        && port.protocol.as_deref().unwrap_or("TCP") == "TCP"
                })
                .map(|port| port.container_port)
                .collect();
            if ports.len() != 1 {
                return None;
            }
            *ports.iter().next()?
        }
    };
    u16::try_from(port).ok().filter(|port| *port > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pod_forward_requires_current_ready_selector_member_and_maps_named_ports() {
        let mut pod: Pod = serde_json::from_value(serde_json::json!({
            "metadata":{"name":"db-0","namespace":"data","uid":"pod-uid","labels":{"app":"db"}},
            "spec":{"containers":[{"name":"db","ports":[{"name":"sql","containerPort":15432}]}]},
            "status":{"phase":"Running","conditions":[{"type":"Ready","status":"True"}]}
        }))
        .unwrap();
        let labels = BTreeMap::from([("app".to_owned(), "db".to_owned())]);
        let port = ServicePort {
            port: 5432,
            target_port: Some(IntOrString::String("sql".to_owned())),
            ..ServicePort::default()
        };
        assert_eq!(pod_target_port(&pod, "data", &labels, &port), Some(15432));
        assert_eq!(pod_target_port(&pod, "other", &labels, &port), None);
        pod.status.as_mut().unwrap().conditions.as_mut().unwrap()[0].status = "False".to_owned();
        assert_eq!(pod_target_port(&pod, "data", &labels, &port), None);
    }

    #[test]
    fn direct_routes_reject_credentials_and_unsupported_schemes() {
        assert!(validate_address("https://service.example:8443/api").is_ok());
        for address in [
            "https://user:secret@service.example",
            "file:///tmp/socket",
            "https://service.example?token=x",
            "https://service.example#secret",
        ] {
            assert!(validate_address(address).is_err(), "{address}");
        }
    }
}

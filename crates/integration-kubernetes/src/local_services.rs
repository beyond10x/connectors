//! Reaching the Services this cluster runs, and deciding which of them this placement may open.
//!
//! Split out of `local.rs` rather than waiving its size: discovery is one arm and the cluster's own
//! workload operations are another, and the file crossed the module cap when the second arm grew.
//! Everything here takes a `kube::Client` and returns a decision — no placement state is touched,
//! which is why it moves cleanly.

use std::collections::BTreeSet;

use connectors_config::KubernetesIntegrationConfig;
use futures_util::AsyncReadExt as _;
use http::{Method, Request as HttpRequest};
use k8s_openapi::api::authentication::v1::SelfSubjectReview;
use k8s_openapi::api::authorization::v1::{
    ResourceAttributes, SelfSubjectAccessReview, SelfSubjectAccessReviewSpec,
};
use k8s_openapi::api::core::v1::{Service, ServicePort};
use kube::api::{ListParams, PostParams};
use kube::{Api, Client};
use protocol::connection::{
    ConnectionError, ConnectionErrorCode, DiscoveryObservationState, DiscoveryObservationSummary,
};
use protocol::operation::{OperationError, OperationErrorCode};
use serde_json::Value;

use super::local::*;

pub(crate) async fn verify_identity(client: Client) -> Result<(), ConnectionError> {
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

pub(crate) async fn discover_services(
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

pub(crate) async fn can_list_services(
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

pub(crate) async fn can_proxy_service(
    client: Client,
    namespace: &str,
    service: &str,
) -> Result<bool, OperationError> {
    let reviews: Api<SelfSubjectAccessReview> = Api::all(client);
    let review = SelfSubjectAccessReview {
        spec: SelfSubjectAccessReviewSpec {
            resource_attributes: Some(ResourceAttributes {
                group: Some(String::new()),
                namespace: Some(namespace.to_owned()),
                resource: Some("services".to_owned()),
                subresource: Some("proxy".to_owned()),
                name: Some(service.to_owned()),
                verb: Some("get".to_owned()),
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
        .map_err(|_| operation_unavailable())?;
    Ok(reviewed.status.is_some_and(|status| status.allowed))
}

pub(crate) async fn can_get_service(
    client: Client,
    namespace: &str,
    service: &str,
) -> Result<bool, OperationError> {
    let reviews: Api<SelfSubjectAccessReview> = Api::all(client);
    let review = SelfSubjectAccessReview {
        spec: SelfSubjectAccessReviewSpec {
            resource_attributes: Some(ResourceAttributes {
                group: Some(String::new()),
                namespace: Some(namespace.to_owned()),
                resource: Some("services".to_owned()),
                name: Some(service.to_owned()),
                verb: Some("get".to_owned()),
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
        .map_err(|_| operation_unavailable())?;
    Ok(reviewed.status.is_some_and(|status| status.allowed))
}

pub(crate) async fn service_is_current(
    client: Client,
    child: &KubernetesServiceConnection,
) -> Result<bool, OperationError> {
    let services: Api<Service> = Api::namespaced(client, &child.namespace);
    let current = services
        .get(&child.service)
        .await
        .map_err(|_| operation_unavailable())?;
    let Some(provider) = recognize_service(&current) else {
        return Ok(false);
    };
    let Some(uid) = current.metadata.uid.as_deref() else {
        return Ok(false);
    };
    let Some(port) = current
        .spec
        .as_ref()
        .and_then(|spec| spec.ports.as_deref())
        .and_then(|ports| select_service_port(provider, ports))
    else {
        return Ok(false);
    };
    let binding = format!(
        "{}\0{}\0{}\0{}\0{}\0{}",
        child.parent_connection_ref, child.namespace, child.service, port, uid, provider
    );
    Ok(provider == child.provider
        && uid == child.resource_uid
        && port == child.port
        && opaque_ref("binding:kubernetes-service:", &binding) == child.resource_binding)
}

pub(crate) async fn proxy_json(
    client: Client,
    child: &KubernetesServiceConnection,
    relative: &str,
) -> Result<Value, OperationError> {
    let route = format!(
        "/api/v1/namespaces/{}/services/{}:{}/proxy{}",
        child.namespace, child.service, child.port, relative
    );
    let request = HttpRequest::builder()
        .method(Method::GET)
        .uri(route)
        .header(http::header::ACCEPT, "application/json")
        .body(Vec::new())
        .map_err(|_| operation_invalid())?;
    let stream = client
        .request_stream(request)
        .await
        .map_err(|_| operation_unavailable())?;
    let mut bytes = Vec::new();
    stream
        .take((MAX_PROXY_RESULT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| operation_unavailable())?;
    if bytes.len() > MAX_PROXY_RESULT_BYTES {
        return Err(OperationError::new(
            OperationErrorCode::ResultTooLarge,
            "Kubernetes Service proxy result exceeded the Connector bound",
            false,
        ));
    }
    serde_json::from_slice(&bytes).map_err(|_| operation_unavailable())
}

pub(crate) fn normalize_services(
    source_connection_ref: &str,
    services: Vec<Service>,
) -> Vec<StoredServiceObservation> {
    let mut seen = BTreeSet::new();
    let mut observations = services
        .into_iter()
        .filter_map(|service| {
            let provider = recognize_service(&service)?;
            let namespace = service.metadata.namespace.as_deref()?;
            let name = service.metadata.name.as_deref()?;
            let uid = service.metadata.uid.as_deref()?;
            let port = select_service_port(provider, service.spec.as_ref()?.ports.as_deref()?)?;
            let binding =
                format!("{source_connection_ref}\0{namespace}\0{name}\0{port}\0{uid}\0{provider}");
            if !seen.insert(binding.clone()) {
                return None;
            }
            let title = format!("{namespace}/{name} ({provider})");
            Some(StoredServiceObservation {
                summary: DiscoveryObservationSummary {
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
                },
                namespace: namespace.to_owned(),
                service: name.to_owned(),
                resource_uid: uid.to_owned(),
                port,
                provider: provider.to_owned(),
                resource_binding: opaque_ref("binding:kubernetes-service:", &binding),
            })
        })
        .collect::<Vec<_>>();
    observations.sort_by(|left, right| left.summary.title.cmp(&right.summary.title));
    observations
}

pub(crate) fn select_service_port(provider: &str, ports: &[ServicePort]) -> Option<String> {
    let candidates = ports
        .iter()
        .filter(|port| {
            port.protocol
                .as_deref()
                .is_none_or(|protocol| protocol == "TCP")
        })
        .collect::<Vec<_>>();
    // Argo CD's `argocd-server` Service publishes `http` on 80 and `https` on 443, both targeting
    // the same container port 8080, where the server decides by TLS. 443 is the pin because that is
    // the listener its own clients use; nothing here dials it, so this is evidence that identifies
    // the API endpoint rather than a route (see `materialize`, which refuses this provider).
    let known_number = match provider {
        "grafana" => 3000,
        "prometheus" => 9090,
        "loki" => 3100,
        "alertmanager" => 9093,
        "argocd" => 443,
        _ => return None,
    };
    let preferred = candidates
        .iter()
        .copied()
        .find(|port| port.port == known_number)
        .or_else(|| {
            candidates.iter().copied().find(|port| {
                port.name.as_deref().is_some_and(|name| {
                    matches!(
                        name,
                        "http"
                            | "http-web"
                            | "web"
                            | "service"
                            | "grafana"
                            | "prometheus"
                            | "loki"
                            | "alertmanager"
                    )
                })
            })
        })
        .or_else(|| (candidates.len() == 1).then_some(candidates[0]))?;
    preferred
        .name
        .as_ref()
        .filter(|name| valid_dns_label(name, 63))
        .cloned()
        .or_else(|| {
            (1..=65_535)
                .contains(&preferred.port)
                .then(|| preferred.port.to_string())
        })
}

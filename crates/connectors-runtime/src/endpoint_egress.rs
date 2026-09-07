//! Composition of source-validated endpoint routes with the sole HTTP/WebSocket socket owner.

use crate::kubernetes_endpoints::{EndpointEgressFactory, KubernetesEndpointBackend};
use integration_kubernetes::endpoints::{
    EndpointBackendFactory, EndpointPrincipalPolicy, EndpointRouteLease, EndpointSourceError,
    KubernetesEndpointSource,
};
use std::sync::Arc;

pub(crate) struct KubernetesEgress;

impl EndpointEgressFactory for KubernetesEgress {
    fn transport(
        &self,
        connection_ref: &str,
        route: &EndpointRouteLease,
    ) -> Result<Arc<dyn service::EgressTransport>, EndpointSourceError> {
        server::egress::ConnectionEgress::for_endpoint_route(
            connection_ref,
            route.logical_url.as_str(),
            route.connect_address,
        )
        .map(|transport| Arc::new(transport) as Arc<dyn service::EgressTransport>)
        .map_err(|_| EndpointSourceError::UnavailableRoute)
    }
}

impl EndpointBackendFactory for KubernetesEgress {
    fn build(
        &self,
        source: Arc<KubernetesEndpointSource>,
        policy: EndpointPrincipalPolicy,
    ) -> Arc<dyn service::ConnectorBackend> {
        Arc::new(KubernetesEndpointBackend::new(source, policy, Arc::new(Self)).refresh_on_start())
    }
}

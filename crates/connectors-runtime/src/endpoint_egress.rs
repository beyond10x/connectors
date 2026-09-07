//! Composition of source-validated endpoint routes with the sole HTTP/WebSocket socket owner.

use integration_kubernetes::endpoints::{
    EndpointEgressFactory, EndpointRouteLease, EndpointSourceError,
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

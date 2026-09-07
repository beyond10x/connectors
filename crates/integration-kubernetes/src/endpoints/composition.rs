//! Runtime injection and authenticated cluster-source policy.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use domain::endpoint::{Endpoint, EndpointCredentialReference};
use service::{ConnectorBackend, PrincipalContext};

use super::KubernetesEndpointSource;

/// The runtime owns generic provider execution and composes it around this source.
pub trait EndpointBackendFactory: Send + Sync + 'static {
    fn build(
        &self,
        source: Arc<KubernetesEndpointSource>,
        policy: EndpointPrincipalPolicy,
    ) -> Arc<dyn ConnectorBackend>;
}

/// Authenticated actor policy, independently intersected with source/provider grants.
#[derive(Clone)]
pub enum EndpointPrincipalPolicy {
    Local(Arc<PrincipalContext>),
    Hosted {
        tenant: String,
        namespace_groups: BTreeMap<String, BTreeSet<String>>,
        operator_groups: BTreeSet<String>,
    },
}

impl EndpointPrincipalPolicy {
    pub fn owns(&self, context: &PrincipalContext) -> bool {
        match self {
            Self::Local(owner) => owner.as_ref() == context,
            Self::Hosted { tenant, .. } => tenant == context.tenant_id(),
        }
    }

    pub fn manages(&self, context: &PrincipalContext) -> bool {
        self.owns(context)
            && match self {
                Self::Local(_) => true,
                Self::Hosted {
                    operator_groups, ..
                } => !operator_groups.is_disjoint(context.verified_groups()),
            }
    }

    pub fn reads(&self, context: &PrincipalContext, endpoint: &Endpoint) -> bool {
        if !self.owns(context) {
            return false;
        }
        match self {
            Self::Local(_) => true,
            Self::Hosted {
                namespace_groups, ..
            } => {
                let mut namespaces = Vec::new();
                if let Some(namespace) = endpoint.namespace.as_deref() {
                    namespaces.push(namespace);
                }
                if let Some(EndpointCredentialReference::KubernetesSecret { namespace, .. }) =
                    endpoint
                        .binding
                        .as_ref()
                        .and_then(|binding| binding.credential.as_ref())
                {
                    namespaces.push(namespace);
                }
                !namespaces.is_empty()
                    && namespaces.into_iter().all(|namespace| {
                        namespace_groups.get(namespace).is_some_and(|groups| {
                            self.manages(context) || !groups.is_disjoint(context.verified_groups())
                        })
                    })
            }
        }
    }
}

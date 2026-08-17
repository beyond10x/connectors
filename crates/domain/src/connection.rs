use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Closed implementation identity for a route through another Connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteAdapter {
    /// Grafana's reviewed data-source proxy prefix, with the data-source identity resolved from a
    /// Connector-owned opaque binding rather than caller input.
    GrafanaDatasourceProxyV1,
    /// Kubernetes API Service proxy for one Connector-owned namespace, Service, and port
    /// binding. Invocation callers can supply only catalog parameters; they cannot supply any
    /// Kubernetes resource identity or proxy path.
    KubernetesServiceProxyV1,
}

impl RouteAdapter {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GrafanaDatasourceProxyV1 => "grafana_datasource_proxy_v1",
            Self::KubernetesServiceProxyV1 => "kubernetes_service_proxy_v1",
        }
    }
}

/// A Connection's immutable execution route.
///
/// A mediated route remains a Connection of the target Provider. The parent supplies transport,
/// not Provider semantics or Grant authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionRoute {
    /// Resolve the target Provider's operator-approved origin directly.
    Direct,
    /// Execute through another authorized Connection and one opaque discovered-resource binding.
    ViaConnection {
        parent_connection: String,
        resource_binding: String,
        adapter: RouteAdapter,
    },
}

/// Which side of a configured Connection may begin an interaction.
///
/// The names are relative to the Connectors boundary. They deliberately avoid `inbound` and
/// `outbound`, whose meaning changes with the observer and which would collide with an Operation's
/// existing read/write direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionInitiator {
    /// A B10x principal may ask Connectors to start a declared Operation at the provider.
    B10x,
    /// The provider may start a declared Channel or session toward B10x.
    Provider,
}

/// The non-empty set of sides that may initiate through one Connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitiationPolicy {
    allowed: BTreeSet<ConnectionInitiator>,
}

impl InitiationPolicy {
    /// Construct a policy. A Connection with no permitted initiator is invalid rather than a
    /// second spelling of an inactive lifecycle state.
    pub fn new(
        allowed: impl IntoIterator<Item = ConnectionInitiator>,
    ) -> Result<Self, ConnectionAuthorityError> {
        let allowed = allowed.into_iter().collect::<BTreeSet<_>>();
        if allowed.is_empty() {
            return Err(ConnectionAuthorityError::NoAllowedInitiator);
        }
        Ok(Self { allowed })
    }

    #[must_use]
    pub fn b10x_only() -> Self {
        Self {
            allowed: BTreeSet::from([ConnectionInitiator::B10x]),
        }
    }

    #[must_use]
    pub fn provider_only() -> Self {
        Self {
            allowed: BTreeSet::from([ConnectionInitiator::Provider]),
        }
    }

    #[must_use]
    pub fn bidirectional() -> Self {
        Self {
            allowed: BTreeSet::from([
                ConnectionInitiator::B10x,
                ConnectionInitiator::Provider,
            ]),
        }
    }

    #[must_use]
    pub fn allows(&self, initiator: ConnectionInitiator) -> bool {
        self.allowed.contains(&initiator)
    }

    pub fn iter(&self) -> impl Iterator<Item = ConnectionInitiator> + '_ {
        self.allowed.iter().copied()
    }
}

/// Connection identity plus its independently configured initiation boundary.
///
/// Grants remain separate: this value says which side may start, not which principal may execute
/// which operation or receive which channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionAuthority {
    id: String,
    initiation: InitiationPolicy,
    route: ConnectionRoute,
}

impl ConnectionAuthority {
    pub fn new(
        id: impl Into<String>,
        initiation: InitiationPolicy,
    ) -> Result<Self, ConnectionAuthorityError> {
        let id = id.into();
        if id.is_empty() {
            return Err(ConnectionAuthorityError::EmptyConnection);
        }
        Ok(Self {
            id,
            initiation,
            route: ConnectionRoute::Direct,
        })
    }

    /// Construct a mediated Connection. The resource binding is opaque Connector-owned state; it
    /// is not a provider UID or URL accepted from an invocation caller.
    pub fn mediated(
        id: impl Into<String>,
        initiation: InitiationPolicy,
        parent_connection: impl Into<String>,
        resource_binding: impl Into<String>,
        adapter: RouteAdapter,
    ) -> Result<Self, ConnectionAuthorityError> {
        let id = id.into();
        let parent_connection = parent_connection.into();
        let resource_binding = resource_binding.into();
        validate_ref(&id)
            .then_some(())
            .ok_or(ConnectionAuthorityError::EmptyConnection)?;
        if !validate_ref(&parent_connection) {
            return Err(ConnectionAuthorityError::InvalidParentConnection);
        }
        if id == parent_connection {
            return Err(ConnectionAuthorityError::RouteCycle);
        }
        if !validate_ref(&resource_binding) {
            return Err(ConnectionAuthorityError::InvalidResourceBinding);
        }
        Ok(Self {
            id,
            initiation,
            route: ConnectionRoute::ViaConnection {
                parent_connection,
                resource_binding,
                adapter,
            },
        })
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn initiation(&self) -> &InitiationPolicy {
        &self.initiation
    }

    #[must_use]
    pub fn route(&self) -> &ConnectionRoute {
        &self.route
    }
}

fn validate_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .chars()
            .all(|character| !character.is_whitespace() && !character.is_control())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConnectionAuthorityError {
    #[error("Connection identity is empty")]
    EmptyConnection,
    #[error("Connection initiation policy allows no initiator")]
    NoAllowedInitiator,
    #[error("mediated route parent Connection identity is invalid")]
    InvalidParentConnection,
    #[error("mediated route resource binding is invalid")]
    InvalidResourceBinding,
    #[error("a Connection cannot route through itself")]
    RouteCycle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_policies_are_explicit_sets() {
        let b10x = InitiationPolicy::b10x_only();
        assert!(b10x.allows(ConnectionInitiator::B10x));
        assert!(!b10x.allows(ConnectionInitiator::Provider));

        let provider = InitiationPolicy::provider_only();
        assert!(!provider.allows(ConnectionInitiator::B10x));
        assert!(provider.allows(ConnectionInitiator::Provider));

        let both = InitiationPolicy::bidirectional();
        assert!(both.allows(ConnectionInitiator::B10x));
        assert!(both.allows(ConnectionInitiator::Provider));
    }

    #[test]
    fn inactive_is_a_lifecycle_state_not_an_empty_policy() {
        assert_eq!(
            InitiationPolicy::new([]),
            Err(ConnectionAuthorityError::NoAllowedInitiator)
        );
    }

    #[test]
    fn mediated_route_is_explicit_and_cannot_self_reference() {
        let connection = ConnectionAuthority::mediated(
            "prometheus-via-grafana",
            InitiationPolicy::b10x_only(),
            "grafana-infra",
            "observation:datasource-1",
            RouteAdapter::GrafanaDatasourceProxyV1,
        )
        .expect("valid mediated route");
        assert!(matches!(
            connection.route(),
            ConnectionRoute::ViaConnection { parent_connection, adapter: RouteAdapter::GrafanaDatasourceProxyV1, .. }
                if parent_connection == "grafana-infra"
        ));

        assert_eq!(
            ConnectionAuthority::mediated(
                "same",
                InitiationPolicy::b10x_only(),
                "same",
                "observation:1",
                RouteAdapter::GrafanaDatasourceProxyV1,
            ),
            Err(ConnectionAuthorityError::RouteCycle)
        );
    }
}

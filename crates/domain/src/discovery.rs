use crate::{ConnectionRoute, RouteAdapter};

/// A bounded fact emitted by one catalog-declared discovery normalizer.
///
/// This is deliberately not a Connection and carries no Grant. The provider-specific resource
/// identity remains an opaque binding used only if the observation becomes a candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryObservation {
    id: String,
    declaration: String,
    source_connection: String,
    observed_type: String,
    title: String,
    evidence_generation: u64,
    evidence_sha256: String,
    resource_binding: String,
    target: Option<DiscoveryTarget>,
}

/// A recognized target Provider contract and its reviewed mediated-route adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DiscoveryTarget {
    provider: String,
    adapter: RouteAdapter,
}

/// A normalized possible Provider instance. It is still unusable until control-plane admission
/// materializes a durable Connection and an independent Connector Grant admits an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionCandidate {
    pub source: ConnectionCandidateSource,
    pub target_provider: String,
    pub title: String,
    pub evidence_generation: u64,
    pub evidence_sha256: String,
    pub route: ConnectionRoute,
}

/// Closed origin of one unusable Connection candidate. Local configuration precedes a direct
/// source Connection; an observation follows an existing source Connection and proposes mediation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionCandidateSource {
    LocalConfiguration { candidate_binding: String },
    Observation { observation: String },
}

impl ConnectionCandidate {
    /// Construct a candidate detected in trusted local configuration. The binding remains private
    /// Connector state and activation still has to verify provider identity and authority.
    pub fn direct(
        candidate_binding: impl Into<String>,
        target_provider: impl Into<String>,
        title: impl Into<String>,
        evidence_generation: u64,
        evidence_sha256: impl Into<String>,
    ) -> Result<Self, DiscoveryError> {
        let candidate = Self {
            source: ConnectionCandidateSource::LocalConfiguration {
                candidate_binding: candidate_binding.into(),
            },
            target_provider: target_provider.into(),
            title: title.into(),
            evidence_generation,
            evidence_sha256: evidence_sha256.into(),
            route: ConnectionRoute::Direct,
        };
        candidate.validate()?;
        Ok(candidate)
    }

    fn validate(&self) -> Result<(), DiscoveryError> {
        let source_valid = match &self.source {
            ConnectionCandidateSource::LocalConfiguration { candidate_binding } => {
                valid_ref(candidate_binding) && matches!(self.route, ConnectionRoute::Direct)
            }
            ConnectionCandidateSource::Observation { observation } => {
                valid_ref(observation)
                    && matches!(self.route, ConnectionRoute::ViaConnection { .. })
            }
        };
        if !source_valid
            || !valid_ref(&self.target_provider)
            || self.title.trim().is_empty()
            || self.title.len() > 256
            || self.title.chars().any(char::is_control)
            || self.evidence_generation == 0
            || self.evidence_sha256.len() != 64
            || !self
                .evidence_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(DiscoveryError::InvalidCandidate);
        }
        Ok(())
    }
}

impl DiscoveryObservation {
    /// Construct a recognized observation from a closed catalog mapping.
    #[allow(clippy::too_many_arguments)]
    pub fn recognized(
        id: impl Into<String>,
        declaration: impl Into<String>,
        source_connection: impl Into<String>,
        observed_type: impl Into<String>,
        title: impl Into<String>,
        evidence_generation: u64,
        evidence_sha256: impl Into<String>,
        resource_binding: impl Into<String>,
        target_provider: impl Into<String>,
        adapter: RouteAdapter,
    ) -> Result<Self, DiscoveryError> {
        Self::new(
            id,
            declaration,
            source_connection,
            observed_type,
            title,
            evidence_generation,
            evidence_sha256,
            resource_binding,
            Some(DiscoveryTarget {
                provider: target_provider.into(),
                adapter,
            }),
        )
    }

    /// Construct an observation whose vendor type has no reviewed target Provider mapping.
    #[allow(clippy::too_many_arguments)]
    pub fn unsupported(
        id: impl Into<String>,
        declaration: impl Into<String>,
        source_connection: impl Into<String>,
        observed_type: impl Into<String>,
        title: impl Into<String>,
        evidence_generation: u64,
        evidence_sha256: impl Into<String>,
        resource_binding: impl Into<String>,
    ) -> Result<Self, DiscoveryError> {
        Self::new(
            id,
            declaration,
            source_connection,
            observed_type,
            title,
            evidence_generation,
            evidence_sha256,
            resource_binding,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        id: impl Into<String>,
        declaration: impl Into<String>,
        source_connection: impl Into<String>,
        observed_type: impl Into<String>,
        title: impl Into<String>,
        evidence_generation: u64,
        evidence_sha256: impl Into<String>,
        resource_binding: impl Into<String>,
        target: Option<DiscoveryTarget>,
    ) -> Result<Self, DiscoveryError> {
        let value = Self {
            id: id.into(),
            declaration: declaration.into(),
            source_connection: source_connection.into(),
            observed_type: observed_type.into(),
            title: title.into(),
            evidence_generation,
            evidence_sha256: evidence_sha256.into(),
            resource_binding: resource_binding.into(),
            target,
        };
        if !valid_ref(&value.id)
            || !valid_ref(&value.declaration)
            || !valid_ref(&value.source_connection)
            || !valid_ref(&value.resource_binding)
            || value.observed_type.is_empty()
            || value.observed_type.len() > 128
            || value.observed_type.chars().any(char::is_control)
            || value.title.trim().is_empty()
            || value.title.len() > 256
            || value.title.chars().any(char::is_control)
            || value.evidence_generation == 0
            || value.evidence_sha256.len() != 64
            || !value
                .evidence_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || value
                .target
                .as_ref()
                .is_some_and(|target| !valid_ref(&target.provider))
        {
            return Err(DiscoveryError::InvalidObservation);
        }
        Ok(value)
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn declaration(&self) -> &str {
        &self.declaration
    }

    #[must_use]
    pub fn source_connection(&self) -> &str {
        &self.source_connection
    }

    #[must_use]
    pub fn observed_type(&self) -> &str {
        &self.observed_type
    }

    /// Produce a candidate only for a recognized mapping. No Connection or authority is created.
    #[must_use]
    pub fn candidate(&self) -> Option<ConnectionCandidate> {
        let target = self.target.as_ref()?;
        Some(ConnectionCandidate {
            source: ConnectionCandidateSource::Observation {
                observation: self.id.clone(),
            },
            target_provider: target.provider.clone(),
            title: self.title.clone(),
            evidence_generation: self.evidence_generation,
            evidence_sha256: self.evidence_sha256.clone(),
            route: ConnectionRoute::ViaConnection {
                parent_connection: self.source_connection.clone(),
                resource_binding: self.resource_binding.clone(),
                adapter: target.adapter,
            },
        })
    }
}

fn valid_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .chars()
            .all(|character| !character.is_whitespace() && !character.is_control())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DiscoveryError {
    #[error("discovery observation is invalid")]
    InvalidObservation,
    #[error("connection candidate is invalid")]
    InvalidCandidate,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest() -> String {
        "a".repeat(64)
    }

    #[test]
    fn recognized_observation_yields_a_candidate_but_not_authority() {
        let observation = DiscoveryObservation::recognized(
            "observation:grafana:prometheus",
            "grafana-data-sources",
            "connection:grafana-infra",
            "prometheus",
            "Infrastructure Prometheus",
            7,
            digest(),
            "binding:sealed-datasource",
            "prometheus",
            RouteAdapter::GrafanaDatasourceProxyV1,
        )
        .unwrap();
        let candidate = observation.candidate().expect("recognized mapping");
        assert_eq!(candidate.target_provider, "prometheus");
        assert!(matches!(
            candidate.route,
            ConnectionRoute::ViaConnection { ref parent_connection, .. }
                if parent_connection == "connection:grafana-infra"
        ));
    }

    #[test]
    fn unknown_vendor_type_remains_an_observation_without_a_candidate() {
        let observation = DiscoveryObservation::unsupported(
            "observation:grafana:unknown",
            "grafana-data-sources",
            "connection:grafana-infra",
            "vendor-private-plugin",
            "Vendor plugin",
            7,
            digest(),
            "binding:sealed-datasource",
        )
        .unwrap();
        assert_eq!(observation.observed_type(), "vendor-private-plugin");
        assert_eq!(observation.candidate(), None);
    }

    #[test]
    fn trusted_local_candidate_proposes_only_a_direct_route() {
        let candidate = ConnectionCandidate::direct(
            "binding:kubeconfig-context",
            "kubernetes",
            "development",
            1,
            digest(),
        )
        .unwrap();
        assert!(matches!(
            candidate.source,
            ConnectionCandidateSource::LocalConfiguration { .. }
        ));
        assert_eq!(candidate.route, ConnectionRoute::Direct);
    }
}

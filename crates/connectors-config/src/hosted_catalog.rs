//! Strict deployment configuration for generic hosted catalog acquisition.

use crate::hosted::default_connect_session_ttl_seconds;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Generic, catalog-driven self-service connections.
///
/// The provider list is deployment policy, not a second catalog: every credential name, address,
/// request template still comes from the compiled Connector catalog. Templated destinations need
/// explicit deployment bindings. An empty list admits every otherwise eligible catalog provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedCatalogConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub public_origin: Option<String>,
    #[serde(default)]
    pub grant_ref: Option<String>,
    #[serde(default)]
    pub providers: Vec<String>,
    /// Deployment-owned values for the catalogue's declared endpoint variables, keyed by provider.
    /// The hosted caller cannot submit or override these values.
    #[serde(default)]
    pub bindings: BTreeMap<String, HostedCatalogBinding>,
    #[serde(default = "default_connect_session_ttl_seconds")]
    pub connect_session_ttl_seconds: u64,
}

impl Default for HostedCatalogConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            public_origin: None,
            grant_ref: None,
            providers: Vec::new(),
            bindings: BTreeMap::new(),
            connect_session_ttl_seconds: default_connect_session_ttl_seconds(),
        }
    }
}

/// Exact endpoint configuration pinned when a hosted Connection acquires its credential.
/// Only explicit deployment configuration may select operator-network reachability.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedCatalogBinding {
    #[serde(default)]
    pub endpoints: BTreeMap<String, String>,
    #[serde(default)]
    pub network: crate::NetworkScopeConfig,
}

#[cfg(test)]
mod tests {
    #[test]
    fn hosted_catalog_endpoint_policy_is_strict_and_public_by_default() {
        let policy: super::HostedCatalogConfig = toml::from_str(
            r#"
enabled = true
public_origin = "https://connectors.example/api/connectors/v1"
grant_ref = "grant:catalog-read"
providers = ["grafana"]
[bindings.grafana.endpoints]
origin = "https://grafana.monitoring.example"
"#,
        )
        .unwrap();
        assert_eq!(
            policy.bindings["grafana"].network,
            crate::NetworkScopeConfig::Public
        );
        let round_trip: super::HostedCatalogConfig =
            toml::from_str(&toml::to_string(&policy).unwrap()).unwrap();
        assert_eq!(round_trip.bindings, policy.bindings);
        for invalid in [
            "[bindings.grafana]\nnetwork = 'unrestricted'",
            "[bindings.grafana]\ncredential = 'forbidden'",
        ] {
            assert!(toml::from_str::<super::HostedCatalogConfig>(invalid).is_err());
        }
    }
}

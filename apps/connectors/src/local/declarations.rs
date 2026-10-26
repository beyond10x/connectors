//! The connector declarations the catalog provider ships
//! (`adapters/catalog/providers/<provider>/connector.json`), compiled in: the
//! provider and this binary are released together. `setup migrate` maps
//! legacy entries through them and `connectors list` lists them.
//! `tests/setup_migrate.rs` holds this list to every provider directory of the
//! tree.
use serde_json::Value;

const DECLARATIONS: &[&str] = &[
    include_str!("../../../../adapters/catalog/providers/claude-subscription/connector.json"),
    include_str!("../../../../adapters/catalog/providers/confluence/connector.json"),
    include_str!("../../../../adapters/catalog/providers/confluence-v1/connector.json"),
    include_str!("../../../../adapters/catalog/providers/gitlab/connector.json"),
    include_str!("../../../../adapters/catalog/providers/google-calendar/connector.json"),
    include_str!("../../../../adapters/catalog/providers/google-drive/connector.json"),
    include_str!("../../../../adapters/catalog/providers/google-gmail/connector.json"),
    include_str!("../../../../adapters/catalog/providers/google-slides/connector.json"),
    include_str!("../../../../adapters/catalog/providers/hubspot/connector.json"),
    include_str!("../../../../adapters/catalog/providers/jira/connector.json"),
    include_str!("../../../../adapters/catalog/providers/opsgenie/connector.json"),
    include_str!("../../../../adapters/catalog/providers/runpod/connector.json"),
    include_str!("../../../../adapters/catalog/providers/zendesk/connector.json"),
];

/// The catalog's implementation id, the only one a declaration maps today.
pub(super) const CATALOG: &str = "catalog";

/// The shipped catalog declarations.
pub(super) fn all() -> Vec<Value> {
    DECLARATIONS
        .iter()
        .filter_map(|text| serde_json::from_str::<Value>(text).ok())
        .filter(|declaration| declaration["adapter_id"] == CATALOG)
        .collect()
}

//! Discovery stays a bounded interpretation of a declared read and every mapping is closed.

use connector_spec::{provider, DiscoveryDriver, RouteAdapter};

const OPERATION: &str = r#"
[[operations]]
id = "acme-inventory-list"
method = "GET"
direction = "read"
path = "/inventory"
description = "List the bounded inventory."
risk = "low"
idempotency = "idempotent"
effects = ["read", "network"]
interaction_shape = "unary"
protocol_driver = "http_v1"
placement_requirement = "connectors_deployment"
implementation_form = "built_in"
required_capabilities = ["private_network"]
"#;

fn definition(discovery: &str) -> String {
    format!(
        r#"id = "acme"
vendor = "Acme"
base_url = "https://acme.example"
{OPERATION}
{discovery}
"#
    )
}

#[test]
fn one_read_can_declare_a_closed_native_provider_mapping() {
    let loaded = provider::load(
        "acme.toml",
        &definition(
            r#"
[[discoveries]]
id = "acme-inventory"
operation = "acme-inventory-list"
driver = "grafana_datasource_v1"

[[discoveries.mappings]]
observed_type = "prometheus"
target_provider = "prometheus"
route_adapter = "grafana_datasource_proxy_v1"
"#,
        ),
    )
    .expect("valid discovery");
    let discovery = &loaded.connector.discoveries[0];
    assert_eq!(discovery.driver, DiscoveryDriver::GrafanaDatasourceV1);
    assert_eq!(
        discovery.mappings[0].route_adapter,
        RouteAdapter::GrafanaDatasourceProxyV1
    );
}

#[test]
fn duplicate_vendor_type_mapping_is_refused_instead_of_precedence_ordered() {
    let error = provider::load(
        "acme.toml",
        &definition(
            r#"
[[discoveries]]
id = "acme-inventory"
operation = "acme-inventory-list"
driver = "grafana_datasource_v1"

[[discoveries.mappings]]
observed_type = "prometheus"
target_provider = "prometheus"
route_adapter = "grafana_datasource_proxy_v1"

[[discoveries.mappings]]
observed_type = "prometheus"
target_provider = "loki"
route_adapter = "grafana_datasource_proxy_v1"
"#,
        ),
    )
    .expect_err("ambiguous mapping must fail");
    assert!(error
        .to_string()
        .contains("maps observed type \"prometheus\" more than once"));
}

#[test]
fn discovery_cannot_name_a_nonexistent_operation() {
    let error = provider::load(
        "acme.toml",
        &definition(
            r#"
[[discoveries]]
id = "acme-inventory"
operation = "acme-missing"
driver = "grafana_datasource_v1"

[[discoveries.mappings]]
observed_type = "prometheus"
target_provider = "prometheus"
route_adapter = "grafana_datasource_proxy_v1"
"#,
        ),
    )
    .expect_err("missing observation operation must fail");
    assert!(error
        .to_string()
        .contains("which no `[[operations]]` block declares"));
}

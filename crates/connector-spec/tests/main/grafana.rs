//! The source-grounded Grafana connector's security and curation boundaries.

use connector_spec::{
    Approval, Audience, CredentialEntry, DiscoveryDriver, PlacementRequirement, RequiredCapability,
    Risk, RouteAdapter,
};

use crate::shipped_provider;

#[test]
fn grafana_is_a_private_reachable_read_surface_with_connector_custody() {
    let connector = shipped_provider::connector("grafana");

    assert_eq!(connector.authority.as_deref(), Some("com.grafana.api"));
    assert_eq!(
        connector.audiences(),
        vec![Audience::Sre, Audience::Developer, Audience::DataAnalyst]
    );
    assert_eq!(
        connector.verify.as_deref(),
        Some("grafana-datasources-list")
    );

    let ids: Vec<&str> = connector
        .operations
        .iter()
        .map(|operation| operation.id.as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "grafana-dashboards-list",
            "grafana-dashboard-get",
            "grafana-datasources-list",
            "grafana-datasource-query",
        ]
    );

    for operation in &connector.operations {
        assert_eq!(operation.direction.word(), "read");
        assert_eq!(
            operation.placement_requirement,
            PlacementRequirement::ConnectorsDeployment
        );
        assert_eq!(
            operation.required_capabilities,
            vec![RequiredCapability::PrivateNetwork]
        );
    }
    assert_eq!(
        connector
            .operation("grafana-datasource-query")
            .expect("query operation")
            .risk,
        Risk::Medium
    );

    let credential = connector
        .auth_method("grafana.service_account_token")
        .expect("service account credential");
    assert!(
        credential.env.is_empty(),
        "Grafana must not read ambient secrets"
    );
    assert_eq!(credential.entry, Some(CredentialEntry::ConnectSession));

    let origin = connector
        .config
        .iter()
        .find(|field| field.name == "origin")
        .expect("origin field");
    assert_eq!(origin.approval, Approval::Operator);
    assert!(!origin.secret);

    let token = connector
        .config
        .iter()
        .find(|field| field.name == "service_account_token")
        .expect("token field");
    assert!(token.secret);
    assert_eq!(token.binds, "credential.grafana.service_account_token");

    let discovery = connector
        .discoveries
        .first()
        .expect("Grafana data-source discovery declaration");
    assert_eq!(discovery.id, "grafana-data-sources");
    assert_eq!(discovery.operation, "grafana-datasources-list");
    assert_eq!(discovery.driver, DiscoveryDriver::GrafanaDatasourceV1);
    assert_eq!(
        discovery
            .mappings
            .iter()
            .map(|mapping| (
                mapping.observed_type.as_str(),
                mapping.target_provider.as_str(),
                mapping.route_adapter,
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "prometheus",
                "prometheus",
                RouteAdapter::GrafanaDatasourceProxyV1,
            ),
            ("loki", "loki", RouteAdapter::GrafanaDatasourceProxyV1,),
            (
                "alertmanager",
                "alertmanager",
                RouteAdapter::GrafanaDatasourceProxyV1,
            ),
        ]
    );
}

#[test]
fn grafana_query_keeps_the_batch_bounded() {
    let connector = shipped_provider::connector("grafana");
    let operation = connector
        .operation("grafana-datasource-query")
        .expect("query operation");
    let queries = operation
        .params
        .body
        .iter()
        .find(|parameter| parameter.name == "queries")
        .expect("queries body parameter");

    assert_eq!(queries.schema["minItems"], 1);
    assert_eq!(queries.schema["maxItems"], 20);
    assert_eq!(queries.schema["items"]["required"][0], "refId");
    assert_eq!(queries.schema["items"]["required"][1], "datasource");
}

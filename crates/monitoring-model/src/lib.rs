#![forbid(unsafe_code)]

//! Neutral monitoring operation metadata shared by direct and mediated routes.

use std::collections::BTreeSet;

use connector_resolve::document::HostEffect;
use protocol::operation::{EffectClass, OperationError, OperationErrorCode};
use serde_json::Value;

pub const GRAFANA: &str = "grafana";
pub const DISCOVERY_REF: &str = "grafana-data-sources";
pub const GRAFANA_DATASOURCES_LIST: &str = "grafana-datasources-list";
pub const GRAFANA_DASHBOARDS_LIST: &str = "grafana-dashboards-list";
pub const GRAFANA_DASHBOARD_GET: &str = "grafana-dashboard-get";
pub const PROMETHEUS_QUERY_RANGE: &str = "prometheus-query-range";
pub const LOKI_QUERY_RANGE: &str = "loki-query-range";
pub const ALERTMANAGER_ALERTS_LIST: &str = "alertmanager-alerts-list";

const GRAFANA_DOCUMENT: &str = include_str!("../../../catalog/grafana.catalog.json");
const PROMETHEUS_DOCUMENT: &str = include_str!("../../../catalog/prometheus.catalog.json");
const LOKI_DOCUMENT: &str = include_str!("../../../catalog/loki.catalog.json");
const ALERTMANAGER_DOCUMENT: &str = include_str!("../../../catalog/alertmanager.catalog.json");

#[must_use]
pub fn operation_ids() -> [&'static str; 6] {
    [
        GRAFANA_DASHBOARDS_LIST,
        GRAFANA_DASHBOARD_GET,
        GRAFANA_DATASOURCES_LIST,
        PROMETHEUS_QUERY_RANGE,
        LOKI_QUERY_RANGE,
        ALERTMANAGER_ALERTS_LIST,
    ]
}

#[must_use]
pub fn supported_operation(operation: &str) -> bool {
    operation_ids().contains(&operation)
}

#[must_use]
pub fn provider_for_operation(operation: &str) -> &'static str {
    match operation {
        GRAFANA_DASHBOARDS_LIST | GRAFANA_DASHBOARD_GET | GRAFANA_DATASOURCES_LIST => GRAFANA,
        PROMETHEUS_QUERY_RANGE => "prometheus",
        LOKI_QUERY_RANGE => "loki",
        ALERTMANAGER_ALERTS_LIST => "alertmanager",
        _ => "",
    }
}

#[must_use]
pub fn document_text(provider: &str) -> &'static str {
    match provider {
        GRAFANA => GRAFANA_DOCUMENT,
        "prometheus" => PROMETHEUS_DOCUMENT,
        "loki" => LOKI_DOCUMENT,
        "alertmanager" => ALERTMANAGER_DOCUMENT,
        _ => "{}",
    }
}

#[must_use]
pub fn operation_document(
    operation: &str,
) -> Option<&'static connector_resolve::document::Operation> {
    connector_resolve::document::provider(provider_for_operation(operation))?.operation(operation)
}

#[must_use]
pub fn audiences_for_operation(operation: &str) -> Vec<String> {
    let Some(operation) = catalog::operation(catalog::OperationKey::id(operation)) else {
        return Vec::new();
    };
    let Some(provider) = catalog::provider(catalog::ProviderKey::id(operation.provider)) else {
        return Vec::new();
    };
    provider
        .services
        .iter()
        .find(|service| service.name == operation.service)
        .map_or(provider.audiences, |service| service.audiences)
        .iter()
        .map(|audience| audience.as_str().to_owned())
        .collect()
}

pub fn response_schema(provider: &str, operation: &str) -> Result<Value, OperationError> {
    let value: Value = serde_json::from_str(document_text(provider)).map_err(|_| unavailable())?;
    value["operations"]
        .as_array()
        .and_then(|operations| {
            operations
                .iter()
                .find(|candidate| candidate["id"] == operation)
        })
        .and_then(|operation| operation.get("response_schema"))
        .cloned()
        .ok_or_else(unavailable)
}

#[must_use]
pub fn target_provider(observed_type: &str) -> Option<&'static str> {
    let provider = catalog::provider(catalog::ProviderKey::id(GRAFANA))?;
    provider
        .discoveries
        .iter()
        .find(|discovery| discovery.id == DISCOVERY_REF)?
        .mappings
        .iter()
        .find(|mapping| mapping.observed_type == observed_type)
        .map(|mapping| mapping.target_provider)
}

pub fn validate_input(operation: &str, input: &Value) -> Result<(), OperationError> {
    if serde_json::to_vec(input).map_or(true, |bytes| {
        bytes.len() > protocol::operation::MAX_ARGUMENT_BYTES
    }) {
        return Err(invalid());
    }
    let object = input.as_object().ok_or_else(invalid)?;
    let expected = operation_document(operation)
        .ok_or_else(not_found)?
        .caller_parameters()
        .into_iter()
        .collect::<BTreeSet<_>>();
    if object.len() != expected.len() || object.keys().any(|key| !expected.contains(key.as_str())) {
        return Err(invalid());
    }
    let string = |name: &str, maximum: usize| {
        object
            .get(name)
            .and_then(Value::as_str)
            .is_some_and(|value| {
                !value.is_empty() && value.len() <= maximum && !value.contains('\0')
            })
    };
    let valid = match operation {
        GRAFANA_DASHBOARDS_LIST => {
            string("namespace", 256)
                && object
                    .get("limit")
                    .and_then(Value::as_u64)
                    .is_some_and(|limit| (1..=1000).contains(&limit))
                && object
                    .get("continue")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value.len() <= 4096 && !value.contains('\0'))
        }
        GRAFANA_DASHBOARD_GET => string("namespace", 256) && string("uid", 256),
        GRAFANA_DATASOURCES_LIST | ALERTMANAGER_ALERTS_LIST => object.is_empty(),
        PROMETHEUS_QUERY_RANGE => {
            string("query", 16 * 1024)
                && string("start", 64)
                && string("end", 64)
                && string("step", 64)
        }
        LOKI_QUERY_RANGE => {
            string("query", 16 * 1024)
                && string("start", 64)
                && string("end", 64)
                && object
                    .get("limit")
                    .and_then(Value::as_u64)
                    .is_some_and(|limit| (1..=5000).contains(&limit))
                && object
                    .get("direction")
                    .and_then(Value::as_str)
                    .is_some_and(|direction| matches!(direction, "forward" | "backward"))
        }
        _ => false,
    };
    valid.then_some(()).ok_or_else(invalid)
}

#[must_use]
pub fn effect(effects: &[HostEffect]) -> EffectClass {
    if effects.contains(&HostEffect::Write) {
        EffectClass::Mutating
    } else {
        EffectClass::ReadOnly
    }
}

#[must_use]
pub fn title(operation: &str) -> &'static str {
    match operation {
        GRAFANA_DASHBOARDS_LIST => "List Grafana dashboards",
        GRAFANA_DASHBOARD_GET => "Get a Grafana dashboard",
        GRAFANA_DATASOURCES_LIST => "Refresh Grafana datasource discovery",
        PROMETHEUS_QUERY_RANGE => "Query Prometheus metrics over a time range",
        LOKI_QUERY_RANGE => "Query Loki logs over a time range",
        ALERTMANAGER_ALERTS_LIST => "List Alertmanager alerts",
        _ => "Unknown operation",
    }
}

fn unavailable() -> OperationError {
    OperationError::new(
        OperationErrorCode::Unavailable,
        "monitoring operation metadata is unavailable",
        false,
    )
}

fn not_found() -> OperationError {
    OperationError::new(
        OperationErrorCode::NotFound,
        "monitoring operation was not found",
        false,
    )
}

fn invalid() -> OperationError {
    OperationError::new(
        OperationErrorCode::InvalidInput,
        "monitoring operation input is invalid",
        false,
    )
}

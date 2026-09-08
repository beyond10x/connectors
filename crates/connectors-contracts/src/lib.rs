//! Higher-level, provider-neutral data and discovery contracts.
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub instance: String,
    pub resource: String,
    pub observed_at_unix_ms: u64,
    pub source_revision: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page<T = Value> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub complete: bool,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointObservation {
    pub id: String,
    pub namespace: String,
    pub service: Option<String>,
    pub address: String,
    pub port: u16,
    pub transport: String,
    pub application_protocol: Option<String>,
    pub ready: Option<bool>,
    pub source_uid: String,
    pub reachability: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Column {
    pub name: String,
    pub native_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryResult {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Value>>,
    pub truncated: bool,
    pub provenance: Provenance,
}

//! Grafana binding of the profile `grafana-datasources` (`datasource.records/v1alpha1`;
//! `spec/ess/domains/records.yaml`, `design.md` beside this adapter). `datasources.list`
//! is one GET of `/api/datasources` through `AuthenticatedHttp`; the binding is unpaged
//! and retains nothing between calls.
//!
//! Each data source is projected onto its uid, name, plugin type, access mode and default
//! flag. Grafana's numeric ids, the backend `url`, `user`, `database`, basic-auth members,
//! `jsonData`, `secureJsonFields` and every other member are never read into a result.
//!
//! The IRM reads (`irm.*`, `spec/ess/domains/irm.yaml`) live in [`irm`]; they need the
//! second port the composition builds for a configured `irm_api_url`.
pub mod auth;
mod irm;

pub use irm::{
    ALERT_GROUP_PAGE, ALERT_GROUPS, INCIDENT_PAGE, INCIDENT_READ_POSTS, INCIDENT_REFS, ONCALL_REF,
};

use async_trait::async_trait;
use connectors_core::{Descriptor, Error, ErrorCode, Result};
use connectors_sdk::{Adapter, AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

const OPERATIONS: [&str; 5] = [
    "datasources.list",
    "irm.alert_groups.list",
    "irm.alert_group.get",
    "irm.incidents.list",
    "irm.incident.get",
];
/// The header that names the stack to the IRM API, fixed by the composition.
pub const STACK_HEADER: &str = "X-Grafana-URL";
/// The deadline of one incident read sent as a POST.
pub const READ_POST_TIMEOUT: Duration = Duration::from_secs(30);

/// The value of [`STACK_HEADER`]: the configured stack URL as IRM's documentation writes
/// it, without the canonical form's final `/`.
pub fn stack_header_value(base_url: &str) -> String {
    base_url.strip_suffix('/').unwrap_or(base_url).to_owned()
}
/// `GET /api/datasources`, relative to the configured base URL.
pub const DATASOURCES: [&str; 2] = ["api", "datasources"];

/// Records returned per call (`DatasourceList.items.count <= 1000`).
pub const RECORDS: usize = 1000;
/// Data sources Grafana may list before the answer is refused as unusable.
pub const PROVIDER_RECORDS: usize = 100_000;
/// `DatasourceRecord.name.count <= 190`, Grafana's own column width.
pub const NAME_CHARS: usize = 190;
/// `DatasourceRecord.type.count <= 128`.
pub const TYPE_CHARS: usize = 128;
/// `DatasourceUid.value.count <= 40`.
pub const UID_CHARS: usize = 40;

pub struct Grafana {
    instance: String,
    descriptor: Descriptor,
    /// The stack (`base_url`): data sources, and the two incident reads sent as a POST.
    http: Arc<dyn AuthenticatedHttp>,
    /// The IRM API URL, carrying the stack header; absent without `irm_api_url`, and then
    /// every `irm.*` read is unavailable.
    irm: Option<Arc<dyn AuthenticatedHttp>>,
    clock: fn() -> u64,
}

impl Grafana {
    pub fn new(
        instance: &str,
        effective_configuration: &Value,
        http: Arc<dyn AuthenticatedHttp>,
    ) -> Result<Self> {
        let descriptor = connectors_sdk::instance_descriptor(
            include_str!("../generated/descriptor.json"),
            instance,
            effective_configuration,
        )?;
        connectors_sdk::verify_handlers(&descriptor, &OPERATIONS)?;
        Ok(Self {
            instance: instance.to_owned(),
            descriptor,
            http,
            irm: None,
            clock: connectors_sdk::now_ms,
        })
    }

    /// A new immutable HTTP capability for one host-admitted use. It carries no IRM port;
    /// [`Self::with_irm`] adds the one composed for the same use.
    pub fn with_authenticated_http(&self, http: Arc<dyn AuthenticatedHttp>) -> Self {
        Self {
            instance: self.instance.clone(),
            descriptor: self.descriptor.clone(),
            http,
            irm: None,
            clock: self.clock,
        }
    }

    /// The IRM API port, which the composition built on the configured `irm_api_url` with
    /// [`STACK_HEADER`] fixed. Without it every `irm.*` read is unavailable.
    pub fn with_irm(mut self, http: Arc<dyn AuthenticatedHttp>) -> Self {
        self.irm = Some(http);
        self
    }

    /// For tests: a fixed clock in milliseconds since the Unix epoch.
    pub fn with_clock(mut self, clock: fn() -> u64) -> Self {
        self.clock = clock;
        self
    }

    fn input_schema(&self, operation: &str) -> Result<&Value> {
        self.descriptor
            .operations
            .iter()
            .find(|o| o.id == operation)
            .map(|o| &o.input_schema)
            .ok_or_else(|| Error::new(ErrorCode::NotFound, "operation is not provided"))
    }

    async fn datasources(&self) -> Result<Value> {
        let response = self
            .http
            .get(&DATASOURCES, &[])
            .await
            .map_err(|e| match e.code {
                ErrorCode::Timeout => e,
                _ => Error::unavailable(),
            })?;
        let answer = answer(response)?;
        let listed = answer.as_array().ok_or_else(malformed)?;
        if listed.len() > PROVIDER_RECORDS {
            return Err(malformed());
        }
        // Validate the whole answer before anything is returned.
        let mut items = Vec::with_capacity(listed.len().min(RECORDS));
        for source in listed {
            let record = record(source)?;
            if items.len() < RECORDS {
                items.push(record);
            }
        }
        Ok(json!({
            "items": items,
            "complete": listed.len() <= RECORDS,
            "next_cursor": null,
            "provenance": {
                "instance": self.instance,
                "resource": "grafana:datasources",
                "observed_at_unix_ms": (self.clock)(),
                "source_revision": null,
            },
        }))
    }
}

/// A provider status as the family's error, never carrying the provider's text.
fn answer(response: HttpResponse) -> Result<Value> {
    match response.status {
        200 => {}
        401 => {
            return Err(Error::new(
                ErrorCode::Unauthorized,
                "Grafana refused access",
            ));
        }
        403 => return Err(Error::new(ErrorCode::Forbidden, "Grafana refused the read")),
        429 => {
            return Err(Error::new(
                ErrorCode::RateLimited,
                "Grafana rate limit reached",
            ));
        }
        _ => return Err(Error::unavailable()),
    }
    connectors_core::read_json::<Value>(&response.body).map_err(|_| malformed())
}

pub(crate) fn malformed() -> Error {
    Error::new(ErrorCode::Unavailable, "malformed Grafana answer")
}

fn text<'a>(
    source: &'a serde_json::Map<String, Value>,
    member: &str,
    chars: usize,
) -> Result<&'a str> {
    let value = source
        .get(member)
        .and_then(Value::as_str)
        .ok_or_else(malformed)?;
    let count = value.chars().count();
    if count == 0 || count > chars {
        return Err(malformed());
    }
    Ok(value)
}

/// One Grafana data source as a `DatasourceRecord`. Only the five selected members are
/// read; a missing or out-of-model member refuses the whole answer.
fn record(source: &Value) -> Result<Value> {
    let source = source.as_object().ok_or_else(malformed)?;
    let uid = text(source, "uid", UID_CHARS)?;
    if !uid
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(malformed());
    }
    let name = text(source, "name", NAME_CHARS)?;
    let kind = text(source, "type", TYPE_CHARS)?;
    let access = match source.get("access").and_then(Value::as_str) {
        Some(access @ ("proxy" | "direct")) => access,
        _ => return Err(malformed()),
    };
    let is_default = source
        .get("isDefault")
        .and_then(Value::as_bool)
        .ok_or_else(malformed)?;
    Ok(json!({
        "uid": uid,
        "name": name,
        "type": kind,
        "access": access,
        "is_default": is_default,
    }))
}

#[async_trait]
impl Adapter for Grafana {
    fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value> {
        connectors_sdk::validate(self.input_schema(operation)?, &input)?;
        match operation {
            "datasources.list" => self.datasources().await,
            "irm.alert_groups.list" => self.alert_groups(&input).await,
            "irm.alert_group.get" => self.alert_group(&input).await,
            "irm.incidents.list" => self.incidents(&input).await,
            "irm.incident.get" => self.incident(&input).await,
            _ => Err(Error::new(ErrorCode::NotFound, "operation is not provided")),
        }
    }
}

//! Projection of a fresh, audited owner snapshot into the selected read-only tools slice.
use super::*;
use connectors_mcp::{names, protocol, results::Envelope};
use serde_json::Value;
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

pub(super) struct Snapshot {
    pub bootstrap: runtime::Bootstrap,
    pub revision: String,
    metadata: Value,
    pub alias: String,
    pub connection: String,
    pub host: String,
    configuration: Configuration,
    adapter: connectors_host::local::config::Adapter,
}

pub(super) fn refusal(code: &str) -> Value {
    json!({"code":-32000,"message":"Connectors request refused","data":{
        "version":"v1alpha2","request_id":null,"status":"error",
        "error":{"code":code,"message":"Connectors request refused"},"audit_ref":null,"audit_status":"unavailable"}})
}
fn owner_error(error: owner::Error) -> Value {
    refusal(match error.code {
        Code::NotGranted => "not_granted",
        Code::Forbidden => "forbidden",
        Code::NotFound => "not_found",
        Code::StaleDescription => "stale_description",
        Code::InvalidInput => "invalid_input",
        Code::Unsupported => "unsupported",
        Code::Capacity => "capacity",
        Code::Timeout => "timeout",
        _ => "unavailable",
    })
}
fn check(until: Instant) -> Result<(), Value> {
    if Instant::now() >= until {
        Err(refusal("timeout"))
    } else {
        Ok(())
    }
}
fn client(paths: &Paths, host: Option<&str>, until: Instant) -> Result<owner::Client, Value> {
    let client = owner::Client::connect_until(paths, host.is_none(), until).map_err(owner_error)?;
    if host.is_some_and(|host| host != client.host_incarnation) {
        return Err(refusal("stale_authority"));
    }
    Ok(client)
}

pub(super) fn snapshot(
    paths: &Paths,
    host: Option<&str>,
    until: Instant,
) -> Result<Snapshot, Value> {
    check(until)?;
    let configuration = load(paths).map_err(owner_error)?;
    let first = configuration
        .document()
        .exposures
        .first()
        .ok_or_else(|| refusal("unsupported"))?;
    let (alias, connection) = (first.adapter_alias.clone(), first.connection_ref.clone());
    // The selected local session binding has one configured instance/connection.
    if configuration
        .document()
        .exposures
        .iter()
        .any(|e| e.adapter_alias != alias || e.connection_ref != connection)
    {
        return Err(refusal("unsupported"));
    }
    let config = Config::load(&paths.config).map_err(|_| refusal("unavailable"))?;
    let adapter = config
        .adapters
        .get(&alias)
        .cloned()
        .ok_or_else(|| refusal("not_granted"))?;
    let owner = client(paths, host, until)?;
    let host = owner.host_incarnation.clone();
    let left = until.saturating_duration_since(Instant::now()).as_millis() as u64;
    check(until)?;
    let bytes = owner
        .projected_describe(&alias, connectors_sdk::now_ms().saturating_add(left))
        .map_err(owner_error)?;
    check(until)?;
    let envelope =
        Envelope::from_owner(&bytes, runtime::RESULT_LIMIT).map_err(|_| refusal("unavailable"))?;
    if envelope.value()["status"] != "success" {
        return Err(envelope.rpc_error().map_err(|_| refusal("unavailable"))?);
    }
    let result = &envelope.value()["result"];
    let bootstrap: runtime::Bootstrap =
        serde_json::from_value(result["bootstrap"].clone()).map_err(|_| refusal("unavailable"))?;
    bootstrap
        .validate_for(adapter.private_protocol())
        .map_err(|_| refusal("unavailable"))?;
    if bootstrap.instance != adapter.instance_id || bootstrap.adapter != adapter.adapter_id {
        return Err(refusal("stale_description"));
    }
    let revision = result["projection_revision"]
        .as_str()
        .filter(|r| connectors_core::valid_id(r))
        .ok_or_else(|| refusal("unavailable"))?
        .to_owned();
    if load(paths).map_err(owner_error)?.document() != configuration.document()
        || Config::load(&paths.config)
            .map_err(|_| refusal("unavailable"))?
            .adapters
            .get(&alias)
            .is_none_or(|current| current.selection() != adapter.selection())
    {
        return Err(refusal("stale_description"));
    }
    Ok(Snapshot {
        bootstrap,
        revision,
        metadata: result["operation_metadata"].clone(),
        alias,
        connection,
        host,
        configuration,
        adapter,
    })
}

impl Snapshot {
    fn limits(&self, operation: &connectors_core::Operation) -> Option<(u64, u64, usize, usize)> {
        // Nesting a schema changes root-relative reference resolution. This first
        // binding advertises only self-contained inline schemas, never a broken
        // projection or an implicit external schema fetch.
        if !inline_schema(&operation.input_schema) || !inline_schema(&operation.output_schema) {
            return None;
        }
        if self.adapter.private_protocol() != runtime::PrivateProtocol::V3 {
            return None;
        }
        let metadata = self.metadata.get(&operation.id)?;
        connectors_core::operation_metadata::Metadata::parse(
            &serde_json::to_vec(metadata).ok()?,
            &operation.profile,
            65536,
        )
        .ok()?;
        let expected = match (metadata["realization"].as_str(), operation.profile.as_str()) {
            (Some("implemented"), "resource") => (20_000, 15_000, 65_536, 4_194_304),
            (Some("generic"), "generic-http" | "generic-http-page") => {
                (40_000, 30_000, 262_144, 4_194_304)
            }
            _ => return None,
        };
        if metadata["approval"] != "not_required"
            || !matches!(
                metadata["idempotency"]["kind"].as_str(),
                Some("none" | "natural")
            )
            || metadata["semantic_effects"]
                .as_array()
                .is_none_or(|effects| !effects.is_empty())
            || metadata["effects"]
                .as_array()
                .is_none_or(|effects| effects.iter().any(|effect| effect != "network"))
            || !self
                .bootstrap
                .requirements
                .iter()
                .any(|r| r.operation == operation.id && r.effect == runtime::Effect::Read)
        {
            return None;
        }
        let limits = &metadata["limits"];
        if limits["execution_ms"].as_u64() != Some(expected.0)
            || limits["provider_ms"].as_u64() != Some(expected.1)
            || limits["request_bytes"].as_u64() != Some(expected.2 as u64)
            || limits["result_bytes"].as_u64() != Some(expected.3 as u64)
            || limits["connect_ms"].as_u64() != Some(5000)
        {
            return None;
        }
        let config = serde_json::to_value(&self.configuration.document().limits).ok()?;
        if config["frame_octets"].as_u64()? < expected.2 as u64 + 8192
            || config["response_octets"].as_u64()? < 7 * expected.3 as u64 + 8192
            || config["request_milliseconds"].as_u64()? < expected.0
        {
            return None;
        }
        Some(expected)
    }
    fn exposure(&self, id: &str) -> Option<&connectors_mcp::configuration::Exposure> {
        self.configuration
            .document()
            .exposures
            .iter()
            .find(|e| e.operation_ref == id)
            .map(Box::as_ref)
    }
    fn tool(&self, operation: &connectors_core::Operation) -> Option<Value> {
        self.limits(operation)?;
        let exposure = self.exposure(&operation.id)?;
        if !exposure.enabled
            || !exposure
                .families
                .iter()
                .any(|f| serde_json::to_value(f).ok() == Some(json!("tools")))
        {
            return None;
        }
        let name = names::encode_name([
            &self.bootstrap.instance,
            &self.bootstrap.adapter,
            &operation.id,
        ])?;
        Some(json!({"name":name,"description":operation.description,
            "inputSchema":{"type":"object","properties":{"input":operation.input_schema},"required":["input"],"additionalProperties":false},
            "outputSchema":response_schema(operation.output_schema.clone()),
            "annotations":{"readOnlyHint":true,"destructiveHint":false},
            "_meta":{protocol::REVISION:self.revision,"io.beyond10x.connectors/operation":{
                "instance":self.bootstrap.instance,"adapter":self.bootstrap.adapter,"operation":operation.id,
                "contract":operation.contract,"profile":operation.profile,"metadata":self.metadata[&operation.id]}}}))
    }
    fn list(&self, params: &Value, primary: bool) -> Result<Value, Value> {
        let descriptor = self
            .bootstrap
            .descriptor()
            .map_err(|_| refusal("unavailable"))?;
        let mut tools: Vec<_> = descriptor
            .operations
            .iter()
            .filter_map(|op| self.tool(op))
            .collect();
        tools.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        let scope = connectors_core::digest(
            &json!({"family":"tools","host":self.host,"revision":self.revision,"alias":self.alias,"connection":self.connection}),
        );
        let mut start = 0;
        if let Some(cursor) = params.get("cursor") {
            let cursor = cursor
                .as_str()
                .filter(|c| c.len() < 100)
                .ok_or_else(|| protocol::error(-32602, "Invalid cursor"))?;
            let (prior, offset) = cursor
                .split_once(':')
                .ok_or_else(|| protocol::error(-32602, "Invalid cursor"))?;
            if prior != scope {
                return Err(refusal("stale_cursor"));
            }
            start = offset
                .parse()
                .map_err(|_| protocol::error(-32602, "Invalid cursor"))?;
            if start == 0 || start > tools.len() || offset != start.to_string() {
                return Err(refusal("stale_cursor"));
            }
        }
        let end = (start + 32).min(tools.len());
        let mut result =
            json!({"tools":tools[start..end],"_meta":{protocol::REVISION:self.revision}});
        if end < tools.len() {
            result["nextCursor"] = json!(format!("{scope}:{end}"));
        }
        Ok(protocol::complete(result, primary, true))
    }
    fn invoke(
        &self,
        paths: &Paths,
        params: &Value,
        primary: bool,
        until: Instant,
        cancelled: &AtomicBool,
    ) -> Result<Value, Value> {
        let name = params["name"]
            .as_str()
            .and_then(names::decode_name)
            .ok_or_else(|| protocol::error(-32602, "Invalid tool name"))?;
        if name[0] != self.bootstrap.instance || name[1] != self.bootstrap.adapter {
            return Err(refusal("forbidden"));
        }
        if !self.adapter.permissions.operations.contains(&name[2]) {
            return Err(refusal("not_granted"));
        }
        let revision = params["_meta"][protocol::REVISION]
            .as_str()
            .filter(|r| connectors_core::valid_id(r))
            .ok_or_else(|| protocol::error(-32602, "Missing projection revision"))?;
        if revision != self.revision {
            return Err(refusal("stale_description"));
        }
        let descriptor = self
            .bootstrap
            .descriptor()
            .map_err(|_| refusal("unavailable"))?;
        let operation = descriptor
            .operation(&name[2])
            .map_err(|_| refusal("not_found"))?;
        let (execution, provider, input_limit, result_limit) =
            self.limits(operation).ok_or_else(|| refusal("not_found"))?;
        let exposure = self
            .exposure(&operation.id)
            .ok_or_else(|| refusal("not_found"))?;
        if !exposure.enabled {
            return Err(refusal("forbidden"));
        }
        if self.tool(operation).is_none() {
            return Err(refusal("not_found"));
        }
        let arguments = params["arguments"]
            .as_object()
            .filter(|args| args.len() == 1 && args.contains_key("input"))
            .ok_or_else(|| protocol::error(-32602, "Expected arguments.input"))?;
        let input = arguments["input"].to_string();
        let schema = owner::schema(&self.bootstrap, &operation.id).map_err(owner_error)?;
        let budget = runtime::ReadBudget::start(execution, provider, input_limit, result_limit)
            .and_then(|b| b.constrain(until))
            .map_err(|_| refusal("timeout"))?;
        if cancelled.load(Ordering::SeqCst) {
            return Err(refusal("timeout"));
        }
        let owner = client(paths, Some(&self.host), until)?;
        if cancelled.load(Ordering::SeqCst) {
            return Err(refusal("timeout"));
        }
        let bytes = owner
            .bounded_projected_read(
                &self.alias,
                &owner::approval_issuance::Request {
                    connection: &self.connection,
                    operation: &operation.id,
                    schema: &schema,
                    revision: &descriptor.revision,
                    input: &input,
                },
                &self.revision,
                budget,
            )
            .map_err(owner_error)?;
        check(until)?;
        let envelope =
            Envelope::from_owner(&bytes, result_limit).map_err(|_| refusal("unavailable"))?;
        if envelope.value()["status"] == "error"
            && (matches!(
                envelope.value()["error"]["code"].as_str(),
                Some(
                    "not_granted" | "forbidden" | "not_found" | "stale_description" | "unsupported"
                )
            ) || envelope.value()["audit_status"] == "unavailable")
        {
            return Err(envelope.rpc_error().map_err(|_| refusal("unavailable"))?);
        }
        Ok(envelope.resolved_tool(primary))
    }
}

pub(super) fn execute(
    paths: &Paths,
    host: &str,
    request: &Value,
    primary: bool,
    until: Instant,
    cancelled: &AtomicBool,
) -> Result<Value, Value> {
    if cancelled.load(Ordering::SeqCst) {
        return Err(refusal("timeout"));
    }
    let current = snapshot(paths, Some(host), until)?;
    match request["method"].as_str() {
        Some("tools/list") => current.list(&request["params"], primary),
        Some("tools/call") => current.invoke(paths, &request["params"], primary, until, cancelled),
        _ => Err(protocol::error(-32601, "Method not found")),
    }
}

// Native projection of the already-selected service envelope: the provider's
// schema is nested under result, never asserted to describe the outer response.
fn response_schema(result: Value) -> Value {
    json!({"type":"object","required":["version","request_id","status","audit_ref","audit_status"],
        "properties":{"version":{"const":"v1alpha2"},"request_id":{"type":["string","null"]},
            "status":{"enum":["success","error"]},"result":result,"error":{"type":"object"},
            "audit_ref":{"type":["string","null"]},"audit_status":{"enum":["complete","incomplete","unavailable","not_required"]},
            "source_audit":{"type":"object"},"mutation":{"type":"object"}},
        "oneOf":[{"properties":{"status":{"const":"success"}},"required":["result"],"not":{"required":["error"]}},
            {"properties":{"status":{"const":"error"}},"required":["error"],"not":{"required":["result"]}}],"additionalProperties":false})
}

fn inline_schema(value: &Value) -> bool {
    match value {
        Value::Object(fields) => fields.iter().all(|(key, value)| {
            !matches!(
                key.as_str(),
                "$ref"
                    | "$dynamicRef"
                    | "$recursiveRef"
                    | "$id"
                    | "$anchor"
                    | "$dynamicAnchor"
                    | "$recursiveAnchor"
            ) && inline_schema(value)
        }),
        Value::Array(values) => values.iter().all(inline_schema),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_projection_does_not_advertise_relocated_schema_references() {
        assert!(inline_schema(
            &json!({"type":"object","properties":{"value":{"type":"number"}}})
        ));
        for schema in [
            json!({"$ref":"#/$defs/value"}),
            json!({"properties":{"nested":{"$dynamicRef":"#node"}}}),
            json!({"$id":"https://schema.invalid/root"}),
        ] {
            assert!(!inline_schema(&schema));
        }
    }
}

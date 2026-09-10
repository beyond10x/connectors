use super::*;
use connectors_host::local::{config::Adapter, owner::Code, runtime::Bootstrap};

fn summary(operation: &connectors_core::Operation) -> Value {
    json!({"id":operation.id,"description":operation.description,"contract":operation.contract,"profile":operation.profile})
}
fn operation(operation: &connectors_core::Operation) -> Value {
    let mut value = summary(operation);
    value["input_schema"] = json!(operation.input_schema.to_string());
    value["output_schema"] = json!(operation.output_schema.to_string());
    value
}
pub(super) fn descriptor(bootstrap: &Bootstrap) -> owner::Result<Value> {
    let descriptor = bootstrap.descriptor()?;
    Ok(
        json!({"version":descriptor.version,"instance":descriptor.instance,"adapter":descriptor.adapter,"revision":descriptor.revision,"configuration_schema":descriptor.configuration_schema.to_string(),"operations":descriptor.operations.iter().map(operation).collect::<Vec<_>>()}),
    )
}
pub(super) fn describe(
    call: &Invocation<'_>,
    paths: &Paths,
    alias: &str,
    adapter: &Adapter,
) -> owner::Result<Value> {
    let bootstrap = owner::cached(paths, alias)?;
    let descriptor = bootstrap.descriptor()?;
    if call.callable == "operations-list" {
        if call.input.get("cursor").is_some_and(|v| !v.is_null()) {
            return Err(Code::StaleCursor.into());
        }
        let limit = call
            .input
            .get("limit")
            .and_then(Value::as_i64)
            .unwrap_or(100);
        if !(1..=500).contains(&limit) {
            return Err(Code::InvalidInput.into());
        }
        let operations = descriptor
            .operations
            .iter()
            .filter(|o| adapter.permissions.operations.contains(&o.id))
            .map(summary)
            .collect::<Vec<_>>();
        if operations.len() > limit as usize {
            return Err(Code::Capacity.into());
        }
        Ok(
            json!({"adapter":alias,"revision":descriptor.revision,"operations":operations,"source":"cached","stale":true}),
        )
    } else {
        let name = call.input["operation"].as_str().ok_or(Code::InvalidInput)?;
        if !adapter.permissions.operations.contains(name) {
            return Err(Code::Forbidden.into());
        }
        let selected = descriptor.operation(name).map_err(|_| Code::NotFound)?;
        Ok(
            json!({"adapter":alias,"revision":descriptor.revision,"schema":owner::schema(&bootstrap,name)?,"operation":operation(selected),"source":"cached","stale":true}),
        )
    }
}
pub(super) fn invoke(call: &Invocation<'_>, deadline: u64) -> owner::Result<Value> {
    let paths = Paths::resolve(
        call.context.config.as_deref(),
        call.context.state_dir.as_deref(),
    )?;
    let alias = call.input["adapter"].as_str().ok_or(Code::InvalidInput)?;
    let document = call.input["input"]
        .as_str()
        .ok_or(Code::InvalidInput)?
        .as_bytes();
    owner::admit_invoke(&paths, alias, &call.input, document)?;
    if connectors_sdk::now_ms() >= deadline {
        return Err(Code::Timeout.into());
    }
    owner::Client::connect(&paths, true)?.invoke(alias, &call.input, document, deadline)
}

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

pub(super) fn dispatch(
    call: &Invocation<'_>,
    read_deadline: u64,
    deadline: Option<owner::mutation::Deadline>,
) -> owner::Result<HandlerReply> {
    use connectors_host::local::{protected, runtime::Effect};
    use owner::{approval_issuance::Request, mutation};
    let paths = Paths::resolve(
        call.context.config.as_deref(),
        call.context.state_dir.as_deref(),
    )?;
    let text = |field: &str| call.input[field].as_str().ok_or(Code::InvalidInput);
    let alias = text("adapter")?;
    let bootstrap = owner::operation_snapshot(&paths, alias, &call.input)?;
    let operation = text("operation")?;
    let requirement = bootstrap
        .requirements
        .iter()
        .find(|r| r.operation == operation)
        .ok_or(Code::NotFound)?;
    let key = call.input["idempotency_key"].as_str();
    let proof_path = call.input["approval_file"].as_str();
    if requirement.effect == Effect::Read {
        if key.is_some() || proof_path.is_some() {
            return Err(Code::InvalidInput.into());
        }
        return invoke(call, read_deadline).map(HandlerReply::Success);
    }
    if requirement.effect != Effect::Write {
        return Err(Code::Unsupported.into());
    }
    let deadline = deadline.ok_or(Code::Unavailable)?;
    let until = deadline.until()?;
    let request = Request {
        connection: text("connection")?,
        operation,
        schema: text("schema")?,
        revision: text("revision")?,
        input: text("input")?,
    };
    // Current result-access admission and exact-key observation happen before
    // even opening the proof file, unlocking custody or starting the owner.
    let original = mutation::observe_original(&paths, alias, &request, key, until)?;
    if let Some(original) = &original
        && !original.pending
    {
        return project(call, &bootstrap, original.delivery.clone());
    }
    let pending = original.is_some();
    let value = {
        // Pending recovery is metadata-only. A deleted/locked proof source
        // cannot block it, and it never supplies a new execution proof.
        let proof = if pending {
            Ok(None)
        } else {
            proof_path
                .map(|path| protected::file_until(std::path::Path::new(path), until, 20 * 1024))
                .transpose()
        };
        let proof = match proof {
            Ok(proof) => proof,
            Err(error) => {
                // Another caller may have reserved the same key while protected
                // entry failed. Reveal only a currently admitted exact winner.
                if let Some(value) = mutation::observe(&paths, alias, &request, key, until)? {
                    return project(call, &bootstrap, value);
                }
                return Err(error);
            }
        };
        let result = owner::WriteClient::connect(&paths, true, deadline)
            .and_then(|client| client.invoke(alias, &request, key, proof.as_ref(), deadline))
            .map_err(|mut error| {
                if pending
                    && matches!(
                        error.code,
                        Code::Unavailable | Code::Timeout | Code::Interrupted | Code::Capacity
                    )
                {
                    error.code = Code::OutcomeUnknown;
                }
                error
            });
        match result {
            Ok(value) => value,
            Err(error) if error.code == Code::OutcomeUnknown => {
                let mut reply = owner_failure(error);
                if let HandlerReply::Error { data, .. } = &mut reply {
                    data["mutation"] = json!({"classification":"unknown","attempt":null,"original_request_id":null,"replayed":false,"cause":{"code":"unavailable","stage":"response"}});
                    data["source_audit"] = json!({"instance":bootstrap.instance,"audit_ref":null,"audit_status":"unavailable"});
                }
                return Ok(reply);
            }
            Err(error) => return Err(error),
        }
    };
    // No payload enters the generated success carrier without native validation.
    // The owner is the authority for current admission and effect classification.
    project(call, &bootstrap, value)
}

fn project(
    call: &Invocation<'_>,
    bootstrap: &Bootstrap,
    mut value: owner::mutation::Delivery,
) -> owner::Result<HandlerReply> {
    use owner::mutation::{Cause, FailureCode, Stage};
    if let Some(payload) = &value.result {
        let descriptor = bootstrap.descriptor()?;
        let operation = descriptor
            .operation(call.input["operation"].as_str().ok_or(Code::InvalidInput)?)
            .map_err(|_| Code::NotFound)?;
        if connectors_sdk::validate_write_value(&operation.output_schema, payload).is_err() {
            value.result = None;
            value.error = Some(owner::mutation::Failure {
                code: FailureCode::Owner(Code::ServiceFailure),
                service_code: Some(connectors_core::ErrorCode::UpstreamProtocol),
            });
            value.mutation.cause = Some(Cause {
                code: connectors_core::ErrorCode::UpstreamProtocol,
                stage: Stage::Response,
            });
        }
    }
    let mut output = if let Some(payload) = value.result {
        json!({"adapter":call.input["adapter"],"operation":call.input["operation"],"revision":call.input["revision"],"result":payload.to_string()})
    } else {
        let error = value.error.as_ref().ok_or(Code::OutcomeUnknown)?;
        let mut data = json!({"kind":"operational","code":error.code,"stage":"dispatch","next_action":"retry_status"});
        if let Some(code) = &error.service_code {
            data["service_code"] = json!(code);
        }
        data
    };
    output["request_id"] = json!(value.request_id);
    output["mutation"] = json!(value.mutation);
    output["source_audit"] = json!(value.source_audit);
    Ok(if value.error.is_some() {
        HandlerReply::Error {
            code: "failure".into(),
            data: output,
        }
    } else {
        HandlerReply::Success(output)
    })
}

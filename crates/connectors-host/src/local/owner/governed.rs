//! Audited reads for the governed service binding. Only the protected owner calls
//! this coordinator; MCP supplies neither authority nor an audit acknowledgement.
use super::{Code, Error, Result, approval_issuance, schema};
use crate::local::{
    audit,
    config::{Adapter, Config, Paths},
    filesystem, runtime,
};
use serde_json::{Value, json};
use std::time::Instant;
use uuid::Uuid;

type Request<'a> = approval_issuance::Request<'a>;
mod metadata;
pub(super) use metadata::describe;
#[cfg(test)]
mod tests;

/// Application-owned projection checks. This can constrain an already admitted
/// owner request; it never supplies generic operation or credential authority.
pub trait ReadPolicy: Send + Sync {
    fn admit(
        &self,
        paths: &Paths,
        alias: &str,
        request: &approval_issuance::Request<'_>,
        projection_revision: &str,
    ) -> Result<()>;
}
pub(super) struct UnboundReadPolicy;
impl ReadPolicy for UnboundReadPolicy {
    fn admit(&self, _: &Paths, _: &str, _: &Request<'_>, _: &str) -> Result<()> {
        Err(Code::Unsupported.into())
    }
}
type Selection<'a> = Option<(&'a dyn ReadPolicy, &'a str)>;
type Context<'a> = (&'a Paths, &'a str, Instant);

pub(super) fn registry_error(error: crate::local::registry::Failure) -> Error {
    match error {
        crate::local::registry::Failure::NotReady => Code::ConnectionNotReady.into(),
        crate::local::registry::Failure::InsufficientScope => Code::InsufficientScope.into(),
        error => error.into(),
    }
}

pub(super) struct ReadPlan {
    pub adapter: Adapter,
    fingerprint: String,
}

/// Metadata admission only: no custody, provider probe or child startup. Profile
/// permission is a credential-use preflight; it cannot hide a stale revision.
pub(super) fn resolve_selected(
    paths: &Paths,
    alias: &str,
    request: &Request<'_>,
    selection: Selection<'_>,
) -> Result<ReadPlan> {
    if selection.is_some_and(|(_, revision)| !connectors_core::valid_id(revision)) {
        return Err(Code::InvalidInput.into());
    }
    for id in [
        alias,
        request.connection,
        request.operation,
        request.schema,
        request.revision,
    ] {
        if !connectors_core::valid_id(id) {
            return Err(Code::InvalidInput.into());
        }
    }
    let config = Config::load(&paths.config)?;
    let adapter = config.adapters.get(alias).ok_or(Code::NotGranted)?;
    if !adapter.permissions.operations.contains(request.operation) {
        return Err(Code::NotGranted.into());
    }
    let bootstrap = runtime::state::State::new(&paths.state)
        .cached(&adapter.instance_id, &adapter.selection())?
        .ok_or(Code::DescriptionUnavailable)?;
    bootstrap.validate_for(adapter.private_protocol())?;
    if bootstrap.instance != adapter.instance_id
        || bootstrap.adapter != adapter.adapter_id
        || bootstrap.configuration_revision != adapter.configuration_revision
        || bootstrap.protocol != adapter.protocol
    {
        return Err(Code::ReadinessMismatch.into());
    }
    if let Some((policy, revision)) = selection {
        policy.admit(paths, alias, request, revision)?;
    }
    let descriptor = bootstrap.descriptor()?;
    if descriptor.revision != request.revision {
        return Err(Code::StaleDescription.into());
    }
    let operation = descriptor
        .operation(request.operation)
        .map_err(|_| Code::NotFound)?;
    let requirement = bootstrap
        .requirements
        .iter()
        .find(|entry| entry.operation == request.operation)
        .ok_or(Code::NotFound)?;
    if schema(&bootstrap, request.operation)? != request.schema {
        return Err(Code::StaleDescription.into());
    }
    if requirement.effect != runtime::Effect::Read {
        return Err(Code::Unsupported.into());
    }
    if request.input.len() > runtime::INPUT_LIMIT {
        return Err(Code::InvalidInput.into());
    }
    let input = connectors_core::json::decode(request.input.as_bytes(), 64)
        .map_err(|_| Code::InvalidInput)?;
    connectors_sdk::validate(&operation.input_schema, &input).map_err(|_| Code::InvalidInput)?;
    Ok(ReadPlan {
        adapter: adapter.clone(),
        fingerprint: connectors_core::digest(&json!({"config":config,"bootstrap":bootstrap})),
    })
}

/// The provider closure is the existing serialized owner worker. A read has no
/// business retry here, and a final-audit failure cannot erase a known result.
pub(super) fn read(
    context: Context<'_>,
    alias: &str,
    request: &Request<'_>,
    selection: Selection<'_>,
    dispatch: impl FnOnce(&Adapter) -> Result<Vec<u8>>,
) -> Result<Value> {
    let audits = audit::Store::new(&context.0.state, 100_000).map_err(|_| Code::Unavailable)?;
    read_using(context, alias, request, &audits, selection, dispatch)
}

#[cfg(test)]
fn read_with_audit(
    paths: &Paths,
    host: &str,
    alias: &str,
    request: &Request<'_>,
    until: Instant,
    audits: &audit::Store,
    dispatch: impl FnOnce(&Adapter) -> Result<Vec<u8>>,
) -> Result<Value> {
    read_using((paths, host, until), alias, request, audits, None, dispatch)
}

fn read_using(
    (paths, host, until): Context<'_>,
    alias: &str,
    request: &Request<'_>,
    audits: &audit::Store,
    selection: Selection<'_>,
    dispatch: impl FnOnce(&Adapter) -> Result<Vec<u8>>,
) -> Result<Value> {
    approval_issuance::check(until)?;
    let request_id = Uuid::new_v4().to_string();
    let plan = resolve_selected(paths, alias, request, selection);
    // A denied operation may still have a verified configured instance. Never
    // use a submitted alias as an audit namespace or manufacture a registry row.
    let instance = plan
        .as_ref()
        .ok()
        .map(|p| p.adapter.instance_id.clone())
        .or_else(|| {
            Config::load(&paths.config)
                .ok()?
                .adapters
                .get(alias)
                .map(|a| a.instance_id.clone())
        })
        .unwrap_or_else(|| host.to_owned());
    let anchor = audit::Anchor {
        instance_id: instance,
        kind: if plan.is_ok() {
            audit::Kind::AdmittedExecution
        } else {
            audit::Kind::EarlyRefusal
        },
        activity: Some(audit::Activity::Invoke),
        hop: audit::Hop::Execution,
        stage: audit::Stage::Admission,
        request_id: Some(request_id.clone()),
        principal_ref: Some(format!("uid:{}", filesystem::uid())),
        operation_id: plan.as_ref().ok().map(|_| request.operation.to_owned()),
        // Connection readiness and custody are execution preflight, not metadata
        // facts. Do not assert a verified persistent relation before that check.
        connection_ref: None,
        descriptor_revision: plan.as_ref().ok().map(|_| request.revision.to_owned()),
        recorded_at_ms: now()?,
        attempt_id: None,
    };
    let acknowledged = match audits.anchor(&anchor) {
        Ok(acknowledged) => acknowledged,
        Err(_) => {
            return response(
                &request_id,
                Err(Code::Unavailable.into()),
                None,
                "unavailable",
            );
        }
    };
    let (reference, outcome) = match (plan, acknowledged) {
        (Ok(plan), audit::Acknowledgement::Execution(receipt)) => {
            let reference = receipt.reference().clone();
            if audits.confirm(receipt, &anchor).is_err() {
                return response(
                    &request_id,
                    Err(Code::Unavailable.into()),
                    Some(&reference),
                    "incomplete",
                );
            }
            let outcome = (|| {
                approval_issuance::check(until)?;
                let current = resolve_selected(paths, alias, request, selection)?;
                if current.fingerprint != plan.fingerprint {
                    return Err(Code::StaleDescription.into());
                }
                let bytes = dispatch(&current.adapter)?;
                if bytes.len() > runtime::RESULT_LIMIT {
                    return Err(Code::Unavailable.into());
                }
                connectors_core::json::decode(&bytes, 64).map_err(|_| {
                    let mut error: Error = Code::ServiceFailure.into();
                    error.service_code = Some(connectors_core::ErrorCode::UpstreamProtocol);
                    error
                })
            })();
            (reference, outcome)
        }
        (Err(error), audit::Acknowledgement::Refusal(reference)) => (reference, Err(error)),
        // A receipt for a different anchor kind is never dispatch authority.
        _ => return Err(Code::Unavailable.into()),
    };
    // Budget the complete envelope before recording its outcome. An oversized
    // result becomes an explicit capacity error, never a truncated JSON value.
    let outcome = if response(&request_id, outcome.clone(), Some(&reference), "incomplete").is_err()
    {
        Err(Code::Capacity.into())
    } else {
        outcome
    };
    let Ok(recorded_at_ms) = now() else {
        return response(&request_id, outcome, Some(&reference), "incomplete");
    };
    let observation = audit::FinalObservation {
        observation_id: Uuid::new_v4(),
        outcome: match &outcome {
            Ok(_) => audit::Outcome::Success,
            Err(error) if error.code == Code::OutcomeUnknown => audit::Outcome::Unknown,
            Err(_) if anchor.kind == audit::Kind::EarlyRefusal => audit::Outcome::Refused,
            Err(_) => audit::Outcome::Error,
        },
        code: outcome.as_ref().err().map(|error| code(error).to_owned()),
        recorded_at_ms,
    };
    let status = if audits
        .append_recovering_with_now(&reference, &observation, until, Instant::now)
        .is_ok()
    {
        "complete"
    } else {
        "incomplete"
    };
    response(&request_id, outcome, Some(&reference), status)
}

fn now() -> Result<i64> {
    i64::try_from(connectors_sdk::now_ms()).map_err(|_| Code::Unavailable.into())
}

fn response(
    request_id: &str,
    outcome: Result<Value>,
    reference: Option<&audit::Reference>,
    status: &str,
) -> Result<Value> {
    response_optional(Some(request_id), outcome, reference, status)
}

fn response_optional(
    request_id: Option<&str>,
    outcome: Result<Value>,
    reference: Option<&audit::Reference>,
    status: &str,
) -> Result<Value> {
    let mut response = json!({"version":"v1alpha2", "request_id":request_id,
        "audit_ref":reference.map(|r| &r.audit_ref), "audit_status":status});
    match outcome {
        Ok(value) => {
            response["status"] = json!("success");
            response["result"] = value;
        }
        Err(error) => {
            response["status"] = json!("error");
            response["error"] = json!({"code":code(&error),"message":"Connectors request refused"});
        }
    }
    if serde_json::to_vec(&response)
        .map_err(|_| Code::Unavailable)?
        .len()
        > runtime::RESULT_LIMIT
    {
        return Err(Code::Capacity.into());
    }
    Ok(response)
}

fn code(error: &Error) -> &'static str {
    use connectors_core::ErrorCode as Service;
    if let Some(code) = &error.service_code {
        return match code {
            Service::InvalidInput => "invalid_input",
            Service::Unsupported => "unsupported",
            Service::Unauthorized => "unauthorized",
            Service::Forbidden => "forbidden",
            Service::NotFound => "not_found",
            Service::StaleDescription => "stale_description",
            Service::StaleCursor => "stale_cursor",
            Service::RateLimited => "rate_limited",
            Service::Unavailable => "unavailable",
            Service::Capacity => "capacity",
            Service::Timeout => "timeout",
            Service::UpstreamProtocol => "upstream_protocol",
            Service::Internal => "internal",
        };
    }
    match error.code {
        Code::InvalidInput => "invalid_input",
        Code::NotGranted => "not_granted",
        Code::ConnectionNotReady => "connection_not_ready",
        Code::InsufficientScope => "insufficient_scope",
        Code::Forbidden => "forbidden",
        Code::NotFound => "not_found",
        Code::StaleDescription => "stale_description",
        Code::StaleCursor => "stale_cursor",
        Code::Unsupported => "unsupported",
        Code::Capacity => "capacity",
        Code::Timeout => "timeout",
        Code::Revoked => "revoked",
        Code::OutcomeUnknown | Code::Interrupted => "outcome_unknown",
        _ => "unavailable",
    }
}

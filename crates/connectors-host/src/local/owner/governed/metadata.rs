//! Protected discovery reads existing cached metadata. It does not acquire a
//! provider, connection, credential, approval or execution grant.
use super::*;

fn snapshot(
    paths: &Paths,
    alias: &str,
    projection: Option<&dyn ReadPolicy>,
) -> Result<(Adapter, Value, String)> {
    if !connectors_core::valid_id(alias) {
        return Err(Code::InvalidInput.into());
    }
    let config = Config::load(&paths.config)?;
    let adapter = config.adapters.get(alias).ok_or(Code::NotGranted)?;
    let mut bootstrap = runtime::state::State::new(&paths.state)
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
    let revision = projection
        .map(|policy| policy.metadata_revision(paths, alias, adapter, &bootstrap))
        .transpose()?;
    if revision
        .as_deref()
        .is_some_and(|value| !connectors_core::valid_id(value))
    {
        return Err(Code::Unavailable.into());
    }
    let fingerprint = connectors_core::digest(
        &json!({"config":config,"bootstrap":bootstrap,"projection_revision":revision}),
    );
    let mut descriptor = bootstrap.descriptor()?;
    descriptor
        .operations
        .retain(|operation| adapter.permissions.operations.contains(&operation.id));
    bootstrap.requirements.retain(|requirement| {
        adapter
            .permissions
            .operations
            .contains(&requirement.operation)
    });
    bootstrap.descriptor = serde_json::to_string(&descriptor).map_err(|_| Code::Unavailable)?;
    // This carrier stays on the authenticated private owner socket. Native
    // composition must explicitly project public fields rather than expose it.
    let value = match revision {
        Some(revision) => json!({"bootstrap":bootstrap,"projection_revision":revision}),
        None => serde_json::to_value(bootstrap).map_err(|_| Code::Unavailable)?,
    };
    Ok((adapter.clone(), value, fingerprint))
}

pub(in crate::local::owner) fn describe(
    (paths, host, until): Context<'_>,
    alias: &str,
    projection: Option<&dyn ReadPolicy>,
) -> Result<Value> {
    let audits = audit::Store::new(&paths.state, 100_000).map_err(|_| Code::Unavailable)?;
    describe_using((paths, host, until), alias, &audits, projection, || {})
}

pub(super) fn describe_using(
    (paths, host, until): Context<'_>,
    alias: &str,
    audits: &audit::Store,
    projection: Option<&dyn ReadPolicy>,
    after_admission: impl FnOnce(),
) -> Result<Value> {
    approval_issuance::check(until)?;
    let selected = snapshot(paths, alias, projection);
    let instance = selected
        .as_ref()
        .ok()
        .map(|(adapter, _, _)| adapter.instance_id.clone())
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
        kind: if selected.is_ok() {
            audit::Kind::AdmittedExecution
        } else {
            audit::Kind::EarlyRefusal
        },
        activity: Some(audit::Activity::Describe),
        hop: audit::Hop::Execution,
        stage: audit::Stage::Admission,
        request_id: None,
        principal_ref: Some(format!("uid:{}", filesystem::uid())),
        operation_id: None,
        connection_ref: None,
        descriptor_revision: None,
        recorded_at_ms: now()?,
        attempt_id: None,
    };
    let acknowledged = match audits.anchor(&anchor) {
        Ok(value) => value,
        Err(_) => {
            return response_optional(None, Err(Code::Unavailable.into()), None, "unavailable");
        }
    };
    let (reference, outcome) = match (selected, acknowledged) {
        (Ok((_, _, fingerprint)), audit::Acknowledgement::Execution(receipt)) => {
            let reference = receipt.reference().clone();
            if audits.confirm(receipt, &anchor).is_err() {
                return response_optional(
                    None,
                    Err(Code::Unavailable.into()),
                    Some(&reference),
                    "incomplete",
                );
            }
            let outcome = (|| {
                after_admission();
                approval_issuance::check(until)?;
                let (_, value, current) = snapshot(paths, alias, projection)?;
                if current != fingerprint {
                    return Err(Code::StaleDescription.into());
                }
                Ok(value)
            })();
            (reference, outcome)
        }
        (Err(error), audit::Acknowledgement::Refusal(reference)) => (reference, Err(error)),
        _ => return Err(Code::Unavailable.into()),
    };
    let outcome =
        if response_optional(None, outcome.clone(), Some(&reference), "incomplete").is_err() {
            Err(Code::Capacity.into())
        } else {
            outcome
        };
    let Ok(recorded_at_ms) = now() else {
        return response_optional(None, outcome, Some(&reference), "incomplete");
    };
    let observation = audit::FinalObservation {
        observation_id: Uuid::new_v4(),
        outcome: match &outcome {
            Ok(_) => audit::Outcome::Success,
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
    response_optional(None, outcome, Some(&reference), status)
}

use super::*;
use connectors_host::local::{
    config::Adapter,
    registry::{self, ObservedConnection, Registry},
};

pub(super) fn execute(
    call: &Invocation<'_>,
    paths: &Paths,
    alias: &str,
    adapter: &Adapter,
    socket: Option<&std::path::Path>,
) -> Result<Value, HandlerReply> {
    let registry = Registry::with_system_clock(&paths.state);
    let text = |field| {
        call.input[field]
            .as_str()
            .ok_or_else(|| failure("invalid_input", "arguments", "none", true))
    };
    // Revocation and acquisition status have no keyring dependency. Observation
    // of connections checks only the existing custody owner's qualified storage,
    // never starts it, opens a secret session, or resolves credential material.
    match call.callable {
        "connections-revalidate" => {
            let deadline = connectors_sdk::now_ms() + 30_000;
            let reference = text("connection")?;
            let revision = text("expected_revision")?;
            owner::admit_revalidation(paths, alias, reference, revision).map_err(owner_failure)?;
            owner::Client::connect(paths, true)
                .and_then(|client| client.revalidate(alias, reference, revision, deadline))
                .map_err(owner_failure)
        }
        "connections-list" => {
            let limit = call
                .input
                .get("limit")
                .and_then(Value::as_i64)
                .unwrap_or(100);
            if !(1..=500).contains(&limit) {
                return Err(failure("invalid_input", "arguments", "none", true));
            }
            let custody = keyring::custody::available_at(socket);
            let page = registry
                .list(
                    &adapter.instance_id,
                    &adapter.adapter_id,
                    &adapter.configuration_revision,
                    registry::PageOptions {
                        limit: limit as u16,
                        cursor: call.input.get("cursor").and_then(Value::as_str),
                    },
                    connectors_sdk::now_ms(),
                    custody,
                )
                .map_err(registry_failure)?;
            let mut result = json!({"connections":page.connections.iter().map(|c| summary(alias,c)).collect::<Vec<_>>(),
                "source":"authority", "stale":page.stale, "observed_at_ms":page.observed_at_ms,"valid_until_ms":page.valid_until_ms});
            if let Some(cursor) = page.next_cursor {
                result["next_cursor"] = json!(cursor);
            }
            Ok(result)
        }
        "connections-describe" | "connections-status" => {
            let reference = call.input.get("connection").and_then(Value::as_str);
            let acquisition = call.input.get("acquisition").and_then(Value::as_str);
            match (reference, acquisition) {
                (Some(reference), None) => {
                    let custody = keyring::custody::available_at(socket);
                    let observed = registry
                        .describe(
                            &adapter.instance_id,
                            &adapter.adapter_id,
                            &adapter.configuration_revision,
                            reference,
                            connectors_sdk::now_ms(),
                            custody,
                        )
                        .map_err(registry_failure)?;
                    Ok(json!({"connection":description(alias,&observed)}))
                }
                (None, Some(reference)) if call.callable == "connections-status" => {
                    let observed = registry
                        .acquisition_status(
                            &adapter.instance_id,
                            &adapter.adapter_id,
                            reference,
                            connectors_sdk::now_ms(),
                        )
                        .map_err(registry_failure)?;
                    let mut value = json!({"instance_id":observed.instance,"acquisition":observed.reference,"state":observed.state,
                        "expires_at_ms":observed.expires_at_ms,"observed_at_ms":observed.observed_at_ms,
                        "next_action":match observed.state {"pending" => "retry_status", "failed" => "retry_explicitly", _ => "none"}});
                    if let Some(reason) = observed.reason {
                        value["reason"] = json!(reason);
                    }
                    if let Some(connection) = observed.connection {
                        value["connection"] = json!(connection);
                    }
                    Ok(json!({"acquisition":value}))
                }
                _ => Err(failure("invalid_input", "arguments", "none", true)),
            }
        }
        "connections-revoke" => {
            let reference = text("connection")?;
            let revision = registry
                .revoke(
                    &adapter.instance_id,
                    &adapter.adapter_id,
                    reference,
                    text("expected_revision")?,
                    connectors_sdk::now_ms(),
                )
                .map_err(registry_failure)?;
            Ok(
                json!({"adapter":alias,"connection":reference,"revision":revision,"local_state":"revoked","provider_outcome":"not_requested"}),
            )
        }
        _ => Err(failure("unsupported", "arguments", "none", true)),
    }
}

fn summary(alias: &str, connection: &ObservedConnection) -> Value {
    json!({"adapter":alias,"instance_id":connection.instance,"connection":connection.reference,"profile":connection.profile,
        "revision":connection.revision,"state":connection.state})
}
fn description(alias: &str, connection: &ObservedConnection) -> Value {
    let mut value = json!({"summary":summary(alias,connection),"observed_at_ms":connection.observed_at_ms,
        "valid_until_ms":connection.valid_until_ms,"source":"authority","stale":connection.stale});
    if let Some(identity) = &connection.identity {
        value["external_identity"] = json!(identity);
    }
    value
}
fn registry_failure(error: registry::Failure) -> HandlerReply {
    use registry::Failure::*;
    let (code, stage, action, usage) = match error {
        MetadataUnavailable => ("metadata_unavailable", "observation", "retry_status", false),
        OutcomeUnknown => ("outcome_unknown", "publication", "retry_status", false),
        NotFound => ("not_found", "observation", "check_configuration", false),
        Conflict => ("lifecycle_conflict", "admission", "retry_status", false),
        Revoked => ("revoked", "admission", "create_connection", false),
        IdentityMismatch => ("identity_mismatch", "admission", "repair_connection", false),
        Expired => ("timeout", "admission", "retry_explicitly", false),
        InvalidInput => ("invalid_input", "arguments", "none", true),
        StaleCursor => ("stale_cursor", "observation", "retry_explicitly", false),
        Capacity => ("capacity", "admission", "retry_explicitly", false),
        NotReady => ("unavailable", "readiness", "repair_connection", false),
        InsufficientScope => ("not_granted", "admission", "repair_connection", false),
        CustodyUnavailable => ("custody_unavailable", "custody", "unlock_keyring", false),
    };
    failure(code, stage, action, usage)
}

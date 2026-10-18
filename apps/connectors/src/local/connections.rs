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
    selection: &Selection,
    configured: &std::collections::BTreeSet<String>,
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
                .map_err(owner_failure)?
                .revalidate(alias, reference, revision, deadline)
                .map_err(|error| {
                    // The class of a transient failure the owner retried, and
                    // how many validations it sent (semantics.md §6
                    // "Revalidation retries").
                    let (cause, attempts) = (error.cause, error.attempts);
                    let mut reply = match error.code {
                        // The owner's own definite answer to a dispatched
                        // revalidation; a lost reply is already `outcome_unknown`.
                        owner::Code::Unavailable | owner::Code::Timeout => {
                            let code = serde_json::to_value(error.code).unwrap_or_default();
                            let code = code.as_str().unwrap_or("unavailable").to_owned();
                            failure(&code, "dispatch", "retry_explicitly", false)
                        }
                        _ => owner_failure(error),
                    };
                    if let (Some(cause), HandlerReply::Error { data, .. }) = (cause, &mut reply)
                        && data["stage"] == "dispatch"
                    {
                        data["cause"] = json!(cause);
                        if let Some(attempts) = attempts {
                            data["attempts"] = json!(attempts);
                        }
                    }
                    reply
                })
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
            let custody = keyring::custody::available_for(selection);
            let page = registry
                .list(
                    &adapter.instance_id,
                    &adapter.adapter_id,
                    adapter.configured(),
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
                    let custody = keyring::custody::available_for(selection);
                    let observed = registry
                        .describe(
                            &adapter.instance_id,
                            &adapter.adapter_id,
                            adapter.configured(),
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
            // A connection left under an instance id no entry configures any
            // longer (the entry moved to a new one) stays revocable here.
            let revision = registry
                .revoke_left_behind(
                    &adapter.instance_id,
                    &adapter.adapter_id,
                    configured,
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

/// `connections launch`: admit here, before any owner starts; then the owner
/// reads the credential and hands this process the captured consumer image and
/// a sealed copy of the credential, which it passes to the consumer as
/// descriptor 3 without reading it. Returns the consumer's exit code.
pub(super) fn launch(call: &Invocation<'_>) -> Result<i32, HandlerReply> {
    let paths = Paths::resolve(
        call.context.config.as_deref(),
        call.context.state_dir.as_deref(),
    )
    .map_err(host_failure)?;
    Config::read(&paths.config).map_err(configuration_refusal)?;
    let text = |field| {
        call.input[field]
            .as_str()
            .ok_or_else(|| failure("invalid_input", "arguments", "none", true))
    };
    let (alias, connection, consumer) = (text("adapter")?, text("connection")?, text("consumer")?);
    // `--args` is one JSON array of strings, appended verbatim after the
    // pinned argv prefix. Anything else, or an element holding NUL, is usage.
    let extra = match call.input.get("args").and_then(Value::as_str) {
        None => Vec::new(),
        Some(value) => serde_json::from_str::<Vec<String>>(value)
            .ok()
            .filter(|args| !args.iter().any(|arg| arg.contains('\0')))
            .ok_or_else(|| failure("invalid_input", "arguments", "none", true))?,
    };
    // No owner is contacted by the preflight, so its `unavailable` is only the
    // connection's readiness, reported as the connection's own observation does.
    owner::admit_launch(&paths, alias, connection, consumer).map_err(|error| {
        if error.code == owner::Code::Unavailable {
            registry_failure(registry::Failure::NotReady)
        } else {
            owner_failure(error)
        }
    })?;
    let deadline = connectors_sdk::now_ms() + 30_000;
    let consumer = owner::Client::connect(&paths, true)
        .and_then(|client| client.launch(alias, connection, consumer, deadline))
        .map_err(owner_failure)?;
    // The consumer's entry selects which of this caller's variables pass.
    consumer
        .run(&extra, std::env::vars_os())
        .map_err(|error| owner_failure(owner::Error::from(error)))
}

/// The commands that select one existing connection and take `--connection`
/// optionally. `connections reconnect` selects it before any handler runs
/// (`held_one`).
const SELECTING: [&str; 4] = [
    "connections-describe",
    "connections-revalidate",
    "connections-revoke",
    "connections-launch",
];

/// story:connect-refused-while-instance-holds-connection: `--connection` may be
/// left out where the selected instance holds exactly one published
/// non-revoked connection, which the command then selects. With none it is
/// `not_found` at admission naming a connect; with more than one,
/// `invalid_input` at arguments: `--connection` chooses. Runs on the input
/// every handler reads, once the selectors named the alias. A configuration or
/// alias that does not resolve is left to the handler's own refusal.
pub(super) fn select(call: &Invocation<'_>, input: &mut Value) -> Result<(), HandlerReply> {
    if !SELECTING.contains(&call.callable) || input.get("connection").is_some_and(|v| !v.is_null())
    {
        return Ok(());
    }
    let Some(alias) = input.get("adapter").and_then(Value::as_str) else {
        return Ok(());
    };
    let Ok(paths) = resolved(call) else {
        return Ok(());
    };
    let Ok(config) = Config::read(&paths.config) else {
        return Ok(());
    };
    let Some(adapter) = config.adapters.get(alias) else {
        return Ok(());
    };
    let held = Registry::with_system_clock(&paths.state)
        .held(&adapter.instance_id, connectors_sdk::now_ms())
        .map_err(registry_failure)?;
    match held.as_slice() {
        [one] => {
            input["connection"] = json!(one);
            Ok(())
        }
        [] => Err(failure(
            "not_found",
            "admission",
            "create_connection",
            false,
        )),
        _ => Err(failure("invalid_input", "arguments", "none", true)),
    }
}

/// `select` for `connections reconnect`, whose protected entry is admitted
/// before any handler runs: the alias's one published non-revoked connection,
/// refused as `select` refuses.
pub(super) fn held_one(paths: &Paths, alias: &str) -> owner::Result<String> {
    let config = Config::load(&paths.config)?;
    let adapter = config.adapters.get(alias).ok_or(owner::Code::NotFound)?;
    let held = Registry::with_system_clock(&paths.state)
        .held(&adapter.instance_id, connectors_sdk::now_ms())?;
    match held.as_slice() {
        [one] => Ok(one.clone()),
        [] => {
            // Nothing to reconnect: a connect makes the instance's connection.
            let mut error = owner::Error::from(owner::Code::NotFound);
            error.reconnect = true;
            Err(error)
        }
        _ => Err(owner::Code::InvalidInput.into()),
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
pub(super) fn registry_failure(error: registry::Failure) -> HandlerReply {
    use registry::Failure::*;
    if let MetadataSchemaNewer(store) = error {
        return schema_newer(store.map(|release| release.to_string()));
    }
    let (code, stage, action, usage) = match error {
        MetadataUnavailable => ("metadata_unavailable", "observation", "retry_status", false),
        MetadataSchemaNewer(_) => ("metadata_schema_newer", "admission", "upgrade", false),
        ConcurrentRevision => (
            "revision_conflict",
            "publication",
            "retry_explicitly",
            false,
        ),
        OutcomeUnknown => ("outcome_unknown", "publication", "retry_status", false),
        NotFound => ("not_found", "observation", "check_configuration", false),
        Conflict => ("lifecycle_conflict", "admission", "retry_status", false),
        UpgradeRequired | ExecutableChanged => (
            "lifecycle_conflict",
            "admission",
            "revalidate_connection",
            false,
        ),
        BindingChanged => (
            "lifecycle_conflict",
            "admission",
            "create_connection",
            false,
        ),
        AuthenticationChanged => (
            "lifecycle_conflict",
            "admission",
            "check_configuration",
            false,
        ),
        // The instance holds its connection: reconnect replaces its credential.
        InstanceConnected => (
            "lifecycle_conflict",
            "admission",
            "repair_connection",
            false,
        ),
        // What it holds no reconnect replaces: revoke it, then connect.
        InstanceConnectionChanged => (
            "lifecycle_conflict",
            "admission",
            "revoke_connection",
            false,
        ),
        Revoked => ("revoked", "admission", "create_connection", false),
        IdentityMismatch => ("identity_mismatch", "admission", "repair_connection", false),
        UpgradeIdentityMismatch => ("identity_mismatch", "admission", "create_connection", false),
        Expired => ("timeout", "admission", "retry_explicitly", false),
        InvalidInput => ("invalid_input", "arguments", "none", true),
        StaleCursor => ("stale_cursor", "observation", "retry_explicitly", false),
        Capacity => ("capacity", "admission", "retry_explicitly", false),
        NotReady => ("unavailable", "readiness", "repair_connection", false),
        EvidenceExpired => ("not_granted", "admission", "revalidate_connection", false),
        InsufficientScope => ("not_granted", "admission", "repair_connection", false),
        CustodyUnavailable => ("custody_unavailable", "custody", "unlock_keyring", false),
        CustodyReadOnly => ("custody_read_only", "custody", "check_configuration", false),
    };
    failure(code, stage, action, usage)
}

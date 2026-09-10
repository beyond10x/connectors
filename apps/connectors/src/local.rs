use connectors_cli_contract::{
    AcquireError, Handler, HandlerReply, Invocation, ProtectedSource, Sources,
};
use connectors_host::local::{
    Failure,
    config::{Config, Paths},
    keyring,
    metadata::Metadata,
};
use serde_json::{Value, json};
use std::ffi::OsString;
mod connections;

/// Protected capture must follow current owner/profile/target admission. Until
/// the private adapter/bootstrap admission is bound, refuse BEFORE reading a source.
/// In particular, never fall back to the generated package's demonstration
/// OsSources implementation or consume stdin while reporting unavailability.
struct AdmittedSources;
impl Sources for AdmittedSources {
    fn acquire(&mut self, _: ProtectedSource) -> Result<String, AcquireError> {
        Err(AcquireError::Unavailable)
    }
}

pub fn run(args: Vec<OsString>) -> connectors_cli_contract::ProcessOutput {
    let root_help = args.len() == 2 && matches!(args[1].to_str(), Some("--help" | "-h"));
    let mut output =
        connectors_cli_contract::run(args, &mut AdmittedSources, &mut LocalHandler, None);
    if root_help && output.exit_code == 0 {
        output.stdout.push_str("\nExplicit service commands (use COMMAND --help for options):\n  describe  Read a complete service descriptor\n  invoke    Invoke an explicit service operation\n  serve     Run the configured federation service\n");
    }
    output
}

struct LocalHandler;
impl Handler for LocalHandler {
    fn call(&mut self, call: &Invocation<'_>) -> HandlerReply {
        match execute(call) {
            Ok(value) => HandlerReply::Success(value),
            Err(failure) => failure,
        }
    }
}

fn failure(code: &str, stage: &str, next_action: &str, usage: bool) -> HandlerReply {
    let data = json!({"kind": if usage { "usage" } else { "operational" }, "code":code, "stage":stage, "next_action":next_action});
    if usage {
        HandlerReply::UsageError {
            code: "failure".into(),
            data,
        }
    } else {
        HandlerReply::Error {
            code: "failure".into(),
            data,
        }
    }
}

fn host_failure(error: Failure) -> HandlerReply {
    match error {
        Failure::InvalidConfiguration => failure(
            "invalid_configuration",
            "configuration",
            "check_configuration",
            true,
        ),
        Failure::ConfigurationExists => failure(
            "configuration_exists",
            "configuration",
            "check_configuration",
            false,
        ),
        Failure::MetadataUnavailable => {
            failure("metadata_unavailable", "observation", "retry_status", false)
        }
        Failure::OutcomeUnknown => failure("outcome_unknown", "publication", "retry_status", false),
    }
}

fn execute(call: &Invocation<'_>) -> Result<Value, HandlerReply> {
    let paths = Paths::resolve(
        call.context.config.as_deref(),
        call.context.state_dir.as_deref(),
    )
    .map_err(host_failure)?;
    if call.callable == "setup-init" {
        Config::initialize(&paths).map_err(host_failure)?;
        return Ok(
            json!({"disposition":"created", "config_path":paths.config, "state_path":paths.state, "os":"linux"}),
        );
    }
    let config = Config::load(&paths.config).map_err(host_failure)?;
    let summary = |alias: &str, adapter: &connectors_host::local::config::Adapter| json!({"adapter":alias,"instance_id":adapter.instance_id,"adapter_id":adapter.adapter_id,"startup":adapter.startup,"restart":adapter.restart});
    if call.callable == "setup-check" {
        let state = keyring::inspect();
        let mut checks =
            vec![json!({"name":"configuration", "state":"ready", "next_action":"none"})];
        checks.push(json!({"name":"metadata", "state": if Metadata::inspect(&paths.state).is_ok() {"ready"} else {"failed"}, "next_action":"check_configuration"}));
        for (alias, adapter) in &config.adapters {
            let ready = adapter.executable.check().is_ok();
            checks.push(json!({"name":format!("artifact:{alias}"), "state":if ready {"ready"} else {"failed"}, "next_action":if ready {"none"} else {"check_configuration"}}));
        }
        checks.push(json!({"name":"keyring", "state": if state == keyring::State::Available {"ready"} else {"failed"}, "next_action":if state == keyring::State::Available {"none"} else {"unlock_keyring"}}));
        // Availability alone is weaker than the qualified implementation and
        // storage binding with durable-write, restart and retirement evidence.
        let qualified = state == keyring::State::Available && keyring::custody::available();
        checks.push(json!({"name":"persistent_custody_qualification", "state":if qualified {"ready"} else {"failed"}, "next_action":"none"}));
        return Ok(
            json!({"config_path":paths.config,"state_path":paths.state,"os":"linux", "keyring":match state {keyring::State::Available => "available",keyring::State::Locked => "locked",keyring::State::Unavailable => "unavailable"},"prerequisites":checks}),
        );
    }
    if call.callable == "adapters-list" {
        if call
            .input
            .get("cursor")
            .is_some_and(|value| !value.is_null())
        {
            return Err(failure(
                "stale_cursor",
                "observation",
                "retry_explicitly",
                false,
            ));
        }
        let limit = call
            .input
            .get("limit")
            .and_then(Value::as_i64)
            .unwrap_or(100);
        if !(1..=500).contains(&limit) {
            return Err(failure("invalid_input", "arguments", "none", true));
        }
        if config.adapters.len() > limit as usize {
            // No cursor owner yet. Refuse an incomplete page rather than claim
            // exhaustion or issue a forgeable/unrecoverable continuation.
            return Err(failure(
                "capacity",
                "observation",
                "retry_explicitly",
                false,
            ));
        }
        return Ok(
            json!({"adapters":config.adapters.iter().map(|(alias, entry)| summary(alias,entry)).collect::<Vec<_>>(),"source":"configuration"}),
        );
    }
    let alias = call.input["adapter"]
        .as_str()
        .ok_or_else(|| failure("invalid_input", "arguments", "none", true))?;
    let adapter = config
        .adapters
        .get(alias)
        .ok_or_else(|| failure("not_found", "configuration", "check_configuration", false))?;
    if matches!(
        call.callable,
        "connections-list" | "connections-describe" | "connections-status" | "connections-revoke"
    ) {
        return connections::execute(call, &paths, alias, adapter);
    }
    match call.callable {
        "adapters-describe" => Ok(
            json!({"configuration":{"summary":summary(alias,adapter),"configuration_revision":adapter.configuration_revision,"protocol":adapter.protocol,"executable":adapter.executable},"source":"configuration","stale":true}),
        ),
        "adapters-status" => Ok(
            json!({"observation":{"adapter":alias,"configuration_revision":adapter.configuration_revision,"state":"owner_unavailable","observed_at_ms":connectors_sdk::now_ms()}}),
        ),
        "operations-list" | "operations-describe" => Err(failure(
            "description_unavailable",
            "observation",
            "refresh_description",
            false,
        )),
        "adapters-stop" => Err(failure("unavailable", "stop", "retry_status", false)),
        // Protected acquisition and business dispatch still require the private
        // adapter/lifecycle binding. Safe metadata cannot substitute for it.
        _ => Err(failure(
            "metadata_unavailable",
            "observation",
            "retry_status",
            false,
        )),
    }
}

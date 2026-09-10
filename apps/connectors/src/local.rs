use connectors_cli_contract::{
    AcquireError, Handler, HandlerReply, Invocation, ProtectedSource, Sources,
};
use connectors_host::local::{
    Failure,
    config::{Config, Paths},
    keyring,
    metadata::Metadata,
    owner,
};
use serde_json::{Value, json};
use std::ffi::OsString;
mod connections;
mod operations;
mod session;

pub fn run(args: Vec<OsString>) -> connectors_cli_contract::ProcessOutput {
    let root_help = args.len() == 2 && matches!(args[1].to_str(), Some("--help" | "-h"));
    let session = session::Session::new(&args);
    let mut output = connectors_cli_contract::run(
        args,
        &mut session::AdmittedSources(session.clone()),
        &mut LocalHandler(session.clone()),
        Some(&mut session::NativeValidator(session.clone())),
    );
    session.borrow_mut().finish(&mut output);
    if root_help && output.exit_code == 0 {
        output.stdout.push_str("\nExplicit service commands (use COMMAND --help for options):\n  describe  Read a complete service descriptor\n  invoke    Invoke an explicit service operation\n  serve     Run the configured federation service\n");
    }
    output
}

struct LocalHandler(session::Shared);
impl Handler for LocalHandler {
    fn call(&mut self, call: &Invocation<'_>) -> HandlerReply {
        let result = if matches!(call.callable, "connections-connect" | "connections-repair") {
            self.0.borrow_mut().complete(call).map_err(owner_failure)
        } else if call.callable == "operations-invoke" {
            operations::invoke(call, self.0.borrow().deadline).map_err(owner_failure)
        } else {
            execute(call)
        };
        match result {
            Ok(value) => HandlerReply::Success(value),
            Err(failure) => {
                if matches!(&failure, HandlerReply::Error { data, .. } if data["kind"] == "interrupted")
                {
                    self.0.borrow_mut().cancelled = true;
                }
                failure
            }
        }
    }
}

fn owner_failure(error: owner::Error) -> HandlerReply {
    use owner::Code::*;
    let (stage, action, usage) = match error.code {
        InvalidInput => ("arguments", "none", true),
        InvalidConfiguration => ("configuration", "check_configuration", true),
        ProtectedEntryUnavailable => ("protected_entry", "select_protected_source", true),
        MetadataUnavailable => ("observation", "retry_status", false),
        CustodyUnavailable => ("custody", "unlock_keyring", false),
        OutcomeUnknown => ("publication", "retry_status", false),
        Forbidden => ("admission", "request_permission", false),
        NotFound => ("admission", "check_configuration", false),
        LifecycleConflict => ("admission", "retry_status", false),
        Revoked => ("admission", "create_connection", false),
        NotGranted | IdentityMismatch => ("admission", "repair_connection", false),
        Unavailable => ("readiness", "retry_status", false),
        Timeout | Capacity => ("admission", "retry_explicitly", false),
        StaleCursor => ("observation", "retry_explicitly", false),
        Interrupted => ("protected_entry", "retry_status", false),
        Unsupported => ("admission", "none", false),
        ReadinessMismatch => ("readiness", "check_configuration", false),
        IncarnationMismatch => ("stop", "retry_status", false),
        StaleDescription | DescriptionUnavailable => ("observation", "refresh_description", false),
        ServiceFailure => ("dispatch", "retry_explicitly", false),
    };
    let mut data = json!({"kind":if error.code == Interrupted {"interrupted"} else if usage {"usage"} else {"operational"},"code":error.code,"stage":stage,"next_action":action});
    if let Some(acquisition) = error.acquisition {
        data["acquisition"] = json!(acquisition);
    }
    if let Some(service_code) = error.service_code {
        data["service_code"] = json!(service_code);
    }
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
        let state = keyring::inspect_at(config.secret_service_socket.as_deref());
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
        let qualified = state == keyring::State::Available
            && keyring::custody::available_at(config.secret_service_socket.as_deref());
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
        return connections::execute(
            call,
            &paths,
            alias,
            adapter,
            config.secret_service_socket.as_deref(),
        );
    }
    match call.callable {
        "adapters-describe" => {
            let mut result = json!({"configuration":{"summary":summary(alias,adapter),"configuration_revision":adapter.configuration_revision,"protocol":adapter.protocol,"executable":adapter.executable},"source":"configuration","stale":true});
            match owner::cached(&paths, alias) {
                Ok(bootstrap) => {
                    result["descriptor"] =
                        operations::descriptor(&bootstrap).map_err(owner_failure)?;
                    result["source"] = json!("cached");
                }
                Err(owner::Error {
                    code: owner::Code::DescriptionUnavailable,
                    ..
                }) => {}
                Err(error) => return Err(owner_failure(error)),
            }
            Ok(result)
        }
        "adapters-status" => match owner::Client::connect(&paths, false) {
            Ok(client) => client.status(alias).map_err(owner_failure),
            Err(owner::Error {
                code: owner::Code::Unavailable,
                ..
            }) => Ok(
                json!({"observation":{"adapter":alias,"configuration_revision":adapter.configuration_revision,"state":"owner_unavailable","observed_at_ms":connectors_sdk::now_ms()}}),
            ),
            Err(error) => Err(owner_failure(error)),
        },
        "operations-list" | "operations-describe" => {
            operations::describe(call, &paths, alias, adapter).map_err(owner_failure)
        }
        "adapters-stop" => {
            let text = |field| {
                call.input[field]
                    .as_str()
                    .ok_or_else(|| failure("invalid_input", "arguments", "none", true))
            };
            owner::Client::connect(&paths, false)
                .map_err(owner_failure)?
                .stop(
                    alias,
                    text("expected_revision")?,
                    text("host_incarnation")?,
                    text("child_incarnation")?,
                )
                .map_err(owner_failure)
        }
        _ => Err(failure(
            "metadata_unavailable",
            "observation",
            "retry_status",
            false,
        )),
    }
}

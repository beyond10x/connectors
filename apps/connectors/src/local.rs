use connectors_cli_contract::{
    AcquireError, Handler, HandlerReply, Invocation, ProtectedSource, Sources,
};
use connectors_host::local::{
    Failure,
    config::{Config, Paths, Refusal},
    keyring,
    metadata::Metadata,
    owner,
};
use serde_json::{Value, json};
use std::ffi::OsString;
mod approval_keys;
mod approvals;
mod checkpoints;
mod connections;
mod cursor;
mod operations;
mod session;

pub fn run(args: Vec<OsString>) -> connectors_cli_contract::ProcessOutput {
    let root_help = is_root_help(&args);
    let session = session::Session::new(&args);
    if let Some(output) = configuration_preflight(&session.borrow()) {
        return output;
    }
    let mut output = connectors_cli_contract::run(
        args,
        &mut session::AdmittedSources(session.clone()),
        &mut LocalHandler(session.clone()),
        Some(&mut session::NativeValidator(session.clone())),
    );
    session.borrow_mut().finish(&mut output);
    // A consumer ran: stdout and stderr were its own, and its exit status is
    // the launch's. No envelope follows it.
    if let Some(code) = session.borrow().launched {
        output = connectors_cli_contract::ProcessOutput {
            exit_code: code,
            stdout: String::new(),
            stderr: String::new(),
        };
    }
    if root_help && output.exit_code == 0 {
        output.stdout.push_str("\nExplicit service commands (use COMMAND --help for options):\n  describe  Read a complete service descriptor\n  invoke    Invoke an explicit service operation\n  serve     Run the configured federation service\n");
    }
    output
}

/// Whether the argument list, after any leading process globals, is exactly
/// root `--help` or `-h`.
fn is_root_help(args: &[OsString]) -> bool {
    let mut rest = args.iter().skip(1);
    while let Some(arg) = rest.next() {
        match arg.to_str() {
            Some("--output" | "--config" | "--state-dir") if rest.next().is_some() => {}
            Some(arg)
                if ["--output=", "--config=", "--state-dir="]
                    .iter()
                    .any(|prefix| arg.starts_with(prefix)) => {}
            Some("--help" | "-h") => return rest.next().is_none(),
            _ => return false,
        }
    }
    false
}

struct LocalHandler(session::Shared);
impl Handler for LocalHandler {
    fn call(&mut self, call: &Invocation<'_>) -> HandlerReply {
        let result = if matches!(call.callable, "connections-connect" | "connections-repair") {
            self.0.borrow_mut().complete(call).map_err(owner_failure)
        } else if matches!(
            call.callable,
            "approval-policy-status"
                | "approval-policy-set"
                | "approval-prepare"
                | "approval-issue"
        ) {
            let mut session = self.0.borrow_mut();
            session
                .signals()
                .and_then(|_| approvals::execute(call, session.approval_deadline))
                .map_err(owner_failure)
        } else if call.callable == "operations-invoke" {
            let session = self.0.borrow();
            return operations::dispatch(call, session.deadline, session.mutation_deadline)
                .unwrap_or_else(invoke_failure);
        } else if call.callable == "connections-launch" {
            connections::launch(call).map(|code| {
                self.0.borrow_mut().launched = Some(code);
                json!({"consumer":call.input["consumer"],"exit_code":code})
            })
        } else if call.callable == "connections-revalidate" {
            self.0
                .borrow_mut()
                .signals()
                .map_err(owner_failure)
                .and_then(|_| execute(call))
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

/// An operation invoke runs on an existing connection, so a credential the
/// provider refuses there is the stored one, such as an OAuth refresh token that
/// was revoked: a retry cannot succeed, a repair can. Everywhere else, and a
/// credential refused while a connection is made in particular, keeps
/// `owner_failure`'s next action.
fn invoke_failure(error: owner::Error) -> HandlerReply {
    let stored = error.code == owner::Code::ServiceFailure
        && error.service_code == Some(connectors_core::ErrorCode::Unauthorized);
    let mut reply = owner_failure(error);
    if stored
        && let HandlerReply::Error { data, .. } = &mut reply
        && data["stage"] == "dispatch"
    {
        data["next_action"] = json!("repair_connection");
    }
    reply
}

fn owner_failure(error: owner::Error) -> HandlerReply {
    use owner::Code::*;
    let provider = error.origin == owner::Origin::Provider;
    let (stage, action, usage) = match error.code {
        // A provider's refusal of a dispatched call is not a host admission refusal.
        Forbidden if provider => ("dispatch", "request_permission", false),
        ServiceFailure
            if provider && error.service_code == Some(connectors_core::ErrorCode::NotFound) =>
        {
            ("dispatch", "none", false)
        }
        // A sent request's deadline that passed, or a marked upstream capacity
        // answer; the host's and the adapter's own limits stay admission.
        Timeout | Capacity if provider => ("dispatch", "retry_explicitly", false),
        // A connect's acquisition that failed before publication: the owner
        // recorded it as failed, as `connections status` then reports. A lost
        // reply never gets here; the client reports it as `outcome_unknown`.
        Unavailable | Timeout if error.acquisition.is_some() => {
            ("dispatch", "retry_explicitly", false)
        }
        InvalidInput => ("arguments", "none", true),
        InvalidConfiguration => ("configuration", "check_configuration", true),
        ProtectedEntryUnavailable => ("protected_entry", "select_protected_source", true),
        MetadataUnavailable => ("observation", "retry_status", false),
        // Not committed, so the same command is safe to run again.
        RevisionConflict => ("publication", "retry_explicitly", false),
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
        OwnerBuildMismatch => ("readiness", "stop_owner", false),
    };
    // A refusal during a configuration upgrade, or of a changed binding: repair
    // refuses a changed binding, so only a new connection helps.
    // Only the connection's validation evidence expired, its credential is
    // intact: a revalidation recollects it, a repair would ask for re-entry.
    let action = if error.reconnect {
        "create_connection"
    } else if error.revalidate {
        "revalidate_connection"
    } else {
        action
    };
    let mut data = json!({"kind":if error.code == Interrupted {"interrupted"} else if usage {"usage"} else {"operational"},"code":error.code,"stage":stage,"next_action":action});
    if let Some(acquisition) = error.acquisition {
        data["acquisition"] = json!(acquisition);
    }
    if let Some(service_code) = error.service_code {
        data["service_code"] = json!(service_code);
    }
    if let Some(service_reason) = error.service_reason {
        data["service_reason"] = json!(service_reason);
    }
    if let Some(retry_after_seconds) = error.retry_after_seconds {
        data["retry_after_seconds"] = json!(retry_after_seconds);
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
        Failure::ConcurrentRevision => failure(
            "revision_conflict",
            "publication",
            "retry_explicitly",
            false,
        ),
        Failure::OutcomeUnknown => failure("outcome_unknown", "publication", "retry_status", false),
    }
}

/// Every command but `setup init` and `setup checkpoints-enable` loads the configuration somewhere in this
/// process: in the handler, a protected source, the dynamic validator or an
/// owner helper. Those later sites keep the owner's payload-free failure, so a
/// file refused by a rule that can name its entry is refused here first, before
/// any source is acquired or dispatch begins. Any other outcome, including every
/// other invalid configuration, proceeds exactly as before.
fn configuration_preflight(
    session: &session::Session,
) -> Option<connectors_cli_contract::ProcessOutput> {
    use connectors_cli_contract::OutputMode;
    let (callable, context) = session.selected()?;
    // Neither reads the configuration: `setup init` writes it, and
    // `setup checkpoints-enable` acts on the state directory alone.
    if callable == "setup-init" || callable == "setup-checkpoints-enable" {
        return None;
    }
    let paths = Paths::resolve(context.config.as_deref(), context.state_dir.as_deref()).ok()?;
    let refusal = match Config::read(&paths.config) {
        Err(refusal @ Refusal::PrivateProtocolMismatch { .. }) => refusal,
        _ => return None,
    };
    let HandlerReply::UsageError { code, data } = configuration_refusal(refusal) else {
        return None;
    };
    // Emit only what the generated contract admits for this callable.
    if !connectors_cli_contract::plan()
        .callables
        .get(callable)
        .and_then(|c| c.errors.get(&code))
        .is_some_and(|c| c.shape.accepts(&data))
    {
        return None;
    }
    let stderr = if context.output == OutputMode::Json {
        format!(
            "{}\n",
            json!({"ok":false,"error":{"code":code,"data":data}})
        )
    } else {
        format!("{code}: {data}\n")
    };
    Some(connectors_cli_contract::ProcessOutput {
        exit_code: 2,
        stdout: String::new(),
        stderr,
    })
}

fn configuration_refusal(refusal: Refusal) -> HandlerReply {
    let mut reply = host_failure(refusal.failure());
    if let (
        Refusal::PrivateProtocolMismatch {
            format,
            instance_id,
        },
        HandlerReply::UsageError { data, .. },
    ) = (refusal, &mut reply)
    {
        data["configuration_format"] = json!(format);
        data["instance_id"] = json!(instance_id);
    }
    reply
}

fn execute(call: &Invocation<'_>) -> Result<Value, HandlerReply> {
    let paths = Paths::resolve(
        call.context.config.as_deref(),
        call.context.state_dir.as_deref(),
    )
    .map_err(host_failure)?;
    if call.callable == "setup-checkpoints-enable" {
        return checkpoints::execute(call, &paths);
    }
    if call.callable == "setup-init" {
        let initialized = Config::initialize(&paths).map_err(host_failure)?;
        return Ok(
            json!({"disposition":initialized.disposition(), "config_path":paths.config, "state_path":paths.state, "os":"linux"}),
        );
    }
    let config = Config::read(&paths.config).map_err(configuration_refusal)?;
    let summary = |alias: &str, adapter: &connectors_host::local::config::Adapter| json!({"adapter":alias,"instance_id":adapter.instance_id,"adapter_id":adapter.adapter_id,"startup":adapter.startup,"restart":adapter.restart});
    if call.callable == "setup-check" {
        let state = keyring::inspect_at(config.secret_service_socket.as_deref());
        let mut checks =
            vec![json!({"name":"configuration", "state":"ready", "next_action":"none"})];
        let metadata = Metadata::inspect(&paths.state).is_ok();
        checks.push(json!({"name":"metadata", "state": if metadata {"ready"} else {"failed"}, "next_action":if metadata {"none"} else {"check_configuration"}}));
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
        let refused = |refusal| match refusal {
            cursor::Refusal::InvalidInput => failure("invalid_input", "arguments", "none", true),
            cursor::Refusal::StaleCursor => {
                failure("stale_cursor", "observation", "retry_explicitly", false)
            }
        };
        let request = cursor::Request::parse(&call.input).map_err(refused)?;
        let adapters = config
            .adapters
            .iter()
            .map(|(alias, entry)| summary(alias, entry))
            .collect::<Vec<_>>();
        // The selection a cursor is bound to: every listed entry and the
        // configuration revision it was listed under.
        let source = config
            .adapters
            .iter()
            .zip(&adapters)
            .map(|((_, entry), listed)| json!([entry.configuration_revision, listed]))
            .collect::<Value>();
        let (range, next) = request.page(&source, adapters.len()).map_err(refused)?;
        let mut result = json!({"adapters":adapters[range],"source":"configuration"});
        if let Some(next) = next {
            result["next_cursor"] = json!(next);
        }
        return Ok(result);
    }
    let alias = call.input["adapter"]
        .as_str()
        .ok_or_else(|| failure("invalid_input", "arguments", "none", true))?;
    let adapter = config
        .adapters
        .get(alias)
        .ok_or_else(|| failure("not_found", "admission", "check_configuration", false))?;
    if call.callable == "approval-clock-check" {
        let selected = config.approval_clock.as_ref().ok_or_else(|| {
            failure(
                "invalid_configuration",
                "configuration",
                "check_configuration",
                true,
            )
        })?;
        let observation = selected
            .acquire()
            .and_then(|clock| clock.observe())
            .map_err(|_| failure("unavailable", "observation", "check_configuration", false))?;
        return Ok(json!({"adapter":alias,"observation":observation}));
    }
    if call.callable.starts_with("approval-keys-") {
        return approval_keys::execute(call, &paths, &config, alias, adapter)
            .map_err(owner_failure);
    }
    if matches!(
        call.callable,
        "connections-list"
            | "connections-describe"
            | "connections-status"
            | "connections-revoke"
            | "connections-revalidate"
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

#[cfg(test)]
mod tests {
    use super::*;

    fn stage(error: owner::Error) -> (Value, Value) {
        let (HandlerReply::Error { data, .. } | HandlerReply::UsageError { data, .. }) =
            owner_failure(error)
        else {
            panic!("not a failure");
        };
        (data["stage"].clone(), data["next_action"].clone())
    }
    fn error(code: owner::Code, origin: owner::Origin) -> owner::Error {
        let mut error = owner::Error::from(code);
        error.origin = origin;
        error
    }

    #[test]
    fn a_provider_timeout_or_capacity_answer_reports_dispatch_and_the_hosts_own_admission() {
        use owner::{Code, Origin};
        for code in [Code::Timeout, Code::Capacity] {
            assert_eq!(
                stage(error(code, Origin::Provider)),
                (json!("dispatch"), json!("retry_explicitly")),
                "{code:?}"
            );
            assert_eq!(
                stage(error(code, Origin::Host)),
                (json!("admission"), json!("retry_explicitly")),
                "{code:?}"
            );
        }
        // Every source of `unsupported` is raised before dispatch.
        assert_eq!(
            stage(error(Code::Unsupported, Origin::Provider)),
            (json!("admission"), json!("none"))
        );
    }

    /// story:service-failure-carries-upstream-reason: the provider's bounded
    /// reason is in the failure data beside `service_code`, and absent when
    /// the owner reported none.
    #[test]
    fn a_service_failure_carries_the_upstream_reason_beside_its_code() {
        let mut error = owner::Error::from(owner::Code::ServiceFailure);
        error.service_code = Some(connectors_core::ErrorCode::Unauthorized);
        let HandlerReply::Error { data, .. } = invoke_failure(error.clone()) else {
            panic!("not a failure");
        };
        assert!(data.get("service_reason").is_none(), "{data}");
        error.service_reason = Some("Unauthorized; scope does not match".into());
        let HandlerReply::Error { data, .. } = invoke_failure(error) else {
            panic!("not a failure");
        };
        assert_eq!(data["code"], "service_failure");
        assert_eq!(data["service_code"], "unauthorized");
        assert_eq!(data["service_reason"], "Unauthorized; scope does not match");
        assert_eq!(data["stage"], "dispatch");
    }

    /// story:catalog-honours-retry-after: a rate-limited read states the delay
    /// the provider named beside `service_code`, so the caller can wait, and
    /// states its absence by omitting it.
    #[test]
    fn a_rate_limited_failure_states_the_providers_delay_or_its_absence() {
        let mut error = owner::Error::from(owner::Code::ServiceFailure);
        error.service_code = Some(connectors_core::ErrorCode::RateLimited);
        let HandlerReply::Error { data, .. } = invoke_failure(error.clone()) else {
            panic!("not a failure");
        };
        assert!(data.get("retry_after_seconds").is_none(), "{data}");
        error.retry_after_seconds = Some(3600);
        let HandlerReply::Error { data, .. } = invoke_failure(error) else {
            panic!("not a failure");
        };
        assert_eq!(data["code"], "service_failure");
        assert_eq!(data["service_code"], "rate_limited");
        assert_eq!(data["retry_after_seconds"], 3600);
        assert_eq!(data["stage"], "dispatch");
        assert_eq!(data["next_action"], "retry_explicitly");
    }

    fn reply(reply: HandlerReply) -> (Value, Value, Value) {
        let (HandlerReply::Error { data, .. } | HandlerReply::UsageError { data, .. }) = reply
        else {
            panic!("not a failure");
        };
        (
            data["code"].clone(),
            data["stage"].clone(),
            data["next_action"].clone(),
        )
    }

    #[test]
    fn a_revision_conflict_is_not_an_unreadable_store_at_any_mapping_site() {
        use connectors_host::local::registry;
        let conflict = (
            json!("revision_conflict"),
            json!("publication"),
            json!("retry_explicitly"),
        );
        let unavailable = (
            json!("metadata_unavailable"),
            json!("observation"),
            json!("retry_status"),
        );
        assert_eq!(reply(host_failure(Failure::ConcurrentRevision)), conflict);
        assert_eq!(
            reply(connections::registry_failure(
                registry::Failure::ConcurrentRevision
            )),
            conflict
        );
        assert_eq!(
            reply(owner_failure(registry::Failure::ConcurrentRevision.into())),
            conflict
        );
        assert_eq!(
            reply(host_failure(Failure::MetadataUnavailable)),
            unavailable
        );
        assert_eq!(
            reply(connections::registry_failure(
                registry::Failure::MetadataUnavailable
            )),
            unavailable
        );
        assert_eq!(
            reply(owner_failure(registry::Failure::MetadataUnavailable.into())),
            unavailable
        );
    }

    /// story:connection-follows-configuration-upgrade, beyond10x/connectors#102:
    /// a connection whose authentication changed under the configuration keeps
    /// `lifecycle_conflict` and names a new connection, never a repair that
    /// cannot help; a stale expected revision keeps `retry_status`.
    #[test]
    fn a_changed_authentication_names_a_new_connection_not_a_repair() {
        use connectors_host::local::registry;
        let changed = owner::Error::from(registry::Failure::BindingChanged);
        assert_eq!(changed.code, owner::Code::LifecycleConflict);
        // Across the owner transport, as the CLI receives it.
        let received: owner::Error =
            serde_json::from_value(serde_json::to_value(&changed).unwrap()).unwrap();
        for error in [changed, received] {
            assert_eq!(
                stage(error),
                (json!("admission"), json!("create_connection"))
            );
        }
        assert_eq!(
            stage(owner::Error::from(registry::Failure::Conflict)),
            (json!("admission"), json!("retry_status"))
        );
        let HandlerReply::Error { data, .. } =
            connections::registry_failure(registry::Failure::BindingChanged)
        else {
            panic!("not a failure");
        };
        assert_eq!(
            (&data["code"], &data["next_action"]),
            (&json!("lifecycle_conflict"), &json!("create_connection"))
        );
    }

    /// A different identity answered during a configuration upgrade keeps
    /// `identity_mismatch` but names a new connection: repair refuses the
    /// changed binding. Outside an upgrade it still names repair.
    #[test]
    fn a_different_identity_during_an_upgrade_names_a_new_connection() {
        use connectors_host::local::registry;
        let changed = owner::Error::from(registry::Failure::UpgradeIdentityMismatch);
        assert_eq!(changed.code, owner::Code::IdentityMismatch);
        let received: owner::Error =
            serde_json::from_value(serde_json::to_value(&changed).unwrap()).unwrap();
        for error in [changed, received] {
            assert_eq!(
                stage(error),
                (json!("admission"), json!("create_connection"))
            );
        }
        assert_eq!(
            stage(owner::Error::from(registry::Failure::IdentityMismatch)),
            (json!("admission"), json!("repair_connection"))
        );
        let HandlerReply::Error { data, .. } =
            connections::registry_failure(registry::Failure::UpgradeIdentityMismatch)
        else {
            panic!("not a failure");
        };
        assert_eq!(
            (&data["code"], &data["next_action"]),
            (&json!("identity_mismatch"), &json!("create_connection"))
        );
    }

    /// A credential the new provider refuses during a configuration upgrade
    /// names a new connection: repair refuses the changed binding. The same
    /// refusal outside an upgrade keeps its own next action.
    #[test]
    fn a_credential_refused_during_an_upgrade_names_a_new_connection() {
        use connectors_host::local::runtime;
        for failure in [
            runtime::Failure::InvalidCredential,
            runtime::Failure::IdentityMismatch,
            runtime::Failure::InsufficientScope,
        ] {
            let outside = owner::Error::from(failure);
            assert_ne!(stage(outside.clone()).1, json!("create_connection"));
            let mut upgrade = outside;
            upgrade.reconnect = true;
            let received: owner::Error =
                serde_json::from_value(serde_json::to_value(&upgrade).unwrap()).unwrap();
            for error in [upgrade, received] {
                assert_eq!(stage(error).1, json!("create_connection"), "{failure:?}");
            }
        }
    }

    /// story:expired-evidence-invoke-advises-revalidate: an invoke refused
    /// only because the connection's validation evidence expired keeps
    /// `not_granted` at admission and names `revalidate_connection`, also as
    /// the CLI receives it across the owner transport. An insufficient scope,
    /// and a connection not ready for any other reason, keep
    /// `repair_connection`.
    #[test]
    fn an_expired_evidence_refusal_names_revalidation_not_repair() {
        use connectors_host::local::{registry, runtime};
        let revalidate = (
            json!("not_granted"),
            json!("admission"),
            json!("revalidate_connection"),
        );
        let repair = (
            json!("not_granted"),
            json!("admission"),
            json!("repair_connection"),
        );
        let expired = owner::Error::from(registry::Failure::EvidenceExpired);
        assert_eq!(expired.code, owner::Code::NotGranted);
        let received: owner::Error =
            serde_json::from_value(serde_json::to_value(&expired).unwrap()).unwrap();
        for error in [expired, received] {
            assert_eq!(reply(invoke_failure(error)), revalidate);
        }
        assert_eq!(
            reply(connections::registry_failure(
                registry::Failure::EvidenceExpired
            )),
            revalidate
        );
        for failure in [
            registry::Failure::InsufficientScope,
            registry::Failure::NotReady,
        ] {
            let error = owner::Error::from(failure);
            let received: owner::Error =
                serde_json::from_value(serde_json::to_value(&error).unwrap()).unwrap();
            for error in [error, received] {
                assert_eq!(reply(invoke_failure(error)), repair, "{failure:?}");
            }
        }
        assert_eq!(
            reply(invoke_failure(runtime::Failure::InsufficientScope.into())),
            repair
        );
    }
}

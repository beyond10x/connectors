use connectors_cli_contract::{
    AcquireError, Handler, HandlerReply, Invocation, ProtectedSource, Sources,
};
use connectors_host::local::{
    Failure,
    config::{Config, Paths, Refusal, Rule, Selection},
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
mod declarations;
mod definition;
mod dialects;
mod list;
mod migrate;
mod operations;
mod owner_stop;
mod select;
mod session;

pub fn run(args: Vec<OsString>) -> connectors_cli_contract::ProcessOutput {
    let args = dialects::switch_shorthand(migrate::shorthand(args));
    // Help is no envelope and runs no command: it says nothing more on standard
    // error, under a deprecated name too; the help itself marks that name.
    if let Some(help) = connectors::help::render(&args) {
        return help_output(help, is_root_help(&args));
    }
    // `operations invoke --definition` is prepared, and rewritten to select the
    // definition's instance, before anything else reads the command line.
    let args = match definition::prepare(args) {
        Ok(args) => args,
        Err(refused) => return refused,
    };
    let deprecations = select::deprecations(&args);
    let json = select::json_output(&args);
    let mut output = run_selected(args);
    if !deprecations.is_empty() {
        select::deprecate(&mut output, json, &deprecations);
    }
    output
}

/// The help `connectors::help` rendered; root help is followed by the explicit
/// service commands, which the generated tree does not carry.
fn help_output(mut stdout: String, root: bool) -> connectors_cli_contract::ProcessOutput {
    if root {
        stdout.push_str("\nNative integration commands (use COMMAND --help for options):\n  server    Serve admitted read-only MCP tools over stdio\n");
        stdout.push_str("\nExplicit service commands (use COMMAND --help for options):\n");
        for (name, summary) in connectors::compat::SUMMARIES {
            stdout.push_str(&format!("  {name:<8}  {summary}\n"));
        }
    }
    connectors_cli_contract::ProcessOutput {
        exit_code: 0,
        stdout,
        stderr: String::new(),
    }
}

fn run_selected(args: Vec<OsString>) -> connectors_cli_contract::ProcessOutput {
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
    // `list` under `--output human` is a table, not the result's JSON text.
    if output.exit_code == 0
        && let Some(("connectors-list", context)) = session.borrow().selected()
        && context.output == connectors_cli_contract::OutputMode::Human
        && let Some(table) = list::human(&output.stdout)
    {
        output.stdout = table;
    }
    // A consumer ran: stdout and stderr were its own, and its exit status is
    // the launch's. No envelope follows it.
    if let Some(code) = session.borrow().launched {
        output = connectors_cli_contract::ProcessOutput {
            exit_code: code,
            stdout: String::new(),
            stderr: String::new(),
        };
    }
    // `connections token --reveal` issued a token: standard output is the
    // token and one newline, in every output mode, and nothing else is written.
    let revealed = session.borrow_mut().revealed.take();
    if let Some(token) = revealed
        && output.exit_code == 0
    {
        output = connectors_cli_contract::ProcessOutput {
            exit_code: 0,
            stdout: format!("{}\n", token.expose()),
            stderr: String::new(),
        };
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
    /// Every handler reads the selected alias in `adapter`, whichever selector
    /// named it; a result names its connector and instance beside it.
    fn call(&mut self, call: &Invocation<'_>) -> HandlerReply {
        let (mut input, config) = match select::resolve(call) {
            Ok(resolved) => resolved,
            Err(refusal) => return refusal,
        };
        if let Err(refusal) = connections::select(call, &mut input) {
            return refusal;
        }
        let call = &Invocation {
            callable: call.callable,
            target: call.target,
            context: call.context.clone(),
            input,
        };
        match self.dispatch(call) {
            HandlerReply::Success(mut result) => {
                select::name(call, &mut result, config);
                HandlerReply::Success(result)
            }
            reply => reply,
        }
    }
}

impl LocalHandler {
    fn dispatch(&mut self, call: &Invocation<'_>) -> HandlerReply {
        let result = if matches!(
            call.callable,
            "connections-connect" | "connections-reconnect"
        ) {
            let mut session = self.0.borrow_mut();
            if call
                .input
                .get("credential_document")
                .is_none_or(Value::is_null)
            {
                session.complete_without_source(call)
            } else {
                session.complete(call)
            }
            .map_err(|error| connect_failure(call.callable, error))
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
                .map_err(|error| {
                    if call.callable == "approval-policy-status" {
                        owner_failure(error)
                    } else {
                        approval_failure(error)
                    }
                })
        } else if call.callable == "operations-invoke" {
            let session = self.0.borrow();
            return operations::dispatch(call, session.deadline, session.mutation_deadline)
                .unwrap_or_else(invoke_failure);
        } else if call.callable == "connections-launch" {
            connections::launch(call).map(|code| {
                self.0.borrow_mut().launched = Some(code);
                json!({"consumer":call.input["consumer"],"exit_code":code})
            })
        } else if call.callable == "connections-token" {
            connections::token(call).map(|(result, token)| {
                self.0.borrow_mut().revealed = token;
                result
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

/// `connections connect` and `reconnect` as `owner_failure` reports them, with
/// one rule of their own (story:connect-refused-while-instance-holds-connection):
/// a reconnect that validated another account's credential is
/// `identity_mismatch` naming `revoke_connection`. Another account is another
/// connection, which the instance holds only once its own is revoked, or a
/// connection of another instance; reconnect again never gets there, and
/// connect is refused naming that reconnect.
fn connect_failure(callable: &str, error: owner::Error) -> HandlerReply {
    let other_account =
        callable == "connections-reconnect" && error.code == owner::Code::IdentityMismatch;
    let mut reply = owner_failure(error);
    if other_account
        && let HandlerReply::Error { data, .. } | HandlerReply::UsageError { data, .. } = &mut reply
    {
        data["next_action"] = json!("revoke_connection");
    }
    reply
}

fn owner_failure(error: owner::Error) -> HandlerReply {
    use owner::Code::*;
    if error.code == MetadataSchemaNewer {
        return schema_newer(error.store_version);
    }
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
        // The selected binding is read-only: only another configuration helps.
        CustodyReadOnly => ("custody", "check_configuration", false),
        OutcomeUnknown => ("publication", "retry_status", false),
        Forbidden => ("admission", "request_permission", false),
        NotFound => ("admission", "check_configuration", false),
        LifecycleConflict => ("admission", "retry_status", false),
        Revoked => ("admission", "create_connection", false),
        NotGranted | IdentityMismatch | ConnectionNotReady | InsufficientScope => {
            ("admission", "repair_connection", false)
        }
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
        MetadataSchemaNewer => ("admission", "upgrade", false),
    };
    // A refusal during a configuration upgrade, or of a changed binding: repair
    // refuses a changed binding, so only a new connection helps.
    // Only the connection's validation evidence expired, its credential is
    // intact: a revalidation recollects it, a repair would ask for re-entry.
    // The connection's identity changed under the configuration: no command
    // under this instance id helps, a new instance id does.
    let action = if error.reconfigure {
        "check_configuration"
    } else if error.revoke {
        "revoke_connection"
    } else if error.held {
        "repair_connection"
    } else if error.reconnect {
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
    if error.clock_format_retired && error.code == InvalidConfiguration {
        data["configuration_rule"] = json!(Rule::ApprovalClockFormatRetired.as_str());
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

/// `owner_failure` of `approvals policy-set`, `prepare` and `issue`: an
/// `invalid_input` names the argument it refused, as the CLI input model names
/// it (contracts/cli/v1alpha1/semantics.md, "The refused argument"). No other
/// command carries `field`, an approved write's `operations invoke` included.
fn approval_failure(error: owner::Error) -> HandlerReply {
    let field = error
        .field
        .filter(|_| error.code == owner::Code::InvalidInput);
    let mut reply = owner_failure(error);
    if let (Some(field), HandlerReply::Error { data, .. } | HandlerReply::UsageError { data, .. }) =
        (field, &mut reply)
    {
        data["field"] = json!(field.as_str());
    }
    reply
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

/// An approval clock that yielded no observation (contracts/service/clock.md,
/// "Refusals"). A retired `roughtime-clock/1` table is `invalid_configuration`
/// naming `approval_clock_format_retired`; a quorum that formed no answer is
/// `unavailable` naming the combined refusal, its next action and each
/// source's line. Never the observation's bounds, the request or the reply.
fn clock_failure(refused: connectors_host::local::clock::Refused) -> HandlerReply {
    if refused.retired {
        return configuration_refusal(Refusal::Rule(Rule::ApprovalClockFormatRetired));
    }
    if refused.failure == connectors_host::local::clock::Failure::InvalidConfiguration {
        return failure(
            "invalid_configuration",
            "configuration",
            "check_configuration",
            true,
        );
    }
    let mut reply = failure(
        "unavailable",
        "observation",
        refused.next_action().as_str(),
        false,
    );
    if let (Some(refusal), HandlerReply::Error { data, .. }) = (refused.refusal, &mut reply) {
        data["clock_refusal"] = json!(refusal);
        data["clock_sources"] = json!(refused.sources);
    }
    reply
}

/// `metadata_schema_newer` (semantics.md §3): a newer binary with another record
/// model wrote the store; only a binary at least as new can use it. `store` is
/// the release its writer marker names, `None` for `unrecorded`.
pub(super) fn schema_newer(store: Option<String>) -> HandlerReply {
    HandlerReply::Error {
        code: "failure".into(),
        data: json!({"kind":"operational","code":"metadata_schema_newer","stage":"admission","next_action":"upgrade",
            "store_version":store.unwrap_or_else(|| "unrecorded".to_owned()),
            "binary_version":connectors_host::local::Release::current().to_string()}),
    }
}

fn host_failure(error: Failure) -> HandlerReply {
    match error {
        Failure::MetadataSchemaNewer(store) => {
            schema_newer(store.map(|release| release.to_string()))
        }
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

/// Every command but `setup init`, `setup checkpoints-enable` and `setup owner-stop` loads the configuration somewhere in this
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
    // None reads the configuration: `setup init` writes it,
    // `setup checkpoints-enable` and `setup owner-stop` act on the state
    // directory alone, and `list` needs none.
    if matches!(
        callable,
        "setup-init" | "setup-checkpoints-enable" | "setup-owner-stop" | "connectors-list"
    ) {
        return None;
    }
    let refusal = match Paths::resolve(context.config.as_deref(), context.state_dir.as_deref()) {
        Err(_) => Refusal::Rule(Rule::ConfigPathUnresolved),
        Ok(paths) => match Config::read(&paths.config) {
            Err(refusal @ (Refusal::PrivateProtocolMismatch { .. } | Refusal::Rule(_))) => refusal,
            _ => return None,
        },
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
    if let HandlerReply::UsageError { data, .. } = &mut reply {
        match refusal {
            Refusal::PrivateProtocolMismatch {
                format,
                instance_id,
            } => {
                data["configuration_format"] = json!(format);
                data["instance_id"] = json!(instance_id);
            }
            Refusal::Rule(rule) => data["configuration_rule"] = json!(rule.as_str()),
            Refusal::Failure(_) => {}
        }
    }
    reply
}

/// The selected paths, or the configuration stage's refusal naming
/// `config_path_unresolved`.
fn resolved(call: &Invocation<'_>) -> Result<Paths, HandlerReply> {
    Paths::resolve(
        call.context.config.as_deref(),
        call.context.state_dir.as_deref(),
    )
    .map_err(|_| configuration_refusal(Refusal::Rule(Rule::ConfigPathUnresolved)))
}

/// `definitions install` (story:definitions-install): the configuration was
/// read by the preflight; the release is fetched, verified and recorded in the
/// lock beside it and the cache under the state directory.
mod definitions_install;

fn execute(call: &Invocation<'_>) -> Result<Value, HandlerReply> {
    if call.callable == "definitions-install" {
        return definitions_install::execute(call);
    }
    if call.callable == "connectors-list" {
        return list::execute(call);
    }
    let paths = resolved(call)?;
    if call.callable == "setup-checkpoints-enable" {
        return checkpoints::execute(call, &paths);
    }
    if call.callable == "setup-owner-stop" {
        return owner_stop::execute(&paths);
    }
    if call.callable == "setup-migrate" {
        return migrate::execute(call, &paths);
    }
    if call.callable == "setup-init" {
        let initialized = Config::initialize(&paths).map_err(host_failure)?;
        return Ok(
            json!({"disposition":initialized.disposition(), "config_path":paths.config, "state_path":paths.state, "os":std::env::consts::OS}),
        );
    }
    // The configured inventory and its prerequisites are the operator's file
    // alone; every command that selects an instance also sees an invoked
    // definition's (story:invoke-definition-file).
    let config = if matches!(call.callable, "setup-check" | "adapters-list") {
        Config::read(&paths.config)
    } else {
        Config::read_at(&paths)
    }
    .map_err(configuration_refusal)?;
    let summary = |alias: &str, adapter: &connectors_host::local::config::Adapter| json!({"adapter":alias,"instance_id":adapter.instance_id,"adapter_id":adapter.adapter_id,"startup":adapter.startup,"restart":adapter.restart});
    if call.callable == "setup-check" {
        let selection = config.selection();
        let state = keyring::inspect_at(config.secret_service_socket.as_deref());
        let mut checks =
            vec![json!({"name":"configuration", "state":"ready", "next_action":"none"})];
        // A store a newer binary wrote is refused, not reported as a prerequisite.
        let metadata = match Metadata::inspect(&paths.state) {
            Err(failure @ Failure::MetadataSchemaNewer(_)) => return Err(host_failure(failure)),
            opened => opened.is_ok(),
        };
        checks.push(json!({"name":"metadata", "state": if metadata {"ready"} else {"failed"}, "next_action":if metadata {"none"} else {"check_configuration"}}));
        for (alias, adapter) in &config.adapters {
            let ready = adapter.executable.check().is_ok();
            checks.push(json!({"name":format!("artifact:{alias}"), "state":if ready {"ready"} else {"failed"}, "next_action":if ready {"none"} else {"check_configuration"}}));
        }
        match &selection {
            Selection::Platform { .. } => {
                checks.push(json!({"name":"keyring", "state": if state == keyring::State::Available {"ready"} else {"failed"}, "next_action":if state == keyring::State::Available {"none"} else {"unlock_keyring"}}));
                // Availability alone is weaker than the qualified implementation and
                // storage binding with durable-write, restart and retirement evidence.
                let qualified = state == keyring::State::Available
                    && keyring::custody::available_for(&selection);
                checks.push(json!({"name":"persistent_custody_qualification", "state":if qualified {"ready"} else {"failed"}, "next_action":"none"}));
            }
            // The explicitly selected read-only directory takes the platform
            // store's place; that store is not consulted for custody.
            Selection::Mounted { directory } => {
                let ready = keyring::inspect_mounted(directory) == keyring::State::Available;
                checks.push(json!({"name":"mounted_custody", "state":if ready {"ready"} else {"failed"}, "next_action":if ready {"none"} else {"check_configuration"}}));
            }
        }
        // `os` is the build target's (`linux`, `macos`, `windows`, the model's
        // variants), and `custody_backend` the selected binding: the platform's,
        // fixed with the build, or the explicitly selected mounted directory.
        return Ok(
            json!({"config_path":paths.config,"state_path":paths.state,"os":std::env::consts::OS, "keyring":match state {keyring::State::Available => "available",keyring::State::Locked => "locked",keyring::State::Unavailable => "unavailable"},"custody_backend":selection.backend().map(keyring::CustodyBackend::as_str),"prerequisites":checks}),
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
            .acquire_explained()
            .and_then(|clock| clock.observe().map_err(Into::into))
            .map_err(clock_failure)?;
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
            &config.selection(),
            &config
                .adapters
                .values()
                .map(|entry| entry.instance_id.clone())
                .collect(),
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

    /// story:pending-connection-refusal-names-its-remedy: a connection saved
    /// under another configuration revision is `pending` with no status change
    /// pending, so its refusal keeps `lifecycle_conflict` at admission and
    /// names the step that clears it: revalidation while only the revision
    /// differs, a new connection when the authentication changed too. The
    /// instance's approval keys refuse the changed configuration naming a new
    /// connection. None of these names `retry_status`, also as the CLI
    /// receives them across the owner transport.
    #[test]
    fn a_stale_connection_never_names_retry_status() {
        use connectors_host::local::{approval_keys::Failure as KeyFailure, registry};
        let conflict = |action: &str| {
            (
                json!("lifecycle_conflict"),
                json!("admission"),
                json!(action),
            )
        };
        for (failure, action) in [
            (registry::Failure::UpgradeRequired, "revalidate_connection"),
            (registry::Failure::BindingChanged, "create_connection"),
            // story:revision-change-keeps-a-recovery-path: a changed provider
            // authority or profile authentication needs a new instance id.
            (
                registry::Failure::AuthenticationChanged,
                "check_configuration",
            ),
        ] {
            let error = owner::Error::from(failure);
            assert_eq!(error.code, owner::Code::LifecycleConflict, "{failure:?}");
            let received: owner::Error =
                serde_json::from_value(serde_json::to_value(&error).unwrap()).unwrap();
            for error in [error, received] {
                assert_eq!(
                    reply(invoke_failure(error)),
                    conflict(action),
                    "{failure:?}"
                );
            }
            assert_eq!(
                reply(connections::registry_failure(failure)),
                conflict(action),
                "{failure:?}"
            );
        }
        let keys = approval_keys::error(KeyFailure::BindingChanged);
        let received: owner::Error =
            serde_json::from_value(serde_json::to_value(&keys).unwrap()).unwrap();
        for error in [keys, received] {
            assert_eq!(reply(owner_failure(error)), conflict("create_connection"));
        }
    }

    /// story:connect-refused-while-instance-holds-connection: a connect refused
    /// beside another connect in flight names `retry_status` and that
    /// connect's acquisition, which `connections status --acquisition`
    /// follows, also across the transport.
    #[test]
    fn a_connect_beside_one_in_flight_names_its_acquisition() {
        use connectors_host::local::registry;
        let mut error = owner::Error::from(registry::Failure::Conflict);
        error.acquisition = Some("acq-in-flight".into());
        let received: owner::Error =
            serde_json::from_value(serde_json::to_value(&error).unwrap()).unwrap();
        for error in [error, received] {
            let HandlerReply::Error { data, .. } = connect_failure("connections-connect", error)
            else {
                panic!("not an operational failure");
            };
            assert_eq!(
                (
                    &data["code"],
                    &data["stage"],
                    &data["next_action"],
                    &data["acquisition"]
                ),
                (
                    &json!("lifecycle_conflict"),
                    &json!("admission"),
                    &json!("retry_status"),
                    &json!("acq-in-flight")
                ),
                "{data}"
            );
        }
    }

    /// story:connect-refused-while-instance-holds-connection: a reconnect that
    /// validated another account's credential names `revoke_connection`,
    /// whether the registry or the adapter answered the other identity and as
    /// the CLI receives it across the transport, so connect (refused naming
    /// the reconnect) and reconnect never send each other round. A connect
    /// and every other refusal of a reconnect keep `owner_failure`'s action.
    #[test]
    fn a_reconnect_to_another_account_names_revocation() {
        use connectors_host::local::{registry, runtime};
        let mismatch = (
            json!("identity_mismatch"),
            json!("admission"),
            json!("revoke_connection"),
        );
        for error in [
            owner::Error::from(registry::Failure::IdentityMismatch),
            owner::Error::from(runtime::Failure::IdentityMismatch),
        ] {
            let received: owner::Error =
                serde_json::from_value(serde_json::to_value(&error).unwrap()).unwrap();
            for error in [error, received] {
                assert_eq!(
                    reply(connect_failure("connections-reconnect", error.clone())),
                    mismatch
                );
                assert_eq!(
                    reply(connect_failure("connections-connect", error.clone())),
                    reply(owner_failure(error))
                );
            }
        }
        for failure in [
            registry::Failure::Conflict,
            registry::Failure::BindingChanged,
            registry::Failure::Revoked,
        ] {
            let error = owner::Error::from(failure);
            assert_eq!(
                reply(connect_failure("connections-reconnect", error.clone())),
                reply(owner_failure(error)),
                "{failure:?}"
            );
        }
    }

    /// story:connect-refused-while-instance-holds-connection: a connect on an
    /// instance that holds a published Live or Stale connection is
    /// `lifecycle_conflict` at admission naming the step that clears it,
    /// `connections reconnect` when it replaces that connection's credential
    /// and `connections revoke` when no reconnect can, as the owner reports it
    /// and as the CLI receives it across the transport.
    #[test]
    fn a_connect_on_an_instance_holding_its_connection_names_reconnect_or_revoke() {
        use connectors_host::local::registry;
        for (failure, action) in [
            (registry::Failure::InstanceConnected, "repair_connection"),
            (
                registry::Failure::InstanceConnectionChanged,
                "revoke_connection",
            ),
        ] {
            let expected = (
                json!("lifecycle_conflict"),
                json!("admission"),
                json!(action),
            );
            let error = owner::Error::from(failure);
            assert_eq!(error.code, owner::Code::LifecycleConflict, "{failure:?}");
            let received: owner::Error =
                serde_json::from_value(serde_json::to_value(&error).unwrap()).unwrap();
            for error in [error, received] {
                assert_eq!(reply(owner_failure(error)), expected, "{failure:?}");
            }
            assert_eq!(
                reply(connections::registry_failure(failure)),
                expected,
                "{failure:?}"
            );
        }
    }
}

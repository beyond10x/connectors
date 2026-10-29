//! `connectors invoke <connector>[/<instance>] <operation>`
//! (story:connectors-do-short-invocation; `contracts/cli/v1alpha1/semantics.md`
//! §3, "Invoking an operation").
//!
//! The command line is prepared here, before the generated runner reads it.
//! The target names a connector and, after one `/`, an instance; with the
//! instance left out the connector's only configured instance is selected and
//! `default_instance` is not consulted. Without `--connection` the instance's
//! one usable connection is selected. `--schema` and `--revision`, left out,
//! are the instance's current admitted description's; given, they are pins a
//! differing description refuses. With no input flag the document is read from
//! standard input, unless standard input is a terminal. `--describe` is
//! answered here, from the description, and reads and invokes nothing; a write
//! without `--approval-file` is refused here, before any provider call. The
//! command line is then rewritten with every value stated and the target as
//! `<connector>/<instance>`, and the generated runner admits and invokes it as
//! `operations invoke` would, the answer naming the connector and instance.
use super::*;
use clap::parser::ValueSource;
use connectors_cli_contract::ProcessOutput;
use connectors_host::local::{
    config::Adapter,
    owner::Code,
    protected,
    registry::{self, Registry, State},
    runtime::Effect,
};
use std::{io::IsTerminal, path::PathBuf};

/// The approval commands a write runs, in order, before it is invoked.
const APPROVAL_STEPS: [&str; 3] = [
    "approvals prepare",
    "approvals clock-check",
    "approvals issue",
];
/// The most connections an instance's listing reads; an instance holds one.
const LISTED: u16 = 500;

/// `<connector>` or `<connector>/<instance>`, each part an admitted id.
pub(super) fn target(text: &str) -> Option<(&str, Option<&str>)> {
    let (connector, instance) = match text.split_once('/') {
        Some((connector, instance)) => (connector, Some(instance)),
        None => (text, None),
    };
    (connectors_core::valid_id(connector) && instance.is_none_or(connectors_core::valid_id))
        .then_some((connector, instance))
}

/// One configured instance `invoke` can name.
#[derive(Debug, PartialEq, Eq)]
struct Named {
    connector: String,
    instance: String,
    /// `None` for a listed instance that has no entry: its connector declares
    /// several profiles and it names none (`Config::awaiting_profile`).
    alias: Option<String>,
}

/// Every configured instance by the names `invoke` knows it by: a
/// `[connectors]` instance or recorded definition by its own; a legacy
/// `[adapters.<alias>]` entry by its alias, under the connector `setup
/// migrate` derives for it, or its implementation when that is not the
/// catalog. A catalog entry no connector is derived for has no name here.
fn named(config: &Config) -> Vec<Named> {
    let legacy = config.legacy_aliases();
    let mut named: Vec<Named> = config
        .adapters
        .iter()
        .filter_map(|(alias, adapter)| {
            let (connector, instance) = if legacy.contains(&alias.as_str()) {
                let connector = match super::migrate::derived_connector(adapter) {
                    Some(connector) => connector,
                    None if adapter.adapter_id != "catalog" => adapter.adapter_id.clone(),
                    None => return None,
                };
                (connector, alias.clone())
            } else {
                let (connector, instance) = config.names(alias)?;
                (connector.to_owned(), instance.to_owned())
            };
            Some(Named {
                connector,
                instance,
                alias: Some(alias.clone()),
            })
        })
        .collect();
    for (connector, table) in config.connectors.iter().flatten() {
        for instance in table.instances.keys() {
            if !named
                .iter()
                .any(|n| n.connector == *connector && n.instance == *instance)
            {
                named.push(Named {
                    connector: connector.clone(),
                    instance: instance.clone(),
                    alias: None,
                });
            }
        }
    }
    named
}

/// Why a target selects no entry.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Unselected {
    /// Several instances and the target names none: each `<connector>/<instance>`, sorted.
    Ambiguous(Vec<String>),
    NotFound,
    ProfileUnnamed,
}

/// The connector, instance and alias the target selects under `config`.
fn select(
    config: &Config,
    connector: &str,
    instance: Option<&str>,
) -> Result<(String, String, String), Unselected> {
    let matching: Vec<Named> = named(config)
        .into_iter()
        .filter(|n| n.connector == connector && instance.is_none_or(|i| n.instance == i))
        .collect();
    match matching.as_slice() {
        [] => Err(Unselected::NotFound),
        [one] => match &one.alias {
            Some(alias) => Ok((one.connector.clone(), one.instance.clone(), alias.clone())),
            None if config
                .awaiting_profile(&one.connector, &one.instance)
                .is_some() =>
            {
                Err(Unselected::ProfileUnnamed)
            }
            None => Err(Unselected::NotFound),
        },
        several => {
            let mut listed: Vec<String> = several
                .iter()
                .map(|n| format!("{}/{}", n.connector, n.instance))
                .collect();
            listed.sort();
            Err(Unselected::Ambiguous(listed))
        }
    }
}

fn unselected(refused: Unselected) -> HandlerReply {
    match refused {
        Unselected::Ambiguous(instances) => {
            let mut reply = failure("instance_ambiguous", "arguments", "none", true);
            if let HandlerReply::UsageError { data, .. } = &mut reply {
                data["instances"] = json!(instances);
            }
            reply
        }
        Unselected::NotFound => failure("not_found", "admission", "check_configuration", false),
        Unselected::ProfileUnnamed => failure(
            "invalid_configuration",
            "admission",
            "check_configuration",
            false,
        ),
    }
}

/// The alias the `target` of an `invoke` input selects, for the steps that
/// read the input before the handler: protected sources and the dynamic
/// validator.
pub(super) fn alias_at(input: &Value, paths: &Paths) -> owner::Result<String> {
    let (connector, instance) = input
        .get("target")
        .and_then(Value::as_str)
        .and_then(target)
        .ok_or(Code::InvalidInput)?;
    let config = Config::load_at(paths)?;
    select(&config, connector, instance)
        .map(|(_, _, alias)| alias)
        .map_err(|refused| match refused {
            Unselected::Ambiguous(_) => Code::InvalidInput.into(),
            Unselected::NotFound => Code::NotFound.into(),
            Unselected::ProfileUnnamed => Code::InvalidConfiguration.into(),
        })
}

/// One connection an instance holds, as `connection_unresolved` lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Held {
    pub(super) connection: String,
    pub(super) state: State,
    /// Admitted under the instance's current configuration.
    pub(super) current: bool,
}

/// The connection an `invoke` without `--connection` uses: the one usable
/// connection the instance holds. Usable is `ready`, or `pending` only because
/// its validation evidence lapsed under the current configuration, which the
/// read that uses it recollects (`owner::admit_invoke`); revoked connections
/// are not held. None usable, or several, is `connection_unresolved` listing
/// every connection held with its state: naming a connect when none is held,
/// a repair when none held is usable, nothing when `--connection` must choose.
pub(super) fn decide(held: &[Held]) -> Result<String, HandlerReply> {
    let usable: Vec<&Held> = held
        .iter()
        .filter(|h| h.state == State::Ready || (h.state == State::Pending && h.current))
        .collect();
    if let [one] = usable.as_slice() {
        return Ok(one.connection.clone());
    }
    let next_action = if held.is_empty() {
        "create_connection"
    } else if usable.is_empty() {
        "repair_connection"
    } else {
        "none"
    };
    let mut reply = failure("connection_unresolved", "admission", next_action, false);
    if let HandlerReply::Error { data, .. } = &mut reply {
        data["connections"] = json!(
            held.iter()
                .map(|h| json!({"connection":h.connection,"state":h.state}))
                .collect::<Vec<_>>()
        );
    }
    Err(reply)
}

/// The connections the instance of `adapter` holds, revoked ones excepted.
fn held(paths: &Paths, config: &Config, adapter: &Adapter) -> Result<Vec<Held>, HandlerReply> {
    let page = Registry::with_system_clock(&paths.state)
        .list(
            &adapter.instance_id,
            &adapter.adapter_id,
            adapter.configured(),
            registry::PageOptions {
                limit: LISTED,
                cursor: None,
            },
            connectors_sdk::now_ms(),
            keyring::custody::available_for(&config.selection()),
        )
        .map_err(super::connections::registry_failure)?;
    Ok(page
        .connections
        .into_iter()
        .filter(|c| c.state != State::Revoked)
        .map(|c| Held {
            connection: c.reference,
            state: c.state,
            current: !c.stale,
        })
        .collect())
}

/// What an `invoke` command line asks for.
struct Request {
    program: OsString,
    globals: Vec<OsString>,
    json: bool,
    config: Option<PathBuf>,
    state: Option<PathBuf>,
    target: String,
    operation: String,
    connection: Option<String>,
    schema: Option<String>,
    revision: Option<String>,
    /// The input source as given: `--input-json=…`, `--input-file=…` or
    /// `--input-stdin`.
    input: Option<OsString>,
    approval_file: Option<OsString>,
    idempotency_key: Option<String>,
    definition: Option<PathBuf>,
    definitions_root: Option<PathBuf>,
    credential_file: Option<PathBuf>,
    describe: bool,
}

/// The generated command with `--schema`, `--revision` and the input source
/// of `invoke` optional: the preparation states them.
fn relaxed() -> clap::Command {
    connectors_cli_contract::command().mut_subcommand("invoke", |invoke| {
        invoke
            .mut_arg("field:schema", |arg| arg.required(false))
            .mut_arg("field:revision", |arg| arg.required(false))
            .mut_group("sources:input", |group| group.required(false))
    })
}

/// The request of an `invoke` command line, `None` for any other command line
/// or one the relaxed command does not parse, which the generated runner then
/// refuses as it does any.
fn parse(args: &[OsString]) -> Option<Request> {
    let matches = relaxed().try_get_matches_from(args).ok()?;
    let ("invoke", leaf) = matches.subcommand()? else {
        return None;
    };
    let given = |id: &str| leaf.value_source(id) == Some(ValueSource::CommandLine);
    let text = |field: &str| {
        let id = format!("field:{field}");
        given(&id)
            .then(|| leaf.get_one::<String>(&id).cloned())
            .flatten()
    };
    let raw = |id: &str, long: &str| {
        given(id)
            .then(|| leaf.get_raw(id).and_then(|mut values| values.next()))
            .flatten()
            .map(|value| {
                let mut arg = OsString::from(format!("--{long}="));
                arg.push(value);
                arg
            })
    };
    let plan = connectors_cli_contract::plan();
    let mut globals = Vec::new();
    for (id, long) in [
        ("global_config", plan.globals.config.clone()),
        ("global_state", Some(plan.globals.state.clone())),
        ("global_output", plan.globals.output.clone()),
    ] {
        let (Some(long), Some(ValueSource::CommandLine)) = (long, matches.value_source(id)) else {
            continue;
        };
        let value = matches.get_raw(id)?.next()?;
        let mut arg = OsString::from(format!("--{long}="));
        arg.push(value);
        globals.push(arg);
    }
    let input = raw("source:input:inline", "input-json")
        .or_else(|| raw("source:input:file", "input-file"))
        .or_else(|| {
            leaf.get_flag("source:input:stdin")
                .then(|| "--input-stdin".into())
        });
    let describe = match text("describe").as_deref() {
        None | Some("false") => false,
        Some("true") => true,
        // Not a Boolean: the generated runner refuses it.
        Some(_) => return None,
    };
    Some(Request {
        program: args.first()?.clone(),
        globals,
        json: matches
            .get_one::<String>("global_output")
            .map(String::as_str)
            == Some("json"),
        config: matches.get_one::<PathBuf>("global_config").cloned(),
        state: matches.get_one::<PathBuf>("global_state").cloned(),
        target: text("target")?,
        operation: text("operation")?,
        connection: text("connection"),
        schema: text("schema"),
        revision: text("revision"),
        input,
        approval_file: raw("field:approval_file", "approval-file"),
        idempotency_key: text("idempotency_key"),
        definition: text("definition").map(PathBuf::from),
        definitions_root: text("definitions_root").map(PathBuf::from),
        credential_file: text("credential_file").map(PathBuf::from),
        describe,
    })
}

/// `invalid_input` at `arguments`, naming the refused argument `field`.
fn refused(field: &str) -> HandlerReply {
    let mut reply = failure("invalid_input", "arguments", "none", true);
    if let HandlerReply::UsageError { data, .. } = &mut reply {
        data["field"] = json!(field);
    }
    reply
}

/// `reply`, naming `field` when it is an `invalid_input` that names none.
fn naming(mut reply: HandlerReply, field: &str) -> HandlerReply {
    if let HandlerReply::UsageError { data, .. } | HandlerReply::Error { data, .. } = &mut reply
        && data["code"] == "invalid_input"
        && data.get("field").is_none_or(Value::is_null)
    {
        data["field"] = json!(field);
    }
    reply
}

/// The arguments the generated runner reads: `args` unchanged unless they are
/// an `invoke`, which is prepared and answered as the rewritten command line;
/// `--describe` is answered here, and a refusal is the one failure envelope.
pub(super) fn prepare(args: Vec<OsString>) -> Result<Vec<OsString>, ProcessOutput> {
    let Some(request) = parse(&args) else {
        return Ok(args);
    };
    let json = request.json;
    let prepared = match protected::Signals::install() {
        Ok(signals) => {
            let prepared = prepare_with(&request, &signals);
            if signals.interrupted() {
                Err(owner_failure(Code::Interrupted.into()))
            } else {
                prepared
            }
        }
        Err(error) => Err(owner_failure(error)),
    };
    match prepared {
        Ok(Prepared::Run(args)) => Ok(args),
        Ok(Prepared::Described(result)) => Err(success(result, json)),
        Err(reply) => Err(definition::render(reply, json)),
    }
}

enum Prepared {
    Run(Vec<OsString>),
    Described(Value),
}

/// The one success envelope, as the generated runner writes a handler's.
fn success(result: Value, json: bool) -> ProcessOutput {
    let stdout = if json {
        format!("{}\n", json!({"ok":true,"result":result}))
    } else {
        format!("{result}\n")
    };
    ProcessOutput {
        exit_code: 0,
        stdout,
        stderr: String::new(),
    }
}

fn prepare_with(request: &Request, signals: &protected::Signals) -> Result<Prepared, HandlerReply> {
    // Arguments first, each refusal naming its argument; nothing is read yet.
    let (connector, instance) = target(&request.target).ok_or_else(|| refused("target"))?;
    if !connectors_core::valid_id(&request.operation) {
        return Err(refused("operation"));
    }
    for (field, value) in [
        ("connection", &request.connection),
        ("schema", &request.schema),
        ("revision", &request.revision),
    ] {
        if value
            .as_deref()
            .is_some_and(|v| !connectors_core::valid_id(v))
        {
            return Err(refused(field));
        }
    }
    if request.definition.is_none() {
        // `--definitions-root` and `--credential-file` belong to `--definition`.
        if request.definitions_root.is_some() {
            return Err(refused("definitions_root"));
        }
        if request.credential_file.is_some() {
            return Err(refused("credential_file"));
        }
    }
    if request
        .idempotency_key
        .as_deref()
        .is_some_and(|key| key.is_empty() || key.len() > 256)
    {
        return Err(refused("idempotency_key"));
    }
    // With no input flag the document is standard input, which is never read
    // when it is a terminal: nothing would end it.
    let input = match &request.input {
        Some(input) => Some(input.clone()),
        None if request.describe => None,
        None if std::io::stdin().is_terminal() => return Err(refused("input")),
        None => Some("--input-stdin".into()),
    };
    let paths = Paths::resolve(request.config.as_deref(), request.state.as_deref())
        .map_err(|_| configuration_refusal(Refusal::Rule(Rule::ConfigPathUnresolved)))?;
    let (connector, instance, alias, connection) = match &request.definition {
        Some(definition) => {
            let prepared = definition::prepare_with(
                &definition::Request {
                    program: request.program.clone(),
                    definition: Some(definition.clone()),
                    root: request.definitions_root.clone(),
                    credential_file: request.credential_file.clone(),
                    selected: false,
                    target: Some((connector.to_owned(), instance.map(str::to_owned))),
                    operation: request.operation.clone(),
                    connection: request.connection.clone(),
                    schema: None,
                    revision: None,
                    config: request.config.clone(),
                    state: request.state.clone(),
                    json: request.json,
                    globals: Vec::new(),
                    kept: Vec::new(),
                },
                definition,
                signals,
            )
            .map_err(|reply| naming(reply, "definition"))?;
            (
                prepared.connector,
                prepared.instance,
                prepared.alias,
                Some(prepared.connection),
            )
        }
        None => {
            let config = Config::read_at(&paths).map_err(configuration_refusal)?;
            let (connector, instance, alias) =
                select(&config, connector, instance).map_err(unselected)?;
            let adapter = config
                .adapters
                .get(&alias)
                .ok_or_else(|| unselected(Unselected::NotFound))?;
            let connection = match &request.connection {
                Some(connection) => Some(connection.clone()),
                // `--describe` uses the one usable connection's binding when
                // there is one, and the instance's description otherwise.
                None if request.describe => decide(&held(&paths, &config, adapter)?).ok(),
                None => Some(decide(&held(&paths, &config, adapter)?)?),
            };
            (connector, instance, alias, connection)
        }
    };
    let bootstrap =
        owner::bound_described(&paths, &alias, connection.as_deref()).map_err(owner_failure)?;
    let adapter = Config::load_at(&paths)
        .map_err(|failure| owner_failure(failure.into()))?
        .adapters
        .get(&alias)
        .cloned()
        .ok_or_else(|| unselected(Unselected::NotFound))?;
    let requirement =
        owner::admit_operation(&adapter, &bootstrap, &request.operation).map_err(owner_failure)?;
    let descriptor = bootstrap
        .descriptor()
        .map_err(|failure| owner_failure(failure.into()))?;
    let schema = owner::schema(&bootstrap, &request.operation).map_err(owner_failure)?;
    let revision = descriptor.revision.clone();
    // A pin is compared, never replaced by the current description.
    if request.schema.as_ref().is_some_and(|pin| *pin != schema)
        || request
            .revision
            .as_ref()
            .is_some_and(|pin| *pin != revision)
    {
        return Err(owner_failure(Code::StaleDescription.into()));
    }
    let effect = requirement.effect;
    if request.describe {
        let operation = descriptor
            .operation(&request.operation)
            .map_err(|_| owner_failure(Code::NotFound.into()))?;
        return Ok(Prepared::Described(described(
            &connector, &instance, operation, effect, schema, revision,
        )));
    }
    match effect {
        Effect::Read if request.idempotency_key.is_some() => {
            return Err(refused("idempotency_key"));
        }
        Effect::Read if request.approval_file.is_some() => return Err(refused("approval_file")),
        Effect::Write if request.approval_file.is_none() => {
            let mut reply = failure("approval_required", "admission", "none", false);
            if let HandlerReply::Error { data, .. } = &mut reply {
                data["approval_steps"] = json!(APPROVAL_STEPS);
            }
            return Err(reply);
        }
        _ => {}
    }
    let connection = connection.ok_or_else(|| owner_failure(Code::InvalidInput.into()))?;
    let mut rewritten = vec![request.program.clone()];
    rewritten.extend(request.globals.iter().cloned());
    rewritten.push("invoke".into());
    for (long, value) in [
        ("connection", connection.as_str()),
        ("schema", schema.as_str()),
        ("revision", revision.as_str()),
    ] {
        rewritten.push(format!("--{long}={value}").into());
    }
    rewritten.extend(input);
    rewritten.extend(request.approval_file.iter().cloned());
    if let Some(key) = &request.idempotency_key {
        rewritten.push(format!("--idempotency-key={key}").into());
    }
    // An operation id may begin with `-`: after `--` both are values.
    rewritten.push("--".into());
    rewritten.push(format!("{connector}/{instance}").into());
    rewritten.push(request.operation.clone().into());
    Ok(Prepared::Run(rewritten))
}

/// `invoke --describe`'s answer (`connectors.cli.InvokeDescription`).
fn described(
    connector: &str,
    instance: &str,
    operation: &connectors_core::Operation,
    effect: Effect,
    schema: String,
    revision: String,
) -> Value {
    json!({"connector":connector,"instance":instance,"operation":operation.id,
        "description":operation.description,"effect":match effect {
            Effect::Read => "read",
            Effect::Write => "write",
            Effect::Unknown => "unknown",
        },
        "schema":schema,"revision":revision,"input_schema":operation.input_schema})
}

/// The `invoke` handler: the target's instance invoked as `operations invoke`
/// invokes it (`operations::dispatch`), the answer naming the connector and
/// instance and never an alias, and every `invalid_input` naming its argument.
pub(super) fn dispatch(
    call: &Invocation<'_>,
    read_deadline: u64,
    deadline: Option<owner::mutation::Deadline>,
) -> HandlerReply {
    let paths = match resolved(call) {
        Ok(paths) => paths,
        Err(reply) => return reply,
    };
    let config = match Config::read_at(&paths) {
        Ok(config) => config,
        Err(refusal) => return configuration_refusal(refusal),
    };
    let Some((connector, instance)) = call.input["target"].as_str().and_then(target) else {
        return refused("target");
    };
    let (connector, instance, alias) = match select(&config, connector, instance) {
        Ok(selected) => selected,
        Err(refusal) => return unselected(refusal),
    };
    let connection = match call.input.get("connection").and_then(Value::as_str) {
        Some(connection) => connection.to_owned(),
        None => {
            let Some(adapter) = config.adapters.get(&alias) else {
                return unselected(Unselected::NotFound);
            };
            match held(&paths, &config, adapter).and_then(|held| decide(&held)) {
                Ok(connection) => connection,
                Err(reply) => return reply,
            }
        }
    };
    let mut input = json!({"adapter":alias,"connection":connection});
    for field in [
        "operation",
        "schema",
        "revision",
        "input",
        "approval_file",
        "idempotency_key",
    ] {
        if let Some(value) = call.input.get(field).filter(|v| !v.is_null()) {
            input[field] = value.clone();
        }
    }
    let call = &Invocation {
        callable: call.callable,
        target: call.target,
        context: call.context.clone(),
        input,
    };
    match super::operations::dispatch_tagged(call, read_deadline, deadline) {
        Ok(HandlerReply::Success(mut result)) => {
            if let Some(fields) = result.as_object_mut() {
                fields.remove("adapter");
                fields.insert("connector".into(), json!(connector));
                fields.insert("instance".into(), json!(instance));
            }
            HandlerReply::Success(result)
        }
        Ok(reply) => reply,
        Err(refusal) => {
            let field = refusal
                .error
                .field
                .map(argument)
                .or(refusal.field)
                .unwrap_or_else(|| unnamed(&call.input));
            naming(invoke_failure(*refusal.error), field)
        }
    }
}

/// An owner refusal of an `invoke` before its handler ran (a protected source
/// or the dynamic validator), rendered as the handler renders one.
pub(super) fn early_failure(error: owner::Error, input: &Value) -> HandlerReply {
    let field = error.field.map(argument).unwrap_or_else(|| unnamed(input));
    naming(invoke_failure(error), field)
}

/// The `invoke` input field an owner argument is: the selected alias is the
/// target's, an input file is the document.
fn argument(argument: owner::Argument) -> &'static str {
    match argument {
        owner::Argument::Adapter => "target",
        owner::Argument::InputFile => "input",
        other => other.as_str(),
    }
}

/// The argument an `invalid_input` that names none is about: the first id
/// the input states that is not an admitted one, else the document.
fn unnamed(input: &Value) -> &'static str {
    ["operation", "connection", "schema", "revision"]
        .into_iter()
        .find(|field| {
            input
                .get(*field)
                .and_then(Value::as_str)
                .is_some_and(|value| !connectors_core::valid_id(value))
        })
        .unwrap_or("input")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(connection: &str, state: State, current: bool) -> Held {
        Held {
            connection: connection.into(),
            state,
            current,
        }
    }

    fn data(reply: HandlerReply) -> Value {
        match reply {
            HandlerReply::Error { data, .. } | HandlerReply::UsageError { data, .. } => data,
            _ => panic!("a refusal"),
        }
    }

    /// story:connectors-do-short-invocation: the one usable connection is
    /// selected, even beside held ones that are not usable; none usable or
    /// several is `connection_unresolved` listing every one held.
    #[test]
    fn the_one_usable_connection_is_selected_and_otherwise_listed() {
        assert_eq!(
            decide(&[held("a", State::Ready, true)]).ok(),
            Some("a".into())
        );
        // Lapsed validation evidence reads `pending`; the read recollects it.
        assert_eq!(
            decide(&[held("a", State::Pending, true)]).ok(),
            Some("a".into())
        );
        assert_eq!(
            decide(&[
                held("a", State::Stale, true),
                held("b", State::Ready, true),
                held("c", State::Pending, false),
            ])
            .ok(),
            Some("b".into())
        );
        let none = data(decide(&[]).unwrap_err());
        assert_eq!(
            none,
            json!({"kind":"operational","code":"connection_unresolved","stage":"admission",
                "next_action":"create_connection","connections":[]}),
            "story:connectors-do-short-invocation"
        );
        let unusable = data(
            decide(&[
                held("a", State::Pending, false),
                held("b", State::ReauthorizationRequired, true),
            ])
            .unwrap_err(),
        );
        assert_eq!(
            unusable,
            json!({"kind":"operational","code":"connection_unresolved","stage":"admission",
                "next_action":"repair_connection","connections":[
                    {"connection":"a","state":"pending"},
                    {"connection":"b","state":"reauthorization_required"}]}),
            "story:connectors-do-short-invocation"
        );
        let two = data(
            decide(&[held("a", State::Ready, true), held("b", State::Ready, true)]).unwrap_err(),
        );
        assert_eq!(
            two,
            json!({"kind":"operational","code":"connection_unresolved","stage":"admission",
                "next_action":"none","connections":[
                    {"connection":"a","state":"ready"},
                    {"connection":"b","state":"ready"}]}),
            "story:connectors-do-short-invocation"
        );
    }

    /// story:connectors-do-short-invocation: a target is a connector, or a
    /// connector and one instance after one `/`, each an admitted id.
    #[test]
    fn a_target_is_a_connector_and_at_most_one_instance() {
        assert_eq!(target("jira"), Some(("jira", None)));
        assert_eq!(target("jira/main"), Some(("jira", Some("main"))));
        for malformed in [
            "",
            "/",
            "jira/",
            "/main",
            "jira/main/x",
            "ji ra",
            "jira/ma in",
        ] {
            assert_eq!(
                target(malformed),
                None,
                "story:connectors-do-short-invocation: {malformed:?}"
            );
        }
    }

    /// story:connectors-do-short-invocation: an `invalid_input` that names no
    /// argument names the first malformed id, else the document.
    #[test]
    fn an_unnamed_invalid_input_names_the_malformed_id_or_the_document() {
        assert_eq!(
            unnamed(&json!({"operation":"a b","connection":"c d"})),
            "operation"
        );
        assert_eq!(
            unnamed(&json!({"operation":"op","revision":"r 1"})),
            "revision"
        );
        assert_eq!(unnamed(&json!({"operation":"op","input":"{}"})), "input");
    }
}

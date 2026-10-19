//! `--connector` and `--instance`, and the deprecated `--adapter`
//! (story:cli-connector-instance-selectors, `contracts/cli/v1alpha1/semantics.md`
//! §3 "Selecting an instance"). A selector is resolved to the alias of one
//! configured entry before any handler, source or validator reads it, so every
//! later step sees exactly what `--adapter <alias>` gave it; a result that names
//! an alias names that entry's connector and instance beside it.
use super::*;
use connectors_cli_contract::{ProcessOutput, wire::Shape};
use connectors_host::local::config::Selector;
use std::sync::OnceLock;

/// A deprecated name the arguments use: the `--adapter` selector, or a
/// renamed command path (story:connections-reconnect-verb), which is any path
/// alias the plan declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Deprecation {
    Adapter,
    Command { old: String, new: String },
}

impl Deprecation {
    /// The entry an `--output json` envelope carries.
    fn entry(&self) -> Value {
        match self {
            Self::Adapter => {
                json!({"option":"--adapter","replacement":["--connector","--instance"]})
            }
            Self::Command { old, new } => json!({"command":old,"replacement":[new]}),
        }
    }

    /// The one line `--output human` writes to standard error.
    fn line(&self) -> String {
        match self {
            Self::Adapter => {
                "deprecated: --adapter; select with --connector <connector> --instance <instance>\n"
                    .to_owned()
            }
            Self::Command { old, new } => format!("deprecated: {old}; use {new}\n"),
        }
    }
}

fn plan() -> &'static connectors_cli_contract::wire::Plan {
    static PLAN: OnceLock<connectors_cli_contract::wire::Plan> = OnceLock::new();
    PLAN.get_or_init(connectors_cli_contract::plan)
}

/// Whether the callable's input carries the selector fields.
fn selects(callable: &str) -> bool {
    plan()
        .callables
        .get(callable)
        .and_then(|callable| callable.input.as_ref())
        .is_some_and(|input| {
            matches!(&input.shape, Shape::Struct { fields } if fields.contains_key("connector"))
        })
}

/// Why the selector fields name no entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Refused {
    /// Both selectors, half of `--connector`/`--instance`, or none and no default.
    InvalidInput,
    /// `--connector`/`--instance` name no configured instance.
    NotFound,
    /// They name a listed instance that has no entry: its connector declares
    /// several profiles and it names none (`Config::awaiting_profile`).
    ProfileUnnamed,
}

impl From<Refused> for owner::Error {
    fn from(refused: Refused) -> Self {
        match refused {
            Refused::InvalidInput => owner::Code::InvalidInput.into(),
            Refused::NotFound => owner::Code::NotFound.into(),
            Refused::ProfileUnnamed => owner::Code::InvalidConfiguration.into(),
        }
    }
}

fn refusal(refused: Refused) -> HandlerReply {
    match refused {
        Refused::InvalidInput => failure("invalid_input", "arguments", "none", true),
        Refused::NotFound => failure("not_found", "admission", "check_configuration", false),
        Refused::ProfileUnnamed => failure(
            "invalid_configuration",
            "admission",
            "check_configuration",
            false,
        ),
    }
}

/// [`refusal`] for `callable`: on `approvals policy-set`, `prepare` and
/// `issue` an `invalid_input` selection names the argument it refused,
/// `adapter` (contracts/cli/v1alpha1/semantics.md, "The refused argument").
/// Every other command, and every other refusal, is unchanged.
fn refusal_of(callable: &str, refused: Refused) -> HandlerReply {
    let mut reply = refusal(refused);
    if refused == Refused::InvalidInput
        && matches!(
            callable,
            "approval-policy-set" | "approval-prepare" | "approval-issue"
        )
        && let HandlerReply::UsageError { data, .. } = &mut reply
    {
        data["field"] = json!(owner::Argument::Adapter.as_str());
    }
    reply
}

/// The given `--adapter` alias, unresolved: it is refused or served exactly as
/// before. `None` when the input selects otherwise; a conflicting selector is
/// refused.
fn given(input: &Value) -> Result<Option<&str>, Refused> {
    let text = |field| match input.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.as_str())),
        Some(_) => Err(Refused::InvalidInput),
    };
    match (text("adapter")?, text("connector")?, text("instance")?) {
        (Some(alias), None, None) => Ok(Some(alias)),
        (Some(_), _, _) => Err(Refused::InvalidInput),
        _ => Ok(None),
    }
}

/// The alias the selector fields of `input` name under `config`: the given
/// `--adapter`, the instance `--connector` and `--instance` name together, or,
/// with neither, the configuration's `default_instance`, else its
/// `default_adapter`.
pub(super) fn alias(input: &Value, config: &Config) -> Result<String, Refused> {
    if let Some(alias) = given(input)? {
        return Ok(alias.to_owned());
    }
    let text = |field| input.get(field).and_then(Value::as_str);
    let selected = match (text("connector"), text("instance")) {
        (Some(connector), Some(instance)) => config
            .select(Selector::Instance {
                connector,
                instance,
            })
            .ok_or(if config.awaiting_profile(connector, instance).is_some() {
                Refused::ProfileUnnamed
            } else {
                Refused::NotFound
            })?,
        (None, None) => config
            .select(Selector::Default)
            .ok_or(Refused::InvalidInput)?,
        _ => return Err(Refused::InvalidInput),
    };
    Ok(selected.alias.to_owned())
}

/// [`alias`], reading the configuration at `paths` only when no alias is given.
pub(super) fn alias_at(input: &Value, paths: &Paths) -> owner::Result<String> {
    if let Some(alias) = given(input)? {
        return Ok(alias.to_owned());
    }
    Ok(alias(input, &Config::load_at(paths)?)?)
}

/// The input every handler reads: `adapter` holds the selected alias and the
/// new selectors are gone. An input selecting by `--adapter` is unchanged.
/// Returns the configuration read to resolve it, if one was.
pub(super) fn resolve(call: &Invocation<'_>) -> Result<(Value, Option<Config>), HandlerReply> {
    let refused = |refused| refusal_of(call.callable, refused);
    if !selects(call.callable) || given(&call.input).map_err(refused)?.is_some() {
        return Ok((call.input.clone(), None));
    }
    let paths = resolved(call)?;
    let config = Config::read_at(&paths).map_err(configuration_refusal)?;
    let alias = alias(&call.input, &config).map_err(refused)?;
    let mut input = call.input.clone();
    if let Some(fields) = input.as_object_mut() {
        fields.remove("connector");
        fields.remove("instance");
        fields.insert("adapter".into(), json!(alias));
    }
    Ok((input, Some(config)))
}

/// Name the connector and instance beside every alias the result names, where
/// its declared type carries them. `config` is read when not given.
pub(super) fn name(call: &Invocation<'_>, result: &mut Value, config: Option<Config>) {
    let Some(shape) = plan()
        .callables
        .get(call.callable)
        .map(|callable| &callable.result.shape)
    else {
        return;
    };
    if !names(shape) {
        return;
    }
    let config = match config {
        Some(config) => config,
        None => match resolved(call)
            .ok()
            .and_then(|paths| Config::read_at(&paths).ok())
        {
            Some(config) => config,
            None => return,
        },
    };
    fill(result, shape, &config);
}

/// Whether the shape declares `connector` and `instance` anywhere.
fn names(shape: &Shape) -> bool {
    match shape.required() {
        Shape::Struct { fields } => fields.contains_key("connector") || fields.values().any(names),
        Shape::List { of } => names(of),
        Shape::Map { value } => names(value),
        _ => false,
    }
}

fn fill(value: &mut Value, shape: &Shape, config: &Config) {
    match shape.required() {
        Shape::Struct { fields } => {
            let Some(object) = value.as_object_mut() else {
                return;
            };
            if fields.contains_key("connector")
                && fields.contains_key("instance")
                && !object.contains_key("connector")
                && let Some((connector, instance)) = object
                    .get("adapter")
                    .and_then(Value::as_str)
                    .and_then(|alias| config.names(alias))
            {
                let (connector, instance) = (json!(connector), json!(instance));
                object.insert("connector".into(), connector);
                object.insert("instance".into(), instance);
            }
            for (name, shape) in fields {
                if let Some(value) = object.get_mut(name) {
                    fill(value, shape, config);
                }
            }
        }
        Shape::List { of } => {
            for item in value.as_array_mut().into_iter().flatten() {
                fill(item, of, config);
            }
        }
        Shape::Map { value: shape } => {
            for item in value
                .as_object_mut()
                .into_iter()
                .flat_map(|o| o.values_mut())
            {
                fill(item, shape, config);
            }
        }
        _ => {}
    }
}

/// The words before a first `--`: the only ones that can be flags.
fn flags(args: &[OsString]) -> impl Iterator<Item = &str> {
    args.iter()
        .skip(1)
        .map(|arg| arg.to_str().unwrap_or(""))
        .take_while(|arg| *arg != "--")
}

/// Whether the arguments select with the deprecated `--adapter`.
fn adapter_selected(args: &[OsString]) -> bool {
    flags(args).any(|arg| arg == "--adapter" || arg.starts_with("--adapter="))
}

/// The command path the arguments name: the words before any other flag,
/// skipping the process globals, which the parser takes anywhere.
fn path(args: &[OsString]) -> Vec<&str> {
    let globals = &plan().globals;
    let globals: Vec<String> = [globals.config.as_ref(), Some(&globals.state)]
        .into_iter()
        .chain([globals.output.as_ref()])
        .flatten()
        .map(|name| format!("--{name}"))
        .collect();
    let mut words = Vec::new();
    let mut rest = flags(args);
    while let Some(arg) = rest.next() {
        let global = globals.iter().find(|global| {
            arg.strip_prefix(global.as_str())
                .is_some_and(|value| value.is_empty() || value.starts_with('='))
        });
        match global {
            Some(global) if arg == global => {
                rest.next();
            }
            Some(_) => {}
            None if arg.starts_with('-') => break,
            None => words.push(arg),
        }
    }
    words
}

/// The deprecated names the arguments use, in the order their lines and
/// entries appear: a renamed command path, then `--adapter`. A renamed path is
/// any path alias of the plan (`apps/connectors/spec/cli.yaml`), answered with
/// the command's own path.
pub(super) fn deprecations(args: &[OsString]) -> Vec<Deprecation> {
    let words = path(args);
    let renamed = plan().commands.iter().find_map(|command| {
        command
            .aliases
            .iter()
            .find(|alias| words.starts_with(&alias.iter().map(String::as_str).collect::<Vec<_>>()))
            .map(|alias| Deprecation::Command {
                old: alias.join(" "),
                new: command.path.join(" "),
            })
    });
    renamed
        .into_iter()
        .chain(adapter_selected(args).then_some(Deprecation::Adapter))
        .collect()
}

/// Whether the arguments select JSON output, as the generated parser decides
/// it before parsing.
pub(super) fn json_output(args: &[OsString]) -> bool {
    let mut previous = false;
    for arg in flags(args) {
        if arg == "--output=json" || (previous && arg == "json") {
            return true;
        }
        previous = arg == "--output";
    }
    false
}

/// A deprecated name keeps working for the deprecation window and says so
/// once: under `--output human` one line each on standard error; under
/// `--output json` nothing more on standard error, and the one envelope,
/// success or failure, carries `deprecations`. Output that is no envelope
/// (completions, a launched consumer's) is left as it is under `--output json`.
/// Help never reaches here: `run` answers it first, and the help marks a
/// deprecated name itself (`connectors::help`).
pub(super) fn deprecate(output: &mut ProcessOutput, json: bool, deprecations: &[Deprecation]) {
    if !json {
        let lines: String = deprecations.iter().map(Deprecation::line).collect();
        output.stderr.insert_str(0, &lines);
        return;
    }
    let entries: Vec<Value> = deprecations.iter().map(Deprecation::entry).collect();
    for stream in [&mut output.stdout, &mut output.stderr] {
        if let Ok(Value::Object(mut envelope)) = serde_json::from_str::<Value>(stream)
            && envelope.contains_key("ok")
        {
            envelope.insert("deprecations".into(), json!(entries));
            *stream = format!("{}\n", Value::Object(envelope));
        }
    }
}

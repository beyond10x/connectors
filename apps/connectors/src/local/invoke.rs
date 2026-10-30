//! `connectors invoke <connector>[/<instance>] <operation>`, and with
//! `--definition <file>` the target left out
//! (story:connectors-do-short-invocation; `contracts/cli/v1alpha1/semantics.md`
//! §3, "Invoking an operation").
//!
//! The command line is prepared here, before the generated runner reads it.
//! The target names a connector and, after one `/`, an instance; with the
//! instance left out the connector's only configured instance is selected and
//! `default_instance` is not consulted. Without `--connection` the instance's
//! one `ready` connection is selected, else its one usable one ([`decide`]).
//! `--schema` and `--revision`, left out, are the instance's current admitted
//! description's; given, they are pins a differing description refuses. With
//! no input flag the document is read from standard input, unless standard
//! input is a terminal. `--describe` is answered here, from the description (a
//! definition's from its lowering), and reads and invokes nothing; a write
//! without `--approval-file` is refused here, before any provider call. The
//! command line is then rewritten with every value stated and the target as
//! `<connector>/<instance>`, and the generated runner admits and invokes it
//! through the instance's selected entry, the answer naming the connector and instance.
//!
//! `--fields` and `--each` (story:invoke-shapes-its-result) are parsed here and
//! left off the rewritten command line: a malformed path, either beside
//! `--describe` and `--each` on a write are refused before any provider call,
//! and the full answer is shaped ([`Shaping`]) after the generated runner,
//! the owner, the audit and the mutation ledger have it.
use super::*;
use clap::parser::ValueSource;
use connectors::shape;
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
/// The connections one page of an instance's listing reads.
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

/// The connector and instance the entry `alias` is known by: a `[connectors]`
/// instance or recorded definition by its own; a legacy `[adapters.<alias>]`
/// entry by its alias, under the connector `setup migrate` derives for it, or
/// its implementation when that is not the catalog. `None` for a legacy
/// catalog entry no connector is derived for, which is no instance.
pub(super) fn entry_name(
    config: &Config,
    alias: &str,
    adapter: &Adapter,
) -> Option<(String, String)> {
    if config.legacy_aliases().contains(&alias) {
        let connector = match super::migrate::derived_connector(adapter) {
            Some(connector) => connector,
            None if adapter.adapter_id != super::declarations::CATALOG => {
                adapter.adapter_id.clone()
            }
            None => return None,
        };
        Some((connector, alias.to_owned()))
    } else {
        let (connector, instance) = config.names(alias)?;
        Some((connector.to_owned(), instance.to_owned()))
    }
}

/// The aliases of the legacy catalog entries no connector is derived for,
/// sorted: entries of the configuration that are no instance.
pub(super) fn unmigrated(config: &Config) -> Vec<String> {
    config
        .adapters
        .iter()
        .filter(|(alias, adapter)| entry_name(config, alias, adapter).is_none())
        .map(|(alias, _)| alias.clone())
        .collect()
}

/// Every configured instance by the names `invoke` knows it by
/// ([`entry_name`]). A catalog entry no connector is derived for has no name
/// here. A `[connectors]` instance wins over a legacy entry that derives its
/// names, as `Config::select` selects it, so each name is listed once.
fn named(config: &Config) -> Vec<Named> {
    let legacy = config.legacy_aliases();
    let mut named: Vec<Named> = config
        .adapters
        .iter()
        .filter(|(alias, _)| !legacy.contains(&alias.as_str()))
        .filter_map(|(alias, adapter)| {
            let (connector, instance) = entry_name(config, alias, adapter)?;
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
    for alias in legacy {
        let Some((connector, instance)) = entry_name(config, alias, &config.adapters[alias]) else {
            continue;
        };
        if !named
            .iter()
            .any(|n| n.connector == connector && n.instance == instance)
        {
            named.push(Named {
                connector,
                instance,
                alias: Some(alias.to_owned()),
            });
        }
    }
    named
}

/// The connector and instance of every entry [`named`] lists, by alias: the
/// instances `instances list` lists and `--connector`/`--instance` select,
/// each name once. A legacy entry whose names a `[connectors]` instance holds
/// is not among them, nor is a legacy catalog entry no connector is derived
/// for (story:connectors-do-short-invocation).
pub(super) fn instances(config: &Config) -> std::collections::BTreeMap<String, (String, String)> {
    named(config)
        .into_iter()
        .filter_map(|n| Some((n.alias?, (n.connector, n.instance))))
        .collect()
}

/// Why a target selects no entry.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Unselected {
    /// Several instances and the target names none: each `<connector>/<instance>`, sorted.
    Ambiguous(Vec<String>),
    NotFound,
    ProfileUnnamed,
    /// The target names a legacy catalog entry no connector is derived for by
    /// the name `catalog` gave it.
    Unmigrated,
}

/// The connector, instance and alias the target selects under `config`: the
/// selection of `invoke` and of `--connector`/`--instance` on every other
/// command (`select::alias`).
pub(super) fn select(
    config: &Config,
    connector: &str,
    instance: Option<&str>,
) -> Result<(String, String, String), Unselected> {
    let matching: Vec<Named> = named(config)
        .into_iter()
        .filter(|n| n.connector == connector && instance.is_none_or(|i| n.instance == i))
        .collect();
    match matching.as_slice() {
        [] if names_unmigrated(config, connector, instance) => Err(Unselected::Unmigrated),
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

/// Whether `connector` and `instance` name a legacy catalog entry no connector
/// is derived for by the name `catalog` gave it: `catalog/<alias>`, or
/// `catalog` alone while there is one.
pub(super) fn names_unmigrated(config: &Config, connector: &str, instance: Option<&str>) -> bool {
    let unmigrated = unmigrated(config);
    connector == super::declarations::CATALOG
        && instance.map_or(!unmigrated.is_empty(), |instance| {
            unmigrated.iter().any(|alias| alias == instance)
        })
}

fn unselected(refused: Unselected) -> HandlerReply {
    match refused {
        Unselected::Unmigrated => configuration_refusal(Refusal::Rule(Rule::LegacyEntryUnmigrated)),
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
            Unselected::ProfileUnnamed | Unselected::Unmigrated => {
                Code::InvalidConfiguration.into()
            }
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

/// The connection an `invoke` without `--connection` uses (D3 amended): the
/// instance's one `ready` connection; with none ready, its one usable
/// connection, `pending` only because its validation evidence lapsed under the
/// current configuration, which the provider re-checks before the call
/// (`owner::admit_invoke`). Revoked connections are not held. Otherwise (none
/// usable, several ready, or several usable and none ready) it is
/// `connection_unresolved` listing every connection held with its state:
/// naming a connect when none is held, a repair when none held is usable,
/// nothing when `--connection` must choose.
pub(super) fn decide(held: &[Held]) -> Result<String, HandlerReply> {
    let ready: Vec<&Held> = held.iter().filter(|h| h.state == State::Ready).collect();
    let usable: Vec<&Held> = held
        .iter()
        .filter(|h| h.state == State::Ready || (h.state == State::Pending && h.current))
        .collect();
    if let ([one], _) | ([], [one]) = (ready.as_slice(), usable.as_slice()) {
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

/// The connections the instance of `adapter` holds, revoked ones excepted:
/// every page of the listing is read, so no number of revoked connections
/// hides a usable one.
fn held(paths: &Paths, config: &Config, adapter: &Adapter) -> Result<Vec<Held>, HandlerReply> {
    let registry = Registry::with_system_clock(&paths.state);
    let custody = keyring::custody::available_for(&config.selection());
    every_page(|cursor| {
        registry
            .list(
                &adapter.instance_id,
                &adapter.adapter_id,
                adapter.configured(),
                registry::PageOptions {
                    limit: LISTED,
                    cursor,
                },
                connectors_sdk::now_ms(),
                custody,
            )
            .map_err(super::connections::registry_failure)
    })
}

/// The connections every page `list` answers holds, revoked ones excepted,
/// each page asked for with the cursor the one before it answered.
fn every_page(
    mut list: impl FnMut(Option<&str>) -> Result<registry::Page, HandlerReply>,
) -> Result<Vec<Held>, HandlerReply> {
    let mut held = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let page = list(cursor.as_deref())?;
        held.extend(
            page.connections
                .into_iter()
                .filter(|c| c.state != State::Revoked)
                .map(|c| Held {
                    connection: c.reference,
                    state: c.state,
                    current: !c.stale,
                }),
        );
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => return Ok(held),
        }
    }
}

/// What an `invoke` command line asks for.
struct Request {
    program: OsString,
    globals: Vec<OsString>,
    json: bool,
    config: Option<PathBuf>,
    state: Option<PathBuf>,
    /// `None` when one positional is given: it is the operation, and only a
    /// `--definition` names the connector then.
    target: Option<String>,
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
    fields: Option<String>,
    each: Option<String>,
    /// The input source as given, for a walk, which reads the document once.
    document: Option<Document>,
    walk: bool,
    max_pages: Option<String>,
    out: Option<String>,
}

/// Where an `invoke`'s input document comes from.
#[derive(Clone)]
enum Document {
    Inline(OsString),
    File(PathBuf),
    Stdin,
}

/// What `--fields` and `--each` ask of a successful answer
/// (`contracts/cli/v1alpha1/semantics.md`, "Shaping a result").
pub(super) struct Shaping {
    fields: Option<shape::Fields>,
    each: Option<shape::Path>,
    json: bool,
}

impl Shaping {
    /// Standard output for `answer`, the full `InvokeResult` the runner
    /// printed: each record of `--each` on its own line, then the shaped
    /// envelope. `None` when it stays printed whole (`shape::answer`).
    pub(super) fn render(&self, answer: &Value) -> Option<String> {
        let shaped = shape::answer(answer, self.fields.as_ref(), self.each.as_ref())?;
        let mut stdout: String = shaped
            .records
            .iter()
            .map(|record| format!("{record}\n"))
            .collect();
        stdout.push_str(&success(shaped.result, self.json).stdout);
        Some(stdout)
    }
}

/// The generated command with both positionals, `--schema`, `--revision` and
/// the input source of `invoke` optional: the preparation states them. A lone
/// positional is the operation ([`parse`]); clap's missing-positional mode is
/// not used, as it moves a value given before an option, or after `--`, to the
/// operation.
fn relaxed() -> clap::Command {
    connectors_cli_contract::command().mut_subcommand("invoke", |invoke| {
        invoke
            .mut_arg("field:target", |arg| arg.required(false))
            .mut_arg("field:operation", |arg| arg.required(false))
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
    // A lone positional is the operation: the target is left out.
    let (target, operation) = match (text("target"), text("operation")) {
        (target, Some(operation)) => (target, operation),
        (Some(operation), None) => (None, operation),
        (None, None) => return None,
    };
    let flag = |field: &str| match text(field).as_deref() {
        None | Some("false") => Some(false),
        Some("true") => Some(true),
        // Not a Boolean: the generated runner refuses it.
        Some(_) => None,
    };
    let describe = flag("describe")?;
    let walk = flag("walk")?;
    let given_raw = |id: &str| {
        given(id)
            .then(|| leaf.get_raw(id).and_then(|mut values| values.next()))
            .flatten()
            .map(OsString::from)
    };
    let document = given_raw("source:input:inline")
        .map(Document::Inline)
        .or_else(|| given_raw("source:input:file").map(|path| Document::File(path.into())))
        .or_else(|| {
            leaf.get_flag("source:input:stdin")
                .then_some(Document::Stdin)
        });
    Some(Request {
        program: args.first()?.clone(),
        globals,
        json: matches
            .get_one::<String>("global_output")
            .map(String::as_str)
            == Some("json"),
        config: matches.get_one::<PathBuf>("global_config").cloned(),
        state: matches.get_one::<PathBuf>("global_state").cloned(),
        target,
        operation,
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
        fields: text("fields"),
        each: text("each"),
        document,
        walk,
        max_pages: text("max_pages"),
        out: text("out"),
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
/// an `invoke`, which is prepared and answered as the rewritten command line,
/// with the shaping its answer takes when `--fields` or `--each` is given;
/// `--describe` is answered here, and a refusal is the one failure envelope.
pub(super) fn prepare(
    args: Vec<OsString>,
) -> Result<(Vec<OsString>, Option<Shaping>), ProcessOutput> {
    let Some(request) = parse(&args) else {
        return Ok((args, None));
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
        Ok(Prepared::Run(args, shaping)) => Ok((args, shaping)),
        Ok(Prepared::Described(result)) => Err(success(result, json)),
        Ok(Prepared::Walk(walking)) => Err(walking.run()),
        Err(reply) => Err(definition::render(reply, json)),
    }
}

enum Prepared {
    Run(Vec<OsString>, Option<Shaping>),
    Described(Value),
    Walk(Box<Walking>),
}

/// The caller's document of a walk: JSON, or text that does not decode under
/// the strict reader (a key named twice included), which one plain invoke of
/// it refuses as it refuses any.
enum Caller {
    Json(Value),
    Raw(OsString),
}

/// Where a walk writes its records.
enum Out {
    Stdout,
    File { file: std::fs::File, path: PathBuf },
}

impl Out {
    /// `--out`: `-`, or a new file created owner-only, never over an
    /// existing path.
    fn create(out: &str) -> Result<Out, HandlerReply> {
        if out == "-" {
            return Ok(Out::Stdout);
        }
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let file = options.open(out).map_err(|_| refused("out"))?;
        Ok(Out::File {
            file,
            path: out.into(),
        })
    }

    /// One page's lines written whole or not at all: a file is cut back to
    /// its length before the page when a write fails partway. Standard
    /// output cannot be taken back, so a reader of `--out -` may see part of
    /// the page the summary does not count.
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        use std::io::{Seek, SeekFrom, Write};
        match self {
            Out::Stdout => {
                let mut stdout = std::io::stdout().lock();
                stdout.write_all(bytes)?;
                stdout.flush()
            }
            Out::File { file, .. } => {
                let before = file.stream_position()?;
                let written = file.write_all(bytes);
                if written.is_err() {
                    let _ = file.set_len(before);
                    let _ = file.seek(SeekFrom::Start(before));
                }
                written
            }
        }
    }

    /// The file this walk created, removed while it is still the one created
    /// and empty: a walk that read nothing leaves nothing behind.
    fn discard(self) {
        #[cfg(unix)]
        if let Out::File { file, path } = self {
            use std::os::unix::fs::MetadataExt;
            if let (Ok(ours), Ok(there)) = (file.metadata(), std::fs::symlink_metadata(&path))
                && (ours.dev(), ours.ino()) == (there.dev(), there.ino())
                && there.len() == 0
            {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

/// An admitted `invoke --walk`: each page is the rewritten command line with
/// the page's input, run as a plain invoke is (`super::run_selected`).
pub(super) struct Walking {
    /// The command line before the input argument, and after it.
    before: Vec<OsString>,
    after: Vec<OsString>,
    walk: connectors::walk::Walk,
    caller: Caller,
    max_pages: u32,
    fields: Option<shape::Fields>,
    out: Out,
    /// `--out` as given, which the summary names.
    named: String,
    json: bool,
}

impl Walking {
    /// The walk run, its records written to `--out` and its summary as the
    /// output; a refused first page is that page's own output.
    fn run(self) -> ProcessOutput {
        let Walking {
            before,
            after,
            walk,
            caller,
            max_pages,
            fields,
            mut out,
            named,
            json,
        } = self;
        let page = |input: OsString| {
            let mut args = before.clone();
            args.push(input);
            args.extend(after.iter().cloned());
            super::run_selected(args, None)
        };
        let caller = match caller {
            Caller::Json(caller) => caller,
            Caller::Raw(input) => {
                out.discard();
                return page(input);
            }
        };
        let mut refused_output = None;
        let walked = connectors::walk::run(
            &walk,
            &caller,
            max_pages,
            fields.as_ref(),
            |input| {
                let output = page(format!("--input-json={input}").into());
                // The page's InvokeResult, in the envelope of the output mode;
                // its provider `result` is the page.
                if output.exit_code == 0
                    && let Ok(mut answer) = serde_json::from_str::<Value>(&output.stdout)
                {
                    let invoked = if json {
                        answer.get_mut("result").map(Value::take)
                    } else {
                        Some(answer)
                    };
                    if let Some(result) =
                        invoked.and_then(|mut invoked| invoked.get_mut("result").map(Value::take))
                    {
                        return connectors::walk::Page::Answered(result);
                    }
                }
                let error = page_error(&output, json);
                refused_output = Some(output);
                connectors::walk::Page::Refused(error)
            },
            |records| {
                let mut lines = String::new();
                for record in records {
                    lines.push_str(&record.to_string());
                    lines.push('\n');
                }
                out.write(lines.as_bytes()).map_err(|_| "write")
            },
        );
        if walked.pages == 0
            && let Some(connectors::walk::Stopped::Refused(_)) = walked.stopped
            && let Some(output) = refused_output
        {
            out.discard();
            return output;
        }
        let mut summary = walked.summary();
        summary.insert("out".into(), json!(named));
        success(Value::Object(summary), json)
    }
}

/// The failure envelope's `error` a refused page printed: `{code, data}` from
/// its JSON envelope, or from the human `code` or `code: data` line; a page
/// that printed no failure and no readable answer is `cli_result`.
fn page_error(output: &ProcessOutput, json: bool) -> Value {
    let text = output.stderr.trim_end();
    if text.is_empty() {
        return json!({"code": "cli_result", "data": {}});
    }
    if json && let Ok(mut envelope) = serde_json::from_str::<Value>(text) {
        return envelope["error"].take();
    }
    let (code, data) = match text.split_once(": ") {
        Some((code, data)) => (
            code,
            serde_json::from_str(data).unwrap_or_else(|_| json!({})),
        ),
        None => (text, json!({})),
    };
    json!({"code": code, "data": data})
}

/// The caller's document of a walk, read once from its source.
fn caller_document(document: Option<&Document>) -> Result<Caller, HandlerReply> {
    let read = |path: Option<&std::path::Path>| {
        protected::document(path).map_err(|error| early_failure(error, &Value::Null))
    };
    let text = match document {
        Some(Document::Inline(text)) => match text.to_str() {
            Some(text) => text.to_owned(),
            None => {
                let mut arg = OsString::from("--input-json=");
                arg.push(text);
                return Ok(Caller::Raw(arg));
            }
        },
        Some(Document::File(path)) => read(Some(path))?,
        Some(Document::Stdin) | None => read(None)?,
    };
    // The repository's strict reader: a document naming a key twice is not
    // walked, and the one plain invoke of it refuses it as it refuses any.
    Ok(match connectors_core::read_json::<Value>(text.as_bytes()) {
        Ok(value) => Caller::Json(value),
        Err(_) => Caller::Raw(format!("--input-json={text}").into()),
    })
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
    let named = match &request.target {
        Some(text) => Some(target(text).ok_or_else(|| refused("target"))?),
        // A definition file names its connector and instance itself.
        None if request.definition.is_some() => None,
        None => return Err(refused("target")),
    };
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
    // The paths are parsed once, before anything is read; `--describe` reads
    // no result to shape.
    let fields = match request.fields.as_deref() {
        Some(_) if request.describe => return Err(refused("fields")),
        Some(text) => Some(shape::Fields::parse(text).map_err(|_| refused("fields"))?),
        None => None,
    };
    let each = match request.each.as_deref() {
        Some(_) if request.describe => return Err(refused("each")),
        Some(text) => Some(shape::Path::parse(text).map_err(|_| refused("each"))?),
        None => None,
    };
    // A walk's own arguments (story:invoke-walks-declared-pages): its records
    // are one per line already, and `--describe` reads no page.
    let walking = if request.walk {
        if request.describe {
            return Err(refused("walk"));
        }
        if each.is_some() {
            return Err(refused("each"));
        }
        let max_pages = connectors::walk::max_pages(request.max_pages.as_deref())
            .map_err(|_| refused("max_pages"))?;
        match request.out.as_deref() {
            Some(out) if !out.is_empty() => Some((max_pages, out)),
            _ => return Err(refused("out")),
        }
    } else {
        for (field, value) in [("max_pages", &request.max_pages), ("out", &request.out)] {
            if value.is_some() {
                return Err(refused(field));
            }
        }
        None
    };
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
    let lowering = || definition::Request {
        root: request.definitions_root.clone(),
        credential_file: request.credential_file.clone(),
        selected: false,
        target: named
            .map(|(connector, instance)| (connector.to_owned(), instance.map(str::to_owned))),
        operation: request.operation.clone(),
        connection: request.connection.clone(),
        config: request.config.clone(),
        state: request.state.clone(),
    };
    let (connector, instance, adapter, bootstrap, connection) = match &request.definition {
        // A definition is described from its lowering: nothing is recorded,
        // and no connection is resolved, asked for or created.
        Some(definition) if request.describe => {
            let lowered = definition::lower(&lowering(), definition, signals)
                .map_err(|reply| naming(reply, "definition"))?;
            let mut bootstrap = lowered.reported;
            bootstrap.configure(&lowered.adapter);
            let bootstrap = bootstrap
                .bound(None)
                .map_err(|failure| owner_failure(failure.into()))?;
            (
                lowered.connector,
                lowered.instance,
                lowered.adapter,
                bootstrap,
                None,
            )
        }
        Some(definition) => {
            let prepared = definition::prepare_with(&lowering(), definition, signals)
                .map_err(|reply| naming(reply, "definition"))?;
            let connection = Some(prepared.connection);
            let (adapter, bootstrap) = bound(&paths, &prepared.alias, connection.as_deref())?;
            (
                prepared.connector,
                prepared.instance,
                adapter,
                bootstrap,
                connection,
            )
        }
        None => {
            let Some((connector, instance)) = named else {
                return Err(refused("target"));
            };
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
            let (adapter, bootstrap) = bound(&paths, &alias, connection.as_deref())?;
            (connector, instance, adapter, bootstrap, connection)
        }
    };
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
    let shown = match effect {
        Effect::Read => "read",
        Effect::Write => "write",
        // No admitted description leaves an effect unknown
        // (`Bootstrap::validate_for`); one that did is refused, never shown.
        Effect::Unknown => return Err(owner_failure(Code::Unsupported.into())),
    };
    if request.describe {
        let operation = descriptor
            .operation(&request.operation)
            .map_err(|_| owner_failure(Code::NotFound.into()))?;
        return Ok(Prepared::Described(described(
            &connector, &instance, operation, shown, schema, revision,
        )));
    }
    match effect {
        // A write is invoked once, never page by page.
        Effect::Write if walking.is_some() => return Err(refused("walk")),
        // A write prints its one answer: `--each` would split it.
        Effect::Write if each.is_some() => return Err(refused("each")),
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
    // An operation id may begin with `-`: after `--` both are values.
    let after: Vec<OsString> = vec![
        "--".into(),
        format!("{connector}/{instance}").into(),
        request.operation.clone().into(),
    ];
    if let Some((max_pages, out)) = walking {
        // The walk the description declares; one that declares none, or a
        // carried value that is no walk, has no pages to read.
        let walk = descriptor
            .operation(&request.operation)
            .ok()
            .and_then(|operation| connectors::walk::Walk::declared(&operation.input_schema).ok())
            .flatten()
            .ok_or_else(|| {
                let mut reply = owner_failure(Code::Unsupported.into());
                if let HandlerReply::Error { data, .. } | HandlerReply::UsageError { data, .. } =
                    &mut reply
                {
                    data["field"] = json!("walk");
                }
                reply
            })?;
        let caller = caller_document(request.document.as_ref())?;
        // Created last, once nothing before the first page can refuse.
        let out_file = Out::create(out)?;
        return Ok(Prepared::Walk(Box::new(Walking {
            before: rewritten,
            after,
            walk,
            caller,
            max_pages,
            fields,
            out: out_file,
            named: out.to_owned(),
            json: request.json,
        })));
    }
    rewritten.extend(input);
    rewritten.extend(request.approval_file.iter().cloned());
    if let Some(key) = &request.idempotency_key {
        rewritten.push(format!("--idempotency-key={key}").into());
    }
    rewritten.extend(after);
    let shaping = (fields.is_some() || each.is_some()).then_some(Shaping {
        fields,
        each,
        json: request.json,
    });
    Ok(Prepared::Run(rewritten, shaping))
}

/// `invoke --describe`'s answer (`connectors.cli.InvokeDescription`).
fn described(
    connector: &str,
    instance: &str,
    operation: &connectors_core::Operation,
    effect: &str,
    schema: String,
    revision: String,
) -> Value {
    json!({"connector":connector,"instance":instance,"operation":operation.id,
        "description":operation.description,"effect":effect,
        "schema":schema,"revision":revision,"input_schema":operation.input_schema})
}

/// The configured entry under `alias` and its description as `connection`
/// reads it (`owner::bound_described`).
fn bound(
    paths: &Paths,
    alias: &str,
    connection: Option<&str>,
) -> Result<(Adapter, connectors_host::local::runtime::Bootstrap), HandlerReply> {
    let bootstrap = owner::bound_described(paths, alias, connection).map_err(owner_failure)?;
    let adapter = Config::load_at(paths)
        .map_err(|failure| owner_failure(failure.into()))?
        .adapters
        .get(alias)
        .cloned()
        .ok_or_else(|| unselected(Unselected::NotFound))?;
    Ok((adapter, bootstrap))
}

/// The `invoke` handler: the target's instance invoked through its selected
/// entry (`operations::dispatch_tagged`), the answer naming the connector and
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

    /// story:connectors-do-short-invocation, D3 amended: the one `ready`
    /// connection is selected, even beside usable ones that are not ready;
    /// with none ready, the one usable connection (`pending` and current, its
    /// lapsed evidence re-checked before the call). None usable, several
    /// ready, or several usable and none ready is `connection_unresolved`
    /// listing every one held.
    #[test]
    fn the_one_ready_else_the_one_usable_connection_is_selected_and_otherwise_listed() {
        assert_eq!(
            decide(&[held("a", State::Ready, true)]).ok(),
            Some("a".into())
        );
        assert_eq!(
            decide(&[held("a", State::Pending, true)]).ok(),
            Some("a".into()),
            "story:connectors-do-short-invocation: D3 amended: lapsed evidence is re-checked before the call"
        );
        assert_eq!(
            decide(&[
                held("a", State::Pending, true),
                held("b", State::Ready, true),
            ])
            .ok(),
            Some("b".into()),
            "story:connectors-do-short-invocation: D3 amended: one ready beside one lapsed"
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
        let lapsed = data(
            decide(&[
                held("a", State::Pending, true),
                held("b", State::Pending, true),
            ])
            .unwrap_err(),
        );
        assert_eq!(
            lapsed,
            json!({"kind":"operational","code":"connection_unresolved","stage":"admission",
                "next_action":"none","connections":[
                    {"connection":"a","state":"pending"},
                    {"connection":"b","state":"pending"}]}),
            "story:connectors-do-short-invocation: D3 amended: several usable and none ready"
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

    /// story:connectors-do-short-invocation: the connections held are read
    /// from every page of the listing, so revoked connections filling the
    /// first page never hide the usable one on a later page.
    #[test]
    fn every_page_of_the_listing_is_read() {
        let observed = |reference: &str, state: State| registry::ObservedConnection {
            reference: reference.into(),
            instance: "fixture-instance".into(),
            profile: "token".into(),
            revision: "r1".into(),
            state,
            identity: None,
            observed_at_ms: 1,
            valid_until_ms: 2,
            stale: false,
            binding: None,
            granted_scopes: None,
            credential_expires_at_ms: None,
        };
        let mut asked = Vec::new();
        let held = every_page(|cursor| {
            asked.push(cursor.map(str::to_owned));
            let (connections, next_cursor) = match cursor {
                None => (
                    vec![observed("a", State::Revoked), observed("b", State::Revoked)],
                    Some("page-2".to_owned()),
                ),
                Some("page-2") => (vec![observed("c", State::Ready)], None),
                Some(other) => panic!("an unknown cursor {other}"),
            };
            Ok(registry::Page {
                connections,
                next_cursor,
                observed_at_ms: 1,
                valid_until_ms: 2,
                stale: false,
            })
        })
        .unwrap_or_else(|_| panic!("story:connectors-do-short-invocation: every page is read"));
        assert_eq!(asked, [None, Some("page-2".to_owned())]);
        assert_eq!(
            decide(&held).ok(),
            Some("c".into()),
            "story:connectors-do-short-invocation: {held:?}"
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

    /// story:invoke-walks-declared-pages: a refused page's `error` is read
    /// from its failure envelope in JSON mode and from its `code` or
    /// `code: data` line in human mode; a page with neither answer nor
    /// failure is `cli_result`.
    #[test]
    fn a_refused_page_error_is_read_in_either_output_mode() {
        let refused = |stderr: &str| ProcessOutput {
            exit_code: 1,
            stdout: String::new(),
            stderr: stderr.into(),
        };
        let error = json!({"code":"failure","data":{"code":"service_failure",
            "service_code":"rate_limited"}});
        assert_eq!(
            page_error(
                &refused(&format!("{}\n", json!({"ok":false,"error":error}))),
                true
            ),
            error
        );
        assert_eq!(
            page_error(&refused(&format!("failure: {}\n", error["data"])), false),
            error
        );
        assert_eq!(
            page_error(&refused("cli_interrupted\n"), false),
            json!({"code":"cli_interrupted","data":{}})
        );
        assert_eq!(
            page_error(&refused(""), true),
            json!({"code":"cli_result","data":{}})
        );
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

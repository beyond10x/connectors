//! `operations invoke --definition <file>` (story:invoke-definition-file;
//! architecture-decision-record:provider-definitions-and-runtimes D12 point 3;
//! `contracts/cli/v1alpha1/semantics.md` §3, "Invoking a definition file").
//!
//! A command line that names `--definition` is prepared here, before the
//! generated runner reads it. The definition is lowered with the reference
//! lowerer into an owner-only cache under the state directory, keyed by the
//! digest of the definition and its source; the installed catalog provider is
//! pinned on the lowered native configuration and that entry is recorded beside
//! the operator's configuration as the instance `definition` of the
//! definition's connector (`connectors_host::local::definitions`). When the
//! instance holds no connection the command asks once whether to connect and,
//! on yes, runs the profile's own acquisition; when another instance of the
//! connector holds a connection whose credential can be reused, the question
//! lists those, and a chosen one is copied and validated by the owner
//! (story:invoke-definition-reuse-credential). The command line is then
//! rewritten to select that instance and its connection, with the schema and
//! revision of the lowered description where the caller gave none, and the
//! generated runner admits, approves, audits and invokes it as it does any
//! configured instance's. The operator's configuration file is never written.
use super::*;
use clap::parser::ValueSource;
use connectors_catalog::definition as lowering;
use connectors_cli_contract::ProcessOutput;
use connectors_host::local::{
    definitions, filesystem, metadata::Metadata, oauth, protected, registry::Registry,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    path::{Component, Path, PathBuf},
};

/// The native configuration file in each cache entry.
const NATIVE: &str = definitions::NATIVE;
/// The most bytes a definition file may hold.
const DEFINITION_LIMIT: u64 = 1024 * 1024;
/// The most bytes a vendored source or amendments file is read for its digest.
const SOURCE_LIMIT: u64 = 256 * 1024 * 1024;
/// How long the connect question waits for its answer.
const QUESTION_MS: u64 = 300_000;
/// The fields this step reads, rewrites or removes; every other argument of the
/// command line is kept as it was given.
const OWNED: [&str; 10] = [
    "adapter",
    "connector",
    "instance",
    "connection",
    "operation",
    "schema",
    "revision",
    "definition",
    "definitions_root",
    "credential_file",
];

/// What the command line asks for.
struct Request {
    /// The program name the command line was given under.
    program: OsString,
    definition: Option<PathBuf>,
    root: Option<PathBuf>,
    credential_file: Option<PathBuf>,
    /// `--adapter`, `--connector` or `--instance` was given.
    selected: bool,
    operation: String,
    connection: Option<String>,
    schema: Option<String>,
    revision: Option<String>,
    config: Option<PathBuf>,
    state: Option<PathBuf>,
    json: bool,
    /// The process globals as given, then the kept arguments.
    globals: Vec<OsString>,
    kept: Vec<OsString>,
}

/// The arguments the generated runner reads: `args` unchanged unless they are
/// an `operations invoke` naming `--definition`, which is prepared and answered
/// as the rewritten command line, or refused with the one failure envelope.
pub(super) fn prepare(args: Vec<OsString>) -> Result<Vec<OsString>, ProcessOutput> {
    let Some(request) = parse(&args) else {
        return Ok(args);
    };
    let usage = || failure("invalid_input", "arguments", "none", true);
    let prepared = match &request.definition {
        Some(definition) => run(&request, definition),
        // `--definitions-root` and `--credential-file` belong to `--definition`.
        None if request.root.is_some() || request.credential_file.is_some() => Err(usage()),
        None => return Ok(args),
    };
    let json = request.json;
    prepared.map_err(|reply| render(reply, json))
}

/// The generated command with `--schema` and `--revision` of `operations
/// invoke` optional, which only a definition may leave out.
fn relaxed() -> clap::Command {
    connectors_cli_contract::command().mut_subcommand("operations", |group| {
        group.mut_subcommand("invoke", |invoke| {
            invoke
                .mut_arg("field:schema", |arg| arg.required(false))
                .mut_arg("field:revision", |arg| arg.required(false))
        })
    })
}

/// The request of an `operations invoke` command line, `None` for any other
/// command line or one the relaxed command does not parse: the generated
/// runner then refuses or runs it as before.
fn parse(args: &[OsString]) -> Option<Request> {
    let command = relaxed();
    let matches = command.clone().try_get_matches_from(args).ok()?;
    let ("operations", group) = matches.subcommand()? else {
        return None;
    };
    let ("invoke", leaf) = group.subcommand()? else {
        return None;
    };
    let given = |id: &str| leaf.value_source(id) == Some(ValueSource::CommandLine);
    let text = |field: &str| {
        let id = format!("field:{field}");
        given(&id)
            .then(|| leaf.get_one::<String>(&id).cloned())
            .flatten()
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
    let invoke = command
        .find_subcommand("operations")?
        .find_subcommand("invoke")?;
    let mut kept = Vec::new();
    for arg in invoke.get_arguments() {
        let id = arg.get_id().as_str();
        let owned = id
            .strip_prefix("field:")
            .is_some_and(|field| OWNED.contains(&field));
        if owned || !given(id) {
            continue;
        }
        let long = arg.get_long()?;
        if arg.get_action().takes_values() {
            let value = leaf.get_raw(id)?.next()?;
            let mut arg = OsString::from(format!("--{long}="));
            arg.push(value);
            kept.push(arg);
        } else {
            kept.push(OsString::from(format!("--{long}")));
        }
    }
    let json = matches
        .get_one::<String>("global_output")
        .map(String::as_str)
        == Some("json");
    Some(Request {
        program: args.first()?.clone(),
        definition: text("definition").map(PathBuf::from),
        root: text("definitions_root").map(PathBuf::from),
        credential_file: text("credential_file").map(PathBuf::from),
        selected: ["adapter", "connector", "instance"]
            .iter()
            .any(|field| text(field).is_some()),
        operation: text("operation")?,
        connection: text("connection"),
        schema: text("schema"),
        revision: text("revision"),
        config: matches.get_one::<PathBuf>("global_config").cloned(),
        state: matches.get_one::<PathBuf>("global_state").cloned(),
        json,
        globals,
        kept,
    })
}

/// The one failure envelope a refused preparation writes, as the generated
/// runner writes a handler's.
fn render(reply: HandlerReply, json: bool) -> ProcessOutput {
    let (code, data, usage) = match reply {
        HandlerReply::UsageError { code, data } => (code, data, true),
        HandlerReply::Error { code, data } => (code, data, false),
        _ => (
            "failure".to_owned(),
            json!({"kind":"operational","code":"unavailable","stage":"admission","next_action":"retry_explicitly"}),
            false,
        ),
    };
    let exit_code = if data["kind"] == "interrupted" {
        130
    } else if usage {
        2
    } else {
        1
    };
    let stderr = if json {
        format!(
            "{}\n",
            json!({"ok":false,"error":{"code":code,"data":data}})
        )
    } else {
        format!("{code}: {data}\n")
    };
    ProcessOutput {
        exit_code,
        stdout: String::new(),
        stderr,
    }
}

fn usage() -> HandlerReply {
    failure("invalid_input", "arguments", "none", true)
}

/// Prepare `definition` and answer the rewritten command line.
///
/// The interrupt handler is installed before anything else runs: reading the
/// configuration may start the installed provider to derive an unpinned
/// instance, and pinning the definition's entry starts it twice, so no child
/// is ever started before an interruption is caught. An interruption at any
/// point of the preparation ends the command as interrupted, whatever the step
/// it arrived in answered, refusals included.
fn run(request: &Request, definition: &Path) -> Result<Vec<OsString>, HandlerReply> {
    let signals = protected::Signals::install().map_err(owner_failure)?;
    let prepared = prepare_with(request, definition, &signals);
    if signals.interrupted() {
        return Err(owner_failure(owner::Code::Interrupted.into()));
    }
    prepared
}

/// [`run`], under the interrupt handler `signals`.
fn prepare_with(
    request: &Request,
    definition: &Path,
    signals: &protected::Signals,
) -> Result<Vec<OsString>, HandlerReply> {
    // A third selector: never beside `--adapter` or `--connector`/`--instance`.
    if request.selected {
        return Err(usage());
    }
    let paths = Paths::resolve(request.config.as_deref(), request.state.as_deref())
        .map_err(|_| configuration_refusal(Refusal::Rule(Rule::ConfigPathUnresolved)))?;
    let config = Config::read(&paths.config).map_err(configuration_refusal)?;
    let file = std::path::absolute(definition).map_err(|_| usage())?;
    let bytes = read_bounded(&file, DEFINITION_LIMIT).ok_or_else(usage)?;
    let parsed = lowering::parse(&bytes).map_err(|_| usage())?;
    let stem = file
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or_else(usage)?
        .to_owned();
    let connector = parsed.id.clone().unwrap_or_else(|| stem.clone());
    let alias = definitions::alias(&connector);
    // The connector, the file stem and the catalog (the stem when the file
    // names none) name an alias, the selection file and the bundle's
    // directory: plain names only, checked before anything is written, so none
    // can step out of a cache entry or another alias.
    if !plain(&connector)
        || !plain(&stem)
        || !plain(parsed.catalog.as_deref().unwrap_or(&stem))
        || !connectors_core::valid_id(&alias)
    {
        return Err(usage());
    }
    // An instance the operator configured is never replaced.
    if definitions::collides(&config, &connector) {
        return Err(configuration_refusal(Refusal::Rule(
            Rule::DefinitionInstanceConfigured,
        )));
    }
    let root = match &request.root {
        Some(root) => std::path::absolute(root).map_err(|_| usage())?,
        None => file
            .parent()
            .and_then(Path::parent)
            .ok_or_else(usage)?
            .to_owned(),
    };
    // The state directory `setup init` made holds the cache.
    Metadata::inspect(&paths.state).map_err(host_failure)?;
    // Nothing after an interruption runs (`interrupted`).
    let interrupted = || {
        if signals.interrupted() {
            Err(owner_failure(owner::Code::Interrupted.into()))
        } else {
            Ok(())
        }
    };
    let cached = cache(&paths, &file, &root, &bytes, &parsed, &alias);
    interrupted()?;
    let (digest, native) = cached?;
    let derived = definitions::derive(&connector, &native);
    interrupted()?;
    let (adapter, reported) = derived.map_err(host_failure)?;
    let recorded = definitions::record(
        &paths,
        &connector,
        &definitions::Entry {
            digest,
            adapter: adapter.clone(),
        },
    )
    .map_err(host_failure);
    interrupted()?;
    recorded?;
    // What the command selects from here on is the entry just recorded: an
    // instance the operator configured meanwhile wins, and another run that
    // recorded another file of this connector since is a conflict to retry.
    let held = Config::read_at(&paths).map_err(configuration_refusal);
    interrupted()?;
    let held = held?;
    if held.definitions.get(&alias) != Some(&connector) {
        return Err(configuration_refusal(Refusal::Rule(
            Rule::DefinitionInstanceConfigured,
        )));
    }
    if held.adapters.get(&alias).map(|a| a.selection()) != Some(adapter.selection()) {
        return Err(host_failure(Failure::ConcurrentRevision));
    }
    interrupted()?;
    let connection = match &request.connection {
        Some(connection) => Ok(connection.clone()),
        None => connection(&paths, &connector, &alias, &adapter, &reported, request),
    };
    interrupted()?;
    let connection = connection?;
    let (schema, revision) = match (&request.schema, &request.revision) {
        (Some(schema), Some(revision)) => (schema.clone(), revision.clone()),
        (schema, revision) => {
            let described = described(
                &paths,
                &alias,
                &adapter,
                reported,
                &connection,
                &request.operation,
            );
            interrupted()?;
            let (described_schema, described_revision) = described?;
            (
                schema.clone().unwrap_or(described_schema),
                revision.clone().unwrap_or(described_revision),
            )
        }
    };
    interrupted()?;
    let mut rewritten = vec![request.program.clone()];
    rewritten.extend(request.globals.iter().cloned());
    rewritten.extend(["operations", "invoke"].map(OsString::from));
    for (long, value) in [
        ("connector", connector.as_str()),
        ("instance", definitions::INSTANCE),
        ("connection", connection.as_str()),
        ("operation", request.operation.as_str()),
        ("schema", schema.as_str()),
        ("revision", revision.as_str()),
    ] {
        rewritten.push(OsString::from(format!("--{long}={value}")));
    }
    rewritten.extend(request.kept.iter().cloned());
    Ok(rewritten)
}

/// A plain name: ASCII letters, digits, `-` and `_`, as a derived instance's
/// connector and instance are.
fn plain(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// A regular file's bytes, at most `limit` of them.
fn read_bounded(path: &Path, limit: u64) -> Option<Vec<u8>> {
    use std::io::Read as _;
    let file = std::fs::File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > limit {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= limit).then_some(bytes)
}

/// A file the definition names under `root`, read as the lowerer resolves it:
/// plain relative components only, still under the root once links are
/// followed.
fn under_root(root: &Path, relative: &str) -> Option<Vec<u8>> {
    let path = Path::new(relative);
    if relative.is_empty()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return None;
    }
    let root = root.canonicalize().ok()?;
    let resolved = root.join(path).canonicalize().ok()?;
    if !resolved.starts_with(&root) {
        return None;
    }
    read_bounded(&resolved, SOURCE_LIMIT)
}

/// A path the definition names under the definitions root, as the lowerer
/// admits it (plain relative components only), as the cache key names it: its
/// components joined with `/`, on every platform and for every spelling of
/// the same path.
fn relative(path: &str) -> Option<String> {
    let components = Path::new(path)
        .components()
        .map(|component| match component {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    (!components.is_empty()).then(|| components.join("/"))
}

/// Every file a lowering of the definition reads, by its path under the
/// definitions root, each read once: the definition itself as
/// `connectors/<file name>`, its vendored source, its amendments and the
/// shared auth file of each profile that uses one. `None` when one cannot be
/// read, is not under the root, or two names would be one path.
fn inputs(
    file: &Path,
    root: &Path,
    bytes: &[u8],
    parsed: &lowering::ProviderDefinition,
) -> Option<BTreeMap<String, Vec<u8>>> {
    let name = file.file_name()?.to_str()?;
    let mut named = vec![(relative(&format!("connectors/{name}"))?, bytes.to_vec())];
    let (_, source) = parsed.source.document().ok()?;
    let mut referenced = vec![source.to_owned()];
    referenced.extend(parsed.source.amendments.clone());
    referenced.extend(
        parsed
            .auth
            .values()
            .filter_map(|auth| auth.library.as_ref())
            .map(|library| format!("auth/{library}.yaml")),
    );
    for path in referenced {
        named.push((relative(&path)?, under_root(root, &path)?));
    }
    let mut files = BTreeMap::new();
    for (path, bytes) in named {
        if files
            .insert(path, bytes.clone())
            .is_some_and(|held| held != bytes)
        {
            return None;
        }
    }
    Some(files)
}

/// The cache key of a definition: the digest of this build's lowerer and of
/// every file its lowering reads, by path ([`inputs`]).
fn key(files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut hash = Sha256::new();
    let mut part = |label: &[u8], data: &[u8]| {
        hash.update((label.len() as u64).to_be_bytes());
        hash.update(label);
        hash.update((data.len() as u64).to_be_bytes());
        hash.update(data);
    };
    part(b"format", b"connectors-definition-cache/2");
    part(b"lowerer", env!("CARGO_PKG_VERSION").as_bytes());
    for (path, bytes) in files {
        part(path.as_bytes(), bytes);
    }
    hex::encode(hash.finalize())
}

/// The cache entry `<state>/definitions/<digest>/` of the definition at `file`
/// and its native configuration: reused when it exists, otherwise lowered into
/// a private staging directory beside it and published under its digest
/// whole. The lowerer reads a private copy of exactly the files the digest was
/// taken over, so an entry holds the lowering of its key and nothing a file
/// changed into meanwhile. A definition the lowerer refuses leaves nothing
/// behind.
fn cache(
    paths: &Paths,
    file: &Path,
    root: &Path,
    bytes: &[u8],
    parsed: &lowering::ProviderDefinition,
    alias: &str,
) -> Result<(String, PathBuf), HandlerReply> {
    let files = inputs(file, root, bytes, parsed).ok_or_else(usage)?;
    let digest = key(&files);
    let (directory, held) = definitions::directory(paths, true).map_err(host_failure)?;
    let entry = directory.join(&digest);
    let native = entry.join(NATIVE);
    if std::fs::symlink_metadata(&entry).is_ok() {
        if published(&entry) {
            return Ok((digest, native));
        }
        // Not an entry this lowering would publish (another kind of file, not
        // owner-only, or native configuration that names another place): it is
        // lowered again.
        std::fs::remove_dir_all(&entry)
            .or_else(|_| std::fs::remove_file(&entry))
            .map_err(|_| host_failure(Failure::InvalidConfiguration))?;
    }
    let unique = uuid::Uuid::new_v4().simple();
    let copied = directory.join(format!(".inputs-{unique}"));
    let staging = directory.join(format!(".lowering-{unique}"));
    let lowered = (|| {
        filesystem::directory(&copied, true, true).map_err(host_failure)?;
        for (path, bytes) in &files {
            let target = path
                .split('/')
                .fold(copied.clone(), |at, name| at.join(name));
            std::fs::create_dir_all(target.parent().ok_or_else(usage)?)
                .and_then(|()| std::fs::write(&target, bytes))
                .map_err(|_| host_failure(Failure::InvalidConfiguration))?;
        }
        filesystem::directory(&staging, true, true).map_err(host_failure)?;
        let definition = copied
            .join("connectors")
            .join(file.file_name().ok_or_else(usage)?);
        let lowered = lowering::lower(&lowering::Request {
            definition: &definition,
            root: &copied,
            out: &staging,
            scratch: &directory,
        })
        .map_err(|_| usage())?;
        // Everything the lowerer wrote is inside this lowering's own staging
        // directory.
        if ![&lowered.bundle, &lowered.operations, &lowered.connector]
            .iter()
            .all(|path| inside(path, &staging))
        {
            return Err(usage());
        }
        let text = native_configuration(parsed, &lowered, &staging, &entry, alias)?;
        let staged = filesystem::directory(&staging, false, true).map_err(host_failure)?;
        filesystem::publish_new(&staged, OsStr::new(NATIVE), &text).map_err(host_failure)?;
        // Durable before it is published under its digest.
        sync_tree(&staging).map_err(|_| host_failure(Failure::OutcomeUnknown))
    })();
    let renamed = lowered.and_then(|()| {
        std::fs::rename(&staging, &entry).or_else(|_| {
            // Another run published the same digest first: its entry is this one.
            if published(&entry) {
                Ok(())
            } else {
                Err(host_failure(Failure::InvalidConfiguration))
            }
        })
    });
    for scratch in [&copied, &staging] {
        if std::fs::symlink_metadata(scratch).is_ok() {
            let _ = std::fs::remove_dir_all(scratch);
        }
    }
    renamed?;
    filesystem::sync_directory(&held).map_err(host_failure)?;
    Ok((digest, native))
}

/// Whether `entry` is a published cache entry: an owner-only directory, no
/// link, whose private native configuration names it as its bundle directory.
fn published(entry: &Path) -> bool {
    let directory = std::fs::symlink_metadata(entry).is_ok_and(|metadata| metadata.is_dir())
        && filesystem::directory(entry, false, true).is_ok();
    directory
        && filesystem::private_file(&entry.join(NATIVE))
            .and_then(|file| filesystem::read_bounded(file, 1024 * 1024))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .is_some_and(|native| {
                // Compared as paths: every admitted spelling of the state
                // directory names the same entry.
                native["bundle_directory"]
                    .as_str()
                    .is_some_and(|named| Path::new(named) == entry)
            })
}

/// Whether `path` is `directory` or under it, by its components.
fn inside(path: &Path, directory: &Path) -> bool {
    path.starts_with(directory)
        && path
            .strip_prefix(directory)
            .is_ok_and(|rest| rest.components().all(|c| matches!(c, Component::Normal(_))))
}

/// Flush every file under `directory`, and the directories themselves.
fn sync_tree(directory: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            sync_tree(&entry.path())?;
        } else if kind.is_file() {
            // Opened for writing: Windows flushes only a handle that may write.
            std::fs::OpenOptions::new()
                .write(true)
                .open(entry.path())?
                .sync_all()?;
        }
    }
    let held = filesystem::directory(directory, false, false)
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
    filesystem::sync_directory(&held).map_err(|_| std::io::Error::from(std::io::ErrorKind::Other))
}

/// The catalog provider's `connectors-catalog-local/2` configuration of the
/// lowered definition, as an instance of a shipped connector is derived from
/// its declaration: the declared endpoint and API path, the definition's
/// default profile, and the lowered bundle and selection at their place in the
/// published entry. The provider's own trust roots apply; a definition names
/// none.
fn native_configuration(
    parsed: &lowering::ProviderDefinition,
    lowered: &lowering::Lowered,
    staging: &Path,
    entry: &Path,
    alias: &str,
) -> Result<Vec<u8>, HandlerReply> {
    let declaration: Value = std::fs::read(&lowered.connector)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .ok_or_else(usage)?;
    // Every variable the base URL names has a default: the endpoint is the
    // declaration's, and an instance that would need another is configured.
    let endpoint = declaration["default_endpoint"]
        .as_str()
        .ok_or_else(usage)?
        .trim_end_matches('/');
    let origin_end = endpoint
        .strip_prefix("https://")
        .map(|rest| "https://".len() + rest.find('/').unwrap_or(rest.len()))
        .ok_or_else(usage)?;
    let (origin, root_path) = endpoint.split_at(origin_end);
    let (api_base, request_prefix) = match declaration["api_path"].as_str() {
        None => (endpoint.to_owned(), None),
        Some(path) => (
            format!("{endpoint}{path}"),
            (!root_path.is_empty() && root_path != "/").then(|| root_path.to_owned()),
        ),
    };
    let profile_id = match parsed.auth.len() {
        1 => parsed.auth.keys().next().cloned(),
        _ => parsed
            .auth
            .iter()
            .find(|(_, auth)| auth.default == Some(true))
            .map(|(id, _)| id.clone()),
    }
    .ok_or_else(usage)?;
    let mut profile = declaration["profiles"]
        .as_array()
        .and_then(|profiles| {
            profiles
                .iter()
                .find(|profile| profile["profile"] == json!(profile_id))
        })
        .cloned()
        .ok_or_else(usage)?;
    if let Some(path) = profile["token_url"]
        .as_str()
        .filter(|url| url.starts_with('/'))
    {
        profile["token_url"] = json!(format!("{origin}{path}"));
    }
    let operations = lowered
        .operations
        .strip_prefix(staging)
        .map_err(|_| usage())?;
    let mut native = json!({
        "format": "connectors-catalog-local/2",
        "instance": alias,
        "provider": lowered.provider,
        "bundle_directory": entry,
        "api_base": api_base,
        "auth": profile,
        "operations_file": entry.join(operations),
    });
    if let Some(prefix) = request_prefix {
        native["request_prefix"] = json!(prefix);
    }
    serde_json::to_vec_pretty(&native).map_err(|_| usage())
}

/// The most candidates the reuse question lists.
const LISTED: usize = 16;
/// The most bytes of candidate listing the reuse question holds, below the
/// 4096 bytes `protected::ask` shows, leaving room for its last line.
const QUESTION_BYTES: usize = 3584;

/// The connection the command runs on: the instance's one connection, or, when
/// it holds none, the one made after the operator answers the one question.
/// When another instance of the connector holds a connection whose credential
/// can be reused (story:invoke-definition-reuse-credential), the question lists
/// those and a number reuses one; otherwise it asks whether to connect. Without
/// a terminal to ask on, or answered anything else, the command is refused
/// naming the connect, and nothing is acquired, copied or sent.
fn connection(
    paths: &Paths,
    connector: &str,
    alias: &str,
    adapter: &connectors_host::local::config::Adapter,
    reported: &connectors_host::local::runtime::Bootstrap,
    request: &Request,
) -> Result<String, HandlerReply> {
    let held = Registry::with_system_clock(&paths.state)
        .held(&adapter.instance_id, connectors_sdk::now_ms())
        .map_err(connections::registry_failure)?;
    match held.as_slice() {
        [one] => return Ok(one.clone()),
        [] => {}
        _ => return Err(usage()),
    }
    let profiles: Vec<&String> = adapter.permissions.profiles.iter().collect();
    let [profile] = profiles.as_slice() else {
        return Err(host_failure(Failure::InvalidConfiguration));
    };
    let fresh = || connect(paths, alias, profile, request.credential_file.as_deref());
    let refused = || failure("not_found", "admission", "create_connection", false);
    let deadline = connectors_sdk::now_ms() + QUESTION_MS;
    // A listing that fails offers no reuse: the plain question is asked.
    let candidates = candidates(paths, alias, adapter, reported, profile).unwrap_or_default();
    if candidates.is_empty() {
        let question = format!(
            "No connection for {connector}/{}. Connect now? [y/N] ",
            definitions::INSTANCE
        );
        return if answered(protected::confirm(&question, deadline))? {
            fresh()
        } else {
            Err(refused())
        };
    }
    let mut question = format!(
        "No connection for {connector}/{}. A stored credential of {connector} can be reused:\n",
        definitions::INSTANCE
    );
    // Listed while the question stays within what the terminal question takes.
    let mut listed = Vec::new();
    for reusable in &candidates {
        let candidate = &reusable.candidate;
        let line = format!(
            "  {}  {}/{}  {}  {} {}\n",
            listed.len() + 1,
            reusable.connector,
            reusable.instance,
            shown(&candidate.provider_authority),
            shown(&candidate.identity.kind),
            shown(&candidate.identity.subject),
        );
        if question.len() + line.len() > QUESTION_BYTES {
            break;
        }
        question.push_str(&line);
        listed.push(reusable);
    }
    let range = match listed.len() {
        1 => "1".to_owned(),
        n => format!("1-{n}"),
    };
    question.push_str(&format!(
        "Reuse one (its number), connect fresh (c), or cancel? [{range}/c/N] "
    ));
    let answer = answered(protected::ask(&question, deadline))?;
    if matches!(answer.as_str(), "c" | "connect") {
        return fresh();
    }
    let Some(chosen) = answer
        .parse::<usize>()
        .ok()
        .and_then(|number| number.checked_sub(1))
        .and_then(|index| listed.get(index).copied())
    else {
        return Err(refused());
    };
    let error = match reuse(paths, alias, chosen, profile) {
        Ok(connection) => return Ok(connection),
        Err(error) => error,
    };
    // Nothing was published, or that is not known: only a definite refusal
    // offers the fresh connect.
    if matches!(
        error.code,
        owner::Code::OutcomeUnknown | owner::Code::Interrupted
    ) {
        return Err(connect_failure("connections-connect", error));
    }
    let question = format!(
        "The stored credential was not reused ({}); nothing was saved. Connect fresh? [y/N] ",
        serde_json::to_value(error.code)
            .ok()
            .and_then(|code| code.as_str().map(str::to_owned))
            .unwrap_or_default()
    );
    match protected::confirm(&question, connectors_sdk::now_ms() + QUESTION_MS) {
        Ok(true) => fresh(),
        Ok(false) => Err(connect_failure("connections-connect", error)),
        Err(asked) if asked.code == owner::Code::InvalidInput => {
            Err(connect_failure("connections-connect", error))
        }
        Err(asked) => Err(owner_failure(asked)),
    }
}

/// A listed value, cut to 128 characters so every listed candidate fits the
/// question.
fn shown(value: &str) -> String {
    match value.char_indices().nth(128) {
        Some((end, _)) => format!("{}...", &value[..end]),
        None => value.to_owned(),
    }
}

/// The answer to a question, or the refusal when none could be asked: no
/// controlling terminal in the foreground is the connect's `not_found`.
fn answered<T>(answer: owner::Result<T>) -> Result<T, HandlerReply> {
    match answer {
        Ok(answer) => Ok(answer),
        Err(error) if error.code == owner::Code::InvalidInput => Err(failure(
            "not_found",
            "admission",
            "create_connection",
            false,
        )),
        Err(error) => Err(owner_failure(error)),
    }
}

/// The connections other instances of the definition's connector hold whose
/// credential its instance may reuse under `profile`, at most [`LISTED`]: none
/// for a profile whose entry an OAuth acquisition refreshes, since one
/// rotation would end the other's refresh.
fn candidates(
    paths: &Paths,
    alias: &str,
    adapter: &connectors_host::local::config::Adapter,
    reported: &connectors_host::local::runtime::Bootstrap,
    profile: &str,
) -> Result<Vec<owner::Reusable>, HandlerReply> {
    let mut bootstrap = reported.clone();
    bootstrap.configure(adapter);
    let declared = bootstrap
        .profile(profile)
        .map_err(|failure| owner_failure(owner::Error::from(failure)))?;
    if declared.acquisition.is_some() {
        return Ok(Vec::new());
    }
    let binding = bootstrap
        .binding(profile)
        .map_err(|failure| owner_failure(owner::Error::from(failure)))?;
    let mut found = owner::reusable(paths, alias, &binding).map_err(owner_failure)?;
    found.truncate(LISTED);
    Ok(found)
}

/// The reuse of `chosen` for the instance under `profile`, in the owner, which
/// alone reads and copies the credential; answers the published connection.
fn reuse(
    paths: &Paths,
    alias: &str,
    chosen: &owner::Reusable,
    profile: &str,
) -> owner::Result<String> {
    let reused = owner::Client::connect(paths, true).and_then(|client| {
        client.reuse(
            alias,
            &chosen.alias,
            &chosen.candidate.reference,
            &chosen.candidate.revision,
            profile,
        )
    })?;
    reused["connection"]["summary"]["connection"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| owner::Code::OutcomeUnknown.into())
}

/// `connections connect` of the instance under `profile`, with its protected
/// entry from the terminal, or from `file` (an OAuth client file becomes the
/// entry by consent, as for `connections connect --credential-file`); under
/// the read-only binding the owner reads the item itself. Answers the
/// published connection.
fn connect(
    paths: &Paths,
    alias: &str,
    profile: &str,
    file: Option<&Path>,
) -> Result<String, HandlerReply> {
    let refused = |error| connect_failure("connections-connect", error);
    owner::admit_capture(paths, alias, Some(profile), None, None).map_err(refused)?;
    let read_only = Config::load_at(paths)
        .map_err(host_failure)?
        .selection()
        .read_only();
    if read_only && file.is_some() {
        return Err(usage());
    }
    let capture = owner::Client::connect(paths, true)
        .and_then(|client| client.begin(alias, Some(profile.to_owned()), None, None))
        .map_err(refused)?;
    let connected = if read_only {
        capture.complete_mounted()
    } else {
        let (acquisition, expires_at_ms) = (capture.acquisition.clone(), capture.expires_at_ms);
        let secret = match file {
            Some(path) => protected::file(path, expires_at_ms)
                .and_then(|document| oauth::entry(&capture.profile, document, expires_at_ms)),
            None => protected::terminal(&capture.profile.fields, expires_at_ms),
        }
        .map_err(|mut error| {
            error.acquisition = Some(acquisition);
            if error.code == owner::Code::InvalidInput {
                error.code = owner::Code::ProtectedEntryUnavailable;
            }
            refused(error)
        })?;
        capture.complete(&secret)
    }
    .map_err(refused)?;
    connected["connection"]["summary"]["connection"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| owner_failure(owner::Code::OutcomeUnknown.into()))
}

/// The schema of `operation` and the revision of the instance's description.
/// A description that cannot be launched for because the connection is not
/// current under the recorded entry answers what a read on that connection
/// answers ([`Registry::admit_read`] against the binding the provider
/// reported): an authority or profile change is today's `lifecycle_conflict`,
/// and nothing is asked.
fn described(
    paths: &Paths,
    alias: &str,
    adapter: &connectors_host::local::config::Adapter,
    mut reported: connectors_host::local::runtime::Bootstrap,
    connection: &str,
    operation: &str,
) -> Result<(String, String), HandlerReply> {
    match owner::described(paths, alias, Some(connection)) {
        Ok(bootstrap) => {
            let schema = owner::schema(&bootstrap, operation).map_err(owner_failure)?;
            let revision = bootstrap
                .descriptor()
                .map_err(|failure| owner_failure(failure.into()))?
                .revision;
            Ok((schema, revision))
        }
        Err(error) if error.code == owner::Code::DescriptionUnavailable => {
            reported.configure(adapter);
            let registry = Registry::with_system_clock(&paths.state);
            for profile in &adapter.permissions.profiles {
                let binding = reported
                    .binding(profile)
                    .map_err(|failure| owner_failure(owner::Error::from(failure)))?;
                registry
                    .admit_read(
                        &binding,
                        connection,
                        &BTreeSet::new(),
                        connectors_sdk::now_ms(),
                    )
                    .map_err(connections::registry_failure)?;
            }
            Err(owner_failure(error))
        }
        Err(error) => Err(owner_failure(error)),
    }
}

/// story:invoke-definition-file: the cache key covers every file a lowering
/// reads, and only files under the definitions root.
#[cfg(test)]
mod tests {
    use super::*;

    const DEFINITION: &str = "schema: urn:beyond10x:connectors-definition:1
source:
  openapi: upstream/forge/openapi.json.gz
  sha256: 01fcc470fbbd1a2c691d5a17c35a00b086ca5e62fdd08747f92cb350e0fc6fa1
  amendments: upstream/forge/amendments.json
base_url: https://gitlab.example.com/api/v4
auth:
  forge.pat:
    use: forge
operations:
  project.get:
    op: getApiV4ProjectsId
";

    /// A definitions root holding the definition and each file it names.
    fn tree(
        text: &str,
    ) -> (
        tempfile::TempDir,
        PathBuf,
        Vec<u8>,
        lowering::ProviderDefinition,
    ) {
        let root = tempfile::tempdir().unwrap();
        for (path, bytes) in [
            ("connectors/forge.yaml", text),
            ("upstream/forge/openapi.json.gz", "source"),
            ("upstream/forge/amendments.json", "amendments"),
            ("auth/forge.yaml", "library"),
        ] {
            let path = root.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        let file = root.path().join("connectors/forge.yaml");
        let parsed = lowering::parse(text.as_bytes()).unwrap();
        (root, file, text.as_bytes().to_vec(), parsed)
    }

    fn digest(
        root: &Path,
        file: &Path,
        bytes: &[u8],
        parsed: &lowering::ProviderDefinition,
    ) -> String {
        key(&inputs(file, root, bytes, parsed).unwrap())
    }

    #[test]
    fn every_file_a_lowering_reads_is_in_the_key() {
        let (root, file, bytes, parsed) = tree(DEFINITION);
        let files = inputs(&file, root.path(), &bytes, &parsed).unwrap();
        assert_eq!(
            files.keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "auth/forge.yaml",
                "connectors/forge.yaml",
                "upstream/forge/amendments.json",
                "upstream/forge/openapi.json.gz",
            ]
        );
        let first = digest(root.path(), &file, &bytes, &parsed);
        assert_eq!(digest(root.path(), &file, &bytes, &parsed), first);
        for path in [
            "upstream/forge/openapi.json.gz",
            "upstream/forge/amendments.json",
            "auth/forge.yaml",
        ] {
            let target = root.path().join(path);
            let before = std::fs::read(&target).unwrap();
            std::fs::write(&target, b"changed").unwrap();
            assert_ne!(digest(root.path(), &file, &bytes, &parsed), first, "{path}");
            std::fs::write(&target, before).unwrap();
        }
        let renamed = root.path().join("connectors/other.yaml");
        assert_ne!(digest(root.path(), &renamed, &bytes, &parsed), first);
    }

    /// The key names each file by its components joined with `/`, whatever the
    /// platform's separator and however the definition spells the path: the
    /// same files are one key on every platform.
    #[test]
    fn the_key_names_each_file_by_slash_joined_components() {
        let (root, file, bytes, parsed) = tree(DEFINITION);
        let clean = inputs(&file, root.path(), &bytes, &parsed).unwrap();
        let text = DEFINITION
            .replace(
                "openapi: upstream/forge/openapi.json.gz",
                "openapi: upstream//forge/./openapi.json.gz",
            )
            .replace(
                "amendments: upstream/forge/amendments.json",
                "amendments: upstream/forge//amendments.json",
            );
        let spelled = lowering::parse(text.as_bytes()).unwrap();
        let other = inputs(&file, root.path(), text.as_bytes(), &spelled).unwrap();
        let names = |files: &BTreeMap<String, Vec<u8>>| files.keys().cloned().collect::<Vec<_>>();
        assert_eq!(names(&other), names(&clean));
        let built = Path::new("upstream").join("forge").join("openapi.json.gz");
        assert_eq!(
            relative(built.to_str().unwrap()).as_deref(),
            Some("upstream/forge/openapi.json.gz")
        );
        for name in names(&clean) {
            assert!(!name.contains('\\'), "{name}");
        }
    }

    #[test]
    fn a_file_outside_the_definitions_root_is_refused() {
        let text = DEFINITION.replace(
            "amendments: upstream/forge/amendments.json",
            "amendments: ../outside.json",
        );
        let (root, file, bytes, parsed) = tree(&text);
        assert!(inputs(&file, root.path(), &bytes, &parsed).is_none());
        let text = DEFINITION.replace("use: forge", "use: ../forge");
        let (root, file, bytes, parsed) = tree(&text);
        assert!(inputs(&file, root.path(), &bytes, &parsed).is_none());
    }
}

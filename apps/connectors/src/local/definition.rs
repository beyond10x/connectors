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
//! on yes, runs the profile's own acquisition. The command line is then
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
    collections::BTreeSet,
    ffi::OsStr,
    path::{Component, Path, PathBuf},
};

/// The native configuration file in each cache entry.
const NATIVE: &str = "native.json";
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
fn run(request: &Request, definition: &Path) -> Result<Vec<OsString>, HandlerReply> {
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
    if !connectors_core::valid_id(&connector) || !connectors_core::valid_id(&alias) {
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
    let _signals = protected::Signals::install().map_err(owner_failure)?;
    let (digest, native) = cache(&paths, &file, &root, &bytes, &parsed, &stem, &alias)?;
    let (adapter, reported) = definitions::derive(&connector, &native).map_err(host_failure)?;
    definitions::record(
        &paths,
        &connector,
        &definitions::Entry {
            digest,
            adapter: adapter.clone(),
        },
    )
    .map_err(host_failure)?;
    // What the command selects from here on is the entry just recorded.
    let held = Config::read_at(&paths).map_err(configuration_refusal)?;
    if held.definitions.get(&alias) != Some(&connector)
        || held.adapters.get(&alias).map(|a| a.selection()) != Some(adapter.selection())
    {
        return Err(configuration_refusal(Refusal::Rule(
            Rule::DefinitionInstanceConfigured,
        )));
    }
    let connection = match &request.connection {
        Some(connection) => connection.clone(),
        None => connection(&paths, &connector, &alias, &adapter, request)?,
    };
    let (schema, revision) = match (&request.schema, &request.revision) {
        (Some(schema), Some(revision)) => (schema.clone(), revision.clone()),
        (schema, revision) => {
            let (described_schema, described_revision) = described(
                &paths,
                &alias,
                &adapter,
                reported,
                &connection,
                &request.operation,
            )?;
            (
                schema.clone().unwrap_or(described_schema),
                revision.clone().unwrap_or(described_revision),
            )
        }
    };
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

/// The cache key of a definition: the digest of this build's lowerer, the
/// file's stem and bytes, the connector declaration it lowers to (which
/// carries every shared auth file it uses), its vendored source and its
/// amendments. `None` when one of them cannot be read.
fn key(
    file: &Path,
    root: &Path,
    bytes: &[u8],
    parsed: &lowering::ProviderDefinition,
    stem: &str,
) -> Option<String> {
    let mut hash = Sha256::new();
    let mut part = |label: &str, data: &[u8]| {
        hash.update(label.as_bytes());
        hash.update([0]);
        hash.update((data.len() as u64).to_be_bytes());
        hash.update(data);
    };
    part("format", b"connectors-definition-cache/1");
    part("lowerer", env!("CARGO_PKG_VERSION").as_bytes());
    part("stem", stem.as_bytes());
    part("definition", bytes);
    part(
        "declaration",
        lowering::connector(file, root).ok()?.as_bytes(),
    );
    let (_, source) = parsed.source.document().ok()?;
    part("source", &under_root(root, source)?);
    if let Some(amendments) = &parsed.source.amendments {
        part("amendments", &under_root(root, amendments)?);
    }
    Some(hex::encode(hash.finalize()))
}

/// The cache entry `<state>/definitions/<digest>/` of the definition at `file`
/// and its native configuration: reused when it exists, otherwise lowered into
/// a private staging directory beside it and published under its digest
/// whole. A definition the lowerer refuses leaves nothing behind.
fn cache(
    paths: &Paths,
    file: &Path,
    root: &Path,
    bytes: &[u8],
    parsed: &lowering::ProviderDefinition,
    stem: &str,
    alias: &str,
) -> Result<(String, PathBuf), HandlerReply> {
    let digest = key(file, root, bytes, parsed, stem).ok_or_else(usage)?;
    let (directory, held) = definitions::directory(paths, true).map_err(host_failure)?;
    let entry = directory.join(&digest);
    let native = entry.join(NATIVE);
    if std::fs::symlink_metadata(&native).is_ok_and(|metadata| metadata.is_file()) {
        return Ok((digest, native));
    }
    let staging = directory.join(format!(".lowering-{}", uuid::Uuid::new_v4().simple()));
    filesystem::directory(&staging, true, true).map_err(host_failure)?;
    let lowered = (|| {
        let lowered = lowering::lower(&lowering::Request {
            definition: file,
            root,
            out: &staging,
            scratch: &directory,
        })
        .map_err(|_| usage())?;
        let text = native_configuration(parsed, &lowered, &staging, &entry, alias)?;
        let staged = filesystem::directory(&staging, false, true).map_err(host_failure)?;
        filesystem::publish_new(&staged, OsStr::new(NATIVE), &text).map_err(host_failure)
    })();
    let published = lowered.and_then(|()| {
        std::fs::rename(&staging, &entry).or_else(|_| {
            // Another run published the same digest first: its entry is this one.
            if std::fs::symlink_metadata(&native).is_ok_and(|metadata| metadata.is_file()) {
                Ok(())
            } else {
                Err(host_failure(Failure::InvalidConfiguration))
            }
        })
    });
    if std::fs::symlink_metadata(&staging).is_ok() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    published?;
    filesystem::sync_directory(&held).map_err(host_failure)?;
    Ok((digest, native))
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

/// The connection the command runs on: the instance's one connection, or, when
/// it holds none, the one made after the operator answers the one question
/// yes. Without a terminal to ask on, or answered anything but yes, the command
/// is refused naming the connect, and nothing is acquired or sent.
fn connection(
    paths: &Paths,
    connector: &str,
    alias: &str,
    adapter: &connectors_host::local::config::Adapter,
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
    let refused = || failure("not_found", "admission", "create_connection", false);
    let question = format!(
        "No connection for {connector}/{}. Connect now? [y/N] ",
        definitions::INSTANCE
    );
    match protected::confirm(&question, connectors_sdk::now_ms() + QUESTION_MS) {
        Ok(true) => {}
        Ok(false) => return Err(refused()),
        // No controlling terminal in the foreground: nothing can be asked.
        Err(error) if error.code == owner::Code::InvalidInput => return Err(refused()),
        Err(error) => return Err(owner_failure(error)),
    }
    let profiles: Vec<&String> = adapter.permissions.profiles.iter().collect();
    let [profile] = profiles.as_slice() else {
        return Err(host_failure(Failure::InvalidConfiguration));
    };
    connect(paths, alias, profile, request.credential_file.as_deref())
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

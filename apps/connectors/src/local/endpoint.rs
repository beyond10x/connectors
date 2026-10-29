//! `connections connect <connector>/<instance> [--endpoint <url>]`
//! (story:connectors-do-short-invocation, decision D10;
//! `contracts/cli/v1alpha1/semantics.md` §3, "Giving an instance its
//! endpoint"). The positional target is the selection `--connector` and
//! `--instance` make, never beside them. The endpoint is a property of the
//! instance, given once: before any credential is asked for, it is written
//! into the instance's table of the configuration file, which keeps every
//! other byte, stays owner-only and is replaced atomically, and is read back
//! before the connect goes on. Another endpoint on an instance that has one is
//! refused, since another host is another instance. No other command takes
//! `--endpoint`.
use super::*;
use connectors_cli_contract::{Context, OutputMode, ProcessOutput};
use connectors_host::local::{
    config::{CONFIG_LIMIT, endpoint_admitted},
    filesystem,
};
use toml_edit::{DocumentMut, InlineTable, Item, Table};

/// The keys of an instance table that pin it to an executable selection
/// (`connectors.cli` configuration, `connectors-local/5`): such an instance
/// takes its endpoint from its pinned arguments, never from this command.
const PINS: [&str; 11] = [
    "alias",
    "instance_id",
    "adapter_id",
    "configuration_revision",
    "credential_revision",
    "protocol",
    "private_protocol",
    "startup",
    "restart",
    "executable",
    "permissions",
];

/// The command `connections describe`, which reads a connection's revision;
/// named by a refused `--expected-revision`
/// (`connectors.cli.Failure.read_with`).
pub(super) const READ_REVISION_WITH: &str = "connections describe";

/// `invalid_input` at the arguments naming the refused `field`.
pub(super) fn argument(field: &str) -> HandlerReply {
    let mut reply = failure("invalid_input", "arguments", "none", true);
    if let HandlerReply::UsageError { data, .. } = &mut reply {
        data["field"] = json!(field);
    }
    reply
}

/// The input of a `connections connect` with its `target` written as the
/// `connector` and `instance` it names. A target beside any other selector, or
/// one that is not `<connector>/<instance>`, is `invalid_input` naming
/// `target`. `endpoint` is kept; the handler drops it.
pub(super) fn selection(input: &Value) -> Result<Value, HandlerReply> {
    let Some(target) = input.get("target").filter(|t| !t.is_null()) else {
        return Ok(input.clone());
    };
    let given = |field| input.get(field).is_some_and(|value| !value.is_null());
    if given("adapter") || given("connector") || given("instance") {
        return Err(argument("target"));
    }
    let (connector, instance) = target
        .as_str()
        .and_then(|target| target.split_once('/'))
        .filter(|(connector, instance)| {
            !connector.is_empty() && !instance.is_empty() && !instance.contains('/')
        })
        .ok_or_else(|| argument("target"))?;
    let mut input = input.clone();
    if let Some(fields) = input.as_object_mut() {
        fields.remove("target");
        fields.insert("connector".into(), json!(connector));
        fields.insert("instance".into(), json!(instance));
    }
    Ok(input)
}

/// What a `connections connect` decides before the generated runner acquires
/// any source: its target, and the endpoint written or required. `None` lets
/// the command go on; otherwise its refusal, rendered as the runner renders a
/// handler's.
pub(super) fn prepare(session: &session::Session) -> Option<ProcessOutput> {
    let (callable, context) = session.selected()?;
    if callable != "connections-connect" {
        return None;
    }
    let selectors = session.selectors()?;
    connect(selectors, context)
        .err()
        .map(|reply| definition::render(reply, context.output == OutputMode::Json))
}

fn connect(selectors: &Value, context: &Context) -> Result<(), HandlerReply> {
    let selectors = selection(selectors)?;
    let text = |field| selectors.get(field).and_then(Value::as_str);
    let named = match (text("adapter"), text("connector"), text("instance")) {
        (None, Some(connector), Some(instance)) => Some((connector, instance)),
        _ => None,
    };
    let paths = || {
        Paths::resolve(context.config.as_deref(), context.state_dir.as_deref())
            .map_err(|_| configuration_refusal(Refusal::Rule(Rule::ConfigPathUnresolved)))
    };
    match (text("endpoint"), named) {
        // Written into a named instance only, and only as the configuration
        // admits it.
        (Some(_), None) => Err(argument("endpoint")),
        (Some(endpoint), Some(_)) if !endpoint_admitted(endpoint) => Err(argument("endpoint")),
        (Some(endpoint), Some((connector, instance))) => {
            write(&paths()?, connector, instance, endpoint)
        }
        (None, Some((connector, instance))) => required(&paths()?, connector, instance),
        (None, None) => Ok(()),
    }
}

/// A shipped catalog connector that declares no default endpoint, and an
/// instance listed without one or not listed at all: `invalid_input` naming
/// `endpoint`. A file this cannot read is left to the command's own refusal.
fn required(paths: &Paths, connector: &str, instance: &str) -> Result<(), HandlerReply> {
    let needs = declarations::all().iter().any(|declaration| {
        declaration["connector_id"] == connector && declaration.get("default_endpoint").is_none()
    });
    if !needs {
        return Ok(());
    }
    let Ok((text, _)) = read(paths) else {
        return Ok(());
    };
    let Ok(document) = text.parse::<DocumentMut>() else {
        return Ok(());
    };
    let table = document
        .get("connectors")
        .and_then(|c| c.get(connector))
        .and_then(|c| c.get("instances"))
        .and_then(|i| i.get(instance));
    let unanswered = match table.and_then(Item::as_table_like) {
        None => table.is_none(),
        Some(table) => {
            table.get("endpoint").is_none() && !PINS.iter().any(|pin| table.contains_key(pin))
        }
    };
    if unanswered {
        return Err(argument("endpoint"));
    }
    Ok(())
}

/// The admitted configuration text and its permission bits.
fn read(paths: &Paths) -> Result<(String, u32), HandlerReply> {
    let refused = |failure: Failure| {
        configuration_refusal(match Config::read(&paths.config) {
            Err(refusal @ Refusal::Rule(_)) => refusal,
            _ => Refusal::Failure(failure),
        })
    };
    let file = filesystem::private_file(&paths.config).map_err(refused)?;
    #[cfg(unix)]
    let mode = {
        use std::os::unix::fs::MetadataExt;
        file.metadata()
            .map_err(|_| refused(Failure::InvalidConfiguration))?
            .mode()
            & 0o700
    };
    #[cfg(not(unix))]
    let mode = 0o600;
    let bytes = filesystem::read_bounded(file, CONFIG_LIMIT).map_err(refused)?;
    let text = String::from_utf8(bytes).map_err(|_| refused(Failure::InvalidConfiguration))?;
    Ok((text, mode))
}

fn invalid_configuration() -> HandlerReply {
    host_failure(Failure::InvalidConfiguration)
}

/// The table under `key` of the table `item`, created as `item` holds its
/// tables (a header table, implicit unless `keys` are given, a dotted one, or
/// an inline table) with `keys` when absent.
fn child<'a>(
    item: &'a mut Item,
    key: &str,
    keys: &[(&str, toml_edit::Value)],
) -> Result<(&'a mut Item, bool), HandlerReply> {
    let created = match item {
        Item::Table(parent) => {
            let mut table = Table::new();
            table.set_implicit(keys.is_empty());
            table.set_dotted(parent.is_dotted());
            for (key, value) in keys {
                table.insert(key, Item::Value(value.clone()));
            }
            Item::Table(table)
        }
        Item::Value(toml_edit::Value::InlineTable(_)) => {
            let mut table = InlineTable::new();
            for (key, value) in keys {
                table.insert(*key, value.clone());
            }
            Item::Value(toml_edit::Value::InlineTable(table))
        }
        _ => return Err(invalid_configuration()),
    };
    let parent = item.as_table_like_mut().ok_or_else(invalid_configuration)?;
    let absent = parent.get(key).is_none();
    if absent {
        parent.insert(key, created);
    }
    let child = parent.get_mut(key).ok_or_else(invalid_configuration)?;
    if !child.is_table_like() {
        return Err(invalid_configuration());
    }
    Ok((child, absent))
}

/// The configuration text with `endpoint` in the instance's table, or `None`
/// when the table already holds exactly that endpoint.
fn edited(
    text: &str,
    connector: &str,
    instance: &str,
    endpoint: &str,
) -> Result<Option<String>, HandlerReply> {
    let mut document = text
        .parse::<DocumentMut>()
        .map_err(|_| invalid_configuration())?;
    // Only `connectors-local/5` has instance tables; an older file is named,
    // and `setup migrate` rewrites it.
    match document.get("format").and_then(Item::as_str) {
        Some("connectors-local/5") => {}
        Some(format) => {
            let mut reply = invalid_configuration();
            if format.starts_with("connectors-local/")
                && let HandlerReply::UsageError { data, .. } = &mut reply
            {
                data["configuration_format"] = json!(format);
            }
            return Err(reply);
        }
        None => return Err(invalid_configuration()),
    }
    let root = document.as_item_mut();
    let (connectors, _) = child(root, "connectors", &[])?;
    let (table, _) = child(connectors, connector, &[("enabled", true.into())])?;
    // A disabled connector table is left as the operator wrote it
    // (`Config::can_hold`).
    if table.get("enabled").and_then(Item::as_bool) != Some(true) {
        return Err(invalid_configuration());
    }
    let (instances, _) = child(table, "instances", &[])?;
    let (listed, created) = child(instances, instance, &[("endpoint", endpoint.into())])?;
    if created {
        return Ok(Some(document.to_string()));
    }
    let listed = listed
        .as_table_like_mut()
        .ok_or_else(invalid_configuration)?;
    if PINS.iter().any(|pin| listed.contains_key(pin)) {
        return Err(argument("endpoint"));
    }
    match listed.get("endpoint").map(|item| item.as_str()) {
        None => {
            listed.insert("endpoint", Item::Value(endpoint.into()));
            Ok(Some(document.to_string()))
        }
        Some(Some(held)) if held == endpoint => Ok(None),
        // Another host is another instance.
        Some(Some(_)) => {
            let mut reply = failure(
                "lifecycle_conflict",
                "configuration",
                "check_configuration",
                false,
            );
            if let HandlerReply::Error { data, .. } = &mut reply {
                data["field"] = json!("endpoint");
            }
            Err(reply)
        }
        Some(None) => Err(invalid_configuration()),
    }
}

/// Write `endpoint` into the instance `<connector>/<instance>`: the new text
/// is checked as every read checks it before anything is written, replaces
/// the file atomically with its mode only while the file still holds the text
/// it was made from, and is read back; a read-back that differs restores the
/// original text.
fn write(
    paths: &Paths,
    connector: &str,
    instance: &str,
    endpoint: &str,
) -> Result<(), HandlerReply> {
    let (text, mode) = read(paths)?;
    let Some(written) = edited(&text, connector, instance, endpoint)? else {
        return Ok(());
    };
    if written.len() > CONFIG_LIMIT {
        return Err(configuration_refusal(Refusal::Rule(Rule::ConfigTooLarge)));
    }
    // A file every later read refuses is never written.
    Config::parse(&written).map_err(configuration_refusal)?;
    publish(paths, &text, &written, mode)?;
    let holds = |config: &Config| {
        config
            .connectors
            .as_ref()
            .and_then(|connectors| connectors.get(connector))
            .and_then(|table| table.instances.get(instance))
            .and_then(|listed| listed.endpoint.as_deref())
            == Some(endpoint)
    };
    match Config::read_source(&paths.config) {
        Ok((config, read)) if read == written && holds(&config) => Ok(()),
        // Another writer replaced the file meanwhile: its text is not ours
        // to restore over.
        Ok((_, read)) if read != written => Err(host_failure(Failure::ConcurrentRevision)),
        _ => {
            publish(paths, &written, &text, mode)?;
            Err(invalid_configuration())
        }
    }
}

/// Replace the configuration's `old` text with `new`, keeping `mode`, while
/// the file still holds `old`.
fn publish(paths: &Paths, old: &str, new: &str, mode: u32) -> Result<(), HandlerReply> {
    let config = &paths.config;
    let parent = config.parent().ok_or_else(invalid_configuration)?;
    let name = config.file_name().ok_or_else(invalid_configuration)?;
    let directory = filesystem::directory(parent, false, false).map_err(host_failure)?;
    if read(paths)?.0 != old {
        return Err(host_failure(Failure::ConcurrentRevision));
    }
    filesystem::publish_replace_keeping(&directory, name, new.as_bytes(), mode)
        .map_err(host_failure)
}

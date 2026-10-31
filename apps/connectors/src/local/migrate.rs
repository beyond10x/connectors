//! `connectors setup migrate` (story:setup-migrate-adapter-entries;
//! `contracts/cli/v1alpha1/semantics.md` §3, "Migrating adapter entries"):
//! each `[adapters.<alias>]` entry whose implementation maps to a shipped
//! connector declaration becomes the pinned instance
//! `[connectors.<connector>.instances.<alias>]`: in the short form, which the
//! installed catalog provider derives the entry from, when it derives the
//! entry unchanged (story:migrate-writes-the-short-instance-form), otherwise
//! carrying every pin. The new file is read back before anything is written,
//! and an entry that reads back with any other value is not migrated, naming
//! the field.
use super::declarations::CATALOG;
use super::*;
use connectors_host::local::{
    approval_policy,
    config::{self, Adapter, CONFIG_LIMIT, ShortForm},
    filesystem,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    path::Path,
};

/// The entry fields a migrated entry must read back with unchanged, in the
/// order a difference is named (`connectors.cli.MigrateField`).
const FIELDS: [&str; 9] = [
    "instance_id",
    "configuration_revision",
    "adapter_id",
    "protocol",
    "private_protocol",
    "startup",
    "restart",
    "executable",
    "permissions",
];

/// The entry fields a short-form instance must derive unchanged, in the order
/// a difference is named in `pinned_field`.
const SHORT_FIELDS: [&str; 5] = [
    "instance_id",
    "configuration_revision",
    "credential_revision",
    "permissions",
    "startup",
];

/// The keys of the catalog provider's configuration revision digest input
/// (`--print-revision-inputs`), in the order the first differing one is named;
/// `instance` is named `instance_id`. `format` is not named: a difference in
/// it alone is `configuration_revision`.
const REVISION_INPUTS: [&str; 13] = [
    "instance",
    "provider",
    "bundle_sha256",
    "source_sha256",
    "api_base",
    "ca_digest",
    "auth",
    "operations",
    "token_ca_digest",
    "feed",
    "request_prefix",
    "hosts",
    "bindings",
];

/// Why an entry stays `[adapters.<alias>]` (`connectors.cli.MigrateKeptReason`),
/// and for `selection_changes` the field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kept {
    Reason(&'static str),
    Selection(&'static str),
}

/// `setup migrate --dry-run` and `--apply` are `--mode dry-run` and `--mode
/// apply`: `ess-cli/1` has no valueless switch source. Only the arguments of
/// a `setup migrate` command before a first `--` are rewritten.
pub(super) fn shorthand(args: Vec<OsString>) -> Vec<OsString> {
    if !selected(&args) {
        return args;
    }
    let mut rewritten = Vec::with_capacity(args.len() + 1);
    let mut trailing = false;
    for arg in args {
        match arg.to_str() {
            _ if trailing => rewritten.push(arg),
            Some("--") => {
                trailing = true;
                rewritten.push(arg);
            }
            Some("--dry-run") => rewritten.extend(["--mode".into(), "dry-run".into()]),
            Some("--apply") => rewritten.extend(["--mode".into(), "apply".into()]),
            _ => rewritten.push(arg),
        }
    }
    rewritten
}

/// Whether the command path, after any leading process globals, is
/// `setup migrate`.
fn selected(args: &[OsString]) -> bool {
    let mut words = Vec::new();
    let mut rest = args.iter().skip(1);
    while let Some(arg) = rest.next() {
        match arg.to_str() {
            Some("--output" | "--config" | "--state-dir") => {
                rest.next();
            }
            Some(arg)
                if ["--output=", "--config=", "--state-dir="]
                    .iter()
                    .any(|prefix| arg.starts_with(prefix)) => {}
            Some(word) if !word.starts_with('-') && words.len() < 2 => words.push(word),
            _ => break,
        }
    }
    words == ["setup", "migrate"]
}

pub(super) fn execute(call: &Invocation<'_>, paths: &Paths) -> Result<Value, HandlerReply> {
    let apply = match call.input["mode"].as_str() {
        Some("dry-run") => false,
        Some("apply") => true,
        _ => return Err(failure("invalid_input", "arguments", "none", true)),
    };
    let (config, text) = Config::read_source(&paths.config).map_err(configuration_refusal)?;
    let legacy: Vec<String> = config
        .legacy_aliases()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut kept = BTreeMap::new();
    let mut moved = BTreeMap::new();
    for alias in &legacy {
        let decided = connector_of(&config.adapters[alias]).and_then(|connector| {
            if config.format == "connectors-local/1" {
                // `/5` requires `private_protocol`, which a `/1` entry selects
                // none of and which enters its executable selection digest.
                Err(Kept::Selection("private_protocol"))
            } else if config.can_hold(&connector, alias) {
                Ok(connector)
            } else {
                Err(Kept::Reason("instance_conflict"))
            }
        });
        match decided {
            Ok(connector) => {
                moved.insert(alias.clone(), connector);
            }
            Err(why) => {
                kept.insert(alias.clone(), why);
            }
        }
    }
    // Each moved entry's short form, or the field that keeps it pinned.
    let mut short = BTreeMap::new();
    let mut pinned = BTreeMap::new();
    for (alias, connector) in &moved {
        match short_form(paths, connector, alias, &config.adapters[alias]) {
            Ok(form) => {
                short.insert(alias.clone(), form);
            }
            Err(field) => {
                pinned.insert(alias.clone(), field);
            }
        }
    }
    let (written, after) = loop {
        if moved.is_empty() {
            break (text.clone(), config.clone());
        }
        let candidate = config
            .with_short_instances(&moved, &short)
            .map_err(host_failure)?;
        let written = candidate.to_text().map_err(host_failure)?;
        // A file every later read refuses is never planned or written.
        if written.len() > CONFIG_LIMIT {
            return Err(too_large(written.len()));
        }
        let after = match Config::parse(&written) {
            Ok(after) => after,
            // Only a short-form instance's derivation can account for a
            // refusal the pinned file does not have.
            Err(_) if !short.is_empty() => {
                for (alias, _) in std::mem::take(&mut short) {
                    pinned.insert(alias, "configuration_revision");
                }
                continue;
            }
            Err(_) => {
                return Err(failure(
                    "invalid_configuration",
                    "configuration",
                    "check_configuration",
                    true,
                ));
            }
        };
        // Nothing but the moved entries' tables may change.
        if shared(&config) != shared(&after) {
            return Err(host_failure(Failure::InvalidConfiguration));
        }
        // A short-form instance that reads back with another entry is
        // written pinned instead.
        let demoted: Vec<(String, &'static str)> = short
            .keys()
            .filter_map(|alias| {
                let entry = &config.adapters[alias];
                short_differing(entry, after.adapters.get(alias))
                    .map(|field| (alias.clone(), field))
            })
            .collect();
        if !demoted.is_empty() {
            for (alias, field) in demoted {
                short.remove(&alias);
                let field = match (field, after.adapters.get(&alias)) {
                    ("configuration_revision", Some(derived)) => {
                        revision_field(&config.adapters[&alias], derived)
                    }
                    (field, _) => field,
                };
                pinned.insert(alias, field);
            }
            continue;
        }
        let mut differences = Vec::new();
        for (alias, entry) in &config.adapters {
            if short.contains_key(alias) {
                continue;
            }
            if let Some(field) = differing(entry, after.adapters.get(alias)) {
                if !moved.contains_key(alias) {
                    return Err(host_failure(Failure::InvalidConfiguration));
                }
                differences.push((alias.clone(), field));
            }
        }
        differences.extend(
            connectors_differing(&candidate, &after, &moved)
                .map_err(|()| host_failure(Failure::InvalidConfiguration))?,
        );
        if differences.is_empty() {
            break (written, after);
        }
        for (alias, field) in differences {
            // The first difference named for an entry is the one reported.
            if moved.remove(&alias).is_some() {
                short.remove(&alias);
                pinned.remove(&alias);
                kept.insert(alias, Kept::Selection(field));
            }
        }
    };
    let entries: Vec<Value> = legacy
        .iter()
        .map(|alias| {
            let form = match (kept.get(alias), short.contains_key(alias)) {
                (Some(why), _) => Err(why),
                (None, true) => Ok(None),
                (None, false) => Ok(Some(pinned.get(alias).copied().unwrap_or("instance"))),
            };
            entry(alias, &config, &after, form)
        })
        .collect();
    let mut result = json!({
        "mode": if apply { "apply" } else { "dry-run" },
        "disposition": match (moved.is_empty(), apply) {
            (true, _) => "unchanged",
            (false, false) => "planned",
            (false, true) => "applied",
        },
        "config_path": paths.config,
        "format_before": config.format,
        "format_after": after.format,
        "configuration": written,
        "entries": entries,
    });
    if apply && !moved.is_empty() {
        result["backup_path"] = json!(publish(&paths.config, &text, &written)?);
    }
    Ok(result)
}

/// The connector a legacy entry maps to. For a catalog entry, the connector
/// whose shipped declaration names its native configuration's `provider`
/// (`--local-config <file>`), or the shipped connector its `--connector`
/// argument names. No other implementation maps to a connector yet.
fn connector_of(adapter: &Adapter) -> Result<String, Kept> {
    if adapter.adapter_id != CATALOG {
        return Err(Kept::Reason("implementation_unmapped"));
    }
    let declarations = declarations::all();
    let args = &adapter.executable.args;
    if let Some(named) = option(args, "--connector") {
        return declarations
            .iter()
            .find(|declaration| declaration["connector_id"] == named)
            .map(|_| named.to_owned())
            .ok_or(Kept::Reason("connector_undeclared"));
    }
    let unreadable = Kept::Reason("native_configuration_unreadable");
    let native = option(args, "--local-config")
        .and_then(native)
        .ok_or(unreadable)?;
    let provider = native
        .get("provider")
        .and_then(Value::as_str)
        .ok_or(unreadable)?;
    let declaration = declarations
        .iter()
        .find(|declaration| declaration["catalog_provider"] == provider)
        .ok_or(Kept::Reason("connector_undeclared"))?;
    // A request prefix the declaration does not produce (it has no
    // `api_path`) is a gateway in front of the provider's API, which no
    // instance of the connector can state.
    if native.get("request_prefix").is_some_and(|p| !p.is_null())
        && declaration.get("api_path").is_none()
    {
        return Err(Kept::Reason("gateway_prefix"));
    }
    declaration["connector_id"]
        .as_str()
        .map(str::to_owned)
        .ok_or(Kept::Reason("connector_undeclared"))
}

/// The connector `setup migrate` would map the legacy entry `adapter` to, if
/// any: the name `invoke` knows that entry's connector by
/// (story:connectors-do-short-invocation).
pub(super) fn derived_connector(adapter: &Adapter) -> Option<String> {
    connector_of(adapter).ok()
}

/// The value of `--<name> <value>` or `--<name>=<value>` in `args`.
fn option<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == name {
            return rest.next().map(String::as_str);
        }
        if let Some(value) = arg.strip_prefix(name).and_then(|v| v.strip_prefix('=')) {
            return Some(value);
        }
    }
    None
}

/// A native configuration, admitted as the catalog provider admits it: an
/// owner-only file of at most 1 MiB holding a JSON object.
fn native(path: &str) -> Option<serde_json::Map<String, Value>> {
    let file = filesystem::private_file(Path::new(path)).ok()?;
    let bytes = filesystem::read_bounded(file, 1024 * 1024).ok()?;
    match serde_json::from_slice(&bytes).ok()? {
        Value::Object(object) => Some(object),
        _ => None,
    }
}

/// What the migration leaves as it was besides the entries: every top-level
/// key but `format`, `adapters` and `connectors`.
fn shared(config: &Config) -> Value {
    json!([
        config.owner_uid,
        config.approval_clock,
        config.secret_service_socket,
        config.default_adapter,
        config.default_instance,
        config.custody,
        config.consumers,
    ])
}

/// `invalid_configuration` naming `config_too_large`, the rewritten text's
/// size and the bound a read admits.
fn too_large(bytes: usize) -> HandlerReply {
    let mut reply = configuration_refusal(Refusal::Rule(Rule::ConfigTooLarge));
    if let HandlerReply::UsageError { data, .. } = &mut reply {
        data["configuration_bytes"] = json!(bytes);
        data["configuration_limit"] = json!(CONFIG_LIMIT);
    }
    reply
}

/// Where the `[connectors]` table and `format` of `after`, the file read
/// back, differ from `expected`, the configuration written: per moved alias,
/// the first field that keeps it. A moved entry's own instance table names
/// `instance` when it is absent or reads back as another instance, else its
/// differing `description`, `endpoint` or `profile`. A difference in its
/// connector's table (`enabled`, an existing instance's keys) keeps every
/// entry moving to that connector, and a differing `format` every moved
/// entry. A difference no moved entry accounts for is `Err`.
fn connectors_differing(
    expected: &Config,
    after: &Config,
    moved: &BTreeMap<String, String>,
) -> Result<Vec<(String, &'static str)>, ()> {
    if after.format != expected.format {
        if moved.is_empty() {
            return Err(());
        }
        return Ok(moved
            .keys()
            .map(|alias| (alias.clone(), "format"))
            .collect());
    }
    if expected.connectors.is_some() != after.connectors.is_some() {
        return Err(());
    }
    let none = BTreeMap::new();
    let (expected, after) = (
        expected.connectors.as_ref().unwrap_or(&none),
        after.connectors.as_ref().unwrap_or(&none),
    );
    let mut kept = Vec::new();
    // Every entry moving to `connector`, kept naming `field`.
    let keep_all = |connector: &str, field: &'static str, kept: &mut Vec<_>| {
        let aliases: Vec<(String, &'static str)> = moved
            .iter()
            .filter(|(_, to)| to.as_str() == connector)
            .map(|(alias, _)| (alias.clone(), field))
            .collect();
        if aliases.is_empty() {
            return Err(());
        }
        kept.extend(aliases);
        Ok(())
    };
    let names: BTreeSet<&String> = expected.keys().chain(after.keys()).collect();
    for connector in names {
        let (wrote, read) = (expected.get(connector), after.get(connector));
        if wrote.map(|t| t.enabled) != read.map(|t| t.enabled) {
            keep_all(connector, "enabled", &mut kept)?;
            continue;
        }
        let empty = BTreeMap::new();
        let (wrote, read) = (
            wrote.map_or(&empty, |t| &t.instances),
            read.map_or(&empty, |t| &t.instances),
        );
        let instances: BTreeSet<&String> = wrote.keys().chain(read.keys()).collect();
        for name in instances {
            let (a, b) = (wrote.get(name), read.get(name));
            if a == b {
                continue;
            }
            let field = match (a, b) {
                (Some(a), Some(b)) if a.alias == b.alias && a.description != b.description => {
                    "description"
                }
                (Some(a), Some(b)) if a.alias == b.alias && a.endpoint != b.endpoint => "endpoint",
                (Some(a), Some(b)) if a.alias == b.alias && a.profile != b.profile => "profile",
                _ => "instance",
            };
            // An instance this migration added is named like its alias.
            if moved.get(name.as_str()) == Some(connector) {
                kept.push((name.clone(), field));
            } else {
                keep_all(connector, field, &mut kept)?;
            }
        }
    }
    Ok(kept)
}

/// The first field of [`FIELDS`] in which `after` differs from `before`.
fn differing(before: &Adapter, after: Option<&Adapter>) -> Option<&'static str> {
    let Some(after) = after else {
        return Some(FIELDS[0]);
    };
    let (before, after) = (json!(before), json!(after));
    if let Some(field) = FIELDS.into_iter().find(|f| before.get(f) != after.get(f)) {
        return Some(field);
    }
    // Every key is one of FIELDS; any other difference is a new key.
    (before != after).then_some(FIELDS[0])
}

/// One listed entry: its names after the migration, why it was kept or, when
/// migrated, its form (`Ok(None)` short, `Ok(Some(field))` pinned because of
/// `field`), and the records' keys before and after.
fn entry(
    alias: &str,
    before: &Config,
    after: &Config,
    form: Result<Option<&'static str>, &Kept>,
) -> Value {
    let (connector, instance) = after.names(alias).unwrap_or((alias, alias));
    let mut entry = json!({
        "adapter": alias,
        "connector": connector,
        "instance": instance,
        "disposition": if form.is_err() { "kept" } else { "migrated" },
        "before": keys(alias, &before.adapters[alias]),
        "after": after.adapters.get(alias).map_or(Value::Null, |a| keys(alias, a)),
    });
    match form {
        Err(Kept::Reason(reason)) => entry["reason"] = json!(reason),
        Err(Kept::Selection(field)) => {
            entry["reason"] = json!("selection_changes");
            entry["differing_field"] = json!(field);
        }
        Ok(None) => entry["form"] = json!("short"),
        Ok(Some(field)) => {
            entry["form"] = json!("pinned");
            entry["pinned_field"] = json!(field);
        }
    }
    entry
}

/// The short form of the legacy entry `entry`, moving to the instance `alias`
/// of `connector`, or the field that keeps it pinned
/// (`connectors.cli.MigrateField`): `executable` while an approval policy is
/// published for it, `api_base` or `auth` for a native configuration no
/// instance of the connector states, and otherwise what the installed
/// provider derives differently ([`short_differing`], [`revision_field`]).
fn short_form(
    paths: &Paths,
    connector: &str,
    alias: &str,
    entry: &Adapter,
) -> Result<ShortForm, &'static str> {
    // The policy is bound to the executable selection digest, which the
    // short form's argument vector changes.
    let published =
        approval_policy::Store::new(&paths.state, &entry.instance_id, &entry.adapter_id)
            .and_then(|store| store.published());
    if published != Ok(false) {
        return Err("executable");
    }
    let (endpoint, profile) = stated(connector, entry)?;
    let form = ShortForm {
        instance_id: entry.instance_id.clone(),
        endpoint,
        profile,
    };
    let derived =
        config::short_entry(connector, alias, &form).map_err(|_| "configuration_revision")?;
    match short_differing(entry, Some(&derived)) {
        None => Ok(form),
        Some("configuration_revision") => Err(revision_field(entry, &derived)),
        Some(field) => Err(field),
    }
}

/// The endpoint and profile a short-form instance of `connector` states for
/// the legacy catalog entry `entry`, each only where it differs from the
/// declaration's default endpoint and single profile: those of its
/// `--connector` arguments, or those its native configuration's `api_base`
/// (less the declaration's `api_path`) and `auth` profile name.
fn stated(
    connector: &str,
    entry: &Adapter,
) -> Result<(Option<String>, Option<String>), &'static str> {
    let declaration = declarations::all()
        .into_iter()
        .find(|declaration| declaration["connector_id"] == connector)
        .ok_or("configuration_revision")?;
    let profiles: Vec<&str> = declaration["profiles"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|profile| profile["profile"].as_str())
        .collect();
    let args = &entry.executable.args;
    let (endpoint, profile) = if option(args, "--connector").is_some() {
        (
            option(args, "--endpoint").map(str::to_owned),
            option(args, "--profile").map(str::to_owned),
        )
    } else {
        let native = option(args, "--local-config")
            .and_then(native)
            .ok_or("configuration_revision")?;
        let api_base = native
            .get("api_base")
            .and_then(Value::as_str)
            .ok_or("api_base")?;
        let endpoint = match declaration.get("api_path").and_then(Value::as_str) {
            Some(path) => api_base.strip_suffix(path).ok_or("api_base")?,
            None => api_base,
        };
        let profile = native
            .get("auth")
            .and_then(|auth| auth.get("profile"))
            .and_then(Value::as_str)
            .filter(|profile| profiles.contains(profile))
            .ok_or("auth")?;
        (Some(endpoint.to_owned()), Some(profile.to_owned()))
    };
    let default = declaration["default_endpoint"].as_str();
    Ok((
        endpoint.filter(|endpoint| Some(endpoint.as_str()) != default),
        profile.filter(|_| profiles.len() != 1),
    ))
}

/// The first field of [`SHORT_FIELDS`] in which `derived`, a short-form
/// instance's entry, differs from the legacy `entry`; `executable` when the
/// entry states a credential revision, to which a connection binds the
/// executable digest, and the derived digest is another.
fn short_differing(entry: &Adapter, derived: Option<&Adapter>) -> Option<&'static str> {
    let Some(derived) = derived else {
        return Some(SHORT_FIELDS[0]);
    };
    let (before, after) = (json!(entry), json!(derived));
    if let Some(field) = SHORT_FIELDS
        .into_iter()
        .find(|field| before.get(field) != after.get(field))
    {
        return Some(field);
    }
    (entry.credential_revision.is_some() && entry.executable.sha256 != derived.executable.sha256)
        .then_some("executable")
}

/// Why `derived` has another configuration revision than `entry`: the first
/// key of [`REVISION_INPUTS`] whose digest the installed provider prints
/// differently for the two argument vectors, or `configuration_revision`
/// when it prints none or no key accounts for it.
fn revision_field(entry: &Adapter, derived: &Adapter) -> &'static str {
    let (Ok(before), Ok(after)) = (
        config::revision_inputs(&entry.executable.args),
        config::revision_inputs(&derived.executable.args),
    ) else {
        return "configuration_revision";
    };
    REVISION_INPUTS
        .into_iter()
        .find(|key| before.get(*key) != after.get(*key))
        .map_or("configuration_revision", |key| match key {
            "instance" => "instance_id",
            key => key,
        })
}

/// What an entry's records are keyed by (`connectors.cli.MigrateSelection`):
/// its connections and approval policy by instance id, configuration revision
/// and executable selection digest, its mounted custody items by
/// `<alias>/<profile>`.
fn keys(alias: &str, adapter: &Adapter) -> Value {
    json!({
        "instance_id": adapter.instance_id,
        "configuration_revision": adapter.configuration_revision,
        "executable_selection": adapter.selection(),
        "custody_items": adapter
            .permissions
            .profiles
            .iter()
            .map(|profile| format!("{alias}/{profile}"))
            .collect::<Vec<_>>(),
    })
}

/// Keep `old`, the text the plan was made from, owner-only as
/// `<file>.before-migrate-<UTC timestamp>` beside the configuration, then
/// replace the configuration with `new` atomically and owner-only, but only
/// while it still holds `old`. Returns the backup's path.
fn publish(config: &Path, old: &str, new: &str) -> Result<String, HandlerReply> {
    let invalid = || host_failure(Failure::InvalidConfiguration);
    let parent = config.parent().ok_or_else(invalid)?;
    let name = config.file_name().ok_or_else(invalid)?;
    let directory = filesystem::directory(parent, false, false).map_err(host_failure)?;
    let backup = format!(
        "{}.before-migrate-{}",
        name.to_str().ok_or_else(invalid)?,
        stamp(std::time::SystemTime::now())
    );
    filesystem::publish_new(&directory, OsStr::new(&backup), old.as_bytes())
        .map_err(host_failure)?;
    let (_, current) = Config::read_source(config).map_err(configuration_refusal)?;
    if current != old {
        return Err(host_failure(Failure::ConcurrentRevision));
    }
    filesystem::publish_replace(&directory, name, new.as_bytes()).map_err(host_failure)?;
    parent
        .join(backup)
        .to_str()
        .map(str::to_owned)
        .ok_or_else(invalid)
}

/// `at` in UTC as `YYYYMMDDTHHMMSSZ`.
fn stamp(at: std::time::SystemTime) -> String {
    let seconds = at
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    // Days since 1970-01-01 to a proleptic Gregorian date (H. Hinnant's
    // `civil_from_days`), for days on or after the epoch.
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn a_timestamp_is_utc_in_basic_iso_8601() {
        let at = |seconds| stamp(UNIX_EPOCH + Duration::from_secs(seconds));
        assert_eq!(at(0), "19700101T000000Z");
        assert_eq!(at(951_782_400), "20000229T000000Z");
        assert_eq!(at(1_792_150_245), "20261016T113045Z");
        assert_eq!(at(4_107_542_399), "21000228T235959Z");
    }

    #[test]
    fn only_a_setup_migrate_command_reads_the_shorthand() {
        let args = |words: &[&str]| words.iter().map(OsString::from).collect::<Vec<_>>();
        assert_eq!(
            shorthand(args(&[
                "c",
                "--output",
                "json",
                "setup",
                "migrate",
                "--dry-run"
            ])),
            args(&[
                "c", "--output", "json", "setup", "migrate", "--mode", "dry-run"
            ])
        );
        assert_eq!(
            shorthand(args(&["c", "setup", "migrate", "--apply", "--", "--apply"])),
            args(&["c", "setup", "migrate", "--mode", "apply", "--", "--apply"])
        );
        for unchanged in [
            &["c", "setup", "init", "--apply"][..],
            &["c", "operations", "describe", "--dry-run"],
            &["c", "migrate", "setup", "--apply"],
        ] {
            assert_eq!(shorthand(args(unchanged)), args(unchanged));
        }
    }

    #[test]
    fn a_difference_names_the_first_differing_field() {
        let entry: Adapter = serde_json::from_value(json!({
            "instance_id":"a","adapter_id":"catalog","configuration_revision":"r",
            "protocol":"v1alpha1","private_protocol":"connectors-private/1",
            "executable":{"path":"/x","sha256":"0".repeat(64),"args":[]}}))
        .unwrap();
        assert_eq!(differing(&entry, Some(&entry)), None);
        assert_eq!(differing(&entry, None), Some("instance_id"));
        let mut other = entry.clone();
        other.private_protocol = None;
        assert_eq!(differing(&entry, Some(&other)), Some("private_protocol"));
        let mut other = entry.clone();
        other.executable.args.push("--x".into());
        assert_eq!(differing(&entry, Some(&other)), Some("executable"));
        let mut other = entry.clone();
        other.permissions.profiles.insert("p".into());
        assert_eq!(differing(&entry, Some(&other)), Some("permissions"));
    }

    /// A short form is kept unless its derivation changes what a connection,
    /// custody item or policy is keyed by; its argument vector and private
    /// protocol are its own, and its executable digest matters only to a
    /// connection bound to a credential revision.
    #[test]
    fn a_short_form_names_the_first_field_its_derivation_changes() {
        let entry: Adapter = serde_json::from_value(json!({
            "instance_id":"a","adapter_id":"catalog","configuration_revision":"r",
            "protocol":"v1alpha1","private_protocol":"connectors-private/1",
            "executable":{"path":"/x","sha256":"0".repeat(64),"args":["--local-config","/n"]},
            "permissions":{"profiles":["p"],"operations":["o"]}}))
        .unwrap();
        assert_eq!(short_differing(&entry, None), Some("instance_id"));
        let mut derived = entry.clone();
        derived.executable.args = vec!["--connector".into(), "c".into()];
        derived.executable.sha256 = "1".repeat(64);
        derived.private_protocol = None;
        assert_eq!(short_differing(&entry, Some(&derived)), None);
        let changed = |change: &dyn Fn(&mut Adapter)| {
            let mut other = derived.clone();
            change(&mut other);
            short_differing(&entry, Some(&other))
        };
        assert_eq!(
            changed(&|a| a.instance_id = "c.a".into()),
            Some("instance_id")
        );
        assert_eq!(
            changed(&|a| a.configuration_revision = "s".into()),
            Some("configuration_revision")
        );
        assert_eq!(
            changed(&|a| a.credential_revision = Some("k".into())),
            Some("credential_revision")
        );
        assert_eq!(
            changed(&|a| {
                a.permissions.operations.insert("w".into());
            }),
            Some("permissions")
        );
        let mut bound = entry.clone();
        bound.credential_revision = Some("k".into());
        let mut same = derived.clone();
        same.credential_revision = Some("k".into());
        assert_eq!(short_differing(&bound, Some(&same)), Some("executable"));
        same.executable.sha256 = bound.executable.sha256.clone();
        assert_eq!(short_differing(&bound, Some(&same)), None);
    }

    /// The `[connectors]` table read back is compared with the one written:
    /// a moved entry's new instance table, its connector's existing table
    /// (`enabled`, existing instances' keys) and `format`. A difference keeps
    /// the entries it belongs to and names the field; one that belongs to no
    /// moved entry refuses.
    #[cfg(unix)]
    #[test]
    fn a_connectors_table_that_reads_back_differently_keeps_its_entries() {
        let pins = |table: &str, alias: &str, extra: &str| {
            format!(
                "[{table}]\n{extra}instance_id='{alias}-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-{alias}'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[{table}.executable]\npath='/opt/connectors/bin/catalog'\nsha256='{}'\nargs=[]\n",
                "1".repeat(64)
            )
        };
        let text = format!(
            "format='connectors-local/5'\nowner_uid={}\n{}{}[connectors.gmail]\nenabled=true\n{}[connectors.drive]\nenabled=true\n",
            filesystem::uid(),
            pins("adapters.mail", "mail", ""),
            pins("adapters.files", "files", ""),
            pins(
                "connectors.gmail.instances.old",
                "pinned",
                "alias='pinned'\nendpoint='https://example.invalid'\n"
            ),
        );
        let config = Config::parse(&text).unwrap();
        let moved: BTreeMap<String, String> = [("mail", "gmail"), ("files", "calendar")]
            .into_iter()
            .map(|(a, c)| (a.to_owned(), c.to_owned()))
            .collect();
        let written = config.with_instances(&moved).unwrap();
        let after = Config::parse(&written.to_text().unwrap()).unwrap();
        assert_eq!(connectors_differing(&written, &after, &moved), Ok(vec![]));
        let changed = |change: &dyn Fn(&mut Config)| {
            let mut after = after.clone();
            change(&mut after);
            connectors_differing(&written, &after, &moved)
        };
        fn tables(
            config: &mut Config,
        ) -> &mut BTreeMap<String, connectors_host::local::config::Connector> {
            config.connectors.as_mut().unwrap()
        }
        let kept = |pairs: &[(&str, &'static str)]| {
            Ok(pairs
                .iter()
                .map(|(alias, field)| ((*alias).to_owned(), *field))
                .collect::<Vec<_>>())
        };
        // The new instance table of `mail`.
        assert_eq!(
            changed(&|c| {
                tables(c)
                    .get_mut("gmail")
                    .unwrap()
                    .instances
                    .get_mut("mail")
                    .unwrap()
                    .description = Some("x".into())
            }),
            kept(&[("mail", "description")])
        );
        assert_eq!(
            changed(&|c| {
                tables(c).get_mut("gmail").unwrap().instances.remove("mail");
            }),
            kept(&[("mail", "instance")])
        );
        assert_eq!(
            changed(&|c| {
                tables(c)
                    .get_mut("calendar")
                    .unwrap()
                    .instances
                    .get_mut("files")
                    .unwrap()
                    .profile = Some("p".into())
            }),
            kept(&[("files", "profile")])
        );
        // An existing instance of the connector `mail` moves to.
        assert_eq!(
            changed(&|c| {
                tables(c)
                    .get_mut("gmail")
                    .unwrap()
                    .instances
                    .get_mut("old")
                    .unwrap()
                    .endpoint = None
            }),
            kept(&[("mail", "endpoint")])
        );
        assert_eq!(
            changed(&|c| tables(c).get_mut("calendar").unwrap().enabled = false),
            kept(&[("files", "enabled")])
        );
        // The format keeps every moved entry.
        assert_eq!(
            changed(&|c| c.format = "connectors-local/4".into()),
            kept(&[("files", "format"), ("mail", "format")])
        );
        // A connector no moved entry belongs to.
        assert_eq!(
            changed(&|c| tables(c).get_mut("drive").unwrap().enabled = false),
            Err(())
        );
    }
}

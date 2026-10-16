//! `connectors setup migrate` (story:setup-migrate-adapter-entries;
//! `contracts/cli/v1alpha1/semantics.md` §3, "Migrating adapter entries"):
//! each `[adapters.<alias>]` entry whose implementation maps to a shipped
//! connector declaration becomes the pinned instance
//! `[connectors.<connector>.instances.<alias>]`, carrying every pin. The new
//! file is read back before anything is written, and an entry that reads back
//! with any other value is not migrated, naming the field.
use super::*;
use connectors_host::local::{
    config::{Adapter, CONFIG_LIMIT},
    filesystem,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    path::Path,
};

/// The connector declarations the catalog provider ships
/// (`adapters/catalog/providers/<provider>/connector.json`), compiled in: the
/// provider and this binary are released together. `tests/setup_migrate.rs`
/// holds this list to every provider directory of the tree.
const DECLARATIONS: &[&str] = &[
    include_str!("../../../../adapters/catalog/providers/confluence/connector.json"),
    include_str!("../../../../adapters/catalog/providers/confluence-v1/connector.json"),
    include_str!("../../../../adapters/catalog/providers/gitlab/connector.json"),
    include_str!("../../../../adapters/catalog/providers/google-calendar/connector.json"),
    include_str!("../../../../adapters/catalog/providers/google-drive/connector.json"),
    include_str!("../../../../adapters/catalog/providers/google-gmail/connector.json"),
    include_str!("../../../../adapters/catalog/providers/google-slides/connector.json"),
    include_str!("../../../../adapters/catalog/providers/hubspot/connector.json"),
    include_str!("../../../../adapters/catalog/providers/jira/connector.json"),
    include_str!("../../../../adapters/catalog/providers/opsgenie/connector.json"),
    include_str!("../../../../adapters/catalog/providers/runpod/connector.json"),
    include_str!("../../../../adapters/catalog/providers/zendesk/connector.json"),
];

/// The catalog's implementation id, the only one a declaration maps today.
const CATALOG: &str = "catalog";

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
    let (written, after) = loop {
        if moved.is_empty() {
            break (text.clone(), config.clone());
        }
        let candidate = config.with_instances(&moved).map_err(host_failure)?;
        let written = candidate.to_text().map_err(host_failure)?;
        // A file every later read refuses is never planned or written.
        if written.len() > CONFIG_LIMIT {
            return Err(too_large(written.len()));
        }
        let after = Config::parse(&written).map_err(|_| {
            failure(
                "invalid_configuration",
                "configuration",
                "check_configuration",
                true,
            )
        })?;
        // Nothing but the moved entries' tables may change.
        if shared(&config) != shared(&after) {
            return Err(host_failure(Failure::InvalidConfiguration));
        }
        let mut differences = Vec::new();
        for (alias, entry) in &config.adapters {
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
                kept.insert(alias, Kept::Selection(field));
            }
        }
    };
    let entries: Vec<Value> = legacy
        .iter()
        .map(|alias| entry(alias, &config, &after, kept.get(alias)))
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
    let declarations = declarations();
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

/// The shipped catalog declarations.
fn declarations() -> Vec<Value> {
    DECLARATIONS
        .iter()
        .filter_map(|text| serde_json::from_str::<Value>(text).ok())
        .filter(|declaration| declaration["adapter_id"] == CATALOG)
        .collect()
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

/// One listed entry: its names after the migration, why it was kept, and the
/// records' keys before and after.
fn entry(alias: &str, before: &Config, after: &Config, kept: Option<&Kept>) -> Value {
    let (connector, instance) = after.names(alias).unwrap_or((alias, alias));
    let mut entry = json!({
        "adapter": alias,
        "connector": connector,
        "instance": instance,
        "disposition": if kept.is_some() { "kept" } else { "migrated" },
        "before": keys(alias, &before.adapters[alias]),
        "after": after.adapters.get(alias).map_or(Value::Null, |a| keys(alias, a)),
    });
    match kept {
        Some(Kept::Reason(reason)) => entry["reason"] = json!(reason),
        Some(Kept::Selection(field)) => {
            entry["reason"] = json!("selection_changes");
            entry["differing_field"] = json!(field);
        }
        None => {}
    }
    entry
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
            &["c", "operations", "invoke", "--dry-run"],
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

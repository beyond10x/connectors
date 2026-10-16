//! story:setup-migrate-adapter-entries: `connectors setup migrate` rewrites each
//! `[adapters.<alias>]` entry whose implementation maps to a shipped connector
//! as the pinned instance `[connectors.<connector>.instances.<alias>]`, keeping
//! every pin, so its instance id, configuration revision, executable selection
//! digest and mounted custody item names stay byte for byte
//! (`contracts/cli/v1alpha1/semantics.md` §3, "Migrating adapter entries").
//! Every other entry stays where it is and the dry run says why.
#![cfg(unix)]
use connectors_host::local::config::Config;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

/// A directory holding no executable: nothing here is started.
const NOT_INSTALLED: &str = "/not-installed";

fn command(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["--output", "json", "--config"])
        .arg(root.join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(args)
        .output()
        .unwrap()
}

/// The success envelope's result: exactly one envelope on stdout, nothing on
/// standard error.
fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{output:?}");
    let text = String::from_utf8(output.stdout.clone()).unwrap();
    assert_eq!(text.lines().count(), 1, "{text}");
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["ok"], true);
    value["result"].clone()
}

/// The one failure envelope on standard error, nothing on stdout.
fn refusal(output: &Output) -> (Option<i32>, Value) {
    assert!(!output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let text = String::from_utf8(output.stderr.clone()).unwrap();
    assert_eq!(text.lines().count(), 1, "{text}");
    let value: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["ok"], false, "{value}");
    (output.status.code(), value)
}

fn private(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

/// One `[adapters.<alias>]` entry of implementation `adapter_id`, whose
/// executable takes `args`.
struct Entry {
    alias: String,
    adapter_id: String,
    args: Vec<String>,
}

impl Entry {
    fn new(alias: &str, adapter_id: &str, args: &[&str]) -> Self {
        Self {
            alias: alias.into(),
            adapter_id: adapter_id.into(),
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        }
    }

    /// A catalog entry whose native configuration, written owner-only under
    /// `root`, states `native` (its `provider` and anything else).
    fn catalog(root: &Path, alias: &str, native: Value) -> Self {
        let path = root.join("native").join(format!("{alias}.json"));
        private(&path, &native.to_string());
        Self::new(
            alias,
            "catalog",
            &["--local-config", path.to_str().unwrap()],
        )
    }

    fn toml(&self, private_protocol: bool) -> String {
        let alias = &self.alias;
        let protocol = if private_protocol {
            "private_protocol='connectors-private/1'\n"
        } else {
            ""
        };
        format!(
            "[adapters.{alias}]\ninstance_id='{alias}-local'\nadapter_id='{}'\nconfiguration_revision='cfg-{alias}'\nprotocol='v1alpha1'\n{protocol}[adapters.{alias}.permissions]\nprofiles=['fixture.token']\noperations=['item.read']\n[adapters.{alias}.executable]\npath='{NOT_INSTALLED}/{}'\nsha256='{}'\nargs={}\n",
            self.adapter_id,
            self.adapter_id,
            "a".repeat(64),
            serde_json::to_string(&self.args).unwrap()
        )
    }
}

/// An initialized root whose configuration is `format` with `extra` top-level
/// keys and the entries `entries` makes, given the root, under which their
/// native configurations are written. Returns the root and the
/// configuration's bytes.
fn configured(
    format: &str,
    extra: &str,
    entries: impl FnOnce(&Path) -> Vec<Entry>,
) -> (tempfile::TempDir, String) {
    let root = tempfile::tempdir().unwrap();
    success(&command(root.path(), &["setup", "init"]));
    let entries = entries(root.path());
    let path = root.path().join("config/config.toml");
    let uid = connectors_host::local::filesystem::uid();
    let private_protocol = format != "connectors-local/1";
    let body: String = entries.iter().map(|e| e.toml(private_protocol)).collect();
    let text = format!("# operator notes\nformat = '{format}'\nowner_uid = {uid}\n{extra}\n{body}");
    private(&path, &text);
    (root, text)
}

/// The entries of a result by alias.
fn entries(result: &Value) -> BTreeMap<String, Value> {
    result["entries"]
        .as_array()
        .unwrap_or_else(|| panic!("{result}"))
        .iter()
        .map(|entry| (entry["adapter"].as_str().unwrap().to_owned(), entry.clone()))
        .collect()
}

/// The source tree's `adapters/catalog/providers`.
fn providers() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/catalog/providers")
}

/// Every provider directory the catalog ships, with the connector its
/// declaration names, `None` where it ships no declaration. Read from the
/// tree, so a declaration added later is held here without editing the test.
fn shipped() -> BTreeMap<String, Option<String>> {
    fs::read_dir(providers())
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let directory = entry.file_name().into_string().unwrap();
            let connector = fs::read(entry.path().join("connector.json"))
                .ok()
                .map(|bytes| {
                    let declaration: Value = serde_json::from_slice(&bytes).unwrap();
                    assert_eq!(declaration["catalog_provider"], directory.as_str());
                    declaration["connector_id"].as_str().unwrap().to_owned()
                });
            (directory, connector)
        })
        .collect()
}

/// For a catalog entry, the connector whose shipped declaration names the
/// native configuration's `provider`: every provider directory with a
/// declaration migrates under that declaration's connector, and every one
/// without (Slack) stays an adapter entry naming `connector_undeclared`.
#[test]
fn every_shipped_declaration_maps_its_native_provider_and_no_other_does() {
    let shipped = shipped();
    assert!(shipped.values().filter(|c| c.is_some()).count() >= 12);
    assert!(shipped.values().any(Option::is_none), "{shipped:?}");
    let (root, before) = configured("connectors-local/4", "", |scratch| {
        shipped
            .keys()
            .map(|provider| {
                Entry::catalog(
                    scratch,
                    &format!("p-{provider}"),
                    json!({"provider": provider, "api_base": "https://fixture.invalid"}),
                )
            })
            .collect()
    });
    let result = success(&command(
        root.path(),
        &["setup", "migrate", "--mode", "dry-run"],
    ));
    assert_eq!(result["disposition"], "planned", "{result}");
    let listed = entries(&result);
    assert_eq!(listed.len(), shipped.len(), "{result}");
    for (provider, connector) in &shipped {
        let alias = format!("p-{provider}");
        let entry = &listed[&alias];
        match connector {
            Some(connector) => {
                assert_eq!(entry["disposition"], "migrated", "{entry}");
                assert_eq!(
                    (&entry["connector"], &entry["instance"]),
                    (&json!(connector), &json!(alias)),
                    "{entry}"
                );
                assert!(entry.get("reason").is_none(), "{entry}");
                let table = format!("[connectors.{connector}.instances.{alias}]");
                assert!(
                    result["configuration"].as_str().unwrap().contains(&table),
                    "{table}"
                );
            }
            None => {
                assert_eq!(entry["disposition"], "kept", "{entry}");
                assert_eq!(entry["reason"], "connector_undeclared", "{entry}");
                assert_eq!(
                    (&entry["connector"], &entry["instance"]),
                    (&json!("catalog"), &json!(alias)),
                    "{entry}"
                );
            }
        }
        assert_eq!(entry["before"], entry["after"], "{entry}");
    }
    // A dry run writes nothing.
    assert_eq!(
        fs::read_to_string(root.path().join("config/config.toml")).unwrap(),
        before
    );
}

/// The fixture's kinds: two catalog entries that migrate, Slack and an
/// Atlassian gateway entry that stay, and one entry of each non-catalog
/// implementation (credential binary, MCP, SQL, Kubernetes), which no
/// connector maps.
fn every_kind(scratch: &Path) -> Vec<Entry> {
    vec![
        Entry::catalog(
            scratch,
            "mail",
            json!({"provider":"google-gmail","api_base":"https://gmail.googleapis.com"}),
        ),
        Entry::catalog(
            scratch,
            "tracker",
            json!({"provider":"jira","api_base":"https://example.atlassian.net"}),
        ),
        Entry::catalog(
            scratch,
            "chat",
            json!({"provider":"slack","api_base":"https://slack.com/api"}),
        ),
        Entry::catalog(
            scratch,
            "tracker-gw",
            json!({"provider":"jira","api_base":"https://api.atlassian.com/ex/jira/cloud-1","request_prefix":"/ex/jira/cloud-1"}),
        ),
        Entry::new("search", "tavily", &[]),
        Entry::new("tools", "mcp", &["--server", "fixture"]),
        Entry::new("warehouse", "sql", &[]),
        Entry::new("cluster", "kubernetes", &[]),
    ]
}

/// What decides each kind, and what the rewritten file holds: the migrated
/// entries as pinned instances under their alias, every other entry as it was,
/// and per entry the same instance id, revision, executable selection and
/// custody item names before and after.
#[test]
fn each_kind_is_migrated_or_kept_naming_why_and_keeps_its_records() {
    let (root, before) = configured("connectors-local/4", "", every_kind);
    let old = Config::load(&root.path().join("config/config.toml")).unwrap();
    let result = success(&command(root.path(), &["setup", "migrate", "--dry-run"]));
    assert_eq!(result["mode"], "dry-run", "{result}");
    assert_eq!(result["disposition"], "planned", "{result}");
    assert_eq!(result["format_before"], "connectors-local/4", "{result}");
    assert_eq!(result["format_after"], "connectors-local/5", "{result}");
    assert!(result.get("backup_path").is_none(), "{result}");
    let listed = entries(&result);
    let expected = [
        ("mail", "migrated", None, "gmail"),
        ("tracker", "migrated", None, "jira"),
        ("chat", "kept", Some("connector_undeclared"), "catalog"),
        ("tracker-gw", "kept", Some("gateway_prefix"), "catalog"),
        ("search", "kept", Some("implementation_unmapped"), "tavily"),
        ("tools", "kept", Some("implementation_unmapped"), "mcp"),
        ("warehouse", "kept", Some("implementation_unmapped"), "sql"),
        (
            "cluster",
            "kept",
            Some("implementation_unmapped"),
            "kubernetes",
        ),
    ];
    assert_eq!(listed.len(), expected.len(), "{result}");
    for (alias, disposition, reason, connector) in expected {
        let entry = &listed[alias];
        assert_eq!(entry["disposition"], disposition, "{entry}");
        assert_eq!(
            entry.get("reason").cloned(),
            reason.map(|r| json!(r)),
            "{entry}"
        );
        assert_eq!(
            (&entry["connector"], &entry["instance"]),
            (&json!(connector), &json!(alias)),
            "{entry}"
        );
        let adapter = &old.adapters[alias];
        let selection = json!({
            "instance_id": adapter.instance_id,
            "configuration_revision": adapter.configuration_revision,
            "executable_selection": adapter.selection(),
            "custody_items": [format!("{alias}/fixture.token")],
        });
        assert_eq!(entry["before"], selection, "{entry}");
        assert_eq!(entry["after"], selection, "{entry}");
    }
    let file = result["configuration"].as_str().unwrap();
    let new = toml_config(root.path(), file);
    for (alias, disposition, _, connector) in expected {
        let adapter = &new.adapters[alias];
        let old = &old.adapters[alias];
        assert_eq!(
            (
                &adapter.instance_id,
                &adapter.configuration_revision,
                adapter.selection()
            ),
            (
                &old.instance_id,
                &old.configuration_revision,
                old.selection()
            ),
            "{alias}"
        );
        assert_eq!(new.names(alias), Some((connector, alias)), "{alias}");
        let table = format!("[connectors.{connector}.instances.{alias}]");
        let legacy = format!("[adapters.{alias}]");
        if disposition == "migrated" {
            assert!(file.contains(&table) && !file.contains(&legacy), "{file}");
        } else {
            assert!(file.contains(&legacy) && !file.contains(&table), "{file}");
        }
    }
    assert_eq!(new.format, "connectors-local/5");
    assert_eq!(
        fs::read_to_string(root.path().join("config/config.toml")).unwrap(),
        before,
        "a dry run writes nothing"
    );
    assert!(backups(root.path()).is_empty());
}

/// The text `file` read as a configuration file next to the real one.
fn toml_config(root: &Path, file: &str) -> Config {
    let path = root.join("config/printed.toml");
    private(&path, file);
    let config = Config::load(&path).unwrap();
    fs::remove_file(path).unwrap();
    config
}

/// The backups beside the configuration, by name.
fn backups(root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(root.join("config"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("config.toml.before-migrate-")
        })
        .collect();
    found.sort();
    found
}

/// `--apply` (and `--mode apply`) writes exactly the file the dry run printed,
/// owner-only, and keeps the old bytes owner-only beside it as
/// `config.toml.before-migrate-<UTC timestamp>`. A second run finds nothing
/// left to migrate and writes nothing.
#[test]
fn apply_writes_the_printed_file_owner_only_and_keeps_the_old_one() {
    for apply in [&["--apply"][..], &["--mode", "apply"]] {
        let (root, before) = configured("connectors-local/4", "", every_kind);
        let path = root.path().join("config/config.toml");
        let planned = success(&command(root.path(), &["setup", "migrate", "--dry-run"]));
        let applied = success(&command(
            root.path(),
            &[&["setup", "migrate"][..], apply].concat(),
        ));
        assert_eq!(applied["mode"], "apply", "{applied}");
        assert_eq!(applied["disposition"], "applied", "{applied}");
        assert_eq!(applied["configuration"], planned["configuration"]);
        assert_eq!(applied["entries"], planned["entries"]);
        let written = fs::read_to_string(&path).unwrap();
        assert_eq!(written, planned["configuration"].as_str().unwrap());
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let backups = backups(root.path());
        assert_eq!(backups.len(), 1, "{backups:?}");
        let backup = &backups[0];
        assert_eq!(
            applied["backup_path"],
            backup.to_str().unwrap(),
            "{applied}"
        );
        assert_eq!(fs::read_to_string(backup).unwrap(), before);
        assert_eq!(
            fs::metadata(backup).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let stamp = backup
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .trim_start_matches("config.toml.before-migrate-")
            .to_owned();
        // `YYYYMMDDTHHMMSSZ`
        assert_eq!(stamp.len(), 16, "{stamp}");
        assert!(
            stamp[..8].bytes().all(|b| b.is_ascii_digit())
                && &stamp[8..9] == "T"
                && stamp[9..15].bytes().all(|b| b.is_ascii_digit())
                && stamp.ends_with('Z'),
            "{stamp}"
        );
        // The migrated instance answers to its new names and its alias.
        let by_instance = success(&command(
            root.path(),
            &[
                "instances",
                "describe",
                "--connector",
                "gmail",
                "--instance",
                "mail",
            ],
        ));
        assert_eq!(by_instance["configuration"]["summary"]["adapter"], "mail");
        assert_eq!(
            by_instance["configuration"]["summary"]["instance_id"],
            "mail-local"
        );

        let again = success(&command(root.path(), apply_args(apply).as_slice()));
        assert_eq!(again["disposition"], "unchanged", "{again}");
        assert!(again.get("backup_path").is_none(), "{again}");
        assert_eq!(fs::read_to_string(&path).unwrap(), written);
        assert_eq!(self::backups(root.path()).len(), 1);
        assert!(
            entries(&again)
                .values()
                .all(|entry| entry["disposition"] == "kept"),
            "{again}"
        );
    }
}

fn apply_args<'a>(apply: &[&'a str]) -> Vec<&'a str> {
    [&["setup", "migrate"][..], apply].concat()
}

/// A configuration with nothing to migrate is answered `unchanged` and is
/// neither rewritten nor backed up, in either mode.
#[test]
fn nothing_to_migrate_writes_nothing() {
    let (root, before) = configured("connectors-local/4", "", |_| {
        vec![Entry::new("tools", "mcp", &[])]
    });
    for mode in ["dry-run", "apply"] {
        let result = success(&command(root.path(), &["setup", "migrate", "--mode", mode]));
        assert_eq!(result["disposition"], "unchanged", "{result}");
        assert_eq!(result["format_after"], "connectors-local/4", "{result}");
        assert_eq!(result["configuration"], before.as_str(), "{result}");
    }
    assert_eq!(
        fs::read_to_string(root.path().join("config/config.toml")).unwrap(),
        before
    );
    assert!(backups(root.path()).is_empty());
}

/// A `connectors-local/1` entry selects no `private_protocol`, which
/// `connectors-local/5` requires and which enters the executable selection
/// digest: moving it would change the selection an approval policy is bound
/// to, so nothing is migrated and each candidate names that field.
#[test]
fn an_entry_whose_selection_would_change_is_kept_naming_the_field() {
    let (root, before) = configured("connectors-local/1", "", |scratch| {
        vec![
            Entry::catalog(scratch, "mail", json!({"provider":"google-gmail"})),
            Entry::new("tools", "mcp", &[]),
        ]
    });
    let result = success(&command(root.path(), &["setup", "migrate", "--apply"]));
    assert_eq!(result["disposition"], "unchanged", "{result}");
    let listed = entries(&result);
    assert_eq!(listed["mail"]["disposition"], "kept");
    assert_eq!(listed["mail"]["reason"], "selection_changes");
    assert_eq!(listed["mail"]["differing_field"], "private_protocol");
    assert_eq!(listed["tools"]["reason"], "implementation_unmapped");
    assert!(listed["tools"].get("differing_field").is_none());
    assert_eq!(
        fs::read_to_string(root.path().join("config/config.toml")).unwrap(),
        before
    );
    assert!(backups(root.path()).is_empty());
}

/// A catalog entry is kept, naming why, when its native configuration cannot
/// be read or names no provider, and when its connector cannot hold it: the
/// connector is configured disabled, or already has an instance of that name.
#[test]
fn an_entry_the_connector_cannot_hold_or_without_a_readable_native_configuration_is_kept() {
    // A pinned `gmail` instance named `taken`, the legacy entry `pinned`.
    let table = "connectors.gmail.instances.taken";
    let extra = format!(
        "[connectors.drive]\nenabled = false\n[connectors.gmail]\nenabled = true\n[{table}]\nalias='pinned'\ninstance_id='pinned-local'\nadapter_id='catalog'\nconfiguration_revision='cfg-pinned'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[{table}.executable]\npath='{NOT_INSTALLED}/catalog'\nsha256='{}'\nargs=[]\n",
        "a".repeat(64)
    );
    let (root, _) = configured("connectors-local/5", &extra, |scratch| {
        let mut entries = vec![
            Entry::new(
                "missing",
                "catalog",
                &["--local-config", "/not-installed/x.json"],
            ),
            Entry::new("bare", "catalog", &[]),
            Entry::catalog(
                scratch,
                "unnamed",
                json!({"api_base":"https://fixture.invalid"}),
            ),
            Entry::catalog(scratch, "files", json!({"provider":"google-drive"})),
            Entry::catalog(scratch, "taken", json!({"provider":"google-gmail"})),
        ];
        let open = scratch.join("native/open.json");
        private(&open, &json!({"provider":"google-gmail"}).to_string());
        fs::set_permissions(&open, fs::Permissions::from_mode(0o644)).unwrap();
        entries.push(Entry::new(
            "open",
            "catalog",
            &["--local-config", open.to_str().unwrap()],
        ));
        entries
    });
    let result = success(&command(root.path(), &["setup", "migrate", "--dry-run"]));
    let listed = entries(&result);
    for alias in ["missing", "bare", "unnamed", "open"] {
        assert_eq!(
            (&listed[alias]["disposition"], &listed[alias]["reason"]),
            (&json!("kept"), &json!("native_configuration_unreadable")),
            "{alias}: {result}"
        );
    }
    for alias in ["files", "taken"] {
        assert_eq!(
            (&listed[alias]["disposition"], &listed[alias]["reason"]),
            (&json!("kept"), &json!("instance_conflict")),
            "{alias}: {result}"
        );
    }
    // An entry that is already an instance is not a candidate.
    assert!(!listed.contains_key("pinned"), "{result}");
    assert_eq!(result["disposition"], "unchanged", "{result}");
}

/// Under `--output json` a refusal is one envelope on standard error and
/// nothing else; a missing mode is a parse refusal.
#[test]
fn a_refusal_is_one_json_envelope() {
    let root = tempfile::tempdir().unwrap();
    let (code, envelope) = refusal(&command(root.path(), &["setup", "migrate", "--dry-run"]));
    assert_eq!(code, Some(2), "{envelope}");
    assert_eq!(envelope["error"]["data"]["code"], "invalid_configuration");
    assert_eq!(
        envelope["error"]["data"]["configuration_rule"],
        "config_missing"
    );
    let (code, envelope) = refusal(&command(root.path(), &["setup", "migrate"]));
    assert_eq!(code, Some(2), "{envelope}");
    assert_eq!(envelope["error"]["code"], "cli_parse", "{envelope}");
    let (code, envelope) = refusal(&command(
        root.path(),
        &["setup", "migrate", "--dry-run", "--apply"],
    ));
    assert_eq!(code, Some(2), "{envelope}");
    assert_eq!(envelope["error"]["code"], "cli_parse", "{envelope}");
}

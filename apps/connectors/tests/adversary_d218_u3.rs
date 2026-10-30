//! Adversary pass on wave D218 unit U3 (story:connectors-do-short-invocation):
//! `operations invoke` is removed with every caller migrated, and `instances
//! list` names each instance by its connector and instance only, a legacy
//! `[adapters.<alias>]` catalog entry by the connector `setup migrate`
//! derives for it, never `catalog`; an entry no connector is derived for is
//! named in `unmigrated` and refused `legacy_entry_unmigrated` when selected.
//!
//! The configurations here use executables that are not installed, with
//! cached descriptions: nothing they run can reach a provider.
use connectors_host::local::{
    config::{Config, Paths},
    registry, runtime,
};
use serde_json::{Value, json};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

const NOT_INSTALLED: &str = if cfg!(windows) {
    "C:/not-installed"
} else {
    "/not-installed"
};
const READ: &str = "item.read";
const WRITE: &str = "item.write";

/// The cached description of instance `id`: one read and one write.
fn bootstrap(id: &str) -> runtime::Bootstrap {
    let schema = json!({"type":"object","additionalProperties":false,"properties":{"name":{"type":"string"}}});
    let operation = |op: &str| connectors_core::Operation {
        id: op.into(),
        description: format!("{op} of a fixture item"),
        contract: "operations/v1alpha1".into(),
        profile: "resource".into(),
        input_schema: schema.clone(),
        output_schema: json!({"type":"object","additionalProperties":false}),
    };
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: id.into(),
        adapter: "catalog".into(),
        revision: "desc-1".into(),
        operations: vec![operation(READ), operation(WRITE)],
        configuration_schema: json!({"type":"object"}),
    };
    let requirement = |op: &str, effect| runtime::Requirement {
        operation: op.into(),
        custody: false,
        profile: "token".into(),
        scopes: BTreeSet::new(),
        effect,
    };
    runtime::Bootstrap {
        bindings: None,
        credential_revision: None,
        executable: None,
        instance: id.into(),
        adapter: "catalog".into(),
        protocol: "v1alpha1".into(),
        configuration_revision: "cfg-1".into(),
        provider_authority: "https://fixture.invalid".into(),
        descriptor: serde_json::to_string(&descriptor).unwrap(),
        profiles: vec![runtime::Profile {
            id: "token".into(),
            revision: "profile-1".into(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            scheme: "http_bearer".into(),
            capability: "http-bearer".into(),
            minimum_scopes: BTreeSet::new(),
            evidence_lifetime_ms: 60_000,
            fields: vec![runtime::EntryField {
                name: "token".into(),
                label: "Token".into(),
                max_bytes: 1024,
            }],
            acquisition: None,
        }],
        requirements: vec![
            requirement(READ, runtime::Effect::Read),
            requirement(WRITE, runtime::Effect::Write),
        ],
    }
}

/// The body of an entry table `table` of implementation `adapter_id`.
fn entry(table: &str, alias: Option<&str>, id: &str, adapter_id: &str, args: &str) -> String {
    let alias = alias.map(|a| format!("alias='{a}'\n")).unwrap_or_default();
    format!(
        "[{table}]\n{alias}instance_id='{id}'\nadapter_id='{adapter_id}'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[{table}.executable]\npath='{NOT_INSTALLED}/connectors-catalog-provider'\nsha256='{}'\nargs={args}\n[{table}.permissions]\nprofiles=['token']\noperations=['{READ}','{WRITE}']\n",
        "a".repeat(64)
    )
}

/// A legacy `[adapters.<alias>]` catalog entry with provider `args`.
fn legacy(alias: &str, id: &str, args: &str) -> String {
    entry(&format!("adapters.{alias}"), None, id, "catalog", args)
}

/// A `[connectors.<connector>.instances.<name>]` instance under `alias`.
fn instance(connector: &str, name: &str, alias: &str, id: &str) -> String {
    entry(
        &format!("connectors.{connector}.instances.{name}"),
        Some(alias),
        id,
        "catalog",
        "[]",
    )
}

/// An initialised root whose configuration is `connectors-local/5` holding
/// `body`, each alias of `cached` with its description cached.
fn root_with(body: &str, cached: &[&str]) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let init = run(root.path(), &["setup", "init"]);
    assert!(init.status.success(), "{init:?}");
    let config = root.path().join("config/config.toml");
    let initialized = fs::read_to_string(&config).unwrap();
    let owner = initialized
        .lines()
        .find(|line| line.starts_with("owner_uid"))
        .unwrap();
    fs::write(
        &config,
        format!("format = \"connectors-local/5\"\n{owner}\n{body}"),
    )
    .unwrap();
    #[cfg(unix)]
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    let paths = Paths {
        config: config.clone(),
        state: root.path().join("state"),
    };
    let loaded = Config::load(&paths.config).unwrap();
    for alias in cached {
        let adapter = &loaded.adapters[*alias];
        runtime::state::State::new(&paths.state)
            .remember(&adapter.selection(), &bootstrap(&adapter.instance_id))
            .unwrap();
    }
    root
}

/// `fixture/main` (alias `forge`), the legacy catalog entry `jira-old` whose
/// connector `setup migrate` derives as `jira`, and the legacy catalog entry
/// `legacy` whose connector cannot be derived.
fn with_legacy() -> tempfile::TempDir {
    let body = legacy("jira-old", "jira-old", "['--connector','jira']")
        + &legacy("legacy", "legacy-local", "[]")
        + "[connectors.fixture]\nenabled=true\n"
        + &instance("fixture", "main", "forge", "forge-local");
    root_with(&body, &["forge", "jira-old", "legacy"])
}

fn command(root: &Path, output: &str, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
    command
        .args(["--output", output, "--config"])
        .arg(root.join("config/config.toml"))
        .arg("--state-dir")
        .arg(root.join("state"))
        .args(args)
        .stdin(Stdio::null());
    command
}

fn run(root: &Path, args: &[&str]) -> Output {
    command(root, "json", args).output().unwrap()
}

/// The JSON envelope the command wrote: stdout on success, stderr otherwise.
#[track_caller]
fn envelope(output: &Output) -> Value {
    let stream = if output.status.success() {
        &output.stdout
    } else {
        &output.stderr
    };
    serde_json::from_slice(stream).unwrap_or_else(|_| panic!("one JSON envelope: {output:?}"))
}

#[track_caller]
fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["ok"], true, "{envelope}");
    envelope["result"].clone()
}

#[track_caller]
fn refusal(output: &Output) -> (Option<i32>, Value) {
    assert!(!output.status.success(), "{output:?}");
    let envelope: Value = serde_json::from_slice(&output.stderr)
        .unwrap_or_else(|_| panic!("one JSON envelope: {output:?}"));
    assert_eq!(envelope["ok"], false, "{envelope}");
    (output.status.code(), envelope["error"]["data"].clone())
}

/// Whether any object in `value` names `catalog` as its connector, or any
/// string is a `catalog/<instance>` target.
fn names_catalog(value: &Value) -> bool {
    match value {
        Value::Object(fields) => {
            fields.get("connector") == Some(&json!("catalog")) || fields.values().any(names_catalog)
        }
        Value::Array(items) => items.iter().any(names_catalog),
        Value::String(text) => text.starts_with("catalog/"),
        _ => false,
    }
}

/// The `(connector, instance)` of every item of an `instances list` result.
fn pairs(result: &Value) -> Vec<(String, String)> {
    result["adapters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            (
                item["connector"].as_str().unwrap().to_owned(),
                item["instance"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn unmigrated_refusal() -> Value {
    json!({"kind":"usage","code":"invalid_configuration","stage":"configuration",
        "next_action":"check_configuration","configuration_rule":"legacy_entry_unmigrated"})
}

// ---------------------------------------------------------------------------
// D9: no caller in the repository still runs `operations invoke`.
// ---------------------------------------------------------------------------

/// Every `.rs` file under `dir`, skipping build output and VCS state.
fn rust_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if !matches!(
                name.as_str(),
                "target" | ".git" | ".local" | "node_modules" | ".engineering"
            ) {
                rust_sources(&path, found);
            }
        } else if kind.is_file() && name.ends_with(".rs") {
            found.push(path);
        }
    }
}

/// story:connectors-do-short-invocation: `operations invoke` is removed and
/// "no caller in the repository still depends on it". A Rust source that
/// still passes `"operations", "invoke"` to the CLI is such a caller: its
/// command line is now the `cli_parse` refusal. The only sources allowed to
/// name the pair are the tests asserting that refusal.
#[test]
fn no_rust_source_still_runs_operations_invoke() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut sources = Vec::new();
    rust_sources(&root, &mut sources);
    assert!(sources.len() > 100, "the scan saw the repository: {root:?}");
    let needle = concat!("\"operations\"", ",", "\"invoke\"");
    let allowed = ["removed_forms.rs", "adversary_d218_u3.rs"];
    let mut callers: Vec<String> = sources
        .iter()
        .filter(|path| {
            !allowed
                .iter()
                .any(|name| path.file_name().is_some_and(|f| f == *name))
        })
        .filter(|path| {
            let text = fs::read_to_string(path).unwrap_or_default();
            let squeezed: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            squeezed.contains(needle)
        })
        .map(|path| {
            path.strip_prefix(&root)
                .unwrap_or(path)
                .display()
                .to_string()
        })
        .collect();
    callers.sort();
    assert!(
        callers.is_empty(),
        "story:connectors-do-short-invocation: these sources still run the removed `operations invoke`: {callers:?}"
    );
}

/// story:connectors-do-short-invocation: `help operations invoke` does not
/// describe the removed command, and the bash completion offers no
/// `operations invoke`.
#[test]
fn help_and_completion_offer_no_operations_invoke() {
    let root = tempfile::tempdir().unwrap();
    let help = command(root.path(), "human", &["help", "operations", "invoke"])
        .output()
        .unwrap();
    let text =
        String::from_utf8_lossy(&help.stdout).into_owned() + &String::from_utf8_lossy(&help.stderr);
    assert!(
        !help.status.success() && !text.contains("Invoke once under selected schema"),
        "story:connectors-do-short-invocation: `help operations invoke` answered: {help:?}"
    );
    let completion = command(root.path(), "human", &["completions", "bash"])
        .output()
        .unwrap();
    assert!(completion.status.success(), "{completion:?}");
    let script = String::from_utf8_lossy(&completion.stdout);
    assert!(
        script.contains("connectors__subcmd__invoke")
            && !script.contains("operations__subcmd__invoke"),
        "story:connectors-do-short-invocation: the completion still offers operations invoke"
    );
}

// ---------------------------------------------------------------------------
// D7: `instances list` and the names it hands out.
// ---------------------------------------------------------------------------

/// story:connectors-do-short-invocation: a `[connectors]` instance and a
/// legacy catalog entry that `setup migrate` derives the same connector and
/// instance for are one name. `invoke` already resolves the pair to the
/// `[connectors]` instance (`invoke.rs` `named`: "each name is listed once");
/// `instances list` must not list the pair twice, under two instance ids.
#[test]
fn a_name_derived_twice_is_listed_once() {
    let body = legacy("old", "old-local", "['--connector','jira']")
        + "[connectors.jira]\nenabled=true\n"
        + &instance("jira", "old", "jira-current", "jira-current");
    let root = root_with(&body, &["old", "jira-current"]);
    let result = success(&run(root.path(), &["instances", "list"]));
    let named = pairs(&result);
    let twice = named
        .iter()
        .filter(|(c, i)| c == "jira" && i == "old")
        .count();
    assert_eq!(
        twice, 1,
        "story:connectors-do-short-invocation: jira/old is listed {twice} times: {result}"
    );
    // The pair `invoke` selects is the `[connectors]` instance.
    let described = success(&run(
        root.path(),
        &["invoke", "jira/old", READ, "--describe"],
    ));
    assert_eq!(
        (&described["connector"], &described["instance"]),
        (&json!("jira"), &json!("old")),
        "{described}"
    );
}

/// story:connectors-do-short-invocation: the connector and instance
/// `instances list` names a legacy entry by are the names that select it.
/// `--adapter` is deprecated in favour of `--connector`/`--instance`, so an
/// operator reading `jira/jira-old` from the list selects it with those.
#[test]
fn the_names_instances_list_hands_out_select_the_instance() {
    let root = with_legacy();
    let listed = pairs(&success(&run(root.path(), &["instances", "list"])));
    assert!(
        listed.contains(&("jira".to_owned(), "jira-old".to_owned())),
        "{listed:?}"
    );
    // The control: a `[connectors]` instance by the names the list gives it.
    success(&run(
        root.path(),
        &[
            "instances",
            "describe",
            "--connector",
            "fixture",
            "--instance",
            "main",
        ],
    ));
    let refused: Vec<String> = [
        &[
            "instances",
            "describe",
            "--connector",
            "jira",
            "--instance",
            "jira-old",
        ][..],
        &[
            "operations",
            "list",
            "--connector",
            "jira",
            "--instance",
            "jira-old",
        ],
    ]
    .into_iter()
    .filter_map(|args| {
        let output = run(root.path(), args);
        (!output.status.success()).then(|| {
            format!(
                "`{}`: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim_end()
            )
        })
    })
    .collect();
    assert!(
        refused.is_empty(),
        "story:connectors-do-short-invocation: the instance `instances list` names jira/jira-old is refused by those names: {refused:#?}"
    );
}

/// story:connectors-do-short-invocation: no output names `catalog` as a
/// connector. A legacy entry whose connector is derived is `jira`, whether it
/// is selected by the catalog name or by the deprecated `--adapter`.
#[test]
fn a_derived_legacy_entry_is_never_answered_as_connector_catalog() {
    let root = with_legacy();
    let catalog: Vec<String> = [
        &[
            "instances",
            "describe",
            "--connector",
            "catalog",
            "--instance",
            "jira-old",
        ][..],
        &["instances", "describe", "--adapter", "jira-old"],
        &[
            "operations",
            "list",
            "--connector",
            "catalog",
            "--instance",
            "jira-old",
        ],
    ]
    .into_iter()
    .filter_map(|args| {
        let output = run(root.path(), args);
        let envelope = envelope(&output);
        let summary = envelope
            .pointer("/result/configuration/summary")
            .cloned()
            .unwrap_or(json!(null));
        names_catalog(&envelope).then(|| {
            format!(
                "`{}` (exit {:?}): summary {summary}",
                args.join(" "),
                output.status.code()
            )
        })
    })
    .collect();
    assert!(
        catalog.is_empty(),
        "story:connectors-do-short-invocation: these answers name catalog as a connector: {catalog:#?}"
    );
}

/// story:connectors-do-short-invocation: `invoke catalog <op>`, the catalog
/// name with no instance, is refused by `legacy_entry_unmigrated` while an
/// unmigrated entry exists, read or `--describe`.
#[test]
fn the_catalog_connector_alone_is_refused_by_the_unmigrated_rule() {
    let root = with_legacy();
    for args in [
        &["invoke", "catalog", READ, "--input-json", "{}"][..],
        &["invoke", "catalog", READ, "--describe"],
        &["invoke", "catalog", WRITE, "--input-json", "{}"],
    ] {
        let (code, data) = refusal(&run(root.path(), args));
        assert_eq!(
            (code, data),
            (Some(2), unmigrated_refusal()),
            "story:connectors-do-short-invocation: {args:?}"
        );
    }
}

/// story:connectors-do-short-invocation: a derived legacy entry selected by
/// the name `catalog` gave it is not found, and the refusal names no catalog.
#[test]
fn a_derived_entry_by_its_catalog_name_is_not_found() {
    let root = with_legacy();
    let output = run(
        root.path(),
        &["invoke", "catalog/jira-old", READ, "--input-json", "{}"],
    );
    let all = envelope(&output);
    let (code, data) = refusal(&output);
    assert_eq!(
        (code, &data["code"]),
        (Some(1), &json!("not_found")),
        "story:connectors-do-short-invocation: {data}"
    );
    assert!(!names_catalog(&all), "{all}");
}

/// story:connectors-do-short-invocation: an unmigrated entry whose alias is
/// itself `catalog` is listed in `unmigrated`, never as an item, and
/// `catalog` or `catalog/catalog` is refused by its rule.
#[test]
fn an_unmigrated_entry_aliased_catalog() {
    let body = legacy("catalog", "catalog-local", "[]")
        + "[connectors.fixture]\nenabled=true\n"
        + &instance("fixture", "main", "forge", "forge-local");
    let root = root_with(&body, &["forge", "catalog"]);
    let result = success(&run(root.path(), &["instances", "list"]));
    assert_eq!(
        pairs(&result),
        [("fixture".to_owned(), "main".to_owned())],
        "story:connectors-do-short-invocation: {result}"
    );
    assert_eq!(result["unmigrated"], json!(["catalog"]), "{result}");
    for target in ["catalog", "catalog/catalog"] {
        let (code, data) = refusal(&run(
            root.path(),
            &["invoke", target, READ, "--input-json", "{}"],
        ));
        assert_eq!(
            (code, data),
            (Some(2), unmigrated_refusal()),
            "story:connectors-do-short-invocation: {target}"
        );
    }
}

/// story:connectors-do-short-invocation: a legacy entry of another
/// implementation is listed under its implementation and alias, never in
/// `unmigrated`, and `unmigrated` is absent when no entry is.
#[test]
fn a_legacy_non_catalog_entry_is_listed_and_not_unmigrated() {
    let body = entry("adapters.graf", None, "graf-local", "grafana", "[]")
        + "[connectors.fixture]\nenabled=true\n"
        + &instance("fixture", "main", "forge", "forge-local");
    let root = root_with(&body, &["forge"]);
    let result = success(&run(root.path(), &["instances", "list"]));
    let mut named = pairs(&result);
    named.sort();
    assert_eq!(
        named,
        [
            ("fixture".to_owned(), "main".to_owned()),
            ("grafana".to_owned(), "graf".to_owned())
        ],
        "story:connectors-do-short-invocation: {result}"
    );
    assert!(
        result.get("unmigrated").is_none(),
        "story:connectors-do-short-invocation: {result}"
    );
}

/// story:connectors-do-short-invocation: paging `instances list` one item at
/// a time visits every instance exactly once, never an unmigrated entry, and
/// every page repeats the same sorted `unmigrated`; the last page has no
/// cursor.
#[test]
fn paging_visits_each_instance_once_and_repeats_unmigrated() {
    let body = legacy("zeta", "zeta-local", "[]")
        + &legacy("alpha", "alpha-local", "[]")
        + &legacy("jira-old", "jira-old", "['--connector','jira']")
        + "[connectors.fixture]\nenabled=true\n"
        + &instance("fixture", "main", "forge", "forge-local")
        + &instance("fixture", "backup", "forge-backup", "forge-backup");
    let root = root_with(&body, &[]);
    let mut seen = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..10 {
        let mut args = vec!["instances", "list", "--limit", "1"];
        if let Some(cursor) = &cursor {
            args.extend(["--cursor", cursor.as_str()]);
        }
        let page = success(&run(root.path(), &args));
        assert_eq!(
            page["unmigrated"],
            json!(["alpha", "zeta"]),
            "story:connectors-do-short-invocation: {page}"
        );
        assert!(page["adapters"].as_array().unwrap().len() <= 1, "{page}");
        seen.extend(pairs(&page));
        match page["next_cursor"].as_str() {
            Some(next) => cursor = Some(next.to_owned()),
            None => {
                cursor = None;
                break;
            }
        }
    }
    assert!(cursor.is_none(), "the pages end");
    let mut sorted = seen.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        [
            ("fixture".to_owned(), "backup".to_owned()),
            ("fixture".to_owned(), "main".to_owned()),
            ("jira".to_owned(), "jira-old".to_owned()),
        ],
        "story:connectors-do-short-invocation: {seen:?}"
    );
    let whole = success(&run(root.path(), &["instances", "list", "--limit", "3"]));
    assert!(whole["next_cursor"].is_null(), "{whole}");
    assert_eq!(pairs(&whole).len(), 3, "{whole}");
}

/// story:connectors-do-short-invocation: a configuration holding only
/// unmigrated entries lists no item, no cursor, and every alias in
/// `unmigrated`, under any `--limit`.
#[test]
fn only_unmigrated_entries_list_no_item() {
    let body = legacy("b-old", "b-old", "[]")
        + &legacy("a-old", "a-old", "[]")
        + "[connectors.fixture]\nenabled=true\n";
    let root = root_with(&body, &[]);
    for args in [
        &["instances", "list"][..],
        &["instances", "list", "--limit", "1"],
    ] {
        let result = success(&run(root.path(), args));
        assert_eq!(
            result,
            json!({"adapters":[],"unmigrated":["a-old","b-old"],"source":"configuration"}),
            "story:connectors-do-short-invocation: {args:?}"
        );
    }
}

/// story:connectors-do-short-invocation: the deprecated `adapters list`
/// answers what `instances list` answers, and the human output of either
/// names no `adapter` member and no `catalog` connector.
#[test]
fn adapters_list_and_human_output_follow_instances_list() {
    let root = with_legacy();
    let instances = success(&run(root.path(), &["instances", "list"]));
    let adapters = run(root.path(), &["adapters", "list"]);
    assert_eq!(
        success(&adapters),
        instances,
        "story:connectors-do-short-invocation"
    );
    for args in [&["instances", "list"][..], &["adapters", "list"]] {
        let output = command(root.path(), "human", args).output().unwrap();
        assert!(output.status.success(), "{output:?}");
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(
            !text.contains("\"adapter\"")
                && !text.contains("\"adapter_id\"")
                && !text.contains("\"connector\":\"catalog\"")
                && text.contains("jira-old")
                && text.contains("legacy"),
            "story:connectors-do-short-invocation: {args:?}: {text}"
        );
    }
}

/// story:connectors-do-short-invocation: `--adapter` keeps working, with its
/// deprecation, on a command that takes it, and is refused as unknown,
/// naming no replacement, by `invoke`.
#[test]
fn adapter_selection_is_deprecated_where_taken_and_unknown_on_invoke() {
    let root = with_legacy();
    let described = run(
        root.path(),
        &["instances", "describe", "--adapter", "forge"],
    );
    let answered: Value =
        serde_json::from_slice(&described.stdout).unwrap_or_else(|_| panic!("{described:?}"));
    assert_eq!(answered["ok"], true, "{answered}");
    assert_eq!(
        answered["deprecations"],
        json!([{"option":"--adapter","replacement":["--connector","--instance"]}]),
        "story:connectors-do-short-invocation: {answered}"
    );
    let invoked = run(
        root.path(),
        &[
            "invoke",
            "fixture/main",
            READ,
            "--adapter",
            "forge",
            "--input-json",
            "{}",
        ],
    );
    assert_eq!(invoked.status.code(), Some(2), "{invoked:?}");
    let refused = envelope(&invoked);
    assert!(
        refused.get("deprecations").is_none(),
        "story:connectors-do-short-invocation: {refused}"
    );
}

//! story:setup-migrate-adapter-entries, the acceptance: on a configuration
//! holding catalog, credential-binary, MCP, SQL and Kubernetes entries with
//! connections, `connectors setup migrate --apply` rewrites each catalog entry
//! that has a connector declaration as `[connectors.<c>.instances.<alias>]` and
//! leaves every other entry as `[adapters.<alias>]`, while `connections list`
//! shows the same connection ids and states and `approvals policy-status` the
//! same policy for every entry, with no credential entered.
//!
//! The production CLI, owner and an adapter child: this test binary, filtered
//! to [`FIXTURE`], answering as whichever entry started it. Two custody
//! bindings: the read-only `mounted_secret` binding in a jail
//! (`connectors_custody::mounted::fixture`), whose item names carry the alias,
//! in the ordinary lane; and a disposable Secret Service, which approval keys
//! and so approval policies need, run explicitly.
#![cfg(target_os = "linux")]
use connectors_custody::mounted::fixture::{Jail, Map};
use connectors_host::local::{registry, runtime};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

const FIXTURE: &str = "setup_migrate_adapter_fixture";
const PROFILE: &str = "fixture.token";
const READ: &str = "fixture.whoami";
const REVISION: &str = "desc-1";
const SECRET: &[u8] = b"fictional-migrate-token-0001";

const CONFIG: &str = "/srv/connectors/config.toml";
const STATE: &str = "/srv/connectors/state";
const CUSTODY: &str = "/run/connectors/custody";
const SOURCE: &str = "/srv/secret";
const STAMP: &str = "..2026_10_16_00_00_00.000000001";

fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// One fixture entry: alias, implementation, the native `provider` of a
/// catalog entry and its `request_prefix`, and the connector it migrates to.
type Fixed = (
    &'static str,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    Option<&'static str>,
);

const ENTRIES: &[Fixed] = &[
    ("mail", "catalog", Some("google-gmail"), None, Some("gmail")),
    ("tracker", "catalog", Some("jira"), None, Some("jira")),
    ("chat", "catalog", Some("slack"), None, None),
    (
        "tracker-gw",
        "catalog",
        Some("jira"),
        Some("/ex/jira/cloud-1"),
        None,
    ),
    ("search", "tavily", None, None, None),
    ("tools", "mcp", None, None, None),
    ("warehouse", "sql", None, None, None),
    ("cluster", "kubernetes", None, None, None),
];

/// The adapter child's description for the entry `alias` of implementation
/// `adapter`: one read under one static profile.
fn bootstrap(alias: &str, adapter: &str) -> runtime::Bootstrap {
    let instance = format!("{alias}-local");
    let descriptor = connectors_core::Descriptor {
        version: "v1alpha1".into(),
        instance: instance.clone(),
        adapter: adapter.into(),
        revision: REVISION.into(),
        operations: vec![connectors_core::Operation {
            id: READ.into(),
            description: "names the credential it was invoked with".into(),
            contract: "operations/v1alpha1".into(),
            profile: "resource".into(),
            input_schema: json!({"type":"object","additionalProperties":false,"properties":{}}),
            output_schema: json!({"type":"object","additionalProperties":false,
                "properties":{"credential":{"type":"string"}},"required":["credential"]}),
        }],
        configuration_schema: json!({"type":"object"}),
    };
    runtime::Bootstrap {
        instance,
        adapter: adapter.into(),
        protocol: "v1alpha1".into(),
        configuration_revision: format!("cfg-{alias}"),
        provider_authority: "https://fixture.invalid".into(),
        descriptor: serde_json::to_string(&descriptor).unwrap(),
        profiles: vec![runtime::Profile {
            id: PROFILE.into(),
            revision: "profile-1".into(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            scheme: "http_bearer".into(),
            capability: "http-bearer".into(),
            minimum_scopes: BTreeSet::new(),
            evidence_lifetime_ms: 300_000,
            fields: vec![runtime::EntryField {
                name: "token".into(),
                label: "Token".into(),
                max_bytes: 1024,
            }],
            acquisition: None,
        }],
        requirements: vec![runtime::Requirement {
            operation: READ.into(),
            profile: PROFILE.into(),
            scopes: BTreeSet::new(),
            effect: runtime::Effect::Read,
        }],
    }
}

/// The adapter child: the entry it serves is named by the words after `--`
/// in its arguments (alias, implementation, log directory). Each validation
/// appends the credential's digest to `<log>/validations`.
struct Fixture {
    alias: String,
    adapter: String,
    log: PathBuf,
}

#[async_trait::async_trait]
impl runtime::Adapter for Fixture {
    fn bootstrap(&self) -> runtime::Bootstrap {
        bootstrap(&self.alias, &self.adapter)
    }
    async fn validate(
        &self,
        _: &str,
        document: connectors_sdk::Secret,
    ) -> runtime::Result<runtime::Baseline> {
        let mut log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log.join("validations"))
            .unwrap();
        writeln!(log, "{}", digest(&document.0)).unwrap();
        let now = connectors_sdk::now_ms();
        Ok(runtime::Baseline {
            identity: registry::ExternalIdentity {
                kind: "fixture.user".into(),
                subject: "fixture-subject-1".into(),
            },
            granted_scopes: None,
            credential_expires_at_ms: None,
            collected_at_ms: now,
            valid_until_ms: now + 290_000,
        })
    }
    async fn invoke(
        &self,
        _: &str,
        _: &str,
        document: connectors_sdk::Secret,
        _: Value,
    ) -> runtime::Result<Value> {
        Ok(json!({"credential": digest(&document.0)}))
    }
}

/// The adapter child the owner starts: this binary, filtered to this test.
/// The ordinary run does nothing.
#[test]
fn setup_migrate_adapter_fixture() {
    let args: Vec<String> = std::env::args().collect();
    if !args.iter().any(|arg| arg == "--connectors-private-fd") {
        return;
    }
    let words = &args[args.iter().position(|arg| arg == "--").unwrap() + 1..];
    let _ = runtime::serve(
        3,
        Fixture {
            alias: words[0].clone(),
            adapter: words[1].clone(),
            log: PathBuf::from(&words[2]),
        },
    );
}

/// The `[adapters.<alias>]` entries of [`ENTRIES`], started as `executable`
/// with digest `sha256`, logging under `log`; a catalog entry's native
/// configuration is at `native/<alias>.json`.
fn entries(executable: &str, sha256: &str, log: &str, native: &Path) -> String {
    let mut text = String::new();
    for (alias, adapter, _, _, _) in ENTRIES {
        let mut args = vec![
            "--exact".to_owned(),
            FIXTURE.to_owned(),
            "--".to_owned(),
            (*alias).to_owned(),
            (*adapter).to_owned(),
            log.to_owned(),
        ];
        if *adapter == "catalog" {
            args.push("--local-config".into());
            args.push(
                native
                    .join(format!("{alias}.json"))
                    .to_str()
                    .unwrap()
                    .into(),
            );
        }
        text.push_str(&format!(
            "[adapters.{alias}]\ninstance_id='{alias}-local'\nadapter_id='{adapter}'\nconfiguration_revision='cfg-{alias}'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.{alias}.permissions]\nprofiles=['{PROFILE}']\noperations=['{READ}']\n[adapters.{alias}.executable]\npath='{executable}'\nsha256='{sha256}'\nargs={}\n",
            serde_json::to_string(&args).unwrap()
        ));
    }
    text
}

/// Each catalog entry's native configuration under `directory` (as the host
/// sees it), owner-only.
fn natives(directory: &Path) {
    fs::create_dir_all(directory).unwrap();
    fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
    for (alias, _, provider, prefix, _) in ENTRIES {
        let Some(provider) = provider else { continue };
        let mut native = json!({"provider": provider, "api_base": "https://fixture.invalid"});
        if let Some(prefix) = prefix {
            native["request_prefix"] = json!(prefix);
        }
        let path = directory.join(format!("{alias}.json"));
        fs::write(&path, native.to_string()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}

fn success(output: &Output) -> Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], true);
    value["result"].clone()
}

/// What a migration must leave alone, per entry: its connections (id, state,
/// instance id, configuration revision) and its approval policy.
fn records(run: &dyn Fn(&[&str]) -> Output) -> BTreeMap<String, (Value, Value)> {
    ENTRIES
        .iter()
        .map(|(alias, ..)| {
            let listed = success(&run(&["connections", "list", "--adapter", alias]));
            let connections: Vec<Value> = listed["connections"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| {
                    json!([
                        c["connection"],
                        c["state"],
                        c["instance_id"],
                        c["configuration_revision"]
                    ])
                })
                .collect();
            assert_eq!(connections.len(), 1, "{alias}: {listed}");
            let policy = success(&run(&["approvals", "policy-status", "--adapter", alias]));
            (
                (*alias).to_owned(),
                (json!(connections), policy["policy"].clone()),
            )
        })
        .collect()
}

/// The migration, checked against `records` before and after: the dry run
/// writes nothing, the application rewrites exactly the expected entries, and
/// every record is unchanged. Returns the applied result.
fn migrate_and_compare(run: &dyn Fn(&[&str]) -> Output, config: &Path) -> Value {
    let before = records(run);
    let original = fs::read_to_string(config).unwrap();
    let planned = success(&run(&["setup", "migrate", "--dry-run"]));
    assert_eq!(planned["disposition"], "planned", "{planned}");
    assert_eq!(fs::read_to_string(config).unwrap(), original);
    let applied = success(&run(&["setup", "migrate", "--apply"]));
    assert_eq!(applied["disposition"], "applied", "{applied}");
    assert_eq!(applied["entries"], planned["entries"]);
    let written = fs::read_to_string(config).unwrap();
    assert_eq!(written, applied["configuration"].as_str().unwrap());
    for (alias, _, _, _, connector) in ENTRIES {
        let pinned = connector.map(|c| format!("[connectors.{c}.instances.{alias}]"));
        let legacy = format!("[adapters.{alias}]");
        match &pinned {
            Some(table) => assert!(
                written.contains(table) && !written.contains(&legacy),
                "{alias}: {written}"
            ),
            None => assert!(written.contains(&legacy), "{alias}: {written}"),
        }
        let entry = applied["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["adapter"] == *alias)
            .unwrap();
        assert_eq!(entry["before"], entry["after"], "{entry}");
        let migrated = if connector.is_some() {
            "migrated"
        } else {
            "kept"
        };
        assert_eq!(entry["disposition"], migrated, "{entry}");
    }
    assert_eq!(records(run), before, "the same connections and policies");
    // The migrated instances answer to their new names with the same records.
    for (alias, _, _, _, connector) in ENTRIES {
        let Some(connector) = connector else { continue };
        let listed = success(&run(&[
            "connections",
            "list",
            "--connector",
            connector,
            "--instance",
            alias,
        ]));
        assert_eq!(
            listed["connections"][0]["connection"], before[*alias].0[0][0],
            "{alias}: {listed}"
        );
    }
    applied
}

/// A read through `connection` of `alias` answers with the stored
/// credential: nothing was re-entered or revalidated.
fn read(run: &dyn Fn(&[&str]) -> Output, connector: &str, alias: &str, connection: &str) {
    let schema = connectors_host::local::owner::schema(&bootstrap(alias, "catalog"), READ).unwrap();
    let read = success(&run(&[
        "operations",
        "invoke",
        "--connector",
        connector,
        "--instance",
        alias,
        "--connection",
        connection,
        "--operation",
        READ,
        "--schema",
        &schema,
        "--revision",
        REVISION,
        "--input-json",
        "{}",
    ]));
    assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");
}

/// The acceptance under the read-only `mounted_secret` binding, in a jail:
/// every entry connects from its mounted item `<alias>/<profile>`, and after
/// the migration the same connections are listed, read with the same item
/// and never validated again.
#[test]
fn migrated_entries_keep_their_connections_under_mounted_custody() {
    let mut jail = Jail::new();
    jail.directory("/srv", 0o755);
    jail.directory("/srv/connectors", 0o700);
    jail.directory(SOURCE, 0o755);
    jail.directory(CUSTODY, 0o755);
    jail.directory("/work", 0o700);
    jail.read_only(SOURCE, CUSTODY);
    jail.install(
        Path::new(env!("CARGO_BIN_EXE_connectors")),
        "/bin-test/connectors",
    );
    let adapter = jail.install(&std::env::current_exe().unwrap(), "/bin-test/adapter");
    let sha256 = digest(&fs::read(adapter).unwrap());
    natives(&jail.path("/srv/native"));
    // A kubelet atomic writer's layout: every alias's item under one stamp.
    let source = jail.path(SOURCE);
    for (alias, ..) in ENTRIES {
        let item = source.join(STAMP).join(alias).join(PROFILE);
        fs::create_dir_all(item.parent().unwrap()).unwrap();
        fs::write(&item, SECRET).unwrap();
        fs::set_permissions(&item, fs::Permissions::from_mode(0o400)).unwrap();
        symlink(Path::new("..data").join(alias), source.join(alias)).unwrap();
    }
    symlink(STAMP, source.join("..data")).unwrap();
    let uid = connectors_host::local::filesystem::uid();
    let config = jail.path(CONFIG);
    fs::write(
        &config,
        format!(
            "format='connectors-local/4'\nowner_uid={uid}\n[custody]\nbinding='mounted_secret'\ndirectory='{CUSTODY}'\n{}",
            entries("/bin-test/adapter", &sha256, "/work", Path::new("/srv/native"))
        ),
    )
    .unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    let run = |args: &[&str]| {
        Jail::output(
            jail.command("/bin-test/connectors", Map::Own)
                .args(["--output", "json", "--config", CONFIG, "--state-dir", STATE])
                .args(args)
                .env_clear()
                .stdin(Stdio::null()),
        )
    };
    success(&run(&["setup", "init"]));
    let mut connections = BTreeMap::new();
    for (alias, ..) in ENTRIES {
        let connected = success(&run(&[
            "connections",
            "connect",
            "--adapter",
            alias,
            "--profile",
            PROFILE,
        ]));
        assert_eq!(connected["connection"]["summary"]["state"], "ready");
        connections.insert(
            *alias,
            connected["connection"]["summary"]["connection"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
    }
    let validations = || fs::read_to_string(jail.path("/work/validations")).unwrap();
    let validated = validations();
    assert_eq!(validated.lines().count(), ENTRIES.len());

    let applied = migrate_and_compare(&run, &config);
    for entry in applied["entries"].as_array().unwrap() {
        let alias = entry["adapter"].as_str().unwrap();
        assert_eq!(
            entry["before"]["custody_items"],
            json!([format!("{alias}/{PROFILE}")]),
            "{entry}"
        );
    }
    read(&run, "gmail", "mail", &connections["mail"]);
    read(&run, "jira", "tracker", &connections["tracker"]);
    assert_eq!(validations(), validated, "nothing was validated again");
    let stopped = run(&["setup", "owner-stop"]);
    assert!(stopped.status.success(), "{stopped:?}");
}

/// The acceptance with approval policies, under a disposable Secret Service:
/// every entry is connected from a credential file once, given an approval
/// signing key and an approval policy, and after the migration lists the same
/// connections and the same policy (id, revision and selection, whose
/// `executable_selection` is the one the migration reported before and
/// after), with no credential entered.
#[test]
#[ignore = "requires qualified disposable GNOME Secret Service; run explicitly"]
fn migrated_entries_keep_their_connections_and_approval_policies() {
    let keyring = connectors_custody::secret_service::fixture::Fixture::new();
    let root = tempfile::tempdir().unwrap();
    let path = root.path();
    let executable = std::env::current_exe().unwrap();
    let sha256 = digest(&fs::read(&executable).unwrap());
    natives(&path.join("native"));
    fs::create_dir_all(path.join("log")).unwrap();
    let config = path.join("config/config.toml");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_connectors"))
            .args(["--output", "json", "--config"])
            .arg(&config)
            .arg("--state-dir")
            .arg(path.join("state"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    success(&run(&["setup", "init"]));
    let uid = connectors_host::local::filesystem::uid();
    let clock_key = |seed: u8| {
        use base64::Engine;
        use ring::signature::KeyPair;
        let pair = ring::signature::Ed25519KeyPair::from_seed_unchecked(&[seed; 32]).unwrap();
        base64::engine::general_purpose::STANDARD.encode(pair.public_key().as_ref())
    };
    let clock = format!(
        "[approval_clock]\nformat='roughtime-quorum/1'\nquorum=2\nmax_rate_error_ppm=10000\n[[approval_clock.sources]]\naddress='127.0.0.1:9'\npublic_key='{}'\nversion='roughtime-0x8000000c'\n[[approval_clock.sources]]\naddress='127.0.0.1:10'\npublic_key='{}'\nversion='roughtime-0x8000000c'\n",
        clock_key(7),
        clock_key(8)
    );
    fs::write(
        &config,
        format!(
            "format='connectors-local/4'\nowner_uid={uid}\nsecret_service_socket='{}'\n{clock}{}",
            keyring.socket.display(),
            entries(
                executable.to_str().unwrap(),
                &sha256,
                path.join("log").to_str().unwrap(),
                &path.join("native"),
            )
        ),
    )
    .unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    let credential = path.join("credential");
    fs::write(&credential, SECRET).unwrap();
    fs::set_permissions(&credential, fs::Permissions::from_mode(0o600)).unwrap();
    let policy = path.join("policy.json");
    fs::write(&policy, r#"{"operations":[]}"#).unwrap();
    fs::set_permissions(&policy, fs::Permissions::from_mode(0o600)).unwrap();
    let mut connections = BTreeMap::new();
    for (alias, ..) in ENTRIES {
        let connected = success(&run(&[
            "connections",
            "connect",
            "--adapter",
            alias,
            "--profile",
            PROFILE,
            "--credential-file",
            credential.to_str().unwrap(),
        ]));
        connections.insert(
            *alias,
            connected["connection"]["summary"]["connection"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
        success(&run(&["approvals", "key-init", "--adapter", alias]));
        let set = success(&run(&[
            "approvals",
            "policy-set",
            "--adapter",
            alias,
            "--input-file",
            policy.to_str().unwrap(),
        ]));
        assert_eq!(set["policy"]["revision"], 1, "{set}");
    }
    let validated = fs::read_to_string(path.join("log/validations")).unwrap();
    let applied = migrate_and_compare(&run, &config);
    for entry in applied["entries"].as_array().unwrap() {
        let alias = entry["adapter"].as_str().unwrap();
        let policy = success(&run(&["approvals", "policy-status", "--adapter", alias]));
        assert_eq!(
            policy["policy"]["selection"]["executable_selection"],
            entry["after"]["executable_selection"],
            "{alias}: {policy}"
        );
        assert_eq!(
            policy["policy"]["selection"]["configuration_revision"],
            entry["after"]["configuration_revision"],
            "{alias}: {policy}"
        );
    }
    read(&run, "gmail", "mail", &connections["mail"]);
    assert_eq!(
        fs::read_to_string(path.join("log/validations")).unwrap(),
        validated,
        "nothing was validated again"
    );
    let stopped = run(&["setup", "owner-stop"]);
    assert!(stopped.status.success(), "{stopped:?}");
}

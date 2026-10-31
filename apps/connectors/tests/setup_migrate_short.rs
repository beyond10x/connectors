//! story:migrate-writes-the-short-instance-form: `connectors setup migrate`
//! writes a legacy catalog entry as the short-form instance (its alias, its
//! old `instance_id`, and the endpoint where it differs from the default) when
//! the installed catalog provider derives the entry from it with the same
//! instance id, configuration revision, credential revision, permissions and
//! startup; otherwise it writes the pinned instance and names why. A live
//! connection of a migrated entry stays live under its old id and revision.
//!
//! The production CLI is copied beside a stand-in catalog provider
//! (`support/standin.rs`), which describes a native configuration or a derived
//! GitLab instance from the digest input the catalog provider computes its
//! revision from. This binary is that stand-in, so it runs without the
//! standard test harness: the host starts the installed provider with
//! provider arguments, which the standard harness refuses.
#[cfg(target_os = "linux")]
#[path = "support/standin.rs"]
mod standin;

#[cfg(target_os = "linux")]
fn main() {
    tests::main();
}

#[cfg(not(target_os = "linux"))]
fn main() {}

#[cfg(target_os = "linux")]
mod tests {
    use super::standin::{self, Installation, parsed, success};
    use connectors_custody::mounted::fixture::{Jail, Map};
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    use std::{
        collections::BTreeMap,
        fs,
        os::unix::fs::{PermissionsExt, symlink},
        path::Path,
        process::Stdio,
    };

    const API_BASE: &str = "https://gitlab.example.test/api/v4";
    const ENDPOINT: &str = "https://gitlab.example.test";
    const DEFAULT_API_BASE: &str = "https://gitlab.com/api/v4";
    const PROFILE: &str = "gitlab.pat";
    const READ: &str = "gitlab.whoami";

    pub fn main() {
        standin::main(&[
            (
                "an_equal_entry_becomes_the_short_form_and_a_differing_one_stays_pinned_naming_the_field",
                an_equal_entry_becomes_the_short_form_and_a_differing_one_stays_pinned_naming_the_field,
            ),
            (
                "a_live_connection_stays_live_after_its_entry_becomes_the_short_form",
                a_live_connection_stays_live_after_its_entry_becomes_the_short_form,
            ),
        ]);
    }

    fn digest(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    fn private(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }

    /// The legacy entry `alias` of the instance `id`, started as `provider`
    /// (digest `sha256`) with the native configuration at `native` and
    /// pinned at `revision`.
    fn legacy(
        alias: &str,
        id: &str,
        revision: &str,
        provider: &str,
        sha256: &str,
        native: &str,
    ) -> String {
        format!(
            "[adapters.{alias}]\ninstance_id='{id}'\nadapter_id='catalog'\nconfiguration_revision='{revision}'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n[adapters.{alias}.permissions]\nprofiles=['{PROFILE}']\noperations=['{READ}']\n[adapters.{alias}.executable]\npath='{provider}'\nsha256='{sha256}'\nargs=['--local-config','{native}']\n"
        )
    }

    /// The listed entries by alias.
    fn entries(result: &Value) -> BTreeMap<String, Value> {
        result["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| (entry["adapter"].as_str().unwrap().to_owned(), entry.clone()))
            .collect()
    }

    /// Four legacy GitLab entries: `work`, whose native configuration is the
    /// one a derived instance of its id at its endpoint has; `scoped`, whose
    /// profile requires other scopes; `onprem`, whose API base is no endpoint
    /// followed by GitLab's API path; and `stale`, pinned at a revision its
    /// configuration no longer has. Only `work` is written in the short form;
    /// each other one is pinned, naming the first differing key of the
    /// revision digest input, or `configuration_revision` when none differs.
    /// The dry run reports the revision before and after and writes nothing;
    /// the application writes exactly the printed file, which reads back.
    pub fn an_equal_entry_becomes_the_short_form_and_a_differing_one_stays_pinned_naming_the_field()
    {
        let installation = Installation::new();
        let root = installation.root.path();
        let natives = root.join("native");
        fs::create_dir(&natives).unwrap();
        fs::set_permissions(&natives, fs::Permissions::from_mode(0o700)).unwrap();
        let provider = root.join("bin/connectors-catalog-provider");
        let sha256 = digest(&fs::read(&provider).unwrap());
        let cases = [
            (
                "work",
                standin::native("gitlab-work-local", API_BASE, &["api"]),
                None,
            ),
            (
                "scoped",
                standin::native("gitlab-scoped-local", API_BASE, &["read_api"]),
                None,
            ),
            (
                "onprem",
                standin::native(
                    "gitlab-onprem-local",
                    "https://gitlab.example.test/v4",
                    &["api"],
                ),
                None,
            ),
            (
                "stale",
                standin::native("gitlab-stale-local", API_BASE, &["api"]),
                Some("cfg-stale"),
            ),
        ];
        let mut body = String::new();
        for (alias, native, pinned_at) in &cases {
            let path = natives.join(format!("{alias}.json"));
            private(&path, native.to_string().as_bytes());
            let revision =
                pinned_at.map_or_else(|| standin::native_revision(native), str::to_owned);
            body.push_str(&legacy(
                alias,
                native["instance"].as_str().unwrap(),
                &revision,
                provider.to_str().unwrap(),
                &sha256,
                path.to_str().unwrap(),
            ));
        }
        let original = installation.configure(&body);

        let planned = success(&installation.run(&["setup", "migrate", "--dry-run"]));
        assert_eq!(planned["disposition"], "planned", "{planned}");
        assert_eq!(installation.text(), original, "a dry run writes nothing");
        let listed = entries(&planned);
        let work = &listed["work"];
        assert_eq!(work["disposition"], "migrated", "{work}");
        assert_eq!(work["form"], "short", "{work}");
        assert!(work.get("pinned_field").is_none(), "{work}");
        assert_eq!(
            (&work["connector"], &work["instance"]),
            (&json!("gitlab"), &json!("work"))
        );
        assert_eq!(work["before"]["instance_id"], "gitlab-work-local", "{work}");
        assert_eq!(work["after"]["instance_id"], "gitlab-work-local", "{work}");
        assert_eq!(
            work["before"]["configuration_revision"], work["after"]["configuration_revision"],
            "{work}"
        );
        assert_eq!(
            work["before"]["custody_items"], work["after"]["custody_items"],
            "{work}"
        );
        // The short form selects the derived argument vector.
        assert_ne!(
            work["before"]["executable_selection"], work["after"]["executable_selection"],
            "{work}"
        );
        for (alias, field) in [
            ("scoped", "auth"),
            ("onprem", "api_base"),
            ("stale", "configuration_revision"),
        ] {
            let entry = &listed[alias];
            assert_eq!(entry["disposition"], "migrated", "{entry}");
            assert_eq!(entry["form"], "pinned", "{entry}");
            assert_eq!(entry["pinned_field"], field, "{entry}");
            assert_eq!(entry["before"], entry["after"], "{entry}");
        }
        let configuration = planned["configuration"].as_str().unwrap();
        let written = parsed(configuration);
        let instances = &written["connectors"]["gitlab"]["instances"];
        assert_eq!(
            instances["work"],
            toml::Value::Table(toml::map::Map::from_iter([
                ("alias".to_owned(), "work".into()),
                ("instance_id".to_owned(), "gitlab-work-local".into()),
                ("endpoint".to_owned(), ENDPOINT.into()),
            ])),
            "{configuration}"
        );
        for alias in ["scoped", "onprem", "stale"] {
            assert_eq!(instances[alias]["alias"], alias.into(), "{configuration}");
            assert!(
                instances[alias].get("adapter_id").is_some(),
                "{configuration}"
            );
        }
        assert!(
            written
                .get("adapters")
                .is_none_or(|adapters| adapters.as_table().unwrap().is_empty())
        );

        let applied = success(&installation.run(&["setup", "migrate", "--apply"]));
        assert_eq!(applied["disposition"], "applied", "{applied}");
        assert_eq!(applied["entries"], planned["entries"]);
        assert_eq!(installation.text(), configuration);
        let described = success(&installation.run(&[
            "instances",
            "describe",
            "--connector",
            "gitlab",
            "--instance",
            "work",
        ]));
        assert_eq!(
            described["configuration"]["executable"]["args"],
            json!([
                "--connector",
                "gitlab",
                "--instance",
                "work",
                "--instance-id",
                "gitlab-work-local",
                "--endpoint",
                ENDPOINT
            ]),
            "{described}"
        );
        let again = success(&installation.run(&["setup", "migrate", "--dry-run"]));
        assert_eq!(again["disposition"], "unchanged", "{again}");
    }

    const CONFIG: &str = "/srv/connectors/config.toml";
    const STATE: &str = "/srv/connectors/state";
    const CUSTODY: &str = "/run/connectors/custody";
    const SOURCE: &str = "/srv/secret";
    const STAMP: &str = "..2026_10_31_00_00_00.000000001";
    const SECRET: &[u8] = b"fictional-short-form-token-0001";
    const PROVIDER: &str = "/bin-test/connectors-catalog-provider";

    /// Under the read-only `mounted_secret` binding, in a jail: the legacy
    /// entry `work` connects from its mounted item, is migrated to the short
    /// form, and afterwards lists the same connection (id, state, instance id
    /// and configuration revision) under its old alias and under
    /// `--connector gitlab --instance work`, has the same approval policy
    /// status, and reads with the stored credential: nothing is entered again.
    /// The entry uses the declaration's default endpoint, so the short form
    /// states none; `connect --endpoint` naming another host then derives the
    /// instance's stated id under its alias, finds the live connection
    /// admitted under the default host and is refused without writing.
    /// The installed provider is named by `CONNECTORS_INSTALLATION`.
    pub fn a_live_connection_stays_live_after_its_entry_becomes_the_short_form() {
        let mut jail = Jail::new();
        jail.directory("/srv", 0o755);
        jail.directory("/srv/connectors", 0o700);
        jail.directory("/srv/native", 0o700);
        jail.directory(SOURCE, 0o755);
        jail.directory(CUSTODY, 0o755);
        jail.read_only(SOURCE, CUSTODY);
        jail.install(
            Path::new(env!("CARGO_BIN_EXE_connectors")),
            "/bin-test/connectors",
        );
        let provider = jail.install(&std::env::current_exe().unwrap(), PROVIDER);
        let sha256 = digest(&fs::read(provider).unwrap());
        let native = standin::native("gitlab-work-local", DEFAULT_API_BASE, &["api"]);
        private(
            &jail.path("/srv/native/work.json"),
            native.to_string().as_bytes(),
        );
        let source = jail.path(SOURCE);
        let item = source.join(STAMP).join("work").join(PROFILE);
        fs::create_dir_all(item.parent().unwrap()).unwrap();
        fs::write(&item, SECRET).unwrap();
        fs::set_permissions(&item, fs::Permissions::from_mode(0o400)).unwrap();
        symlink(Path::new("..data").join("work"), source.join("work")).unwrap();
        symlink(STAMP, source.join("..data")).unwrap();
        let config = jail.path(CONFIG);
        fs::write(
            &config,
            format!(
                "format='connectors-local/4'\nowner_uid={}\n[custody]\nbinding='mounted_secret'\ndirectory='{CUSTODY}'\n{}",
                connectors_host::local::filesystem::uid(),
                legacy(
                    "work",
                    "gitlab-work-local",
                    &standin::native_revision(&native),
                    PROVIDER,
                    &sha256,
                    "/srv/native/work.json"
                )
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
                    .env("CONNECTORS_INSTALLATION", "/bin-test")
                    .stdin(Stdio::null()),
            )
        };
        let ok = |args: &[&str]| {
            let output = run(args);
            assert!(
                output.status.success(),
                "{args:?}\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            serde_json::from_slice::<Value>(&output.stdout).unwrap()["result"].clone()
        };
        ok(&["setup", "init"]);
        let connected = ok(&[
            "connections",
            "connect",
            "--adapter",
            "work",
            "--profile",
            PROFILE,
        ]);
        assert_eq!(
            connected["connection"]["summary"]["state"], "ready",
            "{connected}"
        );
        let connection = connected["connection"]["summary"]["connection"]
            .as_str()
            .unwrap()
            .to_owned();
        let records = |selector: &[&str]| {
            let listed = ok(&[&["connections", "list"][..], selector].concat());
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
            connections
        };
        let before = records(&["--adapter", "work"]);
        assert_eq!(before.len(), 1, "{before:?}");
        assert_eq!(before[0][0], connection.as_str());
        assert_eq!(before[0][2], "gitlab-work-local");
        let policy = ok(&["approvals", "policy-status", "--adapter", "work"]);
        let original = fs::read_to_string(&config).unwrap();

        let planned = ok(&["setup", "migrate", "--dry-run"]);
        assert_eq!(fs::read_to_string(&config).unwrap(), original);
        let entry = &planned["entries"][0];
        assert_eq!(entry["form"], "short", "{planned}");
        let applied = ok(&["setup", "migrate", "--apply"]);
        assert_eq!(applied["entries"], planned["entries"]);
        let written = fs::read_to_string(&config).unwrap();
        assert!(
            written.contains("[connectors.gitlab.instances.work]")
                && !written.contains("[adapters.work]")
                && !written.contains("adapter_id"),
            "{written}"
        );

        assert_eq!(
            records(&["--adapter", "work"]),
            before,
            "the same connection"
        );
        assert_eq!(
            records(&["--connector", "gitlab", "--instance", "work"]),
            before,
            "the same connection under the instance's names"
        );
        assert_eq!(
            ok(&["approvals", "policy-status", "--adapter", "work"]),
            policy
        );
        let args: Vec<String> = [
            "--connector",
            "gitlab",
            "--instance",
            "work",
            "--instance-id",
            "gitlab-work-local",
        ]
        .map(str::to_owned)
        .to_vec();
        let (schema, revision) = standin::read_schema(&args);
        let read = ok(&[
            "invoke",
            "gitlab/work",
            READ,
            "--connection",
            &connection,
            "--schema",
            &schema,
            "--revision",
            &revision,
            "--input-json",
            "{}",
        ]);
        assert_eq!(read["result"]["credential"], digest(SECRET), "{read}");

        // The short form states no endpoint, so another host reaches the
        // check for a live connection, under the alias and stated id.
        assert!(!written.contains("endpoint"), "{written}");
        let (code, refused) = standin::refusal(&run(&[
            "connections",
            "connect",
            "gitlab/work",
            "--profile",
            PROFILE,
            "--endpoint",
            ENDPOINT,
        ]));
        assert_eq!(code, Some(1), "{refused}");
        assert_eq!(
            (&refused["code"], &refused["stage"], &refused["field"]),
            (
                &json!("lifecycle_conflict"),
                &json!("configuration"),
                &json!("endpoint")
            ),
            "{refused}"
        );
        assert_eq!(fs::read_to_string(&config).unwrap(), written);
        assert_eq!(records(&["--adapter", "work"]), before);
        let stopped = run(&["setup", "owner-stop"]);
        assert!(stopped.status.success(), "{stopped:?}");
    }
}

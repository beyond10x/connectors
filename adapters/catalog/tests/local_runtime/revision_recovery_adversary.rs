//! Adversary cases for story:revision-change-keeps-a-recovery-path.
//!
//! Belongs at `adapters/catalog/tests/local_runtime/revision_recovery_adversary.rs`
//! with `cli_journey_mod.patch` (one `mod` line in `cli_journey.rs`). Run alone:
//!
//! ```console
//! cargo build --locked -p connectors -p connectors-catalog-provider --bins
//! CONNECTORS_TEST_CLI="$PWD/target/debug/connectors" \
//!   cargo test --locked -p connectors-catalog-provider --test local_runtime -- \
//!   --ignored --test-threads 1 --nocapture cli_journey::revision_recovery_adversary
//! ```
//!
//! The same production CLI, owner, catalog child, TLS fixture and disposable
//! Secret Service as `revision_recovery.rs`; the harness is repeated here so
//! that this file does not depend on that file's private `Scenario`.
use super::*;

struct Connected {
    cli: Cli,
    _custody: Custody,
    provider: Provider,
    credential: PathBuf,
    reference: String,
    revision: String,
}

fn connected() -> Connected {
    let provider = Provider::new();
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    let summary = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "gitlab",
        "--profile",
        "gitlab.pat",
        "--credential-file",
        credential.to_str().unwrap(),
    ]))["connection"]["summary"]
        .clone();
    assert_eq!(summary["state"], "ready");
    Connected {
        reference: summary["connection"].as_str().unwrap().to_owned(),
        revision: summary["revision"].as_str().unwrap().to_owned(),
        cli,
        _custody: custody,
        provider,
        credential,
    }
}

impl Connected {
    /// Edits the native configuration, then copies its new bootstrap revision
    /// and instance into the adapter entry, as the upgrade guide says; `entry`
    /// edits the rest of the adapter entry's text.
    fn reconfigure(&self, native_edit: impl FnOnce(&mut Value), entry: impl Fn(&str) -> String) {
        let mut native: Value =
            serde_json::from_slice(&fs::read(&self.provider.config).unwrap()).unwrap();
        native_edit(&mut native);
        private(&self.provider.config, &serde_json::to_vec(&native).unwrap());
        let revision = self.provider.selection().configuration_revision;
        let instance = native["instance"].as_str().unwrap().to_owned();
        let q = |value: &str| serde_json::to_string(value).unwrap();
        let text: String = fs::read_to_string(&self.cli.paths.config)
            .unwrap()
            .lines()
            .map(|line| {
                if line.starts_with("configuration_revision=") {
                    format!("configuration_revision={}\n", q(&revision))
                } else if line.starts_with("instance_id=") {
                    format!("instance_id={}\n", q(&instance))
                } else {
                    format!("{}\n", entry(line))
                }
            })
            .collect();
        private(&self.cli.paths.config, text.as_bytes());
        connectors_host::local::config::Config::load(&self.cli.paths.config).unwrap();
    }

    fn revalidate(&self) -> Output {
        self.cli.run(&[
            "connections",
            "revalidate",
            "--adapter",
            "gitlab",
            "--connection",
            &self.reference,
            "--expected-revision",
            &self.revision,
        ])
    }

    fn connect(&self, profile: &str) -> Output {
        self.cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "gitlab",
            "--profile",
            profile,
            "--credential-file",
            self.credential.to_str().unwrap(),
        ])
    }
}

/// The story's Change: an identity change "is refused with one next action
/// that works (`connections connect` creating a new connection, with the old
/// one revocable)". The documented path for a changed profile authentication
/// (docs/local-catalog-provider.md, "Give the instance a new id in the native
/// configuration and the adapter entry, copy the new revision, and connect")
/// is followed literally, then the old connection is revoked. `revoke` filters
/// on the configured instance id (registry/lifecycle.rs `revoke`), so the old
/// connection answers `not_found` and its credential stays in custody with no
/// command that removes it.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn adversary_the_connection_left_by_a_changed_authentication_stays_revocable() {
    let scenario = connected();
    scenario.reconfigure(
        |native| native["auth"]["label"] = json!("GitLab token"),
        str::to_owned,
    );
    let refused = refusal(scenario.revalidate(), "lifecycle_conflict");
    assert_eq!(refused["next_action"], "check_configuration", "{refused}");

    scenario.reconfigure(
        |native| native["instance"] = json!("fixture-gitlab-2"),
        str::to_owned,
    );
    let fresh = success(scenario.connect("gitlab.pat"))["connection"]["summary"].clone();
    assert_eq!(fresh["state"], "ready", "{fresh}");

    let revoke = scenario.cli.run(&[
        "connections",
        "revoke",
        "--adapter",
        "gitlab",
        "--connection",
        &scenario.reference,
        "--expected-revision",
        &scenario.revision,
    ]);
    assert!(
        revoke.status.success(),
        "the old connection is not revocable on the documented path: {}",
        String::from_utf8_lossy(&revoke.stderr)
    );
    assert_eq!(success(revoke)["local_state"], "revoked");
}

/// The story names the profile among the parts of a connection's identity.
/// Changing `auth.profile` (here `gitlab.pat` to `gitlab.token`, both granted
/// in the entry) leaves a working path, `connections connect --profile
/// gitlab.token` under the same instance id, but revalidate of the saved
/// connection fails in `Bootstrap::binding` (`runtime::Failure::Unsupported`)
/// and the CLI answers `unsupported` with `next_action = none`: no command is
/// named, so a caller following next actions is stranded.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn adversary_a_changed_profile_id_names_the_path_that_recovers() {
    let scenario = connected();
    scenario.reconfigure(
        |native| native["auth"]["profile"] = json!("gitlab.token"),
        |line| {
            if line.starts_with("profiles=") {
                "profiles=[\"gitlab.pat\",\"gitlab.token\"]".to_owned()
            } else {
                line.to_owned()
            }
        },
    );
    let output = scenario.revalidate();
    assert!(!output.status.success());
    let refused: Value = serde_json::from_slice(&output.stderr)
        .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stderr)));
    let action = refused["error"]["data"]["next_action"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        matches!(action.as_str(), "create_connection" | "check_configuration"),
        "a changed profile id names no recovery: {refused}"
    );
    // The path that exists: a new connection under the new profile id.
    let fresh = success(scenario.connect("gitlab.token"))["connection"]["summary"].clone();
    assert_eq!(fresh["state"], "ready", "{fresh}");
}

/// The same lifetime-only change, applied and reverted, with a new connection
/// in between: the original connection is admitted again (its binding equals
/// the configured one), the interim one names a new connection, and a new
/// connection under the reverted revision is admitted. Expected green; it
/// guards the back-and-forth case nothing else in the unit drives.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn adversary_a_lifetime_changed_and_reverted_leaves_the_first_connection_usable() {
    let scenario = connected();
    scenario.reconfigure(
        |native| native["auth"]["evidence_lifetime_ms"] = json!(300_000),
        str::to_owned,
    );
    let interim = success(scenario.connect("gitlab.pat"))["connection"]["summary"].clone();
    assert_eq!(interim["state"], "ready", "{interim}");
    scenario.reconfigure(
        |native| {
            native["auth"]
                .as_object_mut()
                .unwrap()
                .remove("evidence_lifetime_ms");
        },
        str::to_owned,
    );
    let revalidated = success(scenario.revalidate())["connection"]["summary"].clone();
    assert_eq!(revalidated["state"], "ready", "{revalidated}");
    let interim_refused = refusal(
        scenario.cli.run(&[
            "connections",
            "revalidate",
            "--adapter",
            "gitlab",
            "--connection",
            interim["connection"].as_str().unwrap(),
            "--expected-revision",
            interim["revision"].as_str().unwrap(),
        ]),
        "lifecycle_conflict",
    );
    assert_eq!(interim_refused["next_action"], "create_connection");
    let third = success(scenario.connect("gitlab.pat"))["connection"]["summary"].clone();
    assert_eq!(third["state"], "ready", "{third}");
}

//! story:revision-change-keeps-a-recovery-path through the production CLI,
//! owner and catalog provider child: a connected instance whose native
//! configuration then changes, its new bootstrap revision copied into the
//! adapter entry as the upgrade guide says. A change that leaves the
//! connection's identity unchanged, the evidence lifetime among them, is
//! followed by `connections revalidate` or a new connection under the same
//! instance id, also while the adapter process of the earlier entry still runs;
//! a change to it names a new instance id; and no
//! next action answered on the way leads back to a refusal already answered.
use super::*;

/// The commands that recover a connection after a revision change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Recovery {
    Revalidate,
    Repair,
    Connect,
}

/// The native configuration change applied after the connection was saved,
/// and whether the owner, and so the adapter process, restarts after it.
#[derive(Clone, Copy, Debug)]
enum Change {
    /// `auth.evidence_lifetime_ms` from the default 60 000 to 300 000.
    Lifetime { restart: bool },
    /// The token entry's label: another profile declaration, so another
    /// authentication of the connection's profile.
    Authentication,
    /// Only the adapter entry's `private_protocol`: the configuration revision
    /// stays, the adapter process of the earlier entry keeps running.
    Entry,
}

/// Field order is drop order: the CLI stops its owner before the Secret
/// Service and the provider fixture go away.
struct Scenario {
    cli: Cli,
    _custody: Custody,
    provider: Provider,
    credential: PathBuf,
    reference: String,
    /// The saved connection's public revision, re-read before a retry.
    revision: std::cell::RefCell<String>,
}

impl Scenario {
    /// A GitLab catalog instance with one `ready` connection, whose adapter
    /// process is still running.
    fn connected() -> Self {
        let provider = Provider::new();
        let custody = Custody::new(provider.root.path());
        let cli = Cli::new(provider.root.path());
        configure(&cli, &provider, &custody);
        let credential = provider.root.path().join("private/credential.json");
        private(&credential, &token(true).0);
        let connected = success(cli.run(&[
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
        assert_eq!(connected["state"], "ready");
        Self {
            reference: connected["connection"].as_str().unwrap().to_owned(),
            revision: connected["revision"].as_str().unwrap().to_owned().into(),
            cli,
            _custody: custody,
            provider,
            credential,
        }
    }

    /// Edits the native configuration, then copies the bootstrap's new
    /// `configuration_revision` (and the native `instance`, which an edit may
    /// move) into the adapter entry. Answers the new revision.
    fn reconfigure(&self, edit: impl FnOnce(&mut Value)) -> String {
        let mut native: Value =
            serde_json::from_slice(&fs::read(&self.provider.config).unwrap()).unwrap();
        edit(&mut native);
        private(&self.provider.config, &serde_json::to_vec(&native).unwrap());
        let revision = self.provider.selection().configuration_revision;
        let instance = native["instance"].as_str().unwrap().to_owned();
        let q = |value: &str| serde_json::to_string(value).unwrap();
        let entry: String = fs::read_to_string(&self.cli.paths.config)
            .unwrap()
            .lines()
            .map(|line| {
                if line.starts_with("configuration_revision=") {
                    format!("configuration_revision={}\n", q(&revision))
                } else if line.starts_with("instance_id=") {
                    format!("instance_id={}\n", q(&instance))
                } else {
                    format!("{line}\n")
                }
            })
            .collect();
        private(&self.cli.paths.config, entry.as_bytes());
        connectors_host::local::config::Config::load(&self.cli.paths.config).unwrap();
        revision
    }

    fn apply(&self, change: Change) -> String {
        let revision = match change {
            Change::Lifetime { .. } => self.reconfigure(|native| {
                native["auth"]["evidence_lifetime_ms"] = json!(300_000);
            }),
            Change::Authentication => self.reconfigure(|native| {
                native["auth"]["label"] = json!("GitLab token");
            }),
            Change::Entry => {
                // `connectors-local/1` forbids the field; `/2` requires it.
                let entry = fs::read_to_string(&self.cli.paths.config)
                    .unwrap()
                    .replace("format='connectors-local/1'", "format='connectors-local/2'")
                    .replace(
                        "protocol='v1alpha1'\n",
                        "protocol='v1alpha1'\nprivate_protocol='connectors-private/2'\n",
                    );
                private(&self.cli.paths.config, entry.as_bytes());
                connectors_host::local::config::Config::load(&self.cli.paths.config).unwrap();
                self.provider.selection().configuration_revision
            }
        };
        if let Change::Lifetime { restart: true } = change {
            self.cli.shutdown();
        }
        revision
    }

    fn run(&self, command: Recovery) -> Output {
        let credential = self.credential.to_str().unwrap();
        let revision = self.revision.borrow().clone();
        let connection = [
            "--adapter",
            "gitlab",
            "--connection",
            &self.reference,
            "--expected-revision",
            &revision,
        ];
        match command {
            Recovery::Revalidate => self
                .cli
                .run(&[&["connections", "revalidate"][..], &connection].concat()),
            Recovery::Repair => self.cli.run(
                &[
                    &["connections", "repair"][..],
                    &connection,
                    &["--credential-file", credential],
                ]
                .concat(),
            ),
            Recovery::Connect => self.cli.run(&[
                "connections",
                "connect",
                "--adapter",
                "gitlab",
                "--profile",
                "gitlab.pat",
                "--credential-file",
                credential,
            ]),
        }
    }

    /// Re-reads the saved connection's public revision, so a retry carries the
    /// current one; a connection the configured instance no longer names keeps
    /// the revision it had.
    fn refresh(&self) {
        let output = self.cli.run(&[
            "connections",
            "describe",
            "--adapter",
            "gitlab",
            "--connection",
            &self.reference,
        ]);
        if output.status.success() {
            let described = success(output);
            *self.revision.borrow_mut() = described["connection"]["summary"]["revision"]
                .as_str()
                .unwrap()
                .to_owned();
        }
    }

    /// Runs `start`, then whatever command each refusal's next action names,
    /// until one succeeds. `check_configuration` is followed by moving the
    /// instance to a new id and connecting, and then by revoking the connection
    /// left under the earlier id; `retry_status` and `retry_explicitly` by the
    /// same command again, with the revision re-read. Every refusal is new: a
    /// refusal answered before, by the same command, is a loop. Answers the
    /// refusals, the successful result and how long each command took.
    fn walk(&self, start: Recovery) -> (Vec<(Recovery, String, String)>, Value, Vec<Duration>) {
        let mut refusals: Vec<(Recovery, String, String)> = Vec::new();
        let mut elapsed = Vec::new();
        let mut moved = false;
        let mut command = start;
        for _ in 0..4 {
            let started = Instant::now();
            let output = self.run(command);
            elapsed.push(started.elapsed());
            if output.status.success() {
                let result = success(output);
                if moved {
                    let revision = self.revision.borrow().clone();
                    let revoked = success(self.cli.run(&[
                        "connections",
                        "revoke",
                        "--adapter",
                        "gitlab",
                        "--connection",
                        &self.reference,
                        "--expected-revision",
                        &revision,
                    ]));
                    assert_eq!(revoked["local_state"], "revoked", "{start:?}: {revoked}");
                }
                return (refusals, result, elapsed);
            }
            let refused: Value = serde_json::from_slice(&output.stderr)
                .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stderr)));
            let data = &refused["error"]["data"];
            let step = (
                command,
                data["code"].as_str().unwrap().to_owned(),
                data["next_action"].as_str().unwrap().to_owned(),
            );
            assert!(
                !refusals.contains(&step),
                "{start:?} returned to an earlier refusal: {refusals:?}, then {step:?}"
            );
            command = match step.2.as_str() {
                "revalidate_connection" => Recovery::Revalidate,
                "repair_connection" => Recovery::Repair,
                "create_connection" => Recovery::Connect,
                "retry_status" | "retry_explicitly" => {
                    self.refresh();
                    command
                }
                "check_configuration" => {
                    self.reconfigure(|native| native["instance"] = json!("fixture-gitlab-2"));
                    moved = true;
                    Recovery::Connect
                }
                other => panic!("{start:?}: no command follows {other}: {refusals:?}, {step:?}"),
            };
            refusals.push(step);
        }
        panic!("{start:?} did not recover within four commands: {refusals:?}")
    }
}

/// The observed defect: the evidence lifetime raised, the new bootstrap
/// revision copied into the entry and the owner restarted. Revalidate answered
/// `lifecycle_conflict` / `create_connection` and connect then answered
/// `lifecycle_conflict` / `retry_status`. Revalidate still names a new
/// connection (the saved one stays bound to the declaration it was admitted
/// under), and now that is the single path: connect under the same instance id
/// answers `ready`, two commands after the change, and the old connection is
/// revocable.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_changed_evidence_lifetime_recovers_with_a_new_connection_in_two_commands() {
    let scenario = Scenario::connected();
    let configured = scenario.apply(Change::Lifetime { restart: true });
    let calls = scenario.provider.count();
    let refused = refusal(scenario.run(Recovery::Revalidate), "lifecycle_conflict");
    assert_eq!(refused["next_action"], "create_connection", "{refused}");
    assert_eq!(scenario.provider.count(), calls);
    let connected = success(scenario.run(Recovery::Connect))["connection"].clone();
    assert_eq!(connected["summary"]["state"], "ready", "{connected}");
    assert_eq!(connected["stale"], false);
    assert_eq!(scenario.cli.status()["configuration_revision"], configured);
    let fresh = connected["summary"]["connection"].as_str().unwrap();
    assert_ne!(fresh, scenario.reference);
    let read = scenario
        .cli
        .operation_result(fresh, "project.get", json!({"id": "org/project"}));
    assert_eq!(read["result"]["body"]["id"], 7);
    success(scenario.cli.run(&[
        "connections",
        "revoke",
        "--adapter",
        "gitlab",
        "--connection",
        &scenario.reference,
        "--expected-revision",
        &scenario.revision.borrow(),
    ]));
}

/// "Also observed": the adapter entry edited (here only its
/// `private_protocol`) while the adapter process of the earlier entry still
/// runs. Revalidate answered `lifecycle_conflict` / `retry_status`, which no
/// retry satisfies until `adapters stop`; it now stops that process and
/// launches the configured one itself, and answers `ready`.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn revalidate_replaces_the_adapter_process_of_an_earlier_entry() {
    let scenario = Scenario::connected();
    let before = scenario.cli.status();
    assert_eq!(before["state"], "ready", "{before}");
    let configured = scenario.apply(Change::Entry);
    assert_eq!(before["configuration_revision"], configured);
    let revalidated = success(scenario.run(Recovery::Revalidate))["connection"].clone();
    assert_eq!(revalidated["summary"]["state"], "ready", "{revalidated}");
    let after = scenario.cli.status();
    assert_eq!(after["state"], "ready", "{after}");
    assert_eq!(after["host_incarnation"], before["host_incarnation"]);
    assert_ne!(after["child_incarnation"], before["child_incarnation"]);
}

/// Every next action the CLI answers after a revision change, followed from
/// each recovery command: none leads back to a refusal already answered, and
/// each walk ends in a `ready` connection. An unchanged identity recovers by
/// revalidation or a new connection under the same instance id, also while the
/// adapter process of the earlier entry runs; a changed authentication by a
/// new instance id, named by every command at once.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn no_next_action_after_a_revision_change_returns_to_an_earlier_refusal() {
    for change in [
        Change::Lifetime { restart: true },
        Change::Lifetime { restart: false },
        Change::Entry,
        Change::Authentication,
    ] {
        for start in [Recovery::Revalidate, Recovery::Repair, Recovery::Connect] {
            let scenario = Scenario::connected();
            scenario.apply(change);
            let (refusals, result, elapsed) = scenario.walk(start);
            assert_eq!(
                result["connection"]["summary"]["state"], "ready",
                "{change:?} from {start:?}: {refusals:?}"
            );
            eprintln!("{change:?} from {start:?}: {refusals:?} then ready, {elapsed:?}");
            if let Change::Authentication = change {
                assert_eq!(
                    refusals
                        .first()
                        .map(|(_, code, action)| (code.as_str(), action.as_str())),
                    Some(("lifecycle_conflict", "check_configuration")),
                    "{start:?}"
                );
            } else {
                assert!(
                    refusals.len() <= 1,
                    "{change:?} from {start:?}: {refusals:?}"
                );
            }
        }
    }
}

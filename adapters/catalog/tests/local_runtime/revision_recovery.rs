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
        Self::connected_under(None, false)
    }

    /// As `connected`, with the native selection limited to `selection` when
    /// given, and the entry stating the bootstrap's `credential_revision`
    /// when `pinned`.
    fn connected_under(selection: Option<&[&str]>, pinned: bool) -> Self {
        let provider = Provider::new();
        if let Some(selection) = selection {
            let path = write_selection(&provider, selection);
            let mut native: Value =
                serde_json::from_slice(&fs::read(&provider.config).unwrap()).unwrap();
            native["operations_file"] = json!(path);
            private(&provider.config, &serde_json::to_vec(&native).unwrap());
        }
        let custody = Custody::new(provider.root.path());
        let cli = Cli::new(provider.root.path());
        configure(&cli, &provider, &custody);
        if let Some(selection) = selection {
            permit(&cli, selection);
        }
        if pinned {
            pin(&cli, &provider);
        }
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
        let selected = self.provider.selection();
        let revision = selected.configuration_revision;
        let credential = selected.credential_revision.unwrap_or_default();
        let instance = native["instance"].as_str().unwrap().to_owned();
        let q = |value: &str| serde_json::to_string(value).unwrap();
        let entry: String = fs::read_to_string(&self.cli.paths.config)
            .unwrap()
            .lines()
            .map(|line| {
                if line.starts_with("configuration_revision=") {
                    format!("configuration_revision={}\n", q(&revision))
                } else if line.starts_with("credential_revision=") {
                    format!("credential_revision={}\n", q(&credential))
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

    /// Selects exactly `selection` natively, copies the new revisions into the
    /// entry and permits every operation of it, as an operator does.
    fn select(&self, selection: &[&str]) {
        write_selection(&self.provider, selection);
        self.reconfigure(|_| {});
        permit(&self.cli, selection);
    }

    /// Adds a second trust root to the CA file: the same server stays
    /// trusted, the credential side of the configuration moves. Answers the
    /// new configuration revision.
    fn trust_another_root(&self) -> String {
        let other = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let roots = format!("{}{}", self.provider.pem, other.cert.pem());
        private(&self.provider.ca, roots.as_bytes());
        self.reconfigure(|_| {})
    }

    /// Replaces the provider executable with a copy that differs in one
    /// trailing byte, `tag`: another digest, the same program. Only the
    /// entry's executable path and digest change.
    fn replace_executable(&self, tag: u8) {
        let original = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"));
        let mut bytes = fs::read(&original).unwrap();
        bytes.push(tag);
        let copy = self
            .provider
            .root
            .path()
            .join(format!("private/provider-{tag}"));
        fs::write(&copy, &bytes).unwrap();
        fs::set_permissions(&copy, fs::Permissions::from_mode(0o700)).unwrap();
        let q = |value: &str| serde_json::to_string(value).unwrap();
        let sha256 = hex::encode(Sha256::digest(&bytes));
        let entry: String = fs::read_to_string(&self.cli.paths.config)
            .unwrap()
            .lines()
            .map(|line| {
                if line.starts_with("path=") {
                    format!("path={}\n", q(copy.to_str().unwrap()))
                } else if line.starts_with("sha256=") {
                    format!("sha256={}\n", q(&sha256))
                } else {
                    format!("{line}\n")
                }
            })
            .collect();
        private(&self.cli.paths.config, entry.as_bytes());
        connectors_host::local::config::Config::load(&self.cli.paths.config).unwrap();
    }

    /// The validations the provider fixture has answered: each asks for the
    /// account the credential authenticates.
    fn validations(&self) -> usize {
        self.provider
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|path| path.split('?').next() == Some("/api/v4/user"))
            .count()
    }

    fn list(&self) -> Value {
        success(
            self.cli
                .run(&["connections", "list", "--adapter", "gitlab"]),
        )
    }

    fn describe(&self) -> Value {
        success(self.cli.run(&[
            "connections",
            "describe",
            "--adapter",
            "gitlab",
            "--connection",
            &self.reference,
        ]))["connection"]
            .clone()
    }

    /// `operations invoke` of `operation` through the saved connection, its
    /// answer unchecked.
    fn invoke(&self, operation: &str, input: Value) -> Output {
        let description = success(self.cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "gitlab",
            "--operation",
            operation,
        ]));
        self.cli.run(&[
            "operations",
            "invoke",
            "--adapter",
            "gitlab",
            "--connection",
            &self.reference,
            "--operation",
            operation,
            "--schema",
            description["schema"].as_str().unwrap(),
            "--revision",
            description["revision"].as_str().unwrap(),
            "--input-json",
            &input.to_string(),
        ])
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

/// Writes the shipped GitLab operations narrowed to `selection` as the
/// provider's operations file, and answers its path.
fn write_selection(provider: &Provider, selection: &[&str]) -> PathBuf {
    let shipped: Value = serde_json::from_slice(
        &fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("providers/gitlab/operations.json"))
            .unwrap(),
    )
    .unwrap();
    let mut narrowed = shipped.clone();
    narrowed["operations"] = json!(
        shipped["operations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|operation| selection.contains(&operation["id"].as_str().unwrap_or("")))
            .cloned()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        narrowed["operations"].as_array().unwrap().len(),
        selection.len()
    );
    let path = provider.root.path().join("private/operations.json");
    private(&path, &serde_json::to_vec(&narrowed).unwrap());
    path
}

/// Permits exactly the operations of `selection` in the adapter entry.
fn permit(cli: &Cli, selection: &[&str]) {
    let operations = selection
        .iter()
        .map(|id| format!("'{id}'"))
        .collect::<Vec<_>>()
        .join(",");
    let entry: String = fs::read_to_string(&cli.paths.config)
        .unwrap()
        .lines()
        .map(|line| {
            if line.starts_with("operations=") {
                format!("operations=[{operations}]\n")
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    private(&cli.paths.config, entry.as_bytes());
    connectors_host::local::config::Config::load(&cli.paths.config).unwrap();
}

/// States the bootstrap's `credential_revision` in the adapter entry, beside
/// its `configuration_revision`, as the upgrade guide says.
fn pin(cli: &Cli, provider: &Provider) {
    let credential = provider
        .selection()
        .credential_revision
        .expect("the catalog provider prints its credential-side digest");
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let entry: String = fs::read_to_string(&cli.paths.config)
        .unwrap()
        .lines()
        .map(|line| {
            if line.starts_with("configuration_revision=") {
                format!("{line}\ncredential_revision={}\n", q(&credential))
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    private(&cli.paths.config, entry.as_bytes());
    connectors_host::local::config::Config::load(&cli.paths.config).unwrap();
}

/// story:connection-binding-excludes-operations: the selection a connection
/// was saved under, and that selection with one read added.
const S1: [&str; 2] = ["project.get", "issues.list"];
const S2: [&str; 3] = ["project.get", "issues.list", "project.languages"];

/// The catalog provider prints its credential-side digest beside the
/// configuration revision: a selection change moves only the revision; a
/// trust root added to the CA file moves both.
#[test]
fn the_printed_credential_revision_moves_with_the_credential_side_only() {
    let provider = Provider::new();
    let path = write_selection(&provider, &S1);
    let mut native: Value = serde_json::from_slice(&fs::read(&provider.config).unwrap()).unwrap();
    native["operations_file"] = json!(path);
    private(&provider.config, &serde_json::to_vec(&native).unwrap());
    let first = provider.selection();
    assert!(first.credential_revision.is_some());
    write_selection(&provider, &S2);
    let selected = provider.selection();
    assert_ne!(
        first.configuration_revision,
        selected.configuration_revision
    );
    assert_eq!(first.credential_revision, selected.credential_revision);
    let other = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    private(
        &provider.ca,
        format!("{}{}", provider.pem, other.cert.pem()).as_bytes(),
    );
    let trusted = provider.selection();
    assert_ne!(
        selected.configuration_revision,
        trusted.configuration_revision
    );
    assert_ne!(selected.credential_revision, trusted.credential_revision);
}

/// story:connection-binding-excludes-operations, acceptance: a connection
/// saved under selection S1 lists `ready`, not stale, after only its selection
/// changes to S2, and S2's added read succeeds with no revalidation, no
/// credential source and no prompt: `operations describe` starts the
/// configured adapter for S2's description, and the provider answers no
/// validation; the public revision is unchanged.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_selection_only_change_leaves_the_connection_ready_and_serves_the_added_read() {
    let scenario = Scenario::connected_under(Some(&S1), true);
    let validations = scenario.validations();
    let revision = scenario.revision.borrow().clone();
    scenario.select(&S2);
    let listed = scenario.list();
    assert_eq!(listed["connections"][0]["state"], "ready", "{listed}");
    assert_eq!(listed["stale"], false, "{listed}");
    let described = scenario.describe();
    assert_eq!(described["summary"]["state"], "ready", "{described}");
    assert_eq!(described["summary"]["revision"], revision, "{described}");
    assert_eq!(described["stale"], false, "{described}");
    let read = success(scenario.invoke("project.languages", json!({"id": "org/project"})));
    assert_eq!(read["result"]["body"]["Rust"], json!(71.42), "{read}");
    assert_eq!(scenario.validations(), validations);
}

/// A changed CA file moves the credential side: the connection lists
/// `pending` and stale until `connections revalidate`, which follows it, after
/// which the read is served. The binding moved, so the revalidate answer names
/// a new public revision of the connection, the one `connections describe`
/// names afterwards, and the instance runs under the configured revision
/// (story:connection-follows-configuration-upgrade).
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_changed_ca_file_stales_the_connection_until_it_is_revalidated() {
    let scenario = Scenario::connected_under(Some(&S1), true);
    let saved = scenario.revision.borrow().clone();
    let configured = scenario.trust_another_root();
    let listed = scenario.list();
    assert_eq!(listed["connections"][0]["state"], "pending", "{listed}");
    assert_eq!(listed["stale"], true, "{listed}");
    let revalidated = success(scenario.run(Recovery::Revalidate))["connection"].clone();
    assert_eq!(revalidated["summary"]["state"], "ready", "{revalidated}");
    assert_eq!(revalidated["stale"], false, "{revalidated}");
    let answered = revalidated["summary"]["revision"].as_str().unwrap();
    assert_ne!(answered, saved, "{revalidated}");
    let described = scenario.describe();
    assert_eq!(described["summary"]["revision"], answered, "{described}");
    let status = scenario.cli.status();
    assert_eq!(status["configuration_revision"], configured, "{status}");
    let read = success(scenario.invoke("project.get", json!({"id": "org/project"})));
    assert_eq!(read["result"]["body"]["id"], 7, "{read}");
}

/// Only the provider executable replaced, the bundle and selection unchanged:
/// the connection lists `ready`, not stale. One validation under the new
/// executable follows it (here the revalidation that caches its
/// description), and the read is served.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_replaced_executable_lists_ready_and_one_validation_follows_it() {
    let scenario = Scenario::connected_under(Some(&S1), true);
    let validations = scenario.validations();
    scenario.replace_executable(1);
    let listed = scenario.list();
    assert_eq!(listed["connections"][0]["state"], "ready", "{listed}");
    assert_eq!(listed["stale"], false, "{listed}");
    assert_eq!(scenario.validations(), validations);
    let revalidated = success(scenario.run(Recovery::Revalidate))["connection"].clone();
    assert_eq!(revalidated["summary"]["state"], "ready", "{revalidated}");
    assert_eq!(scenario.validations(), validations + 1);
    let read = success(scenario.invoke("project.get", json!({"id": "org/project"})));
    assert_eq!(read["result"]["body"]["id"], 7, "{read}");
    assert_eq!(scenario.validations(), validations + 1);
}

/// A replaced executable whose validation answers another identity is
/// refused as `identity_mismatch` and changes nothing: the connection keeps
/// its revision, identity and credential, and once the provider answers the
/// recorded identity again one validation follows the executable.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_replaced_executable_answering_another_identity_changes_nothing() {
    let scenario = Scenario::connected_under(Some(&S1), true);
    let before = scenario.describe();
    scenario.replace_executable(1);
    scenario
        .provider
        .identity_shift
        .store(100, Ordering::SeqCst);
    refusal(scenario.run(Recovery::Revalidate), "identity_mismatch");
    let after = scenario.describe();
    assert_eq!(after["summary"]["revision"], before["summary"]["revision"]);
    assert_eq!(after["summary"]["state"], "ready", "{after}");
    assert_eq!(after["external_identity"], before["external_identity"]);
    scenario.provider.identity_shift.store(0, Ordering::SeqCst);
    let revalidated = success(scenario.run(Recovery::Revalidate))["connection"].clone();
    assert_eq!(revalidated["summary"]["state"], "ready", "{revalidated}");
    let read = success(scenario.invoke("project.get", json!({"id": "org/project"})));
    assert_eq!(read["result"]["body"]["id"], 7, "{read}");
}

/// A connection saved before its binding recorded a credential-side digest
/// (here: under an entry that states none) keeps the configuration-revision
/// rule: a selection-only change stales it until `connections revalidate`,
/// which records the digest; the next selection-only change leaves it ready.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_connection_saved_without_the_digest_keeps_the_revision_rule_until_revalidated() {
    let scenario = Scenario::connected_under(Some(&S1), false);
    pin(&scenario.cli, &scenario.provider);
    scenario.select(&S2);
    let listed = scenario.list();
    assert_eq!(listed["connections"][0]["state"], "pending", "{listed}");
    assert_eq!(listed["stale"], true, "{listed}");
    let revalidated = success(scenario.run(Recovery::Revalidate))["connection"].clone();
    assert_eq!(revalidated["summary"]["state"], "ready", "{revalidated}");
    scenario.refresh();
    let validations = scenario.validations();
    scenario.select(&S1);
    let listed = scenario.list();
    assert_eq!(listed["connections"][0]["state"], "ready", "{listed}");
    assert_eq!(listed["stale"], false, "{listed}");
    assert_eq!(scenario.validations(), validations);
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

// Adversary pass 1 (story:connection-binding-excludes-operations).

/// The story's acceptance statement, read literally: after only its
/// selection changes from S1 to S2, "that added read succeeds with no
/// `connections revalidate`, no credential source and no prompt". No
/// revalidation, no repair, no connect: the operator describes the added read
/// and invokes it.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn adv_the_added_read_succeeds_after_a_selection_only_change_without_revalidation() {
    let scenario = Scenario::connected_under(Some(&S1), true);
    let validations = scenario.validations();
    scenario.select(&S2);
    let described = scenario.cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "gitlab",
        "--operation",
        "project.languages",
    ]);
    assert!(
        described.status.success(),
        "the added read cannot be described without a launch: {}",
        String::from_utf8_lossy(&described.stderr)
    );
    let read = success(scenario.invoke("project.languages", json!({"id": "org/project"})));
    assert_eq!(read["result"]["body"]["Rust"], 71.42, "{read}");
    assert_eq!(scenario.validations(), validations);
}

/// The owner's own validation under a replaced executable (supervisor
/// `Task::Invoke`, `ExecutableChanged`): no journey reaches it, because the
/// replaced-executable journeys run `connections revalidate` first. Here the
/// description is cached by a launch that publishes nothing (a reconnect the
/// registry refuses), and the read itself must validate once and be served.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn adv_a_read_under_a_replaced_executable_validates_once_and_is_served() {
    let scenario = Scenario::connected_under(Some(&S1), true);
    let before = scenario.describe();
    scenario.replace_executable(1);
    let _ = scenario.run(Recovery::Repair);
    scenario.refresh();
    let validations = scenario.validations();
    let read = success(scenario.invoke("project.get", json!({"id": "org/project"})));
    assert_eq!(read["result"]["body"]["id"], 7, "{read}");
    assert_eq!(scenario.validations(), validations + 1);
    let after = scenario.describe();
    assert_eq!(after["summary"]["state"], "ready", "{after}");
    assert_eq!(after["external_identity"], before["external_identity"]);
}

#[path = "revision_recovery_security.rs"]
mod revision_recovery_security;

/// Adversary pass 2. contracts/cli/v1alpha1/semantics.md: "Missing cached
/// operation metadata returns `description_unavailable`, ... never a launch
/// except as follows", and the launch for a description does not resume an
/// operator's `instances stop`. With the stop in place no launch happens, so
/// the command answers as it did before: `description_unavailable`, not an
/// `unavailable` whose `retry_status` no status change will ever clear (the
/// suppression stays until an explicit resume action).
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn adv2_a_stopped_instance_answers_description_unavailable_for_a_new_selection() {
    let scenario = Scenario::connected_under(Some(&S1), true);
    let coordinates = scenario.cli.status();
    success(scenario.cli.run(&[
        "instances",
        "stop",
        "--adapter",
        "gitlab",
        "--expected-revision",
        coordinates["configuration_revision"].as_str().unwrap(),
        "--host-incarnation",
        coordinates["host_incarnation"].as_str().unwrap(),
        "--child-incarnation",
        coordinates["child_incarnation"].as_str().unwrap(),
    ]));
    scenario.select(&S2);
    let validations = scenario.validations();
    let refused = refusal(
        scenario.cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "gitlab",
            "--operation",
            "project.languages",
        ]),
        "description_unavailable",
    );
    assert_ne!(refused["next_action"], "retry_status", "{refused}");
    assert_eq!(scenario.cli.status()["state"], "suppressed");
    assert_eq!(scenario.validations(), validations);
}

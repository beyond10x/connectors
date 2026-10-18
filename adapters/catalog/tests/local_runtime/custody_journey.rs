//! A write whose answer carries a credential it created, through the production
//! CLI, owner, two catalog children, the local TLS fixture, the signing clock
//! and the disposable Secret Service (`docs/local-mutation-ledger.md`, "Created
//! credentials"). The fixture's group token create route mints a token; the
//! owner holds it as a new connection of the target alias `minted`, and the
//! caller receives only that connection and the token's public hash.
use super::*;
use std::collections::BTreeSet;

/// The selection the writer's catalog configuration adds: a write whose answer
/// carries the created token at `/token`, returning only its `hash`.
fn create_selection() -> Value {
    json!({"id": "group_token.create", "operation_id": "postApiV4GroupsIdAccessTokens",
        "effect": "write", "description": "Create a group access token",
        "custody": {"secret": "/token", "returns": {"hash": "/hash"}}})
}

struct Journey {
    provider: Provider,
    custody: Custody,
    _clock: Clock,
    cli: Cli,
    connection: String,
    /// Every CLI output of the journey, searched for the created token.
    outputs: std::cell::RefCell<Vec<u8>>,
}

/// The owner's custody table for the create, as every journey but one configures it.
const CUSTODY_TABLE: &str = "[adapters.gitlab.permissions.custody]\n'group_token.create'={adapter='minted',profile='gitlab.pat'}\n";

impl Journey {
    fn new(create_mode: u8) -> Self {
        Self::with_custody(create_mode, CUSTODY_TABLE)
    }

    /// [`Journey::new`], the writer's entry stating the bootstrap's
    /// `credential_revision`, so its connection is bound to the credential side
    /// and the executable (story:connection-binding-excludes-operations).
    fn pinned(create_mode: u8) -> Self {
        Self::build(create_mode, CUSTODY_TABLE, true)
    }

    /// [`Journey::new`] with `custody_table` as the owner's custody table for
    /// the create; empty configures none.
    fn with_custody(create_mode: u8, custody_table: &str) -> Self {
        Self::build(create_mode, custody_table, false)
    }

    fn build(create_mode: u8, custody_table: &str, pinned: bool) -> Self {
        let provider = Provider::new();
        provider.create_mode.store(create_mode, Ordering::SeqCst);
        let custody = Custody::new(provider.root.path());
        let clock = Clock::new();
        let cli = Cli::new(provider.root.path());
        // The writer's catalog configuration adds the create selection; the
        // target's is the same provider under its own instance.
        let mut writer: Value =
            serde_json::from_slice(&fs::read(&provider.config).unwrap()).unwrap();
        writer["operations"] = json!([create_selection()]);
        private(&provider.config, &serde_json::to_vec(&writer).unwrap());
        let mut target = writer.clone();
        target["instance"] = json!("fixture-minted");
        target["operations"] = json!([]);
        let target_config = provider.root.path().join("private/minted.json");
        private(&target_config, &serde_json::to_vec(&target).unwrap());
        success(cli.run(&["setup", "init"]));
        let entry = |alias: &str, adapter: &Adapter, operations: &str| {
            let q = |value: &str| serde_json::to_string(value).unwrap();
            format!(
                "[adapters.{alias}]\ninstance_id={}\nadapter_id='catalog'\nconfiguration_revision={}\n{}protocol='v1alpha1'\nprivate_protocol='connectors-private/2'\nstartup='on-demand'\nrestart='never'\n[adapters.{alias}.permissions]\nprofiles=['gitlab.pat']\noperations={operations}\n[adapters.{alias}.executable]\npath={}\nsha256={}\nargs={}\n",
                q(&adapter.instance_id),
                q(&adapter.configuration_revision),
                match (&adapter.credential_revision, pinned && alias == "gitlab") {
                    (Some(credential), true) => format!("credential_revision={}\n", q(credential)),
                    _ => String::new(),
                },
                q(adapter.executable.path.to_str().unwrap()),
                q(&adapter.executable.sha256),
                serde_json::to_string(&adapter.executable.args).unwrap()
            )
        };
        let configuration = format!(
            "format='connectors-local/2'\nowner_uid={}\nsecret_service_socket={}\n{}{}{}\n{}",
            filesystem::uid(),
            serde_json::to_string(custody.socket.to_str().unwrap()).unwrap(),
            entry(
                "gitlab",
                &provider.selection(),
                "['project.get','group_token.create']"
            ),
            custody_table,
            entry(
                "minted",
                &provider.selection_of(&target_config, "fixture-minted"),
                "['project.get']"
            ),
            clock.table(),
        );
        private(&cli.paths.config, configuration.as_bytes());
        connectors_host::local::config::Config::load(&cli.paths.config).unwrap();
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
        ]));
        fs::remove_file(&credential).unwrap();
        let connection = connected["connection"]["summary"]["connection"]
            .as_str()
            .unwrap()
            .to_owned();
        success(cli.run(&["approvals", "key-init", "--adapter", "gitlab"]));
        let policy = provider.root.path().join("private/policy.json");
        private(&policy, br#"{"operations":["group_token.create"]}"#);
        success(cli.run(&[
            "approvals",
            "policy-set",
            "--adapter",
            "gitlab",
            "--input-file",
            policy.to_str().unwrap(),
        ]));
        Self {
            provider,
            custody,
            _clock: clock,
            cli,
            connection,
            outputs: Default::default(),
        }
    }

    fn run(&self, args: &[String]) -> Output {
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = self.cli.run(&borrowed);
        let mut outputs = self.outputs.borrow_mut();
        outputs.extend_from_slice(&output.stdout);
        outputs.extend_from_slice(&output.stderr);
        output
    }

    /// The target arguments of the create, under `key`.
    fn target(&self, group: &str, action: &str) -> Vec<String> {
        let description = success(self.cli.run(&[
            "operations",
            "describe",
            "--adapter",
            "gitlab",
            "--operation",
            "group_token.create",
        ]));
        [
            group,
            action,
            "--adapter",
            "gitlab",
            "--connection",
            &self.connection,
            "--operation",
            "group_token.create",
            "--schema",
            description["schema"].as_str().unwrap(),
            "--revision",
            description["revision"].as_str().unwrap(),
            "--input-json",
            &json!({"id": "org", "body": {"name": "tenant", "scopes": ["api"]}}).to_string(),
        ]
        .map(str::to_owned)
        .to_vec()
    }

    /// The approved create invocation under `key`, its proof issued.
    fn approved(&self, key: &str) -> Vec<String> {
        let prepared = success(self.run(&self.target("approvals", "prepare")));
        let proof = self.provider.root.path().join("private/proof.json");
        let mut issue = self.target("approvals", "issue");
        issue.extend(
            [
                "--approve-subject",
                prepared["preparation"]["subject_sha256"].as_str().unwrap(),
                "--proof-output",
                proof.to_str().unwrap(),
            ]
            .map(str::to_owned),
        );
        success(self.run(&issue));
        let mut invocation = self.target("operations", "invoke");
        invocation.extend(
            [
                "--idempotency-key",
                key,
                "--approval-file",
                proof.to_str().unwrap(),
            ]
            .map(str::to_owned),
        );
        invocation
    }

    /// Replaces the writer's provider executable with a copy one trailing byte
    /// longer: another digest, the same program. Only the writer entry's
    /// executable path and digest change.
    fn replace_writer_executable(&self) {
        let mut bytes = fs::read(env!("CARGO_BIN_EXE_connectors-catalog-provider")).unwrap();
        bytes.push(1);
        let copy = self.provider.root.path().join("private/writer-provider");
        fs::write(&copy, &bytes).unwrap();
        fs::set_permissions(&copy, fs::Permissions::from_mode(0o700)).unwrap();
        let q = |value: &str| serde_json::to_string(value).unwrap();
        let mut writer = false;
        let entry: String = fs::read_to_string(&self.cli.paths.config)
            .unwrap()
            .lines()
            .map(|line| {
                if line.starts_with('[') {
                    writer = line == "[adapters.gitlab.executable]";
                }
                if writer && line.starts_with("path=") {
                    format!("path={}\n", q(copy.to_str().unwrap()))
                } else if writer && line.starts_with("sha256=") {
                    format!("sha256={}\n", q(&hex::encode(Sha256::digest(&bytes))))
                } else {
                    format!("{line}\n")
                }
            })
            .collect();
        private(&self.cli.paths.config, entry.as_bytes());
        connectors_host::local::config::Config::load(&self.cli.paths.config).unwrap();
    }

    /// The validations of the writer's credential the fixture answered: each
    /// asks for the account with it. The created token's own validation, as
    /// the new connection of `minted`, carries another token.
    fn validations(&self) -> usize {
        self.provider
            .tokens
            .lock()
            .unwrap()
            .iter()
            .filter(|(route, token)| {
                route == "/api/v4/user" && token.as_deref() == Some("fixture-pat-one")
            })
            .count()
    }

    fn minted(&self) -> Vec<Value> {
        success(
            self.cli
                .run(&["connections", "list", "--adapter", "minted"]),
        )["connections"]
            .as_array()
            .unwrap()
            .clone()
    }

    /// No output of the journey, and no file under its root, carries `secret`.
    fn assert_nowhere(&self, secret: &str) {
        assert!(!carries(&self.outputs.borrow(), secret), "a CLI output");
        let mut pending = vec![self.provider.root.path().to_path_buf()];
        let mut searched = 0;
        while let Some(path) = pending.pop() {
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.is_dir() {
                for entry in fs::read_dir(&path).unwrap() {
                    pending.push(entry.unwrap().path());
                }
            } else if metadata.is_file() {
                searched += 1;
                let bytes = fs::read(&path).unwrap();
                assert!(!carries(&bytes, secret), "{}", path.display());
            }
        }
        // The ledger, audit, registry and keyring files are among them.
        assert!(searched > 10, "{searched} files searched");
    }
}

fn carries(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

#[track_caller]
fn failed(output: Output, code: &str) -> Value {
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["error"]["data"]["code"], code, "{value}");
    value["error"]["data"].clone()
}

/// story:connection-binding-excludes-operations: a write after only the
/// writer's provider executable changed validates its connection once under
/// the new executable and is sent, as a read is; it never answers
/// `revalidate_connection` for a connection that lists `ready`.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_write_after_an_executable_only_change_is_validated_once_and_sent() {
    let journey = Journey::pinned(0);
    let validations = journey.validations();
    journey.replace_writer_executable();
    // `operations describe` starts the new executable for its description,
    // validating nothing. The approval policy binds the executable selection,
    // which D12 leaves as it is: it is published again for the new executable.
    // The connection is not touched.
    success(journey.cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "gitlab",
        "--operation",
        "group_token.create",
    ]));
    assert_eq!(journey.validations(), validations);
    let policy = journey.provider.root.path().join("private/policy.json");
    success(journey.cli.run(&[
        "approvals",
        "policy-set",
        "--adapter",
        "gitlab",
        "--input-file",
        policy.to_str().unwrap(),
        "--expected-revision",
        "1",
    ]));
    let invocation = journey.approved("tenant-key");
    let created = success(journey.run(&invocation));
    assert_eq!(
        created["mutation"]["classification"], "applied",
        "{created}"
    );
    assert_eq!(journey.provider.create_effects.load(Ordering::SeqCst), 1);
    assert_eq!(journey.validations(), validations + 1);
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_created_token_is_held_as_a_new_connection_and_only_its_hash_is_returned() {
    let journey = Journey::new(0);
    assert!(journey.minted().is_empty());
    let invocation = journey.approved("tenant-key");
    let created = success(journey.run(&invocation));
    assert_eq!(created["mutation"]["classification"], "applied");
    let result = created["result"].as_object().unwrap();
    assert_eq!(
        result.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from(["connection", "custody", "hash"])
    );
    assert_eq!(result["custody"], "stored");
    assert_eq!(result["hash"], CREATED_HASH);
    let connection = result["connection"].as_str().unwrap().to_owned();
    assert_eq!(journey.provider.create_effects.load(Ordering::SeqCst), 1);
    // The new connection is the target's, and a read through it sends the
    // created token, which no other connection holds.
    let listed = journey.minted();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["connection"], connection.as_str());
    let description = success(journey.cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "minted",
        "--operation",
        "project.get",
    ]));
    let before = journey.provider.tokens.lock().unwrap().len();
    let read = success(
        journey.run(
            &[
                "operations",
                "invoke",
                "--adapter",
                "minted",
                "--connection",
                &connection,
                "--operation",
                "project.get",
                "--schema",
                description["schema"].as_str().unwrap(),
                "--revision",
                description["revision"].as_str().unwrap(),
                "--input-json",
                r#"{"id":"org/project"}"#,
            ]
            .map(str::to_owned),
        ),
    );
    assert_eq!(read["result"]["body"]["id"], 7);
    let tokens = journey.provider.tokens.lock().unwrap();
    assert!(tokens[before..].iter().any(|(route, token)| {
        route == "/api/v4/projects/org%2Fproject" && token.as_deref() == Some(CREATED_TOKEN)
    }));
    drop(tokens);
    // A replay answers the stored public result and sends nothing.
    let replay = success(journey.run(&invocation));
    assert_eq!(replay["mutation"]["replayed"], true);
    assert_eq!(replay["result"], created["result"]);
    assert_eq!(journey.provider.create_effects.load(Ordering::SeqCst), 1);
    journey.cli.shutdown();
    journey.assert_nowhere(CREATED_TOKEN);
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_create_that_times_out_after_receipt_is_unknown_and_never_sent_again() {
    let mut journey = Journey::new(1);
    let invocation = journey.approved("tenant-key");
    let lost = failed(journey.run(&invocation), "outcome_unknown");
    assert_eq!(lost["mutation"]["classification"], "unknown");
    assert_eq!(journey.provider.create_effects.load(Ordering::SeqCst), 1);
    // A retry observes the attempt; it sends nothing.
    let retried = failed(journey.run(&invocation), "outcome_unknown");
    assert_eq!(retried["mutation"]["attempt"], lost["mutation"]["attempt"]);
    // A restart of the owner and the keyring sends nothing either.
    journey.cli.shutdown();
    journey.custody.restart();
    let restarted = failed(journey.run(&invocation), "outcome_unknown");
    assert_eq!(
        restarted["mutation"]["attempt"],
        lost["mutation"]["attempt"]
    );
    // Released, the held answer reaches no caller.
    journey.provider.create_mode.store(0, Ordering::SeqCst);
    let again = failed(journey.run(&invocation), "outcome_unknown");
    assert_eq!(again["mutation"]["attempt"], lost["mutation"]["attempt"]);
    assert_eq!(journey.provider.create_effects.load(Ordering::SeqCst), 1);
    assert!(journey.minted().is_empty());
    journey.cli.shutdown();
    journey.assert_nowhere(CREATED_TOKEN);
}

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_created_token_custody_refuses_is_reported_by_its_hash_and_held_nowhere() {
    let journey = Journey::new(2);
    let invocation = journey.approved("tenant-key");
    let refused = failed(journey.run(&invocation), "custody_unavailable");
    assert_eq!(refused["mutation"]["classification"], "applied");
    assert_eq!(refused["custody"], "failed");
    assert_eq!(refused["created"], json!({"hash": CREATED_HASH}));
    assert_eq!(journey.provider.create_effects.load(Ordering::SeqCst), 1);
    assert!(journey.minted().is_empty());
    // The ledger records the write applied: a replay answers the same and
    // sends nothing.
    let replay = failed(journey.run(&invocation), "custody_unavailable");
    assert_eq!(replay["mutation"]["replayed"], true);
    assert_eq!(replay["mutation"]["classification"], "applied");
    assert_eq!(replay["custody"], "failed");
    assert_eq!(replay["created"], refused["created"]);
    assert_eq!(journey.provider.create_effects.load(Ordering::SeqCst), 1);
    journey.cli.shutdown();
    journey.assert_nowhere(UNACCEPTED_TOKEN);
}

/// Integrity of the pre-dispatch custody check (design §8a, "Admit"): an
/// operation whose selection declares custody, granted to the writer while the
/// owner names no custody target for it, can only end with a created
/// credential nobody holds. It is refused with nothing sent, so a credential
/// is never created that cannot be held or named by its hash.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_create_whose_operation_has_no_custody_target_is_refused_before_it_is_sent() {
    let journey = Journey::with_custody(0, "");
    let invocation = journey.approved("tenant-key");
    let output = journey.run(&invocation);
    assert!(!output.status.success());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        journey.provider.create_effects.load(Ordering::SeqCst),
        0,
        "the create was sent with no custody target configured: {failure}"
    );
    assert!(journey.minted().is_empty());
    journey.cli.shutdown();
    journey.assert_nowhere(CREATED_TOKEN);
}

// Security review, round 2 (story:connection-binding-excludes-operations): a
// write after an executable-only change keeps its approval, policy and
// identity bindings.

/// A proof issued before only the writer's provider executable changed
/// authorizes no write after it, even once the policy is published again for
/// the new executable (whose subject the proof does not carry): the public
/// connection revision the follow keeps is not the only coordinate the
/// approval binds. Nothing is created and no validation is sent.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn security2_a_proof_issued_before_an_executable_only_change_authorizes_no_write() {
    let journey = Journey::pinned(0);
    let invocation = journey.approved("security2-before");
    let validations = journey.validations();
    journey.replace_writer_executable();
    success(journey.cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "gitlab",
        "--operation",
        "group_token.create",
    ]));
    let policy = journey.provider.root.path().join("private/policy.json");
    success(journey.cli.run(&[
        "approvals",
        "policy-set",
        "--adapter",
        "gitlab",
        "--input-file",
        policy.to_str().unwrap(),
        "--expected-revision",
        "1",
    ]));
    let output = journey.run(&invocation);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    eprintln!("security2 refusal: {stderr}");
    assert!(
        !output.status.success() || !stdout.contains("\"applied\""),
        "a proof issued before the executable change authorized a write: {stdout}"
    );
    assert_eq!(
        journey.provider.create_effects.load(Ordering::SeqCst),
        0,
        "{stderr}"
    );
    assert_eq!(journey.validations(), validations, "{stderr}");
}

/// A write whose new executable's validation answers another identity is
/// refused as `identity_mismatch` after exactly one validation, creates
/// nothing, and leaves the connection's revision and identity as they were.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn security2_a_write_whose_new_executable_answers_another_identity_sends_nothing() {
    let journey = Journey::pinned(0);
    let describe = || {
        success(journey.cli.run(&[
            "connections",
            "describe",
            "--adapter",
            "gitlab",
            "--connection",
            &journey.connection,
        ]))["connection"]
            .clone()
    };
    let before = describe();
    journey.replace_writer_executable();
    success(journey.cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "gitlab",
        "--operation",
        "group_token.create",
    ]));
    let policy = journey.provider.root.path().join("private/policy.json");
    success(journey.cli.run(&[
        "approvals",
        "policy-set",
        "--adapter",
        "gitlab",
        "--input-file",
        policy.to_str().unwrap(),
        "--expected-revision",
        "1",
    ]));
    let invocation = journey.approved("security2-identity");
    let validations = journey.validations();
    journey.provider.identity_shift.store(100, Ordering::SeqCst);
    failed(journey.run(&invocation), "identity_mismatch");
    assert_eq!(journey.provider.create_effects.load(Ordering::SeqCst), 0);
    assert_eq!(journey.validations(), validations + 1);
    journey.provider.identity_shift.store(0, Ordering::SeqCst);
    let after = describe();
    assert_eq!(after["summary"]["revision"], before["summary"]["revision"]);
    assert_eq!(after["external_identity"], before["external_identity"]);
}

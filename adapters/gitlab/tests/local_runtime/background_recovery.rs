//! Independent provider/clock barriers and production owner cleanup. The extra
//! unkeyed preparation is a durable-port producer fixture, not a provider send.
use super::*;

fn states(cli: &Cli) -> Vec<(String, String, Option<i64>, Option<String>)> {
    let db = rusqlite::Connection::open_with_flags(
        cli.paths.state.join("metadata.sqlite3"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut query = db.prepare("SELECT a.request_id,a.state,a.settled_at_ms,k.state FROM mutation_attempts a LEFT JOIN mutation_keys k ON k.attempt_id=a.attempt_id ORDER BY a.request_id").unwrap();
    query
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .map(|row| row.unwrap())
        .collect()
}
fn await_state(cli: &Cli, state: &str) {
    let until = Instant::now() + Duration::from_secs(18);
    loop {
        let rows = states(cli);
        if rows.iter().any(|row| row.1 == state) {
            return;
        }
        assert!(
            Instant::now() < until,
            "background did not reach {state}: {rows:?}"
        );
        std::thread::sleep(Duration::from_millis(30));
    }
}

fn seed_unkeyed(provider: &Provider, subject: &Value) {
    private(
        &provider.root.path().join("private/subject.json"),
        &serde_json::to_vec(subject).unwrap(),
    );
    let producer = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "cli_journey::guarded_merge::prepared_attempt_exit_fixture",
            "--ignored",
            "--nocapture",
        ])
        .env_clear()
        .env("CONNECTORS_PREPARED_FIXTURE", provider.root.path())
        .env("CONNECTORS_PREPARED_FIXTURE_UNKEYED", "1")
        .output()
        .unwrap();
    assert_eq!(producer.status.code(), Some(73));
}

pub(super) fn removed(
    cli: &Cli,
    provider: &Provider,
    custody: &mut Custody,
    clock: &Clock,
    subject: &Value,
    invocation: &[&str],
) {
    cli.shutdown();
    seed_unkeyed(provider, subject);
    let subject: connectors_host::local::approvals::Subject =
        serde_json::from_value(subject.clone()).unwrap();
    success(cli.run(&[
        "connections",
        "revoke",
        "--adapter",
        "forge",
        "--connection",
        &subject.target.connection,
        "--expected-revision",
        &subject.target.connection_revision,
    ]));
    fs::remove_file(provider.root.path().join("private/proof.json")).unwrap();
    let original = fs::read_to_string(&cli.paths.config).unwrap();
    // An independent native instance admits the CLI startup after the original
    // target is removed. The old instance has no worker in this new owner.
    let native_path = provider.root.path().join("private/other-gitlab.json");
    let mut native: Value = serde_json::from_slice(&fs::read(&provider.config).unwrap()).unwrap();
    native["instance"] = json!("fixture-other");
    private(&native_path, &serde_json::to_vec(&native).unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-gitlab"))
        .args([
            "--local-config",
            native_path.to_str().unwrap(),
            "--print-local-bootstrap",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
    bootstrap.validate().unwrap();
    let adapter = provider.selection();
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let removed = format!(
        "{}{}\n[adapters.other]\ninstance_id='fixture-other'\nadapter_id='gitlab'\nconfiguration_revision={}\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.other.permissions]\nprofiles=['gitlab.pat']\noperations=['project.get']\n[adapters.other.executable]\npath={}\nsha256={}\nargs=['--local-config',{}]\n",
        original.split("[adapters.forge]").next().unwrap(),
        &original[original.find("[approval_clock]").unwrap()..],
        q(&bootstrap.configuration_revision),
        q(adapter.executable.path.to_str().unwrap()),
        q(&adapter.executable.sha256),
        q(native_path.to_str().unwrap()),
    );
    private(&cli.paths.config, removed.as_bytes());
    assert!(
        !connectors_host::local::config::Config::load(&cli.paths.config)
            .unwrap()
            .adapters
            .contains_key("forge")
    );
    clock.respond.store(false, Ordering::SeqCst);
    let clock_calls = clock.count.load(Ordering::SeqCst);
    let credential = provider.root.path().join("private/other-credential.json");
    private(&credential, &token(true).0);
    success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "other",
        "--profile",
        "gitlab.pat",
        "--credential-file",
        credential.to_str().unwrap(),
    ]));
    fs::remove_file(credential).unwrap();
    let status =
        success(cli.run(&["adapters", "status", "--adapter", "other"]))["observation"].clone();
    success(cli.run(&[
        "adapters",
        "stop",
        "--adapter",
        "other",
        "--expected-revision",
        status["configuration_revision"].as_str().unwrap(),
        "--host-incarnation",
        status["host_incarnation"].as_str().unwrap(),
        "--child-incarnation",
        status["child_incarnation"].as_str().unwrap(),
    ]));
    drop(custody.daemon.take());
    let native_calls = provider.count();
    let until = Instant::now() + Duration::from_secs(10);
    while clock.count.load(Ordering::SeqCst) == clock_calls {
        assert!(Instant::now() < until, "removed target was not scanned");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(states(cli)[0].1, "prepared");
    clock.respond.store(true, Ordering::SeqCst);
    await_state(cli, "aborted");
    assert_eq!(provider.count(), native_calls);
    assert_eq!(provider.merge_effects.load(Ordering::SeqCst), 0);
    private(&cli.paths.config, original.as_bytes());
    let refused = refusal(cli.run(invocation), "revoked");
    assert!(refused.get("mutation").is_none());
    assert!(cli.status()["child_incarnation"].is_null());
    assert_eq!(provider.count(), native_calls);
    cli.shutdown();
}

pub(super) fn run(
    cli: &Cli,
    provider: &Provider,
    custody: &mut Custody,
    clock: &Clock,
    subject: &Value,
    invocation: &[&str],
) {
    let process = owner_process(cli);
    let mut running = OwnedProcess(cli.command(invocation).spawn().unwrap());
    let until = Instant::now() + Duration::from_secs(10);
    while provider.merge_effects.load(Ordering::SeqCst) == 0 {
        assert!(running.0.try_wait().unwrap().is_none());
        assert!(Instant::now() < until, "provider did not receive merge");
        std::thread::sleep(Duration::from_millis(10));
    }
    // Now no foreground clock request remains. Seed a distinct unkeyed pending
    // attempt on the same instance to make a background pass require time.
    let clock_calls = clock.count.load(Ordering::SeqCst);
    seed_unkeyed(provider, subject);
    let until = Instant::now() + Duration::from_secs(8);
    while clock.count.load(Ordering::SeqCst) == clock_calls {
        assert!(running.0.try_wait().unwrap().is_none());
        assert!(Instant::now() < until, "background did not request time");
        std::thread::sleep(Duration::from_millis(10));
    }
    // Allow the independent signed response and queue handoff to finish while
    // the provider barrier still keeps the exact native exchange live.
    std::thread::sleep(Duration::from_millis(200));
    let pending = states(cli);
    assert_eq!(pending.len(), 2);
    assert!(
        pending
            .iter()
            .any(|row| row.1 == "prepared" && row.3.is_none())
    );
    assert!(
        pending
            .iter()
            .any(|row| row.1 == "dispatching" && row.3.as_deref() == Some("pending"))
    );
    assert!(running.0.try_wait().unwrap().is_none());

    kill_owner(process);
    provider.merge_mode.store(0, Ordering::SeqCst);
    refusal(finish_process(running), "outcome_unknown");
    fs::remove_file(provider.root.path().join("private/proof.json")).unwrap();
    clock.respond.store(false, Ordering::SeqCst);
    let subject: connectors_host::local::approvals::Subject =
        serde_json::from_value(subject.clone()).unwrap();
    // A separately admitted read starts the new owner. No caller resubmits the
    // original write/key to trigger recovery.
    cli.operation_result(
        &subject.target.connection,
        "project.get",
        json!({"project":"org/project"}),
    );
    let status = cli.status();
    success(cli.run(&[
        "adapters",
        "stop",
        "--adapter",
        "forge",
        "--expected-revision",
        status["configuration_revision"].as_str().unwrap(),
        "--host-incarnation",
        status["host_incarnation"].as_str().unwrap(),
        "--child-incarnation",
        status["child_incarnation"].as_str().unwrap(),
    ]));
    drop(custody.daemon.take());
    let native_calls = provider.count();
    await_state(cli, "indeterminate");
    let uncertain = states(cli);
    assert!(
        uncertain
            .iter()
            .any(|row| row.1 == "prepared" && row.3.is_none())
    );
    assert!(uncertain.iter().any(|row| row.1 == "indeterminate"
        && row.2.is_none()
        && row.3.as_deref() == Some("quarantined")));
    clock.respond.store(true, Ordering::SeqCst);
    await_state(cli, "aborted");
    assert_eq!(cli.status()["state"], "suppressed");
    assert_eq!(provider.count(), native_calls);
    assert_eq!(provider.merge_effects.load(Ordering::SeqCst), 1);
    assert_eq!(
        provider
            .methods
            .lock()
            .unwrap()
            .iter()
            .filter(|method| method.as_str() == "PUT")
            .count(),
        1
    );
    let replay = refusal(cli.run(invocation), "outcome_unknown");
    assert_eq!(replay["mutation"]["replayed"], true);
    assert_eq!(provider.count(), native_calls);
    cli.shutdown();
}

//! Preserved native lifecycle obligations through the shipped catalog provider.
use super::*;

#[test]
#[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
fn catalog_cli_failed_repair_and_busy_stop_preserve_authority() {
    let provider = Provider::new();
    let mut custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    fs::set_permissions(&credential, fs::Permissions::from_mode(0o644)).unwrap();
    let refused = refusal(
        cli.run(&[
            "connections",
            "connect",
            "--adapter",
            "gitlab",
            "--profile",
            "gitlab.pat",
            "--credential-file",
            credential.to_str().unwrap(),
        ]),
        "protected_entry_unavailable",
    );
    assert_eq!(refused["kind"], "usage");
    assert_eq!(provider.count(), 0);
    fs::set_permissions(&credential, fs::Permissions::from_mode(0o600)).unwrap();
    let mut command = cli.command(&[
        "connections",
        "connect",
        "--adapter",
        "gitlab",
        "--profile",
        "gitlab.pat",
        "--credential-stdin",
    ]);
    let mut child = command.stdin(Stdio::piped()).spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&token(true).0)
        .unwrap();
    let connected = success(child.wait_with_output().unwrap())["connection"].clone();
    let reference = connected["summary"]["connection"].as_str().unwrap();
    let revision = connected["summary"]["revision"].as_str().unwrap();
    private(&credential, &token(false).0);
    refusal(
        cli.run(&[
            "connections",
            "repair",
            "--adapter",
            "gitlab",
            "--connection",
            reference,
            "--expected-revision",
            revision,
            "--credential-file",
            credential.to_str().unwrap(),
        ]),
        "identity_mismatch",
    );
    let observed = success(cli.run(&[
        "connections",
        "status",
        "--adapter",
        "gitlab",
        "--connection",
        reference,
    ]));
    assert_eq!(observed["connection"]["summary"]["state"], "ready");
    assert_eq!(observed["connection"]["summary"]["revision"], revision);
    let describe = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "gitlab",
        "--operation",
        "project.get",
    ]));
    let schema = describe["schema"].as_str().unwrap();
    let descriptor = describe["revision"].as_str().unwrap();
    let invoke = [
        "operations",
        "invoke",
        "--adapter",
        "gitlab",
        "--connection",
        reference,
        "--operation",
        "project.get",
        "--schema",
        schema,
        "--revision",
        descriptor,
        "--input-json",
        r#"{"id":"org/project"}"#,
    ];
    success(cli.run(&invoke));
    let before = provider.count();
    let mut invalid = invoke;
    *invalid.last_mut().unwrap() = r#"{"id":"org/project","id":"org/project"}"#;
    refusal(cli.run(&invalid), "invalid_input");
    let mut stale = invoke;
    stale[9] = "old-schema";
    refusal(cli.run(&stale), "stale_description");
    let configuration = fs::read_to_string(&cli.paths.config).unwrap();
    private(
        &cli.paths.config,
        configuration
            .replace("operations=['project.get','issues.list']", "operations=[]")
            .as_bytes(),
    );
    refusal(cli.run(&invoke), "forbidden");
    private(&cli.paths.config, configuration.as_bytes());
    assert_eq!(provider.count(), before);
    // The production owner admits only its own executable image. Reach capture
    // through the real CLI, which calls Begin before reading protected stdin.
    let prior_acquisitions = cli.recorded().pending_acquisitions();
    let mut capture = OwnedProcess(
        cli.command(&[
            "connections",
            "connect",
            "--adapter",
            "gitlab",
            "--profile",
            "gitlab.pat",
            "--credential-stdin",
        ])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap(),
    );
    let capture_deadline = Instant::now() + Duration::from_secs(40);
    loop {
        let pending = cli.recorded().pending_acquisitions();
        let added: Vec<_> = pending.difference(&prior_acquisitions).collect();
        if !added.is_empty() {
            assert_eq!(added.len(), 1);
            break;
        }
        assert!(
            capture.0.try_wait().unwrap().is_none(),
            "capture CLI exited before protected entry"
        );
        assert!(
            Instant::now() < capture_deadline,
            "capture was not admitted"
        );
        std::thread::sleep(Duration::from_millis(30));
    }
    let coordinates = cli.status();
    provider
        .pause
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let busy = cli.command(&invoke).spawn().unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while provider.count() == before {
        assert!(Instant::now() < until, "fixture read was not dispatched");
        std::thread::sleep(Duration::from_millis(10));
    }
    // Observation and stop must not wait behind the provider's blocked channel.
    assert_eq!(cli.status()["state"], "ready");
    let start = Instant::now();
    success(cli.run(&[
        "adapters",
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
    assert!(start.elapsed() < Duration::from_secs(6));
    refusal(busy.wait_with_output().unwrap(), "unavailable");
    capture
        .0
        .stdin
        .take()
        .unwrap()
        .write_all(&token(true).0)
        .unwrap();
    refusal(guarded_merge::finish_process(capture), "lifecycle_conflict");
    assert_eq!(cli.status()["state"], "suppressed");
    assert_eq!(provider.count(), before + 1);
    cli.shutdown();
    drop(custody.daemon.take());
    refusal(cli.run(&invoke), "custody_unavailable");
    assert!(!cli.paths.state.join("owner.sock").exists());
    let database = fs::read(cli.paths.state.join("metadata.sqlite3")).unwrap();
    assert!(
        !database
            .windows(b"fixture-pat".len())
            .any(|w| w == b"fixture-pat")
    );
}
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn catalog_cli_explicit_revalidation_after_real_expiry_without_reentry() {
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
    ]))["connection"]
        .clone();
    let reference = connected["summary"]["connection"].as_str().unwrap();
    let revision = connected["summary"]["revision"].as_str().unwrap();
    let revalidate = [
        "connections",
        "revalidate",
        "--adapter",
        "gitlab",
        "--connection",
        reference,
        "--expected-revision",
        revision,
    ];
    let status = [
        "connections",
        "status",
        "--adapter",
        "gitlab",
        "--connection",
        reference,
    ];
    let descriptor = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "gitlab",
        "--operation",
        "project.get",
    ]));
    let invoke = [
        "operations",
        "invoke",
        "--adapter",
        "gitlab",
        "--connection",
        reference,
        "--operation",
        "project.get",
        "--schema",
        descriptor["schema"].as_str().unwrap(),
        "--revision",
        descriptor["revision"].as_str().unwrap(),
        "--input-json",
        r#"{"id":"org/project"}"#,
    ];
    fs::remove_file(&credential).unwrap();
    cli.shutdown();
    let until = connected["valid_until_ms"].as_u64().unwrap();
    while connectors_sdk::now_ms() <= until {
        std::thread::sleep(Duration::from_millis(100));
    }
    let before = provider.count();
    assert_eq!(
        success(cli.run(&status))["connection"]["summary"]["state"],
        "pending"
    );
    refusal(cli.run(&invoke), "not_granted");
    assert_eq!(provider.count(), before);
    assert!(!cli.paths.state.join("owner.sock").exists());
    let refreshed = success(cli.run(&revalidate))["connection"].clone();
    assert_eq!(refreshed["summary"], connected["summary"]);
    assert!(refreshed["valid_until_ms"].as_u64().unwrap() > until);
    assert_eq!(provider.count(), before + 2);
    let result = success(cli.run(&invoke));
    let result: Value = serde_json::from_str(result["result"].as_str().unwrap()).unwrap();
    assert_eq!(result["body"]["name"], "fixture-project");
    // Upstream unavailability is not positive invalidity of the retained token.
    provider
        .response_status
        .store(503, std::sync::atomic::Ordering::SeqCst);
    assert!(!cli.run(&revalidate).status.success());
    assert_eq!(
        success(cli.run(&status))["connection"]["summary"]["state"],
        "ready"
    );
    provider
        .response_status
        .store(0, std::sync::atomic::Ordering::SeqCst);
    success(cli.run(&invoke));
    provider
        .response_status
        .store(401, std::sync::atomic::Ordering::SeqCst);
    refusal(cli.run(&revalidate), "service_failure");
    assert_eq!(
        success(cli.run(&status))["connection"]["summary"]["state"],
        "reauthorization_required"
    );
    cli.shutdown();
    let before = provider.count();
    refusal(cli.run(&revalidate), "not_granted");
    assert_eq!(provider.count(), before);
    assert!(!cli.paths.state.join("owner.sock").exists());
}

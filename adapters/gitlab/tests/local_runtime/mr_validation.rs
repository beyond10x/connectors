use super::*;

#[test]
#[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
fn gitlab_cli_validation_changed_head_after_saved_connection_restart() {
    let provider = Provider::new();
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let config = fs::read_to_string(&cli.paths.config).unwrap();
    private(
        &cli.paths.config,
        config
            .replace(
                "operations=['project.get','issues.list','file.get']",
                "operations=['project.get','issues.list','file.get','merge_request.validate']",
            )
            .as_bytes(),
    );
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "forge",
        "--profile",
        "gitlab.pat",
        "--credential-file",
        credential.to_str().unwrap(),
    ]));
    let connection = connected["connection"]["summary"]["connection"]
        .as_str()
        .unwrap();
    fs::remove_file(&credential).unwrap();
    cli.shutdown();
    let input = json!({"project":"org/project","iid":3,
        "sha":"0123456789abcdef0123456789abcdef01234567","pipeline_id":12});
    let first = cli.operation_result(connection, "merge_request.validate", input.clone());
    assert_eq!(first["item"]["checks_passed"], true);
    assert_eq!(first["item"]["merge_performed"], false);
    let changed = cli.operation_result(connection, "merge_request.validate", input.clone());
    assert_eq!(changed["item"]["checks_passed"], false);
    assert_eq!(changed["item"]["blockers"], json!(["head_changed"]));
    assert_eq!(changed["item"]["merge_performed"], false);
    let mut missing = input.clone();
    missing["iid"] = json!(2);
    let missing = cli.operation_result(connection, "merge_request.validate", missing);
    assert_eq!(missing["item"]["checks_passed"], false);
    assert_eq!(
        missing["item"]["blockers"],
        json!([
            "head_unavailable",
            "merge_checks_pending",
            "pipeline_unavailable"
        ])
    );
    let before = provider.count();
    let mut wrong = input.clone();
    wrong["sha"] = json!("main");
    refusal(
        cli.operation_invoke(connection, "merge_request.validate", wrong),
        "invalid_input",
    );
    let mut wrong = input.clone();
    wrong["project"] = json!("outside");
    refusal(
        cli.operation_invoke(connection, "merge_request.validate", wrong),
        "forbidden",
    );
    assert_eq!(provider.count(), before);
    let description = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "forge",
        "--operation",
        "merge_request.validate",
    ]));
    let config = fs::read_to_string(&cli.paths.config).unwrap();
    private(
        &cli.paths.config,
        config.replace(",'merge_request.validate'", "").as_bytes(),
    );
    refusal(
        cli.run(&[
            "operations",
            "invoke",
            "--adapter",
            "forge",
            "--connection",
            connection,
            "--operation",
            "merge_request.validate",
            "--schema",
            description["schema"].as_str().unwrap(),
            "--revision",
            description["revision"].as_str().unwrap(),
            "--input-json",
            &input.to_string(),
        ]),
        "forbidden",
    );
    assert_eq!(provider.count(), before);
    let calls = provider.calls.lock().unwrap();
    assert_eq!(
        calls
            .iter()
            .filter(|p| p.split('?').next() == Some("/api/v4/user"))
            .count(),
        1
    );
    let reads: Vec<_> = calls
        .iter()
        .filter(|p| p.contains("/merge_requests/"))
        .collect();
    assert_eq!(reads.len(), 3);
    assert!(
        reads
            .iter()
            .all(|p| p.split_once('?').is_none_or(|(_, query)| query.is_empty()))
    );
    drop(calls);
    assert!(
        provider
            .methods
            .lock()
            .unwrap()
            .iter()
            .all(|method| method == "GET")
    );
    cli.shutdown();
    assert!(!cli.paths.state.join("owner.sock").exists());
}

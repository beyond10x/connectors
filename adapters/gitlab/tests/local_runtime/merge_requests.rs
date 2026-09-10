use super::*;
#[test]
#[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
fn gitlab_cli_mr_window_exhaustion_saved_connection_and_current_policy() {
    let provider = Provider::new();
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let config = fs::read_to_string(&cli.paths.config).unwrap();
    private(&cli.paths.config,config.replace("operations=['project.get','issues.list','file.get']","operations=['project.get','issues.list','file.get','merge_request.get','merge_requests.list']").as_bytes());
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
    let renew = || {
        success(
            cli.run(&[
                "connections",
                "revalidate",
                "--adapter",
                "forge",
                "--connection",
                connection,
                "--expected-revision",
                connected["connection"]["summary"]["revision"]
                    .as_str()
                    .unwrap(),
            ]),
        );
    };
    fs::remove_file(&credential).unwrap();
    cli.shutdown();
    let input = json!({"project":"org/project","state":"all","updated_after":"2026-09-01T00:00:00Z","updated_before":"2026-09-10T00:00:00Z","limit":1});
    let first = cli.operation_result(connection, "merge_requests.list", input.clone());
    assert_eq!(first["complete"], false);
    assert_eq!(first["items"][0]["iid"], 1);
    let mut next = input.clone();
    next["cursor"] = first["next_cursor"].clone();
    let last = cli.operation_result(connection, "merge_requests.list", next);
    assert_eq!(last["complete"], true);
    assert_eq!(last["next_cursor"], Value::Null);
    let item = cli.operation_result(
        connection,
        "merge_request.get",
        json!({"project":"org/project","iid":2}),
    );
    assert_eq!(item["item"]["iid"], 2);
    assert_eq!(item["item"]["source_project_id"], Value::Null);
    assert_eq!(item["item"]["sha"], Value::Null);
    assert_eq!(item["item"]["detailed_merge_status"], "future_status");
    renew();
    let before = provider.count();
    let mut bad = input.clone();
    bad["updated_after"] = json!("2026-02-30T00:00:00Z");
    refusal(
        cli.operation_invoke(connection, "merge_requests.list", bad),
        "invalid_input",
    );
    let mut stale = input.clone();
    stale["updated_before"] = json!("2026-09-09T00:00:00Z");
    stale["cursor"] = first["next_cursor"].clone();
    let error = refusal(
        cli.operation_invoke(connection, "merge_requests.list", stale),
        "service_failure",
    );
    assert_eq!(error["service_code"], "stale_cursor");
    refusal(
        cli.operation_invoke(
            connection,
            "merge_request.get",
            json!({"project":"outside","iid":1}),
        ),
        "forbidden",
    );
    assert_eq!(provider.count(), before);
    renew();
    let error = refusal(
        cli.operation_invoke(
            connection,
            "merge_request.get",
            json!({"project":"org/project","iid":999}),
        ),
        "service_failure",
    );
    assert_eq!(error["service_code"], "not_found");
    let description = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "forge",
        "--operation",
        "merge_request.get",
    ]));
    let config = fs::read_to_string(&cli.paths.config).unwrap();
    private(
        &cli.paths.config,
        config.replace("'merge_request.get',", "").as_bytes(),
    );
    let before = provider.count();
    refusal(
        cli.run(&[
            "operations",
            "invoke",
            "--adapter",
            "forge",
            "--connection",
            connection,
            "--operation",
            "merge_request.get",
            "--schema",
            description["schema"].as_str().unwrap(),
            "--revision",
            description["revision"].as_str().unwrap(),
            "--input-json",
            &json!({"project":"org/project","iid":1}).to_string(),
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
        3
    );
    let lists: Vec<_> = calls
        .iter()
        .filter(|p| p.contains("/merge_requests?"))
        .collect();
    assert_eq!(lists.len(), 2);
    assert!(lists.iter().all(|p| p.contains("scope=all")
        && p.contains("order_by=updated_at")
        && p.contains("sort=asc")
        && p.contains("updated_after=2026-09-01T00%3A00%3A00Z")
        && p.contains("updated_before=2026-09-10T00%3A00%3A00Z")));
    drop(calls);
    cli.shutdown();
    assert!(!cli.paths.state.join("owner.sock").exists());
}

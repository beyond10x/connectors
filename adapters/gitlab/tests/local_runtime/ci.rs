use super::*;
use crate::ci_provider::SHA;

#[track_caller]
fn service_refusal(output: Output, code: &str) {
    let error = refusal(output, "service_failure");
    assert_eq!(error["service_code"], code);
    assert_eq!(error["stage"], "dispatch");
}

#[test]
#[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
fn gitlab_cli_exact_commit_ci_job_pages_and_bounded_traces() {
    let provider = Provider::new();
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let old = fs::read_to_string(&cli.paths.config).unwrap();
    private(&cli.paths.config,old.replace("operations=['project.get','issues.list','file.get']",
        "operations=['project.get','issues.list','file.get','pipelines.list','pipeline.get','pipeline.jobs','job.get','job.trace']").as_bytes());
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

    let before = provider.count();
    refusal(
        cli.operation_invoke(
            connection,
            "pipelines.list",
            json!({"project":"org/project","sha":"main","limit":10}),
        ),
        "invalid_input",
    );
    assert_eq!(provider.count(), before);
    let listed = cli.operation_result(
        connection,
        "pipelines.list",
        json!({"project":"org/project","sha":SHA,"limit":10}),
    );
    assert_eq!(listed["items"][0]["id"], 11);
    assert_eq!(listed["provenance"]["source_revision"], SHA);
    for state in ["pending", "running", "failed"] {
        let p = cli.operation_result(
            connection,
            "pipeline.get",
            json!({"project":"org/project","pipeline_id":11,"sha":SHA}),
        );
        assert_eq!(p["item"]["status"], state);
    }
    service_refusal(
        cli.operation_invoke(
            connection,
            "pipeline.get",
            json!({"project":"org/project","pipeline_id":12,"sha":SHA}),
        ),
        "upstream_protocol",
    );
    renew(); // Explicit workflow action; business reads never revalidate implicitly.
    let mut input = json!({"project":"org/project","pipeline_id":11,"sha":SHA,"limit":1});
    let page = cli.operation_result(connection, "pipeline.jobs", input.clone());
    assert_eq!(page["complete"], false);
    assert_eq!(page["items"][0]["id"], 42);
    assert_eq!(page["items"][0]["status"], "failed");
    input["cursor"] = page["next_cursor"].clone();
    let before = provider.count();
    let mut wrong = input.clone();
    wrong["sha"] = json!("f".repeat(40));
    service_refusal(
        cli.operation_invoke(connection, "pipeline.jobs", wrong),
        "stale_cursor",
    );
    assert_eq!(provider.count(), before);
    let page = cli.operation_result(connection, "pipeline.jobs", input);
    assert_eq!(page["items"][0]["id"], 41);
    assert_eq!(page["complete"], true);
    let j = cli.operation_result(
        connection,
        "job.get",
        json!({"project":"org/project","job_id":42,"pipeline_id":11,"sha":SHA}),
    );
    assert_eq!(j["item"]["status"], "failed");
    renew();
    let trace = cli.operation_result(
        connection,
        "job.trace",
        json!({"project":"org/project","job_id":42,"max_bytes":100}),
    );
    assert_eq!(trace["item"]["content"], "test failed: €\n");
    assert_eq!(trace["item"]["complete"], true);
    let trace = cli.operation_result(
        connection,
        "job.trace",
        json!({"project":"org/project","job_id":42,"max_bytes":8}),
    );
    assert_eq!(trace["item"]["content"], "test fai");
    assert_eq!(trace["item"]["complete"], false);
    let trace = cli.operation_result(
        connection,
        "job.trace",
        json!({"project":"org/project","job_id":41,"max_bytes":512000}),
    );
    assert_eq!(trace["item"]["bytes"], 511999);
    assert_eq!(trace["item"]["complete"], false);
    service_refusal(
        cli.operation_invoke(
            connection,
            "job.trace",
            json!({"project":"org/project","job_id":999,"max_bytes":100}),
        ),
        "not_found",
    );

    renew();
    provider
        .response_status
        .store(503, std::sync::atomic::Ordering::SeqCst);
    refusal(
        cli.operation_invoke(
            connection,
            "pipeline.get",
            json!({"project":"org/project","pipeline_id":11,"sha":SHA}),
        ),
        "unavailable",
    );
    provider
        .response_status
        .store(429, std::sync::atomic::Ordering::SeqCst);
    service_refusal(
        cli.operation_invoke(
            connection,
            "pipeline.get",
            json!({"project":"org/project","pipeline_id":11,"sha":SHA}),
        ),
        "rate_limited",
    );
    provider
        .response_status
        .store(0, std::sync::atomic::Ordering::SeqCst);
    let calls = provider.calls.lock().unwrap().clone();
    assert_eq!(
        calls
            .iter()
            .filter(|p| p.split('?').next() == Some("/api/v4/user"))
            .count(),
        4 // Initial connect plus the three explicit saved-credential revalidations.
    );
    assert!(calls.iter().any(|p| p.contains(&format!("sha={SHA}"))));
    assert_eq!(
        calls
            .iter()
            .filter(|p| p.split('?').next().unwrap().ends_with("/jobs/42/trace"))
            .count(),
        2
    );

    // Removing policy authority refuses without another provider request.
    let description = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "forge",
        "--operation",
        "pipeline.get",
    ]));
    let config = fs::read_to_string(&cli.paths.config).unwrap();
    private(
        &cli.paths.config,
        config.replace("'pipeline.get',", "").as_bytes(),
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
            "pipeline.get",
            "--schema",
            description["schema"].as_str().unwrap(),
            "--revision",
            description["revision"].as_str().unwrap(),
            "--input-json",
            &json!({"project":"org/project","pipeline_id":11,"sha":SHA}).to_string(),
        ]),
        "forbidden",
    );
    assert_eq!(provider.count(), before);
    cli.shutdown();
    assert!(!cli.paths.state.join("owner.sock").exists());
}

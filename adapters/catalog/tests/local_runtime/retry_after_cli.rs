//! story:catalog-honours-retry-after through the production CLI: the
//! acceptance names what the caller sees, so the delay is followed from the
//! provider's `Retry-After` through the owner's supervisor to the CLI's JSON.
use super::*;

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_rate_limited_read_is_waited_for_once_or_states_its_delay_in_the_cli_json() {
    let provider = Provider::new();
    let custody = Custody::new(provider.root.path());
    let cli = Cli::new(provider.root.path());
    configure(&cli, &provider, &custody);
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    let reference = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "gitlab",
        "--profile",
        "gitlab.pat",
        "--credential-file",
        credential.to_str().unwrap(),
    ]))["connection"]["summary"]["connection"]
        .as_str()
        .unwrap()
        .to_owned();
    let description = success(cli.run(&[
        "operations",
        "describe",
        "--adapter",
        "gitlab",
        "--operation",
        "project.get",
    ]));
    let schema = description["schema"].as_str().unwrap().to_owned();
    let descriptor = description["revision"].as_str().unwrap().to_owned();
    let read = |project: &str| {
        let input = json!({ "id": format!("org/{project}") }).to_string();
        cli.run(&[
            "operations",
            "invoke",
            "--adapter",
            "gitlab",
            "--connection",
            &reference,
            "--operation",
            "project.get",
            "--schema",
            &schema,
            "--revision",
            &descriptor,
            "--input-json",
            &input,
        ])
    };
    let requests = |project: &str| {
        let route = format!("/api/v4/projects/org%2F{project}");
        provider
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|path| path.split('?').next() == Some(route.as_str()))
            .count()
    };

    // A `Retry-After: 1` that fits the deadline: one successful read, two requests.
    let value = success(read("fixture-busy-once"));
    assert!(value.to_string().contains("fixture-project"), "{value}");
    assert_eq!(requests("fixture-busy-once"), 2);

    // Beyond the deadline: refused after one request, stating the delay.
    let long = refused_data(read("fixture-busy-long"));
    assert_eq!(long["code"], "service_failure", "{long}");
    assert_eq!(long["stage"], "dispatch", "{long}");
    assert_eq!(long["service_code"], "rate_limited", "{long}");
    assert_eq!(long["retry_after_seconds"], 3600, "{long}");
    assert_eq!(requests("fixture-busy-long"), 1);

    // Unreadable or absent: refused after one request, stating no delay.
    for project in ["fixture-busy-garbled", "fixture-busy-bare"] {
        let data = refused_data(read(project));
        assert_eq!(data["service_code"], "rate_limited", "{project}: {data}");
        assert!(
            data.get("retry_after_seconds").is_none(),
            "{project}: {data}"
        );
        assert_eq!(requests(project), 1, "{project}");
    }
}

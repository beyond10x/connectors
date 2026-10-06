//! Adversary case for story:service-failure-carries-upstream-reason: the
//! acceptance names the CLI's JSON output, and no other case follows a
//! provider's reason through the owner's supervisor to it.
use super::*;

#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_provider_reason_reaches_the_cli_failure_json_and_a_credential_echo_does_not() {
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
        let input = json!({ "id": project }).to_string();
        refused_data(cli.run(&[
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
        ]))
    };

    let refused = read("org/fixture-refused");
    assert_eq!(refused["code"], "forbidden", "{refused}");
    assert_eq!(refused["stage"], "dispatch", "{refused}");
    assert_eq!(refused["service_reason"], "403 Forbidden", "{refused}");

    let echo = read("org/fixture-echo");
    assert_eq!(echo["service_code"], "unauthorized", "{echo}");
    assert!(echo.get("service_reason").is_none(), "{echo}");

    let leaky = read("org/fixture-leaky");
    assert_eq!(leaky["service_code"], "unauthorized", "{leaky}");
    assert!(leaky.get("service_reason").is_none(), "{leaky}");

    let scope = read("org/fixture-scope");
    assert_eq!(scope["code"], "service_failure", "{scope}");
    assert_eq!(scope["stage"], "dispatch", "{scope}");
    assert_eq!(scope["service_code"], "unauthorized", "{scope}");
    assert_eq!(
        scope["service_reason"], "Unauthorized; scope does not match",
        "{scope}"
    );
}

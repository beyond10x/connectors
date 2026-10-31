//! story:connectors-do-short-invocation: the explicit endpoint routes
//! (`describe`, `invoke --endpoint`, `serve`) and their compatibility
//! inventory are removed. Root help lists the generated tree and the native
//! server only, and an endpoint command line is a parse refusal that sends no
//! request.

use std::{
    net::TcpListener,
    process::{Command, Output},
    time::{Duration, Instant},
};

fn cli(args: &[&str]) -> Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(args)
        .env("HOME", home.path())
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_STATE_HOME")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap()
}

fn parts(output: &Output) -> (Option<i32>, String, String) {
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn version_flags_print_name_and_workspace_version_and_succeed() {
    let expected = format!("connectors {}\n", env!("CARGO_PKG_VERSION"));
    for flag in ["--version", "-V"] {
        let output = cli(&[flag]);
        assert_eq!(
            (
                output.status.code(),
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap(),
            ),
            (Some(0), expected.clone(), String::new()),
            "`connectors {flag}` must print the name and version and exit 0",
        );
    }
}

/// story:connectors-do-short-invocation: root help names no explicit service
/// command; `invoke` is the generated command, and `describe` and `serve` are
/// parse refusals under `--help` as anywhere.
#[test]
fn root_help_names_no_explicit_service_command() {
    assert!(
        !std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/spec/compatibility.json"
        ))
        .exists(),
        "story:connectors-do-short-invocation: the compatibility inventory is removed"
    );
    let root = cli(&["--help"]);
    assert!(root.status.success());
    let root_help = String::from_utf8(root.stdout).unwrap();
    assert!(
        !root_help.contains("Explicit service commands")
            && !root_help.contains("  describe")
            && !root_help.contains("  serve "),
        "story:connectors-do-short-invocation: {root_help}"
    );
    assert!(
        root_help.contains("  invoke "),
        "story:connectors-do-short-invocation: {root_help}"
    );
    let (code, help, stderr) = parts(&cli(&["invoke", "--help"]));
    assert_eq!((code, stderr.as_str()), (Some(0), ""));
    assert!(
        help.contains("CONNECTOR[/INSTANCE]")
            && !help.contains("--endpoint")
            && !help.contains("--token-file"),
        "story:connectors-do-short-invocation: {help}"
    );
    for name in ["describe", "serve"] {
        assert_eq!(
            parts(&cli(&[name, "--help"])),
            (Some(2), String::new(), "cli_parse\n".to_owned()),
            "story:connectors-do-short-invocation: `{name} --help`"
        );
    }
    // The production grouped setup is unaffected.
    // Use an isolated missing configuration instead of the operator's defaults.
    let temp = tempfile::tempdir().unwrap();
    assert_eq!(
        cli(&[
            "--config",
            temp.path().join("missing.toml").to_str().unwrap(),
            "setup",
            "check"
        ])
        .status
        .code(),
        Some(2)
    );
}

/// story:connectors-do-short-invocation: `describe --endpoint` and `invoke
/// --endpoint` are parse refusals, and the endpoint they name receives no
/// request.
#[test]
fn endpoint_routes_are_parse_refusals_that_send_no_request() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/", listener.local_addr().unwrap());
    let temp = tempfile::tempdir().unwrap();
    let token = temp.path().join("service-token");
    std::fs::write(&token, "fictional-service-token").unwrap();
    let input = temp.path().join("input.json");
    std::fs::write(&input, r#"{"selector":"fixture"}"#).unwrap();
    let common = [
        "--endpoint",
        &endpoint,
        "--allow-plaintext",
        "--token-file",
        token.to_str().unwrap(),
    ];
    let described = cli(&[&["describe"][..], &common].concat());
    let invoked = cli(&[
        &["invoke"][..],
        &common,
        &["--operation", "read", "--input", input.to_str().unwrap()],
    ]
    .concat());
    for (name, output) in [("describe", described), ("invoke", invoked)] {
        assert_eq!(
            parts(&output),
            (Some(2), String::new(), "cli_parse\n".to_owned()),
            "story:connectors-do-short-invocation: `{name} --endpoint` is a parse refusal"
        );
    }
    let deadline = Instant::now() + Duration::from_millis(200);
    while Instant::now() < deadline {
        match listener.accept() {
            Ok(_) => panic!("story:connectors-do-short-invocation: the endpoint was contacted"),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("listener: {error}"),
        }
    }
}

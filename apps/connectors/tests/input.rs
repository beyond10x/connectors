//! story:connectors-do-short-invocation: the explicit service route that
//! refused a repeated input key before contacting the service is removed. Its
//! command line is a parse refusal that contacts nothing; `invoke` refuses a
//! repeated key itself (`invoke_short.rs`,
//! `repeated_input_keys_are_refused_before_any_provider_call`).
use std::{
    net::TcpListener,
    time::{Duration, Instant},
};

#[test]
fn the_endpoint_invoke_route_is_a_parse_refusal_that_contacts_nothing() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input.json");
    for bytes in [r#"{"x":1,"x":2}"#, r#"{"nested":{"x":1,"x":2}}"#] {
        std::fs::write(&input, bytes).unwrap();
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_connectors"))
            .args([
                "invoke",
                "--endpoint",
                &endpoint,
                "--allow-plaintext",
                "--operation",
                "read",
                "--token-file",
            ])
            .arg(temp.path().join("absent-token"))
            .arg("--input")
            .arg(&input)
            .env("HOME", temp.path())
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("XDG_STATE_HOME")
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert_eq!(
            (
                output.status.code(),
                String::from_utf8_lossy(&output.stdout).into_owned(),
                String::from_utf8_lossy(&output.stderr).into_owned()
            ),
            (Some(2), String::new(), "cli_parse\n".to_owned()),
            "story:connectors-do-short-invocation: the endpoint route is a parse refusal"
        );
    }
    let deadline = Instant::now() + Duration::from_millis(200);
    while Instant::now() < deadline {
        match listener.accept() {
            Ok(_) => panic!("story:connectors-do-short-invocation: the service was contacted"),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("listener: {error}"),
        }
    }
}

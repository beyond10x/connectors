//! Existing explicit endpoint routes keep their parser and raw JSON codecs.

use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    process::{Command, Output},
    time::{Duration, Instant},
};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn compatibility_inventory_matches_existing_help() {
    let inventory: Value =
        serde_json::from_str(include_str!("../spec/compatibility.json")).unwrap();
    assert_eq!(inventory["format"], "connectors.cli-compatibility/v1");
    let root = cli(&["--help"]);
    assert!(root.status.success());
    let root_help = String::from_utf8(root.stdout).unwrap();
    for route in inventory["routes"].as_array().unwrap() {
        let name = route["path"][0].as_str().unwrap();
        assert!(root_help.contains(&format!("  {name}")));
        assert_eq!(route["alias_of_grouped_command"], false);
        let output = cli(&[name, "--help"]);
        assert!(output.status.success());
        let help = String::from_utf8(output.stdout).unwrap();
        for flag in route["flags"].as_array().unwrap() {
            assert!(help.contains(&format!("--{}", flag.as_str().unwrap())));
        }
    }
    // The production grouped setup is separate from the explicit endpoint routes.
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

#[test]
fn legacy_describe_is_complete_and_invoke_prints_raw_result_once() {
    let descriptor = json!({
        "version": connectors_core::WIRE_VERSION,
        "instance": "fixture-instance", "adapter": "fixture-adapter", "revision": "r1",
        "operations": [{"id":"read", "description":"Fixture read", "contract":"operations/v1alpha1", "profile":"fixture", "input_schema":{"type":"object"}, "output_schema":{"type":"array"}}],
        "configuration_schema": {"type":"object", "properties":{"selected":{"type":"string"}}}
    });
    let result = json!([{"id":"fixture-result"}, 7]);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/", listener.local_addr().unwrap());
    let response_descriptor = descriptor.clone();
    let response_result = result.clone();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut requests = Vec::new();
        while requests.len() < 3 {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "CLI did not issue its expected requests"
                    );
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(error) => panic!("fixture accept: {error}"),
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(&mut stream);
            let mut first = String::new();
            reader.read_line(&mut first).unwrap();
            let mut length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header == "\r\n" || header.is_empty() {
                    break;
                }
                if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse::<usize>().unwrap();
                }
            }
            assert!(length < 65536);
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            drop(reader);
            let response = if first.starts_with("GET /v1/describe ") {
                requests.push("describe");
                response_descriptor.clone()
            } else {
                assert!(first.starts_with("POST /v1/invoke "));
                requests.push("invoke");
                let request: Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(request["revision"], "r1");
                assert_eq!(request["operation"], "read");
                assert_eq!(request["input"], json!({"selector":"fixture"}));
                json!({"version":connectors_core::WIRE_VERSION,"request_id":request["request_id"],"status":"success","result":response_result})
            };
            let bytes = serde_json::to_vec(&response).unwrap();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", bytes.len()).unwrap();
            stream.write_all(&bytes).unwrap();
        }
        requests
    });
    let temp = tempfile::tempdir().unwrap();
    let token = temp.path().join("service-token");
    std::fs::write(&token, "fictional-service-token").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
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
    assert!(
        described.status.success(),
        "{}",
        String::from_utf8_lossy(&described.stderr)
    );
    assert!(
        invoked.status.success(),
        "{}",
        String::from_utf8_lossy(&invoked.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&described.stdout).unwrap(),
        descriptor
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&invoked.stdout).unwrap(),
        result
    );
    assert_eq!(server.join().unwrap(), ["describe", "describe", "invoke"]);
}

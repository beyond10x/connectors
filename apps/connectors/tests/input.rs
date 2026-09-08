#[test]
fn duplicate_input_keys_are_refused_before_service_contact() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input.json");
    for bytes in [r#"{"x":1,"x":2}"#, r#"{"nested":{"x":1,"x":2}}"#] {
        std::fs::write(&input, bytes).unwrap();
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_connectors"))
            .args([
                "invoke",
                "--endpoint",
                "http://127.0.0.1:1",
                "--allow-plaintext",
                "--operation",
                "read",
                "--token-file",
            ])
            .arg(temp.path().join("absent-token"))
            .arg("--input")
            .arg(&input)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["code"], "invalid_input");
        assert_eq!(error["message"], "invalid JSON input");
    }
}

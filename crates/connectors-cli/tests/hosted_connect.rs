//! Hosted setup selects the existing Identity session without reading local configuration.

use std::process::Command;

#[test]
fn hosted_setup_help_names_the_existing_target_profile_and_private_file() {
    let output = Command::new(env!("CARGO_BIN_EXE_connectors"))
        .args(["setup", "connect", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "--target",
        "--auth-profile",
        "--credential-file",
        "[default: local]",
    ] {
        assert!(help.contains(expected), "missing {expected}");
    }
}

#[test]
fn incompatible_profiles_refuse_before_identity_or_credential_file_access() {
    for (provider, profile) in [
        ("slack", "slack.companion_bot"),
        ("slack", "slack.user_token"),
        ("gitlab", "gitlab.oauth_token"),
        ("jira", "jira.api_token"),
        ("grafana", "grafana.unknown"),
        ("grafana", "anthropic.api_key"),
        ("unknown", "unknown.token"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors"))
            .args([
                "--output",
                "json",
                "setup",
                "connect",
                provider,
                "--target",
                "hosted",
                "--auth-profile",
                profile,
                "--credential-file",
                "/missing/credential",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("one secret entry"), "{text}");
        assert!(!text.contains("hosted-login-required"));
    }
}

#[test]
fn hosted_setup_rejects_local_options_and_missing_inputs_before_identity_or_file_access() {
    for arguments in [
        vec!["--config", "/missing/config"],
        vec!["--state-root", "/missing/state"],
        vec!["--set", "origin=https://foreign.example"],
        vec!["--operator-network"],
        vec!["--allow", "writes"],
        vec!["--instance", "another"],
        vec!["--as", "api_key"],
        vec!["--context", "another"],
        vec!["--instruction-file", "/missing/instructions"],
        vec![],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors"))
            .args([
                "--output", "json", "setup", "connect", "grafana", "--target", "hosted",
            ])
            .args(arguments)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let text = value.to_string();
        assert!(text.contains("invalid-argument") || text.contains("target-conflict"));
        assert!(!text.contains("hosted-login-required"));
    }
}

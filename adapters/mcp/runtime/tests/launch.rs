use clap::{CommandFactory, Parser};
use connectors_mcp::launch;

#[derive(Parser)]
#[command(name = "connectors", disable_help_subcommand = true)]
struct Selectors {
    #[arg(long, global = true)]
    config: Option<std::path::PathBuf>,
    #[arg(long = "state-dir", global = true)]
    state: Option<std::path::PathBuf>,
}

fn command() -> clap::Command {
    Selectors::command()
        .subcommand_required(true)
        .subcommand(launch::command())
}

fn parse(args: &[&str]) -> Result<launch::Input, clap::Error> {
    let matches = command().try_get_matches_from(args)?;
    launch::input(matches.subcommand_matches("server").unwrap())
}

#[test]
fn native_launch_accepts_only_selected_transport_with_process_selectors() {
    for args in [
        vec!["connectors", "server", "--transport", "stdio"],
        vec![
            "connectors",
            "--config",
            "/private/config",
            "--state-dir",
            "/private/state",
            "server",
            "--transport",
            "stdio",
        ],
        vec![
            "connectors",
            "server",
            "--transport=stdio",
            "--config=/private/config",
            "--state-dir=/private/state",
        ],
    ] {
        let input = parse(&args).unwrap();
        assert_eq!(
            serde_json::to_value(input).unwrap(),
            serde_json::json!({"transport": "stdio"})
        );
    }
}

#[test]
fn finite_output_and_protected_source_flags_never_enter_launch_grammar() {
    for forbidden in ["--output", "--credential-stdin", "--password", "--token"] {
        for args in [
            vec![
                "connectors",
                forbidden,
                "json",
                "server",
                "--transport",
                "stdio",
            ],
            vec![
                "connectors",
                "server",
                "--transport",
                "stdio",
                forbidden,
                "json",
            ],
        ] {
            assert!(parse(&args).is_err(), "unexpected acceptance: {args:?}");
        }
    }
}

#[test]
fn absent_unknown_duplicate_and_positional_transport_are_refused() {
    for args in [
        vec!["connectors", "server"],
        vec!["connectors", "server", "--transport", "http"],
        vec!["connectors", "server", "--transport", "STDIO"],
        vec!["connectors", "server", "--transport", ""],
        vec!["connectors", "server", "stdio"],
        vec![
            "connectors",
            "server",
            "--transport",
            "stdio",
            "--transport",
            "stdio",
        ],
        vec!["connectors", "serve", "--transport", "stdio"],
    ] {
        assert!(parse(&args).is_err(), "unexpected acceptance: {args:?}");
    }
}

#[test]
fn help_exposes_native_transport_without_fixture_output_global() {
    let error = command()
        .try_get_matches_from(["connectors", "server", "--help"])
        .unwrap_err();
    assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
    let help = error.to_string();
    for expected in ["--transport", "--config", "--state-dir"] {
        assert!(help.contains(expected), "missing {expected}: {help}");
    }
    assert!(!help.contains("--output"), "{help}");
    assert!(!help.contains("credential"), "{help}");
}

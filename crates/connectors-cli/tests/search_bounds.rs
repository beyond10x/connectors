//! Search limits are local command-line input, with the same bounds as their wire contracts.

use clap::error::ErrorKind;
use std::process::Command;

struct SearchSurface {
    arguments: &'static [&'static str],
    maximum: u16,
}

const SEARCHES: &[SearchSurface] = &[
    SearchSurface {
        arguments: &["operation", "search"],
        maximum: protocol::operation::MAX_SEARCH_RESULTS,
    },
    SearchSurface {
        arguments: &["connection", "list"],
        maximum: protocol::connection::MAX_SEARCH_RESULTS,
    },
    SearchSurface {
        arguments: &["endpoint", "list"],
        maximum: protocol::endpoint::MAX_RESULTS,
    },
    SearchSurface {
        arguments: &["event", "search"],
        maximum: protocol::event::MAX_SEARCH_RESULTS,
    },
];

fn arguments(surface: &SearchSurface) -> Vec<String> {
    std::iter::once("connectors")
        .chain(surface.arguments.iter().copied())
        .map(str::to_owned)
        .collect()
}

fn parsed_limit(arguments: Vec<String>) -> u16 {
    let matches = connectors_cli::command()
        .try_get_matches_from(arguments)
        .unwrap();
    let mut leaf = &matches;
    while let Some((_, command)) = leaf.subcommand() {
        leaf = command;
    }
    *leaf.get_one::<u16>("limit").unwrap()
}

#[test]
fn every_search_preserves_its_default_and_accepts_both_protocol_edges() {
    for surface in SEARCHES {
        assert_eq!(parsed_limit(arguments(surface)), surface.maximum);
        for limit in [1, surface.maximum] {
            let mut argv = arguments(surface);
            argv.extend(["--limit".to_owned(), limit.to_string()]);
            assert_eq!(parsed_limit(argv), limit);
        }
    }
}

#[test]
fn every_search_parser_refuses_zero_and_values_above_the_protocol_maximum() {
    for surface in SEARCHES {
        for limit in [0, surface.maximum + 1, u16::MAX] {
            let mut argv = arguments(surface);
            argv.extend(["--limit".to_owned(), limit.to_string()]);
            let error = connectors_cli::command()
                .try_get_matches_from(argv)
                .unwrap_err();
            assert_eq!(error.kind(), ErrorKind::ValueValidation);
            assert!(error.to_string().contains("--limit"));
        }
    }
}

#[test]
fn every_search_help_names_its_protocol_range_and_existing_default() {
    for surface in SEARCHES {
        let mut argv = arguments(surface);
        argv.push("--help".to_owned());
        let error = connectors_cli::command()
            .try_get_matches_from(argv)
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::DisplayHelp);
        let help = error.to_string();
        assert!(help.contains(&format!("1..={}", surface.maximum)), "{help}");
        assert!(
            help.contains(&format!("[default: {}]", surface.maximum)),
            "{help}"
        );
    }
}

#[test]
fn invalid_search_limits_exit_before_target_configuration_or_transport() {
    for surface in SEARCHES {
        for limit in [0, surface.maximum + 1] {
            for target in ["local", "hosted"] {
                for format in ["text", "json", "yaml"] {
                    let output = Command::new(env!("CARGO_BIN_EXE_connectors"))
                        .env_clear()
                        .args(surface.arguments)
                        .args(["--limit", &limit.to_string()])
                        .args(["--target", target, "--output", format])
                        // This path cannot resolve. Hosted also rejects these local-only flags,
                        // so a range error proves parsing precedes either application path.
                        .args(["--config", "/dev/null/search-bounds-config"])
                        .args(["--state-root", "/dev/null/search-bounds-state"])
                        .output()
                        .unwrap();
                    assert_eq!(output.status.code(), Some(2), "{output:?}");
                    assert!(output.stdout.is_empty(), "{output:?}");
                    let stderr = String::from_utf8(output.stderr).unwrap();
                    assert!(stderr.contains("invalid value"), "{stderr}");
                    assert!(stderr.contains("--limit"), "{stderr}");
                    assert!(!stderr.contains("connector-unreachable"), "{stderr}");
                    assert!(!stderr.contains("target-conflict"), "{stderr}");
                }
            }
        }
    }
}

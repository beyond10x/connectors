use super::*;
use clap::ValueEnum as _;

#[test]
fn normal_help_exposes_the_guided_flow_and_hides_acquisition_plumbing() {
    let mut command = Cli::command();
    for (group, leaf) in [("setup", "connect"), ("session", "login"), ("serve", "mcp")] {
        let help = command
            .find_subcommand_mut(group)
            .unwrap()
            .render_long_help()
            .to_string();
        assert!(help.contains(leaf), "`connectors {group}` hides `{leaf}`");
        assert!(!help.contains("connect-complete"));
    }

    let connection = command.find_subcommand_mut("connection").unwrap();
    let connection_help = connection.render_long_help().to_string();
    assert!(connection_help.contains("list"));
    assert!(!connection_help.contains("create"));
    assert!(!connection_help.contains("status"));
}

/// The tree with every subcommand, at every depth, renamed to a probe that can come from
/// nowhere but the subcommand's own name — `probe_serve_hosted_end` — and the probes.
///
/// Asserting that a script contains the *word* `hosted` proved nothing: the `serve` group's own
/// about-text carries it, and `connect` is a substring of the binary's name, so a deleted
/// variant left the assertion green. A probe is in the script only if the generator wrote that
/// node's name, and every node has a different one.
fn probed(tree: clap::Command, path: &[&str], probes: &mut Vec<String>) -> clap::Command {
    let names: Vec<String> = tree
        .get_subcommands()
        .map(|subcommand| subcommand.get_name().to_owned())
        .collect();
    let mut tree = tree;
    for name in names {
        let mut here: Vec<&str> = path.to_vec();
        here.push(&name);
        let probe = format!("probe_{}_end", here.join("_"));
        probes.push(probe.clone());
        // `Command::name` takes a `&'static str`; a leaked probe is fine for a test.
        let probe: &'static str = Box::leak(probe.into_boxed_str());
        tree = tree.mut_subcommand(&name, |subcommand| {
            probed(subcommand.name(probe), &here, probes)
        });
    }
    tree
}

#[test]
fn every_supported_shell_gets_a_script_naming_the_whole_surface() {
    let mut probes = Vec::new();
    let tree = probed(Cli::command(), &[], &mut probes);
    assert!(
        probes.iter().any(|probe| probe == "probe_serve_hosted_end")
            && probes
                .iter()
                .any(|probe| probe == "probe_setup_connect_end")
            && probes.len() >= 30,
        "the tree was read as {probes:?}; it moved, so read it again before believing this test"
    );
    for shell in clap_complete::Shell::value_variants() {
        let mut script = Vec::new();
        clap_complete::generate(*shell, &mut tree.clone(), "connectors", &mut script);
        let script = String::from_utf8(script).unwrap();
        let missing: Vec<&String> = probes
            .iter()
            .filter(|probe| !script.contains(probe.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "{shell} script does not name these subcommands: {missing:?}"
        );
    }
    let cli = Cli::try_parse_from(["connectors", "setup", "completions", "fish"]).unwrap();
    assert!(matches!(
        cli.command,
        Command::Setup {
            command: SetupCommand::Completions {
                shell: clap_complete::Shell::Fish
            }
        }
    ));
}

#[test]
fn slack_connect_needs_no_internal_reference_or_path_argument() {
    let cli = Cli::try_parse_from(["connectors", "setup", "connect", "slack"]).unwrap();
    let Command::Setup {
        command:
            SetupCommand::Connect {
                provider,
                config,
                label,
                context,
                state_root,
                ..
            },
    } = cli.command
    else {
        panic!("guided connect command was not parsed");
    };
    let provider = provider.expect("the provider mode retains its positional provider");
    assert_eq!(provider, "slack");
    assert!(label.is_none());
    assert!(context.is_none());
    assert!(config.is_none());
    assert!(state_root.is_none());
}

#[test]
fn grafana_connect_uses_the_same_guided_surface() {
    let cli = Cli::try_parse_from(["connectors", "setup", "connect", "grafana"]).unwrap();
    let Command::Setup {
        command: SetupCommand::Connect {
            provider, label, ..
        },
    } = cli.command
    else {
        panic!("guided connect command was not parsed");
    };
    let provider = provider.expect("the provider mode retains its positional provider");
    assert_eq!(provider, "grafana");
    assert!(label.is_none());
}

#[test]
fn kubernetes_connect_accepts_an_exact_context_selection() {
    let cli = Cli::try_parse_from([
        "connectors",
        "setup",
        "connect",
        "kubernetes",
        "--context",
        "dev-cluster",
    ])
    .unwrap();
    let Command::Setup {
        command: SetupCommand::Connect {
            provider, context, ..
        },
    } = cli.command
    else {
        panic!("guided connect command was not parsed");
    };
    let provider = provider.expect("the provider mode retains its positional provider");
    assert_eq!(provider, "kubernetes");
    assert_eq!(context.as_deref(), Some("dev-cluster"));
}

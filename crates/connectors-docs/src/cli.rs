//! The CLI reference page, walked from the two clap definitions the `connectors` binary parses:
//! the command tree generated from the ESS CLI binding (`connectors_cli_contract::command`) and the
//! explicit service commands (`connectors::compat::Args`).

use clap::CommandFactory;

use crate::{SOURCE, cell, front_matter};

/// Where the CLI reference lands, relative to the repository root.
pub const PAGE: &str = "website/docs/reference/cli.md";

/// Every command below `command` that has no subcommands of its own, depth first, without
/// clap's `help`.
fn leaves(command: &clap::Command, out: &mut Vec<clap::Command>) {
    for sub in command.get_subcommands() {
        if sub.get_name() == "help" {
            continue;
        }
        if sub.has_subcommands() {
            leaves(sub, out);
        } else {
            out.push(sub.clone());
        }
    }
}

/// The full command line of a built command, `connectors adapters describe`.
fn written(command: &clap::Command) -> String {
    command
        .get_bin_name()
        .unwrap_or(command.get_name())
        .to_owned()
}

/// The heading anchor Docusaurus gives `## \`connectors adapters describe\``.
fn anchor(command: &clap::Command) -> String {
    written(command).replace(' ', "-")
}

fn text(value: Option<&clap::builder::StyledStr>) -> String {
    value.map(ToString::to_string).unwrap_or_default()
}

/// One row per argument of `command`, skipping help, version and, unless `globals`, the
/// arguments every command inherits: how it is written, whether it is required, its default,
/// its accepted values and its help.
fn arguments(command: &clap::Command, globals: bool) -> Vec<[String; 5]> {
    command
        .get_arguments()
        .filter(|arg| !matches!(arg.get_id().as_str(), "help" | "version"))
        .filter(|arg| globals == arg.is_global_set())
        .map(|arg| {
            let value = arg
                .get_value_names()
                .and_then(|names| names.first().map(ToString::to_string))
                .unwrap_or_else(|| arg.get_id().as_str().to_owned());
            let written = match arg.get_long() {
                Some(long) if !arg.get_action().takes_values() => format!("--{long}"),
                Some(long) => format!("--{long} <{value}>"),
                None => format!("<{value}>"),
            };
            let defaults: Vec<String> = arg
                .get_default_values()
                .iter()
                .map(|default| default.to_string_lossy().into_owned())
                .collect();
            let possible: Vec<String> = arg
                .get_possible_values()
                .iter()
                .filter(|value| !value.is_hide_set())
                .map(|value| format!("`{}`", value.get_name()))
                .collect();
            [
                format!("`{written}`"),
                if arg.is_required_set() { "yes" } else { "no" }.to_owned(),
                if defaults.is_empty() {
                    "none".to_owned()
                } else {
                    format!("`{}`", defaults.join(", "))
                },
                if possible.is_empty() {
                    "any".to_owned()
                } else {
                    possible.join(", ")
                },
                cell(&text(arg.get_long_help().or(arg.get_help()))),
            ]
        })
        .collect()
}

fn argument_table(rows: Vec<[String; 5]>) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let mut out = "\n| Argument | Required | Default | Values | Meaning |\n|---|---|---|---|---|\n"
        .to_owned();
    for row in rows {
        out.push_str(&format!("| {} |\n", row.join(" | ")));
    }
    out
}

/// One section per command: its purpose, usage and arguments.
fn section(command: &mut clap::Command, about: &str) -> String {
    let usage = command.render_usage().to_string();
    let usage = usage.trim_start_matches("Usage: ").trim_end();
    format!(
        "\n## `{}`\n\n{}\n\n```text\n{usage}\n```\n{}",
        written(command),
        if about.is_empty() {
            "No summary is declared.".to_owned()
        } else {
            cell(about)
        },
        argument_table(arguments(command, false))
    )
}

/// The CLI reference page.
pub fn page() -> String {
    let mut generated = connectors_cli_contract::command();
    generated.build();
    let mut explicit = connectors::compat::Args::command().bin_name("connectors");
    explicit.build();
    let mut grouped = Vec::new();
    leaves(&generated, &mut grouped);
    let mut service = Vec::new();
    leaves(&explicit, &mut service);
    let summary = |command: &clap::Command| {
        connectors::compat::SUMMARIES
            .iter()
            .find(|(name, _)| *name == command.get_name())
            .map(|(_, summary)| (*summary).to_owned())
            .unwrap_or_default()
    };

    let mut out = front_matter(
        "connectors CLI reference",
        "connectors CLI",
        "Every command and option of the connectors command line, generated from the clap definitions it parses.",
        1,
    );
    out.push_str(&format!(
        "# `connectors`\n\nGenerated by `connectors-docs` from the two definitions the `connectors` binary parses: the command tree that ESS generates from the CLI binding [`apps/connectors/spec/cli.yaml`]({SOURCE}/apps/connectors/spec/cli.yaml), and the explicit service commands in [`apps/connectors/src/compat.rs`]({SOURCE}/apps/connectors/src/compat.rs). It lists only commands that exist. `connectors --version` prints the version, and `--help` after any command prints its usage.\n\n> {}\n\n## Global options\n\nEvery grouped command accepts these, anywhere on the command line.\n{}\n## Commands\n\n| Command | What it does |\n|---|---|\n",
        cell(&text(generated.get_about())),
        argument_table(arguments(&generated, true)),
    ));
    for command in &grouped {
        out.push_str(&format!(
            "| [`{}`](#{}) | {} |\n",
            written(command),
            anchor(command),
            cell(&text(command.get_about()))
        ));
    }
    for command in &service {
        out.push_str(&format!(
            "| [`{}`](#{}) | {} |\n",
            written(command),
            anchor(command),
            cell(&summary(command))
        ));
    }
    for command in &mut grouped {
        let about = text(command.get_long_about().or(command.get_about()));
        out.push_str(&section(command, &about));
    }
    out.push_str(
        "\n## Explicit service commands\n\nThese talk to an adapter service or a federation host over HTTP instead of the local owner. They accept `--output` for parse refusals only; a successful result is the service's JSON.\n",
    );
    for command in &mut service {
        let about = summary(command);
        out.push_str(&section(command, &about));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_lists_every_leaf_command_of_both_definitions() {
        let page = page();
        for command in [
            "connectors setup init",
            "connectors adapters list",
            "connectors connections connect",
            "connectors operations invoke",
            "connectors approvals clock-check",
            "connectors describe",
            "connectors invoke",
            "connectors serve",
        ] {
            assert!(page.contains(&format!("## `{command}`")), "{command}");
        }
        assert!(
            !page.contains("## `connectors help"),
            "help is not a command"
        );
    }
}

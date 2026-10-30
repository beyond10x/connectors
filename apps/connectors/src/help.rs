//! The help the `connectors` binary prints: the command tree generated from the ESS CLI binding
//! (`connectors_cli_contract::command`) with what the `ess-cli/1` format cannot declare. A group's
//! summary, the deprecation mark on an old command path and on `--adapter`, and help on one line
//! per argument are presentation only, as is the mark on what `invoke`'s preparation states (its
//! target, pins and input) as optional: a command line is parsed by the generated tree.
use clap::{Arg, Command};
use connectors_cli_contract::wire::Plan;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;

/// The summary of each command group of the binding.
pub const GROUPS: [(&str, &str); 6] = [
    (
        "approvals",
        "Approval policy, approval proofs and approval-signing keys for writes",
    ),
    (
        "connections",
        "The stored credential of an instance: connect, reconnect, inspect, launch and revoke it",
    ),
    (
        "definitions",
        "Connectors-definitions releases: install one by tag into the lock and the read-only cache",
    ),
    (
        "instances",
        "Configured instances and the process that serves each",
    ),
    (
        "operations",
        "The operations of an instance's connector: list and describe them",
    ),
    (
        "setup",
        "Local configuration, the metadata store and the lifecycle owner",
    ),
];

/// The `--adapter` selector's help.
pub const ADAPTER_HELP: &str =
    "Deprecated: the alias of a configured entry; select with --connector and --instance";

/// The help of the `invoke` arguments (story:connectors-do-short-invocation;
/// `contracts/cli/v1alpha1/semantics.md` §3, "Invoking an operation").
pub const INVOKE_HELP: [(&str, &str); 14] = [
    (
        "field:target",
        "The connector, and after one `/` its instance; the instance may be left out when the connector has only one; with --definition the target may be left out, the file naming the connector",
    ),
    ("field:operation", "The operation to invoke"),
    (
        "field:connection",
        "The connection to invoke through; the instance's only usable one when left out",
    ),
    (
        "field:schema",
        "A pin: the operation's schema identity, refused when the current description differs; taken from it when left out",
    ),
    (
        "field:revision",
        "A pin: the description's revision, refused when the current description differs; taken from it when left out",
    ),
    ("source:input:inline", "The input document, as JSON text"),
    ("source:input:file", "A file holding the input document"),
    (
        "source:input:stdin",
        "Read the input document from standard input, as happens when no input flag is given",
    ),
    (
        "field:approval_file",
        "The approval proof a write is invoked with",
    ),
    ("field:idempotency_key", "A write's idempotency key"),
    (
        "field:definition",
        "A connectors-definition/1 file to invoke without configuring it; a target, when given, names its connector",
    ),
    (
        "field:definitions_root",
        "With --definition: the tree holding its vendored source and shared auth files [default: the parent of the file's directory]",
    ),
    (
        "field:describe",
        "true: print the operation's description, effect, schema and revision, and invoke nothing",
    ),
    (
        "field:credential_file",
        "With --definition: the OAuth client file to connect with when the instance has no connection",
    ),
];

/// The generated command tree as the binary presents it.
pub fn command() -> Command {
    decorate(
        connectors_cli_contract::command(),
        &connectors_cli_contract::plan(),
    )
}

/// Each deprecated path of `plan` and the path that replaced it: every alias, and every group
/// whose commands are all aliases of one other group.
pub fn renamed(plan: &Plan) -> BTreeMap<Vec<String>, Vec<String>> {
    let canonical: BTreeSet<&String> = plan.commands.iter().map(|c| &c.path[0]).collect();
    let mut renamed = BTreeMap::new();
    let mut groups = BTreeMap::<Vec<String>, BTreeSet<Vec<String>>>::new();
    for command in &plan.commands {
        for alias in &command.aliases {
            renamed.insert(alias.clone(), command.path.clone());
            if alias.len() > 1 && !canonical.contains(&alias[0]) {
                groups
                    .entry(alias[..1].to_vec())
                    .or_default()
                    .insert(command.path[..1].to_vec());
            }
        }
    }
    for (group, targets) in groups {
        if let [target] = Vec::from_iter(targets).as_slice() {
            renamed.insert(group, target.clone());
        }
    }
    renamed
}

/// `root`, generated from `plan`, with group summaries and the deprecation marks.
pub fn decorate(root: Command, plan: &Plan) -> Command {
    let renamed = renamed(plan);
    let mut root = root
        .term_width(0)
        .mut_subcommands(|sub| node(sub, Vec::new(), &renamed));
    root.build();
    for (old, new) in &renamed {
        let Some(sub) = old
            .iter()
            .try_fold(&mut root, |command, word| command.find_subcommand_mut(word))
        else {
            continue;
        };
        let usage = sub.render_usage().to_string();
        let usage = usage.trim_start_matches("Usage: ").trim_end();
        let marked = format!("{usage}  (deprecated; use connectors {})", new.join(" "));
        *sub = std::mem::take(sub).override_usage(marked);
    }
    root
}

fn node(
    command: Command,
    parent: Vec<String>,
    renamed: &BTreeMap<Vec<String>, Vec<String>>,
) -> Command {
    let mut path = parent;
    path.push(command.get_name().to_owned());
    let mut command = command.mut_args(|arg: Arg| {
        if arg.get_id() == "field:adapter" {
            arg.value_name("ALIAS").help(ADAPTER_HELP)
        } else if path == ["invoke"] {
            invoke_arg(arg)
        } else {
            arg
        }
    });
    if let Some(new) = renamed.get(&path) {
        command = command.about(format!("Deprecated name of `connectors {}`", new.join(" ")));
    } else if let [group] = path.as_slice()
        && let Some((_, about)) = GROUPS.iter().find(|(name, _)| name == group)
    {
        command = command.about(*about);
    }
    if path == ["invoke"] {
        // One positional is the operation (`invoke --definition <file>
        // <operation>`), and no input flag reads standard input.
        command = command
            .allow_missing_positional(true)
            .mut_group("sources:input", |group| group.required(false));
    }
    command.mut_subcommands(|sub| node(sub, path.clone(), renamed))
}

/// An `invoke` argument with its help, required only when the command line
/// must state it; `--definitions-root` keeps the definition file's.
fn invoke_arg(arg: Arg) -> Arg {
    let id = arg.get_id().as_str().to_owned();
    let arg = match id.as_str() {
        "field:target" => arg.value_name("CONNECTOR[/INSTANCE]").required(false),
        "field:operation" => arg.value_name("OPERATION"),
        "field:schema" | "field:revision" => arg.required(false),
        _ => arg,
    };
    match INVOKE_HELP.iter().find(|(field, _)| *field == id) {
        Some((_, help)) => arg.help(*help),
        None => arg,
    }
}

/// The help `args` asks for, as the binary prints it; `None` when they ask for none.
pub fn render(args: &[OsString]) -> Option<String> {
    let asked = args
        .iter()
        .skip(1)
        .take_while(|arg| *arg != "--")
        .find(|arg| *arg == "--help" || *arg == "-h" || *arg == "help")?;
    let shown = |command: Command, args: &[OsString]| match command.try_get_matches_from(args) {
        Err(error) if error.kind() == clap::error::ErrorKind::DisplayHelp => {
            Some(error.to_string())
        }
        _ => None,
    };
    shown(command(), args).or_else(|| {
        // `invoke` shows its target as optional through clap's missing-positional
        // mode, which refuses a target given before an option and the operation
        // after it; the generated tree, which parses as this one otherwise does,
        // takes that line, and it asks for `invoke`'s help.
        args.iter()
            .take_while(|arg| *arg != "--")
            .any(|arg| arg == "invoke")
            .then_some(())?;
        shown(connectors_cli_contract::command(), args)?;
        shown(
            command(),
            &[args.first()?.clone(), "invoke".into(), asked.clone()],
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(words: &[&str]) -> Vec<OsString> {
        std::iter::once("connectors")
            .chain(words.iter().copied())
            .map(OsString::from)
            .collect()
    }

    #[test]
    fn every_invoke_line_asks_for_the_help_of_invoke() {
        let help = render(&line(&["invoke", "--help"]));
        assert!(
            help.as_deref()
                .is_some_and(|help| help.contains("[CONNECTOR[/INSTANCE]] <OPERATION>"))
        );
        for words in [
            &[
                "invoke",
                "fixture",
                "--connection",
                "c1",
                "item.probe",
                "--help",
            ][..],
            &[
                "--output",
                "json",
                "invoke",
                "fixture/main",
                "--input-json",
                "{}",
                "item.probe",
                "--help",
            ],
            &[
                "invoke",
                "--definition",
                "forge.yaml",
                "project.get",
                "--help",
            ],
            &["invoke", "fixture", "item.probe", "--help"],
        ] {
            assert_eq!(
                render(&line(words)),
                help,
                "story:connectors-do-short-invocation: `connectors {}` prints the help of invoke",
                words.join(" "),
            );
        }
        assert_eq!(
            render(&line(&[
                "invoke",
                "fixture",
                "item.probe",
                "extra",
                "--help"
            ])),
            None
        );
    }
}

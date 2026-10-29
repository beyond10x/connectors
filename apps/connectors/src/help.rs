//! The help the `connectors` binary prints: the command tree generated from the ESS CLI binding
//! (`connectors_cli_contract::command`) with what the `ess-cli/1` format cannot declare. A group's
//! summary, the deprecation mark on an old command path and on `--adapter`, and help on one line
//! per argument are presentation only: the tree parses exactly as the generated one does.
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
        "The operations of an instance's connector: list, describe and invoke them",
    ),
    (
        "setup",
        "Local configuration, the metadata store and the lifecycle owner",
    ),
];

/// The `--adapter` selector's help.
pub const ADAPTER_HELP: &str =
    "Deprecated: the alias of a configured entry; select with --connector and --instance";

/// The help of the `operations invoke` arguments a definition file adds or
/// states (`contracts/cli/v1alpha1/semantics.md` §3, "Invoking a definition file").
pub const DEFINITION_HELP: [(&str, &str); 5] = [
    (
        "field:definition",
        "A connectors-definition/1 file to invoke without configuring it; instead of --connector and --instance",
    ),
    (
        "field:definitions_root",
        "With --definition: the tree holding its vendored source and shared auth files [default: the parent of the file's directory]",
    ),
    (
        "field:credential_file",
        "With --definition: the OAuth client file to connect with when the instance has no connection",
    ),
    (
        "field:schema",
        "The operation's schema identity; with --definition, the description's when left out",
    ),
    (
        "field:revision",
        "The description's revision; with --definition, the current one when left out",
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
        } else if let Some((_, help)) = DEFINITION_HELP
            .iter()
            .find(|(id, _)| path == ["operations", "invoke"] && arg.get_id() == *id)
        {
            arg.help(*help)
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
    command.mut_subcommands(|sub| node(sub, path.clone(), renamed))
}

/// The help `args` asks for, as the binary prints it; `None` when they ask for none.
pub fn render(args: &[OsString]) -> Option<String> {
    let asks = args
        .iter()
        .skip(1)
        .take_while(|arg| *arg != "--")
        .any(|arg| arg == "--help" || arg == "-h" || arg == "help");
    if !asks {
        return None;
    }
    match command().try_get_matches_from(args) {
        Err(error) if error.kind() == clap::error::ErrorKind::DisplayHelp => {
            Some(error.to_string())
        }
        _ => None,
    }
}

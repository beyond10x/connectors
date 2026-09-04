//! **Adversary pass: the drift `tests/cli_surface.rs` claims to catch, driven through it.**
//!
//! Added by the adversary pass of `story:cli-surface-contract`. It changes no implementation file.
//! Every case here restates the contract of `crates/connectors-cli/tests/cli_surface.rs` as one
//! predicate and feeds it a drift, so that a drift the contract does not refuse fails *here*
//! instead of being invisible.
//!
//! [`contract_refusals`] is a faithful restatement of every assertion
//! `crates/connectors-cli/tests/cli_surface.rs` makes — named there as
//! `the_specification_names_the_binary_the_parser_builds`,
//! `every_path_of_the_parser_is_declared_or_a_named_exception`,
//! `every_named_exception_is_still_a_path_of_the_parser`,
//! `every_declared_group_is_a_group_of_the_parser`, `no_path_is_both_declared_and_excepted`,
//! `the_target_countdown_is_exactly_what_the_parser_still_owes` and
//! `the_committed_generated_tree_is_the_specification_word_for_word` — over a parser tree and a
//! committed generated tree given as arguments rather than read from disk. It exists because those
//! assertions live inside `#[test]` functions of another test binary and cannot be called; nothing
//! here weakens them.
//!
//! The three declarations below are copies of the contract's own. **No line range says so**, and
//! none should: a range is a claim that goes stale silently, which is what the second adversary
//! pass caught it doing. `the_copied_declarations_are_still_copies` compares the two files' text
//! directly, so the copy is checked rather than cited.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize)]
struct Component {
    component: String,
    cli: Option<CommandLineSurface>,
}

#[derive(serde::Deserialize)]
struct CommandLineSurface {
    binary: String,
    #[serde(default)]
    groups: Vec<Group>,
}

#[derive(serde::Deserialize)]
struct Group {
    name: String,
    summary: Option<String>,
}

#[derive(serde::Deserialize)]
struct Document {
    components: Vec<Component>,
}

/// Copies of `crates/connectors-cli/tests/cli_surface.rs`'s `Unspecified`, `UNSPECIFIED_PATHS` and
/// `TARGET_EXCEPTIONS`. `the_copied_declarations_are_still_copies` compares the two files' text and
/// is what refuses a copy that has stopped being one — including a kind or a reason that diverged,
/// which `contract_refusals` reads no column of and would never notice.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Unspecified {
    /// ESS models a read as a `views:` declaration carrying a consistency claim, and nothing in
    /// this tree states that claim for any of them (`ess/system/components.yaml`, the note on
    /// `connectors-service`).
    Read,
    /// Process, session or credential lifecycle: starting a server, writing a configuration,
    /// signing in, supplying a credential. No entity of this specification moves.
    Lifecycle,
    /// A command of this specification that `connectors-service` accepts and this frontend only
    /// forwards. ESS has no construct for a tree over a command a component does not handle.
    Forwarded,
    /// A guided sequence of several steps, not one declared command.
    Flow,
    /// A word that only carries other words and does nothing itself.
    Grouping,
    /// An act the tree performs that no command of this specification names.
    Unmodelled,
}

const UNSPECIFIED_PATHS: &[(&str, Unspecified, &str)] = &[
    // First-level words that are not groups.
    (
        "connect",
        Unspecified::Flow,
        "a guided acquisition flow, not one declared command",
    ),
    (
        "doctor",
        Unspecified::Read,
        "a read of the installation; no entity moves",
    ),
    (
        "init",
        Unspecified::Lifecycle,
        "writes a configuration file; no entity of this specification moves",
    ),
    (
        "login",
        Unspecified::Lifecycle,
        "acquires an Identity session; no entity of this specification moves",
    ),
    (
        "logout",
        Unspecified::Lifecycle,
        "discards an Identity session; no entity of this specification moves",
    ),
    (
        "mcp",
        Unspecified::Lifecycle,
        "serves a transport; no entity moves",
    ),
    (
        "providers",
        Unspecified::Read,
        "a read of the embedded catalogue; ESS models a read as a view",
    ),
    (
        "serve",
        Unspecified::Lifecycle,
        "starts a process; no entity moves",
    ),
    (
        "serve-hosted",
        Unspecified::Lifecycle,
        "starts a process; no entity moves",
    ),
    // Under `admin`. Both are groups of their own, and the tree runs one word deeper here than
    // anywhere else in the binary — which a contract comparing first-level words could not see.
    (
        "admin integrations",
        Unspecified::Grouping,
        "groups the reads of activated Integrations and their readiness",
    ),
    (
        "admin integrations status",
        Unspecified::Read,
        "a read of activated Integrations and their value-free readiness",
    ),
    (
        "admin credentials",
        Unspecified::Grouping,
        "groups credential custody for hosted Integrations",
    ),
    (
        "admin credentials set",
        Unspecified::Lifecycle,
        "supplies a credential a hosted Integration requires; no entity of this specification moves",
    ),
    // Under `auth`.
    (
        "auth status",
        Unspecified::Read,
        "a read of which configured providers have a credential stored",
    ),
    // Under `connection`.
    (
        "connection candidates",
        Unspecified::Read,
        "a read of potential direct Connections; no provider is contacted",
    ),
    (
        "connection activate",
        Unspecified::Forwarded,
        "forwards `connectors.connection.ActivateCandidate`, which `connectors-service` accepts",
    ),
    (
        "connection list",
        Unspecified::Read,
        "a read of non-secret Connection summaries",
    ),
    (
        "connection observations",
        Unspecified::Read,
        "a read of the latest stored discovery observations",
    ),
    (
        "connection materialize",
        Unspecified::Forwarded,
        "forwards `connectors.connection.MaterializeObservation`, which `connectors-service` \
         accepts",
    ),
    // Under `event`.
    (
        "event search",
        Unspecified::Read,
        "a read of admitted Connector channels",
    ),
    (
        "event receive",
        Unspecified::Read,
        "a read of durable events",
    ),
    (
        "event replay",
        Unspecified::Read,
        "a read of one stored event by reference",
    ),
    // Under `operation`.
    (
        "operation search",
        Unspecified::Read,
        "a read of currently callable operations",
    ),
    (
        "operation describe",
        Unspecified::Read,
        "a read of an operation description and its lease",
    ),
    (
        "operation signal",
        Unspecified::Unmodelled,
        "sends DTMF into an established session; no command of this specification names it",
    ),
    (
        "operation invoke",
        Unspecified::Forwarded,
        "forwards `connectors.runtime.InvokeOperation`, which `connectors-service` accepts",
    ),
];

const TARGET_EXCEPTIONS: &[&str] = &["connection", "event", "operation"];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate manifest lives two directories below the repository root")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn declared_surface() -> CommandLineSurface {
    let path = repository_root().join("ess/system/components.yaml");
    let document: Document = serde_norway::from_str(&read(&path))
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    document
        .components
        .into_iter()
        .find(|component| component.component == "connectors-cli")
        .expect("`connectors-cli` is declared in ess/system/components.yaml")
        .cli
        .expect("`connectors-cli` declares a `cli:` block")
}

fn specified_words(surface: &CommandLineSurface) -> BTreeSet<String> {
    let mut words: BTreeSet<String> = surface
        .groups
        .iter()
        .map(|group| group.name.clone())
        .collect();
    words.insert("completions".to_owned());
    words
}

fn committed_tree() -> String {
    read(&repository_root().join("ess/generated/clap/crates/connectors-cli/src/tree.rs"))
}

/// Every path the parser carries, at every depth. Restates `cli_surface.rs`'s `parser_paths`.
fn parser_paths(command: &clap::Command) -> BTreeSet<String> {
    fn walk(command: &clap::Command, prefix: &str, found: &mut BTreeSet<String>) {
        for child in command.get_subcommands() {
            let path = if prefix.is_empty() {
                child.get_name().to_owned()
            } else {
                format!("{prefix} {}", child.get_name())
            };
            walk(child, &path, found);
            found.insert(path);
        }
    }
    let mut found = BTreeSet::new();
    walk(command, "", &mut found);
    found
}

/// Restates `cli_surface.rs`'s `carries_target`: the group's own arguments as well as its leaves'.
fn carries_target(command: &clap::Command) -> bool {
    command
        .get_arguments()
        .any(|argument| argument.get_long() == Some("target"))
        || command.get_subcommands().any(carries_target)
}

/// Restates `cli_surface.rs`'s `deployment_protocol_modules`.
fn deployment_protocol_modules() -> BTreeSet<String> {
    ["connection", "event", "operation"]
        .into_iter()
        .filter(|module| {
            repository_root()
                .join(format!("crates/protocol/src/{module}.rs"))
                .is_file()
        })
        .map(str::to_owned)
        .collect()
}

/// Restates `cli_surface.rs`'s `generated_commands`.
fn generated_commands(source: &str) -> Vec<(String, Option<String>)> {
    const NEW: &str = "::clap::Command::new(";
    const ABOUT: &str = ".about(";

    fn literal(text: &str) -> String {
        let mut out = String::from('"');
        let mut characters = text
            .strip_prefix('"')
            .expect("a generated literal opens with a quote")
            .chars();
        while let Some(character) = characters.next() {
            out.push(character);
            match character {
                '\\' => out.extend(characters.next()),
                '"' => return out,
                _ => {}
            }
        }
        panic!("a generated literal closes with a quote");
    }

    let starts: Vec<usize> = source.match_indices(NEW).map(|(index, _)| index).collect();
    let mut emitted = Vec::new();
    for (position, start) in starts.iter().enumerate() {
        let end = starts.get(position + 1).copied().unwrap_or(source.len());
        let block = &source[*start..end];
        let name = literal(&block[NEW.len()..]);
        let about = block
            .find(ABOUT)
            .map(|index| literal(&block[index + ABOUT.len()..]));
        emitted.push((name, about));
    }
    emitted
}

/// **Every refusal the contract of `cli_surface.rs` would raise, for one parser and one tree.**
///
/// Empty means the contract is green. Each pushed string names the `cli_surface.rs` assertion it
/// comes from, so a reader can check the restatement against the original.
fn contract_refusals(parser: &clap::Command, generated: &str) -> Vec<String> {
    let surface = declared_surface();
    let specified = specified_words(&surface);
    let mut refusals = Vec::new();

    // `the_specification_names_the_binary_the_parser_builds`
    if surface.binary != parser.get_name() {
        refusals.push(format!(
            "cli_surface.rs the binary is `{}` and the parser is `{}`",
            surface.binary,
            parser.get_name()
        ));
    }

    // `every_path_of_the_parser_is_declared_or_a_named_exception` — the whole subtree, not the
    // first level.
    let excepted: BTreeSet<&str> = UNSPECIFIED_PATHS.iter().map(|(path, _, _)| *path).collect();
    let carried = parser_paths(parser);
    for path in &carried {
        if !specified.contains(path) && !excepted.contains(path.as_str()) {
            refusals.push(format!("cli_surface.rs undeclared path `{path}`"));
        }
    }

    // `every_named_exception_is_still_a_path_of_the_parser`
    for (path, _, _) in UNSPECIFIED_PATHS {
        if !carried.contains(*path) {
            refusals.push(format!(
                "cli_surface.rs `{path}` is excused and the parser no longer carries it"
            ));
        }
    }

    // `every_declared_group_is_a_group_of_the_parser`
    for group in &surface.groups {
        match parser.find_subcommand(&group.name) {
            None => refusals.push(format!(
                "cli_surface.rs `{}` is declared and the parser has no such word",
                group.name
            )),
            Some(command) if command.get_subcommands().next().is_none() => refusals.push(format!(
                "cli_surface.rs `{}` is declared as a group and the parser's word carries no \
                 subcommand",
                group.name
            )),
            Some(_) => {}
        }
    }

    // `no_path_is_both_declared_and_excepted`
    for (path, _, _) in UNSPECIFIED_PATHS {
        if specified.contains(*path) {
            refusals.push(format!(
                "cli_surface.rs `{path}` is both declared and excepted"
            ));
        }
    }

    // `the_target_countdown_is_exactly_what_the_parser_still_owes`
    let protocols = deployment_protocol_modules();
    let pending: BTreeSet<String> = surface
        .groups
        .iter()
        .map(|group| group.name.clone())
        .filter(|name| protocols.contains(name))
        .filter(|name| {
            parser
                .find_subcommand(name)
                .is_some_and(|group| !carries_target(group))
        })
        .collect();
    for name in TARGET_EXCEPTIONS {
        if !pending.contains(*name) {
            refusals.push(format!(
                "cli_surface.rs `{name}` now carries `--target`; take it off the allowlist"
            ));
        }
    }
    for name in &pending {
        if !TARGET_EXCEPTIONS.contains(&name.as_str()) {
            refusals.push(format!(
                "cli_surface.rs `{name}` still owes `--target` and is not on the allowlist"
            ));
        }
    }

    // `the_committed_generated_tree_is_the_specification_word_for_word`
    let emitted = generated_commands(generated);
    let mut expected: Vec<(String, Option<String>)> = vec![(format!("{:?}", surface.binary), None)];
    for group in &surface.groups {
        expected.push((
            format!("{:?}", group.name),
            Some(format!(
                "{:?}",
                group
                    .summary
                    .as_deref()
                    .expect("every declared group carries a summary")
            )),
        ));
    }
    expected.push((
        "\"completions\"".to_owned(),
        Some("\"Print a completion script for one shell, from this same command tree\"".to_owned()),
    ));

    let names: Vec<&str> = emitted.iter().map(|(name, _)| name.as_str()).collect();
    let wanted: Vec<&str> = expected.iter().map(|(name, _)| name.as_str()).collect();
    if names != wanted {
        refusals.push(format!(
            "cli_surface.rs the generated tree places {names:?}, not {wanted:?}"
        ));
    }
    for ((name, about), (_, want)) in emitted.iter().zip(expected.iter()) {
        if want.is_some() && about != want {
            refusals.push(format!(
                "cli_surface.rs the generated tree writes {about:?} under {name}, not {want:?}"
            ));
        }
    }

    refusals
}

/// **The three declarations copied from the contract are still copies of it, character for
/// character.**
///
/// `contract_refusals` reads only the path column of `UNSPECIFIED_PATHS`, so a kind or a reason
/// that diverged between the two files would leave every case here green while the drift suite
/// silently described a different list from the one that ships. A line-range citation saying where
/// the copy came from does not close that — it goes stale on its own, which is what the second
/// adversary pass caught. This compares the text.
#[test]
fn the_copied_declarations_are_still_copies() {
    /// The declaration named, from `const`/`enum` to the line that closes it.
    fn declaration(source: &str, opener: &str, closer: &str) -> String {
        let start = source
            .find(opener)
            .unwrap_or_else(|| panic!("a source declares `{opener}`"));
        let end = source[start..]
            .find(closer)
            .unwrap_or_else(|| panic!("`{opener}` closes with `{closer}`"));
        source[start..start + end + closer.len()].to_owned()
    }

    let contract = read(&repository_root().join("crates/connectors-cli/tests/cli_surface.rs"));
    let here = read(&repository_root().join("crates/connectors-cli/tests/cli_surface_drift.rs"));

    let mut diverged = Vec::new();
    for (opener, closer) in [
        ("enum Unspecified {", "\n}"),
        ("const UNSPECIFIED_PATHS", "\n];"),
        ("const TARGET_EXCEPTIONS", ";"),
    ] {
        let theirs = declaration(&contract, opener, closer);
        let ours = declaration(&here, opener, closer);
        if theirs != ours {
            diverged.push(format!(
                "`{opener}`\n  contract: {theirs}\n  drift suite: {ours}"
            ));
        }
    }

    assert!(
        diverged.is_empty(),
        "the drift suite's copies of the contract's declarations are no longer copies, so every \
         case in this file is measuring a list the contract does not carry:\n  {}",
        diverged.join("\n  ")
    );
}

/// The contract is green against the tree exactly as it was handed over. A control: every case
/// below changes one thing, so this has to hold or the cases are measuring the restatement.
#[test]
fn the_restated_contract_is_green_against_the_unchanged_tree() {
    let refusals = contract_refusals(&connectors_cli::command(), &committed_tree());
    assert!(
        refusals.is_empty(),
        "the restatement refuses the unchanged tree, so it is not a restatement:\n  {}",
        refusals.join("\n  ")
    );
}

/// **A command added to the parser and to nothing else is refused.**
///
/// `.engineering/planning/story/cli-surface-contract.md`, `## Acceptance`: "`cargo test -p
/// connectors-cli --locked` fails when a command is added to the parser and not the
/// specification". `connection harvest` is such a command: it is a word a person can type, the
/// specification declares nothing named `harvest`, and no group of the specification places it.
#[test]
fn a_command_added_under_a_declared_group_is_refused() {
    let drifted = connectors_cli::command().mut_subcommand("connection", |group| {
        group.subcommand(clap::Command::new("harvest").about("added by nobody's decision"))
    });
    let refusals = contract_refusals(&drifted, &committed_tree());
    assert!(
        !refusals.is_empty(),
        "`connectors connection harvest` was added to the parser and to nothing else, and the \
         contract of tests/cli_surface.rs raised no refusal"
    );
}

/// **A command added in the wrong place is refused.**
///
/// `.engineering/planning/story/cli-surface-contract.md`, `## Defect`: "nothing catches a command
/// added in the wrong place". `event invoke` is the wrong place: `invoke` is
/// `connectors.runtime.InvokeOperation`, and `ess/system/components.yaml` puts operations under
/// `operation`, whose summary is "Search, describe, or invoke admitted Connector operations."
#[test]
fn a_command_added_under_the_wrong_declared_group_is_refused() {
    let drifted = connectors_cli::command().mut_subcommand("event", |group| {
        group.subcommand(clap::Command::new("invoke").about("in the wrong group"))
    });
    let refusals = contract_refusals(&drifted, &committed_tree());
    assert!(
        !refusals.is_empty(),
        "`connectors event invoke` puts an operation verb under the event group, and the contract \
         of tests/cli_surface.rs raised no refusal"
    );
}

/// **A committed tree left behind by a specification that moved is refused.**
///
/// `the_committed_generated_tree_is_the_specification_word_for_word` compares the whole emitted
/// sequence — every word and every `.about(…)` — with the `cli:` block. Editing one group's
/// `summary:` in `ess/system/components.yaml` and not regenerating is what that is for, and it was
/// measured: with
/// `admin`'s summary changed to one other sentence, `ess generate synthesize --path ess/system
/// --target clap` emits a `tree.rs` differing from the committed one only in that `.about(…)` and
/// in the two digest lines. This case stands that stale tree up from the committed bytes.
#[test]
fn a_committed_tree_whose_group_about_no_longer_matches_the_specification_is_refused() {
    let stale = committed_tree().replace(
        ".about(\"Operate an Identity-protected hosted Connectors instance.\")",
        ".about(\"Operate a hosted Connectors instance, whoever it belongs to.\")",
    );
    assert_ne!(stale, committed_tree(), "the stale tree must differ");
    let refusals = contract_refusals(&connectors_cli::command(), &stale);
    assert!(
        !refusals.is_empty(),
        "the committed tree names a group summary the specification no longer carries, and the \
         contract of tests/cli_surface.rs raised no refusal"
    );
}

/// **A committed tree that swaps `completions` for a word nothing declares is refused.**
///
/// This case was written against a contract that compared a *count* of emitted words, which a tree
/// dropping `completions` for any other word satisfies at seven. It no longer compares a count:
/// `the_committed_generated_tree_is_the_specification_word_for_word` compares the emitted sequence
/// of words with the declared one, so a swap is refused by name and not by arithmetic. The case
/// stays because the drift is still a drift, and it now measures the stronger check.
#[test]
fn a_committed_tree_that_swaps_completions_for_an_undeclared_word_is_refused() {
    let stale = committed_tree().replace(
        "::clap::Command::new(\"completions\")",
        "::clap::Command::new(\"handoff\")",
    );
    assert_ne!(stale, committed_tree(), "the stale tree must differ");
    let refusals = contract_refusals(&connectors_cli::command(), &stale);
    assert!(
        !refusals.is_empty(),
        "the committed tree drops `completions` for `handoff`, which the specification declares \
         nowhere, and the contract of tests/cli_surface.rs raised no refusal"
    );
}

/// **`--target` on the group itself empties the countdown.**
///
/// A flag that applies to every subcommand of `connection` is naturally declared on `connection`,
/// not repeated on each leaf. When this case was written the contract read the leaves' arguments
/// only and never saw such a flag, so the countdown that is supposed to reach zero when
/// `story:explicit-target-never-implicit` lands would not have noticed that it landed. `carries_target`
/// now reads a command's own arguments and its whole subtree, and this case is what holds it there.
#[test]
fn a_target_flag_on_the_group_itself_is_seen_by_the_countdown() {
    let drifted = connectors_cli::command().mut_subcommand("connection", |group| {
        group.arg(
            clap::Arg::new("target")
                .long("target")
                .value_parser(["local", "hosted"]),
        )
    });
    let refusals = contract_refusals(&drifted, &committed_tree());
    assert!(
        refusals
            .iter()
            .any(|refusal| refusal.contains("`connection` now carries `--target`")),
        "`connectors connection --target …` is declared and the `--target` countdown did not \
         notice; refusals were: {refusals:?}"
    );
}

/// **Cutting one group over to the generated tree is refused, and the design document now says so.**
///
/// This case was written asserting the opposite, against
/// `docs/design/19-the-cli-surface.md`'s claim that "the cutover from the hand-written enums to the
/// generated tree happens one group at a time, with the gate green between each". It was right that
/// the claim is false, and it is the reason that page now carries a section saying the cutover
/// cannot start yet and naming what has to change first.
///
/// The assertion is flipped rather than deleted, because the correction is a fact about the
/// repository and a fact is worth holding: the generated `admin`
/// (`ess/generated/clap/crates/connectors-cli/src/tree.rs:17-20`) is a group with
/// `subcommand_required(true)` and nothing under it, which is all the specification can emit while
/// `connectors-cli` accepts no command. Putting it in the parser deletes `admin integrations` and
/// `admin credentials`, and the contract refuses that twice — once for a declared group whose word
/// carries no subcommand, and once for each `UNSPECIFIED_PATHS` entry the parser no longer carries.
/// Neither refusal is weakened to let the cutover through. When ESS can place a forwarded command,
/// the emitted group will carry commands, this case will fail, and that failure is the signal that
/// the cutover has become possible.
#[test]
fn cutting_the_admin_group_over_to_the_generated_tree_is_refused() {
    let cut_over = connectors_cli::command().mut_subcommand("admin", |_| {
        clap::Command::new("admin")
            .subcommand_required(true)
            .arg_required_else_help(true)
            .about("Operate an Identity-protected hosted Connectors instance.")
    });
    let refusals = contract_refusals(&cut_over, &committed_tree());
    assert!(
        refusals.iter().any(|refusal| refusal.contains(
            "`admin` is declared as a group and the parser's word \
                                             carries no subcommand"
        )),
        "the emitted `admin` carries no subcommand, and the contract did not refuse a declared \
         group that has become a leaf; refusals were: {refusals:?}"
    );
    for orphaned in ["admin integrations", "admin credentials"] {
        assert!(
            refusals.iter().any(|refusal| refusal.contains(&format!(
                "`{orphaned}` is excused and the parser no longer carries it"
            ))),
            "cutting `admin` over deletes `{orphaned}`, and `UNSPECIFIED_PATHS` still excuses it; \
             refusals were: {refusals:?}"
        );
    }
}

/// **The specification's citation for the thin-frontend rule points at the rule.**
///
/// `ess/system/components.yaml` says `crates/catalog-build/tests/main/architecture_fence.rs:303`
/// "holds it to that", `that` being "Thin command-line frontend for reusable Connector client and
/// runtime libraries". `crates/catalog-build/tests/main/ess_citation_fence.rs:8-10` states what a
/// citation of this document is for: "a citation which drifts away from what it points at fails
/// here rather than being believed by the next reader."
#[test]
fn the_thin_frontend_citation_points_at_the_thin_frontend_test() {
    let root = repository_root();
    let specification = read(&root.join("ess/system/components.yaml"));
    let marker = "architecture_fence.rs:";
    let index = specification
        .find(marker)
        .expect("components.yaml cites architecture_fence.rs");
    let cited: usize = specification[index + marker.len()..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .expect("the citation names a line");

    let fence = read(&root.join("crates/catalog-build/tests/main/architecture_fence.rs"));
    let lines: Vec<&str> = fence.lines().collect();
    let start = lines
        .iter()
        .position(|line| line.starts_with("fn product_cli_is_a_thin_frontend()"))
        .expect("architecture_fence.rs declares product_cli_is_a_thin_frontend");
    let end = start
        + 1
        + lines[start + 1..]
            .iter()
            .position(|line| *line == "}")
            .expect("the function closes");

    assert!(
        (start..=end + 1).contains(&(cited - 1)),
        "ess/system/components.yaml cites architecture_fence.rs:{cited}, which is `{}`; the test \
         that holds `connectors-cli` to being a thin command-line frontend is \
         `product_cli_is_a_thin_frontend`, lines {}-{}",
        lines[cited - 1].trim(),
        start + 1,
        end + 1
    );
}

/// **Cargo can read the committed emitted manifest.**
///
/// `ess/generated/clap/crates/connectors-cli/Cargo.toml` is a package manifest inside the root
/// workspace directory, and the root `Cargo.toml` lists it in neither `[workspace] members` nor
/// `[workspace] exclude`. `docs/design/19-the-cli-surface.md:78-90` calls the emitted crate a
/// parallel artifact that a cutover moves into, and `ess/generated/clap/PLAN.md:156` reports
/// `component port | connectors-cli` as generated — both of which require that it can be built.
#[test]
fn cargo_can_read_the_committed_emitted_manifest() {
    let manifest = repository_root().join("ess/generated/clap/crates/connectors-cli/Cargo.toml");
    let output = std::process::Command::new(std::env::var("CARGO").unwrap_or("cargo".to_owned()))
        .args([
            "metadata",
            "--no-deps",
            "--offline",
            "--format-version",
            "1",
        ])
        .arg("--manifest-path")
        .arg(&manifest)
        .output()
        .expect("run cargo metadata");
    assert!(
        output.status.success(),
        "cargo refuses the committed emitted manifest {}:\n{}",
        manifest.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

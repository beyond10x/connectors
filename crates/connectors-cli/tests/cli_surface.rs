//! **The command-line surface, held against the specification that declares it.**
//!
//! `ess/system/components.yaml` declares the `connectors` binary and its groups under
//! `connectors-cli`'s `cli:` block. `crates/connectors-cli/src/lib.rs` builds the parser.
//! `ess/generated/clap/` is what `ess generate synthesize --target clap` projects from the
//! declaration. Nothing but this file compares the three, so without it any of them can move alone.
//!
//! # Why the specification carries the groups and not the leaves
//!
//! ESS gives a command one handler and it is the component owning its domain —
//! `ESS-COMPONENT-004`, raised by `ess specify validate` the moment a component accepts a command
//! from a domain it does not own. `connectors-cli` owns `connectors.target` alone, so it accepts no
//! command of `connectors.connection` or `connectors.runtime` and its tree may place none of them:
//! a command-line component's tree is exactly what it accepts, no more and no less. The sixteen
//! commands `connectors-service` accepts are therefore absent from the tree by construction rather
//! than by a filter, and `ess/generated/clap/TARGET.md` lists every one as a target refusal.
//!
//! That is the whole content of "the accepted surface and the command-line surface are not the same
//! set": they are two components, and neither can borrow the other's commands.
//!
//! # What that means for this file
//!
//! Everything the specification cannot yet say is written down here as a **path**, with the reason,
//! and both directions are checked: a path the parser gains that is neither declared nor listed is
//! refused, and a path listed here that the parser no longer carries is refused too. The list is a
//! countdown, and a countdown nobody can add to quietly is the only kind worth keeping.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// One `components:` entry, read for the only two fields this file compares.
#[derive(serde::Deserialize)]
struct Component {
    component: String,
    cli: Option<CommandLineSurface>,
}

/// The `cli:` block: the binary a person types and the first-level words it carries.
#[derive(serde::Deserialize)]
struct CommandLineSurface {
    binary: String,
    #[serde(default)]
    groups: Vec<Group>,
}

/// One first-level word, and the sentence that becomes its `--help` line.
#[derive(serde::Deserialize)]
struct Group {
    name: String,
    summary: Option<String>,
}

/// The whole document.
#[derive(serde::Deserialize)]
struct Document {
    components: Vec<Component>,
}

/// Why a path the parser carries is not in the specification. One of these, never prose, so the
/// header above the list cannot claim something the list does not carry.
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

/// The whole set, so a variant nobody uses is a classification nothing carries.
const EVERY_KIND: &[Unspecified] = &[
    Unspecified::Read,
    Unspecified::Lifecycle,
    Unspecified::Forwarded,
    Unspecified::Flow,
    Unspecified::Grouping,
    Unspecified::Unmodelled,
];

/// **Every path the parser carries that the specification cannot say, its kind, and why.**
///
/// A path is written as a person types it after the binary — `connection activate`, not
/// `activate` — because a word only means something under the word above it, and a contract that
/// compared first-level words alone would let `connectors event invoke` through.
///
/// Both directions are checked, so this list cannot grow by accident and cannot outlive the parser.
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

/// **The groups whose commands do not say which deployment they reach.**
///
/// `story:explicit-target-never-implicit` gives each of them a `--target` flag. This list is a
/// countdown, and its membership is measured against the parser rather than asserted: it must equal
/// the set of declared groups that name a module of `crates/protocol/src` declaring a request
/// enum and that do not yet carry `--target`. A group that gains the flag has to leave the list, and when all three have it the
/// list is empty.
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

/// The `cli:` block `connectors-cli` declares.
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
        .expect(
            "`connectors-cli` declares a `cli:` block; without one there is no command-line \
             surface to hold the parser against",
        )
}

/// **Every path the parser carries, at every depth**, written as a person types it.
///
/// The whole subtree, not the first level: a command added under a declared group, or under the
/// wrong one, is a different path from any the specification names and has to be visible as such.
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

/// Every first-level word the specification accounts for. `completions` is emitted by the clap
/// target for every declared surface, so the specification carries it by construction.
fn specified_words(surface: &CommandLineSurface) -> BTreeSet<String> {
    let mut words: BTreeSet<String> = surface
        .groups
        .iter()
        .map(|group| group.name.clone())
        .collect();
    words.insert("completions".to_owned());
    words
}

/// `true` where this command or anything under it accepts `--target`.
///
/// The group's own arguments as well as its leaves'. In clap's derive API a flag every subcommand
/// shares is declared once on the group, which is where `story:explicit-target-never-implicit`
/// puts it, and a check that read only the leaves would watch the countdown land and not notice.
fn carries_target(command: &clap::Command) -> bool {
    command
        .get_arguments()
        .any(|argument| argument.get_long() == Some("target"))
        || command.get_subcommands().any(carries_target)
}

/// Every module of `crates/protocol/src` that declares a request enum, read from the tree.
///
/// A group named after one of them is a group whose commands reach a deployment, which is exactly
/// what `--target` has to say. **Derived, not listed**, and the difference is the whole point: an
/// earlier version of this function was the literal `["connection", "event", "operation"]` filtered
/// by `is_file()`, and a filter over a literal can only ever remove a module. `crates/protocol`
/// declares five — `catalog`, `connection`, `datasource`, `event` and `operation`, the last two of
/// the five being answered at `crates/server/src/catalog_projection.rs:38` and sent at
/// `crates/connectors-client/src/lib.rs:594` — so a `catalog` or `datasource` group would never
/// have entered the countdown, and the countdown would have reached zero while that group still
/// inferred its target.
fn deployment_protocol_modules() -> BTreeSet<String> {
    let directory = repository_root().join("crates/protocol/src");
    let mut declared = BTreeSet::new();
    for entry in std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()))
    {
        let path = entry.expect("directory entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let carries_a_request = read(&path).lines().any(|line| {
            line.strip_prefix("pub enum ").is_some_and(|rest| {
                rest.split(|character: char| !character.is_alphanumeric())
                    .next()
                    .is_some_and(|name| name.ends_with("Request"))
            })
        });
        if carries_a_request {
            declared.insert(
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .expect("a module name")
                    .to_owned(),
            );
        }
    }
    declared
}

/// One emitted `::clap::Command::new(…)` and the `.about(…)` that follows it, both as written.
///
/// The literals are compared unparsed. The generator writes them with `{:?}`, so an expectation
/// built the same way is byte-comparable, and a comparison that unescaped first would differ from
/// the generator about what it emitted.
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

/// **The binary the specification names is the binary the parser builds.**
#[test]
fn the_specification_names_the_binary_the_parser_builds() {
    let surface = declared_surface();
    assert_eq!(
        surface.binary,
        connectors_cli::command().get_name(),
        "`cli.binary` in ess/system/components.yaml and the parser's own name are the same word, \
         or the generated completion script installs under a name nothing answers to"
    );
}

/// **Every path of the parser is declared, or is a named exception.**
///
/// The direction that fails when a command is added to the parser and nowhere else — at any depth,
/// and under any group. `connection harvest` is a path the specification does not name;
/// `event invoke` is a path the specification does not name either, and that it resembles
/// `operation invoke` is precisely why the comparison is over paths and not over words.
#[test]
fn every_path_of_the_parser_is_declared_or_a_named_exception() {
    let surface = declared_surface();
    let specified = specified_words(&surface);
    let excepted: BTreeSet<&str> = UNSPECIFIED_PATHS.iter().map(|(path, _, _)| *path).collect();

    let undeclared: Vec<String> = parser_paths(&connectors_cli::command())
        .into_iter()
        .filter(|path| !specified.contains(path) && !excepted.contains(path.as_str()))
        .collect();

    assert!(
        undeclared.is_empty(),
        "the parser carries paths the specification does not account for:\n  {}\nDeclare each as \
         a group in the `cli:` block of ess/system/components.yaml, or add it to \
         `UNSPECIFIED_PATHS` with the reason the specification cannot say it.",
        undeclared.join("\n  ")
    );
}

/// **Every named exception is still a path of the parser.**
///
/// The reverse direction, and what makes `UNSPECIFIED_PATHS` a countdown rather than an archive. A
/// path removed or renamed in `lib.rs` leaves an entry here excusing a word nobody can type, and a
/// list that keeps such entries stops being evidence of anything.
#[test]
fn every_named_exception_is_still_a_path_of_the_parser() {
    let carried = parser_paths(&connectors_cli::command());
    let stale: Vec<&str> = UNSPECIFIED_PATHS
        .iter()
        .map(|(path, _, _)| *path)
        .filter(|path| !carried.contains(*path))
        .collect();

    assert!(
        stale.is_empty(),
        "`UNSPECIFIED_PATHS` excuses paths the parser no longer carries: {}\nRemove each entry, \
         or restore the command it was written for.",
        stale.join(", ")
    );
}

/// **Every kind of exception is one the list uses, and every entry carries a reason.**
///
/// The kinds replace a prose header that claimed the list held "reads and process-lifecycle verbs"
/// while carrying four other things. A classification the compiler checks cannot make that claim
/// wrongly; a variant nothing uses is a claim the list no longer supports and is refused here.
#[test]
fn every_kind_of_exception_is_used_and_every_entry_gives_a_reason() {
    let used: BTreeSet<Unspecified> = UNSPECIFIED_PATHS.iter().map(|(_, kind, _)| *kind).collect();
    let unused: Vec<&Unspecified> = EVERY_KIND
        .iter()
        .filter(|kind| !used.contains(kind))
        .collect();
    assert!(
        unused.is_empty(),
        "`Unspecified` declares kinds no entry of `UNSPECIFIED_PATHS` carries: {unused:?}. Delete \
         the variant, or the reason it was added for is missing from the list"
    );

    let silent: Vec<&str> = UNSPECIFIED_PATHS
        .iter()
        .filter(|(_, _, reason)| reason.trim().is_empty())
        .map(|(path, _, _)| *path)
        .collect();
    assert!(
        silent.is_empty(),
        "these exceptions carry a kind and no sentence: {}",
        silent.join(", ")
    );
}

/// **Every group the specification declares is a group of the parser.**
///
/// A group renamed or dropped in `lib.rs` leaves the specification naming a word nobody can type,
/// and the completion script would offer it. A declared group that carries no command is the same
/// defect one step in: `connectors connection` with nothing under it is a word that can only refuse.
#[test]
fn every_declared_group_is_a_group_of_the_parser() {
    let surface = declared_surface();
    let parser = connectors_cli::command();

    let mut wrong = Vec::new();
    for group in &surface.groups {
        match parser.find_subcommand(&group.name) {
            None => wrong.push(format!(
                "`{}` is declared and the parser has no such word",
                group.name
            )),
            Some(command) if command.get_subcommands().next().is_none() => wrong.push(format!(
                "`{}` is declared as a group and the parser's word carries no subcommand",
                group.name
            )),
            Some(_) => {}
        }
    }

    assert!(
        wrong.is_empty(),
        "the `cli:` block of ess/system/components.yaml names groups the parser does not \
         carry:\n  {}",
        wrong.join("\n  ")
    );
}

/// **No path is both declared and excepted.**
///
/// Two answers to one question is an undecided question written down twice, and the exception list
/// would then keep a path alive after the specification learned how to say it.
#[test]
fn no_path_is_both_declared_and_excepted() {
    let surface = declared_surface();
    let specified = specified_words(&surface);
    let both: Vec<&str> = UNSPECIFIED_PATHS
        .iter()
        .map(|(path, _, _)| *path)
        .filter(|path| specified.contains(*path))
        .collect();
    assert!(
        both.is_empty(),
        "these paths are declared in the `cli:` block and also excused as unspecifiable: {}",
        both.join(", ")
    );
}

/// **The `--target` countdown is the set of deployment-protocol groups that do not yet carry it.**
///
/// Every number here comes from the parser and the tree. The candidates are the declared groups
/// named after a module of `crates/protocol/src` that declares a request enum — five today, not
/// three; the pending set is
/// those of them that carry `--target` nowhere in their subtree, the group's own arguments
/// included. `TARGET_EXCEPTIONS` has to equal that pending set exactly, so the list cannot be any
/// other size, cannot keep a group that has already gained the flag, and cannot quietly excuse one
/// that never had it. `story:explicit-target-never-implicit` empties it.
#[test]
fn the_target_countdown_is_exactly_what_the_parser_still_owes() {
    let surface = declared_surface();
    let parser = connectors_cli::command();
    let protocols = deployment_protocol_modules();
    // Not a count: the point of deriving the set is that nobody has to know it. This refuses an
    // extraction that has stopped finding request enums at all, which would empty the candidate
    // set and make the countdown pass by finding nothing.
    for known in ["connection", "event", "operation"] {
        assert!(
            protocols.contains(known),
            "`deployment_protocol_modules` reads `pub enum …Request` out of crates/protocol/src \
             and did not find `{known}`; the modules moved, so read them again before believing \
             any result from this test: {protocols:?}"
        );
    }

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

    let listed: BTreeSet<String> = TARGET_EXCEPTIONS
        .iter()
        .map(|name| (*name).to_owned())
        .collect();

    assert_eq!(
        listed, pending,
        "`TARGET_EXCEPTIONS` is the countdown of deployment-protocol groups that still do not say \
         which deployment they reach. The parser says the pending set is {pending:?} and the list \
         says {listed:?}; a group that gained `--target` leaves the list, and when the list is \
         empty `story:explicit-target-never-implicit` is done"
    );
}

/// **The committed generated tree is the specification, word for word.**
///
/// `ess generate synthesize --target clap` reads the same `cli:` block this file reads, so every
/// word it places and every `--help` line it writes is derivable here. Comparing the whole emitted
/// sequence — names *and* `.about(…)` — is what catches a `summary:` edited in the specification
/// and not regenerated, a group dropped, and a tree that swaps `completions` for a word nothing
/// declares while keeping the count.
///
/// Byte-identity against a fresh generation is `scripts/gate.sh --final`, which has `ess` on its
/// path and also covers the digests, the handler seam and the emitted manifest. This assertion
/// needs no tool and runs in every lane.
#[test]
fn the_committed_generated_tree_is_the_specification_word_for_word() {
    let surface = declared_surface();
    let path = repository_root().join("ess/generated/clap/crates/connectors-cli/src/tree.rs");
    let emitted = generated_commands(&read(&path));

    let mut expected: Vec<(String, Option<String>)> = Vec::new();
    // The binary. Its `.about(…)` is the component's summary, which is the emitter's decision and
    // not this repository's, so the word is checked and the sentence is not.
    expected.push((format!("{:?}", surface.binary), None));
    for group in &surface.groups {
        expected.push((
            format!("{:?}", group.name),
            Some(format!(
                "{:?}",
                group
                    .summary
                    .as_deref()
                    .unwrap_or_else(|| panic!("group `{}` declares a summary", group.name))
            )),
        ));
    }
    // Emitted for every declared surface, whatever the tree places.
    expected.push((
        "\"completions\"".to_owned(),
        Some("\"Print a completion script for one shell, from this same command tree\"".to_owned()),
    ));

    let names: Vec<&str> = emitted.iter().map(|(name, _)| name.as_str()).collect();
    let wanted: Vec<&str> = expected.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        wanted,
        "{} places {names:?}; ess/system/components.yaml declares {wanted:?}. Regenerate it with \
         `ess generate synthesize --path ess/system --target clap --out ess/generated/clap`",
        path.display()
    );

    let mut wrong = BTreeMap::new();
    for ((name, about), (_, want)) in emitted.iter().zip(expected.iter()) {
        if want.is_some() && about != want {
            wrong.insert(name.clone(), (about.clone(), want.clone()));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} writes `--help` lines the specification does not carry. Each entry is `word: (emitted, \
         declared)`:\n  {:#?}\nRegenerate it with `ess generate synthesize --path ess/system \
         --target clap --out ess/generated/clap`",
        path.display(),
        wrong
    );
}

/// **Every `path:line` citation this unit wrote resolves to a line that exists.**
///
/// The class behind one finding. `ess/system/components.yaml` cited
/// `architecture_fence.rs:303`, which pointed at the right line until seven lines were inserted
/// above it and then pointed at a different test — and nothing said so, because
/// `crates/catalog-build/tests/main/ess_citation_fence.rs` checks that a citation is *present*,
/// never that it lands anywhere. Every citation in the two documents this unit owns is resolved
/// here instead: the file has to exist and the last line of the range has to be inside it.
///
/// It does not check that the line *means* what the sentence says — that is what
/// `crates/connectors-cli/tests/cli_surface_drift.rs::the_thin_frontend_citation_points_at_the_thin_frontend_test`
/// does for the one citation where the meaning is load-bearing. This catches the whole class of
/// citations pointing past the end of a file that shrank, or at a file that moved.
#[test]
fn every_citation_this_unit_wrote_resolves() {
    let root = repository_root();
    let mut broken = Vec::new();

    for document in [
        "ess/system/components.yaml",
        "docs/design/19-the-cli-surface.md",
        // The three test files' own doc comments, which cite the tree as freely as the two
        // documents above and had, until the second adversary pass, exactly the same defect.
        "crates/connectors-cli/tests/cli_surface.rs",
        "crates/connectors-cli/tests/cli_surface_drift.rs",
        "crates/connectors-cli/tests/cli_surface_pass_two.rs",
    ] {
        let text = read(&root.join(document));
        for (index, _) in text.match_indices(':') {
            // Walk back over the path: everything up to the delimiter that opened it.
            let head = &text[..index];
            let start = head
                .rfind(|character: char| {
                    character.is_whitespace() || character == '`' || character == '('
                })
                .map_or(0, |position| position + 1);
            let path = &head[start..];
            if !path.contains('/') || !(path.ends_with(".rs") || path.ends_with(".yaml")) {
                continue;
            }
            let numbers: String = text[index + 1..]
                .chars()
                .take_while(|character| character.is_ascii_digit() || *character == '-')
                .collect();
            let Some(last) = numbers
                .rsplit('-')
                .find(|part| !part.is_empty())
                .and_then(|part| part.parse::<usize>().ok())
            else {
                continue;
            };
            let cited = root.join(path);
            if !cited.is_file() {
                broken.push(format!(
                    "{document} cites `{path}:{numbers}`, and no such file exists"
                ));
                continue;
            }
            let lines = read(&cited).lines().count();
            if last > lines {
                broken.push(format!(
                    "{document} cites `{path}:{numbers}`, and that file has {lines} lines"
                ));
            }
        }
    }

    assert!(
        broken.is_empty(),
        "citations that no longer resolve:\n  {}",
        broken.join("\n  ")
    );
}

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
    // Under `setup`.
    (
        "setup init",
        Unspecified::Lifecycle,
        "writes a configuration file; no entity of this specification moves",
    ),
    (
        "setup connect",
        Unspecified::Flow,
        "a guided acquisition flow, not one declared command",
    ),
    (
        "setup completions",
        Unspecified::Unmodelled,
        "renders this binary's own command tree as a shell script; no command of this \
         specification names it",
    ),
    // Under `inspect`.
    (
        "inspect doctor",
        Unspecified::Read,
        "a read of the installation; no entity moves",
    ),
    (
        "inspect providers",
        Unspecified::Read,
        "a read of the embedded catalogue; ESS models a read as a view",
    ),
    (
        "inspect auth",
        Unspecified::Read,
        "a read of which configured providers have a credential stored",
    ),
    // Under `session`.
    (
        "session login",
        Unspecified::Lifecycle,
        "acquires an Identity session; no entity of this specification moves",
    ),
    (
        "session logout",
        Unspecified::Lifecycle,
        "discards an Identity session; no entity of this specification moves",
    ),
    // Under `serve`.
    (
        "serve local",
        Unspecified::Lifecycle,
        "starts a process; no entity moves",
    ),
    (
        "serve hosted",
        Unspecified::Lifecycle,
        "starts a process; no entity moves",
    ),
    (
        "serve mcp",
        Unspecified::Lifecycle,
        "serves a transport; no entity moves",
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
        "the parser carries paths the specification does not account for:\n  {}\nDeclare each in \
         the `cli:` block of ess/system/components.yaml. If the specification genuinely cannot \
         carry one, that is a fact about the specification and belongs in it: write the path into \
         the `unspecified-path:` enumeration of the `connectors-cli` note in that same document, \
         with the reason, and `UNSPECIFIED_PATHS` then has to carry it too — \
         `the_exception_list_is_the_set_the_specification_enumerates` holds the two together and \
         `an_exception_whose_kind_the_tree_contradicts_is_refused` holds the reason to the tree. \
         The list here is a projection, not a place to answer this.",
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

/// The documents whose `path:line` citations are held here: the two this unit wrote, and every
/// test file of this crate, whose doc comments cite the tree as freely as the two documents do and
/// had, until the second adversary pass, exactly the same defect. The directory is read rather than
/// listed: a list of three here is how a fourth and a fifth file came to carry citations nothing
/// read.
fn documents_this_unit_cites() -> Vec<String> {
    let root = repository_root();
    let mut documents = vec![
        "ess/system/components.yaml".to_owned(),
        "docs/design/19-the-cli-surface.md".to_owned(),
    ];
    let mut tests: Vec<String> = std::fs::read_dir(root.join("crates/connectors-cli/tests"))
        .expect("read crates/connectors-cli/tests")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .map(|path| {
            path.strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string()
        })
        .collect();
    tests.sort();
    assert!(
        tests.len() >= 5,
        "only {tests:?} were found under crates/connectors-cli/tests; the layout moved"
    );
    documents.extend(tests);
    documents
}

/// One `path:line` or `path:a-b` citation, and the backticked symbol it names, when it names one.
struct Citation {
    document: String,
    path: String,
    numbers: String,
    first: usize,
    last: usize,
    symbol: Option<String>,
}

/// The item kinds a backticked token may carry in front of its name: `` `enum Command` ``.
const KINDS: &[&str] = &[
    "fn",
    "struct",
    "enum",
    "trait",
    "type",
    "const",
    "static",
    "mod",
    "macro_rules!",
];

/// Every citation of `text`, found the way `every_citation_this_unit_wrote_resolves` has always
/// found them: a path with a `/` in it ending `.rs` or `.yaml`, a colon, digits.
///
/// A citation **names a symbol** when a backticked token stands immediately in front of it, with
/// nothing between the two but whitespace, a comment marker, an opening parenthesis, a comma and
/// the citation's own backtick — `` `product_cli_is_a_thin_frontend`
/// (`crates/catalog-build/tests/main/architecture_fence.rs:292`) ``. The token is an identifier,
/// on its own or after one of [`KINDS`]. A path in backticks is not one, so a citation in front of
/// a citation names nothing, and a word of prose between the two — "answered at", "and" — means
/// the sentence is not saying that the citation *is* the symbol.
fn citations_of(document: &str, text: &str) -> Vec<Citation> {
    let mut found = Vec::new();
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
        let mut parts = numbers
            .split('-')
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse::<usize>().ok());
        let Some(first) = parts.next() else {
            continue;
        };
        let last = parts.last().unwrap_or(first);
        found.push(Citation {
            document: document.to_owned(),
            path: path.to_owned(),
            numbers,
            first,
            last,
            symbol: symbol_named_before(&head[..start]),
        });
    }
    found
}

/// The backticked identifier that stands immediately in front of a citation, if one does.
fn symbol_named_before(before: &str) -> Option<String> {
    let before = before.strip_suffix('`').unwrap_or(before);
    let before = before.trim_end_matches(|character: char| {
        character.is_whitespace() || matches!(character, '#' | '/' | '(' | ',')
    });
    let before = before.strip_suffix('`')?;
    let identifier = before
        .rfind(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .map_or(0, |position| position + 1);
    let name = &before[identifier..];
    if name.is_empty() || name.starts_with(|character: char| character.is_ascii_digit()) {
        return None;
    }
    let opener = &before[..identifier];
    let backticked = opener.ends_with('`');
    let after_a_kind = KINDS
        .iter()
        .any(|kind| opener.ends_with(&format!("`{kind} ")));
    (backticked || after_a_kind).then(|| name.to_owned())
}

/// The 1-based lines on which `source` declares `name`.
///
/// In Rust: an item — `fn name(`, `pub enum name {`, `const name:` — after any visibility and
/// qualifier, or a variant or field at the head of a line, `name(`, `name {`, `name,`, `name: `.
/// In YAML: `- name: X` or `X:`.
fn declaration_lines(source: &str, name: &str, yaml: bool) -> Vec<usize> {
    const QUALIFIERS: &[&str] = &[
        "pub(crate) ",
        "pub(super) ",
        "pub(self) ",
        "pub ",
        "async ",
        "unsafe ",
        "extern \"C\" ",
        "default ",
    ];
    let declares = |line: &str| -> bool {
        let mut text = line.trim_start();
        if yaml {
            return text == format!("- name: {name}") || text.starts_with(&format!("{name}:"));
        }
        loop {
            let Some(qualifier) = QUALIFIERS
                .iter()
                .find(|qualifier| text.starts_with(*qualifier))
            else {
                break;
            };
            text = &text[qualifier.len()..];
        }
        for kind in KINDS {
            let Some(mut rest) = text.strip_prefix(&format!("{kind} ")) else {
                continue;
            };
            if *kind == "const" {
                if let Some(function) = rest.strip_prefix("fn ") {
                    rest = function;
                }
            }
            return rest.strip_prefix(name).is_some_and(|after| {
                !after.starts_with(|character: char| {
                    character.is_ascii_alphanumeric() || character == '_'
                })
            });
        }
        text.strip_prefix(name).is_some_and(|after| {
            after.starts_with(['(', '{', ','])
                || after.starts_with(": ")
                || after.starts_with(" {")
                || after.starts_with(" =")
                || after.starts_with(" (")
        })
    };
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| declares(line))
        .map(|(index, _)| index + 1)
        .collect()
}

/// The first line of the run of attributes and doc comments directly above `declaration`, both
/// 1-based: `#[test]` above `fn`, `#[derive(…)]` above `enum`, a `///` block above either.
fn span_start(lines: &[&str], declaration: usize) -> usize {
    let mut first = declaration;
    while first > 1 {
        let above = lines[first - 2].trim_start();
        if above.starts_with("#[")
            || above.starts_with("///")
            || above.starts_with("//!")
            || above.starts_with('#')
        {
            first -= 1;
        } else {
            break;
        }
    }
    first
}

/// Whether `line` carries `name` delimited on both sides by something that is not part of an
/// identifier: `"admin"` does, `administer` does not.
fn names_as_a_word(line: &str, name: &str) -> bool {
    line.match_indices(name).any(|(at, _)| {
        let is_part = |character: char| character.is_ascii_alphanumeric() || character == '_';
        !line[..at].chars().next_back().is_some_and(is_part)
            && !line[at + name.len()..].chars().next().is_some_and(is_part)
    })
}

/// **Every `path:line` citation this unit wrote resolves to a line that exists.**
///
/// The class behind one finding. `ess/system/components.yaml` cited
/// `architecture_fence.rs:303`, which pointed at the right line until seven lines were inserted
/// above it and then pointed at a different test — and nothing said so, because
/// `crates/catalog-build/tests/main/ess_citation_fence.rs` checks that a citation is *present*,
/// never that it lands anywhere. Every citation in the documents of
/// [`documents_this_unit_cites`] is resolved here instead: the file has to exist and the last line
/// of the range has to be inside it.
///
/// It does not check that the line *means* what the sentence says. For a citation that names a
/// symbol, `every_citation_that_names_a_symbol_lands_on_its_declaration` does; this catches the
/// whole class of citations pointing past the end of a file that shrank, or at a file that moved.
#[test]
fn every_citation_this_unit_wrote_resolves() {
    let root = repository_root();
    let mut broken = Vec::new();
    let mut resolved = 0usize;

    for document in documents_this_unit_cites() {
        let text = read(&root.join(&document));
        for citation in citations_of(&document, &text) {
            let cited = root.join(&citation.path);
            if !cited.is_file() {
                broken.push(format!(
                    "{document} cites `{}:{}`, and no such file exists",
                    citation.path, citation.numbers
                ));
                continue;
            }
            let lines = read(&cited).lines().count();
            if citation.last > lines {
                broken.push(format!(
                    "{document} cites `{}:{}`, and that file has {lines} lines",
                    citation.path, citation.numbers
                ));
                continue;
            }
            resolved += 1;
        }
    }

    assert!(
        resolved > 20,
        "only {resolved} citations were found in the documents this unit cites; the extraction is \
         wrong, and a check that finds nothing to look at passes without looking"
    );
    assert!(
        broken.is_empty(),
        "citations that no longer resolve:\n  {}",
        broken.join("\n  ")
    );
}

/// **A citation that names a symbol lands on that symbol's declaration.**
///
/// The class behind five findings in one day. `every_citation_this_unit_wrote_resolves` checks that
/// the cited line exists, and the first version of
/// `cli_surface_drift.rs::the_thin_frontend_citation_points_at_the_thin_frontend_test` checked that
/// one citation fell anywhere inside one function's body. Both passed
/// `architecture_fence.rs:319` for `product_cli_is_a_thin_frontend` — line 319 being
/// `"rpassword",` inside an array that function declares — and both passed `tree.rs:17-20` for
/// the generated `admin`, those lines being the generated `setup`. A line number that is inside the
/// file is almost always resolvable and almost never meaningful.
///
/// So a citation with a backticked symbol immediately in front of it has to land on that symbol.
/// A single line is the declaration line, or one of the attribute and doc-comment lines directly
/// above it — `#[test]` above `fn`, `#[derive]` above `enum`. A range contains the declaration
/// line, or starts in that run above it. Where the cited file declares nothing by that name — the
/// generated tree names `admin` only as `Command::new("admin")` — the first cited line has to carry
/// the name as a whole word, which is the least a citation can mean.
#[test]
fn every_citation_that_names_a_symbol_lands_on_its_declaration() {
    let root = repository_root();
    let mut wrong = Vec::new();
    let mut checked = 0usize;

    for document in documents_this_unit_cites() {
        let text = read(&root.join(&document));
        for citation in citations_of(&document, &text) {
            let Some(symbol) = &citation.symbol else {
                continue;
            };
            let cited = root.join(&citation.path);
            if !cited.is_file() {
                // `every_citation_this_unit_wrote_resolves` reports it.
                continue;
            }
            let source = read(&cited);
            let lines: Vec<&str> = source.lines().collect();
            checked += 1;
            let declarations = declaration_lines(&source, symbol, citation.path.ends_with(".yaml"));
            let lands = if declarations.is_empty() {
                lines
                    .get(citation.first - 1)
                    .is_some_and(|line| names_as_a_word(line, symbol))
            } else {
                declarations.iter().any(|&declaration| {
                    (span_start(&lines, declaration)..=declaration).contains(&citation.first)
                        || (citation.first..=citation.last).contains(&declaration)
                })
            };
            if !lands {
                wrong.push(format!(
                    "{} cites `{}:{}` as `{symbol}`; line {} is `{}`, and `{symbol}` is declared \
                     at {}",
                    citation.document,
                    citation.path,
                    citation.numbers,
                    citation.first,
                    lines.get(citation.first - 1).map_or("", |line| line.trim()),
                    if declarations.is_empty() {
                        "no line of that file".to_owned()
                    } else {
                        format!("{declarations:?}")
                    }
                ));
            }
        }
    }

    assert!(
        checked >= 8,
        "only {checked} citations naming a symbol were found in the documents this unit cites; \
         the extraction is wrong, and a check that finds nothing to look at passes without looking"
    );
    assert!(
        wrong.is_empty(),
        "citations that name a symbol and do not land on it:\n  {}\nCite the declaration line, or \
         the attribute above it; a line inside the body moves every time the body is edited.",
        wrong.join("\n  ")
    );
}

/// **Every declared group's `--help` line in the parser is the summary the specification
/// declares.**
///
/// `the_committed_generated_tree_is_the_specification_word_for_word` compares the *generated*
/// tree's `.about(…)` with the `cli:` block, and `scripts/gate.sh --final` regenerates and diffs
/// it. Neither reads the parser. So a group's doc comment in `crates/connectors-cli/src/lib.rs`
/// could be rewritten alone and the whole suite stayed green — measured, on the `Admin` variant —
/// while `connectors admin --help` printed a sentence the specification does not carry and
/// `connectors completions` shipped it to every shell.
///
/// This reads `clap::Command::get_about`, which is the sentence a person actually sees.
///
/// One transformation is modelled rather than asserted away: `clap_derive` drops a single trailing
/// period from the summary line of a doc comment, so `/// Search or receive durable normalized
/// data events.` reaches `get_about` without its full stop. The specification's summaries carry
/// one. Exactly that period is removed before the comparison — not trimmed, so a sentence ending
/// in an ellipsis still has to match — for the same reason
/// `the_committed_generated_tree_is_the_specification_word_for_word` compares `{:?}` literals: a
/// comparison that disagreed with the generator about what it emits would be measuring itself.
#[test]
fn every_declared_group_help_line_is_the_summary_the_specification_declares() {
    let surface = declared_surface();
    let parser = connectors_cli::command();

    let mut wrong = Vec::new();
    for group in &surface.groups {
        let summary = group
            .summary
            .as_deref()
            .unwrap_or_else(|| panic!("group `{}` declares a summary", group.name));
        let declared = summary.strip_suffix('.').unwrap_or(summary);
        let Some(command) = parser.find_subcommand(&group.name) else {
            continue;
        };
        let carried = command.get_about().map(ToString::to_string);
        if carried.as_deref() != Some(declared) {
            wrong.push(format!(
                "`{}`\n    parser:        {carried:?}\n    specification: {declared:?}",
                group.name
            ));
        }
    }

    assert!(
        wrong.is_empty(),
        "`connectors <group> --help` prints the parser's own sentence, and the completion script \
         is generated from the same tree, so a doc comment in crates/connectors-cli/src/lib.rs \
         that no longer matches the `cli:` block of ess/system/components.yaml is a surface the \
         specification does not describe:\n  {}",
        wrong.join("\n  ")
    );
}

/// **No word of the parser answers to a name the specification cannot declare.**
///
/// An alias is a word a person can type that the `cli:` block has no construct for, so
/// `ess generate synthesize` cannot emit it and `connectors completions` cannot offer it: it is a
/// surface that exists only in the parser. `#[command(alias = "conn")]` on `Connection` was added
/// and the whole suite stayed green, `connectors conn --help` worked, and the fish completion
/// listed the word zero times — a command-line surface nothing in this repository describes.
///
/// Both directions are already held for names; this holds the aliases, which are names too. If the
/// specification ever gains an alias construct, the `cli:` block declares them and this reads the
/// declaration instead of asserting there are none.
#[test]
fn no_word_of_the_parser_answers_to_a_name_the_specification_cannot_declare() {
    fn walk(command: &clap::Command, prefix: &str, found: &mut Vec<String>) {
        for child in command.get_subcommands() {
            let path = if prefix.is_empty() {
                child.get_name().to_owned()
            } else {
                format!("{prefix} {}", child.get_name())
            };
            for alias in child.get_all_aliases() {
                found.push(format!("`{path}` also answers to `{alias}`"));
            }
            for alias in child.get_all_short_flag_aliases() {
                found.push(format!("`{path}` also answers to `-{alias}`"));
            }
            for alias in child.get_all_long_flag_aliases() {
                found.push(format!("`{path}` also answers to `--{alias}`"));
            }
            walk(child, &path, found);
        }
    }
    let mut aliases = Vec::new();
    walk(&connectors_cli::command(), "", &mut aliases);

    assert!(
        aliases.is_empty(),
        "the `cli:` block of ess/system/components.yaml has no construct for an alias, so an \
         aliased word is one `ess generate synthesize` cannot emit and `connectors completions` \
         cannot offer — a word that works at a shell and appears in no completion script and in \
         no document:\n  {}\nRemove the alias, or declare it in the specification once ESS can \
         carry one and read the declaration here.",
        aliases.join("\n  ")
    );
}

/// Every `unspecified-path:` line of the `connectors-cli` note in `ess/system/components.yaml`,
/// as the document writes it: `<path> — <kind>`.
///
/// The specification's own enumeration of what its command-line surface carries and cannot yet
/// declare, **with the kind beside each path**. Read out of the document rather than restated
/// here, because a restatement is what `UNSPECIFIED_PATHS` already is and the point of this is to
/// have something behind it.
///
/// The kind is carried here and not only in the Rust list because an adversary pass measured what
/// the Rust list alone was worth: relabelling nineteen entries one at a time produced two
/// refusals. Thirteen of the twenty-seven kinds are derived from the tree by
/// [`kinds_the_tree_derives`] and rest on nothing anybody wrote down; the other fourteen are a
/// claim, and a claim belongs in the reviewed document rather than in the test that reads it.
fn paths_the_specification_names() -> BTreeSet<String> {
    let document = read(&repository_root().join("ess/system/components.yaml"));
    let named: BTreeSet<String> = document
        .lines()
        .filter_map(|line| {
            line.trim_start()
                .strip_prefix('#')?
                .trim_start()
                .strip_prefix("unspecified-path:")
                .map(|entry| entry.trim().to_owned())
        })
        .collect();
    assert!(
        !named.is_empty(),
        "no `unspecified-path:` line was read out of ess/system/components.yaml; the enumeration \
         moved or the marker was renamed, so read it again before believing any result from a \
         check that uses it"
    );
    let unshaped: Vec<&String> = named
        .iter()
        .filter(|entry| !entry.contains(" — "))
        .collect();
    assert!(
        unshaped.is_empty(),
        "every `unspecified-path:` line is `<path> — <kind>`; these carry no kind: {unshaped:?}"
    );
    named
}

/// Every command `ess/system/components.yaml` says a component accepts.
fn accepted_commands() -> BTreeSet<String> {
    let document = read(&repository_root().join("ess/system/components.yaml"));
    let mut accepted = BTreeSet::new();
    let mut inside = false;
    for line in document.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        if line.trim() == "commands:" {
            inside = true;
            continue;
        }
        let Some(name) = line.strip_prefix("        - ") else {
            if !line.trim().is_empty() {
                inside = false;
            }
            continue;
        };
        if inside {
            accepted.insert(name.trim().to_owned());
        }
    }
    assert!(
        accepted.contains("connectors.connection.ActivateCandidate"),
        "the `accepts.commands` extraction found {} entries and not the one this file names in a \
         reason; the block moved, so read it again: {accepted:?}",
        accepted.len()
    );
    accepted
}

/// `true` where any document of `ess/system` declares a real `views:` block.
///
/// The stated reason a read cannot be declared. It is a countdown too: the moment ESS carries a
/// consistency claim and this specification states one, every `Read` entry has an answer and the
/// exception stops being one.
fn the_specification_declares_a_view() -> bool {
    let root = repository_root().join("ess/system");
    let mut stack = vec![root];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current).expect("ess/system").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|name| name.to_str()) != Some("yaml") {
                continue;
            }
            if read(&path)
                .lines()
                .any(|line| !line.trim_start().starts_with('#') && line.trim() == "views:")
            {
                return true;
            }
        }
    }
    false
}

/// The word a person would type for each command the specification declares.
///
/// `docs/design/19-the-cli-surface.md` states the rule and the clap target implements it: a
/// command's typed word is its `naming.wire`, or the qualified name's last segment, verbatim and
/// un-cased. So this is the set of words the specification *can* name, which is what decides
/// whether a path is really an act it does not model.
///
/// **Un-cased means untouched.** An earlier revision lowercased the last segment, which put
/// `supervisechannel` in this set and none of the eleven CamelCase words both documents name as
/// what a whole-surface `cli:` block produced — so the set was disjoint from the words the rule
/// is about, and the `Unmodelled` contradiction was inert for every command carrying no
/// `naming.wire`. `adversary_fence_probe.rs` reads the eleven out of the design document and is
/// what holds this.
fn words_the_specification_can_type() -> BTreeSet<String> {
    let root = repository_root().join("ess/system/domains");
    let mut words = BTreeSet::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("ess/system/domains")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|name| name.to_str()) == Some("yaml"))
        .collect();
    entries.sort();
    for path in entries {
        let document = read(&path);
        let lines: Vec<&str> = document.lines().collect();
        let commands = lines
            .iter()
            .position(|line| *line == "commands:")
            .unwrap_or(lines.len());
        let mut current: Option<String> = None;
        for line in lines.iter().skip(commands + 1) {
            let top_level_key =
                !line.is_empty() && !line.starts_with(' ') && !line.starts_with('#');
            if top_level_key {
                break;
            }
            if let Some(name) = line.strip_prefix("  - name: ") {
                if let Some(previous) = current.take() {
                    words.insert(previous);
                }
                current = name.trim().rsplit('.').next().map(str::to_owned);
            }
            if let Some(wire) = line.strip_prefix("      wire: ") {
                current = Some(wire.trim().to_owned());
            }
        }
        if let Some(previous) = current {
            words.insert(previous);
        }
    }
    assert!(
        words.contains("invoke")
            && words.contains("materialize")
            && words.contains("SuperviseChannel"),
        "the typed-word extraction read {words:?} out of ess/system/domains and not the two `naming.wire` values and the one un-cased last segment this file names; the blocks moved, so read them again"
    );
    words
}

/// The `naming.wire` of every command some component of the specification accepts.
///
/// A path that forwards one of these is `Forwarded`, and nothing about the sentence beside it
/// decides that: the wire is the `method` tag on the frame the handler builds.
fn wires_of_accepted_commands() -> BTreeSet<String> {
    let accepted = accepted_commands();
    let root = repository_root().join("ess/system/domains");
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("ess/system/domains")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|name| name.to_str()) == Some("yaml"))
        .collect();
    entries.sort();
    let mut wires = BTreeSet::new();
    for path in entries {
        let document = read(&path);
        let mut current: Option<String> = None;
        for line in document.lines() {
            if let Some(name) = line.strip_prefix("  - name: ") {
                current = Some(name.trim().to_owned());
            }
            if let Some(wire) = line.strip_prefix("      wire: ") {
                if current
                    .as_deref()
                    .is_some_and(|name| accepted.contains(name))
                {
                    wires.insert(wire.trim().to_owned());
                }
            }
        }
    }
    assert!(
        wires.contains("materialize"),
        "the accepted-command wires were read as {wires:?} and do not carry `materialize`; the \
         `accepts.commands` block or the `naming.wire` lines moved, so read them again"
    );
    wires
}

/// The variant names of every `pub enum …Request` of `crates/protocol/src`, by module.
///
/// Derived, not listed. An earlier revision of this file named the three request enums the
/// command line speaks as literals, which is the shape `deployment_protocol_modules` was corrected
/// away from in this same diff: a literal can only ever shrink the set, so a fourth protocol would
/// be invisible to every check below rather than refused by one.
fn protocol_request_variants() -> BTreeMap<String, BTreeSet<String>> {
    let directory = repository_root().join("crates/protocol/src");
    let mut declared = BTreeMap::new();
    for entry in std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()))
    {
        let path = entry.expect("directory entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let source = read(&path);
        let lines: Vec<&str> = source.lines().collect();
        let Some(open) = lines.iter().position(|line| {
            line.strip_prefix("pub enum ").is_some_and(|rest| {
                rest.split(|character: char| !character.is_alphanumeric())
                    .next()
                    .is_some_and(|name| name.ends_with("Request"))
            })
        }) else {
            continue;
        };
        let close = open
            + 1
            + lines[open + 1..]
                .iter()
                .position(|line| *line == "}")
                .expect("the request enum closes");
        let variants: BTreeSet<String> = lines[open + 1..close]
            .iter()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with('#'))
            .map(|line| {
                line.chars()
                    .take_while(|character| character.is_ascii_alphanumeric())
                    .collect::<String>()
            })
            .filter(|name| !name.is_empty())
            .collect();
        declared.insert(
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .expect("a module name")
                .to_owned(),
            variants,
        );
    }
    assert!(
        declared.len() >= 5 && declared["connection"].contains("CandidateSearch"),
        "the request enums of crates/protocol/src were read as {declared:?}; the modules moved, \
         so read them again before believing any result from a check that uses this"
    );
    declared
}

/// The three protocol modules the read-verb marker of `ess/system/components.yaml` cites.
///
/// Read out of the marker itself rather than named here, so the partition below is over exactly
/// the protocols the specification says it is talking about.
fn protocols_the_read_verb_marker_cites() -> BTreeSet<String> {
    let document = read(&repository_root().join("ess/system/components.yaml"));
    let sentence = document
        .split("The read verbs of the three protocols —")
        .nth(1)
        .expect("ess/system/components.yaml enumerates the read verbs of the three protocols")
        .split("— change no entity")
        .next()
        .expect("the enumeration closes with `— change no entity`");
    let cited: BTreeSet<String> = sentence
        .match_indices("crates/protocol/src/")
        .map(|(index, marker)| {
            sentence[index + marker.len()..]
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect::<String>()
        })
        .filter(|module| !module.is_empty())
        .collect();
    assert_eq!(
        cited.len(),
        3,
        "the read-verb marker cites {cited:?} and it is a claim about three protocols; the marker \
         moved, so read it again"
    );
    cited
}

/// The read verbs `ess/system/components.yaml` enumerates: the request variants of the three
/// protocols that "change no entity".
///
/// This sentence decides eight of the derived kinds, so it is not left as prose nobody opens:
/// `the_read_verb_enumeration_partitions_the_protocols_it_names` holds it against the enums it
/// cites, the same way `every_declared_wire_name_is_a_method_the_protocol_accepts` holds a
/// `naming.wire` against the enum that would decode it.
fn read_verbs_the_specification_names() -> BTreeSet<String> {
    let document = read(&repository_root().join("ess/system/components.yaml"));
    let sentence = document
        .split("The read verbs of the three protocols —")
        .nth(1)
        .expect("ess/system/components.yaml enumerates the read verbs of the three protocols")
        .split("— change no entity")
        .next()
        .expect("the enumeration closes with `— change no entity`");
    let verbs: BTreeSet<String> = sentence
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|word| !word.contains('/') && !word.contains(':'))
        .map(str::to_owned)
        .collect();
    assert!(
        verbs.contains("Search") && verbs.contains("Receive"),
        "the read-verb enumeration was read as {verbs:?}; the marker moved, so read it again"
    );
    verbs
}

/// The request variants `ess/system/components.yaml` names as neither an accepted command nor a
/// read — the third part of the partition, written down so it cannot absorb a verb silently.
fn neither_command_nor_read() -> BTreeSet<String> {
    let document = read(&repository_root().join("ess/system/components.yaml"));
    let sentence = document
        .split("Neither an accepted command nor a read —")
        .nth(1)
        .expect("ess/system/components.yaml names what is neither an accepted command nor a read")
        .split("— sends")
        .next()
        .expect("that sentence closes with `— sends`");
    let named: BTreeSet<String> = sentence
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|word| !word.contains('/') && !word.contains(':'))
        .map(str::to_owned)
        .collect();
    assert!(
        !named.is_empty(),
        "the `neither an accepted command nor a read` marker names nothing; it moved, so read it \
         again"
    );
    named
}

/// Every method of `impl LocalClient`, by name, with its body.
fn local_client_methods() -> BTreeMap<String, String> {
    let source = read(&repository_root().join("crates/connectors-client/src/lib.rs"));
    let lines: Vec<&str> = source.lines().collect();
    let open = lines
        .iter()
        .position(|line| *line == "impl LocalClient {")
        .expect("crates/connectors-client/src/lib.rs declares `impl LocalClient`");
    let close = open
        + 1
        + lines[open + 1..]
            .iter()
            .position(|line| *line == "}")
            .expect("the `impl LocalClient` block closes");

    let mut starts: Vec<(usize, String)> = Vec::new();
    for (offset, line) in lines[open + 1..close].iter().enumerate() {
        let Some(rest) = line.strip_prefix("    ") else {
            continue;
        };
        let rest = rest.strip_prefix("pub ").unwrap_or(rest);
        let rest = rest.strip_prefix("async ").unwrap_or(rest);
        let Some(rest) = rest.strip_prefix("fn ") else {
            continue;
        };
        let name: String = rest
            .chars()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        if !name.is_empty() {
            starts.push((open + 1 + offset, name));
        }
    }
    assert!(
        starts.len() >= 8,
        "only {} methods were read out of `impl LocalClient`; the block moved, so read it again",
        starts.len()
    );
    let mut bodies = BTreeMap::new();
    for (position, (index, name)) in starts.iter().enumerate() {
        let stop = starts.get(position + 1).map_or(close, |(next, _)| *next);
        bodies.insert(name.clone(), lines[*index..stop].join("\n"));
    }
    bodies
}

/// Every `crates/protocol` request variant a body constructs, by the qualified path the tree
/// writes: `<Module>Request::<Variant>`.
fn requests_built_by(body: &str, enums: &BTreeMap<String, BTreeSet<String>>) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for module in enums.keys() {
        let mut marker = String::new();
        let mut characters = module.chars();
        if let Some(first) = characters.next() {
            marker.extend(first.to_uppercase());
        }
        marker.push_str(characters.as_str());
        marker.push_str("Request::");
        for (index, _) in body.match_indices(&marker) {
            let variant: String = body[index + marker.len()..]
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect();
            if !variant.is_empty() {
                found.insert(variant);
            }
        }
    }
    found
}

/// **The kind of an exception path, where the tree decides it rather than a sentence.**
///
/// Every path is followed to the frames it puts on a socket, and the set of those frames settles
/// the kind with no prose in the way except the partition below, which is itself checked:
///
/// * **several** requests — a guided sequence of steps, not one declared command: `Flow`;
/// * **one**, whose `method` tag is the `naming.wire` of a command the specification says a
///   component accepts — the path **forwards** it: `Forwarded`;
/// * **one**, which is a read verb `ess/system/components.yaml` enumerates as changing no entity:
///   `Read`;
/// * **one**, which that document names as neither: `Unmodelled`;
/// * **none** — the tree says nothing, and `docs/design/19-the-cli-surface.md` names the paths
///   this leaves.
///
/// **The callee is followed one hop.** An adversary pass measured what stopping at the dispatch
/// arm cost: `connectors connect` hands off to `connect::dispatch` in `crates/connectors-console`,
/// which drives a `LocalClient` and reaches seven request variants, three of them accepted
/// commands — while this derivation saw an arm that builds nothing and the design document said
/// the tree was silent about it. One hop is enough for this tree, because
/// `crates/catalog-build/tests/main/architecture_fence.rs::product_cli_is_a_thin_frontend` is what
/// keeps the frontend thin: an arm hands the work to one `connectors-console` module and that
/// module talks to the client. If that stops being true, the count guard below fires rather than
/// this quietly deriving less.
fn kinds_the_tree_derives() -> BTreeMap<String, Unspecified> {
    /// `CandidateActivate` -> `candidate_activate`, which is what `rename_all = "snake_case"` does.
    fn snake_case(variant: &str) -> String {
        let mut out = String::new();
        for (index, character) in variant.chars().enumerate() {
            if character.is_ascii_uppercase() {
                if index > 0 {
                    out.push('_');
                }
                out.push(character.to_ascii_lowercase());
            } else {
                out.push(character);
            }
        }
        out
    }
    /// `ServeHosted` -> `serve-hosted`, which is what `clap_derive` does to a variant identifier.
    fn kebab_case(variant: &str) -> String {
        snake_case(variant).replace('_', "-")
    }

    let root = repository_root();
    let whole = read(&root.join("crates/connectors-cli/src/lib.rs"));
    // The parser and its dispatch only. The test module below repeats `Command::Connect` in a
    // pattern, and reading it as a dispatch arm would split the real one.
    let source = whole.split("\n#[cfg(test)]").next().unwrap_or(&whole);
    let enums = protocol_request_variants();
    let wires = wires_of_accepted_commands();
    let reads = read_verbs_the_specification_names();
    let neither = neither_command_nor_read();
    let methods = local_client_methods();

    // What each `connectors-console` module reaches: what it builds itself, and what the
    // `LocalClient` methods it calls build.
    let mut modules: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let console = root.join("crates/connectors-console/src");
    for entry in std::fs::read_dir(&console)
        .unwrap_or_else(|error| panic!("read {}: {error}", console.display()))
    {
        let path = entry.expect("directory entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let text = read(&path);
        let mut reached = requests_built_by(&text, &enums);
        for (name, body) in &methods {
            if text.contains(&format!(".{name}(")) {
                reached.extend(requests_built_by(body, &enums));
            }
        }
        modules.insert(
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .expect("a module name")
                .to_owned(),
            reached,
        );
    }
    assert!(
        modules.values().any(|reached| reached.len() > 1),
        "no crates/connectors-console module reaches more than one protocol request; the modules \
         or `impl LocalClient` moved, so read them again"
    );

    // Every dispatch arm: `Command::<Leaf>` at the top level, and `<Group>Command::<Leaf>` under a
    // declared group whose enum this file carries. The group names come from the `cli:` block, so
    // a fourth group is picked up rather than missed by a literal.
    let components = read(&root.join("ess/system/components.yaml"));
    let mut groups: Vec<String> = Vec::new();
    let mut inside = false;
    for line in components.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        if line.trim() == "groups:" {
            inside = true;
            continue;
        }
        match line.strip_prefix("        - name: ") {
            Some(name) if inside => groups.push(name.trim().to_owned()),
            _ => {
                if !line.trim().is_empty() && !line.starts_with("          ") {
                    inside = false;
                }
            }
        }
    }
    assert!(
        groups.len() >= 5,
        "the `cli:` block was read as {groups:?}; it moved, so read it again"
    );
    let mut prefixes: Vec<(String, String)> = vec![(String::new(), "Command::".to_owned())];
    for group in groups {
        let mut camel = String::new();
        let mut characters = group.chars();
        if let Some(first) = characters.next() {
            camel.extend(first.to_uppercase());
        }
        camel.push_str(characters.as_str());
        let marker = format!("{camel}Command::");
        if source.contains(&format!("enum {camel}Command")) {
            prefixes.push((group, marker));
        }
    }

    let mut marks: Vec<(usize, String, String)> = Vec::new();
    for (group, marker) in &prefixes {
        for (index, _) in source.match_indices(marker.as_str()) {
            let preceded = source[..index]
                .chars()
                .next_back()
                .is_some_and(|character| character.is_alphanumeric() || character == '_');
            if preceded {
                continue;
            }
            let leaf: String = source[index + marker.len()..]
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect();
            if !leaf.is_empty() {
                marks.push((index, group.clone(), leaf));
            }
        }
    }
    marks.sort_by_key(|(index, _, _)| *index);

    let mut derived = BTreeMap::new();
    for (position, (index, group, leaf)) in marks.iter().enumerate() {
        let end = marks
            .get(position + 1)
            .map_or(source.len(), |(next, _, _)| *next);
        let arm = &source[*index..end];
        let mut reached = requests_built_by(arm, &enums);
        for (module, requests) in &modules {
            if arm.contains(&format!("{module}::")) {
                reached.extend(requests.iter().cloned());
            }
        }
        let path = if group.is_empty() {
            kebab_case(leaf)
        } else {
            format!("{group} {}", kebab_case(leaf))
        };
        let kind = match reached.len() {
            0 => continue,
            1 => {
                let variant = reached.iter().next().expect("one request");
                if wires.contains(&snake_case(variant)) {
                    Unspecified::Forwarded
                } else if reads.contains(variant) {
                    Unspecified::Read
                } else if neither.contains(variant) {
                    Unspecified::Unmodelled
                } else {
                    panic!(
                        "`{path}` builds `{variant}`, which ess/system/components.yaml places in \
                         none of its three parts — not an accepted command's `naming.wire`, not a \
                         read verb, and not named as neither. The partition is checked by \
                         `the_read_verb_enumeration_partitions_the_protocols_it_names`"
                    )
                }
            }
            _ => Unspecified::Flow,
        };
        derived.entry(path).or_insert(kind);
    }

    assert!(
        derived.len() >= 13,
        "only {} dispatch arms of crates/connectors-cli/src/lib.rs were followed to a protocol \
         request; the dispatch moved, so read it again before believing any result from a check \
         that uses this: {derived:?}",
        derived.len()
    );
    derived
}

/// Every qualified command id a reason names, as `connectors.<domain>.<Command>`.
fn commands_named_by(reason: &str) -> Vec<String> {
    reason
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '.'))
        .filter(|word| {
            let mut segments = word.split('.');
            segments.next() == Some("connectors")
                && segments.next().is_some_and(|domain| !domain.is_empty())
                && segments
                    .next()
                    .is_some_and(|name| name.starts_with(|c: char| c.is_ascii_uppercase()))
                && segments.next().is_none()
        })
        .map(str::to_owned)
        .collect()
}

/// **Every refusal the exception list's own contract would raise, for one list.**
///
/// Taken as an argument so a drift can be handed a list the repository does not ship, which is the
/// only way to measure what the list refuses without editing the file that declares it.
///
/// Four things are checked.
///
/// **The path and its kind have to be the pair `ess/system/components.yaml` enumerates.** Before
/// it, the list answered to nothing — a command added to the parser could be absorbed by one entry
/// with a sentence nobody could check, and the refusal that would have caught it *instructed
/// exactly that*. The list is a projection of the specification, and adding to it or relabelling
/// an entry in it means editing the document the whole contract is about.
///
/// **Where the tree decides the kind, the tree decides it.** [`kinds_the_tree_derives`] settles
/// thirteen of the twenty-seven off the dispatch in `crates/connectors-cli/src/lib.rs`, the accepted
/// commands' `naming.wire` values and the read verbs the specification enumerates. An entry whose
/// kind disagrees is refused whatever either document says.
///
/// **`Grouping` is decided by the parser**, and no other kind may carry subcommands.
///
/// **The reason is held to what it names.** A `Forwarded` reason has to name a command some
/// component accepts; a reason that names one may not be labelled anything else.
///
/// An adversary pass measured what the last two were worth on their own: relabelling each of the
/// non-`Lifecycle` entries the list then carried `Lifecycle`, with a sentence naming no command,
/// was refused for two. The kind column of the specification and the thirteen derivations are the
/// answer to that, and `docs/design/19-the-cli-surface.md` states which of the twenty-seven each
/// one covers.
fn exception_list_refusals(
    entries: &[(&str, Unspecified, &str)],
    parser: &clap::Command,
    named: &BTreeSet<String>,
    accepted: &BTreeSet<String>,
    typeable: &BTreeSet<String>,
    declares_a_view: bool,
) -> Vec<String> {
    /// `Unspecified::Read` -> `read`, which is how the specification's enumeration spells it.
    fn spelling(kind: &Unspecified) -> String {
        format!("{kind:?}").to_ascii_lowercase()
    }

    let listed: BTreeSet<String> = entries
        .iter()
        .map(|(path, kind, _)| format!("{path} — {}", spelling(kind)))
        .collect();
    let paths_named: BTreeSet<&str> = named
        .iter()
        .filter_map(|entry| entry.split_once(" — "))
        .map(|(path, _)| path)
        .collect();
    let mut refusals = Vec::new();

    for entry in listed.difference(named) {
        let (path, kind) = entry.split_once(" — ").unwrap_or((entry.as_str(), "?"));
        if paths_named.contains(path) {
            let declared = named
                .iter()
                .find(|line| line.starts_with(&format!("{path} — ")))
                .map_or("?", |line| line.split_once(" — ").map_or("?", |(_, k)| k));
            refusals.push(format!(
                "`{path}` is excused as `{kind}` and ess/system/components.yaml records it as \
                 `{declared}`. The kind is a claim about what this path is, and it is recorded in \
                 the specification so that changing it is an edit to the reviewed document rather \
                 than to the list that reads it"
            ));
        } else {
            refusals.push(format!(
                "`{path}` is excused in UNSPECIFIED_PATHS and ess/system/components.yaml does not \
                 name it. Declare the command in the `cli:` block, or write `{path} — {kind}` into \
                 the `unspecified-path:` enumeration of that document with the reason it cannot be \
                 declared — the list here is a projection of the specification and is not the \
                 place a new word is answered"
            ));
        }
    }
    for entry in named.difference(&listed) {
        let (path, _) = entry.split_once(" — ").unwrap_or((entry.as_str(), "?"));
        if listed
            .iter()
            .any(|line| line.starts_with(&format!("{path} — ")))
        {
            continue;
        }
        refusals.push(format!(
            "ess/system/components.yaml names `{entry}` as a path this specification cannot \
             declare, and UNSPECIFIED_PATHS does not carry it"
        ));
    }

    let derived = kinds_the_tree_derives();
    for (path, kind, _) in entries {
        let Some(tree) = derived.get(*path) else {
            continue;
        };
        if tree != kind {
            refusals.push(format!(
                "`{path}` is excused as `{kind:?}` and the tree says `{tree:?}`. Its dispatch arm \
                 in crates/connectors-cli/src/lib.rs builds a crates/protocol request, and the \
                 method on that frame is what decides: an accepted command's `naming.wire` is \
                 `Forwarded`, a read verb ess/system/components.yaml enumerates is `Read`, and \
                 neither is `Unmodelled`. No sentence in this list changes that"
            ));
        }
    }

    let carried = parser_paths(parser);
    for (path, kind, reason) in entries {
        let leaf = path.rsplit(' ').next().unwrap_or(path);
        let subcommands = path
            .split(' ')
            .try_fold(parser, |command, word| command.find_subcommand(word))
            .is_some_and(|command| command.get_subcommands().next().is_some());
        let named_commands = commands_named_by(reason);

        if !carried.contains(*path) {
            continue;
        }
        match kind {
            Unspecified::Grouping if !subcommands => refusals.push(format!(
                "`{path}` is excused as a grouping and the parser's word carries no subcommand"
            )),
            Unspecified::Grouping => {}
            _ if subcommands => refusals.push(format!(
                "`{path}` carries subcommands and is excused as `{kind:?}`; a word that only \
                 carries other words is `Grouping`"
            )),
            Unspecified::Forwarded => {
                let unaccepted: Vec<&String> = named_commands
                    .iter()
                    .filter(|name| !accepted.contains(*name))
                    .collect();
                if named_commands.is_empty() {
                    refusals.push(format!(
                        "`{path}` is excused as forwarded and its reason names no command of this \
                         specification. A forwarded path forwards something: name it as \
                         `connectors.<domain>.<Command>`, and ess/system/components.yaml has to \
                         say a component accepts it"
                    ));
                } else if !unaccepted.is_empty() {
                    refusals.push(format!(
                        "`{path}` is excused as forwarding {unaccepted:?}, which no `accepts.commands` \
                         block of ess/system/components.yaml carries — so it forwards nothing this \
                         specification declares"
                    ));
                }
            }
            Unspecified::Read if declares_a_view => refusals.push(format!(
                "`{path}` is excused because ESS models a read as a `views:` declaration and this \
                 specification states none. It states one now, so this read has an answer and the \
                 exception is over"
            )),
            Unspecified::Read
            | Unspecified::Lifecycle
            | Unspecified::Flow
            | Unspecified::Unmodelled => {
                if !named_commands.is_empty() {
                    refusals.push(format!(
                        "`{path}` is excused as `{kind:?}` and its reason names {named_commands:?}, \
                         which this specification declares. A path whose reason names a command is \
                         `Forwarded`, and a `{kind:?}` reason that names one is describing a \
                         different entry"
                    ));
                }
                if matches!(kind, Unspecified::Unmodelled) && typeable.contains(leaf) {
                    refusals.push(format!(
                        "`{path}` is excused as an act no command of this specification names, and \
                         this specification names a command whose typed word is `{leaf}`"
                    ));
                }
            }
        }
    }
    refusals
}

/// **The exception list is the set the specification enumerates, and every kind is one the tree
/// answers.**
#[test]
fn the_exception_list_is_the_set_the_specification_enumerates() {
    let refusals = exception_list_refusals(
        UNSPECIFIED_PATHS,
        &connectors_cli::command(),
        &paths_the_specification_names(),
        &accepted_commands(),
        &words_the_specification_can_type(),
        the_specification_declares_a_view(),
    );
    assert!(
        refusals.is_empty(),
        "the exception list and ess/system/components.yaml disagree about what this binary \
         carries and cannot declare:\n  {}",
        refusals.join("\n  ")
    );
}

/// **A command added to the parser and absorbed into the exception list is refused.**
///
/// The construction an independent review made: `connection prune` added to the parser, then
/// absorbed by one `Unmodelled` entry with an arbitrary reason, its copy in the drift suite, and
/// two numbers in `docs/design/19-the-cli-surface.md`. Twenty-five of twenty-five green,
/// `ess/system` and `ess/generated` byte-identical — and the refusal that should have caught it
/// named the exception list as the first thing to do about it.
#[test]
fn a_command_absorbed_into_the_exception_list_alone_is_refused() {
    let drifted = connectors_cli::command().mut_subcommand("connection", |group| {
        group.subcommand(clap::Command::new("prune").about("added by nobody's decision"))
    });
    let mut absorbed = UNSPECIFIED_PATHS.to_vec();
    absorbed.push((
        "connection prune",
        Unspecified::Unmodelled,
        "drops stale Connection summaries; no command of this specification names it",
    ));

    let refusals = exception_list_refusals(
        &absorbed,
        &drifted,
        &paths_the_specification_names(),
        &accepted_commands(),
        &words_the_specification_can_type(),
        the_specification_declares_a_view(),
    );
    assert!(
        refusals
            .iter()
            .any(|refusal| refusal.contains("`connection prune` is excused in UNSPECIFIED_PATHS")),
        "`connection prune` was added to the parser and absorbed into the exception list, and \
         ess/system/components.yaml was left alone; the contract raised: {refusals:?}"
    );
}

/// **An exception whose kind the tree contradicts is refused, for each kind the tree can answer.**
///
/// The second half of the same finding — "no check ties a `Forwarded` or `Read` reason to
/// anything". Each case below writes a reason that reads perfectly well and is false about the
/// tree, which is what an arbitrary reason is.
#[test]
fn an_exception_whose_kind_the_tree_contradicts_is_refused() {
    let parser = connectors_cli::command();
    let named = paths_the_specification_names();
    let accepted = accepted_commands();
    let typeable = words_the_specification_can_type();
    let refuses = |entries: &[(&str, Unspecified, &str)], marker: &str| {
        let refusals =
            exception_list_refusals(entries, &parser, &named, &accepted, &typeable, false);
        assert!(
            refusals.iter().any(|refusal| refusal.contains(marker)),
            "an exception the tree contradicts was not refused; expected a refusal naming \
             `{marker}` and got: {refusals:?}"
        );
    };

    let mut forwards_nothing = UNSPECIFIED_PATHS.to_vec();
    for entry in &mut forwards_nothing {
        if entry.0 == "connection activate" {
            entry.2 = "forwards `connectors.connection.PruneConnections`, which \
                       `connectors-service` accepts";
        }
    }
    refuses(
        &forwards_nothing,
        "forwards nothing this specification declares",
    );

    let mut forwards_unnamed = UNSPECIFIED_PATHS.to_vec();
    for entry in &mut forwards_unnamed {
        if entry.0 == "connection materialize" {
            entry.2 = "the service handles it and this frontend passes it along";
        }
    }
    refuses(&forwards_unnamed, "names no command of this specification");

    let mut read_of_a_command = UNSPECIFIED_PATHS.to_vec();
    for entry in &mut read_of_a_command {
        if entry.0 == "connection list" {
            entry.2 = "a read that forwards `connectors.connection.ActivateCandidate`";
        }
    }
    refuses(&read_of_a_command, "which this specification declares");

    let mut wrong_shape = UNSPECIFIED_PATHS.to_vec();
    for entry in &mut wrong_shape {
        if entry.0 == "admin credentials" {
            entry.1 = Unspecified::Lifecycle;
        }
    }
    refuses(&wrong_shape, "carries subcommands and is excused as");

    let mut modelled_act = UNSPECIFIED_PATHS.to_vec();
    for entry in &mut modelled_act {
        if entry.0 == "operation invoke" {
            entry.1 = Unspecified::Unmodelled;
            entry.2 = "starts a call; no command of this specification names it";
        }
    }
    refuses(
        &modelled_act,
        "this specification names a command whose typed word is `invoke`",
    );

    // The one the tree answers on its own. `connection list` builds a `ConnectionRequest::Search`,
    // which `ess/system/components.yaml` enumerates as a read verb that changes no entity, so it
    // is a `Read` whatever anybody writes beside it. Nothing here volunteers a string that
    // incriminates the entry: the kind moves and the reason stays a sentence about a read.
    let mut relabelled = UNSPECIFIED_PATHS.to_vec();
    for entry in &mut relabelled {
        if entry.0 == "connection list" {
            entry.1 = Unspecified::Lifecycle;
        }
    }
    refuses(
        &relabelled,
        "`connection list` is excused as `Lifecycle` and the tree says `Read`",
    );
}

/// **A read that has gained a `views:` declaration stops being an exception.**
///
/// The countdown half. Every `Read` entry is excused by one sentence — that ESS models a read as a
/// `views:` declaration and this specification states none — and the day that stops being true,
/// every one of them has an answer. Nothing would have said so.
#[test]
fn a_read_stops_being_an_exception_once_the_specification_declares_a_view() {
    let refusals = exception_list_refusals(
        UNSPECIFIED_PATHS,
        &connectors_cli::command(),
        &paths_the_specification_names(),
        &accepted_commands(),
        &words_the_specification_can_type(),
        true,
    );
    assert!(
        refusals
            .iter()
            .any(|refusal| refusal.contains("It states one now, so this read has an answer")),
        "the specification was given a `views:` declaration and no `Read` exception was called \
         in; refusals were: {refusals:?}"
    );
}

/// **The command that regenerates the committed tree is the command this repository runs, and the
/// emitted headers still name one it does not.**
///
/// Every generated file carries a `do not edit: regenerate with …` line, and none of them names a
/// command that exists here: `tree.rs`, `handler.rs`, `main.rs` and the emitted `Cargo.toml` say
/// `cargo xtask synth --target clap`, and `PLAN.md` and `TARGET.md` say `ess synthesize`. There is
/// no `xtask` in this repository and `ess` has no `synthesize` subcommand. The command is
/// `ess generate synthesize --path ess/system --target clap --out ess/generated/clap`, which is
/// what `scripts/gate.sh` runs and diffs the committed bytes against.
///
/// Those headers cannot be corrected here: they are compared byte for byte with a fresh
/// generation, so editing one fails the gate. What can be done is to say so where a reader looks,
/// and to hold that note to still being needed — the second half below is pinned to the defect
/// existing, so the day `ess` writes the real command this case fails and the paragraph in
/// `docs/design/19-the-cli-surface.md` goes with it rather than outliving what it describes.
#[test]
fn the_regeneration_command_the_documents_name_is_the_one_the_gate_runs() {
    const COMMAND: &str = "ess generate synthesize --path ess/system --target clap";
    let root = repository_root();

    let script = read(&root.join("scripts/gate.sh"));
    assert!(
        script.contains(COMMAND),
        "scripts/gate.sh no longer runs `{COMMAND}`, so the command named in \
         docs/design/19-the-cli-surface.md and in this test is not the one that regenerates the \
         committed tree"
    );
    let document = read(&root.join("docs/design/19-the-cli-surface.md"));
    assert!(
        document.contains(COMMAND),
        "docs/design/19-the-cli-surface.md has to name `{COMMAND}`; the generated files name two \
         commands this repository does not have, and this page is the only place a reader finds \
         the real one"
    );

    let mut wrong = Vec::new();
    for (file, named) in [
        (
            "ess/generated/clap/crates/connectors-cli/src/tree.rs",
            "cargo xtask synth",
        ),
        (
            "ess/generated/clap/crates/connectors-cli/src/handler.rs",
            "cargo xtask synth",
        ),
        (
            "ess/generated/clap/crates/connectors-cli/src/main.rs",
            "cargo xtask synth",
        ),
        (
            "ess/generated/clap/crates/connectors-cli/Cargo.toml",
            "cargo xtask synth",
        ),
        ("ess/generated/clap/PLAN.md", "ess synthesize"),
        ("ess/generated/clap/TARGET.md", "ess synthesize"),
    ] {
        if read(&root.join(file)).contains(named) {
            wrong.push(format!("{file} says `{named}`"));
        }
    }
    assert_eq!(
        wrong.len(),
        6,
        "the emitted files are supposed to still name a regeneration command this repository does \
         not have — that is the upstream defect docs/design/19-the-cli-surface.md records, and it \
         is pinned so the note cannot outlive it. {} of 6 still do:\n  {}\nIf the generator has \
         been fixed, delete that paragraph from the design document and this case with it.",
        wrong.len(),
        wrong.join("\n  ")
    );
    assert!(
        !script.contains("cargo xtask synth")
            && !document.contains("regenerate with `ess synthesize`"),
        "a file this repository owns now instructs a reader to run a command it does not have"
    );
}

/// **The tree decides the kind of every exception path whose handler speaks the protocol, and it
/// decides the one the list carries.**
///
/// `kinds_the_tree_derives` is the half of the kind column that rests on nothing written down.
/// This is both directions of it: every path it derives is a path the list carries with that
/// kind, and the count it derives is the count `docs/design/19-the-cli-surface.md` states — so the
/// page cannot say "twelve" while the derivation covers three.
#[test]
fn the_kinds_the_tree_derives_are_the_kinds_the_list_carries() {
    let derived = kinds_the_tree_derives();
    let listed: BTreeMap<&str, Unspecified> = UNSPECIFIED_PATHS
        .iter()
        .map(|(path, kind, _)| (*path, *kind))
        .collect();

    let mut wrong = Vec::new();
    for (path, kind) in &derived {
        match listed.get(path.as_str()) {
            None => wrong.push(format!(
                "the tree derives `{path}` as `{kind:?}` and UNSPECIFIED_PATHS does not carry it"
            )),
            Some(carried) if carried != kind => wrong.push(format!(
                "the tree derives `{path}` as `{kind:?}` and the list carries `{carried:?}`"
            )),
            Some(_) => {}
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n  "));

    // Every number the page's accounting states, and the residual paths it names, computed here.
    // Only the first of the three used to be held, and by `contains` — so a page that stated two
    // of them, or named a residual path the tree does derive, passed.
    let parser = connectors_cli::command();
    let grouping: BTreeSet<&str> = UNSPECIFIED_PATHS
        .iter()
        .filter(|(path, _, _)| {
            path.split(' ')
                .try_fold(&parser, |command, word| command.find_subcommand(word))
                .is_some_and(|command| command.get_subcommands().next().is_some())
        })
        .map(|(path, _, _)| *path)
        .collect();
    let residual: Vec<&str> = UNSPECIFIED_PATHS
        .iter()
        .map(|(path, _, _)| *path)
        .filter(|path| !derived.contains_key(*path) && !grouping.contains(path))
        .collect();
    assert_eq!(
        derived.len() + grouping.len() + residual.len(),
        UNSPECIFIED_PATHS.len(),
        "the three parts of the accounting have to cover the list exactly"
    );

    let document = read(&repository_root().join("docs/design/19-the-cli-surface.md"));
    let one_line = document.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut unstated = Vec::new();
    for claim in [
        format!(
            "**{} of the {}** kinds are derived from the tree",
            derived.len(),
            UNSPECIFIED_PATHS.len()
        ),
        format!("**{} more** are `Grouping`", grouping.len()),
        format!("**The remaining {}**", residual.len()),
    ] {
        if !one_line.contains(&claim) {
            unstated.push(claim);
        }
    }
    // The residual sentence itself, both directions. It has to name every residual path it does
    // not cover by the phrase "the seven `Lifecycle` steps", and it may not name one the tree
    // derives — which is exactly how `connect` came to be described as measured by nothing.
    let sentence = one_line
        .split("**The remaining ")
        .nth(1)
        .map(|rest| {
            rest.split(" reach no protocol request")
                .next()
                .unwrap_or(rest)
                .to_owned()
        })
        .unwrap_or_default();
    let lifecycle_steps = [
        "setup init",
        "session login",
        "session logout",
        "serve local",
        "serve hosted",
        "serve mcp",
    ];
    for path in &residual {
        let covered_by_the_phrase =
            lifecycle_steps.contains(path) || **path == *"admin credentials set";
        if !covered_by_the_phrase && !sentence.contains(&format!("`{path}`")) {
            unstated.push(format!(
                "`{path}`, which the residual sentence does not name"
            ));
        }
    }
    for path in derived.keys() {
        if sentence.contains(&format!("`{path}`")) {
            unstated.push(format!(
                "`{path}` is named among the paths the tree says nothing about, and the tree \
                 derives it as `{:?}`",
                derived[path]
            ));
        }
    }
    assert!(
        unstated.is_empty(),
        "docs/design/19-the-cli-surface.md has to state the whole accounting, and the tree makes \
         it {} derived, {} grouping and {} residual — {residual:?}. These are not stated:\n  {}",
        derived.len(),
        grouping.len(),
        residual.len(),
        unstated.join("\n  ")
    );
}

/// **Every declaration the adversary probe copies is still a copy, and the list of them is not
/// hand-kept.**
///
/// `adversary_fence_probe.rs` calls this file's own `exception_list_refusals` over this file's own
/// `UNSPECIFIED_PATHS`, so a copy that had drifted would make its cases measure that file instead.
/// It carries a control of its own that compares eleven declarations by name — and the eleven are
/// a list, which is the defect one level up: a helper added here and copied there is outside it,
/// and the two would diverge silently.
///
/// This derives the set instead. Every top-level declaration the probe carries that this file also
/// carries has to be identical, so the next copied helper is covered the moment it exists.
///
/// **An absent probe is a skip that says so, not a panic.** This file is tracked and the probe is
/// an adversary artifact; a `read` that panics on a missing file would make a clone of this branch
/// red inside a copy comparison, naming nothing about why. It is reported instead, because a
/// comparison with nothing to compare has proved nothing and should not read as a pass either.
#[test]
fn every_declaration_the_adversary_probe_copies_is_still_a_copy() {
    /// Every top-level `fn`, `enum`, `struct`, `const` or `type`, public or not, with its text.
    ///
    /// The visibility and item keywords are enumerated rather than guessed: an extractor blind to
    /// `pub fn` or to `struct` would silently stop comparing a copied declaration the day one is
    /// added, which is the failure this whole case is about.
    fn declarations(source: &str) -> BTreeMap<String, String> {
        const ITEMS: &[(&str, &str)] = &[
            ("fn ", "}"),
            ("enum ", "}"),
            ("struct ", "}"),
            ("impl ", "}"),
            ("const ", "];"),
            ("static ", "];"),
            ("type ", ";"),
        ];
        let mut found = BTreeMap::new();
        let lines: Vec<&str> = source.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let bare = line.strip_prefix("pub ").unwrap_or(line);
            let bare = bare.strip_prefix("async ").unwrap_or(bare);
            if bare.len() == line.len() && line.starts_with(char::is_whitespace) {
                continue;
            }
            let Some((keyword, closer)) = ITEMS
                .iter()
                .find(|(keyword, _)| bare.starts_with(keyword))
                .copied()
            else {
                continue;
            };
            let name: String = bare[keyword.len()..]
                .chars()
                .take_while(|character| character.is_alphanumeric() || *character == '_')
                .collect();
            if name.is_empty() {
                continue;
            }
            let Some(end) = lines[index..]
                .iter()
                .position(|line| line.trim_end() == closer)
            else {
                continue;
            };
            found.insert(name, lines[index..=index + end].join("\n"));
        }
        found
    }

    let root = repository_root();
    let probe_path = root.join("crates/connectors-cli/tests/adversary_fence_probe.rs");
    let Ok(probe_source) = std::fs::read_to_string(&probe_path) else {
        panic!(
            "{} is not present, and this case is what keeps its copies of this file's \
             `UNSPECIFIED_PATHS` and `exception_list_refusals` honest. Without the file there is \
             nothing to compare and the adversary cases that call those copies do not run at all, \
             so this is a gap rather than a pass. Restore the probe, or delete this case together \
             with the reason it was added.",
            probe_path.display()
        )
    };

    let here = declarations(&read(
        &root.join("crates/connectors-cli/tests/cli_surface.rs"),
    ));
    let probe = declarations(&probe_source);

    let shared: Vec<&String> = probe
        .keys()
        .filter(|name| here.contains_key(*name))
        .collect();
    assert!(
        shared.len() >= 14,
        "only {} declarations are shared between this file and the adversary probe, and it copies \
         fourteen. A comparison that finds fewer than it should is one that has stopped comparing \
         some of them: {shared:?}",
        shared.len()
    );

    let diverged: Vec<&&String> = shared
        .iter()
        .filter(|name| here[**name] != probe[**name])
        .collect();
    assert!(
        diverged.is_empty(),
        "crates/connectors-cli/tests/adversary_fence_probe.rs carries copies of these that are no \
         longer copies, so its cases are measuring that file rather than this one: {diverged:?}"
    );
}

/// **The read-verb enumeration is a partition of the protocols it cites, not a sentence.**
///
/// Eight of the kinds `kinds_the_tree_derives` settles are `Read`, and `Read` is decided by a
/// prose enumeration inside a YAML comment — a comment `ess specify validate` never opens. An
/// adversary pass ran this unit's own edit to that sentence backwards, removing the one word it
/// had added, and `connection candidates` moved from `Read` to `Unmodelled` with nothing in the
/// repository objecting.
///
/// So the sentence is held to the tree, the way
/// `crates/catalog-build/tests/main/ess_claim_fence.rs::every_declared_wire_name_is_a_method_the_protocol_accepts`
/// holds a `naming.wire` to the enum that would decode it. `ess/system/components.yaml` divides
/// the request variants of the three protocols it cites into three parts — the commands some
/// component accepts, the verbs that change no entity, and what is neither — and this requires
/// those three to be **disjoint and exhaustive** over the enums themselves. A verb dropped from
/// the read half is then not silently `Unmodelled`; it is a variant in no part, and refused here.
///
/// What it does not do is decide which part a variant belongs in. That is a judgement about
/// whether a call changes an entity, `crates/protocol` does not state it, and
/// `docs/design/19-the-cli-surface.md` says so rather than implying the split is measured.
#[test]
fn the_read_verb_enumeration_partitions_the_protocols_it_names() {
    let enums = protocol_request_variants();
    let cited = protocols_the_read_verb_marker_cites();
    let mut all: BTreeSet<String> = BTreeSet::new();
    for module in &cited {
        let variants = enums.get(module).unwrap_or_else(|| {
            panic!(
                "ess/system/components.yaml cites crates/protocol/src/{module}.rs and it \
                    declares no request enum"
            )
        });
        all.extend(variants.iter().cloned());
    }
    assert!(
        all.len() > 10,
        "the three protocols the marker cites declare {all:?}; the enums moved, so read them again"
    );

    let reads = read_verbs_the_specification_names();
    let neither = neither_command_nor_read();
    let commands: BTreeSet<String> = {
        let wires = wires_of_accepted_commands();
        all.iter()
            .filter(|variant| {
                let mut snake = String::new();
                for (index, character) in variant.chars().enumerate() {
                    if character.is_ascii_uppercase() {
                        if index > 0 {
                            snake.push('_');
                        }
                        snake.push(character.to_ascii_lowercase());
                    } else {
                        snake.push(character);
                    }
                }
                wires.contains(&snake)
            })
            .cloned()
            .collect()
    };

    let mut wrong = Vec::new();
    for variant in &all {
        let parts: Vec<&str> = [
            (commands.contains(variant), "an accepted command"),
            (reads.contains(variant), "a read verb"),
            (neither.contains(variant), "neither"),
        ]
        .iter()
        .filter(|(carried, _)| *carried)
        .map(|(_, name)| *name)
        .collect();
        match parts.len() {
            1 => {}
            0 => wrong.push(format!(
                "`{variant}` is in no part: ess/system/components.yaml does not name it as a read \
                 verb, no accepted command carries it as a `naming.wire`, and it is not named as \
                 neither"
            )),
            _ => wrong.push(format!(
                "`{variant}` is in {parts:?}, and the parts are disjoint"
            )),
        }
    }
    for named in reads.iter().chain(neither.iter()) {
        if !all.contains(named) {
            wrong.push(format!(
                "ess/system/components.yaml names `{named}`, and no request enum of the three \
                 protocols it cites declares that variant"
            ));
        }
    }

    assert!(
        wrong.is_empty(),
        "the three parts ess/system/components.yaml divides these protocols into have to be \
         disjoint and to cover every variant, or a request variant's kind is decided by which \
         sentence somebody remembered to edit:\n  {}",
        wrong.join("\n  ")
    );
}

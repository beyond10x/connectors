//! **Adversary pass: two claims of `story:cli-surface-fences`, driven through the code that is
//! supposed to hold them.**
//!
//! Added by an adversary pass. It changes no implementation file and weakens no existing case.
//!
//! Two things are measured here.
//!
//! 1. A `path:line` citation this unit wrote is opened, not merely resolved.
//!    `crates/connectors-cli/tests/cli_surface.rs`'s
//!    `every_citation_this_unit_wrote_resolves` checks that a cited line exists, and says in its
//!    own comment that it "does not check that the line *means* what the sentence says". The two
//!    documents this unit owns both cite one line as the enforcement of the `naming.wire` rule,
//!    and the same commit moved that rule down the file it cites.
//!
//! 2. The kinds. `docs/design/19-the-cli-surface.md` says the kind of an exception entry is "a
//!    claim the tree can contradict". For the two `Grouping` entries and the three `Forwarded`
//!    ones it is. For the rest the only thing that can contradict a kind is a sentence the author
//!    of the entry wrote, so the refusal fires exactly when the author volunteered the string that
//!    incriminates them.
//!
//! The declarations below are copies of `crates/connectors-cli/tests/cli_surface.rs`'s own, taken
//! character for character so that the two cases about the kinds run the shipped contract rather
//! than a paraphrase of it. `the_copies_this_probe_carries_are_still_copies` compares the two
//! files' text, on the same argument the drift suite gives for doing it that way.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

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

// Copied from `crates/connectors-cli/tests/cli_surface.rs`, character for character, because
// `exception_list_refusals` above calls it.
// Copied from `crates/connectors-cli/tests/cli_surface.rs`, character for character, because
// `kinds_the_tree_derives` above calls it.
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

// Copied from `crates/connectors-cli/tests/cli_surface.rs`, character for character, because
// `kinds_the_tree_derives` above calls it.
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

// Copied from `crates/connectors-cli/tests/cli_surface.rs`, character for character, because
// `kinds_the_tree_derives` above calls it.
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

// Copied from `crates/connectors-cli/tests/cli_surface.rs`, character for character, because
// `kinds_the_tree_derives` above calls it.
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

// Copied from `crates/connectors-cli/tests/cli_surface.rs`, character for character, because
// `exception_list_refusals` above calls it.
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

// Copied from `crates/connectors-cli/tests/cli_surface.rs`, character for character, because
// `exception_list_refusals` above calls it.
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

/// The line `document` cites as the enforcement of the `naming.wire` rule, and what that line is.
fn cited_claim_fence_line(document: &str) -> (usize, String) {
    const MARKER: &str = "crates/catalog-build/tests/main/ess_claim_fence.rs:";
    let text = read(&repository_root().join(document));
    let index = text
        .find(MARKER)
        .unwrap_or_else(|| panic!("{document} cites `{MARKER}<line>`"));
    let cited: usize = text[index + MARKER.len()..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or_else(|error| panic!("{document}: the citation names a line: {error}"));
    let fence = read(&repository_root().join("crates/catalog-build/tests/main/ess_claim_fence.rs"));
    let lines: Vec<&str> = fence.lines().collect();
    assert!(
        cited >= 1 && cited <= lines.len(),
        "{document} cites ess_claim_fence.rs:{cited} and that file has {} lines",
        lines.len()
    );
    (cited, lines[cited - 1].trim().to_owned())
}

/// The first and last line of `every_declared_wire_name_is_a_method_the_protocol_accepts`,
/// doc comment and `#[test]` attribute included, as 1-based lines.
fn the_wire_name_rule() -> (usize, usize) {
    let fence = read(&repository_root().join("crates/catalog-build/tests/main/ess_claim_fence.rs"));
    let lines: Vec<&str> = fence.lines().collect();
    let signature = lines
        .iter()
        .position(|line| {
            line.starts_with("fn every_declared_wire_name_is_a_method_the_protocol_accepts()")
        })
        .expect(
            "ess_claim_fence.rs declares `every_declared_wire_name_is_a_method_the_protocol_accepts`",
        );
    let mut first = signature;
    while first > 0 && (lines[first - 1].starts_with("///") || lines[first - 1].starts_with("#[")) {
        first -= 1;
    }
    let last = signature
        + 1
        + lines[signature + 1..]
            .iter()
            .position(|line| *line == "}")
            .expect("the function closes");
    (first + 1, last + 1)
}

/// **The design document's citation for the `naming.wire` rule points at the rule.**
///
/// `docs/design/19-the-cli-surface.md` says a `naming.wire` is the protocol `method` tag and
/// nothing else, "enforced by `crates/catalog-build/tests/main/ess_claim_fence.rs:156`". That
/// enforcement is `every_declared_wire_name_is_a_method_the_protocol_accepts`, and this unit's own
/// diff inserted lines above it in the file the sentence cites.
///
/// This is the shape of
/// `cli_surface_drift.rs::the_thin_frontend_citation_points_at_the_thin_frontend_test`, which the
/// same unit kept for the one other citation whose meaning is load-bearing.
#[test]
fn the_wire_name_rule_citation_in_the_design_document_points_at_the_rule() {
    let document = "docs/design/19-the-cli-surface.md";
    let (cited, text) = cited_claim_fence_line(document);
    let (first, last) = the_wire_name_rule();
    assert!(
        (first..=last).contains(&cited),
        "{document} cites ess_claim_fence.rs:{cited} as what enforces the `naming.wire` rule, and \
         that line is `{text}`. The rule is enforced by \
         `every_declared_wire_name_is_a_method_the_protocol_accepts`, lines {first}-{last}. \
         `every_citation_this_unit_wrote_resolves` passes this because {cited} is inside the file; \
         nothing opens it."
    );
}

/// **The specification's citation for the `naming.wire` rule points at the rule.**
///
/// The same sentence, in `ess/system/components.yaml`. It is the document
/// `crates/catalog-build/tests/main/ess_citation_fence.rs` exists for, and
/// `every_citation_of_the_specification_resolves` passes it for the same reason.
#[test]
fn the_wire_name_rule_citation_in_the_specification_points_at_the_rule() {
    let document = "ess/system/components.yaml";
    let (cited, text) = cited_claim_fence_line(document);
    let (first, last) = the_wire_name_rule();
    assert!(
        (first..=last).contains(&cited),
        "{document} cites ess_claim_fence.rs:{cited} as what enforces the `naming.wire` rule, and \
         that line is `{text}`. The rule is enforced by \
         `every_declared_wire_name_is_a_method_the_protocol_accepts`, lines {first}-{last}."
    );
}

/// The five arguments `exception_list_refusals` reads out of the tree, as the repository ships it.
fn tree() -> (
    clap::Command,
    BTreeSet<String>,
    BTreeSet<String>,
    BTreeSet<String>,
) {
    (
        connectors_cli::command(),
        paths_the_specification_names(),
        accepted_commands(),
        words_the_specification_can_type(),
    )
}

/// A sentence that reads perfectly well, describes a lifecycle step, and names no command.
const A_LIFECYCLE_SENTENCE: &str =
    "a process or credential lifecycle step; no entity of this specification moves";

/// **Every entry that is not a lifecycle step is refused when it claims to be one.**
///
/// `docs/design/19-the-cli-surface.md` says the kinds are "held to the tree" and that the change
/// "makes the *kind* a claim the tree can contradict". A claim the tree can contradict is one that
/// fails when it is false, so every entry whose shipped kind is not `Lifecycle` is relabelled
/// `Lifecycle` here, one at a time, with a reason that names no command — and each has to be
/// refused. There are twenty of them; the number is not asserted, because the loop is over the list
/// rather than over a count of it.
///
/// Nothing about the parser, the specification or the accepted-command set is changed; only the
/// kind column and the sentence beside it, which are the two things the claim is about.
#[test]
fn every_entry_that_is_not_a_lifecycle_step_is_refused_when_it_claims_to_be_one() {
    let (parser, named, accepted, typeable) = tree();
    let mut unchecked = Vec::new();
    let mut refused = 0usize;
    let mut considered = 0usize;

    for (path, kind, _) in UNSPECIFIED_PATHS {
        if matches!(kind, Unspecified::Lifecycle) {
            continue;
        }
        considered += 1;
        let relabelled: Vec<(&str, Unspecified, &str)> = UNSPECIFIED_PATHS
            .iter()
            .map(|(entry, entry_kind, reason)| {
                if entry == path {
                    (*entry, Unspecified::Lifecycle, A_LIFECYCLE_SENTENCE)
                } else {
                    (*entry, *entry_kind, *reason)
                }
            })
            .collect();
        let refusals = exception_list_refusals(
            &relabelled,
            &parser,
            &named,
            &accepted,
            &typeable,
            the_specification_declares_a_view(),
        );
        if refusals.iter().any(|refusal| refusal.contains(path)) {
            refused += 1;
        } else {
            unchecked.push(format!("`{path}`, shipped as `{kind:?}`"));
        }
    }

    assert!(
        considered > 10,
        "only {considered} entries were read out of `UNSPECIFIED_PATHS`; the copy is wrong"
    );
    assert!(
        unchecked.is_empty(),
        "{refused} of {considered} entries are refused when their kind is replaced with \
         `Lifecycle` and their reason with a sentence naming no command. These {} are not, so \
         their kind is not a claim the tree can contradict — it is a claim only the entry's own \
         sentence can contradict, and the entry's own sentence is written by whoever wants the \
         entry:\n  {}",
        unchecked.len(),
        unchecked.join("\n  ")
    );
}

/// **The reason the contract refuses under `Forwarded` is still refused one word later.**
///
/// `cli_surface.rs::an_exception_whose_kind_the_tree_contradicts_is_refused` builds exactly this
/// entry — `connection materialize` excused with "the service handles it and this frontend passes
/// it along" — and asserts the contract refuses it for naming no command of the specification.
/// That case passes. This one changes nothing but the kind column: the same path, the same false
/// sentence, relabelled from `Forwarded` to `Lifecycle`.
///
/// `connection materialize` forwards `connectors.connection.MaterializeObservation`, which
/// `ess/system/components.yaml` lists under `accepts.commands`, so the entry is false about the
/// tree either way.
#[test]
fn a_forwarding_reason_that_names_no_command_is_refused_whatever_kind_it_carries() {
    let (parser, named, accepted, typeable) = tree();
    let relabelled: Vec<(&str, Unspecified, &str)> = UNSPECIFIED_PATHS
        .iter()
        .map(|(path, kind, reason)| {
            if *path == "connection materialize" {
                (
                    *path,
                    Unspecified::Lifecycle,
                    "the service handles it and this frontend passes it along",
                )
            } else {
                (*path, *kind, *reason)
            }
        })
        .collect();

    let refusals = exception_list_refusals(
        &relabelled,
        &parser,
        &named,
        &accepted,
        &typeable,
        the_specification_declares_a_view(),
    );
    assert!(
        refusals
            .iter()
            .any(|refusal| refusal.contains("connection materialize")),
        "`connection materialize` forwards `connectors.connection.MaterializeObservation`, and \
         the contract refuses this exact sentence under `Forwarded`. Under `Lifecycle` it raises: \
         {refusals:?}"
    );
}

/// **The copies above are still copies of the contract, character for character.**
///
/// The control for the two cases about the kinds: they call the contract's own
/// `exception_list_refusals` over the contract's own list, so a copy that had drifted would make
/// them measure this file instead. Same argument, and same method, as
/// `cli_surface_drift.rs::the_copied_declarations_are_still_copies`.
#[test]
fn the_copies_this_probe_carries_are_still_copies() {
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
    let here =
        read(&repository_root().join("crates/connectors-cli/tests/adversary_fence_probe.rs"));

    let mut diverged = Vec::new();
    for (opener, closer) in [
        ("fn repository_root(", "\n}"),
        ("fn read(", "\n}"),
        ("fn parser_paths(", "\n}"),
        ("enum Unspecified {", "\n}"),
        ("const UNSPECIFIED_PATHS", "\n];"),
        ("fn paths_the_specification_names(", "\n}"),
        ("fn accepted_commands(", "\n}"),
        ("fn the_specification_declares_a_view(", "\n}"),
        ("fn words_the_specification_can_type(", "\n}"),
        ("fn commands_named_by(", "\n}"),
        ("fn exception_list_refusals(", "\n}"),
    ] {
        if declaration(&contract, opener, closer) != declaration(&here, opener, closer) {
            diverged.push(opener.to_owned());
        }
    }

    assert!(
        diverged.is_empty(),
        "this probe's copies of the contract's declarations are no longer copies, so the cases \
         above are measuring this file rather than the contract: {diverged:?}"
    );
}

/// **The words the specification can type are the words the two documents say it types.**
///
/// `docs/design/19-the-cli-surface.md` states the rule twice and measures it once: "a command's
/// typed word is its `naming.wire`, or the qualified name's last segment, verbatim and un-cased",
/// and a `cli:` block over the whole accepted surface "produced `SuperviseChannel`,
/// `ReconnectChannel`, `ConnectChannel`, `StopChannel`, `AuthorizeConnection`, ... as words at a
/// shell". `ess/system/components.yaml` names three of the same words — "Placing them would emit
/// `SuperviseChannel`, `FinishConnectSession` and `SettleSession` as words at a shell — measured,
/// not supposed".
///
/// `words_the_specification_can_type` is the set that decides whether an `Unmodelled` exception is
/// contradicted, and it lowercases the last segment. So the set the contract works from holds
/// `supervisechannel`, a word neither document says anything about, and holds none of the eleven
/// words both documents name.
///
/// This case reads the words out of `docs/design/19-the-cli-surface.md` rather than restating
/// them, so it cannot be right today and quietly wrong after the next command lands.
#[test]
fn the_typeable_words_are_the_words_the_design_document_names() {
    let document = read(&repository_root().join("docs/design/19-the-cli-surface.md"));
    let sentence = document
        .split("and it produced ")
        .nth(1)
        .expect("the design document records what a whole-surface `cli:` block produced")
        .split(" as words at a shell")
        .next()
        .expect("the sentence closes with `as words at a shell`");
    let named: Vec<String> = sentence
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect();
    assert!(
        named.len() >= 5,
        "only {named:?} was read out of the design document's own measurement; the sentence \
         moved, so read it again before believing any result from this case"
    );

    let typeable = words_the_specification_can_type();
    let missing: Vec<&String> = named
        .iter()
        .filter(|word| !typeable.contains(*word))
        .collect();
    assert!(
        missing.is_empty(),
        "docs/design/19-the-cli-surface.md and ess/system/components.yaml both say the word a \
         person would type for a command carrying no `naming.wire` is the qualified name's last \
         segment `verbatim and un-cased`, and name these as the words a whole-surface `cli:` block \
         produced. `words_the_specification_can_type` in cli_surface.rs lowercases the segment, so \
         the set it hands `an_exception_whose_kind_the_tree_contradicts_is_refused` carries none \
         of them:\n  {missing:?}\nthe set it does carry is:\n  {typeable:?}"
    );
}

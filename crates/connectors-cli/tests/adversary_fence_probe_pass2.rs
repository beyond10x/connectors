//! **Adversary pass two: the documents this unit wrote, driven against the tree it wrote.**
//!
//! Added by an adversary pass. It changes no implementation file, deletes nothing, and weakens no
//! existing case.
//!
//! The unit's second pass answered "how much of the kind column rests on the tree" with a
//! derivation, `cli_surface.rs::kinds_the_tree_derives`, and wrote the answer into two documents:
//! `docs/design/19-the-cli-surface.md` states which of the twenty-six each mechanism covers, and
//! `ess/system/components.yaml` carries the enumeration the derivation reads. Both halves were
//! written in one commit by the author of the check that reads them, so a statement in one and a
//! behaviour in the other can agree consistently and be wrong together. Nothing else compares
//! them. These cases do.
//!
//! Every case here reads the tree; none constructs a parser, so none of them can be satisfied by
//! rewording a test.

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

/// `CandidateActivate` -> `candidate_activate`, which is what `rename_all = "snake_case"` does.
///
/// The same transformation `cli_surface.rs::kinds_the_tree_derives` applies, for the same reason:
/// the word a caller puts on the wire is the request variant in snake_case, because all three
/// request enums are `#[serde(tag = "method", rename_all = "snake_case")]`.
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

/// Every command `ess/system/components.yaml` says a component accepts.
///
/// A copy of `cli_surface.rs::accepted_commands`, because that file is a test target and exports
/// nothing. It carries the same refusal so a moved block panics rather than passing on an empty
/// set.
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
        "the `accepts.commands` extraction found {} entries and not the one this file names; the \
         block moved, so read it again: {accepted:?}",
        accepted.len()
    );
    accepted
}

/// The `naming.wire` of every command some component of the specification accepts.
///
/// A copy of `cli_surface.rs::wires_of_accepted_commands`, for the same reason.
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
         blocks moved, so read them again"
    );
    wires
}

/// The read verbs the enumeration in `document` names, read the way
/// `cli_surface.rs::read_verbs_the_specification_names` reads them.
///
/// Taken as an argument rather than read from disk, which is the whole point: it is the only way
/// to measure what the derivation does with a different sentence without editing the reviewed
/// document.
fn read_verbs_named_by(document: &str) -> BTreeSet<String> {
    let sentence = document
        .split("The read verbs of the three protocols —")
        .nth(1)
        .expect("ess/system/components.yaml enumerates the read verbs of the three protocols")
        .split("— change no entity")
        .next()
        .expect("the enumeration closes with `— change no entity`");
    sentence
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|word| !word.contains('/') && !word.contains(':'))
        .map(str::to_owned)
        .collect()
}

/// The request variants a body constructs, by the qualified path the tree writes.
fn requests_built_by(body: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for marker in [
        "ConnectionRequest::",
        "EventRequest::",
        "OperationRequest::",
    ] {
        for (index, _) in body.match_indices(marker) {
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

/// The variants of the `pub enum …Request` of one `crates/protocol/src` module.
fn request_variants_of(module: &str) -> BTreeSet<String> {
    let source = read(&repository_root().join(format!("crates/protocol/src/{module}.rs")));
    let lines: Vec<&str> = source.lines().collect();
    let open = lines
        .iter()
        .position(|line| {
            line.strip_prefix("pub enum ").is_some_and(|rest| {
                rest.split(|character: char| !character.is_alphanumeric())
                    .next()
                    .is_some_and(|name| name.ends_with("Request"))
            })
        })
        .unwrap_or_else(|| panic!("crates/protocol/src/{module}.rs declares a request enum"));
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
    assert!(
        !variants.is_empty(),
        "no variant was read out of crates/protocol/src/{module}.rs"
    );
    variants
}

/// The variants `document` names as neither an accepted command nor a read.
fn neither_named_by(document: &str) -> BTreeSet<String> {
    let sentence = document
        .split("Neither an accepted command nor a read —")
        .nth(1)
        .expect("ess/system/components.yaml names what is neither an accepted command nor a read")
        .split("— sends")
        .next()
        .expect("that sentence closes with `— sends`");
    sentence
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|word| !word.contains('/') && !word.contains(':'))
        .map(str::to_owned)
        .collect()
}

/// Every method of `impl LocalClient` in `crates/connectors-client/src/lib.rs`, by name, with its
/// body.
///
/// A method is a line at four-space indent opening `fn`, `async fn`, `pub fn` or `pub async fn`;
/// its body runs to the next such line or to the end of the `impl`. The extraction refuses an
/// empty result rather than reporting that nothing was found.
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
        if name.is_empty() {
            continue;
        }
        starts.push((open + 1 + offset, name));
    }
    assert!(
        starts.len() >= 8,
        "only {} methods were read out of `impl LocalClient`; the block moved, so read it again \
         before believing any result from a check that uses this",
        starts.len()
    );

    let mut bodies = BTreeMap::new();
    for (position, (index, name)) in starts.iter().enumerate() {
        let stop = starts.get(position + 1).map_or(close, |(next, _)| *next);
        bodies.insert(name.clone(), lines[*index..stop].join("\n"));
    }
    bodies
}

/// **Every path the design document says reaches no protocol request reaches none — and `connect`,
/// which reaches seven, is not one of them.**
///
/// *Kept as an invariant, not as the snapshot it was written as.* It opened by asserting that this
/// sentence was still in the page:
///
/// > **The remaining 12** — `connect`, `doctor`, `providers`, `auth status`,
/// > `admin integrations status` and the seven `Lifecycle` steps — send no protocol request, and
/// > the tree says nothing about them.
///
/// That sentence was false. `Command::Connect` dispatches to `connect::dispatch` in
/// `crates/connectors-console`, which drives a `LocalClient`, and those client methods build seven
/// `ConnectionRequest` variants — three of them carrying a `method` that is the `naming.wire` of a
/// command `ess/system/components.yaml` says `connectors-service` accepts, which is the
/// derivation's own definition of `Forwarded`. `kinds_the_tree_derives` could not see it because
/// it read one file and stopped at the first call boundary.
///
/// A case that only asserts the wrong sentence is still there can be answered by rewriting the
/// case, which is the failure this whole pass is about, one level up —
/// `crates/connectors-cli/tests/cli_surface_pass_two.rs` says exactly that in its own header and
/// took the same decision for the same reason. So the predicate is now the invariant the finding
/// was about, in a form no rewording satisfies: the residual list is **read out of the page** and
/// every path on it has to reach nothing, and `connect` has to reach the seven and be absent from
/// it. The derivation follows the callee now, so the page names twelve and `setup connect` is
/// derived.
#[test]
fn the_paths_the_design_document_says_send_no_protocol_request_send_none() {
    let root = repository_root();
    let document = read(&root.join("docs/design/19-the-cli-surface.md"));
    let one_line = document.split_whitespace().collect::<Vec<_>>().join(" ");

    // The residual sentence, and the paths it names.
    let sentence = one_line
        .split("**The remaining ")
        .nth(1)
        .expect("docs/design/19-the-cli-surface.md accounts for the paths the tree cannot decide")
        .split(" reach no protocol request")
        .next()
        .expect("that sentence says the paths reach no protocol request")
        .to_owned();
    let named: Vec<String> = sentence
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|word| !word.contains("Lifecycle"))
        .map(str::to_owned)
        .collect();
    assert!(
        named.len() >= 4,
        "only {named:?} was read out of the residual sentence `{sentence}`; it moved, so read it \
         again before believing any result from this case"
    );

    // What each of them reaches: its own dispatch arm, and one hop into the `connectors-console`
    // module that arm hands off to, whose `LocalClient` calls are read too.
    let cli = read(&root.join("crates/connectors-cli/src/lib.rs"));
    let methods = local_client_methods();
    let module_reaches = |module: &str| -> BTreeSet<String> {
        let path = root.join(format!("crates/connectors-console/src/{module}.rs"));
        let Ok(text) = std::fs::read_to_string(&path) else {
            return BTreeSet::new();
        };
        let mut reached = requests_built_by(&text);
        for (name, body) in &methods {
            if text.contains(&format!(".{name}(")) {
                reached.extend(requests_built_by(body));
            }
        }
        reached
    };
    // `<Group>Command::<Leaf>` for a path under one of the first level's groups, `Command::<Leaf>`
    // for a first-level word. Both forms are here because `story:cli-first-level-groups` moved
    // every leaf of the residual sentence one level down: a marker built from the whole path,
    // `Command::InspectDoctor`, matches nothing, and a loop over paths that resolve to nothing is a
    // loop that passes without reading the tree at all.
    let marker = |path: &str| -> String {
        // Every hyphen segment capitalised, as the derive names a variant: `serve-hosted` is
        // `ServeHosted`. A first draft of this rewrite capitalised the first character alone,
        // yielding `Servehosted`, which matches no arm — and a path that matches no arm reads as
        // reaching nothing, which is a pass. The control for that is asserted right below.
        let camel = |word: &str| -> String {
            word.split('-')
                .map(|segment| {
                    let mut characters = segment.chars();
                    let mut out = String::new();
                    if let Some(first) = characters.next() {
                        out.extend(first.to_uppercase());
                    }
                    out.push_str(characters.as_str());
                    out
                })
                .collect()
        };
        match path.split_once(' ') {
            Some((group, leaf)) => format!("{}Command::{}", camel(group), camel(leaf)),
            None => format!("Command::{}", camel(path)),
        }
    };
    assert_eq!(marker("serve-hosted"), "Command::ServeHosted");
    assert_eq!(marker("serve hosted"), "ServeCommand::Hosted");
    let reaches = |path: &str| -> BTreeSet<String> {
        let marker = marker(path);
        let Some(open) = cli.find(&format!("{marker} ")).or_else(|| {
            cli.find(&format!("{marker} {{"))
                .or_else(|| cli.find(&format!("{marker},")))
        }) else {
            return BTreeSet::new();
        };
        // To the next dispatch arm, at whatever depth: a line whose first word is a `…Command::…`
        // pattern. Terminating on one indentation would run an inner arm into its siblings.
        let arm: String = cli[open..]
            .split_inclusive('\n')
            .enumerate()
            .take_while(|(index, line)| {
                *index == 0 || {
                    let head = line.trim_start();
                    !(head.starts_with(char::is_uppercase) && head.contains("Command::"))
                }
            })
            .map(|(_, line)| line)
            .collect();
        let arm = arm.as_str();
        let mut reached = requests_built_by(arm);
        for module in [
            "connect",
            "doctor",
            "providers",
            "auth",
            "admin",
            "init",
            "enrol",
        ] {
            if arm.contains(&format!("{module}::")) {
                reached.extend(module_reaches(module));
            }
        }
        reached
    };

    let connect = reaches("setup connect");
    assert!(
        connect.len() >= 7,
        "`connectors connect` reaches {connect:?} through `connect::dispatch` and `LocalClient`; \
         this case was written for the seven it reached, so the chain has to be read again"
    );
    let wires = wires_of_accepted_commands();
    let forwarded: BTreeSet<&String> = connect
        .iter()
        .filter(|variant| wires.contains(&snake_case(variant)))
        .collect();
    assert!(
        forwarded.len() >= 3,
        "`connectors connect` reaches {forwarded:?} accepted commands; it reached three when this \
         case was written"
    );

    let mut wrong = Vec::new();
    if named.iter().any(|path| path == "setup connect") {
        wrong.push(format!(
            "`setup connect` is named as reaching no protocol request, and it reaches {connect:?}, \
             of which {forwarded:?} carry the `naming.wire` of a command `connectors-service` \
             accepts"
        ));
    }
    // A path the residual sentence names and this file cannot find an arm for is read as reaching
    // nothing, which is how a renamed path would make this loop silently stop measuring. `admin`
    // runs its own subtree in `crates/connectors-console`, so a path under it legitimately has no
    // arm here; every other path has to be found. Named exactly, rather than as an allowance of
    // one: an allowance is consumed by the first path that stops resolving, unmeasured.
    let unresolved: Vec<&String> = named
        .iter()
        .filter(|path| !cli.contains(&format!("{} ", marker(path))))
        .collect();
    let outside_admin: Vec<&&String> = unresolved
        .iter()
        .filter(|path| !path.starts_with("admin "))
        .collect();
    assert!(
        outside_admin.is_empty(),
        "these paths the residual sentence names resolve to no dispatch arm of \
         crates/connectors-cli/src/lib.rs: {outside_admin:?} (marker {:?}). They are read as \
         reaching nothing, so this case would pass without measuring them",
        outside_admin
            .iter()
            .map(|path| marker(path))
            .collect::<Vec<_>>()
    );
    for path in &named {
        let reached = reaches(path);
        if !reached.is_empty() {
            wrong.push(format!(
                "`{path}` is named as reaching no protocol request, and it reaches {reached:?}"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "docs/design/19-the-cli-surface.md accounts for the paths whose kind nothing measures by \
         saying they reach no protocol request. These do:\n  {}\nEither the sentence goes, or \
         `kinds_the_tree_derives` follows the call.",
        wrong.join("\n  ")
    );
}

/// **The sentence that decides eight of the derived kinds is held to the protocol it describes.**
///
/// *Kept as an invariant, not as the snapshot it was written as.* It opened by asserting that the
/// page still said the derived kinds "rest on no sentence at all", and then that removing one word
/// from a prose enumeration inside a YAML comment moved `connection candidates` from `Read` to
/// `Unmodelled`. Both halves were true. Answering it by deleting the page's sentence would have
/// left the enumeration exactly as unchecked as it was, so the predicate is now the thing that
/// makes the finding harmless: `ess/system/components.yaml` divides the request variants of the
/// three protocols it cites into the commands some component accepts, the verbs that change no
/// entity, and what is neither — and those three have to be **disjoint and exhaustive** over the
/// enums themselves.
///
/// Removing `` `CandidateSearch` `` from the read half then does not quietly re-derive a kind. It
/// leaves a variant in no part, and this fails. The original construction is kept below as the
/// control: the same edit, run in memory, has to break the partition.
///
/// Which part a variant belongs in is still a judgement `crates/protocol` does not state. That is
/// what `docs/design/19-the-cli-surface.md` now says, instead of saying the derived kinds rest on
/// nothing.
#[test]
fn the_kinds_the_design_document_says_rest_on_no_sentence_rest_on_no_sentence() {
    let root = repository_root();
    let page = read(&root.join("docs/design/19-the-cli-surface.md"));
    let one_line = page.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        !one_line.contains("These rest on no sentence at all."),
        "docs/design/19-the-cli-surface.md says the derived kinds rest on no sentence again. Eight \
         of them are `Read`, and `Read` is decided by the read-verb enumeration in a YAML comment"
    );
    assert!(
        one_line.contains("turns on the read-verb enumeration"),
        "docs/design/19-the-cli-surface.md has to say which half of the derivation reads that \
         enumeration; a page that does not is the page this case was written against"
    );

    // The partition, over the shipped document and over the same document with the one word this
    // unit's diff added taken back out.
    let shipped = read(&root.join("ess/system/components.yaml"));
    let word = "`CandidateSearch`, ";
    assert!(
        shipped.contains(word),
        "the read-verb enumeration no longer names {word}; this case was written for the edit \
         that put it there"
    );
    let partition = |document: &str| -> Vec<String> {
        let reads = read_verbs_named_by(document);
        let neither = neither_named_by(document);
        let wires = wires_of_accepted_commands();
        let mut all: BTreeSet<String> = BTreeSet::new();
        for module in ["connection", "event", "operation"] {
            all.extend(request_variants_of(module));
        }
        let mut wrong = Vec::new();
        for variant in &all {
            let parts = usize::from(wires.contains(&snake_case(variant)))
                + usize::from(reads.contains(variant))
                + usize::from(neither.contains(variant));
            if parts != 1 {
                wrong.push(format!("`{variant}` is in {parts} of the three parts"));
            }
        }
        wrong
    };

    let shipped_gaps = partition(&shipped);
    assert!(
        shipped_gaps.is_empty(),
        "the three parts ess/system/components.yaml divides these protocols into are not a \
         partition:\n  {}",
        shipped_gaps.join("\n  ")
    );

    let without = shipped.replacen(word, "", 1);
    let without_gaps = partition(&without);
    assert!(
        !without_gaps.is_empty(),
        "`CandidateSearch` was removed from the read-verb enumeration — this unit's own edit, run \
         backwards — and the partition still holds, so the sentence that decides eight derived \
         kinds can still be edited with nothing objecting"
    );
}

/// **Every file the committed contract opens at test time is a file a fresh clone has.**
///
/// `crates/connectors-cli/tests/cli_surface.rs` is tracked. This unit added
/// `every_declaration_the_adversary_probe_copies_is_still_a_copy` to it, which opens
/// `crates/connectors-cli/tests/adversary_fence_probe.rs` through the same panicking `read` every
/// other path in that file uses — and that probe is an adversary artifact that is **not tracked**.
///
/// A tracked test that panics on an untracked file is green in the tree it was written in and red
/// in every clone of the branch that merges it. The gate the unit ran cannot see the difference,
/// because the gate runs in the working tree.
///
/// This is not an argument against keeping the probe: `crates/connectors-console/tests/
/// adversary_readability_pass2.rs` is a probe from an earlier pass and is tracked, which is the
/// precedent. It is an argument that the coupling has to be committed with the case that depends
/// on it, and nothing in the suite says so.
#[test]
fn every_file_the_committed_contract_opens_is_a_file_a_clone_has() {
    const MARKER: &str = ".join(\"";
    let root = repository_root();
    let suite = "crates/connectors-cli/tests/cli_surface.rs";
    let source = read(&root.join(suite));

    let mut opened: BTreeSet<String> = BTreeSet::new();
    for (index, _) in source.match_indices(MARKER) {
        let rest = &source[index + MARKER.len()..];
        let Some(end) = rest.find('"') else {
            continue;
        };
        let path = &rest[..end];
        if root.join(path).is_file() {
            opened.insert(path.to_owned());
        }
    }
    assert!(
        opened.len() >= 5,
        "only {} paths were read out of {suite}; the extraction is wrong, and a check that finds \
         nothing to look at passes without looking: {opened:?}",
        opened.len()
    );

    let listed = std::process::Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(&root)
        .output()
        .expect("`git ls-files` runs in the repository");
    assert!(
        listed.status.success(),
        "`git ls-files` exited {}",
        listed.status
    );
    let tracked: BTreeSet<&str> = std::str::from_utf8(&listed.stdout)
        .expect("git writes utf-8 paths here")
        .split('\0')
        .filter(|path| !path.is_empty())
        .collect();
    assert!(
        tracked.len() > 100,
        "`git ls-files` reported {} files; that is not this repository, so nothing below it is \
         evidence",
        tracked.len()
    );

    let missing: Vec<&String> = opened
        .iter()
        .filter(|path| !tracked.contains(path.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "{suite} is tracked and opens these with a `read` that panics when the file is absent, \
         and git does not track them:\n  {missing:#?}\nA clone of the branch that merges this \
         runs `cargo test -p connectors-cli` and panics in \
         `every_declaration_the_adversary_probe_copies_is_still_a_copy` before it compares \
         anything. Commit the probe with the case that reads it, or make the read tolerate its \
         absence and say what it then proves."
    );
}

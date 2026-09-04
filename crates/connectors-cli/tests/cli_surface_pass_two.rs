//! **Adversary pass 2: the documents this unit wrote about its own contract, driven against it.**
//!
//! Added by the second adversary pass of `story:cli-surface-contract`. It changes no
//! implementation file and weakens nothing.
//!
//! `crates/connectors-cli/tests/cli_surface.rs` is the contract. Three documents describe it —
//! `docs/design/19-the-cli-surface.md`, the module header of
//! `crates/connectors-cli/tests/cli_surface_drift.rs`, and the doc comments on that file's cases.
//! `every_citation_this_unit_wrote_resolves` checks that a `path:line` in two of those documents
//! lands inside a file that exists; it says so itself, and it says it checks nothing about what the
//! line means. These cases check the meaning.
//!
//! **Every case was written as a snapshot and is kept as an invariant.** Each opened by asserting
//! that the wrong sentence was still there, which made it a case that could only be answered by
//! rewriting the case — the defect fixed, the assertion left describing a repository that no longer
//! exists. That is the same failure the whole pass is about, one level up. So each now asserts the
//! *corrected* property in a form the next edit has to keep satisfying: a number the page states is
//! read out of the constant it describes, a citation is replaced by the absence of citations, and a
//! derivation is required to still be a derivation. None of them can be answered by editing prose.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate manifest lives two directories below the repository root")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repository_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// The prose of a Rust source, as one line: `///` and `//!` markers removed, wrapped sentences
/// rejoined, runs of whitespace collapsed. A quotation that a doc comment wrapped over two lines
/// is one string here, which is how it has to be compared with the file it is attributed to.
fn prose(source: &str) -> String {
    let unwrapped: Vec<&str> = source
        .lines()
        .map(|line| {
            let line = line.trim_start();
            for marker in ["/// ", "///", "//! ", "//!"] {
                if let Some(rest) = line.strip_prefix(marker) {
                    return rest;
                }
            }
            line
        })
        .collect();
    unwrapped
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every path `UNSPECIFIED_PATHS` carries, read out of the source of `cli_surface.rs` rather than
/// linked to: the constant is private to that test binary, and this file may not change it.
fn unspecified_paths(contract: &str) -> Vec<String> {
    let block = contract
        .split_once("const UNSPECIFIED_PATHS")
        .expect("cli_surface.rs declares UNSPECIFIED_PATHS")
        .1;
    let block = block.split_once("\n];").expect("the list closes").0;
    let lines: Vec<&str> = block.lines().collect();
    let mut paths = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.trim() != "(" {
            continue;
        }
        let entry = lines[index + 1].trim().trim_end_matches(',');
        let path = entry
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or_else(|| panic!("the first field of an entry is a string literal: {entry}"));
        paths.push(path.to_owned());
    }
    assert!(
        !paths.is_empty(),
        "no entry of `UNSPECIFIED_PATHS` was read out of cli_surface.rs; the extraction is wrong \
         and every case below would be measuring itself"
    );
    paths
}

/// **The design document names only constants that exist.**
///
/// `docs/design/19-the-cli-surface.md` is this unit's own account of the contract, and a reader
/// sent to a constant by name has no other way to check it. Every screaming-snake identifier the
/// page writes in backticks has to be a constant some file of this repository declares.
#[test]
fn the_design_document_names_only_constants_that_exist() {
    let document = read("docs/design/19-the-cli-surface.md");
    let sources = [
        read("crates/connectors-cli/tests/cli_surface.rs"),
        read("crates/connectors-cli/tests/cli_surface_drift.rs"),
        read("crates/catalog-build/tests/main/architecture_fence.rs"),
    ];

    let mut named = BTreeSet::new();
    for span in document.split('`').skip(1).step_by(2) {
        let looks_like_a_constant = span.len() >= 4
            && span.contains('_')
            && span
                .chars()
                .all(|character| character.is_ascii_uppercase() || character == '_');
        if looks_like_a_constant {
            named.insert(span.to_owned());
        }
    }
    assert!(
        !named.is_empty(),
        "no backticked constant was read out of the design document; the extraction is wrong"
    );

    let missing: Vec<&String> = named
        .iter()
        .filter(|name| !sources.iter().any(|source| source.contains(name.as_str())))
        .collect();

    assert!(
        missing.is_empty(),
        "docs/design/19-the-cli-surface.md names constants that no file of this repository \
         declares: {missing:?}. The page describes the contract this unit shipped, and a reader \
         following one of these names finds nothing"
    );
}

/// **The design document states the shape of the exception list, and the numbers are the list's.**
///
/// This case was written against a page that called the entries "first-level words" while 17 of
/// the 26 were two- and three-word paths — the correction this unit had already made, described as
/// if it had not been. The fix is not a better adjective: the page now states the three numbers,
/// and they are read out of `UNSPECIFIED_PATHS` here, so the sentence cannot be right today and
/// quietly wrong after the next command lands.
#[test]
fn the_design_document_states_the_shape_of_the_exception_list() {
    let document = read("docs/design/19-the-cli-surface.md");
    let paths = unspecified_paths(&read("crates/connectors-cli/tests/cli_surface.rs"));
    let deeper = paths.iter().filter(|path| path.contains(' ')).count();
    let flat = paths.len() - deeper;

    let claim = format!(
        "**{} entries, {flat} of them one word and {deeper} of them two or three**",
        paths.len()
    );
    assert!(
        document.contains(&claim),
        "docs/design/19-the-cli-surface.md has to state the shape of `UNSPECIFIED_PATHS`, and the \
         list is now `{claim}`. A page that describes the exception list as first-level words is \
         describing the contract this unit replaced, and a reader has no other way to find out"
    );

    assert!(
        !document.contains("names each first-level word"),
        "docs/design/19-the-cli-surface.md still calls the exception list first-level words; it is \
         paths at every depth, which is what makes `connectors connection harvest` and \
         `connectors event invoke` visible"
    );
}

/// **The design document describes the countdown assertion the contract makes.**
///
/// The page said "the test asserts the list has exactly three members". The contract asserts
/// nothing about the list's size — it requires the list to equal a pending set derived from the
/// parser, which is a different and stronger thing. Both halves are checked: the claim is gone, and
/// the thing that would make it true again is gone too.
#[test]
fn the_design_document_describes_the_countdown_assertion_the_contract_makes() {
    let document = read("docs/design/19-the-cli-surface.md");
    let contract = read("crates/connectors-cli/tests/cli_surface.rs");

    assert!(
        !document.contains("asserts the list has exactly three members"),
        "docs/design/19-the-cli-surface.md says the test asserts `TARGET_EXCEPTIONS` has exactly \
         three members; it asserts nothing about its length"
    );
    assert!(
        !contract.contains("TARGET_EXCEPTIONS.len()"),
        "cli_surface.rs asserts on the length of `TARGET_EXCEPTIONS` again. A literal compared with \
         a `const` in the same file measures the file; the countdown is the pending set read off \
         the parser"
    );
    assert!(
        document.contains("the_target_countdown_is_exactly_what_the_parser_still_owes"),
        "docs/design/19-the-cli-surface.md describes the countdown without naming the assertion \
         that makes it, so a reader cannot check the description against anything"
    );
}

/// **The drift suite attributes nothing to the contract that the contract does not carry.**
///
/// It used to explain three of its cases by quoting `cli_surface.rs` and citing a line range. Two
/// of the quotations described a defect this unit then fixed — a count assertion, and a
/// `carries_target` that read only the leaves — so a reader was told the live contract had a hole
/// it does not have, and the line ranges pointed at unrelated code besides.
///
/// A quotation cannot be checked; an identifier can. The drift suite now names the contract's
/// assertions instead of quoting them, and every name it uses has to be declared over there.
#[test]
fn the_drift_suite_attributes_nothing_to_the_contract_that_is_not_there() {
    let drift = read("crates/connectors-cli/tests/cli_surface_drift.rs");
    let contract = read("crates/connectors-cli/tests/cli_surface.rs");

    let stale: Vec<&str> = drift
        .match_indices("cli_surface.rs:")
        .map(|(index, _)| &drift[index..])
        .filter(|rest| {
            rest["cli_surface.rs:".len()..]
                .starts_with(|character: char| character.is_ascii_digit())
        })
        .map(|rest| rest.lines().next().unwrap_or(rest).trim())
        .collect();
    assert!(
        stale.is_empty(),
        "cli_surface_drift.rs cites `cli_surface.rs` by line number again. A line range is a claim \
         that goes stale on its own; name the assertion instead:\n  {}",
        stale.join("\n  ")
    );

    let mut named = BTreeSet::new();
    for span in prose(&drift).split('`').skip(1).step_by(2) {
        let looks_like_a_test = span.len() >= 8
            && span.contains('_')
            && span
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '_');
        if looks_like_a_test {
            named.insert(span.to_owned());
        }
    }
    assert!(
        !named.is_empty(),
        "no backticked identifier was read out of cli_surface_drift.rs; the extraction is wrong"
    );

    let missing: Vec<&String> = named
        .iter()
        .filter(|name| !contract.contains(name.as_str()) && !drift.contains(name.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "cli_surface_drift.rs names identifiers that neither it nor \
         crates/connectors-cli/tests/cli_surface.rs declares: {missing:?}"
    );
}

/// **The drift suite's copies are checked, not cited.**
///
/// It used to say its constants were "copied verbatim from `cli_surface.rs:62-94`" — a range that
/// held `enum Unspecified` and `EVERY_KIND` and neither constant. Whether the copy is still a copy
/// is the one thing that decides whether the drift suite measures the contract or measures itself,
/// and a citation is the weakest possible way to establish it: it needs a reader, and it goes stale
/// when anything above it moves.
///
/// So there is no citation, and there is a test. This checks that both of those are true, and
/// independently re-derives the comparison so that a `the_copied_declarations_are_still_copies`
/// which stopped comparing anything would be caught here.
#[test]
fn the_drift_suites_copies_are_checked_rather_than_cited() {
    let drift = read("crates/connectors-cli/tests/cli_surface_drift.rs");
    let contract = read("crates/connectors-cli/tests/cli_surface.rs");

    assert!(
        !prose(&drift).contains("copied verbatim from `cli_surface.rs:"),
        "cli_surface_drift.rs cites a line range as evidence that its constants are copies. The \
         evidence is `the_copied_declarations_are_still_copies`, which compares the text"
    );
    assert!(
        drift.contains("fn the_copied_declarations_are_still_copies()"),
        "cli_surface_drift.rs declares no test comparing its copied declarations with the \
         contract's, so a kind or a reason that diverged would leave every case in that file green \
         while it described a different list from the one that ships"
    );

    for (opener, closer) in [
        ("enum Unspecified {", "\n}"),
        ("const UNSPECIFIED_PATHS", "\n];"),
        ("const TARGET_EXCEPTIONS", ";"),
    ] {
        let cut = |source: &str| {
            let start = source
                .find(opener)
                .unwrap_or_else(|| panic!("a source declares `{opener}`"));
            let end = source[start..]
                .find(closer)
                .unwrap_or_else(|| panic!("`{opener}` closes with `{closer}`"));
            source[start..start + end + closer.len()].to_owned()
        };
        assert_eq!(
            cut(&contract),
            cut(&drift),
            "`{opener}` differs between cli_surface.rs and cli_surface_drift.rs"
        );
    }
}

/// **The `--target` countdown's candidates are derived, and they are every protocol a deployment
/// answers.**
///
/// `deployment_protocol_modules` was the literal `["connection", "event", "operation"]` filtered by
/// `is_file()`, documented as "read from the tree". A filter over a literal can only shrink it, and
/// `crates/protocol` declares five request enums, not three:
/// `crates/server/src/catalog_projection.rs:38` answers a `CatalogRequest` and
/// `crates/connectors-client/src/lib.rs:594` sends a `DatasourceRequest`. A `catalog` or
/// `datasource` group could never have entered the countdown, so the countdown would have reached
/// zero while that group still inferred its target.
///
/// This reads `crates/protocol/src` the same way and requires the contract to have no literal left
/// to filter.
#[test]
fn the_target_countdown_candidates_are_derived_from_every_protocol_a_deployment_answers() {
    let contract = read("crates/connectors-cli/tests/cli_surface.rs");
    let literal = r#"["connection", "event", "operation"]"#;
    let derivation = contract
        .split_once("fn deployment_protocol_modules()")
        .expect("cli_surface.rs declares `deployment_protocol_modules`")
        .1
        .split_once("\n}")
        .expect("the function closes")
        .0;
    assert!(
        !derivation.contains(literal),
        "`deployment_protocol_modules` names {literal} again. A filter over a literal can only \
         remove a module, so a group named after a protocol outside the literal never enters \
         `TARGET_EXCEPTIONS` and the countdown reaches zero without it"
    );
    assert!(
        derivation.contains("read_dir"),
        "`deployment_protocol_modules` no longer reads crates/protocol/src, so its candidates are \
         named somewhere rather than derived"
    );

    let directory = repository_root().join("crates/protocol/src");
    let mut declared = BTreeSet::new();
    for entry in std::fs::read_dir(&directory).expect("read crates/protocol/src") {
        let path = entry.expect("directory entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read a protocol module");
        let carries_a_request = source.lines().any(|line| {
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

    for beyond_the_old_literal in ["catalog", "datasource"] {
        assert!(
            declared.contains(beyond_the_old_literal),
            "crates/protocol no longer declares a request enum in `{beyond_the_old_literal}`; this \
             case is written against the two modules the old literal could not reach and has to be \
             re-read. Found: {declared:?}"
        );
    }
    assert!(
        declared.len() > 3,
        "the candidate set is derived precisely because it is larger than the three groups that \
         happen to be declared today; crates/protocol declares {declared:?}"
    );
}

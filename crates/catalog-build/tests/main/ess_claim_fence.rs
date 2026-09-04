//! **The prose claims of `ess/system/`, read against the tree they describe.**
//!
//! [`ess_citation_fence`](super::ess_citation_fence) checks that a `path:line` citation points at
//! the thing it names. This file checks the sentence *around* the citation. The specification does
//! not only cite: it asserts. It says a command performs exactly one refusal; it says a code path
//! carries no guard; it says a transition is reachable; it declares the word a caller puts on the
//! wire. `ess validate` reads none of those — it checks the document against its own schema — and
//! a citation fence reads none of them either, because every one of these claims is true or false
//! of a *range* the citation only points inside.
//!
//! Each check below is one such sentence, expressed as an assertion over the source it is a
//! sentence about. Where the fix could be to the document *or* to the tree, the assertion is
//! written as an implication — *the document still says X* implies *the tree still does X* — so
//! that correcting either side turns it green, and neither side can be corrected alone by
//! accident.

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate manifest lives two directories below the workspace root")
        .to_path_buf()
}

fn read(root: &Path, relative: &str) -> String {
    let path = root.join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// Every ESS document of the specification, in a stable order, as `(relative path, source)`.
fn specification(root: &Path) -> Vec<(String, String)> {
    let mut relative = vec![
        "ess/system/system.yaml".to_owned(),
        "ess/system/components.yaml".to_owned(),
    ];
    let mut domains = std::fs::read_dir(root.join("ess/system/domains"))
        .expect("ess/system/domains")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "yaml")
        })
        .map(|path| {
            format!(
                "ess/system/domains/{}",
                path.file_name().expect("a file name").to_string_lossy()
            )
        })
        .collect::<Vec<_>>();
    domains.sort();
    relative.append(&mut domains);
    relative
        .into_iter()
        .map(|path| {
            let source = read(root, &path);
            (path, source)
        })
        .collect()
}

/// The body of the first item whose declaration begins on a line trimming to `opener`, as
/// `(1-based first line, 1-based last line, text)`. Brace-counted from the first line that ends
/// in an opening brace, so a signature broken over several lines is handled.
fn item(source: &str, opener: &str) -> Option<(usize, usize, String)> {
    let lines = source.lines().collect::<Vec<_>>();
    let start = lines.iter().position(|line| line.trim() == opener)?;
    let open = start
        + lines
            .iter()
            .skip(start)
            .position(|line| line.trim_end().ends_with('{'))?;
    let mut depth = 0usize;
    for (offset, line) in lines.iter().enumerate().skip(open) {
        depth += line.matches('{').count();
        depth -= line.matches('}').count();
        if depth == 0 {
            return Some((start + 1, offset + 1, lines[start..=offset].join("\n")));
        }
    }
    None
}

/// `ConnectSessionCreate` -> `connect_session_create`, which is what
/// `#[serde(rename_all = "snake_case")]` does to a variant identifier.
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

/// The variant identifiers of one `pub enum <name>` declaration.
fn variants(source: &str, name: &str) -> Vec<String> {
    let (_, _, body) = item(source, &format!("pub enum {name} {{"))
        .unwrap_or_else(|| panic!("`pub enum {name}` is declared"));
    body.lines()
        .skip(1)
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("//")
                && !line.starts_with('#')
                && !line.starts_with('}')
        })
        .filter_map(|line| {
            let identifier = line
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect::<String>();
            (!identifier.is_empty()).then_some(identifier)
        })
        .collect()
}

/// Every line of one function body that refuses — returns an error instead of a value — as
/// `(1-based line, text)`.
///
/// The three forms this tree uses: an early `return Err(..)`, an `ok_or_else(..)?` on a lookup
/// that may be empty, and a `map_err(..)?` on a fallible effect.
fn refusal_sites(first_line: usize, body: &str) -> Vec<(usize, String)> {
    body.lines()
        .enumerate()
        .filter(|(_, line)| {
            let line = line.trim();
            line.contains("return Err(")
                || line.contains(".ok_or_else(")
                || line.contains(".ok_or(")
                || line.contains(".map_err(")
        })
        .map(|(offset, line)| (first_line + offset, line.trim().to_owned()))
        .collect()
}

/// A YAML comment block or entry as one line: `#` markers removed, wrapped sentences rejoined,
/// runs of whitespace collapsed.
///
/// Every sentence this file asserts the existence of is wrapped over two or three lines in the
/// document, so a `contains` against the raw bytes would be a check on where the author happened
/// to break the line.
fn prose(text: &str) -> String {
    let mut words = Vec::new();
    for line in text.lines() {
        let line = line.trim_start();
        let line = line.strip_prefix('#').unwrap_or(line);
        words.extend(line.split_whitespace());
    }
    words.join(" ")
}

/// One `- name: <id>` entry of a top-level block, with the comment block directly above it and
/// everything under it up to the next entry — as `(1-based first line, text)`.
///
/// The comment run immediately above the *next* entry is given back to that entry, so a marker
/// written for the following command is not read as evidence for this one.
fn declaration(source: &str, id: &str) -> (usize, String) {
    let lines = source.lines().collect::<Vec<_>>();
    let opener = format!("- name: {id}");
    let entry = lines
        .iter()
        .position(|line| line.trim() == opener)
        .unwrap_or_else(|| panic!("`{opener}` is declared"));

    let mut start = entry;
    while start > 0 && lines[start - 1].trim_start().starts_with('#') {
        start -= 1;
    }

    let mut end = lines.len();
    for (offset, line) in lines.iter().enumerate().skip(entry + 1) {
        let top_level_key = !line.is_empty() && !line.starts_with(' ') && !line.starts_with('#');
        if top_level_key || line.starts_with("  - name: ") {
            end = offset;
            break;
        }
    }
    while end > entry + 1 && lines[end - 1].trim_start().starts_with('#') {
        end -= 1;
    }

    (start + 1, lines[start..end].join("\n"))
}

/// Every `path:line` and `path:first-last` citation of a text, as `(path, first, last)`.
///
/// A single-line citation is the range of one line, so a caller only ever asks whether a line is
/// inside a cited range.
fn citations(text: &str) -> Vec<(String, usize, usize)> {
    let mut found = Vec::new();
    for (index, _) in text.match_indices(':') {
        let head = &text[..index];
        let start = head
            .rfind(|character: char| {
                !(character.is_ascii_alphanumeric() || "._-/".contains(character))
            })
            .map_or(0, |position| position + 1);
        let path = &head[start..];
        if !path.contains('/') || !(path.ends_with(".rs") || path.ends_with(".yaml")) {
            continue;
        }
        let digits: String = text[index + 1..]
            .chars()
            .take_while(|character| character.is_ascii_digit() || *character == '-')
            .collect();
        let mut parts = digits.split('-').filter(|part| !part.is_empty());
        let Some(Ok(first)) = parts.next().map(str::parse::<usize>) else {
            continue;
        };
        let last = parts
            .next()
            .and_then(|part| part.parse::<usize>().ok())
            .unwrap_or(first);
        found.push((path.to_owned(), first, last));
    }
    found
}

/// The `UNMAPPED:` markers of one declaration: each from its marker line to the end of the
/// contiguous comment run that carries it.
fn unmapped_markers(declaration: &str) -> String {
    let lines = declaration.lines().collect::<Vec<_>>();
    let mut kept = Vec::new();
    let mut inside = false;
    for line in &lines {
        let comment = line.trim_start().starts_with('#');
        if !comment {
            inside = false;
            continue;
        }
        if line.contains("UNMAPPED:") {
            inside = true;
        }
        if inside {
            kept.push(*line);
        }
    }
    kept.join("\n")
}

/// The `summary:` lines of the declared outcomes — the document's own citation for each outcome it
/// declares, and nothing else. A comment is not one.
fn outcome_summaries(declaration: &str) -> String {
    declaration
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter(|line| line.trim_start().starts_with("summary:"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The number a counting word names, for the words this document counts refusals with.
fn counted(word: &str) -> Option<usize> {
    [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    ]
    .iter()
    .position(|spelling| *spelling == word)
}

/// **A command declares an outcome for every refusal its cited function performs, or marks the
/// site unmapped — and says how many there are.**
///
/// This is the shape of the two claims below, and it is written once because they are one claim.
/// The document states, in the comment above each command, how many places the cited function
/// refuses in; that sentence has to be there and the number in it has to be the measured one. Then
/// every one of those lines has to fall inside a citation the *command itself* carries — either in
/// a declared outcome's `summary:`, or inside an `UNMAPPED:` marker that says why it is not
/// declared. The explanatory comment above the command grants nothing: it is prose about the
/// declaration, not part of it, which is exactly the distinction an earlier version of this file
/// lost when it made the whole assertion conditional on one sentence still being present.
///
/// Deleting a declared outcome therefore fails here, because the refusal it covered is left
/// uncovered — which is the defect an independent review constructed and this fence did not catch.
fn refusal_coverage_refusals(
    document: &str,
    declaration: &str,
    command: &str,
    path: &str,
    function: &str,
    first_line: usize,
    sites: &[(usize, String)],
) -> Vec<String> {
    let mut refusals = Vec::new();
    let prose = prose(declaration);

    let stated = prose
        .split_once("refuses in ")
        .and_then(|(_, rest)| rest.split_once(" places"))
        .map(|(word, _)| word.to_owned());
    match stated.as_deref().and_then(counted) {
        None => refusals.push(format!(
            "{document} declares `{command}` and does not say how many places `{function}` \
             refuses in. The sentence has to be there: this document's premise is that every \
             outcome was read from the function, and a count nobody wrote down is a reading \
             nobody can check. `{function}` refuses in {} places.",
            sites.len()
        )),
        Some(count) if count != sites.len() => refusals.push(format!(
            "{document} says `{function}` refuses in {count} places; it refuses in {} — \
             {path}:{}",
            sites.len(),
            sites
                .iter()
                .map(|(line, _)| line.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
        Some(_) => {}
    }

    if !prose.contains(&format!("{path}:{first_line}")) {
        refusals.push(format!(
            "{document} declares `{command}` without citing `{function}` at {path}:{first_line}, \
             which is the function this fence measures"
        ));
    }

    let declared = citations(&outcome_summaries(declaration));
    let unmapped = citations(&unmapped_markers(declaration));
    let covers = |citation: &[(String, usize, usize)], line: usize| {
        citation
            .iter()
            .any(|(cited, first, last)| cited == path && *first <= line && line <= *last)
    };
    for (line, text) in sites {
        if covers(&declared, *line) || covers(&unmapped, *line) {
            continue;
        }
        refusals.push(format!(
            "{path}:{line} refuses — `{text}` — and no outcome of `{command}` cites it and no \
             `UNMAPPED:` marker of `{command}` cites it. A refusal the tree performs and the \
             specification neither declares nor marks is a refusal a caller is never told about."
        ));
    }
    refusals
}

/// **Every `naming.wire` names a method the protocol actually accepts.**
///
/// `naming.wire` is not decoration: it is the address, taken verbatim, by everything downstream
/// that turns this specification into an artifact. Each command that carries one is cited by this
/// document to a variant of `ConnectionRequest` or `OperationRequest`, and both enums are
/// `#[serde(tag = "method", rename_all = "snake_case")]`, so the word a caller puts on the wire is
/// that variant in snake_case. The frozen contract vectors spell the same words out:
/// `contracts/connector-connection/v0alpha1/vectors.json` carries
/// `"method":"connect_session_create"` and `"method":"candidate_activate"`.
///
/// A `naming.wire` that is not one of them names a method every decoder in the tree refuses.
#[test]
fn every_declared_wire_name_is_a_method_the_protocol_accepts() {
    let root = workspace_root();

    let connection = read(&root, "crates/protocol/src/connection.rs");
    let operation = read(&root, "crates/protocol/src/operation.rs");
    let mut accepted = variants(&connection, "ConnectionRequest")
        .iter()
        .chain(variants(&operation, "OperationRequest").iter())
        .map(|variant| snake_case(variant))
        .collect::<Vec<_>>();
    accepted.sort();
    assert!(
        accepted
            .iter()
            .any(|method| method == "connect_session_create"),
        "this check reads the two request enums for the accepted method names and recognised \
         none of what it found; the enums moved, so read them again before believing any result \
         from it: {accepted:?}"
    );

    let mut wrong = Vec::new();
    for (path, source) in specification(&root) {
        let mut command = String::new();
        for (index, line) in source.lines().enumerate() {
            if let Some(name) = line.strip_prefix("  - name: ") {
                command = name.trim().to_owned();
            }
            let Some(wire) = line.strip_prefix("      wire: ") else {
                continue;
            };
            let wire = wire.trim();
            if !accepted.iter().any(|method| method == wire) {
                wrong.push(format!(
                    "{path}:{} `{command}` declares `naming.wire: {wire}`",
                    index + 1
                ));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "a `naming.wire` is the word a caller writes into the `method` tag, and both request \
         enums are `rename_all = \"snake_case\"`, so the accepted set is exactly:\n  {}\nThese \
         declare a word in neither enum, and a frame carrying one is refused by \
         `deny_unknown_fields` before it reaches any handler:\n  {}",
        accepted.join(", "),
        wrong.join("\n  ")
    );
}

/// `fn materialize` and the declaration that is a claim about it.
fn materialize() -> (String, String, usize, Vec<(usize, String)>) {
    let root = workspace_root();
    let declared = read(&root, "ess/system/domains/connection.yaml");
    let (_, declaration) = declaration(&declared, "connectors.connection.MaterializeObservation");
    let source = read(&root, "crates/integration-monitoring/src/backend.rs");
    let (first, _, body) = item(&source, "fn materialize(&self, observation_ref: &str) -> Result<ConnectionDescription, ConnectionError> {")
        .expect("`fn materialize` is declared in crates/integration-monitoring/src/backend.rs");
    (declared, declaration, first, refusal_sites(first, &body))
}

/// `fn session_terminate` and the declaration that is a claim about it.
fn session_terminate() -> (String, String, usize, Vec<(usize, String)>) {
    let root = workspace_root();
    let declared = read(&root, "ess/system/domains/runtime.yaml");
    let (_, declaration) = declaration(&declared, "connectors.runtime.TerminateSession");
    let source = read(&root, "crates/integration-sip/src/backend/sessions.rs");
    let (first, _, body) = item(&source, "pub(super) fn session_terminate(").expect(
        "`fn session_terminate` is declared in crates/integration-sip/src/backend/sessions.rs",
    );
    (declared, declaration, first, refusal_sites(first, &body))
}

/// **`MaterializeObservation` declares an outcome for every refusal `fn materialize` performs, or
/// marks the site unmapped.**
#[test]
fn materialize_declares_or_marks_every_refusal_its_cited_function_performs() {
    let (_, declaration, first, sites) = materialize();
    let refusals = refusal_coverage_refusals(
        "ess/system/domains/connection.yaml",
        &declaration,
        "connectors.connection.MaterializeObservation",
        "crates/integration-monitoring/src/backend.rs",
        "fn materialize",
        first,
        &sites,
    );
    assert!(refusals.is_empty(), "{}", refusals.join("\n  "));
}

/// **Deleting `MaterializeObservation`'s declared refusal outcomes is refused.**
///
/// The construction an independent review made against the shipped fence, which stayed green: the
/// three refusal outcomes were removed from `ess/system/domains/connection.yaml` and nothing
/// failed, because the assertion was `!declared.contains(claim) || …` and the claim had already
/// left the document. The deletion is made here against the text rather than the file, so the
/// case is permanent and needs no probe.
#[test]
fn deleting_materializes_declared_refusal_outcomes_is_refused() {
    let (_, declaration, first, sites) = materialize();
    let cut = without_error_outcomes(&declaration);
    assert_ne!(cut, declaration, "the deletion has to change the text");
    let refusals = refusal_coverage_refusals(
        "ess/system/domains/connection.yaml",
        &cut,
        "connectors.connection.MaterializeObservation",
        "crates/integration-monitoring/src/backend.rs",
        "fn materialize",
        first,
        &sites,
    );
    assert!(
        refusals.len() >= 4,
        "`MaterializeObservation`'s three refusal outcomes were deleted and the fence raised \
         {} refusals; the four refusal sites they cite — 1183, 1185, 1208 and 1218 — each have \
         to be reported as uncovered: {refusals:?}",
        refusals.len()
    );
}

/// **`TerminateSession` declares an outcome for every refusal `fn session_terminate` performs.**
#[test]
fn session_terminate_declares_or_marks_every_refusal_its_cited_function_performs() {
    let (_, declaration, first, sites) = session_terminate();
    let refusals = refusal_coverage_refusals(
        "ess/system/domains/runtime.yaml",
        &declaration,
        "connectors.runtime.TerminateSession",
        "crates/integration-sip/src/backend/sessions.rs",
        "fn session_terminate",
        first,
        &sites,
    );
    assert!(refusals.is_empty(), "{}", refusals.join("\n  "));
}

/// **Deleting `TerminateSession`'s declared refusal outcomes is refused.** The same construction,
/// against the other command that carries the same claim.
#[test]
fn deleting_session_terminates_declared_refusal_outcomes_is_refused() {
    let (_, declaration, first, sites) = session_terminate();
    let cut = without_error_outcomes(&declaration);
    assert_ne!(cut, declaration, "the deletion has to change the text");
    let refusals = refusal_coverage_refusals(
        "ess/system/domains/runtime.yaml",
        &cut,
        "connectors.runtime.TerminateSession",
        "crates/integration-sip/src/backend/sessions.rs",
        "fn session_terminate",
        first,
        &sites,
    );
    assert!(
        refusals.len() >= 2,
        "`TerminateSession`'s two refusal outcomes were deleted and the fence raised {} \
         refusals: {refusals:?}",
        refusals.len()
    );
}

/// **Deleting the sentence that counts the refusal sites is refused.**
///
/// The other half of the class. A fence that needs a sentence to exist has to assert the sentence
/// exists; the shipped one asserted the opposite by construction, so removing the sentence was the
/// cheapest way to make it pass.
#[test]
fn deleting_the_sentence_that_counts_the_refusal_sites_is_refused() {
    for (document, command, path, function, declaration, first, sites) in [
        (
            "ess/system/domains/connection.yaml",
            "connectors.connection.MaterializeObservation",
            "crates/integration-monitoring/src/backend.rs",
            "fn materialize",
            materialize().1,
            materialize().2,
            materialize().3,
        ),
        (
            "ess/system/domains/runtime.yaml",
            "connectors.runtime.TerminateSession",
            "crates/integration-sip/src/backend/sessions.rs",
            "fn session_terminate",
            session_terminate().1,
            session_terminate().2,
            session_terminate().3,
        ),
    ] {
        let cut = declaration
            .lines()
            .filter(|line| !line.contains("refuses in "))
            .collect::<Vec<_>>()
            .join("\n");
        assert_ne!(
            cut, declaration,
            "{command}: the deletion has to change the text"
        );
        let refusals =
            refusal_coverage_refusals(document, &cut, command, path, function, first, &sites);
        assert!(
            refusals
                .iter()
                .any(|refusal| refusal.contains("does not say how many places")),
            "{command}: the sentence counting the refusal sites was deleted and the fence did \
             not refuse: {refusals:?}"
        );
    }
}

/// One declaration with every outcome that carries an `error:` removed — the deletion an
/// independent review made by hand.
fn without_error_outcomes(declaration: &str) -> String {
    let lines = declaration.lines().collect::<Vec<_>>();
    let mut kept = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let starts_an_outcome = lines[index].starts_with("      - name: ");
        if !starts_an_outcome {
            kept.push(lines[index]);
            index += 1;
            continue;
        }
        let mut end = index + 1;
        while end < lines.len()
            && !lines[end].starts_with("      - name: ")
            && !lines[end].trim_start().starts_with('#')
            && !lines[end].trim().is_empty()
        {
            end += 1;
        }
        let carries_an_error = lines[index..end]
            .iter()
            .any(|line| line.trim_start().starts_with("error:"));
        if !carries_an_error {
            kept.extend_from_slice(&lines[index..end]);
        }
        index = end;
    }
    kept.join("\n")
}

/// Every file under `crates/integration-catalog/src` that assigns `ConnectSessionState::Failed`.
fn hosted_failed_writers() -> Vec<String> {
    let root = workspace_root();
    let mut writers = Vec::new();
    let mut stack = vec![root.join("crates/integration-catalog/src")];
    while let Some(current) = stack.pop() {
        let entries = std::fs::read_dir(&current).expect("crates/integration-catalog/src");
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if path.is_dir() {
                stack.push(path);
            } else if name.ends_with(".rs") && !name.ends_with("_tests.rs") {
                let source = std::fs::read_to_string(&path).expect("a rust source");
                for (index, line) in source.lines().enumerate() {
                    if line.contains("state = ConnectSessionState::Failed")
                        || line.contains("state: ConnectSessionState::Failed")
                    {
                        writers.push(format!(
                            "{}:{}",
                            path.strip_prefix(&root).unwrap_or(&path).display(),
                            index + 1
                        ));
                    }
                }
            }
        }
    }
    writers
}

/// **The hosted registry never reaches `Failed`, and the document says so in those words.**
///
/// `ess/system/domains/connection.yaml` states, of the `ConnectSession` marker, that "nothing
/// under `crates/integration-catalog/src` ever assigns `ConnectSessionState::Failed`" and that
/// `Failed` "is written only by the other registry, at
/// crates/service/src/connect_session.rs:233". Both halves are asserted, and the sentence itself
/// is asserted to be there: an earlier revision of this check read
/// `!declared.contains(claim) || !writers.is_empty()`, which was vacuous once the claim left the
/// document and, had the guard been dropped, would have asserted the *opposite* of what the
/// corrected document says.
fn hosted_failed_refusals(declared: &str, writers: &[String], other_registry: &str) -> Vec<String> {
    const CLAIM: &str = "nothing under `crates/integration-catalog/src` ever assigns \
                         `ConnectSessionState::Failed`";
    const ELSEWHERE: &str = "`Failed` is written only by the other registry, at \
                             crates/service/src/connect_session.rs:233";
    let prose = prose(declared);
    let mut refusals = Vec::new();
    if !prose.contains(CLAIM) {
        refusals.push(format!(
            "ess/system/domains/connection.yaml has to state \"{CLAIM}\"; the `ConnectSession` \
             marker is what tells the next reader that half of the hosted terminal set is \
             unreachable, and a marker that stops saying it leaves the reader to guess"
        ));
    }
    if !prose.contains(ELSEWHERE) {
        refusals.push(format!(
            "ess/system/domains/connection.yaml has to state \"{ELSEWHERE}\", which is the other \
             half of the same fact"
        ));
    }
    if !writers.is_empty() {
        refusals.push(format!(
            "the document says nothing under crates/integration-catalog/src assigns \
             `ConnectSessionState::Failed`, and these do:\n  {}",
            writers.join("\n  ")
        ));
    }
    if !other_registry.contains("state = ConnectSessionState::Failed") {
        refusals.push(
            "crates/service/src/connect_session.rs:233 no longer assigns \
             `ConnectSessionState::Failed`, so the document cites a line that does not write it"
                .to_owned(),
        );
    }
    refusals
}

#[test]
fn the_hosted_registry_never_reaches_the_state_its_marker_says_it_cannot() {
    let root = workspace_root();
    let declared = read(&root, "ess/system/domains/connection.yaml");
    let other = read(&root, "crates/service/src/connect_session.rs");
    let line = other.lines().nth(232).unwrap_or("").to_owned();
    let refusals = hosted_failed_refusals(&declared, &hosted_failed_writers(), &line);
    assert!(refusals.is_empty(), "{}", refusals.join("\n  "));
}

/// **A hosted registry that gained a `Failed` write, and a document that dropped the sentence, are
/// both refused.**
#[test]
fn the_hosted_failed_claim_is_refused_from_either_side() {
    let root = workspace_root();
    let declared = read(&root, "ess/system/domains/connection.yaml");
    let other = read(&root, "crates/service/src/connect_session.rs");
    let line = other.lines().nth(232).unwrap_or("").to_owned();

    let writer = vec!["crates/integration-catalog/src/hosted.rs:900".to_owned()];
    assert!(
        !hosted_failed_refusals(&declared, &writer, &line).is_empty(),
        "the hosted registry gained a `ConnectSessionState::Failed` write and the marker still \
         says nothing writes one, and the fence raised nothing"
    );

    let silent = declared.replace("ever assigns `ConnectSessionState::Failed`", "");
    assert_ne!(silent, declared, "the deletion has to change the text");
    assert!(
        !hosted_failed_refusals(&silent, &[], &line).is_empty(),
        "the sentence this check exists for was deleted from the document and the fence raised \
         nothing, which is exactly how the shipped assertion passed"
    );
}

/// The guard lines and the `Completed` write of `complete_hosted_session`, as absolute lines.
fn hosted_completion() -> (Vec<usize>, usize, usize, usize) {
    let root = workspace_root();
    let source = read(&root, "crates/integration-catalog/src/hosted.rs");
    let (first, last, body) = item(&source, "async fn complete_hosted_session(")
        .expect("`complete_hosted_session` is the function that writes `Completed`");
    let body = body.lines().collect::<Vec<_>>();
    let write = body
        .iter()
        .position(|line| line.contains("state = ConnectSessionState::Completed"))
        .expect("the function writes `Completed`");
    let guards = body
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains("state == ConnectSessionState::Pending"))
        .map(|(offset, _)| first + offset)
        .collect();
    (guards, first + write, first, last)
}

/// **The hosted completion path takes the `Pending` guard once, where the document says it does,
/// and does not re-take it after the awaits.**
///
/// `ess/system/domains/connection.yaml` says `complete_hosted_session` "does take the `Pending`
/// guard the other registry takes — it filters on `session.state ==
/// ConnectSessionState::Pending` (crates/integration-catalog/src/hosted.rs:731)", and that it then
/// "re-looks the session up at :754-758 without re-checking the state". Both halves are one
/// measurement: the set of guard lines is exactly the one line the document cites. A guard that
/// moved fails, a guard that disappeared fails, and a re-check that was added fails — the last of
/// those being the signal that `story:hosted-session-completes-out-of-terminal-state` has landed
/// and this marker has to go.
///
/// An earlier revision asserted `guards.is_empty()`, guarded on a sentence that had already left
/// the document — so it was vacuous, and dropping the guard would have asserted the opposite of
/// what the corrected document says.
fn hosted_guard_refusals(declared: &str, guards: &[usize], write: usize) -> Vec<String> {
    const CLAIM: &str = "it filters on `session.state == ConnectSessionState::Pending` \
                         (crates/integration-catalog/src/hosted.rs:731)";
    const RACE: &str = "re-looks the session up at :754-758 without re-checking the state";
    let prose = prose(declared);
    let mut refusals = Vec::new();
    if !prose.contains(CLAIM) {
        refusals.push(format!(
            "ess/system/domains/connection.yaml has to state \"{CLAIM}\". The settle clause of \
             this marker used to ask for a guard the path already takes, and saying which line \
             takes it is what stopped that being askable again"
        ));
    }
    if !prose.contains(RACE) {
        refusals.push(format!(
            "ess/system/domains/connection.yaml has to state \"{RACE}\", which is the narrower \
             defect the marker actually carries"
        ));
    }
    if guards != [731] {
        refusals.push(format!(
            "the document says `complete_hosted_session` guards on `Pending` at \
             crates/integration-catalog/src/hosted.rs:731 and re-checks nowhere; the guard lines \
             are {guards:?} and the `Completed` write is at line {write}. One guard, at the cited \
             line: a second one after the awaits means the re-check landed and \
             `story:hosted-session-completes-out-of-terminal-state` is done, so drop this marker \
             from ess/system/domains/connection.yaml rather than editing this assertion"
        ));
    }
    refusals
}

#[test]
fn the_hosted_completion_path_takes_the_guard_the_marker_says_it_takes() {
    let root = workspace_root();
    let declared = read(&root, "ess/system/domains/connection.yaml");
    let (guards, write, _, _) = hosted_completion();
    let refusals = hosted_guard_refusals(&declared, &guards, write);
    assert!(refusals.is_empty(), "{}", refusals.join("\n  "));
}

/// **A path that lost the guard, one that gained a re-check, and a document that dropped the
/// sentence are all refused.**
#[test]
fn the_hosted_guard_claim_is_refused_from_either_side() {
    let root = workspace_root();
    let declared = read(&root, "ess/system/domains/connection.yaml");
    let (guards, write, _, _) = hosted_completion();

    assert!(
        !hosted_guard_refusals(&declared, &[], write).is_empty(),
        "`complete_hosted_session` lost its `Pending` guard and the fence raised nothing"
    );
    assert!(
        !hosted_guard_refusals(&declared, &[731, 757], write).is_empty(),
        "`complete_hosted_session` gained a re-check after the awaits — which is the defect this \
         marker carries, fixed — and the fence raised nothing, so nothing would ever say the \
         marker can go"
    );

    let silent = declared.replace(
        "it filters on `session.state",
        "it never looks at `session.state",
    );
    assert_ne!(silent, declared, "the deletion has to change the text");
    assert!(
        !hosted_guard_refusals(&silent, &guards, write).is_empty(),
        "the document stopped saying the guard is taken and the fence raised nothing"
    );
}

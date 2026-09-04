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

/// Every line of one function body that refuses — returns an error instead of a value.
///
/// The three forms this tree uses: an early `return Err(..)`, an `ok_or_else(..)?` on a lookup
/// that may be empty, and a `map_err(..)?` on a fallible effect.
fn refusal_sites(first_line: usize, body: &str) -> Vec<String> {
    body.lines()
        .enumerate()
        .filter(|(_, line)| {
            let line = line.trim();
            line.contains("return Err(")
                || line.contains(".ok_or_else(")
                || line.contains(".ok_or(")
                || line.contains(".map_err(")
        })
        .map(|(offset, line)| format!("{}: {}", first_line + offset, line.trim()))
        .collect()
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

/// **`MaterializeObservation` says it performs one refusal; its cited function performs six.**
///
/// `ess/system/domains/connection.yaml:608` reads "That is the `Withdrawn` state and it is the one
/// refusal this command performs", and the command declares that one refusal outcome and no
/// other. The function it cites — `fn materialize`,
/// `crates/integration-monitoring/src/backend.rs:1177` — refuses an unknown `observation_ref`
/// with `NotFound` on its first statement, which is the same refusal the sibling
/// `ActivateCandidate` command *does* declare, as `connectors.connection.CandidateNotFound`.
#[test]
fn materialize_performs_the_one_refusal_the_specification_claims_for_it() {
    let root = workspace_root();
    let claim = "it is the one refusal this command performs";
    let document = "ess/system/domains/connection.yaml";
    let declared = read(&root, document);

    let source = read(&root, "crates/integration-monitoring/src/backend.rs");
    let (first, _, body) = item(&source, "fn materialize(&self, observation_ref: &str) -> Result<ConnectionDescription, ConnectionError> {")
        .expect("`fn materialize` is declared in crates/integration-monitoring/src/backend.rs");
    let sites = refusal_sites(first, &body);

    assert!(
        !declared.contains(claim) || sites.len() == 1,
        "{document} says of `connectors.connection.MaterializeObservation`: \"{claim}\". \
         `fn materialize` at crates/integration-monitoring/src/backend.rs:{first} refuses in {} \
         places:\n  {}\nThe first is an unknown `observation_ref`, which is exactly what \
         `connectors.connection.ActivateCandidate` models as `CandidateNotFound`. Either the \
         command gains the outcomes, or the sentence stops claiming there is one.",
        sites.len(),
        sites.join("\n  ")
    );
}

/// **`TerminateSession` says `not-found` is its only refusal; the path refuses twice.**
///
/// `ess/system/domains/runtime.yaml:258` reads "It is the only refusal this path performs", and
/// the command declares `terminating` and `not-found` and nothing else. `fn session_terminate`
/// also propagates an audit-append failure as `OperationErrorCode::Unavailable`
/// (`crates/integration-sip/src/backend/sessions.rs:65`, through `unavailable()` at
/// `crates/integration-sip/src/backend/mod.rs:733-739`), and this document's error block declares
/// no counterpart for it.
#[test]
fn session_terminate_performs_the_only_refusal_the_specification_claims_for_it() {
    let root = workspace_root();
    let claim = "It is the only refusal this path performs";
    let document = "ess/system/domains/runtime.yaml";
    let declared = read(&root, document);

    let source = read(&root, "crates/integration-sip/src/backend/sessions.rs");
    let (first, _, body) = item(&source, "pub(super) fn session_terminate(").expect(
        "`fn session_terminate` is declared in crates/integration-sip/src/backend/sessions.rs",
    );
    let sites = refusal_sites(first, &body);

    assert!(
        !declared.contains(claim) || sites.len() == 1,
        "{document} says of `connectors.runtime.TerminateSession`'s `not-found` outcome: \
         \"{claim}\". `fn session_terminate` at \
         crates/integration-sip/src/backend/sessions.rs:{first} refuses in {} places:\n  {}\n\
         The second is `OperationErrorCode::Unavailable` from the audit append, and no outcome \
         of this command names it.",
        sites.len(),
        sites.join("\n  ")
    );
}

/// **The hosted `UNMAPPED:` marker names a transition nothing in that registry can perform.**
///
/// `ess/system/domains/connection.yaml:209` says that on the hosted completion path
/// "`Expired -> Completed` and `Failed -> Completed` are reachable". The hosted registry holds its
/// sessions in a `Mutex<BTreeMap<String, Session>>` built empty at open and never restored from
/// the state file, and nothing under `crates/integration-catalog/src` ever puts
/// `ConnectSessionState::Failed` into one — that variant is written only by
/// `ConnectSessionLifecycle::fail_pending`, in the *other* registry, over a different map.
///
/// So half the marker states a move no code can make. This document's premise is that a
/// transition it names was read from a site that performs it; naming an unperformable one in a
/// comment is the same defect as declaring one, moved out of the checked part of the file.
#[test]
fn the_hosted_registry_can_reach_the_state_its_marker_says_it_reaches() {
    let root = workspace_root();
    let claim = "`Failed -> Completed` are reachable";
    let document = "ess/system/domains/connection.yaml";
    let declared = read(&root, document);

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

    assert!(
        !declared.contains(claim) || !writers.is_empty(),
        "{document} states that on the hosted path {claim}. Nothing under \
         crates/integration-catalog/src ever assigns `ConnectSessionState::Failed`, and the \
         hosted session map is built empty (crates/integration-catalog/src/hosted.rs:181) and \
         never deserialized, so a hosted session is never `Failed` and that half of the marker \
         names a transition the tree cannot perform. `Failed` is written only by the other \
         registry, at crates/service/src/connect_session.rs:233, over its own map."
    );
}

/// **The hosted completion path is claimed to have no state guard, and it has one.**
///
/// `ess/system/domains/connection.yaml:208` says
/// `crates/integration-catalog/src/hosted.rs:754-758` "looks the session up and writes `Completed`
/// with no state guard at all", and the marker asks to be settled by "that path taking the guard
/// the other registry has (`crates/service/src/connect_session.rs:204`)".
///
/// `complete_hosted_session` already takes that guard: it filters on
/// `session.state == ConnectSessionState::Pending` before it does anything and returns
/// `HostedCompletionError::NotFound` when the filter drops the session. What it does not do is
/// re-check after `verify_credential` and `commit_connection` are awaited and the lock has been
/// released. That is a narrower defect with a different fix, and the settle clause as written is
/// already satisfied — so the marker cannot be closed by doing what it asks.
#[test]
fn the_hosted_completion_path_writes_completed_without_the_guard_the_marker_denies_it_has() {
    let root = workspace_root();
    let claim = "with no state guard at all";
    let document = "ess/system/domains/connection.yaml";
    let declared = read(&root, document);

    let source = read(&root, "crates/integration-catalog/src/hosted.rs");
    let (first, last, body) = item(&source, "async fn complete_hosted_session(")
        .expect("`complete_hosted_session` is the function that writes hosted.rs:758");
    let body = body.lines().collect::<Vec<_>>();
    let write = body
        .iter()
        .position(|line| line.contains("state = ConnectSessionState::Completed"))
        .expect("the function writes `Completed`");
    let guards = body[..write]
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains("state == ConnectSessionState::Pending"))
        .map(|(offset, line)| format!("{}: {}", first + offset, line.trim()))
        .collect::<Vec<_>>();

    assert!(
        !declared.contains(claim) || guards.is_empty(),
        "{document} says of crates/integration-catalog/src/hosted.rs:754-758 that it writes \
         `Completed` \"{claim}\", and asks to be settled by that path taking the guard \
         crates/service/src/connect_session.rs:204 has. The path already guards on `Pending` \
         before its write at line {}:\n  {}\nWhat it lacks is a re-check after the awaits, \
         inside crates/integration-catalog/src/hosted.rs:{first}-{last} — a narrower claim with \
         a different fix, which doing what the settle clause literally asks would not make.",
        first + write,
        guards.join("\n  ")
    );
}

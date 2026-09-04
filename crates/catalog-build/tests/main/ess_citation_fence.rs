//! **The ESS specification's citations, read against the tree they cite.**
//!
//! `ess/system/` states one thing about itself: every state, transition and field is read from
//! this repository and cited by `path:line`, and everything that could not be read carries an
//! `UNMAPPED:` marker naming what would settle it. `ess validate` checks that the document is
//! well-formed against its own schema; it never opens a cited file. Nothing else compares the two.
//!
//! These checks open them. Each one is a single claim the document makes, expressed as an
//! assertion over the cited source, so that a citation which drifts away from what it points at
//! fails here rather than being believed by the next reader.

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate manifest lives two directories below the workspace root")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// Every ESS document of the specification, in a stable order.
fn specification_files(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![
        root.join("ess/system/system.yaml"),
        root.join("ess/system/components.yaml"),
    ];
    let mut domains = std::fs::read_dir(root.join("ess/system/domains"))
        .expect("ess/system/domains")
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "yaml")
        })
        .collect::<Vec<_>>();
    domains.sort();
    files.extend(domains);
    files
}

/// A `path:line` citation is a source suffix followed immediately by a digit. The document only
/// ever cites `.rs`, `.md` and `.yaml` sources, so the three suffixes are the whole vocabulary.
fn carries_a_citation(text: &str) -> bool {
    [".rs:", ".md:", ".yaml:"].iter().any(|suffix| {
        text.match_indices(suffix).any(|(index, _)| {
            text[index + suffix.len()..]
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
        })
    })
}

/// The comment block immediately above `index`, which is where this document puts its citations.
fn preceding_comment_block(lines: &[&str], index: usize) -> String {
    let mut first = index;
    while first > 0 && lines[first - 1].trim_start().starts_with('#') {
        first -= 1;
    }
    lines[first..index].join("\n")
}

/// Every entry of one top-level block (`errors:`, `commands:`, `entities:`) with its line number.
fn entries_of_block(source: &str, block: &str) -> Vec<(usize, String)> {
    let lines = source.lines().collect::<Vec<_>>();
    let Some(start) = lines.iter().position(|line| *line == format!("{block}:")) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    for (offset, line) in lines.iter().enumerate().skip(start + 1) {
        let is_top_level_key = !line.is_empty() && !line.starts_with(' ') && !line.starts_with('#');
        if is_top_level_key {
            break;
        }
        if let Some(name) = line.strip_prefix("  - name: ") {
            entries.push((offset, name.trim().to_owned()));
        }
    }
    entries
}

/// The 1-based inclusive line span of `pub struct <name> {` in `source`, brace-counted.
fn struct_span(source: &str, name: &str) -> Option<(usize, usize)> {
    let lines = source.lines().collect::<Vec<_>>();
    let opener = format!("pub struct {name} {{");
    let start = lines.iter().position(|line| line.trim() == opener)?;
    let mut depth = 0usize;
    for (offset, line) in lines.iter().enumerate().skip(start) {
        depth += line.matches('{').count();
        depth -= line.matches('}').count();
        if depth == 0 {
            return Some((start + 1, offset + 1));
        }
    }
    None
}

/// Every `*.rs` file under `directory` that is not itself a test.
fn implementation_sources(directory: &Path) -> Vec<PathBuf> {
    fn walk(directory: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if path.is_dir() {
                if !matches!(name, "target" | "tests") {
                    walk(&path, found);
                }
            } else if name.ends_with(".rs") && !name.ends_with("_tests.rs") {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(directory, &mut found);
    found.sort();
    found
}

/// **Every error the specification declares is either cited or explicitly unmapped.**
///
/// The document's own rule. An error that names neither a refusal in the tree nor an `UNMAPPED:`
/// marker is a refusal nothing performs, presented as one that something does.
#[test]
fn every_declared_error_is_cited_or_unmapped() {
    let root = workspace_root();
    let mut uncited = Vec::new();
    for file in specification_files(&root) {
        let source = read(&file);
        let lines = source.lines().collect::<Vec<_>>();
        for (index, name) in entries_of_block(&source, "errors") {
            let block = preceding_comment_block(&lines, index);
            if !block.contains("UNMAPPED:") && !carries_a_citation(&block) {
                uncited.push(format!(
                    "{}:{} {name}",
                    file.strip_prefix(&root).unwrap_or(&file).display(),
                    index + 1
                ));
            }
        }
    }
    assert!(
        uncited.is_empty(),
        "the specification claims every error is read from the tree and cited, or carries an \
         `UNMAPPED:` marker naming what would settle it. These carry neither:\n  {}",
        uncited.join("\n  ")
    );
}

/// **A `Fields:` citation spans the struct it names.**
///
/// Every entity says which projection its fields were read from, as ``Fields: `X`, path:a-b``.
/// The range has to contain `X`: a range that stops inside the struct silently drops the fields
/// below the cut, and reads to the next person as if the whole projection had been modelled.
#[test]
fn field_citations_span_the_struct_they_name() {
    let root = workspace_root();
    let mut short = Vec::new();
    for file in specification_files(&root) {
        let source = read(&file);
        for (index, _) in source.match_indices("Fields: `") {
            let rest = &source[index + "Fields: `".len()..];
            let (name, rest) = rest.split_once('`').expect("a closing backtick");
            let citation = rest
                .trim_start_matches([',', ' '])
                .lines()
                .next()
                .expect("a citation on one line")
                .trim()
                .trim_end_matches('.');
            let (path, range) = citation.rsplit_once(':').expect("a `path:line` citation");
            let (from, to) = range.split_once('-').expect("a `path:a-b` citation");
            let from: usize = from.trim().parse().expect("a first line number");
            let to: usize = to.trim().parse().expect("a last line number");
            let cited = read(&root.join(path));
            let (start, end) = struct_span(&cited, name)
                .unwrap_or_else(|| panic!("`pub struct {name}` is not declared in {path}"));
            if from > start || to < end {
                short.push(format!(
                    "{}: `{name}` is declared at {path}:{start}-{end}, cited as {path}:{from}-{to}",
                    file.strip_prefix(&root).unwrap_or(&file).display()
                ));
            }
        }
    }
    assert!(
        short.is_empty(),
        "a `Fields:` citation must contain the struct it names:\n  {}",
        short.join("\n  ")
    );
}

/// **Every place that writes a ConnectSession state is a place the specification read.**
///
/// `connectors.connection.ConnectSession` says its transitions were read from "the registry that
/// performs them", singular, and that every terminal is reached only from `Pending` because one
/// cited line enforces it. That is a claim about the whole tree, so every file that performs one
/// of those writes has to be a file the document opened.
#[test]
fn every_connect_session_state_write_is_cited() {
    let root = workspace_root();
    let specification = read(&root.join("ess/system/domains/connection.yaml"));
    let mut unread = Vec::new();
    for file in implementation_sources(&root.join("crates")) {
        let source = read(&file);
        if !source.contains("state = ConnectSessionState::") {
            continue;
        }
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string();
        if !specification.contains(&relative) {
            let lines = source
                .lines()
                .enumerate()
                .filter(|(_, line)| line.contains("state = ConnectSessionState::"))
                .map(|(index, line)| format!("{}: {}", index + 1, line.trim()))
                .collect::<Vec<_>>();
            unread.push(format!("{relative}\n      {}", lines.join("\n      ")));
        }
    }
    assert!(
        unread.is_empty(),
        "the ConnectSession lifecycle was read from one registry, and these also move it:\n  {}",
        unread.join("\n  ")
    );
}

/// **The `ChannelSummary` that carries a `connection_ref` is cited, and modelled.**
///
/// The protocol declares two projections named `ChannelSummary` and they differ: the one in
/// `connection.rs` is nested inside `ConnectionDescription`, where the Connection is already
/// named, and the one in `event.rs` carries `pub connection_ref: String`, not optional. A
/// specification that reads only the first one leaves the Channel-to-Connection direction
/// unmapped when the tree states it.
///
/// This replaces the case that asserted no `ChannelSummary` carries a `connection_ref`. That
/// assertion could only ever hold by deleting the field from a frozen protocol projection: the
/// finding was that the document's *marker* was wrong, and the marker is gone, so the check now
/// pins what the corrected document has to keep saying.
#[test]
fn the_channel_summary_that_carries_a_connection_ref_is_cited_and_modelled() {
    let root = workspace_root();
    let specification = read(&root.join("ess/system/domains/connection.yaml"));

    let mut carriers = Vec::new();
    for file in implementation_sources(&root.join("crates/protocol")) {
        let source = read(&file);
        let Some((start, end)) = struct_span(&source, "ChannelSummary") else {
            continue;
        };
        if source
            .lines()
            .take(end)
            .skip(start)
            .any(|line| line.trim().starts_with("pub connection_ref"))
        {
            carriers.push(
                file.strip_prefix(&root)
                    .unwrap_or(&file)
                    .display()
                    .to_string(),
            );
        }
    }
    assert!(
        !carriers.is_empty(),
        "this check exists for the `ChannelSummary` that carries a `connection_ref`; no \
         projection in `crates/protocol` carries one any more, so the finding it guards is gone"
    );

    let uncited = carriers
        .iter()
        .filter(|path| !specification.contains(path.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        uncited.is_empty(),
        "a `ChannelSummary` in the tree carries a `connection_ref`, and the specification does \
         not read the file that declares it:\n  {}",
        uncited.join("\n  ")
    );

    let channel = specification
        .split("- name: connectors.connection.Channel\n")
        .nth(1)
        .expect("the Channel entity");
    let channel = &channel[..channel.find("\n  - name: ").unwrap_or(channel.len())];
    let field = channel
        .lines()
        .position(|line| line.trim() == "- name: connection_ref")
        .expect(
            "the Channel entity carries a `connection_ref`: the tree states the relation, so \
             leaving it off the entity states the opposite",
        );
    let declared = channel.lines().nth(field + 1).unwrap_or("").trim();
    assert!(
        !declared.contains("Optional<"),
        "`crates/protocol/src/event.rs` declares `pub connection_ref: String`, which is not \
         optional; the entity declares it as `{declared}`"
    );
}

/// **`reobserve` lands where the specification says it lands.**
///
/// `DiscoveryObservation.reobserve` is declared `Withdrawn -> Observed`, read from the line of the
/// refresh pass that re-activates an observation it saw again. The state is not stored: it is
/// derived, and the derivation reaches `Materialized` before it reaches `Observed`, from a
/// `connection_ref` the re-activating branch never clears. So a materialized observation that was
/// withdrawn and then seen again returns to `Materialized`, and that transition is not declared.
#[test]
fn the_reobserve_site_leaves_a_connection_ref_and_the_specification_says_so() {
    let root = workspace_root();
    let path = root.join("crates/integration-monitoring/src/backend.rs");
    let source = read(&path);
    let lines = source.lines().collect::<Vec<_>>();

    let derivation = lines
        .iter()
        .position(|line| line.contains("fn observation_summary"))
        .expect("`observation_summary` derives the projected state");
    let materialized = lines
        .iter()
        .skip(derivation)
        .position(|line| line.contains("DiscoveryObservationState::Materialized"))
        .expect("the derivation reaches Materialized");
    let observed = lines
        .iter()
        .skip(derivation)
        .position(|line| line.contains("DiscoveryObservationState::Observed"))
        .expect("the derivation reaches Observed");
    assert!(
        materialized < observed,
        "this check assumes `connection_ref` outranks `target_provider` in the derivation; it no \
         longer does, so the finding it guards has changed shape"
    );

    let reactivation = lines
        .iter()
        .position(|line| line.trim() == "observation.active = true;")
        .expect("the refresh pass re-activates an observation it saw again");
    let branch_start = lines[..reactivation]
        .iter()
        .rposition(|line| line.trim_end().ends_with("=> {"))
        .expect("the re-seen branch");
    let mut depth = 0usize;
    let mut branch_end = branch_start;
    for (offset, line) in lines.iter().enumerate().skip(branch_start) {
        depth += line.matches('{').count();
        depth -= line.matches('}').count();
        if depth == 0 {
            branch_end = offset;
            break;
        }
    }
    let branch = lines[branch_start..=branch_end].join("\n");

    // Pinned to what the adapter does **today**, deliberately, and not to what it arguably should
    // do. The re-seen branch does not clear `connection_ref`, and `observation_summary` reads that
    // field before `target_provider` — so a withdrawn observation that had been materialized comes
    // back as `Materialized`, not `Observed`. The specification carries that as `rematerialize`
    // rather than declaring a transition the tree does not perform.
    //
    // Whether the adapter should clear the reference instead is
    // `story:reobserve-returns-a-withdrawn-observation-to-materialized`. That is a behaviour change
    // to a shipping adapter with an uncovered surface — clearing the reference leaves the child in
    // `state.children`, and `child_is_current` (`backend.rs:1684-1690`) never reads
    // `connection_ref`, so a later `materialize` may mint a duplicate — and it is not this unit's
    // to take.
    //
    // **When that story lands this case goes red**, which is the point. The fix then is to drop
    // `rematerialize` from `connection.yaml`, restore `reobserve: Withdrawn -> Observed`, and
    // invert the first assertion below.
    assert!(
        !branch.contains("connection_ref"),
        "the re-seen branch at crates/integration-monitoring/src/backend.rs:{} now mentions \
         `connection_ref`, so the behaviour this case pins has changed. Read \
         `story:reobserve-returns-a-withdrawn-observation-to-materialized`, then drop \
         `rematerialize` from ess/system/domains/connection.yaml and invert this \
         assertion:\n{branch}",
        reactivation + 1
    );

    let specification = read(&root.join("ess/system/domains/connection.yaml"));
    assert!(
        specification.contains("rematerialize"),
        "the adapter still re-derives a materialized observation into `Materialized` — \
         `connection_ref` is read at crates/integration-monitoring/src/backend.rs:{} before \
         `target_provider` — so the specification has to carry `rematerialize`. Declaring only \
         `reobserve: Withdrawn -> Observed` states a transition the tree does not perform.",
        derivation + materialized + 1
    );
}

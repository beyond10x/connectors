//! Adversary pass 2 against `story:mcp-inbound-local-binding`, written 2026-09-12 on
//! branch `unit/mcp-inbound-local-binding-20260912` at `2df2e0b`.
//!
//! Pass 1 measured that the traces inherited the session vocabulary's lifecycle and not
//! its clock. The correction answered that class and, in answering it, moved every
//! lease-dependent act in four of the eight traces onto the exact instant its lease
//! expires, and grew the deliverable from six scenario files to eight without the
//! acceptance statement that counts them changing. These two cases measure those.
//!
//! Neither case carries a millisecond literal or a file count of its own: the lease
//! ceiling is read out of `contracts/sessions/v1alpha1/semantics.md` §4.1 — reached
//! through the `# Normative owner:` comment of `ess/domains/sessions.yaml`, and
//! cross-checked against that model's own `lease lifetime <= … ms` — and the number of
//! scenario files is read out of the story's own `## Acceptance` statement. Each one
//! therefore survives its fix: correct the trace, or amend the statement, and the case
//! measures the corrected pair rather than a number this file remembers.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The story whose scenario files these cases quantify over. Two sibling stories add
/// files to the same directory under their own names.
const OWNER: &str = "story:mcp-inbound-local-binding";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    let path = root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn binding_document() -> String {
    read("adapters/mcp/contracts/server/v1alpha1/semantics.md")
}

/// A document as one line, so a sentence quoted from it is found whatever column the
/// author's hard wrap fell in.
fn flattened(document: &str) -> String {
    document.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The scenario files this story owns, by file stem, decided by the structured
/// `# Owner:` marker each file carries.
fn owned_scenarios() -> BTreeMap<String, String> {
    let directory = root().join("adapters/mcp/contracts/server/v1alpha1/scenarios");
    let mut found = BTreeMap::new();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&directory)
        .unwrap_or_else(|e| panic!("read {}: {e}", directory.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            path.is_file()
                && matches!(
                    path.extension().and_then(|ext| ext.to_str()),
                    Some("yaml" | "yml")
                )
        })
        .collect();
    paths.sort();
    for path in paths {
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let owns = text
            .lines()
            .find(|line| line.starts_with("# Owner:"))
            .map(|line| line["# Owner:".len()..].trim().to_string())
            .is_some_and(|owner| owner.split_whitespace().next() == Some(OWNER));
        if owns {
            let stem = path
                .file_stem()
                .expect("a scenario file name")
                .to_string_lossy()
                .to_string();
            found.insert(stem, text);
        }
    }
    found
}

// ── Case 1: the boundary the correction adopted ─────────────────────────────────────

/// The contract `ess/domains/sessions.yaml` names as the normative owner of the timing
/// obligations a reuser of `connectors.sessions.Session` inherits, read out of its own
/// comment rather than pinned here.
fn sessions_contract() -> String {
    let model = read("ess/domains/sessions.yaml");
    let path = regex::Regex::new(r"#\s*Normative owner:\s*([A-Za-z0-9_./-]+\.md)")
        .expect("a valid pattern")
        .captures(&model)
        .unwrap_or_else(|| {
            panic!(
                "ess/domains/sessions.yaml no longer names the normative owner of its timing \
                 obligations; this case cannot be derived without it"
            )
        })[1]
        .to_string();
    read(&path)
}

/// The maximum lifetime of a live data lease, in seconds, stated by the contract and
/// repeated by the model, with the two required to agree.
fn lease_ceiling_seconds() -> i64 {
    let contract = sessions_contract();
    let in_contract = regex::Regex::new(
        r"effective deadline of a data lease is at most \*\*([0-9,]+)\s*ms",
    )
    .expect("a valid pattern")
    .captures(&contract)
    .unwrap_or_else(|| {
        panic!(
            "the sessions contract no longer caps the effective deadline of a data lease; this \
             case cannot be derived without it"
        )
    })[1]
        .replace(',', "")
        .parse::<i64>()
        .expect("a whole number of milliseconds");
    let model = read("ess/domains/sessions.yaml");
    let in_model = regex::Regex::new(r"lease lifetime <=\s*([0-9,]+)\s*ms")
        .expect("a valid pattern")
        .captures(&model)
        .expect("ess/domains/sessions.yaml states the live data-lease ceiling")[1]
        .replace(',', "")
        .parse::<i64>()
        .expect("a whole number of milliseconds");
    assert_eq!(
        in_model, in_contract,
        "the model bounds a live data lease at {in_model} ms and its own normative owner bounds \
         it at {in_contract} ms; no trace can be held to a ceiling the two sources disagree about"
    );
    assert_eq!(
        in_contract % 1_000,
        0,
        "the data-lease ceiling is {in_contract} ms, which is not a whole number of seconds; the \
         traces are written in whole seconds and this case compares them in whole seconds"
    );
    in_contract / 1_000
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let shifted = (month + 9) % 12;
    let day_of_year = (153 * shifted + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// A scenario timestamp `YYYY-MM-DDTHH:MM:SSZ` as whole seconds since the epoch.
fn instant(text: &str) -> i64 {
    let bytes = text.as_bytes();
    assert!(
        text.len() == 20
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes[10] == b'T'
            && bytes[13] == b':'
            && bytes[16] == b':'
            && bytes[19] == b'Z',
        "a scenario timestamp is not of the form `YYYY-MM-DDTHH:MM:SSZ`: {text:?}"
    );
    let field = |from: usize, to: usize| {
        text[from..to]
            .parse::<i64>()
            .unwrap_or_else(|_| panic!("a scenario timestamp is not numeric: {text:?}"))
    };
    days_from_civil(field(0, 4), field(5, 7), field(8, 10)) * 86_400
        + field(11, 13) * 3_600
        + field(14, 16) * 60
        + field(17, 19)
}

/// The 1-based line of the first line containing `needle`, so a failure opens at the act
/// it is about.
fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.contains(needle))
        .map(|index| index + 1)
        .unwrap_or(0)
}

/// An act of a trace, in the terms the timing rules are written in.
struct Act {
    index: usize,
    at: i64,
    at_text: String,
    command: String,
    outcome: String,
    lease: Option<(i64, i64)>,
}

fn timeline(stem: &str, text: &str) -> Vec<Act> {
    let trace: serde_yaml_ng::Value = serde_yaml_ng::from_str(text)
        .unwrap_or_else(|e| panic!("parse scenarios/{stem}.yaml: {e}"));
    trace["timeline"]
        .as_sequence()
        .unwrap_or_else(|| panic!("scenarios/{stem}.yaml declares no timeline"))
        .iter()
        .enumerate()
        .map(|(index, act)| {
            let at_text = act["at"]
                .as_str()
                .unwrap_or_else(|| panic!("scenarios/{stem}.yaml act {index} has no `at:`"))
                .to_string();
            let lease = act["input"].get("lease").and_then(|lease| {
                Some((
                    instant(lease.get("issued_at")?.as_str()?),
                    instant(lease.get("effective_expiry")?.as_str()?),
                ))
            });
            Act {
                index,
                at: instant(&at_text),
                at_text,
                command: act["command"].as_str().unwrap_or_default().to_string(),
                outcome: act["outcome"].as_str().unwrap_or_default().to_string(),
                lease,
            }
        })
        .collect()
}

/// Every act that a live data lease has to admit — an admitted data decision, and the
/// renewal that replaces the lease — must fall **strictly before** the effective expiry
/// of the lease admitting it, never on it.
///
/// Two sources say so and the document under test is one of them.
///
/// `contracts/sessions/v1alpha1/semantics.md` §4.1 writes the expiry row as "gate stops
/// **by** the effective expiry, with **no additional 2 s grace**", makes that expiry "a
/// local terminal fact", and says a live-data-lease expiry observed "during readiness,
/// data admission or renewal atomically enters `closing`" — so at the expiry instant the
/// gate has stopped and the session is no longer admitting. The same section defines the
/// deadline as "at most 2,000 ms after authoritative issuance, **including delivery
/// delay, clock uncertainty, scheduling delay and already-buffered output**": the
/// deadline is the moment by which admitted output has already left, so an admission
/// placed on it necessarily delivers past it. And "A renewal arriving after local expiry
/// … cannot reopen the same session".
///
/// The binding document states the same rule in its own words, which is the assertion
/// this case makes first: `adapters/mcp/contracts/server/v1alpha1/semantics.md` §3.3
/// says the supervisor renews "**before** the effective deadline", and §2 says a lease
/// the supervisor "does not renew before the effective deadline" expires. Four of the
/// eight traces the same commit ships do it *on* the deadline instead.
#[test]
fn no_act_that_needs_a_live_lease_is_admitted_at_or_after_its_effective_expiry() {
    let document = flattened(&binding_document());
    assert!(
        document.contains("before the effective deadline"),
        "the inbound binding document no longer states that the supervisor renews a data lease \
         before its effective deadline; that sentence is one of the two sources this case \
         derives strictness from, and without it the rule below is not the document's own"
    );

    let ceiling = lease_ceiling_seconds();
    let mut offending = Vec::new();
    for (stem, text) in owned_scenarios() {
        let mut deadline: Option<(i64, usize)> = None;
        for act in timeline(&stem, &text) {
            let needs_lease = matches!(act.outcome.as_str(), "permitted" | "renewed");
            if needs_lease
                && let Some((live, issuer)) = deadline
                && act.at >= live
            {
                offending.push(format!(
                    "scenarios/{stem}.yaml:{} — act {} `{}` ({}) is admitted at {}, and the \
                     lease act {issuer} issued has its effective expiry at that same instant; \
                     the gate stops by the effective expiry, so the last admissible moment is \
                     before it",
                    line_of(&text, &format!("at: '{}'", act.at_text)),
                    act.index,
                    act.command,
                    act.outcome,
                    act.at_text,
                ));
            }
            if let Some((issued, expiry)) = act.lease
                && matches!(act.outcome.as_str(), "ready" | "renewed")
            {
                deadline = Some((std::cmp::min(expiry, issued + ceiling), act.index));
            }
        }
    }
    assert!(
        offending.is_empty(),
        "a trace admits data, or renews, at the very instant the lease admitting it expires. \
         §4.1 of the sessions contract stops the gate *by* the effective expiry with no grace \
         after it, makes that expiry a local terminal fact that atomically enters `closing`, \
         counts already-buffered output and delivery delay inside the deadline, and refuses a \
         renewal arriving after local expiry; the binding document itself says the supervisor \
         renews *before* the effective deadline. The last admissible moment is therefore \
         strictly earlier than the expiry, and these acts sit on it:\n{}",
        offending.join("\n")
    );
}

// ── Case 2: the count the acceptance states ─────────────────────────────────────────

fn number_word(word: &str) -> Option<usize> {
    Some(match word {
        "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        _ => return None,
    })
}

/// The `## Acceptance` section of the story that owns this document, without its heading.
fn acceptance_statement() -> String {
    let story = read(".engineering/planning/story/mcp-inbound-local-binding.md");
    let mut collecting = false;
    let mut found = String::new();
    for line in story.lines() {
        if line.starts_with("## ") {
            if collecting {
                break;
            }
            collecting = line.trim() == "## Acceptance";
            continue;
        }
        if collecting {
            found.push_str(line);
            found.push('\n');
        }
    }
    assert!(
        !found.trim().is_empty(),
        "{OWNER} has no `## Acceptance` section; this case measures the deliverable against it"
    );
    found
}

/// The acceptance states how many scenario files this story ships, and it states it twice
/// — "the six behaviours and their six scenario files correspond one to one" and "those
/// two plus the six scenario files above are the inbound half of the epic's acceptance
/// criterion 5". The deliverable has to carry that many.
///
/// The correction at `d0010db` split two behaviours into two routes each and gave every
/// route its own scenario file, taking the directory from six files to eight. That was
/// the right answer to pass 1's finding 5 read as a *table* defect; as a *deliverable* it
/// changed the countable thing the acceptance counts, and no planning record moved with
/// it — `git diff 4ad2ee5..2df2e0b -- .engineering/planning/` is empty. The check that
/// used to pin the number at six now compares the directory against the document's own
/// row count (`no_scenario_this_story_owns_names_an_outcome_the_document_does_not`,
/// `owned.len() == declared.len()`), so both halves are the unit's and nothing compares
/// either to the acceptance any more.
///
/// The count here is read out of the acceptance rather than written down, so amending the
/// statement fixes this case exactly as merging the traces back to six would.
#[test]
fn the_story_ships_the_number_of_scenario_files_its_acceptance_states() {
    // AMENDED by the coordinator, story:mcp-inbound-local-binding.
    //
    // This case read a literal count out of the acceptance and compared the directory
    // to it. That was right while the acceptance carried one: it was the only thing
    // comparing the deliverable to the statement rather than to the document the same
    // unit wrote, and it caught the count going stale when two behaviours were split
    // into routes.
    //
    // The coordinator amended the acceptance to derive the count instead of naming it,
    // because the split was correct and the number was what had gone stale. That
    // removes the literal this case read, so it can no longer assert what it asserted.
    //
    // What it asserts now is the property that keeps the drift from returning: the
    // acceptance states a per-route rule and carries no literal count at all. A later
    // editor who writes a number back re-creates exactly the defect this case was
    // written for, and this goes red on it.
    // Flattened: the statement is hard-wrapped prose, so a phrase can straddle a line
    // break. This file already learned that once — its first run failed on its own
    // derivation guard for exactly this reason.
    let acceptance = acceptance_statement()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let literal = regex::Regex::new(r"([a-z]+|\d+) scenario files")
        .expect("a valid pattern")
        .captures_iter(&acceptance)
        // Only a number counts. `number_word` is the file's own reader, so "their scenario
        // files" is prose and "six scenario files" is the defect.
        .filter(|capture| number_word(&capture[1]).is_some() || capture[1].parse::<usize>().is_ok())
        .map(|capture| capture[1].to_string())
        .collect::<Vec<_>>();
    assert!(
        literal.is_empty(),
        "{OWNER}'s acceptance names a literal scenario-file count {literal:?}. The count is \
         derived from the enumeration now, because a behaviour may carry more than one \
         observable route; a literal here goes stale the next time a route is added, which is \
         the defect this case exists for."
    );

    assert!(
        acceptance.contains("one scenario file per observable route"),
        "{OWNER}'s acceptance no longer states how the scenario-file count is derived. It must \
         say one scenario file per observable route, so the deliverable can be compared to the \
         statement and not only to the document the same unit wrote."
    );

    // The deliverable still has to be non-empty and owned: a derived count is not a licence
    // for an empty directory.
    let owned = owned_scenarios();
    assert!(
        !owned.is_empty(),
        "{OWNER} ships no scenario file this story owns"
    );
}

//! Adversary pass 1 for `story:mcp-inbound-local-binding`, written 2026-09-12 against
//! `1d22f58` on branch `unit/mcp-inbound-local-binding-20260912`.
//!
//! The unit shipped six scenario files and eight checks over them. The checks read the
//! `connectors.sessions.Session` *lifecycle* out of `ess/domains/sessions.yaml` — which
//! transition exists, which states it runs between, which command outcome performs it —
//! and nothing reads the *timing* obligations that live in the same vocabulary. The
//! binding document says in its own §6 why that matters: ESS "compiles obligations but
//! does NOT execute a sequential trace", so a trace can name only legal transitions, in
//! a legal order, and still describe a session no conforming supervisor could run.
//!
//! §4.1 of the normative owner, `contracts/sessions/v1alpha1/semantics.md`, is where
//! those obligations are:
//!
//! * "The effective deadline of a data lease is at most **2,000 ms after authoritative
//!   issuance**, including delivery delay, clock uncertainty, scheduling delay and
//!   already-buffered output";
//! * "gate stops by the effective expiry, with **no additional 2 s grace**"; a live
//!   lease "is mandatory for every admitted data path";
//! * the recorded close deadlines are "bounded by §4.1; earlier expiry/drain deadlines
//!   dominate", and a lease is renewed "never beyond a drain or other earlier deadline".
//!
//! Each case below derives its bound from two independent files and requires them to
//! agree, so a fix that edits one source into agreement with a trace is caught by the
//! other. `MCP_ADVERSARY_ROOT` points the cases at a copy carrying a candidate
//! correction, so a fix can be watched going green without editing the tree under
//! review — the override the sibling adversary files in this directory already use.

use std::path::{Path, PathBuf};

/// The story whose scenario files these cases hold to the timing contract. Sibling
/// stories add files to the same directory under their own names.
const OWNER: &str = "story:mcp-inbound-local-binding";

const SCENARIOS: &str = "adapters/mcp/contracts/server/v1alpha1/scenarios";

/// The private model that declares the session vocabulary these traces reuse.
const MODEL: &str = "ess/domains/sessions.yaml";

/// The normative owner that model names in its own header comment.
const CONTRACT: &str = "contracts/sessions/v1alpha1/semantics.md";

fn repo() -> PathBuf {
    match std::env::var_os("MCP_ADVERSARY_ROOT") {
        Some(root) => PathBuf::from(root),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
    }
}

fn read(relative: &str) -> String {
    let path = repo().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The maximum lifetime of a live data lease, in milliseconds, read out of the two
/// files that state it and required to agree. Never a literal in this file: a bound
/// this file carried itself would be a bound a correction could not move.
fn lease_ceiling_ms() -> i64 {
    let model = read(MODEL);
    let in_model = regex::Regex::new(r"lease lifetime <=\s*([0-9,]+)\s*ms")
        .expect("a valid pattern")
        .captures(&model)
        .unwrap_or_else(|| {
            panic!(
                "{MODEL} no longer states the live data-lease ceiling these traces inherit; \
                 the cases below cannot be derived without it"
            )
        })[1]
        .replace(',', "")
        .parse::<i64>()
        .expect("a whole number of milliseconds");
    let contract = read(CONTRACT);
    let in_contract =
        regex::Regex::new(r"effective deadline of a data lease is at most \*\*([0-9,]+)\s*ms")
            .expect("a valid pattern")
            .captures(&contract)
            .unwrap_or_else(|| {
                panic!("{CONTRACT} §4.1 no longer states the effective deadline of a data lease")
            })[1]
            .replace(',', "")
            .parse::<i64>()
            .expect("a whole number of milliseconds");
    assert_eq!(
        in_model, in_contract,
        "{MODEL} bounds a live data lease at {in_model} ms and its own normative owner \
         {CONTRACT} §4.1 bounds it at {in_contract} ms; the traces below cannot be held to \
         a ceiling the two sources disagree about"
    );
    in_model
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let shifted = (month + 9) % 12;
    let day_of_year = (153 * shifted + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// A scenario timestamp as whole seconds since the epoch. The traces write
/// `YYYY-MM-DDTHH:MM:SSZ` and their own header comments say whole-second acts express
/// causal order, so no sub-second form is accepted here.
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

/// The 1-based line of the first line of `text` containing `needle`, for a message a
/// reader can open at the right place.
fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line.contains(needle))
        .map(|index| index + 1)
        .unwrap_or(0)
}

/// The scenario files this story owns, as `(file stem, text, parsed)`.
fn owned_traces() -> Vec<(String, String, serde_yaml_ng::Value)> {
    let directory = repo().join(SCENARIOS);
    let mut paths = std::fs::read_dir(&directory)
        .unwrap_or_else(|e| panic!("read {}: {e}", directory.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            path.is_file()
                && matches!(
                    path.extension().and_then(|extension| extension.to_str()),
                    Some("yaml" | "yml")
                )
        })
        .collect::<Vec<_>>();
    paths.sort();
    let traces: Vec<(String, String, serde_yaml_ng::Value)> = paths
        .into_iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            if !text.contains(OWNER) {
                return None;
            }
            let stem = path
                .file_stem()
                .expect("a scenario file name")
                .to_string_lossy()
                .to_string();
            let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text)
                .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
            Some((stem, text, parsed))
        })
        .collect();
    assert!(
        !traces.is_empty(),
        "no scenario under {SCENARIOS} names {OWNER}; these cases would quantify over nothing"
    );
    traces
}

/// The acts of a trace's timeline, as `(index, at, command, outcome, input)`.
fn timeline(
    stem: &str,
    trace: &serde_yaml_ng::Value,
) -> Vec<(usize, i64, String, String, serde_yaml_ng::Value)> {
    trace["timeline"]
        .as_sequence()
        .unwrap_or_else(|| panic!("scenarios/{stem}.yaml declares no timeline"))
        .iter()
        .enumerate()
        .map(|(index, act)| {
            let at = act["at"]
                .as_str()
                .unwrap_or_else(|| panic!("scenarios/{stem}.yaml act {index} carries no `at:`"));
            (
                index,
                instant(at),
                act["command"].as_str().unwrap_or_default().to_string(),
                act["outcome"].as_str().unwrap_or_default().to_string(),
                act["input"].clone(),
            )
        })
        .collect()
}

/// `(issued_at, effective_expiry)` of an act's lease input, if it carries one.
fn lease(input: &serde_yaml_ng::Value) -> Option<(i64, i64)> {
    let lease = input.get("lease")?;
    let issued = lease.get("issued_at")?.as_str()?;
    let expiry = lease.get("effective_expiry")?.as_str()?;
    Some((instant(issued), instant(expiry)))
}

// ── The ceiling the traces inherit by reusing the vocabulary ────────────────────────

/// `contracts/sessions/v1alpha1/semantics.md` §4.1: "The effective deadline of a data
/// lease is at most **2,000 ms after authoritative issuance**, including delivery delay,
/// clock uncertainty, scheduling delay and already-buffered output." A trace that issues
/// a longer one describes a supervisor that would have to refuse its own session
/// admission ("It must refuse session admission if the selected binding cannot enforce
/// these ceilings for every data path").
#[test]
fn no_lease_a_trace_issues_outlives_the_ceiling_the_sessions_contract_sets() {
    let ceiling = lease_ceiling_ms();
    let mut offending = Vec::new();
    for (stem, text, trace) in owned_traces() {
        for (index, _, command, outcome, input) in timeline(&stem, &trace) {
            let Some((issued, expiry)) = lease(&input) else {
                continue;
            };
            let lifetime = (expiry - issued) * 1_000;
            if lifetime > ceiling {
                let expiry_text = input["lease"]["effective_expiry"]
                    .as_str()
                    .unwrap_or_default();
                offending.push(format!(
                    "scenarios/{stem}.yaml:{} — act {index} `{command}` ({outcome}) issues a \
                     lease of {lifetime} ms, and the ceiling is {ceiling} ms",
                    line_of(&text, &format!("effective_expiry: '{expiry_text}'")),
                ));
            }
        }
    }
    assert!(
        offending.is_empty(),
        "{} of the traces this story ships issue a data lease longer than the ceiling \
         {CONTRACT} §4.1 sets and {MODEL} repeats. ESS compiles them because it does not \
         execute a trace, and nothing else compares a trace to the timing obligations of \
         the vocabulary it reuses:\n{}",
        offending.len(),
        offending.join("\n")
    );
}

/// §4.1: a live data lease "is mandatory for every admitted data path", and "gate stops
/// by the effective expiry, with **no additional 2 s grace**". The deadline a supervisor
/// may act on is therefore the earlier of the expiry the trace writes down and the
/// ceiling after issuance — a longer expiry is not a longer licence. An admitted data
/// act after that deadline describes a session that, under the same contract, had
/// already entered `closing` with `lease_expired`.
#[test]
fn no_data_act_is_admitted_after_the_lease_that_admits_it_can_legally_be_live() {
    let ceiling = lease_ceiling_ms();
    let mut offending = Vec::new();
    for (stem, text, trace) in owned_traces() {
        let mut deadline: Option<i64> = None;
        for (index, at, command, outcome, input) in timeline(&stem, &trace) {
            let admitted = matches!(outcome.as_str(), "permitted" | "renewed");
            if let Some(live) = deadline
                && admitted
                && at > live
            {
                let at_text = trace["timeline"][index]["at"].as_str().unwrap_or_default();
                offending.push(format!(
                    "scenarios/{stem}.yaml:{} — act {index} `{command}` is admitted \
                     ({outcome}) {} ms after the last moment its lease can legally be live",
                    line_of(&text, &format!("at: '{at_text}'")),
                    (at - live) * 1_000,
                ));
            }
            if let Some((issued, expiry)) = lease(&input)
                && matches!(outcome.as_str(), "ready" | "renewed")
            {
                deadline = Some(std::cmp::min(expiry, issued + ceiling / 1_000));
            }
        }
    }
    assert!(
        offending.is_empty(),
        "a trace admits data after the live data lease that admits it could still be \
         current. {CONTRACT} §4.1 caps a lease at {ceiling} ms from issuance and forbids \
         any grace past the effective expiry, and the binding document's §3.3 rules out \
         the one transition that would extend it — \"The transition is `permit_data` \
         (`Ready` -> `Ready`), and it is not `renew`\". Shortening the trace hides this; \
         a trace that runs longer than a lease has to renew one:\n{}",
        offending.join("\n")
    );
}

/// `ess/domains/sessions.yaml`: the close deadlines "are bounded by §4.1; earlier
/// expiry/drain deadlines dominate", and §4.1 says a lease may never be extended
/// "beyond a drain or other earlier deadline". A close that records a cutoff later than
/// the live lease's own deadline records a deadline the model says cannot be the
/// operative one — the "additional 2 s grace" §4.1 names and forbids.
#[test]
fn no_close_records_a_cutoff_later_than_the_lease_it_ends() {
    let ceiling = lease_ceiling_ms();
    let mut offending = Vec::new();
    for (stem, text, trace) in owned_traces() {
        let mut deadline: Option<i64> = None;
        for (index, _, command, outcome, input) in timeline(&stem, &trace) {
            if let (Some(live), Some(cutoff)) = (
                deadline,
                input.get("cutoff_due_at").and_then(|v| v.as_str()),
            ) && outcome == "closing"
                && instant(cutoff) > live
            {
                offending.push(format!(
                    "scenarios/{stem}.yaml:{} — act {index} `{command}` records a cutoff \
                     deadline {} ms after the live lease's own deadline",
                    line_of(&text, &format!("cutoff_due_at: '{cutoff}'")),
                    (instant(cutoff) - live) * 1_000,
                ));
            }
            if let Some((issued, expiry)) = lease(&input)
                && matches!(outcome.as_str(), "ready" | "renewed")
            {
                deadline = Some(std::cmp::min(expiry, issued + ceiling / 1_000));
            }
        }
    }
    assert!(
        offending.is_empty(),
        "a close records a data-cutoff deadline later than the deadline of the lease it \
         ends, which {MODEL} says the earlier deadline dominates:\n{}",
        offending.join("\n")
    );
}

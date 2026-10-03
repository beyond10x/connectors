//! Authored auth obligations, not a credential store, OAuth client or runtime target.
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::PathBuf, process::Command};

const DIR: &str = "adapters/mcp/contracts/client/v1alpha1";
const PIN: &str = "adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912";
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Suite {
    format: String,
    cases: Vec<Case>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    revision: String,
    obligation: String,
    action: String,
    facts: Facts,
    expected: Value,
    citations: Vec<String>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Facts {
    credential: String,
    transport: String,
    binding_available: bool,
    admitted: bool,
    protected_channel: bool,
    custody_ack: bool,
    publication_ack: bool,
    correlation_live: bool,
    consume_ack: bool,
    issuer_matches: bool,
    audience_matches: bool,
    identity_matches: bool,
    grants_valid: bool,
    candidate_valid: bool,
    refresh_state: String,
    new_live_authorization: bool,
    current_fence: bool,
    provider_revoke_supported: bool,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}
fn inputs() -> (String, Suite) {
    (
        read(&format!("{DIR}/auth.md")),
        serde_json::from_str(&read(&format!("{DIR}/auth-cases.json"))).unwrap(),
    )
}
fn revisions() -> BTreeSet<String> {
    read("adapters/mcp/contracts/protocol/v1alpha1/selection.md")
        .lines()
        .filter_map(|line| {
            let c: Vec<_> = line.split('|').map(str::trim).collect();
            if c.len() < 6 || !matches!(c[2], "both" | "outbound") || c[4] != "supported" {
                return None;
            }
            c[1].trim_matches('`')
                .strip_prefix("revision:")
                .map(str::to_owned)
        })
        .collect()
}
fn model(path: &str) -> Value {
    serde_yaml_ng::from_str(&read(path)).unwrap()
}
fn type_variants(path: &str, name: &str) -> BTreeSet<String> {
    model(path)["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == name)
        .unwrap()["variants"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}
fn refresh_states() -> BTreeSet<String> {
    model("ess/domains/refresh.yaml")["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "connectors.refresh.RefreshAttempt")
        .unwrap()["lifecycle"]["states"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}
// This decision table interprets ONLY the listed document facts. No fact is obtained
// from a server, clock, registry or custody port, and no permission is issued.
fn observation(c: &Case) -> Value {
    let f = &c.facts;
    let mut out = json!({"outcome":"", "code":null, "reuse":false, "publish":false,
        "token_exchanges":0, "business_dispatches":0, "anonymous_fallback":false,
        "ordinary_fields":[], "provider_revocation":"not_attempted", "refresh_next":null});
    let (outcome, code) = if f.transport == "stdio" {
        ("stdio-held", Some("unsupported"))
    } else if !f.binding_available {
        ("binding-unselected", Some("unsupported"))
    } else if !f.admitted {
        ("management-refused", Some("not_granted"))
    } else if f.credential == "revoked" {
        ("revoked", Some("revoked"))
    } else {
        action(c, &mut out)
    };
    out["outcome"] = json!(outcome);
    out["code"] = json!(code);
    out
}
fn action<'a>(c: &Case, out: &mut Value) -> (&'a str, Option<&'a str>) {
    let f = &c.facts;
    match c.action.as_str() {
        "inspect" | "restart" => match f.credential.as_str() {
            "missing" => ("missing", Some("connection_not_ready")),
            "expired" => ("expired", Some("connection_not_ready")),
            "unavailable" => ("custody-unavailable", Some("unavailable")),
            _ if c.action == "restart"
                && (!f.custody_ack || !f.publication_ack || !f.current_fence) =>
            {
                ("persistence-unacknowledged", Some("connection_not_ready"))
            }
            _ => {
                out["reuse"] = json!(c.action == "restart");
                ("credential-ready", None)
            }
        },
        "begin" => {
            if !f.protected_channel {
                ("protected-channel-unavailable", Some("unavailable"))
            } else {
                out["ordinary_fields"] = json!(["acquisition_ref", "expires_at", "action_kind"]);
                ("protected-action", None)
            }
        }
        "complete" | "repair" => {
            if !f.correlation_live || !f.consume_ack {
                return ("completion-refused", Some("unauthorized"));
            }
            if !f.issuer_matches || !f.audience_matches {
                return ("authority-mismatch", Some("unauthorized"));
            }
            if !f.identity_matches {
                return ("identity-mismatch", Some("unauthorized"));
            }
            if !f.grants_valid {
                return ("insufficient-scope", Some("insufficient_scope"));
            }
            if !f.candidate_valid || !f.custody_ack || !f.publication_ack || !f.current_fence {
                return ("publication-refused", Some("connection_not_ready"));
            }
            out["publish"] = json!(true);
            out["ordinary_fields"] = json!(["acquisition_ref", "state", "connection_ref"]);
            (
                if c.action == "repair" {
                    "repair-published"
                } else {
                    "acquisition-published"
                },
                None,
            )
        }
        "refresh" => {
            if f.refresh_state == "Uncertain" || f.refresh_state == "Authorized" {
                out["refresh_next"] = json!("Uncertain");
                return ("refresh-uncertain", Some("outcome_unknown"));
            }
            if f.refresh_state != "Reserved" || !f.new_live_authorization || !f.current_fence {
                return ("refresh-refused", Some("connection_not_ready"));
            }
            if f.credential == "missing" || f.credential == "unavailable" {
                return ("refresh-material-unavailable", Some("unavailable"));
            }
            out["token_exchanges"] = json!(1);
            out["refresh_next"] = json!("Authorized");
            ("refresh-send-once", None)
        }
        "recover" => {
            if f.refresh_state != "ResponseStored"
                || !f.current_fence
                || !f.candidate_valid
                || !f.custody_ack
                || !f.publication_ack
                || !f.identity_matches
                || !f.grants_valid
                || !f.issuer_matches
                || !f.audience_matches
            {
                return ("recovery-refused", Some("connection_not_ready"));
            }
            out["publish"] = json!(true);
            out["refresh_next"] = json!("Published");
            ("publication-only-recovery", None)
        }
        "revoke" => {
            if !f.publication_ack {
                return ("revocation-unacknowledged", Some("outcome_unknown"));
            }
            out["provider_revocation"] = json!(if f.provider_revoke_supported {
                "separately_admitted_pending"
            } else {
                "unsupported"
            });
            ("locally-revoked", None)
        }
        _ => unreachable!("validated action"),
    }
}
const OBLIGATIONS: &[(&str, &str)] = &[
    ("missing", "inspect"),
    ("expired", "inspect"),
    ("revoked", "restart"),
    ("unavailable", "inspect"),
    ("restart", "restart"),
    ("unacknowledged", "restart"),
    ("protected-entry", "begin"),
    ("no-protected-channel", "begin"),
    ("completion", "complete"),
    ("issuer", "complete"),
    ("audience", "complete"),
    ("consumption", "complete"),
    ("refresh-once", "refresh"),
    ("refresh-uncertain", "refresh"),
    ("refresh-authorized", "refresh"),
    ("refresh-recovery", "recover"),
    ("refresh-wrong-state", "recover"),
    ("repair", "repair"),
    ("repair-identity", "repair"),
    ("revoke", "revoke"),
    ("revoke-unknown", "revoke"),
    ("stdio", "begin"),
    ("unselected-binding", "restart"),
    ("management-admission", "begin"),
];
fn validate(doc: &str, suite: &Suite) -> Result<(), String> {
    if suite.format != "mcp-auth-document-cases/1" {
        return Err("format".into());
    }
    let versions = revisions();
    let codes = type_variants(
        "ess/domains/service_wire.yaml",
        "connectors.service_wire.ErrorCode",
    );
    let states = refresh_states();
    let manifest: Value =
        serde_json::from_str(&read(&format!("{PIN}/specification-source-hashes.json"))).unwrap();
    let mut ids = BTreeSet::new();
    let mut covered = BTreeSet::new();
    for c in &suite.cases {
        if !ids.insert(c.id.clone()) {
            return Err("duplicate case".into());
        }
        if !versions.contains(&c.revision) {
            return Err("unselected revision".into());
        }
        if !matches!(
            c.facts.credential.as_str(),
            "ready" | "missing" | "expired" | "revoked" | "unavailable"
        ) || !matches!(c.facts.transport.as_str(), "streamable-http" | "stdio")
            || !states.contains(&c.facts.refresh_state)
        {
            return Err("fact vocabulary".into());
        }
        if !OBLIGATIONS
            .iter()
            .any(|(o, a)| *o == c.obligation && *a == c.action)
        {
            return Err("obligation/action".into());
        }
        let f = &c.facts;
        let required_fact = match c.obligation.as_str() {
            "missing" => f.credential == "missing",
            "expired" => f.credential == "expired",
            "revoked" => f.credential == "revoked",
            "unavailable" => f.credential == "unavailable",
            "restart" => c.expected["reuse"] == true,
            "unacknowledged" => !f.custody_ack || !f.publication_ack,
            "protected-entry" => f.protected_channel,
            "no-protected-channel" => !f.protected_channel,
            "completion" | "repair" => c.expected["publish"] == true,
            "issuer" => !f.issuer_matches,
            "audience" => !f.audience_matches,
            "consumption" => !f.consume_ack,
            "refresh-once" => c.expected["token_exchanges"] == 1,
            "refresh-uncertain" | "refresh-wrong-state" => f.refresh_state == "Uncertain",
            "refresh-authorized" => f.refresh_state == "Authorized" && !f.new_live_authorization,
            "refresh-recovery" => {
                f.refresh_state == "ResponseStored" && c.expected["publish"] == true
            }
            "repair-identity" => !f.identity_matches,
            "revoke" => f.publication_ack && !f.provider_revoke_supported,
            "revoke-unknown" => !f.publication_ack,
            "stdio" => f.transport == "stdio",
            "unselected-binding" => !f.binding_available,
            "management-admission" => !f.admitted,
            _ => false,
        };
        if !required_fact {
            return Err(format!("obligation facts {}", c.id));
        }
        if !covered.insert((c.revision.clone(), c.obligation.clone())) {
            return Err("duplicate obligation".into());
        }
        if c.expected["code"]
            .as_str()
            .is_some_and(|s| !codes.contains(s))
        {
            return Err("error vocabulary".into());
        }
        if c.expected != observation(c) {
            return Err(format!("contradictory {}", c.id));
        }
        if c.citations.is_empty() {
            return Err("uncited".into());
        }
        for citation in &c.citations {
            let (file, number) = citation.rsplit_once(':').ok_or("citation shape")?;
            let e = manifest
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["file"] == file && e["revision"] == c.revision)
                .ok_or("citation file/revision")?;
            let output = Command::new("gzip")
                .arg("-cd")
                .arg(root().join(PIN).join(e["archive"].as_str().unwrap()))
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success()
                || number
                    .parse::<usize>()
                    .ok()
                    .filter(|n| {
                        *n > 0 && *n <= String::from_utf8_lossy(&output.stdout).lines().count()
                    })
                    .is_none()
            {
                return Err("citation line".into());
            }
        }
    }
    check_document_inventory(doc, suite)?;
    for version in versions {
        for (obligation, _) in OBLIGATIONS {
            if !covered.contains(&(version.clone(), (*obligation).to_owned())) {
                return Err(format!("missing {version}/{obligation}"));
            }
        }
    }
    // Four distinct outcomes are required even where shared error codes coincide.
    for version in revisions() {
        let failures: BTreeSet<_> = suite
            .cases
            .iter()
            .filter(|c| {
                c.revision == version
                    && matches!(
                        c.obligation.as_str(),
                        "missing" | "expired" | "revoked" | "unavailable"
                    )
            })
            .map(|c| c.expected["outcome"].as_str().unwrap())
            .collect();
        if failures.len() != 4 {
            return Err("collapsed credential failures".into());
        }
    }
    Ok(())
}

/// Compare complete rendered table rows, retaining multiplicity. A code example
/// cannot stand in for the required table, nor can a quoted copy claim authority.
/// Other prose tables are outside this case inventory, but cannot hide case rows.
fn check_document_inventory(doc: &str, suite: &Suite) -> Result<(), String> {
    let expected: BTreeSet<Vec<String>> = suite
        .cases
        .iter()
        .map(|c| {
            vec![
                c.id.clone(),
                c.revision.clone(),
                c.obligation.clone(),
                c.expected["outcome"].as_str().unwrap().to_owned(),
            ]
        })
        .collect();
    let mut table_count = 0;
    let mut quote_depth = 0usize;
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut cells: Vec<String> = Vec::new();
    let mut in_cell = false;
    let mut paragraph: Option<String> = None;
    for event in Parser::new_ext(doc, Options::ENABLE_TABLES) {
        match event {
            Event::Start(Tag::BlockQuote(_)) => quote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => quote_depth -= 1,
            Event::Start(Tag::Paragraph) => paragraph = Some(String::new()),
            Event::End(TagEnd::Paragraph) => {
                if let Some(text) = paragraph.take() {
                    // A blank line ends a Markdown table. A subsequent visible
                    // case-shaped paragraph is a malformed inventory claim,
                    // not a second row to silently discard. Fenced examples do
                    // not produce paragraphs and quoted text is not authority.
                    if quote_depth == 0
                        && text.lines().any(|line| {
                            line.contains('|')
                                && line
                                    .trim()
                                    .trim_start_matches('|')
                                    .trim_start()
                                    .starts_with("auth.")
                        })
                    {
                        return Err("document/case bijection: case row outside table".into());
                    }
                }
            }
            Event::Start(Tag::Table(_)) => rows.clear(),
            Event::Start(Tag::TableHead | Tag::TableRow) => cells.clear(),
            Event::Start(Tag::TableCell) => {
                cells.push(String::new());
                in_cell = true;
            }
            Event::End(TagEnd::TableCell) => in_cell = false,
            Event::Text(text) | Event::Code(text) if in_cell => cells
                .last_mut()
                .ok_or("document/case bijection: cell outside row")?
                .push_str(&text),
            Event::SoftBreak | Event::HardBreak if in_cell => cells
                .last_mut()
                .ok_or("document/case bijection: break outside row")?
                .push('\n'),
            Event::InlineHtml(_) | Event::Html(_) => {
                return Err("document/case bijection: unsupported raw HTML".into());
            }
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => rows.push(cells.clone()),
            Event::End(TagEnd::Table) => {
                let expected_header = rows
                    .first()
                    .is_some_and(|header| header == &["Case", "Revision", "Obligation", "Outcome"]);
                let contains_case = rows
                    .iter()
                    .skip(1)
                    .any(|row| row.first().is_some_and(|id| id.starts_with("auth.")));
                if !expected_header && !contains_case {
                    continue;
                }
                if !expected_header || quote_depth > 0 {
                    return Err("document/case bijection: misplaced case table".into());
                }
                table_count += 1;
                if table_count != 1 {
                    return Err("document/case bijection: duplicate table".into());
                }
                let mut ids = BTreeSet::new();
                let mut actual = BTreeSet::new();
                for row in rows.iter().skip(1) {
                    if row.len() != 4 || !ids.insert(row[0].clone()) {
                        return Err("document/case bijection: duplicate or malformed row".into());
                    }
                    actual.insert(row.clone());
                }
                if actual != expected {
                    return Err(
                        "document/case bijection: missing, unknown or contradictory row".into(),
                    );
                }
            }
            Event::Text(text) | Event::Code(text) if paragraph.is_some() => {
                paragraph.as_mut().unwrap().push_str(&text)
            }
            Event::SoftBreak | Event::HardBreak if paragraph.is_some() => {
                paragraph.as_mut().unwrap().push('\n')
            }
            _ => {}
        }
    }
    if table_count != 1 {
        return Err("document/case bijection: missing table".into());
    }
    Ok(())
}
#[test]
fn authored_auth_contract_has_complete_unique_source_bound_cases() {
    let (doc, suite) = inputs();
    assert_eq!(validate(&doc, &suite), Ok(()));
}
#[test]
fn missing_duplicate_and_orphan_rows_fail() {
    let (doc, suite) = inputs();
    let mut bad = suite.clone();
    let removed = bad.cases.pop().unwrap();
    let missing_doc = doc
        .lines()
        .filter(|line| !line.starts_with(&format!("| `{}` |", removed.id)))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        validate(&missing_doc, &bad)
            .unwrap_err()
            .contains("missing")
    );
    let mut bad = suite.clone();
    bad.cases.push(bad.cases[0].clone());
    assert!(validate(&doc, &bad).unwrap_err().contains("duplicate"));
    assert!(
        validate(&format!("{doc}\n| `auth.orphan` |"), &suite)
            .unwrap_err()
            .contains("bijection")
    );
}
#[test]
fn anonymous_retry_and_private_disclosure_claims_fail() {
    let (doc, suite) = inputs();
    for (key, value) in [
        ("anonymous_fallback", json!(true)),
        ("business_dispatches", json!(1)),
        ("ordinary_fields", json!(["refresh_token"])),
        ("ordinary_fields", json!(["completion_url"])),
        ("ordinary_fields", json!(["custody_version_ref"])),
    ] {
        let mut bad = suite.clone();
        bad.cases[0].expected[key] = value;
        assert!(validate(&doc, &bad).unwrap_err().contains("contradictory"));
    }
}
#[test]
fn uncertain_refresh_and_unacknowledged_persistence_cannot_claim_success() {
    let (doc, suite) = inputs();
    for (obligation, key, value) in [
        ("refresh-uncertain", "token_exchanges", json!(1)),
        ("refresh-authorized", "token_exchanges", json!(1)),
        ("refresh-recovery", "token_exchanges", json!(1)),
        ("unacknowledged", "reuse", json!(true)),
        ("repair-identity", "publish", json!(true)),
        ("revoke-unknown", "provider_revocation", json!("succeeded")),
    ] {
        let mut bad = suite.clone();
        bad.cases
            .iter_mut()
            .find(|c| c.obligation == obligation)
            .unwrap()
            .expected[key] = value;
        assert!(validate(&doc, &bad).unwrap_err().contains("contradictory"));
    }
}
#[test]
fn real_fact_changes_invalidate_old_outcomes() {
    let (doc, suite) = inputs();
    for obligation in [
        "completion",
        "repair",
        "refresh-recovery",
        "refresh-once",
        "restart",
    ] {
        let mut bad = suite.clone();
        bad.cases
            .iter_mut()
            .find(|c| c.obligation == obligation)
            .unwrap()
            .facts
            .current_fence = false;
        assert!(
            validate(&doc, &bad).unwrap_err().contains("contradictory"),
            "stale fence accepted for {obligation}"
        );
    }
    for field in [
        "custody_ack",
        "publication_ack",
        "candidate_valid",
        "identity_matches",
        "grants_valid",
        "audience_matches",
    ] {
        let mut bad = suite.clone();
        let facts = &mut bad
            .cases
            .iter_mut()
            .find(|c| c.obligation == "completion")
            .unwrap()
            .facts;
        match field {
            "custody_ack" => facts.custody_ack = false,
            "publication_ack" => facts.publication_ack = false,
            "candidate_valid" => facts.candidate_valid = false,
            "identity_matches" => facts.identity_matches = false,
            "grants_valid" => facts.grants_valid = false,
            "audience_matches" => facts.audience_matches = false,
            _ => unreachable!(),
        }
        assert!(
            validate(&doc, &bad).unwrap_err().contains("contradictory"),
            "invalid {field} accepted"
        );
    }
    let mut bad = suite.clone();
    bad.cases
        .iter_mut()
        .find(|c| c.obligation == "completion")
        .unwrap()
        .facts
        .issuer_matches = false;
    assert!(validate(&doc, &bad).unwrap_err().contains("contradictory"));
    let mut bad = suite.clone();
    bad.cases
        .iter_mut()
        .find(|c| c.obligation == "refresh-recovery")
        .unwrap()
        .facts
        .refresh_state = "Uncertain".into();
    assert!(
        validate(&doc, &bad)
            .unwrap_err()
            .contains("obligation facts")
    );
    let mut bad = suite.clone();
    bad.cases[0].expected["code"] = json!("missing_credential");
    assert!(validate(&doc, &bad).unwrap_err().contains("vocabulary"));
}

#[test]
fn review_contradictory_duplicate_document_rows_are_rejected() {
    let (doc, suite) = inputs();
    let mut accepted = Vec::new();
    for case in &suite.cases {
        let contradictory = format!(
            "{doc}\n| `{}` | {} | {} | `anonymous-dispatch-permitted` |\n",
            case.id, case.revision, case.obligation
        );
        if validate(&contradictory, &suite).is_ok() {
            accepted.push(case.id.clone());
        }
    }
    assert!(
        accepted.is_empty(),
        "contradictory duplicate document rows accepted for: {accepted:?}"
    );
}

#[test]
fn correspondence_requires_one_visible_structural_table() {
    let (doc, suite) = inputs();
    for case in &suite.cases {
        for outcome in [case.expected["outcome"].as_str().unwrap(), "contradictory"] {
            let extra_row = format!(
                "{}\n| `{}` | {} | {} | `{outcome}` |\n",
                doc.trim_end(),
                case.id,
                case.revision,
                case.obligation
            );
            assert!(
                validate(&extra_row, &suite).is_err(),
                "duplicate rendered row accepted: {}",
                case.id
            );
        }
    }
    let header = "| Case | Revision | Obligation | Outcome |\n";
    let (_, tail) = doc.split_once(header).unwrap();
    let table = format!("{header}{tail}");
    let quoted = table
        .lines()
        .map(|line| format!("> {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    for replacement in [
        format!("```text\n{table}\n```"),
        quoted.clone(),
        table.replace("| Case |", "| Other |"),
    ] {
        assert!(
            validate(&doc.replace(&table, &replacement), &suite).is_err(),
            "non-authoritative table counted"
        );
    }
    for extra in [table.clone(), quoted] {
        assert!(
            validate(&format!("{doc}\n\n{extra}\n"), &suite).is_err(),
            "second correspondence table accepted"
        );
    }
    // Non-rendered examples do not count as rows or duplicate authoritative tables.
    assert_eq!(
        validate(&format!("{doc}\n\n```text\n{table}\n```\n"), &suite),
        Ok(())
    );
    let rows: Vec<_> = table.lines().collect();
    let reordered = rows[..2]
        .iter()
        .copied()
        .chain(rows[2..].iter().rev().copied())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(validate(&doc.replace(&table, &reordered), &suite), Ok(()));
}

#[test]
fn review_visible_html_case_inventory_cannot_bypass_correspondence() {
    let (doc, suite) = inputs();
    let mut accepted = Vec::new();
    for case in suite
        .cases
        .iter()
        .filter(|case| case.obligation == "missing")
    {
        let contradictory = format!(
            "{doc}\n\n<table>\n<tr><th>Case</th><th>Revision</th><th>Obligation</th><th>Outcome</th></tr>\n<tr><td>{}</td><td>{}</td><td>{}</td><td>anonymous-dispatch-permitted</td></tr>\n</table>\n",
            case.id, case.revision, case.obligation
        );
        if validate(&contradictory, &suite).is_ok() {
            accepted.push(case.id.clone());
        }
    }
    assert!(
        accepted.is_empty(),
        "contradictory visible HTML case inventory accepted for: {accepted:?}"
    );
}

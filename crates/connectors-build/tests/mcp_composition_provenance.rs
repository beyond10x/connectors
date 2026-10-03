//! Document obligations only; this executes no MCP adapter or runtime conformance.
use std::path::PathBuf;
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
#[test]
fn composition_contract_exists_with_authored_cases() {
    let dir = root().join("adapters/mcp/contracts/composition/v1alpha1");
    let doc = std::fs::read_to_string(dir.join("semantics.md"))
        .expect("composition contract must exist before the story can close");
    let cases: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("composition-cases.json")).unwrap())
            .unwrap();
    assert_eq!(cases["format"], "mcp-composition-document-cases/1");
    assert!(doc.contains("| Case | Inbound | Outbound | Family | Obligation | Outcome |"));
}
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
const DIR: &str = "adapters/mcp/contracts/composition/v1alpha1";
const OBLIGATIONS: &[&str] = &[
    "paired",
    "caller-retarget",
    "credential-forward",
    "account-fallback",
    "metadata-authority",
    "inbound-admission",
    "outbound-admission",
    "inbound-stale",
    "outbound-unselected",
    "delivery-admission",
    "mapping-unavailable",
    "auth-unavailable",
    "mutation-unbound",
    "resource-write",
    "prompt-untyped",
    "audit-peer",
    "audit-unack",
    "audit-trusted",
    "request-overflow",
    "result-overflow",
    "exact-bound",
    "multibyte",
    "remaining-deadline",
    "deadline-exhausted",
    "partial",
    "lost-reply",
    "host-known",
    "modern-array",
    "modern-null",
];
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
    inbound: String,
    outbound: String,
    family: String,
    obligation: String,
    facts: Facts,
    expected: Value,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Facts {
    fixed_target: bool,
    credential_forward_requested: bool,
    fallback_requested: bool,
    metadata_claims_authority: bool,
    inbound_admitted: bool,
    outbound_admitted: bool,
    inbound_revision_current: bool,
    outbound_revision_selected: bool,
    delivery_admitted: bool,
    mapping_complete: bool,
    auth_ready: bool,
    mutation: bool,
    mutation_bound: bool,
    read_only: bool,
    prompt_schema: bool,
    source_trusted: bool,
    source_acknowledged: bool,
    source_instance: String,
    source_ref: String,
    request: String,
    request_limit: usize,
    outbound_response: String,
    outbound_limit: usize,
    inbound_limit: usize,
    payload: Value,
    budget_ms: u64,
    auth_ms: u64,
    processing_ms: u64,
    terminal: bool,
    caller_received: bool,
    host_knows_applied: bool,
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}
fn inputs() -> (String, Suite) {
    (
        read(&format!("{DIR}/semantics.md")),
        serde_json::from_str(&read(&format!("{DIR}/composition-cases.json"))).unwrap(),
    )
}
fn revisions(direction: &str) -> BTreeSet<String> {
    read("adapters/mcp/contracts/protocol/v1alpha1/selection.md")
        .lines()
        .filter_map(|line| {
            let cols: Vec<_> = line.split('|').map(str::trim).collect();
            (cols.len() >= 6
                && (cols[2] == "both" || cols[2] == direction)
                && cols[4] == "supported")
                .then(|| {
                    cols[1]
                        .trim_matches('`')
                        .strip_prefix("revision:")
                        .map(str::to_owned)
                })
                .flatten()
        })
        .collect()
}
fn source(c: &Case) -> Value {
    if c.facts.source_trusted && c.facts.source_acknowledged {
        json!({"instance":c.facts.source_instance,"audit_ref":c.facts.source_ref,"audit_status":"complete"})
    } else {
        Value::Null
    }
}
// Synthetic already-safe service observation. This is NOT a new native MCP result
// mapping: that typed mapping remains an advertisement prerequisite.
fn inbound_wire(c: &Case) -> String {
    let mut response = json!({"version":"v1alpha2","request_id":"local-request","status":"success",
        "result":c.facts.payload,"audit_ref":"local-audit","audit_status":"complete"});
    if !source(c).is_null() {
        response["source_audit"] = source(c);
    }
    let mut result = match c.family.as_str() {
        "tools" => {
            json!({"content":[{"type":"text","text":response.to_string()}],"structuredContent":response,"isError":false})
        }
        "resources" => {
            json!({"contents":[{"uri":"connectors-mcp:///c1_61_62_63?revision=72","mimeType":"application/json","text":response.to_string()}]})
        }
        "prompts" => {
            json!({"messages":[{"role":"user","content":{"type":"text","text":"declared message"}}],"_meta":{"io.beyond10x.connectors/response":response}})
        }
        _ => unreachable!(),
    };
    if c.inbound == "2026-07-28" {
        result["resultType"] = json!("complete");
        if c.family == "resources" {
            result["ttlMs"] = json!(0);
            result["cacheScope"] = json!("private");
        }
    }
    json!({"jsonrpc":"2.0","id":1,"result":result}).to_string()
}
fn decision(c: &Case) -> (&'static str, Option<&'static str>) {
    let f = &c.facts;
    if !f.inbound_admitted {
        ("inbound-denied", Some("not_granted"))
    } else if !f.fixed_target {
        ("retarget-refused", Some("forbidden"))
    } else if !f.inbound_revision_current {
        ("stale-inbound", Some("stale_description"))
    } else if !f.mapping_complete {
        ("unbound", Some("not_found"))
    } else if f.mutation && !f.mutation_bound {
        ("mutation-omitted", Some("not_found"))
    } else if c.family == "resources" && !f.read_only {
        ("resource-ineligible", Some("unsupported"))
    } else if c.family == "prompts" && (!f.read_only || !f.prompt_schema) {
        ("prompt-ineligible", Some("unsupported"))
    } else if f.credential_forward_requested {
        ("credential-forward-refused", Some("forbidden"))
    } else if !f.outbound_admitted {
        ("outbound-denied", Some("not_granted"))
    } else if !f.outbound_revision_selected {
        ("unselected-outbound", Some("unsupported"))
    } else if !f.auth_ready {
        ("auth-unavailable", Some("connection_not_ready"))
    } else if f.fallback_requested {
        ("fallback-refused", Some("forbidden"))
    } else if f.request.len() > f.request_limit {
        ("request-bound", Some("invalid_input"))
    } else if f.auth_ms.saturating_add(f.processing_ms) >= f.budget_ms {
        ("deadline-exhausted", Some("timeout"))
    } else if !f.delivery_admitted {
        ("delivery-denied", Some("forbidden"))
    } else if !f.caller_received {
        ("caller-unobserved", Some("outcome_unknown"))
    } else if !f.terminal {
        ("incomplete", Some("outcome_unknown"))
    } else if f.outbound_response.len() > f.outbound_limit
        || inbound_wire(c).len() > f.inbound_limit
    {
        ("result-bound", Some("capacity"))
    } else {
        ("preserved", None)
    }
}
fn observation(c: &Case) -> Value {
    let f = &c.facts;
    let (outcome, code) = decision(c);
    let reached_observation = matches!(
        outcome,
        "delivery-denied" | "caller-unobserved" | "incomplete" | "result-bound" | "preserved"
    );
    let host = if !reached_observation {
        "not-observed"
    } else if f.host_knows_applied {
        "applied"
    } else if f.terminal {
        "peer-report"
    } else {
        "unknown"
    };
    json!({"outcome":outcome,"code":code,
        "source_audit":if outcome == "preserved" { source(c) } else { Value::Null },
        "remaining_ms":f.budget_ms.saturating_sub(f.auth_ms.saturating_add(f.processing_ms)),
        "inbound_bytes":inbound_wire(c).len(),"outbound_bytes":f.outbound_response.len(),
        "automatic_redispatches":0,"forwarded_credentials":0,"account_substitutions":0,
        "metadata_grants_authority":false,
        "host_observation":host,
        "caller_observation":if !reached_observation {"not-observed"} else if !f.delivery_admitted {"withheld"} else if !f.caller_received || !f.terminal {"unknown"} else {host}})
}
fn required_semantics(c: &Case) -> bool {
    let f = &c.facts;
    let (outcome, _) = decision(c);
    // A row named for a late branch must actually reach it. Relabeling it with
    // an early policy failure cannot satisfy the coverage inventory.
    match c.obligation.as_str() {
        "paired" => outcome == "preserved" && f.read_only && f.mapping_complete,
        "caller-retarget" => outcome == "retarget-refused",
        "credential-forward" => outcome == "credential-forward-refused",
        "account-fallback" => outcome == "auth-unavailable" && f.fallback_requested,
        "metadata-authority" => outcome == "outbound-denied" && f.metadata_claims_authority,
        "inbound-admission" => outcome == "inbound-denied" && !f.inbound_revision_current,
        "outbound-admission" => outcome == "outbound-denied" && f.inbound_admitted,
        "inbound-stale" => outcome == "stale-inbound",
        "outbound-unselected" => outcome == "unselected-outbound",
        "delivery-admission" => outcome == "delivery-denied",
        "mapping-unavailable" => outcome == "unbound",
        "auth-unavailable" => outcome == "auth-unavailable" && !f.fallback_requested,
        "mutation-unbound" => outcome == "mutation-omitted" && c.family == "tools",
        "resource-write" => outcome == "resource-ineligible" && c.family == "resources",
        "prompt-untyped" => outcome == "prompt-ineligible" && f.read_only && c.family == "prompts",
        "audit-peer" => outcome == "preserved" && !f.source_trusted && f.source_acknowledged,
        "audit-unack" => outcome == "preserved" && f.source_trusted && !f.source_acknowledged,
        "audit-trusted" => outcome == "preserved" && f.source_trusted && f.source_acknowledged,
        "request-overflow" => outcome == "request-bound" && f.request.len() == f.request_limit + 1,
        "result-overflow" => {
            outcome == "result-bound"
                && f.outbound_response.len() <= f.outbound_limit
                && inbound_wire(c).len() == f.inbound_limit + 1
        }
        "exact-bound" => {
            outcome == "preserved"
                && inbound_wire(c).len() == f.inbound_limit
                && f.outbound_response.len() == f.outbound_limit
                && f.request.len() == f.request_limit
        }
        "multibyte" => {
            outcome == "result-bound"
                && inbound_wire(c).chars().count() <= f.inbound_limit
                && inbound_wire(c).len() > f.inbound_limit
                && f.outbound_response.len() <= f.outbound_limit
        }
        "remaining-deadline" => {
            outcome == "preserved"
                && f.auth_ms > 0
                && f.processing_ms > 0
                && f.budget_ms - f.auth_ms - f.processing_ms == 1
        }
        "deadline-exhausted" => {
            outcome == "deadline-exhausted" && f.auth_ms + f.processing_ms == f.budget_ms
        }
        "partial" => {
            outcome == "incomplete" && !f.terminal && f.caller_received && !f.host_knows_applied
        }
        "lost-reply" => outcome == "caller-unobserved" && f.host_knows_applied && f.terminal,
        "host-known" => {
            outcome == "result-bound" && f.host_knows_applied && f.caller_received && f.terminal
        }
        "modern-array" | "modern-null" => {
            outcome == "preserved"
                && c.inbound == "2025-11-25"
                && c.outbound == "2026-07-28"
                && c.family == "tools"
                && if c.obligation == "modern-array" {
                    f.payload.is_array()
                } else {
                    f.payload.is_null()
                }
        }
        _ => false,
    }
}
fn validate(doc: &str, suite: &Suite) -> Result<(), String> {
    if suite.format != "mcp-composition-document-cases/1" {
        return Err("format".into());
    }
    let versions = revisions("inbound");
    if versions != revisions("outbound") {
        return Err("independent selected revision inventories differ".into());
    }
    if versions.len() != 2 {
        return Err("selected revision inventory changed".into());
    }
    let mut ids = BTreeSet::new();
    let mut obligations = BTreeSet::new();
    let mut pairs = BTreeSet::new();
    for c in &suite.cases {
        if !ids.insert(c.id.clone()) {
            return Err("duplicate case".into());
        }
        if !versions.contains(&c.inbound)
            || !versions.contains(&c.outbound)
            || !["tools", "resources", "prompts"].contains(&c.family.as_str())
        {
            return Err("unselected vocabulary".into());
        }
        if c.facts.source_instance.is_empty() || c.facts.source_ref.is_empty() {
            return Err("invalid source fixture".into());
        }
        check_wire_fixture(c)?;
        if !required_semantics(c) {
            return Err(format!("obligation premise: {}", c.id));
        }
        if c.expected != observation(c) {
            return Err(format!("contradictory observation: {}", c.id));
        }
        if c.obligation == "paired" {
            if !pairs.insert((c.inbound.clone(), c.outbound.clone(), c.family.clone())) {
                return Err("duplicate pair".into());
            }
        } else if !obligations.insert(c.obligation.as_str()) {
            return Err("duplicate obligation".into());
        }
    }
    for obligation in OBLIGATIONS.iter().filter(|o| **o != "paired") {
        if !obligations.contains(obligation) {
            return Err(format!("missing obligation: {obligation}"));
        }
    }
    for inbound in &versions {
        for outbound in &versions {
            for family in ["tools", "resources", "prompts"] {
                if !pairs.contains(&(inbound.clone(), outbound.clone(), family.into())) {
                    return Err("missing pair".into());
                }
            }
        }
    }
    check_document_inventory(doc, suite)
}

// Fixture hygiene only, not an implementation of either full revision's schema.
fn check_wire_fixture(c: &Case) -> Result<(), String> {
    let request: Value = serde_json::from_str(&c.facts.request).map_err(|_| "request fixture")?;
    let response: Value =
        serde_json::from_str(&c.facts.outbound_response).map_err(|_| "response fixture")?;
    let method = match c.family.as_str() {
        "tools" => "tools/call",
        "resources" => "resources/read",
        "prompts" => "prompts/get",
        _ => return Err("family fixture".into()),
    };
    let request_id = request.get("id");
    let response_id = response.get("id");
    if !request_id.is_some_and(|id| id.is_string() || id.is_i64() || id.is_u64())
        || response_id != request_id
        || !request.is_object()
        || !response.is_object()
        || request.get("result").is_some()
        || request.get("error").is_some()
        || response.get("result").is_none()
        || response.get("error").is_some()
        || response.get("method").is_some()
        || request["method"] != method
        || request["jsonrpc"] != "2.0"
        || response["jsonrpc"] != "2.0"
    {
        return Err("correlation/family fixture".into());
    }
    let result = &response["result"];
    if (c.outbound == "2026-07-28" && result["resultType"] != "complete")
        || (c.outbound == "2025-11-25" && result.get("resultType").is_some())
    {
        return Err("revision fixture".into());
    }
    match c.family.as_str() {
        "tools"
            if result["structuredContent"] != c.facts.payload
                || result["content"].as_array().is_none_or(Vec::is_empty)
                || (c.outbound == "2025-11-25" && !result["structuredContent"].is_object()) =>
        {
            return Err("tool fixture".into());
        }
        "resources"
            if !result["contents"].is_array()
                || (c.outbound == "2026-07-28"
                    && (result["ttlMs"] != 0 || result["cacheScope"] != "private")) =>
        {
            return Err("resource fixture".into());
        }
        "prompts" if !result["messages"].is_array() => return Err("prompt fixture".into()),
        _ => {}
    }
    Ok(())
}
fn check_document_inventory(doc: &str, suite: &Suite) -> Result<(), String> {
    let expected: BTreeSet<Vec<String>> = suite
        .cases
        .iter()
        .map(|c| {
            vec![
                c.id.clone(),
                c.inbound.clone(),
                c.outbound.clone(),
                c.family.clone(),
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
                                    .starts_with("compose.")
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
                let expected_header = rows.first().is_some_and(|header| {
                    header
                        == &[
                            "Case",
                            "Inbound",
                            "Outbound",
                            "Family",
                            "Obligation",
                            "Outcome",
                        ]
                });
                let contains_case = rows
                    .iter()
                    .skip(1)
                    .any(|row| row.first().is_some_and(|id| id.starts_with("compose.")));
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
                    if row.len() != 6 || !ids.insert(row[0].clone()) {
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
fn composition_has_all_independent_pairs_and_deciding_premises() {
    let (doc, suite) = inputs();
    assert_eq!(validate(&doc, &suite), Ok(()));
}
fn table(suite: &Suite) -> String {
    let mut doc = String::from(
        "| Case | Inbound | Outbound | Family | Obligation | Outcome |\n|---|---|---|---|---|---|\n",
    );
    for c in &suite.cases {
        doc.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | `{}` |\n",
            c.id,
            c.inbound,
            c.outbound,
            c.family,
            c.obligation,
            c.expected["outcome"].as_str().unwrap()
        ));
    }
    doc
}
#[test]
fn removing_any_required_case_even_with_its_row_fails() {
    let (_, suite) = inputs();
    for index in 0..suite.cases.len() {
        let mut bad = suite.clone();
        bad.cases.remove(index);
        assert!(
            validate(&table(&bad), &bad).is_err(),
            "case {index} disappeared"
        );
    }
}
#[test]
fn late_semantics_cannot_be_replaced_by_consistent_early_refusals() {
    let (_, suite) = inputs();
    for c in &suite.cases {
        if c.obligation == "inbound-admission" {
            continue;
        }
        let mut bad = suite.clone();
        let changed = bad.cases.iter_mut().find(|v| v.id == c.id).unwrap();
        changed.facts.inbound_admitted = false;
        changed.expected = observation(changed);
        assert!(
            validate(&table(&bad), &bad)
                .unwrap_err()
                .contains("premise"),
            "{}",
            c.id
        );
    }
}
#[test]
fn false_authority_secret_forwarding_retry_and_knowledge_claims_fail() {
    let (doc, suite) = inputs();
    for (key, value) in [
        ("forwarded_credentials", json!(1)),
        ("account_substitutions", json!(1)),
        ("automatic_redispatches", json!(1)),
        ("metadata_grants_authority", json!(true)),
        (
            "source_audit",
            json!({"instance":"peer-self-claim","audit_ref":"unacknowledged","audit_status":"complete"}),
        ),
        ("remaining_ms", json!(1000)),
    ] {
        let mut bad = suite.clone();
        bad.cases[0].expected[key] = value;
        assert!(validate(&doc, &bad).unwrap_err().contains("contradictory"));
    }
    for (obligation, key, value) in [
        ("lost-reply", "caller_observation", json!("applied")),
        ("host-known", "host_observation", json!("unknown")),
        ("partial", "caller_observation", json!("peer-report")),
    ] {
        let mut bad = suite.clone();
        bad.cases
            .iter_mut()
            .find(|c| c.obligation == obligation)
            .unwrap()
            .expected[key] = value;
        assert!(validate(&table(&bad), &bad).is_err());
    }
}
#[test]
fn wrapped_bytes_include_escaping_duplicates_and_legacy_object_carrier() {
    let (_, suite) = inputs();
    for c in suite.cases.iter().filter(|c| c.family == "tools") {
        let wire = inbound_wire(c);
        let value: Value = serde_json::from_str(&wire).unwrap();
        let wrapped = &value["result"]["structuredContent"];
        assert!(wrapped.is_object());
        assert_eq!(wrapped["result"], c.facts.payload);
        let text: Value =
            serde_json::from_str(value["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(&text, wrapped);
        assert!(wire.len() > wrapped.to_string().len());
        assert!(wire.len() > c.facts.outbound_response.len());
    }
    for obligation in [
        "request-overflow",
        "result-overflow",
        "exact-bound",
        "multibyte",
    ] {
        let mut bad = suite.clone();
        let c = bad
            .cases
            .iter_mut()
            .find(|c| c.obligation == obligation)
            .unwrap();
        c.facts.inbound_limit = usize::MAX;
        c.facts.request_limit = usize::MAX;
        c.expected = observation(c);
        assert!(
            validate(&table(&bad), &bad)
                .unwrap_err()
                .contains("premise")
        );
    }
}
#[test]
fn parsed_inventory_refuses_duplicates_contradictions_html_and_hidden_tables() {
    let (doc, suite) = inputs();
    let inventory = table(&suite);
    assert!(doc.contains(&inventory));
    for c in &suite.cases {
        for outcome in [
            c.expected["outcome"].as_str().unwrap(),
            "credential-forward-allowed",
        ] {
            let extra = format!(
                "{doc}\n| `{}` | {} | {} | {} | {} | `{outcome}` |\n",
                c.id, c.inbound, c.outbound, c.family, c.obligation
            );
            assert!(validate(&extra, &suite).is_err());
        }
    }
    let quoted = inventory
        .lines()
        .map(|l| format!("> {l}"))
        .collect::<Vec<_>>()
        .join("\n");
    for replacement in [
        format!("```text\n{inventory}\n```"),
        quoted.clone(),
        inventory.replace("| Case |", "| Other |"),
        "<table><tr><td>compose.false</td></tr></table>".into(),
    ] {
        assert!(validate(&doc.replace(&inventory, &replacement), &suite).is_err());
    }
    for extra in [
        inventory.clone(),
        quoted,
        "<table><tr><td>compose.false</td></tr></table>".into(),
    ] {
        assert!(validate(&format!("{doc}\n{extra}"), &suite).is_err());
    }
    // Fenced illustrative text is not an authoritative extra table.
    assert_eq!(
        validate(&format!("{doc}\n```text\n{inventory}\n```"), &suite),
        Ok(())
    );
}
#[test]
fn irrelevant_metadata_and_table_order_do_not_create_authority() {
    let (_, mut suite) = inputs();
    suite.cases.reverse();
    for c in &mut suite.cases {
        if c.obligation != "metadata-authority" {
            c.facts.metadata_claims_authority = !c.facts.metadata_claims_authority;
        }
    }
    assert_eq!(validate(&table(&suite), &suite), Ok(()));
}
#[test]
fn vocabulary_is_owned_by_selected_contracts_and_shared_ess() {
    let (_, suite) = inputs();
    let model: Value = serde_yaml_ng::from_str(&read("ess/domains/service_wire.yaml")).unwrap();
    let ty = |name: &str| {
        model["types"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .unwrap()
    };
    let codes = ty("connectors.service_wire.ErrorCode")["variants"]
        .as_array()
        .unwrap();
    for c in &suite.cases {
        if !c.expected["code"].is_null() {
            assert!(codes.contains(&c.expected["code"]));
        }
    }
    let source_fields: BTreeSet<_> = ty("connectors.service_wire.SourceAudit")["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        source_fields,
        BTreeSet::from(["instance", "audit_ref", "audit_status"])
    );
    let limit_fields: BTreeSet<_> = ty("connectors.service_wire.OperationLimits")["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        limit_fields,
        BTreeSet::from([
            "request_bytes",
            "result_bytes",
            "execution_ms",
            "provider_ms",
            "connect_ms"
        ])
    );
    let state = read("adapters/mcp/spec/ess/domains/state.yaml");
    let state = state.split_once("The relation census, closed").unwrap().1;
    assert_eq!(
        state
            .lines()
            .filter(|l| l.trim_start().starts_with("#") && l.contains(". UNMAPPED: "))
            .count(),
        7
    );
    let resolved: Vec<_> = state
        .lines()
        .filter(|l| l.trim_start().starts_with("#") && l.contains(". RESOLVED: "))
        .collect();
    assert_eq!(resolved.len(), 1);
    assert!(resolved[0].contains("McpServerBinding → supervised OS process"));
}
#[test]
fn revision_and_family_fixture_contradictions_are_rejected() {
    let (_, suite) = inputs();
    for family in ["tools", "resources", "prompts"] {
        let mut bad = suite.clone();
        let c = bad
            .cases
            .iter_mut()
            .find(|c| c.family == family && c.outbound == "2026-07-28")
            .unwrap();
        let mut wire: Value = serde_json::from_str(&c.facts.outbound_response).unwrap();
        wire["result"].as_object_mut().unwrap().remove("resultType");
        c.facts.outbound_response = wire.to_string();
        c.expected = observation(c);
        assert!(
            validate(&table(&bad), &bad)
                .unwrap_err()
                .contains("revision fixture")
        );
    }
    let mut bad = suite.clone();
    let c = bad
        .cases
        .iter_mut()
        .find(|c| c.obligation == "modern-array")
        .unwrap();
    c.outbound = "2025-11-25".into();
    let mut wire: Value = serde_json::from_str(&c.facts.outbound_response).unwrap();
    wire["result"].as_object_mut().unwrap().remove("resultType");
    c.facts.outbound_response = wire.to_string();
    c.expected = observation(c);
    assert!(
        validate(&table(&bad), &bad)
            .unwrap_err()
            .contains("tool fixture")
    );
}

#[test]
fn adversary_success_pairs_require_real_correlation_and_exclusive_result() {
    let (_, suite) = inputs();
    for revision in ["2025-11-25", "2026-07-28"] {
        for family in ["tools", "resources", "prompts"] {
            for defect in ["missing-request-id", "missing-both-ids", "error-and-result"] {
                let mut bad = suite.clone();
                let c = bad
                    .cases
                    .iter_mut()
                    .find(|c| {
                        c.obligation == "paired" && c.outbound == revision && c.family == family
                    })
                    .unwrap();
                let mut request: Value = serde_json::from_str(&c.facts.request).unwrap();
                let mut response: Value = serde_json::from_str(&c.facts.outbound_response).unwrap();
                match defect {
                    "missing-request-id" => {
                        request.as_object_mut().unwrap().remove("id");
                    }
                    "missing-both-ids" => {
                        request.as_object_mut().unwrap().remove("id");
                        response.as_object_mut().unwrap().remove("id");
                    }
                    "error-and-result" => {
                        response["error"] = json!({"code":-32603,"message":"failure"});
                    }
                    _ => unreachable!(),
                }
                c.facts.request = request.to_string();
                c.facts.outbound_response = response.to_string();
                c.expected = observation(c);
                assert!(
                    validate(&table(&bad), &bad).is_err(),
                    "invalid success fixture accepted: {revision}/{family}/{defect}"
                );
            }
        }
    }
}

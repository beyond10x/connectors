//! Checks authored replay documents. No MCP request, ledger or provider is executed.
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const DOCUMENT: &str = "adapters/mcp/contracts/server/v1alpha1/mutations.md";
const CASES: &str = "adapters/mcp/contracts/server/v1alpha1/mutation-cases.json";
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cases {
    format: String,
    evidence: String,
    mutation_advertised: bool,
    attempts: Vec<StateRow>,
    reservations: Vec<StateRow>,
    coordinates: Vec<Coordinate>,
    errors: Vec<ErrorRow>,
    decisions: Vec<Decision>,
    correlations: Vec<Correlation>,
    observations: Vec<Observation>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    id: String,
    observer: String,
    stage: String,
    evidence: String,
    cause: String,
    terminal_durable: bool,
    reply_received: bool,
    classification: String,
    code: String,
    known_result_replayable: bool,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct StateRow {
    state: String,
    observation: String,
    retention: String,
    dispatch_permission: bool,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Coordinate {
    owner: String,
    field: String,
    source: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorRow {
    code: String,
    channel: String,
    meaning: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Decision {
    id: String,
    policy: String,
    observation_admitted: bool,
    revision_current: bool,
    lookup: String,
    input_valid: bool,
    kind: String,
    key: Option<String>,
    reservation: String,
    fingerprint_matches: bool,
    final_disclosure_admitted: bool,
    approval: String,
    winner_recheck: String,
    expected: String,
    extra_dispatches: u32,
    extra_spends: u32,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Correlation {
    id: String,
    left_mcp_id: Value,
    right_mcp_id: Value,
    left_host_id: String,
    right_host_id: String,
    same_namespace: bool,
    same_key: bool,
    same_fingerprint: bool,
    expected: String,
}
fn read(path: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap_or_else(|error| panic!("read {path}: {error}"))
}
fn inputs() -> (String, Cases) {
    (
        read(DOCUMENT),
        serde_json::from_str(&read(CASES)).expect("closed document-case schema"),
    )
}
fn model(path: &str) -> Value {
    serde_yaml_ng::from_str(&read(path)).expect("model YAML")
}
fn declaration<'a>(model: &'a Value, group: &str, name: &str) -> &'a Value {
    model[group]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == name)
        .unwrap()
}
fn strings(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap().to_owned())
        .collect()
}
fn fields(model: &Value, name: &str) -> BTreeSet<String> {
    declaration(model, "types", name)["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["name"].as_str().unwrap().to_owned())
        .collect()
}
fn decision(case: &Decision) -> Result<&str, String> {
    if case.policy == "deny" {
        return Ok("not_granted");
    }
    if case.policy == "unavailable" {
        return Ok("unavailable");
    }
    if case.policy != "admit" {
        return Err("unknown policy".into());
    }
    if !case.observation_admitted {
        return Ok("forbidden");
    }
    if !case.revision_current {
        return Ok("stale_description");
    }
    match case.lookup.as_str() {
        "absent" => return Ok("not_found"),
        "disabled" => return Ok("forbidden"),
        "enabled" => {}
        _ => return Err("unknown lookup decision".into()),
    }
    if !case.input_valid {
        return Ok("invalid_input");
    }
    if (case.kind == "keyed" && case.key.as_deref().is_none_or(str::is_empty))
        || (case.kind != "keyed" && case.key.is_some())
    {
        return Ok("invalid_input");
    }
    if case.kind == "keyed" {
        if !case.final_disclosure_admitted {
            return Ok("forbidden");
        }
        if case.reservation == "unreadable" {
            return Ok("outcome_unknown");
        }
        if ["Pending", "Replayable", "Quarantined"].contains(&case.reservation.as_str()) {
            if !case.fingerprint_matches {
                return Ok("idempotency_conflict");
            }
            return Ok(match case.reservation.as_str() {
                "Pending" => "wait_original",
                "Replayable" => "retained_result",
                _ => "outcome_unknown",
            });
        }
        if !["absent", "Expired"].contains(&case.reservation.as_str()) {
            return Err("unknown reservation observation".into());
        }
    }
    let refusal = match case.approval.as_str() {
        "fresh" => return Ok("candidate_new"),
        "missing" => "approval_required",
        "refused" => "approval_refused",
        "spent" => "approval_replayed",
        "preflight_unavailable" => "unavailable",
        _ => return Err("unknown candidate preflight".into()),
    };
    if case.kind == "keyed" {
        return Ok(match case.winner_recheck.as_str() {
            "exact" => "observe_winner",
            "conflict" => "idempotency_conflict",
            "unreadable" => "outcome_unknown",
            "absent" => refusal,
            _ => return Err("unknown winner recheck".into()),
        });
    }
    Ok(refusal)
}
// Required document examples must reach the obligation they name. Output
// agreement alone would let an earlier refusal erase all later-stage coverage.
// These predicates constrain premises, not opaque fixture values or outcomes.
fn decision_premises(case: &Decision) -> bool {
    let admitted = case.policy == "admit" && case.observation_admitted;
    let current = admitted && case.revision_current;
    let enabled = current && case.lookup == "enabled";
    let valid_input = enabled && case.input_valid;
    let valid_key = case.kind == "keyed" && case.key.as_deref().is_some_and(|key| !key.is_empty());
    let keyed = valid_input && valid_key;
    let disclosed = keyed && case.final_disclosure_admitted;
    let live = ["Pending", "Replayable", "Quarantined"].contains(&case.reservation.as_str());
    let retained = valid_key && live && case.fingerprint_matches && case.final_disclosure_admitted;
    let matched = disclosed && live && case.fingerprint_matches;
    let candidate = disclosed && ["absent", "Expired"].contains(&case.reservation.as_str());
    let approval_failure = ["missing", "refused", "spent"].contains(&case.approval.as_str());
    let preflight_failure = approval_failure || case.approval == "preflight_unavailable";
    match case.id.as_str() {
        "policy-before-stale" => case.policy == "deny" && !case.revision_current,
        "scope-before-stale" => {
            case.policy == "admit" && !case.observation_admitted && !case.revision_current
        }
        "revision-before-key" => {
            admitted
                && !case.revision_current
                && case.lookup == "enabled"
                && case.input_valid
                && retained
        }
        "disabled-before-key" => {
            current && case.lookup == "disabled" && case.input_valid && retained
        }
        "input-before-key" => enabled && !case.input_valid && retained,
        "key-required" => valid_input && case.kind == "keyed" && !valid_key,
        "none-rejects-key" => valid_input && case.kind == "none" && case.key.is_some(),
        "natural-rejects-key" => valid_input && case.kind == "natural" && case.key.is_some(),
        "exact-replay" => matched && case.reservation == "Replayable" && preflight_failure,
        "pending-waits" => matched && case.reservation == "Pending",
        "quarantine-never-resends" => matched && case.reservation == "Quarantined",
        "conflict" => disclosed && live && !case.fingerprint_matches,
        "revoked-before-delivery" => {
            keyed && live && case.fingerprint_matches && !case.final_disclosure_admitted
        }
        "missing-approval" => {
            candidate && case.approval == "missing" && case.winner_recheck == "absent"
        }
        "refused-approval" => {
            candidate && case.approval == "refused" && case.winner_recheck == "absent"
        }
        "spent-approval" => {
            candidate && case.approval == "spent" && case.winner_recheck == "absent"
        }
        "winner-after-miss" => candidate && approval_failure && case.winner_recheck == "exact",
        "conflict-after-miss" => {
            candidate && preflight_failure && case.winner_recheck == "conflict"
        }
        "unreadable-after-miss" => {
            candidate && preflight_failure && case.winner_recheck == "unreadable"
        }
        "preflight-winner" => {
            candidate && case.approval == "preflight_unavailable" && case.winner_recheck == "exact"
        }
        "expired-new-approval" => {
            candidate
                && case.reservation == "Expired"
                && approval_failure
                && case.winner_recheck == "absent"
        }
        "candidate-new" => candidate && case.approval == "fresh",
        "unreadable-ledger" => disclosed && case.reservation == "unreadable",
        _ => false,
    }
}
fn correlation_premises(case: &Correlation) -> bool {
    if case.left_host_id == case.right_host_id {
        return false;
    }
    let same_correlation = case.left_mcp_id == case.right_mcp_id;
    match case.id.as_str() {
        "new-correlation-same-key" => {
            !same_correlation && case.same_namespace && case.same_key && case.same_fingerprint
        }
        "same-correlation-new-key" => {
            same_correlation && case.same_namespace && !case.same_key && case.same_fingerprint
        }
        "same-key-changed-meaning" => {
            case.same_namespace && case.same_key && !case.same_fingerprint
        }
        "same-spelling-other-authority" => {
            same_correlation && !case.same_namespace && case.same_key && case.same_fingerprint
        }
        _ => false,
    }
}
fn observation_premises(case: &Observation) -> bool {
    let (observer, evidence, durable, stage) = match case.id.as_str() {
        "caller-lost-reply" if !case.reply_received => ("caller", "applied", true, "observation"),
        "live-applied-store-failure" => ("live_host", "applied", false, "attempt_store"),
        "live-applied-audit-failure" => ("live_host", "applied", true, "audit"),
        "recovery-after-dispatch" => ("recovery", "none", false, "observation"),
        "fenced-abort" => ("recovery", "not_attempted", true, "dispatch"),
        "definitive-refusal" => ("live_host", "refused", true, "response"),
        _ => return false,
    };
    case.observer == observer
        && case.evidence == evidence
        && case.terminal_durable == durable
        && case.stage == stage
}
fn parsed_tables(document: &str) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut tables = BTreeMap::new();
    let mut rows = Vec::new();
    let mut cells: Vec<String> = Vec::new();
    let mut in_cell = false;
    for event in Parser::new_ext(document, Options::ENABLE_TABLES) {
        match event {
            Event::Start(Tag::Table(_)) => rows.clear(),
            Event::Start(Tag::TableHead | Tag::TableRow) => cells.clear(),
            Event::Start(Tag::TableCell) => {
                cells.push(String::new());
                in_cell = true;
            }
            Event::End(TagEnd::TableCell) => in_cell = false,
            Event::Text(text) | Event::Code(text) if in_cell => {
                cells.last_mut().unwrap().push_str(&text)
            }
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                rows.push(format!("| {} |", cells.join(" | ")))
            }
            Event::End(TagEnd::Table) => {
                let header = rows.first().ok_or("empty table")?.clone();
                if tables.insert(header, rows[1..].to_vec()).is_some() {
                    return Err("duplicate table".into());
                }
            }
            _ => {}
        }
    }
    Ok(tables)
}
fn check(document: &str, cases: &Cases) -> Result<(), String> {
    if cases.format != "mcp-inbound-replay-document/1"
        || cases.evidence != "document-only"
        || cases.mutation_advertised
    {
        return Err("document work cannot advertise mutation".into());
    }
    let mutations = model("ess/domains/mutations.yaml");
    let idempotency = model("ess/domains/idempotency.yaml");
    let wire = model("ess/domains/service_wire.yaml");
    let mut expected: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut state_rows = Vec::new();
    for (owner, rows, source, entity) in [
        (
            "attempt",
            &cases.attempts,
            &mutations,
            "connectors.mutations.AttemptRecord",
        ),
        (
            "reservation",
            &cases.reservations,
            &idempotency,
            "connectors.idempotency.KeyReservation",
        ),
    ] {
        let authoritative =
            strings(&declaration(source, "entities", entity)["lifecycle"]["states"]);
        let actual: BTreeSet<_> = rows.iter().map(|row| row.state.clone()).collect();
        if rows.len() != authoritative.len() || actual != authoritative {
            return Err("incomplete or duplicate lifecycle inventory".into());
        }
        for row in rows {
            let (observation, retention) = match (owner, row.state.as_str()) {
                ("attempt", "Prepared") => (
                    "unknown_until_observed_or_fenced_abort",
                    "audit_independent",
                ),
                ("attempt", "Dispatching") => {
                    ("unknown_without_definitive_evidence", "audit_independent")
                }
                ("attempt", "Aborted") => ("not_attempted", "audit_independent"),
                ("attempt", "Completed") => ("applied", "audit_independent"),
                ("attempt", "Failed") => ("refused", "audit_independent"),
                ("attempt", "Indeterminate") => ("unknown", "audit_independent"),
                ("reservation", "Pending") => ("wait_original", "no_automatic_expiry"),
                ("reservation", "Replayable") => {
                    ("retained_result", "settlement_plus_86400_seconds")
                }
                ("reservation", "Quarantined") => ("outcome_unknown", "no_automatic_expiry"),
                ("reservation", "Expired") => ("candidate_new", "known_result_only"),
                _ => return Err("unmapped lifecycle state".into()),
            };
            if row.observation != observation
                || row.retention != retention
                || row.dispatch_permission
            {
                return Err("state grants dispatch or weakens observation/retention".into());
            }
            state_rows.push(format!(
                "| {owner} | {} | {observation} | {retention} | false |",
                row.state
            ));
        }
    }
    expected.insert(
        "| Owner | State | Observation | Retention | Dispatch permission |".into(),
        state_rows,
    );
    let mut authoritative_coordinates = BTreeSet::new();
    for (owner, type_name) in [
        ("namespace", "KeyNamespace"),
        ("authority", "AuthorityScope"),
        ("origin", "Origin"),
        ("fingerprint", "RequestFingerprint"),
        ("route", "RouteBinding"),
    ] {
        authoritative_coordinates.extend(
            fields(&idempotency, &format!("connectors.idempotency.{type_name}"))
                .into_iter()
                .map(|field| (owner.to_owned(), field)),
        );
    }
    let actual_coordinates: BTreeSet<_> = cases
        .coordinates
        .iter()
        .map(|row| (row.owner.clone(), row.field.clone()))
        .collect();
    if actual_coordinates != authoritative_coordinates
        || cases.coordinates.len() != authoritative_coordinates.len()
        || cases
            .coordinates
            .iter()
            .any(|row| row.source != "resolved_host")
    {
        return Err("unqualified, duplicate or caller-controlled coordinates".into());
    }
    expected.insert(
        "| Owner | Field | Source |".into(),
        cases
            .coordinates
            .iter()
            .map(|row| format!("| {} | {} | {} |", row.owner, row.field, row.source))
            .collect(),
    );
    let legal_codes =
        strings(&declaration(&wire, "types", "connectors.service_wire.ErrorCode")["variants"]);
    let required: BTreeSet<_> = [
        "approval_required",
        "approval_refused",
        "approval_replayed",
        "idempotency_conflict",
        "outcome_unknown",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let actual: BTreeSet<_> = cases.errors.iter().map(|row| row.code.clone()).collect();
    if actual != required
        || cases.errors.len() != required.len()
        || !required.is_subset(&legal_codes)
    {
        return Err("missing, duplicate or unknown mutation error".into());
    }
    let projection: Value = serde_json::from_str(&read(
        "adapters/mcp/contracts/server/v1alpha1/projection-cases.json",
    ))
    .unwrap();
    for row in &cases.errors {
        let source = projection["errors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["code"] == row.code)
            .ok_or("missing projection error")?;
        if source["tool_execution"] != row.channel || row.meaning.is_empty() {
            return Err("mutation error contradicts projection".into());
        }
    }
    expected.insert(
        "| Error | Resolved tool channel | Observable meaning |".into(),
        cases
            .errors
            .iter()
            .map(|row| format!("| {} | {} | {} |", row.code, row.channel, row.meaning))
            .collect(),
    );
    let kinds = strings(
        &declaration(&mutations, "types", "connectors.mutations.IdempotencyKind")["variants"],
    );
    let mut ids = BTreeSet::new();
    let mut decision_rows = Vec::new();
    for case in &cases.decisions {
        if !ids.insert(case.id.clone()) || !kinds.contains(&case.kind) {
            return Err("duplicate decision or unknown idempotency kind".into());
        }
        if !decision_premises(case) {
            return Err(format!("missing required decision premises: {}", case.id));
        }
        if decision(case)? != case.expected || case.extra_dispatches != 0 || case.extra_spends != 0
        {
            return Err("unsafe replay decision".into());
        }
        decision_rows.push(format!("| {} | {} | 0 | 0 |", case.id, case.expected));
    }
    let required_decisions = [
        "policy-before-stale",
        "scope-before-stale",
        "revision-before-key",
        "disabled-before-key",
        "input-before-key",
        "key-required",
        "none-rejects-key",
        "natural-rejects-key",
        "exact-replay",
        "pending-waits",
        "quarantine-never-resends",
        "conflict",
        "revoked-before-delivery",
        "missing-approval",
        "refused-approval",
        "spent-approval",
        "winner-after-miss",
        "conflict-after-miss",
        "unreadable-after-miss",
        "preflight-winner",
        "expired-new-approval",
        "candidate-new",
        "unreadable-ledger",
    ];
    if ids != required_decisions.into_iter().map(str::to_owned).collect() {
        return Err("incomplete decision inventory".into());
    }
    expected.insert(
        "| Case | Decision | Extra dispatches | Extra spends |".into(),
        decision_rows,
    );
    let mut correlation_rows = Vec::new();
    let mut correlations = BTreeSet::new();
    for case in &cases.correlations {
        if !correlations.insert(case.id.clone()) {
            return Err("duplicate correlation case".into());
        }
        if !correlation_premises(case) {
            return Err(format!(
                "missing required correlation premises: {}",
                case.id
            ));
        }
        if ![&case.left_mcp_id, &case.right_mcp_id]
            .iter()
            .all(|id| id.is_string() || id.is_i64())
            || case.left_host_id.is_empty()
            || case.right_host_id.is_empty()
        {
            return Err("invalid fixture correlation".into());
        }
        let outcome = if !case.same_namespace || !case.same_key {
            "distinct_intent"
        } else if !case.same_fingerprint {
            "idempotency_conflict"
        } else {
            "same_reservation"
        };
        if case.expected != outcome {
            return Err("correlation used as replay authority".into());
        }
        correlation_rows.push(format!("| {} | {outcome} |", case.id));
    }
    if correlations
        != [
            "new-correlation-same-key",
            "same-correlation-new-key",
            "same-key-changed-meaning",
            "same-spelling-other-authority",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    {
        return Err("incomplete correlation inventory".into());
    }
    expected.insert(
        "| Correlation case | Business identity decision |".into(),
        correlation_rows,
    );
    let effects = strings(
        &declaration(&mutations, "types", "connectors.mutations.EffectKnowledge")["variants"],
    );
    let stages =
        strings(&declaration(&wire, "types", "connectors.service_wire.CauseStage")["variants"]);
    let mut observation_ids = BTreeSet::new();
    let mut observation_rows = Vec::new();
    for case in &cases.observations {
        if !observation_ids.insert(case.id.clone())
            || !["caller", "live_host", "recovery"].contains(&case.observer.as_str())
            || (!effects.contains(&case.evidence) && case.evidence != "none")
            || !legal_codes.contains(&case.cause)
            || !stages.contains(&case.stage)
        {
            return Err("invalid observation vocabulary".into());
        }
        if !observation_premises(case) {
            return Err(format!(
                "missing required observation premises: {}",
                case.id
            ));
        }
        let classification =
            if case.evidence == "none" || (case.observer == "caller" && !case.reply_received) {
                "unknown"
            } else {
                &case.evidence
            };
        let code = if classification == "unknown" {
            "outcome_unknown"
        } else {
            &case.cause
        };
        let replayable = case.terminal_durable && classification != "unknown";
        if case.classification != classification
            || case.code != code
            || case.known_result_replayable != replayable
        {
            return Err("lost-reply viewpoint or known-effect contradiction".into());
        }
        observation_rows.push(format!(
            "| {} | {classification} | {code} | {replayable} | {} |",
            case.id, case.reply_received
        ));
    }
    if observation_ids
        != [
            "caller-lost-reply",
            "live-applied-store-failure",
            "live-applied-audit-failure",
            "recovery-after-dispatch",
            "fenced-abort",
            "definitive-refusal",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    {
        return Err("incomplete observation inventory".into());
    }
    expected.insert(
        "| Observation case | Classification | Code | Known result replayable | Reply received |"
            .into(),
        observation_rows,
    );
    let tables = parsed_tables(document)?;
    if tables.keys().collect::<Vec<_>>() != expected.keys().collect::<Vec<_>>() {
        return Err("missing or unknown correspondence table".into());
    }
    for (header, rows) in expected {
        let actual = &tables[&header];
        if actual.len() != rows.len()
            || actual.iter().collect::<BTreeSet<_>>() != rows.iter().collect::<BTreeSet<_>>()
        {
            return Err(format!(
                "incomplete, duplicate or contradictory rows: {header}"
            ));
        }
    }
    Ok(())
}
#[test]
fn authored_replay_contract_matches_existing_owners() {
    let (document, cases) = inputs();
    check(&document, &cases).unwrap();
    let mut varied = cases.clone();
    for case in &mut varied.decisions {
        if case.key.is_some() {
            case.key = Some(" another opaque key ".into());
        }
        if case.id == "exact-replay" {
            case.approval = "spent".into();
        }
    }
    for case in &mut varied.correlations {
        let same = case.left_mcp_id == case.right_mcp_id;
        case.left_mcp_id = Value::from("other-correlation-a");
        case.right_mcp_id = Value::from(if same {
            "other-correlation-a"
        } else {
            "other-correlation-b"
        });
        case.left_host_id = "other-host-request-a".into();
        case.right_host_id = "other-host-request-b".into();
    }
    check(&document, &varied).expect("irrelevant spelling and non-fresh approval variation");
    let mut varied_document = document.clone();
    for case in &mut varied.observations {
        if case.observer != "caller" {
            let previous = format!(
                "| {} | {} | {} | {} | {} |",
                case.id,
                case.classification,
                case.code,
                case.known_result_replayable,
                case.reply_received
            );
            // Host/recovery knowledge does not depend on caller delivery.
            case.reply_received = !case.reply_received;
            varied_document = varied_document.replace(
                &previous,
                &format!(
                    "| {} | {} | {} | {} | {} |",
                    case.id,
                    case.classification,
                    case.code,
                    case.known_result_replayable,
                    case.reply_received
                ),
            );
        }
    }
    varied
        .correlations
        .iter_mut()
        .find(|case| case.id == "same-key-changed-meaning")
        .unwrap()
        .right_mcp_id = Value::from("other-correlation-a");
    check(&varied_document, &varied).expect("delivery and unrelated correlation variation");
}
#[test]
fn copied_cases_cannot_grant_replay_dispatch_or_bypass_admission() {
    let (document, cases) = inputs();
    for id in [
        "policy-before-stale",
        "revision-before-key",
        "revoked-before-delivery",
        "winner-after-miss",
        "quarantine-never-resends",
    ] {
        let mut changed = cases.clone();
        changed
            .decisions
            .iter_mut()
            .find(|case| case.id == id)
            .unwrap()
            .expected = "candidate_new".into();
        assert!(check(&document, &changed).is_err(), "unsafe decision {id}");
    }
    for index in 0..cases.decisions.len() {
        let mut changed = cases.clone();
        changed.decisions[index].extra_dispatches = 1;
        assert!(check(&document, &changed).is_err());
        changed.decisions[index].extra_dispatches = 0;
        changed.decisions[index].extra_spends = 1;
        assert!(check(&document, &changed).is_err());
    }
    for index in 0..cases.decisions.len() {
        let mut changed = cases.clone();
        let case = &mut changed.decisions[index];
        let previous = case.expected.clone();
        if case.id == "policy-before-stale" {
            case.revision_current = true;
        } else {
            case.policy = "deny".into();
            case.expected = "not_granted".into();
        }
        let altered = document.replace(
            &format!("| {} | {previous} | 0 | 0 |", case.id),
            &format!("| {} | {} | 0 | 0 |", case.id, case.expected),
        );
        assert!(
            check(&altered, &changed).is_err(),
            "semantic decision premise erased: {}",
            cases.decisions[index].id
        );
    }
}
#[test]
fn copied_lifecycle_and_coordinate_inventories_remain_closed() {
    let (document, cases) = inputs();
    let mut changed = cases.clone();
    changed.coordinates.pop();
    assert!(check(&document, &changed).is_err());
    let mut changed = cases.clone();
    changed.coordinates.push(changed.coordinates[0].clone());
    assert!(check(&document, &changed).is_err());
    for index in 0..cases.coordinates.len() {
        let mut changed = cases.clone();
        changed.coordinates[index].source = "mcp_arguments".into();
        assert!(check(&document, &changed).is_err());
    }
    for index in 0..cases.reservations.len() {
        let mut changed = cases.clone();
        changed.reservations[index].dispatch_permission = true;
        assert!(check(&document, &changed).is_err());
    }
    let mut changed = cases.clone();
    changed
        .reservations
        .iter_mut()
        .find(|row| row.state == "Quarantined")
        .unwrap()
        .retention = "settlement_plus_86400_seconds".into();
    assert!(check(&document, &changed).is_err());
}
#[test]
fn copied_errors_and_correlation_cannot_erase_uncertainty() {
    let (document, cases) = inputs();
    let mut changed = cases.clone();
    changed.errors.pop();
    assert!(check(&document, &changed).is_err());
    let mut changed = cases.clone();
    changed.errors.push(changed.errors[0].clone());
    assert!(check(&document, &changed).is_err());
    let mut changed = cases.clone();
    changed.errors[0].channel = "generic_failure".into();
    assert!(check(&document, &changed).is_err());
    for index in 0..cases.correlations.len() {
        let mut changed = cases.clone();
        changed.correlations[index].expected = "automatic_redispatch".into();
        assert!(check(&document, &changed).is_err());
    }
    let mut changed = cases.clone();
    changed.mutation_advertised = true;
    assert!(check(&document, &changed).is_err());
    for index in 0..cases.correlations.len() {
        let mut changed = cases.clone();
        let case = &mut changed.correlations[index];
        let previous = case.expected.clone();
        case.same_namespace = !case.same_namespace;
        case.expected = if !case.same_namespace || !case.same_key {
            "distinct_intent"
        } else if !case.same_fingerprint {
            "idempotency_conflict"
        } else {
            "same_reservation"
        }
        .into();
        let altered = document.replace(
            &format!("| {} | {previous} |", case.id),
            &format!("| {} | {} |", case.id, case.expected),
        );
        assert!(
            check(&altered, &changed).is_err(),
            "semantic correlation premise erased: {}",
            cases.correlations[index].id
        );
    }
}
#[test]
fn every_document_table_rejects_extra_missing_and_contradictory_rows() {
    let (document, cases) = inputs();
    check(&document, &cases).unwrap();
    let tables = parsed_tables(&document).unwrap();
    assert_eq!(tables.len(), 6);
    for (header, rows) in tables {
        for row in &rows {
            let duplicate = document.replace(row, &format!("{row}\n{row}"));
            assert!(check(&duplicate, &cases).is_err(), "duplicate {row}");
            let missing = document.replace(row, "");
            assert!(check(&missing, &cases).is_err(), "missing {row}");
            let contradiction = document.replace(
                row,
                &format!("{row}\n| contradictory | {}", row.trim_start_matches('|')),
            );
            assert!(
                check(&contradiction, &cases).is_err(),
                "contradiction {row}"
            );
        }
        let separator = format!("|{}", "---|".repeat(header.matches('|').count() - 1));
        let duplicate_table = format!("{document}\n\n{header}\n{separator}\n{}\n", rows.join("\n"));
        assert!(
            check(&duplicate_table, &cases).is_err(),
            "duplicate table {header}"
        );
    }
}

#[test]
fn copied_observations_preserve_unknown_and_applied_with_error() {
    let (document, cases) = inputs();
    for id in ["caller-lost-reply", "recovery-after-dispatch"] {
        let mut changed = cases.clone();
        changed
            .observations
            .iter_mut()
            .find(|case| case.id == id)
            .unwrap()
            .code = "timeout".into();
        assert!(check(&document, &changed).is_err());
    }
    for id in ["live-applied-store-failure", "live-applied-audit-failure"] {
        let mut changed = cases.clone();
        changed
            .observations
            .iter_mut()
            .find(|case| case.id == id)
            .unwrap()
            .classification = "unknown".into();
        assert!(check(&document, &changed).is_err());
    }
    let mut changed = cases.clone();
    changed
        .observations
        .iter_mut()
        .find(|case| case.id == "live-applied-store-failure")
        .unwrap()
        .known_result_replayable = true;
    assert!(check(&document, &changed).is_err());
    for index in 0..cases.observations.len() {
        let mut changed = cases.clone();
        let case = &mut changed.observations[index];
        case.stage = if case.stage == "preflight" {
            "response"
        } else {
            "preflight"
        }
        .into();
        assert!(
            check(&document, &changed).is_err(),
            "semantic failure stage erased: {}",
            cases.observations[index].id
        );
        let mut changed = cases.clone();
        let case = &mut changed.observations[index];
        // Recompute the copied row so a correspondence mismatch cannot be the
        // reason for rejecting loss of the named observer's viewpoint.
        let previous = format!(
            "| {} | {} | {} | {} | {} |",
            case.id,
            case.classification,
            case.code,
            case.known_result_replayable,
            case.reply_received
        );
        case.observer = if case.observer == "live_host" {
            "recovery"
        } else {
            "live_host"
        }
        .into();
        case.classification = if case.evidence == "none" {
            "unknown".into()
        } else {
            case.evidence.clone()
        };
        case.code = if case.classification == "unknown" {
            "outcome_unknown".into()
        } else {
            case.cause.clone()
        };
        case.known_result_replayable = case.terminal_durable && case.classification != "unknown";
        let altered = document.replace(
            &previous,
            &format!(
                "| {} | {} | {} | {} | {} |",
                case.id,
                case.classification,
                case.code,
                case.known_result_replayable,
                case.reply_received
            ),
        );
        assert!(
            check(&altered, &changed).is_err(),
            "semantic observer erased: {}",
            cases.observations[index].id
        );
    }
}

#[test]
fn adversary_named_replay_cases_cannot_be_replaced_by_policy_denials() {
    let (document, cases) = inputs();
    check(&document, &cases).expect("positive authored control");
    let mut accepted = Vec::new();
    for id in [
        "exact-replay",
        "revision-before-key",
        "winner-after-miss",
        "preflight-winner",
    ] {
        let mut changed = cases.clone();
        let case = changed
            .decisions
            .iter_mut()
            .find(|case| case.id == id)
            .unwrap();
        let previous = case.expected.clone();
        case.policy = "deny".into();
        case.expected = "not_granted".into();
        let changed_document = document.replace(
            &format!("| {id} | {previous} | 0 | 0 |"),
            &format!("| {id} | not_granted | 0 | 0 |"),
        );
        assert_ne!(changed_document, document);
        if check(&changed_document, &changed).is_ok() {
            accepted.push(id);
        }
    }
    assert!(
        accepted.is_empty(),
        "named obligations erased while checker accepts: {accepted:?}"
    );
}

#[test]
fn adversary_recovery_case_must_exercise_recovery_observer() {
    let (document, cases) = inputs();
    check(&document, &cases).expect("positive authored control");
    let mut changed = cases.clone();
    let case = changed
        .observations
        .iter_mut()
        .find(|case| case.id == "recovery-after-dispatch")
        .unwrap();
    assert_eq!(case.observer, "recovery");
    case.observer = "live_host".into();
    assert!(
        check(&document, &changed).is_err(),
        "recovery obligation erased without changing any displayed row"
    );
}

#[test]
fn adversary_retained_admission_revision_and_winner_precedence() {
    let (_, cases) = inputs();
    let original = cases
        .decisions
        .iter()
        .find(|case| case.id == "exact-replay")
        .unwrap();
    for (reservation, expected) in [
        ("Pending", "wait_original"),
        ("Replayable", "retained_result"),
        ("Quarantined", "outcome_unknown"),
    ] {
        let mut case = original.clone();
        case.reservation = reservation.into();
        assert_eq!(decision(&case).unwrap(), expected);
        case.revision_current = false;
        assert_eq!(decision(&case).unwrap(), "stale_description");
        case.policy = "deny".into();
        assert_eq!(decision(&case).unwrap(), "not_granted");
        case.policy = "admit".into();
        case.revision_current = true;
        case.final_disclosure_admitted = false;
        assert_eq!(decision(&case).unwrap(), "forbidden");
    }
    for approval in ["missing", "refused", "spent", "preflight_unavailable"] {
        let mut case = original.clone();
        case.reservation = "absent".into();
        case.approval = approval.into();
        for (winner, expected) in [
            ("exact", "observe_winner"),
            ("conflict", "idempotency_conflict"),
            ("unreadable", "outcome_unknown"),
        ] {
            case.winner_recheck = winner.into();
            assert_eq!(decision(&case).unwrap(), expected);
            case.final_disclosure_admitted = false;
            assert_eq!(decision(&case).unwrap(), "forbidden");
            case.final_disclosure_admitted = true;
        }
    }
}

#[test]
fn adversary_second_observer_knowledge_cannot_be_self_consistently_rewritten() {
    let (document, cases) = inputs();
    check(&document, &cases).expect("unchanged authored positive control");
    for index in 0..cases.observations.len() {
        for change_durability in [false, true] {
            let mut changed = cases.clone();
            let case = &mut changed.observations[index];
            let original_row = format!(
                "| {} | {} | {} | {} | {} |",
                case.id,
                case.classification,
                case.code,
                case.known_result_replayable,
                case.reply_received
            );
            if change_durability {
                case.terminal_durable = !case.terminal_durable;
            } else {
                case.evidence = if case.evidence == "applied" {
                    "none"
                } else {
                    "applied"
                }
                .into();
            }
            // Keep output and Markdown mutually consistent: rejection must come
            // from erasing the named knowledge/durability obligation itself.
            case.classification =
                if case.evidence == "none" || (case.observer == "caller" && !case.reply_received) {
                    "unknown".into()
                } else {
                    case.evidence.clone()
                };
            case.code = if case.classification == "unknown" {
                "outcome_unknown".into()
            } else {
                case.cause.clone()
            };
            case.known_result_replayable =
                case.terminal_durable && case.classification != "unknown";
            let altered = document.replace(
                &original_row,
                &format!(
                    "| {} | {} | {} | {} | {} |",
                    case.id,
                    case.classification,
                    case.code,
                    case.known_result_replayable,
                    case.reply_received
                ),
            );
            let expected_refusal = format!("missing required observation premises: {}", case.id);
            let refusal =
                check(&altered, &changed).expect_err("required knowledge cannot disappear");
            assert_eq!(
                refusal, expected_refusal,
                "must reject erased semantics rather than incidental row mismatch"
            );
        }
    }
}

//! Authored document consistency checks, not an MCP runtime or conformance target.
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::Path;

const CONTRACT: &str = "adapters/mcp/contracts/server/v1alpha1/projection.md";
const CASES: &str = "adapters/mcp/contracts/server/v1alpha1/projection-cases.json";

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cases {
    format: String,
    evidence: String,
    errors: Vec<ErrorRow>,
    names: Vec<NameCase>,
    visibility: Vec<VisibilityCase>,
    families: Vec<FamilyCase>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ErrorRow {
    code: String,
    tool_execution: String,
    resource: String,
    prompt: String,
    observation: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct NameCase {
    id: String,
    instance: String,
    adapter: String,
    operation: String,
    name: Option<String>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct VisibilityCase {
    id: String,
    implemented: bool,
    bound: bool,
    enabled: bool,
    metadata_admitted: bool,
    policy: String,
    observation_admitted: bool,
    revision_current: bool,
    input_valid: bool,
    dependency_ready: bool,
    approval_ready: bool,
    visible: bool,
    outcome: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct FamilyCase {
    id: String,
    family: String,
    complete_binding: bool,
    read_only: bool,
    empty_input: bool,
    string_arguments: bool,
    prompt_messages: bool,
    advertised: bool,
}
fn read(path: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap_or_else(|e| panic!("read {path}: {e}"))
}
fn inputs() -> (String, Cases, BTreeSet<String>) {
    let document = read(CONTRACT);
    let cases = serde_json::from_str(&read(CASES)).expect("strict document case format");
    let model: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&read("ess/domains/service_wire.yaml")).unwrap();
    let variants = model["types"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|entry| entry["name"].as_str() == Some("connectors.service_wire.ErrorCode"))
        .unwrap()["variants"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect();
    (document, cases, variants)
}
fn hex(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        write!(encoded, "{byte:02x}").expect("writing to a string");
    }
    encoded
}
fn encode(instance: &str, adapter: &str, operation: &str) -> Option<String> {
    if [instance, adapter, operation]
        .iter()
        .any(|part| part.is_empty())
    {
        return None;
    }
    let name = format!("c1_{}_{}_{}", hex(instance), hex(adapter), hex(operation));
    (name.len() <= 128).then_some(name)
}
fn decode(name: &str) -> Option<[String; 3]> {
    let parts: Vec<_> = name.strip_prefix("c1_")?.split('_').collect();
    if parts.len() != 3 {
        return None;
    }
    let mut result = Vec::new();
    for part in parts {
        if part.is_empty()
            || !part.len().is_multiple_of(2)
            || !part
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return None;
        }
        let bytes: Option<Vec<_>> = part
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
            .collect();
        result.push(String::from_utf8(bytes?).ok()?);
    }
    let result: [String; 3] = result.try_into().ok()?;
    (encode(&result[0], &result[1], &result[2]).as_deref() == Some(name)).then_some(result)
}
fn visibility(case: &VisibilityCase) -> Result<(bool, &str), String> {
    let visible = case.implemented
        && case.bound
        && case.enabled
        && case.metadata_admitted
        && case.policy != "unavailable";
    let outcome = match case.policy.as_str() {
        "deny" => "not_granted",
        "unavailable" => "unavailable",
        "admit" if !case.observation_admitted => "forbidden",
        "admit" if !case.revision_current => "stale_description",
        "admit" if !case.implemented || !case.bound => "not_found",
        "admit" if !case.enabled => "forbidden",
        "admit" if !case.input_valid => "invalid_input",
        "admit" => "candidate_or_retained_key",
        _ => return Err("unknown policy decision".into()),
    };
    // Readiness and fresh approval are intentionally not visibility/lookup inputs.
    let _ = (case.dependency_ready, case.approval_ready);
    Ok((visible, outcome))
}
const ERROR_HEADER: &str =
    "| Shared code | Resolved tool execution | Resource | Prompt | Observable service fact |";
const NAME_HEADER: &str = "| Case | Advertised name |";
const VISIBILITY_HEADER: &str = "| Case | Discovery | First lookup/input outcome |";
const FAMILY_HEADER: &str = "| Case | Family | Compatibility decision before admission |";

/// Every rendered Markdown table must correspond to exactly one closed case table.
/// Parsing the whole document prevents a second table or a row after a blank line
/// from escaping a check that only scans the first contiguous matching block.
fn check_closed_tables(
    document: &str,
    expected: &BTreeMap<&str, BTreeSet<String>>,
) -> Result<(), String> {
    let mut observed_headers = BTreeSet::new();
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
                cells
                    .last_mut()
                    .ok_or("table cell without row")?
                    .push_str(&text);
            }
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                rows.push(format!("| {} |", cells.join(" | ")));
            }
            Event::End(TagEnd::Table) => {
                let header = rows.first().ok_or("empty correspondence table")?;
                if !observed_headers.insert(header.clone()) {
                    return Err("duplicate correspondence table".into());
                }
                let expected_rows = expected
                    .get(header.as_str())
                    .ok_or("unknown correspondence table")?;
                let unique: BTreeSet<_> = rows.iter().skip(1).cloned().collect();
                if unique.len() != rows.len() - 1 || &unique != expected_rows {
                    return Err(format!(
                        "missing, duplicate, unknown or contradictory rows: {header}"
                    ));
                }
            }
            _ => {}
        }
    }
    if observed_headers != expected.keys().map(|header| (*header).to_owned()).collect() {
        return Err("missing correspondence table".into());
    }
    Ok(())
}

fn check(document: &str, cases: &Cases, codes: &BTreeSet<String>) -> Result<(), String> {
    let mut tables: BTreeMap<_, BTreeSet<String>> =
        [ERROR_HEADER, NAME_HEADER, VISIBILITY_HEADER, FAMILY_HEADER]
            .into_iter()
            .map(|header| (header, BTreeSet::new()))
            .collect();
    if cases.format != "mcp-inbound-projection-document/1" || cases.evidence != "document-only" {
        return Err("document evidence boundary".into());
    }
    let mut seen = BTreeSet::new();
    for error in &cases.errors {
        if !seen.insert(error.code.clone()) {
            return Err("duplicate error".into());
        }
        if error.tool_execution != "tool-error"
            || error.resource != "rpc-error"
            || error.prompt != "rpc-error"
            || error.observation.is_empty()
        {
            return Err("error channel or observation".into());
        }
        let row = format!(
            "| {} | tool-error | rpc-error | rpc-error | {} |",
            error.code, error.observation
        );
        if !document.lines().any(|line| line == row) {
            return Err(format!("missing error row {}", error.code));
        }
        tables.get_mut(ERROR_HEADER).unwrap().insert(row);
    }
    if &seen != codes {
        return Err("error vocabulary differs from ESS".into());
    }
    let error_table = document
        .split_once("| Shared code |")
        .ok_or("missing error table")?
        .1;
    let table_codes: Vec<_> = error_table
        .lines()
        .skip(2)
        .take_while(|line| line.starts_with('|'))
        .map(|line| line.split('|').nth(1).unwrap_or_default().trim())
        .collect();
    if table_codes.len() != codes.len()
        || table_codes.iter().copied().collect::<BTreeSet<_>>()
            != codes.iter().map(String::as_str).collect()
    {
        return Err("error table has missing, duplicate or unknown mapping".into());
    }
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for case in &cases.names {
        if !ids.insert(case.id.as_str()) {
            return Err("duplicate case ID".into());
        }
        let expected = encode(&case.instance, &case.adapter, &case.operation);
        if expected != case.name {
            return Err("ambiguous or noncanonical qualified name".into());
        }
        if let Some(name) = &case.name
            && (!names.insert(name)
                || decode(name)
                    != Some([
                        case.instance.clone(),
                        case.adapter.clone(),
                        case.operation.clone(),
                    ]))
        {
            return Err("name collision or inverse".into());
        }
        let row = format!(
            "| {} | {} |",
            case.id,
            case.name.as_deref().unwrap_or("omitted")
        );
        if !document.lines().any(|line| line == row) {
            return Err("name case absent from contract".into());
        }
        tables.get_mut(NAME_HEADER).unwrap().insert(row);
    }
    for case in &cases.visibility {
        if !ids.insert(case.id.as_str()) {
            return Err("duplicate case ID".into());
        }
        let (visible, outcome) = visibility(case)?;
        if visible != case.visible || outcome != case.outcome {
            return Err("contradicted visibility or refusal precedence".into());
        }
        let row = format!(
            "| {} | {} | {} |",
            case.id,
            if visible { "visible" } else { "omitted" },
            outcome
        );
        if !document.lines().any(|line| line == row) {
            return Err("visibility case absent from contract".into());
        }
        tables.get_mut(VISIBILITY_HEADER).unwrap().insert(row);
    }
    let mut families = BTreeSet::new();
    for case in &cases.families {
        if !ids.insert(case.id.as_str()) {
            return Err("duplicate case ID".into());
        }
        families.insert(case.family.as_str());
        let compatible = match case.family.as_str() {
            "tools" => case.complete_binding,
            "resources" => case.complete_binding && case.read_only && case.empty_input,
            "prompts" => {
                case.complete_binding
                    && case.read_only
                    && case.string_arguments
                    && case.prompt_messages
            }
            _ => return Err("unselected capability family".into()),
        };
        if case.advertised != compatible {
            return Err("weakened family projection".into());
        }
        let row = format!(
            "| {} | {} | {} |",
            case.id,
            case.family,
            if compatible { "eligible" } else { "omitted" }
        );
        if !document.lines().any(|line| line == row) {
            return Err("family case absent from contract".into());
        }
        tables.get_mut(FAMILY_HEADER).unwrap().insert(row);
    }
    if families != BTreeSet::from(["tools", "resources", "prompts"])
        || cases.names.len() < 5
        || cases.visibility.len() < 12
    {
        return Err("missing acceptance cases".into());
    }
    check_closed_tables(document, &tables)
}
#[test]
fn authored_projection_matches_closed_errors_and_named_cases() {
    let (document, cases, codes) = inputs();
    check(&document, &cases, &codes).unwrap();
}
#[test]
fn checker_rejects_missing_and_duplicate_error_rows() {
    let (document, cases, codes) = inputs();
    let mut missing = cases.clone();
    missing.errors.pop();
    assert_eq!(
        check(&document, &missing, &codes).unwrap_err(),
        "error vocabulary differs from ESS"
    );
    let mut duplicate = cases.clone();
    duplicate.errors.push(duplicate.errors[0].clone());
    assert_eq!(
        check(&document, &duplicate, &codes).unwrap_err(),
        "duplicate error"
    );
    let altered = document.replace(
        "| outcome_unknown | tool-error |",
        "| outcome_unknown | rpc-error |",
    );
    assert!(
        check(&altered, &cases, &codes)
            .unwrap_err()
            .contains("outcome_unknown")
    );
    let first_row = document
        .lines()
        .find(|line| line.starts_with("| invalid_input |"))
        .unwrap();
    let duplicated_document = document.replace(first_row, &format!("{first_row}\n{first_row}"));
    assert_eq!(
        check(&duplicated_document, &cases, &codes).unwrap_err(),
        "error table has missing, duplicate or unknown mapping"
    );
}
#[test]
fn checker_rejects_ambiguous_names_and_invalid_inverse_spellings() {
    let (document, mut cases, codes) = inputs();
    let duplicate_name = cases.names[0].name.clone();
    cases.names[1].name = duplicate_name;
    assert_eq!(
        check(&document, &cases, &codes).unwrap_err(),
        "ambiguous or noncanonical qualified name"
    );
    for invalid in [
        "c1_61_62_63_64",
        "c1_6A_62_63",
        "c1__62_63",
        "c1_ff_62_63",
        "c1_6_62_63",
    ] {
        assert!(decode(invalid).is_none(), "accepted {invalid}");
    }
}
#[test]
fn checker_rejects_visibility_and_precedence_contradictions() {
    let (document, cases, codes) = inputs();
    for id in [
        "dependency-unready",
        "approval-missing",
        "policy-beats-stale",
        "revision-beats-hidden",
        "scope-beats-stale",
    ] {
        let mut changed = cases.clone();
        let row = changed
            .visibility
            .iter_mut()
            .find(|case| case.id == id)
            .unwrap();
        if id == "dependency-unready" || id == "approval-missing" {
            row.visible = false;
        } else {
            row.outcome = "not_found".into();
        }
        assert_eq!(
            check(&document, &changed, &codes).unwrap_err(),
            "contradicted visibility or refusal precedence"
        );
    }
}
#[test]
fn checker_rejects_weak_family_projection() {
    let (document, mut cases, codes) = inputs();
    let row = cases
        .families
        .iter_mut()
        .find(|case| case.id == "resource-write-refused")
        .unwrap();
    row.advertised = true;
    assert_eq!(
        check(&document, &cases, &codes).unwrap_err(),
        "weakened family projection"
    );
}

#[test]
fn adversary_rejects_contradictory_duplicate_visibility_row() {
    let (document, cases, codes) = inputs();
    let expected = "| ready | visible | candidate_or_retained_key |";
    assert_eq!(document.lines().filter(|line| *line == expected).count(), 1);
    let contradicted = document.replace(
        expected,
        &format!("{expected}\n| ready | omitted | not_granted |"),
    );
    assert!(
        check(&contradicted, &cases, &codes).is_err(),
        "document checker accepted contradictory visibility/refusal rows for the same case"
    );
}

fn authored_table_blocks(document: &str) -> Vec<&str> {
    let tables: Vec<_> = document
        .split("\n\n")
        .filter_map(|block| {
            if block.starts_with('|') {
                Some(block)
            } else {
                block.find("\n| ").map(|start| &block[start + 1..])
            }
        })
        .collect();
    assert_eq!(
        tables.len(),
        4,
        "enumerate every authored correspondence table"
    );
    tables
}

#[test]
fn every_authored_table_rejects_duplicate_or_contradictory_rows() {
    let (document, cases, codes) = inputs();
    check(&document, &cases, &codes).unwrap();
    for table in authored_table_blocks(&document) {
        for row in table.lines().skip(2) {
            let duplicate = document.replace(row, &format!("{row}\n{row}"));
            assert!(
                check(&duplicate, &cases, &codes).is_err(),
                "duplicate accepted: {row}"
            );
            let mut cells: Vec<_> = row.trim_matches('|').split('|').map(str::trim).collect();
            *cells.last_mut().unwrap() = "contradiction";
            let changed = format!("| {} |", cells.join(" | "));
            let contradicted = document.replace(row, &format!("{row}\n{changed}"));
            assert!(
                check(&contradicted, &cases, &codes).is_err(),
                "contradiction accepted: {row}"
            );
        }
    }
}

#[test]
fn every_authored_table_rejects_missing_unknown_rows_and_duplicate_tables() {
    let (document, cases, codes) = inputs();
    for table in authored_table_blocks(&document) {
        let row = table.lines().nth(2).unwrap();
        let missing = document.replace(&format!("{row}\n"), "");
        assert!(
            check(&missing, &cases, &codes).is_err(),
            "missing row accepted: {row}"
        );
        let mut cells: Vec<_> = row.trim_matches('|').split('|').map(str::trim).collect();
        cells[0] = "unrecognized-case";
        let unknown = document.replace(row, &format!("{row}\n| {} |", cells.join(" | ")));
        assert!(
            check(&unknown, &cases, &codes).is_err(),
            "unknown row accepted: {row}"
        );
        let repeated = format!("{document}\n\n{table}\n");
        assert!(
            check(&repeated, &cases, &codes).is_err(),
            "repeated table accepted: {row}"
        );
        let separate_unknown =
            format!("{document}\n\n| Unknown table |\n|---|\n| contradictory |\n");
        assert!(
            check(&separate_unknown, &cases, &codes).is_err(),
            "unknown table accepted"
        );
    }
}

#[test]
fn complete_unique_tables_remain_valid_after_row_reordering() {
    let (document, cases, codes) = inputs();
    check(&document, &cases, &codes).unwrap();
    for table in authored_table_blocks(&document) {
        let lines: Vec<_> = table.lines().collect();
        let reordered = lines[..2]
            .iter()
            .copied()
            .chain(lines[2..].iter().rev().copied())
            .collect::<Vec<_>>()
            .join("\n");
        check(&document.replace(table, &reordered), &cases, &codes).unwrap();
    }
}

#[test]
fn adversary_rejects_nonvisible_and_blockquoted_correspondence_tables() {
    let (document, cases, codes) = inputs();
    for table in authored_table_blocks(&document) {
        let hidden = document.replace(table, &format!("```text\n{table}\n```"));
        assert!(
            check(&hidden, &cases, &codes).is_err(),
            "required correspondence table accepted only inside a fenced code block"
        );
        let quoted = table
            .lines()
            .map(|line| format!("> {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let repeated = format!("{document}\n\n{quoted}\n");
        assert!(
            check(&repeated, &cases, &codes).is_err(),
            "duplicate correspondence table inside a blockquote was ignored"
        );
    }
}

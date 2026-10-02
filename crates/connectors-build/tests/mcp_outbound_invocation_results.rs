//! Authored mapping checks only: no MCP runtime, wire decoder or peer is executed.
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::PathBuf, process::Command};

const CLIENT: &str = "adapters/mcp/contracts/client/v1alpha1";
const EVIDENCE: &str = "adapters/mcp/contracts/protocol/v1alpha1/evidence/20260912";

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cases {
    format: String,
    limits: Vec<String>,
    cases: Vec<Case>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    revision: String,
    family: String,
    obligation: String,
    tool_annotations: Value,
    request_valid: bool,
    request: String,
    request_limit: usize,
    sent: bool,
    response: String,
    eof: bool,
    result_limit: usize,
    citations: Vec<String>,
    expected: Value,
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}
fn inputs() -> (String, Cases) {
    (
        read(&format!("{CLIENT}/invocation.md")),
        serde_json::from_str(&read(&format!("{CLIENT}/invocation-cases.json"))).unwrap(),
    )
}
fn model_members(name: &str, key: &str) -> BTreeSet<String> {
    let model: Value = serde_yaml_ng::from_str(&read("ess/domains/service_wire.yaml")).unwrap();
    let ty = model["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap();
    ty[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            if key == "fields" {
                v["name"].as_str().unwrap()
            } else {
                v.as_str().unwrap()
            }
            .to_owned()
        })
        .collect()
}
fn selected(prefix: &str) -> BTreeSet<String> {
    read("adapters/mcp/contracts/protocol/v1alpha1/selection.md")
        .lines()
        .filter_map(|line| {
            let cols: Vec<_> = line.split('|').map(str::trim).collect();
            if cols.len() < 6 || !matches!(cols[2], "outbound" | "both") || cols[4] != "supported" {
                return None;
            }
            cols[1]
                .trim_matches('`')
                .strip_prefix(prefix)
                .map(str::to_owned)
        })
        .collect()
}
// Small decision table over authored observations, deliberately not a production decoder.
// Cases exercise precedence, octet counting, revision distinction and literal preservation.
fn observation(c: &Case) -> Value {
    let prefix = &c.response.as_bytes()[..c.response.len().min(c.result_limit)];
    let hex = prefix
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let mut result = json!({"outcome":"", "code":null, "response_bytes":c.response.len(), "retained_hex":hex,
        "complete":false, "effect":if c.sent {"unknown"} else {"not_dispatched"}, "automatic_redispatches":0,
        "local_write_authority":false, "preserved":null});
    let (outcome, code) = if !c.request_valid {
        ("caller-input", Some("invalid_input"))
    } else if c.request.len() > c.request_limit {
        ("request-bound", Some("invalid_input"))
    } else if c.response.len() > c.result_limit {
        ("result-bound", Some("capacity"))
    } else if !c.eof {
        ("unobserved", Some("outcome_unknown"))
    } else {
        classify_response(c, &mut result)
    };
    result["outcome"] = json!(outcome);
    result["code"] = json!(code);
    result
}
fn classify_response<'a>(c: &Case, out: &mut Value) -> (&'a str, Option<&'a str>) {
    let Ok(v) = serde_json::from_str::<Value>(&c.response) else {
        return ("malformed-peer", Some("upstream_protocol"));
    };
    if v["jsonrpc"] != "2.0"
        || v["id"] != 1
        || v.get("result").is_some() == v.get("error").is_some()
    {
        return ("malformed-peer", Some("upstream_protocol"));
    }
    if let Some(e) = v.get("error") {
        if e["code"].as_i64().is_none() || !e["message"].is_string() {
            return ("malformed-peer", Some("upstream_protocol"));
        }
        out["complete"] = json!(true);
        out["preserved"] = e.clone();
        return (
            "peer-protocol-error",
            Some(match e["code"].as_i64().unwrap() {
                -32020 if c.revision == "2026-07-28" => "internal",
                -32021 | -32022 if c.revision == "2026-07-28" => "unsupported",
                _ => "upstream_protocol",
            }),
        );
    }
    let r = &v["result"];
    if !r.is_object() {
        return ("malformed-peer", Some("upstream_protocol"));
    }
    if c.revision == "2026-07-28" {
        match r["resultType"].as_str() {
            Some("complete") => {}
            Some(_) => {
                out["preserved"] = r.clone();
                return ("unselected-result", Some("unsupported"));
            }
            None => return ("malformed-peer", Some("upstream_protocol")),
        }
    }
    let field = match c.family.as_str() {
        "tools" => "content",
        "resources" => "contents",
        "prompts" => "messages",
        _ => unreachable!(),
    };
    let Some(items) = r[field].as_array() else {
        return ("malformed-peer", Some("upstream_protocol"));
    };
    if c.revision == "2026-07-28"
        && c.family == "resources"
        && (r["ttlMs"].as_f64().is_none_or(|ttl| ttl < 0.0)
            || !matches!(r["cacheScope"].as_str(), Some("public" | "private")))
    {
        return ("malformed-peer", Some("upstream_protocol"));
    }
    if c.family == "tools"
        && (r.get("isError").is_some_and(|b| !b.is_boolean())
            || (c.revision == "2025-11-25"
                && r.get("structuredContent").is_some_and(|s| !s.is_object())))
    {
        return ("malformed-peer", Some("upstream_protocol"));
    }
    // Known content shapes are obligations for the future real decoder. This guard only
    // distinguishes a named unknown discriminator; it does not validate the MCP schema.
    let unknown = items.iter().any(|item| {
        let block = if c.family == "prompts" {
            &item["content"]
        } else {
            item
        };
        c.family != "resources"
            && block["type"].as_str().is_some_and(|kind| {
                !matches!(
                    kind,
                    "text" | "image" | "audio" | "resource" | "resource_link"
                )
            })
    });
    out["preserved"] = r.clone();
    if unknown {
        return ("unknown-content", Some("unsupported"));
    }
    out["complete"] = json!(true);
    if c.family == "tools" && r["isError"] == true {
        ("provider-business-error", None)
    } else {
        ("result", None)
    }
}
fn validate(doc: &str, cases: &Cases) -> Result<(), String> {
    if cases.format != "mcp-invocation-document-cases/1" {
        return Err("format".into());
    }
    let actual_limits: BTreeSet<_> = cases.limits.iter().cloned().collect();
    if actual_limits.len() != cases.limits.len()
        || actual_limits != model_members("connectors.service_wire.OperationLimits", "fields")
    {
        return Err("limit vocabulary".into());
    }
    let revisions = selected("revision:");
    let families = selected("capability:server/");
    let codes = model_members("connectors.service_wire.ErrorCode", "variants");
    let mut ids = BTreeSet::new();
    let mut coverage = BTreeSet::new();
    let manifest: Value = serde_json::from_str(&read(&format!(
        "{EVIDENCE}/specification-source-hashes.json"
    )))
    .unwrap();
    for c in &cases.cases {
        if !ids.insert(c.id.clone()) {
            return Err(format!("duplicate {}", c.id));
        }
        if !revisions.contains(&c.revision) || !families.contains(&c.family) {
            return Err(format!("unselected {}", c.id));
        }
        if c.sent != (c.request_valid && c.request.len() <= c.request_limit)
            || (!c.sent && (!c.response.is_empty() || c.eof))
        {
            return Err(format!("dispatch facts {}", c.id));
        }
        if c.citations.is_empty() {
            return Err(format!("uncited {}", c.id));
        }
        for citation in &c.citations {
            let (file, line) = citation.rsplit_once(':').ok_or("citation shape")?;
            let entry = manifest
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["file"] == file && v["revision"] == c.revision)
                .ok_or("citation revision/file")?;
            let output = Command::new("gzip")
                .arg("-cd")
                .arg(
                    root()
                        .join(EVIDENCE)
                        .join(entry["archive"].as_str().unwrap()),
                )
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success()
                || line
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
        if c.expected["code"]
            .as_str()
            .is_some_and(|code| !codes.contains(code))
        {
            return Err(format!("error vocabulary {}", c.id));
        }
        if c.expected != observation(c) {
            return Err(format!(
                "contradictory mapping {}: expected {}, derived {}",
                c.id,
                c.expected,
                observation(c)
            ));
        }
        let outcome = c.expected["outcome"].as_str().ok_or("outcome")?;
        let matches_obligation = match c.obligation.as_str() {
            "tools" | "resources" | "prompts" => c.family == c.obligation && outcome == "result",
            "structured" => {
                outcome == "result" && c.expected["preserved"].get("structuredContent").is_some()
            }
            "business" => outcome == "provider-business-error",
            "protocol" => outcome == "peer-protocol-error",
            "caller-input" => outcome == "caller-input",
            "malformed-peer" => outcome == "malformed-peer",
            "partial" => outcome == "unobserved",
            "bound" => outcome == "result-bound",
            "request-bound" => outcome == "request-bound",
            "annotations" => outcome == "result" && c.tool_annotations.is_object(),
            "unknown-content" => outcome == "unknown-content",
            "unselected-result" => outcome == "unselected-result",
            _ => false,
        };
        if !matches_obligation {
            return Err(format!("obligation facts {}", c.id));
        }
        let row = format!(
            "| `{}` | {} | {} | {} | `{}` |",
            c.id,
            c.revision,
            c.family,
            c.obligation,
            c.expected["outcome"].as_str().unwrap()
        );
        if doc.lines().filter(|line| *line == row).count() != 1 {
            return Err(format!("document mapping {}", c.id));
        }
        coverage.insert((c.revision.clone(), c.obligation.clone()));
    }
    let doc_ids: BTreeSet<_> = doc
        .lines()
        .filter(|l| l.starts_with("| `inv."))
        .filter_map(|l| l.split('`').nth(1).map(str::to_owned))
        .collect();
    if ids != doc_ids {
        return Err("document/case bijection".into());
    }
    for revision in &revisions {
        for obligation in [
            "tools",
            "resources",
            "prompts",
            "structured",
            "business",
            "protocol",
            "caller-input",
            "malformed-peer",
            "partial",
            "bound",
            "request-bound",
            "annotations",
            "unknown-content",
        ] {
            if !coverage.contains(&(revision.clone(), obligation.to_owned())) {
                return Err(format!("missing {revision}/{obligation}"));
            }
        }
        for family in ["tools", "resources", "prompts"] {
            if !cases.cases.iter().any(|c| {
                c.revision == *revision && c.family == family && c.expected["outcome"] == "result"
            }) {
                return Err(format!("missing result {revision}/{family}"));
            }
        }
    }
    Ok(())
}
#[test]
fn invocation_document_and_cases_agree_with_selected_sources() {
    let (doc, cases) = inputs();
    assert_eq!(validate(&doc, &cases), Ok(()));
}
#[test]
fn contradictory_effect_retry_authority_and_bytes_are_rejected() {
    let (doc, cases) = inputs();
    for (key, value) in [
        ("effect", json!("none")),
        ("automatic_redispatches", json!(1)),
        ("local_write_authority", json!(true)),
        ("response_bytes", json!(0)),
        ("retained_hex", json!("")),
        ("complete", json!(false)),
    ] {
        let mut bad = cases.clone();
        let c = bad
            .cases
            .iter_mut()
            .find(|c| c.expected["outcome"] == "result")
            .unwrap();
        c.expected[key] = value;
        assert!(validate(&doc, &bad).is_err(), "accepted {key}");
    }
}
#[test]
fn missing_duplicate_and_document_only_mappings_are_rejected() {
    let (doc, cases) = inputs();
    let mut missing = cases.clone();
    missing.cases.retain(|c| c.obligation != "prompts");
    let missing_doc = doc
        .lines()
        .filter(|l| !l.contains(" | prompts | prompts |"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        validate(&missing_doc, &missing)
            .unwrap_err()
            .contains("missing")
    );
    let mut duplicate = cases.clone();
    duplicate.cases.push(duplicate.cases[0].clone());
    assert!(
        validate(&doc, &duplicate)
            .unwrap_err()
            .contains("duplicate")
    );
    assert!(
        validate(&format!("{doc}\n| `inv.fake` |"), &cases)
            .unwrap_err()
            .contains("bijection")
    );
}
#[test]
fn revision_and_error_and_limit_mutations_are_rejected() {
    let (doc, cases) = inputs();
    let mut wrong = cases.clone();
    wrong.cases[0].revision = "2025-03-26".into();
    assert!(validate(&doc, &wrong).unwrap_err().contains("unselected"));
    let mut wrong = cases.clone();
    wrong.cases[0].expected["code"] = json!("made_up");
    assert!(validate(&doc, &wrong).unwrap_err().contains("vocabulary"));
    let mut wrong = cases.clone();
    wrong.limits.push("decoded_characters".into());
    assert!(validate(&doc, &wrong).unwrap_err().contains("limit"));
}

#[test]
fn modern_resource_cache_metadata_is_required_by_the_pinned_schema() {
    let (_, cases) = inputs();
    let mut c = cases
        .cases
        .iter()
        .find(|c| c.revision == "2026-07-28" && c.obligation == "resources")
        .unwrap()
        .clone();
    let mut response: Value = serde_json::from_str(&c.response).unwrap();
    response["result"].as_object_mut().unwrap().remove("ttlMs");
    response["result"]
        .as_object_mut()
        .unwrap()
        .remove("cacheScope");
    c.response = serde_json::to_string(&response).unwrap();
    c.result_limit = c.response.len();
    assert_eq!(observation(&c)["outcome"], "malformed-peer");
}

#[test]
fn partial_business_and_structured_observations_cannot_be_relabelled() {
    let (doc, cases) = inputs();
    for (obligation, key, value) in [
        ("partial", "complete", json!(true)),
        ("partial", "code", json!("invalid_input")),
        ("business", "outcome", json!("peer-protocol-error")),
        ("structured", "preserved", json!(null)),
        ("bound", "complete", json!(true)),
    ] {
        let mut bad = cases.clone();
        let c = bad
            .cases
            .iter_mut()
            .find(|c| c.obligation == obligation)
            .unwrap();
        c.expected[key] = value;
        assert!(
            validate(&doc, &bad).unwrap_err().contains("contradictory"),
            "accepted {obligation}/{key}"
        );
    }
    // Both document and case can agree on a false claim; obligation checks must
    // still reject relabelling an ordinary result as structured-output coverage.
    let mut bad = cases.clone();
    let c = bad
        .cases
        .iter_mut()
        .find(|c| c.obligation == "tools")
        .unwrap();
    let changed_doc = doc.replace(
        &format!("{} | tools | tools |", c.revision),
        &format!("{} | tools | structured |", c.revision),
    );
    c.obligation = "structured".into();
    assert!(
        validate(&changed_doc, &bad)
            .unwrap_err()
            .contains("obligation facts")
    );
}

// Adversarial probes exercise copies of authored observations only; no wire run.
#[test]
fn review_exact_octet_ceilings_and_terminal_loss_preserve_uncertainty() {
    let (_, cases) = inputs();
    for source in cases.cases.iter().filter(|c| c.obligation == "tools") {
        let mut c = source.clone();
        c.request = c.request.replace("selected", "caf\u{e9}");
        c.request_limit = c.request.len();
        c.result_limit = c.response.len();
        assert_eq!(observation(&c)["outcome"], "result");
        assert_eq!(observation(&c)["effect"], "unknown");
        c.eof = false;
        assert_eq!(observation(&c)["code"], "outcome_unknown");
        assert_eq!(observation(&c)["complete"], false);
        c.result_limit -= 1;
        assert_eq!(observation(&c)["code"], "capacity");
        assert_eq!(observation(&c)["effect"], "unknown");
        assert_eq!(observation(&c)["automatic_redispatches"], 0);
        c.request_limit -= 1;
        c.sent = false;
        c.response.clear();
        assert_eq!(observation(&c)["outcome"], "request-bound");
        assert_eq!(observation(&c)["effect"], "not_dispatched");
    }
}

#[test]
fn review_non_string_and_unselected_result_types_across_selected_families() {
    let (_, cases) = inputs();
    for source in cases
        .cases
        .iter()
        .filter(|c| c.revision == "2026-07-28" && c.obligation == c.family)
    {
        for discriminator in [json!(null), json!(7), json!({}), json!("future")] {
            let mut c = source.clone();
            let mut response: Value = serde_json::from_str(&c.response).unwrap();
            response["result"]["resultType"] = discriminator.clone();
            c.response = serde_json::to_string(&response).unwrap();
            c.result_limit = c.response.len();
            let actual = observation(&c);
            assert_eq!(
                actual["code"],
                if discriminator.is_string() {
                    "unsupported"
                } else {
                    "upstream_protocol"
                }
            );
            assert_eq!(actual["complete"], false);
            assert_eq!(actual["effect"], "unknown");
            assert_eq!(actual["automatic_redispatches"], 0);
        }
    }
}

#[test]
fn review_peer_errors_and_malformed_answers_never_establish_non_execution() {
    let (_, cases) = inputs();
    for source in cases.cases.iter().filter(|c| c.obligation == "protocol") {
        let mut c = source.clone();
        for response in [
            "{broken",
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32099,"message":"future","data":null}}"#,
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32099,"message":"future"},"result":{}}"#,
        ] {
            c.response = response.into();
            c.result_limit = c.response.len();
            let actual = observation(&c);
            assert_eq!(actual["code"], "upstream_protocol");
            assert_eq!(actual["effect"], "unknown");
            assert_eq!(actual["automatic_redispatches"], 0);
            assert_eq!(actual["local_write_authority"], false);
            if actual["outcome"] == "peer-protocol-error" {
                assert_eq!(
                    actual["preserved"],
                    json!({"code":-32099,"message":"future","data":null})
                );
                assert_eq!(actual["complete"], true);
            } else {
                assert_eq!(actual["outcome"], "malformed-peer");
                assert_eq!(actual["complete"], false);
            }
        }
    }
}

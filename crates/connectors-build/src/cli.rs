//! Generate the selected CLI contract fixture with the exact pinned ESS tool.

use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeSet, fs, path::Path, process::Command};

use super::Result;

pub fn run(root: &Path, ess: &Path, check: bool) -> Result<()> {
    connectors_spec::toolchain::check(ess)?;
    let mut command = Command::new(ess);
    command.current_dir(root).args([
        "generate",
        "cli",
        "--path",
        "ess",
        "--binding",
        "apps/connectors/spec/cli.yaml",
        "--out",
        "apps/connectors-cli-contract",
    ]);
    if check {
        command.arg("--check");
    }
    let status = command.status()?;
    if !status.success() {
        return Err(format!("ESS CLI generation failed: {status}").into());
    }
    validate_values(root, ess)?;
    println!(
        "CLI contract fixture {}; custody and lifecycle runtime conformance remain obligations",
        if check { "matches" } else { "generated" }
    );
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Values {
    format: String,
    cases: Vec<ValueCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ValueCase {
    id: String,
    #[serde(rename = "type")]
    type_name: String,
    valid: bool,
    value: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CachedExpectations {
    cases: Vec<CachedExpectation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CachedExpectation {
    id: String,
    fixture_id: String,
    expected_source: String,
    expected_stale: bool,
}

fn acquisition_is_consistent(value: &Value) -> bool {
    let failed = value["state"] == "failed";
    let completed = value["state"] == "completed";
    matches!(
        value["state"].as_str(),
        Some("pending" | "completed" | "failed")
    ) && (value["reason"].as_str().is_some() == failed)
        && (value["connection"].as_str().is_some() == completed)
}

fn list_freshness_is_consistent(value: &Value) -> bool {
    matches!(value["source"].as_str(), Some("authority" | "cached"))
        && (value["stale"] == true
            || (value["source"] == "authority"
                && matches!((value["observed_at_ms"].as_i64(), value["valid_until_ms"].as_i64()),
                    (Some(observed), Some(deadline)) if deadline > observed)))
}

fn cached_values_are_labeled(value: &Value) -> bool {
    match value {
        Value::Object(fields) => {
            (fields.get("source") != Some(&Value::String("cached".into()))
                || fields.get("stale") == Some(&Value::Bool(true)))
                && fields.values().all(cached_values_are_labeled)
        }
        Value::Array(items) => items.iter().all(cached_values_are_labeled),
        _ => true,
    }
}

fn validate_values(root: &Path, ess: &Path) -> Result<()> {
    let base = root.join(".local/tmp");
    fs::create_dir_all(&base)?;
    let temp = tempfile::Builder::new()
        .prefix("cli-values-")
        .tempdir_in(base)?;
    super::run(
        Command::new(ess)
            .current_dir(root)
            .args(["generate", "--path", "ess", "--kind", "schema", "--out"])
            .arg(temp.path().join("schemas")),
    )?;
    let fixture_path = root.join("contracts/cli/v1alpha1/fixtures/values.json");
    let fixtures: Values = serde_json::from_slice(&fs::read(fixture_path)?)?;
    if fixtures.format != "connectors-cli-values/1" || fixtures.cases.is_empty() {
        return Err("invalid or empty CLI fixture set".into());
    }
    let mut ids = BTreeSet::new();
    let mut acquisitions = 0;
    let mut lists = 0;
    for case in &fixtures.cases {
        if case.id.is_empty()
            || !ids.insert(&case.id)
            || !case.type_name.starts_with("connectors.cli.")
            || !case
                .type_name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"._".contains(&c))
        {
            return Err("duplicate CLI fixture id or invalid model type selector".into());
        }
        let schema: Value = serde_json::from_slice(&fs::read(
            temp.path()
                .join("schemas/schema/types")
                .join(format!("{}.schema.json", case.type_name)),
        )?)?;
        let validator = jsonschema::validator_for(&schema)?;
        if validator.is_valid(&case.value) != case.valid {
            let reasons = validator
                .iter_errors(&case.value)
                .map(|error| format!("{}: {error}", error.instance_path))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(format!(
                "CLI structural fixture {} disagrees with its selected model (expected valid={}): {}",
                case.id, case.valid, if reasons.is_empty() { "no validation errors" } else { &reasons }
            )
            .into());
        }
        if case.valid && !cached_values_are_labeled(&case.value) {
            return Err(format!(
                "CLI fixture {} presents cached facts without a stale label",
                case.id
            )
            .into());
        }
        if !case.valid {
            continue;
        }
        let acquisition = match case.type_name.as_str() {
            "connectors.cli.AcquisitionObservation" => Some(&case.value),
            "connectors.cli.ConnectionStatusResult" => {
                case.value.get("acquisition").filter(|v| !v.is_null())
            }
            _ => None,
        };
        if let Some(value) = acquisition {
            acquisitions += 1;
            if !acquisition_is_consistent(value) {
                return Err(format!(
                    "CLI fixture {} has inconsistent public acquisition fields",
                    case.id
                )
                .into());
            }
        }
        if case.type_name == "connectors.cli.ConnectionListResult" {
            lists += 1;
            if !list_freshness_is_consistent(&case.value) {
                return Err(format!(
                    "CLI fixture {} makes an unsupported page freshness claim",
                    case.id
                )
                .into());
            }
        }
    }
    if acquisitions == 0 || lists == 0 {
        return Err("CLI acquisition or connection-list fixture selection is empty".into());
    }
    let expectations: CachedExpectations = serde_json::from_slice(&fs::read(
        root.join("contracts/cli/v1alpha1/fixtures/cached-expectations.json"),
    )?)?;
    let cases = &expectations.cases;
    if cases.is_empty() {
        return Err("empty cached CLI expectation set".into());
    }
    let mut expectation_ids = BTreeSet::new();
    let mut fixture_ids = BTreeSet::new();
    for case in cases {
        let id = &case.fixture_id;
        if case.id.is_empty()
            || !expectation_ids.insert(&case.id)
            || !fixture_ids.insert(id)
            || case.expected_source != "cached"
            || !case.expected_stale
        {
            return Err("invalid or duplicate cached CLI expectation".into());
        }
        let fixture = fixtures
            .cases
            .iter()
            .find(|fixture| fixture.id == *id)
            .ok_or("cached expectation selects no fixture")?;
        if !fixture.valid
            || fixture.value["source"] != case.expected_source
            || fixture.value["stale"] != case.expected_stale
        {
            return Err(format!("cached CLI expectation for {id} failed").into());
        }
    }
    println!(
        "CLI values: {} structural, {} cached expectations, {acquisitions} acquisition and {lists} page consistency cases passed; runtime lifecycle/custody traces remain obligations",
        fixtures.cases.len(),
        cases.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn public_acquisition_examples_cannot_expose_internal_states_or_wrong_terminal_fields() {
        for value in [
            json!({"state":"pending"}),
            json!({"state":"completed","connection":"fixture-connection"}),
            json!({"state":"failed","reason":"expired"}),
        ] {
            assert!(acquisition_is_consistent(&value));
        }
        for value in [
            json!({"state":"completing"}),
            json!({"state":"expired","reason":"expired"}),
            json!({"state":"pending","connection":"fixture-connection"}),
            json!({"state":"pending","reason":"expired"}),
            json!({"state":"completed"}),
            json!({"state":"failed"}),
            json!({"state":"failed","reason":"expired","connection":"fixture-connection"}),
        ] {
            assert!(!acquisition_is_consistent(&value), "{value}");
        }
    }

    #[test]
    fn cache_labels_and_page_deadlines_cannot_assert_unobserved_freshness() {
        for value in [
            json!({"source":"cached","stale":true,"observed_at_ms":10}),
            json!({"source":"authority","stale":false,"observed_at_ms":10,"valid_until_ms":11}),
            json!({"source":"authority","stale":true,"observed_at_ms":10,"valid_until_ms":9}),
        ] {
            assert!(list_freshness_is_consistent(&value));
        }
        for value in [
            json!({"source":"cached","stale":false,"observed_at_ms":10,"valid_until_ms":11}),
            json!({"source":"configuration","stale":true,"observed_at_ms":10}),
            json!({"source":"authority","stale":false,"observed_at_ms":10}),
            json!({"source":"authority","stale":false,"observed_at_ms":10,"valid_until_ms":10}),
            json!({"source":"authority","stale":false,"observed_at_ms":10,"valid_until_ms":9}),
        ] {
            assert!(!list_freshness_is_consistent(&value), "{value}");
        }
        assert!(!cached_values_are_labeled(
            &json!({"nested":[{"source":"cached","stale":false}]})
        ));
        assert!(cached_values_are_labeled(
            &json!({"nested":[{"source":"cached","stale":true}]})
        ));
    }
}

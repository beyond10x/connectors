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
    let schemas = generate_schemas(root, ess)?;
    let types = schemas.path().join("schemas/schema/types");
    validate_values(root, &types)?;
    validate_wire_vectors(root, &types)?;
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

/// The JSON Schema of every ESS type, generated into a temporary directory that is removed
/// when the returned handle drops; no generated schema tree is committed.
fn generate_schemas(root: &Path, ess: &Path) -> Result<tempfile::TempDir> {
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
    Ok(temp)
}

fn type_schema(types: &Path, type_name: &str, prefix: &str) -> Result<Value> {
    if !type_name.starts_with(prefix)
        || !type_name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._".contains(&c))
    {
        return Err(format!("invalid model type selector {type_name:?}").into());
    }
    Ok(serde_json::from_slice(&fs::read(
        types.join(format!("{type_name}.schema.json")),
    )?)?)
}

fn validate_values(root: &Path, types: &Path) -> Result<()> {
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
        let schema = type_schema(types, &case.type_name, "connectors.cli.")?;
        let validator = jsonschema::validator_for(&schema)?;
        if validator.is_valid(&case.value) != case.valid {
            let reasons = validator
                .iter_errors(&case.value)
                .map(|error| format!("{}: {error}", error.instance_path()))
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

/// The service wire codec vectors (`task:v1alpha2-invoke-wire-codec`) checked against the
/// ESS-generated schema of their declared type.
const WIRE_VECTORS: &str = "crates/connectors-core/tests/vectors/v1alpha2";

#[derive(Debug, Default, PartialEq, Eq)]
struct WireCounts {
    schema_valid: usize,
    schema_invalid: usize,
    raw: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireVectors {
    format: String,
    #[serde(rename = "type")]
    type_name: String,
    cases: Vec<WireCase>,
}

/// One codec vector. `code` and `request_id` are the codec's expectations and are read by
/// `connectors-core`'s own vector tests, not by the schema check.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCase {
    id: String,
    #[serde(default)]
    schema_valid: Option<bool>,
    expect: String,
    #[serde(default)]
    #[allow(dead_code)]
    code: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    request_id: Option<Value>,
    #[serde(default)]
    value: Option<Value>,
    #[serde(default)]
    raw: Option<String>,
}

/// Every `*.json` file under [`WIRE_VECTORS`] is checked; any other entry is refused, so a
/// vector cannot be added where the check does not read it.
fn validate_wire_vectors(root: &Path, types: &Path) -> Result<()> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root.join(WIRE_VECTORS))? {
        let path = entry?.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("json") {
            return Err(format!("unexpected entry in the wire vectors: {}", path.display()).into());
        }
        files.push(path);
    }
    if files.is_empty() {
        return Err("empty v1alpha2 wire vector selection".into());
    }
    files.sort();
    let mut total = WireCounts::default();
    for path in &files {
        let counts = check_wire_vectors(&fs::read(path)?, types)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        total.schema_valid += counts.schema_valid;
        total.schema_invalid += counts.schema_invalid;
        total.raw += counts.raw;
    }
    println!(
        "v1alpha2 wire vectors: {} files, {} schema-valid and {} schema-invalid values agree with the generated schema; {} raw-byte refusals excluded by rule",
        files.len(),
        total.schema_valid,
        total.schema_invalid,
        total.raw
    );
    Ok(())
}

/// Checks one vector file against the generated schema of its declared type.
///
/// - A `value` case states `schema_valid`, and the generated schema must agree with it;
///   a case the schema refuses must expect refusal.
/// - Raw-bytes exclusion: a `raw` case (duplicate members, malformed JSON) is not one JSON
///   value a schema can judge, so it carries no `schema_valid` and must expect refusal.
fn check_wire_vectors(bytes: &[u8], types: &Path) -> Result<WireCounts> {
    let vectors: WireVectors = serde_json::from_slice(bytes)?;
    if vectors.format != "connectors-service-wire-vectors/1" || vectors.cases.is_empty() {
        return Err("invalid or empty service wire vector set".into());
    }
    let schema = type_schema(types, &vectors.type_name, "connectors.service_wire.")?;
    let validator = jsonschema::validator_for(&schema)?;
    let mut ids = BTreeSet::new();
    let mut counts = WireCounts::default();
    for case in &vectors.cases {
        let id = &case.id;
        if id.is_empty() || !ids.insert(id) {
            return Err(format!("empty or duplicate wire vector id {id:?}").into());
        }
        if !matches!(case.expect.as_str(), "round_trip" | "refuse") {
            return Err(format!("wire vector {id} has unknown expectation").into());
        }
        let (value, schema_valid) = match (&case.value, &case.raw, case.schema_valid) {
            (Some(value), None, Some(schema_valid)) => (value, schema_valid),
            (None, Some(_), None) if case.expect == "refuse" => {
                counts.raw += 1;
                continue;
            }
            (None, Some(_), _) => {
                return Err(format!(
                    "wire vector {id}: raw bytes are excluded from the schema check only as a refusal without schema_valid"
                )
                .into());
            }
            _ => {
                return Err(format!(
                    "wire vector {id}: exactly one of value/raw, and schema_valid with every value"
                )
                .into());
            }
        };
        if validator.is_valid(value) != schema_valid {
            let reasons = validator
                .iter_errors(value)
                .map(|error| format!("{}: {error}", error.instance_path()))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(format!(
                "wire vector {id} disagrees with the generated {} schema (expected schema_valid={schema_valid}): {}",
                vectors.type_name,
                if reasons.is_empty() { "no validation errors" } else { &reasons }
            )
            .into());
        }
        if schema_valid {
            counts.schema_valid += 1;
        } else if case.expect == "refuse" {
            counts.schema_invalid += 1;
        } else {
            return Err(
                format!("wire vector {id} is schema-invalid but expected to round-trip").into(),
            );
        }
    }
    if counts.schema_valid == 0 || counts.schema_invalid == 0 {
        return Err(format!(
            "{}: the vector set needs both schema-valid and schema-invalid values",
            vectors.type_name
        )
        .into());
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request_vectors(root: &Path) -> Value {
        serde_json::from_slice(
            &fs::read(root.join(WIRE_VECTORS).join("invoke-request.json")).unwrap(),
        )
        .unwrap()
    }

    fn case_mut<'a>(vectors: &'a mut Value, id: &str) -> &'a mut Value {
        vectors["cases"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|case| case["id"] == id)
            .unwrap_or_else(|| panic!("no vector {id}"))
    }

    fn refused(vectors: &Value, types: &Path) -> String {
        check_wire_vectors(&serde_json::to_vec(vectors).unwrap(), types)
            .expect_err("mutated vector set was accepted")
            .to_string()
    }

    /// `task:v1alpha2-invoke-wire-spec`: every v1alpha2 vector agrees with the generated
    /// schema, and a vector that disagrees with it is refused by name.
    #[test]
    fn wire_vectors_agree_with_the_generated_schema_and_a_mutated_vector_is_refused() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let ess = connectors_spec::toolchain::resolve(None).unwrap();
        let schemas = generate_schemas(&root, &ess).unwrap();
        let types = schemas.path().join("schemas/schema/types");

        validate_wire_vectors(&root, &types).unwrap();
        let counts = check_wire_vectors(
            &fs::read(root.join(WIRE_VECTORS).join("invoke-request.json")).unwrap(),
            &types,
        )
        .unwrap();
        assert!(counts.schema_valid > 0 && counts.schema_invalid > 0 && counts.raw > 0);

        // A §4 member added to an accepted request leaves the closed schema.
        let mut vectors = request_vectors(&root);
        case_mut(&mut vectors, "request-object-input")["value"]["connection"] = json!("c1");
        assert!(refused(&vectors, &types).contains("request-object-input"));

        // An expectation flipped either way is refused.
        let mut vectors = request_vectors(&root);
        case_mut(&mut vectors, "request-missing-input")["schema_valid"] = json!(true);
        assert!(refused(&vectors, &types).contains("request-missing-input"));
        let mut vectors = request_vectors(&root);
        case_mut(&mut vectors, "request-null-input")["schema_valid"] = json!(false);
        assert!(refused(&vectors, &types).contains("request-null-input"));

        // Schema-invalid is a refusal; it cannot be expected to round-trip.
        let mut vectors = request_vectors(&root);
        case_mut(&mut vectors, "request-missing-input")["expect"] = json!("round_trip");
        assert!(refused(&vectors, &types).contains("request-missing-input"));

        // The raw-bytes exclusion is named, refusal-only and carries no schema verdict.
        let mut vectors = request_vectors(&root);
        case_mut(&mut vectors, "request-duplicate-member")["expect"] = json!("round_trip");
        assert!(refused(&vectors, &types).contains("request-duplicate-member"));
        let mut vectors = request_vectors(&root);
        case_mut(&mut vectors, "request-duplicate-member")["schema_valid"] = json!(false);
        assert!(refused(&vectors, &types).contains("request-duplicate-member"));
        let mut vectors = request_vectors(&root);
        case_mut(&mut vectors, "request-null-input")
            .as_object_mut()
            .unwrap()
            .remove("schema_valid");
        assert!(refused(&vectors, &types).contains("request-null-input"));

        // The vector set selects a declared service wire type.
        let mut vectors = request_vectors(&root);
        vectors["type"] = json!("connectors.service_wire.Missing");
        refused(&vectors, &types);
        let mut vectors = request_vectors(&root);
        vectors["type"] = json!("connectors.cli.AcquisitionObservation");
        refused(&vectors, &types);
    }

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

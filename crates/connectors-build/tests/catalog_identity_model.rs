//! Every identity probe the catalog guides tell a reader to configure is a value of the catalog
//! adapter's own model, `connectors_catalog.identity.Probe` in `adapters/catalog/spec/ess`,
//! compiled to JSON Schema by the pinned `ess`. A documented `auth.identity` with a field or a
//! `source` the model does not name fails here; the provider's own refusal of the model's
//! invariants is `adapters/catalog/tests/runpod.rs`.
use serde_json::Value;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The `auth.identity` of every JSON example in `docs/`, whole configurations and `"auth": {…}`
/// excerpts alike, with the guide it came from.
fn documented() -> Vec<(String, Value)> {
    let mut guides: Vec<PathBuf> = std::fs::read_dir(root().join("docs"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "md"))
        .collect();
    guides.sort();
    let mut found = Vec::new();
    for guide in guides {
        let text = std::fs::read_to_string(&guide).unwrap();
        let name = guide.file_name().unwrap().to_string_lossy().into_owned();
        for block in text
            .split("```json\n")
            .skip(1)
            .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        {
            let trimmed = block.trim();
            let value: Option<Value> = serde_json::from_str(trimmed)
                .ok()
                .or_else(|| serde_json::from_str(&format!("{{{trimmed}}}")).ok());
            if let Some(identity) = value.as_ref().and_then(|v| v.pointer("/auth/identity")) {
                found.push((name.clone(), identity.clone()));
            }
        }
    }
    found
}

#[test]
fn every_documented_identity_probe_is_a_value_of_the_catalog_identity_model() {
    let ess = connectors_spec::toolchain::resolve(None).expect("pinned ess");
    let out = tempfile::tempdir().unwrap();
    let status = std::process::Command::new(ess)
        .current_dir(root())
        .args([
            "generate",
            "--path",
            "adapters/catalog/spec/ess",
            "--kind",
            "schema",
            "--out",
        ])
        .arg(out.path().join("schemas"))
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    let schema: Value = serde_json::from_slice(
        &std::fs::read(
            out.path()
                .join("schemas/schema/types/connectors_catalog.identity.Probe.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        schema["$defs"]["connectors_catalog.identity.Source"]["enum"],
        serde_json::json!(["api", "id_token", "configuration"])
    );
    let validator = jsonschema::validator_for(&schema).unwrap();
    let documented = documented();
    let sources: std::collections::BTreeSet<&str> = documented
        .iter()
        .map(|(_, identity)| identity["source"].as_str().unwrap_or("api"))
        .collect();
    assert_eq!(
        sources,
        ["api", "configuration", "id_token"].into(),
        "the guides no longer exercise every source, so this check would not cover the model"
    );
    for (guide, identity) in &documented {
        let errors: Vec<String> = validator
            .iter_errors(identity)
            .map(|e| format!("{}: {e}", e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{guide}: {identity}: {errors:?}");
    }
    assert!(
        documented
            .iter()
            .any(|(guide, identity)| guide == "catalog-runpod.md"
                && identity["source"] == "configuration"),
        "the Runpod guide's configured identity was not read"
    );
}

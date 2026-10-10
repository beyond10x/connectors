//! Proposed new file: crates/connectors-build/tests/catalog_binary_model.rs (unit U1 of wave
//! 20261010e; outside U1's paths, so left for the coordinator to place).
//!
//! Every shipped selection's binary members are a value of the catalog adapter's own model in
//! `adapters/catalog/spec/ess` (`connectors_catalog.binary.BinarySelection`, compiled to JSON
//! Schema by the pinned `ess`), and the engine's reader carries every field of them: read and
//! written back, a selection keeps its bytes. A binary body the engine answers is a value of
//! `connectors_catalog.binary.Body`.
use connectors_catalog_provider::Selection;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every operation in every shipped selection file, by file and id.
fn shipped() -> Vec<(String, Value)> {
    let providers = root().join("adapters/catalog/providers");
    let mut files: Vec<PathBuf> = std::fs::read_dir(providers)
        .unwrap()
        .flat_map(|entry| std::fs::read_dir(entry.unwrap().path()).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    let mut found = Vec::new();
    for file in files {
        let raw: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        for operation in raw["operations"].as_array().into_iter().flatten() {
            found.push((
                format!("{}: {}", file.display(), operation["id"]),
                operation.clone(),
            ));
        }
    }
    found
}

/// The JSON Schema the pinned `ess` compiles for one type of the catalog model.
fn compiled(name: &str) -> Value {
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
    serde_json::from_slice(
        &std::fs::read(
            out.path()
                .join(format!("schemas/schema/types/{name}.schema.json")),
        )
        .unwrap(),
    )
    .unwrap()
}

const MEMBERS: [&str; 4] = ["operation_id", "response", "binary", "download"];

fn projection(raw: &Value) -> Value {
    Value::Object(
        MEMBERS
            .iter()
            .filter_map(|member| Some((member.to_string(), raw.get(*member)?.clone())))
            .filter(|(_, value)| !value.is_null())
            .collect(),
    )
}

#[test]
fn every_shipped_selection_is_a_value_of_the_catalog_binary_model() {
    let schema = compiled("connectors_catalog.binary.BinarySelection");
    let invariants =
        schema["$defs"]["connectors_catalog.binary.BinarySelection"]["x-ess-invariants"]
            .to_string();
    assert!(
        invariants.contains("defined(binary.hosts)"),
        "the model no longer requires a download to reach a host: {invariants}"
    );
    let validator = jsonschema::validator_for(&schema).unwrap();
    let shipped = shipped();
    for member in ["binary", "download"] {
        assert!(
            shipped.iter().any(|(_, s)| s.get(member).is_some()),
            "no shipped selection declares `{member}`, so this check would not cover it"
        );
    }
    for (name, raw) in &shipped {
        let errors: Vec<String> = validator
            .iter_errors(&projection(raw))
            .map(|e| format!("{}: {e}", e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{name}: {errors:?}");
        let selection: Selection =
            serde_json::from_value(raw.clone()).unwrap_or_else(|error| panic!("{name}: {error}"));
        let mut written = serde_json::to_value(&selection).unwrap();
        written
            .as_object_mut()
            .unwrap()
            .retain(|key, value| !value.is_null() || raw.get(key).is_some());
        assert_eq!(&written, raw, "{name}");
    }
}

/// The compiled `Binary` bounds `max_bytes` exactly as the engine's ceiling does.
#[test]
fn the_model_bound_is_the_engine_bound() {
    let schema = compiled("connectors_catalog.binary.Binary");
    let max_bytes = &schema["$defs"]["connectors_catalog.binary.Binary"]["properties"]["max_bytes"];
    assert_eq!(
        max_bytes["maximum"],
        json!(connectors_catalog_provider::BINARY_LIMIT)
    );
    assert_eq!(max_bytes["minimum"], json!(1));
}

//! Every shipped read's walk is a value of the catalog adapter's own model in
//! `adapters/catalog/spec/ess` (`connectors_catalog.walk.ReadWalk`, compiled to JSON Schema by
//! the pinned `ess`), and the engine's reader carries every field of it: read and written back,
//! the walk is the same document (story:catalog-operation-declares-its-walk).
use connectors_catalog_provider::walk::Walk;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every walk in every shipped selection file, by file and id.
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
            if let Some(walk) = operation.get("walk") {
                found.push((
                    format!("{}: {}", file.display(), operation["id"]),
                    walk.clone(),
                ));
            }
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

#[test]
fn every_shipped_walk_is_a_value_of_the_catalog_walk_model() {
    let schema = compiled("connectors_catalog.walk.ReadWalk");
    let invariants = schema["$defs"]["connectors_catalog.walk.End"]["x-ess-invariants"].to_string();
    assert!(
        invariants
            .contains("defined(when) or defined(absent) or defined(present) or defined(empty)"),
        "the model no longer requires an end to test something: {invariants}"
    );
    let invariants =
        schema["$defs"]["connectors_catalog.walk.Walk"]["x-ess-invariants"].to_string();
    assert!(
        invariants.contains("ends.count <= 8"),
        "the model no longer bounds a walk's ends as the engine does: {invariants}"
    );
    let validator = jsonschema::validator_for(&schema).unwrap();
    let shipped = shipped();
    assert!(
        shipped.len() >= 20,
        "only {} shipped walks: {shipped:?}",
        shipped.len()
    );
    for (name, walk) in &shipped {
        let errors: Vec<String> = validator
            .iter_errors(&json!({"walk": walk}))
            .map(|e| format!("{}: {e}", e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{name}: {errors:?}");
        let read: Walk =
            serde_json::from_value(walk.clone()).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(&serde_json::to_value(&read).unwrap(), walk, "{name}");
    }
}

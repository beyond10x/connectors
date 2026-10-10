//! Every guard a shipped selection declares is a value of the catalog adapter's own model in
//! `adapters/catalog/spec/ess` (`connectors_catalog.guard.Guard`, compiled to JSON Schema by the
//! pinned `ess`), and the engine's reader carries every field of it: read and written back, it is
//! the same document, so a guard keeps its bytes and its configuration revision. A guard with a
//! member the model does not name fails here, and so does a model member the engine drops.
use connectors_catalog_provider::Guard;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every operation with a `guard` in every shipped selection file, by file and id.
fn guarded() -> Vec<(String, Value)> {
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
            if let Some(guard) = operation.get("guard") {
                found.push((
                    format!("{}: {}", file.display(), operation["id"]),
                    guard.clone(),
                ));
            }
        }
    }
    found
}

#[test]
fn every_shipped_guard_is_a_value_of_the_catalog_guard_model() {
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
                .join("schemas/schema/types/connectors_catalog.guard.Guard.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    // JSON Schema carries the model's invariants without enforcing them; the engine refuses
    // them when the selection loads (`adapters/catalog/tests/jira_transition_run.rs`).
    let invariants =
        schema["$defs"]["connectors_catalog.guard.Postflight"]["x-ess-invariants"].to_string();
    assert!(
        invariants.contains("defined(read)"),
        "the model no longer requires a check of a postflight read: {invariants}"
    );
    let validator = jsonschema::validator_for(&schema).unwrap();
    let guarded = guarded();
    for member in ["further_preflights", "postflight/read"] {
        assert!(
            guarded
                .iter()
                .any(|(_, guard)| guard.pointer(&format!("/{member}")).is_some()),
            "no shipped guard declares `{member}`, so this check would not cover it"
        );
    }
    for (name, raw) in &guarded {
        let errors: Vec<String> = validator
            .iter_errors(raw)
            .map(|e| format!("{}: {e}", e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{name}: {errors:?}");
        let guard: Guard =
            serde_json::from_value(raw.clone()).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(&serde_json::to_value(&guard).unwrap(), raw, "{name}");
    }
}

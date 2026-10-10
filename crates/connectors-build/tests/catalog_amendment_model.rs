//! Every committed `connectors-source-amendments/1` file is a value of the catalog adapter's own
//! model in `adapters/catalog/spec/ess` (`connectors_catalog.amendment.Amendments`, compiled to
//! JSON Schema by the pinned `ess`), and the model names both kinds of amendment, an added
//! optional query parameter and a path correction, each of which a committed file uses
//! (story:catalog-path-correction-and-value-bound).
use serde_json::Value;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `*.amendments.json` under an adapter's `upstream/` directory.
fn committed() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(root().join("adapters"))
        .unwrap()
        .map(|entry| entry.unwrap().path().join("upstream"))
        .filter(|upstream| upstream.is_dir())
        .flat_map(|upstream| std::fs::read_dir(upstream).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".amendments.json"))
        })
        .collect();
    files.sort();
    files
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
fn every_committed_amendment_file_is_a_value_of_the_catalog_amendment_model() {
    let schema = compiled("connectors_catalog.amendment.Amendments");
    let invariants =
        schema["$defs"]["connectors_catalog.amendment.Amendment"]["x-ess-invariants"].to_string();
    assert!(
        invariants.contains("defined(add_parameter)")
            && invariants.contains("defined(correct_path)"),
        "the model no longer requires exactly one change per amendment: {invariants}"
    );
    let validator = jsonschema::validator_for(&schema).unwrap();
    let files = committed();
    let mut kinds = std::collections::BTreeSet::new();
    for file in &files {
        let raw: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&raw)
            .map(|e| format!("{}: {e}", e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{}: {errors:?}", file.display());
        for amendment in raw["amendments"].as_array().unwrap() {
            for kind in ["add_parameter", "correct_path"] {
                if amendment.get(kind).is_some() {
                    kinds.insert(kind);
                }
            }
        }
    }
    assert_eq!(
        kinds.into_iter().collect::<Vec<_>>(),
        ["add_parameter", "correct_path"],
        "a committed file must use each kind of amendment, or this check would not cover it: {files:?}"
    );
}

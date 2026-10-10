//! Every guard a shipped selection declares is a value of the catalog adapter's own model in
//! `adapters/catalog/spec/ess` (`connectors_catalog.guard.Guard`, compiled to JSON Schema by the
//! pinned `ess`), and the engine's reader carries every field of it: read and written back, it is
//! the same document, so a guard keeps its bytes and its configuration revision. A guard with a
//! member the model does not name fails here, and so does a model member the engine drops.
//!
//! The same holds for the members of a selection that close, type, require and fix a write's body
//! (`connectors_catalog.selection.ClosedBody`): every shipped selection's projection onto them is
//! a value of the model, and the whole selection reads and writes back unchanged.
use connectors_catalog_provider::{Guard, Selection};
use serde_json::Value;
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

/// Every operation with a `guard` in every shipped selection file, by file and id.
fn guarded() -> Vec<(String, Value)> {
    shipped()
        .into_iter()
        .filter_map(|(name, operation)| Some((name, operation.get("guard")?.clone())))
        .collect()
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

fn assert_valid(validator: &jsonschema::Validator, name: &str, raw: &Value) {
    let errors: Vec<String> = validator
        .iter_errors(raw)
        .map(|e| format!("{}: {e}", e.instance_path()))
        .collect();
    assert!(errors.is_empty(), "{name}: {errors:?}");
}

#[test]
fn every_shipped_guard_is_a_value_of_the_catalog_guard_model() {
    let schema = compiled("connectors_catalog.guard.Guard");
    // JSON Schema carries the model's invariants without enforcing them; the engine refuses
    // them when the selection loads (`adapters/catalog/tests/jira_transition_run.rs`,
    // `adapters/catalog/tests/gitlab_mr_variants.rs`).
    let invariants =
        schema["$defs"]["connectors_catalog.guard.Postflight"]["x-ess-invariants"].to_string();
    assert!(
        invariants.contains("defined(read)"),
        "the model no longer requires a check of a postflight read: {invariants}"
    );
    assert!(
        invariants.contains("any_of.count"),
        "the model no longer requires two comparisons of an any_of: {invariants}"
    );
    let validator = jsonschema::validator_for(&schema).unwrap();
    let guarded = guarded();
    for member in ["further_preflights", "postflight/read", "postflight/any_of"] {
        assert!(
            guarded
                .iter()
                .any(|(_, guard)| guard.pointer(&format!("/{member}")).is_some()),
            "no shipped guard declares `{member}`, so this check would not cover it"
        );
    }
    for (name, raw) in &guarded {
        assert_valid(&validator, name, raw);
        let guard: Guard =
            serde_json::from_value(raw.clone()).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(&serde_json::to_value(&guard).unwrap(), raw, "{name}");
    }
}

#[test]
fn every_shipped_closed_body_is_a_value_of_the_catalog_selection_model() {
    const MEMBERS: [&str; 4] = ["body_keys", "body_types", "body_required", "body_fixed"];
    let schema = compiled("connectors_catalog.selection.ClosedBody");
    let invariants =
        schema["$defs"]["connectors_catalog.selection.ClosedBody"]["x-ess-invariants"].to_string();
    assert!(
        invariants.contains("defined(body_fixed)"),
        "the model no longer requires a fixed key's body to be closed: {invariants}"
    );
    let validator = jsonschema::validator_for(&schema).unwrap();
    let shipped = shipped();
    for member in MEMBERS {
        assert!(
            shipped.iter().any(|(_, s)| s.get(member).is_some()),
            "no shipped selection declares `{member}`, so this check would not cover it"
        );
    }
    for (name, raw) in &shipped {
        let projection: serde_json::Map<String, Value> = MEMBERS
            .iter()
            .filter_map(|member| Some((member.to_string(), raw.get(*member)?.clone())))
            .collect();
        assert_valid(&validator, name, &Value::Object(projection));
        // The whole selection reads and writes back member for member: nothing the file
        // carries is dropped, and an omitted member is not written back. `description`,
        // `guard` and `response` are written as `null` when absent.
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

/// Adversary: the engine's reader and the compiled model agree on the shape of a guard. For each
/// variation of the shipped Jira guard, the reader admits it exactly when the compiled schema
/// does. A postflight `read` written as `null` is not a value of the model (`read` is an
/// `Optional<Read>`, compiled as an object or absent), and the reader admits it as absent.
#[test]
fn the_guard_reader_admits_exactly_the_shapes_the_compiled_model_admits() {
    let schema = compiled("connectors_catalog.guard.Guard");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let (_, shipped) = guarded()
        .into_iter()
        .find(|(_, guard)| guard["postflight"].get("read").is_some())
        .expect("a shipped guard with a postflight read");
    type Change<'a> = &'a dyn Fn(&mut Value);
    let variations: [(&str, Change); 4] = [
        ("as shipped", &|_| {}),
        ("postflight read null", &|g| {
            g["postflight"]["read"] = Value::Null
        }),
        ("further_preflights null", &|g| {
            g["further_preflights"] = Value::Null
        }),
        ("any_of null", &|g| g["postflight"]["any_of"] = Value::Null),
    ];
    for (name, change) in variations {
        let mut raw = shipped.clone();
        change(&mut raw);
        let model = validator.is_valid(&raw);
        let reader = serde_json::from_value::<Guard>(raw.clone()).is_ok();
        assert_eq!(
            reader, model,
            "{name}: reader admits {reader}, model admits {model}"
        );
    }
}

//! Every identity probe the catalog guides tell a reader to configure is a value of the catalog
//! adapter's own model in `adapters/catalog/spec/ess`, compiled to JSON Schema by the pinned
//! `ess`: `connectors_catalog.identity.Probe` for `auth.identity`, and
//! `connectors_catalog.identity.Profile` for the profile's scheme with its identity. A documented
//! identity with a field, a `source` or a scheme the model does not name fails here, and so does
//! a configured subject under an OAuth scheme, which the model's invariants refuse. The
//! provider's own refusal of those invariants is `adapters/catalog/tests/runpod.rs`.
use serde_json::Value;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The `auth` profile, with an `identity`, of every JSON example in `docs/`, whole
/// configurations and `"auth": {…}` excerpts alike, with the guide it came from.
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
            if let Some(auth) = value
                .as_ref()
                .and_then(|v| v.get("auth"))
                .filter(|auth| auth.get("identity").is_some())
            {
                found.push((name.clone(), auth.clone()));
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
    let read = |name: &str| -> Value {
        serde_json::from_slice(
            &std::fs::read(out.path().join(format!(
                "schemas/schema/types/connectors_catalog.identity.{name}.schema.json"
            )))
            .unwrap(),
        )
        .unwrap()
    };
    let schema = read("Profile");
    assert_eq!(
        schema["$defs"]["connectors_catalog.identity.Source"]["enum"],
        serde_json::json!(["api", "id_token", "configuration"])
    );
    assert_eq!(
        schema["$defs"]["connectors_catalog.identity.Scheme"]["enum"],
        serde_json::json!([
            "token",
            "basic",
            "oauth2_refresh",
            "oauth2_client_credentials"
        ])
    );
    // JSON Schema carries the model's invariants without enforcing them: the profile invariant is
    // checked by hand below, and here only that the model still states it.
    let invariants =
        schema["$defs"]["connectors_catalog.identity.Profile"]["x-ess-invariants"].to_string();
    for scheme in ["oauth2_refresh", "oauth2_client_credentials"] {
        assert!(
            invariants.contains(&format!("scheme == {scheme}")),
            "the model no longer refuses a configured subject under `{scheme}`: {invariants}"
        );
    }
    let probe = jsonschema::validator_for(&read("Probe")).unwrap();
    let profile = jsonschema::validator_for(&schema).unwrap();
    let documented = documented();
    let sources: std::collections::BTreeSet<&str> = documented
        .iter()
        .map(|(_, auth)| auth["identity"]["source"].as_str().unwrap_or("api"))
        .collect();
    assert_eq!(
        sources,
        ["api", "configuration", "id_token"].into(),
        "the guides no longer exercise every source, so this check would not cover the model"
    );
    for (guide, auth) in &documented {
        let identity = &auth["identity"];
        let mut projection = serde_json::json!({"identity": identity});
        if let Some(scheme) = auth.get("scheme") {
            projection["scheme"] = scheme.clone();
        }
        let errors: Vec<String> = probe
            .iter_errors(identity)
            .chain(profile.iter_errors(&projection))
            .map(|e| format!("{}: {e}", e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{guide}: {auth}: {errors:?}");
        let scheme = auth["scheme"].as_str().unwrap_or("token");
        assert!(
            identity["source"] != "configuration" || ["token", "basic"].contains(&scheme),
            "{guide}: a configured subject under `{scheme}`"
        );
    }
    assert!(
        documented
            .iter()
            .any(|(guide, auth)| guide == "catalog-runpod.md"
                && auth["identity"]["source"] == "configuration"),
        "the Runpod guide's configured identity was not read"
    );
}

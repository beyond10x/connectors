use connectors_spec::v2::{Spec, generate, hash, import, tree};
use serde_json::{Value, json};
use std::path::Path;

fn ess() -> std::path::PathBuf {
    connectors_spec::toolchain::resolve(None).unwrap()
}

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}
fn source() -> Value {
    serde_json::from_slice(
        &std::fs::read(root().join("adapters/gitlab/spec/adapter.json")).unwrap(),
    )
    .unwrap()
}
fn parse(value: &Value) -> connectors_core::Result<Spec> {
    Spec::parse(&serde_json::to_vec(value).unwrap())
}

#[test]
fn checked_in_bundle_matches_its_pinned_sources_and_toolchain() {
    generate(
        &root().join("adapters/gitlab/spec/adapter.json"),
        &root().join("adapters/gitlab/generated"),
        &ess(),
        true,
    )
    .unwrap();
}

#[test]
fn public_import_refuses_specs_changed_after_validation() {
    let path = root().join("adapters/gitlab/spec/adapter.json");
    let mut spec = parse(&source()).unwrap();
    spec.mappings[0].response_prefix_limit = Some(0);
    assert!(
        import(&spec, &path)
            .unwrap_err()
            .message
            .contains("prefix limit")
    );
    let mut spec = parse(&source()).unwrap();
    spec.operations.clear();
    assert!(
        import(&spec, &path)
            .unwrap_err()
            .message
            .contains("undeclared operation")
    );
    let mut spec = parse(&source()).unwrap();
    spec.mappings[0].path_parameters.insert(
        "id".into(),
        connectors_spec::v2::Parameter::Input {
            name: "absent".into(),
        },
    );
    assert!(
        import(&spec, &path)
            .unwrap_err()
            .message
            .contains("missing input")
    );
}

#[test]
fn strict_v1_and_v2_refuse_unimplemented_or_ambiguous_mappings() {
    let v1 = include_bytes!("../../../adapters/kubernetes/spec/adapter.json");
    let mut old: Value = serde_json::from_slice(v1).unwrap();
    old["mappings"] = json!([]);
    assert!(connectors_spec::compile(&serde_json::to_vec(&old).unwrap()).is_err());
    for change in 0..7 {
        let mut s = source();
        match change {
            0 => s["mappings"][0]["obligations"] = json!([]),
            1 => s["mappings"].as_array_mut().unwrap().truncate(2),
            2 => s["mappings"][0]["path_parameters"]["id"]["name"] = json!("unknown"),
            3 => s["mappings"][0]["method"] = json!("post"),
            4 => {
                s["operations"][0]["input_schema"]["properties"]["project"]["type"] = json!("array")
            }
            5 => {
                s["mappings"][0]["query_parameters"]["new"] =
                    json!({"kind":"binding","name":"self"})
            }
            _ => s["upstream"]["unknown"] = json!(true),
        }
        assert!(parse(&s).is_err(), "case {change} admitted");
    }
    // Similar names must be refused without panicking inside PascalCase lowering.
    let mut s = source();
    s["operations"][0]["id"] = json!("a__b");
    s["mappings"][0]["operation"] = json!("a__b");
    s["operations"][1]["id"] = json!("a_b");
    s["mappings"][1]["operation"] = json!("a_b");
    assert!(parse(&s).is_err());
}

#[test]
fn response_prefix_is_explicit_bounded_and_absent_in_legacy_serialization() {
    let mut value = source();
    for invalid in [
        json!(0),
        json!(1048577),
        json!(-1),
        json!(1.5),
        json!("512000"),
        json!(null),
    ] {
        value["mappings"][0]["response_prefix_limit"] = invalid;
        assert!(parse(&value).is_err());
    }
    for valid in [1, 512000, 1048576] {
        value["mappings"][0]["response_prefix_limit"] = json!(valid);
        assert_eq!(
            parse(&value).unwrap().mappings[0].response_prefix_limit,
            Some(valid)
        );
    }
    value["mappings"][0]
        .as_object_mut()
        .unwrap()
        .remove("response_prefix_limit");
    let serialized = serde_json::to_value(parse(&value).unwrap()).unwrap();
    assert!(
        serialized["mappings"][0]
            .get("response_prefix_limit")
            .is_none()
    );
}

#[test]
fn pinned_source_and_required_source_semantics_are_enforced() {
    let temp = tempfile::tempdir().unwrap();
    let mut spec = source();
    spec["upstream"]["path"] = json!("upstream.json");
    let mut upstream = json!({"openapi":"3.0.0","info":{"title":"authored test fixture","version":"1"},"paths":{}});
    for m in spec["mappings"].as_array().unwrap() {
        let mut params = vec![];
        for place in ["path", "query"] {
            for (name, _) in m[format!("{place}_parameters")].as_object().unwrap() {
                params.push(json!({"in":place,"name":name,"required":place=="path","schema":{"type":if ["page","per_page","pipeline_id","job_id"].contains(&name.as_str()) {"integer"} else {"string"}}}));
            }
        }
        upstream["paths"][m["path"].as_str().unwrap()] = json!({"get":{"operationId":m["upstream_operation"],"parameters":params,"responses":{"200":{"description":"test response"}}}});
    }
    for mutation in 0..7 {
        let mut u = upstream.clone();
        let op = &mut u["paths"]["/api/v4/projects/{id}"]["get"];
        match mutation {
            0 => {},
            1 => op["operationId"] = json!("changed"),
            2 => op["parameters"].as_array_mut().unwrap().push(json!({"in":"query","name":"required_new","required":true,"schema":{"type":"string"}})),
            3 => op["requestBody"] = json!({}),
            4 => op["parameters"][0]["style"] = json!("matrix"),
            5 => op["parameters"][0]["schema"]["pattern"] = json!("^[0-9]+$"),
            _ => op["responses"]["200"]["content"] = json!({"application/json":{"schema":{"$ref":"https://invalid.example/schema"}}}),
        }
        let bytes = serde_json::to_vec(&u).unwrap();
        std::fs::write(temp.path().join("upstream.json"), &bytes).unwrap();
        spec["upstream"]["sha256"] = json!(hash(&bytes));
        let result = import(&parse(&spec).unwrap(), &temp.path().join("adapter.json"));
        assert_eq!(result.is_ok(), mutation == 0, "case {mutation}");
    }
    spec["upstream"]["sha256"] = json!("0".repeat(64));
    assert!(import(&parse(&spec).unwrap(), &temp.path().join("adapter.json")).is_err());
}

#[test]
fn regeneration_is_reproducible_preserves_handwritten_files_and_detects_drift() {
    let temp = tempfile::tempdir().unwrap();
    let spec = root().join("adapters/gitlab/spec/adapter.json");
    let out = temp.path().join("generated");
    generate(&spec, &out, &ess(), false).unwrap();
    let original = tree(&out).unwrap();
    assert!(
        !original
            .keys()
            .any(|path| path.starts_with("rust/.ess-output/"))
    );
    let manifest: Value = serde_json::from_slice(&original["manifest.json"]).unwrap();
    assert!(
        !manifest["files"]
            .as_object()
            .unwrap()
            .keys()
            .any(|path| path.starts_with("rust/.ess-output/"))
    );
    let other = temp.path().join("other");
    generate(&spec, &other, &ess(), false).unwrap();
    assert_eq!(original, tree(&other).unwrap());
    std::fs::write(out.join("handwritten.rs"), b"keep this\n").unwrap();
    generate(&spec, &out, &ess(), false).unwrap();
    assert_eq!(
        std::fs::read(out.join("handwritten.rs")).unwrap(),
        b"keep this\n"
    );
    generate(&spec, &out, &ess(), true).unwrap();
    std::fs::write(out.join("runtime.rs"), b"corrupt").unwrap();
    assert!(generate(&spec, &out, &ess(), true).is_err());
    // Malformed ownership is refused before touching any generated file.
    let mut manifest: Value =
        serde_json::from_slice(&std::fs::read(out.join("manifest.json")).unwrap()).unwrap();
    manifest["files"]["../handwritten.txt"] = json!("0".repeat(64));
    std::fs::write(
        out.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    assert!(generate(&spec, &out, &ess(), false).is_err());
    assert_eq!(std::fs::read(out.join("runtime.rs")).unwrap(), b"corrupt");
}

#[test]
fn regeneration_refuses_unowned_files_and_output_symlinks() {
    let temp = tempfile::tempdir().unwrap();
    let spec = root().join("adapters/gitlab/spec/adapter.json");
    let out = temp.path().join("generated");
    std::fs::create_dir(&out).unwrap();
    std::fs::write(out.join("runtime.rs"), b"handwritten").unwrap();
    assert!(generate(&spec, &out, &ess(), false).is_err());
    assert_eq!(tree(&out).unwrap().len(), 1);
    #[cfg(unix)]
    {
        let linked = temp.path().join("linked");
        std::os::unix::fs::symlink(&out, &linked).unwrap();
        assert!(generate(&spec, &linked, &ess(), false).is_err());
    }
}

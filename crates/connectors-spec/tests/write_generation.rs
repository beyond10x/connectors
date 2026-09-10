use connectors_spec::v2::{hash, json_bytes, tree, write};
use connectors_spec::v3::{Spec, import};
use serde_json::{Value, json};
use std::path::Path;

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}
fn legacy() -> Value {
    serde_json::from_slice(
        &std::fs::read(root().join("adapters/gitlab/spec/adapter.json")).unwrap(),
    )
    .unwrap()
}

#[test]
fn old_readers_reject_the_write_extension_before_activation() {
    let old = legacy();
    assert!(connectors_spec::v2::Spec::parse(&serde_json::to_vec(&old).unwrap()).is_ok());
    let v1_schema: Value =
        serde_json::from_str(include_str!("../../../spec-kinds/adapter/v1/schema.json")).unwrap();
    let v1 = jsonschema::validator_for(&v1_schema).unwrap();
    // Start from a valid v1 document too: otherwise the upstream/mapping fields
    // belonging to v2 could hide a v1 reader that mistakenly accepts writes.
    let mut v1_document = old.clone();
    v1_document["kind"] = json!("connectors.adapter/v1");
    v1_document.as_object_mut().unwrap().remove("upstream");
    v1_document.as_object_mut().unwrap().remove("mappings");
    assert!(v1.is_valid(&v1_document));
    assert!(serde_json::from_value::<connectors_spec::AdapterSpec>(v1_document.clone()).is_ok());
    v1_document["writes"] = json!([]);
    assert!(!v1.is_valid(&v1_document));
    assert!(serde_json::from_value::<connectors_spec::AdapterSpec>(v1_document).is_err());
    for kind in [
        "connectors.adapter/v1",
        "connectors.adapter/v2",
        "connectors.adapter/v3",
    ] {
        let mut changed = old.clone();
        changed["kind"] = json!(kind);
        changed["writes"] = json!([]);
        let bytes = serde_json::to_vec(&changed).unwrap();
        assert!(!v1.is_valid(&changed));
        assert!(serde_json::from_slice::<connectors_spec::AdapterSpec>(&bytes).is_err());
        assert!(connectors_spec::v2::Spec::parse(&bytes).is_err());
    }
    let mut changed = old;
    changed["mappings"][0]["method"] = json!("put");
    assert!(connectors_spec::v2::Spec::parse(&serde_json::to_vec(&changed).unwrap()).is_err());
}

fn object(properties: Value) -> Value {
    json!({"type":"object","additionalProperties":false,"required":properties.as_object().unwrap().keys().collect::<Vec<_>>(),"properties":properties})
}
fn fixture() -> (Value, Value) {
    let input = object(
        json!({"id":{"type":"string","minLength":1},"version":{"type":"integer"},"enabled":{"type":"boolean"},"at":{"type":"string","format":"date-time"}}),
    );
    let output = object(
        json!({"id":{"type":"string","minLength":1},"detail":object(json!({"version":{"type":"integer"},"enabled":{"type":"boolean"},"note":{"anyOf":[{"type":"string"},{"type":"null"}]}}))}),
    );
    let revision = "a".repeat(40);
    let spec = json!({"kind":"connectors.adapter/v3","id":"fixture","version":"1","sources":["https://example.invalid/source"],"configuration_schema":object(json!({})),"upstream":{"path":"upstream.json","url":format!("https://example.invalid/{revision}/openapi.json"),"revision":revision,"sha256":"0".repeat(64),"base_path":"/v1"},
        "operations":[{"id":"item.get","description":"Fixture read","contract":"fixture/1","profile":"read","input_schema":object(json!({"id":{"type":"string"}})),"output_schema":{"type":"object"}}],
        "mappings":[{"operation":"item.get","upstream_operation":"getItem","path":"/v1/items/{id}","method":"get","path_parameters":{"id":{"kind":"input","name":"id"}},"query_parameters":{},"obligations":["prepare","finish"]}],
        "writes":[{"operation":{"id":"item.update","description":"Fixture guarded write","contract":"fixture/1","profile":"write","input_schema":input,"output_schema":output},"mapping":{"operation":"item.update","upstream_operation":"putItem","path":"/v1/items/{id}","method":"put","path_parameters":{"id":{"kind":"input","name":"id"}},"query_parameters":{"version":{"kind":"input","name":"version"}},"body":{"enabled":{"kind":"input","name":"enabled"},"at":{"kind":"input","name":"at"},"deferred":{"kind":"boolean","value":false},"label":{"kind":"string","value":"owned fixture"},"number":{"kind":"integer","value":-9223372036854775808i64}}}}]});
    let source = json!({"openapi":"3.0.0","info":{"title":"Authored write fixture","version":"1"},"paths":{"/v1/items/{id}":{
        "get":{"operationId":"getItem","parameters":[{"in":"path","name":"id","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"Read"}}},
        "put":{"operationId":"putItem","parameters":[{"in":"path","name":"id","required":true,"schema":{"type":"string","minLength":1}},{"in":"query","name":"version","required":true,"schema":{"type":"integer"}}],"requestBody":{"$ref":"#/components/requestBodies/Update"},"responses":{"200":{"description":"Written","content":{"application/json":{"schema":{"$ref":"#/components/schemas/Result"}}}}}}
    }},"components":{"requestBodies":{"Update":{"required":true,"content":{"application/json":{"schema":{"$ref":"#/components/schemas/Body"}}}}},"schemas":{"Body":{"type":"object","required":["enabled","at"],"properties":{"enabled":{"type":"boolean"},"at":{"type":"string","format":"date-time"},"deferred":{"type":"boolean"},"label":{"type":"string"},"number":{"type":"integer"},"unused":{"type":"string"}}},"Result":{"type":"object"}}}});
    (spec, source)
}
fn parse(value: &Value) -> connectors_core::Result<Spec> {
    Spec::parse(&json_bytes(value).unwrap())
}
fn store(temp: &Path, mut spec: Value, upstream: &Value) -> std::path::PathBuf {
    let bytes = json_bytes(upstream).unwrap();
    spec["upstream"]["sha256"] = json!(hash(&bytes));
    write(&temp.join("upstream.json"), &bytes).unwrap();
    write(&temp.join("adapter.json"), &json_bytes(&spec).unwrap()).unwrap();
    temp.join("adapter.json")
}
fn check_import(spec: Value, source: &Value) -> connectors_core::Result<(Value, Value)> {
    let temp = tempfile::tempdir().unwrap();
    let path = store(temp.path(), spec, source);
    let spec = Spec::parse(&std::fs::read(&path).unwrap())?;
    import(&spec, &path)
}

#[test]
fn separate_projection_and_closed_schema_refusals() {
    let (spec, _) = fixture();
    let parsed = parse(&spec).unwrap();
    let read = parsed.descriptor().unwrap();
    let private = parsed.private_descriptor().unwrap();
    assert!(read.operation("item.update").is_err());
    assert!(private.operation("item.update").is_ok());
    assert_ne!(private.revision, read.revision);
    assert_eq!(
        connectors_spec::compile(&json_bytes(&spec).unwrap())
            .unwrap()
            .revision,
        read.revision
    );
    let mut changed = spec.clone();
    changed["writes"][0]["mapping"]["body"]["deferred"]["value"] = json!(true);
    assert_eq!(
        parse(&changed).unwrap().descriptor().unwrap().revision,
        read.revision
    );
    assert_ne!(
        parse(&changed)
            .unwrap()
            .private_descriptor()
            .unwrap()
            .revision,
        private.revision
    );
    let cases = [
        ("/writes/0/mapping/method", json!("get")),
        ("/writes/0/mapping/operation", json!("item.get")),
        ("/writes/0/operation/id", json!("item_get")),
        ("/writes/0/mapping/path", json!("/v1/items/../{id}")),
        ("/writes/0/mapping/path", json!("/v1/items/%2e%2e/{id}")),
        ("/writes/0/mapping/path", json!("/v1/{id}/{id}")),
        ("/writes/0/mapping/path_parameters", json!({})),
        (
            "/writes/0/mapping/body/enabled",
            json!({"kind":"input","name":"absent"}),
        ),
        (
            "/writes/0/mapping/body/deferred",
            json!({"kind":"boolean","value":"false"}),
        ),
        (
            "/writes/0/mapping/body/number",
            json!({"kind":"integer","value":9223372036854775808u64}),
        ),
        ("/writes/0/operation/input_schema/required", json!(["id"])),
        (
            "/writes/0/operation/input_schema/additionalProperties",
            json!(true),
        ),
        (
            "/writes/0/operation/input_schema/properties/enabled",
            json!({"type":"array"}),
        ),
        (
            "/writes/0/operation/input_schema/properties/at/format",
            json!("uuid"),
        ),
        (
            "/writes/0/operation/output_schema/properties/detail",
            json!({"type":"array","items":{"type":"string"}}),
        ),
        (
            "/writes/0/operation/output_schema/properties/detail/properties/note",
            json!({"anyOf":[{"type":"string","not":{"const":"x"}},{"type":"null"}]}),
        ),
        ("/writes/0/operation/output_schema/required", json!(["id"])),
        ("/writes", json!([])),
    ];
    for (path, value) in cases {
        let mut changed = spec.clone();
        *changed.pointer_mut(path).unwrap() = value;
        assert!(parse(&changed).is_err(), "accepted {path}");
    }
    let mut changed = spec.clone();
    changed["writes"][0]["mapping"]["retry"] = json!(true);
    assert!(parse(&changed).is_err());
    let mut changed = spec;
    changed["writes"][0]["operation"]["input_schema"]["unevaluatedProperties"] = json!(false);
    assert!(parse(&changed).is_err());
}

#[test]
fn pinned_import_refuses_missing_requirements_and_unsupported_meaning() {
    let (spec, source) = fixture();
    let (selected, coverage) = check_import(spec.clone(), &source).unwrap();
    assert_eq!(
        selected["paths"]["/v1/items/{id}"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
    assert!(
        selected
            .pointer("/components/requestBodies/Update")
            .is_some()
    );
    assert_eq!(
        coverage["selected_operations"][0]["excluded_optional_body_properties"],
        json!(["unused"])
    );
    let cases = [
        ("/paths/~1v1~1items~1{id}/put/operationId", json!("changed")),
        ("/paths/~1v1~1items~1{id}/put/parameters", json!(null)),
        (
            "/paths/~1v1~1items~1{id}/put/parameters/0/schema",
            json!({"type":"integer"}),
        ),
        (
            "/paths/~1v1~1items~1{id}/put/parameters/0/schema",
            json!({"type":"string","oneOf":[{"type":"string","minLength":100}]}),
        ),
        (
            "/paths/~1v1~1items~1{id}/put/parameters/0/schema",
            json!({"oneOf":[{"type":"string"},{"type":"string"}]}),
        ),
        (
            "/paths/~1v1~1items~1{id}/put/parameters/0/schema",
            json!({"type":"string","minLength":2}),
        ),
        (
            "/paths/~1v1~1items~1{id}/put/parameters/1/required",
            json!("true"),
        ),
        (
            "/components/schemas/Body/required",
            json!(["enabled", "unused"]),
        ),
        ("/components/schemas/Body/required", json!("enabled")),
        (
            "/components/schemas/Body/required",
            json!(["enabled", "enabled"]),
        ),
        (
            "/components/schemas/Body/properties/enabled",
            json!({"type":"string"}),
        ),
        (
            "/components/schemas/Body/properties/deferred",
            json!({"type":"boolean","enum":[true]}),
        ),
        (
            "/components/schemas/Body/properties/number",
            json!({"type":"integer","minimum":0}),
        ),
        (
            "/components/schemas/Body/properties/at",
            json!({"type":"string","pattern":"^x"}),
        ),
        (
            "/components/schemas/Body/properties/label",
            json!({"type":"string","maxLength":1}),
        ),
        (
            "/components/schemas/Body/properties/label",
            json!({"type":"string","nullable":"true"}),
        ),
        (
            "/components/schemas/Body/properties/label",
            json!({"$ref":"#/components/schemas/Body"}),
        ),
        (
            "/components/schemas/Body/properties/label",
            json!({"$ref":"https://example.invalid/body"}),
        ),
        (
            "/components/requestBodies/Update/content/application~1json/schema",
            json!({"$ref":"#/components/schemas/Body","description":"ambiguous sibling"}),
        ),
        (
            "/components/schemas/Result",
            json!({"$ref":"https://example.invalid/result"}),
        ),
    ];
    for (path, value) in cases {
        let mut changed = source.clone();
        *changed.pointer_mut(path).unwrap() = value;
        assert!(
            check_import(spec.clone(), &changed).is_err(),
            "accepted {path}"
        );
    }
    let mut source = source;
    source["paths"]["/v1/items/{id}"]["put"]["parameters"]
        .as_array_mut()
        .unwrap()
        .push(json!({"in":"header","name":"required","required":true,"schema":{"type":"string"}}));
    assert!(check_import(spec.clone(), &source).is_err());
    let temp = tempfile::tempdir().unwrap();
    let path = store(temp.path(), spec, &fixture().1);
    let spec = Spec::parse(&std::fs::read(&path).unwrap()).unwrap();
    std::fs::write(temp.path().join("upstream.json"), b"{}").unwrap();
    assert!(import(&spec, &path).is_err());
}

#[test]
fn gitlab_merge_mapping_admits_the_existing_pinned_vendor_source() {
    let mut value = legacy();
    let operations = value["operations"].as_array().unwrap();
    let validation = operations
        .iter()
        .find(|o| o["id"] == "merge_request.validate")
        .unwrap();
    let read = operations
        .iter()
        .find(|o| o["id"] == "merge_request.get")
        .unwrap();
    let write = json!({"operation":{"id":"merge_request.merge","description":"Fixture of the selected native merge contract; no production activation","contract":"gitlab.merge-requests/v1alpha1","profile":"guarded-merge","input_schema":validation["input_schema"],"output_schema":read["output_schema"]},"mapping":{"operation":"merge_request.merge","upstream_operation":"putApiV4ProjectsIdMergeRequestsMergeRequestIidMerge","path":"/api/v4/projects/{id}/merge_requests/{merge_request_iid}/merge","method":"put","path_parameters":{"id":{"kind":"input","name":"project"},"merge_request_iid":{"kind":"input","name":"iid"}},"query_parameters":{},"body":{"sha":{"kind":"input","name":"sha"},"auto_merge":{"kind":"boolean","value":false},"should_remove_source_branch":{"kind":"boolean","value":false}}}});
    value["kind"] = json!("connectors.adapter/v3");
    value["writes"] = json!([write]);
    let parsed = parse(&value).unwrap();
    let (_, coverage) = import(&parsed, &root().join("adapters/gitlab/spec/adapter.json")).unwrap();
    assert_eq!(
        coverage["source"]["sha256"],
        json!("f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530")
    );
    assert_eq!(
        coverage["selected_operations"][0]["mapped_body_properties"],
        json!(["auto_merge", "sha", "should_remove_source_branch"])
    );
    assert!(
        parsed
            .descriptor()
            .unwrap()
            .operation("merge_request.merge")
            .is_err()
    );
}

#[test]
fn generated_bundle_is_reproducible_and_executes_without_read_authority() {
    let temp = tempfile::tempdir().unwrap();
    let (spec, source) = fixture();
    let path = store(temp.path(), spec, &source);
    let ess = connectors_spec::toolchain::resolve(None).unwrap();
    let first = temp.path().join("generated");
    let second = temp.path().join("second");
    connectors_spec::generate(&path, &first, &ess, false).unwrap();
    connectors_spec::generate(&path, &second, &ess, false).unwrap();
    let original = tree(&first).unwrap();
    assert_eq!(original, tree(&second).unwrap());
    assert!(!original.keys().any(|p| p.contains(".ess-output/")));
    // The old projection is byte-identical to independent v2 generation.
    let mut read: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    read.as_object_mut().unwrap().remove("writes");
    read["kind"] = json!("connectors.adapter/v2");
    let read_path = temp.path().join("read.json");
    write(&read_path, &json_bytes(&read).unwrap()).unwrap();
    let read_out = temp.path().join("read");
    connectors_spec::v2::generate(&read_path, &read_out, &ess, false).unwrap();
    for (name, bytes) in tree(&read_out).unwrap() {
        if name != "manifest.json" {
            assert_eq!(original[&name], bytes, "read drift: {name}");
        }
    }
    run_fixture(temp.path());
    write(&first.join("authored.txt"), b"keep").unwrap();
    connectors_spec::generate(&path, &first, &ess, true).unwrap();
    write(&first.join("writes.rs"), b"drift").unwrap();
    assert!(connectors_spec::generate(&path, &first, &ess, true).is_err());
    connectors_spec::generate(&path, &first, &ess, false).unwrap();
    assert_eq!(std::fs::read(first.join("authored.txt")).unwrap(), b"keep");
    let collision = temp.path().join("collision");
    write(&collision.join("writes.rs"), b"owned elsewhere").unwrap();
    assert!(connectors_spec::generate(&path, &collision, &ess, false).is_err());
    assert_eq!(tree(&collision).unwrap().len(), 1);
    #[cfg(unix)]
    {
        let linked = temp.path().join("linked");
        std::os::unix::fs::symlink(&first, &linked).unwrap();
        assert!(connectors_spec::generate(&path, &linked, &ess, false).is_err());
    }
}

fn run_fixture(temp: &Path) {
    use std::process::Command;
    // This child has its own target directory: never contend with the parent
    // Cargo invocation or reuse artifacts across distinct compiler selections.
    let manifest = format!(
        r#"[package]
name = "generated-write-fixture"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
connectors-core = {{ path = {:?} }}
connectors-sdk = {{ path = {:?} }}
fixture-types = {{ path = "generated/rust/crates/fixture-types" }}
fixture_writes-types = {{ path = "generated/write-rust/crates/fixture_writes-types" }}
serde_json = "1"
async-trait = "0.1"
tokio = {{ version = "1", features = ["macros", "rt"] }}
[profile.dev]
debug = 0
incremental = false
"#,
        root().join("crates/connectors-core"),
        root().join("crates/connectors-sdk")
    );
    write(&temp.join("Cargo.toml"), manifest.as_bytes()).unwrap();
    write(
        &temp.join("src/lib.rs"),
        include_bytes!("fixtures/write_runtime.rs"),
    )
    .unwrap();
    // Reuse the repository's dependency selections; Cargo records this fixture's
    // local package identities before the subsequent locked execution.
    std::fs::copy(root().join("Cargo.lock"), temp.join("Cargo.lock")).unwrap();
    for args in [
        &["metadata", "--format-version", "1", "--offline"][..],
        &["test", "--locked", "--offline"][..],
    ] {
        let output = Command::new("cargo")
            .args(args)
            .current_dir(temp)
            .env("CARGO_BUILD_JOBS", "2")
            .env("CARGO_TARGET_DIR", temp.join("target"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "fixture cargo {args:?}:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if args[0] == "metadata" {
            let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
            let lock = std::fs::read_to_string(root().join("Cargo.lock")).unwrap();
            for package in metadata["packages"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["source"].is_string())
            {
                let identity = format!(
                    "name = {}\nversion = {}\n",
                    package["name"], package["version"]
                );
                assert!(
                    lock.contains(&identity),
                    "fixture changed repository pin: {identity}"
                );
            }
        } else {
            println!(
                "fixture cargo {args:?}:\n{}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
    }
}

//! The Google Discovery projection, row by row and over the four pinned documents.
//!
//! Each `discovery_rule_<row>` case reads one small Discovery document from
//! `tests/discovery/<row>/discovery.json` and compares the projection against the
//! hand-written `openapi.json` and `record.json` beside it. The goldens were written
//! from the story's rule table before the projector existed; they are not captures
//! of its output. A record golden omits `source_sha256` and `source_bytes`, which
//! every case checks against the fixture's own bytes instead.
//!
//! The cases over `adapters/google/upstream/` read the pinned files in place; there
//! is no second copy of them under this crate.
use connectors_catalog::discovery::{self, PROJECTOR, Reason, Refusal};
use connectors_catalog::{ingest, inventory, parse_document};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// The four pinned Discovery documents: directory under `adapters/google/upstream/`
/// and file name. The story pins exactly these.
const PINNED: [(&str, &str); 4] = [
    ("calendar", "calendar-api.json"),
    ("drive", "drive-api.json"),
    ("gmail", "gmail-api.json"),
    ("slides", "slides-api.json"),
];

/// The document parameters no projection may carry: they authenticate the caller.
const CREDENTIAL_PARAMETERS: [&str; 3] = ["access_token", "oauth_token", "key"];

fn upstream() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/google/upstream")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/discovery")
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn json(path: &Path) -> Value {
    serde_json::from_slice(&read(path)).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn pinned(api: &str, file: &str) -> Vec<u8> {
    read(&upstream().join(api).join(file))
}

fn projected(api: &str, file: &str) -> discovery::Projection {
    discovery::project(&pinned(api, file))
        .unwrap_or_else(|refusal| panic!("{api}: the pinned document is refused: {refusal}"))
}

/// The canonical form of a JSON output: keys sorted at every depth (through
/// `connectors_core::canonical`, which sorts whatever serde_json's map feature
/// is), pretty-printed, one final newline. An output equal to this form of
/// itself is canonical; one with any key out of order is not.
fn canonical_pretty(bytes: &[u8]) -> Vec<u8> {
    let value: Value = serde_json::from_slice(bytes).expect("output is JSON");
    let sorted: Value = serde_json::from_slice(&connectors_core::canonical(&value)).unwrap();
    let mut out = serde_json::to_vec_pretty(&sorted).unwrap();
    out.push(b'\n');
    out
}

/// The Discovery method count, computed here from the document itself: every
/// method under the top-level `methods` and under every `resources` entry at any
/// depth. Deliberately independent of the projector's own walk.
fn discovery_methods(document: &Value) -> Vec<String> {
    fn walk(node: &Value, out: &mut Vec<String>) {
        if let Some(methods) = node.get("methods").and_then(Value::as_object) {
            for method in methods.values() {
                out.push(method["id"].as_str().expect("method id").to_owned());
            }
        }
        if let Some(resources) = node.get("resources").and_then(Value::as_object) {
            for resource in resources.values() {
                walk(resource, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(document, &mut out);
    out.sort();
    out
}

fn record_value(projection: &discovery::Projection) -> Value {
    serde_json::to_value(&projection.record).expect("record serializes")
}

/// One rule-table row: the fixture projects to exactly its goldens.
fn rule(row: &str) {
    let directory = fixtures().join(row);
    let source = read(&directory.join("discovery.json"));
    let projection = discovery::project(&source)
        .unwrap_or_else(|refusal| panic!("row `{row}`: fixture refused: {refusal}"));
    let expected = json(&directory.join("openapi.json"));
    let actual: Value =
        serde_json::from_slice(&projection.openapi).expect("projection output is JSON");
    assert_eq!(
        actual,
        expected,
        "row `{row}`: projection differs from the golden\nactual:\n{}",
        serde_json::to_string_pretty(&actual).unwrap()
    );
    let mut record = record_value(&projection);
    let object = record.as_object_mut().expect("record is an object");
    assert_eq!(
        object.remove("source_sha256"),
        Some(json!(sha256(&source))),
        "row `{row}`"
    );
    assert_eq!(
        object.remove("source_bytes"),
        Some(json!(source.len())),
        "row `{row}`"
    );
    let expected = json(&directory.join("record.json"));
    assert_eq!(
        record,
        expected,
        "row `{row}`: record differs from the golden\nactual:\n{}",
        serde_json::to_string_pretty(&record).unwrap()
    );
}

#[test]
fn discovery_rule_servers() {
    rule("servers");
}

#[test]
fn discovery_rule_document_info() {
    rule("document_info");
}

#[test]
fn discovery_rule_operation_id() {
    rule("operation_id");
}

#[test]
fn discovery_rule_path() {
    rule("path");
}

#[test]
fn discovery_rule_http_method() {
    rule("http_method");
}

#[test]
fn discovery_rule_parameter_location() {
    rule("parameter_location");
}

#[test]
fn discovery_rule_parameter_keys() {
    rule("parameter_keys");
}

#[test]
fn discovery_rule_repeated() {
    rule("repeated");
}

#[test]
fn discovery_rule_request_ref() {
    rule("request_ref");
}

#[test]
fn discovery_rule_response_ref() {
    rule("response_ref");
}

#[test]
fn discovery_rule_no_response() {
    rule("no_response");
}

#[test]
fn discovery_rule_media_upload() {
    rule("media_upload");
}

#[test]
fn discovery_rule_ignored_keys() {
    rule("ignored_keys");
}

#[test]
fn discovery_rule_scopes() {
    rule("scopes");
}

#[test]
fn discovery_rule_document_parameters() {
    rule("document_parameters");
}

#[test]
fn discovery_rule_schemas() {
    rule("schemas");
}

#[test]
fn discovery_rule_schema_ref() {
    rule("schema_ref");
}

#[test]
fn discovery_rule_excluded_methods() {
    rule("excluded_methods");
}

#[test]
fn discovery_rule_nested_resources() {
    rule("nested_resources");
}

/// Every row directory has its case above: a fixture nobody runs is a row the
/// table claims and nothing checks.
#[test]
fn every_rule_fixture_has_its_case() {
    let source = String::from_utf8(read(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/discovery.rs"),
    ))
    .expect("this test file is UTF-8");
    let mut rows: Vec<String> = std::fs::read_dir(fixtures())
        .expect("fixture directory")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    rows.sort();
    assert!(rows.len() >= 18, "{rows:?}");
    for row in rows {
        assert!(
            source.contains(&format!("fn discovery_rule_{row}()"))
                && source.contains(&format!("rule(\"{row}\")")),
            "fixture row `{row}` has no discovery_rule_{row} case"
        );
    }
}

/// The smallest document the projector accepts, for the refusal cases to break.
fn base() -> Value {
    json!({
        "kind": "discovery#restDescription",
        "discoveryVersion": "v1",
        "protocol": "rest",
        "title": "Fixture API",
        "version": "v1",
        "revision": "20260101",
        "rootUrl": "https://fixture.googleapis.com/",
        "servicePath": "fixture/v1/",
        "auth": {"oauth2": {"scopes": {"https://www.googleapis.com/auth/fixture": {"description": "Fixture scope"}}}},
        "resources": {
            "things": {
                "methods": {
                    "get": {
                        "id": "fixture.things.get",
                        "path": "things/{thingId}",
                        "httpMethod": "GET",
                        "parameters": {
                            "thingId": {"type": "string", "location": "path", "required": true}
                        },
                        "response": {"$ref": "Thing"},
                        "scopes": ["https://www.googleapis.com/auth/fixture"]
                    }
                }
            }
        },
        "schemas": {
            "Thing": {
                "id": "Thing",
                "type": "object",
                "properties": {"name": {"type": "string"}}
            }
        }
    })
}

fn refusal(document: &Value) -> Refusal {
    match discovery::project(document.to_string().as_bytes()) {
        Ok(_) => panic!("accepted: {document}"),
        Err(refusal) => refusal,
    }
}

/// Put `value` at `pointer`, creating any missing parent object on the way.
fn set(document: &mut Value, pointer: &str, value: Value) {
    let mut keys: Vec<String> = pointer
        .split('/')
        .skip(1)
        .map(|key| key.replace("~1", "/").replace("~0", "~"))
        .collect();
    let last = keys.pop().expect("pointer names a key");
    let mut node = document;
    for key in keys {
        node = node
            .as_object_mut()
            .unwrap_or_else(|| panic!("{pointer}: a parent is not an object"))
            .entry(key)
            .or_insert_with(|| json!({}));
    }
    node.as_object_mut()
        .unwrap_or_else(|| panic!("{pointer}: the parent is not an object"))
        .insert(last, value);
}

#[test]
fn the_base_document_is_accepted() {
    let projection = discovery::project(base().to_string().as_bytes()).expect("base accepted");
    assert_eq!(projection.record.operations, vec!["fixture.things.get"]);
}

#[test]
fn discovery_refuses_unknown_key() {
    for pointer in [
        "/labels",
        "/resources/things/methods/get/etagRequired",
        "/resources/things/methods/get/parameters/thingId/location2",
        "/resources/things/methods/get/response/parameterName2",
        "/resources/things/extra",
        "/schemas/Thing/properties/name/location",
        "/schemas/Thing/pattern",
        "/auth/oauth2/scopes/https:~1~1www.googleapis.com~1auth~1fixture/title",
    ] {
        let mut document = base();
        set(&mut document, pointer, json!("x"));
        let refused = refusal(&document);
        assert_eq!(refused.pointer, pointer, "{refused}");
        assert_eq!(refused.reason, Reason::UnknownKey, "{refused}");
        assert!(refused.to_string().contains(pointer), "{refused}");
    }
}

#[test]
fn discovery_refuses_unresolved_ref() {
    for pointer in [
        "/resources/things/methods/get/response/$ref",
        "/schemas/Thing/properties/name/$ref",
    ] {
        let mut document = base();
        if pointer.starts_with("/schemas") {
            set(
                &mut document,
                "/schemas/Thing/properties/name",
                json!({"$ref": "Missing"}),
            );
        } else {
            set(&mut document, pointer, json!("Missing"));
        }
        let refused = refusal(&document);
        assert_eq!(refused.pointer, pointer, "{refused}");
        assert_eq!(
            refused.reason,
            Reason::UnresolvedRef("Missing".into()),
            "{refused}"
        );
        assert!(refused.to_string().contains(pointer), "{refused}");
    }
}

#[test]
fn discovery_refuses_non_https_root() {
    for root in [
        "http://fixture.googleapis.com/",
        "fixture.googleapis.com/",
        "https:///",
        "https://fixture.googleapis.com",
        "https://fixture.googleapis.com/?q=1/",
        "https://user@fixture.googleapis.com/",
    ] {
        let mut document = base();
        set(&mut document, "/rootUrl", json!(root));
        let refused = refusal(&document);
        assert_eq!(refused.pointer, "/rootUrl", "{root}: {refused}");
        assert_eq!(
            refused.reason,
            Reason::NotAbsoluteHttps,
            "{root}: {refused}"
        );
        assert!(refused.to_string().contains("/rootUrl"), "{refused}");
    }
}

/// The rest of the class "a value the rule table does not map": each is refused
/// by name at its pointer, never projected into something near it. `@` stands
/// for the base document's one method, `/resources/things/methods/get`.
#[test]
fn discovery_refuses_values_outside_the_table() {
    let cases: Vec<(&str, Value, &str)> = vec![
        ("/discoveryVersion", json!("v2"), "/discoveryVersion"),
        ("/kind", json!("discovery#directoryList"), "/kind"),
        ("/protocol", json!("rpc"), "/protocol"),
        ("/revision", json!(20260101), "/revision"),
        ("/servicePath", json!("/fixture/v1/"), "/servicePath"),
        (
            "/baseUrl",
            json!("https://elsewhere.googleapis.com/fixture/v1/"),
            "/baseUrl",
        ),
        ("/basePath", json!("/other/"), "/basePath"),
        ("@/httpMethod", json!("HEAD"), "@/httpMethod"),
        (
            "@/parameters/thingId/location",
            json!("header"),
            "@/parameters/thingId/location",
        ),
        (
            "@/parameters/thingId/required",
            json!(false),
            "@/parameters/thingId/required",
        ),
        (
            "@/parameters/thingId/format",
            json!("uuid"),
            "@/parameters/thingId/format",
        ),
        (
            "@/parameters/thingId/repeated",
            json!(true),
            "@/parameters/thingId/repeated",
        ),
        (
            "@/parameters/count",
            json!({"type": "integer", "location": "query", "minimum": "one"}),
            "@/parameters/count/minimum",
        ),
        (
            "@/parameters/flag",
            json!({"type": "boolean", "location": "query", "default": "yes"}),
            "@/parameters/flag/default",
        ),
        (
            "@/parameters/fields",
            json!({"type": "string", "location": "query"}),
            "@/parameters/fields",
        ),
        (
            "@/parameters/other",
            json!({"type": "string", "location": "path", "required": true}),
            "@/parameters/other",
        ),
        ("@/path", json!("things/{thingId}/{undeclared}"), "@/path"),
        ("@/path", json!("things/{/thingId}"), "@/path"),
        ("@/path", json!("/things/{thingId}"), "@/path"),
        (
            "@/scopes",
            json!(["https://www.googleapis.com/auth/undeclared"]),
            "@/scopes/0",
        ),
        (
            "@/supportsMediaUpload",
            json!(true),
            "@/supportsMediaUpload",
        ),
        (
            "/schemas/Thing/properties/name/format",
            json!("google-color"),
            "/schemas/Thing/properties/name/format",
        ),
        (
            "/schemas/Thing/properties/name/type",
            json!("null"),
            "/schemas/Thing/properties/name/type",
        ),
        ("/schemas/Thing/id", json!("Other"), "/schemas/Thing/id"),
        (
            "/schemas/Thing/annotations",
            json!({"optional": ["fixture.things.get"]}),
            "/schemas/Thing/annotations/optional",
        ),
        (
            "/schemas/Thing/properties/list",
            json!({"type": "array"}),
            "/schemas/Thing/properties/list",
        ),
        (
            "/parameters/tenant",
            json!({"type": "string", "location": "query"}),
            "/parameters/tenant",
        ),
    ];
    let method = |pointer: &str| pointer.replacen('@', "/resources/things/methods/get", 1);
    for (pointer, value, refused_at) in cases {
        let (pointer, refused_at) = (method(pointer), method(refused_at));
        let mut document = base();
        set(&mut document, &pointer, value.clone());
        let refused = refusal(&document);
        assert_eq!(
            refused.pointer, refused_at,
            "{pointer} = {value}: {refused}"
        );
        assert!(refused.to_string().contains(&refused_at), "{refused}");
    }
}

/// Two methods on one path and one HTTP method cannot both be operations, and
/// two methods with one id cannot both be named: either refuses the document,
/// rather than merging the two or dropping one.
#[test]
fn discovery_refuses_colliding_operations() {
    let mut document = base();
    set(
        &mut document,
        "/resources/things/methods/fetch",
        json!({
            "id": "fixture.things.fetch",
            "path": "things/{thingId}",
            "httpMethod": "GET",
            "parameters": {"thingId": {"type": "string", "location": "path", "required": true}}
        }),
    );
    let refused = refusal(&document);
    // Whichever of the two the walk reaches second is the one refused.
    assert!(
        refused.pointer == "/resources/things/methods/fetch/path"
            || refused.pointer == "/resources/things/methods/get/path",
        "{refused}"
    );
    let mut document = base();
    set(
        &mut document,
        "/resources/others",
        json!({"methods": {"get": {
            "id": "fixture.things.get",
            "path": "others",
            "httpMethod": "GET"
        }}}),
    );
    let refused = refusal(&document);
    assert!(
        refused.pointer == "/resources/others/methods/get/id"
            || refused.pointer == "/resources/things/methods/get/id",
        "{refused}"
    );
}

/// Which spelling of a name-only collision is projected does not depend on the
/// order the document declares the methods in.
#[test]
fn discovery_name_only_collision_is_order_independent() {
    let source = read(&fixtures().join("excluded_methods/discovery.json"));
    let mut document: Value = serde_json::from_slice(&source).unwrap();
    // Swap the resource names, so a walk in key order meets the spellings the
    // other way round.
    let resources = document["resources"].as_object_mut().unwrap();
    let things = resources.remove("things").unwrap();
    let others = resources.remove("others").unwrap();
    resources.insert("aaa".into(), things);
    resources.insert("zzz".into(), others);
    for method in ["get", "delete"] {
        let methods = document["resources"]["aaa"]["methods"]
            .as_object_mut()
            .unwrap();
        let moved = methods.remove(method).unwrap();
        methods.insert(format!("z{method}"), moved);
    }
    let reordered = discovery::project(document.to_string().as_bytes()).unwrap();
    let original = discovery::project(&source).unwrap();
    assert_eq!(reordered.openapi, original.openapi);
    assert_eq!(
        reordered.record.excluded_methods,
        original.record.excluded_methods
    );
}

#[test]
fn discovery_refuses_malformed_json() {
    let refused = discovery::project(br#"{"kind": "a", "kind": "b"}"#).unwrap_err();
    assert_eq!(refused.pointer, "");
    assert_eq!(refused.reason, Reason::Malformed);
    let refused = discovery::project(b"[]").unwrap_err();
    assert_eq!(refused.pointer, "");
}

#[test]
fn google_sources_pinned() {
    let root = upstream();
    assert!(
        root.join("LICENSE").is_file(),
        "the upstream LICENSE is pinned"
    );
    let license = String::from_utf8(read(&root.join("LICENSE"))).unwrap();
    assert!(license.contains("Redistribution and use in source and binary forms"));
    for (api, file) in PINNED {
        let directory = root.join(api);
        let bytes = read(&directory.join(file));
        let digest = sha256(&bytes);
        let archived = std::process::Command::new("gzip")
            .arg("-dc")
            .arg(directory.join("vendor").join(format!("{file}.gz")))
            .output()
            .expect("gzip runs");
        assert!(archived.status.success(), "{api}: archive decompresses");
        assert_eq!(
            archived.stdout, bytes,
            "{api}: the archive holds the pinned bytes"
        );
        let manifest = json(&directory.join(format!("{api}-source-hashes.json")));
        let records = manifest.as_array().expect("manifest is a list");
        assert_eq!(records.len(), 1, "{api}: one pinned document");
        assert_eq!(records[0]["file"], file, "{api}");
        assert_eq!(records[0]["sha256"], digest.as_str(), "{api}");
        assert_eq!(records[0]["bytes"], bytes.len(), "{api}");
        assert_eq!(records[0]["archive"], format!("vendor/{file}.gz"), "{api}");
        let url = records[0]["url"].as_str().expect("url");
        assert!(
            url.contains("ec13a0cd76ecc7fede8932d040a3156804515da9"),
            "{api}: the url names the pinned commit"
        );
        let readme = String::from_utf8(read(&directory.join("README.md"))).unwrap();
        assert!(readme.contains(&digest), "{api}: README names the digest");
        assert!(readme.contains(url), "{api}: README names the url");
        let size = bytes.len().to_string();
        let grouped = {
            let mut out = String::new();
            for (index, digit) in size.chars().enumerate() {
                if index > 0 && (size.len() - index).is_multiple_of(3) {
                    out.push(',');
                }
                out.push(digit);
            }
            out
        };
        assert!(
            readme.contains(&grouped),
            "{api}: README names the size {grouped}"
        );
        let revision = serde_json::from_slice::<Value>(&bytes).unwrap()["revision"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(
            readme.contains(&revision),
            "{api}: README names the revision"
        );
    }
}

#[test]
fn discovery_conserves_methods() {
    for (api, file) in PINNED {
        let document: Value = serde_json::from_slice(&pinned(api, file)).unwrap();
        let methods = discovery_methods(&document);
        let projection = projected(api, file);
        let record = &projection.record;
        println!(
            "{api}: {} Discovery methods, {} projected, {} excluded",
            methods.len(),
            record.operations.len(),
            record.excluded_methods.len()
        );
        assert_eq!(record.method_count, methods.len(), "{api}");
        assert_eq!(
            record.operations.len() + record.excluded_methods.len(),
            methods.len(),
            "{api}"
        );
        let mut named: Vec<String> = record
            .operations
            .iter()
            .cloned()
            .chain(record.excluded_methods.iter().map(|e| e.id.clone()))
            .collect();
        named.sort();
        assert_eq!(named, methods, "{api}: every method is named exactly once");
        // The output carries exactly the projected operations.
        let openapi: Value = serde_json::from_slice(&projection.openapi).unwrap();
        let mut ids: Vec<String> = openapi["paths"]
            .as_object()
            .unwrap()
            .values()
            .flat_map(|item| item.as_object().unwrap().values())
            .map(|operation| operation["operationId"].as_str().unwrap().to_owned())
            .collect();
        ids.sort();
        let mut operations = record.operations.clone();
        operations.sort();
        assert_eq!(ids, operations, "{api}");
    }
}

#[test]
fn discovery_refs_resolve() {
    fn refs(value: &Value, at: String, out: &mut Vec<(String, String)>) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    if key == "$ref" {
                        out.push((at.clone(), child.as_str().expect("$ref is a string").into()));
                    }
                    refs(child, format!("{at}/{key}"), out);
                }
            }
            Value::Array(items) => {
                for (index, child) in items.iter().enumerate() {
                    refs(child, format!("{at}/{index}"), out);
                }
            }
            _ => {}
        }
    }
    for (api, file) in PINNED {
        let openapi: Value = serde_json::from_slice(&projected(api, file).openapi).unwrap();
        let mut found = Vec::new();
        refs(&openapi, String::new(), &mut found);
        assert!(!found.is_empty(), "{api}: the output carries references");
        for (at, reference) in found {
            let target = reference
                .strip_prefix("#/components/schemas/")
                .unwrap_or_else(|| panic!("{api}: {at} = {reference} is not a schema ref"));
            assert!(
                openapi["components"]["schemas"].get(target).is_some(),
                "{api}: {at} = {reference} does not resolve"
            );
        }
    }
}

#[test]
fn discovery_no_credential_parameters() {
    for (api, file) in PINNED {
        let projection = projected(api, file);
        let openapi: Value = serde_json::from_slice(&projection.openapi).unwrap();
        let mut seen = 0usize;
        for item in openapi["paths"].as_object().unwrap().values() {
            for operation in item.as_object().unwrap().values() {
                for parameter in operation["parameters"].as_array().into_iter().flatten() {
                    seen += 1;
                    let name = parameter["name"].as_str().unwrap();
                    assert!(
                        !CREDENTIAL_PARAMETERS.contains(&name),
                        "{api}: {} carries `{name}`",
                        operation["operationId"]
                    );
                }
            }
        }
        assert!(seen > 0, "{api}: the walk read parameters");
        // Every credential parameter the document declares is listed as excluded;
        // Calendar, for one, declares no `access_token`, so none is listed for it.
        let document: Value = serde_json::from_slice(&pinned(api, file)).unwrap();
        let declared = document["parameters"]
            .as_object()
            .expect("document parameters");
        let mut listed = 0usize;
        for name in CREDENTIAL_PARAMETERS {
            let excluded = projection
                .record
                .excluded_parameters
                .iter()
                .any(|p| p == name);
            assert_eq!(
                excluded,
                declared.contains_key(name),
                "{api}: `{name}` is listed as excluded exactly when declared"
            );
            listed += usize::from(excluded);
        }
        assert!(listed >= 2, "{api}: the walk read the document parameters");
    }
}

#[test]
fn discovery_is_deterministic() {
    for (api, file) in PINNED {
        let first = projected(api, file);
        let second = projected(api, file);
        assert_eq!(first.openapi, second.openapi, "{api}");
        assert_eq!(first.record_bytes(), second.record_bytes(), "{api}");
        // Canonical: sorted keys at every depth, pretty-printed, one final newline.
        assert!(
            first.openapi == canonical_pretty(&first.openapi),
            "{api}: output is not canonical"
        );
        assert!(
            first.record_bytes() == canonical_pretty(&first.record_bytes()),
            "{api}: record is not canonical"
        );
        assert_eq!(first.record.projector, PROJECTOR);
    }
}

#[test]
fn discovery_openapi_parses_independently() {
    for (api, file) in PINNED {
        let projection = projected(api, file);
        let parsed: openapiv3::OpenAPI = serde_json::from_slice(&projection.openapi)
            .unwrap_or_else(|e| panic!("{api}: openapiv3 refuses the projection: {e}"));
        assert_eq!(parsed.openapi, "3.0.3", "{api}");
        assert_eq!(
            parsed.operations().count(),
            projection.record.operations.len(),
            "{api}"
        );
        assert_eq!(parsed.servers.len(), 1, "{api}");
        assert!(parsed.servers[0].url.starts_with("https://"), "{api}");
    }
}

#[test]
fn discovery_ingests() {
    for (api, file) in PINNED {
        let projection = projected(api, file);
        let source = ingest(&format!("{api}-openapi.json"), &projection.openapi)
            .unwrap_or_else(|refusal| panic!("{api}: ingest refuses: {}", refusal.reason()));
        assert_eq!(source.openapi, "3.0.3", "{api}");
        let document = parse_document(&projection.openapi).unwrap();
        let inventory = inventory::extract(&document);
        assert_eq!(inventory.unsupported, vec![], "{api}");
        let mut inventoried: Vec<String> = inventory
            .operations
            .iter()
            .map(|operation| operation.operation_id.clone().expect("operation id"))
            .collect();
        inventoried.sort();
        let mut operations = projection.record.operations.clone();
        operations.sort();
        assert_eq!(inventoried, operations, "{api}");
    }
}

/// Adversary pass 1 tightened `rootUrl` and integer ranges; values at the edge
/// of what is allowed still project, with the value written as Discovery wrote it.
#[test]
fn discovery_accepts_values_at_their_edges() {
    let mut document = base();
    set(
        &mut document,
        "/rootUrl",
        json!("https://fixture.googleapis.com:8443/"),
    );
    let parameters = "/resources/things/methods/get/parameters";
    let edges = [
        (
            "a",
            json!({"type": "integer", "format": "int32", "location": "query",
                     "minimum": "-2147483648", "maximum": "2147483647"}),
        ),
        (
            "b",
            json!({"type": "integer", "format": "uint32", "location": "query",
                     "minimum": "0", "default": "4294967295"}),
        ),
        (
            "c",
            json!({"type": "string", "format": "int64", "location": "query",
                     "default": "-9223372036854775808"}),
        ),
        (
            "d",
            json!({"type": "string", "format": "uint64", "location": "query",
                     "default": "18446744073709551615"}),
        ),
        (
            "e",
            json!({"type": "number", "format": "float", "location": "query",
                     "maximum": "3.4028235e38"}),
        ),
    ];
    for (name, value) in edges {
        set(&mut document, &format!("{parameters}/{name}"), value);
    }
    let projection = discovery::project(document.to_string().as_bytes())
        .unwrap_or_else(|refusal| panic!("edge values refused: {refusal}"));
    let openapi: Value = serde_json::from_slice(&projection.openapi).unwrap();
    assert_eq!(
        openapi["servers"][0]["url"],
        concat!("https://fixture.googleapis.com", ":8443", "/fixture/v1")
    );
    let schema = |name: &str| -> Value {
        openapi["paths"]["/things/{thingId}"]["get"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap_or_else(|| panic!("{name} projected"))["schema"]
            .clone()
    };
    assert_eq!(schema("a")["minimum"], json!(-2147483648i64));
    assert_eq!(schema("a")["maximum"], json!(2147483647));
    assert_eq!(schema("b")["default"], json!(4294967295u64));
    assert_eq!(schema("c")["default"], json!("-9223372036854775808"));
    assert_eq!(schema("d")["default"], json!("18446744073709551615"));
    assert_eq!(schema("e")["maximum"], json!(3.4028235e38));
}

/// The rest of the class adversary pass 1 found in `int32`/`uint32`: a value
/// that contradicts its own `format` is refused at its pointer, for the string
/// integer formats and for `float` as well.
#[test]
fn discovery_refuses_values_outside_their_format() {
    let parameter = "/resources/things/methods/get/parameters/count";
    let cases = [
        (
            json!({"type": "string", "format": "int64", "location": "query",
                "default": "9223372036854775808"}),
            "default",
        ),
        (
            json!({"type": "string", "format": "uint64", "location": "query",
                "default": "-1"}),
            "default",
        ),
        (
            json!({"type": "string", "format": "int64", "location": "query",
                "default": "ten"}),
            "default",
        ),
        (
            json!({"type": "number", "format": "float", "location": "query",
                "maximum": "1e39"}),
            "maximum",
        ),
        (
            json!({"type": "integer", "format": "int32", "location": "query",
                "minimum": "1.5"}),
            "minimum",
        ),
    ];
    for (value, key) in cases {
        let mut document = base();
        set(&mut document, parameter, value.clone());
        let refused = refusal(&document);
        assert_eq!(
            refused.pointer,
            format!("{parameter}/{key}"),
            "{value}: {refused}"
        );
    }
    for root in [
        "https://a..b/",
        "https://-fixture.googleapis.com/",
        "https://fixture.googleapis.com:/",
    ] {
        let mut document = base();
        set(&mut document, "/rootUrl", json!(root));
        let refused = refusal(&document);
        assert_eq!(
            refused.reason,
            Reason::NotAbsoluteHttps,
            "{root}: {refused}"
        );
    }
}

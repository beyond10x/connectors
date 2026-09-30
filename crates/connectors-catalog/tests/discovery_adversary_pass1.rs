//! Adversary pass 1 against the Google Discovery projection
//! (`story:catalog-discovery-projection`). Each case asserts a promise the
//! projector's own module documentation (`src/discovery.rs:1-54`) or the story's
//! rule table makes, over the pinned documents or a hand-made fragment.
use connectors_catalog::discovery::{self, Reason, Refusal};
use serde_json::{Value, json};
use std::path::Path;

fn pinned(api: &str, file: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../adapters/google/upstream")
        .join(api)
        .join(file);
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The same smallest document `tests/discovery.rs` breaks.
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
        "resources": {
            "things": {
                "methods": {
                    "get": {
                        "id": "fixture.things.get",
                        "path": "things/{thingId}",
                        "httpMethod": "GET",
                        "parameters": {
                            "thingId": {"type": "string", "location": "path", "required": true}
                        }
                    }
                }
            }
        }
    })
}

fn project(document: &Value) -> Result<discovery::Projection, Refusal> {
    discovery::project(document.to_string().as_bytes())
}

/// Every method pointer (`/resources/.../methods/<name>`) in a Discovery document.
fn methods(node: &Value, at: &str, out: &mut Vec<(String, Value)>) {
    let escape = |key: &str| key.replace('~', "~0").replace('/', "~1");
    if let Some(map) = node.get("methods").and_then(Value::as_object) {
        for (name, method) in map {
            out.push((format!("{at}/methods/{}", escape(name)), method.clone()));
        }
    }
    if let Some(map) = node.get("resources").and_then(Value::as_object) {
        for (name, resource) in map {
            methods(resource, &format!("{at}/resources/{}", escape(name)), out);
        }
    }
}

/// `src/discovery.rs:7-10`: "Every Discovery key is read by one row below, or the
/// document is refused ... nothing is dropped silently. What a row does not carry
/// into the output is named in the ProjectionRecord instead."
///
/// `mediaUpload.accept`, `mediaUpload.maxSize` and `protocols.<p>.multipart` are
/// read (`src/discovery.rs:1169-1181`) and then neither written to the output nor
/// named in `ignored_keys`; `excluded_upload_paths` carries only the protocol and
/// path. Drive and Gmail reach this on eight pinned methods.
#[test]
fn adversary_media_upload_keys_are_named_not_dropped() {
    let mut missing = Vec::new();
    for (api, file) in [("drive", "drive-api.json"), ("gmail", "gmail-api.json")] {
        let bytes = pinned(api, file);
        let document: Value = serde_json::from_slice(&bytes).unwrap();
        let projection = discovery::project(&bytes).expect("pinned document projects");
        let mut found = Vec::new();
        methods(&document, "", &mut found);
        let mut reached = 0usize;
        for (at, method) in found {
            let Some(media) = method.get("mediaUpload") else {
                continue;
            };
            reached += 1;
            let mut dropped = vec![
                format!("{at}/mediaUpload/accept"),
                format!("{at}/mediaUpload/maxSize"),
            ];
            for protocol in media["protocols"].as_object().unwrap().keys() {
                dropped.push(format!("{at}/mediaUpload/protocols/{protocol}/multipart"));
            }
            for pointer in dropped {
                if !projection.record.ignored_keys.contains(&pointer) {
                    missing.push(format!("{api}: {pointer}"));
                }
            }
        }
        assert!(
            reached > 0,
            "{api}: the pinned document carries media upload"
        );
    }
    assert!(
        missing.is_empty(),
        "{} Discovery keys read and dropped without a record entry:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );
}

/// `src/discovery.rs:27`: `minimum`/`maximum`/`default` "become the schema type's
/// own values". A `number` bound must keep the value Discovery wrote. The
/// projector's `canonical` (`src/discovery.rs:312-318`) re-reads its own output
/// with `serde_json::from_slice`, whose float parse depends on the crate graph's
/// serde_json features: in this crate's graph (no `float_roundtrip`) the value
/// comes back one ULP off, while the `connectors-build` graph (which enables
/// `float_roundtrip` and `arbitrary_precision`) writes it exactly — the same
/// Discovery bytes project to different OpenAPI bytes by build.
#[test]
fn adversary_number_bound_keeps_its_value() {
    let written = "123456789.12345679";
    let mut document = base();
    document["resources"]["things"]["methods"]["get"]["parameters"]["ratio"] =
        json!({"type": "number", "location": "query", "minimum": written, "default": written});
    let projection = project(&document).expect("projects");
    let openapi: Value = serde_json::from_slice(&projection.openapi).unwrap();
    let parameter = openapi["paths"]["/things/{thingId}"]["get"]["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "ratio")
        .expect("ratio projected")
        .clone();
    let text = String::from_utf8(projection.openapi.clone()).unwrap();
    let line = text
        .lines()
        .find(|line| line.contains("\"minimum\""))
        .unwrap()
        .trim()
        .to_owned();
    assert_eq!(
        line,
        format!("\"minimum\": {written},"),
        "the projected bound is not the Discovery value (parameter: {parameter})"
    );
}

/// `src/discovery.rs:50`: "two methods on one path and one HTTP method ... refused".
/// OpenAPI 3.0.3 treats `/things/{thingId}` and `/things/{otherId}` as one path.
/// Spelled with the same template name, the second GET is refused
/// (`discovery_refuses_colliding_operations`); spelled with another name, the same
/// collision is turned into an exclusion (`select`, `src/discovery.rs:960-972`)
/// and the document projects.
#[test]
fn adversary_second_get_on_a_renamed_template_is_refused() {
    let mut document = base();
    document["resources"]["things"]["methods"]["fetch"] = json!({
        "id": "fixture.things.fetch",
        "path": "things/{otherId}",
        "httpMethod": "GET",
        "parameters": {"otherId": {"type": "string", "location": "path", "required": true}}
    });
    match project(&document) {
        Ok(projection) => panic!(
            "two GET methods on one OpenAPI path were accepted; excluded: {:?}",
            projection.record.excluded_methods
        ),
        Err(refused) => assert!(
            refused.pointer.starts_with("/resources/things/methods/"),
            "{refused}"
        ),
    }
}

/// Story rule table: `rootUrl` "absolute https; anything else refused";
/// `Reason::NotAbsoluteHttps` promises `https://<host>/`. The authority filter
/// (`src/discovery.rs:783-791`) admits any mix of alphanumerics, `.`, `-` and `:`,
/// so an empty host, a dot host and a non-numeric port become the server URL.
#[test]
fn adversary_root_url_without_a_host_is_refused() {
    let mut accepted = Vec::new();
    for root in [
        "https://:/",
        "https://../",
        "https://fixture.googleapis.com:port/",
        "https://fixture.googleapis.com::/",
    ] {
        let mut document = base();
        document["rootUrl"] = json!(root);
        match project(&document) {
            Ok(projection) => {
                let openapi: Value = serde_json::from_slice(&projection.openapi).unwrap();
                accepted.push(format!("{root} -> {}", openapi["servers"][0]["url"]));
            }
            Err(refused) => {
                assert_eq!(refused.pointer, "/rootUrl", "{root}: {refused}");
                assert_eq!(
                    refused.reason,
                    Reason::NotAbsoluteHttps,
                    "{root}: {refused}"
                );
            }
        }
    }
    assert!(accepted.is_empty(), "accepted as server URLs: {accepted:?}");
}

/// `src/discovery.rs:19`: `basePath` "must equal ... `/` + servicePath". The check
/// (`src/discovery.rs:821-825`) strips every leading `/`, so a basePath with none
/// or with several is accepted.
#[test]
fn adversary_base_path_is_one_slash_and_the_service_path() {
    let mut accepted = Vec::new();
    for path in ["fixture/v1/", "//fixture/v1/", "///fixture/v1/"] {
        let mut document = base();
        document["basePath"] = json!(path);
        match project(&document) {
            Ok(_) => accepted.push(path),
            Err(refused) => assert_eq!(refused.pointer, "/basePath", "{path}: {refused}"),
        }
    }
    assert!(
        accepted.is_empty(),
        "basePath values accepted: {accepted:?}"
    );
}

/// Story rule table: parameter `format`, `minimum`, `maximum`, `default` map "one
/// to one", and "a non-numeric value is refused". A bound or default outside the
/// declared integer format (`int32`, `uint32`) is projected as written
/// (`typed`, `src/discovery.rs:455-459`, parses any i64/u64), producing a schema
/// that contradicts its own `format`.
#[test]
fn adversary_integer_values_fit_their_format() {
    let cases = [
        ("int32", "maximum", "4294967296"),
        ("int32", "default", "-2147483649"),
        ("uint32", "minimum", "-1"),
        ("uint32", "default", "4294967296"),
    ];
    let mut accepted = Vec::new();
    for (format, key, value) in cases {
        let mut document = base();
        document["resources"]["things"]["methods"]["get"]["parameters"]["count"] =
            json!({"type": "integer", "format": format, "location": "query", key: value});
        match project(&document) {
            Ok(_) => accepted.push(format!("{format} {key}={value}")),
            Err(refused) => assert_eq!(
                refused.pointer,
                format!("/resources/things/methods/get/parameters/count/{key}"),
                "{refused}"
            ),
        }
    }
    assert!(
        accepted.is_empty(),
        "out-of-format integers projected: {accepted:?}"
    );
}

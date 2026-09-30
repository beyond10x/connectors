//! Adversary pass 2 against the Google Discovery projection
//! (`story:catalog-discovery-projection`), over the correction round
//! `34a954c5c...628638db1`. Each case asserts a promise the projector's own
//! documentation makes, on a hand-made fragment.
use connectors_catalog::discovery::{self, Refusal};
use serde_json::{Value, json};

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

/// A method on `things/{<name>}`.
fn on(name: &str, id: &str, verb: &str) -> Value {
    json!({
        "id": id,
        "path": format!("things/{{{name}}}"),
        "httpMethod": verb,
        "parameters": {name: {"type": "string", "location": "path", "required": true}}
    })
}

/// `src/discovery.rs:49-51`: "paths that differ only in template names, with the
/// same HTTP method | refused: one OpenAPI path and one HTTP method" and "two
/// methods on one path and one HTTP method ... refused".
///
/// `select` (`src/discovery.rs:1030-1045`) only compares a method on a non-kept
/// spelling against the kept spelling. Two PUT methods that both sit off the
/// kept spelling — on one spelling, or on two — never meet each other and are
/// both excluded, so the document projects.
#[test]
fn adversary2_same_verb_off_the_kept_spelling_is_refused() {
    let variants = [
        ("one unkept spelling", "aId", "aId"),
        ("two unkept spellings", "aId", "bId"),
    ];
    let mut accepted = Vec::new();
    for (label, first, second) in variants {
        let mut document = base();
        let methods = &mut document["resources"]["things"]["methods"];
        // The kept spelling `{thingId}` carries GET, POST and DELETE: three methods.
        methods["create"] = on("thingId", "fixture.things.create", "POST");
        methods["delete"] = on("thingId", "fixture.things.delete", "DELETE");
        methods["renameA"] = on(first, "fixture.things.renameA", "PUT");
        methods["renameB"] = on(second, "fixture.things.renameB", "PUT");
        match project(&document) {
            Ok(projection) => accepted.push(format!(
                "{label}: excluded {:?}",
                projection
                    .record
                    .excluded_methods
                    .iter()
                    .map(|m| m.id.as_str())
                    .collect::<Vec<_>>()
            )),
            Err(refused) => assert!(
                refused
                    .pointer
                    .starts_with("/resources/things/methods/rename"),
                "{label}: {refused}"
            ),
        }
    }
    assert!(
        accepted.is_empty(),
        "two PUT methods on one OpenAPI path were accepted: {accepted:?}"
    );
}

/// `src/discovery.rs:465-469` (`typed`, written in the correction round): "A value
/// outside its format is refused, so no schema contradicts its own `format`."
/// Only the integer formats and `float` are checked; a `default` on a string
/// `date-time`, `date` or `byte` is projected whatever it holds.
#[test]
fn adversary2_string_format_defaults_fit_their_format() {
    let cases = [
        ("google-datetime", "yesterday"),
        ("date-time", "yesterday"),
        ("date", "soon"),
        ("byte", "!!not base64!!"),
    ];
    let mut accepted = Vec::new();
    for (format, value) in cases {
        let mut document = base();
        document["resources"]["things"]["methods"]["get"]["parameters"]["when"] = json!({
            "type": "string", "format": format, "location": "query", "default": value
        });
        match project(&document) {
            Ok(_) => accepted.push(format!("{format} default={value}")),
            Err(refused) => assert_eq!(
                refused.pointer, "/resources/things/methods/get/parameters/when/default",
                "{refused}"
            ),
        }
    }
    assert!(
        accepted.is_empty(),
        "defaults that contradict their format projected: {accepted:?}"
    );
}

/// `src/discovery.rs:267`: `UploadPath` is "A media upload protocol path,
/// excluded; its method's metadata path is projected", and the module table
/// (`src/discovery.rs:35`) says the same. A method excluded for a name-only
/// collision still contributes its upload paths (`method` pushes them before
/// `select` runs), so the record names an upload path whose metadata path was
/// not projected — while the same method's `{+x}` rewrite is dropped
/// (`select` extends `rewritten` only for kept methods).
#[test]
fn adversary2_upload_paths_belong_to_projected_methods() {
    let mut document = base();
    let methods = &mut document["resources"]["things"]["methods"];
    methods["delete"] = on("thingId", "fixture.things.delete", "DELETE");
    let mut upload = on("otherId", "fixture.things.update", "PUT");
    upload["supportsMediaUpload"] = json!(true);
    upload["mediaUpload"] = json!({
        "protocols": {"simple": {"path": "/upload/fixture/v1/things/{otherId}"}}
    });
    methods["update"] = upload;
    let projection = project(&document).unwrap_or_else(|refused| panic!("refused: {refused}"));
    let record = &projection.record;
    assert_eq!(
        record
            .excluded_methods
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        ["fixture.things.update"],
        "precondition: the PUT is excluded for its name-only collision"
    );
    let orphaned: Vec<&str> = record
        .excluded_upload_paths
        .iter()
        .filter(|u| !record.operations.contains(&u.operation_id))
        .map(|u| u.operation_id.as_str())
        .collect();
    assert!(
        orphaned.is_empty(),
        "upload paths listed for methods that were not projected: {orphaned:?}"
    );
}

/// `src/discovery.rs:7-10`: nothing is dropped silently; what a row does not carry
/// is named in the record. `supportsMediaUpload: false` is listed
/// (`src/discovery.rs:1281-1283`), and each protocol path is listed, but
/// `supportsMediaUpload: true` with `protocols: {}` leaves no trace in the
/// output or the record.
#[test]
fn adversary2_media_upload_without_protocols_leaves_a_trace() {
    let mut document = base();
    let method = &mut document["resources"]["things"]["methods"]["get"];
    method["supportsMediaUpload"] = json!(true);
    method["mediaUpload"] = json!({"protocols": {}});
    let projection = match project(&document) {
        Ok(projection) => projection,
        // A refusal is also an answer: the document is not silently reduced.
        Err(refused) => {
            assert!(
                refused.pointer.contains("/methods/get/"),
                "refused at the wrong pointer: {refused}"
            );
            return;
        }
    };
    let record = &projection.record;
    let traced = !record.excluded_upload_paths.is_empty()
        || record.ignored_keys.iter().any(|key| {
            key.contains("/methods/get/supportsMediaUpload")
                || key.contains("/methods/get/mediaUpload")
        });
    assert!(
        traced,
        "supportsMediaUpload: true with no protocols left no trace; ignored_keys: {:?}",
        record.ignored_keys
    );
}

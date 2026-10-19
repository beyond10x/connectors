//! Adversary cases for story:write-postflight-present-check in the definition
//! format: `"?"` lowers to `{"present": true}` wherever an `expect` or `any`
//! sits, `=?` stays the literal `?`, a YAML `?` that is not the string `"?"`
//! never lowers silently, and the importer refuses an input reference `?` it
//! could not write back.
#![cfg(unix)]
use connectors_catalog::definition::{self, Refusal, Request};
use connectors_catalog::import;
use serde_json::{Value, json};
use std::io::Write as _;
use std::path::{Path, PathBuf};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

const HEAD: &str = r#"schema: urn:beyond10x:connectors-definition:1
source:
  openapi: upstream/zendesk/zendesk-support.yaml.gz
  sha256: 5dd6cf1eda8febf02f8fb2f2cfa72d7cb6bc7d5bab188ba97b49ab55fc51dbf2
  amendments: upstream/zendesk/zendesk-support.amendments.json
base_url: https://{subdomain}.zendesk.com
config: {subdomain: {type: string}}
auth:
  zendesk.basic: {scheme: basic, label: API token, account_label: Account email followed by /token, identity: {path: api/v2/users/me, kind: zendesk.user, subject: /user/id}}
operations:
  ticket.show: {op: ShowTicket}
  probe.write:
    op: UpdateTicket
    effect: write
    body: {/ticket/type: string, /ticket/updated_stamp: string}
    guard:
"#;

/// Lower the definition whose guard is `guard` (already indented), the
/// lowered `probe.write` entry or the refusal.
fn lower(guard: &str) -> Result<Value, Refusal> {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let upstream = root.join("upstream/zendesk");
    std::fs::create_dir_all(&upstream).unwrap();
    let source = repository().join("adapters/zendesk/upstream");
    std::fs::write(
        upstream.join("zendesk-support.yaml.gz"),
        gzip(&std::fs::read(source.join("zendesk-support.yaml")).unwrap()),
    )
    .unwrap();
    std::fs::copy(
        source.join("zendesk-support.amendments.json"),
        upstream.join("zendesk-support.amendments.json"),
    )
    .unwrap();
    let providers = temp.path().join("providers");
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&providers).unwrap();
    std::fs::create_dir_all(&scratch).unwrap();
    let file = providers.join("zendesk.yaml");
    std::fs::write(&file, format!("{HEAD}{guard}")).unwrap();
    let lowered = definition::lower(&Request {
        definition: &file,
        root: &root,
        out: &temp.path().join("out"),
        scratch: &scratch,
    })?;
    let selection: Value =
        serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap();
    Ok(selection["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == "probe.write")
        .unwrap()
        .clone())
}

#[test]
fn a_question_mark_lowers_to_a_presence_in_before_after_and_any() {
    let entry = lower(concat!(
        "      before: {read: ticket.show, expect: {/ticket/updated_at: body.ticket.updated_stamp, /ticket/id: \"?\"}}\n",
        "      after: {expect: {/ticket/id: \"?\"}, any: {/ticket/status: \"?\", /ticket/type: \"=?\"}}\n",
    ))
    .unwrap_or_else(|refusal| panic!("the definition lowers: {refusal:?}"));
    let guard = &entry["guard"];
    assert_eq!(
        guard["preflight"]["checks"],
        json!([{"pointer": "/ticket/updated_at", "expect": {"input": "body.ticket.updated_stamp"}},
               {"pointer": "/ticket/id", "expect": {"present": true}}])
    );
    assert_eq!(
        guard["postflight"]["checks"],
        json!([{"pointer": "/ticket/id", "expect": {"present": true}}])
    );
    assert_eq!(
        guard["postflight"]["any_of"],
        json!([{"pointer": "/ticket/status", "expect": {"present": true}},
               {"pointer": "/ticket/type", "expect": {"literal": "?"}}])
    );
}

/// A YAML `?` written without quotes is YAML's explicit-key indicator, not the
/// string `"?"`. Whatever the parser makes of it, it must not lower to an input
/// reference or a literal: it is refused, or it is the presence it reads as.
#[test]
fn an_unquoted_question_mark_never_lowers_to_another_expectation() {
    for (case, guard) in [
        (
            "flow",
            "      before: {read: ticket.show, expect: {/ticket/updated_at: body.ticket.updated_stamp}}\n      after: {expect: {/ticket/id: ?}}\n",
        ),
        (
            "block",
            "      before: {read: ticket.show, expect: {/ticket/updated_at: body.ticket.updated_stamp}}\n      after:\n        expect:\n          /ticket/id: ?\n",
        ),
    ] {
        match lower(guard) {
            Err(_) => {}
            Ok(entry) => assert_eq!(
                entry["guard"]["postflight"]["checks"],
                json!([{"pointer": "/ticket/id", "expect": {"present": true}}]),
                "{case}"
            ),
        }
    }
}

/// The importer cannot write `{"input": "?"}` back as `?`, which now reads as
/// a presence, so it refuses it rather than turning an input comparison into a
/// presence on the round trip.
#[test]
fn an_import_refuses_an_input_reference_named_question_mark() {
    let temp = tempfile::tempdir().unwrap();
    let mut shipped: Value = serde_json::from_slice(
        &std::fs::read(repository().join("adapters/catalog/providers/zendesk/operations.json"))
            .unwrap(),
    )
    .unwrap();
    for entry in shipped["operations"].as_array_mut().unwrap() {
        if entry["id"] == "ticket.note" {
            entry["guard"]["postflight"]["checks"] =
                json!([{"pointer": "/ticket/id", "expect": {"input": "?"}}]);
        }
    }
    let selection = temp.path().join("operations.json");
    std::fs::write(&selection, serde_json::to_vec_pretty(&shipped).unwrap()).unwrap();
    let root = temp.path().join("definitions");
    std::fs::create_dir_all(&root).unwrap();
    let Err(failure) = import::import(&import::Request {
        connectors: &repository(),
        catalog: "zendesk",
        selection: Some(&selection),
        definitions: &root,
        name: Some("zendesk"),
    }) else {
        panic!("an input reference `?` imports");
    };
    assert!(failure.to_string().contains('?'), "{failure}");
}

/// A shipped `present` in a preflight and in `any_of` imports as `"?"` and
/// lowers back to the same selection.
#[test]
fn a_presence_in_a_preflight_and_any_of_round_trips_through_the_importer() {
    let temp = tempfile::tempdir().unwrap();
    let mut shipped: Value = serde_json::from_slice(
        &std::fs::read(repository().join("adapters/catalog/providers/zendesk/operations.json"))
            .unwrap(),
    )
    .unwrap();
    for entry in shipped["operations"].as_array_mut().unwrap() {
        if entry["id"] == "ticket.note" {
            entry["guard"]["preflight"]["checks"]
                .as_array_mut()
                .unwrap()
                .push(json!({"pointer": "/ticket/id", "expect": {"present": true}}));
            entry["guard"]["postflight"]["any_of"] = json!([
                {"pointer": "/ticket/status", "expect": {"present": true}},
                {"pointer": "/ticket/type", "expect": {"literal": "?"}}
            ]);
        }
    }
    let selection = temp.path().join("operations.json");
    std::fs::write(&selection, serde_json::to_vec_pretty(&shipped).unwrap()).unwrap();
    let root = temp.path().join("definitions");
    std::fs::create_dir_all(&root).unwrap();
    let imported = import::import(&import::Request {
        connectors: &repository(),
        catalog: "zendesk",
        selection: Some(&selection),
        definitions: &root,
        name: Some("zendesk"),
    })
    .unwrap_or_else(|failure| panic!("the edited selection imports: {failure}"));
    let out = temp.path().join("out");
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let lowered = definition::lower(&Request {
        definition: &imported.definition,
        root: &root,
        out: &out,
        scratch: &scratch,
    })
    .unwrap_or_else(|refusal| panic!("the imported definition lowers: {refusal}"));
    let back: Value = serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap();
    assert_eq!(back, shipped);
}

//! Adversary cases for the 0.54.0 write members of the definition format
//! (`body_enums`, `body_any_of`, `body_prefixes`, `conflict_status`, a
//! preflight `mismatch`, `string_list`): each holds the lowerer to the rule the
//! engine's loader (`adapters/catalog/src/closed.rs`, `check_conflict_status`)
//! applies to the selection it writes, and the importer to a round trip that
//! keeps every member.
#![cfg(unix)]
use connectors_catalog::definition::{self, Refusal, Request};
use connectors_catalog::import;
use serde_json::Value;
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

struct Lowering {
    _temp: tempfile::TempDir,
    outcome: Result<definition::Lowered, Refusal>,
}

impl Lowering {
    fn selection(&self) -> Value {
        let lowered = self.outcome.as_ref().expect("the definition lowers");
        serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap()
    }
    fn entry(&self, id: &str) -> Value {
        self.selection()["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == id)
            .unwrap_or_else(|| panic!("the selection lacks `{id}`"))
            .clone()
    }
    fn refusal(self) -> Refusal {
        self.outcome.expect_err("the definition is refused")
    }
}

fn lower(text: &str, root: &Path) -> Lowering {
    let temp = tempfile::tempdir().unwrap();
    let providers = temp.path().join("providers");
    std::fs::create_dir_all(&providers).unwrap();
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let file = providers.join("zendesk.yaml");
    std::fs::write(&file, text).unwrap();
    let outcome = definition::lower(&Request {
        definition: &file,
        root,
        out: &temp.path().join("out"),
        scratch: &scratch,
    });
    Lowering {
        _temp: temp,
        outcome,
    }
}

fn zendesk_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let upstream = root.path().join("upstream/zendesk");
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
    root
}

/// The definition's head and its read; each case appends one write.
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
"#;

/// A write of `UpdateTicket` keyed `probe.write`, its members as given.
fn with_write(members: &str) -> String {
    format!("{HEAD}  probe.write:\n    op: UpdateTicket\n    effect: write\n{members}")
}

const BODY: &str = "    body: {/ticket/group_id: integer, /ticket/tags: string_list, /ticket/priority: string, /ticket/type: string, /ticket/safe_update: =true}\n";

fn lowered(members: &str) -> Lowering {
    let root = zendesk_root();
    let lowering = lower(&with_write(members), root.path());
    drop(root);
    lowering
}

fn enum_refusal(members: &str) -> String {
    match lowered(members).refusal() {
        Refusal::BodyEnum { key, field } => {
            assert_eq!(key, "probe.write");
            field
        }
        other => panic!("refused as {other:?}"),
    }
}

fn any_of_refusal(members: &str) -> String {
    match lowered(members).refusal() {
        Refusal::BodyAnyOf { key, detail } => {
            assert_eq!(key, "probe.write");
            detail
        }
        other => panic!("refused as {other:?}"),
    }
}

fn prefix_refusal(members: &str) -> String {
    match lowered(members).refusal() {
        Refusal::BodyPrefix { key, field } => {
            assert_eq!(key, "probe.write");
            field
        }
        other => panic!("refused as {other:?}"),
    }
}

// ---- the lowerer against the engine's loader ------------------------------------------------

/// The engine refuses `body_keys` on anything but a write whose operation takes
/// a body (`closed.rs` `Closed::new`), so `body_enums` on a read can never
/// load; the lowerer must refuse it before any runtime reads it.
#[test]
fn a_body_enum_on_a_read_is_refused() {
    let root = zendesk_root();
    let text = HEAD.replace(
        "  ticket.show: {op: ShowTicket}\n",
        "  ticket.show:\n    op: ShowTicket\n    body: {/ticket/priority: string}\n    body_enums: {/ticket/priority: [low, high]}\n",
    );
    let lowering = lower(&text, root.path());
    assert!(
        lowering.outcome.is_err(),
        "a read declaring body and body_enums lowered; the engine refuses its body_keys: {}",
        lowering.selection()
    );
}

/// The engine's `path` refuses a dotted plain name, so a `body` key written in
/// the guard's dotted notation (`ticket.priority`) with an enum can never load.
#[test]
fn a_body_enum_on_a_dotted_key_is_refused() {
    let lowering = lowered(
        "    body: {ticket.priority: string}\n    body_enums: {ticket.priority: [low, high]}\n",
    );
    assert!(
        lowering.outcome.is_err(),
        "a dotted body key with body_enums lowered; the engine refuses its body_keys: {}",
        lowering.selection()
    );
}

/// The engine resolves a member's key by path (`admitted`), so `/priority`
/// names the top-level key `priority`; the lowerer compares spellings.
#[test]
fn a_body_enum_naming_a_top_level_key_by_its_pointer_lowers() {
    let lowering = lowered(
        "    body: {priority: string, type: string}\n    body_enums: {/priority: [low, high]}\n",
    );
    match &lowering.outcome {
        Ok(_) => assert_eq!(
            lowering.entry("probe.write")["body_enums"]["/priority"],
            serde_json::json!(["low", "high"])
        ),
        Err(refusal) => {
            panic!("refused as {refusal:?}; the engine admits `/priority` for `priority`")
        }
    }
}

// ---- refusals the suite did not pin -----------------------------------------------------------

#[test]
fn an_empty_body_enum_is_refused() {
    let field = enum_refusal(&format!("{BODY}    body_enums: {{/ticket/priority: []}}\n"));
    assert_eq!(field, "/ticket/priority");
}

#[test]
fn a_body_enum_with_an_empty_value_is_refused() {
    let field = enum_refusal(&format!(
        "{BODY}    body_enums: {{/ticket/type: [problem, \"\"]}}\n"
    ));
    assert_eq!(field, "/ticket/type");
}

#[test]
fn a_body_enum_on_a_string_list_key_is_refused() {
    let field = enum_refusal(&format!("{BODY}    body_enums: {{/ticket/tags: [a, b]}}\n"));
    assert_eq!(field, "/ticket/tags");
}

#[test]
fn a_body_enum_on_a_fixed_key_is_refused() {
    let field = enum_refusal(&format!(
        "{BODY}    body_enums: {{/ticket/safe_update: [\"true\"]}}\n"
    ));
    assert_eq!(field, "/ticket/safe_update");
}

#[test]
fn a_body_enum_on_an_undeclared_key_is_refused() {
    let field = enum_refusal(&format!(
        "{BODY}    body_enums: {{/ticket/status: [open]}}\n"
    ));
    assert_eq!(field, "/ticket/status");
}

#[test]
fn a_second_bad_body_enum_is_named_not_the_first_good_one() {
    let field = enum_refusal(&format!(
        "{BODY}    body_enums: {{/ticket/priority: [low, high], /ticket/type: [task, task]}}\n"
    ));
    assert_eq!(field, "/ticket/type");
}

#[test]
fn a_body_any_of_naming_a_key_twice_is_refused() {
    let detail = any_of_refusal(&format!(
        "{BODY}    body_any_of: [/ticket/tags, /ticket/type, /ticket/tags]\n"
    ));
    assert_eq!(detail, "names `/ticket/tags` twice");
}

#[test]
fn an_empty_body_any_of_is_refused() {
    let detail = any_of_refusal(&format!("{BODY}    body_any_of: []\n"));
    assert_eq!(detail, "names no key");
}

#[test]
fn a_body_any_of_naming_a_fixed_key_is_refused() {
    let detail = any_of_refusal(&format!(
        "{BODY}    body_any_of: [/ticket/tags, /ticket/safe_update]\n"
    ));
    assert_eq!(
        detail,
        "names `/ticket/safe_update`, which body does not declare for the caller"
    );
}

#[test]
fn a_body_any_of_with_a_top_level_key_and_a_nested_one_is_refused() {
    let detail = any_of_refusal(
        "    body: {subject: string, /ticket/type: string}\n    body_any_of: [subject, /ticket/type]\n",
    );
    assert_eq!(detail, "names keys under more than one parent");
}

#[test]
fn a_body_any_of_of_a_top_level_name_and_a_top_level_pointer_lowers() {
    let lowering = lowered(
        "    body: {subject: string, /status: string}\n    body_any_of: [subject, /status]\n",
    );
    assert_eq!(
        lowering.entry("probe.write")["body_any_of"],
        serde_json::json!(["subject", "/status"])
    );
}

#[test]
fn a_body_any_of_keeps_its_written_order() {
    let lowering = lowered(&format!(
        "{BODY}    body_any_of: [/ticket/type, /ticket/group_id, /ticket/tags]\n"
    ));
    assert_eq!(
        lowering.entry("probe.write")["body_any_of"],
        serde_json::json!(["/ticket/type", "/ticket/group_id", "/ticket/tags"])
    );
}

#[test]
fn a_body_prefix_on_an_integer_key_is_refused() {
    let field = prefix_refusal(&format!(
        "{BODY}    body_prefixes: {{/ticket/group_id: \"1\"}}\n"
    ));
    assert_eq!(field, "/ticket/group_id");
}

#[test]
fn a_body_prefix_on_an_undeclared_key_is_refused() {
    let field = prefix_refusal(&format!(
        "{BODY}    body_prefixes: {{/ticket/subject: \"x \"}}\n"
    ));
    assert_eq!(field, "/ticket/subject");
}

#[test]
fn a_body_prefix_on_an_any_key_is_refused() {
    let field = prefix_refusal(
        "    body: {/ticket/comment/body: any}\n    body_prefixes: {/ticket/comment/body: draft}\n",
    );
    assert_eq!(field, "/ticket/comment/body");
}

#[test]
fn a_conflict_status_on_a_write_declared_read_is_refused() {
    let root = zendesk_root();
    let text = format!(
        "{HEAD}  probe.write:\n    op: UpdateTicket\n    effect: read\n    conflict_status: 409\n"
    );
    match lower(&text, root.path()).refusal() {
        Refusal::ConflictStatus { key, status } => {
            assert_eq!(key, "probe.write");
            assert_eq!(status, 409);
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_conflict_status_beyond_a_status_is_refused_naming_the_member() {
    match lowered(&format!("{BODY}    conflict_status: 70000\n")).refusal() {
        Refusal::Malformed(detail) => assert!(detail.contains("conflict_status"), "{detail}"),
        Refusal::ConflictStatus { key, .. } => assert_eq!(key, "probe.write"),
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_null_conflict_status_lowers_without_the_member() {
    let lowering = lowered(&format!("{BODY}    conflict_status: null\n"));
    assert_eq!(lowering.entry("probe.write").get("conflict_status"), None);
}

const GUARD_BEFORE: &str = "      before: {read: ticket.show, expect: {/ticket/updated_at: body.ticket.updated_stamp}, mismatch: conflict}\n";

const STAMPED: &str = "    body: {/ticket/type: string, /ticket/updated_stamp: string}\n";

#[test]
fn a_mismatch_on_a_further_preflight_reaches_further_preflights() {
    let lowering = lowered(&format!(
        "{STAMPED}    guard:\n{GUARD_BEFORE}      also:\n        - {{read: ticket.show, expect: {{/ticket/status: =open}}, mismatch: conflict}}\n        - {{read: ticket.show, expect: {{/ticket/type: body.ticket.type}}}}\n"
    ));
    let guard = &lowering.entry("probe.write")["guard"];
    assert_eq!(guard["preflight"]["mismatch"], "conflict");
    assert_eq!(guard["further_preflights"][0]["mismatch"], "conflict");
    assert_eq!(guard["further_preflights"][1].get("mismatch"), None);
}

#[test]
fn a_mismatch_after_the_write_is_refused_as_an_unknown_field() {
    let lowering = lowered(&format!(
        "{STAMPED}    guard:\n{GUARD_BEFORE}      after: {{expect: {{/ticket/id: ticket_id}}, mismatch: conflict}}\n"
    ));
    match lowering.refusal() {
        Refusal::Malformed(detail) | Refusal::UnknownField(detail) => {
            assert!(detail.contains("mismatch"), "{detail}")
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_null_mismatch_lowers_without_the_member() {
    let lowering = lowered(&format!(
        "{STAMPED}    guard:\n      before: {{read: ticket.show, expect: {{/ticket/updated_at: body.ticket.updated_stamp}}, mismatch: null}}\n"
    ));
    assert_eq!(
        lowering.entry("probe.write")["guard"]["preflight"].get("mismatch"),
        None
    );
}

#[test]
fn a_required_string_list_lowers_typed_and_required() {
    let lowering = lowered("    body: {/ticket/tags: string_list!, /ticket/type: string}\n");
    let entry = lowering.entry("probe.write");
    assert_eq!(entry["body_types"]["/ticket/tags"], "string_list");
    assert_eq!(entry["body_required"], serde_json::json!(["/ticket/tags"]));
}

// ---- the importer's round trip ---------------------------------------------------------------

/// The shipped Zendesk selection with `body_any_of` out of body order and a
/// further preflight carrying `mismatch`, imported and lowered back: every
/// member survives, in its order.
#[test]
fn an_import_keeps_any_of_order_and_a_further_preflights_mismatch() {
    let temp = tempfile::tempdir().unwrap();
    let mut shipped: Value = serde_json::from_slice(
        &std::fs::read(repository().join("adapters/catalog/providers/zendesk/operations.json"))
            .unwrap(),
    )
    .unwrap();
    let reversed = serde_json::json!([
        "/ticket/type",
        "/ticket/priority",
        "/ticket/tags",
        "/ticket/group_id"
    ]);
    for entry in shipped["operations"].as_array_mut().unwrap() {
        if entry["id"] == "ticket.update" {
            entry["body_any_of"] = reversed.clone();
        }
        if entry["id"] == "ticket.note" {
            entry["guard"]["further_preflights"] = serde_json::json!([{
                "operation_id": "ShowTicket",
                "values": {"ticket_id": "ticket_id"},
                "checks": [{"pointer": "/ticket/status", "expect": {"literal": "open"}}],
                "mismatch": "conflict"
            }]);
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
    let written: Value =
        serde_yaml_ng::from_slice(&std::fs::read(&imported.definition).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(&written["operations"]["ticket.update"]["body_any_of"]).unwrap(),
        reversed
    );
    assert_eq!(
        serde_json::to_value(&written["operations"]["ticket.note"]["guard"]["also"][0]["mismatch"])
            .unwrap(),
        "conflict"
    );
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
    // story:catalog-operation-declares-its-walk: a shipped read's `walk` has no
    // definition field yet (story:definition-lowerer-carries-walk). The import
    // must report exactly the walks it left out, and the rest round-trips whole.
    let mut walked = Vec::new();
    for operation in shipped["operations"].as_array_mut().unwrap() {
        let operation = operation.as_object_mut().unwrap();
        if operation.remove("walk").is_some() {
            walked.push((
                operation["id"].as_str().unwrap().to_owned(),
                "walk".to_owned(),
            ));
        }
    }
    let dropped: Vec<(String, String)> = imported
        .dropped
        .iter()
        .map(|dropped| (dropped.operation.clone(), dropped.member.clone()))
        .collect();
    assert_eq!(dropped, walked);
    assert_eq!(back, shipped);
}

/// The engine sends a write only for POST, PUT, PATCH or DELETE
/// (`write_method` in `adapters/catalog/src/lib.rs`) and refuses the selection
/// at load otherwise, so the lowerer refuses a declared write on a GET.
#[test]
fn a_write_effect_on_a_get_operation_is_refused() {
    let root = zendesk_root();
    let text = format!(
        "{HEAD}  probe.write:\n    op: ShowTicket\n    effect: write\n    body: {{name: string}}\n"
    );
    match lower(&text, root.path()).refusal() {
        Refusal::WriteMethod { key, op, method } => {
            assert_eq!(key, "probe.write");
            assert_eq!(op, "ShowTicket");
            assert_eq!(method, "get");
        }
        other => panic!("refused as {other:?}"),
    }
}

/// A download names no `op`, so a declared write on it has no method at all.
#[test]
fn a_write_effect_on_a_download_with_no_op_is_refused() {
    let root = zendesk_root();
    let text = format!(
        "{HEAD}  probe.write:\n    effect: write\n    download: {{path_prefix: attachments}}\n"
    );
    match lower(&text, root.path()).refusal() {
        Refusal::WriteMethod { key, op, method } => {
            assert_eq!(key, "probe.write");
            assert_eq!((op.as_str(), method.as_str()), ("", ""));
        }
        other => panic!("refused as {other:?}"),
    }
}

/// `x` and `/x` are one path to the engine, which refuses `body_keys` that
/// repeat one (`closed.rs` `Closed::new`).
#[test]
fn a_body_key_declared_by_name_and_by_pointer_is_refused() {
    match lowered("    body: {x: string, /x: integer}\n").refusal() {
        Refusal::BodyType { key, field, .. } => {
            assert_eq!(key, "probe.write");
            assert_eq!(field, "/x");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn an_any_of_over_one_key_spelled_twice_is_refused() {
    match lowered("    body: {x: string, /x: string}\n    body_any_of: [x, /x]\n").refusal() {
        Refusal::BodyType { key, field, .. } => {
            assert_eq!(key, "probe.write");
            assert_eq!(field, "/x");
        }
        other => panic!("refused as {other:?}"),
    }
}

/// The engine refuses `body_keys` where one key's path lies inside another's
/// (`closed.rs` `inside`), so `/a` with `/a/b` can never load.
#[test]
fn a_body_key_inside_another_body_key_is_refused() {
    match lowered("    body: {/ticket/comment: string, /ticket/comment/body: string}\n").refusal() {
        Refusal::BodyType { key, field, .. } => {
            assert_eq!(key, "probe.write");
            assert_eq!(field, "/ticket/comment/body");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_body_key_beside_another_with_a_longer_name_lowers() {
    lowered("    body: {/ticket/comment: string, /ticket/comments: string}\n").selection();
}

const GUARDED: &str = "    guard:\n      before: {read: ticket.show, expect: {/ticket/updated_at: body.ticket.updated_stamp}}\n";

fn guard_refusal(body: &str, guard: &str) -> String {
    match lowered(&format!("{body}{guard}")).refusal() {
        Refusal::BodyType { key, field, spec } => {
            assert_eq!(key, "probe.write");
            assert_eq!(field, "body.ticket.updated_stamp");
            spec
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_guard_reading_a_body_key_the_body_declares_lowers() {
    lowered(&format!(
        "    body: {{/ticket/priority: string, /ticket/updated_stamp: string}}\n{GUARDED}"
    ))
    .selection();
}

#[test]
fn a_guard_reading_a_body_key_the_body_does_not_admit_is_refused() {
    let spec = guard_refusal("    body: {/ticket/priority: string}\n", GUARDED);
    assert!(spec.contains("does not admit"), "{spec}");
}

#[test]
fn a_guard_reading_a_body_key_the_body_fixes_is_refused() {
    let spec = guard_refusal(
        "    body: {/ticket/priority: string, /ticket/updated_stamp: =x}\n",
        GUARDED,
    );
    assert!(spec.contains("fixes"), "{spec}");
}

#[test]
fn an_after_check_reading_a_body_key_the_body_does_not_admit_is_refused() {
    let spec = guard_refusal(
        "    body: {/ticket/priority: string}\n",
        "    guard:\n      before: {read: ticket.show}\n      after: {expect: {/ticket/updated_at: body.ticket.updated_stamp}}\n",
    );
    assert!(spec.contains("does not admit"), "{spec}");
}

#[test]
fn a_with_binding_reading_a_body_key_the_body_does_not_admit_is_refused() {
    let spec = guard_refusal(
        "    body: {/ticket/priority: string}\n",
        "    guard:\n      before: {read: ticket.show, with: {ticket_id: body.ticket.updated_stamp}}\n",
    );
    assert!(spec.contains("does not admit"), "{spec}");
}

/// The engine refuses an operation whose read declares only `body_enums` or
/// `body_prefixes` (`closed.rs`): each member is named in the refusal.
#[test]
fn a_read_declaring_only_an_empty_body_enums_or_body_prefixes_is_refused() {
    for member in ["body_enums", "body_prefixes"] {
        let root = zendesk_root();
        let text = HEAD.replace(
            "  ticket.show: {op: ShowTicket}\n",
            &format!("  ticket.show:\n    op: ShowTicket\n    {member}: {{}}\n"),
        );
        let lowering = lower(&text, root.path());
        match lowering.refusal() {
            Refusal::Malformed(detail) => assert!(detail.contains(member), "{detail}"),
            other => panic!("refused as {other:?}"),
        }
    }
}

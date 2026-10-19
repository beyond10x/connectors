//! Writes in a `connectors-definition/1` file: guards and closed bodies. All
//! fifteen GitLab merge-request operations lower to the shipped selection,
//! field by field, guards included, and Zendesk's three ticket writes carry
//! the members the 0.54.0 engine added (`string_list`, `body_enums`,
//! `body_any_of`, `body_prefixes`, `conflict_status`, a preflight
//! `mismatch`); each refusal is named.
#![cfg(unix)]
use connectors_catalog::definition::{self, Refusal, Request};
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
    fn refusal(self) -> Refusal {
        self.outcome.expect_err("the definition is refused")
    }
}

/// Lower `text` as `providers/<provider>.yaml`, sources resolved under `root`.
fn lower(provider: &str, text: &str, root: &Path) -> Lowering {
    let temp = tempfile::tempdir().unwrap();
    let providers = temp.path().join("providers");
    std::fs::create_dir_all(&providers).unwrap();
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let file = providers.join(format!("{provider}.yaml"));
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

/// A definitions root holding GitLab's source gzipped and its amendments, as
/// `beyond10x/connectors-definitions` vendors them.
fn gitlab_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let upstream = root.path().join("upstream/gitlab");
    std::fs::create_dir_all(&upstream).unwrap();
    let source = repository().join("adapters/gitlab/upstream");
    std::fs::write(
        upstream.join("openapi_v3.yaml.gz"),
        gzip(&std::fs::read(source.join("openapi_v3.yaml")).unwrap()),
    )
    .unwrap();
    std::fs::copy(
        source.join("openapi_v3.amendments.json"),
        upstream.join("openapi_v3.amendments.json"),
    )
    .unwrap();
    root
}

/// A definitions root holding Zendesk's source gzipped and its amendments.
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

fn shipped(provider: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(repository().join(format!(
            "adapters/catalog/providers/{provider}/operations.json"
        )))
        .unwrap(),
    )
    .unwrap()
}

/// The definition `beyond10x/connectors-definitions` holds for GitLab's
/// merge-request domain on 2026-10-17.
const GITLAB: &str = r#"schema: urn:beyond10x:connectors-definition:1
source:
  openapi: upstream/gitlab/openapi_v3.yaml.gz
  sha256: f9e830bd3d2b99c49d60a7713fe1a64f5164418aca24b559287daab075beb530
  amendments: upstream/gitlab/openapi_v3.amendments.json
base_url: https://{hostname}
config:
  hostname: {type: string, default: gitlab.com}
auth:
  gitlab.pat: {scheme: header, header: PRIVATE-TOKEN, label: GitLab personal access token, identity: {path: user, kind: gitlab.user, subject: /id}, scopes: {path: personal_access_tokens/self, pointer: /scopes, minimum: [api]}}
paging:
  pages: {request: page, next: "header:x-next-page", limit: per_page, max: 100}
operations:
  projects.list: {op: getApiV4Projects}
  project.get: {op: getApiV4ProjectsId}
  merge_requests.list: {op: getApiV4ProjectsIdMergeRequests}
  merge_request.get: {op: getApiV4ProjectsIdMergeRequestsMergeRequestIid}
  merge_request.diffs: {op: getApiV4ProjectsIdMergeRequestsMergeRequestIidDiffs}
  merge_request.discussions: {op: getApiV4ProjectsIdMergeRequestsNoteableIdDiscussions}
  merge_request.discussion.get: {op: getApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId}
  merge_request.create:
    op: postApiV4ProjectsIdMergeRequests
    effect: write
    guard:
      before: {read: getApiV4ProjectsIdRepositoryBranchesBranch, with: {branch: body.source_branch}, expect: {/commit/id: sha}}
      after: {expect: {/sha: sha}}
  merge_request.update:
    op: putApiV4ProjectsIdMergeRequestsMergeRequestIid
    effect: write
    guard:
      before: {read: merge_request.get, expect: {/sha: sha, /state: =opened}}
      after: {expect: {/sha: sha}}
  merge_request.merge:
    op: putApiV4ProjectsIdMergeRequestsMergeRequestIidMerge
    effect: write
    guard:
      before:
        read: merge_request.get
        expect: {/sha: body.sha, /state: =opened, /detailed_merge_status: =mergeable, /head_pipeline/id: pipeline_id, /head_pipeline/status: =success}
      after: {expect: {/state: =merged, /sha: body.sha}}
  merge_request.auto_merge:
    op: putApiV4ProjectsIdMergeRequestsMergeRequestIidMerge
    effect: write
    body: {auto_merge: "=true", sha: any}
    guard:
      before: {read: merge_request.get, expect: {/sha: body.sha, /state: =opened}}
      after: {expect: {/sha: body.sha}, any: {/merge_when_pipeline_succeeds: "=true", /state: =merged}}
  merge_request.reopen:
    op: putApiV4ProjectsIdMergeRequestsMergeRequestIid
    effect: write
    body: {state_event: =reopen}
    guard:
      before: {read: merge_request.get, expect: {/sha: sha, /state: =closed}}
      after: {expect: {/sha: sha, /state: =opened}}
  merge_request.note.create:
    op: postApiV4ProjectsIdMergeRequestsNoteableIdNotes
    effect: write
    body: {body: string!, internal: boolean}
  merge_request.discussion.reply:
    op: postApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionIdNotes
    effect: write
    body: {body: string!}
  merge_request.discussion.resolve:
    op: putApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId
    effect: write
    body: {resolved: boolean}
    guard:
      before: {read: merge_request.discussion.get, expect: {/id: discussion_id, /resolvable: "=true"}}
      after: {expect: {/id: discussion_id, /resolved: body.resolved}}
"#;

const FIELDS: [&str; 8] = [
    "operation_id",
    "effect",
    "bounds",
    "body_keys",
    "body_types",
    "body_required",
    "body_fixed",
    "guard",
];

#[test]
fn the_gitlab_merge_request_domain_lowers_to_the_shipped_selection_guards_included() {
    let root = gitlab_root();
    let lowering = lower("gitlab", GITLAB, root.path());
    let shipped = shipped("gitlab");
    let entries = lowering.selection()["operations"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(entries.len(), 15);
    for entry in &entries {
        let id = entry["id"].as_str().unwrap();
        let expected = shipped["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .unwrap_or_else(|| panic!("operations.json lacks `{id}`"));
        for field in FIELDS {
            assert_eq!(entry.get(field), expected.get(field), "`{id}` {field}");
        }
    }
    let guarded = entries.iter().filter(|e| e.get("guard").is_some()).count();
    assert_eq!(guarded, 6);
}

#[test]
fn a_guard_read_that_is_neither_a_key_nor_an_operation_id_is_refused() {
    let root = gitlab_root();
    let text = GITLAB.replace(
        "{read: merge_request.get, expect: {/sha: sha, /state: =opened}}",
        "{read: merge_request.gone, expect: {/sha: sha, /state: =opened}}",
    );
    match lower("gitlab", &text, root.path()).refusal() {
        Refusal::UnknownGuardRead { key, read } => {
            assert_eq!(key, "merge_request.update");
            assert_eq!(read, "merge_request.gone");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_guard_binding_a_parameter_its_read_lacks_is_refused() {
    let root = gitlab_root();
    let text = GITLAB.replace(
        "with: {branch: body.source_branch}",
        "with: {tree: body.source_branch}",
    );
    match lower("gitlab", &text, root.path()).refusal() {
        Refusal::GuardParameter { key, parameter } => {
            assert_eq!(key, "merge_request.create");
            assert_eq!(parameter, "tree");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_body_spec_outside_the_vocabulary_is_refused() {
    let root = gitlab_root();
    let text = GITLAB.replace("body: {body: string!}", "body: {body: text!}");
    match lower("gitlab", &text, root.path()).refusal() {
        Refusal::BodyType { key, field, spec } => {
            assert_eq!(key, "merge_request.discussion.reply");
            assert_eq!(field, "body");
            assert_eq!(spec, "text!");
        }
        other => panic!("refused as {other:?}"),
    }
}

/// Zendesk's ticket reads and its three guarded writes, as
/// `beyond10x/connectors-definitions` holds them.
const ZENDESK: &str = r#"schema: urn:beyond10x:connectors-definition:1
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
  ticket.update:
    op: UpdateTicket
    effect: write
    body:
      /ticket/group_id: integer
      /ticket/tags: string_list
      /ticket/priority: string
      /ticket/type: string
      /ticket/updated_stamp: string
      /ticket/safe_update: =true
    body_enums: {/ticket/priority: [urgent, high, normal, low], /ticket/type: [problem, incident, question, task]}
    body_any_of: [/ticket/group_id, /ticket/tags, /ticket/priority, /ticket/type]
    conflict_status: 409
    guard:
      before: {read: ticket.show, expect: {/ticket/updated_at: body.ticket.updated_stamp}, mismatch: conflict}
      after: {expect: {/ticket/id: ticket_id}}
  ticket.note:
    op: UpdateTicket
    effect: write
    body: {/ticket/comment/body: string+!, /ticket/comment/public: =false, /ticket/updated_stamp: string, /ticket/safe_update: =true}
    conflict_status: 409
    guard:
      before: {read: ticket.show, expect: {/ticket/updated_at: body.ticket.updated_stamp}, mismatch: conflict}
      after: {expect: {/ticket/id: ticket_id}}
  ticket.draft_reply:
    op: UpdateTicket
    effect: write
    body: {/ticket/comment/body: string+!, /ticket/comment/public: =false, /ticket/updated_stamp: string, /ticket/safe_update: =true}
    body_prefixes: {/ticket/comment/body: "[Draft reply - not sent to the requester]\n"}
    conflict_status: 409
    guard:
      before: {read: ticket.show, expect: {/ticket/updated_at: body.ticket.updated_stamp}, mismatch: conflict}
      after: {expect: {/ticket/id: ticket_id}}
"#;

/// The members of a write the 0.54.0 engine added, beside the earlier ones.
const WRITE_FIELDS: [&str; 11] = [
    "operation_id",
    "effect",
    "body_keys",
    "body_types",
    "body_required",
    "body_nonempty",
    "body_fixed",
    "body_enums",
    "body_any_of",
    "body_prefixes",
    "conflict_status",
];

#[test]
fn the_zendesk_ticket_writes_lower_to_the_shipped_selection_members_of_0_54_0_included() {
    let root = zendesk_root();
    let lowering = lower("zendesk", ZENDESK, root.path());
    let shipped = shipped("zendesk");
    let entries = lowering.selection()["operations"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(entries.len(), 4);
    for id in ["ticket.update", "ticket.note", "ticket.draft_reply"] {
        let entry = entries
            .iter()
            .find(|e| e["id"] == id)
            .unwrap_or_else(|| panic!("the selection lacks `{id}`"));
        let expected = shipped["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .unwrap_or_else(|| panic!("operations.json lacks `{id}`"));
        for field in WRITE_FIELDS {
            assert_eq!(entry.get(field), expected.get(field), "`{id}` {field}");
        }
        // The guard, member by member, so a difference names its member.
        let guard = &entry["guard"];
        let expected_guard = &expected["guard"];
        assert_eq!(
            guard["preflight"]["mismatch"], expected_guard["preflight"]["mismatch"],
            "`{id}` mismatch"
        );
        assert_eq!(guard, expected_guard, "`{id}` guard");
    }
    let update = entries.iter().find(|e| e["id"] == "ticket.update").unwrap();
    assert_eq!(update["body_types"]["/ticket/tags"], "string_list");
    assert_eq!(
        update["body_enums"]["/ticket/priority"],
        serde_json::json!(["urgent", "high", "normal", "low"])
    );
    assert_eq!(
        update["body_any_of"],
        serde_json::json!([
            "/ticket/group_id",
            "/ticket/tags",
            "/ticket/priority",
            "/ticket/type"
        ])
    );
    assert_eq!(update["conflict_status"], 409);
    assert_eq!(update["guard"]["preflight"]["mismatch"], "conflict");
    let draft = entries
        .iter()
        .find(|e| e["id"] == "ticket.draft_reply")
        .unwrap();
    assert_eq!(
        draft["body_prefixes"]["/ticket/comment/body"],
        "[Draft reply - not sent to the requester]\n"
    );
    // A read carries none of them.
    let show = entries.iter().find(|e| e["id"] == "ticket.show").unwrap();
    for field in [
        "body_enums",
        "body_any_of",
        "body_prefixes",
        "conflict_status",
        "guard",
    ] {
        assert_eq!(show.get(field), None, "`ticket.show` {field}");
    }
}

#[test]
fn a_preflight_without_mismatch_lowers_without_the_member() {
    let root = zendesk_root();
    let text = ZENDESK.replace(", mismatch: conflict}", "}");
    let lowering = lower("zendesk", &text, root.path());
    let entries = lowering.selection()["operations"]
        .as_array()
        .unwrap()
        .clone();
    let update = entries.iter().find(|e| e["id"] == "ticket.update").unwrap();
    assert_eq!(update["guard"]["preflight"].get("mismatch"), None);
}

#[test]
fn a_conflict_status_other_than_409_or_412_is_refused() {
    let root = zendesk_root();
    let text = ZENDESK.replacen("conflict_status: 409", "conflict_status: 418", 1);
    match lower("zendesk", &text, root.path()).refusal() {
        Refusal::ConflictStatus { key, status } => {
            assert_eq!(key, "ticket.update");
            assert_eq!(status, 418);
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_conflict_status_of_412_lowers() {
    let root = zendesk_root();
    let text = ZENDESK.replacen("conflict_status: 409", "conflict_status: 412", 1);
    let lowering = lower("zendesk", &text, root.path());
    let entries = lowering.selection()["operations"]
        .as_array()
        .unwrap()
        .clone();
    let update = entries.iter().find(|e| e["id"] == "ticket.update").unwrap();
    assert_eq!(update["conflict_status"], 412);
}

#[test]
fn a_conflict_status_on_a_read_is_refused() {
    let root = zendesk_root();
    let text = ZENDESK.replace(
        "ticket.show: {op: ShowTicket}",
        "ticket.show: {op: ShowTicket, conflict_status: 409}",
    );
    match lower("zendesk", &text, root.path()).refusal() {
        Refusal::ConflictStatus { key, status } => {
            assert_eq!(key, "ticket.show");
            assert_eq!(status, 409);
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_body_any_of_with_one_key_is_refused() {
    let root = zendesk_root();
    let text = ZENDESK.replace(
        "body_any_of: [/ticket/group_id, /ticket/tags, /ticket/priority, /ticket/type]",
        "body_any_of: [/ticket/group_id]",
    );
    match lower("zendesk", &text, root.path()).refusal() {
        Refusal::BodyAnyOf { key, detail } => {
            assert_eq!(key, "ticket.update");
            assert_eq!(detail, "names one key");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_body_any_of_naming_a_key_the_body_does_not_declare_is_refused() {
    let root = zendesk_root();
    let text = ZENDESK.replace(
        "body_any_of: [/ticket/group_id, /ticket/tags, /ticket/priority, /ticket/type]",
        "body_any_of: [/ticket/group_id, /ticket/subject]",
    );
    match lower("zendesk", &text, root.path()).refusal() {
        Refusal::BodyAnyOf { key, detail } => {
            assert_eq!(key, "ticket.update");
            assert_eq!(
                detail,
                "names `/ticket/subject`, which body does not declare for the caller"
            );
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_body_any_of_under_two_parents_is_refused() {
    let root = zendesk_root();
    let text = ZENDESK
        .replace(
            "body_any_of: [/ticket/group_id, /ticket/tags, /ticket/priority, /ticket/type]",
            "body_any_of: [/ticket/group_id, /ticket/updated_stamp, /other/tags]",
        )
        .replace(
            "      /ticket/safe_update: =true\n    body_enums",
            "      /ticket/safe_update: =true\n      /other/tags: string_list\n    body_enums",
        );
    match lower("zendesk", &text, root.path()).refusal() {
        Refusal::BodyAnyOf { key, detail } => {
            assert_eq!(key, "ticket.update");
            assert_eq!(detail, "names keys under more than one parent");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_body_enum_on_a_key_that_is_not_a_string_is_refused() {
    let root = zendesk_root();
    let text = ZENDESK.replace(
        "body_enums: {/ticket/priority: [urgent, high, normal, low], ",
        "body_enums: {/ticket/group_id: [\"1\", \"2\"], ",
    );
    match lower("zendesk", &text, root.path()).refusal() {
        Refusal::BodyEnum { key, field } => {
            assert_eq!(key, "ticket.update");
            assert_eq!(field, "/ticket/group_id");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_body_enum_with_a_repeated_value_is_refused() {
    let root = zendesk_root();
    let text = ZENDESK.replace("[urgent, high, normal, low]", "[urgent, high, urgent]");
    match lower("zendesk", &text, root.path()).refusal() {
        Refusal::BodyEnum { key, field } => {
            assert_eq!(key, "ticket.update");
            assert_eq!(field, "/ticket/priority");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_body_prefix_on_a_fixed_key_is_refused() {
    let root = zendesk_root();
    let text = ZENDESK.replace(
        "body_prefixes: {/ticket/comment/body: \"[Draft reply - not sent to the requester]\\n\"}",
        "body_prefixes: {/ticket/safe_update: draft}",
    );
    match lower("zendesk", &text, root.path()).refusal() {
        Refusal::BodyPrefix { key, field } => {
            assert_eq!(key, "ticket.draft_reply");
            assert_eq!(field, "/ticket/safe_update");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn an_empty_body_prefix_is_refused() {
    let root = zendesk_root();
    let text = ZENDESK.replace(
        "body_prefixes: {/ticket/comment/body: \"[Draft reply - not sent to the requester]\\n\"}",
        "body_prefixes: {/ticket/comment/body: \"\"}",
    );
    match lower("zendesk", &text, root.path()).refusal() {
        Refusal::BodyPrefix { key, field } => {
            assert_eq!(key, "ticket.draft_reply");
            assert_eq!(field, "/ticket/comment/body");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_non_empty_string_list_is_refused() {
    let root = zendesk_root();
    let text = ZENDESK.replace("/ticket/tags: string_list", "/ticket/tags: string_list+");
    match lower("zendesk", &text, root.path()).refusal() {
        Refusal::BodyType { key, field, spec } => {
            assert_eq!(key, "ticket.update");
            assert_eq!(field, "/ticket/tags");
            assert_eq!(spec, "string_list+");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_mismatch_outside_the_vocabulary_is_refused_as_malformed() {
    let root = zendesk_root();
    let text = ZENDESK.replacen("mismatch: conflict}", "mismatch: forbidden}", 1);
    match lower("zendesk", &text, root.path()).refusal() {
        Refusal::Malformed(detail) => assert!(detail.contains("forbidden"), "{detail}"),
        other => panic!("refused as {other:?}"),
    }
}

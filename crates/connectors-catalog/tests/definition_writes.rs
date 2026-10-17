//! Writes in a `connectors-definition/1` file: guards and closed bodies. All
//! fifteen GitLab merge-request operations lower to the shipped selection,
//! field by field, guards included; each refusal is named.
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

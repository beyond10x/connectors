//! Datasources in a `connectors-definition/1` file: GitLab's merge-request feed
//! lowers to the shipped `datasource.feed/v1alpha1` declaration exactly; each
//! refusal is named.
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

/// GitLab's definition in `beyond10x/connectors-definitions` on 2026-10-17.
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
datasources:
  merge_requests:
    kind: feed
    profile: gitlab-merge-requests/1
    capabilities: {deletions: not-observed, kind: fixed-word, revision: update-time, visibility: all-private}
    containers:
      list: projects.list
      lookup: project.get
      query: {membership: "true", order_by: id, sort: asc}
      cursor: {parameter: id_after, last: /id}
      id: /id
      name: /name_with_namespace
      kind_word: project
    items:
      list: merge_requests.list
      query: {order_by: updated_at, sort: asc, state: all, scope: all}
      position: {time: updated_after}
      id: /iid
      revision: /updated_at
      created_at: /created_at
      updated_at: /updated_at
      author: {id: /author/id, display_name: /author/name}
      body: {pointer: /description, representation: text/markdown}
    filters:
      mine_open: {state: opened, scope: assigned_to_me}
"#;

#[test]
fn the_gitlab_merge_request_feed_lowers_to_the_shipped_declaration() {
    let root = gitlab_root();
    let selection = lower("gitlab", GITLAB, root.path()).selection();
    assert_eq!(selection["feed"], shipped("gitlab")["feed"]);
}

#[test]
fn a_definition_without_datasources_writes_no_feed() {
    let root = gitlab_root();
    let text = GITLAB.split("datasources:").next().unwrap();
    assert!(
        lower("gitlab", text, root.path())
            .selection()
            .get("feed")
            .is_none()
    );
}

#[test]
fn a_datasource_naming_an_undeclared_operation_is_refused() {
    let root = gitlab_root();
    let text = GITLAB.replace("list: merge_requests.list", "list: issues.list");
    match lower("gitlab", &text, root.path()).refusal() {
        Refusal::UnknownDatasourceOperation { datasource, key } => {
            assert_eq!(datasource, "merge_requests");
            assert_eq!(key, "issues.list");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_lookup_with_other_than_one_path_parameter_is_refused() {
    let root = gitlab_root();
    let text = GITLAB.replace("lookup: project.get", "lookup: merge_request.get");
    match lower("gitlab", &text, root.path()).refusal() {
        Refusal::DatasourceOperation { key, .. } => assert_eq!(key, "merge_request.get"),
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_filter_setting_a_parameter_the_items_list_lacks_is_refused() {
    let root = gitlab_root();
    let text = GITLAB.replace(
        "{state: opened, scope: assigned_to_me}",
        "{state: opened, owner: me}",
    );
    match lower("gitlab", &text, root.path()).refusal() {
        Refusal::FilterParameter {
            filter, parameter, ..
        } => {
            assert_eq!(filter, "mine_open");
            assert_eq!(parameter, "owner");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn two_feed_datasources_are_refused_while_the_selection_carries_one() {
    let root = gitlab_root();
    let datasource = GITLAB.split("datasources:\n").nth(1).unwrap();
    let second = datasource.replacen("  merge_requests:", "  merge_requests_again:", 1);
    let text = format!("{GITLAB}{second}");
    assert!(matches!(
        lower("gitlab", &text, root.path()).refusal(),
        Refusal::FeedCount(2)
    ));
}

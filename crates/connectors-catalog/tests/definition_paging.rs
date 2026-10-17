//! Paging styles in a `connectors-definition/1` file: which style an operation
//! takes, the page-size bound it lowers to, and each refusal, by name. GitLab's
//! merge-request reads lower to the committed bundle and to the shipped bounds.
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

const JIRA: &str = "schema: urn:beyond10x:connectors-definition:1
source:
  openapi: adapters/atlassian/upstream/vendor/jira-platform-v3.json.gz
  sha256: 655b4790a5c8543c81755c8250c6a8b8f58ee62712168a39d628d6d927549709
base_url: https://{site}.atlassian.net
config:
  site: {type: string}
auth:
  atlassian.basic: {scheme: basic, label: API token, account_label: Account email, identity: {path: myself, kind: atlassian.account, subject: /accountId}}
paging:
  token: {request: nextPageToken, next: /nextPageToken, limit: maxResults, last: /isLast}
  offset: {request: startAt, limit: maxResults, total: /total}
  offset-last: {request: startAt, limit: maxResults, last: /isLast}
operations:
  issue.get: {op: getIssue}
  issue.comments: {op: getComments, paging: offset}
  issue.changelog: {op: getChangeLogs, paging: offset-last}
  issues.search: {op: searchAndReconsileIssuesUsingJql}
";

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

const GITLAB: &str = "schema: urn:beyond10x:connectors-definition:1
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
  pages: {request: page, next: \"header:x-next-page\", limit: per_page, max: 100}
operations:
  projects.list: {op: getApiV4Projects}
  project.get: {op: getApiV4ProjectsId}
  merge_requests.list: {op: getApiV4ProjectsIdMergeRequests}
  merge_request.get: {op: getApiV4ProjectsIdMergeRequestsMergeRequestIid}
  merge_request.diffs: {op: getApiV4ProjectsIdMergeRequestsMergeRequestIidDiffs}
  merge_request.discussions: {op: getApiV4ProjectsIdMergeRequestsNoteableIdDiscussions}
  merge_request.discussion.get: {op: getApiV4ProjectsIdMergeRequestsNoteableIdDiscussionsDiscussionId}
";

fn shipped(provider: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(repository().join(format!(
            "adapters/catalog/providers/{provider}/operations.json"
        )))
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn gitlab_merge_request_reads_lower_to_the_committed_bundle_and_the_shipped_bounds() {
    let root = gitlab_root();
    let lowering = lower("gitlab", GITLAB, root.path());
    let lowered = lowering
        .outcome
        .as_ref()
        .expect("the GitLab definition lowers");
    assert_eq!(
        std::fs::read(&lowered.bundle).unwrap(),
        std::fs::read(repository().join("adapters/catalog/generated/bundles/gitlab.bundle.json"))
            .unwrap(),
        "the lowered bundle is the committed one, byte for byte"
    );
    let shipped = shipped("gitlab");
    let entries = lowering.selection()["operations"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(entries.len(), 7);
    for entry in &entries {
        let id = entry["id"].as_str().unwrap();
        let expected = shipped["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .unwrap_or_else(|| panic!("operations.json lacks `{id}`"));
        assert_eq!(entry["operation_id"], expected["operation_id"], "`{id}`");
        assert_eq!(entry["effect"], expected["effect"], "`{id}`");
        assert_eq!(entry.get("bounds"), expected.get("bounds"), "`{id}` bounds");
    }
    let bounded = entries.iter().filter(|e| e.get("bounds").is_some()).count();
    assert_eq!(
        bounded, 4,
        "the four lists take per_page 1..100 from the style"
    );
}

#[test]
fn jira_operations_take_the_named_or_the_one_matching_style_and_no_bound_without_max() {
    let lowering = lower("jira", JIRA, &repository());
    let entries = lowering.selection()["operations"]
        .as_array()
        .unwrap()
        .clone();
    let ids: Vec<&str> = entries.iter().map(|e| e["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        [
            "issue.changelog",
            "issue.comments",
            "issue.get",
            "issues.search"
        ]
    );
    for entry in &entries {
        assert!(
            entry.get("bounds").is_none(),
            "`{}`: Jira's styles declare no max, and Jira ships no bounds",
            entry["id"]
        );
    }
}

#[test]
fn an_operation_two_styles_match_must_name_one() {
    let text = JIRA.replace("{op: getComments, paging: offset}", "{op: getComments}");
    match lower("jira", &text, &repository()).refusal() {
        Refusal::AmbiguousPaging { key, styles } => {
            assert_eq!(key, "issue.comments");
            assert_eq!(styles, ["offset", "offset-last"]);
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_style_the_definition_does_not_declare_is_refused() {
    let text = JIRA.replace("paging: offset}", "paging: cursor}");
    match lower("jira", &text, &repository()).refusal() {
        Refusal::UnknownPaging { key, style } => {
            assert_eq!(key, "issue.comments");
            assert_eq!(style, "cursor");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_named_style_whose_parameter_the_operation_lacks_is_refused() {
    let text = JIRA.replace("{op: getIssue}", "{op: getIssue, paging: token}");
    match lower("jira", &text, &repository()).refusal() {
        Refusal::PagingParameter {
            key,
            style,
            parameter,
        } => {
            assert_eq!(key, "issue.get");
            assert_eq!(style, "token");
            assert_eq!(parameter, "nextPageToken");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn an_unknown_field_in_a_style_is_refused() {
    let text = JIRA.replace(
        "last: /isLast}\n  offset:",
        "last: /isLast, cursor: x}\n  offset:",
    );
    assert_eq!(
        lower("jira", &text, &repository()).refusal().name(),
        "unknown_field"
    );
}

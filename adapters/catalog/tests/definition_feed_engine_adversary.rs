//! Adversary, story:definition-reader-refuses-what-the-feed-engine-refuses: a definition the
//! reader (`connectors_catalog::definition::lower`) lowers must carry a feed the engine
//! (`Engine::with_feed`) loads over the bundle the same lowering wrote. Each case lowers an
//! edited GitLab definition and, if the reader lowers it, builds the engine from its output; a
//! reader that lowers a feed the engine refuses at load is the defect.
#![cfg(unix)]
use connectors_catalog::{
    bundle,
    definition::{self, Request},
};
use connectors_catalog_provider::{Engine, feed::Declaration};
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

/// What the reader and the engine said of one definition.
enum Outcome {
    ReaderRefused(String),
    EngineLoaded,
    EngineRefused(String),
}

fn lower_then_load(text: &str) -> Outcome {
    let root = gitlab_root();
    let temp = tempfile::tempdir().unwrap();
    let providers = temp.path().join("providers");
    std::fs::create_dir_all(&providers).unwrap();
    let scratch = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch).unwrap();
    let out = temp.path().join("out");
    let file = providers.join("gitlab.yaml");
    std::fs::write(&file, text).unwrap();
    let lowered = match definition::lower(&Request {
        definition: &file,
        root: root.path(),
        out: &out,
        scratch: &scratch,
    }) {
        Ok(lowered) => lowered,
        Err(refusal) => return Outcome::ReaderRefused(refusal.to_string()),
    };
    let selection: Value =
        serde_json::from_slice(&std::fs::read(&lowered.operations).unwrap()).unwrap();
    let feed: Declaration =
        serde_json::from_value(selection["feed"].clone()).expect("the lowered feed parses");
    let bundle = bundle::load(&out, &lowered.provider).expect("the lowered bundle loads");
    match Engine::with_feed(&bundle, "/api/v4", &[], Some(&feed)) {
        Ok(_) => Outcome::EngineLoaded,
        Err(error) => Outcome::EngineRefused(error.message),
    }
}

/// The reader and the engine agree on `text`: either the reader refuses it, or the engine loads
/// the feed the reader lowered.
fn agree(text: &str) {
    match lower_then_load(text) {
        Outcome::ReaderRefused(_) | Outcome::EngineLoaded => {}
        Outcome::EngineRefused(message) => {
            panic!("the reader lowered a feed the engine refuses at load: {message}")
        }
    }
}

fn edit(text: &str, from: &str, to: &str) -> String {
    assert_eq!(text.matches(from).count(), 1, "`{from}` occurs once");
    text.replacen(from, to, 1)
}

fn gitlab(edits: &[(&str, &str)]) -> String {
    edits
        .iter()
        .fold(GITLAB.to_owned(), |text, (from, to)| edit(&text, from, to))
}

/// The unedited definition: the reader lowers it and the engine loads it (the harness works).
#[test]
fn adversary_the_gitlab_definition_lowers_and_loads() {
    match lower_then_load(GITLAB) {
        Outcome::EngineLoaded => {}
        Outcome::ReaderRefused(message) => panic!("reader refused: {message}"),
        Outcome::EngineRefused(message) => panic!("engine refused: {message}"),
    }
}

/// The engine's `bind` refuses a lookup that leaves a required parameter unbound ("feed leaves
/// `from` of `…` unbound, which it requires"); the reader binds only the lookup's path parameter.
#[test]
fn adversary_a_lookup_with_a_required_query_parameter() {
    agree(&gitlab(&[(
        "project.get: {op: getApiV4ProjectsId}",
        "project.get: {op: getApiV4ProjectsIdRepositoryChangedPaths}",
    )]));
}

/// The same `bind` rule on `items.list`: `from` is required, and nothing binds it.
#[test]
fn adversary_an_items_list_with_a_required_parameter_left_unbound() {
    agree(&gitlab(&[
        (
            "merge_requests.list: {op: getApiV4ProjectsIdMergeRequests}",
            "merge_requests.list: {op: getApiV4ProjectsIdRepositoryChangedPaths}",
        ),
        (
            "query: {order_by: updated_at, sort: asc, state: all, scope: all}",
            "query: {to: main}",
        ),
        (
            "position: {time: updated_after}",
            "position: {time: find_renames}",
        ),
        (
            "    filters:\n      mine_open: {state: opened, scope: assigned_to_me}\n",
            "",
        ),
    ]));
}

/// The engine binds feed operations only when each is a GET under the base path.
#[test]
fn adversary_a_lookup_that_is_not_a_get() {
    agree(&gitlab(&[(
        "lookup: project.get",
        "lookup: merge_request.create",
    )]));
}

/// The engine takes exactly one of `kind` and `kind_word`; the reader emits `kind_word` only.
#[test]
fn adversary_containers_without_a_kind_word() {
    agree(&gitlab(&[("      kind_word: project\n", "")]));
}

/// The engine refuses a capability claim the declaration does not support (`supports`).
#[test]
fn adversary_a_capability_claim_the_declaration_does_not_support() {
    agree(&gitlab(&[("revision: update-time", "revision: opaque")]));
}

/// The engine refuses a profile that is not 1 to 128 visible ASCII bytes.
#[test]
fn adversary_a_profile_with_a_space() {
    agree(&gitlab(&[(
        "profile: gitlab-merge-requests/1",
        "profile: gitlab merge requests",
    )]));
}

/// The engine refuses a `max_limit` above 100; the reader carries the items' style's `max`.
#[test]
fn adversary_a_page_size_cap_above_one_hundred() {
    agree(&gitlab(&[(
        "limit: per_page, max: 100}",
        "limit: per_page, max: 500}",
    )]));
}

/// The engine refuses an `items.id` that is not a JSON pointer.
#[test]
fn adversary_an_item_id_that_is_not_a_pointer() {
    agree(&gitlab(&[("      id: /iid\n", "      id: iid\n")]));
}

/// The engine refuses a body that names no representation.
#[test]
fn adversary_a_body_without_a_representation() {
    agree(&gitlab(&[(
        "representation: text/markdown",
        "representation: \"\"",
    )]));
}

/// GitLab's definition in `beyond10x/connectors-definitions` on 2026-10-17, as
/// `crates/connectors-catalog/tests/definition_feed.rs` carries it.
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

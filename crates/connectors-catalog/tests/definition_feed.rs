//! Datasources in a `connectors-definition/1` file: GitLab's merge-request feed,
//! Jira's issue feed and Confluence's page feed lower to the shipped
//! `datasource.feed/v1alpha1` declarations exactly; each refusal is named, one
//! for each feed rule the definitions schema states in its domain summary.
#![cfg(unix)]
use connectors_catalog::definition::{self, Refusal, Request};
use serde_json::Value;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// The committed copy of the definitions repository's files.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/definitions")
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

/// The vendored Atlassian sources, each as the definitions root names it and
/// gzipped once for every test.
fn atlassian_sources() -> &'static [(&'static str, Vec<u8>)] {
    static SOURCES: OnceLock<Vec<(&'static str, Vec<u8>)>> = OnceLock::new();
    SOURCES.get_or_init(|| {
        [
            (
                "upstream/jira/jira-platform-v3.json.gz",
                "adapters/atlassian/upstream/jira-platform-v3.json",
            ),
            (
                "upstream/confluence-v1/confluence-rest-v1.yaml.gz",
                "adapters/atlassian/contracts/confluence-rest-v1/confluence-rest-v1.yaml",
            ),
        ]
        .into_iter()
        .map(|(vendored, source)| {
            (
                vendored,
                gzip(&std::fs::read(repository().join(source)).unwrap()),
            )
        })
        .collect()
    })
}

/// A definitions root holding the Jira and Confluence REST v1 sources gzipped
/// and the shared `auth/atlassian.yaml`, as `beyond10x/connectors-definitions`
/// vendors them.
fn atlassian_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    for (vendored, bytes) in atlassian_sources() {
        let path = root.path().join(vendored);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
    }
    std::fs::create_dir_all(root.path().join("auth")).unwrap();
    std::fs::copy(
        fixtures().join("auth/atlassian.yaml"),
        root.path().join("auth/atlassian.yaml"),
    )
    .unwrap();
    root
}

/// `connectors/<name>.yaml` of `beyond10x/connectors-definitions` at `536945d`.
fn definition_text(name: &str) -> String {
    std::fs::read_to_string(fixtures().join(format!("connectors/{name}.yaml"))).unwrap()
}

/// The catalog provider's 0.56.0 feed fixture `adapters/catalog/tests/feed/<directory>/feed.json`.
fn recorded_feed(directory: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(
            repository().join(format!("adapters/catalog/tests/feed/{directory}/feed.json")),
        )
        .unwrap(),
    )
    .unwrap()
}

/// `text` with its one `from` replaced by `to`.
fn edit(text: &str, from: &str, to: &str) -> String {
    assert_eq!(text.matches(from).count(), 1, "`{from}` occurs once");
    text.replacen(from, to, 1)
}

/// The Jira definition with each `(from, to)` edit applied, lowered.
fn jira(edits: &[(&str, &str)]) -> Lowering {
    let root = atlassian_root();
    let text = edits
        .iter()
        .fold(definition_text("jira"), |text, (from, to)| {
            edit(&text, from, to)
        });
    lower("jira", &text, root.path())
}

/// The Confluence REST v1 definition with each `(from, to)` edit applied, lowered.
fn confluence(edits: &[(&str, &str)]) -> Lowering {
    let root = atlassian_root();
    let text = edits
        .iter()
        .fold(definition_text("confluence-v1"), |text, (from, to)| {
            edit(&text, from, to)
        });
    lower("confluence-v1", &text, root.path())
}

/// The lowering is refused by the feed rule `rule`, for the datasource
/// `datasource`, with a message that says `mentions`.
fn assert_rule(lowering: Lowering, rule: &str, datasource: &str, mentions: &str) {
    let refusal = lowering.refusal();
    let message = refusal.to_string();
    assert_eq!(refusal.name(), rule, "refused as {message}");
    assert!(
        message.starts_with(&format!("{rule}: datasource `{datasource}`: ")),
        "{message}"
    );
    assert!(message.contains(mentions), "`{message}` says `{mentions}`");
}

const JIRA_CURSOR: &str =
    "cursor: {parameter: startAt, offset: {complete: {pointer: /isLast, equals: true}}}";
const CONFLUENCE_CURSOR: &str =
    "cursor: {parameter: start, next: /_links/next, url_parameter: start}";
const JIRA_QUERY: &str = "        query:
          parameter: jql
          language: jql
          first: \"project = {container} ORDER BY updated ASC\"
          resume: \"project = {container} AND updated >= {since} ORDER BY updated ASC\"
          utc_offsets: {least: -720, greatest: 840}
";
const JIRA_FIRST: &str = "first: \"project = {container} ORDER BY updated ASC\"";
const JIRA_RESUME: &str =
    "resume: \"project = {container} AND updated >= {since} ORDER BY updated ASC\"";
const JIRA_OFFSETS: &str = "utc_offsets: {least: -720, greatest: 840}";
const JIRA_PAGE: &str = "        page: {parameter: nextPageToken, next: /nextPageToken}\n";

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

// ---- Jira and Confluence: connectors-definitions 536945d ------------------------------------

/// `connectors/jira.yaml` at `536945d` (datasource `issues`, profile
/// `jira-issues/1`) lowers to the `feed` the shipped Jira selection carries,
/// which is the 0.56.0 fixture: an offset container listing ended by
/// `/isLast`, and a JQL query position at minute precision within
/// UTC-12:00..UTC+14:00, continued by `nextPageToken`.
#[test]
fn the_jira_issue_feed_lowers_to_the_shipped_declaration() {
    let fixture = recorded_feed("jira");
    let lowered = jira(&[]).selection();
    assert_eq!(lowered["feed"], fixture, "the lowered feed is the fixture");
    assert_eq!(
        shipped("jira")["feed"],
        fixture,
        "the shipped feed is the fixture"
    );
}

/// `connectors/confluence-v1.yaml` at `536945d` (datasource `pages`, profile
/// `confluence-pages/1`) lowers to the `feed` the shipped `confluence-v1`
/// selection carries, which is the 0.56.0 fixture: spaces continued by the
/// `start` of the next link, a CQL query position continued by its `cursor`,
/// and item links under the answer's base.
#[test]
fn the_confluence_page_feed_lowers_to_the_shipped_declaration() {
    let fixture = recorded_feed("confluence");
    let lowered = confluence(&[]).selection();
    assert_eq!(lowered["feed"], fixture, "the lowered feed is the fixture");
    assert_eq!(
        shipped("confluence-v1")["feed"],
        fixture,
        "the shipped feed is the fixture"
    );
}

/// Rule 1: a cursor names exactly one of `next`, `last` and `offset`; with none
/// the listing has no way to continue.
#[test]
fn a_cursor_naming_none_of_next_last_offset_is_refused() {
    assert_rule(
        jira(&[(JIRA_CURSOR, "cursor: {parameter: startAt}")]),
        "feed_cursor_continuation",
        "issues",
        "names none",
    );
}

/// Rule 1, the other way: two of them leave the continuation ambiguous.
#[test]
fn a_cursor_naming_two_of_next_last_offset_is_refused() {
    assert_rule(
        confluence(&[(
            CONFLUENCE_CURSOR,
            "cursor: {parameter: start, next: /_links/next, last: /key, url_parameter: start}",
        )]),
        "feed_cursor_continuation",
        "pages",
        "names next and last",
    );
    assert_rule(
        jira(&[(
            JIRA_CURSOR,
            "cursor: {parameter: startAt, last: /id, offset: {complete: {pointer: /isLast, equals: true}}}",
        )]),
        "feed_cursor_continuation",
        "issues",
        "names last and offset",
    );
}

/// Rule 2: `url_parameter` reads a parameter of the next link, so it appears
/// only beside `next`.
#[test]
fn url_parameter_without_next_is_refused() {
    assert_rule(
        confluence(&[(
            CONFLUENCE_CURSOR,
            "cursor: {parameter: start, last: /key, url_parameter: start}",
        )]),
        "feed_cursor_url_parameter",
        "pages",
        "url_parameter `start`",
    );
    assert_rule(
        jira(&[(
            JIRA_CURSOR,
            "cursor: {parameter: startAt, url_parameter: startAt, offset: {complete: {pointer: /isLast, equals: true}}}",
        )]),
        "feed_cursor_url_parameter",
        "issues",
        "url_parameter `startAt`",
    );
}

/// Rule 3: a position names exactly one of `time` and `query`; with neither
/// the items have no "changed since" to resume by.
#[test]
fn a_position_naming_neither_time_nor_query_is_refused() {
    assert_rule(
        jira(&[(JIRA_QUERY, "")]),
        "feed_position_filter",
        "issues",
        "names neither",
    );
}

/// Rule 3, the other way: a time parameter beside a query filters twice.
#[test]
fn a_position_naming_both_time_and_query_is_refused() {
    assert_rule(
        jira(&[(JIRA_QUERY, &format!("        time: updated\n{JIRA_QUERY}"))]),
        "feed_position_filter",
        "issues",
        "names both",
    );
}

/// Rule 4: `first` holds `{container}` once, neither missing nor twice (and `{since}`
/// never, `adversary_a_first_template_naming_since_is_refused`).
#[test]
fn a_first_template_without_one_container_is_refused() {
    for (first, count) in [
        ("first: \"ORDER BY updated ASC\"", 0),
        (
            "first: \"project = {container} OR project = {container} ORDER BY updated ASC\"",
            2,
        ),
    ] {
        assert_rule(
            jira(&[(JIRA_FIRST, first)]),
            "feed_query_first",
            "issues",
            &format!("first holds {{container}} {count} times"),
        );
    }
}

/// Rule 5: `resume` holds `{container}` and `{since}` exactly once each.
#[test]
fn a_resume_template_without_one_container_and_one_since_is_refused() {
    for (resume, says) in [
        (
            "resume: \"project = {container} ORDER BY updated ASC\"",
            "{since} 0 times",
        ),
        (
            "resume: \"project = {container} AND updated >= {since} AND created >= {since} ORDER BY updated ASC\"",
            "{since} 2 times",
        ),
        (
            "resume: \"updated >= {since} ORDER BY updated ASC\"",
            "{container} 0 times",
        ),
        (
            "resume: \"project = {container} AND project = {container} AND updated >= {since} ORDER BY updated ASC\"",
            "{container} 2 times",
        ),
    ] {
        assert_rule(
            jira(&[(JIRA_RESUME, resume)]),
            "feed_query_resume",
            "issues",
            says,
        );
    }
}

/// Rule 6: a query position is at `minute` precision, whether another is named
/// or none is.
#[test]
fn a_query_position_not_at_minute_precision_is_refused() {
    for (precision, says) in [
        ("        precision: instant\n", "precision instant"),
        ("", "no precision"),
    ] {
        assert_rule(
            jira(&[("        precision: minute\n", precision)]),
            "feed_query_precision",
            "issues",
            says,
        );
    }
}

/// Rule 7: with a query position the query names the container, so the items
/// operation takes no container parameter; one with a path parameter is
/// refused, naming it.
#[test]
fn items_with_a_query_position_naming_a_container_parameter_is_refused() {
    assert_rule(
        jira(&[("list: issues.search", "list: issue.comments")]),
        "feed_query_container",
        "issues",
        "`issueIdOrKey`",
    );
    assert_rule(
        confluence(&[("list: pages.changed", "list: page.comments")]),
        "feed_query_container",
        "pages",
        "`id`",
    );
}

/// Rule 8: each of `least` and `greatest` lies within a day of UTC, -1439 to
/// 1439 minutes.
#[test]
fn utc_offsets_outside_minus_1439_to_1439_is_refused() {
    for (offsets, says) in [
        ("utc_offsets: {least: -1440, greatest: 840}", "least -1440"),
        (
            "utc_offsets: {least: -720, greatest: 1440}",
            "greatest 1440",
        ),
    ] {
        assert_rule(
            jira(&[(JIRA_OFFSETS, offsets)]),
            "feed_utc_offsets",
            "issues",
            says,
        );
    }
}

/// Rule 8, the order: `least` is not above `greatest`.
#[test]
fn utc_offsets_least_above_greatest_is_refused() {
    assert_rule(
        jira(&[(JIRA_OFFSETS, "utc_offsets: {least: 840, greatest: -720}")]),
        "feed_utc_offsets",
        "issues",
        "least 840 is above greatest -720",
    );
}

/// Rule 9: a resume shifted back by `utc_offsets` re-reads what it already
/// kept, so it needs `page`, the continuation within one read.
#[test]
fn utc_offsets_without_page_is_refused() {
    assert_rule(
        jira(&[(JIRA_PAGE, "")]),
        "feed_utc_offsets_page",
        "issues",
        "no page",
    );
}

/// Rule 10: an item's `url`, `url_base` and `parent` are JSON pointers, empty
/// or starting with `/`.
#[test]
fn a_url_url_base_or_parent_that_is_not_a_pointer_is_refused() {
    for (from, to, says) in [
        (
            "      url: /_links/webui\n",
            "      url: _links/webui\n",
            "items.url `_links/webui`",
        ),
        (
            "      url_base: /_links/base\n",
            "      url_base: _links/base\n",
            "items.url_base `_links/base`",
        ),
        (
            "      url_base: /_links/base\n",
            "      url_base: /_links/base\n      parent: ancestors/0/id\n",
            "items.parent `ancestors/0/id`",
        ),
    ] {
        assert_rule(
            confluence(&[(from, to)]),
            "feed_item_pointer",
            "pages",
            says,
        );
    }
}

/// Rule `feed_position_parameter`: the parameters a position names are query parameters of
/// the items' list operation, which the engine binds them to. `time` is held here; the
/// adversary's cases below hold `query.parameter` and `page.parameter`.
#[test]
fn a_time_parameter_the_items_list_lacks_is_refused() {
    let root = gitlab_root();
    let text = GITLAB.replace(
        "position: {time: updated_after}",
        "position: {time: changed_after}",
    );
    assert_rule(
        lower("gitlab", &text, root.path()),
        "feed_position_parameter",
        "merge_requests",
        "items.position.time `changed_after`",
    );
}

// ---- adversary, wave 20261022 U1 -----------------------------------------------------------

/// The catalog engine loads a query whose `first` names `{since}` as no feed at all
/// (`adapters/catalog/src/feed.rs`, "names `{container}` once and `{since}` never"): the first
/// read has no time to render there. A definition that lowers to it is refused here, by the
/// rule `first` breaks, not by the provider when an instance starts.
#[test]
fn adversary_a_first_template_naming_since_is_refused() {
    assert_rule(
        jira(&[(
            JIRA_FIRST,
            "first: \"project = {container} AND updated >= {since} ORDER BY updated ASC\"",
        )]),
        "feed_query_first",
        "issues",
        "{since}",
    );
}

/// The engine binds `query.parameter` and `page.parameter` as query parameters of the items
/// operation and refuses a feed naming one it does not take ("feed binds `…`, which `…` does not
/// take there"), as the reader already refuses a filter's parameter. A definition naming such a
/// parameter is refused when it lowers, naming the parameter. Each edit is its own case, so
/// one refused for another reason cannot hide the other.
mod adversary_a_query_or_page_parameter_the_items_list_lacks_is_refused {
    use super::{assert_rule, jira};

    #[test]
    fn query_parameter() {
        assert_rule(
            jira(&[("          parameter: jql\n", "          parameter: jqlx\n")]),
            "feed_position_parameter",
            "issues",
            "items.position.query.parameter `jqlx`",
        );
    }

    #[test]
    fn page_parameter() {
        assert_rule(
            jira(&[(
                "page: {parameter: nextPageToken,",
                "page: {parameter: pageTokenx,",
            )]),
            "feed_position_parameter",
            "issues",
            "items.position.page.parameter `pageTokenx`",
        );
    }
}

/// Rule 8 holds both bounds inclusive: `least` and `greatest` at -1439 and 1439, and `least`
/// equal to `greatest`, lower and are carried as written.
#[test]
fn adversary_utc_offsets_at_their_bounds_lower() {
    for (offsets, least, greatest) in [
        ("utc_offsets: {least: -1439, greatest: 1439}", -1439, 1439),
        ("utc_offsets: {least: 330, greatest: 330}", 330, 330),
    ] {
        let lowered = jira(&[(JIRA_OFFSETS, offsets)]).selection();
        assert_eq!(
            lowered["feed"]["items"]["position"]["value"]["query"]["utc_offsets"],
            serde_json::json!({"least": least, "greatest": greatest}),
            "{offsets}"
        );
    }
}

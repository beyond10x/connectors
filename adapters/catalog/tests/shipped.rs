//! The reviewed GitLab selection set shipped with the repository resolves
//! against the committed bundle, carries every operation the retired native
//! GitLab adapter exposed, so one configuration serves the provider, and
//! carries the repository reads the knowledge-ingest consumer selects.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use serde_json::Value;
use std::path::Path;

#[test]
fn shipped_gitlab_selections_resolve_and_cover_the_former_native_surface() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let shipped: Value = serde_json::from_slice(
        &std::fs::read(root.join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(shipped["format"], "connectors-catalog-operations/1");
    assert_eq!(shipped["provider"], "gitlab");
    let selections: Vec<Selection> = serde_json::from_value(shipped["operations"].clone()).unwrap();
    let bundle = bundle::load(&root.join("generated/bundles"), "gitlab").unwrap();
    let engine = Engine::new(&bundle, "/api/v4", &selections).unwrap();
    let mut declared: Vec<String> = engine
        .declarations(&[Effect::Read, Effect::Write])
        .into_iter()
        .map(|o| o.id)
        .collect();
    declared.sort();

    // The complete shipped list: every operation the retired native GitLab
    // adapter declared, except its composite `merge_request.validate`, which
    // is two of these reads and a comparison the merge guard now makes itself;
    // then the repository reads, the unguarded issue create, and the merge-request
    // note, discussion read, reply and guarded resolve, and the repository
    // tree, single commit, commit diff and branch list reads, and the
    // auto-merge and reopen variants of merge and update, and the
    // merge-request diff and discussion list reads. A renamed, dropped or
    // added id fails here.
    let mut expected = vec![
        "project.get",
        "issues.list",
        "file.get",
        "pipelines.list",
        "pipeline.get",
        "pipeline.jobs",
        "job.get",
        "job.trace",
        "merge_request.get",
        "merge_requests.list",
        "merge_request.update",
        "merge_request.merge",
        "merge_request.create",
        "branch.get",
        "projects.list",
        "tags.list",
        "releases.list",
        "project.events",
        "commits.list",
        "repository.compare",
        "deployments.list",
        "issue.create",
        "merge_request.note.create",
        "merge_request.discussion.get",
        "merge_request.discussion.reply",
        "merge_request.discussion.resolve",
        "repository.tree",
        "commit.get",
        "commit.diff",
        "branches.list",
        "merge_request.auto_merge",
        "merge_request.reopen",
        "merge_request.diffs",
        "merge_request.discussions",
    ];
    expected.sort();
    assert_eq!(declared, expected);
    assert_eq!(declared.len(), 34);
    assert_eq!(engine.effect("merge_request.merge"), Some(Effect::Write));
    assert_eq!(engine.effect("job.trace"), Some(Effect::Read));
    assert_eq!(engine.effect("issue.create"), Some(Effect::Write));

    // The repository reads keep the ids and source operations a consumer
    // already selects, and are reads.
    for (id, operation_id) in [
        ("projects.list", "getApiV4Projects"),
        ("tags.list", "getApiV4ProjectsIdRepositoryTags"),
        ("releases.list", "getApiV4ProjectsIdReleases"),
        ("project.events", "getApiV4ProjectsIdEvents"),
        ("commits.list", "getApiV4ProjectsIdRepositoryCommits"),
        ("repository.compare", "getApiV4ProjectsIdRepositoryCompare"),
        ("deployments.list", "getApiV4ProjectsIdDeployments"),
        ("repository.tree", "getApiV4ProjectsIdRepositoryTree"),
        ("commit.get", "getApiV4ProjectsIdRepositoryCommitsSha"),
        ("commit.diff", "getApiV4ProjectsIdRepositoryCommitsShaDiff"),
        ("branches.list", "getApiV4ProjectsIdRepositoryBranches"),
        (
            "merge_request.diffs",
            "getApiV4ProjectsIdMergeRequestsMergeRequestIidDiffs",
        ),
        (
            "merge_request.discussions",
            "getApiV4ProjectsIdMergeRequestsNoteableIdDiscussions",
        ),
    ] {
        let selection = selections
            .iter()
            .find(|s| s.id == id)
            .unwrap_or_else(|| panic!("`{id}` is not shipped"));
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(engine.effect(id), Some(Effect::Read), "`{id}`");
    }
}

/// GitLab serves at most 100 items per page, so every shipped selection whose
/// source operation takes a `per_page` query parameter bounds it at 100, and
/// no other selection carries a bound. A list read added without its bound
/// fails here, not in a caller's truncated walk.
#[test]
fn every_shipped_gitlab_read_that_pages_bounds_per_page_at_the_provider_cap() {
    use connectors_catalog::inventory::Location;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let shipped: Value = serde_json::from_slice(
        &std::fs::read(root.join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    let bundle = bundle::load(&root.join("generated/bundles"), "gitlab").unwrap();
    let mut paged = Vec::new();
    for selection in shipped["operations"].as_array().unwrap() {
        let id = selection["id"].as_str().unwrap();
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == selection["operation_id"].as_str())
            .unwrap();
        let pages = operation
            .parameters
            .iter()
            .any(|p| p.name == "per_page" && p.location == Location::Query);
        let expected = if pages {
            paged.push(id);
            serde_json::json!({"per_page": {"minimum": 1, "maximum": 100}})
        } else {
            Value::Null
        };
        assert_eq!(selection["bounds"], expected, "`{id}`");
    }
    paged.sort();
    assert_eq!(
        paged,
        [
            "branches.list",
            "commit.diff",
            "commits.list",
            "deployments.list",
            "issues.list",
            "merge_request.diffs",
            "merge_request.discussions",
            "merge_requests.list",
            "pipeline.jobs",
            "pipelines.list",
            "project.events",
            "projects.list",
            "releases.list",
            "repository.tree",
            "tags.list",
        ]
    );
}

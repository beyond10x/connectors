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
    // then the repository reads. A renamed, dropped or added id fails here.
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
    ];
    expected.sort();
    assert_eq!(declared, expected);
    assert_eq!(declared.len(), 18);
    assert_eq!(engine.effect("merge_request.merge"), Some(Effect::Write));
    assert_eq!(engine.effect("job.trace"), Some(Effect::Read));

    // The repository reads keep the ids and source operations a consumer
    // already selects, and are reads.
    for (id, operation_id) in [
        ("projects.list", "getApiV4Projects"),
        ("tags.list", "getApiV4ProjectsIdRepositoryTags"),
        ("releases.list", "getApiV4ProjectsIdReleases"),
        ("project.events", "getApiV4ProjectsIdEvents"),
    ] {
        let selection = selections
            .iter()
            .find(|s| s.id == id)
            .unwrap_or_else(|| panic!("`{id}` is not shipped"));
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(engine.effect(id), Some(Effect::Read), "`{id}`");
    }
}

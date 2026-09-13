//! The committed GitLab bundle is exactly what the pipeline produces from the
//! pinned source. Two runs over the same bytes agree, and neither agrees with a
//! bundle somebody edited.
use connectors_catalog::{bundle, pipeline};
use std::path::Path;

#[test]
fn committed_gitlab_bundle_matches_a_fresh_pipeline_run() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let committed = root.join("generated/bundles");
    let source = root.join("../gitlab/upstream/openapi_v3.yaml");
    let temp = tempfile::tempdir().unwrap();
    let run = pipeline::run(&pipeline::Request {
        provider: "gitlab",
        source: &source,
        directory: temp.path(),
        auth_profile: "gitlab.pat",
        replace: false,
    })
    .unwrap();
    assert_eq!(run.coverage.unsupported, 0);
    assert_eq!(
        std::fs::read(temp.path().join("gitlab.bundle.json")).unwrap(),
        std::fs::read(committed.join("gitlab.bundle.json")).unwrap(),
        "committed bundle drifted from the pinned source"
    );
    let fresh = bundle::read_index(temp.path()).unwrap();
    let indexed = bundle::read_index(&committed).unwrap();
    assert_eq!(fresh.entries, indexed.entries);
    let loaded = bundle::load(&committed, "gitlab").unwrap();
    assert_eq!(loaded.inventory.operations.len(), 1847);
    assert!(
        loaded
            .inventory
            .operations
            .iter()
            .any(|o| o.operation_id.as_deref() == Some("postApiV4ProjectsIdMergeRequests"))
    );
}

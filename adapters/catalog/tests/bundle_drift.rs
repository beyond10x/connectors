//! Every committed bundle is exactly what the pipeline produces from its pinned
//! source. One fresh run over every indexed provider reproduces each bundle and
//! the whole index byte for byte, and an indexed provider with no pinned source
//! named here fails rather than going unchecked.
use connectors_catalog::{bundle, pipeline};
use std::path::Path;

/// The pinned source and auth profile of each indexed provider, relative to
/// this crate. The profile is a literal, so a bundle rebuilt with another
/// profile fails here instead of being read back as correct.
const SOURCES: [(&str, &str, &str); 2] = [
    ("gitlab", "../gitlab/upstream/openapi_v3.yaml", "gitlab.pat"),
    (
        "jira",
        "../atlassian/upstream/jira-platform-v3.json",
        "jira.basic",
    ),
];

#[test]
fn every_committed_bundle_matches_a_fresh_pipeline_run() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let committed = root.join("generated/bundles");
    let indexed = bundle::read_index(&committed).unwrap();
    let mut providers: Vec<&str> = indexed
        .entries
        .iter()
        .map(|e| e.provider.as_str())
        .collect();
    providers.sort();
    let mut named: Vec<&str> = SOURCES.iter().map(|(provider, _, _)| *provider).collect();
    named.sort();
    assert_eq!(
        providers, named,
        "every indexed provider needs its pinned source here"
    );
    let temp = tempfile::tempdir().unwrap();
    for (provider, source, auth_profile) in SOURCES {
        let run = pipeline::run(&pipeline::Request {
            provider,
            source: &root.join(source),
            directory: temp.path(),
            auth_profile,
            replace: false,
        })
        .unwrap();
        assert_eq!(run.coverage.unsupported, 0, "`{provider}`");
        let file_name = format!("{provider}.bundle.json");
        assert_eq!(
            std::fs::read(temp.path().join(&file_name)).unwrap(),
            std::fs::read(committed.join(&file_name)).unwrap(),
            "committed `{provider}` bundle drifted from its pinned source"
        );
    }
    assert_eq!(
        std::fs::read(temp.path().join("index.json")).unwrap(),
        std::fs::read(committed.join("index.json")).unwrap(),
        "committed index drifted"
    );
    let fresh = bundle::read_index(temp.path()).unwrap();
    assert_eq!(fresh.entries, indexed.entries);

    let gitlab = bundle::load(&committed, "gitlab").unwrap();
    assert_eq!(gitlab.inventory.operations.len(), 1847);
    assert!(
        gitlab
            .inventory
            .operations
            .iter()
            .any(|o| o.operation_id.as_deref() == Some("postApiV4ProjectsIdMergeRequests"))
    );
    let jira = bundle::load(&committed, "jira").unwrap();
    assert_eq!(jira.auth_profile, "jira.basic");
    assert!(
        jira.inventory
            .operations
            .iter()
            .any(|o| o.operation_id.as_deref() == Some("searchAndReconsileIssuesUsingJql"))
    );
}

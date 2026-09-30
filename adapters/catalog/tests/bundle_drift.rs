//! Every committed bundle is exactly what the pipeline produces from its pinned
//! source. One fresh run over every indexed provider reproduces each bundle and
//! the whole index byte for byte, and an indexed provider with no pinned source
//! named here fails rather than going unchecked.
use connectors_catalog::{bundle, pipeline};
use std::path::Path;

/// The pinned source, auth profile and unsupported-operation count of each
/// indexed provider, relative to this crate. The profile and the count are
/// literals, so a bundle rebuilt with another profile, or one that gains or
/// loses a gap, fails here instead of being read back as correct. Confluence's
/// 30 are writes whose request body is a `$ref`; no shipped read is among them.
const SOURCES: [(&str, &str, &str, usize); 4] = [
    (
        "confluence",
        "../atlassian/upstream/confluence/confluence-v2.json",
        "atlassian.basic",
        30,
    ),
    (
        "gitlab",
        "../gitlab/upstream/openapi_v3.yaml",
        "gitlab.pat",
        0,
    ),
    (
        "hubspot",
        "../hubspot/upstream/crm-objects-2026-09.json",
        "hubspot.private-app",
        0,
    ),
    (
        "jira",
        "../atlassian/upstream/jira-platform-v3.json",
        "atlassian.basic",
        0,
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
    let mut named: Vec<&str> = SOURCES
        .iter()
        .map(|(provider, _, _, _)| *provider)
        .collect();
    named.sort();
    assert_eq!(
        providers, named,
        "every indexed provider needs its pinned source here"
    );
    let temp = tempfile::tempdir().unwrap();
    for (provider, source, auth_profile, unsupported) in SOURCES {
        let run = pipeline::run(&pipeline::Request {
            provider,
            source: &root.join(source),
            directory: temp.path(),
            auth_profile,
            replace: false,
        })
        .unwrap();
        assert_eq!(run.coverage.unsupported, unsupported, "`{provider}`");
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
    assert_eq!(jira.auth_profile, "atlassian.basic");
    assert!(
        jira.inventory
            .operations
            .iter()
            .any(|o| o.operation_id.as_deref() == Some("searchAndReconsileIssuesUsingJql"))
    );
    // Jira and Confluence share one Atlassian profile.
    let confluence = bundle::load(&committed, "confluence").unwrap();
    assert_eq!(confluence.auth_profile, jira.auth_profile);
    assert!(
        confluence
            .inventory
            .operations
            .iter()
            .any(
                |o| o.operation_id.as_deref() == Some("getPages") && o.path == "/wiki/api/v2/pages"
            )
    );
}

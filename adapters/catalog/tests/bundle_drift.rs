//! Every committed bundle is exactly what the pipeline produces from its pinned
//! source. One fresh run over every indexed provider reproduces each bundle and
//! the whole index byte for byte, and an indexed provider with no pinned source
//! named here fails rather than going unchecked. A source projected from a
//! pinned Google Discovery or Swagger 2.0 document is itself regenerated from
//! that document first, with its projection record, and must match byte for
//! byte too.
use connectors_catalog::{bundle, derived, pipeline};
use std::path::Path;

/// The pinned source, the Discovery or Swagger 2.0 document it is projected
/// from (if any), auth profile and unsupported-operation count of each indexed
/// provider, relative to this crate. The profile and the count are literals, so a bundle
/// rebuilt with another profile, or one that gains or loses a gap, fails here
/// instead of being read back as correct. Confluence's 30 are writes whose
/// request body is a `$ref`; no shipped read is among them. GitLab's 67 are
/// array query parameters declared `explode: false`, each sent as the one value
/// a caller gives, as before arrays were read; shipped reads are among them
/// (`labels` and `iids` on issues and merge requests, `topic` on projects), and
/// none of them changes what is sent. Zendesk's 35 are `deepObject` query
/// parameters this pass does not expand, each sent as one value as given: 34
/// `page` parameters that also accept an offset page number, and one nested
/// `filter`; no shipped read is among them.
const SOURCES: [(&str, &str, Option<&str>, &str, usize); 10] = [
    (
        "confluence",
        "../atlassian/upstream/confluence/confluence-v2.json",
        None,
        "atlassian.basic",
        30,
    ),
    (
        "gitlab",
        "../gitlab/upstream/openapi_v3.yaml",
        None,
        "gitlab.pat",
        67,
    ),
    (
        "google-calendar",
        "../google/generated/calendar.openapi.json",
        Some("../google/upstream/calendar/calendar-api.json"),
        "google.oauth",
        0,
    ),
    (
        "google-drive",
        "../google/generated/drive.openapi.json",
        Some("../google/upstream/drive/drive-api.json"),
        "google.oauth",
        0,
    ),
    (
        "google-gmail",
        "../google/generated/gmail.openapi.json",
        Some("../google/upstream/gmail/gmail-api.json"),
        "google.oauth",
        0,
    ),
    (
        "google-slides",
        "../google/generated/slides.openapi.json",
        Some("../google/upstream/slides/slides-api.json"),
        "google.oauth",
        0,
    ),
    (
        "hubspot",
        "../hubspot/upstream/crm-objects-2026-09.json",
        None,
        "hubspot.private-app",
        0,
    ),
    (
        "jira",
        "../atlassian/upstream/jira-platform-v3.json",
        None,
        "atlassian.basic",
        0,
    ),
    (
        "runpod",
        "../runpod/upstream/runpod-rest-v1.json",
        None,
        "runpod.api-key",
        0,
    ),
    (
        "zendesk",
        "../zendesk/upstream/zendesk-support.yaml",
        None,
        "zendesk.basic",
        35,
    ),
];

/// The cited amendments applied to a provider's pinned source, relative to this
/// crate. A provider absent here is built from its source as written.
const AMENDMENTS: [(&str, &str); 1] = [(
    "zendesk",
    "../zendesk/upstream/zendesk-support.amendments.json",
)];

/// The committed projection at `source` and its record beside it are exactly
/// what projecting the pinned document at `from` produces now, with the
/// projector its content names (Discovery or Swagger 2.0).
fn assert_projection_is_fresh(provider: &str, source: &Path, from: &Path) {
    let projection = derived::project(&std::fs::read(from).unwrap()).unwrap();
    assert!(
        std::fs::read(source).unwrap() == projection.openapi(),
        "committed `{provider}` projection drifted from its pinned {} document",
        projection.format()
    );
    let stem = source
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".json"))
        .unwrap();
    let record = source.with_file_name(format!("{stem}.projection.json"));
    assert!(
        std::fs::read(&record).unwrap() == projection.record_bytes(),
        "committed `{provider}` projection record drifted"
    );
}

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
        .map(|(provider, _, _, _, _)| *provider)
        .collect();
    named.sort();
    assert_eq!(
        providers, named,
        "every indexed provider needs its pinned source here"
    );
    let temp = tempfile::tempdir().unwrap();
    for (provider, source, derived_from, auth_profile, unsupported) in SOURCES {
        let source = root.join(source);
        let amendments = AMENDMENTS
            .iter()
            .find(|(name, _)| *name == provider)
            .map(|(_, path)| root.join(path));
        let request = pipeline::Request {
            provider,
            source: &source,
            directory: temp.path(),
            auth_profile,
            replace: false,
            amendments: amendments.as_deref(),
        };
        let run = match derived_from {
            Some(from) => {
                let from = root.join(from);
                assert_projection_is_fresh(provider, &source, &from);
                pipeline::run_derived(&request, &from)
            }
            None => pipeline::run(&request),
        }
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

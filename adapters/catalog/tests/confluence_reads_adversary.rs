//! Adversarial cases for story:catalog-confluence-reads: the documented stop
//! rule against the pinned list schema, the recorded break of the Jira profile,
//! the base-path refusal under the widened `/wiki` base, and the raw values the
//! engine hands the transport for the delta read.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::Result;
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{fs, path::Path, sync::Mutex};

type Call = (Vec<String>, Vec<(String, String)>);

struct Reads {
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls.lock().unwrap().push((
            path.iter().map(|s| s.to_string()).collect(),
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: br#"{"results":[],"_links":{}}"#.to_vec(),
        })
    }
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped(provider: &str) -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &fs::read(root().join(format!("providers/{provider}/operations.json"))).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn pinned() -> Value {
    serde_json::from_slice(
        &fs::read(root().join("../atlassian/upstream/confluence/confluence-v2.json")).unwrap(),
    )
    .unwrap()
}
fn guide() -> String {
    fs::read_to_string(root().join("../../docs/catalog-confluence.md")).unwrap()
}

/// The pinned list schema does not require `version` on a listed page, so a
/// walker applying the documented stop rule will meet a page without
/// `version.createdAt`. The guide's `pages.changed` rule must say what to do
/// with it (keep, skip, stop or refuse); silence leaves each caller to guess,
/// and the guide's own fixture walker panics on it.
#[test]
fn the_stop_rule_says_what_a_page_without_a_version_does() {
    let document = pinned();
    let bulk = &document["components"]["schemas"]["PageBulk"];
    let required: Vec<&str> = bulk["required"]
        .as_array()
        .map(|r| r.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    assert!(bulk["properties"]["version"].is_object());
    assert!(
        !required.contains(&"version"),
        "precondition: the pinned schema lets a listed page omit `version`"
    );
    let guide = guide();
    let rule = guide
        .split("\n- ")
        .find(|bullet| bullet.starts_with("**`pages.changed`**"))
        .expect("the guide's `pages.changed` bullet");
    // Backticks dropped and lowercased, so only a phrase that ties the absence
    // to `version` itself counts; "`_links.next` is absent" does not.
    let plain = rule.replace('`', "").to_lowercase();
    let covers_absent_version = [
        "without version",
        "without a version",
        "no version",
        "lacks version",
        "missing version",
        "absent version",
        "version is absent",
        "version is missing",
        "version.createdat is absent",
        "version.createdat is missing",
    ]
    .iter()
    .any(|phrase| plain.contains(phrase));
    assert!(
        covers_absent_version,
        "the `pages.changed` stop rule does not say what a listed page without `version.createdAt` does:\n{rule}"
    );
}

/// The coordinator's decision and the wave README both say the move from
/// `jira.basic` to `atlassian.basic` is a breaking change recorded in the
/// CHANGELOG. A 0.16/0.17 operator reads the CHANGELOG, not the Jira guide.
#[test]
fn the_changelog_records_the_jira_profile_break() {
    let changelog = fs::read_to_string(root().join("../../CHANGELOG.md")).unwrap();
    assert!(
        changelog.contains("jira.basic") && changelog.contains("atlassian.basic"),
        "CHANGELOG.md does not record that `jira.basic` became `atlassian.basic`"
    );
}

/// The Confluence base is the site's `/wiki` root. A selection outside the
/// configured base path is still refused at load, both ways between the two
/// Atlassian bundles.
#[test]
fn a_selection_outside_the_configured_base_is_still_refused() {
    let directory = root().join("generated/bundles");
    let confluence = bundle::load(&directory, "confluence").unwrap();
    let jira = bundle::load(&directory, "jira").unwrap();
    assert!(Engine::new(&confluence, "/wiki", &shipped("confluence")).is_ok());
    assert!(Engine::new(&confluence, "/wiki/api/v2", &shipped("confluence")).is_ok());
    for base in ["/rest/api/3", "/wiki/rest/api", "/wiki/api/v1", "/api/v2"] {
        assert!(
            Engine::new(&confluence, base, &shipped("confluence")).is_err(),
            "confluence under {base}"
        );
    }
    assert!(Engine::new(&jira, "/wiki", &shipped("jira")).is_err());
}

/// `pages.changed` hands the transport the space ids, the sort and an opaque
/// cursor unchanged, below the `/wiki` base, in the document's parameter order.
#[tokio::test]
async fn pages_changed_passes_space_ids_sort_and_cursor_through_raw() {
    let bundle = bundle::load(&root().join("generated/bundles"), "confluence").unwrap();
    let engine = Engine::new(&bundle, "/wiki", &shipped("confluence")).unwrap();
    let http = Reads {
        calls: Mutex::new(Vec::new()),
    };
    engine
        .read(
            &http,
            "fixture",
            "pages.changed",
            json!({"space-id": "65538,98305", "sort": "-modified-date",
                   "body-format": "storage", "cursor": "eyJpZCI6MX0=+/", "limit": "250"}),
        )
        .await
        .unwrap();
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, ["api", "v2", "pages"]);
    let query: Vec<(&str, &str)> = calls[0]
        .1
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    assert!(query.contains(&("space-id", "65538,98305")), "{query:?}");
    assert!(query.contains(&("sort", "-modified-date")), "{query:?}");
    assert!(query.contains(&("body-format", "storage")), "{query:?}");
    assert!(query.contains(&("cursor", "eyJpZCI6MX0=+/")), "{query:?}");
    assert!(query.contains(&("limit", "250")), "{query:?}");
    assert_eq!(query.len(), 5, "{query:?}");
}

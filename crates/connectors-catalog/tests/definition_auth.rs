//! Auth profiles in a `connectors-definition/1` file: several per connector, one
//! default, shared through `auth/<name>.yaml` and extended per connector. They
//! lower into the connector declaration, where each profile equals the `auth`
//! block the connector's guide documents.
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
    fn lowered(&self) -> &definition::Lowered {
        self.outcome.as_ref().expect("the definition lowers")
    }
    fn json(path: &Path) -> Value {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }
    fn declaration(&self) -> Value {
        Self::json(&self.lowered().connector)
    }
    fn profile(&self, id: &str) -> Value {
        self.declaration()["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|profile| profile["profile"] == id)
            .unwrap_or_else(|| panic!("the declaration has no profile `{id}`"))
            .clone()
    }
    fn selection(&self) -> Value {
        Self::json(&self.lowered().operations)
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

const ATLASSIAN: &str = "schema: urn:beyond10x:connectors-auth:1
profiles:
  atlassian.basic: {scheme: basic, label: API token, account_label: Account email, identity: {kind: atlassian.account, subject: /accountId}}
  atlassian.bearer: {scheme: bearer, label: Atlassian access token, identity: {kind: atlassian.account, subject: /accountId}}
";

const JIRA: &str = "schema: urn:beyond10x:connectors-definition:1
source:
  openapi: upstream/jira/jira-platform-v3.json.gz
  sha256: 655b4790a5c8543c81755c8250c6a8b8f58ee62712168a39d628d6d927549709
base_url: https://{site}.atlassian.net
config:
  site: {type: string}
auth:
  atlassian.basic: {use: atlassian, default: true, identity: {path: myself}}
  atlassian.bearer: {use: atlassian, identity: {path: myself}}
operations:
  issue.get: {op: getIssue}
";

const CONFLUENCE_V1: &str = "schema: urn:beyond10x:connectors-definition:1
source:
  openapi: upstream/confluence-v1/confluence-rest-v1.yaml.gz
  sha256: fed740a5343255686c481d63c53644eb3163a90676061f9fb1c12659c4fa234e
base_url: https://{site}.atlassian.net/wiki
config:
  site: {type: string}
auth:
  atlassian.basic: {use: atlassian, identity: {path: rest/api/user/current}, access: {path: rest/api/space, query: {limit: \"1\"}}}
paging:
  offset: {request: start, limit: limit, max: 100}
  cursor: {request: cursor, next: /_links/next, limit: limit, max: 100}
operations:
  spaces.list: {op: getSpaces}
  space.get: {op: getSpace}
  space.pages: {op: getContent}
  pages.changed: {op: searchContentByCQL}
  page.get: {op: getContentById}
  page.comments: {op: getContentComments}
  content.search: {op: searchByCQL}
";

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
operations:
  project.get: {op: getApiV4ProjectsId}
";

/// A definitions root as `beyond10x/connectors-definitions` lays it out: the
/// shared Atlassian profiles and the vendored sources, gzipped.
fn root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let put = |relative: &str, bytes: &[u8]| {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    };
    let read = |relative: &str| std::fs::read(repository().join(relative)).unwrap();
    put("auth/atlassian.yaml", ATLASSIAN.as_bytes());
    put(
        "upstream/jira/jira-platform-v3.json.gz",
        &read("adapters/atlassian/upstream/vendor/jira-platform-v3.json.gz"),
    );
    put(
        "upstream/confluence-v1/confluence-rest-v1.yaml.gz",
        &gzip(&read(
            "adapters/atlassian/contracts/confluence-rest-v1/confluence-rest-v1.yaml",
        )),
    );
    put(
        "upstream/gitlab/openapi_v3.yaml.gz",
        &gzip(&read("adapters/gitlab/upstream/openapi_v3.yaml")),
    );
    put(
        "upstream/gitlab/openapi_v3.amendments.json",
        &read("adapters/gitlab/upstream/openapi_v3.amendments.json"),
    );
    root
}

/// The `auth` block of the first JSON configuration in `guide` naming
/// `profile` whose text also contains `marker`.
fn documented(guide: &str, profile: &str, marker: &str) -> Value {
    let text = std::fs::read_to_string(repository().join(guide)).unwrap();
    let block = text
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains(&format!("\"profile\": \"{profile}\"")) && body.contains(marker))
        .unwrap_or_else(|| {
            panic!("{guide} documents no `{profile}` configuration with `{marker}`")
        });
    serde_json::from_str::<Value>(block).unwrap()["auth"].clone()
}

#[test]
fn jira_takes_both_shared_atlassian_profiles_and_records_the_default() {
    let root = root();
    let lowering = lower("jira", JIRA, root.path());
    assert_eq!(
        Lowering::json(&lowering.lowered().bundle)["auth_profile"],
        "atlassian.basic"
    );
    let declaration = lowering.declaration();
    assert_eq!(declaration["format"], "connectors-catalog-connector/1");
    assert_eq!(declaration["connector_id"], "jira");
    assert_eq!(declaration["provider_id"], "atlassian");
    assert_eq!(declaration["catalog_provider"], "jira");
    assert_eq!(declaration.get("default_endpoint"), None);
    let ids: Vec<&str> = declaration["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|profile| profile["profile"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["atlassian.basic", "atlassian.bearer"]);
    assert_eq!(
        lowering.profile("atlassian.basic"),
        documented(
            "docs/catalog-jira.md",
            "atlassian.basic",
            "\"provider\": \"jira\""
        )
    );
    assert_eq!(
        lowering.profile("atlassian.bearer"),
        documented(
            "docs/catalog-jira.md",
            "atlassian.bearer",
            "\"provider\": \"jira\""
        )
    );
}

#[test]
fn confluence_extends_the_shared_basic_profile_and_lowers_to_the_shipped_bundle_and_bounds() {
    let root = root();
    let lowering = lower("confluence-v1", CONFLUENCE_V1, root.path());
    assert_eq!(
        lowering.profile("atlassian.basic"),
        documented(
            "docs/catalog-confluence.md",
            "atlassian.basic",
            "\"path\": \"rest/api/space\""
        )
    );
    assert_eq!(
        std::fs::read(&lowering.lowered().bundle).unwrap(),
        std::fs::read(
            repository().join("adapters/catalog/generated/bundles/confluence-v1.bundle.json")
        )
        .unwrap()
    );
    let shipped = Lowering::json(
        &repository().join("adapters/catalog/providers/confluence-v1/operations.json"),
    );
    for entry in lowering.selection()["operations"].as_array().unwrap() {
        let id = entry["id"].as_str().unwrap();
        let expected = shipped["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .unwrap_or_else(|| panic!("operations.json lacks `{id}`"));
        for field in ["operation_id", "effect", "bounds"] {
            assert_eq!(entry.get(field), expected.get(field), "`{id}` {field}");
        }
    }
}

#[test]
fn gitlab_sends_its_token_in_a_named_header_and_probes_its_scopes() {
    let root = root();
    let lowering = lower("gitlab", GITLAB, root.path());
    assert_eq!(
        lowering.profile("gitlab.pat"),
        documented(
            "docs/local-catalog-provider.md",
            "gitlab.pat",
            "PRIVATE-TOKEN"
        )
    );
}

#[test]
fn a_defaulted_host_is_the_default_endpoint_and_the_path_under_a_variable_host_is_api_path() {
    let root = root();
    let declaration = lower("gitlab", GITLAB, root.path()).declaration();
    assert_eq!(declaration["default_endpoint"], "https://gitlab.com");
    assert_eq!(declaration.get("api_path"), None);
    let text = GITLAB.replace(
        "base_url: https://{hostname}\n",
        "base_url: https://{hostname}/api/v4\n",
    );
    let declaration = lower("gitlab", &text, root.path()).declaration();
    assert_eq!(declaration["default_endpoint"], "https://gitlab.com");
    assert_eq!(declaration["api_path"], "/api/v4");
    let text = text.replace("{type: string, default: gitlab.com}", "{type: string}");
    let declaration = lower("gitlab", &text, root.path()).declaration();
    assert_eq!(declaration.get("default_endpoint"), None);
    assert_eq!(declaration["api_path"], "/api/v4");
}

#[test]
fn a_token_url_on_the_base_origin_is_written_as_its_path() {
    let root = root();
    let text = GITLAB.replace(
        "  gitlab.pat: {scheme: header, header: PRIVATE-TOKEN,",
        "  gitlab.oauth: {scheme: oauth2_client_credentials, token_url: \"https://{hostname}/oauth/token\", scopes: {requested: [api]}, identity: {path: user, kind: gitlab.user, subject: /id}, label: OAuth client secret}\n  gitlab.pat: {default: true, scheme: header, header: PRIVATE-TOKEN,",
    );
    let lowering = lower("gitlab", &text, root.path());
    let oauth = lowering.profile("gitlab.oauth");
    assert_eq!(oauth["token_url"], "/oauth/token");
    assert_eq!(oauth["scheme"], "oauth2_client_credentials");
    assert_eq!(oauth["bearer"], true);
}

#[test]
fn profiles_of_two_providers_are_refused() {
    let root = root();
    let text = GITLAB.replace(
        "  gitlab.pat: {scheme: header,",
        "  code.pat: {default: true, scheme: header, header: PRIVATE-TOKEN, label: Code token, identity: {path: user, kind: code.user}}\n  gitlab.pat: {scheme: header,",
    );
    assert_eq!(
        lower("gitlab", &text, root.path()).refusal().name(),
        "provider_id"
    );
}

#[test]
fn a_shared_file_without_the_profile_is_refused() {
    let root = root();
    let text = JIRA.replace(
        "  atlassian.bearer: {use: atlassian,",
        "  atlassian.oauth: {use: atlassian,",
    );
    match lower("jira", &text, root.path()).refusal() {
        Refusal::AuthProfileMissing { key, library } => {
            assert_eq!(key, "atlassian.oauth");
            assert_eq!(library, "atlassian");
        }
        other => panic!("refused as {other:?}"),
    }
}

#[test]
fn a_use_that_is_not_a_plain_name_or_names_no_file_is_refused() {
    let root = root();
    for library in ["../atlassian", "Atlassian", "missing"] {
        let text = JIRA.replace(
            "{use: atlassian, default",
            &format!("{{use: {library}, default"),
        );
        assert_eq!(
            lower("jira", &text, root.path()).refusal().name(),
            "auth_library",
            "use: {library}"
        );
    }
}

#[test]
fn a_profile_missing_a_field_its_scheme_needs_is_refused() {
    let root = root();
    let text = GITLAB.replace(" header: PRIVATE-TOKEN,", "");
    match lower("gitlab", &text, root.path()).refusal() {
        Refusal::AuthIncomplete { key, field } => {
            assert_eq!(key, "gitlab.pat");
            assert_eq!(field, "header");
        }
        other => panic!("refused as {other:?}"),
    }
}

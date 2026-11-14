//! Each catalog connector ships its declaration (`providers/<name>/connector.json`),
//! and the provider binary builds an instance's native configuration from a
//! connector name, an instance name and an optional endpoint. A derived
//! configuration digests exactly as an equal hand-written one, and every
//! hand-written configuration keeps the revision it had before declarations.
#![cfg(unix)]

use crate::hand_written;

use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const BINARY: &str = env!("CARGO_BIN_EXE_connectors-catalog-provider");
const DECLARATION: &str = "connector.json";
const DECLARATION_FORMAT: &str = "connectors-catalog-connector/1";
const GMAIL_READONLY: &str = "https://www.googleapis.com/auth/gmail.readonly";
const GMAIL_COMPOSE: &str = "https://www.googleapis.com/auth/gmail.compose";
/// The catalog providers that ship no declaration and stay pinned. Grafana's
/// connector name, `grafana`, is the native `connectors-grafana` connector's, so
/// its catalog selection runs from its definition file or as a pinned instance.
const UNDECLARED: [&str; 2] = ["grafana", "openrouter"];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn root_path(relative: &str) -> PathBuf {
    root().join(relative).canonicalize().unwrap()
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

/// A hand-written configuration whose paths are relative to the crate root,
/// made absolute and written owner-only, as `--local-config` requires.
struct Written {
    _directory: tempfile::TempDir,
    path: PathBuf,
}
fn written(mut config: Value) -> Written {
    for key in ["bundle_directory", "operations_file"] {
        if let Some(relative) = config[key].as_str() {
            config[key] = json!(root_path(relative));
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("catalog.json");
    private(&path, &serde_json::to_vec(&config).unwrap());
    Written {
        _directory: directory,
        path,
    }
}

fn run(args: &[&str]) -> Output {
    Command::new(BINARY).args(args).output().unwrap()
}
fn bootstrap(output: Output) -> Value {
    assert!(
        output.status.success(),
        "exit {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn legacy(config: &Value) -> Value {
    let file = written(config.clone());
    bootstrap(run(&[
        "--local-config",
        file.path.to_str().unwrap(),
        "--print-local-bootstrap",
    ]))
}
fn derived(args: &[&str]) -> Output {
    let mut all = args.to_vec();
    all.push("--print-local-bootstrap");
    run(&all)
}

/// Every `providers/<name>/connector.json`, by directory name.
fn declarations() -> BTreeMap<String, Value> {
    let mut found = BTreeMap::new();
    for entry in fs::read_dir(root().join("providers")).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().join(DECLARATION);
        if path.exists() {
            found.insert(
                entry.file_name().to_str().unwrap().to_owned(),
                read_json(&path),
            );
        }
    }
    found
}

/// Every native configuration the guides document, as `(page, configuration)`,
/// with its `/absolute/path/adapters/catalog/` prefix made crate-relative: those
/// a guide still shows, then those it showed before connectors were declared
/// (`hand_written`), in the order the guide showed them, then those a website guide shows.
fn documented() -> Vec<(String, Value)> {
    let mut found = Vec::new();
    let mut pages: Vec<PathBuf> = fs::read_dir(root().join("../../docs"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_str().unwrap();
            (name.starts_with("catalog-") || name == "local-catalog-provider.md")
                && name.ends_with(".md")
        })
        .collect();
    pages.sort();
    for page in pages {
        let name = page.file_name().unwrap().to_str().unwrap();
        for body in hand_written::documented_blocks(name)
            .iter()
            .filter(|body| body.contains("\"format\": \"connectors-catalog-local/2\""))
        {
            let mut config: Value = serde_json::from_str(body).unwrap();
            for key in ["bundle_directory", "operations_file"] {
                if let Some(path) = config[key].as_str() {
                    let relative = path
                        .strip_prefix("/absolute/path/adapters/catalog/")
                        .expect("a documented catalog path");
                    config[key] = json!(relative);
                }
            }
            found.push((name.to_owned(), config));
        }
    }
    // A guide page of the website that shows a native configuration: a pinned instance of a
    // connector that cannot be derived because its client id is the instance's configuration.
    let mut pages: Vec<PathBuf> = fs::read_dir(root().join("../../website/docs/connectors"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .collect();
    pages.sort();
    for page in pages {
        let name = page.file_name().unwrap().to_str().unwrap();
        let text = fs::read_to_string(&page).unwrap();
        for body in text
            .split("```json\n")
            .skip(1)
            .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
            .filter(|body| body.contains("\"format\": \"connectors-catalog-local/2\""))
        {
            let mut config: Value = serde_json::from_str(body).unwrap();
            for key in ["bundle_directory", "operations_file"] {
                if let Some(path) = config[key].as_str() {
                    let relative = path
                        .strip_prefix("/absolute/path/adapters/catalog/")
                        .expect("a documented catalog path");
                    config[key] = json!(relative);
                }
            }
            found.push((format!("website/docs/connectors/{name}"), config));
        }
    }
    found
}

/// The origin of an https URL: scheme and authority, no path.
fn origin(url: &str) -> &str {
    let rest = url.strip_prefix("https://").unwrap();
    let end = rest.find('/').unwrap_or(rest.len());
    &url[.."https://".len() + end]
}
/// A declared `token_url` that starts with `/` is on the instance's endpoint host.
fn resolved_auth(profile: &Value, endpoint: &str) -> Value {
    let mut auth = profile.clone();
    if let Some(path) = auth["token_url"].as_str().filter(|p| p.starts_with('/')) {
        auth["token_url"] = json!(format!("{}{path}", origin(endpoint)));
    }
    auth
}
fn scopes(auth: &Value, key: &str) -> BTreeSet<String> {
    auth[key]
        .as_array()
        .map(|values| {
            values
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// Acceptance: given only the connector name `gmail` and an instance name, the
/// provider prints the revision of the committed reference configuration, whose
/// `auth` requests both the read-only and the compose scope.
#[test]
fn gmail_derived_from_its_declaration_has_the_reference_revision() {
    let reference = read_json(&root().join("tests/connector_declarations/gmail.json"));
    for key in ["minimum_scopes", "requested_scopes"] {
        let named = scopes(&reference["auth"], key);
        assert!(named.contains(GMAIL_READONLY) && named.contains(GMAIL_COMPOSE));
    }
    let expected = legacy(&reference);
    let actual = bootstrap(derived(&[
        "--connector",
        "gmail",
        "--instance",
        "private",
        "--profile",
        "google.oauth",
    ]));
    assert_eq!(
        actual["configuration_revision"],
        expected["configuration_revision"]
    );
    assert_eq!(actual, expected);
    assert_eq!(actual["instance"], "gmail.private");
}

/// OpenRouter holds one selection run as a management and an inference instance
/// joined by custody, so it ships no declaration and stays pinned, and Grafana's
/// catalog selection ships none beside the native `grafana` connector; every other
/// catalog provider directory ships exactly one, naming its own directory, its
/// own selection and a distinct connector. Slack's declares the bot selection
/// (`operations.json`) as the connector `slack`; its user selection
/// (`user-operations.json`) stays a pinned instance, because a directory holds
/// one declaration.
#[test]
fn every_catalog_provider_but_openrouter_ships_one_declaration() {
    let declarations = declarations();
    let directories: BTreeSet<String> = fs::read_dir(root().join("providers"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_str().unwrap().to_owned())
        .collect();
    let expected: BTreeSet<String> = directories
        .iter()
        .filter(|name| !UNDECLARED.contains(&name.as_str()))
        .cloned()
        .collect();
    assert_eq!(
        declarations.keys().cloned().collect::<BTreeSet<_>>(),
        expected
    );
    let mut connectors = BTreeSet::new();
    for (directory, declaration) in &declarations {
        assert_eq!(declaration["format"], DECLARATION_FORMAT, "{directory}");
        assert_eq!(declaration["adapter_id"], "catalog", "{directory}");
        assert_eq!(declaration["catalog_provider"], directory.as_str());
        let connector = declaration["connector_id"].as_str().unwrap();
        assert!(connectors.insert(connector.to_owned()), "{connector} twice");
        assert!(declaration["provider_id"].as_str().is_some(), "{directory}");
        let selection = declaration["selection"].as_str().unwrap();
        let operations = read_json(&root().join("providers").join(directory).join(selection));
        assert_eq!(operations["provider"], directory.as_str());
        let profiles = declaration["profiles"].as_array().unwrap();
        assert!(!profiles.is_empty(), "{directory}");
        let ids: BTreeSet<&str> = profiles
            .iter()
            .map(|p| p["profile"].as_str().unwrap())
            .collect();
        assert_eq!(ids.len(), profiles.len(), "{directory}: a profile twice");
    }
    assert_eq!(
        connectors,
        [
            "basecamp",
            "calendar",
            "chatgpt",
            "claude-subscription",
            "confluence",
            "confluence-v1",
            "drive",
            "gitlab",
            "gmail",
            "hubspot",
            "jira",
            "opsgenie",
            "runpod",
            "slack",
            "slides",
            "zendesk",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    );
}

/// Every declared profile is one the guides document for that provider, field
/// for field, except its scopes: it requests every scope a documented instance
/// of it requests, so the read instance and the write instance become one. A
/// Google connector's `minimum_scopes` is checked against the pinned Discovery
/// document instead (`google_minimum_scopes_are_the_smallest_set_covering_the_selection`);
/// any other's equals the documented one. A configuration through an
/// API gateway (`request_prefix`) names the user's site in its path, which no
/// declaration fixes; such an instance keeps its hand-written configuration.
#[test]
fn declared_profiles_are_the_documented_ones_with_every_scope() {
    let declarations = declarations();
    let documented = documented();
    let mut seen = BTreeSet::new();
    for (page, config) in &documented {
        let provider = config["provider"].as_str().unwrap();
        // A declaration whose `endpoint_path` the endpoint fills (Basecamp's
        // account) derives a `request_prefix`, so its documented
        // configuration carries one too.
        let fills_endpoint_path = declarations
            .get(provider)
            .is_some_and(|declaration| declaration.get("endpoint_path").is_some());
        if config.get("request_prefix").is_some() && !fills_endpoint_path {
            continue;
        }
        let Some(declaration) = declarations.get(provider) else {
            assert!(
                UNDECLARED.contains(&provider),
                "{page}: {provider} has no declaration"
            );
            continue;
        };
        // A configuration of another selection of the provider (Slack's user
        // selection) is a pinned instance its declaration does not cover.
        let selection = format!("/{}", declaration["selection"].as_str().unwrap());
        if config["operations_file"]
            .as_str()
            .is_some_and(|file| !file.ends_with(&selection))
        {
            continue;
        }
        let id = config["auth"]["profile"].as_str().unwrap();
        let profile = declaration["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["profile"] == id)
            .unwrap_or_else(|| panic!("{page}: {provider} declares no {id}"));
        let endpoint = config["api_base"].as_str().unwrap();
        let mut declared = resolved_auth(profile, endpoint);
        let mut written = config["auth"].clone();
        for key in ["minimum_scopes", "requested_scopes"] {
            let (named, documented) = (scopes(&declared, key), scopes(&written, key));
            if key == "requested_scopes" {
                assert!(
                    named.is_superset(&documented),
                    "{page}: {provider} {id} {key}"
                );
            } else if !provider.starts_with("google-") {
                assert_eq!(named, documented, "{page}: {provider} {id} {key}");
            }
            declared.as_object_mut().unwrap().remove(key);
            written.as_object_mut().unwrap().remove(key);
        }
        assert_eq!(declared, written, "{page}: {provider} {id}");
        seen.insert((provider.to_owned(), id.to_owned()));
    }
    for (directory, declaration) in &declarations {
        for profile in declaration["profiles"].as_array().unwrap() {
            let id = profile["profile"].as_str().unwrap().to_owned();
            assert!(
                seen.contains(&(directory.clone(), id.clone())),
                "{directory} declares {id}, which no guide documents"
            );
        }
    }
    let gmail = declarations["google-gmail"]["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|profile| profile["profile"] == WRITE_PROFILE)
        .expect("Gmail declares the write-capable profile");
    for key in ["minimum_scopes", "requested_scopes"] {
        let named = scopes(gmail, key);
        assert!(named.contains(GMAIL_READONLY) && named.contains(GMAIL_COMPOSE));
    }
}

/// For every declared connector and profile, the derived configuration's
/// bootstrap equals that of a hand-written configuration with the
/// declaration's fields: file paths are not digested. A grant profile that
/// ships no client id is refused as invalid configuration instead.
#[test]
fn every_declared_profile_derives_what_its_hand_written_configuration_derives() {
    let documented = documented();
    let mut checked = Vec::new();
    let mut refused = Vec::new();
    for (directory, declaration) in declarations() {
        let connector = declaration["connector_id"].as_str().unwrap();
        for profile in declaration["profiles"].as_array().unwrap() {
            let id = profile["profile"].as_str().unwrap();
            // A grant profile that ships no client id cannot be derived: the client id is the
            // instance's configuration, which a derived instance does not carry.
            if profile["scheme"] == "oauth2_grant" && profile["oauth2"]["client_id"].is_null() {
                let output = derived(&[
                    "--connector",
                    connector,
                    "--instance",
                    "fixture",
                    "--profile",
                    id,
                ]);
                assert!(!output.status.success(), "{connector} {id} derived");
                assert_eq!(
                    String::from_utf8_lossy(&output.stderr).trim(),
                    "InvalidConfiguration",
                    "{connector} {id}"
                );
                refused.push(format!("{connector} {id}"));
                continue;
            }
            let endpoint = match declaration["default_endpoint"].as_str() {
                Some(endpoint) => endpoint.to_owned(),
                None => documented
                    .iter()
                    .map(|(_, config)| config)
                    .find(|config| {
                        config["provider"] == directory.as_str()
                            && config["auth"]["profile"] == id
                            && (config.get("request_prefix").is_none()
                                || declaration.get("endpoint_path").is_some())
                    })
                    .and_then(|config| config["api_base"].as_str())
                    .unwrap_or_else(|| panic!("{directory}: no documented endpoint"))
                    .to_owned(),
            };
            let mut hand = json!({
                "format": "connectors-catalog-local/2",
                "instance": format!("{connector}.fixture"),
                "provider": directory,
                "bundle_directory": "generated/bundles",
                "api_base": format!(
                    "{endpoint}{}",
                    declaration["api_path"].as_str().unwrap_or_default()
                ),
                "auth": resolved_auth(profile, &endpoint),
                "operations_file": format!(
                    "providers/{directory}/{}",
                    declaration["selection"].as_str().unwrap()
                ),
            });
            // The endpoint's path names what `endpoint_path` stands for, and is
            // the request prefix in front of the bundle's paths.
            if declaration.get("endpoint_path").is_some() {
                let rest = endpoint.strip_prefix("https://").unwrap();
                let path = &rest[rest.find('/').expect("an endpoint with a path")..];
                hand["request_prefix"] = json!(path.trim_end_matches('/'));
            }
            let expected = legacy(&hand);
            let mut args = vec![
                "--connector",
                connector,
                "--instance",
                "fixture",
                "--profile",
                id,
            ];
            if declaration["default_endpoint"].is_null() {
                args.extend(["--endpoint", endpoint.as_str()]);
            }
            let actual = bootstrap(derived(&args));
            assert_eq!(actual, expected, "{connector} {id}");
            checked.push(format!("{connector} {id}"));
        }
    }
    // Fourteen connectors, Slack's since 0.63.1 and Basecamp's, whose endpoint names the
    // account, Zendesk with two profiles and Gmail, Calendar and Drive with two each (a
    // read-only one and a write-capable one): a lane that finds no declaration checks nothing
    // and must not pass. ChatGPT's one profile and Claude's two are refused.
    assert_eq!(checked.len(), 18, "{checked:?}");
    assert_eq!(
        refused,
        [
            "chatgpt chatgpt.subscription",
            "claude-subscription claude.subscription",
            "claude-subscription claude.subscription-pasted"
        ]
    );
}

/// Scopes the minimality check keeps although every operation that accepts
/// them also accepts another declared scope: Discovery names the scopes a
/// method accepts, not what each reaches.
const KEPT_FOR_REACH: [(&str, &str, &str); 1] = [(
    "google-drive",
    "https://www.googleapis.com/auth/drive.readonly",
    "`drive.file` reaches only the files the app uses (docs/catalog-google-drive.md, Authentication)",
)];

/// Every object of a Discovery document with an `id` and `scopes`: its methods.
fn discovery_methods(value: &Value, found: &mut BTreeMap<String, BTreeSet<String>>) {
    match value {
        Value::Object(object) => {
            if let (Some(id), Some(Value::Array(scopes))) = (
                object.get("id").and_then(Value::as_str),
                object.get("scopes"),
            ) {
                found.insert(
                    id.to_owned(),
                    scopes
                        .iter()
                        .map(|s| s.as_str().unwrap().to_owned())
                        .collect(),
                );
            }
            object
                .values()
                .for_each(|value| discovery_methods(value, found));
        }
        Value::Array(values) => values
            .iter()
            .for_each(|value| discovery_methods(value, found)),
        _ => {}
    }
}
/// Every `operation_id` a selection names, guard reads included.
fn operation_ids(value: &Value, found: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(id) = object.get("operation_id").and_then(Value::as_str) {
                found.insert(id.to_owned());
            }
            object
                .values()
                .for_each(|value| operation_ids(value, found));
        }
        Value::Array(values) => values.iter().for_each(|value| operation_ids(value, found)),
        _ => {}
    }
}

/// A Google connector's `minimum_scopes` is the smallest set that covers its
/// selection: every operation it names, guard reads included, accepts one of
/// them in the pinned Discovery document, and dropping any one of them leaves
/// an operation that accepts none of the rest, unless `KEPT_FOR_REACH` names
/// it. A grant of that set alone is then never refused as insufficient.
#[test]
fn google_minimum_scopes_are_the_smallest_set_covering_the_selection() {
    let mut checked = Vec::new();
    for (directory, declaration) in declarations() {
        let Some(api) = directory.strip_prefix("google-") else {
            continue;
        };
        let mut accepted = BTreeMap::new();
        discovery_methods(
            &read_json(&root().join(format!("../google/upstream/{api}/{api}-api.json"))),
            &mut accepted,
        );
        let selection = read_json(
            &root()
                .join("providers")
                .join(&directory)
                .join(declaration["selection"].as_str().unwrap()),
        );
        // A read-only profile offers the reads of the selection alone, so its
        // scopes cover those, guard reads of a write included only for the
        // profile that has the write.
        let operations_of = |read_only: bool| -> BTreeSet<String> {
            let mut kept = selection.clone();
            if read_only {
                kept["operations"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|operation| operation["effect"] == "read");
            }
            let mut found = BTreeSet::new();
            operation_ids(&kept, &mut found);
            found
        };
        let uncovered =
            |operations: &BTreeSet<String>, minimum: &BTreeSet<String>| -> Vec<String> {
                operations
                    .iter()
                    .filter(|id| {
                        accepted
                            .get(*id)
                            .unwrap_or_else(|| panic!("{directory}: Discovery has no {id}"))
                            .is_disjoint(minimum)
                    })
                    .cloned()
                    .collect()
            };
        for profile in declaration["profiles"].as_array().unwrap() {
            let minimum = scopes(profile, "minimum_scopes");
            assert!(
                scopes(profile, "requested_scopes").is_superset(&minimum),
                "{directory}: a minimum scope is not requested"
            );
            let operations = operations_of(profile["read_only"] == true);
            assert!(!operations.is_empty(), "{directory}");
            let uncovered = |minimum: &BTreeSet<String>| uncovered(&operations, minimum);
            assert_eq!(uncovered(&minimum), Vec::<String>::new(), "{directory}");
            for scope in &minimum {
                if KEPT_FOR_REACH
                    .iter()
                    .any(|(kept, name, _)| *kept == directory && name == scope)
                {
                    continue;
                }
                let mut fewer = minimum.clone();
                fewer.remove(scope);
                assert!(
                    !uncovered(&fewer).is_empty(),
                    "{directory}: {scope} covers no operation the rest of minimum_scopes does not"
                );
            }
            for (kept, name, _) in KEPT_FOR_REACH {
                assert!(
                    kept != directory || minimum.contains(name),
                    "{directory}: KEPT_FOR_REACH names {name}, which it does not require"
                );
            }
            checked.push(directory.clone());
        }
    }
    assert_eq!(
        checked,
        [
            "google-calendar",
            "google-calendar",
            "google-drive",
            "google-drive",
            "google-gmail",
            "google-gmail",
            "google-slides"
        ]
    );
}

/// A GitLab instance states the host its users know, as
/// `docs/design/connector-vocabulary.md` writes it, and the API base the bundle
/// needs is derived from it: the derived bootstrap is that of a hand-written
/// configuration whose `api_base` is the host followed by `/api/v4`. A
/// relative-root install keeps its path; an endpoint that is already the API
/// base is refused rather than doubled.
#[test]
fn a_gitlab_endpoint_is_the_host_and_the_api_base_is_derived() {
    let declaration = &declarations()["gitlab"];
    let hand = |endpoint: &str| {
        let mut config = json!({
            "format": "connectors-catalog-local/2",
            "instance": "gitlab.work",
            "provider": "gitlab",
            "bundle_directory": "generated/bundles",
            "api_base": format!("{endpoint}/api/v4"),
            "auth": declaration["profiles"][0],
            "operations_file": "providers/gitlab/operations.json",
        });
        // A relative root is a request prefix in front of the bundle's `/api/v4`.
        let root = &endpoint[origin(endpoint).len()..];
        if !root.is_empty() {
            config["request_prefix"] = json!(root);
        }
        legacy(&config)
    };
    let default = bootstrap(derived(&["--connector", "gitlab", "--instance", "work"]));
    assert_eq!(default, hand("https://gitlab.com"));
    for endpoint in ["https://gitlab.example.com", "https://example.com/gitlab"] {
        let actual = bootstrap(derived(&[
            "--connector",
            "gitlab",
            "--instance",
            "work",
            "--endpoint",
            endpoint,
        ]));
        assert_eq!(actual, hand(endpoint), "{endpoint}");
        assert_eq!(
            actual["provider_authority"],
            format!("{endpoint}/api/v4/"),
            "{endpoint}"
        );
    }
    for endpoint in [
        "https://gitlab.example.com/api/v4",
        "https://gitlab.example.com/api/v4/",
    ] {
        let output = derived(&[
            "--connector",
            "gitlab",
            "--instance",
            "work",
            "--endpoint",
            endpoint,
        ]);
        assert_eq!(output.status.code(), Some(1), "{endpoint}");
    }
}

/// An instance may state its own endpoint; it replaces the default and enters
/// the revision as `api_base` does.
#[test]
fn a_stated_endpoint_replaces_the_default() {
    let mut reference = read_json(&root().join("tests/connector_declarations/gmail.json"));
    let default = bootstrap(derived(&[
        "--connector",
        "gmail",
        "--instance",
        "private",
        "--profile",
        "google.oauth",
    ]));
    reference["api_base"] = json!("https://gmail.example.com");
    let expected = legacy(&reference);
    let actual = bootstrap(derived(&[
        "--connector",
        "gmail",
        "--instance",
        "private",
        "--profile",
        "google.oauth",
        "--endpoint",
        "https://gmail.example.com",
    ]));
    assert_eq!(actual, expected);
    assert_ne!(
        actual["configuration_revision"],
        default["configuration_revision"]
    );
}

/// What a derived configuration refuses: an unknown or undeclared connector
/// (OpenRouter), a connector without a default endpoint and no stated one, a
/// connector of several profiles without one named, a profile it does not
/// declare, an instance name that is not an id, and both forms at once.
#[test]
fn a_derived_configuration_refuses_what_it_cannot_derive() {
    let gmail = written(read_json(
        &root().join("tests/connector_declarations/gmail.json"),
    ));
    let local = gmail.path.to_str().unwrap();
    let endpoint = "https://your-domain.atlassian.net/rest/api/3";
    let zendesk = "https://your-subdomain.zendesk.com";
    // 1 is a refusal by the provider, 2 a usage error before it loads anything.
    for (code, args) in [
        (1, vec!["--connector", "outlook", "--instance", "private"]),
        (
            1,
            vec!["--connector", "openrouter", "--instance", "private"],
        ),
        (
            1,
            vec![
                "--connector",
                "slack",
                "--instance",
                "private",
                "--profile",
                "slack.user",
            ],
        ),
        (
            1,
            vec!["--connector", "google-gmail", "--instance", "private"],
        ),
        (1, vec!["--connector", "jira", "--instance", "work"]),
        (
            1,
            vec![
                "--connector",
                "zendesk",
                "--instance",
                "work",
                "--endpoint",
                zendesk,
            ],
        ),
        (
            1,
            vec![
                "--connector",
                "gmail",
                "--instance",
                "private",
                "--profile",
                "gitlab.pat",
            ],
        ),
        (1, vec!["--connector", "gmail", "--instance", "a/b"]),
        (1, vec!["--connector", "gmail", "--instance", ""]),
        (2, vec!["--connector", "gmail"]),
        (2, vec!["--instance", "private"]),
        (
            2,
            vec![
                "--connector",
                "gmail",
                "--instance",
                "private",
                "--local-config",
                local,
            ],
        ),
    ] {
        let output = derived(&args);
        assert_eq!(output.status.code(), Some(code), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?} printed");
    }
    let jira = bootstrap(derived(&[
        "--connector",
        "jira",
        "--instance",
        "work",
        "--endpoint",
        endpoint,
    ]));
    assert_eq!(jira["instance"], "jira.work");
    let zendesk = bootstrap(derived(&[
        "--connector",
        "zendesk",
        "--instance",
        "work",
        "--endpoint",
        zendesk,
        "--profile",
        "zendesk.basic",
    ]));
    assert_eq!(zendesk["profiles"][0]["id"], "zendesk.basic");
}

/// Acceptance (story:slack-connector-declaration): given only the connector
/// name `slack` and an instance name, the provider derives the bot instance the
/// Slack guide documents, with the guide's bot selection and profile, and so
/// its revision; the user selection's profile is not the connector's.
#[test]
fn slack_derived_from_its_declaration_is_the_documented_bot_instance() {
    let (_, mut reference) = documented()
        .into_iter()
        .find(|(page, config)| {
            page == "catalog-slack.md" && config["auth"]["profile"] == "slack.bot"
        })
        .expect("the Slack guide documents the bot instance");
    assert_eq!(
        reference["operations_file"],
        "providers/slack/operations.json"
    );
    reference["instance"] = json!("slack.work");
    let expected = legacy(&reference);
    let actual = bootstrap(derived(&["--connector", "slack", "--instance", "work"]));
    assert_eq!(
        actual["configuration_revision"],
        expected["configuration_revision"]
    );
    assert_eq!(actual, expected);
    assert_eq!(actual["instance"], "slack.work");
    let user = derived(&[
        "--connector",
        "slack",
        "--instance",
        "work",
        "--profile",
        "slack.user",
    ]);
    assert_eq!(user.status.code(), Some(1));
}

/// Every configuration the guides document keeps the revision it computed
/// before declarations existed (measured on `06339356e3`). A regenerated
/// bundle or selection moves these legitimately, in the same change: the three
/// Jira pins moved when `issue.create` and `issue.comment.add` required their
/// new `id` present after the write, and again with the two `confluence-v1`
/// pins when the Jira and Confluence v1 selections gained their `feed` (and
/// Jira `projects.list` and `project.get`). The eighteen pins of Confluence,
/// Google Calendar, Drive and Gmail, HubSpot, Jira, Slack and Zendesk moved
/// when their selections declared a `walk` (story:catalog-operation-declares-its-walk),
/// and again when the bundles were rebuilt to record each parameter's `enum` and
/// `format: date-time` (story:catalog-validates-enum-and-date-time). The Grafana
/// guide's pin is new with its selection (story:catalog-grafana-reads); it moved
/// the index of every later page and no revision. It moved when the selection
/// and the bundle gained the three data-source proxy reads and each proxy
/// `uid` its pattern (story:catalog-grafana-datasource-proxy-reads).
/// The Jira revisions moved again when `issue.edit` gained `body_null_clears`
/// (story:required-body-key-refuses-null).
/// The HubSpot guide's pin (`f22df2fc…` to `8968cae8…`) moved when the bundle
/// gained the two owner reads from the Owners document, as cited amendments
/// bound to the CRM Objects digest, and the selection gained `owners.list`
/// (with its walk) and `owner.get` (story:hubspot-owners-and-line-items).
/// The Gmail, Calendar and Drive guides each gained a read-only configuration:
/// three new pins that moved the index of every later page and no revision
/// (story:google-read-only-profiles-and-company-domain).
/// The Calendar, Drive, Gmail and Slides revisions moved when the Discovery
/// projector became `discovery-openapi/2` (story:drive-file-content-read).
/// The Basecamp guide's pin is new with its selection (story:catalog-basecamp);
/// it moved the index of every later page and no revision.
#[test]
fn documented_native_configurations_keep_their_revisions() {
    let pinned: BTreeMap<String, &str> = [
        (
            "00 catalog-basecamp.md",
            "25293c693e1accbc446b11a41c103744fdad1c39fd3d008457ce99c30b2d2797",
        ),
        (
            "01 catalog-confluence.md",
            "f241ef1b2fbe206c2059021881e3158f6d4f8c0b14d55458b601481ebb07b508",
        ),
        (
            "02 catalog-confluence.md",
            "cf2b9b88cde553a08d9655804d782d95f71b410053200beb825d06aacb82cd16",
        ),
        (
            "03 catalog-confluence.md",
            "fd76431c0eee380736f69fb23bd2ce11c9e61a9e060c57d472de100e93ba7eb1",
        ),
        (
            "04 catalog-confluence.md",
            "85991fe70f2ecb22220c8e59153aad2f3a7bc6dafc1b3e2870dafb4cba02455b",
        ),
        (
            "05 catalog-confluence.md",
            "f241ef1b2fbe206c2059021881e3158f6d4f8c0b14d55458b601481ebb07b508",
        ),
        (
            "06 catalog-google-calendar.md",
            "21f28863253c2e15af454dfb8c47030a73ab7ce2b6169f0c1d06849b9753e681",
        ),
        (
            "07 catalog-google-calendar.md",
            "c5f570fdbdb699c897682491a05a2d529d20b72ec793d08b7679cf21bb3c1b65",
        ),
        (
            "08 catalog-google-calendar.md",
            "fd44078e33997b4f12b5ba087d9744362682363da103ff37c867ba06f1de801a",
        ),
        (
            "09 catalog-google-drive.md",
            "e75bbfd16d0fc1e1e25dfa2f1aea46b3710d2ac0ae2353b3544be88098234e01",
        ),
        (
            "10 catalog-google-drive.md",
            "9d0a6ec64bb117a05d74a2caf06e18ac1d9864559450e88d9b156392db8c583c",
        ),
        (
            "11 catalog-google-drive.md",
            "cd83db10d3e1a08f44eb577a541004f454d247b595a7f5b0c13aab2da6cdcf0f",
        ),
        (
            "12 catalog-google-gmail.md",
            "540402e036de353590995a034b2dcb1ae491a37379bfa4f56532e74c765780c1",
        ),
        (
            "13 catalog-google-gmail.md",
            "e30ab8646cd261edbfe24ceb49433f0a2d6afe584457c366c2bfe842bb770c7b",
        ),
        (
            "14 catalog-google-slides.md",
            "6ded40ea130bebef82c4c795fdd8c5440c1890d032c9a0572c3f01e667549d33",
        ),
        (
            "15 catalog-grafana.md",
            "cd3a3ef9f85e05a990f6d8754e7aa2c5a190b71f0f629bec840b0d713b6dc948",
        ),
        (
            "16 catalog-hubspot.md",
            "8968cae86a853651f69e6b4eee869e6b7a68264089ca85fd6cb105ca2c7b945d",
        ),
        (
            "17 catalog-jira.md",
            "0cfc0e55dd791de912e49d4a22a79b4d06a353c2ba19282737a9ca2e3c1171bf",
        ),
        (
            "18 catalog-jira.md",
            "35926b20b288459ddcd4ffa3883b8a2ee6d182d768fe7445759baa0ccec3753d",
        ),
        (
            "19 catalog-jira.md",
            "b53c9bb9c3529ef712146895a73232e9f358d2a8cc684665991a11fd546cc039",
        ),
        (
            "20 catalog-openrouter.md",
            "7426f67e0bae66733ee6e8c6ecc6f602d85a3cb79a43d8058f54f90224a8a731",
        ),
        (
            "21 catalog-openrouter.md",
            "6472c864700269c252a6040a05397e3370fccd7bce155098ce25916b16ffa7bc",
        ),
        (
            "22 catalog-opsgenie.md",
            "2429e78e18775efb337892511a61756edf2e2027ef3f76dda50c3b850e95ed1e",
        ),
        (
            "23 catalog-runpod.md",
            "d07cd9583fdd53e294e6c535ecdc665b8fff85191d55d0cf875bdca5ae0138bc",
        ),
        (
            "24 catalog-slack.md",
            "e7220d48c9c68cfbb78e363d2a6ea5d4ce3e2595c4baad8f86492b02b010f79b",
        ),
        (
            "25 catalog-slack.md",
            "abe8a377d7f6cc4e17df36e888e4df0c4a55cfad753b4eb1b1039ac2509a1009",
        ),
        (
            "26 catalog-zendesk.md",
            "d14dae6d913cda517f992e5890521b8d481be0900acbff4807476d3a42387a50",
        ),
        (
            "27 catalog-zendesk.md",
            "181d0a89b6bb91e32d9666157c053594794e9841946c42598e7bffd9e9bcca2a",
        ),
        (
            "28 local-catalog-provider.md",
            "e4b752f75723b2de2c968185cf72fd3701158f8b991c16cb197c47695f5965bd",
        ),
    ]
    .into_iter()
    .map(|(key, revision)| (key.to_owned(), revision))
    .collect();
    let mut actual = BTreeMap::new();
    // A website guide's configuration is newer than declarations: it has no earlier revision.
    for (index, (page, mut config)) in documented()
        .into_iter()
        .filter(|(page, _)| !page.starts_with("website/"))
        .enumerate()
    {
        // The trust roots are instance material, digested by their bytes.
        if let Some(object) = config.as_object_mut() {
            object.remove("ca_file");
        }
        let revision = legacy(&config)["configuration_revision"]
            .as_str()
            .unwrap()
            .to_owned();
        actual.insert(format!("{index:02} {page}"), revision);
    }
    let actual: BTreeMap<String, &str> = actual
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str()))
        .collect();
    assert_eq!(
        actual,
        pinned,
        "{}",
        serde_json::to_string_pretty(&Value::Object(
            actual
                .iter()
                .map(|(k, v)| (k.clone(), json!(v)))
                .collect::<Map<_, _>>()
        ))
        .unwrap()
    );
}

/// story:migrate-writes-the-short-instance-form: `--instance-id` derives the
/// instance under the id it names instead of `<connector>.<instance>`, so a
/// migrated instance keeps its old id. The derivation is exactly the
/// hand-written configuration with that `instance`: the same bootstrap and
/// revision. An id that is not one is refused, and `--instance-id` without a
/// connector is a usage error.
#[test]
fn an_instance_id_derives_the_instance_under_that_id() {
    let mut reference = read_json(&root().join("tests/connector_declarations/gmail.json"));
    reference["instance"] = json!("mail-local");
    let expected = legacy(&reference);
    let actual = bootstrap(derived(&[
        "--connector",
        "gmail",
        "--instance",
        "mail",
        "--profile",
        "google.oauth",
        "--instance-id",
        "mail-local",
    ]));
    assert_eq!(actual["instance"], "mail-local");
    assert_eq!(
        actual["configuration_revision"],
        expected["configuration_revision"]
    );
    assert_eq!(actual, expected);
    let unnamed = bootstrap(derived(&[
        "--connector",
        "gmail",
        "--instance",
        "mail",
        "--profile",
        "google.oauth",
    ]));
    assert_eq!(unnamed["instance"], "gmail.mail");
    assert_ne!(
        unnamed["configuration_revision"],
        actual["configuration_revision"]
    );
    for (code, args) in [
        (
            1,
            vec![
                "--connector",
                "gmail",
                "--instance",
                "mail",
                "--instance-id",
                "a/b",
            ],
        ),
        (
            1,
            vec![
                "--connector",
                "gmail",
                "--instance",
                "mail",
                "--instance-id",
                "",
            ],
        ),
        (2, vec!["--instance-id", "mail-local"]),
    ] {
        let output = derived(&args);
        assert_eq!(output.status.code(), Some(code), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?} printed");
    }
}

/// `--print-revision-inputs` prints, per key of the configuration revision's
/// digest input, the digest of that key's value: a derived instance and the
/// hand-written configuration it equals print the same, and one that requests
/// other scopes differs in `auth` alone, which `setup migrate` names.
#[test]
fn revision_inputs_name_the_key_in_which_two_configurations_differ() {
    let inputs = |output: Output| -> BTreeMap<String, String> {
        serde_json::from_value(bootstrap(output)).unwrap()
    };
    let mut reference = read_json(&root().join("tests/connector_declarations/gmail.json"));
    reference["instance"] = json!("mail-local");
    let printed = |config: &Value| {
        let file = written(config.clone());
        inputs(run(&[
            "--local-config",
            file.path.to_str().unwrap(),
            "--print-revision-inputs",
        ]))
    };
    let derived = inputs(run(&[
        "--connector",
        "gmail",
        "--instance",
        "mail",
        "--profile",
        "google.oauth",
        "--instance-id",
        "mail-local",
        "--print-revision-inputs",
    ]));
    let keys: BTreeSet<&str> = derived.keys().map(String::as_str).collect();
    for key in [
        "format",
        "instance",
        "provider",
        "bundle_sha256",
        "source_sha256",
        "api_base",
        "auth",
        "operations",
    ] {
        assert!(keys.contains(key), "{key}: {derived:?}");
    }
    assert!(
        derived
            .values()
            .all(|digest| digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit())),
        "{derived:?}"
    );
    assert_eq!(printed(&reference), derived);
    let mut narrower = reference.clone();
    narrower["auth"]["requested_scopes"] = json!(["openid", GMAIL_READONLY]);
    narrower["auth"]["minimum_scopes"] = json!([GMAIL_READONLY]);
    let differing: Vec<String> = {
        let narrower = printed(&narrower);
        keys.iter()
            .filter(|key| narrower.get(**key) != derived.get(**key))
            .map(|key| (*key).to_owned())
            .collect()
    };
    assert_eq!(differing, ["auth"]);
    let mut elsewhere = reference.clone();
    elsewhere["api_base"] = json!("https://mail.example.test");
    let elsewhere = printed(&elsewhere);
    let differing: Vec<&str> = keys
        .iter()
        .copied()
        .filter(|key| elsewhere.get(*key) != derived.get(*key))
        .collect();
    assert_eq!(differing, ["api_base"]);
}

/// The Google connectors that carry a write-capable profile, with the scope
/// each one's read-only profile requests.
const GOOGLE_READ_ONLY: [(&str, &str, &str); 3] = [
    (
        "gmail",
        "google-gmail",
        "https://www.googleapis.com/auth/gmail.readonly",
    ),
    (
        "calendar",
        "google-calendar",
        "https://www.googleapis.com/auth/calendar.readonly",
    ),
    (
        "drive",
        "google-drive",
        "https://www.googleapis.com/auth/drive.readonly",
    ),
];
const READ_ONLY_PROFILE: &str = "google.oauth.readonly";
const WRITE_PROFILE: &str = "google.oauth";

/// The profiles and the operations, each with its profile, that the provider
/// child of the derived instance `args` name describes over private protocol
/// two, which describes writes.
fn described(args: &[&str]) -> (Vec<String>, Vec<(String, String)>) {
    use connectors_host::local::{
        config::{Adapter, Executable, Restart, Startup},
        runtime::{Bootstrap, Child, PrivateProtocol},
    };
    use sha2::{Digest as _, Sha256};
    let printed: Bootstrap = serde_json::from_value(bootstrap(derived(args))).unwrap();
    let binary = PathBuf::from(BINARY).canonicalize().unwrap();
    let child = Child::spawn(&Adapter {
        credential_revision: None,
        private_protocol: Some(PrivateProtocol::V2),
        permissions: Default::default(),
        instance_id: printed.instance.clone(),
        adapter_id: "catalog".into(),
        configuration_revision: printed.configuration_revision.clone(),
        protocol: "v1alpha1".into(),
        startup: Startup::OnDemand,
        restart: Restart::Never,
        executable: Executable {
            sha256: hex::encode(Sha256::digest(fs::read(&binary).unwrap())),
            path: binary,
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        },
    })
    .unwrap();
    let descriptor = child.bootstrap().descriptor().unwrap();
    let profiles = child
        .bootstrap()
        .profiles
        .iter()
        .map(|profile| profile.id.clone())
        .collect();
    (
        profiles,
        descriptor
            .operations
            .iter()
            .map(|operation| (operation.id.clone(), operation.profile.clone()))
            .collect(),
    )
}

/// Gmail, Calendar and Drive each declare a read-only profile that requests
/// only the read scope, and it is the profile an instance that names none
/// takes (story:google-read-only-profiles-and-company-domain). The write-capable
/// profile stays, under its own name, for the instance that names it.
#[test]
fn each_google_connector_declares_a_read_only_default_profile() {
    let declarations = declarations();
    for (connector, directory, read) in GOOGLE_READ_ONLY {
        let declaration = &declarations[directory];
        assert_eq!(
            declaration["default_profile"], READ_ONLY_PROFILE,
            "{connector}"
        );
        let profiles = declaration["profiles"].as_array().unwrap();
        let ids: Vec<&str> = profiles
            .iter()
            .map(|p| p["profile"].as_str().unwrap())
            .collect();
        assert_eq!(ids, [READ_ONLY_PROFILE, WRITE_PROFILE], "{connector}");
        let readonly = &profiles[0];
        assert_eq!(readonly["read_only"], true, "{connector}");
        assert_eq!(
            scopes(readonly, "minimum_scopes"),
            BTreeSet::from([read.to_owned()]),
            "{connector}"
        );
        assert_eq!(
            scopes(readonly, "requested_scopes"),
            BTreeSet::from(["openid".to_owned(), read.to_owned()]),
            "{connector}"
        );
        assert!(profiles[1].get("read_only").is_none(), "{connector}");
        assert!(
            scopes(&profiles[1], "minimum_scopes").len() > 1,
            "{connector}: the write-capable profile keeps its write scope"
        );
    }
}

/// An instance of Gmail, Calendar or Drive that names no profile is on the
/// read-only one and its provider describes no write operation, even over
/// private protocol two; the same connector on `google.oauth` describes its
/// writes, and an instance on the read-only profile named outright is the same
/// instance as the default one.
#[test]
fn a_read_only_google_instance_offers_no_write_operation() {
    for (connector, _, read) in GOOGLE_READ_ONLY {
        let by_default = bootstrap(derived(&["--connector", connector, "--instance", "work"]));
        let named = bootstrap(derived(&[
            "--connector",
            connector,
            "--instance",
            "work",
            "--profile",
            READ_ONLY_PROFILE,
        ]));
        assert_eq!(by_default, named, "{connector}");
        assert_eq!(by_default["profiles"][0]["id"], READ_ONLY_PROFILE);
        assert_eq!(
            by_default["profiles"][0]["minimum_scopes"],
            json!([read]),
            "{connector}"
        );

        let (profiles, operations) = described(&["--connector", connector, "--instance", "work"]);
        assert_eq!(profiles, [READ_ONLY_PROFILE], "{connector}");
        assert!(!operations.is_empty(), "{connector}");
        let writes: Vec<&(String, String)> = operations
            .iter()
            .filter(|(_, profile)| profile == "mutation")
            .collect();
        assert!(writes.is_empty(), "{connector} offers {writes:?}");

        let (profiles, operations) = described(&[
            "--connector",
            connector,
            "--instance",
            "work",
            "--profile",
            WRITE_PROFILE,
        ]);
        assert_eq!(profiles, [WRITE_PROFILE], "{connector}");
        assert!(
            operations.iter().any(|(_, profile)| profile == "mutation"),
            "{connector}: the write-capable profile lost its writes"
        );
    }
}

/// Adversary (story:google-read-only-profiles-and-company-domain): a
/// connection saved before the read-only default is under `google.oauth`, the
/// one profile Gmail, Calendar and Drive declared. The host keeps a connection
/// bound to the profile record it was saved under and never rebinds it to
/// another profile (`registry::revalidation::upgradable`), so an instance
/// naming no profile, which declares only `google.oauth.readonly`, cannot
/// revalidate it. What an upgrading user must do is therefore exact and is
/// said in the CHANGELOG and in each guide: name `profile = "google.oauth"`
/// (the instance keeps its connection), or revoke the connection and connect
/// again under the default.
#[test]
fn adversary_a_connection_saved_before_the_read_only_default_is_kept_or_replaced_as_documented() {
    let declares = |args: &[&str], profile: &str| {
        let printed: connectors_host::local::runtime::Bootstrap =
            serde_json::from_value(bootstrap(derived(args))).unwrap();
        printed.binding(profile).is_ok()
    };
    for (connector, _, _) in GOOGLE_READ_ONLY {
        let default = ["--connector", connector, "--instance", "private"];
        assert!(
            declares(&default, READ_ONLY_PROFILE) && !declares(&default, WRITE_PROFILE),
            "{connector}: the default instance does not declare the read-only profile alone"
        );
        let mut named = default.to_vec();
        named.extend(["--profile", WRITE_PROFILE]);
        assert!(
            declares(&named, WRITE_PROFILE) && !declares(&named, READ_ONLY_PROFILE),
            "{connector}: naming {WRITE_PROFILE} does not keep a connection saved under it"
        );
    }

    let flat = |text: String| text.split_whitespace().collect::<Vec<_>>().join(" ");
    // The entry lives in the section, `## Unreleased` or `## <version> — <date>`,
    // that introduced the read-only default: the oldest one naming its profile.
    // Sections are newest first, so a later release mentioning it is skipped.
    let changelog = fs::read_to_string(root_path("../../CHANGELOG.md")).unwrap();
    let introduced = changelog
        .split("\n## ")
        .skip(1)
        .filter(|section| {
            section.starts_with("Unreleased") || section.starts_with(|c: char| c.is_ascii_digit())
        })
        .filter(|section| section.contains(READ_ONLY_PROFILE))
        .last()
        .map(|section| flat(section.to_owned()))
        .expect("no CHANGELOG section introduces the read-only default");
    let mut pages = vec![("CHANGELOG".to_owned(), introduced)];
    for page in [
        "docs/catalog-google-gmail.md",
        "docs/catalog-google-calendar.md",
        "docs/catalog-google-drive.md",
        "docs/catalog-google-oauth.md",
        "website/docs/connectors/gmail.md",
        "website/docs/connectors/calendar.md",
        "website/docs/connectors/drive.md",
    ] {
        pages.push((
            page.to_owned(),
            flat(fs::read_to_string(root_path(&format!("../../{page}"))).unwrap()),
        ));
    }
    for (name, text) in &pages {
        for term in [
            "profile = \"google.oauth\"",
            "connections revoke",
            "never rebound",
        ] {
            assert!(
                text.contains(term),
                "{name} does not tell an upgrading user: {term}"
            );
        }
        assert!(
            !text.contains("naming no profile changes configuration revision"),
            "{name} still says an unrenamed connection is pending until it revalidates"
        );
    }
}

/// Adversary (story:google-read-only-profiles-and-company-domain): the
/// host starts a derived instance with `--allowed-domain`, and nothing else
/// in the suite runs that flag. On Gmail it reaches the instance (another
/// credential revision); on a profile whose `hd` is never read, Zendesk's
/// `oauth2_client_credentials` and Jira's `basic`, it is refused at load,
/// never ignored.
#[test]
fn adversary_the_allowed_domain_flag_reaches_a_derived_instance_or_is_refused() {
    let gmail = |extra: &[&str]| {
        let mut args = vec!["--connector", "gmail", "--instance", "private"];
        args.extend_from_slice(extra);
        bootstrap(derived(&args))
    };
    let open = gmail(&[]);
    let admitted = gmail(&["--allowed-domain", "corp.test"]);
    assert_ne!(
        open["credential_revision"], admitted["credential_revision"],
        "--allowed-domain did not reach the derived instance"
    );
    for args in [
        &[
            "--connector",
            "zendesk",
            "--instance",
            "work",
            "--endpoint",
            "https://example.zendesk.com",
            "--profile",
            "zendesk.oauth",
        ][..],
        &[
            "--connector",
            "jira",
            "--instance",
            "work",
            "--endpoint",
            "https://jira.example.org",
        ][..],
    ] {
        bootstrap(derived(args));
        let mut with_domain = args.to_vec();
        with_domain.extend_from_slice(&["--allowed-domain", "corp.test"]);
        assert!(
            !derived(&with_domain).status.success(),
            "{args:?}: a domain list the profile never enforces loaded"
        );
    }
}

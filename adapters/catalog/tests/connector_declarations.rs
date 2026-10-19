//! Each catalog connector ships its declaration (`providers/<name>/connector.json`),
//! and the provider binary builds an instance's native configuration from a
//! connector name, an instance name and an optional endpoint. A derived
//! configuration digests exactly as an equal hand-written one, and every
//! hand-written configuration keeps the revision it had before declarations.
#![cfg(unix)]

mod hand_written;

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
/// The catalog providers that ship no declaration and stay pinned.
const UNDECLARED: [&str; 2] = ["openrouter", "slack"];

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
/// (`hand_written`), in the order the guide showed them.
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
    let actual = bootstrap(derived(&["--connector", "gmail", "--instance", "private"]));
    assert_eq!(
        actual["configuration_revision"],
        expected["configuration_revision"]
    );
    assert_eq!(actual, expected);
    assert_eq!(actual["instance"], "gmail.private");
}

/// Slack holds two selections run as two instances, and OpenRouter one selection
/// run as a management and an inference instance joined by custody, so neither
/// ships a declaration and both stay pinned; every other catalog provider
/// directory ships exactly one,
/// naming its own directory, its own selection and a distinct connector.
#[test]
fn every_catalog_provider_but_slack_and_openrouter_ships_one_declaration() {
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
            "calendar",
            "confluence",
            "confluence-v1",
            "drive",
            "gitlab",
            "gmail",
            "hubspot",
            "jira",
            "opsgenie",
            "runpod",
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
        if config.get("request_prefix").is_some() {
            continue;
        }
        let provider = config["provider"].as_str().unwrap();
        let Some(declaration) = declarations.get(provider) else {
            assert!(
                UNDECLARED.contains(&provider),
                "{page}: {provider} has no declaration"
            );
            continue;
        };
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
    let gmail = &declarations["google-gmail"]["profiles"][0];
    for key in ["minimum_scopes", "requested_scopes"] {
        let named = scopes(gmail, key);
        assert!(named.contains(GMAIL_READONLY) && named.contains(GMAIL_COMPOSE));
    }
}

/// For every declared connector and profile, the derived configuration's
/// bootstrap equals that of a hand-written configuration with the
/// declaration's fields: file paths are not digested.
#[test]
fn every_declared_profile_derives_what_its_hand_written_configuration_derives() {
    let documented = documented();
    let mut checked = Vec::new();
    for (directory, declaration) in declarations() {
        let connector = declaration["connector_id"].as_str().unwrap();
        for profile in declaration["profiles"].as_array().unwrap() {
            let id = profile["profile"].as_str().unwrap();
            let endpoint = match declaration["default_endpoint"].as_str() {
                Some(endpoint) => endpoint.to_owned(),
                None => documented
                    .iter()
                    .map(|(_, config)| config)
                    .find(|config| {
                        config["provider"] == directory.as_str()
                            && config["auth"]["profile"] == id
                            && config.get("request_prefix").is_none()
                    })
                    .and_then(|config| config["api_base"].as_str())
                    .unwrap_or_else(|| panic!("{directory}: no documented endpoint"))
                    .to_owned(),
            };
            let hand = json!({
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
    // Twelve connectors, Zendesk with two profiles: a lane that finds no
    // declaration checks nothing and must not pass.
    assert_eq!(checked.len(), 13, "{checked:?}");
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
        let mut operations = BTreeSet::new();
        operation_ids(
            &read_json(
                &root()
                    .join("providers")
                    .join(&directory)
                    .join(declaration["selection"].as_str().unwrap()),
            ),
            &mut operations,
        );
        assert!(!operations.is_empty(), "{directory}");
        let uncovered = |minimum: &BTreeSet<String>| -> Vec<String> {
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
            "google-drive",
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
    let default = bootstrap(derived(&["--connector", "gmail", "--instance", "private"]));
    reference["api_base"] = json!("https://gmail.example.com");
    let expected = legacy(&reference);
    let actual = bootstrap(derived(&[
        "--connector",
        "gmail",
        "--instance",
        "private",
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
/// (Slack), a connector without a default endpoint and no stated one, a
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
        (1, vec!["--connector", "slack", "--instance", "private"]),
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

/// Every configuration the guides document keeps the revision it computed
/// before declarations existed (measured on `06339356e3`). A regenerated
/// bundle or selection moves these legitimately, in the same change.
#[test]
fn documented_native_configurations_keep_their_revisions() {
    let pinned: BTreeMap<String, &str> = [
        (
            "00 catalog-confluence.md",
            "e29d7d0838aa6bdc2a86e6758db521f874db73616dc1caf21c02ceb52df94b24",
        ),
        (
            "01 catalog-confluence.md",
            "86a936572eaa670f31dfb80ea503d568c7e3732384bbe46207047796a5599e28",
        ),
        (
            "02 catalog-confluence.md",
            "3b2d03659dc0d1603da69c65eb6c727cb025c71a2a9d414a7ea7be8f71c84bd0",
        ),
        (
            "03 catalog-confluence.md",
            "cc01bd621a306d99e9c32c429dd2c8b20593583142db6128c1cfcfe7a7e71901",
        ),
        (
            "04 catalog-confluence.md",
            "e29d7d0838aa6bdc2a86e6758db521f874db73616dc1caf21c02ceb52df94b24",
        ),
        (
            "05 catalog-google-calendar.md",
            "ac90599c7a1255715ebc16c8c0ec17cc4925985676880d2e1ba189988005c459",
        ),
        (
            "06 catalog-google-calendar.md",
            "36d1eb4505ba0be70273d91e8fdda3368474123c5773be6d32a7191e4d281428",
        ),
        (
            "07 catalog-google-drive.md",
            "c9ff1845cca4a082e724aab12c226409ede2d62ba2d7897d63feeba0c79f49ce",
        ),
        (
            "08 catalog-google-drive.md",
            "d673ca0a9d79874a1dd83e6ef9f28be01ca9937a6f94429314e45ac704d0ef6a",
        ),
        (
            "09 catalog-google-gmail.md",
            "c5167a1d610da35a2225dc895719e2b72709bde2b044f8a9db62d75ef5461dd1",
        ),
        (
            "10 catalog-google-slides.md",
            "59351e7bbcf7431d2e398170f4bc1fdb3b46ef4447440aaa105bb4cfc02cb58b",
        ),
        (
            "11 catalog-hubspot.md",
            "ab1aae40aff91fa177426e3dddd3ab78bcc1d7ac1a60a960d0ca6d951106e17e",
        ),
        (
            "12 catalog-jira.md",
            "717881d0e9f55a316cf22029aa4cfff2803ceecd29e9b31d3edfa55e839f508c",
        ),
        (
            "13 catalog-jira.md",
            "5ac4de0707cd592352a5bee8f88e33b1714c947d55a2e885f74ff14fd551fc59",
        ),
        (
            "14 catalog-jira.md",
            "0ebc9ab5cbde840d682a13cc6d4deda057f0d26e4d628d042b11f35618abb248",
        ),
        (
            "15 catalog-openrouter.md",
            "edc1e2986e61cdce1a2d1dcc59571f483d8c8b4c30f5fa7ac7d694d2a21b7108",
        ),
        (
            "16 catalog-openrouter.md",
            "88cd4b2b3d83233bed4808105ed0bea68c0e423ee357efab57989e18226e906e",
        ),
        (
            "17 catalog-opsgenie.md",
            "e3bd0d88779169436ac8cede723c60c38ad18ba5f993aee6c240effd84baad4b",
        ),
        (
            "18 catalog-runpod.md",
            "4fd4b8d636a48f045765eda600ae2a14897fa7ebaab9c3fdd5c34c5daac3deea",
        ),
        (
            "19 catalog-slack.md",
            "23e877ed8e54ad38b319d0ffd0c482203719b26ecd094e7e65bc8d976a6af6ca",
        ),
        (
            "20 catalog-slack.md",
            "1dc8109ddc54339d4cbcabbedc13c427fa4a180d2a74213bc4505a7e44fa8404",
        ),
        (
            "21 catalog-zendesk.md",
            "60a56bb42996cb06670413c3924c75285c8fe3f09b4df0cfcedf3a923a54eae5",
        ),
        (
            "22 catalog-zendesk.md",
            "56c177a9a9bec9ff41b937ea8c74222ee2eed53d9eb2fe576fc9938bc7d4fbf0",
        ),
        (
            "23 local-catalog-provider.md",
            "46e04a9d96d0c783771ca07baf47995e159e6d2f3a8dfa461b541f45dd35d9cb",
        ),
    ]
    .into_iter()
    .map(|(key, revision)| (key.to_owned(), revision))
    .collect();
    let mut actual = BTreeMap::new();
    for (index, (page, mut config)) in documented().into_iter().enumerate() {
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

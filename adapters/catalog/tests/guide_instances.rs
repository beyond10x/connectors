//! The catalog guides configure a connector and a named instance: every
//! `[connectors.<connector>.instances.<instance>]` table a guide shows is
//! derived by the provider binary from its connector, instance name and the
//! `endpoint` and `profile` it states, and no guide asks the reader to copy a
//! pin (a configuration revision, an executable digest, a bundle directory) or
//! to select an instance with `--adapter`. Each Google API guide configures one
//! instance per account, whose consent requests every scope its connector
//! declares, and the Google OAuth guide says that an app used only by its
//! developer can be published without verification.
#![cfg(unix)]
mod hand_written;

use serde_json::Value;
use std::{fs, path::Path, process::Command};

const BINARY: &str = env!("CARGO_BIN_EXE_connectors-catalog-provider");

/// Each guide and the connectors whose instances it configures.
const GUIDES: [(&str, &[&str]); 10] = [
    ("catalog-confluence.md", &["confluence", "confluence-v1"]),
    ("catalog-hubspot.md", &["hubspot"]),
    ("catalog-jira.md", &["jira"]),
    ("catalog-opsgenie.md", &["opsgenie"]),
    ("catalog-runpod.md", &["runpod"]),
    ("catalog-zendesk.md", &["zendesk"]),
    ("catalog-google-calendar.md", &["calendar"]),
    ("catalog-google-drive.md", &["drive"]),
    ("catalog-google-gmail.md", &["gmail"]),
    ("catalog-google-slides.md", &["slides"]),
];

/// The Google API guides and the provider directory of their connector.
const GOOGLE: [(&str, &str); 4] = [
    ("catalog-google-calendar.md", "google-calendar"),
    ("catalog-google-drive.md", "google-drive"),
    ("catalog-google-gmail.md", "google-gmail"),
    ("catalog-google-slides.md", "google-slides"),
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn guide(name: &str) -> String {
    fs::read_to_string(root().join("../../docs").join(name)).unwrap()
}

/// One instance table of a guide: connector, instance name, and the
/// `endpoint` and `profile` it states.
#[derive(Debug)]
struct Table {
    connector: String,
    instance: String,
    endpoint: Option<String>,
    profile: Option<String>,
}

/// Every instance table in the guide's `toml` blocks, and every connector
/// table those blocks enable.
fn tables(text: &str) -> (Vec<Table>, Vec<String>) {
    let mut instances = Vec::new();
    let mut enabled = Vec::new();
    for block in text
        .split("```toml\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
    {
        let mut current: Option<Table> = None;
        let mut connector_table: Option<String> = None;
        for line in block.lines().map(str::trim) {
            if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                instances.extend(current.take());
                connector_table = None;
                let Some(rest) = header.strip_prefix("connectors.") else {
                    continue;
                };
                match rest.split_once(".instances.") {
                    Some((connector, instance)) => {
                        current = Some(Table {
                            connector: connector.to_owned(),
                            instance: instance.to_owned(),
                            endpoint: None,
                            profile: None,
                        });
                    }
                    None => connector_table = Some(rest.to_owned()),
                }
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim().trim_matches('"').to_owned());
            if let Some(table) = current.as_mut() {
                match key {
                    "endpoint" => table.endpoint = Some(value),
                    "profile" => table.profile = Some(value),
                    "description" => {}
                    other => panic!("instance table key `{other}` is a pin, not derived"),
                }
            } else if let Some(connector) = &connector_table
                && key == "enabled"
                && value == "true"
            {
                enabled.push(connector.clone());
            }
        }
        instances.extend(current.take());
    }
    (instances, enabled)
}

/// The bootstrap the provider derives for `table`.
fn derived(table: &Table) -> Value {
    let mut args = vec![
        "--connector",
        table.connector.as_str(),
        "--instance",
        table.instance.as_str(),
    ];
    if let Some(endpoint) = &table.endpoint {
        args.extend(["--endpoint", endpoint.as_str()]);
    }
    if let Some(profile) = &table.profile {
        args.extend(["--profile", profile.as_str()]);
    }
    args.push("--print-local-bootstrap");
    let output = Command::new(BINARY).args(&args).output().unwrap();
    assert!(
        output.status.success(),
        "{table:?}: exit {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Every guide configures at least one instance of each of its connectors,
/// enables that connector, and every instance it shows derives, with the
/// instance id `<connector>.<instance>`.
#[test]
fn each_guide_configures_instances_the_provider_derives() {
    let mut checked = 0;
    for (name, connectors) in GUIDES {
        let (instances, enabled) = tables(&guide(name));
        for connector in connectors {
            assert!(
                instances.iter().any(|t| t.connector == *connector),
                "{name}: no `[connectors.{connector}.instances.<name>]` table"
            );
            assert!(
                enabled.iter().any(|c| c == connector),
                "{name}: no `[connectors.{connector}]` table with `enabled = true`"
            );
        }
        for table in &instances {
            let bootstrap = derived(table);
            assert_eq!(
                bootstrap["instance"],
                format!("{}.{}", table.connector, table.instance),
                "{name}"
            );
            checked += 1;
        }
    }
    assert!(checked >= GUIDES.len(), "{checked} instance tables checked");
}

/// No guide asks the reader to copy a pin or select an instance by alias.
#[test]
fn no_guide_shows_a_pin_or_the_alias_selectors() {
    let mut found = Vec::new();
    for (name, _) in GUIDES.iter().chain(&[("catalog-google-oauth.md", &[][..])]) {
        for (number, line) in guide(name).lines().enumerate() {
            for pin in [
                "configuration_revision",
                "sha256",
                "bundle_directory",
                "--adapter",
                "connections repair",
            ] {
                if line.contains(pin) {
                    found.push(format!("{name}:{}: {pin}", number + 1));
                }
            }
        }
    }
    assert!(found.is_empty(), "{found:#?}");
}

/// Each Google API guide configures exactly one instance, and names every
/// scope its connector's profile requests: one consent covers the reads and
/// the writes. No guide routes the writes through a second instance.
#[test]
fn each_google_guide_configures_one_instance_with_every_declared_scope() {
    for (name, directory) in GOOGLE {
        let text = guide(name);
        let (instances, _) = tables(&text);
        assert_eq!(instances.len(), 1, "{name}: {instances:?}");
        let declaration: Value = serde_json::from_slice(
            &fs::read(
                root()
                    .join("providers")
                    .join(directory)
                    .join("connector.json"),
            )
            .unwrap(),
        )
        .unwrap();
        for scope in declaration["profiles"][0]["requested_scopes"]
            .as_array()
            .unwrap()
        {
            let scope = scope.as_str().unwrap();
            assert!(text.contains(&format!("`{scope}`")), "{name}: {scope}");
        }
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        for stale in ["separate instance", "write instance", "read instance"] {
            assert!(!flat.contains(stale), "{name} still says \"{stale}\"");
        }
    }
}

/// A gateway instance is pinned, not derived: the Jira and Confluence guides
/// show only the values it states over the site form. Each such fragment is
/// the gateway configuration the gateway tests run (`hand_written`,
/// `gateway_prefix.rs`, `gateway_prefix_adversary.rs`), key for key.
#[test]
fn each_gateway_fragment_is_a_configuration_the_gateway_tests_run() {
    let mut checked = 0;
    for page in ["catalog-jira.md", "catalog-confluence.md"] {
        let configurations: Vec<Value> = hand_written::blocks(page)
            .iter()
            .map(|body| serde_json::from_str(body).unwrap())
            .collect();
        for body in guide(page)
            .split("```json\n")
            .skip(1)
            .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
            .filter(|body| body.contains("\"request_prefix\""))
        {
            let fragment: Value = serde_json::from_str(body).unwrap();
            let fragment = fragment.as_object().unwrap();
            let tested = configurations.iter().any(|configuration| {
                fragment
                    .iter()
                    .all(|(key, value)| &configuration[key] == value)
            });
            assert!(tested, "{page}: no gateway configuration states {body}");
            checked += 1;
        }
    }
    assert_eq!(checked, 4, "two gateway fragments per guide");
}

/// The section of `text` that starts with the heading `heading` and ends
/// before the next heading of the same or a higher level.
fn section<'a>(text: &'a str, heading: &str) -> &'a str {
    let start = text
        .find(&format!("\n{heading}\n"))
        .unwrap_or_else(|| panic!("no section {heading}"));
    let level = heading.split(' ').next().unwrap();
    let body = &text[start + 1..];
    let end = body[heading.len()..]
        .match_indices("\n#")
        .map(|(at, _)| at + heading.len())
        .find(|at| {
            let next = body[at + 1..].split(' ').next().unwrap();
            next.len() <= level.len()
        })
        .unwrap_or(body.len());
    &body[..end]
}

/// The OAuth guide's Testing-mode expiry: publishing an app used only by its
/// developer needs no verification and lifts the 7-day expiry, with Google's
/// support page as the source. Its Reconnect section replaces Repair.
#[test]
fn the_oauth_guide_says_a_personal_app_publishes_without_verification() {
    let text = guide("catalog-google-oauth.md");
    let expiry = section(&text, "## Testing-mode expiry");
    let flat = expiry.split_whitespace().collect::<Vec<_>>().join(" ");
    for term in [
        "support.google.com/cloud/answer/13464323",
        "*In production*",
        "without verification",
        "seven days",
        "personal use",
    ] {
        assert!(
            flat.contains(term),
            "Testing-mode expiry does not say {term}"
        );
    }
    assert!(
        !flat.contains("subject to Google's verification"),
        "Testing-mode expiry still says publishing is subject to verification"
    );
    let reconnect = section(&text, "## Reconnect");
    for term in ["connections reconnect --connector", "--instance", "`stale`"] {
        assert!(reconnect.contains(term), "Reconnect does not say {term}");
    }
}

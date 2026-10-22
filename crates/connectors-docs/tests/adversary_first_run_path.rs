//! Adversary cases for `story:docs-first-run-path`: what the first-run pages promise, checked
//! against the code, the selections and the planning store they describe.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Acceptance: "`set-up-the-local-cli.md` gives the default paths literally". The code default
/// is `$HOME/.local/state` joined with `connectors` (`crates/connectors-host/src/local/config.rs`,
/// `Paths::resolve`), so the page names `~/.local/state/connectors`, not a description of it.
#[test]
fn set_up_the_local_cli_gives_the_default_paths_literally() {
    let page = read("website/docs/guides/set-up-the-local-cli.md");
    let missing: Vec<_> = [
        "$XDG_CONFIG_HOME/connectors/config.toml",
        "$XDG_STATE_HOME/connectors",
        "~/.config/connectors/config.toml",
        "~/.local/state/connectors",
    ]
    .into_iter()
    .filter(|path| !page.contains(path))
    .collect();
    assert!(
        missing.is_empty(),
        "set-up-the-local-cli.md does not give {missing:?}"
    );
}

/// README.md's support table now gives a count and says "each one's page lists its
/// operations". Every operation a catalog selection ships is named on the onboarding page the
/// count links.
#[test]
fn each_catalog_connector_page_lists_every_selected_operation() {
    let pages = [
        ("confluence", "confluence"),
        ("confluence-v1", "confluence"),
        ("gitlab", "gitlab"),
        ("google-calendar", "calendar"),
        ("google-drive", "drive"),
        ("google-gmail", "gmail"),
        ("google-slides", "slides"),
        ("hubspot", "hubspot"),
        ("jira", "jira"),
        ("openrouter", "openrouter"),
        ("opsgenie", "opsgenie"),
        ("runpod", "runpod"),
        ("slack", "slack"),
        ("zendesk", "zendesk"),
    ];
    let mut missing = Vec::new();
    for (provider, page) in pages {
        let text = read(&format!("website/docs/connectors/{page}.md"));
        let directory = root().join("adapters/catalog/providers").join(provider);
        let mut selections: Vec<_> = std::fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with("operations.json"))
            })
            .collect();
        selections.sort();
        assert!(!selections.is_empty(), "{provider}: no selection");
        for selection in selections {
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&selection).unwrap()).unwrap();
            for operation in json["operations"].as_array().unwrap() {
                let id = operation["id"].as_str().unwrap();
                if !text.contains(&format!("`{id}`")) {
                    missing.push(format!("{page}.md lacks `{id}` ({provider})"));
                }
            }
        }
    }
    assert!(missing.is_empty(), "{missing:#?}");
}

/// The story's status lines name custody per platform: on Linux the Secret Service or
/// `mounted_secret`, on macOS the login keychain, on Windows the Credential Manager.
/// `the_readme_names_every_custody_binding` holds only `mounted_secret` and its link, so a README
/// that drops the other three, or says "a single binding per platform", still passes it.
#[test]
fn the_readme_names_the_custody_binding_of_each_platform() {
    let readme = read("README.md");
    let flat = readme.split_whitespace().collect::<Vec<_>>().join(" ");
    let missing: Vec<_> = [
        "Secret Service",
        "`mounted_secret`",
        "login keychain",
        "Credential Manager",
    ]
    .into_iter()
    .filter(|binding| !flat.contains(binding))
    .collect();
    assert!(missing.is_empty(), "README.md does not name {missing:?}");
    let lower = flat.to_ascii_lowercase();
    for phrase in ["binding per platform", "binding for each platform"] {
        assert!(!lower.contains(phrase), "README.md says {phrase:?}");
    }
}

/// README.md says which decisions MCP waits on. A decision-blocker it names as open is open in
/// the planning store.
#[test]
fn the_readme_names_no_cleared_decision_blocker_as_open() {
    let readme = read("README.md");
    let mut cleared = Vec::new();
    for (at, _) in readme.match_indices("`decision-blocker:") {
        let rest = &readme[at + "`decision-blocker:".len()..];
        let id = &rest[..rest.find('`').unwrap()];
        let record = read(&format!(".engineering/planning/decision-blocker/{id}.md"));
        let status = record
            .lines()
            .find_map(|line| line.strip_prefix("status: "))
            .unwrap_or("missing");
        if status != "open" {
            cleared.push(format!("decision-blocker:{id} is {status}"));
        }
    }
    assert!(cleared.is_empty(), "README.md names as open: {cleared:?}");
}

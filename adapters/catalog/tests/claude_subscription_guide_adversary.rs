//! Adversary cases for story:claude-subscription-connector: the Claude subscription guide,
//! `website/docs/connectors/claude-subscription.md`, read as the instructions a person follows.
//! It shows one native configuration per profile and one pinned instance table; a person who
//! writes either configuration and pins it as the guide shows must be able to connect it.
#![cfg(unix)]

use serde_json::Value;
use std::{fs, path::Path};

fn guide() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../website/docs/connectors/claude-subscription.md"),
    )
    .unwrap()
}

/// The bodies of the guide's fenced blocks of `language`.
fn blocks(text: &str, language: &str) -> Vec<String> {
    text.split(&format!("```{language}\n"))
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body.to_owned()))
        .collect()
}

/// Every native configuration the guide shows, by its `auth.profile`.
fn documented_profiles(text: &str) -> Vec<String> {
    blocks(text, "json")
        .iter()
        .filter(|body| body.contains("\"format\": \"connectors-catalog-local/2\""))
        .map(|body| {
            let config: Value = serde_json::from_str(body).unwrap();
            config["auth"]["profile"].as_str().unwrap().to_owned()
        })
        .collect()
}

/// Every profile a `permissions` table of the guide's TOML blocks permits.
fn permitted_profiles(text: &str) -> Vec<String> {
    let mut permitted = Vec::new();
    for body in blocks(text, "toml") {
        for line in body.lines() {
            let Some(list) = line.trim().strip_prefix("profiles = ") else {
                continue;
            };
            let list: Vec<String> = serde_json::from_str(list).unwrap();
            permitted.extend(list);
        }
    }
    permitted
}

/// The guide shows the pasted-code configuration (`auth.profile` `claude.subscription-pasted`)
/// and tells the reader to connect it with `--profile claude.subscription-pasted`, but the only
/// instance table it shows permits `profiles = ["claude.subscription"]`. The owner refuses a
/// capture whose profile the instance does not permit as `Forbidden`
/// (`crates/connectors-host/src/local/owner.rs`, `capture_profile`), so a reader who follows
/// the pasted-code path as written cannot connect.
#[test]
fn every_profile_the_guide_configures_is_permitted_by_the_instance_it_pins() {
    let text = guide();
    let configured = documented_profiles(&text);
    assert_eq!(
        configured,
        ["claude.subscription", "claude.subscription-pasted"],
        "the guide's native configurations"
    );
    assert!(
        text.contains("--profile claude.subscription-pasted"),
        "the guide no longer connects the pasted-code profile"
    );
    let permitted = permitted_profiles(&text);
    for profile in &configured {
        assert!(
            permitted.contains(profile),
            "the guide configures `{profile}` and tells the reader to connect it, but its \
             instance table permits only {permitted:?}"
        );
    }
}

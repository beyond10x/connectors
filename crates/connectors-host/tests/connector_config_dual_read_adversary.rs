//! Adversary cases for story:connector-config-dual-read (`connectors-local/5`,
//! `[connectors.<connector>.instances.<instance>]` beside `[adapters.<alias>]`).
//! Unix only: the fixtures use Unix absolute executable paths and mode bits.
#![cfg(unix)]
use connectors_host::local::{
    Failure,
    config::{Config, Paths, Refusal},
    filesystem,
};
use std::fs;

const SLACK: &str = "[connectors.slack]\nenabled=true\n[connectors.slack.instances.bot]\nalias='slack-bot'\ninstance_id='slack-bot'\nadapter_id='slack-credentials'\nconfiguration_revision='cfg-7'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[connectors.slack.instances.bot.executable]\npath='/opt/connectors/bin/slack-credentials'\nsha256='2222222222222222222222222222222222222222222222222222222222222222'\n[connectors.slack.instances.bot.permissions]\nprofiles=['slack.bot']\noperations=['auth.test']\n";

fn read(format: &str, body: &str) -> Result<Config, Refusal> {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths::resolve(
        Some(&root.path().join("config/config.toml")),
        Some(&root.path().join("state")),
    )
    .unwrap();
    Config::initialize(&paths).unwrap();
    fs::write(
        &paths.config,
        format!("format='{format}'\nowner_uid={}\n{body}", filesystem::uid()),
    )
    .unwrap();
    Config::read(&paths.config)
}

/// The variants of `connectors.cli.ConfigurationFormat` in the CLI's ESS domain.
fn declared_formats() -> Vec<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ess/domains/cli.yaml");
    let domain: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let format = domain["types"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|t| t["name"].as_str() == Some("connectors.cli.ConfigurationFormat"))
        .expect("connectors.cli.ConfigurationFormat is declared");
    format["variants"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}

// contracts/cli/v1alpha1/semantics.md: "`configuration_format` is the closed
// enum `ConfigurationFormat`". A `/5` entry lacking `private_protocol` is named
// with `format = "connectors-local/5"`, which the CLI copies into
// `configuration_format`; the generated contract refuses a value outside the
// enum, so the operator gets a contract failure instead of the named refusal.
#[test]
fn a_format_five_mismatch_names_a_declared_configuration_format() {
    let bare = SLACK.replace("private_protocol='connectors-private/1'\n", "");
    let Err(Refusal::PrivateProtocolMismatch { format, .. }) = read("connectors-local/5", &bare)
    else {
        panic!("a /5 instance without private_protocol is a mismatch");
    };
    let declared = declared_formats();
    assert!(
        declared.contains(&format),
        "the host names configuration_format {format:?}, the CLI contract declares {declared:?}"
    );
}

// A format gates its tables: an older binary refuses `[connectors]` in any
// file (`ConfigInput` denies unknown fields), so a `/4` file carrying it must
// not load here either, empty or not.
#[test]
fn an_empty_connectors_table_is_refused_outside_format_five() {
    for format in ["connectors-local/2", "connectors-local/4"] {
        for body in ["[connectors]\n", "connectors={}\n"] {
            assert_eq!(
                read(format, body).err(),
                Some(Refusal::Failure(Failure::InvalidConfiguration)),
                "{format} admitted {body:?}"
            );
        }
    }
}

// `Config.adapters` is the loaded view every caller reads and the repository's
// tests edit before `toml::to_string` (owner.rs, approval_issuance/tests.rs).
// For an instance alias, `Written` drops the edited entry and writes the
// connector table read from the file, so the edit is lost without an error.
#[test]
fn an_edited_instance_entry_survives_writing_the_configuration() {
    let mut config = read("connectors-local/5", SLACK).unwrap();
    config
        .adapters
        .get_mut("slack-bot")
        .unwrap()
        .configuration_revision = "cfg-8".into();
    let text = toml::to_string(&config).unwrap();
    let reloaded: Config = toml::from_str(&text).unwrap();
    assert_eq!(
        reloaded.adapters["slack-bot"].configuration_revision, "cfg-8",
        "{text}"
    );
}

#[test]
fn a_removed_instance_entry_stays_removed_after_writing_the_configuration() {
    let mut config = read("connectors-local/5", SLACK).unwrap();
    config.adapters.remove("slack-bot");
    let text = toml::to_string(&config).unwrap();
    let reloaded: Config = toml::from_str(&text).unwrap();
    assert!(!reloaded.adapters.contains_key("slack-bot"), "{text}");
}

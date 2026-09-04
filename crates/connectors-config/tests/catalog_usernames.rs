//! **The non-secret user half of a `basic` credential, in the configuration that has to hold it.**
//!
//! An Atlassian API token travels as `base64(email:token)`. The token is the secret and lives in
//! the credential store; the email is not, and until 2026-09-04 there was nowhere in a personal
//! placement to write it down. The consequence was not a missing feature but a lie: `auth status`
//! reported the credential as stored, and every call refused with `not_granted: no stored
//! credential satisfies this operation's declared mechanisms`. Both statements were true and
//! neither was the problem.
//!
//! These go through [`PersonalConfig::read`] rather than `toml::from_str` on purpose — it is the
//! one entry point the daemon uses, so a configuration these tests accept is one the daemon starts
//! on, including the custody checks and the validation pass.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;

use connectors_config::PersonalConfig;

/// The minimum a `[[catalog]]` entry needs, so a test about one field is about that field.
fn written(block: &str) -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().expect("a temporary directory");
    let path = root.path().join("connectors.toml");
    fs::write(
        &path,
        format!(
            r#"
[owner]
tenant_id = "tenant-local"
agent_id = "agent-dev"
agent_revision = 1
authority_snapshot_id = "authority-1"
authority_snapshot_sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

[[catalog]]
provider = "jira"
grant_ref = "grant:jira:local"
initiation = "platform"
credential = "jira.api_token"
operator_approved = true
{block}
"#
        ),
    )
    .expect("the fixture writes");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("owner-only");
    (root, path)
}

#[test]
fn a_basic_credential_carries_its_user_half_beside_its_endpoints() {
    // The shape a Basic-auth connector needs and had nowhere to put: the token is in the store, the
    // account name is here, and the two are joined at assembly. Keyed by the credential's flat name
    // because that is the key `ConfigField::Username` asks under — not by the configuration field's
    // local name, which is `email`.
    let (_root, path) = written(
        r#"
[catalog.endpoints]
cloud_id = "11111111-2222-3333-4444-555555555555"

[catalog.usernames]
"jira.api_token" = "ops@example.test"
"#,
    );
    let config = PersonalConfig::read(&path).expect("the daemon's own reader accepts it");
    let entry = &config.catalog[0];
    assert_eq!(
        entry.endpoints.get("cloud_id").map(String::as_str),
        Some("11111111-2222-3333-4444-555555555555")
    );
    assert_eq!(
        entry.usernames.get("jira.api_token").map(String::as_str),
        Some("ops@example.test")
    );
}

#[test]
fn an_entry_with_no_user_half_still_reads_and_reports_none() {
    // Most of the catalogue is bearer-authenticated and states no user half at all, so the section
    // is absent rather than empty in every configuration written before this existed. A new field
    // that made those fail to load would take every other provider down with it.
    let (_root, path) = written("");
    let config = PersonalConfig::read(&path).expect("an entry with no user half still reads");
    assert!(config.catalog[0].usernames.is_empty());
}

#[test]
fn a_user_half_is_refused_when_it_is_empty_or_could_not_travel() {
    // Refused at load rather than at assembly, so a configuration that cannot work is one the
    // daemon never starts on. An empty value in particular would otherwise compose `base64(":token")`
    // and send it, and the vendor's answer would be about the credential rather than about this.
    for block in [
        "[catalog.usernames]\n\"jira.api_token\" = \"\"\n",
        "[catalog.usernames]\n\"jira.api_token\" = \"ops@example.test\\u0000\"\n",
        "[catalog.usernames]\n\"\" = \"ops@example.test\"\n",
    ] {
        let (_root, path) = written(block);
        assert!(
            PersonalConfig::read(&path).is_err(),
            "a user half that cannot travel is refused at load: {block}"
        );
    }
}

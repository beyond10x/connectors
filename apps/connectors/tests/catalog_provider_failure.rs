//! story:catalog-provider-failure-is-named: `connections connect
//! <connector>/<instance> --endpoint <url>` derives the instance by starting
//! the catalog provider installed beside the CLI. A provider that cannot run
//! refused nothing, so the command names it, its path and why, instead of
//! blaming the endpoint; an endpoint a running provider refuses is still the
//! argument's refusal. `setup check` lists the provider as a prerequisite,
//! and a configuration `setup init` creates, or `setup migrate` moves, takes
//! `--endpoint`.
//!
//! The production CLI is copied beside a stand-in catalog provider
//! (`support/standin.rs`), which is this binary, so it runs without the
//! standard test harness.
#[cfg(unix)]
#[path = "support/standin.rs"]
mod standin;

#[cfg(unix)]
fn main() {
    tests::main();
}

#[cfg(not(unix))]
fn main() {}

#[cfg(unix)]
mod tests {
    use super::standin::*;
    use serde_json::{Value, json};
    use std::fs;

    const HOST: &str = "https://git.example.org";

    pub fn main() {
        super::standin::main(&[
            (
                "no_provider_beside_the_cli_is_named_not_blamed_on_the_endpoint",
                no_provider_beside_the_cli_is_named_not_blamed_on_the_endpoint,
            ),
            (
                "a_provider_that_exits_is_killed_or_prints_nothing_is_named",
                a_provider_that_exits_is_killed_or_prints_nothing_is_named,
            ),
            (
                "an_endpoint_the_running_provider_refuses_is_still_the_argument",
                an_endpoint_the_running_provider_refuses_is_still_the_argument,
            ),
            (
                "setup_check_lists_the_installed_provider",
                setup_check_lists_the_installed_provider,
            ),
            (
                "a_provider_past_its_deadline_is_named_by_setup_check",
                a_provider_past_its_deadline_is_named_by_setup_check,
            ),
            (
                "setup_init_creates_a_file_connect_endpoint_writes_into",
                setup_init_creates_a_file_connect_endpoint_writes_into,
            ),
            (
                "migrate_moves_a_file_without_legacy_entries_to_format_five",
                migrate_moves_a_file_without_legacy_entries_to_format_five,
            ),
        ]);
    }

    fn connect(installation: &Installation, endpoint: &str) -> std::process::Output {
        installation.run(&[
            "connections",
            "connect",
            "gitlab/probe",
            "--profile",
            "gitlab.pat",
            "--endpoint",
            endpoint,
        ])
    }

    /// The refusal of a provider that does not run: `unavailable` at
    /// `installed_provider` (exit 1), naming the provider's path and `reason`,
    /// with the configuration unchanged.
    fn assert_named(installation: &Installation, before: &str, endpoint: &str, reason: &str) {
        let (code, data) = refusal(&connect(installation, endpoint));
        assert_ne!(data["code"], "invalid_input", "{endpoint}: {data}");
        assert_eq!(
            data,
            json!({"kind":"operational","code":"unavailable","stage":"installed_provider",
                "next_action":"check_configuration",
                "installed_provider_path":installation.provider().to_str().unwrap(),
                "installed_provider_failure":reason}),
            "{endpoint}"
        );
        assert_eq!(code, Some(1), "{endpoint}: {data}");
        assert_eq!(
            installation.text(),
            before,
            "{endpoint}: the file is unchanged"
        );
    }

    /// The prerequisite named `installed_provider` of a `setup check`.
    fn prerequisite(installation: &Installation) -> Value {
        let checked = success(&installation.run(&["setup", "check"]));
        let found = checked["prerequisites"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "installed_provider")
            .cloned();
        found.unwrap_or_else(|| panic!("no installed_provider prerequisite: {checked}"))
    }

    /// `connectors` alone in its directory: every endpoint form, the one the
    /// provider would refuse included, is refused naming the missing provider.
    pub fn no_provider_beside_the_cli_is_named_not_blamed_on_the_endpoint() {
        let installation = Installation::new();
        let before = installation.configure("");
        fs::remove_file(installation.provider()).unwrap();
        for endpoint in [
            HOST.to_owned(),
            format!("{HOST}/"),
            format!("{HOST}/api/v4"),
        ] {
            assert_named(&installation, &before, &endpoint, "missing");
        }
    }

    /// A provider that exits non-zero, ends on a signal, or prints nothing
    /// for every argument, `--version` included, is named with that reason.
    pub fn a_provider_that_exits_is_killed_or_prints_nothing_is_named() {
        for (how, reason) in [
            (Misbehaviour::Exit, "exited"),
            (Misbehaviour::Kill, "killed"),
            (Misbehaviour::Silent, "unreadable_output"),
        ] {
            let installation = Installation::new();
            let before = installation.configure("");
            installation.misbehave(how);
            assert_named(&installation, &before, HOST, reason);
        }
    }

    /// GitLab refuses an endpoint that already ends in `/api/v4`: the provider
    /// ran and refused it, so the refusal is the argument's, nothing written.
    pub fn an_endpoint_the_running_provider_refuses_is_still_the_argument() {
        let installation = Installation::new();
        let before = installation.configure("");
        let (code, data) = refusal(&connect(&installation, &format!("{HOST}/api/v4")));
        assert_eq!(
            data,
            json!({"kind":"usage","code":"invalid_input","stage":"arguments",
                "next_action":"none","field":"endpoint"})
        );
        assert_eq!(code, Some(2));
        assert_eq!(installation.text(), before);
    }

    /// `installed_provider` is `ready` while the provider prints its version,
    /// `failed` with the reason otherwise.
    pub fn setup_check_lists_the_installed_provider() {
        let installation = Installation::new();
        installation.configure("");
        assert_eq!(
            prerequisite(&installation),
            json!({"name":"installed_provider","state":"ready","next_action":"none"})
        );
        installation.misbehave(Misbehaviour::Exit);
        assert_eq!(
            prerequisite(&installation),
            json!({"name":"installed_provider","state":"failed",
                "next_action":"check_configuration","installed_provider_failure":"exited"})
        );
        fs::remove_file(installation.provider()).unwrap();
        assert_eq!(
            prerequisite(&installation),
            json!({"name":"installed_provider","state":"failed",
                "next_action":"check_configuration","installed_provider_failure":"missing"})
        );
    }

    /// A provider that outlives the ten-second deadline is `deadline`, and the
    /// check is not held past it. One start only, so it stays near ten seconds.
    pub fn a_provider_past_its_deadline_is_named_by_setup_check() {
        let installation = Installation::new();
        installation.configure("");
        installation.misbehave(Misbehaviour::Hang);
        let started = std::time::Instant::now();
        assert_eq!(
            prerequisite(&installation),
            json!({"name":"installed_provider","state":"failed",
                "next_action":"check_configuration","installed_provider_failure":"deadline"})
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(15),
            "{:?}",
            started.elapsed()
        );
    }

    /// `setup init` writes `connectors-local/5`, and `connect --endpoint` on
    /// that file writes the instance and goes on to the credential.
    pub fn setup_init_creates_a_file_connect_endpoint_writes_into() {
        let installation = Installation::new();
        success(&installation.run(&["setup", "init"]));
        let created = installation.text();
        assert!(
            created.contains("format = \"connectors-local/5\""),
            "{created}"
        );
        let (_, data) = refusal(&connect(&installation, HOST));
        assert_eq!(data["stage"], "protected_entry", "{data}");
        let text = installation.text();
        assert_eq!(
            parsed(&text)["connectors"]["gitlab"]["instances"]["probe"]["endpoint"],
            HOST.into(),
            "{text}"
        );
    }

    /// A `connectors-local/2` file without legacy entries is planned, then
    /// applied, as the same file at `connectors-local/5`, its comment kept
    /// and the original backed up; `connect --endpoint` then writes into it.
    pub fn migrate_moves_a_file_without_legacy_entries_to_format_five() {
        let installation = Installation::new();
        success(&installation.run(&["setup", "init"]));
        let uid = connectors_host::local::filesystem::uid();
        let old =
            format!("# kept\nformat = \"connectors-local/2\"\nowner_uid = {uid}\n\n[adapters]\n");
        fs::write(installation.config(), &old).unwrap();
        let planned = success(&installation.run(&["setup", "migrate", "--dry-run"]));
        let expected = old.replace("connectors-local/2", "connectors-local/5");
        assert_eq!(planned["disposition"], "planned", "{planned}");
        assert_eq!(planned["format_before"], "connectors-local/2", "{planned}");
        assert_eq!(planned["format_after"], "connectors-local/5", "{planned}");
        assert_eq!(planned["configuration"], expected.as_str(), "{planned}");
        assert_eq!(installation.text(), old, "a dry run writes nothing");
        let applied = success(&installation.run(&["setup", "migrate", "--apply"]));
        assert_eq!(applied["disposition"], "applied", "{applied}");
        assert_eq!(installation.text(), expected);
        let backup = applied["backup_path"].as_str().unwrap();
        assert_eq!(fs::read_to_string(backup).unwrap(), old);
        let again = success(&installation.run(&["setup", "migrate", "--apply"]));
        assert_eq!(again["disposition"], "unchanged", "{again}");
        let (_, data) = refusal(&connect(&installation, HOST));
        assert_eq!(data["stage"], "protected_entry", "{data}");
    }
}

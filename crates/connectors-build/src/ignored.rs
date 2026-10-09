//! Explicit, classified execution of the workspace's ignored libtest inventory.
use crate::Result;
use clap::{Args, ValueEnum};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::File,
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, PermissionsExt},
        process::{CommandExt, ExitStatusExt},
    },
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

#[derive(Args, Debug)]
pub struct Options {
    /// Families to execute. Merely having credentials never selects live tests.
    #[arg(long, value_enum, default_value = "disposable", value_delimiter = ',')]
    family: Vec<Family>,
    /// List the compiled inventory and prerequisites without running tests.
    #[arg(long)]
    inventory: bool,
    /// Missing prerequisites in any selected family make the command fail.
    #[arg(long, conflicts_with = "inventory")]
    required: bool,
    #[arg(long, default_value = ".local/ignored-suites.json")]
    report: PathBuf,
    /// A short, physical, owner-private fixture directory for long checkouts.
    #[arg(long)]
    tmpdir: Option<PathBuf>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    Disposable,
    Live,
    Timing,
    Deferred,
}
#[derive(Clone, Debug)]
struct Suite {
    package: String,
    target: String,
    executable: PathBuf,
}
#[derive(Clone, Debug)]
struct Entry {
    suite: Suite,
    name: String,
    family: Option<Family>,
    helper: bool,
    prerequisites: Vec<&'static str>,
}
impl Entry {
    fn id(&self) -> String {
        format!(
            "{}::{}::{}",
            self.suite.package, self.suite.target, self.name
        )
    }
}
#[derive(Debug, Serialize)]
struct Outcome {
    name: String,
    family: Option<Family>,
    selected: bool,
    status: String,
    reasons: Vec<String>,
}
#[derive(Debug, Default, Serialize)]
struct Report {
    inventory: usize,
    selected: usize,
    executed: usize,
    skipped: usize,
    failed: usize,
    missing: usize,
    unknown: usize,
    outcomes: Vec<Outcome>,
}
impl Report {
    fn accepted(&self, required: bool) -> bool {
        self.failed == 0
            && self.unknown == 0
            && (!required
                || (self.missing == 0 && self.selected > 0 && self.executed == self.selected))
    }
}
const CUSTODY: &str = "qualified GNOME Secret Service and /usr/bin/dbus-daemon";
const CLI: &str = "CONNECTORS_TEST_CLI";
const PROVIDER: &str = "CONNECTORS_TEST_CATALOG_PROVIDER";
const CHROME: &str = "google-chrome-stable";
const OLD_CLI: &str = "CONNECTORS_ADVERSARY_PRE_HANDSHAKE";
const PG: &str = "CONNECTORS_PG_SANDBOX";
const PG_CONTAINER: &str = "CONNECTORS_PG_CONTAINER";
const DOCKER: &str = "docker";
const K8S: &str = "CONNECTORS_K8S_SANDBOX";
const CA: &str = "CONNECTORS_K8S_CA";
const TOKEN: &str = "CONNECTORS_K8S_TOKEN";
const KUBECONFIG: &str = "CONNECTORS_K8S_KUBECONFIG";
const KUBECTL: &str = "/usr/bin/kubectl";
const SECCOMP: &str = "Linux x86_64 seccomp USER_NOTIF";
// Same qualified artifact as the custody implementation; a test below detects pin drift.
const GNOME_SHA256: &str = "b9a71f6b4c4bfaf1759a99036b7b3ab6cf98b2b479c0c2a24f02f5f2ca53958b";

/// Each entry names an exact package, target and test. New tests, even in an
/// existing module, are unknown until someone reviews their authority and needs.
fn classify(suite: &Suite, name: &str) -> Entry {
    use Family::*;
    let mut entry = Entry {
        suite: suite.clone(),
        name: name.into(),
        family: None,
        helper: false,
        prerequisites: vec![],
    };
    let classification = match (suite.package.as_str(), suite.target.as_str(), name) {
        ("connectors-host", "connectors_host", name) => match name {
            "local::mutations::tests::crash_child"
            | "local::approvals::tests::crash_child"
            | "local::audit::tests::crash_child"
            | "local::approval_policy::tests::policy_child"
            | "local::approval_keys::tests::key_crash_child"
            | "local::runtime::launch::tests::consumer_fixture"
            | "local::runtime::launch::tests::launch_runner_fixture"
            | "local::runtime::launch::tests::review_conformance::reviewing_consumer_fixture"
            | "local::runtime::launch::tests::review_conformance::reviewing_runner_fixture" => {
                entry.helper = true;
                return entry;
            }
            "local::clock::tests::independent_live_source" => Some((Live, vec![])),
            "local::registry::tests::read_invoke_metadata_time"
            | "local::registry::store_cost_tests::read_invoke_cost_by_store_size"
            | "local::registry::store_cost_tests::batch_cost_by_subject" => Some((Timing, vec![])),
            "local::oauth_adversary_tests::chrome_following_the_redirect_is_accepted"
            | "local::oauth_adversary_tests::chrome_opens_one_connection_per_navigation"
            | "local::oauth_adversary_pass2_tests::chrome_with_loopback_cookies_following_the_redirect_is_accepted" => {
                Some((Disposable, vec![CHROME]))
            }
            "local::keyring::custody::tests::disposable_secret_service_restart_and_failures"
            | "local::keyring::custody::tests::custody_neither_reads_nor_changes_the_default_alias"
            | "local::keyring::custody::tests::disposable_changed_keyring_format_is_refused_before_transfer"
            | "local::approval_keys::tests::durable_key_management_with_qualified_custody"
            | "local::approval_keys::tests::restart_failed_rotation_recovery_and_revocation_fences"
            | "local::approval_keys::tests::key_use_excludes_rotation_and_concurrent_init_has_one_winner"
            | "local::approval_keys::tests::wrong_recovery_material_never_publishes"
            | "local::approval_keys::tests::process_exits_keep_pre_and_post_publication_distinct"
            | "local::approval_keys::tests::unknown_storage_acknowledgement_requires_exact_reconfirmation"
            | "local::approval_keys::tests::purpose_isolation_for_identical_private_uuid_tuples"
            | "local::approval_keys::tests::uncertain_deletion_keeps_retirement_fence_and_exact_retry" => {
                Some((Disposable, vec![CUSTODY]))
            }
            "local::keyring::custody::tests::disposable_registry_publication_cli_and_retirement_restart"
            | "local::owner::approval_issuance::tests::production_cli_approval_issuance_and_restart"
            | "local::approval_keys::tests::production_cli_key_journey"
            | "local::runtime::launch::tests::disposable_consumer_launch_delivers_fd3_and_refuses_by_code" => {
                Some((Disposable, vec![CUSTODY, CLI]))
            }
            _ => None,
        },
        (
            "connectors-catalog-provider",
            "local_runtime",
            "cli_journey::guarded_merge::prepared_attempt_exit_fixture"
            | "cli_journey::guarded_merge::owner_replay::catalog_same_image_owner_fixture",
        ) => {
            entry.helper = true;
            return entry;
        }
        (
            "connectors-catalog-provider",
            "local_runtime",
            "cli_journey::lifecycle::catalog_cli_explicit_revalidation_after_real_expiry_without_reentry",
        ) => Some((Timing, vec![CUSTODY, CLI])),
        (
            "connectors-catalog-provider",
            "local_runtime",
            "cli_journey::lifecycle::catalog_cli_failed_repair_and_busy_stop_preserve_authority"
            | "cli_journey::guarded_merge::catalog_cli_guarded_merge_applied_refused_and_lost_response_restart"
            | "cli_journey::guarded_merge::catalog_cli_guarded_merge_revocation_finishes_admitted_audit"
            | "cli_journey::guarded_merge::catalog_cli_recovers_abandoned_preparation_only_with_trusted_time"
            | "cli_journey::guarded_merge::catalog_cli_background_recovery_waits_for_live_work_and_recovers_unkeyed_attempts"
            | "cli_journey::guarded_merge::catalog_cli_background_recovers_revoked_removed_target_without_disclosure",
        ) => Some((Disposable, vec![CUSTODY, CLI])),
        (
            "connectors-catalog-provider",
            "local_runtime",
            "cli_journey::guarded_merge::catalog_cli_failed_settlement_keeps_the_known_effect_and_recovers_conservatively"
            | "cli_journey::guarded_merge::catalog_adversary_revoked_disclosure_of_an_applied_merge_never_answers_not_attempted",
        ) => Some((Disposable, vec![CUSTODY, CLI, SECCOMP])),
        (
            "connectors-catalog-provider",
            "local_runtime",
            "cli_journey::gitlab_catalog_cli_reuses_custody_across_owner_and_keyring_restart"
            | "cli_journey::gitlab_catalog_cli_reads_the_merge_request_feed_through_a_saved_connection"
            | "cli_journey::adversary_u23_a_read_result_the_child_admitted_reaches_the_cli_through_the_owner"
            | "cli_journey::basic_catalog_cli_connects_by_file_and_stdin_and_refuses_by_code"
            | "cli_journey::basic_catalog_cli_refuses_a_credential_below_minimum_scopes"
            | "cli_journey::adversary_basic_cli_wrong_token_is_unauthorized_and_malformed_is_invalid_input"
            | "cli_journey::a_provider_refusal_reads_dispatch_and_a_host_refusal_reads_admission"
            | "cli_journey::oauth_connect_journey"
            | "cli_journey::oauth_repair_journey"
            | "oauth2_refresh::oauth_invalid_grant_reports_repair"
            | "oauth2_refresh::oauth_connect_refusal_does_not_say_repair"
            | "oauth2_refresh_adversary_pass2::adversary2_oauth_cached_bootstrap_round_trips_across_owner_restart",
        ) => Some((Disposable, vec![CUSTODY, CLI])),
        (
            "connectors-loki",
            "local_runtime",
            "cli_journey::loki_cli_connects_with_a_bearer_token_and_answers_each_read_on_the_saved_connection",
        ) => Some((Disposable, vec![CUSTODY, CLI])),
        (
            "connectors-grafana",
            "local_runtime",
            "cli_journey::grafana_cli_connects_with_a_service_account_token_and_lists_datasources_on_the_saved_connection",
        ) => Some((Disposable, vec![CUSTODY, CLI])),
        ("connectors-kubernetes", "local_runtime", name) => match name {
            "cli_journey::persistent_kubernetes_cli_owner_and_keyring_restart"
            | "cli_journey::kubernetes_cli_refuses_a_changed_cluster_identity_and_preserves_the_saved_credential"
            | "cli_journey::kubernetes_cli_reads_helm_release_history_and_redacted_values" => {
                Some((Disposable, vec![CUSTODY, CLI]))
            }
            "cli_journey::a_real_cluster_session_persists_across_cli_and_owner_restart" => {
                Some((Live, vec![CUSTODY, CLI, K8S, CA, TOKEN]))
            }
            "cli_journey::kubernetes_cli_reuses_each_admitted_read_after_restart"
            | "cli_journey::kubernetes_cli_provider_rbac_denial_is_not_empty_success"
            | "cli_journey::kubernetes_cli_continuation_preserves_scope_and_revision"
            | "cli_journey::kubernetes_cli_repair_revoke_and_stop_preserve_authority" => Some((
                Live,
                vec![CUSTODY, CLI, K8S, CA, TOKEN, KUBECONFIG, KUBECTL],
            )),
            _ => None,
        },
        (
            "connectors-sql",
            "local_runtime",
            "cli_journey::a_real_postgres_session_persists_across_cli_and_owner_restart"
            | "cli_journey::postgres_cli_preserves_join_group_utc_and_quoted_parameters"
            | "cli_journey::postgres_cli_distinguishes_empty_truncated_capacity_and_timeout"
            | "cli_journey::postgres_cli_cannot_escape_read_only_transaction"
            | "cli_journey::postgres_repair_revoke_and_busy_stop_preserve_authority",
        ) => Some((Live, vec![CUSTODY, CLI, PG, PG_CONTAINER, DOCKER])),
        (
            "connectors-sql",
            "local_runtime",
            "cli_journey::postgres_dropped_invocation_cancels_its_backend",
        ) => Some((Live, vec![PG, PG_CONTAINER, DOCKER])),
        (
            "connectors",
            "failed_connect",
            "a_connect_to_an_unreachable_provider_reports_the_owners_cause_at_dispatch",
        ) => Some((Disposable, vec![CUSTODY, PROVIDER])),
        (
            "connectors",
            "failed_connect",
            "a_revalidation_the_owner_definitely_fails_reports_dispatch_at_the_cli"
            | "a_revalidation_after_a_configuration_change_reaches_the_owner_without_a_cached_description"
            | "an_adapter_identity_mismatch_during_an_upgrade_names_a_new_connection"
            | "an_adapter_identity_mismatch_outside_an_upgrade_names_a_repair",
        ) => Some((Disposable, vec![CUSTODY])),
        (
            "connectors",
            "local_cli",
            "adversary2_a_real_pre_handshake_owner_is_refused_by_name_and_the_reverse_is_observed",
        ) => Some((Disposable, vec![OLD_CLI])),
        (
            "connectors-conformance",
            "configuration_refusal_adversary",
            "the_contract_refuses_configuration_coordinates_that_are_not_the_declared_ones",
        ) => Some((Deferred, vec![])),
        _ => None,
    };
    if let Some((family, prerequisites)) = classification {
        entry.family = Some(family);
        entry.prerequisites = prerequisites;
    }
    entry
}
fn discover(suite: &Suite) -> Result<Vec<Entry>> {
    let output = crate::run(Command::new(&suite.executable).args([
        "--ignored",
        "--list",
        "--format",
        "terse",
    ]))?;
    let mut entries = Vec::new();
    for line in std::str::from_utf8(&output.stdout)?
        .lines()
        .filter(|line| !line.is_empty())
    {
        let name = line
            .strip_suffix(": test")
            .ok_or_else(|| format!("unsupported inventory line in {}: {line}", suite.target))?;
        entries.push(classify(suite, name));
    }
    Ok(entries)
}
fn evaluate(
    entries: Vec<Entry>,
    options: &Options,
    mut missing: impl FnMut(&Entry) -> Vec<String>,
    mut execute: impl FnMut(&Entry) -> Result<bool>,
) -> Report {
    let mut report = Report {
        inventory: entries.len(),
        unknown: entries
            .iter()
            .filter(|e| e.family.is_none() && !e.helper)
            .count(),
        ..Report::default()
    };
    for entry in entries {
        let selected = entry
            .family
            .is_some_and(|family| options.family.contains(&family));
        let mut outcome = Outcome {
            name: entry.id(),
            family: entry.family,
            selected,
            status: String::new(),
            reasons: vec![],
        };
        if entry.helper {
            outcome.status = "helper".into();
            outcome
                .reasons
                .push("subprocess-only entry; its parent owns execution".into());
        } else if entry.family.is_none() {
            outcome.status = "unknown".into();
            outcome
                .reasons
                .push("inventory classification required".into());
        } else if !selected {
            outcome.status = "not-selected".into();
            outcome
                .reasons
                .push("family requires explicit selection".into());
        } else {
            report.selected += 1;
            outcome.reasons = missing(&entry);
            if !outcome.reasons.is_empty() {
                report.missing += 1;
                outcome.status = "missing-prerequisite".into();
            } else if report.unknown != 0 {
                outcome.status = "blocked".into();
                outcome
                    .reasons
                    .push("unknown inventory prevents all execution".into());
            } else if options.inventory {
                outcome.status = "inventory-only".into();
                outcome
                    .reasons
                    .push("inventory mode executes no tests".into());
            } else {
                match execute(&entry) {
                    Ok(passed) => {
                        report.executed += 1;
                        outcome.status = if passed { "passed" } else { "failed" }.into();
                        if !passed {
                            report.failed += 1;
                            outcome
                                .reasons
                                .push("test reported a failure; see log".into());
                        }
                    }
                    Err(error) => {
                        report.failed += 1;
                        outcome.status = "execution-error".into();
                        outcome.reasons.push(error.to_string());
                    }
                }
            }
        }
        if outcome.status != "passed" && outcome.status != "failed" {
            report.skipped += 1;
        }
        report.outcomes.push(outcome);
    }
    report
}

/// Own one fixture group. The unreaped leader reserves its identity until the
/// group has been signalled. No signal is ever sent after reaping begins.
struct FixtureGroup {
    child: Child,
    signalled: bool,
    finished: bool,
    leader_status: Option<ExitStatus>,
}
impl FixtureGroup {
    fn exited(&self) -> Result<bool> {
        // SAFETY: zero is a valid initial siginfo_t; waitid initializes its
        // fields before observation. WNOWAIT deliberately retains the leader.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                self.child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY: successful waitid above initialized this siginfo_t.
        Ok(unsafe { info.si_pid() } != 0)
    }
    fn finish(&mut self) -> Result<ExitStatus> {
        // Linux child PIDs fit pid_t. This leader remains unreaped until the
        // first group signal, so its PID cannot refer to an unrelated group.
        let group = self.child.id() as libc::pid_t;
        if !self.signalled {
            // SAFETY: negative group addresses only the group created for our child.
            let result = unsafe { libc::kill(-group, libc::SIGKILL) };
            if result < 0 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error.into());
                }
            }
            self.signalled = true;
        }
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            let mut status = 0;
            // SAFETY: status is writable; this waits only for our fixture's
            // group, including orphan descendants adopted as a subreaper.
            let reaped = unsafe { libc::waitpid(-group, &mut status, libc::WNOHANG) };
            if reaped == group {
                self.leader_status = Some(ExitStatus::from_raw(status));
            }
            if reaped < 0 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() == Some(libc::ECHILD) {
                    self.finished = true;
                    return self
                        .leader_status
                        .ok_or_else(|| "fixture leader status unavailable".into());
                }
                if error.raw_os_error() != Some(libc::EINTR) {
                    return Err(error.into());
                }
            }
            if Instant::now() >= until {
                return Err("fixture group cleanup did not finish within two seconds".into());
            }
            if reaped <= 0 {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}
impl Drop for FixtureGroup {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.finish();
        }
    }
}

/// Execute one exact test, preserve its output and refuse an empty green selection.
/// The owned process group gets ten minutes, including costly opt-in probes.
fn execute_test(
    suite: &Suite,
    name: &str,
    temp: &Path,
    logs: &Path,
    environment: &BTreeMap<&str, OsString>,
) -> Result<bool> {
    let log = logs.join(format!(
        "{}.log",
        crate::hash(format!("{}::{}::{name}", suite.package, suite.target).as_bytes())
    ));
    let file = File::create(&log)?;
    // SAFETY: this Linux process adopts orphans, so fixture group members can
    // be reaped after their parent exits. Every wait below selects its own
    // group, never children of another concurrently executing fixture.
    if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) } < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let child = Command::new(&suite.executable)
        .args([
            "--ignored",
            "--exact",
            name,
            "--nocapture",
            "--test-threads=1",
            "--format=pretty",
        ])
        .envs(environment)
        .env("TMPDIR", temp)
        .stdin(Stdio::null())
        .stdout(file.try_clone()?)
        .stderr(file)
        .process_group(0)
        .spawn()?;
    let mut group = FixtureGroup {
        child,
        signalled: false,
        finished: false,
        leader_status: None,
    };
    let until = Instant::now() + Duration::from_secs(600);
    let timed_out = loop {
        if group.exited()? {
            break false;
        }
        if Instant::now() >= until {
            break true;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let success = group.finish()?.success() && !timed_out;
    let output = std::fs::read_to_string(&log)?;
    let passed = output
        .lines()
        .any(|line| line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;"));
    let failed = output
        .lines()
        .any(|line| line.starts_with("test result: FAILED. 0 passed; 1 failed; 0 ignored;"));
    if !passed && !failed {
        return Err("test did not report exactly one executed case (empty selection, timeout or runner error); see log".into());
    }
    Ok(success && passed)
}
#[cfg(test)]
fn execute_one(suite: &Suite, name: &str, temp: &Path) -> Result<bool> {
    execute_test(suite, name, temp, temp, &BTreeMap::new())
}

fn executable(path: &Path) -> bool {
    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}
fn prerequisites(environment: &BTreeMap<&str, OsString>) -> BTreeMap<&'static str, String> {
    let mut missing = BTreeMap::new();
    if !seccomp_notification_abi_available() {
        missing.insert(SECCOMP, "settlement fixtures require Linux x86_64, the user_notif kernel action and notification ABI sizes 80/24/64; runtime policy must also permit listener installation and owned-process fd inspection".into());
    }
    let custody = executable(Path::new("/usr/bin/dbus-daemon"))
        && std::fs::read("/usr/bin/gnome-keyring-daemon")
            .is_ok_and(|bytes| crate::hash(&bytes) == GNOME_SHA256);
    if !custody {
        missing.insert(CUSTODY, "requires dbus-daemon and the exact qualified GNOME Keyring artifact (see docs/local-secret-service.md)".into());
    }
    for key in [CLI, PROVIDER, OLD_CLI] {
        let path = environment
            .get(key)
            .cloned()
            .or_else(|| std::env::var_os(key));
        if !path.is_some_and(|path| executable(Path::new(&path))) {
            missing.insert(key, format!("{key} must name an executable file"));
        }
    }
    if !std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .any(|path| executable(&path.join(CHROME)))
    {
        missing.insert(
            CHROME,
            "google-chrome-stable is not executable on PATH".into(),
        );
    }
    if !std::env::var(PG).ok().is_some_and(|value| {
        value.rsplit_once(':').is_some_and(|(host, port)| {
            !host.is_empty() && port.parse::<u16>().is_ok_and(|port| port > 0)
        })
    }) {
        missing.insert(
            PG,
            "CONNECTORS_PG_SANDBOX must name the prepared sandbox as host:port".into(),
        );
    }
    if !std::env::var(PG_CONTAINER).is_ok_and(|value| {
        value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
    }) {
        missing.insert(
            PG_CONTAINER,
            "CONNECTORS_PG_CONTAINER must name the owned disposable PostgreSQL container with psql"
                .into(),
        );
    }
    if !std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .any(|path| executable(&path.join(DOCKER)))
    {
        missing.insert(DOCKER, "docker is not executable on PATH".into());
    }
    if !std::env::var(K8S).is_ok_and(|value| !value.trim().is_empty()) {
        missing.insert(
            K8S,
            "CONNECTORS_K8S_SANDBOX must name the prepared sandbox API base".into(),
        );
    }
    if !executable(Path::new(KUBECTL)) {
        missing.insert(KUBECTL, "/usr/bin/kubectl is not executable".into());
    }
    for key in [CA, TOKEN, KUBECONFIG] {
        if !std::env::var_os(key).is_some_and(|path| {
            std::fs::metadata(path).is_ok_and(|metadata| {
                metadata.is_file()
                    && (key != KUBECONFIG || metadata.permissions().mode() & 0o077 == 0)
            })
        }) {
            missing.insert(
                key,
                format!("{key} must name the sandbox's owner-only file"),
            );
        }
    }
    missing
}

fn seccomp_notification_abi_available() -> bool {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        if !std::fs::read_to_string("/proc/sys/kernel/seccomp/actions_avail").is_ok_and(|actions| {
            actions
                .split_whitespace()
                .any(|action| action == "user_notif")
        }) {
            return false;
        }
        const GET_NOTIF_SIZES: libc::c_uint = 3;
        let mut sizes = [0_u16; 3];
        // SAFETY: GET_NOTIF_SIZES only writes the documented three-u16 output;
        // it installs no filter and changes no process policy.
        let result =
            unsafe { libc::syscall(libc::SYS_seccomp, GET_NOTIF_SIZES, 0, sizes.as_mut_ptr()) };
        result == 0 && sizes == [80, 24, 64]
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        false
    }
}

/// Build every default-feature workspace test target, then query the actual
/// libtest executables. Cargo metadata maps opaque package IDs to names.
fn workspace(root: &Path) -> Result<(Vec<Suite>, BTreeMap<&'static str, OsString>)> {
    let metadata: Value = serde_json::from_slice(
        &crate::run(Command::new("cargo").current_dir(root).args([
            "metadata",
            "--locked",
            "--offline",
            "--no-deps",
            "--format-version=1",
        ]))?
        .stdout,
    )?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("Cargo workspace members missing")?;
    let packages: BTreeMap<_, _> = metadata["packages"]
        .as_array()
        .ok_or("Cargo packages missing")?
        .iter()
        .filter(|package| members.contains(&package["id"]))
        .map(|package| {
            Ok((
                package["id"]
                    .as_str()
                    .ok_or("Cargo package id missing")?
                    .to_owned(),
                package["name"]
                    .as_str()
                    .ok_or("Cargo package name missing")?
                    .to_owned(),
            ))
        })
        .collect::<Result<_>>()?;
    let output = crate::run(
        Command::new("cargo")
            .current_dir(root)
            .env("CARGO_BUILD_JOBS", "2")
            .args([
                "test",
                "--workspace",
                "--no-run",
                "--locked",
                "--offline",
                "--message-format=json",
            ]),
    )?;
    let mut suites = Vec::new();
    let mut environment = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for line in std::str::from_utf8(&output.stdout)?.lines() {
        let artifact: Value = serde_json::from_str(line)?;
        if artifact["reason"] != "compiler-artifact" {
            continue;
        }
        let Some(package) = artifact["package_id"]
            .as_str()
            .and_then(|id| packages.get(id))
        else {
            continue;
        };
        let Some(executable) = artifact["executable"].as_str() else {
            continue;
        };
        let target = artifact["target"]["name"]
            .as_str()
            .ok_or("Cargo target name missing")?;
        if artifact["profile"]["test"] == true && seen.insert(executable.to_owned()) {
            suites.push(Suite {
                package: package.clone(),
                target: target.into(),
                executable: executable.into(),
            });
        } else if artifact["profile"]["test"] == false {
            let key = match (package.as_str(), target) {
                ("connectors", "connectors") => Some(CLI),
                ("connectors-catalog-provider", "connectors-catalog-provider") => Some(PROVIDER),
                _ => None,
            };
            if let Some(key) = key {
                environment.insert(key, OsString::from(executable));
            }
        }
    }
    if suites.is_empty() {
        return Err("Cargo produced no workspace test executables".into());
    }
    suites.sort_by(|a, b| (&a.package, &a.target).cmp(&(&b.package, &b.target)));
    Ok((suites, environment))
}

fn temporary(root: &Path, selected: Option<&Path>) -> Result<tempfile::TempDir> {
    let base = selected
        .map(|path| root.join(path))
        .unwrap_or_else(|| root.join(".local/tmp"));
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&base)?;
    if base.canonicalize()? != base {
        return Err("fixture temporary directory must be a physical path, without symlinks or parent traversal".into());
    }
    let stat = std::fs::metadata(&base)?;
    if stat.uid() != std::fs::metadata("/proc/self")?.uid()
        || (selected.is_some() && stat.mode() & 0o077 != 0)
    {
        return Err(
            "--tmpdir must be an owner-private directory (mode 700) owned by this user".into(),
        );
    }
    Ok(tempfile::Builder::new()
        .prefix("ignored-")
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir_in(base)?)
}

pub fn run(root: &Path, options: Options) -> Result<()> {
    let (suites, environment) = workspace(root)?;
    let mut entries = Vec::new();
    let mut names = BTreeSet::new();
    for suite in &suites {
        for entry in discover(suite)? {
            if !names.insert(entry.id()) {
                return Err(format!("duplicate inventory entry {}", entry.id()).into());
            }
            entries.push(entry);
        }
    }
    if entries.is_empty() {
        return Err("workspace ignored inventory is empty".into());
    }
    let report_path = root.join(&options.report);
    let logs = report_path.with_extension("logs");
    std::fs::create_dir_all(&logs)?;
    let temp = temporary(root, options.tmpdir.as_deref())?;
    let mut unavailable = prerequisites(&environment);
    // The longest custody fixture adds a random directory and a daemon control
    // socket. Admission refuses symlinks, so a proc-fd alias is not a substitute.
    if temp.path().as_os_str().len() + 48 >= 108 {
        let reason = "fixture path is too long for private Unix sockets; supply --tmpdir with a short physical owner-private directory";
        unavailable
            .entry(CUSTODY)
            .and_modify(|existing| {
                existing.push_str("; ");
                existing.push_str(reason);
            })
            .or_insert_with(|| reason.into());
    }
    let report = evaluate(
        entries,
        &options,
        |entry| {
            entry
                .prerequisites
                .iter()
                .filter_map(|key| unavailable.get(key).cloned())
                .collect()
        },
        |entry| {
            eprintln!("ignored: running {}", entry.id());
            execute_test(&entry.suite, &entry.name, temp.path(), &logs, &environment)
        },
    );
    let value = serde_json::json!({"format":"connectors.ignored-suites/1", "mode":if options.inventory { "inventory" } else { "execute" }, "scope":"workspace default features", "families":options.family, "required":options.required, "test_binaries":suites.len(), "logs":logs, "result":report});
    crate::save_json(&report_path, &value)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    if !report.accepted(options.required) {
        return Err(
            "ignored suite execution refused or failed; see the named outcomes in the report"
                .into(),
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn options() -> Options {
        Options {
            family: vec![Family::Disposable],
            inventory: false,
            required: true,
            report: PathBuf::new(),
            tmpdir: None,
        }
    }
    /// A bounded real Rust libtest executable. No provider, shell script or ambient service.
    fn fixture(fail: bool, unknown: bool) -> (tempfile::TempDir, Suite) {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("fixture.rs");
        std::fs::write(&source, format!(r#"
mod local {{
    mod keyring {{ pub mod custody {{ mod tests {{
        #[test] #[ignore] fn disposable_secret_service_restart_and_failures() {{ assert!({}); }}
    }} }} }}
    mod clock {{ mod tests {{ #[test] #[ignore] fn independent_live_source() {{ panic!("live never authorized"); }} }} }}
    mod audit {{ mod tests {{ #[test] #[ignore] fn crash_child() {{ panic!("helper never top level"); }} }} }}
}}
{}
"#, !fail, if unknown { "#[test] #[ignore] fn unclassified() {}" } else { "" })).unwrap();
        let executable = root.path().join("fixture");
        let output = Command::new("rustc")
            .args(["--test", "--edition=2024"])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        (
            root,
            Suite {
                package: "connectors-host".into(),
                target: "connectors_host".into(),
                executable,
            },
        )
    }
    fn run_fixture(fail: bool, unknown: bool, missing: bool, opts: Options) -> Report {
        let (root, suite) = fixture(fail, unknown);
        evaluate(
            discover(&suite).unwrap(),
            &opts,
            |_| {
                if missing {
                    vec!["qualified disposable Secret Service unavailable".into()]
                } else {
                    vec![]
                }
            },
            |entry| execute_one(&entry.suite, &entry.name, root.path()),
        )
    }
    #[test]
    fn ignored_runner_has_an_explicit_cli_and_defaults_to_disposable() {
        let args = crate::Args::try_parse_from(["connectors-build", "ignored", "--inventory"])
            .expect("the ignored suite runner is an independent CLI command");
        let crate::Action::Ignored(options) = args.command else {
            panic!("ignored action required");
        };
        assert_eq!(options.family, [Family::Disposable]);
        assert!(
            crate::Args::try_parse_from([
                "connectors-build",
                "ignored",
                "--inventory",
                "--required"
            ])
            .is_err()
        );
    }
    #[test]
    fn ignored_inventory_accounted() {
        assert!(
            include_str!("../../connectors-host/src/local/keyring/custody/gnome.rs")
                .contains(GNOME_SHA256),
            "custody qualification changed; review runner prerequisites"
        );
        let report = run_fixture(false, false, false, options());
        assert_eq!(
            (
                report.inventory,
                report.selected,
                report.executed,
                report.skipped
            ),
            (3, 1, 1, 2)
        );
        assert!(report.accepted(true));
        assert_eq!(report.outcomes.len(), 3);
    }
    #[test]
    fn missing_prerequisite_is_explicit() {
        let root = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let owned = temporary(root.path(), Some(root.path())).unwrap();
        assert_eq!(
            std::fs::metadata(owned.path()).unwrap().mode() & 0o777,
            0o700
        );
        let alias = root.path().join("alias");
        std::os::unix::fs::symlink(root.path(), &alias).unwrap();
        assert!(temporary(root.path(), Some(&alias)).is_err());
        let public = root.path().join("public");
        std::fs::create_dir(&public).unwrap();
        std::fs::set_permissions(&public, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(temporary(root.path(), Some(&public)).is_err());
        let report = run_fixture(false, false, true, options());
        assert_eq!(
            (report.selected, report.executed, report.missing),
            (1, 0, 1)
        );
        assert!(!report.accepted(true));
        assert!(report.accepted(false));
        assert!(
            report
                .outcomes
                .iter()
                .any(|o| o.reasons.iter().any(|r| r.contains("Secret Service")))
        );
    }
    #[test]
    fn selected_test_failure_is_nonzero() {
        let report = run_fixture(true, false, false, options());
        assert_eq!((report.executed, report.failed), (1, 1));
        assert!(!report.accepted(false));
    }
    #[test]
    fn subprocess_helper_never_top_level() {
        let mut opts = options();
        opts.family = vec![
            Family::Disposable,
            Family::Live,
            Family::Timing,
            Family::Deferred,
        ];
        let report = run_fixture(false, false, false, opts);
        assert_eq!(report.executed, 2);
        assert!(
            report
                .outcomes
                .iter()
                .any(|o| o.name.ends_with("crash_child") && !o.selected && o.status == "helper")
        );
    }
    #[test]
    fn unknown_inventory_refuses_before_any_execution() {
        let report = run_fixture(false, true, false, options());
        assert_eq!(report.unknown, 1);
        assert_eq!(report.executed, 0);
        assert!(!report.accepted(false));
    }
    #[test]
    fn adversary_nonprivate_kubeconfig_is_missing_before_dispatch() {
        const CHILD: &str = "CONNECTORS_CLASSIFIER_MODE_PROBE";
        if let Ok(expected) = std::env::var(CHILD) {
            let missing = prerequisites(&BTreeMap::new());
            assert_eq!(
                missing.contains_key(KUBECONFIG),
                expected == "missing",
                "kubeconfig prerequisite must distinguish mode-0644 from mode-0600"
            );
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("kubeconfig");
        std::fs::write(&path, b"fixture configuration, no credential\n").unwrap();
        for (mode, expected) in [(0o644, "missing"), (0o600, "present")] {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
            let output = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "ignored::tests::adversary_nonprivate_kubeconfig_is_missing_before_dispatch",
                    "--nocapture",
                ])
                .env(CHILD, expected)
                .env(KUBECONFIG, &path)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "isolated prerequisite probe failed: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn inventory_mode_never_executes_selected_cases() {
        let mut opts = options();
        opts.inventory = true;
        let report = run_fixture(true, false, false, opts);
        assert_eq!(
            (report.inventory, report.selected, report.executed),
            (3, 1, 0)
        );
    }
    #[test]
    fn zero_test_success_is_not_execution() {
        let (root, suite) = fixture(false, false);
        assert!(
            execute_one(&suite, "nonexistent", root.path())
                .unwrap_err()
                .to_string()
                .contains("exactly one executed case")
        );
        let report = evaluate(
            discover(&suite).unwrap(),
            &options(),
            |_| vec![],
            |_| Err("not started".into()),
        );
        assert_eq!((report.selected, report.executed, report.failed), (1, 0, 1));
        assert!(!report.accepted(false));
    }
    fn adversary_descendant_fixture(panics: bool) -> (tempfile::TempDir, Suite) {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("descendant.rs");
        let pid = root.path().join("descendant.pid");
        std::fs::write(
            &source,
            format!(
                r#"
#[test]
#[ignore]
fn selected_fixture() {{
    let child = std::process::Command::new("/bin/sleep")
        .arg("900")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn().unwrap();
    std::fs::write({pid:?}, child.id().to_string()).unwrap();
    assert!(!{panics}, "fixture fails after launching its child");
}}
"#,
            ),
        )
        .unwrap();
        let executable = root.path().join("descendant");
        let output = Command::new("rustc")
            .args(["--test", "--edition=2024"])
            .arg(source)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        (
            root,
            Suite {
                package: "connectors-host".into(),
                target: "connectors_host".into(),
                executable,
            },
        )
    }

    fn adversary_assert_descendants_stopped(panics: bool) {
        let (root, suite) = adversary_descendant_fixture(panics);
        let outcome = execute_one(&suite, "selected_fixture", root.path());
        let pid = std::fs::read_to_string(root.path().join("descendant.pid")).unwrap();
        let running = std::fs::read_to_string(format!("/proc/{pid}/status"))
            .is_ok_and(|status| !status.lines().any(|line| line.starts_with("State:\tZ")));
        // Clean exactly the descendant this test created before any assertion can panic.
        if running {
            let killed = Command::new("/bin/kill")
                .args(["-KILL", "--", pid.trim()])
                .status()
                .unwrap();
            assert!(killed.success());
        }
        assert_eq!(outcome.unwrap(), !panics);
        assert!(
            !running,
            "runner returned while fixture descendant {pid} was still running; child planned to outlive the 600-second fixture deadline"
        );
    }

    #[test]
    fn adversary_passing_fixture_does_not_leave_running_descendants() {
        adversary_assert_descendants_stopped(false);
    }

    #[test]
    fn adversary_failing_fixture_does_not_leave_running_descendants() {
        adversary_assert_descendants_stopped(true);
    }

    #[test]
    fn adversary_error_drops_live_group_without_reaping_another_fixture() {
        let spawn = || {
            Command::new("/bin/sleep")
                .arg("900")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .process_group(0)
                .spawn()
                .unwrap()
        };
        let mut other_fixture = spawn();
        let child = spawn();
        let owned_pid = child.id() as libc::pid_t;
        let started = Instant::now();
        let result: Result<()> = (|| {
            let group = FixtureGroup {
                child,
                signalled: false,
                finished: false,
                leader_status: None,
            };
            assert!(!group.exited()?);
            Err("injected runner error after spawn".into())
        })();
        let elapsed = started.elapsed();
        let mut status = 0;
        // SAFETY: observe only the exact child created above; no process scan.
        let waited = unsafe { libc::waitpid(owned_pid, &mut status, libc::WNOHANG) };
        let error = std::io::Error::last_os_error();
        if waited == 0 {
            // A failing cleanup assertion must not leave this test's own child.
            // SAFETY: waitpid just proved it remains our unreaped live child.
            unsafe {
                libc::kill(owned_pid, libc::SIGKILL);
                libc::waitpid(owned_pid, &mut status, 0);
            }
        }
        let other_still_running = other_fixture.try_wait().unwrap().is_none();
        if other_still_running {
            other_fixture.kill().unwrap();
        }
        other_fixture.wait().unwrap();
        assert_eq!(
            result.unwrap_err().to_string(),
            "injected runner error after spawn"
        );
        assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
        assert_eq!(waited, -1, "owned live leader must already be reaped");
        assert_eq!(error.raw_os_error(), Some(libc::ECHILD));
        assert!(other_still_running, "another fixture group was affected");
    }
}

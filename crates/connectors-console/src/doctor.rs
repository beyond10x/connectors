//! `connectors doctor` — why it is not working, before you have to guess.
//!
//! Every check here exists because someone lost time to the thing it reports. The socket-path
//! budget is the clearest case: a state root a few characters too deep produces
//! `path must be shorter than SUN_LEN` if you are lucky and *"connection management is temporarily
//! unavailable"* if you are not — a retriable-sounding message for something that can never work,
//! at a moment when the operator has no reason to suspect path length. It cost the concurrent
//! Zwirn session a debugging session, and it cost this one a failed first run.
//!
//! A check reports one of three states. `fail` is "this cannot work"; `warn` is "this works and you
//! should know"; `ok` is silence worth confirming. Only `fail` sets the exit code, so `doctor` in a
//! script means *is this installation usable*, not *is it perfect*.

use std::os::unix::fs::MetadataExt as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use connectors_config::PersonalConfig;
use serde_json::{json, Value};

/// The longest path `bind(2)` accepts for a Unix socket, less the terminating NUL.
///
/// `sockaddr_un.sun_path` is 108 bytes on Linux and the kernel requires room for the NUL, so 107 is
/// the last length that binds.
const MAX_SOCKET_PATH_BYTES: usize = 107;

/// What a Connect Session endpoint adds below the state root: `/connect-sessions/<uuid>.sock`.
///
/// Sized against the *deepest* path the daemon will try to bind, not the shallowest. Checking
/// `connectors.sock` alone is what let the deeper failure through: the daemon starts, publishes
/// readiness, and only fails later when someone tries to hand it a credential.
const CONNECT_SESSION_PATH_BYTES: usize = "/connect-sessions/".len() + 36 + ".sock".len();

/// The daemon's own control socket, below the state root.
const CONTROL_SOCKET_PATH_BYTES: usize = "/connectors.sock".len();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Ok,
    Warn,
    Fail,
}

impl Status {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Fail => "fail",
        }
    }
}

struct Check {
    name: &'static str,
    status: Status,
    detail: String,
}

impl Check {
    fn new(name: &'static str, status: Status, detail: impl Into<String>) -> Self {
        Self {
            name,
            status,
            detail: detail.into(),
        }
    }
}

/// The report, and whether anything in it is fatal.
pub struct Report {
    checks: Vec<Check>,
}

impl Report {
    /// True when at least one check says this installation cannot work.
    #[must_use]
    pub fn healthy(&self) -> bool {
        !self.checks.iter().any(|check| check.status == Status::Fail)
    }

    /// The report as data, so every output format renders the same facts.
    #[must_use]
    pub fn to_value(&self) -> Value {
        json!({
            "healthy": self.healthy(),
            "checks": self.checks.iter().map(|check| json!({
                "check": check.name,
                "status": check.status.as_str(),
                "detail": check.detail,
            })).collect::<Vec<_>>(),
        })
    }
}

/// Inspect this installation without changing any of it.
///
/// Deliberately read-only: a diagnostic that repairs things is a diagnostic you cannot trust to
/// tell you what state you were in.
#[must_use]
pub fn run(config_path: &Path, state_root: &Path) -> Report {
    let mut checks = Vec::new();

    checks.push(check_config(config_path));
    checks.push(check_state_root(state_root));
    checks.extend(check_socket_budget(state_root));
    checks.push(check_daemon(state_root));
    checks.push(check_credential_store(state_root));

    Report { checks }
}

fn check_config(path: &Path) -> Check {
    if !path.exists() {
        return Check::new(
            "configuration",
            Status::Fail,
            format!(
                "no configuration at {}; run `connectors init`",
                path.display()
            ),
        );
    }
    match PersonalConfig::read(path) {
        Ok(config) => {
            let mut declared = Vec::new();
            if config.slack.is_some() {
                declared.push("slack");
            }
            if config.grafana.is_some() {
                declared.push("grafana");
            }
            if config.kubernetes.is_some() {
                declared.push("kubernetes");
            }
            if config.platform.is_some() {
                declared.push("platform");
            }
            if config.sip.is_some() {
                declared.push("sip");
            }
            // Catalogued providers are named individually rather than as one "catalog" entry: an
            // operator asking what this placement can reach wants the provider names, and the
            // whole point of the generic adapter is that adding one is a row, not a component.
            let catalogued: Vec<String> = config
                .catalog
                .iter()
                .map(|entry| entry.provider.clone())
                .collect();
            let mut named: Vec<String> = declared.iter().map(|item| (*item).to_owned()).collect();
            named.extend(catalogued);
            if named.is_empty() {
                return Check::new(
                    "configuration",
                    Status::Warn,
                    format!("{} declares no integration", path.display()),
                );
            }
            Check::new(
                "configuration",
                Status::Ok,
                format!("{} declares {}", path.display(), named.join(", ")),
            )
        }
        Err(error) => Check::new(
            "configuration",
            Status::Fail,
            format!("{} cannot be read: {error}", path.display()),
        ),
    }
}

fn check_state_root(path: &Path) -> Check {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return Check::new(
            "state-root",
            Status::Warn,
            format!(
                "{} does not exist yet; it is created on first serve",
                path.display()
            ),
        );
    };
    if !metadata.is_dir() {
        return Check::new(
            "state-root",
            Status::Fail,
            format!("{} is not a directory", path.display()),
        );
    }
    let owner = rustix::process::geteuid().as_raw();
    if metadata.uid() != owner {
        return Check::new(
            "state-root",
            Status::Fail,
            format!(
                "{} belongs to uid {}, not {owner}",
                path.display(),
                metadata.uid()
            ),
        );
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Check::new(
            "state-root",
            Status::Fail,
            format!(
                "{} is readable by others (mode {:o}); the daemon refuses it",
                path.display(),
                metadata.permissions().mode() & 0o7777
            ),
        );
    }
    Check::new(
        "state-root",
        Status::Ok,
        format!("{} is owner-only", path.display()),
    )
}

/// The two socket paths the daemon will bind, measured against what `bind(2)` accepts.
///
/// Two checks rather than one, because they fail at different times and mean different things: the
/// control socket failing means the daemon will not start at all, and the Connect Session path
/// failing means it starts, works, and then cannot accept a credential.
fn check_socket_budget(state_root: &Path) -> Vec<Check> {
    let root = state_root.as_os_str().as_encoded_bytes().len();
    let control = root + CONTROL_SOCKET_PATH_BYTES;
    let session = root + CONNECT_SESSION_PATH_BYTES;

    let control_check = if control > MAX_SOCKET_PATH_BYTES {
        Check::new(
            "socket-path",
            Status::Fail,
            format!(
                "the control socket would be {control} bytes and bind(2) accepts \
                 {MAX_SOCKET_PATH_BYTES}; choose a shorter --state-root"
            ),
        )
    } else {
        Check::new(
            "socket-path",
            Status::Ok,
            format!("control socket fits in {control} of {MAX_SOCKET_PATH_BYTES} bytes"),
        )
    };

    let session_check = if session > MAX_SOCKET_PATH_BYTES {
        Check::new(
            "connect-session-path",
            Status::Fail,
            format!(
                "a Connect Session endpoint would be {session} bytes and bind(2) accepts \
                 {MAX_SOCKET_PATH_BYTES}: the daemon starts and then cannot accept a credential. \
                 Choose a --state-root at most {} bytes long",
                MAX_SOCKET_PATH_BYTES - CONNECT_SESSION_PATH_BYTES
            ),
        )
    } else {
        Check::new(
            "connect-session-path",
            Status::Ok,
            format!("credential endpoints fit in {session} of {MAX_SOCKET_PATH_BYTES} bytes"),
        )
    };

    vec![control_check, session_check]
}

fn check_daemon(state_root: &Path) -> Check {
    let socket = state_root.join("connectors.sock");
    if !socket.exists() {
        return Check::new(
            "daemon",
            Status::Warn,
            "not running; one-shot commands work, `event receive` needs `connectors serve`",
        );
    }
    match std::os::unix::net::UnixStream::connect(&socket) {
        Ok(_) => Check::new(
            "daemon",
            Status::Ok,
            format!("listening on {}", socket.display()),
        ),
        // A socket file with nothing behind it is what an ungraceful stop leaves. The next `serve`
        // removes it, so this is a warning rather than a failure — but silence here would read as
        // "the daemon is up".
        Err(error) => Check::new(
            "daemon",
            Status::Warn,
            format!(
                "{} exists but nothing is listening ({error}); a previous daemon was killed",
                socket.display()
            ),
        ),
    }
}

/// What protects a stored credential at rest, said plainly.
///
/// The personal store is an unencrypted file whose only protection is Unix ownership and mode. That
/// is a real guarantee against another user and no guarantee at all against a copied backup, so it
/// is reported as a warning rather than passed over in silence.
fn check_credential_store(state_root: &Path) -> Check {
    let file = state_root.join("credentials.store");
    let keyring = which("secret-tool").is_some();
    match (keyring, file.exists()) {
        // Both: the keyring is what new credentials go to, and the file is the two-phase store
        // Slack still uses. Saying so beats reporting only the better one.
        (true, true) => Check::new(
            "credential-store",
            Status::Warn,
            format!(
                "OS keyring for values; {} still holds Slack's two-phase store, protected by file \
                 ownership only and not encrypted at rest",
                file.display()
            ),
        ),
        (true, false) => Check::new(
            "credential-store",
            Status::Ok,
            "OS keyring (freedesktop Secret Service)",
        ),
        (false, true) => Check::new(
            "credential-store",
            Status::Warn,
            format!(
                "{} holds credentials protected by file ownership only — not encrypted at rest. \
                 Install `secret-tool` (libsecret) to use the OS keyring instead",
                file.display()
            ),
        ),
        (false, false) => Check::new(
            "credential-store",
            Status::Warn,
            "no OS keyring: `secret-tool` (libsecret) is not installed, so credentials would be \
             stored in an unencrypted owner-only file",
        ),
    }
}

/// Whether a program is on `PATH`. Presence only — the store itself decides whether the service
/// behind it answers.
fn which(program: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(program))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_budget_is_measured_against_the_deepest_path_the_daemon_binds() {
        // A state root that fits the control socket and not a Connect Session endpoint is exactly
        // the case that starts cleanly and fails later. It must be reported before that.
        let root_len = MAX_SOCKET_PATH_BYTES - CONNECT_SESSION_PATH_BYTES + 1;
        let root = std::path::PathBuf::from(format!("/{}", "x".repeat(root_len - 1)));
        let checks = check_socket_budget(&root);
        assert_eq!(
            checks[0].status,
            Status::Ok,
            "the control socket still fits"
        );
        assert_eq!(
            checks[1].status,
            Status::Fail,
            "a credential endpoint does not"
        );
        assert!(checks[1].detail.contains("cannot accept a credential"));
    }

    #[test]
    fn a_short_state_root_passes_both_budgets() {
        let checks = check_socket_budget(std::path::Path::new("/home/x/.local/state/connectors"));
        assert!(checks.iter().all(|check| check.status == Status::Ok));
    }

    #[test]
    fn a_missing_configuration_is_fatal_and_names_the_command_that_fixes_it() {
        let check = check_config(std::path::Path::new("/nonexistent/connectors.toml"));
        assert_eq!(check.status, Status::Fail);
        assert!(check.detail.contains("connectors init"));
    }

    #[test]
    fn a_report_is_unhealthy_only_when_something_cannot_work() {
        let warned = Report {
            checks: vec![Check::new("x", Status::Warn, "y")],
        };
        assert!(warned.healthy(), "a warning is not a failure");
        let failed = Report {
            checks: vec![Check::new("x", Status::Fail, "y")],
        };
        assert!(!failed.healthy());
    }

    #[test]
    fn the_report_renders_every_check_as_data() {
        let report = Report {
            checks: vec![Check::new("configuration", Status::Ok, "fine")],
        };
        let value = report.to_value();
        assert_eq!(value["healthy"], true);
        assert_eq!(value["checks"][0]["check"], "configuration");
        assert_eq!(value["checks"][0]["status"], "ok");
    }

    #[test]
    fn every_state_a_check_can_report_reaches_the_reader_as_its_own_marker() {
        // This module ranks its three states — `fail` is "this cannot work", `warn` is "this works
        // and you should know" — and the rank is worth nothing if the last inch flattens it, which
        // is what the generic pretty-printer used to do. Asserted through the real renderer rather
        // than against a table of words, so the two cannot drift apart.
        let report = Report {
            checks: vec![
                Check::new("first", Status::Ok, "fine"),
                Check::new("second", Status::Warn, "worth knowing"),
                Check::new("third", Status::Fail, "cannot work"),
            ],
        };
        let rendered = crate::output::render(crate::output::Format::Text, &report.to_value())
            .expect("a text rendering");
        let marker = |name: &str| {
            rendered
                .lines()
                .find(|line| line.contains(name))
                .unwrap_or_else(|| panic!("no row for `{name}`:\n{rendered}"))
                .trim_start()
                .chars()
                .next()
                .expect("a leading marker")
        };
        let markers = [marker("first"), marker("second"), marker("third")];
        assert_eq!(
            std::collections::BTreeSet::from(markers).len(),
            markers.len(),
            "two states share one marker: {markers:?}\n{rendered}"
        );
        assert!(
            !markers.contains(&'?'),
            "a state the renderer cannot rank: {markers:?}\n{rendered}"
        );
    }
}

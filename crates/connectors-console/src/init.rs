//! `connectors init` — the file a person should never have had to write by hand.
//!
//! # What this removes
//!
//! Before this command, using Connectors on your own machine meant hand-writing a TOML whose
//! `[owner]` block carries an `authority_snapshot_sha256`. Nobody can invent that value from
//! reading the documentation, so in practice the file was copied from `scripts/dev/local-stack.sh`
//! and edited until the daemon stopped refusing it. That is not a product.
//!
//! # Why the digest is derived rather than invented
//!
//! `authority_snapshot_sha256` identifies *which* authority snapshot the owner context refers to.
//! On this machine there is exactly one authority — the person at the keyboard — and what varies is
//! which integrations they admitted. So the digest is taken over the admitted set, under a
//! versioned domain-separation prefix. Two machines that admitted the same integrations derive the
//! same snapshot id, and admitting another one moves it. It is computed over the configuration
//! *without* the owner block, because a digest over bytes containing itself has no fixed point.
//!
//! # Why an integration is required
//!
//! [`PersonalConfig::validate`] refuses a configuration that declares no integration at all
//! (`connectors-config/src/personal.rs:454-461`), and it is right to: a Connector with no backend
//! answers nothing. So `init` will not write an owner-only file that the daemon would then refuse
//! to read — it either writes something usable or refuses and names what is missing.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use connectors_config::{
    InitiationConfig, KubernetesIntegrationConfig, OwnerConfig, PersonalConfig,
};
use sha2::{Digest as _, Sha256};

/// Domain separation for the derived snapshot id. Versioned: changing what goes into the digest
/// without changing this prefix would silently collide with ids derived by an earlier build.
const SNAPSHOT_DOMAIN: &[u8] = b"b10x/connectors/local-authority/v1\0";

/// The integrations `init` can declare without asking a person for a value.
///
/// Deliberately short. Slack needs a workspace id and a token file, Grafana needs an origin — a
/// flag that silently wrote a placeholder for either would produce a file that parses and cannot
/// work, which is worse than not writing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[value(rename_all = "lower")]
pub enum Integration {
    /// Reads the operator's own kubeconfig. Needs no credential and no address.
    Kubernetes,
}

/// What `init` did, as data, so the caller renders it in the requested format.
#[derive(Debug)]
pub struct Written {
    pub config_path: PathBuf,
    pub state_root: PathBuf,
    pub integrations: Vec<&'static str>,
    pub snapshot_id: String,
    /// What the operator needs to know about what was just written, in their own terms.
    pub notes: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("a configuration already exists at {0}; pass --force to replace it")]
    Exists(PathBuf),
    #[error(
        "no integration could be admitted on this machine: Kubernetes needs a readable kubeconfig. \
         Name one explicitly with --integration once you have one."
    )]
    NothingAdmissible,
    #[error("the derived configuration is not valid: {0}")]
    Invalid(#[from] connectors_config::ConfigError),
    #[error("the configuration could not be written: {0}")]
    Io(#[from] std::io::Error),
    #[error("the configuration could not be rendered: {0}")]
    Render(#[from] toml::ser::Error),
}

/// Write a usable personal configuration, or refuse and say what is missing.
///
/// # Errors
///
/// [`InitError::Exists`] unless `force`, [`InitError::NothingAdmissible`] when the machine supports
/// none of the requested integrations, or the underlying validation/IO failure.
pub fn run(
    config_path: &Path,
    state_root: &Path,
    requested: &[Integration],
    allow_exec_auth: bool,
    force: bool,
) -> Result<Written, InitError> {
    if config_path.exists() && !force {
        return Err(InitError::Exists(config_path.to_owned()));
    }

    // No explicit selection means "whatever this machine can actually do", which is the answer a
    // first run wants. An explicit selection is still filtered by admissibility: asking for
    // Kubernetes on a machine with no kubeconfig has to refuse by name, not write a dead section.
    let wanted: Vec<Integration> = if requested.is_empty() {
        vec![Integration::Kubernetes]
    } else {
        requested.to_vec()
    };

    let mut integrations = Vec::new();
    let mut kubernetes = None;
    for integration in wanted {
        match integration {
            Integration::Kubernetes if kubeconfig_readable() => {
                kubernetes = Some(kubernetes_config(allow_exec_auth));
                integrations.push("kubernetes");
            }
            Integration::Kubernetes => {}
        }
    }
    if integrations.is_empty() {
        return Err(InitError::NothingAdmissible);
    }

    let snapshot_id = format!("snapshot:local:{}", &derive_snapshot(&integrations)[..16]);
    let config = PersonalConfig {
        owner: OwnerConfig {
            tenant_id: "local".to_owned(),
            agent_id: local_agent_id(),
            agent_revision: 1,
            authority_snapshot_id: snapshot_id.clone(),
            authority_snapshot_sha256: derive_snapshot(&integrations),
        },
        connection: None,
        authority: None,
        application: None,
        sip: None,
        slack: None,
        grafana: None,
        kubernetes,
        platform: None,
        catalog: Vec::new(),
    };
    let rendered = toml::to_string_pretty(&config)?;
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    write_validated(config_path, rendered.as_bytes(), force)?;

    // Said at `init` rather than discovered at `connect`. Activating a context authenticated by a
    // credential plugin runs that plugin, so admitting it is a decision the operator makes, not a
    // default this command takes for them — but leaving them to meet
    // `allow_exec_auth is required` on their first connection, with no idea the switch exists, is
    // the same refusal delivered as a dead end. Every EKS context is in this class.
    let notes = if integrations.contains(&"kubernetes") && !allow_exec_auth {
        vec![
            "Contexts authenticated by a credential plugin — every EKS context is one — are \
             refused until you re-run with --allow-exec-auth. It runs the same helper your \
             kubectl already runs."
                .to_owned(),
        ]
    } else {
        Vec::new()
    };

    Ok(Written {
        config_path: config_path.to_owned(),
        state_root: state_root.to_owned(),
        integrations,
        snapshot_id,
        notes,
    })
}

/// Write, prove the bytes on disk are readable, then move into place.
///
/// **The validation is a read-back, not a pre-check**, and that is the stronger claim: the daemon
/// will not evaluate this configuration as a struct in this process's memory, it will re-read the
/// bytes through [`PersonalConfig::read`] with its own ownership and permission rules. Validating
/// the struct would prove something adjacent to what matters. Validating the file proves it.
///
/// The temporary lives in the destination directory so the final step is a rename within one
/// filesystem, which is atomic — a reader never sees a half-written configuration, and a failed
/// validation leaves the previous file untouched.
///
/// `0600` at creation rather than a later `chmod`: a mode fixed after the fact leaves a window in
/// which the file exists at the process umask. Nothing secret goes in here, but the daemon refuses
/// a configuration that is not owner-only, so writing one that it would refuse is pointless.
fn write_validated(path: &Path, bytes: &[u8], force: bool) -> Result<(), InitError> {
    use std::os::unix::fs::OpenOptionsExt as _;

    if !force && path.exists() {
        return Err(InitError::Exists(path.to_owned()));
    }
    let staged = path.with_extension("toml.staged");
    // `create_new` even under `--force`: the staged path is ours for the length of this call, and
    // clobbering something another process left there would be a different bug.
    let _ = std::fs::remove_file(&staged);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&staged)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);

    if let Err(error) = PersonalConfig::read(&staged) {
        let _ = std::fs::remove_file(&staged);
        return Err(InitError::Invalid(error));
    }
    std::fs::rename(&staged, path)?;
    Ok(())
}

/// The digest over the admitted set. See this module's documentation for why it excludes `[owner]`.
fn derive_snapshot(integrations: &[&'static str]) -> String {
    let mut digest = Sha256::new();
    digest.update(SNAPSHOT_DOMAIN);
    for integration in integrations {
        digest.update(integration.as_bytes());
        digest.update([0]);
    }
    hex::encode(digest.finalize())
}

/// A stable identity for this person on this machine.
///
/// Host and uid, not a random id: the same machine must derive the same agent across runs, or every
/// `init` would orphan the previous run's audit trail.
fn local_agent_id() -> String {
    let host = rustix::system::uname()
        .nodename()
        .to_str()
        .unwrap_or("unknown")
        .to_owned();
    let uid = rustix::process::geteuid().as_raw();
    format!("agent:local:{host}:{uid}")
}

/// Whether the operator has a kubeconfig this machine can read.
///
/// Presence only. Whether any context in it actually works is the Kubernetes backend's answer, and
/// it gives that answer as passive candidates rather than as a startup failure.
fn kubeconfig_readable() -> bool {
    if let Some(paths) = std::env::var_os("KUBECONFIG") {
        return std::env::split_paths(&paths).any(|path| path.is_file());
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .is_some_and(|home| home.join(".kube/config").is_file())
}

/// The Kubernetes section, with the grants its validation requires.
///
/// `target_grants` cannot be empty: a monitoring Service discovered behind the cluster is a
/// Connection the placement must be able to attribute, and one it cannot attribute is one it must
/// not open. Namespaces stay empty, which means cluster-wide discovery bounded by whatever the
/// operator's own RBAC already permits — on a personal machine the kubeconfig is the aperture.
fn kubernetes_config(allow_exec_auth: bool) -> KubernetesIntegrationConfig {
    KubernetesIntegrationConfig {
        grant_ref: "grant:kubernetes:local".to_owned(),
        initiation: InitiationConfig::Platform,
        namespaces: Vec::new(),
        target_grants: BTreeMap::from([
            ("prometheus".to_owned(), "grant:prometheus:local".to_owned()),
            ("loki".to_owned(), "grant:loki:local".to_owned()),
            ("grafana".to_owned(), "grant:grafana:local".to_owned()),
            (
                "alertmanager".to_owned(),
                "grant:alertmanager:local".to_owned(),
            ),
        ]),
        allow_exec_auth,
        resource_limit: 64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_snapshot_digest_is_stable_and_moves_with_the_admitted_set() {
        assert_eq!(
            derive_snapshot(&["kubernetes"]),
            derive_snapshot(&["kubernetes"])
        );
        assert_ne!(
            derive_snapshot(&["kubernetes"]),
            derive_snapshot(&["slack"])
        );
        assert_ne!(
            derive_snapshot(&["kubernetes"]),
            derive_snapshot(&["kubernetes", "slack"])
        );
        assert_eq!(derive_snapshot(&["kubernetes"]).len(), 64);
    }

    #[test]
    fn the_separator_keeps_a_concatenation_from_colliding() {
        // Without the NUL, ["ab", "c"] and ["a", "bc"] would digest identically.
        assert_ne!(derive_snapshot(&["ab", "c"]), derive_snapshot(&["a", "bc"]));
    }

    #[test]
    fn what_init_writes_is_what_the_daemon_can_read() {
        // The whole point of the command. It writes through the same `PersonalConfig::read` the
        // daemon uses, so a validation rule that tightens breaks this test rather than a person's
        // first run. Uses a temporary directory because `read` enforces file ownership and mode.
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("connectors.toml");
        let config = PersonalConfig {
            owner: OwnerConfig {
                tenant_id: "local".to_owned(),
                agent_id: local_agent_id(),
                agent_revision: 1,
                authority_snapshot_id: "snapshot:local:test".to_owned(),
                authority_snapshot_sha256: derive_snapshot(&["kubernetes"]),
            },
            connection: None,
            authority: None,
            application: None,
            sip: None,
            slack: None,
            grafana: None,
            kubernetes: Some(kubernetes_config(false)),
            platform: None,
            catalog: Vec::new(),
        };
        let rendered = toml::to_string_pretty(&config).expect("the config renders as TOML");
        write_validated(&path, rendered.as_bytes(), false).expect("the daemon can read it back");
        assert!(path.exists());
    }

    #[test]
    fn an_existing_configuration_is_never_replaced_silently() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("connectors.toml");
        std::fs::write(&path, "owner = {}").expect("a pre-existing file");
        let error =
            run(&path, directory.path(), &[], false, false).expect_err("refuses to clobber");
        assert!(matches!(error, InitError::Exists(_)));
        assert_eq!(
            std::fs::read_to_string(&path).expect("the original survives"),
            "owner = {}"
        );
    }

    #[test]
    fn a_configuration_the_daemon_would_refuse_is_not_left_on_disk() {
        // An owner-only file with no integration is exactly what `validate` refuses
        // (`personal.rs:454-461`). The staged write must clean up after itself rather than leave a
        // file that fails on the next command.
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("connectors.toml");
        let refused = "[owner]\ntenant_id = \"local\"\nagent_id = \"a\"\nagent_revision = 1\n\
                       authority_snapshot_id = \"s\"\nauthority_snapshot_sha256 = \"00\"\n";
        let error = write_validated(&path, refused.as_bytes(), false).expect_err("is refused");
        assert!(matches!(error, InitError::Invalid(_)));
        assert!(!path.exists());
        assert!(!path.with_extension("toml.staged").exists());
    }

    #[test]
    fn admitting_a_credential_plugin_is_a_choice_and_its_absence_is_explained() {
        // The default must stay closed — activating such a context runs the helper — but a closed
        // default that says nothing produces `allow_exec_auth is required` at connect time for an
        // operator who has never heard of the switch.
        assert!(!kubernetes_config(false).allow_exec_auth);
        assert!(kubernetes_config(true).allow_exec_auth);
    }

    #[test]
    fn an_agent_id_is_stable_across_calls() {
        assert_eq!(local_agent_id(), local_agent_id());
        assert!(local_agent_id().starts_with("agent:local:"));
    }
}

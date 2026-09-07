//! The guided provider flows: one hidden prompt, one one-use socket, one Connection.
//!
//! # What changed when this moved out of the binary
//!
//! These flows used to `println!` their way through. That read well on a terminal and meant
//! `connectors connect kubernetes -o json` silently ignored the format it was given — the one
//! command a first-run script most wants to read. Every flow here now **returns its outcome as
//! data**, and the frontend renders it in whichever format the caller asked for. The prompt itself
//! still goes to the terminal, because a prompt has nowhere else to go.
//!
//! # The credential path is the point
//!
//! A token is typed with echo disabled and written straight to a Connector-issued one-use socket,
//! validated by [`CompletionEndpoint::validate`] before a byte is sent — it must live directly
//! below this state root's `connect-sessions`, be a socket rather than a symlink, and be reachable
//! by nobody but its owner. It is never an argument, never an environment variable, never a file.
//! Nothing in this module can print it: the value is held in a [`Zeroizing`] buffer and moves once.

use std::path::Path;

use connectors_client::{CandidateActivationOutcome, ClientError, CompletionEndpoint, LocalClient};
use connectors_config::PersonalConfig;
use serde_json::{json, Value};
use zeroize::Zeroizing;

/// The providers the guided flow can complete today.
///
/// Short because each one needs hand-written prompt text and a hand-written outcome. The generic
/// catalog-driven flow is what removes this list: a provider's declared credentials and
/// configuration fields already describe what to ask for.
const GUIDED: [&str; 3] = ["slack", "grafana", "kubernetes"];

/// Which flow a provider uses, and why.
///
/// The guided flow completes a **Connect Session** against a running Connector — how a credential
/// the provider itself issues (a Socket Mode app token, an OAuth grant) arrives without passing
/// through a caller. It is only reachable when that curated Integration is actually **composed**,
/// which for Slack and Grafana means the configuration declares its section.
///
/// So the test is not "is this one of three names" but "is its curated backend there to answer".
/// Deciding on the name alone sent `connect slack` into a Connect Session against a daemon with no
/// Slack backend, which refused with `no Integration owns this Connection request` — a true
/// statement that tells an operator nothing about what to do. A catalogued provider whose curated
/// Integration is absent is connected from its declarations instead, which works.
#[must_use]
pub fn is_guided(provider: &str, config: &PersonalConfig) -> bool {
    match provider {
        "slack" => config.slack.is_some(),
        "grafana" => config.grafana.is_some(),
        // Kubernetes has no catalogued surface at all — it is not in the catalogue — so its guided
        // flow is the only way in, configured or not. Saying so beats falling through to a
        // catalogue lookup that can only fail.
        "kubernetes" => true,
        _ => false,
    }
}

/// Run whichever flow this provider uses.
///
/// # Errors
///
/// Whatever the selected flow refuses with.
pub async fn dispatch(
    provider: &str,
    config: &PersonalConfig,
    config_path: &Path,
    state_root: &Path,
    label: Option<String>,
    context: Option<String>,
    options: crate::enrol::Options,
) -> Result<Value, ConnectError> {
    if is_guided(provider, config) {
        return run(provider, config, state_root, label, context).await;
    }
    Ok(crate::enrol::run(provider, config_path, state_root, options).await?)
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("personal OAuth needs exactly one configured binding and its declared profile")]
    PersonalOAuthConfiguration,
    #[error(
        "personal OAuth needs a controlling terminal or a new owner-only instruction file in a private directory"
    )]
    PrivateInstructionsRequired,
    #[error(transparent)]
    Enrol(#[from] crate::enrol::EnrolError),
    #[error("the guided connection flow does not support provider `{0}` yet")]
    Unsupported(String),
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error("the credential could not be read from the terminal: {0}")]
    Prompt(#[from] std::io::Error),
    #[error("the Connector returned an invalid connection response")]
    InvalidResponse,
    #[error(transparent)]
    Refused(#[from] crate::envelope::ReducedError),
}

/// Run one guided flow and return what happened, without printing it.
///
/// # Errors
///
/// [`ConnectError::Unsupported`] for a provider outside [`GUIDED`], or the underlying client,
/// terminal or protocol failure.
pub async fn run(
    provider: &str,
    config: &PersonalConfig,
    state_root: &Path,
    label: Option<String>,
    context: Option<String>,
) -> Result<Value, ConnectError> {
    if !GUIDED.contains(&provider) {
        return Err(ConnectError::Unsupported(provider.to_owned()));
    }
    let client = LocalClient::new(state_root.join("connectors.sock"));
    let owner = config.owner_context();

    if provider == "kubernetes" {
        return kubernetes(&client, &owner, label, context).await;
    }

    let display_name = if provider == "slack" {
        "Slack"
    } else {
        "Grafana"
    };
    let label = label.unwrap_or_else(|| display_name.to_owned());
    // Straight to the terminal, and deliberately not part of the returned value: this is the
    // instruction that makes the next line safe to type, and it is worthless in a JSON document.
    eprintln!("Connect {display_name}");
    eprintln!("Input is hidden and sent only to the local Connector.");
    let pending = client
        .begin_connect_session(&owner, provider.to_owned(), label)
        .await?;
    let credential_prompt = if provider == "slack" {
        "Slack app token: "
    } else {
        "Grafana service account token: "
    };
    submit_credential(state_root, &pending.completion_endpoint, credential_prompt).await?;
    let description = client
        .finish_connect_session(&owner, pending.session_ref)
        .await?;

    if provider == "slack" {
        let channel = description
            .channels
            .first()
            .ok_or(ConnectError::InvalidResponse)?;
        return Ok(json!({
            "provider": "slack",
            "connected": true,
            "connection": description.summary.label,
            "connection_ref": description.summary.connection_ref,
            "events": channel.events,
        }));
    }

    discovered(
        &client,
        &owner,
        "grafana",
        description.summary.connection_ref,
    )
    .await
}

/// Kubernetes needs no credential — it reads the operator's own kubeconfig — so its flow either
/// activates a named context or reports the ones it detected.
async fn kubernetes(
    client: &LocalClient,
    owner: &protocol::operation::OwnerContext,
    label: Option<String>,
    context: Option<String>,
) -> Result<Value, ConnectError> {
    let outcome = client
        .activate_candidate(owner, "kubernetes".to_owned(), label, context)
        .await?;
    let CandidateActivationOutcome::Connected {
        connection,
        observations: _,
    } = outcome
    else {
        let CandidateActivationOutcome::SelectionRequired(candidates) = outcome else {
            unreachable!("the outcome is one of two variants")
        };
        return Ok(json!({
            "provider": "kubernetes",
            "connected": false,
            // The next command, in the payload rather than only on the terminal, so a caller
            // reading JSON is told what to do rather than left with an empty result.
            "next": "connectors setup connect kubernetes --context <name>",
            "contexts": candidates.iter().map(|candidate| candidate.title.clone()).collect::<Vec<_>>(),
        }));
    };
    discovered(
        client,
        owner,
        "kubernetes",
        connection.summary.connection_ref,
    )
    .await
}

async fn discovered(
    client: &LocalClient,
    owner: &protocol::operation::OwnerContext,
    provider: &str,
    source_ref: String,
) -> Result<Value, ConnectError> {
    use protocol::endpoint::{EndpointRequest, ListRequest, RefreshRequest};
    let refreshed = client
        .endpoint(
            owner,
            EndpointRequest::Refresh(RefreshRequest {
                source_ref: Some(source_ref.clone()),
            }),
        )
        .await?;
    let refreshed = crate::reduce_envelope!(refreshed)?;
    let endpoints = client
        .endpoint(
            owner,
            EndpointRequest::List(ListRequest {
                source_ref: Some(source_ref.clone()),
                query: String::new(),
                limit: protocol::endpoint::MAX_RESULTS,
                cursor: None,
            }),
        )
        .await?;
    let endpoints = crate::reduce_envelope!(endpoints)?;
    Ok(json!({
        "provider": provider, "connected": true, "source_ref": source_ref,
        "endpoints": endpoints["endpoints"], "next_cursor": endpoints["next_cursor"],
        "warnings": refreshed["warnings"],
        "next": "connectors endpoint list",
    }))
}

/// Prompt with echo disabled and hand the value to the validated one-use endpoint.
async fn submit_credential(
    state_root: &Path,
    completion_endpoint: &Path,
    prompt: &str,
) -> Result<(), ConnectError> {
    let endpoint = CompletionEndpoint::validate(state_root, completion_endpoint)?;
    let token = Zeroizing::new(rpassword::prompt_password(prompt)?);
    endpoint.submit(token.as_bytes()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A configuration declaring no curated Integration — the ordinary state of a fresh install.
    fn minimal_config() -> PersonalConfig {
        let toml = "[owner]\ntenant_id = \"local\"\nagent_id = \"a\"\nagent_revision = 1\n\
                    authority_snapshot_id = \"s\"\nauthority_snapshot_sha256 = \"00\"\n";
        toml::from_str(toml).expect("a minimal owner-only configuration parses")
    }

    #[test]
    fn a_catalogued_provider_whose_curated_backend_is_absent_takes_the_catalogue_path() {
        // The case that produced `no Integration owns this Connection request`: Slack is a guided
        // name, but with no `[slack]` section there is no backend to hold a Connect Session, and
        // the catalogue declares eight Slack operations that a bot token can serve today.
        let config = minimal_config();
        assert!(!is_guided("slack", &config));
        assert!(!is_guided("grafana", &config));
        // Kubernetes is not catalogued, so there is no other path to fall through to.
        assert!(is_guided("kubernetes", &config));
    }

    #[test]
    fn a_provider_outside_the_guided_set_is_refused_by_name() {
        // Refusing before any socket is opened is what keeps `connectors connect gitlab` from
        // looking like a transport failure when it is really an unimplemented flow.
        assert!(!GUIDED.contains(&"gitlab"));
        assert!(GUIDED.contains(&"slack"));
        assert!(GUIDED.contains(&"grafana"));
        assert!(GUIDED.contains(&"kubernetes"));
    }

    #[test]
    fn the_error_for_an_unknown_provider_names_it() {
        let error = ConnectError::Unsupported("gitlab".to_owned());
        assert!(error.to_string().contains("gitlab"));
    }
}

/// Select a configured OAuth registration before any session or human instruction is created.
/// Existing raw enrollment remains available when the caller did not select a deployed OAuth purpose.
pub async fn dispatch_with_personal_oauth(
    provider: &str,
    config: &PersonalConfig,
    config_path: &Path,
    state_root: &Path,
    label: Option<String>,
    context: Option<String>,
    oauth: PersonalOAuthOptions,
) -> Result<Value, ConnectError> {
    let options = oauth.enrol;
    let configured: Vec<_> = config
        .catalog
        .iter()
        .filter(|entry| entry.provider == provider && entry.oauth.is_some())
        .collect();
    let selected = oauth
        .auth_profile
        .as_deref()
        .or(options.credential.as_deref());
    let matches: Vec<_> = configured
        .iter()
        .copied()
        .filter(|entry| {
            selected.is_none_or(|profile| {
                entry
                    .oauth
                    .as_ref()
                    .is_some_and(|registration| registration.auth_profile == profile)
            })
        })
        .collect();
    if oauth.auth_profile.is_none()
        && oauth.instruction_file.is_none()
        && (configured.is_empty() || selected.is_some() && matches.is_empty())
    {
        return dispatch(
            provider,
            config,
            config_path,
            state_root,
            label,
            context,
            options,
        )
        .await;
    }
    let [entry] = matches.as_slice() else {
        return Err(ConnectError::PersonalOAuthConfiguration);
    };
    if context.is_some()
        || !options.values.is_empty()
        || options.allow_writes
        || options.operator_network
        || options.force
        || options.credential_file.is_some()
        || options.instance.is_some()
    {
        return Err(ConnectError::PersonalOAuthConfiguration);
    }
    let registration = entry
        .oauth
        .as_ref()
        .ok_or(ConnectError::PersonalOAuthConfiguration)?;
    registration
        .validate()
        .map_err(|_| ConnectError::PersonalOAuthConfiguration)?;
    let principal = config
        .principal_context()
        .map_err(|_| ConnectError::PersonalOAuthConfiguration)?;
    let origins = integration_catalog::personal_oauth_admitted_origins(&principal, entry)
        .map_err(|_| ConnectError::PersonalOAuthConfiguration)?;
    let [origin] = origins.as_slice() else {
        return Err(ConnectError::PersonalOAuthConfiguration);
    };
    let expected_connection_ref =
        integration_catalog::personal_oauth_admitted_connection_ref(&principal, entry)
            .map_err(|_| ConnectError::PersonalOAuthConfiguration)?;
    let display_label = label.unwrap_or_else(|| entry.label());
    let mut destination = PrivateInstructionDestination::open(oauth.instruction_file.as_deref())?;
    let client = LocalClient::new(state_root.join("connectors.sock"));
    let pending = client
        .begin_personal_oauth(
            &config.owner_context(),
            provider.to_owned(),
            display_label.clone(),
            registration.auth_profile.clone(),
            expected_connection_ref,
        )
        .await?;
    let instructions = client.personal_oauth_instructions(&pending, origin).await?;
    instructions
        .write_human(&mut destination.file)
        .map_err(ConnectError::Prompt)?;
    drop(instructions);
    let owner = config.owner_context();
    let completion = client.finish_personal_oauth(&owner, &pending);
    tokio::pin!(completion);
    let connection = tokio::select! {
        result = &mut completion => result?,
        _ = pending.instruction_expiry() => {
            destination.clear()?;
            completion.await?
        }
    };
    destination.clear()?;
    Ok(json!({ "provider": provider, "connected": true,
        "connection_ref": connection.summary.connection_ref,
        "connection": display_label }))
}

/// Only the selected profile and private output path cross the CLI/console boundary.
#[derive(Default)]
pub struct PersonalOAuthOptions {
    pub enrol: crate::enrol::Options,
    pub auth_profile: Option<String>,
    pub instruction_file: Option<std::path::PathBuf>,
}

pub(crate) struct PrivateInstructionDestination {
    pub(crate) file: std::fs::File,
    path: Option<std::path::PathBuf>,
    identity: Option<(u64, u64)>,
}
impl PrivateInstructionDestination {
    pub(crate) fn open(path: Option<&Path>) -> Result<Self, ConnectError> {
        use std::io::IsTerminal as _;
        use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _};
        let Some(path) = path else {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")
                .map_err(|_| ConnectError::PrivateInstructionsRequired)?;
            if !file.is_terminal() {
                return Err(ConnectError::PrivateInstructionsRequired);
            }
            return Ok(Self {
                file,
                path: None,
                identity: None,
            });
        };
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let parent = parent
            .canonicalize()
            .map_err(|_| ConnectError::PrivateInstructionsRequired)?;
        let metadata =
            std::fs::metadata(&parent).map_err(|_| ConnectError::PrivateInstructionsRequired)?;
        if !metadata.is_dir()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o077 != 0
        {
            return Err(ConnectError::PrivateInstructionsRequired);
        }
        let path = parent.join(
            path.file_name()
                .ok_or(ConnectError::PrivateInstructionsRequired)?,
        );
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(|_| ConnectError::PrivateInstructionsRequired)?;
        let metadata = file
            .metadata()
            .map_err(|_| ConnectError::PrivateInstructionsRequired)?;
        if !metadata.is_file()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o077 != 0
            || metadata.nlink() != 1
        {
            return Err(ConnectError::PrivateInstructionsRequired);
        }
        Ok(Self {
            file,
            path: Some(path),
            identity: Some((metadata.dev(), metadata.ino())),
        })
    }
    pub(crate) fn clear(&mut self) -> Result<(), ConnectError> {
        use std::os::unix::fs::MetadataExt as _;
        let Some(path) = self.path.as_ref() else {
            return Ok(());
        };
        self.file.set_len(0).map_err(ConnectError::Prompt)?;
        self.file.sync_all().map_err(ConnectError::Prompt)?;
        match std::fs::symlink_metadata(path) {
            Ok(metadata)
                if Some((metadata.dev(), metadata.ino())) == self.identity
                    && metadata.is_file() =>
            {
                std::fs::remove_file(path).map_err(ConnectError::Prompt)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err(ConnectError::PrivateInstructionsRequired),
        }
        self.path = None;
        Ok(())
    }
}
impl Drop for PrivateInstructionDestination {
    fn drop(&mut self) {
        let _ = self.clear();
    }
}

#[cfg(test)]
mod personal_oauth_tests {
    use super::*;
    use std::io::Write as _;
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};

    #[test]
    fn instruction_file_is_exclusive_owner_only_and_cleared_on_drop() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            directory.path().metadata().unwrap().mode() & 0o777,
            0o700,
            "the fixture must supply a private directory"
        );
        let path = directory.path().join("instructions");
        {
            let mut destination = PrivateInstructionDestination::open(Some(&path)).unwrap();
            destination.file.write_all(b"PRIVATE-SENTINEL").unwrap();
            let metadata = path.metadata().unwrap();
            assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
            assert_eq!(metadata.uid(), rustix::process::geteuid().as_raw());
            assert!(PrivateInstructionDestination::open(Some(&path)).is_err());
        }
        assert!(!path.exists());
    }

    #[test]
    fn instruction_file_refuses_shared_parent_symlink_and_existing_content() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let existing = directory.path().join("existing");
        std::fs::write(&existing, "original").unwrap();
        let link = directory.path().join("link");
        std::os::unix::fs::symlink(&existing, &link).unwrap();
        assert!(PrivateInstructionDestination::open(Some(&link)).is_err());
        assert!(PrivateInstructionDestination::open(Some(&existing)).is_err());
        assert_eq!(std::fs::read_to_string(&existing).unwrap(), "original");
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(PrivateInstructionDestination::open(Some(&directory.path().join("new"))).is_err());
    }

    #[test]
    fn instruction_cleanup_never_removes_a_replacement_inode() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.path().join("instructions");
        let mut destination = PrivateInstructionDestination::open(Some(&path)).unwrap();
        destination.file.write_all(b"PRIVATE-SENTINEL").unwrap();
        std::fs::rename(&path, directory.path().join("moved")).unwrap();
        std::fs::write(&path, "replacement").unwrap();
        assert!(destination.clear().is_err());
        drop(destination);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "replacement");
        assert!(std::fs::read(directory.path().join("moved"))
            .unwrap()
            .is_empty());
    }
}

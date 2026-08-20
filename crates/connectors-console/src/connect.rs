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

    let display_name = if provider == "slack" { "Slack" } else { "Grafana" };
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

    let observations = client
        .observations(&owner, description.summary.connection_ref.clone())
        .await?;
    let materialized = client.materialize_admitted(&owner, observations).await?;
    Ok(json!({
        "provider": "grafana",
        "connected": true,
        "connection": description.summary.label,
        "connection_ref": description.summary.connection_ref,
        "targets": materialized.connections.iter().map(|target| json!({
            "label": target.label,
            "integration_ref": target.integration_ref,
            "connection_ref": target.connection_ref,
        })).collect::<Vec<_>>(),
        // Reported rather than dropped: a data source that was seen and not connected is the
        // question an operator asks next, and silence about it reads as "there were none".
        "unsupported": materialized.unsupported,
        "not_granted": materialized.not_granted,
    }))
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
        observations,
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
            "next": "connectors connect kubernetes --context <name>",
            "contexts": candidates.iter().map(|candidate| candidate.title.clone()).collect::<Vec<_>>(),
        }));
    };
    Ok(json!({
        "provider": "kubernetes",
        "connected": true,
        "connection_ref": connection.summary.connection_ref,
        "observations": observations.iter().map(|observation| json!({
            "title": observation.title,
            "observation_ref": observation.observation_ref,
        })).collect::<Vec<_>>(),
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

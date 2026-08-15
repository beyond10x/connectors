#![forbid(unsafe_code)]

use std::io;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use connectors_client::{CandidateActivationOutcome, CompletionEndpoint, LocalClient};
use connectors_runtime::{
    default_config_path, default_state_root, validate_state_root, HostedRuntime, PersonalConfig,
    PersonalRuntime, RuntimeError,
};
use protocol::connection::{
    CandidateActivateRequest, CandidateSearchRequest, ConnectionRequest, MaterializeRequest,
    ObservationSearchRequest, SearchRequest as ConnectionSearchRequest,
};
use protocol::event::{
    EventRequest, ReceiveRequest, ReplayRequest, SearchRequest as EventSearchRequest,
};
use protocol::operation::{
    DescribeRequest as OperationDescribeRequest, InvokeRequest as OperationInvokeRequest,
    OperationRequest, SearchRequest as OperationSearchRequest,
};
use zeroize::Zeroizing;

#[derive(Debug, Parser)]
#[command(name = "connectors", version, about = "B10x Connectors service")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Serve the owner-permissioned personal-local Connector protocols.
    Serve {
        /// Strict value-free deployment configuration.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Owner-only state root. Defaults below XDG_STATE_HOME (or ~/.local/state).
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Serve the Identity-authenticated hosted Operation and Connection APIs.
    ServeHosted {
        /// Strict value-free server and Integration configuration.
        #[arg(long)]
        config: PathBuf,
    },
    /// Add a provider through one guided, secret-safe flow.
    Connect {
        /// Provider to add. The personal-local alpha supports `slack`, `grafana`, and `kubernetes`.
        provider: String,
        /// Strict value-free deployment configuration.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Human label for the resulting Connection.
        #[arg(long)]
        label: Option<String>,
        /// Exact detected kubeconfig context when connecting Kubernetes.
        #[arg(long)]
        context: Option<String>,
        /// Owner-only state root used by the running Connector.
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Manage durable Connections through the credential-free control socket.
    Connection {
        #[command(subcommand)]
        command: ConnectionCommand,
    },
    /// Search or receive durable normalized data events.
    Event {
        #[command(subcommand)]
        command: EventCommand,
    },
    /// Search, describe, or invoke admitted Connector operations.
    Operation {
        #[command(subcommand)]
        command: OperationCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ConnectionCommand {
    /// Passively list potential direct Connections without contacting their providers.
    Candidates {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        integration: String,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long, default_value_t = 64)]
        limit: u16,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Explicitly contact and activate one opaque direct-Connection candidate.
    Activate {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        candidate: String,
        #[arg(long)]
        label: String,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// List non-secret Connection summaries.
    List {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long, default_value_t = 64)]
        limit: u16,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// List the latest stored discovery observations for a source Connection.
    Observations {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        source: String,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long, default_value_t = 64)]
        limit: u16,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Materialize one recognized observation as a callable mediated Connection.
    Materialize {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        observation: String,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum OperationCommand {
    /// List currently callable operations and their admitted Connections.
    Search {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long, default_value_t = 25)]
        limit: u16,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Read a fresh operation description and its opaque lease.
    Describe {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Invoke an operation using a fresh description lease.
    Invoke {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        connection: String,
        #[arg(long)]
        description_ref: String,
        /// Strict JSON object containing only catalog-declared caller input.
        #[arg(long)]
        input_json: String,
        #[arg(long)]
        approval_evidence_ref: Option<String>,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum EventCommand {
    /// List admitted Connector channels.
    Search {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long, default_value_t = 64)]
        limit: u16,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Pull durable events, optionally waiting for the next one.
    Receive {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        channel: String,
        #[arg(long)]
        after: Option<String>,
        #[arg(long, default_value_t = 25)]
        limit: u16,
        #[arg(long, default_value_t = 30_000)]
        wait_ms: u32,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Replay one stored event by its opaque Connector event reference.
    Replay {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        event: String,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
}

#[derive(Debug, thiserror::Error)]
enum MainError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Config(#[from] connectors_runtime::ConfigError),
    #[error(transparent)]
    Client(#[from] connectors_client::ClientError),
    #[error("local Connector request failed: {0}")]
    Io(#[from] io::Error),
    #[error("local Connector response was malformed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the guided connection flow does not support provider `{0}` yet")]
    UnsupportedProvider(String),
    #[error("the Connector returned an invalid connection response")]
    InvalidConnectionResponse,
}

#[tokio::main]
async fn main() -> Result<(), MainError> {
    match Cli::parse().command {
        Command::Serve { config, state_root } => serve(config, state_root).await,
        Command::ServeHosted { config } => serve_hosted(&config).await,
        Command::Connect {
            provider,
            config,
            label,
            context,
            state_root,
        } => connect(provider, config, label, context, state_root).await,
        Command::Connection { command } => connection(command).await,
        Command::Event { command } => event(command).await,
        Command::Operation { command } => operation(command).await,
    }
}

async fn serve_hosted(config_path: &Path) -> Result<(), MainError> {
    let runtime = HostedRuntime::bind(config_path).await?;
    println!("{}", runtime.readiness());
    runtime
        .serve_until(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

async fn connect(
    provider: String,
    config_path: Option<PathBuf>,
    label: Option<String>,
    context: Option<String>,
    state_root: Option<PathBuf>,
) -> Result<(), MainError> {
    if !matches!(provider.as_str(), "slack" | "grafana" | "kubernetes") {
        return Err(MainError::UnsupportedProvider(provider));
    }
    let config_path = config_path.map_or_else(default_config_path, Ok)?;
    let config = PersonalConfig::read(&config_path)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    validate_state_root(&state_root)?;
    let client = LocalClient::new(state_root.join("connectors.sock"));
    let owner = config.owner_context();

    if provider == "kubernetes" {
        return connect_kubernetes(&client, &owner, label, context).await;
    }

    let display_name = match provider.as_str() {
        "slack" => "Slack",
        "grafana" => "Grafana",
        _ => unreachable!("provider was validated"),
    };
    let label = label.unwrap_or_else(|| display_name.to_owned());
    println!("Connect {display_name}");
    println!("Input is hidden and sent only to the local Connector.");
    let pending = client
        .begin_connect_session(&owner, provider.clone(), label)
        .await?;
    let credential_prompt = match provider.as_str() {
        "slack" => "Slack app token: ",
        "grafana" => "Grafana service account token: ",
        _ => unreachable!("provider was validated"),
    };
    prompt_and_submit_completion(&state_root, &pending.completion_endpoint, credential_prompt)
        .await?;
    let description = client
        .finish_connect_session(&owner, pending.session_ref)
        .await?;
    println!();
    if provider == "slack" {
        let channel = description
            .channels
            .first()
            .ok_or(MainError::InvalidConnectionResponse)?;
        println!("Slack is connected and ready to receive messages.");
        println!("Connection: {}", description.summary.label);
        println!("Events: {}", channel.events.join(", "));
    } else {
        let observations = client
            .observations(&owner, description.summary.connection_ref.clone())
            .await?;
        let materialized = client.materialize_admitted(&owner, observations).await?;
        println!("Grafana is connected and its data sources are ready.");
        println!("Connection: {}", description.summary.label);
        for target in &materialized.connections {
            println!(
                "Target: {} ({}) -> {}",
                target.label, target.integration_ref, target.connection_ref
            );
        }
        if materialized.unsupported > 0 {
            println!(
                "Observed but unsupported data sources: {}",
                materialized.unsupported
            );
        }
        if materialized.not_granted > 0 {
            println!(
                "Recognized data sources without a configured target Grant: {}",
                materialized.not_granted
            );
        }
    }
    Ok(())
}

async fn connect_kubernetes(
    client: &LocalClient,
    owner: &protocol::operation::OwnerContext,
    label: Option<String>,
    context: Option<String>,
) -> Result<(), MainError> {
    let outcome = client
        .activate_candidate(owner, "kubernetes".to_owned(), label, context)
        .await?;
    let CandidateActivationOutcome::Connected {
        connection,
        observations,
    } = outcome
    else {
        let CandidateActivationOutcome::SelectionRequired(candidates) = outcome else {
            unreachable!()
        };
        if candidates.is_empty() {
            println!("No kubeconfig contexts were detected.");
        } else {
            println!("Detected kubeconfig contexts:");
            for candidate in candidates {
                println!("  {}", candidate.title);
            }
            println!();
            println!("Choose one with: connectors connect kubernetes --context <name>");
        }
        return Ok(());
    };
    println!(
        "Kubernetes is connected: {}",
        connection.summary.connection_ref
    );
    if observations.is_empty() {
        println!("No supported monitoring Services were visible in the admitted namespace scope.");
    } else {
        println!("Discovered monitoring Services:");
        for observation in observations {
            println!("  {} -> {}", observation.title, observation.observation_ref);
        }
    }
    Ok(())
}

async fn serve(config_path: Option<PathBuf>, state_root: Option<PathBuf>) -> Result<(), MainError> {
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    let runtime = PersonalRuntime::bind(config_path.as_deref(), state_root).await?;
    println!("{}", runtime.readiness());
    runtime
        .serve_until(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

async fn connection(command: ConnectionCommand) -> Result<(), MainError> {
    let (config_path, state_root, request) = match command {
        ConnectionCommand::Candidates {
            config,
            integration,
            query,
            limit,
            state_root,
        } => (
            config,
            state_root,
            ConnectionRequest::CandidateSearch(CandidateSearchRequest {
                integration_ref: integration,
                query,
                limit,
            }),
        ),
        ConnectionCommand::Activate {
            config,
            candidate,
            label,
            state_root,
        } => (
            config,
            state_root,
            ConnectionRequest::CandidateActivate(CandidateActivateRequest {
                candidate_ref: candidate,
                label,
            }),
        ),
        ConnectionCommand::List {
            config,
            query,
            limit,
            state_root,
        } => (
            config,
            state_root,
            ConnectionRequest::Search(ConnectionSearchRequest { query, limit }),
        ),
        ConnectionCommand::Observations {
            config,
            source,
            query,
            limit,
            state_root,
        } => (
            config,
            state_root,
            ConnectionRequest::ObservationSearch(ObservationSearchRequest {
                source_connection_ref: source,
                query,
                limit,
            }),
        ),
        ConnectionCommand::Materialize {
            config,
            observation,
            state_root,
        } => (
            config,
            state_root,
            ConnectionRequest::Materialize(MaterializeRequest {
                observation_ref: observation,
            }),
        ),
    };
    let config = PersonalConfig::read(&config_path)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    validate_state_root(&state_root)?;
    let response = LocalClient::new(state_root.join("connectors.sock"))
        .connection(&config.owner_context(), request)
        .await?;
    print_response(serde_json::to_value(response)?)?;
    Ok(())
}

async fn event(command: EventCommand) -> Result<(), MainError> {
    let (config_path, state_root, request) = match command {
        EventCommand::Search {
            config,
            query,
            limit,
            state_root,
        } => (
            config,
            state_root,
            EventRequest::Search(EventSearchRequest { query, limit }),
        ),
        EventCommand::Receive {
            config,
            channel,
            after,
            limit,
            wait_ms,
            state_root,
        } => (
            config,
            state_root,
            EventRequest::Receive(ReceiveRequest {
                channel_ref: channel,
                after,
                limit,
                wait_ms,
            }),
        ),
        EventCommand::Replay {
            config,
            event,
            state_root,
        } => (
            config,
            state_root,
            EventRequest::Replay(ReplayRequest { event_ref: event }),
        ),
    };
    let config = PersonalConfig::read(&config_path)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    validate_state_root(&state_root)?;
    let response = LocalClient::new(state_root.join("connectors.sock"))
        .event(&config.owner_context(), request)
        .await?;
    print_response(serde_json::to_value(response)?)?;
    Ok(())
}

async fn operation(command: OperationCommand) -> Result<(), MainError> {
    let (config_path, state_root, request) = match command {
        OperationCommand::Search {
            config,
            query,
            limit,
            state_root,
        } => (
            config,
            state_root,
            OperationRequest::Search(OperationSearchRequest { query, limit }),
        ),
        OperationCommand::Describe {
            config,
            operation,
            state_root,
        } => (
            config,
            state_root,
            OperationRequest::Describe(OperationDescribeRequest {
                operation_ref: operation,
            }),
        ),
        OperationCommand::Invoke {
            config,
            operation,
            connection,
            description_ref,
            input_json,
            approval_evidence_ref,
            state_root,
        } => {
            let input = serde_json::from_str(&input_json)?;
            (
                config,
                state_root,
                OperationRequest::Invoke(OperationInvokeRequest {
                    operation_ref: operation,
                    connection_ref: connection,
                    description_ref,
                    input,
                    approval_evidence_ref,
                }),
            )
        }
    };
    let config = PersonalConfig::read(&config_path)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    validate_state_root(&state_root)?;
    let response = LocalClient::new(state_root.join("connectors.sock"))
        .operation(&config.owner_context(), request)
        .await?;
    print_response(serde_json::to_value(response)?)?;
    Ok(())
}

async fn prompt_and_submit_completion(
    state_root: &Path,
    completion_endpoint: &Path,
    prompt: &str,
) -> Result<(), MainError> {
    let endpoint = CompletionEndpoint::validate(state_root, completion_endpoint)?;
    let token = Zeroizing::new(rpassword::prompt_password(prompt).map_err(MainError::Io)?);
    endpoint.submit(token.as_bytes()).await?;
    Ok(())
}

fn print_response(response: serde_json::Value) -> Result<(), MainError> {
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory as _;

    #[test]
    fn normal_help_exposes_the_guided_flow_and_hides_acquisition_plumbing() {
        let root_help = Cli::command().render_long_help().to_string();
        assert!(root_help.contains("connect"));
        assert!(!root_help.contains("connect-complete"));

        let mut command = Cli::command();
        let connection = command.find_subcommand_mut("connection").unwrap();
        let connection_help = connection.render_long_help().to_string();
        assert!(connection_help.contains("list"));
        assert!(!connection_help.contains("create"));
        assert!(!connection_help.contains("status"));
    }

    #[test]
    fn slack_connect_needs_no_internal_reference_or_path_argument() {
        let cli = Cli::try_parse_from(["connectors", "connect", "slack"]).unwrap();
        let Command::Connect {
            provider,
            config,
            label,
            context,
            state_root,
        } = cli.command
        else {
            panic!("guided connect command was not parsed");
        };
        assert_eq!(provider, "slack");
        assert!(label.is_none());
        assert!(context.is_none());
        assert!(config.is_none());
        assert!(state_root.is_none());
    }

    #[test]
    fn grafana_connect_uses_the_same_guided_surface() {
        let cli = Cli::try_parse_from(["connectors", "connect", "grafana"]).unwrap();
        let Command::Connect {
            provider, label, ..
        } = cli.command
        else {
            panic!("guided connect command was not parsed");
        };
        assert_eq!(provider, "grafana");
        assert!(label.is_none());
    }

    #[test]
    fn kubernetes_connect_accepts_an_exact_context_selection() {
        let cli = Cli::try_parse_from([
            "connectors",
            "connect",
            "kubernetes",
            "--context",
            "dev-cluster",
        ])
        .unwrap();
        let Command::Connect {
            provider, context, ..
        } = cli.command
        else {
            panic!("guided connect command was not parsed");
        };
        assert_eq!(provider, "kubernetes");
        assert_eq!(context.as_deref(), Some("dev-cluster"));
    }
}

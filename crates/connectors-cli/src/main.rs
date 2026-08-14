#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use connectors_cli::{
    load_authority_issuer, MonitoringBackend, PersonalConfig, RefusingBackend, RuntimeLauncher,
    SipOperationBackend, SlackBackend,
};
use protocol::connection::{
    ConnectSessionCreateRequest, ConnectSessionState, ConnectSessionStatusRequest,
    ConnectionDescription, ConnectionRequest, ConnectionResult, DiscoveryObservationState,
    MaterializeRequest, ObservationSearchRequest, RequestEnvelope as ConnectionEnvelope,
    ResponseEnvelope as ConnectionResponseEnvelope, ResponseStatus as ConnectionResponseStatus,
    SearchRequest as ConnectionSearchRequest,
};
use protocol::event::{
    EventRequest, ReceiveRequest, ReplayRequest, RequestEnvelope as EventEnvelope,
    SearchRequest as EventSearchRequest,
};
use protocol::operation::{
    DescribeRequest as OperationDescribeRequest, InvokeRequest as OperationInvokeRequest,
    OperationRequest, RequestEnvelope as OperationEnvelope,
    SearchRequest as OperationSearchRequest,
};
use server::local::{LocalOperationDaemon, OperationBackend};
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::UnixStream;
use tokio::time::{sleep, Duration};
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
    /// Add a provider through one guided, secret-safe flow.
    Connect {
        /// Provider to add. The personal-local alpha supports `slack` and `grafana`.
        provider: String,
        /// Strict value-free deployment configuration.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Human label for the resulting Connection.
        #[arg(long)]
        label: Option<String>,
        /// Owner-only state root used by the running Connector.
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Manage durable Connections through the credential-free control socket.
    Connection {
        #[command(subcommand)]
        command: ConnectionCommand,
    },
    /// Complete one Connect Session directly from an operator terminal.
    #[command(hide = true)]
    ConnectComplete {
        /// The single-use completion endpoint returned by `connection create`.
        #[arg(long)]
        completion_endpoint: PathBuf,
        /// Owner-only state root containing that endpoint.
        #[arg(long)]
        state_root: Option<PathBuf>,
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
    /// Create a short-lived, single-use credential acquisition session.
    #[command(hide = true)]
    Create {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, default_value = "slack")]
        integration: String,
        #[arg(long)]
        label: String,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Poll a Connect Session; only a completed status names the durable Connection.
    #[command(hide = true)]
    Status {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        session: String,
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
    #[error("a personal-local state root could not be derived")]
    MissingStateRoot,
    #[error("a personal-local configuration path could not be derived")]
    MissingConfigPath,
    #[error("the state root must be absolute and outside the current working tree")]
    UnsafeStateRoot,
    #[error(transparent)]
    Config(#[from] connectors_cli::ConfigError),
    #[error("voice runtime configuration was refused: {0}")]
    Runtime(#[from] connectors_cli::LaunchError),
    #[error("operation backend configuration was refused: {0}")]
    Backend(#[from] protocol::operation::OperationError),
    #[error(transparent)]
    Slack(#[from] connectors_cli::SlackError),
    #[error(transparent)]
    Monitoring(#[from] connectors_cli::MonitoringError),
    #[error(transparent)]
    Daemon(#[from] server::local::LocalDaemonError),
    #[error("local Connector request failed: {0}")]
    Io(#[from] io::Error),
    #[error("local Connector response was malformed: {0}")]
    Json(#[from] serde_json::Error),
    #[error(
        "the Connect Session completion endpoint is not an owner-only socket under this state root"
    )]
    UnsafeCompletionEndpoint,
    #[error("the guided connection flow does not support provider `{0}` yet")]
    UnsupportedProvider(String),
    #[error("the Connector refused the connection request: {0}")]
    ConnectionRefused(String),
    #[error("the Connector returned an invalid connection response")]
    InvalidConnectionResponse,
    #[error("the Connector refused the submitted credential")]
    CompletionRefused,
    #[error("the new Connection did not become callable before the local deadline")]
    ConnectionNotCallable,
}

#[tokio::main]
async fn main() -> Result<(), MainError> {
    match Cli::parse().command {
        Command::Serve { config, state_root } => serve(config, state_root).await,
        Command::Connect {
            provider,
            config,
            label,
            state_root,
        } => connect(provider, config, label, state_root).await,
        Command::Connection { command } => connection(command).await,
        Command::ConnectComplete {
            completion_endpoint,
            state_root,
        } => connect_complete(completion_endpoint, state_root).await,
        Command::Event { command } => event(command).await,
        Command::Operation { command } => operation(command).await,
    }
}

async fn connect(
    provider: String,
    config_path: Option<PathBuf>,
    label: Option<String>,
    state_root: Option<PathBuf>,
) -> Result<(), MainError> {
    if !matches!(provider.as_str(), "slack" | "grafana") {
        return Err(MainError::UnsupportedProvider(provider));
    }
    let config_path = config_path.map_or_else(default_config_path, Ok)?;
    let config = PersonalConfig::read(&config_path)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    validate_state_root(&state_root)?;
    let socket = state_root.join("connectors.sock");
    let owner = config.owner_context();

    let display_name = match provider.as_str() {
        "slack" => "Slack",
        "grafana" => "Grafana",
        _ => unreachable!("provider was validated"),
    };
    let label = label.unwrap_or_else(|| display_name.to_owned());
    println!("Connect {display_name}");
    println!("Input is hidden and sent only to the local Connector.");
    let created = send_connection_request(
        &socket,
        &owner,
        ConnectionRequest::ConnectSessionCreate(ConnectSessionCreateRequest {
            integration_ref: provider.clone(),
            label,
        }),
    )
    .await?;
    let ConnectionResult::ConnectSessionCreate(created) = created else {
        return Err(MainError::InvalidConnectionResponse);
    };
    if created.state != ConnectSessionState::Pending {
        return Err(MainError::InvalidConnectionResponse);
    }
    let session_ref = created.connect_session_ref;
    let endpoint = created
        .completion_endpoint
        .map(PathBuf::from)
        .ok_or(MainError::InvalidConnectionResponse)?;
    let credential_prompt = match provider.as_str() {
        "slack" => "Slack app token: ",
        "grafana" => "Grafana service account token: ",
        _ => unreachable!("provider was validated"),
    };
    prompt_and_submit_completion(&state_root, &endpoint, credential_prompt).await?;

    let completed = send_connection_request(
        &socket,
        &owner,
        ConnectionRequest::ConnectSessionStatus(ConnectSessionStatusRequest {
            connect_session_ref: session_ref,
        }),
    )
    .await?;
    let ConnectionResult::ConnectSessionStatus(completed) = completed else {
        return Err(MainError::InvalidConnectionResponse);
    };
    if completed.state != ConnectSessionState::Completed {
        return Err(MainError::CompletionRefused);
    }
    let connection_ref = completed
        .connection_ref
        .ok_or(MainError::InvalidConnectionResponse)?;
    let description = wait_for_callable(&socket, &owner, &connection_ref).await?;
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
        let observations = send_connection_request(
            &socket,
            &owner,
            ConnectionRequest::ObservationSearch(ObservationSearchRequest {
                source_connection_ref: connection_ref,
                query: String::new(),
                limit: protocol::connection::MAX_SEARCH_RESULTS,
            }),
        )
        .await?;
        let ConnectionResult::ObservationSearch { observations } = observations else {
            return Err(MainError::InvalidConnectionResponse);
        };
        let mut materialized = Vec::new();
        let mut unsupported = 0_usize;
        let mut not_granted = 0_usize;
        for observation in observations {
            let Some(target_provider) = observation.target_provider_ref.as_deref() else {
                unsupported += 1;
                continue;
            };
            if observation.state == DiscoveryObservationState::Unsupported {
                unsupported += 1;
                continue;
            }
            if config
                .grafana
                .as_ref()
                .and_then(|grafana| grafana.target_grant(target_provider))
                .is_none()
            {
                not_granted += 1;
                continue;
            }
            let result = send_connection_request(
                &socket,
                &owner,
                ConnectionRequest::Materialize(MaterializeRequest {
                    observation_ref: observation.observation_ref,
                }),
            )
            .await?;
            let ConnectionResult::Materialize(target) = result else {
                return Err(MainError::InvalidConnectionResponse);
            };
            materialized.push(target.summary);
        }
        println!("Grafana is connected and its data sources are ready.");
        println!("Connection: {}", description.summary.label);
        for target in &materialized {
            println!(
                "Target: {} ({}) -> {}",
                target.label, target.integration_ref, target.connection_ref
            );
        }
        if unsupported > 0 {
            println!("Observed but unsupported data sources: {unsupported}");
        }
        if not_granted > 0 {
            println!("Recognized data sources without a configured target Grant: {not_granted}");
        }
    }
    Ok(())
}

async fn wait_for_callable(
    socket: &Path,
    owner: &protocol::operation::OwnerContext,
    connection_ref: &str,
) -> Result<ConnectionDescription, MainError> {
    for _ in 0..20 {
        let result = send_connection_request(
            socket,
            owner,
            ConnectionRequest::Describe(protocol::connection::DescribeRequest {
                connection_ref: connection_ref.to_owned(),
            }),
        )
        .await?;
        let ConnectionResult::Describe(description) = result else {
            return Err(MainError::InvalidConnectionResponse);
        };
        if description.summary.state == protocol::connection::ConnectionState::Callable {
            return Ok(description);
        }
        sleep(Duration::from_millis(500)).await;
    }
    Err(MainError::ConnectionNotCallable)
}

async fn serve(config_path: Option<PathBuf>, state_root: Option<PathBuf>) -> Result<(), MainError> {
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    validate_state_root(&state_root)?;
    let socket_path = state_root.join("connectors.sock");
    let (backend, verifying_key, slack_connections, monitoring_connections): (
        Arc<dyn OperationBackend>,
        Option<String>,
        Option<usize>,
        Option<usize>,
    ) = if let Some(config_path) = config_path {
        let config = PersonalConfig::read(&config_path)?;
        let owner = config.owner_context();
        let (mut backend, verifying_key): (Arc<dyn OperationBackend>, Option<String>) =
            if let Some(voice) = config.voice()? {
                let issuer = Arc::new(load_authority_issuer(&voice.authority)?);
                let verifying_key = hex::encode(issuer.verifying_key().to_bytes());
                let launcher = Arc::new(RuntimeLauncher::new(
                    Arc::clone(&issuer),
                    voice.application.endpoint.clone(),
                    voice.application.connect_address,
                    voice.application.tls_server_name.clone(),
                ));
                (
                    Arc::new(SipOperationBackend::new(voice, launcher, &state_root)?),
                    Some(verifying_key),
                )
            } else {
                (Arc::new(RefusingBackend), None)
            };
        let slack_connections = if let Some(slack) = config.slack.clone() {
            let slack = SlackBackend::open(owner, slack, &state_root, backend).await?;
            let count = slack.connection_count();
            backend = Arc::new(slack);
            Some(count)
        } else {
            None
        };
        let monitoring_connections = if let Some(grafana) = config.grafana.clone() {
            let monitoring =
                MonitoringBackend::open(config.owner_context(), grafana, &state_root, backend)?;
            let count = monitoring.connection_count();
            backend = Arc::new(monitoring);
            Some(count)
        } else {
            None
        };
        (
            backend,
            verifying_key,
            slack_connections,
            monitoring_connections,
        )
    } else {
        (Arc::new(RefusingBackend), None, None, None)
    };
    let daemon = LocalOperationDaemon::bind(&socket_path, backend).await?;
    println!(
        "{}",
        serde_json::json!({
            "ready": true,
            "protocol": protocol::operation::CONTRACT,
            "protocols": [
                protocol::operation::CONTRACT,
                protocol::connection::CONTRACT,
                protocol::event::CONTRACT,
            ],
            "socket": daemon.socket_path(),
            "sip_dial_configured": verifying_key.is_some(),
            "voice_authority_verifying_key": verifying_key,
            "slack_configured": slack_connections.is_some(),
            "slack_connections": slack_connections,
            "grafana_configured": monitoring_connections.is_some(),
            "monitoring_connections": monitoring_connections,
        })
    );
    daemon
        .serve_until(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

async fn connection(command: ConnectionCommand) -> Result<(), MainError> {
    let (config_path, state_root, request) = match command {
        ConnectionCommand::Create {
            config,
            integration,
            label,
            state_root,
        } => (
            config,
            state_root,
            ConnectionRequest::ConnectSessionCreate(ConnectSessionCreateRequest {
                integration_ref: integration,
                label,
            }),
        ),
        ConnectionCommand::Status {
            config,
            session,
            state_root,
        } => (
            config,
            state_root,
            ConnectionRequest::ConnectSessionStatus(ConnectSessionStatusRequest {
                connect_session_ref: session,
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
    let envelope = ConnectionEnvelope {
        protocol: protocol::connection::CONTRACT.to_owned(),
        request_id: request_id(),
        context: config.owner_context(),
        request,
    };
    envelope
        .validate()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
    print_response(send_frame(&state_root.join("connectors.sock"), &envelope).await?)?;
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
    let envelope = EventEnvelope {
        protocol: protocol::event::CONTRACT.to_owned(),
        request_id: request_id(),
        context: config.owner_context(),
        request,
    };
    envelope
        .validate()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
    print_response(send_frame(&state_root.join("connectors.sock"), &envelope).await?)?;
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
    let envelope = OperationEnvelope {
        protocol: protocol::operation::CONTRACT.to_owned(),
        request_id: request_id(),
        context: config.owner_context(),
        request,
    };
    envelope
        .validate()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
    print_response(send_frame(&state_root.join("connectors.sock"), &envelope).await?)?;
    Ok(())
}

async fn send_connection_request(
    socket: &Path,
    owner: &protocol::operation::OwnerContext,
    request: ConnectionRequest,
) -> Result<ConnectionResult, MainError> {
    let envelope = ConnectionEnvelope {
        protocol: protocol::connection::CONTRACT.to_owned(),
        request_id: request_id(),
        context: owner.clone(),
        request,
    };
    envelope
        .validate()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
    let response: ConnectionResponseEnvelope =
        serde_json::from_value(send_frame(socket, &envelope).await?)?;
    response
        .validate()
        .map_err(|_| MainError::InvalidConnectionResponse)?;
    match response.status {
        ConnectionResponseStatus::Ok => response
            .response
            .ok_or(MainError::InvalidConnectionResponse),
        ConnectionResponseStatus::Error => Err(MainError::ConnectionRefused(
            response
                .error
                .ok_or(MainError::InvalidConnectionResponse)?
                .to_string(),
        )),
    }
}

async fn connect_complete(
    completion_endpoint: PathBuf,
    state_root: Option<PathBuf>,
) -> Result<(), MainError> {
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    validate_state_root(&state_root)?;
    prompt_and_submit_completion(&state_root, &completion_endpoint, "Connector credential: ")
        .await?;
    println!("Credential accepted by the Connector.");
    Ok(())
}

async fn prompt_and_submit_completion(
    state_root: &Path,
    completion_endpoint: &Path,
    prompt: &str,
) -> Result<(), MainError> {
    validate_completion_endpoint(state_root, completion_endpoint)?;
    let token = Zeroizing::new(rpassword::prompt_password(prompt).map_err(MainError::Io)?);
    let mut stream = UnixStream::connect(completion_endpoint).await?;
    stream.write_all(token.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.shutdown().await?;
    let mut response = String::new();
    BufReader::new(stream)
        .take(1025)
        .read_line(&mut response)
        .await?;
    if response.len() > 1024 {
        return Err(MainError::InvalidConnectionResponse);
    }
    let response: CompletionAcknowledgement = serde_json::from_str(&response)?;
    if !response.accepted {
        return Err(MainError::CompletionRefused);
    }
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CompletionAcknowledgement {
    accepted: bool,
}

async fn send_frame<T: serde::Serialize>(
    socket: &Path,
    envelope: &T,
) -> Result<serde_json::Value, MainError> {
    let mut stream = UnixStream::connect(socket).await?;
    let mut bytes = serde_json::to_vec(envelope)?;
    bytes.push(b'\n');
    stream.write_all(&bytes).await?;
    stream.shutdown().await?;
    let mut response = String::new();
    BufReader::new(stream)
        .take((protocol::event::MAX_RESPONSE_BYTES + 1) as u64)
        .read_line(&mut response)
        .await?;
    if response.is_empty() || response.len() > protocol::event::MAX_RESPONSE_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "bounded response missing").into());
    }
    Ok(serde_json::from_str(&response)?)
}

fn print_response(response: serde_json::Value) -> Result<(), MainError> {
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

fn request_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("client-{}-{timestamp}", std::process::id())
}

fn validate_completion_endpoint(state_root: &Path, endpoint: &Path) -> Result<(), MainError> {
    if !endpoint.is_absolute()
        || endpoint.parent() != Some(state_root.join("connect-sessions").as_path())
    {
        return Err(MainError::UnsafeCompletionEndpoint);
    }
    let parent = fs::symlink_metadata(endpoint.parent().expect("checked parent"))
        .map_err(|_| MainError::UnsafeCompletionEndpoint)?;
    let metadata =
        fs::symlink_metadata(endpoint).map_err(|_| MainError::UnsafeCompletionEndpoint)?;
    let owner = rustix::process::geteuid().as_raw();
    if !parent.file_type().is_dir()
        || parent.file_type().is_symlink()
        || parent.uid() != owner
        || parent.permissions().mode() & 0o077 != 0
        || !metadata.file_type().is_socket()
        || metadata.file_type().is_symlink()
        || metadata.uid() != owner
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(MainError::UnsafeCompletionEndpoint);
    }
    Ok(())
}

fn default_state_root() -> Result<PathBuf, MainError> {
    if let Some(root) = env::var_os("XDG_STATE_HOME") {
        return Ok(PathBuf::from(root).join("b10x/connectors"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/state/b10x/connectors"))
        .ok_or(MainError::MissingStateRoot)
}

fn default_config_path() -> Result<PathBuf, MainError> {
    if let Some(root) = env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(root).join("b10x/connectors.toml"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".config/b10x/connectors.toml"))
        .ok_or(MainError::MissingConfigPath)
}

fn validate_state_root(root: &Path) -> Result<(), MainError> {
    if !root.is_absolute() {
        return Err(MainError::UnsafeStateRoot);
    }
    let current = env::current_dir()
        .and_then(std::fs::canonicalize)
        .map_err(|_| MainError::UnsafeStateRoot)?;
    let comparable = existing_ancestor(root)
        .and_then(|ancestor| std::fs::canonicalize(ancestor).ok())
        .ok_or(MainError::UnsafeStateRoot)?;
    if comparable.starts_with(&current) {
        return Err(MainError::UnsafeStateRoot);
    }
    Ok(())
}

fn existing_ancestor(path: &Path) -> Option<&Path> {
    path.ancestors().find(|candidate| candidate.exists())
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
            state_root,
        } = cli.command
        else {
            panic!("guided connect command was not parsed");
        };
        assert_eq!(provider, "slack");
        assert!(label.is_none());
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
}

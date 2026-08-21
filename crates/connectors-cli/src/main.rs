#![forbid(unsafe_code)]

use std::io;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use connectors_client::LocalClient;
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

use connectors_console::{auth, connect, doctor, enrol, init, input, output, reduce_envelope, Format};

#[derive(Debug, Parser)]
#[command(name = "connectors", version, about = "B10x Connectors service")]
struct Cli {
    /// How results are rendered. `json` and `yaml` also carry failures on stdout, so a pipe reads
    /// the refusal instead of an empty stream.
    #[arg(long, short = 'o', value_enum, default_value_t = Format::Text, global = true)]
    output: Format,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Write a usable configuration for this machine, so nothing has to be authored by hand.
    Init {
        /// Where to write. Defaults below XDG_CONFIG_HOME.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Owner-only state root it will be served with.
        #[arg(long)]
        state_root: Option<PathBuf>,
        /// Integration to declare. Repeatable. Omit to admit whatever this machine supports.
        #[arg(long = "integration", value_enum)]
        integrations: Vec<init::Integration>,
        /// Admit kubeconfig contexts authenticated by a credential plugin. Every EKS context is one.
        #[arg(long)]
        allow_exec_auth: bool,
        /// Replace an existing configuration.
        #[arg(long)]
        force: bool,
    },
    /// Which configured providers have their credential stored. Never reads one.
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// What every catalogued provider needs before it can be connected.
    Providers {
        /// Narrow to providers whose id or vendor contains this text.
        #[arg(long, default_value = "")]
        query: String,
    },
    /// What is configured, what is running, and what cannot work.
    Doctor {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
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
        ///
        /// Required, unlike every personal-local command: a hosted deployment's configuration is
        /// installed by whoever operates it, and defaulting to a path in the invoking user's home
        /// directory would be the wrong file every time.
        #[arg(long)]
        config: PathBuf,
    },
    /// Add a provider through one guided, secret-safe flow.
    Connect {
        /// Provider to add. `connectors providers` lists every one the catalogue declares.
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
        /// Which declared credential to supply.
        #[arg(long = "as")]
        credential: Option<String>,
        /// A declared configuration value, as `field=value`. Repeatable.
        #[arg(long = "set", value_parser = enrol::parse_setting)]
        settings: Vec<(String, String)>,
        /// Admit write operations. Reads only without it.
        #[arg(long = "allow", value_parser = ["writes"])]
        allow: Option<String>,
        /// Admit private destinations, for a self-hosted instance on your own network.
        #[arg(long)]
        operator_network: bool,
        /// Read the credential from an owner-only file rather than prompting.
        #[arg(long)]
        credential_file: Option<PathBuf>,
        /// Which instance, when this placement holds one provider more than once.
        #[arg(long)]
        instance: Option<String>,
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
enum AuthCommand {
    /// Which configured providers are connected, and which are not.
    Status {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum ConnectionCommand {
    /// Passively list potential direct Connections without contacting their providers.
    Candidates {
        #[arg(long)]
        config: Option<PathBuf>,
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
        config: Option<PathBuf>,
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
        config: Option<PathBuf>,
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
        config: Option<PathBuf>,
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
        config: Option<PathBuf>,
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
        config: Option<PathBuf>,
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
        config: Option<PathBuf>,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Send DTMF into a session that is already established.
    Signal {
        #[arg(long)]
        config: Option<PathBuf>,
        /// The `execution_ref` the invocation that placed the call returned.
        #[arg(long)]
        execution_ref: String,
        /// Digits to send, from `0123456789*#ABCD`.
        #[arg(long)]
        dtmf: String,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Invoke an operation using a fresh description lease.
    Invoke {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        connection: String,
        #[arg(long)]
        description_ref: String,
        /// Strict JSON object of catalog-declared caller input. See also --input-file and --input.
        #[arg(long, group = "caller-input")]
        input_json: Option<String>,
        /// Read the input object from a file.
        #[arg(long, group = "caller-input")]
        input_file: Option<PathBuf>,
        /// Read the input object from stdin. The only accepted value is `-`.
        #[arg(long, group = "caller-input")]
        input: Option<String>,
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
        config: Option<PathBuf>,
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
        config: Option<PathBuf>,
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
        config: Option<PathBuf>,
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
    #[error("the Connector returned an invalid connection response")]
    InvalidConnectionResponse,
    #[error(transparent)]
    Connect(#[from] connect::ConnectError),
    #[error(transparent)]
    Input(#[from] input::InputError),
    #[error(transparent)]
    Auth(#[from] auth::AuthError),
    #[error(transparent)]
    Enrol(#[from] enrol::EnrolError),
    #[error(transparent)]
    Refused(#[from] connectors_console::envelope::ReducedError),
    #[error(transparent)]
    Init(#[from] init::InitError),
    #[error(transparent)]
    Output(#[from] output::OutputError),
    /// `doctor` found something that cannot work. The detail is in the report it already printed.
    #[error("this installation has a problem `connectors doctor` named above")]
    Unhealthy,

}

impl MainError {
    /// A stable token naming the *class* of fault, for a caller that branches on failures.
    ///
    /// Deliberately coarse and deliberately not the message: a script should be able to match on
    /// `configuration` without depending on the sentence a human reads, and the sentence is free to
    /// improve. A refusal forwards the Connector's own code, which is the contract's vocabulary and
    /// more precise than anything this layer could invent. No arm can carry a credential.
    fn code(&self) -> &str {
        match self {
            Self::Runtime(_) => "runtime",
            Self::Config(_) | Self::Init(_) => "configuration",
            Self::Client(_) => "connector-unreachable",
            Self::Io(_) => "io",
            Self::Json(_) | Self::InvalidConnectionResponse => "malformed-response",
            Self::Connect(connect::ConnectError::Unsupported(_)) => "unsupported-provider",
            Self::Connect(_) => "connect",
            Self::Output(_) => "output",
            Self::Refused(refusal) => &refusal.code,
            Self::Unhealthy => "unhealthy",
            Self::Input(_) => "invalid-argument",
            Self::Auth(_) => "credential-store",
            Self::Enrol(_) => "connect",
        }
    }
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let cli = Cli::parse();
    // Captured before dispatch: a failure must be rendered in the format the caller asked for, and
    // the command that failed is no longer available to ask.
    let format = cli.output;
    match run(cli).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            output::emit_error(format, error.code(), &error.to_string());
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<(), MainError> {
    let format = cli.output;
    match cli.command {
        Command::Init {
            config,
            state_root,
            integrations,
            allow_exec_auth,
            force,
        } => initialize(format, config, state_root, &integrations, allow_exec_auth, force),
        Command::Auth { command } => {
            let AuthCommand::Status { config, state_root } = command;
            let config = read_config(config)?;
            let state_root = state_root.map_or_else(default_state_root, Ok)?;
            output::emit(format, &auth::status(&config, &state_root).await?)?;
            Ok(())
        }
        Command::Providers { query } => {
            output::emit(format, &connectors_console::providers::run(&query))?;
            Ok(())
        }
        Command::Doctor { config, state_root } => diagnose(format, config, state_root),
        Command::Serve { config, state_root } => serve(config, state_root).await,
        Command::ServeHosted { config } => serve_hosted(&config).await,
        Command::Connect {
            provider,
            config,
            label,
            context,
            state_root,
            credential,
            settings,
            allow,
            operator_network,
            credential_file,
            instance,
        } => {
            let config_path = config.map_or_else(default_config_path, Ok)?;
            let state_root = state_root.map_or_else(default_state_root, Ok)?;
            validate_state_root(&state_root)?;
            let personal = PersonalConfig::read(&config_path)?;
            let options = enrol::Options {
                credential,
                values: settings.into_iter().collect(),
                allow_writes: allow.as_deref() == Some("writes"),
                operator_network,
                force: false,
                credential_file,
                instance,
                acquire: enrol::acquires(&provider)
                    .then(|| connectors_runtime::argocd_acquisition(operator_network)),
            };
            let outcome = connect::dispatch(
                &provider,
                &personal,
                &config_path,
                &state_root,
                label,
                context,
                options,
            )
            .await?;
            output::emit(format, &outcome)?;
            Ok(())
        }
        Command::Connection { command } => connection(format, command).await,
        Command::Event { command } => event(format, command).await,
        Command::Operation { command } => operation(format, command).await,
    }
}

/// Report on this installation, and exit non-zero when something in it cannot work.
fn diagnose(
    format: Format,
    config: Option<PathBuf>,
    state_root: Option<PathBuf>,
) -> Result<(), MainError> {
    let config_path = config.map_or_else(default_config_path, Ok)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    let report = doctor::run(&config_path, &state_root);
    output::emit(format, &report.to_value())?;
    if report.healthy() {
        Ok(())
    } else {
        // The report was already rendered; this only sets the exit code, so a script can branch on
        // `connectors doctor` without parsing it.
        Err(MainError::Unhealthy)
    }
}

fn initialize(
    format: Format,
    config: Option<PathBuf>,
    state_root: Option<PathBuf>,
    integrations: &[init::Integration],
    allow_exec_auth: bool,
    force: bool,
) -> Result<(), MainError> {
    let config_path = config.map_or_else(default_config_path, Ok)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    let written = init::run(
        &config_path,
        &state_root,
        integrations,
        allow_exec_auth,
        force,
    )?;
    output::emit(
        format,
        &serde_json::json!({
            "config": written.config_path.display().to_string(),
            "state_root": written.state_root.display().to_string(),
            "authority_snapshot_id": written.snapshot_id,
            "integrations": written.integrations,
            "notes": written.notes,
        }),
    )?;
    Ok(())
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

async fn serve(config_path: Option<PathBuf>, state_root: Option<PathBuf>) -> Result<(), MainError> {
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    // **An omitted `--config` means the well-known path, not "no configuration".** Passing `None`
    // straight through composed a daemon with an empty registry: `connectors init` followed by
    // `connectors serve` published a readiness document reporting every integration as absent, and
    // the operator's next command found nothing callable with no error anywhere to explain it.
    //
    // A genuinely absent file still composes the refusing protocol skeleton, which is what a
    // machine that has never run `init` should get — so the fallback is "the default path if it
    // exists", not "the default path, and fail if it does not".
    let config_path = match config_path {
        Some(explicit) => Some(explicit),
        None => default_config_path().ok().filter(|path| path.exists()),
    };
    let runtime = PersonalRuntime::bind(config_path.as_deref(), state_root).await?;
    println!("{}", runtime.readiness());
    runtime
        .serve_until(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

async fn connection(format: Format, command: ConnectionCommand) -> Result<(), MainError> {
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
    let config = read_config(config_path)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    validate_state_root(&state_root)?;
    let response = LocalClient::new(state_root.join("connectors.sock"))
        .connection(&config.owner_context(), request)
        .await?;
    output::emit(format, &reduce_envelope!(response)?)?;
    Ok(())
}

async fn event(format: Format, command: EventCommand) -> Result<(), MainError> {
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
    let config = read_config(config_path)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    validate_state_root(&state_root)?;
    let response = LocalClient::new(state_root.join("connectors.sock"))
        .event(&config.owner_context(), request)
        .await?;
    output::emit(format, &reduce_envelope!(response)?)?;
    Ok(())
}

async fn operation(format: Format, command: OperationCommand) -> Result<(), MainError> {
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
        OperationCommand::Signal {
            config,
            execution_ref,
            dtmf,
            state_root,
        } => (
            config,
            state_root,
            OperationRequest::SessionSignal(protocol::operation::SessionSignalRequest {
                execution_ref,
                signal: protocol::operation::ChannelSignal::Dtmf { digits: dtmf },
            }),
        ),
        OperationCommand::Invoke {
            config,
            operation,
            connection,
            description_ref,
            input_json,
            input_file,
            input,
            approval_evidence_ref,
            state_root,
        } => {
            let input = input::read(input_json, input_file, input)?;
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
    let config = read_config(config_path)?;
    let state_root = state_root.map_or_else(default_state_root, Ok)?;
    validate_state_root(&state_root)?;
    let response = LocalClient::new(state_root.join("connectors.sock"))
        .operation(&config.owner_context(), request)
        .await?;
    output::emit(format, &reduce_envelope!(response)?)?;
    Ok(())
}

/// Read the personal configuration, defaulting to the well-known path.
///
/// Every personal-local command takes `--config` as an option rather than a requirement. Before
/// this, each one demanded an explicit path, so the shortest working invocation of the product
/// carried a path the person had to know and retype.
fn read_config(path: Option<PathBuf>) -> Result<PersonalConfig, MainError> {
    let path = path.map_or_else(default_config_path, Ok)?;
    Ok(PersonalConfig::read(&path)?)
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
            ..
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

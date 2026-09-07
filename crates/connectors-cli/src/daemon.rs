//! Process lifecycle argument parsing and thin dispatch.

use super::*;

#[derive(Debug, Subcommand)]
pub(super) enum DaemonCommand {
    /// Start the background daemon and wait until it is ready.
    Start {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Inspect the running daemon without contacting providers.
    Status {
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
    /// Gracefully stop the daemon and its active connections.
    Stop {
        #[arg(long)]
        state_root: Option<PathBuf>,
    },
}

pub(super) async fn run(format: Format, command: DaemonCommand) -> Result<(), MainError> {
    use connectors_console::daemon;
    let value = match command {
        DaemonCommand::Start { config, state_root } => {
            let config = config.map_or_else(default_config_path, Ok)?;
            let _ = PersonalConfig::read(&config)?;
            let root = state_root.map_or_else(default_state_root, Ok)?;
            daemon::start(&config, &root).await?
        }
        DaemonCommand::Status { state_root } => {
            let root = state_root.map_or_else(default_state_root, Ok)?;
            daemon::status(&root).await?
        }
        DaemonCommand::Stop { state_root } => {
            let root = state_root.map_or_else(default_state_root, Ok)?;
            daemon::stop(&root).await?
        }
    };
    emit(format, &value)
}

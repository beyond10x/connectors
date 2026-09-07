//! Background process lifecycle, using the daemon's owner-only control socket.

use std::fs::{self, OpenOptions};
use std::io;
use std::os::unix::fs::{
    FileTypeExt as _, MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _,
};
use std::os::unix::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use connectors_client::{ClientError, LocalClient};
use protocol::lifecycle::{DaemonStatus, LifecycleRequest};
use serde_json::{json, Value};

const START_TIMEOUT: Duration = Duration::from_secs(30);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

struct StartingChild {
    process: Child,
    ready: bool,
}

impl Drop for StartingChild {
    fn drop(&mut self) {
        if !self.ready {
            let _ = self.process.kill();
            let _ = self.process.wait();
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("local provider access requires the daemon; run `connectors daemon start` with the same --config and --state-root")]
    Required,
    #[error(
        "the running daemon serves another configuration; stop it or use another --state-root"
    )]
    ConfigurationConflict,
    #[error(
        "the local daemon path must be an owner-only directory containing an owner-only socket"
    )]
    UnsafePath,
    #[error("the daemon did not become ready; inspect the owner-only log at {0}")]
    StartFailed(PathBuf),
    #[error("the daemon did not complete graceful shutdown")]
    StopTimeout,
    #[error(
        "cannot start the daemon from this embedded command; run the standalone connectors binary"
    )]
    UnsupportedLauncher,
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error("daemon lifecycle I/O failed: {0}")]
    Io(#[from] io::Error),
}

fn private_root(root: &Path, create: bool) -> Result<bool, DaemonError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) => {
            if !metadata.is_dir()
                || metadata.uid() != rustix::process::geteuid().as_raw()
                || metadata.mode() & 0o077 != 0
            {
                return Err(DaemonError::UnsafePath);
            }
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound && create => {
            fs::create_dir_all(root)?;
            fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
            private_root(root, false)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn socket_present(root: &Path) -> Result<bool, DaemonError> {
    if !private_root(root, false)? {
        return Ok(false);
    }
    match fs::symlink_metadata(root.join("connectors.sock")) {
        Ok(metadata)
            if metadata.file_type().is_socket()
                && metadata.uid() == rustix::process::geteuid().as_raw()
                && metadata.mode() & 0o077 == 0 =>
        {
            Ok(true)
        }
        Ok(_) => Err(DaemonError::UnsafePath),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Inspect the running daemon. Missing or stale sockets report stopped; malformed peers refuse.
pub async fn inspect(root: &Path) -> Result<Option<DaemonStatus>, DaemonError> {
    if !socket_present(root)? {
        return Ok(None);
    }
    match tokio::time::timeout(
        REQUEST_TIMEOUT,
        LocalClient::new(root.join("connectors.sock")).lifecycle(LifecycleRequest::Status {}),
    )
    .await
    {
        Ok(Ok(status)) => Ok(Some(status)),
        Ok(Err(ClientError::Io(error)))
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) =>
        {
            Ok(None)
        }
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, "daemon status timed out").into()),
    }
}

/// Enforce the one-daemon execution boundary without constructing any provider runtime.
pub async fn require(root: &Path) -> Result<(), DaemonError> {
    if socket_present(root)? {
        Ok(())
    } else {
        Err(DaemonError::Required)
    }
}

/// Structured status for both terminal and automation callers.
pub async fn status(root: &Path) -> Result<Value, DaemonError> {
    let daemon = inspect(root).await?;
    Ok(json!({"running": daemon.is_some(), "state_root": root, "daemon": daemon}))
}

/// Start this build in its own process group and wait for its authenticated lifecycle response.
pub async fn start(config: &Path, root: &Path) -> Result<Value, DaemonError> {
    let config = config.canonicalize()?;
    if let Some(status) = inspect(root).await? {
        if status.configuration.as_deref() != config.to_str() {
            return Err(DaemonError::ConfigurationConflict);
        }
        return Ok(
            json!({"running": true, "started": false, "state_root": root, "daemon": status}),
        );
    }
    private_root(root, true)?;
    let root = root.canonicalize()?;
    let log_path = root.join("daemon.log");
    let log = OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(&log_path)?;
    let metadata = log.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o077 != 0
        || metadata.nlink() != 1
    {
        return Err(DaemonError::UnsafePath);
    }
    let mut command = launcher()?;
    command
        .args(["serve", "local"])
        .arg("--config")
        .arg(&config)
        .arg("--state-root")
        .arg(&root)
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .process_group(0);
    let mut child = StartingChild {
        process: command.spawn()?,
        ready: false,
    };
    let deadline = tokio::time::Instant::now() + START_TIMEOUT;
    while tokio::time::Instant::now() < deadline {
        if child.process.try_wait()?.is_some() {
            return Err(DaemonError::StartFailed(log_path));
        }
        if let Some(status) = inspect(&root).await? {
            if status.process_id != child.process.id()
                || status.configuration.as_deref() != config.to_str()
            {
                // Only the process handle returned by our spawn may be terminated on a race.
                return Err(DaemonError::ConfigurationConflict);
            }
            child.ready = true;
            return Ok(json!({"running": true, "started": true, "state_root": root,
                "log": log_path, "daemon": status}));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(DaemonError::StartFailed(log_path))
}

fn launcher() -> Result<Command, DaemonError> {
    let executable = std::env::current_exe()?;
    let mut command = Command::new(&executable);
    if executable
        .file_stem()
        .is_some_and(|name| name == "connectors")
    {
        return Ok(command);
    }
    // Zwirn deliberately embeds this same CLI. Preserve its explicit subcommand prefix.
    if std::env::args_os()
        .nth(1)
        .is_some_and(|argument| argument == "connectors")
    {
        command.arg("connectors");
        return Ok(command);
    }
    Err(DaemonError::UnsupportedLauncher)
}

/// Stop the exact socket owner, then wait until it has released its socket and provider resources.
pub async fn stop(root: &Path) -> Result<Value, DaemonError> {
    let Some(before) = inspect(root).await? else {
        return Ok(json!({"running": false, "stopped": false, "state_root": root}));
    };
    let acknowledged = tokio::time::timeout(
        REQUEST_TIMEOUT,
        LocalClient::new(root.join("connectors.sock")).lifecycle(LifecycleRequest::Stop {}),
    )
    .await
    .map_err(|_| DaemonError::StopTimeout)??;
    if acknowledged.process_id != before.process_id {
        return Err(DaemonError::ConfigurationConflict);
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while tokio::time::Instant::now() < deadline {
        if !socket_present(root)? {
            return Ok(json!({"running": false, "stopped": true, "state_root": root}));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(DaemonError::StopTimeout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_daemon_does_not_create_state() {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("absent");
        assert!(matches!(require(&state).await, Err(DaemonError::Required)));
        assert!(!state.exists());
    }

    #[tokio::test]
    async fn symlink_socket_is_not_a_daemon() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        std::os::unix::fs::symlink("absent", root.path().join("connectors.sock")).unwrap();
        assert!(matches!(
            inspect(root.path()).await,
            Err(DaemonError::UnsafePath)
        ));
    }
}

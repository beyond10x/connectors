//! Owner-authenticated Unix-socket binding for the local Connector contracts.

use std::fs::{File, OpenOptions};
use std::future::Future;
use std::io;
use std::os::unix::fs::{
    FileTypeExt as _, MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use fs2::FileExt as _;
use protocol::operation::{RequestEnvelope, ResponseEnvelope, MAX_FRAME_BYTES};
use serde::Deserialize;
use service::{ConnectorBackend, PrincipalContext};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinSet;

const MAX_LOCAL_CLIENTS: usize = 64;
const FRAME_READ_DEADLINE: Duration = Duration::from_secs(5);
const BACKEND_SHUTDOWN_DEADLINE: Duration = Duration::from_secs(15);

/// Bound personal-local daemon. Binding completes before this value is returned, so callers can
/// publish readiness without racing the accept loop.
pub struct LocalOperationDaemon<B: ?Sized> {
    listener: UnixListener,
    socket_path: PathBuf,
    owner_uid: u32,
    backend: Arc<B>,
    _state_lock: File,
}

#[derive(Debug, thiserror::Error)]
pub enum LocalDaemonError {
    #[error("local Connector socket path has no parent directory")]
    MissingParent,
    #[error("local Connector state directory is not an owner-only real directory")]
    UnsafeStateDirectory,
    #[error("existing local Connector socket path is not the owner's Unix socket")]
    UnsafeExistingSocket,
    #[error("local Connector state lock is unsafe")]
    UnsafeStateLock,
    #[error("another Connector daemon owns this state root")]
    AlreadyRunning,
    #[error("local Connector socket I/O failed: {0}")]
    Io(#[from] io::Error),
}

impl<B: ConnectorBackend + ?Sized> LocalOperationDaemon<B> {
    /// Bind an owner-only Unix socket, refusing symlinks, foreign ownership, broad permissions, and
    /// an existing non-socket object.
    pub async fn bind(
        socket_path: impl Into<PathBuf>,
        backend: Arc<B>,
    ) -> Result<Self, LocalDaemonError> {
        let socket_path = socket_path.into();
        let parent = socket_path
            .parent()
            .ok_or(LocalDaemonError::MissingParent)?;
        let owner_uid = rustix::process::geteuid().as_raw();
        prepare_parent(parent, owner_uid)?;
        let state_lock = acquire_state_lock(parent, owner_uid)?;
        remove_owned_stale_socket(&socket_path, owner_uid)?;
        let listener = UnixListener::bind(&socket_path)?;
        std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))?;
        Ok(Self {
            listener,
            socket_path,
            owner_uid,
            backend,
            _state_lock: state_lock,
        })
    }

    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Serve until shutdown, then abort and join every incomplete local client before removing the
    /// exact socket path.
    pub async fn serve_until<F>(self, shutdown: F) -> Result<(), LocalDaemonError>
    where
        F: Future<Output = ()>,
    {
        let mut clients = JoinSet::new();
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                () = &mut shutdown => break,
                accepted = self.listener.accept() => {
                    let (stream, _) = accepted?;
                    if clients.len() >= MAX_LOCAL_CLIENTS {
                        drop(stream);
                        continue;
                    }
                    let backend = Arc::clone(&self.backend);
                    let owner_uid = self.owner_uid;
                    clients.spawn(async move {
                        let _ = serve_client(stream, owner_uid, backend).await;
                    });
                }
                Some(_) = clients.join_next(), if !clients.is_empty() => {}
            }
        }
        clients.abort_all();
        while clients.join_next().await.is_some() {}
        let _ = tokio::time::timeout(BACKEND_SHUTDOWN_DEADLINE, self.backend.shutdown()).await;
        drop(self.listener);
        remove_owned_stale_socket(&self.socket_path, self.owner_uid)?;
        Ok(())
    }
}

async fn serve_client<B: ConnectorBackend + ?Sized>(
    mut stream: UnixStream,
    owner_uid: u32,
    backend: Arc<B>,
) -> Result<(), LocalDaemonError> {
    let credential = stream.peer_cred()?;
    if credential.uid() != owner_uid {
        return Ok(());
    }
    let frame = {
        let mut reader = BufReader::new(&mut stream);
        match tokio::time::timeout(FRAME_READ_DEADLINE, read_frame(&mut reader)).await {
            Ok(frame) => frame?,
            Err(_) => return Ok(()),
        }
    };
    let Some(frame) = frame else {
        return Ok(());
    };
    let probe: ProtocolProbe = match serde_json::from_slice(&frame) {
        Ok(probe) => probe,
        Err(_) => return Ok(()),
    };
    let Some(mut bytes) = dispatch_frame(&frame, &probe.protocol, backend).await? else {
        return Ok(());
    };
    bytes.push(b'\n');
    stream.write_all(&bytes).await?;
    stream.shutdown().await?;
    Ok(())
}

#[derive(Deserialize)]
struct ProtocolProbe {
    protocol: String,
}

async fn dispatch_frame<B: ConnectorBackend + ?Sized>(
    frame: &[u8],
    protocol_name: &str,
    backend: Arc<B>,
) -> Result<Option<Vec<u8>>, LocalDaemonError> {
    let bytes = match protocol_name {
        protocol::operation::CONTRACT => {
            let request: RequestEnvelope = match serde_json::from_slice(frame) {
                Ok(request) => request,
                Err(_) => return Ok(None),
            };
            if request.validate().is_err() {
                return Ok(None);
            }
            let context = match PrincipalContext::local(&request.context) {
                Ok(context) => context,
                Err(_) => return Ok(None),
            };
            let request_id = request.request_id;
            let response = match backend.handle(&context, request.request).await {
                Ok(response) => ResponseEnvelope::success(&request_id, response),
                Err(error) => ResponseEnvelope::failure(&request_id, error),
            };
            let response = match response.validate() {
                Ok(()) => response,
                Err(error) => ResponseEnvelope::failure(request_id, error),
            };
            serde_json::to_vec(&response).map_err(io::Error::other)?
        }
        protocol::connection::CONTRACT => {
            let request: protocol::connection::RequestEnvelope = match serde_json::from_slice(frame)
            {
                Ok(request) => request,
                Err(_) => return Ok(None),
            };
            if request.validate().is_err() {
                return Ok(None);
            }
            let context = match PrincipalContext::local(&request.context) {
                Ok(context) => context,
                Err(_) => return Ok(None),
            };
            let request_id = request.request_id;
            let response = match backend.handle_connection(&context, request.request).await {
                Ok(response) => {
                    protocol::connection::ResponseEnvelope::success(&request_id, response)
                }
                Err(error) => protocol::connection::ResponseEnvelope::failure(&request_id, error),
            };
            let response = match response.validate() {
                Ok(()) => response,
                Err(error) => protocol::connection::ResponseEnvelope::failure(request_id, error),
            };
            serde_json::to_vec(&response).map_err(io::Error::other)?
        }
        protocol::catalog::CONTRACT => {
            let request: protocol::catalog::RequestEnvelope = match serde_json::from_slice(frame) {
                Ok(request) => request,
                Err(_) => return Ok(None),
            };
            if request.validate().is_err() || PrincipalContext::local(&request.context).is_err() {
                return Ok(None);
            }
            let request_id = request.request_id;
            let response = match crate::catalog_projection::handle(request.request) {
                Ok(result) => protocol::catalog::ResponseEnvelope::success(&request_id, result),
                Err(error) => protocol::catalog::ResponseEnvelope::failure(&request_id, error),
            };
            let response = match response.validate() {
                Ok(()) => response,
                Err(error) => protocol::catalog::ResponseEnvelope::failure(request_id, error),
            };
            serde_json::to_vec(&response).map_err(io::Error::other)?
        }
        protocol::event::CONTRACT => {
            let request: protocol::event::RequestEnvelope = match serde_json::from_slice(frame) {
                Ok(request) => request,
                Err(_) => return Ok(None),
            };
            if request.validate().is_err() {
                return Ok(None);
            }
            let context = match PrincipalContext::local(&request.context) {
                Ok(context) => context,
                Err(_) => return Ok(None),
            };
            let request_id = request.request_id;
            let response = match backend.handle_event(&context, request.request).await {
                Ok(response) => protocol::event::ResponseEnvelope::success(&request_id, response),
                Err(error) => protocol::event::ResponseEnvelope::failure(&request_id, error),
            };
            let response = match response.validate() {
                Ok(()) => response,
                Err(error) => protocol::event::ResponseEnvelope::failure(request_id, error),
            };
            serde_json::to_vec(&response).map_err(io::Error::other)?
        }
        protocol::datasource::CONTRACT => {
            let request: protocol::datasource::RequestEnvelope = match serde_json::from_slice(frame)
            {
                Ok(request) => request,
                Err(_) => return Ok(None),
            };
            if request.validate().is_err() {
                return Ok(None);
            }
            let context = match PrincipalContext::local(&request.context) {
                Ok(context) => context,
                Err(_) => return Ok(None),
            };
            let request_id = request.request_id;
            let response = match backend.handle_datasource(&context, request.request).await {
                Ok(response) => {
                    protocol::datasource::ResponseEnvelope::success(&request_id, response)
                }
                Err(error) => protocol::datasource::ResponseEnvelope::failure(&request_id, error),
            };
            let response = match response.validate() {
                Ok(()) => response,
                Err(error) => protocol::datasource::ResponseEnvelope::failure(request_id, error),
            };
            serde_json::to_vec(&response).map_err(io::Error::other)?
        }
        _ => return Ok(None),
    };
    Ok(Some(bytes))
}

async fn read_frame<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
) -> io::Result<Option<Vec<u8>>> {
    let mut frame = Vec::with_capacity(4096);
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "unterminated operation frame",
                ))
            };
        }
        if let Some(newline) = available.iter().position(|byte| *byte == b'\n') {
            if frame.len() + newline > MAX_FRAME_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "operation frame exceeds bound",
                ));
            }
            frame.extend_from_slice(&available[..newline]);
            reader.consume(newline + 1);
            return Ok(Some(frame));
        }
        if frame.len() + available.len() > MAX_FRAME_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "operation frame exceeds bound",
            ));
        }
        let consumed = available.len();
        frame.extend_from_slice(available);
        reader.consume(consumed);
    }
}

fn acquire_state_lock(parent: &Path, owner_uid: u32) -> Result<File, LocalDaemonError> {
    let path = parent.join(".connectors.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map_err(|_| LocalDaemonError::UnsafeStateLock)?;
    let metadata = file
        .metadata()
        .map_err(|_| LocalDaemonError::UnsafeStateLock)?;
    if !metadata.file_type().is_file()
        || metadata.uid() != owner_uid
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(LocalDaemonError::UnsafeStateLock);
    }
    file.try_lock_exclusive()
        .map_err(|error| match error.kind() {
            io::ErrorKind::WouldBlock => LocalDaemonError::AlreadyRunning,
            _ => LocalDaemonError::UnsafeStateLock,
        })?;
    Ok(file)
}

fn prepare_parent(parent: &Path, owner_uid: u32) -> Result<(), LocalDaemonError> {
    if !parent.exists() {
        std::fs::create_dir_all(parent)?;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
    }
    let metadata = std::fs::symlink_metadata(parent)?;
    if !metadata.file_type().is_dir()
        || metadata.file_type().is_symlink()
        || metadata.uid() != owner_uid
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(LocalDaemonError::UnsafeStateDirectory);
    }
    Ok(())
}

fn remove_owned_stale_socket(path: &Path, owner_uid: u32) -> Result<(), LocalDaemonError> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if !metadata.file_type().is_socket()
        || metadata.file_type().is_symlink()
        || metadata.uid() != owner_uid
    {
        return Err(LocalDaemonError::UnsafeExistingSocket);
    }
    std::fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    use async_trait::async_trait;
    use protocol::connection::{ConnectionError, ConnectionRequest, ConnectionResult};
    use protocol::event::{EventError, EventRequest, EventResult};
    use protocol::operation::{
        ApprovalPosture, EffectClass, OperationError, OperationRequest, OperationResult,
        OperationSummary, OwnerContext, ResponseStatus, SearchRequest,
    };
    use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _};
    use tokio::sync::oneshot;

    use super::*;

    static SEQUENCE: AtomicU64 = AtomicU64::new(1);

    #[derive(Default)]
    struct SyntheticBackend {
        shutdown: AtomicBool,
        connection_called: AtomicBool,
        event_called: AtomicBool,
    }

    #[async_trait]
    impl ConnectorBackend for SyntheticBackend {
        async fn ready(&self) -> Result<(), service::BackendReadinessError> {
            // This process-local transport test backend has no configured dependency.
            Ok(())
        }

        async fn handle(
            &self,
            _context: &PrincipalContext,
            request: OperationRequest,
        ) -> Result<OperationResult, OperationError> {
            assert!(matches!(
                request,
                OperationRequest::Search(SearchRequest { .. })
            ));
            Ok(OperationResult::Search {
                operations: vec![OperationSummary {
                    operation_ref: "sip.dial".to_owned(),
                    title: "Dial SIP".to_owned(),
                    effect: EffectClass::Mutating,
                    approval: ApprovalPosture::Required,
                    connections: Vec::new(),
                }],
            })
        }

        async fn handle_connection(
            &self,
            _context: &PrincipalContext,
            request: ConnectionRequest,
        ) -> Result<ConnectionResult, ConnectionError> {
            assert!(matches!(request, ConnectionRequest::Search(_)));
            self.connection_called.store(true, Ordering::Release);
            Ok(ConnectionResult::Search {
                connections: Vec::new(),
            })
        }

        async fn handle_event(
            &self,
            _context: &PrincipalContext,
            request: EventRequest,
        ) -> Result<EventResult, EventError> {
            assert!(matches!(request, EventRequest::Search(_)));
            self.event_called.store(true, Ordering::Release);
            Ok(EventResult::Search {
                channels: Vec::new(),
            })
        }

        async fn shutdown(&self) {
            self.shutdown.store(true, Ordering::Release);
        }
    }

    fn temporary_socket() -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "b10x-local-operation-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        (root.join("connectors.sock"), root)
    }

    fn context() -> OwnerContext {
        OwnerContext {
            tenant_id: "tenant-local".to_owned(),
            agent_id: "agent-dev".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "authority-1".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        }
    }

    #[tokio::test]
    async fn owner_socket_serves_one_strict_bounded_operation_frame() {
        let (socket, root) = temporary_socket();
        let backend = Arc::new(SyntheticBackend::default());
        let daemon = LocalOperationDaemon::bind(&socket, Arc::clone(&backend))
            .await
            .unwrap();
        assert_eq!(
            std::fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let serving = tokio::spawn(async move {
            daemon
                .serve_until(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        let request = RequestEnvelope {
            protocol: protocol::operation::CONTRACT.to_owned(),
            request_id: "request-1".to_owned(),
            context: context(),
            request: OperationRequest::Search(SearchRequest {
                query: "sip".to_owned(),
                limit: 10,
            }),
        };
        let mut stream = UnixStream::connect(&socket).await.unwrap();
        let mut bytes = serde_json::to_vec(&request).unwrap();
        bytes.push(b'\n');
        stream.write_all(&bytes).await.unwrap();
        let mut response = String::new();
        BufReader::new(stream)
            .read_line(&mut response)
            .await
            .unwrap();
        let response: ResponseEnvelope = serde_json::from_str(&response).unwrap();
        assert_eq!(response.status, ResponseStatus::Ok);
        response.validate().unwrap();

        shutdown_tx.send(()).unwrap();
        serving.await.unwrap();
        assert!(backend.shutdown.load(Ordering::Acquire));
        assert!(!socket.exists());
        std::fs::remove_file(root.join(".connectors.lock")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[tokio::test]
    async fn one_socket_dispatches_the_value_free_connection_and_event_contracts() {
        let backend = Arc::new(SyntheticBackend::default());
        let connection = protocol::connection::RequestEnvelope {
            protocol: protocol::connection::CONTRACT.to_owned(),
            request_id: "connection-request-1".to_owned(),
            context: context(),
            request: ConnectionRequest::Search(protocol::connection::SearchRequest {
                query: String::new(),
                limit: 10,
            }),
        };
        let bytes = serde_json::to_vec(&connection).unwrap();
        let response = dispatch_frame(&bytes, protocol::connection::CONTRACT, Arc::clone(&backend))
            .await
            .unwrap()
            .unwrap();
        let response: protocol::connection::ResponseEnvelope =
            serde_json::from_slice(&response).unwrap();
        response.validate().unwrap();

        let event = protocol::event::RequestEnvelope {
            protocol: protocol::event::CONTRACT.to_owned(),
            request_id: "event-request-1".to_owned(),
            context: context(),
            request: EventRequest::Search(protocol::event::SearchRequest {
                query: String::new(),
                limit: 10,
            }),
        };
        let bytes = serde_json::to_vec(&event).unwrap();
        let response = dispatch_frame(&bytes, protocol::event::CONTRACT, Arc::clone(&backend))
            .await
            .unwrap()
            .unwrap();
        let response: protocol::event::ResponseEnvelope =
            serde_json::from_slice(&response).unwrap();
        response.validate().unwrap();

        assert!(backend.connection_called.load(Ordering::Acquire));
        assert!(backend.event_called.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn a_second_daemon_cannot_unlink_the_live_daemons_socket() {
        let (socket, root) = temporary_socket();
        let first = LocalOperationDaemon::bind(&socket, Arc::new(SyntheticBackend::default()))
            .await
            .unwrap();
        assert!(matches!(
            LocalOperationDaemon::bind(&socket, Arc::new(SyntheticBackend::default())).await,
            Err(LocalDaemonError::AlreadyRunning)
        ));
        assert!(socket.exists());

        drop(first);
        let replacement =
            LocalOperationDaemon::bind(&socket, Arc::new(SyntheticBackend::default()))
                .await
                .unwrap();
        drop(replacement);
        std::fs::remove_file(&socket).unwrap();
        std::fs::remove_file(root.join(".connectors.lock")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[tokio::test]
    async fn a_broad_state_directory_refuses_without_repair() {
        let (socket, root) = temporary_socket();
        std::fs::create_dir_all(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(
            LocalOperationDaemon::bind(socket, Arc::new(SyntheticBackend::default())).await,
            Err(LocalDaemonError::UnsafeStateDirectory)
        ));
        assert_eq!(
            std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o755
        );
        std::fs::remove_dir(root).unwrap();
    }
}

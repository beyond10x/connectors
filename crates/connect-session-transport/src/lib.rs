#![forbid(unsafe_code)]

//! Owner-authenticated, one-use Unix transport for Connect Session credential submission.

use std::fs;
use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::time::Duration;

use connector_secrets::Secret;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::{UnixListener, UnixStream};
use zeroize::Zeroizing;

const MAX_ENDPOINT_ID_BYTES: usize = 128;
const MAX_SUBMISSION_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CompletionTransportError {
    #[error("Connect Session endpoint policy was invalid")]
    InvalidPolicy,
    #[error("Connect Session endpoint state was unsafe")]
    UnsafeEndpoint,
    #[error("Connect Session endpoint I/O failed")]
    Io,
    #[error("Connect Session expired before an owner submission")]
    Expired,
    #[error("Connect Session credential submission was malformed")]
    InvalidSubmission,
}

/// One bound owner-only, single-use completion endpoint.
pub struct BoundCompletionEndpoint {
    listener: UnixListener,
    path: PathBuf,
}

impl BoundCompletionEndpoint {
    /// Bind a new endpoint below an owner-only directory, refusing every pre-existing path.
    pub fn bind(directory: &Path, endpoint_id: &str) -> Result<Self, CompletionTransportError> {
        if endpoint_id.is_empty()
            || endpoint_id.len() > MAX_ENDPOINT_ID_BYTES
            || !endpoint_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(CompletionTransportError::InvalidPolicy);
        }
        ensure_owner_directory(directory)?;
        let path = directory.join(format!("{endpoint_id}.sock"));
        if fs::symlink_metadata(&path).is_ok() {
            return Err(CompletionTransportError::UnsafeEndpoint);
        }
        let listener = UnixListener::bind(&path).map_err(|_| CompletionTransportError::Io)?;
        if fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).is_err() {
            let _ = fs::remove_file(&path);
            return Err(CompletionTransportError::Io);
        }
        Ok(Self { listener, path })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Wait for the process owner, read exactly one bounded line, and retire the endpoint.
    pub async fn receive(
        self,
        accept_deadline: Duration,
        read_deadline: Duration,
        maximum_secret_bytes: usize,
    ) -> Result<CompletionSubmission, CompletionTransportError> {
        if accept_deadline.is_zero()
            || read_deadline.is_zero()
            || maximum_secret_bytes == 0
            || maximum_secret_bytes > MAX_SUBMISSION_BYTES
        {
            return Err(CompletionTransportError::InvalidPolicy);
        }
        let accepted = tokio::time::timeout(accept_deadline, accept_owner(&self.listener)).await;
        let mut stream = match accepted {
            Ok(result) => result?,
            Err(_) => return Err(CompletionTransportError::Expired),
        };
        let mut bytes = Zeroizing::new(Vec::with_capacity(maximum_secret_bytes.min(256)));
        {
            let reader = BufReader::new(&mut stream);
            let mut bounded = reader.take((maximum_secret_bytes + 3) as u64);
            tokio::time::timeout(read_deadline, bounded.read_until(b'\n', &mut bytes))
                .await
                .map_err(|_| CompletionTransportError::InvalidSubmission)?
                .map_err(|_| CompletionTransportError::Io)?;
        }
        if bytes.last() != Some(&b'\n') || bytes.len() > maximum_secret_bytes + 2 {
            return Err(CompletionTransportError::InvalidSubmission);
        }
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
        let value =
            std::str::from_utf8(&bytes).map_err(|_| CompletionTransportError::InvalidSubmission)?;
        if value.is_empty()
            || value.len() > maximum_secret_bytes
            || value
                .bytes()
                .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
        {
            return Err(CompletionTransportError::InvalidSubmission);
        }
        let value = String::from_utf8(std::mem::take(&mut *bytes))
            .map_err(|_| CompletionTransportError::InvalidSubmission)?;
        Ok(CompletionSubmission {
            secret: Secret::new(value),
            stream,
        })
    }
}

impl Drop for BoundCompletionEndpoint {
    fn drop(&mut self) {
        let _ = remove_endpoint(&self.path);
    }
}

/// Owner-authenticated secret submission whose acknowledgement remains transport-owned.
pub struct CompletionSubmission {
    secret: Secret,
    stream: UnixStream,
}

impl CompletionSubmission {
    #[must_use]
    pub fn secret(&self) -> &Secret {
        &self.secret
    }

    #[must_use]
    pub fn into_secret(self) -> Secret {
        self.secret
    }

    pub async fn respond(mut self, accepted: bool) -> Result<(), CompletionTransportError> {
        let response = if accepted {
            b"{\"accepted\":true}\n".as_slice()
        } else {
            b"{\"accepted\":false}\n".as_slice()
        };
        self.stream
            .write_all(response)
            .await
            .map_err(|_| CompletionTransportError::Io)?;
        self.stream
            .shutdown()
            .await
            .map_err(|_| CompletionTransportError::Io)
    }
}

pub fn remove_endpoint(path: &Path) -> Result<(), CompletionTransportError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(CompletionTransportError::Io),
    };
    if !metadata.file_type().is_socket() || metadata.uid() != rustix::process::geteuid().as_raw() {
        return Err(CompletionTransportError::UnsafeEndpoint);
    }
    fs::remove_file(path).map_err(|_| CompletionTransportError::Io)
}

fn ensure_owner_directory(path: &Path) -> Result<(), CompletionTransportError> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(|_| CompletionTransportError::Io)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| CompletionTransportError::Io)?;
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| CompletionTransportError::Io)?;
    if !metadata.file_type().is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(CompletionTransportError::UnsafeEndpoint);
    }
    Ok(())
}

async fn accept_owner(listener: &UnixListener) -> Result<UnixStream, CompletionTransportError> {
    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|_| CompletionTransportError::Io)?;
        let credential = stream
            .peer_cred()
            .map_err(|_| CompletionTransportError::Io)?;
        if credential.uid() == rustix::process::geteuid().as_raw() {
            return Ok(stream);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn endpoint_is_owner_only_one_use_and_removed_after_submission() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let endpoint = BoundCompletionEndpoint::bind(root.path(), "session-one").unwrap();
        let path = endpoint.path().to_path_buf();
        assert_eq!(
            fs::symlink_metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let client = tokio::spawn({
            let path = path.clone();
            async move {
                let mut stream = UnixStream::connect(path).await.unwrap();
                stream
                    .write_all(b"SENTINEL-NOT-A-REAL-SECRET\n")
                    .await
                    .unwrap();
                let mut response = String::new();
                BufReader::new(stream)
                    .read_line(&mut response)
                    .await
                    .unwrap();
                response
            }
        });
        let submission = endpoint
            .receive(Duration::from_secs(1), Duration::from_secs(1), 128)
            .await
            .unwrap();
        assert_eq!(
            submission.secret().expose_secret(),
            "SENTINEL-NOT-A-REAL-SECRET"
        );
        submission.respond(true).await.unwrap();
        assert_eq!(client.await.unwrap(), "{\"accepted\":true}\n");
        assert!(!path.exists());
    }

    #[test]
    fn unsafe_directory_refuses() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(
            BoundCompletionEndpoint::bind(root.path(), "session-one"),
            Err(CompletionTransportError::UnsafeEndpoint)
        ));
    }
}

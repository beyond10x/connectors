use async_trait::async_trait;
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::{Credential, Secret, SecretStore};
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::{
    ffi::OsStrExt,
    fs::{MetadataExt, OpenOptionsExt},
};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CredentialRef {
    Environment {
        #[schemars(length(min = 1, max = 512))]
        name: String,
    },
    File {
        #[schemars(length(min = 1, max = 4096))]
        path: PathBuf,
    },
}

#[async_trait]
impl Credential for CredentialRef {
    async fn resolve(&self) -> Result<Secret> {
        match self {
            Self::Environment { name } => {
                let value = std::env::var_os(name).ok_or_else(|| {
                    Error::new(
                        ErrorCode::Unauthorized,
                        "required credential is unavailable",
                    )
                })?;
                #[cfg(unix)]
                let value = value.as_os_str().as_bytes().to_vec();
                // Windows environment values are UTF-16; a value that is not
                // valid Unicode is not a credential.
                #[cfg(windows)]
                let value = value
                    .into_string()
                    .map_err(|_| {
                        Error::new(
                            ErrorCode::Unauthorized,
                            "required credential is unavailable",
                        )
                    })?
                    .into_bytes();
                bounded_secret(value)
            }
            Self::File { path } => read_secret_file(path),
        }
    }
}

fn bounded_secret(mut value: Vec<u8>) -> Result<Secret> {
    if value.last() == Some(&b'\n') {
        value.pop();
        if value.last() == Some(&b'\r') {
            value.pop();
        }
    }
    if value.is_empty() || value.len() > 8192 {
        return Err(Error::new(
            ErrorCode::Unauthorized,
            "credential is empty or exceeds its limit",
        ));
    }
    Ok(Secret(value))
}

/// Windows: the same rule on the opened handle, the owner SID and access
/// control list in place of uid and mode bits; a final reparse point refuses.
#[cfg(windows)]
fn read_secret_file(path: &Path) -> Result<Secret> {
    use crate::local::filesystem::windows;
    let mut file = windows::open_no_follow(path)
        .map_err(|_| Error::new(ErrorCode::Unavailable, "credential file unavailable"))?;
    let metadata = file.metadata().map_err(|_| Error::unavailable())?;
    if !metadata.is_file() || windows::check_owner_only(&file).is_err() || metadata.len() > 8192 {
        return Err(Error::new(
            ErrorCode::Forbidden,
            "credential file must be private, owner-bound and bounded",
        ));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(8193)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::unavailable())?;
    bounded_secret(bytes)
}

#[cfg(unix)]
fn read_secret_file(path: &Path) -> Result<Secret> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| Error::new(ErrorCode::Unavailable, "credential file unavailable"))?;
    let metadata = file.metadata().map_err(|_| Error::unavailable())?;
    // SAFETY: geteuid has no preconditions and does not mutate process state.
    if !metadata.is_file()
        || metadata.mode() & 0o077 != 0
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.len() > 8192
    {
        return Err(Error::new(
            ErrorCode::Forbidden,
            "credential file must be private, owner-bound and bounded",
        ));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(8193)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::unavailable())?;
    bounded_secret(bytes)
}

pub struct MemorySecrets(pub BTreeMap<String, Vec<u8>>);
#[async_trait]
impl SecretStore for MemorySecrets {
    async fn read(&self, reference: &str) -> Result<Secret> {
        self.0.get(reference).cloned().map(Secret).ok_or_else(|| {
            Error::new(
                ErrorCode::Unauthorized,
                "credential reference is unavailable",
            )
        })
    }
}

pub struct DirectorySecrets(pub PathBuf);
#[async_trait]
impl SecretStore for DirectorySecrets {
    async fn read(&self, reference: &str) -> Result<Secret> {
        if !connectors_core::valid_id(reference) || reference == "." || reference == ".." {
            return Err(Error::invalid("invalid credential reference"));
        }
        read_secret_file(&self.0.join(reference))
    }
}

pub struct BoundCredential {
    pub store: Arc<dyn SecretStore>,
    pub reference: String,
}
#[async_trait]
impl Credential for BoundCredential {
    async fn resolve(&self) -> Result<Secret> {
        self.store.read(&self.reference).await
    }
}

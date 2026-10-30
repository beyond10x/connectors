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

/// The TokenReview instance token (`token_review.credential_file`), read at
/// each use. It is read as every other credential file, and is also admitted
/// at exactly `0640` or `0440` under a group the process holds, as the kubelet presents a
/// projected token under `fsGroup`. No other credential file admits group
/// read.
#[derive(Clone, Debug)]
pub(crate) struct ReviewTokenFile(pub(crate) PathBuf);

#[async_trait]
impl Credential for ReviewTokenFile {
    async fn resolve(&self) -> Result<Secret> {
        #[cfg(unix)]
        let secret = read_file(&self.0, Readers::OwnerAndGroup);
        #[cfg(windows)]
        let secret = read_secret_file(&self.0);
        secret
    }
}

/// Who besides its owner may read a credential file.
#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Readers {
    /// No bit for group or others, owned by the effective user.
    Owner,
    /// [`Self::Owner`], or exactly one of [`GROUP_READ_MODES`], owned by the
    /// effective user or root, its group the effective or a supplementary
    /// group of the process.
    OwnerAndGroup,
}

/// The modes [`Readers::OwnerAndGroup`] admits beside the private rule.
#[cfg(unix)]
const GROUP_READ_MODES: [u32; 2] = [0o640, 0o440];

#[cfg(unix)]
fn read_secret_file(path: &Path) -> Result<Secret> {
    read_file(path, Readers::Owner)
}

#[cfg(unix)]
fn read_file(path: &Path, readers: Readers) -> Result<Secret> {
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| Error::new(ErrorCode::Unavailable, "credential file unavailable"))?;
    let metadata = file.metadata().map_err(|_| Error::unavailable())?;
    if !metadata.is_file()
        || metadata.len() > 8192
        || !admitted(
            metadata.mode(),
            metadata.uid(),
            metadata.gid(),
            readers,
            &Process::current,
        )
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

/// Whether a file of `mode`, `uid` and `gid` may be read under `readers` by
/// the process `process` describes; it is asked only for group read.
#[cfg(unix)]
fn admitted(
    mode: u32,
    uid: u32,
    gid: u32,
    readers: Readers,
    process: &dyn Fn() -> Option<Process>,
) -> bool {
    let shared = mode & 0o077;
    if shared == 0 {
        // SAFETY: geteuid has no preconditions and does not mutate process state.
        return uid == unsafe { libc::geteuid() };
    }
    // Group read is exactly `0640` or `0440`: no special bit, no owner
    // execute, nothing else for group or others.
    readers == Readers::OwnerAndGroup
        && GROUP_READ_MODES.contains(&(mode & 0o7777))
        && process().is_some_and(|process| {
            (uid == 0 || uid == process.uid)
                && (gid == process.gid || process.groups.contains(&gid))
        })
}

/// The effective user, effective group and supplementary groups.
#[cfg(unix)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct Process {
    uid: u32,
    gid: u32,
    groups: Vec<u32>,
}

#[cfg(unix)]
impl Process {
    /// `None` when the supplementary groups cannot be read, which admits no
    /// group read.
    fn current() -> Option<Self> {
        // SAFETY: geteuid, getegid and a sizing getgroups have no preconditions.
        let (uid, gid, count) = unsafe {
            (
                libc::geteuid(),
                libc::getegid(),
                libc::getgroups(0, std::ptr::null_mut()),
            )
        };
        let mut groups = vec![0; usize::try_from(count).ok()?];
        // SAFETY: the buffer holds `count` entries.
        let written = unsafe { libc::getgroups(count, groups.as_mut_ptr()) };
        groups.truncate(usize::try_from(written).ok()?);
        Some(Self { uid, gid, groups })
    }
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    /// A process whose effective user is this one's, with effective group
    /// 100 and supplementary groups 200 and 300.
    fn process() -> Option<Process> {
        Some(Process {
            // SAFETY: geteuid has no preconditions.
            uid: unsafe { libc::geteuid() },
            gid: 100,
            groups: vec![200, 300],
        })
    }

    /// D275: the review token's group read, decided on the file's mode,
    /// owner and group against the process identity; every other credential
    /// file keeps the private rule.
    #[test]
    fn credentials_admit_group_read_only_for_the_review_token_under_a_held_group() {
        let me = process().unwrap().uid;
        let other = me.wrapping_add(1).max(1);
        let review = Readers::OwnerAndGroup;
        // Private and owned by the effective user: admitted under both rules.
        for readers in [Readers::Owner, review] {
            assert!(admitted(0o100600, me, 999, readers, &process));
            assert!(admitted(0o100400, me, 999, readers, &process));
            assert!(!admitted(0o100600, other, 100, readers, &process));
        }
        // Group read under the effective or a supplementary group, owned by
        // the effective user or root.
        for gid in [100, 200, 300] {
            for uid in [me, 0] {
                assert!(
                    admitted(0o100640, uid, gid, review, &process),
                    "{uid}:{gid}"
                );
                assert!(
                    admitted(0o100440, uid, gid, review, &process),
                    "{uid}:{gid}"
                );
                assert!(!admitted(0o100640, uid, gid, Readers::Owner, &process));
            }
        }
        if me != 0 {
            assert!(!admitted(0o100640, other, 100, review, &process));
        }
        // A group the process does not hold.
        assert!(!admitted(0o100640, me, 999, review, &process));
        assert!(!admitted(0o100640, 0, 999, review, &process));
        // Group write, group execute, any bit for others.
        for mode in [
            0o100660, 0o100620, 0o100650, 0o100644, 0o100604, 0o100641, 0o100670,
        ] {
            assert!(!admitted(mode, me, 100, review, &process), "{mode:o}");
        }
        // Supplementary groups that cannot be read admit no group read.
        assert!(!admitted(0o100640, me, 100, review, &|| None));
    }

    /// D275 correction: every one of the 4096 permission modes, under a group
    /// the process holds. The review token admits a mode with no bit for
    /// group or others, as every credential file does, or exactly `0640` or
    /// `0440`; special bits and owner execute beside group read refuse.
    #[test]
    fn credentials_admit_group_read_at_exactly_0640_or_0440() {
        let me = process().unwrap().uid;
        for mode in 0..=0o7777u32 {
            let private = mode & 0o077 == 0;
            let file = 0o100000 | mode;
            assert_eq!(
                admitted(file, me, 100, Readers::Owner, &process),
                private,
                "owner rule, {mode:o}"
            );
            assert_eq!(
                admitted(file, me, 100, Readers::OwnerAndGroup, &process),
                private || mode == 0o640 || mode == 0o440,
                "review token, {mode:o}"
            );
        }
    }

    /// The identity read from this process holds its effective group.
    #[test]
    fn credentials_read_the_process_identity() {
        let process = Process::current().unwrap();
        // SAFETY: geteuid and getegid have no preconditions.
        assert_eq!(process.uid, unsafe { libc::geteuid() });
        assert_eq!(process.gid, unsafe { libc::getegid() });
    }
}

//! The Windows binding: generic credentials in the user's Credential Manager,
//! persisted on the local machine and never roaming. The target name is
//! `connectors/` and the purpose, scope and version ids
//! ([`Version::item_name`]). A credential blob holds at most 2,560 bytes
//! (`CRED_MAX_CREDENTIAL_BLOB_SIZE`); a larger value is `InvalidMaterial`
//! before any call, and is never split or written elsewhere.
use crate::{
    Backend, CustodyBackend, Failure, Material, Result, Scope, State, Version, WINDOWS_MAX_BYTES,
    admit_material,
};
use std::time::Instant;
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_NO_SUCH_LOGON_SESSION, ERROR_NOT_FOUND, GetLastError, HANDLE,
        WAIT_ABANDONED, WAIT_OBJECT_0,
    },
    Security::Credentials::{
        CRED_MAX_CREDENTIAL_BLOB_SIZE, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW,
        CredDeleteW, CredFree, CredReadW, CredWriteW,
    },
    System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject},
};

const _: () = assert!(CRED_MAX_CREDENTIAL_BLOB_SIZE as usize == WINDOWS_MAX_BYTES);

/// Every target name starts with this prefix.
pub const TARGET_PREFIX: &str = "connectors/";
/// Serializes custody writes and deletions of this user's logon session, so
/// the check for an existing version and the write that follows are one step.
const MUTEX: &str = "Local\\connectors-custody";
const PROBE: &str = "connectors/state-probe";

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn target(version: Version) -> Vec<u16> {
    wide(&format!("{TARGET_PREFIX}{}", version.item_name()))
}

/// The session-wide custody mutex, held until dropped. An abandoned mutex (a
/// holder that exited) is acquired: the store itself is consistent per call.
struct SessionLock(HANDLE);

impl SessionLock {
    fn acquire(until: Instant) -> Result<Self> {
        let name = wide(MUTEX);
        // SAFETY: name is NUL-terminated and outlives the call.
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(Failure::Unavailable);
        }
        let wait = until
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(u128::from(u32::MAX - 1)) as u32;
        // SAFETY: handle is a live mutex handle owned here.
        let result = unsafe { WaitForSingleObject(handle, wait) };
        if result == WAIT_OBJECT_0 || result == WAIT_ABANDONED {
            Ok(Self(handle))
        } else {
            // SAFETY: handle is owned here and not used again.
            unsafe { CloseHandle(handle) };
            Err(Failure::Unavailable)
        }
    }
}

impl Drop for SessionLock {
    fn drop(&mut self) {
        // SAFETY: this thread owns the mutex and its handle.
        unsafe {
            ReleaseMutex(self.0);
            CloseHandle(self.0);
        }
    }
}

enum Read {
    Found(Material),
    Missing,
}

/// Read one generic credential, copying its blob out and zeroing the buffer
/// the API returned before it is freed.
fn read_target(name: &[u16]) -> Result<Read> {
    let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: name is NUL-terminated; credential is a valid output location.
    if unsafe { CredReadW(name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) } == 0 {
        // SAFETY: no preconditions.
        return match unsafe { GetLastError() } {
            ERROR_NOT_FOUND => Ok(Read::Missing),
            _ => Err(Failure::Unavailable),
        };
    }
    // SAFETY: CredReadW succeeded; the structure is valid until CredFree.
    let result = unsafe {
        let record = &*credential;
        let size = record.CredentialBlobSize as usize;
        let outcome =
            if record.Type != CRED_TYPE_GENERIC || record.Persist != CRED_PERSIST_LOCAL_MACHINE {
                Err(Failure::Denied)
            } else if record.CredentialBlob.is_null() || size == 0 || size > WINDOWS_MAX_BYTES {
                Err(Failure::Unavailable)
            } else {
                Ok(Read::Found(Material::from_slice(
                    std::slice::from_raw_parts(record.CredentialBlob, size),
                )))
            };
        if !record.CredentialBlob.is_null() {
            std::ptr::write_bytes(record.CredentialBlob, 0, size);
        }
        outcome
    };
    // SAFETY: credential was allocated by CredReadW and is freed once.
    unsafe { CredFree(credential.cast()) };
    result
}

/// The Credential Manager opened for one scope.
#[derive(Debug)]
pub struct CredentialManager {
    scope: Scope,
}

impl CredentialManager {
    /// Opening touches nothing; the store is reached per call.
    pub fn open(scope: Scope) -> Result<Self> {
        Ok(Self { scope })
    }

    /// Passive state: a read of a name custody never writes. A session with
    /// no credential store answers `Unavailable`.
    pub fn inspect() -> State {
        match read_target(&wide(PROBE)) {
            Ok(_) => State::Available,
            Err(_) => State::Unavailable,
        }
    }
}

impl Backend for CredentialManager {
    fn kind(&self) -> CustodyBackend {
        CustodyBackend::WindowsCredentialManager
    }

    fn scope(&self) -> Scope {
        self.scope
    }

    fn state(&self) -> State {
        Self::inspect()
    }

    fn admit(&self, version: Version) -> Result<()> {
        if version.scope() != self.scope {
            return Err(Failure::Denied);
        }
        if self.state() != State::Available {
            return Err(Failure::Unavailable);
        }
        Ok(())
    }

    fn write_new(
        &self,
        version: Version,
        material: &Material,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        // The bound is checked before any call: nothing is written for a
        // larger value.
        admit_material(self.scope, material, WINDOWS_MAX_BYTES)?;
        let _lock = SessionLock::acquire(until)?;
        self.admit(version)?;
        guard()?;
        let mut name = target(version);
        if let Read::Found(_) = read_target(&name)? {
            return Err(Failure::Conflict);
        }
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: name.as_mut_ptr(),
            CredentialBlobSize: material.len() as u32,
            CredentialBlob: material.as_bytes().as_ptr().cast_mut(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..Default::default()
        };
        // SAFETY: every pointer in credential is valid for the call; the API
        // copies the blob and does not write through it.
        if unsafe { CredWriteW(&credential, 0) } == 0 {
            // SAFETY: no preconditions.
            return Err(match unsafe { GetLastError() } {
                ERROR_NO_SUCH_LOGON_SESSION => Failure::Unavailable,
                _ => Failure::OutcomeUnknown,
            });
        }
        match read_target(&name) {
            Ok(Read::Found(retained)) if retained.ct_eq(material.as_bytes()) => Ok(()),
            _ => Err(Failure::OutcomeUnknown),
        }
    }

    fn read(&self, version: Version) -> Result<Material> {
        self.admit(version)?;
        match read_target(&target(version))? {
            Read::Found(material) => Ok(material),
            Read::Missing => Err(Failure::Missing),
        }
    }

    fn confirm(
        &self,
        version: Version,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
        validate: impl FnOnce(&Material) -> Result<()>,
    ) -> Result<()> {
        let _lock = SessionLock::acquire(until)?;
        self.admit(version)?;
        guard()?;
        let material = self.read(version)?;
        admit_material(self.scope, &material, WINDOWS_MAX_BYTES)?;
        validate(&material)
    }

    fn delete(
        &self,
        version: Version,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        let _lock = SessionLock::acquire(until)?;
        self.admit(version)?;
        guard()?;
        let name = target(version);
        // SAFETY: name is NUL-terminated and outlives the call.
        if unsafe { CredDeleteW(name.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0 {
            // SAFETY: no preconditions.
            if unsafe { GetLastError() } != ERROR_NOT_FOUND {
                return Err(Failure::OutcomeUnknown);
            }
        }
        match read_target(&name) {
            Ok(Read::Missing) => Ok(()),
            _ => Err(Failure::OutcomeUnknown),
        }
    }
}

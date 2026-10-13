//! The Windows binding: generic credentials in the user's Credential Manager,
//! persisted on the local machine and never roaming. A credential blob holds
//! at most 2,560 bytes (`CRED_MAX_CREDENTIAL_BLOB_SIZE`), so every version is
//! stored in the split layout of [`crate::parts`]: numbered part entries
//! written first, then a header entry naming the length, the part count and
//! the SHA-256, written last. The header's target name is `connectors/` and
//! the purpose, scope and version ids ([`Version::item_name`]); a part's
//! appends `/part/` and its number. The bound is 64 KiB, as on every platform.
//!
//! This module is the entry store only; the layout, its ordering and its crash
//! handling are platform-neutral and tested everywhere.
use crate::{
    Failure, Material, Result, Scope, State,
    parts::{Entries, PART_BYTES, Parted},
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

/// Failing Credential Manager calls and their `GetLastError` codes, kept in
/// tests only: `Failure` carries no diagnostics, and a test that sees
/// `OutcomeUnknown` prints these to name the call that failed.
#[cfg(test)]
pub(crate) static OS_ERRORS: std::sync::Mutex<Vec<(&str, u32)>> = std::sync::Mutex::new(Vec::new());

#[cfg(test)]
fn record(call: &'static str, code: u32) {
    if let Ok(mut errors) = OS_ERRORS.lock() {
        errors.push((call, code));
    }
}

pub use crate::parts::TARGET_PREFIX;

const _: () = assert!(CRED_MAX_CREDENTIAL_BLOB_SIZE as usize == PART_BYTES);

/// Serializes custody writes and deletions of this user's logon session, so
/// the check for an existing version and the writes that follow are one step.
const MUTEX: &str = "Local\\connectors-custody";
const PROBE: &str = "connectors/state-probe";

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The session-wide custody mutex, held until dropped. An abandoned mutex (a
/// holder that exited) is acquired: the store itself is consistent per call,
/// and the layout leaves a header-less remainder for a delete to reclaim.
pub struct SessionLock(HANDLE);

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

/// Read one generic credential, copying its blob out and zeroing the buffer
/// the API returned before it is freed.
fn read_target(name: &[u16]) -> Result<Option<Material>> {
    let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: name is NUL-terminated; credential is a valid output location.
    if unsafe { CredReadW(name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) } == 0 {
        // SAFETY: no preconditions.
        let code = unsafe { GetLastError() };
        #[cfg(test)]
        if code != ERROR_NOT_FOUND {
            record("CredReadW", code);
        }
        return match code {
            ERROR_NOT_FOUND => Ok(None),
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
            } else if record.CredentialBlob.is_null() || size == 0 || size > PART_BYTES {
                Err(Failure::Unavailable)
            } else {
                Ok(Some(Material::from_slice(std::slice::from_raw_parts(
                    record.CredentialBlob,
                    size,
                ))))
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

/// The user's Credential Manager as an entry store.
#[derive(Clone, Copy, Debug, Default)]
pub struct CredentialEntries;

impl Entries for CredentialEntries {
    type Lock<'a> = SessionLock;

    fn lock(&self, until: Instant) -> Result<SessionLock> {
        SessionLock::acquire(until)
    }

    /// Passive state: a read of a name custody never writes. A session with
    /// no credential store answers `Unavailable`.
    fn state(&self) -> State {
        match read_target(&wide(PROBE)) {
            Ok(_) => State::Available,
            Err(_) => State::Unavailable,
        }
    }

    fn read(&self, name: &str) -> Result<Option<Material>> {
        read_target(&wide(name))
    }

    fn write(&self, name: &str, blob: &[u8]) -> Result<()> {
        if blob.is_empty() || blob.len() > PART_BYTES {
            return Err(Failure::InvalidMaterial);
        }
        let mut name = wide(name);
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: name.as_mut_ptr(),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_ptr().cast_mut(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..Default::default()
        };
        // SAFETY: every pointer in credential is valid for the call; the API
        // copies the blob and does not write through it.
        if unsafe { CredWriteW(&credential, 0) } == 0 {
            // SAFETY: no preconditions.
            let code = unsafe { GetLastError() };
            #[cfg(test)]
            record("CredWriteW", code);
            return Err(match code {
                ERROR_NO_SUCH_LOGON_SESSION => Failure::Unavailable,
                _ => Failure::OutcomeUnknown,
            });
        }
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<()> {
        let name = wide(name);
        // SAFETY: name is NUL-terminated and outlives the call.
        if unsafe { CredDeleteW(name.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0 {
            // SAFETY: no preconditions.
            let code = unsafe { GetLastError() };
            if code != ERROR_NOT_FOUND {
                #[cfg(test)]
                record("CredDeleteW", code);
                return Err(Failure::OutcomeUnknown);
            }
        }
        Ok(())
    }
}

/// The Credential Manager opened for one scope.
pub type CredentialManager = Parted<CredentialEntries>;

impl Parted<CredentialEntries> {
    /// Opening touches nothing; the store is reached per call.
    pub fn open(scope: Scope) -> Result<Self> {
        Ok(Self::new(CredentialEntries, scope))
    }

    /// Passive state of the user's Credential Manager.
    pub fn inspect() -> State {
        CredentialEntries.state()
    }
}

//! The host's custody coordinator over the selected binding: immutable, scoped
//! versions with definite receipts. The bindings themselves (the Secret
//! Service on Linux, docs/local-secret-service.md, and the explicitly selected
//! read-only mounted directory, docs/local-mounted-secret-custody.md) are
//! `connectors-custody`; this module keeps the host's in-process writer lock,
//! the lock deadline it shares with the lifecycle lock, and the receipts the
//! registry's fences consume. No public management endpoint exposes a binding
//! or its private locators.
#[cfg(target_os = "linux")]
use connectors_custody::mounted::{self, MountedSecret};
use connectors_custody::{Backend, Material, Platform, Purpose, ReadBackend};
use connectors_sdk::Secret;
use std::path::{Path, PathBuf};

use super::super::config::Selection;
#[cfg(target_os = "linux")]
pub use connectors_custody::mounted::Item;
pub use connectors_custody::{Failure, Scope, Version};
type Result<T> = std::result::Result<T, Failure>;

/// The mounted binding's own directory in the state directory: its pin key
/// and one pin record per version, never the credential bytes.
pub const MOUNTED_PINS: &str = "custody-mounted";

/// A definite backend acknowledgement, never constructed from caller JSON.
/// Under the read-only binding it acknowledges a checked, pinned read.
pub struct WrittenVersion {
    version: Version,
}
impl WrittenVersion {
    pub(crate) fn matches(&self, version: Version) -> bool {
        self.version == version
    }
    #[cfg(test)]
    pub(crate) fn fixture(version: Version) -> Self {
        Self { version }
    }
}
pub struct DeletedVersion {
    version: Version,
}
impl DeletedVersion {
    pub(crate) fn matches(&self, version: Version) -> bool {
        self.version == version
    }
}

enum Binding {
    Platform(Platform),
    #[cfg(target_os = "linux")]
    Mounted(MountedSecret),
}

/// Persistent custody for one admitted scope through the selected binding.
/// No activation, unlock, prompt, or fallback occurs.
pub struct Store {
    binding: Binding,
    writer: std::sync::Mutex<()>,
}

impl Store {
    pub fn open(scope: Scope) -> Result<Self> {
        Self::open_at(scope, None)
    }
    /// The platform binding, with the Secret Service bus `socket` on Linux.
    pub fn open_at(scope: Scope, socket: Option<&Path>) -> Result<Self> {
        connectors_custody::open(scope, socket).map(Self::from_backend)
    }
    /// The binding `selection` names, for the state directory `state`.
    pub fn open_for(scope: Scope, selection: &Selection, state: &Path) -> Result<Self> {
        match selection {
            Selection::Platform {
                secret_service_socket,
            } => Self::open_at(scope, secret_service_socket.as_deref()),
            #[cfg(target_os = "linux")]
            Selection::Mounted { directory } => MountedSecret::open(scope, directory, &pins(state))
                .map(|binding| Self {
                    binding: Binding::Mounted(binding),
                    writer: std::sync::Mutex::new(()),
                }),
            #[cfg(not(target_os = "linux"))]
            Selection::Mounted { .. } => {
                let _ = state;
                Err(Failure::Unavailable)
            }
        }
    }

    fn from_backend(backend: Platform) -> Self {
        Self {
            binding: Binding::Platform(backend),
            writer: std::sync::Mutex::new(()),
        }
    }

    fn reader(&self) -> &dyn ReadBackend {
        match &self.binding {
            Binding::Platform(backend) => backend,
            #[cfg(target_os = "linux")]
            Binding::Mounted(backend) => backend,
        }
    }

    /// The writable binding, or `ReadOnly`.
    fn writer(&self) -> Result<&Platform> {
        match &self.binding {
            Binding::Platform(backend) => Ok(backend),
            #[cfg(target_os = "linux")]
            Binding::Mounted(_) => Err(Failure::ReadOnly),
        }
    }

    #[cfg(target_os = "linux")]
    fn mounted(&self) -> Result<&MountedSecret> {
        match &self.binding {
            Binding::Mounted(backend) => Ok(backend),
            Binding::Platform(_) => Err(Failure::Denied),
        }
    }

    /// Whether the binding writes nothing (`mounted_secret`).
    pub fn read_only(&self) -> bool {
        !self.reader().kind().writable()
    }

    /// The binding's write: a definite acknowledgement after the exact bytes
    /// were read back and made durable. Any failure once the store may have
    /// changed is OutcomeUnknown; it is never retried here, an existing version
    /// is never overwritten or silently accepted. A read-only binding is
    /// `ReadOnly` before anything is written.
    pub(crate) fn write_new_guarded(
        &self,
        version: Version,
        material: &Secret,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<WrittenVersion> {
        let backend = self.writer()?;
        backend.admit(version)?;
        let material = Material::from_slice(&material.0);
        connectors_custody::admit_material(backend.scope(), &material, backend.max_bytes())?;
        let until = lock_until();
        let _writer = self.writer_lock(until)?;
        // The coordinator rechecks publication/retirement in the guard while the
        // binding's physical lock excludes deletion.
        backend.write_new(version, &material, until, guard)?;
        Ok(WrittenVersion { version })
    }

    /// The read-only binding's acknowledgement: `item` was read once under
    /// every check and its bytes, `material`, are pinned to `version` while the
    /// coordinator's guard holds. Nothing is written to the mounted directory.
    /// A platform binding is `Denied`: its versions are written.
    #[cfg(target_os = "linux")]
    pub(crate) fn pin_guarded(
        &self,
        version: Version,
        item: &Item,
        reading: &mounted::Reading,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<WrittenVersion> {
        let backend = self.mounted()?;
        let until = lock_until();
        let _writer = self.writer_lock(until)?;
        backend.pin(version, item, reading, guard)?;
        Ok(WrittenVersion { version })
    }

    #[cfg(all(test, target_os = "linux"))]
    fn write_new(&self, version: Version, material: &Secret) -> Result<()> {
        self.write_new_guarded(version, material, || Ok(()))
            .map(|_| ())
    }

    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn fail_synchronization(&self, fail: bool) {
        self.backend().fail_synchronization(fail);
    }

    /// The binding, for the qualification fixtures.
    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn backend(&self) -> &Platform {
        self.writer().expect("a platform binding")
    }

    pub fn read(&self, version: Version) -> Result<Secret> {
        self.reader()
            .read(version)
            .map(|material| Secret(material.into_vec()))
    }

    /// Recovery of a staged signing key only. Readback alone never converts a
    /// credential version into publication authority. The owner validates the
    /// seed's public key while the physical writer lock excludes retirement.
    pub(crate) fn confirm_signing_key_guarded(
        &self,
        version: Version,
        guard: impl FnOnce() -> Result<()>,
        validate: impl FnOnce(&Secret) -> Result<()>,
    ) -> Result<WrittenVersion> {
        let backend = self.writer()?;
        if backend.scope().purpose() != Purpose::ApprovalSigning {
            return Err(Failure::Denied);
        }
        backend.admit(version)?;
        let until = lock_until();
        let _writer = self.writer_lock(until)?;
        backend.confirm(version, until, guard, |material| {
            validate(&Secret(material.as_bytes().to_vec()))
        })?;
        Ok(WrittenVersion { version })
    }

    pub(crate) fn delete_guarded(
        &self,
        version: Version,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<DeletedVersion> {
        let backend = self.writer()?;
        backend.admit(version)?;
        let until = lock_until();
        let _writer = self.writer_lock(until)?;
        backend.delete(version, until, guard)?;
        Ok(DeletedVersion { version })
    }

    #[cfg(all(test, target_os = "linux"))]
    fn delete_for_qualification(&self, version: Version) -> Result<()> {
        self.delete_guarded(version, || Ok(())).map(|_| ())
    }

    fn writer_lock(&self, until: std::time::Instant) -> Result<std::sync::MutexGuard<'_, ()>> {
        loop {
            match self.writer.try_lock() {
                Ok(guard) => return Ok(guard),
                Err(std::sync::TryLockError::Poisoned(_)) => return Err(Failure::Unavailable),
                Err(std::sync::TryLockError::WouldBlock) => {}
            }
            if std::time::Instant::now() >= until {
                return Err(Failure::Unavailable);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

/// The mounted binding's pin directory in the state directory `state`.
pub fn pins(state: &Path) -> PathBuf {
    state.join(MOUNTED_PINS)
}

/// Read `item` from the mounted directory `selection` names, once under every
/// check, before a version names it. A platform selection is `Denied`.
#[cfg(target_os = "linux")]
pub fn read_mounted(selection: &Selection, state: &Path, item: &Item) -> Result<mounted::Reading> {
    match selection {
        Selection::Mounted { directory } => mounted::read(directory, &pins(state), item),
        Selection::Platform { .. } => Err(Failure::Denied),
    }
}

/// Passive qualification of the current custody owner and its storage.
/// Opens no secret session and reads no credential item or encrypted payload.
pub fn available() -> bool {
    available_at(None)
}
pub fn available_at(socket: Option<&Path>) -> bool {
    connectors_custody::available(socket)
}

/// Passive qualification of the binding `selection` names. The mounted
/// binding is qualified when its directory passes its checks on a read-only
/// mount; no item is read.
pub fn available_for(selection: &Selection) -> bool {
    match selection {
        Selection::Platform {
            secret_service_socket,
        } => available_at(secret_service_socket.as_deref()),
        #[cfg(target_os = "linux")]
        Selection::Mounted { directory } => mounted::available(directory),
        #[cfg(not(target_os = "linux"))]
        Selection::Mounted { .. } => false,
    }
}

/// The writer and persistence locks wait as long as the lifecycle lock.
fn lock_until() -> std::time::Instant {
    std::time::Instant::now() + super::super::metadata::lifecycle_lock_wait()
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) mod tests;

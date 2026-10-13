//! The host's custody coordinator over the platform binding: immutable, scoped
//! versions with definite receipts. The binding itself (the Secret Service on
//! Linux, docs/local-secret-service.md) is `connectors-custody`; this module
//! keeps the host's in-process writer lock, the lock deadline it shares with
//! the lifecycle lock, and the receipts the registry's fences consume. No
//! public management endpoint exposes this binding or its private locators.
use connectors_custody::{Backend, Material, Platform, Purpose};
use connectors_sdk::Secret;

pub use connectors_custody::{Failure, Scope, Version};
type Result<T> = std::result::Result<T, Failure>;

/// A definite backend acknowledgement, never constructed from caller JSON.
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

/// Persistent custody for one admitted scope through the platform binding. No
/// activation, unlock, prompt, or fallback occurs.
pub struct Store {
    backend: Platform,
    writer: std::sync::Mutex<()>,
}

impl Store {
    pub fn open(scope: Scope) -> Result<Self> {
        Self::open_at(scope, None)
    }
    pub fn open_at(scope: Scope, socket: Option<&std::path::Path>) -> Result<Self> {
        connectors_custody::open(scope, socket).map(Self::from_backend)
    }

    fn from_backend(backend: Platform) -> Self {
        Self {
            backend,
            writer: std::sync::Mutex::new(()),
        }
    }

    /// The binding's write: a definite acknowledgement after the exact bytes
    /// were read back and made durable. Any failure once the store may have
    /// changed is OutcomeUnknown; it is never retried here, an existing version
    /// is never overwritten or silently accepted.
    pub(crate) fn write_new_guarded(
        &self,
        version: Version,
        material: &Secret,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<WrittenVersion> {
        self.backend.admit(version)?;
        let material = Material::from_slice(&material.0);
        connectors_custody::admit_material(
            self.backend.scope(),
            &material,
            self.backend.max_bytes(),
        )?;
        let until = lock_until();
        let _writer = self.writer_lock(until)?;
        // The coordinator rechecks publication/retirement in the guard while the
        // binding's physical lock excludes deletion.
        self.backend.write_new(version, &material, until, guard)?;
        Ok(WrittenVersion { version })
    }

    #[cfg(test)]
    fn write_new(&self, version: Version, material: &Secret) -> Result<()> {
        self.write_new_guarded(version, material, || Ok(()))
            .map(|_| ())
    }

    #[cfg(test)]
    pub(crate) fn fail_synchronization(&self, fail: bool) {
        self.backend.fail_synchronization(fail);
    }

    /// The binding, for the qualification fixtures.
    #[cfg(test)]
    pub(crate) fn backend(&self) -> &Platform {
        &self.backend
    }

    pub fn read(&self, version: Version) -> Result<Secret> {
        self.backend
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
        if self.backend.scope().purpose() != Purpose::ApprovalSigning {
            return Err(Failure::Denied);
        }
        self.backend.admit(version)?;
        let until = lock_until();
        let _writer = self.writer_lock(until)?;
        self.backend.confirm(version, until, guard, |material| {
            validate(&Secret(material.as_bytes().to_vec()))
        })?;
        Ok(WrittenVersion { version })
    }

    pub(crate) fn delete_guarded(
        &self,
        version: Version,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<DeletedVersion> {
        self.backend.admit(version)?;
        let until = lock_until();
        let _writer = self.writer_lock(until)?;
        self.backend.delete(version, until, guard)?;
        Ok(DeletedVersion { version })
    }

    #[cfg(test)]
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

/// Passive qualification of the current custody owner and its storage.
/// Opens no secret session and reads no credential item or encrypted payload.
pub fn available() -> bool {
    available_at(None)
}
pub fn available_at(socket: Option<&std::path::Path>) -> bool {
    connectors_custody::available(socket)
}

/// The writer and persistence locks wait as long as the lifecycle lock.
fn lock_until() -> std::time::Instant {
    std::time::Instant::now() + super::super::metadata::lifecycle_lock_wait()
}

#[cfg(test)]
pub(crate) mod tests;

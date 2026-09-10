//! Immutable, scoped Secret Service versions. No public management endpoint
//! exposes this binding or its private locators. See docs/local-secret-service.md.
mod gnome;

use super::{Service, State};
use connectors_sdk::Secret;
use std::collections::HashMap;
use uuid::Uuid;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

const SERVICE: &str = "/org/freedesktop/secrets";
const SERVICE_INTERFACE: &str = "org.freedesktop.Secret.Service";
const COLLECTION_INTERFACE: &str = "org.freedesktop.Secret.Collection";
const ITEM_INTERFACE: &str = "org.freedesktop.Secret.Item";
const SESSION_INTERFACE: &str = "org.freedesktop.Secret.Session";
const CONTENT_TYPE: &str = "application/octet-stream";
const MAX_BYTES: usize = 64 * 1024;

/// Closed failures deliberately contain neither bus diagnostics nor locators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Missing,
    Unavailable,
    Denied,
    InvalidMaterial,
    Conflict,
    OutcomeUnknown,
}
type Result<T> = std::result::Result<T, Failure>;
type WireSecret = (OwnedObjectPath, Vec<u8>, Vec<u8>, String);

/// Host-assigned namespace. This is an injective mapping of the existing
/// CustodyVersion scope, not a provider identity or a public connection selector.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Scope {
    authority: Uuid,
    allocation: Uuid,
    purpose: Purpose,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Purpose {
    Credential,
    ApprovalSigning,
}

impl Scope {
    pub fn new(authority: Uuid, allocation: Uuid) -> Result<Self> {
        if authority.is_nil() || allocation.is_nil() {
            return Err(Failure::Denied);
        }
        Ok(Self {
            authority,
            allocation,
            purpose: Purpose::Credential,
        })
    }

    pub(crate) fn approval_signing(authority: Uuid, allocation: Uuid) -> Result<Self> {
        let mut scope = Self::new(authority, allocation)?;
        scope.purpose = Purpose::ApprovalSigning;
        Ok(scope)
    }
}

/// The coordinator allocates and records this exact tuple BEFORE attempting a
/// write. An unknown response therefore does not lose the candidate's identity.
/// Only authoritative private metadata may restore it after restart.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Version {
    scope: Scope,
    id: Uuid,
}

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

impl Version {
    pub fn scope(&self) -> Scope {
        self.scope
    }
    pub fn new(scope: Scope, id: Uuid) -> Result<Self> {
        if id.is_nil() {
            return Err(Failure::Denied);
        }
        Ok(Self { scope, id })
    }
}

/// Persistent custody for one admitted scope, pinned to one bus owner and its
/// qualified backing store. No activation, unlock, prompt, or fallback occurs.
pub struct Store {
    service: Service,
    scope: Scope,
    session: OwnedObjectPath,
    persistence: gnome::Persistence,
    writer: std::sync::Mutex<()>,
}

impl Store {
    pub fn open(scope: Scope) -> Result<Self> {
        Self::open_at(scope, None)
    }
    pub fn open_at(scope: Scope, socket: Option<&std::path::Path>) -> Result<Self> {
        Self::connect(super::local_stream_at(socket).map_err(unavailable)?, scope)
    }

    fn connect(stream: std::os::unix::net::UnixStream, scope: Scope) -> Result<Self> {
        let service = Service::connect(stream).map_err(unavailable)?;
        if service.state().map_err(unavailable)? != State::Available {
            return Err(Failure::Unavailable);
        }
        let persistence = gnome::Persistence::admit(&service)?;
        let proxy = service
            .proxy(SERVICE, SERVICE_INTERFACE)
            .map_err(unavailable)?;
        // The bus and service are authenticated to this local UID. This plain
        // session is never transported over TCP. D-Bus/OS buffers are outside
        // the transient application-buffer zeroization guarantee.
        let (output, session): (OwnedValue, OwnedObjectPath) = proxy
            .call("OpenSession", &("plain", Value::from("")))
            .map_err(unavailable)?;
        if <&str>::try_from(&output).ok() != Some("") || session.as_str() == "/" {
            return Err(Failure::Unavailable);
        }
        drop(proxy);
        Ok(Self {
            service,
            scope,
            session,
            persistence,
            writer: std::sync::Mutex::new(()),
        })
    }

    /// A successful method call alone is insufficient. Verify the exact stored
    /// bytes, then synchronize the encrypted backing file and its directories.
    /// Any failure once CreateItem may have been sent is OutcomeUnknown. Never
    /// retry it here, overwrite, or silently accept an existing version.
    pub(crate) fn write_new_guarded(
        &self,
        version: Version,
        material: &Secret,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<WrittenVersion> {
        self.admit(version)?;
        if material.0.is_empty()
            || material.0.len() > MAX_BYTES
            || (self.scope.purpose == Purpose::ApprovalSigning && material.0.len() != 32)
        {
            return Err(Failure::InvalidMaterial);
        }
        let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let _writer = self.writer_lock(until)?;
        let _lock = self.persistence.lock(until)?;
        self.admit(version)?;
        // The coordinator rechecks publication/retirement while this physical
        // lock excludes deletion. It closes its metadata handle before returning.
        guard()?;
        if self.find(version)?.is_some() {
            return Err(Failure::Conflict);
        }
        let attrs = self.attributes(version);
        let mut properties: HashMap<&str, Value<'_>> = HashMap::new();
        properties.insert(
            "org.freedesktop.Secret.Item.Label",
            Value::from(match self.scope.purpose {
                Purpose::Credential => "Connectors credential",
                Purpose::ApprovalSigning => "Connectors approval signing key",
            }),
        );
        properties.insert("org.freedesktop.Secret.Item.Attributes", Value::from(attrs));
        let secret = (
            &self.session,
            Vec::<u8>::new(),
            material.0.as_slice(),
            CONTENT_TYPE,
        );
        let proxy = self
            .service
            .proxy(self.service.collection.as_str(), COLLECTION_INTERFACE)
            .map_err(unavailable)?;
        let (item, prompt): (OwnedObjectPath, OwnedObjectPath) = proxy
            .call("CreateItem", &(properties, secret, false))
            .map_err(unknown)?;
        if prompt.as_str() != "/" || !self.is_item(&item) {
            return Err(Failure::OutcomeUnknown);
        }
        // Check for duplicates as well as a wrong returned item before ack.
        if self
            .find(version)
            .map_err(|_| Failure::OutcomeUnknown)?
            .as_ref()
            != Some(&item)
        {
            return Err(Failure::OutcomeUnknown);
        }
        let retained = self.read(version).map_err(|_| Failure::OutcomeUnknown)?;
        use subtle::ConstantTimeEq;
        if !bool::from(retained.0.ct_eq(&material.0)) {
            return Err(Failure::OutcomeUnknown);
        }
        self.persistence
            .synchronize()
            .map_err(|_| Failure::OutcomeUnknown)?;
        self.admit(version).map_err(|_| Failure::OutcomeUnknown)?;
        Ok(WrittenVersion { version })
    }

    #[cfg(test)]
    fn write_new(&self, version: Version, material: &Secret) -> Result<()> {
        self.write_new_guarded(version, material, || Ok(()))
            .map(|_| ())
    }

    #[cfg(test)]
    pub(crate) fn fail_synchronization(&self, fail: bool) {
        self.persistence
            .fail_sync
            .store(fail, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn read(&self, version: Version) -> Result<Secret> {
        self.admit(version)?;
        let item = self.find(version)?.ok_or(Failure::Missing)?;
        let proxy = self
            .service
            .proxy(item.as_str(), ITEM_INTERFACE)
            .map_err(unavailable)?;
        let ((session, parameters, bytes, content_type),): (WireSecret,) = proxy
            .call("GetSecret", &(&self.session,))
            .map_err(unavailable)?;
        let secret = Secret(bytes);
        if session != self.session
            || !parameters.is_empty()
            // This qualified GNOME implementation always returns text/plain,
            // including for binary values. The custody contract treats bytes as
            // opaque; this backend label cannot select a parser or authority.
            || content_type != "text/plain"
            || secret.0.is_empty()
            || secret.0.len() > MAX_BYTES
        {
            return Err(Failure::Unavailable);
        }
        self.admit(version)?;
        Ok(secret)
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
        if self.scope.purpose != Purpose::ApprovalSigning {
            return Err(Failure::Denied);
        }
        self.admit(version)?;
        let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let _writer = self.writer_lock(until)?;
        let _lock = self.persistence.lock(until)?;
        self.admit(version)?;
        guard()?;
        let material = self.read(version)?;
        if material.0.len() != 32 {
            return Err(Failure::InvalidMaterial);
        }
        validate(&material)?;
        self.persistence
            .synchronize()
            .map_err(|_| Failure::OutcomeUnknown)?;
        self.admit(version).map_err(|_| Failure::OutcomeUnknown)?;
        Ok(WrittenVersion { version })
    }

    pub(crate) fn delete_guarded(
        &self,
        version: Version,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<DeletedVersion> {
        self.admit(version)?;
        let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let _writer = self.writer_lock(until)?;
        let _lock = self.persistence.lock(until)?;
        self.admit(version)?;
        guard()?;
        if let Some(item) = self.find(version)? {
            let proxy = self
                .service
                .proxy(item.as_str(), ITEM_INTERFACE)
                .map_err(unavailable)?;
            let prompt: OwnedObjectPath = proxy.call("Delete", &()).map_err(unknown)?;
            if prompt.as_str() != "/" {
                return Err(Failure::OutcomeUnknown);
            }
        }
        if self
            .find(version)
            .map_err(|_| Failure::OutcomeUnknown)?
            .is_some()
        {
            return Err(Failure::OutcomeUnknown);
        }
        self.persistence
            .synchronize()
            .map_err(|_| Failure::OutcomeUnknown)?;
        Ok(DeletedVersion { version })
    }

    #[cfg(test)]
    fn delete_for_qualification(&self, version: Version) -> Result<()> {
        self.delete_guarded(version, || Ok(())).map(|_| ())
    }

    fn admit(&self, version: Version) -> Result<()> {
        if version.scope != self.scope {
            return Err(Failure::Denied);
        }
        self.persistence.check_process()?;
        if self.service.state().map_err(unavailable)? != State::Available {
            return Err(Failure::Unavailable);
        }
        self.persistence.check_encrypted()?;
        Ok(())
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

    fn attributes(&self, version: Version) -> HashMap<&'static str, String> {
        let (schema, format) = match self.scope.purpose {
            Purpose::Credential => ("org.beyond10x.Connectors.Credential", "custody/1"),
            Purpose::ApprovalSigning => (
                "org.beyond10x.Connectors.ApprovalSigningKey",
                "approval-signing-custody/1",
            ),
        };
        HashMap::from([
            ("xdg:schema", schema.to_owned()),
            ("connectors.format", format.to_owned()),
            ("connectors.authority", self.scope.authority.to_string()),
            ("connectors.scope", self.scope.allocation.to_string()),
            ("connectors.version", version.id.to_string()),
        ])
    }

    fn find(&self, version: Version) -> Result<Option<OwnedObjectPath>> {
        let proxy = self
            .service
            .proxy(self.service.collection.as_str(), COLLECTION_INTERFACE)
            .map_err(unavailable)?;
        let items: Vec<OwnedObjectPath> = proxy
            .call("SearchItems", &(self.attributes(version),))
            .map_err(unavailable)?;
        if items.len() > 1 {
            return Err(Failure::Conflict);
        }
        let Some(item) = items.into_iter().next() else {
            return Ok(None);
        };
        if !self.is_item(&item) {
            return Err(Failure::Unavailable);
        }
        let item_proxy = self
            .service
            .proxy(item.as_str(), ITEM_INTERFACE)
            .map_err(unavailable)?;
        let attributes: HashMap<String, String> =
            item_proxy.get_property("Attributes").map_err(unavailable)?;
        let expected = self.attributes(version);
        if attributes.len() != expected.len()
            || expected.iter().any(|(k, v)| attributes.get(*k) != Some(v))
        {
            return Err(Failure::Denied);
        }
        drop(item_proxy);
        Ok(Some(item))
    }

    fn is_item(&self, path: &OwnedObjectPath) -> bool {
        path.as_str()
            .strip_prefix(self.service.collection.as_str())
            .is_some_and(|suffix| {
                suffix.starts_with('/') && suffix.len() > 1 && !suffix[1..].contains('/')
            })
    }
}

/// Passive qualification of the current custody owner and encrypted storage.
/// Opens no secret session and reads no credential item or encrypted payload.
pub fn available() -> bool {
    available_at(None)
}
pub fn available_at(socket: Option<&std::path::Path>) -> bool {
    (|| -> Result<()> {
        let service = Service::connect(super::local_stream_at(socket).map_err(unavailable)?)
            .map_err(unavailable)?;
        if service.state().map_err(unavailable)? != State::Available {
            return Err(Failure::Unavailable);
        }
        let persistence = gnome::Persistence::admit(&service)?;
        persistence.check_process()?;
        Ok(())
    })()
    .is_ok()
}

impl Drop for Store {
    fn drop(&mut self) {
        if let Ok(proxy) = self.service.proxy(self.session.as_str(), SESSION_INTERFACE) {
            let _: zbus::Result<()> = proxy.call("Close", &());
        }
    }
}

fn unavailable<T>(_: T) -> Failure {
    Failure::Unavailable
}
fn unknown<T>(_: T) -> Failure {
    Failure::OutcomeUnknown
}

#[cfg(test)]
pub(crate) mod tests;

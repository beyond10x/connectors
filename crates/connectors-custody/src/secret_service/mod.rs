//! The Linux binding: immutable, scoped versions in the qualified Secret
//! Service `login` collection, over an owner-checked local transport.
//! Inspection never activates a service, unlocks a collection or reads
//! credentials. No public management endpoint exposes this binding or its
//! private locators. See docs/local-secret-service.md.
#[cfg(any(test, feature = "test-support"))]
pub mod fixture;
mod fs;
mod gnome;

use crate::{
    Backend, CustodyBackend, Failure, MAX_BYTES, Material, Purpose, ReadBackend, Result, Scope,
    State, Version, admit_material,
};
use std::{
    collections::HashMap,
    os::{
        fd::AsRawFd,
        unix::{
            fs::{FileTypeExt, MetadataExt},
            net::UnixStream,
        },
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

/// The persistent collection custody uses, addressed by object path.
pub const LOGIN_COLLECTION: &str = "/org/freedesktop/secrets/collection/login";
const SERVICE: &str = "/org/freedesktop/secrets";
const SERVICE_INTERFACE: &str = "org.freedesktop.Secret.Service";
const COLLECTION_INTERFACE: &str = "org.freedesktop.Secret.Collection";
const ITEM_INTERFACE: &str = "org.freedesktop.Secret.Item";
const SESSION_INTERFACE: &str = "org.freedesktop.Secret.Session";
const CONTENT_TYPE: &str = "application/octet-stream";

type WireSecret = (OwnedObjectPath, Vec<u8>, Vec<u8>, String);

/// Passive state of the owner's runtime bus Secret Service.
pub fn inspect() -> State {
    inspect_at(None)
}
pub fn inspect_at(socket: Option<&Path>) -> State {
    local_stream_at(socket)
        .and_then(Service::connect)
        .and_then(|service| service.state())
        .unwrap_or(State::Unavailable)
}

fn local_stream_at(socket: Option<&Path>) -> zbus::Result<UnixStream> {
    // The initial Linux profile binds the owner's runtime bus. Do not accept a
    // caller-controlled TCP DBUS_SESSION_BUS_ADDRESS as local owner authority.
    let default = PathBuf::from(format!("/run/user/{}/bus", fs::uid()));
    let path = socket.unwrap_or(&default);
    fs::validate_path(path)
        .map_err(|_| zbus::Error::Failure("local transport unavailable".into()))?;
    let parent = path
        .parent()
        .ok_or_else(|| zbus::Error::Failure("local transport unavailable".into()))?;
    if fs::directory(parent, true).is_err() {
        return Err(zbus::Error::Failure("local transport unavailable".into()));
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_socket() || metadata.uid() != fs::uid() {
        return Err(zbus::Error::Failure("local transport unavailable".into()));
    }
    let stream = UnixStream::connect(path)?;
    Ok(stream)
}

fn check_peer(stream: &UnixStream) -> zbus::Result<()> {
    let mut peer: libc::ucred = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: peer is an initialized correctly sized output buffer, length is
    // writable, and stream retains ownership of the socket throughout the call.
    let status = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            std::ptr::from_mut(&mut peer).cast(),
            &mut length,
        )
    };
    if status != 0 || length as usize != std::mem::size_of::<libc::ucred>() || peer.uid != fs::uid()
    {
        return Err(zbus::Error::Failure("local transport unavailable".into()));
    }
    Ok(())
}

struct Service {
    connection: zbus::blocking::Connection,
    owner: zbus::names::OwnedUniqueName,
    collection: OwnedObjectPath,
}

impl Service {
    fn connect(stream: UnixStream) -> zbus::Result<Self> {
        check_peer(&stream)?;
        let connection = zbus::blocking::connection::Builder::async_io_unix_stream(stream)
            .method_timeout(Duration::from_secs(2))
            .build()?;
        let bus = zbus::blocking::fdo::DBusProxy::new(&connection)?;
        let service = zbus::names::BusName::try_from("org.freedesktop.secrets")?;
        if !bus.name_has_owner(service.clone())? {
            return Err(zbus::Error::Failure("service unavailable".into()));
        }
        let owner = bus.get_name_owner(service)?;
        if bus.get_connection_unix_user(owner.clone().into())? != fs::uid() {
            return Err(zbus::Error::Failure("service unavailable".into()));
        }
        // Pin the unique bus owner so a replaced service cannot inherit this check.
        let service = zbus::blocking::Proxy::new(
            &connection,
            owner.as_str(),
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
        )?;
        // The persistent `login` collection is addressed by its object path. The
        // `default` alias belongs to every other application on the desktop, so it is
        // never read, required or changed here.
        let collection = OwnedObjectPath::try_from(LOGIN_COLLECTION)?;
        let collections: Vec<OwnedObjectPath> = service.get_property("Collections")?;
        if !collections.contains(&collection) {
            return Err(zbus::Error::Failure("collection unavailable".into()));
        }
        drop(service);
        Ok(Self {
            connection,
            owner,
            collection,
        })
    }

    fn proxy<'a>(
        &'a self,
        path: &'a str,
        interface: &'a str,
    ) -> zbus::Result<zbus::blocking::Proxy<'a>> {
        zbus::blocking::Proxy::new(&self.connection, self.owner.as_str(), path, interface)
    }

    fn state(&self) -> zbus::Result<State> {
        let collection = self.proxy(
            self.collection.as_str(),
            "org.freedesktop.Secret.Collection",
        )?;
        let locked: bool = collection.get_property("Locked")?;
        Ok(if locked {
            State::Locked
        } else {
            State::Available
        })
    }
}

/// Persistent custody for one admitted scope, pinned to one bus owner and its
/// qualified backing store. No activation, unlock, prompt, or fallback occurs.
pub struct SecretService {
    service: Service,
    scope: Scope,
    session: OwnedObjectPath,
    persistence: gnome::Persistence,
}

impl std::fmt::Debug for SecretService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretService")
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

impl SecretService {
    pub fn open(scope: Scope) -> Result<Self> {
        Self::open_at(scope, None)
    }
    pub fn open_at(scope: Scope, socket: Option<&Path>) -> Result<Self> {
        Self::connect(local_stream_at(socket).map_err(unavailable)?, scope)
    }

    /// Open over an already connected bus stream. The peer, the service owner
    /// and the qualification are checked exactly as [`Self::open_at`] does;
    /// only the socket path admission is skipped. Test fixtures only.
    #[cfg(any(test, feature = "test-support"))]
    pub fn connect_stream(stream: UnixStream, scope: Scope) -> Result<Self> {
        Self::connect(stream, scope)
    }

    fn connect(stream: UnixStream, scope: Scope) -> Result<Self> {
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
        })
    }

    /// Make the durability barrier fail, as a failed fsync would.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fail_synchronization(&self, fail: bool) {
        self.persistence
            .fail_sync
            .store(fail, std::sync::atomic::Ordering::SeqCst);
    }

    /// Whether the collection holds an item for this exact version.
    #[cfg(any(test, feature = "test-support"))]
    pub fn contains(&self, version: Version) -> Result<bool> {
        self.find(version).map(|item| item.is_some())
    }

    /// Run the durability barrier alone.
    #[cfg(any(test, feature = "test-support"))]
    pub fn synchronize(&self) -> Result<()> {
        self.persistence.synchronize()
    }

    /// Add an unexpected attribute to a version's native item, as another
    /// application on the bus could.
    #[cfg(any(test, feature = "test-support"))]
    pub fn tamper_attributes(&self, version: Version) -> Result<()> {
        let item = self.find(version)?.ok_or(Failure::Missing)?;
        let proxy = self
            .service
            .proxy(item.as_str(), ITEM_INTERFACE)
            .map_err(unavailable)?;
        let mut attrs = self.attributes(version);
        attrs.insert("unexpected.attribute", "fictional".to_owned());
        proxy
            .set_property("Attributes", attrs)
            .map_err(|_| Failure::Unavailable)
    }

    fn attributes(&self, version: Version) -> HashMap<&'static str, String> {
        let (schema, format) = match self.scope.purpose() {
            Purpose::Credential => ("org.beyond10x.Connectors.Credential", "custody/1"),
            Purpose::ApprovalSigning => (
                "org.beyond10x.Connectors.ApprovalSigningKey",
                "approval-signing-custody/1",
            ),
        };
        HashMap::from([
            ("xdg:schema", schema.to_owned()),
            ("connectors.format", format.to_owned()),
            ("connectors.authority", self.scope.authority().to_string()),
            ("connectors.scope", self.scope.allocation().to_string()),
            ("connectors.version", version.id().to_string()),
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

impl ReadBackend for SecretService {
    fn kind(&self) -> CustodyBackend {
        CustodyBackend::SecretService
    }

    fn scope(&self) -> Scope {
        self.scope
    }

    fn state(&self) -> State {
        self.service.state().unwrap_or(State::Unavailable)
    }

    fn admit(&self, version: Version) -> Result<()> {
        if version.scope() != self.scope {
            return Err(Failure::Denied);
        }
        self.persistence.check_process()?;
        if self.service.state().map_err(unavailable)? != State::Available {
            return Err(Failure::Unavailable);
        }
        self.persistence.check_encrypted()?;
        Ok(())
    }

    fn read(&self, version: Version) -> Result<Material> {
        self.admit(version)?;
        let item = self.find(version)?.ok_or(Failure::Missing)?;
        let proxy = self
            .service
            .proxy(item.as_str(), ITEM_INTERFACE)
            .map_err(unavailable)?;
        let ((session, parameters, bytes, content_type),): (WireSecret,) = proxy
            .call("GetSecret", &(&self.session,))
            .map_err(unavailable)?;
        let secret = Material::new(bytes);
        if session != self.session
            || !parameters.is_empty()
            // This qualified GNOME implementation always returns text/plain,
            // including for binary values. The custody contract treats bytes as
            // opaque; this backend label cannot select a parser or authority.
            || content_type != "text/plain"
            || secret.is_empty()
            || secret.len() > MAX_BYTES
        {
            return Err(Failure::Unavailable);
        }
        self.admit(version)?;
        Ok(secret)
    }
}

impl Backend for SecretService {
    /// A successful method call alone is insufficient. Verify the exact stored
    /// bytes, then synchronize the encrypted backing file and its directories.
    /// Any failure once CreateItem may have been sent is OutcomeUnknown. Never
    /// retry it here, overwrite, or silently accept an existing version.
    fn write_new(
        &self,
        version: Version,
        material: &Material,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        admit_material(self.scope, material, MAX_BYTES)?;
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
            Value::from(match self.scope.purpose() {
                Purpose::Credential => "Connectors credential",
                Purpose::ApprovalSigning => "Connectors approval signing key",
            }),
        );
        properties.insert("org.freedesktop.Secret.Item.Attributes", Value::from(attrs));
        let secret = (
            &self.session,
            Vec::<u8>::new(),
            material.as_bytes(),
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
        if !retained.ct_eq(material.as_bytes()) {
            return Err(Failure::OutcomeUnknown);
        }
        self.persistence
            .synchronize()
            .map_err(|_| Failure::OutcomeUnknown)?;
        self.admit(version).map_err(|_| Failure::OutcomeUnknown)?;
        Ok(())
    }

    /// Recovery of a staged signing key only. Readback alone never converts a
    /// credential version into publication authority; the caller validates the
    /// seed while the physical writer lock excludes retirement.
    fn confirm(
        &self,
        version: Version,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
        validate: impl FnOnce(&Material) -> Result<()>,
    ) -> Result<()> {
        let _lock = self.persistence.lock(until)?;
        self.admit(version)?;
        guard()?;
        let material = self.read(version)?;
        admit_material(self.scope, &material, MAX_BYTES)?;
        validate(&material)?;
        self.persistence
            .synchronize()
            .map_err(|_| Failure::OutcomeUnknown)?;
        self.admit(version).map_err(|_| Failure::OutcomeUnknown)?;
        Ok(())
    }

    fn delete(
        &self,
        version: Version,
        until: Instant,
        guard: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
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
        Ok(())
    }
}

/// Passive qualification of the current custody owner and encrypted storage.
/// Opens no secret session and reads no credential item or encrypted payload.
pub fn available() -> bool {
    available_at(None)
}
pub fn available_at(socket: Option<&Path>) -> bool {
    (|| -> Result<()> {
        let service =
            Service::connect(local_stream_at(socket).map_err(unavailable)?).map_err(unavailable)?;
        if service.state().map_err(unavailable)? != State::Available {
            return Err(Failure::Unavailable);
        }
        let persistence = gnome::Persistence::admit(&service)?;
        persistence.check_process()?;
        Ok(())
    })()
    .is_ok()
}

/// Passive state over an already connected bus stream. Test fixtures only.
#[cfg(any(test, feature = "test-support"))]
pub fn state_of(stream: UnixStream) -> Option<State> {
    Service::connect(stream).ok().and_then(|s| s.state().ok())
}

impl Drop for SecretService {
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

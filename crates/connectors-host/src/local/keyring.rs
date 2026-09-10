//! Owner-checked Secret Service transport. Inspection never activates a service,
//! unlocks a collection, or reads credentials.
pub mod custody;
use std::{
    os::{
        fd::AsRawFd,
        unix::{
            fs::{FileTypeExt, MetadataExt},
            net::UnixStream,
        },
    },
    path::PathBuf,
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Available,
    Locked,
    Unavailable,
}

pub fn inspect() -> State {
    inspect_inner().unwrap_or(State::Unavailable)
}

fn inspect_inner() -> zbus::Result<State> {
    let service = Service::connect(local_stream()?)?;
    service.state()
}

fn local_stream() -> zbus::Result<UnixStream> {
    // The initial Linux profile binds the owner's runtime bus. Do not accept a
    // caller-controlled TCP DBUS_SESSION_BUS_ADDRESS as local owner authority.
    let parent = PathBuf::from(format!("/run/user/{}", super::filesystem::uid()));
    if super::filesystem::directory(&parent, false, true).is_err() {
        return Err(zbus::Error::Failure("local transport unavailable".into()));
    }
    let path = parent.join("bus");
    let metadata = std::fs::symlink_metadata(&path)?;
    if !metadata.file_type().is_socket() || metadata.uid() != super::filesystem::uid() {
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
    if status != 0
        || length as usize != std::mem::size_of::<libc::ucred>()
        || peer.uid != super::filesystem::uid()
    {
        return Err(zbus::Error::Failure("local transport unavailable".into()));
    }
    Ok(())
}

struct Service {
    connection: zbus::blocking::Connection,
    owner: zbus::names::OwnedUniqueName,
    collection: zbus::zvariant::OwnedObjectPath,
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
        if bus.get_connection_unix_user(owner.clone().into())? != super::filesystem::uid() {
            return Err(zbus::Error::Failure("service unavailable".into()));
        }
        // Pin the unique bus owner so a replaced service cannot inherit this check.
        let service = zbus::blocking::Proxy::new(
            &connection,
            owner.as_str(),
            "/org/freedesktop/secrets",
            "org.freedesktop.Secret.Service",
        )?;
        let collection: zbus::zvariant::OwnedObjectPath =
            service.call("ReadAlias", &("default",))?;
        let session: zbus::zvariant::OwnedObjectPath = service.call("ReadAlias", &("session",))?;
        if collection.as_str() == "/" || collection == session {
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

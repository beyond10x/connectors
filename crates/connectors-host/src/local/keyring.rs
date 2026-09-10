//! Non-interactive availability observation, without activation or secret reads.
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
    // The initial Linux profile binds the owner's runtime bus. Do not accept a
    // caller-controlled TCP DBUS_SESSION_BUS_ADDRESS as local owner authority.
    let parent = PathBuf::from(format!("/run/user/{}", super::filesystem::uid()));
    if super::filesystem::directory(&parent, false, true).is_err() {
        return Ok(State::Unavailable);
    }
    let path = parent.join("bus");
    let metadata = std::fs::symlink_metadata(&path)?;
    if !metadata.file_type().is_socket() || metadata.uid() != super::filesystem::uid() {
        return Ok(State::Unavailable);
    }
    let stream = UnixStream::connect(path)?;
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
        return Ok(State::Unavailable);
    }
    let connection = zbus::blocking::connection::Builder::async_io_unix_stream(stream)
        .method_timeout(Duration::from_secs(2))
        .build()?;
    let bus = zbus::blocking::fdo::DBusProxy::new(&connection)?;
    let service = zbus::names::BusName::try_from("org.freedesktop.secrets")?;
    if !bus.name_has_owner(service.clone())? {
        return Ok(State::Unavailable);
    }
    let owner = bus.get_name_owner(service)?;
    if bus.get_connection_unix_user(owner.clone().into())? != super::filesystem::uid() {
        return Ok(State::Unavailable);
    }
    // Pin the unique bus owner so a replaced service cannot inherit this check.
    let service = zbus::blocking::Proxy::new(
        &connection,
        owner.as_str(),
        "/org/freedesktop/secrets",
        "org.freedesktop.Secret.Service",
    )?;
    let collection: zbus::zvariant::OwnedObjectPath = service.call("ReadAlias", &("default",))?;
    let session: zbus::zvariant::OwnedObjectPath = service.call("ReadAlias", &("session",))?;
    if collection.as_str() == "/" || collection == session {
        return Ok(State::Unavailable);
    }
    let collection = zbus::blocking::Proxy::new(
        &connection,
        owner.as_str(),
        collection.as_str(),
        "org.freedesktop.Secret.Collection",
    )?;
    let locked: bool = collection.get_property("Locked")?;
    Ok(if locked {
        State::Locked
    } else {
        State::Available
    })
}

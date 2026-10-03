//! Owned nonblocking local connection; callers independently verify path and peer.
use std::{
    io,
    os::{
        fd::FromRawFd,
        unix::{ffi::OsStrExt, net::UnixStream},
    },
    path::Path,
    time::Instant,
};

pub(super) fn connect(path: &Path, until: Instant) -> io::Result<UnixStream> {
    if Instant::now() >= until {
        return Err(io::ErrorKind::TimedOut.into());
    }
    // Linux AF_UNIX nonblocking connect must not wait indefinitely for a full
    // accept queue. EAGAIN refuses; it does not mean an in-progress connection.
    let bytes = path.as_os_str().as_bytes();
    // SAFETY: sockaddr_un is an integer family and byte array, both zero-valid.
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if bytes.is_empty() || bytes.len() >= address.sun_path.len() || bytes.contains(&0) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (to, from) in address.sun_path.iter_mut().zip(bytes) {
        *to = *from as libc::c_char;
    }
    // SAFETY: socket creates a fresh owned descriptor; no caller descriptor is reused.
    let fd = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            0,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful socket returned a unique owned descriptor.
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    // SAFETY: address is initialized and the supplied size is its exact allocation.
    if unsafe {
        libc::connect(
            fd,
            std::ptr::from_ref(&address).cast(),
            std::mem::size_of_val(&address) as libc::socklen_t,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    if Instant::now() >= until {
        return Err(io::ErrorKind::TimedOut.into());
    }
    stream.set_nonblocking(false)?;
    Ok(stream)
}

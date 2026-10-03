//! An owned socket cutoff for one custody sequence, including D-Bus setup/close.
use std::{
    io,
    net::Shutdown,
    os::{
        fd::FromRawFd,
        unix::{ffi::OsStrExt, net::UnixStream},
    },
    path::Path,
    sync::mpsc,
    thread::JoinHandle,
    time::Instant,
};

pub(super) struct Guard {
    done: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}
impl Guard {
    pub(super) fn new(stream: &UnixStream, until: Instant) -> io::Result<Self> {
        if Instant::now() >= until {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let held = stream.try_clone()?;
        let (done, receiver) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("connectors-custody-cutoff".into())
            .spawn(move || {
                if matches!(
                    receiver.recv_timeout(until.saturating_duration_since(Instant::now())),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    let _ = held.shutdown(Shutdown::Both);
                }
            })?;
        Ok(Self {
            done: Some(done),
            thread: Some(thread),
        })
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(done) = self.done.take() {
            let _ = done.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::time::Duration;
    #[test]
    fn original_cutoff_closes_only_the_owned_socket_and_guard_joins() {
        let (mut owned, mut peer) = UnixStream::pair().unwrap();
        let (mut other, mut other_peer) = UnixStream::pair().unwrap();
        let until = Instant::now() + Duration::from_millis(80);
        let guard = Guard::new(&owned, until).unwrap();
        owned
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let started = Instant::now();
        assert_eq!(owned.read(&mut [0]).unwrap(), 0);
        assert!(started.elapsed() < Duration::from_millis(600));
        assert!(peer.write_all(b"x").is_err());
        drop(guard);
        other.write_all(b"x").unwrap();
        let mut byte = [0];
        other_peer.read_exact(&mut byte).unwrap();
        assert_eq!(byte, [b'x']);
    }
    #[test]
    fn early_completion_cancels_and_joins_the_guard_without_closing_socket() {
        let (mut owned, mut peer) = UnixStream::pair().unwrap();
        let started = Instant::now();
        let guard = Guard::new(&owned, started + Duration::from_secs(30)).unwrap();
        drop(guard);
        assert!(started.elapsed() < Duration::from_secs(1));
        owned.write_all(b"x").unwrap();
        peer.read_exact(&mut [0]).unwrap();
    }
    #[test]
    fn expired_selection_does_not_connect() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bus");
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        listener.set_nonblocking(true).unwrap();
        assert_eq!(
            connect(&path, Instant::now()).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    }

    #[test]
    fn custody_read_cuts_off_stalled_dbus_setup_and_closes_its_connection() {
        use super::super::custody::{Scope, Store, Version};
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let path = directory.path().join("bus");
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let peer = std::thread::spawn(move || {
            listener.set_nonblocking(true).unwrap();
            let until = Instant::now() + Duration::from_secs(2);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(value) => break value,
                    Err(error)
                        if error.kind() == io::ErrorKind::WouldBlock && Instant::now() < until =>
                    {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => panic!("custody did not connect: {:?}", error.kind()),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut observed = false;
            let mut bytes = [0; 1024];
            loop {
                match stream.read(&mut bytes) {
                    Ok(0) => return observed,
                    Ok(_) => observed = true,
                    Err(error) => panic!("custody connection stayed open: {:?}", error.kind()),
                }
            }
        });
        let version = Version::new(
            Scope::new(uuid::Uuid::new_v4(), uuid::Uuid::new_v4()).unwrap(),
            uuid::Uuid::new_v4(),
        )
        .unwrap();
        let started = Instant::now();
        assert!(
            Store::read_at_until(version, Some(&path), started + Duration::from_millis(200))
                .is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(peer.join().unwrap(), "D-Bus setup was not reached");
    }
}

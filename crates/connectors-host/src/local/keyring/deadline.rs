//! An owned socket cutoff for one custody sequence, including D-Bus setup/close.
use std::{
    io, net::Shutdown, os::unix::net::UnixStream, sync::mpsc, thread::JoinHandle, time::Instant,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local::unix::connect;
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

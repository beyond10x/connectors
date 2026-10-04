use super::*;

/// A child process killed and reaped when dropped.
struct OwnedProcess(std::process::Child);

impl Drop for OwnedProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A disposable session bus and unlocked GNOME Keyring under `parent`.
pub(super) struct Custody {
    _daemon: OwnedProcess,
    _bus: OwnedProcess,
    pub(super) socket: PathBuf,
}

impl Custody {
    pub(super) fn new(parent: &Path) -> Self {
        use connectors_host::local::{filesystem, keyring};
        let directory = parent.join("custody");
        for path in [
            directory.clone(),
            directory.join("home"),
            directory.join("data/keyrings"),
            directory.join("runtime"),
        ] {
            filesystem::directory(&path, true, true).unwrap();
        }
        let socket = directory.join("bus");
        let bus = OwnedProcess(
            Command::new("/usr/bin/dbus-daemon")
                .args(["--session", "--nofork", "--nopidfile"])
                .arg(format!("--address=unix:path={}", socket.display()))
                .env_clear()
                .env("PATH", "/usr/bin")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let until = Instant::now() + Duration::from_secs(5);
        while UnixStream::connect(&socket).is_err() {
            assert!(Instant::now() < until, "fixture bus did not start");
            std::thread::sleep(Duration::from_millis(20));
        }
        let mut daemon = OwnedProcess(
            Command::new("/usr/bin/gnome-keyring-daemon")
                .args([
                    "--foreground",
                    "--components=secrets",
                    "--unlock",
                    "--control-directory",
                ])
                .arg(directory.join("runtime"))
                .env_clear()
                .env("PATH", "/usr/bin")
                .env("HOME", directory.join("home"))
                .env("XDG_DATA_HOME", directory.join("data"))
                .env("XDG_RUNTIME_DIR", directory.join("runtime"))
                .env(
                    "DBUS_SESSION_BUS_ADDRESS",
                    format!("unix:path={}", socket.display()),
                )
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        daemon
            .0
            .stdin
            .take()
            .unwrap()
            .write_all(b"fictional-failed-connect-keyring-password")
            .unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while !keyring::custody::available_at(Some(&socket)) {
            assert!(Instant::now() < until, "fixture custody unavailable");
            assert!(daemon.0.try_wait().unwrap().is_none());
            std::thread::sleep(Duration::from_millis(30));
        }
        Self {
            _daemon: daemon,
            _bus: bus,
            socket,
        }
    }
}

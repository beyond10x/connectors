//! A private, disposable Secret Service: its own `dbus-daemon` and the
//! qualified `gnome-keyring-daemon`, under a task-owned `TMPDIR`. These
//! fixtures own every process and path they touch. They never connect to the
//! desktop bus, use a user's existing collection, or invoke an unlock prompt.
use super::SecretService;
use crate::{Result, Scope, State};
use std::{
    io::Write,
    os::unix::{fs::DirBuilderExt, fs::PermissionsExt, net::UnixStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub struct Fixture {
    daemon: Option<OwnedChild>,
    _bus: OwnedChild,
    pub root: tempfile::TempDir,
    pub socket: PathBuf,
    pub data: PathBuf,
    incarnation: u32,
}

fn private_directory(path: &Path) {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .unwrap();
}

impl Default for Fixture {
    fn default() -> Self {
        Self::new()
    }
}

impl Fixture {
    pub fn new() -> Self {
        let temporary = std::env::var_os("TMPDIR").expect("task-owned TMPDIR required");
        let root = tempfile::Builder::new()
            .prefix("secret-service-")
            .tempdir_in(temporary)
            .unwrap();
        // Production socket admission requires a private parent.
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let socket = root.path().join("bus");
        let data = root.path().join("data");
        private_directory(&data.join("keyrings"));
        private_directory(&root.path().join("home"));
        let bus = OwnedChild(
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
            assert!(Instant::now() < until, "private fixture bus unavailable");
            std::thread::sleep(Duration::from_millis(20));
        }
        let mut fixture = Self {
            daemon: None,
            _bus: bus,
            root,
            socket,
            data,
            incarnation: 0,
        };
        fixture.start(true);
        fixture
    }

    pub fn start(&mut self, unlock: bool) {
        assert!(self.daemon.is_none());
        self.incarnation += 1;
        let runtime = self
            .root
            .path()
            .join(format!("runtime-{}", self.incarnation));
        private_directory(&runtime);
        let mut command = Command::new("/usr/bin/gnome-keyring-daemon");
        command
            .args([
                "--foreground",
                "--components=secrets",
                "--control-directory",
            ])
            .arg(&runtime)
            .env_clear()
            .env("PATH", "/usr/bin")
            .env("HOME", self.root.path().join("home"))
            .env("XDG_DATA_HOME", &self.data)
            .env("XDG_RUNTIME_DIR", &runtime)
            .env(
                "DBUS_SESSION_BUS_ADDRESS",
                format!("unix:path={}", self.socket.display()),
            )
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if unlock {
            command.arg("--unlock").stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }
        let mut daemon = OwnedChild(command.spawn().unwrap());
        if unlock {
            // Fictional fixture password, supplied only over the owned pipe.
            daemon
                .0
                .stdin
                .take()
                .unwrap()
                .write_all(b"connectors-disposable-keyring-password")
                .unwrap();
        }
        self.daemon = Some(daemon);
        let expected = if unlock {
            State::Available
        } else {
            State::Locked
        };
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            let state = UnixStream::connect(&self.socket)
                .ok()
                .and_then(super::state_of);
            if state == Some(expected) {
                break;
            }
            assert!(
                Instant::now() < until,
                "private fixture collection did not become ready"
            );
            assert!(
                self.daemon
                    .as_mut()
                    .unwrap()
                    .0
                    .try_wait()
                    .unwrap()
                    .is_none(),
                "fixture daemon exited"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn stop(&mut self) {
        drop(self.daemon.take());
    }

    /// The binding for one scope over this fixture's bus.
    pub fn backend(&self, scope: Scope) -> Result<SecretService> {
        SecretService::connect_stream(UnixStream::connect(&self.socket).unwrap(), scope)
    }
}

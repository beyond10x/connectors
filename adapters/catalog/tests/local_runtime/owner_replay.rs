//! Same-image host-library owner/2 acceptance after production CLI settlement.
//! The real owner and parent share the libtest image; no build identity is forged.
use super::*;
use std::os::unix::{net::UnixStream, process::CommandExt};

pub(super) struct FixtureOwner {
    process: Option<OwnedProcess>,
    paths: Paths,
    pub(super) incarnation: String,
}

impl FixtureOwner {
    pub(super) fn start(cli: &Cli, root: &Path) -> Self {
        assert!(!cli.paths.state.join("owner.sock").exists());
        let directory = filesystem::directory(&cli.paths.state, false, true).unwrap();
        let lock =
            filesystem::private_file_at(&directory, std::ffi::OsStr::new("owner.lock")).unwrap();
        // SAFETY: the verified task-owned file remains held until its duplicate
        // has been inherited by this exact child process.
        assert_eq!(
            unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        let (startup, child_channel) = UnixStream::pair().unwrap();
        let duplicate = |fd| {
            // SAFETY: duplicate a live owned source above the destination slots.
            let raw = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 10) };
            assert!(raw >= 0);
            // SAFETY: successful duplication returns a fresh owned descriptor.
            unsafe { fs::File::from_raw_fd(raw) }
        };
        let private = duplicate(child_channel.as_raw_fd());
        let lifetime = duplicate(lock.as_raw_fd());
        let (private_fd, lock_fd) = (private.as_raw_fd(), lifetime.as_raw_fd());
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "cli_journey::guarded_merge::owner_replay::catalog_same_image_owner_fixture",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env_clear()
            .env("CONNECTORS_CATALOG_OWNER_FIXTURE", root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(tmp) = std::env::var_os("TMPDIR") {
            command.env("TMPDIR", tmp);
        }
        // SAFETY: isolated child-only setup uses async-signal-safe syscalls and
        // scalar descriptors. No process-wide FD mutation occurs in libtest.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() < 0
                    || libc::dup2(private_fd, 3) < 0
                    || libc::dup2(lock_fd, 4) < 0
                    || libc::fcntl(3, libc::F_SETFD, 0) < 0
                    || libc::fcntl(4, libc::F_SETFD, 0) < 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let process = OwnedProcess(command.spawn().unwrap());
        drop((startup, child_channel, private, lifetime, lock));
        let mut fixture = Self {
            process: Some(process),
            paths: Paths {
                config: cli.paths.config.clone(),
                state: cli.paths.state.clone(),
            },
            incarnation: String::new(),
        };
        let until = Instant::now() + Duration::from_secs(10);
        while !fixture.paths.state.join("owner.sock").exists() {
            if fixture
                .process
                .as_mut()
                .unwrap()
                .0
                .try_wait()
                .unwrap()
                .is_some()
            {
                let output = finish_process(fixture.process.take().unwrap());
                panic!("fixture owner exited before readiness: {output:?}");
            }
            assert!(Instant::now() < until, "fixture owner did not listen");
            std::thread::sleep(Duration::from_millis(20));
        }
        let client = owner::Client::connect(&fixture.paths, false).unwrap();
        fixture.incarnation = client.host_incarnation.clone();
        assert!(client.status("gitlab").unwrap()["child_incarnation"].is_null());
        eprintln!("same-image host-library owner: real handshake and build admission passed");
        fixture
    }

    pub(super) fn status(&self) -> Value {
        let client = owner::Client::connect(&self.paths, false).unwrap();
        assert_eq!(client.host_incarnation, self.incarnation);
        client.status("gitlab").unwrap()
    }

    pub(super) fn shutdown(mut self) {
        let client = owner::Client::connect(&self.paths, false).unwrap();
        assert_eq!(client.host_incarnation, self.incarnation);
        client.shutdown(&self.incarnation).unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        let status = loop {
            if let Some(status) = self.process.as_mut().unwrap().0.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < until, "fixture owner did not exit");
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(status.success(), "fixture owner exit: {status}");
        let output = finish_process(self.process.take().unwrap());
        assert!(output.status.success(), "{output:?}");
        assert!(!self.paths.state.join("owner.sock").exists());
        let directory = filesystem::directory(&self.paths.state, false, true).unwrap();
        let lock =
            filesystem::private_file_at(&directory, std::ffi::OsStr::new("owner.lock")).unwrap();
        // SAFETY: probe the verified exact lifetime lock after observed exit.
        assert_eq!(
            unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        eprintln!("same-image host-library owner: exit0, socket removed, lifetime lock released");
    }
}
impl Drop for FixtureOwner {
    fn drop(&mut self) {
        if self.process.is_some() {
            // Panic cleanup owns only this child and its private state. Normal
            // acceptance uses shutdown(), which requires a successful exit.
            if let Ok(client) = owner::Client::connect(&self.paths, false) {
                let host = client.host_incarnation.clone();
                let _ = client.shutdown(&host);
            }
        }
    }
}

#[test]
#[ignore = "helper requires explicit task-owned catalog fixture root; not a journey"]
fn catalog_same_image_owner_fixture() {
    let root = PathBuf::from(
        std::env::var_os("CONNECTORS_CATALOG_OWNER_FIXTURE")
            .expect("explicit private owner fixture selection required"),
    );
    assert!(root.is_absolute());
    filesystem::directory(&root, false, true).unwrap();
    owner::serve(Paths {
        config: root.join("cli/config.toml"),
        state: root.join("cli/state"),
    })
    .unwrap();
}

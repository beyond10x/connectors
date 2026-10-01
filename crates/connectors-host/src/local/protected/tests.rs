use super::*;
use std::{
    os::unix::{
        fs::{PermissionsExt, symlink},
        process::CommandExt,
    },
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn bounded_document_uses_original_deadline_and_rejects_links_and_excess_input() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("input.json");
    std::fs::write(&path, b"{\"operations\":[]}").unwrap();
    assert_eq!(
        document_until(Some(&path), Instant::now(), 1024)
            .unwrap_err()
            .code,
        Code::Timeout
    );
    assert_eq!(
        document_until(Some(&path), Instant::now() + Duration::from_secs(1), 4)
            .unwrap_err()
            .code,
        Code::InvalidInput
    );
    assert_eq!(
        document_until(Some(&path), Instant::now() + Duration::from_secs(1), 1024).unwrap(),
        "{\"operations\":[]}"
    );
    let link = root.path().join("link.json");
    symlink(&path, &link).unwrap();
    assert_eq!(
        document_until(Some(&link), Instant::now() + Duration::from_secs(1), 1024)
            .unwrap_err()
            .code,
        Code::InvalidInput
    );
}

#[test]
fn source_admission_rejects_links_modes_and_oversized_files() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("input");
    std::fs::write(&path, b"fictional-private-document").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let deadline = connectors_sdk::now_ms() + 2000;
    assert!(file(&path, deadline).is_ok());
    let link = root.path().join("link");
    symlink(&path, &link).unwrap();
    assert!(file(&link, deadline).is_err());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(file(&path, deadline).is_err());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let hard = root.path().join("hard");
    std::fs::hard_link(&path, &hard).unwrap();
    assert!(file(&path, deadline).is_err());
    std::fs::remove_file(hard).unwrap();
    std::fs::write(&path, vec![b'x'; 65537]).unwrap();
    assert!(file(&path, deadline).is_err());
}

// Re-entered in its own process/session by the owning PTY test below. Secret
// assertions deliberately avoid Debug, JSON output and test panic diagnostics.
#[test]
fn terminal_fixture() {
    let Ok(mode) = std::env::var("CONNECTORS_TERMINAL_FIXTURE") else {
        return;
    };
    let signals = Signals::install().unwrap();
    let result = terminal(
        &[EntryField {
            name: "token".into(),
            label: "Fictional credential".into(),
            max_bytes: 64,
        }],
        std::env::var("CONNECTORS_TERMINAL_FIXTURE_DEADLINE_MS")
            .unwrap()
            .parse()
            .unwrap(),
    );
    if mode == "interrupt" {
        assert!(matches!(
            result,
            Err(Error {
                code: Code::Interrupted,
                ..
            })
        ));
        assert!(signals.interrupted());
    } else {
        assert!(result.is_ok());
        assert!(result.unwrap().0 == br#"{"token":"fictional-input"}"#);
    }
}
struct Child(std::process::Child);
impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn attrs(file: &File) -> libc::termios {
    let mut mode = unsafe { std::mem::zeroed::<libc::termios>() };
    assert_eq!(unsafe { libc::tcgetattr(file.as_raw_fd(), &mut mode) }, 0);
    mode
}
/// Neither terminal mode consults its deadline except as a hang guard, so the
/// bound this test hands the child, and waits on itself, is one only a hang
/// reaches.
const TERMINAL_BUDGET: Duration = Duration::from_secs(120);

#[test]
fn controlling_terminal_hides_input_and_restores_echo_after_sigint() {
    // `late` answers the prompt 5.5 s after it appears, past the 5 s the
    // fixture used to assume, so it passes only on the deadline this test
    // hands the child.
    for mode in ["complete", "interrupt", "late"] {
        let (mut master_fd, mut slave_fd) = (-1, -1);
        // SAFETY: initialized output slots; default terminal size/mode requested.
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master_fd,
                    &mut slave_fd,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                )
            },
            0
        );
        let (mut master, slave) =
            unsafe { (File::from_raw_fd(master_fd), File::from_raw_fd(slave_fd)) };
        assert_eq!(
            unsafe { libc::fcntl(master_fd, libc::F_SETFL, libc::O_NONBLOCK) },
            0
        );
        assert_eq!(
            unsafe { libc::fcntl(master_fd, libc::F_SETFD, libc::FD_CLOEXEC) },
            0
        );
        assert_eq!(
            unsafe { libc::fcntl(slave_fd, libc::F_SETFD, libc::FD_CLOEXEC) },
            0
        );
        let original = attrs(&slave);
        assert_ne!(original.c_lflag & libc::ECHO, 0);
        // The child's own assertion message is the only record of why it
        // failed; keep it where the failure below can report it.
        let diagnostics = tempfile::tempfile().unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "local::protected::tests::terminal_fixture",
                "--nocapture",
            ])
            .env("CONNECTORS_TERMINAL_FIXTURE", mode)
            .env(
                "CONNECTORS_TERMINAL_FIXTURE_DEADLINE_MS",
                (connectors_sdk::now_ms() + TERMINAL_BUDGET.as_millis() as u64).to_string(),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(diagnostics.try_clone().unwrap());
        // SAFETY: this child alone creates the session and adopts the already
        // owned slave. Parent retains both fds; no foreign process is signalled.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() < 0 || libc::ioctl(slave_fd, libc::TIOCSCTTY, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = Child(command.spawn().unwrap());
        let mut observed = Vec::new();
        let until = Instant::now() + TERMINAL_BUDGET;
        while !observed.ends_with(b": ") {
            let mut bytes = [0; 256];
            match master.read(&mut bytes) {
                Ok(n) => observed.extend_from_slice(&bytes[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => panic!("fixture terminal unavailable"),
            }
            assert!(Instant::now() < until, "fixture prompt did not arrive");
            assert!(child.0.try_wait().unwrap().is_none());
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(attrs(&slave).c_lflag & libc::ECHO, 0);
        if mode == "late" {
            std::thread::sleep(Duration::from_millis(5_500));
        }
        master.write_all(b"fictional-inputx\x7f").unwrap();
        if mode == "interrupt" {
            master.write_all(&[3]).unwrap();
        } else {
            master.write_all(b"\n").unwrap();
        }
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < until, "fixture did not finish");
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(status.success(), "fixture assertion failed ({mode}): {}", {
            let mut stderr = String::new();
            let mut diagnostics = diagnostics;
            std::io::Seek::rewind(&mut diagnostics).unwrap();
            diagnostics.read_to_string(&mut stderr).unwrap();
            stderr
        });
        let restored = attrs(&slave);
        assert_eq!(restored.c_lflag, original.c_lflag);
        assert_eq!(restored.c_cc, original.c_cc);
        let mut bytes = [0; 256];
        while let Ok(n) = master.read(&mut bytes) {
            if n == 0 {
                break;
            }
            observed.extend_from_slice(&bytes[..n]);
        }
        assert!(
            !observed
                .windows(b"fictional-input".len())
                .any(|v| v == b"fictional-input")
        );
    }
}

/// Stands in for the line discipline handling `^C` between `wait` seeing a
/// queued byte and the read that follows: the queue is flushed, so the
/// non-blocking read finds it empty. In `before` the interrupt is handled
/// before that read; in `after` it reaches the waiting thread only once the
/// read has returned, which is the order a loaded machine produced.
struct Flushing {
    file: File,
    mode: String,
    armed: bool,
    flushed_read: Option<std::io::ErrorKind>,
    helper: Option<std::thread::JoinHandle<()>>,
}
impl Read for Flushing {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if !std::mem::take(&mut self.armed) {
            return self.file.read(buffer);
        }
        // SAFETY: a live descriptor this fixture owns.
        assert_eq!(
            unsafe { libc::tcflush(self.file.as_raw_fd(), libc::TCIFLUSH) },
            0
        );
        if self.mode == "before" {
            // SAFETY: the guard installed by the fixture handles SIGINT.
            assert_eq!(unsafe { libc::raise(libc::SIGINT) }, 0);
        } else {
            // SAFETY: scalar identity of the calling thread.
            let target = unsafe { libc::pthread_self() };
            self.helper = Some(std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(200));
                // SAFETY: the fixture joins this helper before its target
                // thread can finish, so the target is alive.
                unsafe {
                    libc::pthread_kill(target, libc::SIGINT);
                }
            }));
        }
        let result = self.file.read(buffer);
        self.flushed_read = Some(match &result {
            Ok(_) => std::io::ErrorKind::Other,
            Err(e) => e.kind(),
        });
        result
    }
}
impl AsRawFd for Flushing {
    fn as_raw_fd(&self) -> i32 {
        self.file.as_raw_fd()
    }
}

// Re-entered in its own process by the owning test below, so the signal flag
// and handler it installs are not shared with any other test.
#[test]
fn flush_fixture() {
    let Ok(mode) = std::env::var("CONNECTORS_FLUSH_FIXTURE") else {
        return;
    };
    let signals = Signals::install().unwrap();
    let (mut master_fd, mut slave_fd) = (-1, -1);
    // SAFETY: initialized output slots; default terminal size/mode requested.
    assert_eq!(
        unsafe {
            libc::openpty(
                &mut master_fd,
                &mut slave_fd,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
            )
        },
        0
    );
    let (mut master, slave) =
        unsafe { (File::from_raw_fd(master_fd), File::from_raw_fd(slave_fd)) };
    // The protected terminal is opened non-blocking; so is this one.
    assert_eq!(
        unsafe { libc::fcntl(slave_fd, libc::F_SETFL, libc::O_NONBLOCK) },
        0
    );
    master.write_all(b"x\n").unwrap();
    let mut flushing = Flushing {
        file: slave,
        mode,
        armed: true,
        flushed_read: None,
        helper: None,
    };
    let mut byte = [0];
    let result = read_ready(
        &mut flushing,
        &mut byte,
        connectors_sdk::now_ms() + TERMINAL_BUDGET.as_millis() as u64,
    );
    if let Some(helper) = flushing.helper.take() {
        helper.join().unwrap();
    }
    assert_eq!(
        flushing.flushed_read,
        Some(std::io::ErrorKind::WouldBlock),
        "the forced interleaving did not happen"
    );
    let code = result.as_ref().err().map(|e| e.code);
    assert_eq!(code, Some(Code::Interrupted));
    assert!(signals.interrupted());
}

#[test]
fn interrupt_that_flushes_polled_input_before_the_read_is_reported_as_interrupted() {
    for mode in ["before", "after"] {
        let diagnostics = tempfile::tempfile().unwrap();
        let mut child = Child(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "local::protected::tests::flush_fixture",
                    "--nocapture",
                ])
                .env("CONNECTORS_FLUSH_FIXTURE", mode)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(diagnostics.try_clone().unwrap())
                .spawn()
                .unwrap(),
        );
        let until = Instant::now() + TERMINAL_BUDGET;
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < until, "flush fixture did not finish");
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(status.success(), "flush fixture failed ({mode}): {}", {
            let mut stderr = String::new();
            let mut diagnostics = diagnostics;
            std::io::Seek::rewind(&mut diagnostics).unwrap();
            diagnostics.read_to_string(&mut stderr).unwrap();
            stderr
        });
    }
}

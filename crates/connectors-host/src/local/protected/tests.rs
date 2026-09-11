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
        connectors_sdk::now_ms() + 5000,
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
#[test]
fn controlling_terminal_hides_input_and_restores_echo_after_sigint() {
    for mode in ["complete", "interrupt"] {
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
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "local::protected::tests::terminal_fixture",
                "--nocapture",
            ])
            .env("CONNECTORS_TERMINAL_FIXTURE", mode)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
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
        let until = Instant::now() + Duration::from_secs(5);
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
        assert!(status.success(), "fixture assertion failed");
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

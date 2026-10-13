//! Windows protected entry, run on the Windows runner: owner-only files, stdin
//! admitting a protected file and refusing a pipe, and console echo cleared
//! and restored.
use super::*;
use std::{
    ffi::OsStr,
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use windows_sys::Win32::System::Console::{AllocConsole, ENABLE_ECHO_INPUT, FreeConsole};

const DETACHED_PROCESS: u32 = 0x0000_0008;
const CHILD: &str = "CONNECTORS_PROTECTED_WINDOWS_CHILD";

fn private_directory() -> (tempfile::TempDir, File, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("private");
    let directory = fs::directory(&path, true, true).unwrap();
    (root, directory, path)
}

fn child(scenario: &str, stdin: Stdio, flags: u32) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "local::protected::windows_tests::protected_child",
            "--ignored",
            "--nocapture",
        ])
        .env(CHILD, scenario)
        .stdin(stdin)
        .creation_flags(flags)
        .status()
        .unwrap()
}

/// An owner-only file the host published is read; an extra trustee, a second
/// link and an oversized file refuse.
#[test]
fn protected_file_admission_uses_the_owner_only_rule() {
    let (_root, directory, path) = private_directory();
    fs::publish_new(
        &directory,
        OsStr::new("secret"),
        b"{\"token\":\"fictional\"}",
    )
    .unwrap();
    let secret = path.join("secret");
    let deadline = connectors_sdk::now_ms() + 2000;
    assert_eq!(
        file(&secret, deadline).unwrap().0,
        b"{\"token\":\"fictional\"}"
    );
    let status = Command::new("icacls")
        .arg(&secret)
        .args(["/grant", "*S-1-1-0:(R)"])
        .stdout(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    assert!(file(&secret, deadline).is_err());
    fs::publish_new(&directory, OsStr::new("linked"), b"x").unwrap();
    std::fs::hard_link(path.join("linked"), path.join("second")).unwrap();
    assert!(file(&path.join("linked"), deadline).is_err());
    fs::publish_new(&directory, OsStr::new("large"), &vec![b'x'; 65537]).unwrap();
    assert!(file(&path.join("large"), deadline).is_err());
}

/// Business documents read from a file within the original deadline.
#[test]
fn bounded_document_reads_a_file_and_refuses_excess() {
    let (_root, directory, path) = private_directory();
    fs::publish_new(&directory, OsStr::new("input.json"), b"{\"operations\":[]}").unwrap();
    let input = path.join("input.json");
    assert_eq!(
        document_until(Some(&input), Instant::now(), 1024)
            .unwrap_err()
            .code,
        Code::Timeout
    );
    assert_eq!(
        document_until(Some(&input), Instant::now() + Duration::from_secs(1), 4)
            .unwrap_err()
            .code,
        Code::InvalidInput
    );
    assert_eq!(
        document_until(Some(&input), Instant::now() + Duration::from_secs(1), 1024).unwrap(),
        "{\"operations\":[]}"
    );
}

/// Protected stdin admits an owner-only file and refuses a pipe.
#[test]
fn protected_stdin_admits_a_private_file_and_refuses_a_pipe() {
    let (_root, directory, path) = private_directory();
    fs::publish_new(
        &directory,
        OsStr::new("secret"),
        b"{\"token\":\"fictional\"}",
    )
    .unwrap();
    let opened = File::open(path.join("secret")).unwrap();
    assert!(child("stdin-admitted", Stdio::from(opened), 0).success());
    assert!(child("stdin-refused", Stdio::piped(), 0).success());
}

/// A process with no console refuses terminal entry; in a console, echo is
/// cleared while entry is held and the saved mode is restored on drop.
#[test]
fn terminal_entry_clears_echo_and_restores_the_console_mode() {
    assert!(child("no-console", Stdio::null(), DETACHED_PROCESS).success());
    assert!(child("console-mode", Stdio::null(), DETACHED_PROCESS).success());
}

#[test]
#[ignore = "child fixture; launched by the Windows protected entry tests"]
fn protected_child() {
    let deadline = connectors_sdk::now_ms() + 5000;
    match std::env::var(CHILD).unwrap().as_str() {
        "stdin-admitted" => assert_eq!(stdin(deadline).unwrap().0, b"{\"token\":\"fictional\"}"),
        "stdin-refused" => assert_eq!(
            stdin(deadline).err().map(|error| error.code),
            Some(Code::InvalidInput)
        ),
        "no-console" => assert!(Terminal::open().is_err()),
        "console-mode" => {
            // SAFETY: a detached process has no console; this allocates one.
            assert_ne!(unsafe { AllocConsole() }, 0);
            let before = {
                let terminal = Terminal::open().unwrap();
                let held = terminal.mode();
                assert_eq!(held & ENABLE_ECHO_INPUT, 0);
                terminal.original
            };
            assert_ne!(before & ENABLE_ECHO_INPUT, 0, "a new console echoes");
            let after = Terminal::open().unwrap();
            assert_eq!(after.original, before, "drop restored the saved mode");
            drop(after);
            // SAFETY: frees the console this child allocated.
            unsafe { FreeConsole() };
        }
        _ => panic!("unknown child scenario"),
    }
}

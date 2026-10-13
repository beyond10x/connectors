//! Adversary cases for the Windows capture, consumer launch and consumer job
//! (consumer-launch.md §8; semantics.md, "Windows platform guarantees"). Each
//! test states what the contract, or the Linux behaviour it stands in for,
//! asks of Windows; a failure is a finding, not a broken fixture. The
//! consumer is this test binary, re-entered through libtest's filter syntax
//! as `consumer_tests` does.
use super::*;
use crate::local::{filesystem::windows::final_path, runtime::artifact};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    fs::File,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const FIXTURE: &str = "local::runtime::launch::consumer_tests::adversary_b::consumer_fixture";
const GRANDCHILD: &str = "local::runtime::launch::consumer_tests::adversary_b::grandchild_fixture";
/// `CREATE_BREAKAWAY_FROM_JOB`, as the owner spawn first asks for it.
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

fn arg(prefix: &str) -> Option<String> {
    std::env::args().find_map(|a| a.strip_prefix(prefix).map(String::from))
}

/// The consumer. Modes:
/// - `grandchild`: starts a process of its own, as a consumer that runs a
///   helper does, and exits at once;
/// - `breakaway`: starts one with `CREATE_BREAKAWAY_FROM_JOB`, as
///   `owner/transport/windows.rs` `spawn` does for an owner;
/// - `socket`: opens a loopback listener;
/// - `names`: records the names of its environment variables.
///
/// It writes `ok`, `refused …` or the names to its out file and exits 0. The
/// ordinary test invocation does nothing.
#[test]
fn consumer_fixture() {
    let (Some(out), Some(mode)) = (arg("adversary-b-out="), arg("adversary-b-mode=")) else {
        return;
    };
    let result = match mode.as_str() {
        "grandchild" | "breakaway" => {
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command
                .args([
                    "--exact",
                    GRANDCHILD,
                    "--",
                    &format!("adversary-b-marker={out}.marker"),
                ])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            if mode == "breakaway" {
                command.creation_flags(CREATE_BREAKAWAY_FROM_JOB);
            }
            match command.spawn() {
                Ok(_) => "ok".to_owned(),
                Err(error) => format!("refused {error}"),
            }
        }
        "socket" => match std::net::TcpListener::bind("127.0.0.1:0") {
            Ok(_) => "ok".to_owned(),
            Err(error) => format!("refused {error}"),
        },
        "names" => {
            let mut names: Vec<String> = std::env::vars_os()
                .map(|(name, _)| name.to_string_lossy().into_owned())
                .collect();
            names.sort();
            names.join(",")
        }
        other => format!("unknown mode {other}"),
    };
    std::fs::write(&out, result).unwrap();
    std::process::exit(0);
}

/// A process a consumer starts: it waits a moment, after its consumer has
/// exited, and then leaves a marker.
#[test]
fn grandchild_fixture() {
    let Some(marker) = arg("adversary-b-marker=") else {
        return;
    };
    std::thread::sleep(Duration::from_millis(1500));
    std::fs::write(marker, b"alive").unwrap();
}

/// A capture of this test binary and the copy's path, read from its handle
/// without making a `Launch` (which would remove the copy when dropped).
fn capture_self() -> (artifact::Image, PathBuf) {
    let exe = std::env::current_exe().unwrap();
    let hash = hex::encode(Sha256::digest(std::fs::read(&exe).unwrap()));
    let image = artifact::capture(
        File::open(&exe).unwrap(),
        &hash,
        Instant::now() + Duration::from_secs(30),
    )
    .unwrap();
    let copy = final_path(&image).unwrap();
    (image, copy)
}

/// Whether the copy is still there. Removes it and its directory either way,
/// so a failing case leaves nothing in `%TEMP%` itself.
fn left_behind(copy: &Path) -> bool {
    let left = copy.exists();
    let _ = std::fs::remove_file(copy);
    if let Some(directory) = copy.parent() {
        let _ = std::fs::remove_dir(directory);
    }
    left
}

fn launch(mode: &str, out: &Path, pass_env: &[&str], caller: Vec<(OsString, OsString)>) -> i32 {
    let (image, copy) = capture_self();
    let consumer = Consumer::new(
        image,
        sealed(b"fictional credential").unwrap(),
        vec!["--exact".into(), FIXTURE.into(), "--".into()],
        pass_env.iter().map(|prefix| (*prefix).to_owned()).collect(),
    );
    let code = consumer
        .run(
            &[
                format!("adversary-b-out={}", out.display()),
                format!("adversary-b-mode={mode}"),
            ],
            caller,
        )
        .unwrap();
    let _ = left_behind(&copy);
    code
}

fn wait_for(path: &Path, wait: Duration) -> bool {
    let deadline = Instant::now() + wait;
    while !path.exists() {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    true
}

/// A launch refused before the consumer exists leaves nothing behind
/// (consumer-launch.md §8, "Image"): `run` takes the copy's custody before it
/// refuses an argument holding NUL.
#[test]
fn adversary_b_a_nul_argument_leaves_no_copy_behind() {
    let (image, copy) = capture_self();
    let consumer = Consumer::new(
        image,
        sealed(b"fictional credential").unwrap(),
        Vec::new(),
        BTreeSet::new(),
    );
    assert!(matches!(
        consumer.run(&["a\0b".into()], Vec::new()),
        Err(Failure::InvalidInput)
    ));
    assert!(
        !left_behind(&copy),
        "a refused launch left its image copy in %TEMP%"
    );
}

/// A creation that fails leaves nothing behind. `CreateProcessW` refuses a
/// command line over 32,767 characters; the `Launch` holds the copy's only
/// handle, which it closes before it removes the copy.
#[test]
fn adversary_b_a_failed_creation_leaves_no_copy_behind() {
    let (image, copy) = capture_self();
    let consumer = Consumer::new(
        image,
        sealed(b"fictional credential").unwrap(),
        Vec::new(),
        BTreeSet::new(),
    );
    assert!(consumer.run(&["x".repeat(40_000)], Vec::new()).is_err());
    assert!(
        !left_behind(&copy),
        "a failed creation left its image copy in %TEMP%"
    );
}

/// `deny_write_copy`: from the deny-write open, no process can rename the copy
/// or move it from under the path the launch creates the process from. The
/// copy itself refuses a rename; its directory must too, or a process of the
/// same user moves it away and puts another `image.exe` at the launch path
/// between `launch_path`'s identity check and `CreateProcessW`. The image
/// holds a handle on the directory that does not share deletion.
#[test]
fn adversary_b_the_copy_directory_cannot_be_renamed_while_the_copy_is_held() {
    let (image, copy) = capture_self();
    let directory = copy.parent().unwrap().to_path_buf();
    let moved = directory.with_file_name(format!("adversary-b-{}", uuid::Uuid::new_v4()));
    let renamed = std::fs::rename(&directory, &moved).is_ok();
    if renamed && std::fs::rename(&moved, &directory).is_err() {
        drop(image);
        let _ = left_behind(&moved.join("image.exe"));
    } else {
        drop(image);
        let _ = left_behind(&copy);
    }
    assert!(
        !renamed,
        "the directory of a held copy was renamed: the launch path can name another file"
    );
}

/// On Linux a process the consumer starts lives on after the consumer exits
/// (`PR_SET_PDEATHSIG` is the consumer's alone). On Windows `run` closes the
/// consumer's kill-on-close job as soon as the consumer has exited, which
/// ends every process the consumer started without breakaway, though the
/// launcher did not die: the Windows difference consumer-launch.md §8 states
/// ("The consumer's own children").
#[test]
fn adversary_b_a_grandchild_ends_with_a_consumer_that_exited() {
    let root = tempfile::tempdir().unwrap();
    let out = root.path().join("out");
    assert_eq!(launch("grandchild", &out, &[], Vec::new()), 0);
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "ok");
    let marker = PathBuf::from(format!("{}.marker", out.display()));
    assert!(
        !wait_for(&marker, Duration::from_secs(5)),
        "a process the consumer started without breakaway outlived the consumer's job"
    );
}

/// A consumer that runs `connectors` may start the shared owner. The owner
/// spawn asks for `CREATE_BREAKAWAY_FROM_JOB` and, refused, is created inside
/// the caller's job: inside the consumer job, which has no
/// `JOB_OBJECT_LIMIT_BREAKAWAY_OK`, so the owner and, through its own job,
/// every adapter end when that consumer exits. The consumer's job permits
/// breakaway on request (consumer-launch.md §8, "Breakaway").
#[test]
fn adversary_b_a_consumer_can_start_a_process_that_leaves_its_job() {
    let root = tempfile::tempdir().unwrap();
    let out = root.path().join("out");
    assert_eq!(launch("breakaway", &out, &[], Vec::new()), 0);
    let result = std::fs::read_to_string(&out).unwrap();
    // Let a started process finish before its directory goes.
    let _ = wait_for(
        &PathBuf::from(format!("{}.marker", out.display())),
        Duration::from_secs(10),
    );
    assert_eq!(
        result, "ok",
        "breakaway refused in the consumer job: an owner started there ends with the consumer"
    );
}

/// The adapter is given `SystemRoot` because Windows needs it to start its
/// network stack (`runtime/process.rs` `spawn_windows`); without it a
/// consumer whose entry has no `pass_env` for it could not open a socket
/// (`WSAStartup` fails with `WSAEPROVIDERFAILEDINIT`). The consumer gets it
/// too (consumer-launch.md §8, "Environment").
#[test]
fn adversary_b_a_consumer_with_an_empty_environment_can_open_a_socket() {
    let root = tempfile::tempdir().unwrap();
    let out = root.path().join("out");
    assert_eq!(launch("socket", &out, &[], Vec::new()), 0);
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        "ok",
        "a consumer without SystemRoot cannot use the network"
    );
}

/// Windows compares variable names without regard to case, and `run` already
/// drops `CONNECTORS_CREDENTIAL_HANDLE` in any letter case; the `pass_env`
/// prefix match does too, so an entry naming `PATH` selects the caller's
/// `Path`, the name Windows gives it.
#[test]
fn adversary_b_pass_env_matches_a_name_in_any_letter_case() {
    let root = tempfile::tempdir().unwrap();
    let out = root.path().join("out");
    let caller = vec![(OsString::from("Path"), OsString::from(r"C:\Windows"))];
    assert_eq!(launch("names", &out, &["PATH"], caller), 0);
    let names = std::fs::read_to_string(&out).unwrap();
    assert!(
        names
            .split(',')
            .any(|name| name.eq_ignore_ascii_case("PATH")),
        "the caller's Path did not reach a consumer whose pass_env names PATH: {names}"
    );
}

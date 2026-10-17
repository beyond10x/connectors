//! story:journey-owners-are-stopped. The owners a journey's CLI starts, found
//! by their exact argument vector, and two journeys that hold the harness to
//! stopping them: one that panics after its owner started, and one whose owner
//! is tied to a process that then ends (contracts/cli/v1alpha1/owner.md).
//!
//! Linux only: the owners are found through `/proc` and held by pidfd, which a
//! reused process id cannot alias.
#![cfg(target_os = "linux")]
use super::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

/// One live owner of a journey's state directory, held by a pidfd.
pub(in super::super) struct Owner {
    pub(in super::super) pid: u32,
    fd: OwnedFd,
}

impl Owner {
    /// Whether the process has exited, waiting at most `bound`.
    pub(in super::super) fn exited_within(&self, bound: Duration) -> bool {
        let mut event = libc::pollfd {
            fd: self.fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let millis = bound.as_millis().min(i32::MAX as u128) as i32;
        // SAFETY: one valid pollfd; a pidfd is readable once its process exited.
        let ready = unsafe { libc::poll(&mut event, 1, millis) };
        ready == 1 && event.revents & libc::POLLIN != 0
    }

    /// Sends SIGKILL through the pidfd: never to a process that merely reused
    /// the id. An already exited owner is not an error.
    pub(in super::super) fn kill(&self) {
        // SAFETY: the pidfd is held; no siginfo, no flags.
        let result = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                self.fd.as_raw_fd(),
                libc::SIGKILL,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            assert_eq!(
                error.raw_os_error(),
                Some(libc::ESRCH),
                "owner signal: {error}"
            );
        }
    }
}

fn arguments(pid: u32) -> Option<Vec<Vec<u8>>> {
    let bytes = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    Some(
        bytes
            .split(|byte| *byte == 0)
            .filter(|part| !part.is_empty())
            .map(<[u8]>::to_vec)
            .collect(),
    )
}

/// Every live `__connectors-owner <config> <state>` of `paths`, the arguments
/// the CLI starts an owner with (`transport.rs` `spawn`).
pub(in super::super) fn owners_of(paths: &Paths) -> Vec<Owner> {
    use std::os::unix::ffi::OsStrExt;
    let expected: Vec<Vec<u8>> = [
        std::ffi::OsStr::new("__connectors-owner"),
        paths.config.as_os_str(),
        paths.state.as_os_str(),
    ]
    .iter()
    .map(|part| part.as_bytes().to_vec())
    .collect();
    let matches =
        |pid: u32| arguments(pid).is_some_and(|argv| argv.get(1..) == Some(&expected[..]));
    let mut owners = Vec::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        return owners;
    };
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        if !matches(pid) {
            continue;
        }
        // SAFETY: pidfd_open takes a scalar id; success returns a new descriptor.
        let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        if raw < 0 {
            continue;
        }
        // SAFETY: the descriptor was just returned to this process.
        let fd = unsafe { OwnedFd::from_raw_fd(raw as i32) };
        // The id still names an owner of these paths now that it is held: it
        // was not reused between the read and the open.
        if matches(pid) {
            owners.push(Owner { pid, fd });
        }
    }
    owners
}

/// Stops each owner: one that has not exited within `grace` of the harness's
/// shutdown request is killed through its pidfd, and every one is waited for.
/// The owners are not this process's children (each starts its own session and
/// is reparented when its CLI exits), so the kernel reaps them once exited.
/// Returns the ids that had to be killed.
pub(in super::super) fn stop(owners: Vec<Owner>, grace: Duration) -> Vec<u32> {
    let mut killed = Vec::new();
    for owner in owners {
        if owner.exited_within(grace) {
            continue;
        }
        owner.kill();
        killed.push(owner.pid);
        assert!(
            owner.exited_within(Duration::from_secs(5)),
            "owner {} survived SIGKILL",
            owner.pid
        );
    }
    killed
}

fn connected(provider: &Provider, cli: &Cli) {
    let credential = provider.root.path().join("private/credential.json");
    private(&credential, &token(true).0);
    let connected = success(cli.run(&[
        "connections",
        "connect",
        "--adapter",
        "gitlab",
        "--profile",
        "gitlab.pat",
        "--credential-file",
        credential.to_str().unwrap(),
    ]))["connection"]
        .clone();
    assert_eq!(connected["summary"]["state"], "ready");
}

/// Acceptance: "after a journey test, pass or fail (panic included), no owner
/// it started is alive". The journey panics once its owner serves; when its
/// harness has been dropped, that owner has exited.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_journey_that_panics_after_its_owner_started_leaves_no_owner_alive() {
    let provider = Provider::new();
    let custody = Custody::new(provider.root.path());
    let started = std::sync::Mutex::new(Vec::new());
    let journey = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let cli = Cli::new(provider.root.path());
        configure(&cli, &provider, &custody);
        connected(&provider, &cli);
        let owners = owners_of(&cli.paths);
        assert_eq!(owners.len(), 1, "the journey's owner serves");
        *started.lock().unwrap() = owners;
        panic!("the journey fails after its owner started");
    }));
    assert!(journey.is_err());
    let owners = started.into_inner().unwrap();
    assert_eq!(owners.len(), 1);
    for owner in &owners {
        assert!(
            owner.exited_within(Duration::ZERO),
            "owner {} is alive after its panicked journey's harness was dropped",
            owner.pid
        );
    }
    drop(custody);
}

/// Acceptance: "a test owner exits when the process that started it is gone".
/// The harness names a stand-in process instead of itself; the production CLI
/// passes it to the owner it starts, and once it ends the owner exits within
/// the bound, with the harness still running.
#[test]
#[ignore = "requires built production CLI and qualified disposable Secret Service"]
fn a_journey_owner_exits_when_the_process_it_is_tied_to_ends() {
    let provider = Provider::new();
    let custody = Custody::new(provider.root.path());
    let mut stand_in = OwnedProcess(Command::new("/bin/sleep").arg("600").spawn().unwrap());
    let mut cli = Cli::new(provider.root.path());
    cli.parent = Some(stand_in.0.id());
    configure(&cli, &provider, &custody);
    connected(&provider, &cli);
    let owners = owners_of(&cli.paths);
    assert_eq!(owners.len(), 1, "the journey's owner serves");
    assert!(!owners[0].exited_within(Duration::from_millis(500)));
    stand_in.0.kill().unwrap();
    stand_in.0.wait().unwrap();
    let bound = Duration::from_secs(5);
    assert!(
        owners[0].exited_within(bound),
        "owner {} still serves {bound:?} after the process it is tied to ended",
        owners[0].pid
    );
    assert!(owners_of(&cli.paths).is_empty());
}

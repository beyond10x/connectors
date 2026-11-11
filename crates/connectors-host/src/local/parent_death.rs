//! A child that ends with the process that started it
//! (story:test-children-die-with-their-parent).
//!
//! On Linux [`bind`] asks the kernel to send a child `SIGKILL` when its parent
//! dies, so a parent killed without running any `Drop`, such as a test that
//! the build slot's timeout ends, leaves nothing behind. Other platforms have
//! no parent-death signal and the command is left as it is (`parent_death_kill`
//! in contracts/cli/v1alpha1/semantics.md).
//!
//! The kernel ties the signal to the parent *thread* that started the child,
//! not to the process: the thread that calls `spawn` must outlive the child.
//! The callers here start a child and wait for it on the same thread.
//!
//! An owner is not started through this. It outlives the CLI that started it
//! by design (contracts/cli/v1alpha1/owner.md); only a test build ties it to a
//! named process, by `CONNECTORS_TEST_OWNER_PARENT`.
use std::process::Command;

/// Have the kernel end `command`'s child with `SIGKILL` when the process
/// starting it dies. The child checks, before it executes, that its parent is
/// still the process that bound it: a parent that died in between would have
/// left the signal unset for good.
pub(crate) fn bind(command: &mut Command) -> &mut Command {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: getpid has no preconditions.
        let parent = unsafe { libc::getpid() };
        // SAFETY: the hook calls only async-signal-safe functions on a
        // captured scalar.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0
                    || libc::getppid() != parent
                {
                    return Err(std::io::Error::from_raw_os_error(libc::ECHILD));
                }
                Ok(())
            });
        }
    }
    command
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) mod fixture {
    use std::{
        path::Path,
        time::{Duration, Instant},
    };

    /// Kills the process it names when dropped, so a failing case leaves none.
    pub(crate) struct Reaper(pub(crate) libc::pid_t);
    impl Drop for Reaper {
        fn drop(&mut self) {
            // SAFETY: signals the one pid a case read from its own fixture.
            unsafe { libc::kill(self.0, libc::SIGKILL) };
        }
    }

    /// Whether `pid` is still a process: a zombie waiting to be reaped is not.
    pub(crate) fn running(pid: libc::pid_t) -> bool {
        match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            Ok(stat) => stat
                .rsplit_once(')')
                .and_then(|(_, rest)| rest.split_whitespace().next().map(|state| state != "Z"))
                .unwrap_or(false),
            Err(_) => false,
        }
    }

    /// Writes `pid` where [`pid_in`] reads it, whole or not at all.
    pub(crate) fn publish(file: &Path, pid: u32) {
        let staging = file.with_extension("tmp");
        std::fs::write(&staging, format!("{pid}\n")).unwrap();
        std::fs::rename(staging, file).unwrap();
    }

    /// The pid the harness published in `file`.
    pub(crate) fn pid_in(file: &Path, harness: &mut std::process::Child) -> libc::pid_t {
        let until = Instant::now() + Duration::from_secs(60);
        loop {
            if let Ok(text) = std::fs::read_to_string(file) {
                return text.trim().parse().unwrap();
            }
            assert!(Instant::now() < until, "the child never started");
            assert!(harness.try_wait().unwrap().is_none(), "harness ended early");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// SIGKILLs `harness`, which runs no `Drop`, and requires `child` gone
    /// within two seconds.
    pub(crate) fn kill_harness_and_expect_gone(
        harness: &mut std::process::Child,
        child: libc::pid_t,
    ) {
        assert!(running(child));
        // SAFETY: signals only the harness the case started.
        assert_eq!(
            unsafe { libc::kill(harness.id() as libc::pid_t, libc::SIGKILL) },
            0
        );
        harness.wait().unwrap();
        let killed = Instant::now();
        while running(child) {
            assert!(
                killed.elapsed() < Duration::from_secs(2),
                "process {child} outlived its killed parent"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::fixture::{Reaper, kill_harness_and_expect_gone, pid_in};
    use crate::local::config::Executable;
    use std::{
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        process::{Command, Stdio},
    };

    const ROOT: &str = "CONNECTORS_TEST_PARENT_DEATH_ROOT";

    /// Run as the harness: a process that starts the catalog provider the way
    /// a derivation does and then waits for it. An ordinary run does nothing.
    #[test]
    fn harness_fixture() {
        let Some(root) = std::env::var_os(ROOT).map(PathBuf::from) else {
            return;
        };
        let provider = root.join("provider.sh");
        std::fs::write(
            &provider,
            format!(
                "#!/bin/sh\necho $$ > '{pid}.tmp'\nmv '{pid}.tmp' '{pid}'\nexec /bin/sleep 300\n",
                pid = root.join("provider.pid").display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&provider, std::fs::Permissions::from_mode(0o700)).unwrap();
        let selection = Executable {
            path: provider,
            sha256: String::new(),
            args: Vec::new(),
        };
        let _ = selection.print("--version");
    }

    /// A process that starts the provider a derivation prints from and is
    /// killed leaves no provider behind, whatever the killed process could
    /// still run.
    #[test]
    fn a_killed_harness_leaves_no_provider_running() {
        let root = tempfile::tempdir().unwrap();
        let mut harness = Command::new(std::env::current_exe().unwrap());
        harness
            .args([
                "--exact",
                "local::parent_death::tests::harness_fixture",
                "--nocapture",
            ])
            .env(ROOT, root.path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut harness = harness.spawn().unwrap();
        let provider = pid_in(&root.path().join("provider.pid"), &mut harness);
        let _reaper = Reaper(provider);
        kill_harness_and_expect_gone(&mut harness, provider);
    }
}

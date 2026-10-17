//! A process that may hold a credential in memory writes no core file: the
//! owner, which holds a created credential between the provider's answer and
//! custody, and every private adapter child, which reads it out of that answer
//! (design §8a of the catalog provider). Both call [`forbid`] first.

/// Set the core-file limit to zero, soft and hard, and on Linux mark the
/// process not dumpable (`PR_SET_DUMPABLE` 0), so neither a crash nor a
/// same-user `ptrace` or `/proc/<pid>/mem` read exposes its memory. Called
/// before the process reads any protected input. A no-op on Windows, which
/// writes no core file unless a debugger or Windows Error Reporting is
/// configured to.
pub fn forbid() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let none = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: setrlimit reads the struct it is given; lowering both limits
        // needs no privilege.
        if unsafe { libc::setrlimit(libc::RLIMIT_CORE, &none) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    #[cfg(target_os = "linux")]
    // SAFETY: PR_SET_DUMPABLE sets a flag of this process only.
    if unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    /// The limits are process-wide, so they are set in a forked child and
    /// read back there; the test process keeps its own. The child calls only
    /// the system calls [`super::forbid`] makes and their readers.
    #[test]
    fn a_process_that_forbids_core_dumps_has_no_core_limit_and_is_not_dumpable() {
        // SAFETY: the child makes system calls only and leaves through _exit.
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            let forbidden = super::forbid().is_ok();
            let mut limit = libc::rlimit {
                rlim_cur: libc::RLIM_INFINITY,
                rlim_max: libc::RLIM_INFINITY,
            };
            // SAFETY: getrlimit writes the struct it is given.
            let read = unsafe { libc::getrlimit(libc::RLIMIT_CORE, &mut limit) } == 0;
            let no_core = read && limit.rlim_cur == 0 && limit.rlim_max == 0;
            #[cfg(target_os = "linux")]
            // SAFETY: PR_GET_DUMPABLE reads a flag of this process.
            let not_dumpable = unsafe { libc::prctl(libc::PR_GET_DUMPABLE) } == 0;
            #[cfg(not(target_os = "linux"))]
            let not_dumpable = true;
            let code =
                i32::from(!forbidden) | i32::from(!no_core) << 1 | i32::from(!not_dumpable) << 2;
            // SAFETY: leave the forked child without running the parent's destructors.
            unsafe { libc::_exit(code) }
        }
        let mut status = 0;
        // SAFETY: waits for the child this test forked.
        assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
        assert!(libc::WIFEXITED(status), "the child did not exit");
        assert_eq!(
            libc::WEXITSTATUS(status),
            0,
            "bit 0: forbid failed, bit 1: a core limit remains, bit 2: still dumpable"
        );
    }
}

//! Process observations and stop fences independent of a busy adapter channel.
use super::*;
#[cfg(target_os = "linux")]
use std::{fs::File, os::fd::AsRawFd};

/// The stop path's handle on an owned child: a duplicated pidfd on Linux; on
/// macOS a share of the child's reap lock (`exact_process_signal`:
/// `child_reap_lock`).
#[cfg(target_os = "linux")]
pub(super) type Handle = File;
#[cfg(target_os = "macos")]
pub(super) type Handle = runtime::artifact::ExactChild;

pub(super) struct Owned {
    pub handle: Handle,
    pub incarnation: String,
    pub revision: String,
}
#[derive(Default)]
pub(super) struct Control {
    pub epoch: u64,
    pub child: Option<Owned>,
    pub stopping: bool,
    pub starting: bool,
    pub failed: bool,
}
impl Control {
    pub fn check(&self, epoch: u64) -> Result<()> {
        if self.stopping || self.epoch != epoch {
            Err(Code::LifecycleConflict.into())
        } else {
            Ok(())
        }
    }
    pub fn observe(
        &self,
        alias: &str,
        adapter: &Adapter,
        host: &str,
        suppressed: bool,
    ) -> Result<Value> {
        let running = self
            .child
            .as_ref()
            .map(|c| exited(&c.handle))
            .transpose()?
            .is_some_and(|v| !v);
        let state = if running {
            if self.stopping { "stopping" } else { "ready" }
        } else if self.starting {
            "starting"
        } else if suppressed {
            "suppressed"
        } else if self.failed || self.child.is_some() {
            "failed"
        } else {
            "not_running"
        };
        let revision = self
            .child
            .as_ref()
            .map(|c| c.revision.as_str())
            .unwrap_or(&adapter.configuration_revision);
        let mut observation = json!({"adapter":alias,"configuration_revision":revision,"state":state,"host_incarnation":host,"observed_at_ms":connectors_sdk::now_ms()});
        if running {
            observation["child_incarnation"] =
                json!(self.child.as_ref().ok_or(Code::Unavailable)?.incarnation);
        }
        Ok(json!({"observation":observation}))
    }
}
/// Signals the child only while its reap lock shows it unreaped; a reaped or
/// vanished child is sent nothing, as `ESRCH` is on Linux.
#[cfg(target_os = "macos")]
pub(super) fn signal(handle: &Handle, signal: i32) -> Result<()> {
    handle
        .signal(signal)
        .map(|_| ())
        .map_err(|_| Code::Unavailable.into())
}
/// `NOTE_EXIT` from kqueue `EVFILT_PROC`, or the reap.
#[cfg(target_os = "macos")]
pub(super) fn exited(handle: &Handle) -> Result<bool> {
    handle.exited().map_err(|_| Code::Unavailable.into())
}
#[cfg(target_os = "linux")]
pub(super) fn signal(handle: &File, signal: i32) -> Result<()> {
    // SAFETY: a duplicated pidfd, never a caller-supplied numeric process id.
    let result = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            handle.as_raw_fd(),
            signal,
            std::ptr::null::<libc::siginfo_t>(),
            0,
        )
    };
    if result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(Code::Unavailable.into())
    }
}
#[cfg(target_os = "linux")]
pub(super) fn exited(handle: &File) -> Result<bool> {
    let mut poll = libc::pollfd {
        fd: handle.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: this initialized pollfd and its owned descriptor remain live.
    if unsafe { libc::poll(&mut poll, 1, 0) } < 0 || poll.revents & libc::POLLNVAL != 0 {
        return Err(Code::Unavailable.into());
    }
    Ok(poll.revents & libc::POLLIN != 0)
}
pub(super) fn terminate(handle: &Handle, deadline: Instant) -> Result<()> {
    signal(handle, libc::SIGTERM)?;
    while Instant::now() < deadline {
        if exited(handle)? {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    signal(handle, libc::SIGKILL)?;
    // The worker reaps the process through its retained Child. SIGKILL is exact,
    // but the kernel need not have completed termination when signalling returns.
    Ok(())
}

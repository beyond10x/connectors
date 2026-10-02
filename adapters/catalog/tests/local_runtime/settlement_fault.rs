//! Linux-only controlled EIO on exact task-owned metadata file identities.
//! This is a fault fixture, never a security boundary: ambiguous fd mappings
//! continue and fail the fixture; they never justify an EIO to a guessed target.
use super::*;
use std::{
    io, mem,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
        unix::{fs::MetadataExt, net::UnixStream, process::CommandExt},
    },
    sync::atomic::{AtomicBool, Ordering},
    thread::JoinHandle,
};

#[repr(C)]
#[derive(Default)]
struct Data {
    nr: i32,
    arch: u32,
    ip: u64,
    args: [u64; 6],
}
#[repr(C)]
#[derive(Default)]
struct Notification {
    id: u64,
    pid: u32,
    flags: u32,
    data: Data,
}
#[repr(C)]
struct Response {
    id: u64,
    val: i64,
    error: i32,
    flags: u32,
}
#[repr(C)]
#[derive(Default)]
struct Sizes {
    notification: u16,
    response: u16,
    data: u16,
}

fn ioctl_code(nr: u32, size: usize) -> libc::c_ulong {
    ((3u32 << 30) | ((size as u32) << 16) | (33 << 8) | nr) as _
}

#[derive(Default)]
struct State {
    armed: bool,
    epoch: u64,
    denied: u64,
    target_continued: u64,
    other_continued: u64,
    canceled: u64,
    errors: Vec<String>,
    events: Vec<Value>,
    processes: std::collections::BTreeMap<u32, OwnedFd>,
}

pub(super) struct Controller {
    targets: Arc<Vec<PathBuf>>,
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    monitors: Mutex<Vec<JoinHandle<()>>>,
}

pub(super) struct Armed(Arc<Mutex<State>>);
impl Drop for Armed {
    fn drop(&mut self) {
        self.0.lock().unwrap().armed = false;
    }
}

impl Controller {
    pub(super) fn new(root: &Path) -> Self {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => {}
            host => panic!("settlement fixture requires Linux x86_64; got {host:?}"),
        }
        let actions = fs::read_to_string("/proc/sys/kernel/seccomp/actions_avail")
            .expect("seccomp actions prerequisite unavailable");
        assert!(
            actions.split_whitespace().any(|v| v == "user_notif"),
            "seccomp USER_NOTIF prerequisite unavailable"
        );
        let mut sizes = Sizes::default();
        // SAFETY: kernel writes the documented three-u16 size structure.
        assert_eq!(
            unsafe { libc::syscall(libc::SYS_seccomp, 3, 0, &mut sizes) },
            0,
            "seccomp notification ABI query: {}",
            io::Error::last_os_error()
        );
        assert_eq!(
            (
                sizes.notification as usize,
                sizes.response as usize,
                sizes.data as usize
            ),
            (
                mem::size_of::<Notification>(),
                mem::size_of::<Response>(),
                mem::size_of::<Data>()
            ),
            "unsupported seccomp notification ABI"
        );
        let state_dir = root.canonicalize().unwrap().join("cli/state");
        let targets = [
            "metadata.sqlite3",
            "metadata.sqlite3-wal",
            "metadata.sqlite3-shm",
            "metadata.sqlite3-journal",
        ]
        .into_iter()
        .map(|name| state_dir.join(name))
        .collect();
        Self {
            targets: Arc::new(targets),
            state: Arc::new(Mutex::new(State::default())),
            stop: Arc::new(AtomicBool::new(false)),
            monitors: Mutex::new(Vec::new()),
        }
    }

    /// Start the unfiltered receiver before spawn can block on its exec pipe.
    pub(super) fn configure(&self, command: &mut Command) {
        let (receiver, sender) = UnixStream::pair().unwrap();
        let state = self.state.clone();
        let targets = self.targets.clone();
        let stop = self.stop.clone();
        let monitor = std::thread::spawn(move || monitor(receiver, targets, state, stop));
        self.monitors.lock().unwrap().push(monitor);
        let mut filter = vec![
            libc::sock_filter {
                code: 0x20,
                jt: 0,
                jf: 0,
                k: 4,
            },
            libc::sock_filter {
                code: 0x15,
                jt: 1,
                jf: 0,
                k: 0xc000003e,
            },
            libc::sock_filter {
                code: 6,
                jt: 0,
                jf: 0,
                k: 0x80000000,
            },
            libc::sock_filter {
                code: 0x20,
                jt: 0,
                jf: 0,
                k: 0,
            },
        ];
        for nr in [
            libc::SYS_write,
            libc::SYS_pwrite64,
            libc::SYS_writev,
            libc::SYS_pwritev,
            libc::SYS_pwritev2,
            libc::SYS_fsync,
            libc::SYS_fdatasync,
            libc::SYS_ftruncate,
        ] {
            filter.push(libc::sock_filter {
                code: 0x15,
                jt: 0,
                jf: 1,
                k: nr as u32,
            });
            filter.push(libc::sock_filter {
                code: 6,
                jt: 0,
                jf: 0,
                k: 0x7fc00000,
            });
        }
        filter.push(libc::sock_filter {
            code: 6,
            jt: 0,
            jf: 0,
            k: 0x7fff0000,
        });
        // SAFETY: the child hook only uses preallocated memory and
        // async-signal-safe syscalls. No locks, allocation or Rust unwinding.
        unsafe {
            command.pre_exec(move || {
                let program = libc::sock_fprog {
                    len: filter.len() as u16,
                    filter: filter.as_ptr().cast_mut(),
                };
                if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                    return Err(io::Error::last_os_error());
                }
                let listener = libc::syscall(libc::SYS_seccomp, 1, 8, &program) as i32;
                if listener < 0 {
                    return Err(io::Error::last_os_error());
                }
                let result = send_listener(sender.as_raw_fd(), listener);
                libc::close(listener);
                result
            });
        }
    }

    pub(super) fn arm(&self) -> Armed {
        assert_eq!(
            self.targets[0].parent().unwrap().canonicalize().unwrap(),
            self.targets[0].parent().unwrap()
        );
        let mut state = self.state.lock().unwrap();
        if state.armed || !state.errors.is_empty() || state.target_continued == 0 {
            let message = format!(
                "cannot arm fixture: armed={}, target_continued={}, errors={:?}",
                state.armed, state.target_continued, state.errors
            );
            // A fixture assertion must not poison the monitor's cleanup lock.
            drop(state);
            panic!("{message}");
        }
        state.epoch += 1;
        state.armed = true;
        Armed(self.state.clone())
    }

    pub(super) fn disarm(&self) {
        self.state.lock().unwrap().armed = false;
    }

    pub(super) fn continued(&self) -> u64 {
        self.state.lock().unwrap().target_continued
    }

    pub(super) fn assert_resumed(&self, before: u64) {
        assert!(
            self.continued() > before,
            "no metadata I/O resumed after disarm"
        );
    }

    /// Revoked replay intentionally need not write. This independent fixture
    /// control synchronizes an existing read-only file, then runs CLI --version;
    /// it proves restoration without claiming production recovery or mutation.
    pub(super) fn prove_readonly_sync_restored(&self, binary: &Path, mode: u8) {
        let directory =
            filesystem::directory(self.targets[0].parent().unwrap(), false, true).unwrap();
        let file =
            filesystem::private_file_at(&directory, std::ffi::OsStr::new("metadata.sqlite3"))
                .unwrap();
        let identity = file.metadata().unwrap();
        let named = fs::symlink_metadata(&self.targets[0]).unwrap();
        assert_eq!((identity.dev(), identity.ino()), (named.dev(), named.ino()));
        // SAFETY: query access mode of the held verified file descriptor.
        assert_eq!(
            unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) } & libc::O_ACCMODE,
            libc::O_RDONLY
        );
        let state = self.state.lock().unwrap();
        let (armed, before, denied, event_start) = (
            state.armed,
            state.target_continued,
            state.denied,
            state.events.len(),
        );
        drop(state);
        assert!(!armed);
        let mut command = Command::new(binary);
        command
            .arg("--version")
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        self.configure(&mut command);
        // SAFETY: registered after the filter/bootstrap hook. Only genuine
        // fsync on the preopened read-only descriptor; no allocation or locks.
        unsafe {
            command.pre_exec(move || {
                if libc::fsync(file.as_raw_fd()) == 0 {
                    Ok(())
                } else {
                    Err(io::Error::last_os_error())
                }
            });
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "fixture fsync/CLI version failed: {output:?}"
        );
        self.assert_resumed(before);
        let state = self.state.lock().unwrap();
        let found = state.events[event_start..].iter().any(|event| {
            event["syscall"] == libc::SYS_fsync
                && event["response"] == "CONTINUE"
                && event["device"] == identity.dev()
                && event["inode"] == identity.ino()
        });
        let after_denied = state.denied;
        drop(state);
        assert!(found, "missing exact-file fsync CONTINUE control");
        assert_eq!(denied, after_denied);
        self.evidence(mode, "fixture-readonly-fsync-restored", true);
        eprintln!(
            "catalog mode {mode}: fixture read-only fsync succeeded; production CLI --version exit0 (not production recovery)"
        );
    }

    pub(super) fn evidence(&self, mode: u8, phase: &str, require_eio: bool) {
        let state = self.state.lock().unwrap();
        eprintln!(
            "catalog seccomp mode {mode} {phase}: {}",
            json!({
                "armed":state.armed,"epoch":state.epoch,"denied":state.denied,
                "target_continued":state.target_continued,"other_continued":state.other_continued,
                "canceled":state.canceled,"errors":state.errors,"events":state.events,
            })
        );
        let (errors, target, other, denied) = (
            state.errors.clone(),
            state.target_continued,
            state.other_continued,
            state.denied,
        );
        // The assertion may unwind while live tracees still need CONTINUE.
        drop(state);
        assert!(errors.is_empty(), "ambiguous seccomp fixture: {:?}", errors);
        assert!(target > 0 && other > 0, "missing real CONTINUE controls");
        if require_eio {
            assert!(denied > 0, "no actual targeted EIO was returned");
        } else {
            assert_eq!(denied, 0, "unarmed control injected an error");
        }
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        self.disarm();
        // Every filtered process seen at a notification has a stable pidfd.
        // Normal Cli shutdown precedes this fallback. Keep servicing CONTINUE
        // while forcibly retiring any remaining owned tracee during unwind.
        let processes = mem::take(&mut self.state.lock().unwrap().processes);
        let mut forced = false;
        for (pid, process) in processes {
            let mut event = libc::pollfd {
                fd: process.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: the held pidfd was opened while a validated notification
            // pinned the identity of this fixture's filtered process.
            if unsafe { libc::poll(&mut event, 1, 0) } != 1 {
                forced = true;
                let sent = unsafe {
                    libc::syscall(
                        libc::SYS_pidfd_send_signal,
                        process.as_raw_fd(),
                        libc::SIGKILL,
                        std::ptr::null::<libc::siginfo_t>(),
                        0,
                    )
                };
                eprintln!("seccomp cleanup pid={pid}: signal={sent}");
                assert_eq!(
                    unsafe { libc::poll(&mut event, 1, 5000) },
                    1,
                    "filtered process survived cleanup"
                );
            }
            assert_ne!(event.revents & libc::POLLIN, 0);
        }
        self.stop.store(true, Ordering::SeqCst);
        for monitor in self.monitors.get_mut().unwrap().drain(..) {
            let result = monitor.join();
            if std::thread::panicking() {
                eprintln!("seccomp monitor joined during unwind: {result:?}");
            } else {
                result.expect("seccomp monitor panicked");
            }
        }
        eprintln!("seccomp listener monitors joined and descriptors closed after owner cleanup");
        if !std::thread::panicking() {
            assert!(!forced, "filtered process leaked past normal owner cleanup");
        }
    }
}

// Called only inside pre_exec; stack-only ancillary construction.
unsafe fn send_listener(socket: RawFd, listener: RawFd) -> io::Result<()> {
    let mut byte = 1u8;
    let mut iov = libc::iovec {
        iov_base: (&mut byte as *mut u8).cast(),
        iov_len: 1,
    };
    let mut ancillary = [0usize; 4];
    // SAFETY: all pointers describe local, aligned initialized storage.
    unsafe {
        let mut message: libc::msghdr = mem::zeroed();
        message.msg_iov = &mut iov;
        message.msg_iovlen = 1;
        message.msg_control = ancillary.as_mut_ptr().cast();
        message.msg_controllen = libc::CMSG_SPACE(4) as usize;
        let header = libc::CMSG_FIRSTHDR(&message);
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        (*header).cmsg_len = libc::CMSG_LEN(4) as usize;
        std::ptr::write(libc::CMSG_DATA(header).cast::<i32>(), listener);
        if libc::sendmsg(socket, &message, libc::MSG_NOSIGNAL) != 1 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

fn receive_listener(socket: RawFd) -> io::Result<OwnedFd> {
    let mut byte = 0u8;
    let mut iov = libc::iovec {
        iov_base: (&mut byte as *mut u8).cast(),
        iov_len: 1,
    };
    let mut ancillary = [0usize; 4];
    // SAFETY: receive one descriptor into aligned bounded local storage; take
    // ownership before validation so every received fd closes on refusal.
    unsafe {
        let mut message: libc::msghdr = mem::zeroed();
        message.msg_iov = &mut iov;
        message.msg_iovlen = 1;
        message.msg_control = ancillary.as_mut_ptr().cast();
        message.msg_controllen = mem::size_of_val(&ancillary);
        let count = libc::recvmsg(socket, &mut message, libc::MSG_CMSG_CLOEXEC);
        if count < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut descriptors = Vec::new();
        let mut header = libc::CMSG_FIRSTHDR(&message);
        let mut headers = 0;
        while !header.is_null() {
            headers += 1;
            if (*header).cmsg_level == libc::SOL_SOCKET && (*header).cmsg_type == libc::SCM_RIGHTS {
                let bytes = (*header)
                    .cmsg_len
                    .saturating_sub(libc::CMSG_LEN(0) as usize);
                for index in 0..bytes / mem::size_of::<i32>() {
                    descriptors.push(OwnedFd::from_raw_fd(std::ptr::read(
                        libc::CMSG_DATA(header).cast::<i32>().add(index),
                    )));
                }
            }
            header = libc::CMSG_NXTHDR(&message, header);
        }
        if count != 1
            || byte != 1
            || headers != 1
            || descriptors.len() != 1
            || message.msg_flags & (libc::MSG_CTRUNC | libc::MSG_TRUNC) != 0
        {
            return Err(io::Error::other("noncanonical listener transfer"));
        }
        let listener = descriptors.pop().unwrap();
        let flags = libc::fcntl(listener.as_raw_fd(), libc::F_GETFL);
        if flags < 0
            || libc::fcntl(
                listener.as_raw_fd(),
                libc::F_SETFL,
                flags | libc::O_NONBLOCK,
            ) < 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(listener)
    }
}

fn ready(fd: RawFd, events: i16) -> io::Result<i16> {
    let mut poll = libc::pollfd {
        fd,
        events,
        revents: 0,
    };
    // SAFETY: one initialized poll entry held for the syscall.
    let result = unsafe { libc::poll(&mut poll, 1, 50) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(poll.revents)
    }
}
fn valid(listener: RawFd, id: &u64) -> io::Result<bool> {
    // SAFETY: ioctl reads one live notification id.
    if unsafe { libc::ioctl(listener, 0x40082102u64 as libc::c_ulong, id) } == 0 {
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ENOENT) {
        Ok(false)
    } else {
        Err(error)
    }
}

fn monitor(
    socket: UnixStream,
    targets: Arc<Vec<PathBuf>>,
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
) {
    let record_error = |error: String| state.lock().unwrap().errors.push(error);
    let until = Instant::now() + Duration::from_secs(10);
    let listener = loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        match ready(socket.as_raw_fd(), libc::POLLIN) {
            Ok(events) if events & libc::POLLIN != 0 => {
                match receive_listener(socket.as_raw_fd()) {
                    Ok(fd) => break fd,
                    Err(error) => {
                        record_error(format!("listener transfer: {error}"));
                        return;
                    }
                }
            }
            Ok(events) if events & (libc::POLLHUP | libc::POLLERR) != 0 => {
                record_error("CLI ended before listener transfer".into());
                return;
            }
            Err(error) if error.kind() != io::ErrorKind::Interrupted => {
                record_error(format!("listener poll: {error}"));
                return;
            }
            _ => {}
        }
        if Instant::now() >= until {
            record_error("listener transfer deadline".into());
            return;
        }
    };
    drop(socket);
    while !stop.load(Ordering::SeqCst) {
        match ready(listener.as_raw_fd(), libc::POLLIN) {
            Ok(events) if events & libc::POLLIN != 0 => {}
            Ok(events) if events & libc::POLLHUP != 0 => break,
            Err(error) if error.kind() != io::ErrorKind::Interrupted => {
                record_error(format!("notification poll: {error}"));
                continue;
            }
            _ => continue,
        }
        let mut notification = Notification::default();
        // SAFETY: kernel ABI size was validated before selecting the fixture.
        if unsafe {
            libc::ioctl(
                listener.as_raw_fd(),
                ioctl_code(0, mem::size_of::<Notification>()),
                &mut notification,
            )
        } != 0
        {
            let error = io::Error::last_os_error();
            if matches!(
                error.raw_os_error(),
                Some(libc::ENOENT | libc::EAGAIN | libc::EINTR)
            ) {
                state.lock().unwrap().canceled += 1;
                continue;
            }
            record_error(format!("notification receive: {error}"));
            continue;
        }
        match valid(listener.as_raw_fd(), &notification.id) {
            Ok(true) => {}
            Ok(false) => {
                state.lock().unwrap().canceled += 1;
                continue;
            }
            Err(error) => {
                record_error(format!("notification validity: {error}"));
            }
        }
        if let Err(error) = remember_process(&notification, listener.as_raw_fd(), &state) {
            if matches!(valid(listener.as_raw_fd(), &notification.id), Ok(false)) {
                state.lock().unwrap().canceled += 1;
                continue;
            }
            record_error(format!("tracee identity: {error}"));
        }
        let observed = identify(&notification, &targets);
        let exact = match observed {
            Ok(identity) => identity,
            Err(error) => {
                if matches!(valid(listener.as_raw_fd(), &notification.id), Ok(false)) {
                    state.lock().unwrap().canceled += 1;
                    continue;
                }
                record_error(format!(
                    "ambiguous fd tid={} fd={}: {error}",
                    notification.pid, notification.data.args[0]
                ));
                None
            }
        };
        match valid(listener.as_raw_fd(), &notification.id) {
            Ok(false) => {
                state.lock().unwrap().canceled += 1;
                continue;
            }
            Err(error) => record_error(format!("final notification validity: {error}")),
            Ok(true) => {}
        }
        let mut state = state.lock().unwrap();
        let deny = state.armed && exact.is_some() && state.errors.is_empty();
        let response = Response {
            id: notification.id,
            val: 0,
            error: if deny { -libc::EIO } else { 0 },
            flags: if deny { 0 } else { 1 },
        };
        // SAFETY: response references the freshly validated notification. A
        // cancellation racing this send is an explicit non-denial observation.
        if unsafe {
            libc::ioctl(
                listener.as_raw_fd(),
                ioctl_code(1, mem::size_of::<Response>()),
                &response,
            )
        } != 0
        {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ENOENT) {
                state.canceled += 1;
            } else {
                state.errors.push(format!("notification response: {error}"));
            }
            continue;
        }
        if deny {
            state.denied += 1;
        } else if exact.is_some() {
            state.target_continued += 1;
        } else {
            state.other_continued += 1;
        }
        if let Some((path, device, inode)) = exact {
            let epoch = state.epoch;
            if state.events.len() == 100_000 {
                state
                    .errors
                    .push("bounded I/O evidence capacity exhausted".into());
            } else {
                state.events.push(json!({"id":notification.id,"tid":notification.pid,"fd":notification.data.args[0],"syscall":notification.data.nr,"path":path,"device":device,"inode":inode,"epoch":epoch,"response":if deny{"EIO"}else{"CONTINUE"}}));
            }
        }
    }
}

fn remember_process(
    notification: &Notification,
    listener: RawFd,
    state: &Mutex<State>,
) -> io::Result<()> {
    let status = fs::read_to_string(format!("/proc/{}/status", notification.pid))?;
    let pid: u32 = status
        .lines()
        .find_map(|line| line.strip_prefix("Tgid:"))
        .ok_or_else(|| io::Error::other("tracee lacks Tgid"))?
        .trim()
        .parse()
        .map_err(io::Error::other)?;
    let mut state = state.lock().unwrap();
    if cached_process_is_live(&mut state.processes, pid)? {
        return Ok(());
    }
    // SAFETY: the notification belongs only to our inherited filter. Validate
    // it again after opening to reject cancellation / numeric PID reuse.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) } as i32;
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    if !valid(listener, &notification.id)? {
        return Err(io::Error::other(
            "notification canceled while opening pidfd",
        ));
    }
    state.processes.insert(pid, fd);
    Ok(())
}

fn cached_process_is_live(
    processes: &mut std::collections::BTreeMap<u32, OwnedFd>,
    pid: u32,
) -> io::Result<bool> {
    let Some(process) = processes.get(&pid) else {
        return Ok(false);
    };
    let mut event = libc::pollfd {
        fd: process.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: observe this held cached pidfd; no signalling or numeric-PID lookup.
    let result = unsafe { libc::poll(&mut event, 1, 0) };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    if result == 0 {
        return Ok(true);
    }
    if event.revents & libc::POLLIN == 0 {
        return Err(io::Error::other(
            "cached pidfd returned unexpected poll events",
        ));
    }
    processes.remove(&pid);
    Ok(false)
}

#[test]
fn catalog_fault_notification_cache_expires_an_exited_handle() {
    let mut child = OwnedProcess(
        Command::new("/usr/bin/sleep")
            .arg("60")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let pid = child.0.id();
    // SAFETY: the unreaped direct child retains its exact identity.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) } as i32;
    assert!(fd >= 0);
    let mut processes =
        std::collections::BTreeMap::from([(pid, unsafe { OwnedFd::from_raw_fd(fd) })]);
    assert!(cached_process_is_live(&mut processes, pid).unwrap());
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    assert!(!cached_process_is_live(&mut processes, pid).unwrap());
    assert!(!processes.contains_key(&pid));
    assert!(!cached_process_is_live(&mut processes, pid).unwrap());
    eprintln!(
        "cache control: real task-owned live handle retained; exited handle removed; no PID reuse claimed"
    );
}

fn identify(
    notification: &Notification,
    targets: &[PathBuf],
) -> io::Result<Option<(PathBuf, u64, u64)>> {
    let descriptor = PathBuf::from(format!(
        "/proc/{}/fd/{}",
        notification.pid, notification.data.args[0]
    ));
    let path = fs::read_link(&descriptor)?;
    if !targets.contains(&path) {
        if targets
            .iter()
            .any(|target| path.as_os_str() == format!("{} (deleted)", target.display()).as_str())
        {
            return Err(io::Error::other(
                "target descriptor refers to a deleted inode",
            ));
        }
        return Ok(None);
    }
    let first = fs::metadata(&descriptor)?;
    let named = fs::symlink_metadata(&path)?;
    if !first.is_file()
        || !named.is_file()
        || (first.dev(), first.ino()) != (named.dev(), named.ino())
    {
        return Err(io::Error::other(
            "named inode differs from notified descriptor",
        ));
    }
    let second = fs::metadata(&descriptor)?;
    let named_again = fs::symlink_metadata(&path)?;
    if fs::read_link(&descriptor)? != path
        || (first.dev(), first.ino()) != (second.dev(), second.ino())
        || (first.dev(), first.ino()) != (named_again.dev(), named_again.ino())
    {
        return Err(io::Error::other(
            "descriptor mapping changed during observation",
        ));
    }
    Ok(Some((path, first.dev(), first.ino())))
}

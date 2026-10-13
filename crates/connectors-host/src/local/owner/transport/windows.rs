//! The owner transport on Windows (contracts/cli/v1alpha1/owner.md,
//! "Windows"): a named pipe in place of the owner socket, two byte-range
//! locks in place of `flock`, and inherited handles in place of descriptors
//! 3 and 4 for a starting owner.
//!
//! The pipe's name is derived from the current user's SID and the state
//! directory's final path, as the socket's path is from the state directory.
//! Its first instance is created with `FILE_FLAG_FIRST_PIPE_INSTANCE` and every
//! instance with a list granting the current user's SID alone. A CLI requires
//! the pipe's server process to run as the current user before it sends
//! anything; a pipe of another user is no owner.
//!
//! The lock file carries two one-byte locks. Byte 0 is the lifetime lock: an
//! owner holds it until it exits. Byte 1 is the startup lock: a CLI that
//! starts an owner holds it from before the owner exists until the owner has
//! answered. A Windows lock belongs to the process that took it, so the CLI
//! releases byte 0 once the owner is created and the owner takes it on its
//! own open of the lock file; byte 1 keeps any other CLI from starting a
//! second owner meanwhile.
use super::super::super::filesystem::{self as fs, windows as win};
use super::*;
use runtime::{
    artifact::quote,
    channel::{self, Stream},
};
use std::{
    ffi::OsStr,
    fs::OpenOptions,
    os::windows::{
        ffi::OsStrExt,
        fs::OpenOptionsExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
};
use windows_sys::Win32::{
    Foundation::{
        DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_ACCESS_DENIED, ERROR_IO_PENDING,
        ERROR_LOCK_VIOLATION, ERROR_NO_DATA, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, HANDLE,
        WAIT_OBJECT_0,
    },
    Security::GetLengthSid,
    Storage::FileSystem::{
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
        LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx, UnlockFileEx,
    },
    System::{
        IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
        Pipes::{ConnectNamedPipe, PIPE_UNLIMITED_INSTANCES, WaitNamedPipeW},
        Threading::{
            CREATE_BREAKAWAY_FROM_JOB, CREATE_NEW_PROCESS_GROUP, CREATE_UNICODE_ENVIRONMENT,
            CreateEventW, CreateProcessW, DETACHED_PROCESS, DeleteProcThreadAttributeList,
            EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess, InitializeProcThreadAttributeList,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROCESS_INFORMATION, STARTF_USESTDHANDLES,
            STARTUPINFOEXW, TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
        },
    },
};

/// The variable that tells a starting owner its inherited startup channel and
/// lock file handles, as two decimal values. The CLI starts the owner with
/// nothing else in its environment but `SystemRoot`.
const STARTUP_HANDLES: &str = "CONNECTORS_OWNER_HANDLES";
/// The lifetime lock's byte, then the startup lock's.
const LIFETIME: u32 = 0;
const STARTUP: u32 = 1;

fn last_error() -> u32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0) as u32
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

/// The owner pipe of the state directory held as `directory`:
/// `\\.\pipe\connectors-owner-` and the SHA-256 of the current user's SID, a
/// zero byte and the directory's final path. An unreadable SID or path gives
/// an empty name, which no connect or listen admits.
pub(super) fn socket_path(directory: &File) -> PathBuf {
    let (Ok(user), Ok(path)) = (win::current_user(), win::final_path(directory)) else {
        return PathBuf::new();
    };
    // SAFETY: a valid SID, read for its own length.
    let sid = unsafe {
        std::slice::from_raw_parts(
            user.as_psid() as *const u8,
            GetLengthSid(user.as_psid()) as usize,
        )
    };
    let mut digest = Sha256::new();
    digest.update(sid);
    digest.update([0]);
    for unit in path.as_os_str().encode_wide() {
        digest.update(unit.to_le_bytes());
    }
    PathBuf::from(format!(
        r"\\.\pipe\connectors-owner-{}",
        hex::encode(digest.finalize())
    ))
}

/// Connects to the owner pipe at `path`. A missing pipe, one whose every
/// instance stays busy for half a second, one the current user may not open,
/// and one whose server process runs as another user are all no owner
/// (`Unavailable`). Returns the server's process id, the pipe's identity.
pub(super) fn connect(path: &Path) -> Result<(Stream, u32)> {
    if path.as_os_str().is_empty() {
        return Err(Code::Unavailable.into());
    }
    let name = wide(path.as_os_str());
    let busy_until = Instant::now() + Duration::from_millis(500);
    loop {
        match channel::windows::open_client(&name) {
            Ok(handle) => {
                let stream = Stream::from_owned(handle).map_err(|_| Code::Unavailable)?;
                let peer = channel::peer(&stream).map_err(|_| Code::Unavailable)?;
                return Ok((stream, peer.pid));
            }
            Err(error)
                if error.raw_os_error() == Some(ERROR_PIPE_BUSY as i32)
                    && Instant::now() < busy_until =>
            {
                // SAFETY: a NUL-terminated name; waits at most 50 ms.
                unsafe { WaitNamedPipeW(name.as_ptr(), 50) };
            }
            Err(_) => return Err(Code::Unavailable.into()),
        }
    }
}

/// The server process id of the owner pipe at `path` now, if one answers.
pub(super) fn identity(path: &Path) -> Option<u32> {
    if path.as_os_str().is_empty() {
        return None;
    }
    let handle = channel::windows::open_client(&wide(path.as_os_str())).ok()?;
    let stream = Stream::from_owned(handle).ok()?;
    channel::peer(&stream).ok().map(|peer| peer.pid)
}

/// One instance of the owner pipe waiting for a client.
struct Pending {
    handle: Option<OwnedHandle>,
    event: OwnedHandle,
    /// Boxed: the kernel writes it until the connect completes or is cancelled.
    overlapped: Box<OVERLAPPED>,
    waiting: bool,
}
impl Pending {
    fn start(name: &[u16], first: bool) -> Result<Self> {
        let handle = channel::windows::create_instance(name, first, PIPE_UNLIMITED_INSTANCES)
            .map_err(|_| Code::Unavailable)?;
        // SAFETY: no attributes and no name; a manual-reset event, not signalled.
        let event = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
        if event.is_null() {
            return Err(Code::Unavailable.into());
        }
        // SAFETY: CreateEventW returned a new owned handle.
        let event = unsafe { OwnedHandle::from_raw_handle(event as _) };
        let mut pending = Self {
            overlapped: Box::new(OVERLAPPED {
                hEvent: event.as_raw_handle() as HANDLE,
                ..Default::default()
            }),
            handle: Some(handle),
            event,
            waiting: false,
        };
        // SAFETY: a live server instance and a boxed OVERLAPPED that outlives
        // the connect: Drop cancels and waits for it.
        if unsafe { ConnectNamedPipe(pending.raw(), &mut *pending.overlapped) } == 0 {
            match last_error() {
                ERROR_IO_PENDING => pending.waiting = true,
                // Connected, or connected and already closed again: the
                // exchange reads end of file.
                ERROR_PIPE_CONNECTED | ERROR_NO_DATA => {}
                _ => return Err(Code::Unavailable.into()),
            }
        }
        Ok(pending)
    }
    fn raw(&self) -> HANDLE {
        self.handle.as_ref().map_or(std::ptr::null_mut(), |handle| {
            handle.as_raw_handle() as HANDLE
        })
    }
    /// Whether a client has connected.
    fn connected(&mut self) -> std::io::Result<bool> {
        if !self.waiting {
            return Ok(true);
        }
        // SAFETY: a live event handle; no wait.
        if unsafe { WaitForSingleObject(self.event.as_raw_handle() as HANDLE, 0) } != WAIT_OBJECT_0
        {
            return Ok(false);
        }
        let mut done = 0u32;
        // SAFETY: the connect has completed; reads its result.
        let ok = unsafe { GetOverlappedResult(self.raw(), &*self.overlapped, &mut done, 0) };
        self.waiting = false;
        if ok != 0 || last_error() == ERROR_NO_DATA {
            Ok(true)
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
    fn into_handle(mut self) -> OwnedHandle {
        self.waiting = false;
        self.handle.take().expect("an instance until taken")
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        if self.waiting {
            let mut done = 0u32;
            // SAFETY: cancels this connect alone, then waits for it so the
            // OVERLAPPED is no longer written when it is freed.
            unsafe {
                CancelIoEx(self.raw(), &*self.overlapped);
                GetOverlappedResult(self.raw(), &*self.overlapped, &mut done, 1);
            }
        }
    }
}

/// The owner pipe's listening side: one instance always waits for the next
/// client, so a connect finds an instance unless the pipe was closed.
pub(super) struct Listener {
    name: Vec<u16>,
    pending: Option<Pending>,
    /// A client that connected just before the pipe was closed, still served.
    ready: Option<OwnedHandle>,
    closed: bool,
}
impl Listener {
    /// Creates the pipe's first instance. A pipe of that name that already
    /// exists, whoever created it, refuses: the owner does not serve.
    pub(super) fn bind(path: &Path) -> Result<Self> {
        if path.as_os_str().is_empty() {
            return Err(Code::Unavailable.into());
        }
        let name = wide(path.as_os_str());
        let pending = Pending::start(&name, true)?;
        Ok(Self {
            name,
            pending: Some(pending),
            ready: None,
            closed: false,
        })
    }
    /// Listens again after [`Listener::close`]. With `served`, a connection of
    /// this owner may still hold an instance, so the pipe may still exist: its
    /// first instance is created if it is gone, and a new instance joins it
    /// otherwise. Without, no instance of this owner is left and only a first
    /// instance is created. A pipe someone else created meanwhile under the
    /// name is never the owner to a client: every client requires the server
    /// process to run as the current user ([`connect`]).
    pub(super) fn rebind(path: &Path, served: bool) -> Result<Self> {
        match Self::bind(path) {
            Err(_) if served => {
                let name = wide(path.as_os_str());
                let pending = Pending::start(&name, false)?;
                Ok(Self {
                    name,
                    pending: Some(pending),
                    ready: None,
                    closed: false,
                })
            }
            result => result,
        }
    }
    /// A connected client, or `WouldBlock` when none is waiting. The next
    /// instance is created before the connected one is handed over.
    pub(super) fn accept(&mut self) -> std::io::Result<(Stream, ())> {
        let stream =
            |handle| Stream::from_owned(handle).map_err(|_| std::io::Error::other("not a pipe"));
        if let Some(handle) = self.ready.take() {
            return Ok((stream(handle)?, ()));
        }
        if self.closed {
            return Err(std::io::ErrorKind::WouldBlock.into());
        }
        if self.pending.is_none() {
            self.pending = Some(
                Pending::start(&self.name, false).map_err(|_| std::io::Error::other("instance"))?,
            );
        }
        let pending = self.pending.as_mut().expect("an instance was started");
        if !pending.connected()? {
            return Err(std::io::ErrorKind::WouldBlock.into());
        }
        let connected = self.pending.take().expect("an instance was started");
        // A failure leaves no instance; the next accept starts one.
        self.pending = Pending::start(&self.name, false).ok();
        Ok((stream(connected.into_handle())?, ()))
    }
    /// Removes the pipe: the waiting instance is closed, so no later connect
    /// reaches this owner once the instances being served close too. A client
    /// that connected to it first is still returned by [`Listener::accept`].
    pub(super) fn close(&mut self) -> std::io::Result<()> {
        if let Some(mut pending) = self.pending.take()
            && pending.connected().unwrap_or(false)
        {
            self.ready = Some(pending.into_handle());
        }
        self.closed = true;
        Ok(())
    }
}

/// Locks one byte of `file` now, or fails (`WouldBlock` while another
/// process holds it).
fn lock_byte(file: &File, byte: u32) -> std::io::Result<()> {
    let mut overlapped = OVERLAPPED::default();
    overlapped.Anonymous.Anonymous.Offset = byte;
    // SAFETY: a live handle opened for reading and a synchronous OVERLAPPED
    // naming the byte's offset.
    if unsafe {
        LockFileEx(
            file.as_raw_handle() as HANDLE,
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            1,
            0,
            &mut overlapped,
        )
    } != 0
    {
        return Ok(());
    }
    Err(match last_error() {
        ERROR_LOCK_VIOLATION => std::io::ErrorKind::WouldBlock.into(),
        _ => std::io::Error::last_os_error(),
    })
}

fn unlock_byte(file: &File, byte: u32) {
    let mut overlapped = OVERLAPPED::default();
    overlapped.Anonymous.Anonymous.Offset = byte;
    // SAFETY: a live handle; unlocking a byte this process locked.
    unsafe { UnlockFileEx(file.as_raw_handle() as HANDLE, 0, 1, 0, &mut overlapped) };
}

/// The startup lock and then the lifetime lock, both now, or neither: the
/// counterpart of `flock(LOCK_EX | LOCK_NB)` on the lock file. Held, no owner
/// runs and no other CLI is starting one.
pub(super) fn try_lock(file: &File) -> std::io::Result<()> {
    lock_byte(file, STARTUP)?;
    if let Err(error) = lock_byte(file, LIFETIME) {
        unlock_byte(file, STARTUP);
        return Err(error);
    }
    Ok(())
}

/// The owner's lifetime lock on its own open of the lock file, retried until
/// `deadline` while the starting CLI or a CLI probing for an owner holds it.
pub(super) fn hold_lifetime(file: &File, deadline: Instant) -> Result<()> {
    loop {
        match lock_byte(file, LIFETIME) {
            Ok(()) => return Ok(()),
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return Err(Code::Unavailable.into()),
        }
    }
}

/// A started owner, held until it has answered.
pub(super) struct Process(OwnedHandle);
impl Process {
    pub(super) fn kill(&mut self) -> std::io::Result<()> {
        // SAFETY: the process handle this CLI created the owner with.
        if unsafe { TerminateProcess(self.0.as_raw_handle() as HANDLE, 1) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    /// Waits for the owner's exit, at most five seconds.
    pub(super) fn wait(&mut self) -> std::io::Result<()> {
        // SAFETY: a live process handle.
        match unsafe { WaitForSingleObject(self.0.as_raw_handle() as HANDLE, 5_000) } {
            WAIT_OBJECT_0 => Ok(()),
            _ => Err(std::io::ErrorKind::TimedOut.into()),
        }
    }
}

/// An inheritable duplicate of `handle`, for one child's creation.
fn inheritable(handle: HANDLE) -> Result<OwnedHandle> {
    let mut duplicate: HANDLE = std::ptr::null_mut();
    // SAFETY: duplicates a live handle within this process; a returned handle
    // is new and owned.
    if unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            handle,
            GetCurrentProcess(),
            &mut duplicate,
            0,
            1,
            DUPLICATE_SAME_ACCESS,
        )
    } == 0
    {
        return Err(Code::Unavailable.into());
    }
    // SAFETY: DuplicateHandle returned a new owned handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(duplicate as _) })
}

/// Creates `application` with `line`, `environment` and exactly `handles`
/// inherited (`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`), standard handles null.
fn create(
    application: &Path,
    mut line: Vec<u16>,
    environment: &[u16],
    handles: &[HANDLE],
    flags: u32,
) -> std::io::Result<OwnedHandle> {
    let mut size = 0usize;
    // SAFETY: a size query with a null list.
    unsafe { InitializeProcThreadAttributeList(std::ptr::null_mut(), 1, 0, &mut size) };
    let mut storage = vec![0u64; size.div_ceil(8)];
    let list = storage.as_mut_ptr().cast();
    // SAFETY: the storage holds `size` bytes, aligned.
    if unsafe { InitializeProcThreadAttributeList(list, 1, 0, &mut size) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    struct List(*mut std::ffi::c_void);
    impl Drop for List {
        fn drop(&mut self) {
            // SAFETY: an initialized list, deleted once.
            unsafe { DeleteProcThreadAttributeList(self.0) };
        }
    }
    let list = List(list);
    // SAFETY: the handle array outlives the list's use in CreateProcessW.
    if unsafe {
        UpdateProcThreadAttribute(
            list.0,
            0,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
            handles.as_ptr().cast(),
            std::mem::size_of_val(handles),
            std::ptr::null_mut(),
            std::ptr::null(),
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: zero is a valid STARTUPINFOEXW.
    let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.lpAttributeList = list.0;
    // SAFETY: zero is a valid PROCESS_INFORMATION.
    let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let application = wide(application.as_os_str());
    line.push(0);
    // SAFETY: NUL-terminated application and writable command line, a
    // double-NUL-terminated environment block, and live startup information.
    if unsafe {
        CreateProcessW(
            application.as_ptr(),
            line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
            flags,
            environment.as_ptr().cast(),
            std::ptr::null(),
            &startup.StartupInfo,
            &mut information,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: CreateProcessW returned new owned handles; the thread's is not kept.
    unsafe {
        drop(OwnedHandle::from_raw_handle(information.hThread as _));
        Ok(OwnedHandle::from_raw_handle(information.hProcess as _))
    }
}

/// Starts the owner (owner.md, "Detaching from the CLI"): this executable by
/// its full path, detached from the console and broken away from the CLI's
/// job, so it outlives the CLI. A job that permits no breakaway keeps the
/// owner, which then lives as long as that job. The startup channel's client
/// end and a separate open of the lock file are the only handles it inherits;
/// their values are in [`STARTUP_HANDLES`]. Once the owner exists, this CLI
/// releases the lifetime byte for it and keeps the startup byte.
///
/// The owner measures its build from its executable's path
/// (`own_image_build_measure`: `launch_path_read`); a file put at that path
/// meanwhile is another build, which the caller refuses.
pub(super) fn spawn(
    paths: &Paths,
    lock: &File,
    _deadline: Instant,
) -> Result<(Stream, Process, ())> {
    let binary = std::env::current_exe().map_err(|_| Code::Unavailable)?;
    let (parent, child) = channel::windows::pair()?;
    let lifetime = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(win::final_path(lock).map_err(|_| Code::Unavailable)?)
        .map_err(|_| Code::Unavailable)?;
    if win::identity(&lifetime)? != win::identity(lock)? {
        return Err(Code::InvalidConfiguration.into());
    }
    let channel = inheritable(child.as_raw_handle() as HANDLE)?;
    let lifetime = inheritable(lifetime.as_raw_handle() as HANDLE)?;
    let handles = [
        channel.as_raw_handle() as HANDLE,
        lifetime.as_raw_handle() as HANDLE,
    ];
    let mut environment: Vec<u16> = format!(
        "{STARTUP_HANDLES}={} {}",
        handles[0] as usize, handles[1] as usize
    )
    .encode_utf16()
    .chain(Some(0))
    .collect();
    if let Some(root) = std::env::var_os("SystemRoot") {
        environment.extend("SystemRoot=".encode_utf16());
        environment.extend(root.encode_wide());
        environment.push(0);
    }
    environment.push(0);
    let mut line = Vec::new();
    for argument in [
        binary.as_os_str(),
        OsStr::new("__connectors-owner"),
        paths.config.as_os_str(),
        paths.state.as_os_str(),
    ] {
        quote(argument, &mut line);
    }
    let flags = DETACHED_PROCESS
        | CREATE_NEW_PROCESS_GROUP
        | CREATE_UNICODE_ENVIRONMENT
        | EXTENDED_STARTUPINFO_PRESENT;
    let process = match create(
        &binary,
        line.clone(),
        &environment,
        &handles,
        flags | CREATE_BREAKAWAY_FROM_JOB,
    ) {
        Err(error) if error.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) => {
            create(&binary, line, &environment, &handles, flags)
        }
        result => result,
    }
    .map_err(|_| Code::Unavailable)?;
    unlock_byte(lock, LIFETIME);
    Ok((parent, Process(process), ()))
}

/// The startup channel and lock file a starting owner inherited, named by
/// [`STARTUP_HANDLES`]. Each is taken over as a handle that is not
/// inheritable and the inherited one closed. A missing or malformed value,
/// or a handle of the wrong kind, is `InvalidConfiguration`: an owner started
/// by hand never mistakes another handle for inherited authority.
pub(super) fn inherited() -> Result<(Stream, File)> {
    let value = std::env::var(STARTUP_HANDLES).map_err(|_| Code::InvalidConfiguration)?;
    let mut values = value.split(' ').map(str::parse::<usize>);
    let (Some(Ok(channel)), Some(Ok(lock)), None) = (values.next(), values.next(), values.next())
    else {
        return Err(Code::InvalidConfiguration.into());
    };
    let channel = channel::windows::adopt(channel).map_err(|_| Code::InvalidConfiguration)?;
    let lock = channel::windows::adopt(lock).map_err(|_| Code::InvalidConfiguration)?;
    let stream = Stream::from_owned(channel).map_err(|_| Code::InvalidConfiguration)?;
    channel::peer(&stream)?;
    let lifetime = File::from(lock);
    fs::check_private_file(&lifetime)?;
    Ok((stream, lifetime))
}

#[cfg(test)]
mod tests;

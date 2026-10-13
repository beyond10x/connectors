//! The channel on Windows (contracts/cli/v1alpha1/owner.md and
//! private-adapter.md, "Windows"). There is no Unix socket: both ends are a
//! byte-mode named pipe opened for overlapped I/O, whose access control list
//! grants the current user's SID alone and which refuses remote clients.
//!
//! Every read and write is one overlapped operation waited on with its own
//! deadline and cancelled when that passes, so no wait is unbounded and no
//! handle is ever switched between blocking modes. Bytes a peer wrote before
//! it closed stay in the pipe and are read whole; end of file follows.
//!
//! The peer (`peer_credential_snapshot`, weaker form `client_token_query`) is
//! the process at the other end, by the id the pipe recorded at connect
//! (`GetNamedPipeClientProcessId` on a server end,
//! `GetNamedPipeServerProcessId` on a client end). That process is opened once;
//! its token's user SID must be the current user's, its integrity level this
//! thread's and the token not restricted, and the process handle opened then
//! is the one every later handle duplication goes through.
use super::super::{Failure, Result};
use crate::local::filesystem::windows::current_user;
use std::{
    ffi::c_void,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::{Arc, OnceLock},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{
        DUPLICATE_CLOSE_SOURCE, DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_ACCESS_DENIED,
        ERROR_BROKEN_PIPE, ERROR_IO_PENDING, ERROR_NO_DATA, ERROR_OPERATION_ABORTED,
        ERROR_PIPE_CONNECTED, ERROR_PIPE_NOT_CONNECTED, GetHandleInformation, HANDLE,
        HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE, WAIT_OBJECT_0,
    },
    Security::{
        ACCESS_ALLOWED_ACE, ACL, ACL_REVISION, AddAccessAllowedAceEx, EqualSid, GetLengthSid,
        GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, InitializeAcl,
        InitializeSecurityDescriptor, IsTokenRestricted, PSID, SE_DACL_PROTECTED,
        SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR, SetSecurityDescriptorControl,
        SetSecurityDescriptorDacl, SetSecurityDescriptorOwner, TOKEN_MANDATORY_LABEL, TOKEN_QUERY,
        TOKEN_USER, TokenIntegrityLevel, TokenUser,
    },
    Storage::FileSystem::{
        CreateFileW, FILE_ALL_ACCESS, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED,
        FILE_TYPE_PIPE, GetFileType, OPEN_EXISTING, PIPE_ACCESS_DUPLEX, ReadFile,
        SECURITY_ANONYMOUS, SECURITY_SQOS_PRESENT, WriteFile,
    },
    System::{
        IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED},
        Pipes::{
            ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId, GetNamedPipeInfo,
            GetNamedPipeServerProcessId, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_SERVER_END, PIPE_TYPE_BYTE, PIPE_WAIT, PeekNamedPipe,
        },
        Threading::{
            CreateEventW, GetCurrentProcess, GetCurrentProcessId, INFINITE, OpenProcess,
            OpenProcessToken, PROCESS_DUP_HANDLE, PROCESS_QUERY_LIMITED_INFORMATION,
            WaitForSingleObject,
        },
    },
};

/// Each direction's pipe buffer.
const BUFFER: u32 = 64 * 1024;
const SECURITY_DESCRIPTOR_REVISION: u32 = 1;

/// One end of a named pipe, opened for overlapped I/O.
pub(crate) struct Pipe {
    handle: OwnedHandle,
    /// Manual-reset event each overlapped operation signals on completion.
    event: OwnedHandle,
    peer: OnceLock<Peer>,
}

/// The process at the other end of a pipe, opened once (`client_token_query`).
#[derive(Clone)]
pub(crate) struct Peer {
    pub pid: u32,
    process: Arc<OwnedHandle>,
}
impl std::fmt::Debug for Peer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Peer")
            .field("pid", &self.pid)
            .finish()
    }
}
impl Peer {
    /// The peer's process handle, opened with `PROCESS_DUP_HANDLE`.
    pub(crate) fn process(&self) -> HANDLE {
        self.process.as_raw_handle() as HANDLE
    }
}

fn last_error() -> u32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0) as u32
}

/// Milliseconds for a wait, rounded up, below `INFINITE`.
fn millis(wait: Duration) -> u32 {
    let rounded = wait.as_nanos().div_ceil(1_000_000);
    rounded.min(u128::from(INFINITE - 1)) as u32
}

impl Pipe {
    /// Takes ownership of an open pipe handle. Anything but a pipe refuses.
    pub(crate) fn from_owned(handle: OwnedHandle) -> Result<Self> {
        // SAFETY: a live handle owned by the caller.
        if unsafe { GetFileType(handle.as_raw_handle() as HANDLE) } != FILE_TYPE_PIPE {
            return Err(Failure::Unavailable);
        }
        // SAFETY: no attributes and no name; a manual-reset event, not signalled.
        let event = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
        if event.is_null() {
            return Err(Failure::Unavailable);
        }
        Ok(Self {
            handle,
            // SAFETY: CreateEventW returned a new owned handle.
            event: unsafe { OwnedHandle::from_raw_handle(event as _) },
            peer: OnceLock::new(),
        })
    }

    pub(crate) fn raw(&self) -> HANDLE {
        self.handle.as_raw_handle() as HANDLE
    }

    /// Whether this is the server end, the one `CreateNamedPipeW` returned.
    fn server_end(&self) -> Result<bool> {
        let mut flags = 0u32;
        // SAFETY: a live pipe handle and one writable flags word.
        if unsafe {
            GetNamedPipeInfo(
                self.raw(),
                &mut flags,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(Failure::Unavailable);
        }
        Ok(flags & PIPE_SERVER_END != 0)
    }

    /// Reads at most `buffer.len()` bytes within `wait`. End of file, the
    /// peer's close after everything it wrote has been read, is `Ok(0)`. A
    /// wait that passes cancels the read and is `TimedOut`.
    pub(crate) fn read_within(
        &mut self,
        buffer: &mut [u8],
        wait: Duration,
    ) -> std::io::Result<usize> {
        self.transfer(true, buffer.as_mut_ptr(), buffer.len(), wait)
    }

    /// Writes at most `bytes.len()` bytes within `wait`. A closed peer is
    /// `BrokenPipe` (`ERROR_NO_DATA` or `ERROR_BROKEN_PIPE`, the counterpart of
    /// `EPIPE` under `MSG_NOSIGNAL`).
    pub(crate) fn write_within(&mut self, bytes: &[u8], wait: Duration) -> std::io::Result<usize> {
        self.transfer(false, bytes.as_ptr().cast_mut(), bytes.len(), wait)
    }

    fn transfer(
        &self,
        read: bool,
        buffer: *mut u8,
        length: usize,
        wait: Duration,
    ) -> std::io::Result<usize> {
        let length = length.min(1 << 20) as u32;
        let mut overlapped = OVERLAPPED {
            hEvent: self.event.as_raw_handle() as HANDLE,
            ..Default::default()
        };
        // SAFETY: the buffer is live for `length` bytes, and the OVERLAPPED
        // and its event outlive the operation: it completes, or is cancelled
        // and waited for, before this function returns.
        let started = unsafe {
            if read {
                ReadFile(
                    self.raw(),
                    buffer,
                    length,
                    std::ptr::null_mut(),
                    &mut overlapped,
                )
            } else {
                WriteFile(
                    self.raw(),
                    buffer,
                    length,
                    std::ptr::null_mut(),
                    &mut overlapped,
                )
            }
        };
        if started == 0 {
            let error = last_error();
            if error != ERROR_IO_PENDING {
                return failed(read, error);
            }
        }
        // SAFETY: a live event handle.
        if unsafe { WaitForSingleObject(overlapped.hEvent, millis(wait)) } != WAIT_OBJECT_0 {
            // SAFETY: cancels only this operation on this handle.
            unsafe { CancelIoEx(self.raw(), &overlapped) };
        }
        let mut done = 0u32;
        // SAFETY: waits for this operation, completed or cancelled above, so
        // the wait is bounded; the OVERLAPPED is still live.
        if unsafe { GetOverlappedResult(self.raw(), &overlapped, &mut done, 1) } != 0 {
            return Ok(done as usize);
        }
        match last_error() {
            ERROR_OPERATION_ABORTED => Err(std::io::ErrorKind::TimedOut.into()),
            error => failed(read, error),
        }
    }
}

fn failed(read: bool, error: u32) -> std::io::Result<usize> {
    match error {
        ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED if read => Ok(0),
        ERROR_BROKEN_PIPE | ERROR_PIPE_NOT_CONNECTED | ERROR_NO_DATA => {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        error => Err(std::io::Error::from_raw_os_error(error as i32)),
    }
}

/// The refusal [`peer`] and [`admit`] give a peer of the current user whose
/// token has another integrity level than this thread's effective token, or
/// is restricted. Every caller that answers or reports a peer maps it to
/// `lifecycle_conflict` (owner.md, "Windows", "Pipe peer"); a peer of another
/// user is `Unavailable`, as no owner.
pub(crate) const SESSION_MISMATCH: Failure = Failure::Forbidden;

/// `GetCurrentThreadEffectiveToken()`: the pseudo handle of the token this
/// thread acts with, its impersonation token while it impersonates and the
/// process token otherwise. It needs no closing.
const EFFECTIVE_TOKEN: HANDLE = -6isize as HANDLE;

/// The peer of `pipe`: the process at its other end, which must run as the
/// current user, at this thread's integrity level, with a token that is not
/// restricted. Read once per pipe and then kept, so the checks and every
/// handle duplication into the peer go through the one process handle.
///
/// A process at a higher integrity level refuses `PROCESS_DUP_HANDLE` to this
/// one. Only then is it opened a second time, for its token alone, so that a
/// peer of the current user at another level is refused by name
/// ([`SESSION_MISMATCH`]) rather than as no peer.
pub(crate) fn peer(pipe: &Pipe) -> Result<Peer> {
    if let Some(peer) = pipe.peer.get() {
        return Ok(peer.clone());
    }
    let mut pid = 0u32;
    // SAFETY: a live pipe handle and one writable id.
    let found = unsafe {
        if pipe.server_end()? {
            GetNamedPipeClientProcessId(pipe.raw(), &mut pid)
        } else {
            GetNamedPipeServerProcessId(pipe.raw(), &mut pid)
        }
    };
    if found == 0 || pid == 0 {
        return Err(Failure::Unavailable);
    }
    let user = current_user().map_err(|_| Failure::Unavailable)?.as_psid();
    let Some(process) = open_process(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_DUP_HANDLE)
    else {
        if last_error() == ERROR_ACCESS_DENIED
            && let Some(query) = open_process(pid, PROCESS_QUERY_LIMITED_INFORMATION)
        {
            admit(&query, user)?;
        }
        return Err(Failure::Unavailable);
    };
    admit(&process, user)?;
    let peer = Peer {
        pid,
        process: Arc::new(process),
    };
    Ok(pipe.peer.get_or_init(|| peer).clone())
}

fn open_process(pid: u32, access: u32) -> Option<OwnedHandle> {
    // SAFETY: documented scalar arguments; a returned handle is new and owned.
    let process = unsafe { OpenProcess(access, 0, pid) };
    // SAFETY: OpenProcess returned a new owned handle.
    (!process.is_null()).then(|| unsafe { OwnedHandle::from_raw_handle(process as _) })
}

/// Requires the user SID of `process`'s token to be `expected`
/// (`Unavailable` otherwise), and then its integrity level to equal this
/// thread's effective token's and the token not to be restricted
/// ([`SESSION_MISMATCH`] otherwise).
pub(crate) fn admit(process: &OwnedHandle, expected: PSID) -> Result<()> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: a live process handle and a writable out pointer.
    if unsafe { OpenProcessToken(process.as_raw_handle() as HANDLE, TOKEN_QUERY, &mut token) } == 0
    {
        return Err(Failure::Unavailable);
    }
    // SAFETY: OpenProcessToken returned a new owned handle.
    let token = unsafe { OwnedHandle::from_raw_handle(token as _) };
    let token = token.as_raw_handle() as HANDLE;
    let mut buffer = vec![0u64; 64];
    let mut length = 0u32;
    // SAFETY: the buffer is 512 bytes, passed as its size, and aligned for TOKEN_USER.
    if unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            (buffer.len() * 8) as u32,
            &mut length,
        )
    } == 0
    {
        return Err(Failure::Unavailable);
    }
    // SAFETY: a successful TokenUser query wrote a TOKEN_USER at the start.
    let user = unsafe { &*(buffer.as_ptr() as *const TOKEN_USER) };
    // SAFETY: both are valid SIDs for the duration of the call.
    if unsafe { EqualSid(expected, user.User.Sid) } == 0 {
        return Err(Failure::Unavailable);
    }
    // SAFETY: a live token opened with TOKEN_QUERY.
    let restricted = unsafe { IsTokenRestricted(token) } != 0;
    if restricted || integrity_level(token)? != integrity_level(EFFECTIVE_TOKEN)? {
        return Err(SESSION_MISMATCH);
    }
    Ok(())
}

/// The relative identifier of a token's mandatory integrity label, such as
/// 0x2000 for medium and 0x3000 for high.
fn integrity_level(token: HANDLE) -> Result<u32> {
    let mut buffer = vec![0u64; 16];
    let mut length = 0u32;
    // SAFETY: the buffer is 128 bytes, passed as its size, and aligned for
    // TOKEN_MANDATORY_LABEL.
    if unsafe {
        GetTokenInformation(
            token,
            TokenIntegrityLevel,
            buffer.as_mut_ptr().cast(),
            (buffer.len() * 8) as u32,
            &mut length,
        )
    } == 0
    {
        return Err(Failure::Unavailable);
    }
    // SAFETY: a successful query wrote a TOKEN_MANDATORY_LABEL whose SID
    // follows it in the buffer; the SID's sub-authorities are read within it.
    unsafe {
        let label = &*(buffer.as_ptr() as *const TOKEN_MANDATORY_LABEL);
        let count = *GetSidSubAuthorityCount(label.Label.Sid);
        if count == 0 {
            return Err(Failure::Unavailable);
        }
        Ok(*GetSidSubAuthority(label.Label.Sid, u32::from(count) - 1))
    }
}

/// Waits until `pipe` has bytes to read or its peer has closed, without a
/// deadline: an idle adapter waits here for its next request. The pipe is
/// polled, so a read that follows returns at once.
pub(crate) fn wait_readable(pipe: &Pipe) -> Result<()> {
    let mut pause = Duration::from_millis(1);
    loop {
        let mut available = 0u32;
        // SAFETY: a live pipe handle; no buffer is read, one count is written.
        if unsafe {
            PeekNamedPipe(
                pipe.raw(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut available,
                std::ptr::null_mut(),
            )
        } == 0
            || available > 0
        {
            // A closed or broken pipe is reported by the read that follows.
            return Ok(());
        }
        std::thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(50));
    }
}

/// An owner-only security descriptor for a pipe: the current user as owner
/// and a protected list granting that user's SID alone, for one call.
pub(crate) fn with_current_user_only<T>(
    create: impl FnOnce(*const SECURITY_ATTRIBUTES) -> T,
) -> Result<T> {
    let user = current_user().map_err(|_| Failure::Unavailable)?;
    // SAFETY: the SID is valid.
    let sid_length = unsafe { GetLengthSid(user.as_psid()) } as usize;
    let acl_length = (std::mem::size_of::<ACL>() + std::mem::size_of::<ACCESS_ALLOWED_ACE>()
        - std::mem::size_of::<u32>()
        + sid_length)
        .next_multiple_of(4);
    let mut acl = vec![0u32; acl_length / 4];
    let acl_pointer = acl.as_mut_ptr() as *mut ACL;
    // SAFETY: zero is a valid SECURITY_DESCRIPTOR to initialize.
    let mut descriptor: SECURITY_DESCRIPTOR = unsafe { std::mem::zeroed() };
    let descriptor_pointer = (&mut descriptor as *mut SECURITY_DESCRIPTOR).cast::<c_void>();
    // SAFETY: the ACL buffer is acl_length bytes; the descriptor and SID are
    // live for every call, and the descriptor only points at them.
    let ok = unsafe {
        InitializeAcl(acl_pointer, acl_length as u32, ACL_REVISION) != 0
            && AddAccessAllowedAceEx(
                acl_pointer,
                ACL_REVISION,
                0,
                FILE_ALL_ACCESS,
                user.as_psid(),
            ) != 0
            && InitializeSecurityDescriptor(descriptor_pointer, SECURITY_DESCRIPTOR_REVISION) != 0
            && SetSecurityDescriptorOwner(descriptor_pointer, user.as_psid(), 0) != 0
            && SetSecurityDescriptorDacl(descriptor_pointer, 1, acl_pointer, 0) != 0
            && SetSecurityDescriptorControl(
                descriptor_pointer,
                SE_DACL_PROTECTED,
                SE_DACL_PROTECTED,
            ) != 0
    };
    if !ok {
        return Err(Failure::Unavailable);
    }
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor_pointer,
        bInheritHandle: 0,
    };
    let result = create(&attributes);
    drop(acl);
    Ok(result)
}

/// A NUL-terminated wide pipe name.
pub(crate) fn wide(name: &str) -> Vec<u16> {
    name.encode_utf16().chain(Some(0)).collect()
}

/// Creates one server instance of the pipe `name` (NUL-terminated), owner-only,
/// byte mode, overlapped, refusing remote clients. With `first`, the call
/// fails unless this creates the pipe (`FILE_FLAG_FIRST_PIPE_INSTANCE`), so a
/// pipe of that name that someone else created first is never joined.
pub(crate) fn create_instance(name: &[u16], first: bool, instances: u32) -> Result<OwnedHandle> {
    let mode = PIPE_ACCESS_DUPLEX
        | FILE_FLAG_OVERLAPPED
        | if first {
            FILE_FLAG_FIRST_PIPE_INSTANCE
        } else {
            0
        };
    // SAFETY: a NUL-terminated name and live security attributes.
    let handle = with_current_user_only(|attributes| unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            mode,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            instances,
            BUFFER,
            BUFFER,
            0,
            attributes,
        )
    })?;
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        return Err(Failure::Unavailable);
    }
    // SAFETY: CreateNamedPipeW returned a new owned handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle as _) })
}

/// Opens the client end of the pipe `name` (NUL-terminated) for overlapped
/// I/O, not inheritable. The server may identify this process but never act
/// as it (`SECURITY_ANONYMOUS`), whoever created the pipe.
pub(crate) fn open_client(name: &[u16]) -> std::io::Result<OwnedHandle> {
    // SAFETY: a NUL-terminated name; a returned handle is new and owned.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            windows_sys::Win32::Foundation::GENERIC_READ
                | windows_sys::Win32::Foundation::GENERIC_WRITE,
            0,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_ANONYMOUS,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: CreateFileW returned a new owned handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle as _) })
}

/// A private channel between this process and a child it is about to create
/// (private-adapter.md, "Channel"; owner.md, the startup channel): a pipe with
/// one instance under a fresh unguessable name, whose client end this process
/// opens itself. Returns the server end and the client end, which is not
/// inheritable; the creator makes it inheritable for exactly the child's
/// creation. A client that connected first fails the call: this process must
/// be the recorded client.
pub(crate) fn pair() -> Result<(Pipe, OwnedHandle)> {
    let name = wide(&format!(r"\\.\pipe\connectors-{}", uuid::Uuid::new_v4()));
    let server = create_instance(&name, true, 1)?;
    let client = open_client(&name).map_err(|_| Failure::Unavailable)?;
    let server = Pipe::from_owned(server)?;
    connected(&server)?;
    let mut pid = 0u32;
    // SAFETY: a live server end and one writable id.
    if unsafe { GetNamedPipeClientProcessId(server.raw(), &mut pid) } == 0
        // SAFETY: no preconditions.
        || pid != unsafe { GetCurrentProcessId() }
    {
        return Err(Failure::Unavailable);
    }
    Ok((server, client))
}

/// Completes the connection of a server end whose client has already opened
/// it. A connection still pending (no client) is cancelled and refused.
fn connected(server: &Pipe) -> Result<()> {
    let mut overlapped = OVERLAPPED {
        hEvent: server.event.as_raw_handle() as HANDLE,
        ..Default::default()
    };
    // SAFETY: a live server end; the OVERLAPPED is waited for before return.
    if unsafe { ConnectNamedPipe(server.raw(), &mut overlapped) } != 0 {
        return Ok(());
    }
    match last_error() {
        ERROR_PIPE_CONNECTED => Ok(()),
        ERROR_IO_PENDING => {
            let mut done = 0u32;
            // SAFETY: cancels and then waits for this connect alone.
            unsafe {
                CancelIoEx(server.raw(), &overlapped);
                GetOverlappedResult(server.raw(), &overlapped, &mut done, 1);
            }
            Err(Failure::Unavailable)
        }
        _ => Err(Failure::Unavailable),
    }
}

/// Takes over a pipe handle this process inherited, whose value it was told
/// (`--connectors-private-fd` for an adapter, the startup handles for an
/// owner). Anything but a pipe refuses. The inherited handle is replaced by a
/// duplicate that is not inheritable and closed in the same call
/// (`DUPLICATE_CLOSE_SOURCE`), the counterpart of `F_DUPFD_CLOEXEC` and
/// `close` of the inherited descriptor.
pub(crate) fn adopt(value: usize) -> Result<OwnedHandle> {
    let handle = value as HANDLE;
    if value == 0 || handle == INVALID_HANDLE_VALUE {
        return Err(Failure::InvalidInput);
    }
    let mut flags = 0u32;
    // SAFETY: GetHandleInformation only reads the handle table entry.
    if unsafe { GetHandleInformation(handle, &mut flags) } == 0 {
        return Err(Failure::Unavailable);
    }
    let mut owned: HANDLE = std::ptr::null_mut();
    // SAFETY: duplicates within this process; the source is closed whatever
    // the result, and a returned handle is new and owned.
    if unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            handle,
            GetCurrentProcess(),
            &mut owned,
            0,
            0,
            DUPLICATE_SAME_ACCESS | DUPLICATE_CLOSE_SOURCE,
        )
    } == 0
    {
        return Err(Failure::Unavailable);
    }
    // SAFETY: DuplicateHandle returned a new owned handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(owned as _) })
}

/// Whether `handle` would be inherited by a child created with inheritance.
pub(crate) fn inheritable(handle: HANDLE) -> Result<bool> {
    let mut flags = 0u32;
    // SAFETY: GetHandleInformation only reads the handle table entry.
    if unsafe { GetHandleInformation(handle, &mut flags) } == 0 {
        return Err(Failure::Unavailable);
    }
    Ok(flags & HANDLE_FLAG_INHERIT != 0)
}

/// `PROCESS_BASIC_INFORMATION` as `NtQueryInformationProcess` writes it. The
/// binding's own struct needs the `Win32_System_Kernel` feature for its PEB
/// pointer type alone; the layout here is the same, the PEB an opaque pointer.
#[repr(C)]
struct BasicInformation {
    exit_status: i32,
    environment_block: *mut c_void,
    affinity: usize,
    priority: i32,
    pid: usize,
    parent: usize,
}

/// The id of the process that created this one. Windows keeps no parent link
/// beyond this recorded id; the adapter compares it with the pipe's server.
pub(crate) fn parent_process_id() -> Option<u32> {
    use windows_sys::Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation};
    // SAFETY: zero is a valid BasicInformation.
    let mut information: BasicInformation = unsafe { std::mem::zeroed() };
    let mut returned = 0u32;
    // SAFETY: the pseudo handle of this process and a writable buffer of the
    // size passed, laid out as ProcessBasicInformation is written.
    let status = unsafe {
        NtQueryInformationProcess(
            GetCurrentProcess(),
            ProcessBasicInformation,
            (&mut information as *mut BasicInformation).cast(),
            std::mem::size_of::<BasicInformation>() as u32,
            &mut returned,
        )
    };
    (status >= 0 && information.parent != 0).then_some(information.parent as u32)
}

#[cfg(test)]
mod tests;

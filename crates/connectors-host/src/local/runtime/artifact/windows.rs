//! Windows capture and process control (contracts/cli/v1alpha1/semantics.md,
//! "Windows platform guarantees").
//!
//! - [`capture`]: `sealed_executable` holds only as `deny_write_copy`. The copy
//!   is made in a fresh owner-only directory inside the state directory's
//!   owner-only `run` directory ([`enter`]), never the system's temporary
//!   directory, and opened for reading without `FILE_SHARE_WRITE` or
//!   `FILE_SHARE_DELETE` and checked through that handle, which is held, with
//!   one on its directory that does not share deletion, until the child exists.
//! - [`Launch`]: the copy is created by its path and removed once the
//!   launched process has exited; an [`Image`] dropped before it was launched
//!   removes it, and so does a [`Launch`] dropped on any early return.
//! - [`Job`]: `parent_death_kill` holds. Every child is created inside a job
//!   with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` whose only handle its launcher
//!   holds (`PROC_THREAD_ATTRIBUTE_JOB_LIST`), so it never runs outside it.
//!   An adapter's job permits no breakaway; a consumer's permits it on request.
//! - [`Process`]: `exact_process_signal` holds. The child's process handle
//!   names exactly that process; exit is waited for on it and stop terminates
//!   through it.
use super::super::{Failure, Result};
use crate::local::filesystem::{
    self as fs,
    windows::{create_new, final_path, identity, path_identity},
};
use sha2::{Digest, Sha256};
use std::{
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    os::windows::{
        ffi::OsStrExt,
        fs::OpenOptionsExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
    sync::OnceLock,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    Storage::FileSystem::{
        DeleteFileW, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ, FILE_SHARE_READ,
        RemoveDirectoryW,
    },
    System::{
        JobObjects::{
            CreateJobObjectW, JOB_OBJECT_LIMIT_BREAKAWAY_OK, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        },
        Threading::{
            CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DeleteProcThreadAttributeList,
            EXTENDED_STARTUPINFO_PRESENT, GetCurrentProcess, GetExitCodeProcess, INFINITE,
            InitializeProcThreadAttributeList, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
            PROC_THREAD_ATTRIBUTE_JOB_LIST, PROCESS_INFORMATION, STARTF_USESTDHANDLES,
            STARTUPINFOEXW, TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject,
        },
    },
};

const LIMIT: usize = 512 * 1024 * 1024;
const DIRECTORY_PREFIX: &str = "connectors-image-";
const IMAGE: &str = "image.exe";
/// The owner-only directory in the state directory that holds every scratch
/// file of the owner (owner.md, "Windows", "Scratch").
const RUN: &str = "run";
/// The leading bytes kept for the PE check: the DOS header names the offset
/// of the PE signature, which linkers place well inside this.
const HEAD: usize = 4096;

/// The `run` directory this process makes its scratch in, once [`enter`] set it.
static SCRATCH: OnceLock<PathBuf> = OnceLock::new();

/// The owner-only `run` directory of the state directory held as `state`,
/// created owner-only when missing, as its final path. One that exists and is
/// not owner-only, or is a reparse point, is `InvalidConfiguration`.
pub(crate) fn run_directory(state: &File) -> Result<PathBuf> {
    let path = final_path(state)
        .map_err(|_| Failure::InvalidConfiguration)?
        .join(RUN);
    let directory = fs::directory(&path, true, true).map_err(|_| Failure::InvalidConfiguration)?;
    final_path(&directory).map_err(|_| Failure::InvalidConfiguration)
}

/// Makes the state directory's `run` directory this process's scratch for
/// image copies and credentials, and removes every image copy directory a
/// process that died left there whose copy can be deleted: a copy a running
/// process still holds, or runs as its image, stays.
pub(crate) fn enter(state: &File) -> Result<PathBuf> {
    let run = run_directory(state)?;
    if *SCRATCH.get_or_init(|| run.clone()) != run {
        return Err(Failure::Unavailable);
    }
    sweep(&run);
    Ok(run)
}

/// The scratch directory, which [`enter`] set. A process that has not
/// entered a state directory has none and makes no copy; the system's
/// temporary directory is never used, except by this crate's own unit tests,
/// which have no owner.
pub(crate) fn scratch() -> Result<PathBuf> {
    match SCRATCH.get() {
        Some(run) => Ok(run.clone()),
        None if cfg!(test) => Ok(std::env::temp_dir()),
        None => Err(Failure::Unavailable),
    }
}

/// Removes each `connectors-image-*` directory in `run` whose copy can be
/// deleted, with the directory. A name that is not a plain directory, a
/// junction or symbolic link included, is left alone.
fn sweep(run: &Path) {
    let Ok(entries) = std::fs::read_dir(run) else {
        return;
    };
    for entry in entries.flatten() {
        let plain = entry
            .file_type()
            .is_ok_and(|kind| kind.is_dir() && !kind.is_symlink());
        let ours = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(DIRECTORY_PREFIX));
        if plain && ours {
            discard(&entry.path().join(IMAGE));
        }
    }
}

/// Copies `source` into a fresh owner-only directory in the scratch directory,
/// opens the copy for reading without `FILE_SHARE_WRITE` or
/// `FILE_SHARE_DELETE`, and checks the configured SHA-256 and the PE header
/// through that handle, as process creation will read the file. From that
/// open, no process can write, rename or delete the copy while the handle, or
/// a duplicate of it, is open, nor rename its directory while the [`Image`]
/// holds it (`deny_write_copy`). A refused copy is removed with its directory.
pub(crate) fn capture(mut source: File, expected_sha256: &str, until: Instant) -> Result<Image> {
    source
        .seek(SeekFrom::Start(0))
        .map_err(|_| Failure::InvalidConfiguration)?;
    let requested = scratch()?.join(format!("{DIRECTORY_PREFIX}{}", uuid::Uuid::new_v4()));
    if std::fs::symlink_metadata(&requested).is_ok() {
        return Err(Failure::Unavailable);
    }
    let directory = fs::directory(&requested, true, true).map_err(|_| Failure::Unavailable)?;
    let image = match final_path(&directory) {
        Ok(held) => held.join(IMAGE),
        Err(_) => {
            drop(directory);
            discard(&requested.join(IMAGE));
            return Err(Failure::Unavailable);
        }
    };
    match copy_and_check(&mut source, &image, expected_sha256, until) {
        Ok(file) => Ok(Image {
            file: Some(file),
            directory: Some(directory),
            path: image,
        }),
        Err(error) => {
            drop(directory);
            discard(&image);
            Err(error)
        }
    }
}

fn copy_and_check(
    source: &mut File,
    image: &Path,
    expected_sha256: &str,
    until: Instant,
) -> Result<File> {
    let mut copy = create_new(image).map_err(|_| Failure::Unavailable)?;
    pump(source, until, |bytes| {
        copy.write_all(bytes).map_err(|_| Failure::Unavailable)
    })?;
    let written = identity(&copy).map_err(|_| Failure::Unavailable)?;
    drop(copy);
    // At least FILE_GENERIC_READ: the owner duplicates this handle into the
    // launching CLI with that access.
    let mut file = OpenOptions::new()
        .access_mode(FILE_GENERIC_READ)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(image)
        .map_err(|_| Failure::Unavailable)?;
    if identity(&file).map_err(|_| Failure::Unavailable)? != written {
        return Err(Failure::Unavailable);
    }
    let mut digest = Sha256::new();
    let mut head = Vec::with_capacity(HEAD);
    pump(&mut file, until, |bytes| {
        let take = bytes.len().min(HEAD - head.len());
        head.extend_from_slice(&bytes[..take]);
        digest.update(bytes);
        Ok(())
    })?;
    if !portable_executable(&head) || hex::encode(digest.finalize()) != expected_sha256 {
        return Err(Failure::InvalidConfiguration);
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|_| Failure::Unavailable)?;
    Ok(file)
}

/// A PE image: the `MZ` DOS header, whose `e_lfanew` at offset 0x3c names the
/// offset of the `PE\0\0` signature. A batch file or script is neither.
fn portable_executable(head: &[u8]) -> bool {
    let Some(offset) = head
        .get(0x3c..0x40)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
    else {
        return false;
    };
    head.starts_with(b"MZ")
        && usize::try_from(offset)
            .ok()
            .and_then(|start| head.get(start..start.checked_add(4)?))
            == Some(b"PE\0\0".as_slice())
}

/// Reads `from` to its end in bounded chunks, as the Linux capture does: a
/// passed deadline is `Timeout`, a read failure or more than 512 MiB is
/// `InvalidConfiguration`.
fn pump(from: &mut File, until: Instant, mut each: impl FnMut(&[u8]) -> Result<()>) -> Result<()> {
    let mut buffer = [0u8; 65536];
    let mut length = 0usize;
    loop {
        if Instant::now() >= until {
            return Err(Failure::Timeout);
        }
        let count = from
            .read(&mut buffer)
            .map_err(|_| Failure::InvalidConfiguration)?;
        if count == 0 {
            return Ok(());
        }
        length = length
            .checked_add(count)
            .filter(|n| *n <= LIMIT)
            .ok_or(Failure::InvalidConfiguration)?;
        each(&buffer[..count])?;
    }
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

/// Removes a copy and then its directory. A copy still open without delete
/// sharing, or still mapped as a running image, stays, and so does its
/// directory.
fn discard(image: &Path) -> bool {
    let name = wide(image);
    // SAFETY: a NUL-terminated path; a failure only leaves the file.
    let removed = unsafe { DeleteFileW(name.as_ptr()) } != 0;
    if let Some(directory) = image.parent() {
        let directory = wide(directory);
        // SAFETY: a NUL-terminated path; a directory holding anything stays.
        unsafe { RemoveDirectoryW(directory.as_ptr()) };
    }
    removed
}

/// A captured copy and the deny-write handle on it, with a handle on its
/// directory that does not share deletion; it reads as the copy's handle.
/// Dropping it while it still holds the handle removes the copy and its
/// directory, so a refusal between capture and creation leaves nothing behind.
/// A copy whose handle was duplicated into another process cannot be removed
/// while that duplicate is open, and stays for that process's [`Launch`].
pub(crate) struct Image {
    file: Option<File>,
    directory: Option<File>,
    path: PathBuf,
}
impl std::ops::Deref for Image {
    type Target = File;
    fn deref(&self) -> &File {
        self.file
            .as_ref()
            .expect("an image holds its handle until dropped")
    }
}
impl std::ops::DerefMut for Image {
    fn deref_mut(&mut self) -> &mut File {
        self.file
            .as_mut()
            .expect("an image holds its handle until dropped")
    }
}
impl Drop for Image {
    fn drop(&mut self) {
        if let Some(file) = self.file.take() {
            drop(file);
            drop(self.directory.take());
            discard(&self.path);
        }
    }
}
/// The handle alone, for a launch to come, here or in another process: the
/// copy leaves the image's custody, and only a [`Launch`] removes it.
impl From<Image> for File {
    fn from(mut image: Image) -> File {
        image
            .file
            .take()
            .expect("an image holds its handle until dropped")
    }
}
/// The copy's custody passes whole to the launch, with both handles.
impl From<Image> for Launch {
    fn from(mut image: Image) -> Launch {
        Launch {
            image: image.file.take(),
            directory: image.directory.take(),
            path: std::mem::take(&mut image.path),
        }
    }
}

/// The path to create a captured copy's process from, and the custody of the
/// copy: from the moment it exists, dropping it, on any early return
/// included, removes the copy and its directory. It holds the copy's
/// deny-write handle until [`Launch::created`], and a handle on the copy's
/// directory that does not share deletion until it is dropped, so neither can
/// be renamed while the path is in use. Held until the launched process has
/// exited: a running image cannot be removed.
pub(crate) struct Launch {
    image: Option<File>,
    directory: Option<File>,
    path: PathBuf,
}
impl Launch {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
    /// The child exists: its mapped image keeps the copy unwritable from now
    /// on, so the deny-write handle is closed.
    pub(crate) fn created(&mut self) {
        self.image = None;
    }
}
impl Drop for Launch {
    fn drop(&mut self) {
        drop(self.image.take());
        drop(self.directory.take());
        // The image section of a process that has just exited may outlive its
        // exit by a moment.
        for _ in 0..20 {
            if discard(&self.path) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

/// Takes custody of a handle on a copy [`capture`] made, here or in the owner
/// that sent it: named as a capture names one, in an owner-only directory now
/// held against renaming, and the file at that name is the handle's own.
/// Anything else is `Unavailable`, and is never removed.
pub(crate) fn launch_path(image: File) -> Result<Launch> {
    let path = final_path(&image).map_err(|_| Failure::Unavailable)?;
    let ours = path.file_name() == Some(OsStr::new(IMAGE))
        && path
            .parent()
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
            .is_some_and(|name| name.starts_with(DIRECTORY_PREFIX));
    if !ours {
        return Err(Failure::Unavailable);
    }
    let parent = path.parent().ok_or(Failure::Unavailable)?;
    let directory = fs::directory(parent, false, true).map_err(|_| Failure::Unavailable)?;
    if final_path(&directory).map_err(|_| Failure::Unavailable)? != parent
        || path_identity(&path) != Some(identity(&image).map_err(|_| Failure::Unavailable)?)
    {
        return Err(Failure::Unavailable);
    }
    Ok(Launch {
        image: Some(image),
        directory: Some(directory),
        path,
    })
}

/// Appends `argument` to a command line as the Microsoft C runtime and Rust's
/// standard library parse it back: quoted when empty or holding a space, tab,
/// newline or quote, backslashes doubled before a quote.
pub(crate) fn quote(argument: &OsStr, line: &mut Vec<u16>) {
    let units: Vec<u16> = argument.encode_wide().collect();
    if !line.is_empty() {
        line.push(u16::from(b' '));
    }
    let plain = !units.is_empty()
        && !units
            .iter()
            .any(|&unit| matches!(unit, 0x20 | 0x09 | 0x0a | 0x0b | 0x22));
    if plain {
        line.extend(units);
        return;
    }
    line.push(u16::from(b'"'));
    let mut backslashes = 0usize;
    for unit in units {
        if unit == u16::from(b'\\') {
            backslashes += 1;
            continue;
        }
        let repeat = if unit == u16::from(b'"') {
            backslashes * 2 + 1
        } else {
            backslashes
        };
        line.extend(std::iter::repeat_n(u16::from(b'\\'), repeat));
        line.push(unit);
        backslashes = 0;
    }
    line.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes * 2));
    line.push(u16::from(b'"'));
}

/// An inheritable duplicate of `handle`, for one child's creation, with
/// `access` alone, or the same access when `None`.
pub(crate) fn inheritable(handle: HANDLE, access: Option<u32>) -> Result<OwnedHandle> {
    let mut duplicate: HANDLE = std::ptr::null_mut();
    // SAFETY: duplicates a live handle within this process; a returned handle
    // is new and owned.
    if unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            handle,
            GetCurrentProcess(),
            &mut duplicate,
            access.unwrap_or(0),
            1,
            if access.is_some() {
                0
            } else {
                DUPLICATE_SAME_ACCESS
            },
        )
    } == 0
    {
        return Err(Failure::Unavailable);
    }
    // SAFETY: DuplicateHandle returned a new owned handle.
    Ok(unsafe { OwnedHandle::from_raw_handle(duplicate as _) })
}

/// A job object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, whose only handle
/// this value is: when it closes, as it does when its holder dies, every
/// process in the job ends (`parent_death_kill`).
pub(crate) struct Job(OwnedHandle);
impl Job {
    /// A job that permits no breakaway, as an adapter's.
    pub(crate) fn new() -> std::io::Result<Self> {
        Self::with_limits(JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE)
    }
    /// A consumer's job (consumer-launch.md §8, "Breakaway"): a process the
    /// consumer creates with `CREATE_BREAKAWAY_FROM_JOB`, such as an owner,
    /// leaves it (`JOB_OBJECT_LIMIT_BREAKAWAY_OK`); one created without stays
    /// and ends with the job. Never the silent form, which would take every
    /// process the consumer creates out of the job.
    pub(crate) fn consumer() -> std::io::Result<Self> {
        Self::with_limits(JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_BREAKAWAY_OK)
    }
    fn with_limits(flags: u32) -> std::io::Result<Self> {
        // SAFETY: default security, so the handle is not inheritable, and no
        // name, so no other process can open it.
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: CreateJobObjectW returned a new owned handle.
        let job = Self(unsafe { OwnedHandle::from_raw_handle(handle as _) });
        // SAFETY: zero is a valid JOBOBJECT_EXTENDED_LIMIT_INFORMATION.
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags = flags;
        // SAFETY: a live job handle and a structure of the size passed.
        if unsafe {
            SetInformationJobObject(
                job.raw(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        Ok(job)
    }
    /// This process's one job for its adapter children, held for its life.
    pub(crate) fn adapters() -> Result<&'static Self> {
        static JOB: OnceLock<Option<Job>> = OnceLock::new();
        JOB.get_or_init(|| Job::new().ok())
            .as_ref()
            .ok_or(Failure::Unavailable)
    }
    pub(crate) fn raw(&self) -> HANDLE {
        self.0.as_raw_handle() as HANDLE
    }
}

/// What [`create`] starts: the image path, the arguments after argv\[0\], the
/// whole environment, the standard handles (null for none) and every handle
/// the child inherits, the standard ones included.
pub(crate) struct Creation<'a> {
    pub application: &'a Path,
    pub arguments: &'a [OsString],
    pub environment: &'a [(OsString, OsString)],
    pub standard: [HANDLE; 3],
    pub inherited: &'a [HANDLE],
    pub flags: u32,
}

/// The environment block: `name=value` entries sorted by name without regard
/// to case, as Windows keeps it, each NUL-terminated, and a final NUL.
fn environment_block(environment: &[(OsString, OsString)]) -> Vec<u16> {
    let mut entries: Vec<_> = environment.iter().collect();
    entries.sort_by_cached_key(|(name, _)| name.to_string_lossy().to_uppercase());
    let mut block = Vec::new();
    for (name, value) in entries {
        block.extend(name.encode_wide());
        block.push(u16::from(b'='));
        block.extend(value.encode_wide());
        block.push(0);
    }
    if block.is_empty() {
        block.push(0);
    }
    block.push(0);
    block
}

/// Creates `creation.application` inside `job` (`PROC_THREAD_ATTRIBUTE_JOB_LIST`),
/// so it never runs outside the job, with exactly `creation.inherited`
/// inherited (`PROC_THREAD_ATTRIBUTE_HANDLE_LIST`) and the application's own
/// directory, the copy's owner-only directory, as its current directory,
/// never this process's. No shell and no search path: the application is the
/// full path, and argv\[0\] is that path.
pub(crate) fn create(creation: &Creation<'_>, job: &Job) -> std::io::Result<Process> {
    let mut line = Vec::new();
    quote(creation.application.as_os_str(), &mut line);
    for argument in creation.arguments {
        quote(argument, &mut line);
    }
    line.push(0);
    let environment = environment_block(creation.environment);
    let directory = wide(
        creation
            .application
            .parent()
            .ok_or(std::io::ErrorKind::InvalidInput)?,
    );
    struct List(Vec<u64>);
    impl Drop for List {
        fn drop(&mut self) {
            // SAFETY: an initialized list, deleted once.
            unsafe { DeleteProcThreadAttributeList(self.0.as_mut_ptr().cast()) };
        }
    }
    let inherit = !creation.inherited.is_empty();
    let count = 1 + u32::from(inherit);
    let mut size = 0usize;
    // SAFETY: a size query with a null list.
    unsafe { InitializeProcThreadAttributeList(std::ptr::null_mut(), count, 0, &mut size) };
    let mut storage = vec![0u64; size.div_ceil(8)];
    // SAFETY: the storage holds `size` bytes, aligned.
    if unsafe {
        InitializeProcThreadAttributeList(storage.as_mut_ptr().cast(), count, 0, &mut size)
    } == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    let mut list = List(storage);
    let jobs = [job.raw()];
    // SAFETY: the job array outlives the list's use in CreateProcessW.
    if unsafe {
        UpdateProcThreadAttribute(
            list.0.as_mut_ptr().cast(),
            0,
            PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
            jobs.as_ptr().cast(),
            std::mem::size_of_val(&jobs),
            std::ptr::null_mut(),
            std::ptr::null(),
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: the handle array outlives the list's use in CreateProcessW.
    if inherit
        && unsafe {
            UpdateProcThreadAttribute(
                list.0.as_mut_ptr().cast(),
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                creation.inherited.as_ptr().cast(),
                std::mem::size_of_val(creation.inherited),
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
    [
        startup.StartupInfo.hStdInput,
        startup.StartupInfo.hStdOutput,
        startup.StartupInfo.hStdError,
    ] = creation.standard;
    startup.lpAttributeList = list.0.as_mut_ptr().cast();
    let flags = creation.flags | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT;
    // SAFETY: zero is a valid PROCESS_INFORMATION.
    let mut information: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let application = wide(creation.application);
    // SAFETY: NUL-terminated application, current directory and writable
    // command line, a double-NUL-terminated environment block, and live
    // startup information whose attribute list names live handles.
    if unsafe {
        CreateProcessW(
            application.as_ptr(),
            line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            i32::from(inherit),
            flags,
            environment.as_ptr().cast(),
            directory.as_ptr(),
            &startup.StartupInfo,
            &mut information,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    drop(list);
    // SAFETY: CreateProcessW returned new owned handles; the thread's is not kept.
    unsafe {
        drop(OwnedHandle::from_raw_handle(information.hThread as _));
        Ok(Process::new(OwnedHandle::from_raw_handle(
            information.hProcess as _,
        )))
    }
}

/// A created child, owned through its process handle, which keeps its id
/// from reuse while open (`exact_process_signal`).
pub(crate) struct Process {
    handle: OwnedHandle,
    status: Option<u32>,
}
impl Process {
    fn new(handle: OwnedHandle) -> Self {
        Self {
            handle,
            status: None,
        }
    }
    fn raw(&self) -> HANDLE {
        self.handle.as_raw_handle() as HANDLE
    }
    /// A duplicate of the process handle, for a stop path of its own.
    pub(crate) fn try_clone_handle(&self) -> std::io::Result<OwnedHandle> {
        self.handle.try_clone()
    }
    /// The exit code once the child has exited, without blocking.
    pub(crate) fn try_wait(&mut self) -> std::io::Result<Option<u32>> {
        if self.status.is_none() && exited(&self.handle)? {
            self.status = Some(exit_code(&self.handle)?);
        }
        Ok(self.status)
    }
    /// Waits for exit on the process handle.
    pub(crate) fn wait(&mut self) -> std::io::Result<u32> {
        if let Some(status) = self.status {
            return Ok(status);
        }
        // SAFETY: a live process handle.
        if unsafe { WaitForSingleObject(self.raw(), INFINITE) } != WAIT_OBJECT_0 {
            return Err(std::io::Error::last_os_error());
        }
        let status = exit_code(&self.handle)?;
        self.status = Some(status);
        Ok(status)
    }
    pub(crate) fn kill(&mut self) -> std::io::Result<()> {
        terminate(&self.handle)
    }
}

/// Whether the process `handle` names has exited, without blocking.
pub(crate) fn exited(handle: &OwnedHandle) -> std::io::Result<bool> {
    // SAFETY: a live process handle and no wait.
    match unsafe { WaitForSingleObject(handle.as_raw_handle() as HANDLE, 0) } {
        WAIT_OBJECT_0 => Ok(true),
        WAIT_TIMEOUT => Ok(false),
        _ => Err(std::io::Error::last_os_error()),
    }
}

/// Terminates exactly the process `handle` names. One that has already
/// exited is sent nothing and is no error.
pub(crate) fn terminate(handle: &OwnedHandle) -> std::io::Result<()> {
    // SAFETY: a process handle this process owns, never an id from metadata.
    if unsafe { TerminateProcess(handle.as_raw_handle() as HANDLE, 1) } != 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if exited(handle)? { Ok(()) } else { Err(error) }
}

fn exit_code(handle: &OwnedHandle) -> std::io::Result<u32> {
    let mut code = 0u32;
    // SAFETY: a live process handle and one writable code.
    if unsafe { GetExitCodeProcess(handle.as_raw_handle() as HANDLE, &mut code) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn until() -> Instant {
        Instant::now() + Duration::from_secs(30)
    }

    /// A PE source in a fresh directory, and its digest.
    fn source() -> (tempfile::TempDir, PathBuf, String) {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("source.exe");
        let bytes = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        std::fs::write(&path, &bytes).unwrap();
        (root, path, hex::encode(Sha256::digest(&bytes)))
    }

    /// `sealed_executable`: `deny_write_copy`. The checked copy keeps the
    /// admitted bytes whatever happens to the source, and while its handle is
    /// held no process can open it for writing, rename it or delete it.
    #[test]
    fn a_capture_is_a_checked_copy_no_process_can_write_rename_or_remove() {
        let (root, path, hash) = source();
        let expected = std::fs::read(&path).unwrap();
        let mut image = capture(File::open(&path).unwrap(), &hash, until()).unwrap();
        std::fs::write(&path, b"MZ changed in place").unwrap();
        std::fs::rename(&path, root.path().join("old.exe")).unwrap();
        let mut captured = Vec::new();
        image.read_to_end(&mut captured).unwrap();
        assert_eq!(captured, expected);

        let copy = image.path.clone();
        assert!(OpenOptions::new().write(true).open(&copy).is_err());
        assert!(OpenOptions::new().append(true).open(&copy).is_err());
        assert!(std::fs::remove_file(&copy).is_err());
        assert!(std::fs::rename(&copy, copy.with_file_name("moved.exe")).is_err());
        assert!(copy.exists());

        // A refusal between capture and creation leaves nothing behind.
        drop(image);
        assert!(!copy.exists());
        assert!(!copy.parent().unwrap().exists());
    }

    #[test]
    fn a_wrong_digest_a_script_or_a_passed_deadline_refuses() {
        let (root, path, hash) = source();
        let other = "0".repeat(64);
        assert!(matches!(
            capture(File::open(&path).unwrap(), &other, until()),
            Err(Failure::InvalidConfiguration)
        ));
        let script = root.path().join("run.cmd");
        let bytes = b"@echo off\r\nexit /b 0\r\n";
        std::fs::write(&script, bytes).unwrap();
        assert!(matches!(
            capture(
                File::open(&script).unwrap(),
                &hex::encode(Sha256::digest(bytes)),
                until()
            ),
            Err(Failure::InvalidConfiguration)
        ));
        // A DOS header alone, with no PE signature where it points.
        let mut dos = vec![0u8; 0x80];
        dos[..2].copy_from_slice(b"MZ");
        dos[0x3c] = 0x40;
        assert!(!portable_executable(&dos));
        dos[0x40..0x44].copy_from_slice(b"PE\0\0");
        assert!(portable_executable(&dos));
        assert!(matches!(
            capture(File::open(&path).unwrap(), &hash, Instant::now()),
            Err(Failure::Timeout)
        ));
    }

    /// Only a capture's own copy is launched or removed by path.
    #[test]
    fn a_launch_path_names_only_a_captured_copy_and_removes_it_after_use() {
        let (_root, path, hash) = source();
        assert!(matches!(
            launch_path(File::open(&path).unwrap()),
            Err(Failure::Unavailable)
        ));
        let image = capture(File::open(&path).unwrap(), &hash, until()).unwrap();
        let copy = image.path.clone();
        // Released as a handle, as the owner sends it, the copy outlives its
        // image, and the launch that takes the handle takes its custody.
        let mut launch = launch_path(File::from(image)).unwrap();
        assert_eq!(launch.path(), copy);
        launch.created();
        assert!(copy.exists());
        drop(launch);
        assert!(!copy.exists());
        assert!(!copy.parent().unwrap().exists());
        assert!(path.exists());
    }

    /// The owner sweeps its `run` directory when it starts: a copy no process
    /// holds is removed with its directory, one still held stays.
    #[test]
    fn a_sweep_removes_left_copies_that_no_process_holds() {
        let (root, path, _) = source();
        let run = root.path().join("run");
        std::fs::create_dir(&run).unwrap();
        let left = run.join(format!("{DIRECTORY_PREFIX}left"));
        std::fs::create_dir(&left).unwrap();
        std::fs::copy(&path, left.join(IMAGE)).unwrap();
        let held = run.join(format!("{DIRECTORY_PREFIX}held"));
        std::fs::create_dir(&held).unwrap();
        std::fs::copy(&path, held.join(IMAGE)).unwrap();
        let holder = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(held.join(IMAGE))
            .unwrap();
        let other = run.join("other");
        std::fs::create_dir(&other).unwrap();
        sweep(&run);
        assert!(!left.exists(), "a copy no process holds was left");
        assert!(held.join(IMAGE).exists(), "a held copy was removed");
        assert!(
            other.exists(),
            "a directory not named as a copy was removed"
        );
        drop(holder);
    }

    /// `deny_write_copy`: while an image or its launch holds the copy, its
    /// directory cannot be renamed, so the launch path names the copy until the
    /// child exists.
    #[test]
    fn the_directory_of_a_held_copy_cannot_be_renamed() {
        let (root, path, hash) = source();
        let image = capture(File::open(&path).unwrap(), &hash, until()).unwrap();
        let directory = image.path.parent().unwrap().to_path_buf();
        let moved = root.path().join("moved");
        assert!(std::fs::rename(&directory, &moved).is_err());
        let launch = launch_path(File::from(image)).unwrap();
        assert!(std::fs::rename(&directory, &moved).is_err());
        drop(launch);
        assert!(!directory.exists());
    }

    /// The process a job test starts: it only waits, until the job ends it.
    #[test]
    fn job_member_fixture() {
        if std::env::args().any(|argument| argument == "job-member") {
            std::thread::sleep(Duration::from_secs(60));
        }
    }

    fn member(job: &Job) -> Process {
        let arguments: Vec<OsString> = [
            "--exact",
            "local::runtime::artifact::windows::tests::job_member_fixture",
            "--",
            "job-member",
        ]
        .map(OsString::from)
        .to_vec();
        create(
            &Creation {
                application: &std::env::current_exe().unwrap(),
                arguments: &arguments,
                environment: &[],
                standard: [std::ptr::null_mut(); 3],
                inherited: &[],
                flags: 0,
            },
            job,
        )
        .unwrap()
    }

    fn ends_within(process: &mut Process, wait: Duration) {
        let deadline = Instant::now() + wait;
        while process.try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "the process outlived its end");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// `parent_death_kill`: closing the job's only handle, as its holder's
    /// death does, ends every process in it.
    #[test]
    fn closing_the_job_ends_its_processes() {
        let job = Job::new().unwrap();
        let mut process = member(&job);
        std::thread::sleep(Duration::from_millis(200));
        assert!(process.try_wait().unwrap().is_none());
        drop(job);
        ends_within(&mut process, Duration::from_secs(10));
    }

    /// `exact_process_signal`: a stop through a duplicated process handle ends
    /// exactly that process, and one that has exited is sent nothing.
    #[test]
    fn a_stop_handle_terminates_exactly_its_process() {
        let job = Job::new().unwrap();
        let mut process = member(&job);
        let mut other = member(&job);
        let handle = process.try_clone_handle().unwrap();
        assert!(!exited(&handle).unwrap());
        terminate(&handle).unwrap();
        ends_within(&mut process, Duration::from_secs(10));
        assert!(exited(&handle).unwrap());
        assert_eq!(process.try_wait().unwrap(), Some(1));
        terminate(&handle).unwrap();
        assert!(other.try_wait().unwrap().is_none());
        other.kill().unwrap();
        other.wait().unwrap();
    }

    #[test]
    fn the_environment_block_is_sorted_and_double_terminated() {
        assert_eq!(environment_block(&[]), vec![0, 0]);
        let block = environment_block(&[("b".into(), "2".into()), ("A".into(), "1".into())]);
        let text = String::from_utf16(&block).unwrap();
        assert_eq!(text, "A=1\0b=2\0\0");
    }
}

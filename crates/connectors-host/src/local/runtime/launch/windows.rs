//! Consumer launch delivery on Windows (consumer-launch.md §8): the owner's
//! credential handle and the transfer of the image and credential handles to
//! the launching CLI. Creating the consumer from them is the CLI's.
use super::super::{Failure, Result, channel};
use crate::local::filesystem::{
    self as fs,
    windows::{create_new, final_path, identity},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::windows::{
        ffi::OsStrExt,
        fs::OpenOptionsExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::Path,
    time::Instant,
};
use windows_sys::Win32::{
    Foundation::{DUPLICATE_CLOSE_SOURCE, DuplicateHandle, HANDLE, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{
        DELETE, DeleteFileW, FILE_DISPOSITION_FLAG_DELETE, FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
        FILE_DISPOSITION_INFO_EX, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ, FILE_TYPE_DISK,
        FileDispositionInfoEx, GetFileType, RemoveDirectoryW, SetFileInformationByHandle,
    },
    System::Threading::GetCurrentProcess,
};

/// The control section that carries the handle values, in place of the
/// marker byte and its `SCM_RIGHTS` message.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Handles {
    handles: Vec<u64>,
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

/// The credential (consumer-launch.md §8, "Credential"): `material` is
/// written verbatim into a file in a fresh owner-only directory, the file's
/// name is removed with POSIX delete semantics while a handle stays open, and
/// only a read-only handle positioned at its start is returned, not
/// inheritable. No path names the bytes afterwards; the directory is removed.
pub(crate) fn sealed(material: &[u8]) -> Result<File> {
    /// Removes the credential file, then its directory, on every exit.
    struct Scratch(Vec<u16>, Vec<u16>);
    impl Drop for Scratch {
        fn drop(&mut self) {
            // SAFETY: NUL-terminated paths this function created; a name
            // already gone is a harmless failure.
            unsafe {
                DeleteFileW(self.1.as_ptr());
                RemoveDirectoryW(self.0.as_ptr());
            }
        }
    }
    let requested =
        std::env::temp_dir().join(format!("connectors-credential-{}", uuid::Uuid::new_v4()));
    if std::fs::symlink_metadata(&requested).is_ok() {
        return Err(Failure::Unavailable);
    }
    let directory = fs::directory(&requested, true, true).map_err(|_| Failure::Unavailable)?;
    let held = final_path(&directory).map_err(|_| Failure::Unavailable)?;
    let path = held.join("credential");
    let scratch = Scratch(wide(&held), wide(&path));
    let mut writable = create_new(&path).map_err(|_| Failure::Unavailable)?;
    writable
        .write_all(material)
        .map_err(|_| Failure::Unavailable)?;
    let written = identity(&writable).map_err(|_| Failure::Unavailable)?;
    drop(writable);
    let deleting = OpenOptions::new()
        .access_mode(FILE_GENERIC_READ | DELETE)
        .share_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(&path)
        .map_err(|_| Failure::Unavailable)?;
    if identity(&deleting).map_err(|_| Failure::Unavailable)? != written
        || deleting.metadata().map_err(|_| Failure::Unavailable)?.len() != material.len() as u64
    {
        return Err(Failure::Unavailable);
    }
    let disposition = FILE_DISPOSITION_INFO_EX {
        Flags: FILE_DISPOSITION_FLAG_DELETE | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
    };
    // SAFETY: a live handle opened with DELETE and a structure of the size passed.
    if unsafe {
        SetFileInformationByHandle(
            deleting.as_raw_handle() as HANDLE,
            FileDispositionInfoEx,
            (&disposition as *const FILE_DISPOSITION_INFO_EX).cast(),
            std::mem::size_of::<FILE_DISPOSITION_INFO_EX>() as u32,
        )
    } == 0
    {
        return Err(Failure::Unavailable);
    }
    let mut readable: HANDLE = std::ptr::null_mut();
    // SAFETY: duplicates within this process with read access alone, not
    // inheritable; a returned handle is new and owned.
    if unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            deleting.as_raw_handle() as HANDLE,
            GetCurrentProcess(),
            &mut readable,
            FILE_GENERIC_READ,
            0,
            0,
        )
    } == 0
    {
        return Err(Failure::Unavailable);
    }
    // SAFETY: DuplicateHandle returned a new owned handle.
    let readable = unsafe { File::from_raw_handle(readable as _) };
    drop(deleting);
    drop(directory);
    drop(scratch);
    if std::fs::symlink_metadata(&path).is_ok() || std::fs::symlink_metadata(&held).is_ok() {
        return Err(Failure::Unavailable);
    }
    Ok(readable)
}

/// Duplicates each of `files` into the peer's process with read access
/// alone and inheritance off (`atomic_close_on_exec`: holds), through the one
/// process handle opened when the peer was identified. Returns the values the
/// handles have there. On a failure, every duplicate made is closed again.
fn duplicate_into(peer: &channel::Peer, files: &[&File]) -> Result<Vec<u64>> {
    let mut values = Vec::with_capacity(files.len());
    for file in files {
        let mut target: HANDLE = std::ptr::null_mut();
        // SAFETY: a live source handle, the peer's process handle opened with
        // PROCESS_DUP_HANDLE and a writable out value.
        if unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                file.as_raw_handle() as HANDLE,
                peer.process(),
                &mut target,
                FILE_GENERIC_READ,
                0,
                0,
            )
        } == 0
        {
            close_in(peer, &values);
            return Err(Failure::Unavailable);
        }
        values.push(target as usize as u64);
    }
    Ok(values)
}

/// Closes handles this process duplicated into the peer and the peer never
/// took over (`DUPLICATE_CLOSE_SOURCE` from the same process handle).
fn close_in(peer: &channel::Peer, values: &[u64]) {
    for value in values {
        // SAFETY: closes a handle in the peer's process that this process
        // created there; no handle is created in return.
        unsafe {
            DuplicateHandle(
                peer.process(),
                *value as usize as HANDLE,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
                0,
                DUPLICATE_CLOSE_SOURCE,
            );
        }
    }
}

/// Pass `files` to the peer: each handle is duplicated into the peer's
/// process and the values follow in one frame, in place of `SCM_RIGHTS`.
/// Only the handles travel; no byte of their content is read or written. A
/// frame that is not sent whole leaves the duplicates closed again.
pub(crate) fn send(stream: &mut channel::Stream, files: &[&File], until: Instant) -> Result<()> {
    let peer = channel::peer(stream)?;
    let values = duplicate_into(&peer, files)?;
    let sent = channel::write(
        stream,
        &Handles {
            handles: values.clone(),
        },
        None,
        &[],
        until,
    );
    if sent.is_err() {
        close_in(&peer, &values);
    }
    sent
}

/// Receive exactly `count` handles sent by [`send`]. Anything else, another
/// count or a value that is not an open file handle that a child would not
/// inherit, refuses; handles already taken over are closed.
pub(crate) fn receive(
    stream: &mut channel::Stream,
    count: usize,
    until: Instant,
) -> Result<Vec<File>> {
    let frame = channel::read::<Handles>(stream, until, false, 0)?;
    if frame.control.handles.len() != count {
        return Err(Failure::Protocol);
    }
    let mut files = Vec::with_capacity(count);
    for value in frame.control.handles {
        let handle = usize::try_from(value).map_err(|_| Failure::Protocol)? as HANDLE;
        if handle.is_null()
            || handle == INVALID_HANDLE_VALUE
            // SAFETY: GetFileType only queries a handle value.
            || unsafe { GetFileType(handle) } != FILE_TYPE_DISK
            || channel::windows::inheritable(handle).unwrap_or(true)
        {
            return Err(Failure::Protocol);
        }
        // SAFETY: the owner duplicated this handle into this process for this
        // receive; it is taken over exactly once.
        files.push(unsafe { File::from_raw_handle(handle as _) });
    }
    Ok(files)
}

#[cfg(test)]
mod tests;

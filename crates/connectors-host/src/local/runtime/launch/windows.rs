//! Consumer launch delivery on Windows (consumer-launch.md §8): the owner's
//! credential handle and the transfer of the image and credential handles to
//! the launching CLI. Creating the consumer from them is the CLI's.
use super::super::{Failure, Result, artifact, channel};
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
    Foundation::{
        CloseHandle, DUPLICATE_CLOSE_SOURCE, DuplicateHandle, HANDLE, INVALID_HANDLE_VALUE,
    },
    Storage::FileSystem::{
        DELETE, DeleteFileW, FILE_DISPOSITION_FLAG_DELETE, FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
        FILE_DISPOSITION_INFO_EX, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
        FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_TYPE_DISK,
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
/// written verbatim into a file in a fresh owner-only directory in the
/// scratch directory, the state directory's `run` ([`artifact::scratch`]),
/// never the system's temporary directory; the file's name is removed with
/// POSIX delete semantics while a handle stays open, and only a read-only
/// handle positioned at its start is returned, not inheritable. No path names
/// the bytes afterwards; the directory is removed.
pub(crate) fn sealed(material: &[u8]) -> Result<File> {
    /// Removes the credential file, then its directory, on every exit, after
    /// closing its own handle on the directory, which does not share deletion.
    struct Scratch {
        directory: Option<File>,
        held: Vec<u16>,
        path: Vec<u16>,
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            drop(self.directory.take());
            // SAFETY: NUL-terminated paths this function created; a name
            // already gone is a harmless failure.
            unsafe {
                DeleteFileW(self.path.as_ptr());
                RemoveDirectoryW(self.held.as_ptr());
            }
        }
    }
    let requested =
        artifact::scratch()?.join(format!("connectors-credential-{}", uuid::Uuid::new_v4()));
    if std::fs::symlink_metadata(&requested).is_ok() {
        return Err(Failure::Unavailable);
    }
    let directory = fs::directory(&requested, true, true).map_err(|_| Failure::Unavailable)?;
    let held = match final_path(&directory) {
        Ok(held) => held,
        Err(_) => {
            drop(directory);
            // SAFETY: a NUL-terminated path this function created.
            unsafe { RemoveDirectoryW(wide(&requested).as_ptr()) };
            return Err(Failure::Unavailable);
        }
    };
    let path = held.join("credential");
    let scratch = Scratch {
        directory: Some(directory),
        held: wide(&held),
        path: wide(&path),
    };
    let mut writable = create_new(&path).map_err(|_| Failure::Unavailable)?;
    writable
        .write_all(material)
        .map_err(|_| Failure::Unavailable)?;
    let written = identity(&writable).map_err(|_| Failure::Unavailable)?;
    drop(writable);
    // The read-only handle is a file object of its own. A POSIX delete
    // removes the name when the file object that set it is cleaned up, which
    // is when its last handle closes: a duplicate of the deleting handle
    // would keep the name, and the directory, in place. Neither open shares
    // writing, so no writer can open the file from here on.
    let readable = OpenOptions::new()
        .access_mode(FILE_GENERIC_READ)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(&path)
        .map_err(|_| Failure::Unavailable)?;
    let deleting = OpenOptions::new()
        .access_mode(DELETE | FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(&path)
        .map_err(|_| Failure::Unavailable)?;
    if identity(&readable).map_err(|_| Failure::Unavailable)? != written
        || identity(&deleting).map_err(|_| Failure::Unavailable)? != written
        || readable.metadata().map_err(|_| Failure::Unavailable)?.len() != material.len() as u64
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
    // Closing the deleting handle removes the name; `readable`, opened by
    // `OpenOptions` without inheritance and at offset 0, stays.
    drop(deleting);
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

/// Whether `handle` is one the owner could have duplicated in: an open file
/// handle that a child would not inherit.
fn arrived(handle: HANDLE) -> bool {
    !handle.is_null()
        && handle != INVALID_HANDLE_VALUE
        // SAFETY: GetFileType only queries a handle value.
        && unsafe { GetFileType(handle) } == FILE_TYPE_DISK
        && !channel::windows::inheritable(handle).unwrap_or(true)
}

/// Receive exactly `count` handles sent by [`send`]. Anything else refuses:
/// another count, a value named twice, which would be taken over twice and
/// closed twice, or a value that is not an open file handle that a child
/// would not inherit. A refused frame leaves nothing open: every distinct
/// value it names that is such a handle, one the owner duplicated in, is
/// closed; none is taken over.
pub(crate) fn receive(
    stream: &mut channel::Stream,
    count: usize,
    until: Instant,
) -> Result<Vec<File>> {
    let frame = channel::read::<Handles>(stream, until, false, 0)?;
    let values: Vec<HANDLE> = frame
        .control
        .handles
        .iter()
        .map(|value| usize::try_from(*value).map_or(INVALID_HANDLE_VALUE, |value| value as HANDLE))
        .collect();
    let distinct: std::collections::BTreeSet<usize> =
        values.iter().map(|handle| *handle as usize).collect();
    if values.len() != count
        || distinct.len() != values.len()
        || !values.iter().all(|handle| arrived(*handle))
    {
        for value in distinct {
            let handle = value as HANDLE;
            if arrived(handle) {
                // SAFETY: a handle the owner duplicated into this process for
                // this frame, never taken over, closed once.
                unsafe { CloseHandle(handle) };
            }
        }
        return Err(Failure::Protocol);
    }
    Ok(values
        .into_iter()
        .map(|handle| {
            // SAFETY: the owner duplicated each of these distinct handles into
            // this process for this receive; each is taken over exactly once.
            unsafe { File::from_raw_handle(handle as _) }
        })
        .collect())
}

#[cfg(test)]
mod tests;

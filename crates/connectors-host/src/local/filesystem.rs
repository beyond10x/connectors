//! Descriptor-relative Linux path admission. No component follows a symlink.
use super::{Failure, Result};
use std::{
    ffi::{CString, OsStr},
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path},
};

pub fn uid() -> u32 {
    // SAFETY: geteuid has no preconditions.
    unsafe { libc::geteuid() }
}

fn name(value: &OsStr) -> Result<CString> {
    if value.is_empty() || value == "." || value == ".." || value.as_bytes().contains(&b'/') {
        return Err(Failure::InvalidConfiguration);
    }
    CString::new(value.as_bytes()).map_err(|_| Failure::InvalidConfiguration)
}

fn open_at(parent: &File, name: &OsStr, flags: i32, mode: u32) -> std::io::Result<File> {
    let name =
        self::name(name).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    // SAFETY: parent and name remain live; successful ownership passes to File.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            mode,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: openat returned a new, owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn admitted_directory(file: &File, private: bool) -> Result<()> {
    let stat = file.metadata().map_err(|_| Failure::InvalidConfiguration)?;
    if !stat.is_dir()
        || (stat.uid() != uid() && stat.uid() != 0)
        || stat.mode() & 0o022 != 0
        || (private && (stat.uid() != uid() || stat.mode() & 0o077 != 0))
    {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(())
}

pub fn validate_path(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path.as_os_str().as_bytes().len() > 4096
        || path
            .components()
            .any(|p| !matches!(p, Component::RootDir | Component::Normal(_)))
    {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(())
}

/// Open an admitted directory, optionally creating missing components privately.
/// Existing directories are never chmodded or repaired implicitly.
pub fn directory(path: &Path, create: bool, private: bool) -> Result<File> {
    validate_path(path)?;
    let mut current = File::open("/").map_err(|_| Failure::InvalidConfiguration)?;
    admitted_directory(&current, false)?;
    for component in path.components() {
        let Component::Normal(part) = component else {
            continue;
        };
        let next = match open_at(&current, part, libc::O_RDONLY | libc::O_DIRECTORY, 0) {
            Ok(next) => next,
            Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {
                let part_c = name(part)?;
                // SAFETY: both the directory descriptor and C string are valid.
                let result = unsafe { libc::mkdirat(current.as_raw_fd(), part_c.as_ptr(), 0o700) };
                if result != 0
                    && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
                {
                    return Err(Failure::InvalidConfiguration);
                }
                let next = open_at(&current, part, libc::O_RDONLY | libc::O_DIRECTORY, 0)
                    .map_err(|_| Failure::InvalidConfiguration)?;
                next.sync_all().map_err(|_| Failure::OutcomeUnknown)?;
                current.sync_all().map_err(|_| Failure::OutcomeUnknown)?;
                next
            }
            Err(_) => return Err(Failure::InvalidConfiguration),
        };
        admitted_directory(&next, false)?;
        current = next;
    }
    admitted_directory(&current, private)?;
    Ok(current)
}

pub fn private_file_at(parent: &File, child: &OsStr) -> Result<File> {
    let file =
        open_at(parent, child, libc::O_RDONLY, 0).map_err(|_| Failure::InvalidConfiguration)?;
    check_private_file(&file)?;
    Ok(file)
}

pub fn check_private_file(file: &File) -> Result<()> {
    let stat = file.metadata().map_err(|_| Failure::InvalidConfiguration)?;
    if !stat.is_file() || stat.uid() != uid() || stat.mode() & 0o077 != 0 || stat.nlink() != 1 {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(())
}

pub fn private_file(path: &Path) -> Result<File> {
    validate_path(path)?;
    let parent = directory(
        path.parent().ok_or(Failure::InvalidConfiguration)?,
        false,
        false,
    )?;
    private_file_at(
        &parent,
        path.file_name().ok_or(Failure::InvalidConfiguration)?,
    )
}

pub fn read_bounded(file: File, limit: usize) -> Result<Vec<u8>> {
    if file
        .metadata()
        .map_err(|_| Failure::InvalidConfiguration)?
        .len()
        > limit as u64
    {
        return Err(Failure::InvalidConfiguration);
    }
    let mut data = Vec::new();
    file.take((limit + 1) as u64)
        .read_to_end(&mut data)
        .map_err(|_| Failure::InvalidConfiguration)?;
    if data.len() > limit {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(data)
}

/// Inspect a held directory without following the destination or creating it.
pub(crate) fn require_absent(parent: &File, child: &OsStr) -> Result<()> {
    let child = name(child)?;
    // SAFETY: fstatat initializes the live stat buffer on success. NOFOLLOW
    // makes even a dangling link an existing destination, never an empty name.
    let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            child.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        return Err(Failure::ConfigurationExists);
    }
    if std::io::Error::last_os_error().kind() != std::io::ErrorKind::NotFound {
        return Err(Failure::InvalidConfiguration);
    }
    Ok(())
}

/// Publish one durable private file, atomically refusing any existing name.
pub fn publish_new(parent: &File, child: &OsStr, contents: &[u8]) -> Result<()> {
    let temporary = format!(".connectors-{}", uuid::Uuid::new_v4());
    let temp_name = OsStr::new(&temporary);
    let mut file = open_at(
        parent,
        temp_name,
        libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        0o600,
    )
    .map_err(|_| Failure::InvalidConfiguration)?;
    let temporary_c = name(temp_name)?;
    let child_c = name(child)?;
    let result = (|| {
        file.write_all(contents)
            .map_err(|_| Failure::InvalidConfiguration)?;
        file.sync_all().map_err(|_| Failure::OutcomeUnknown)?;
        // SAFETY: valid directory descriptors and C strings; RENAME_NOREPLACE
        // gives concurrent creators a single winner, including symlink targets.
        let status = unsafe {
            libc::renameat2(
                parent.as_raw_fd(),
                temporary_c.as_ptr(),
                parent.as_raw_fd(),
                child_c.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if status != 0 {
            return Err(
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::AlreadyExists {
                    Failure::ConfigurationExists
                } else {
                    Failure::InvalidConfiguration
                },
            );
        }
        parent.sync_all().map_err(|_| Failure::OutcomeUnknown)
    })();
    // SAFETY: only this call's unguessable temporary name is removed. After
    // successful rename it no longer exists; never remove the published file.
    unsafe {
        libc::unlinkat(parent.as_raw_fd(), temporary_c.as_ptr(), 0);
    }
    result
}

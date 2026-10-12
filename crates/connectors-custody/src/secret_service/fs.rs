//! Descriptor-relative path admission for the Secret Service binding, the
//! subset of the host's `local::filesystem` that custody reads through. No
//! component follows a symlink, and custody never creates or repairs a path.
use std::{
    ffi::{CString, OsStr},
    fs::File,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::MetadataExt},
    },
    path::{Component, Path},
};

/// Refused; custody maps every admission failure to `Unavailable`.
pub(crate) struct Refused;
type Result<T> = std::result::Result<T, Refused>;

pub(crate) fn uid() -> u32 {
    // SAFETY: geteuid has no preconditions.
    unsafe { libc::geteuid() }
}

fn name(value: &OsStr) -> Result<CString> {
    if value.is_empty() || value == "." || value == ".." || value.as_bytes().contains(&b'/') {
        return Err(Refused);
    }
    CString::new(value.as_bytes()).map_err(|_| Refused)
}

fn open_at(parent: &File, name: &OsStr, flags: i32) -> Result<File> {
    let name = self::name(name)?;
    // SAFETY: parent and name remain live; successful ownership passes to File.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            0,
        )
    };
    if fd < 0 {
        return Err(Refused);
    }
    // SAFETY: openat returned a new, owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn admitted_directory(file: &File, private: bool) -> Result<()> {
    let stat = file.metadata().map_err(|_| Refused)?;
    if !stat.is_dir()
        || (stat.uid() != uid() && stat.uid() != 0)
        || stat.mode() & 0o022 != 0
        || (private && (stat.uid() != uid() || stat.mode() & 0o077 != 0))
    {
        return Err(Refused);
    }
    Ok(())
}

pub(crate) fn validate_path(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path.as_os_str().as_bytes().len() > 4096
        || path
            .components()
            .any(|p| !matches!(p, Component::RootDir | Component::Normal(_)))
    {
        return Err(Refused);
    }
    Ok(())
}

/// Open an existing admitted directory.
pub(crate) fn directory(path: &Path, private: bool) -> Result<File> {
    validate_path(path)?;
    let mut current = File::open("/").map_err(|_| Refused)?;
    admitted_directory(&current, false)?;
    for component in path.components() {
        let Component::Normal(part) = component else {
            continue;
        };
        let next = open_at(&current, part, libc::O_RDONLY | libc::O_DIRECTORY)?;
        admitted_directory(&next, false)?;
        current = next;
    }
    admitted_directory(&current, private)?;
    Ok(current)
}

pub(crate) fn private_file_at(parent: &File, child: &OsStr) -> Result<File> {
    let file = open_at(parent, child, libc::O_RDONLY)?;
    let stat = file.metadata().map_err(|_| Refused)?;
    if !stat.is_file() || stat.uid() != uid() || stat.mode() & 0o077 != 0 || stat.nlink() != 1 {
        return Err(Refused);
    }
    Ok(file)
}

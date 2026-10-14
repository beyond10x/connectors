//! A jail for tests of the mounted binding, without privileges: a child process
//! enters its own user and mount namespaces, binds the host's system
//! directories into a new root directory the test owns, binds chosen
//! directories read-only and changes its root to it.
//!
//! Why a jail: in an unprivileged user namespace only the test's own user is
//! mapped, so host directories owned by root appear owned by the overflow user
//! and the binding's ancestor checks refuse them, as they should. The jail's
//! root and everything the test lays out in it are the test's own.
//!
//! It needs a kernel that lets an unprivileged process create user namespaces
//! with capabilities (`kernel.unprivileged_userns_clone`, and on Ubuntu
//! `kernel.apparmor_restrict_unprivileged_userns = 0`). A child that cannot
//! enter them fails to start, and [`Jail::spawn_error`] names the prerequisite.
use std::{
    ffi::CString,
    fs,
    os::unix::{ffi::OsStrExt, fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::Command,
};

/// How the test's user appears inside the jail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Map {
    /// As user and group 0: everything the test owns appears root-owned.
    Root,
    /// As itself, a user that is not root.
    Own,
}

struct Bind {
    source: CString,
    target: CString,
    read_only: bool,
}

/// A new root directory, owned by the test's user, and the binds a child
/// makes before it enters it.
pub struct Jail {
    root: tempfile::TempDir,
    binds: Vec<(PathBuf, PathBuf, bool)>,
}

/// System directories bound into every jail, when the host has them as directories.
const SYSTEM: [&str; 4] = ["usr", "etc", "proc", "dev"];
/// Top-level entries that are symbolic links on merged-`/usr` hosts.
const LINKS: [&str; 6] = ["bin", "sbin", "lib", "lib32", "lib64", "libx32"];

impl Default for Jail {
    fn default() -> Self {
        Self::new()
    }
}

impl Jail {
    pub fn new() -> Self {
        let root = tempfile::Builder::new()
            .prefix("mounted-jail-")
            .tempdir()
            .expect("a jail root in the temporary directory");
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
        let mut binds = Vec::new();
        for name in SYSTEM {
            let host = Path::new("/").join(name);
            if host.is_dir() && !host.is_symlink() {
                fs::create_dir(root.path().join(name)).unwrap();
                binds.push((host, root.path().join(name), false));
            }
        }
        for name in LINKS {
            let host = Path::new("/").join(name);
            if let Ok(target) = fs::read_link(&host) {
                std::os::unix::fs::symlink(target, root.path().join(name)).unwrap();
            } else if host.is_dir() {
                fs::create_dir(root.path().join(name)).unwrap();
                binds.push((host, root.path().join(name), false));
            }
        }
        Self { root, binds }
    }

    /// The host path of an absolute path inside the jail.
    pub fn path(&self, inside: &str) -> PathBuf {
        let relative = inside.trim_start_matches('/');
        if relative.is_empty() {
            self.root.path().to_owned()
        } else {
            self.root.path().join(relative)
        }
    }

    /// Create a directory inside the jail with `mode`, and its parents 0755.
    pub fn directory(&self, inside: &str, mode: u32) -> PathBuf {
        let path = self.path(inside);
        fs::create_dir_all(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        path
    }

    /// Bind `source` onto `target` read-only, both inside the jail. Changes the
    /// test makes under `source` from the host stay visible through `target`.
    pub fn read_only(&mut self, source: &str, target: &str) {
        let (source, target) = (self.path(source), self.path(target));
        assert!(source.is_dir() && target.is_dir(), "both ends exist");
        self.binds.push((source, target, true));
    }

    /// Bind `source` onto `target` writable, both inside the jail. `target`
    /// may lie under an earlier read-only bind: it is resolved when the child
    /// mounts it.
    pub fn read_write(&mut self, source: &str, target: &str) {
        let (source, target) = (self.path(source), self.path(target));
        assert!(source.is_dir(), "the source exists");
        self.binds.push((source, target, false));
    }

    /// Copy an executable into the jail, owner-only.
    pub fn install(&self, source: &Path, inside: &str) -> PathBuf {
        let destination = self.path(inside);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::copy(source, &destination).unwrap();
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o700)).unwrap();
        destination
    }

    /// A command for `program`, a path inside the jail, that runs in the jail
    /// under `map`. The caller sets its arguments and environment.
    pub fn command(&self, program: &str, map: Map) -> Command {
        // SAFETY: geteuid and getegid have no preconditions.
        let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
        let inner = |id: u32| if map == Map::Root { 0 } else { id };
        let uid_map = format!("{} {uid} 1", inner(uid)).into_bytes();
        let gid_map = format!("{} {gid} 1", inner(gid)).into_bytes();
        let c = |path: &Path| CString::new(path.as_os_str().as_bytes()).unwrap();
        let binds = self
            .binds
            .iter()
            .map(|(source, target, read_only)| Bind {
                source: c(source),
                target: c(target),
                read_only: *read_only,
            })
            .collect::<Vec<_>>();
        let root = c(self.root.path());
        let mut command = Command::new(program);
        // SAFETY: the closure makes system calls only, on data prepared above;
        // it allocates nothing between fork and exec.
        unsafe {
            command.pre_exec(move || enter(&uid_map, &gid_map, &binds, &root));
        }
        command
    }

    /// Why a jailed child failed to start, naming the kernel prerequisite.
    pub fn spawn_error(error: &std::io::Error) -> String {
        format!(
            "a jailed child could not enter its user and mount namespaces ({error}): \
             these tests need unprivileged user namespaces with capabilities \
             (kernel.unprivileged_userns_clone = 1, and on Ubuntu \
             kernel.apparmor_restrict_unprivileged_userns = 0)"
        )
    }
}

fn check(result: libc::c_int) -> std::io::Result<()> {
    if result < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn write_file(path: &[u8], bytes: &[u8]) -> std::io::Result<()> {
    // SAFETY: path is NUL-terminated; the descriptor is closed before return.
    unsafe {
        let fd = libc::open(path.as_ptr().cast(), libc::O_WRONLY | libc::O_CLOEXEC);
        check(fd)?;
        let written = libc::write(fd, bytes.as_ptr().cast(), bytes.len());
        libc::close(fd);
        if written != bytes.len() as isize {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

/// The locked flags a read-only remount must keep: those the bind inherited.
fn kept_flags(target: &CString) -> std::io::Result<libc::c_ulong> {
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: target is NUL-terminated and stat is storage for one statvfs.
    check(unsafe { libc::statvfs(target.as_ptr(), stat.as_mut_ptr()) })?;
    // SAFETY: statvfs succeeded.
    let flags = unsafe { stat.assume_init() }.f_flag;
    let mut kept = 0;
    for (st, ms) in [
        (libc::ST_NOSUID, libc::MS_NOSUID),
        (libc::ST_NODEV, libc::MS_NODEV),
        (libc::ST_NOEXEC, libc::MS_NOEXEC),
        (libc::ST_NOATIME, libc::MS_NOATIME),
        (libc::ST_NODIRATIME, libc::MS_NODIRATIME),
        (libc::ST_RELATIME, libc::MS_RELATIME),
    ] {
        if flags & st != 0 {
            kept |= ms;
        }
    }
    Ok(kept)
}

/// Between fork and exec: system calls only.
fn enter(uid_map: &[u8], gid_map: &[u8], binds: &[Bind], root: &CString) -> std::io::Result<()> {
    // SAFETY: each call below takes NUL-terminated strings prepared before
    // fork, or null pointers where the call admits them.
    unsafe {
        check(libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNS))?;
        write_file(b"/proc/self/setgroups\0", b"deny")?;
        write_file(b"/proc/self/uid_map\0", uid_map)?;
        write_file(b"/proc/self/gid_map\0", gid_map)?;
        check(libc::mount(
            std::ptr::null(),
            c"/".as_ptr(),
            std::ptr::null(),
            libc::MS_REC | libc::MS_PRIVATE,
            std::ptr::null(),
        ))?;
        for bind in binds {
            check(libc::mount(
                bind.source.as_ptr(),
                bind.target.as_ptr(),
                std::ptr::null(),
                libc::MS_BIND | libc::MS_REC,
                std::ptr::null(),
            ))?;
            if bind.read_only {
                let kept = kept_flags(&bind.target)?;
                check(libc::mount(
                    std::ptr::null(),
                    bind.target.as_ptr(),
                    std::ptr::null(),
                    libc::MS_BIND | libc::MS_REMOUNT | libc::MS_RDONLY | kept,
                    std::ptr::null(),
                ))?;
            }
        }
        check(libc::chroot(root.as_ptr()))?;
        check(libc::chdir(c"/".as_ptr()))?;
    }
    Ok(())
}

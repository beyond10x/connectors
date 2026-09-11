//! Protected local entry. Secret buffers bypass ordinary JSON value allocations.
use super::{
    filesystem as fs,
    owner::{Code, Error, Result},
    runtime::EntryField,
};

#[cfg(test)]
mod tests;
use connectors_sdk::Secret;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt},
        },
    },
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
extern "C" fn interrupt(_: i32) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}
pub struct Signals {
    old_int: libc::sigaction,
    old_term: libc::sigaction,
}
impl Signals {
    pub fn install() -> Result<Self> {
        if ACTIVE
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(Code::Unavailable.into());
        }
        INTERRUPTED.store(false, Ordering::SeqCst);
        // SAFETY: sigaction structures are initialized and live for each syscall.
        let (mut action, mut old_int, mut old_term) = unsafe {
            (
                std::mem::zeroed::<libc::sigaction>(),
                std::mem::zeroed::<libc::sigaction>(),
                std::mem::zeroed::<libc::sigaction>(),
            )
        };
        action.sa_sigaction = interrupt as *const () as usize;
        // No SA_RESTART: bounded reads must wake and restore terminal state.
        unsafe {
            libc::sigemptyset(&mut action.sa_mask);
        }
        if unsafe { libc::sigaction(libc::SIGINT, &action, &mut old_int) } != 0 {
            ACTIVE.store(false, Ordering::SeqCst);
            return Err(Code::Unavailable.into());
        }
        if unsafe { libc::sigaction(libc::SIGTERM, &action, &mut old_term) } != 0 {
            unsafe {
                libc::sigaction(libc::SIGINT, &old_int, std::ptr::null_mut());
            }
            ACTIVE.store(false, Ordering::SeqCst);
            return Err(Code::Unavailable.into());
        }
        Ok(Self { old_int, old_term })
    }
    pub fn interrupted(&self) -> bool {
        INTERRUPTED.load(Ordering::SeqCst)
    }
}
impl Drop for Signals {
    fn drop(&mut self) {
        // SAFETY: these are the saved handlers installed by this exclusive guard.
        unsafe {
            libc::sigaction(libc::SIGINT, &self.old_int, std::ptr::null_mut());
            libc::sigaction(libc::SIGTERM, &self.old_term, std::ptr::null_mut());
        }
        ACTIVE.store(false, Ordering::SeqCst);
    }
}
fn check(deadline: u64) -> Result<()> {
    if INTERRUPTED.load(Ordering::SeqCst) {
        Err(Code::Interrupted.into())
    } else if connectors_sdk::now_ms() >= deadline {
        Err(Code::Timeout.into())
    } else {
        Ok(())
    }
}
pub(crate) fn cancellation() -> Result<()> {
    if ACTIVE.load(Ordering::SeqCst) && INTERRUPTED.load(Ordering::SeqCst) {
        Err(Code::Interrupted.into())
    } else {
        Ok(())
    }
}
fn source_error(_: impl Sized) -> Error {
    Code::InvalidInput.into()
}
fn duplicate(fd: i32) -> Result<File> {
    // SAFETY: fcntl validates the source and returns a new owned descriptor.
    let raw = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 10) };
    if raw < 0 {
        return Err(Code::InvalidInput.into());
    }
    Ok(unsafe { File::from_raw_fd(raw) })
}
fn private(file: &File) -> Result<()> {
    fs::check_private_file(file).map_err(source_error)?;
    if !matches!(
        file.metadata().map_err(source_error)?.mode() & 0o777,
        0o400 | 0o600
    ) {
        return Err(Code::InvalidInput.into());
    }
    Ok(())
}
pub fn file(path: &Path, deadline: u64) -> Result<Secret> {
    let file = fs::private_file(path).map_err(source_error)?;
    private(&file)?;
    read(file, 65536, deadline)
}
/// Bounded protected proof acquisition under the invocation's original budget.
/// Only an owner-private regular file is admitted; buffers zeroize on all exits.
pub fn file_until(path: &Path, until: std::time::Instant, limit: usize) -> Result<Secret> {
    if limit > 65536 {
        return Err(Code::InvalidInput.into());
    }
    let check = || -> Result<()> {
        cancellation()?;
        if std::time::Instant::now() >= until {
            return Err(Code::Timeout.into());
        }
        Ok(())
    };
    check()?;
    let mut file = fs::private_file(path).map_err(source_error)?;
    private(&file)?;
    if file.metadata().map_err(source_error)?.len() > limit as u64 {
        return Err(Code::InvalidInput.into());
    }
    let mut bytes = Secret(Vec::with_capacity(limit + 1));
    let mut buffer = Secret(vec![0; 4096]);
    loop {
        check()?;
        let remaining = (limit + 1 - bytes.0.len()).min(buffer.0.len());
        let count = file
            .read(&mut buffer.0[..remaining])
            .map_err(source_error)?;
        if count == 0 {
            break;
        }
        bytes.0.extend_from_slice(&buffer.0[..count]);
        if bytes.0.len() > limit {
            return Err(Code::InvalidInput.into());
        }
    }
    check()?;
    Ok(bytes)
}
pub fn stdin(deadline: u64) -> Result<Secret> {
    let file = duplicate(0)?;
    let info = file.metadata().map_err(source_error)?;
    if info.is_file() {
        private(&file)?;
        let target = std::fs::read_link("/proc/self/fd/0").map_err(source_error)?;
        fs::directory(target.parent().ok_or(Code::InvalidInput)?, false, false)
            .map_err(source_error)?;
    } else {
        let target = std::fs::read_link("/proc/self/fd/0").map_err(source_error)?;
        let name = target.as_os_str().as_bytes();
        // A deliberate anonymous pipe within the same owner/session. fstat does
        // not authenticate an arbitrary producer; the trusted launcher owns it.
        if info.mode() & libc::S_IFMT != libc::S_IFIFO
            || info.uid() != fs::uid()
            || !name.starts_with(b"pipe:[")
            || !name.ends_with(b"]")
        {
            return Err(Code::InvalidInput.into());
        }
        let parent = unsafe { libc::getppid() };
        if parent <= 1
            || unsafe { libc::getsid(parent) } != unsafe { libc::getsid(0) }
            || std::fs::metadata(format!("/proc/{parent}"))
                .map_err(source_error)?
                .uid()
                != fs::uid()
        {
            return Err(Code::InvalidInput.into());
        }
    }
    read(file, 65536, deadline)
}
fn wait(file: &File, deadline: u64) -> Result<()> {
    loop {
        check(deadline)?;
        let ms = deadline.saturating_sub(connectors_sdk::now_ms()).min(1000) as i32;
        let mut poll = libc::pollfd {
            fd: file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one initialized pollfd held across the bounded syscall.
        let result = unsafe { libc::poll(&mut poll, 1, ms.max(1)) };
        if result < 0 {
            check(deadline)?;
            return Err(Code::InvalidInput.into());
        }
        if result > 0 {
            return Ok(());
        }
    }
}
fn read(mut file: File, limit: usize, deadline: u64) -> Result<Secret> {
    let info = file.metadata().map_err(source_error)?;
    if info.is_file() && info.len() > limit as u64 {
        return Err(Code::InvalidInput.into());
    }
    let mut data = Secret(Vec::with_capacity(limit + 1));
    let mut buffer = Secret(vec![0; 4096]);
    loop {
        wait(&file, deadline)?;
        let remaining = (limit + 1 - data.0.len()).min(buffer.0.len());
        let count = file.read(&mut buffer.0[..remaining]).map_err(|e| {
            if e.kind() == std::io::ErrorKind::Interrupted {
                Code::Interrupted.into()
            } else {
                source_error(e)
            }
        })?;
        if count == 0 {
            break;
        }
        data.0.extend_from_slice(&buffer.0[..count]);
        if data.0.len() > limit {
            return Err(Code::InvalidInput.into());
        }
    }
    check(deadline)?;
    Ok(data)
}
fn document_file(path: Option<&Path>) -> Result<File> {
    if let Some(path) = path {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(path)
            .map_err(source_error)?;
        if !file.metadata().map_err(source_error)?.is_file() {
            return Err(Code::InvalidInput.into());
        }
        Ok(file)
    } else {
        duplicate(0)
    }
}
pub fn document(path: Option<&Path>) -> Result<String> {
    let mut bytes = read(
        document_file(path)?,
        1024 * 1024,
        connectors_sdk::now_ms() + 30_000,
    )?;
    String::from_utf8(std::mem::take(&mut bytes.0)).map_err(source_error)
}

/// Nonsecret document acquisition within an original monotonic helper budget.
/// Time spent parsing arguments, waiting for stdin and reading a file is shared
/// with subsequent approval resolution and issuance; no phase restarts it.
pub fn document_until(
    path: Option<&Path>,
    until: std::time::Instant,
    limit: usize,
) -> Result<String> {
    if limit > 1024 * 1024 {
        return Err(Code::InvalidInput.into());
    }
    let check = || -> Result<()> {
        cancellation()?;
        if std::time::Instant::now() >= until {
            return Err(Code::Timeout.into());
        }
        Ok(())
    };
    check()?;
    let mut file = document_file(path)?;
    if file.metadata().map_err(source_error)?.len() > limit as u64 {
        return Err(Code::InvalidInput.into());
    }
    let mut bytes = Vec::with_capacity(limit + 1);
    let mut buffer = [0; 4096];
    loop {
        check()?;
        let ms = until
            .saturating_duration_since(std::time::Instant::now())
            .as_millis()
            .min(100) as i32;
        let mut poll = libc::pollfd {
            fd: file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: a live descriptor and one initialized pollfd; bounded wait.
        let ready = unsafe { libc::poll(&mut poll, 1, ms.max(1)) };
        check()?;
        if ready < 0 {
            return Err(Code::InvalidInput.into());
        }
        if ready == 0 {
            continue;
        }
        let remaining = (limit + 1 - bytes.len()).min(buffer.len());
        let count = file.read(&mut buffer[..remaining]).map_err(source_error)?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
        if bytes.len() > limit {
            return Err(Code::InvalidInput.into());
        }
    }
    check()?;
    String::from_utf8(bytes).map_err(source_error)
}
struct Terminal {
    file: File,
    original: libc::termios,
}
impl Drop for Terminal {
    fn drop(&mut self) {
        // SAFETY: original termios came from this held controlling terminal.
        while unsafe { libc::tcsetattr(self.file.as_raw_fd(), libc::TCSANOW, &self.original) } != 0
        {
            if std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
                break;
            }
        }
    }
}
pub fn terminal(fields: &[EntryField], deadline: u64) -> Result<Secret> {
    if fields.is_empty() || fields.len() > 16 {
        return Err(Code::InvalidInput.into());
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOCTTY | libc::O_NONBLOCK)
        .open("/dev/tty")
        .map_err(source_error)?;
    let fd = file.as_raw_fd();
    // SAFETY: scalar terminal/session queries. The current process must own its
    // foreground controlling session; no supplied terminal descriptor is used.
    if unsafe { libc::getuid() } != fs::uid()
        || unsafe { libc::tcgetsid(fd) } != unsafe { libc::getsid(0) }
        || unsafe { libc::tcgetpgrp(fd) } != unsafe { libc::getpgrp() }
    {
        return Err(Code::InvalidInput.into());
    }
    let mut original = unsafe { std::mem::zeroed::<libc::termios>() };
    if unsafe { libc::tcgetattr(fd, &mut original) } != 0 {
        return Err(Code::InvalidInput.into());
    }
    let mut terminal = Terminal { file, original };
    let mut mode = original;
    mode.c_lflag &= !(libc::ECHO | libc::ECHONL | libc::ICANON);
    mode.c_cc[libc::VMIN] = 1;
    mode.c_cc[libc::VTIME] = 0;
    if unsafe { libc::tcsetattr(fd, libc::TCSAFLUSH, &mode) } != 0 {
        return Err(Code::InvalidInput.into());
    }
    let mut document = Secret(Vec::with_capacity(65536));
    append(&mut document, b"{")?;
    for (index, field) in fields.iter().enumerate() {
        if !connectors_core::valid_id(&field.name)
            || field.label.is_empty()
            || field.label.len() > 128
            || field.label.chars().any(char::is_control)
            || !(1..=65536).contains(&field.max_bytes)
        {
            return Err(Code::InvalidInput.into());
        }
        terminal
            .file
            .write_all(field.label.as_bytes())
            .and_then(|_| terminal.file.write_all(b": "))
            .map_err(source_error)?;
        let mut value = Secret(Vec::with_capacity(field.max_bytes as usize));
        let mut byte = Secret(vec![0]);
        loop {
            wait(&terminal.file, deadline)?;
            let count = terminal.file.read(&mut byte.0).map_err(|e| {
                if e.kind() == std::io::ErrorKind::Interrupted {
                    Code::Interrupted.into()
                } else {
                    source_error(e)
                }
            })?;
            if count == 0 {
                return Err(Code::InvalidInput.into());
            }
            match byte.0[0] {
                b'\n' | b'\r' => break,
                4 => return Err(Code::Interrupted.into()),
                8 | 127 => {
                    while let Some(last) = value.0.last_mut() {
                        let b = *last;
                        *last = 0;
                        value.0.pop();
                        if b & 0b1100_0000 != 0b1000_0000 {
                            break;
                        }
                    }
                }
                b => {
                    if value.0.len() >= field.max_bytes as usize {
                        return Err(Code::InvalidInput.into());
                    }
                    value.0.push(b);
                }
            }
        }
        terminal.file.write_all(b"\n").map_err(source_error)?;
        if index > 0 {
            append(&mut document, b",")?;
        }
        serde_json::to_writer(Bounded(&mut document), &field.name).map_err(source_error)?;
        append(&mut document, b":")?;
        let value = std::str::from_utf8(&value.0).map_err(source_error)?;
        serde_json::to_writer(Bounded(&mut document), value).map_err(source_error)?;
    }
    append(&mut document, b"}")?;
    check(deadline)?;
    Ok(document)
}
fn append(target: &mut Secret, bytes: &[u8]) -> Result<()> {
    if target.0.len() + bytes.len() > 65536 {
        return Err(Code::InvalidInput.into());
    }
    target.0.extend_from_slice(bytes);
    Ok(())
}
struct Bounded<'a>(&'a mut Secret);
impl Write for Bounded<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        append(self.0, bytes).map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

//! Windows protected entry (contracts/cli/v1alpha1/semantics.md, "Windows
//! platform guarantees"): console control events in place of SIGINT and
//! SIGTERM, console mode in place of termios, and protected stdin limited to an
//! owner-only regular file.
use super::{ACTIVE, Code, INTERRUPTED, Result, check, fs, source_error};
use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::Read,
    os::windows::io::{AsHandle, AsRawHandle},
    sync::atomic::Ordering,
};
use windows_sys::Win32::{
    Foundation::{ERROR_BROKEN_PIPE, HANDLE, WAIT_OBJECT_0},
    Storage::FileSystem::{FILE_TYPE_CHAR, FILE_TYPE_DISK, FILE_TYPE_PIPE, GetFileType},
    System::{
        Console::{
            CONSOLE_MODE, CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT, ENABLE_ECHO_INPUT,
            ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, FlushConsoleInputBuffer, GetConsoleMode,
            INPUT_RECORD, KEY_EVENT, ReadConsoleInputW, SetConsoleCtrlHandler, SetConsoleMode,
            WriteConsoleW,
        },
        Pipes::PeekNamedPipe,
        Threading::WaitForSingleObject,
    },
};

fn raw(file: &File) -> HANDLE {
    file.as_raw_handle() as HANDLE
}

unsafe extern "system" fn interrupt(event: u32) -> windows_sys::core::BOOL {
    if matches!(event, CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT) {
        INTERRUPTED.store(true, Ordering::SeqCst);
        1
    } else {
        0
    }
}

/// Console control events stand in for SIGINT and SIGTERM: Ctrl+C, Ctrl+Break
/// and closing the console mark the capture interrupted.
pub struct Signals(());
impl Signals {
    pub fn install() -> Result<Self> {
        if ACTIVE
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(Code::Unavailable.into());
        }
        INTERRUPTED.store(false, Ordering::SeqCst);
        // SAFETY: a static handler routine, added once and removed on drop.
        if unsafe { SetConsoleCtrlHandler(Some(interrupt), 1) } == 0 {
            ACTIVE.store(false, Ordering::SeqCst);
            return Err(Code::Unavailable.into());
        }
        Ok(Self(()))
    }
    pub fn interrupted(&self) -> bool {
        INTERRUPTED.load(Ordering::SeqCst)
    }
}
impl Drop for Signals {
    fn drop(&mut self) {
        // SAFETY: removes the routine this exclusive guard added.
        unsafe { SetConsoleCtrlHandler(Some(interrupt), 0) };
        ACTIVE.store(false, Ordering::SeqCst);
    }
}

/// A non-inheritable duplicate of standard input; there is no other inherited
/// descriptor to read.
pub(super) fn duplicate(fd: i32) -> Result<File> {
    if fd != 0 {
        return Err(Code::InvalidInput.into());
    }
    let handle = std::io::stdin()
        .as_handle()
        .try_clone_to_owned()
        .map_err(source_error)?;
    let file = File::from(handle);
    // SAFETY: a live handle.
    if unsafe { GetFileType(raw(&file)) } == 0 {
        return Err(Code::InvalidInput.into());
    }
    Ok(file)
}

/// Protected stdin on Windows admits an owner-only regular file whose parent
/// directories are admitted. A pipe is refused until a producer check is
/// specified for it, and so are a console and every other handle type.
pub(super) fn admit_stdin(file: &File) -> Result<()> {
    // SAFETY: a live handle.
    if unsafe { GetFileType(raw(file)) } != FILE_TYPE_DISK
        || !file.metadata().map_err(source_error)?.is_file()
    {
        return Err(Code::InvalidInput.into());
    }
    fs::check_private_file(file).map_err(source_error)?;
    let target = fs::windows::final_path(file).map_err(source_error)?;
    fs::directory(target.parent().ok_or(Code::InvalidInput)?, false, false)
        .map_err(source_error)?;
    Ok(())
}

/// Whether a read would not wait, within `ms`. A pipe reports its queued
/// bytes or its closed writer; a console its input events; a disk file is
/// always ready.
pub(super) fn ready(file: &File, ms: u32) -> Result<bool> {
    // SAFETY: a live handle.
    match unsafe { GetFileType(raw(file)) } {
        FILE_TYPE_PIPE => {
            let mut available = 0u32;
            // SAFETY: a live handle; only the count is written.
            let ok = unsafe {
                PeekNamedPipe(
                    raw(file),
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut available,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                // A closed writer reads as end of input.
                return if std::io::Error::last_os_error().raw_os_error()
                    == Some(ERROR_BROKEN_PIPE as i32)
                {
                    Ok(true)
                } else {
                    Err(Code::InvalidInput.into())
                };
            }
            if available > 0 {
                return Ok(true);
            }
            std::thread::sleep(std::time::Duration::from_millis(u64::from(ms.min(10))));
            Ok(false)
        }
        // SAFETY: a live waitable console handle and a bounded wait.
        FILE_TYPE_CHAR => Ok(unsafe { WaitForSingleObject(raw(file), ms.max(1)) } == WAIT_OBJECT_0),
        _ => Ok(true),
    }
}

fn wait(file: &File, deadline: u64) -> Result<()> {
    loop {
        check(deadline)?;
        let ms = deadline.saturating_sub(connectors_sdk::now_ms()).min(1000) as u32;
        if ready(file, ms.max(1))? {
            return Ok(());
        }
    }
}

/// Reads once `wait` reports input; an empty read on a pipe is end of input.
pub(super) fn read_ready(file: &mut File, buffer: &mut [u8], deadline: u64) -> Result<usize> {
    loop {
        wait(file, deadline)?;
        match file.read(buffer) {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                return Err(Code::Interrupted.into());
            }
            result => return result.map_err(source_error),
        }
    }
}

/// A business document named by path: a regular file, opened without
/// following a final reparse point.
pub(super) fn document_file(path: &std::path::Path) -> Result<File> {
    let file = fs::windows::open_no_follow(path).map_err(source_error)?;
    if !file.metadata().map_err(source_error)?.is_file() {
        return Err(Code::InvalidInput.into());
    }
    Ok(file)
}

/// The process's console, with echo and line input off until drop restores
/// the saved mode, on success, failure and interruption alike.
pub(super) struct Terminal {
    input: File,
    output: File,
    pub(super) original: CONSOLE_MODE,
    pending: VecDeque<u8>,
    high: Option<u16>,
}
impl Drop for Terminal {
    fn drop(&mut self) {
        // SAFETY: the saved mode of this held console input handle.
        unsafe { SetConsoleMode(raw(&self.input), self.original) };
        self.pending.iter_mut().for_each(|byte| *byte = 0);
    }
}
impl Terminal {
    /// A process with no console has no trusted terminal and refuses.
    pub(super) fn open() -> Result<Self> {
        let input = OpenOptions::new()
            .read(true)
            .write(true)
            .open("CONIN$")
            .map_err(source_error)?;
        let output = OpenOptions::new()
            .write(true)
            .open("CONOUT$")
            .map_err(source_error)?;
        let mut original: CONSOLE_MODE = 0;
        // SAFETY: a live console input handle and a writable mode.
        if unsafe { GetConsoleMode(raw(&input), &mut original) } == 0 {
            return Err(Code::InvalidInput.into());
        }
        let terminal = Self {
            input,
            output,
            original,
            pending: VecDeque::new(),
            high: None,
        };
        let mode = (original & !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT)) | ENABLE_PROCESSED_INPUT;
        // SAFETY: a live console input handle. Pending input typed before the
        // prompt is discarded, as TCSAFLUSH discards it.
        if unsafe { SetConsoleMode(raw(&terminal.input), mode) } == 0
            || unsafe { FlushConsoleInputBuffer(raw(&terminal.input)) } == 0
        {
            return Err(Code::InvalidInput.into());
        }
        Ok(terminal)
    }

    /// The console mode in effect, for tests.
    #[cfg(test)]
    pub(super) fn mode(&self) -> CONSOLE_MODE {
        let mut mode = 0;
        // SAFETY: a live console input handle and a writable mode.
        unsafe { GetConsoleMode(raw(&self.input), &mut mode) };
        mode
    }

    pub(super) fn write_all(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?;
        let wide: Vec<u16> = text.encode_utf16().collect();
        let mut offset = 0;
        while offset < wide.len() {
            let mut written = 0u32;
            // SAFETY: a live console output handle and a slice of the length passed.
            if unsafe {
                WriteConsoleW(
                    raw(&self.output),
                    wide[offset..].as_ptr(),
                    (wide.len() - offset) as u32,
                    &mut written,
                    std::ptr::null(),
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            offset += written as usize;
        }
        Ok(())
    }

    /// One UTF-8 byte of what was typed. Key-down characters are read as
    /// console input records, never echoed; Enter arrives as `\r`.
    pub(super) fn read_byte(&mut self, buffer: &mut [u8], deadline: u64) -> Result<usize> {
        loop {
            if let Some(byte) = self.pending.pop_front() {
                buffer[0] = byte;
                return Ok(1);
            }
            wait(&self.input, deadline)?;
            // SAFETY: zero is a valid INPUT_RECORD.
            let mut record: INPUT_RECORD = unsafe { std::mem::zeroed() };
            let mut count = 0u32;
            // SAFETY: a signalled console input handle and one writable record.
            if unsafe { ReadConsoleInputW(raw(&self.input), &mut record, 1, &mut count) } == 0 {
                return Err(Code::InvalidInput.into());
            }
            if count != 1 || u32::from(record.EventType) != KEY_EVENT {
                continue;
            }
            // SAFETY: EventType names the KeyEvent member.
            let key = unsafe { record.Event.KeyEvent };
            // SAFETY: UnicodeChar is the member ReadConsoleInputW fills.
            let unit = unsafe { key.uChar.UnicodeChar };
            if key.bKeyDown == 0 || unit == 0 {
                continue;
            }
            let character = match (self.high.take(), unit) {
                (None, 0xD800..=0xDBFF) => {
                    self.high = Some(unit);
                    continue;
                }
                (Some(high), 0xDC00..=0xDFFF) => char::decode_utf16([high, unit]).next(),
                (_, unit) => char::decode_utf16([unit]).next(),
            };
            let Some(Ok(character)) = character else {
                return Err(Code::InvalidInput.into());
            };
            let mut encoded = [0u8; 4];
            for _ in 0..key.wRepeatCount.max(1) {
                self.pending
                    .extend(character.encode_utf8(&mut encoded).as_bytes());
            }
            encoded.iter_mut().for_each(|byte| *byte = 0);
        }
    }
}

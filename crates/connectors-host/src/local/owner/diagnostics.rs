//! The owner's diagnostic log: `owner.log` in the private state directory.
//!
//! The CLI starts the owner with its standard streams discarded, so the cause
//! of a failure behind `outcome_unknown` used to be lost. The owner writes one
//! line per failure here instead, and the CLI names this file, and the last
//! code the owner logged since it connected, in the failure's data.
//!
//! A line is `<unix_ms> pid=<pid> <event> code=<code>`. [`record`] takes a
//! fixed event name and a [`Code`], never text from a request, a provider or a
//! credential, so no secret can reach the log.
//!
//! Bounded by truncation, not rotation: one file keeps one stable path to name
//! in an error, and a panic or a refusal is always the newest line. When a line
//! would pass [`LIMIT`] bytes the file is emptied first and says so.
//!
//! The file is created `0600` on Unix; the state directory is already admitted
//! as private (`0700`, own user) before the owner or the client opens it. On
//! Windows the file takes the DACL of that owner-only directory, which is what
//! admits every other state file there, and has no mode to set.
use super::Code;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

/// The file name in the state directory.
pub const NAME: &str = "owner.log";
/// The most bytes the log holds.
pub const LIMIT: u64 = 256 * 1024;

/// The log of the owner this process last connected to, and its length then.
static WATCHED: Mutex<Option<(PathBuf, u64)>> = Mutex::new(None);
/// Serializes this process's writers, so a truncation never splits a line.
static WRITING: Mutex<()> = Mutex::new(());

/// The log of the owner of `state`.
#[must_use]
pub fn path(state: &Path) -> PathBuf {
    state.join(NAME)
}

fn code_name(code: Code) -> String {
    serde_json::to_string(&code)
        .map(|name| name.trim_matches('"').to_owned())
        .unwrap_or_else(|_| "unknown".to_owned())
}

fn open(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).append(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let info = file.metadata()?;
        if !info.is_file() || info.uid() != super::super::filesystem::uid() {
            return Err(std::io::ErrorKind::PermissionDenied.into());
        }
        if info.mode() & 0o077 != 0 {
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
    }
    Ok(file)
}

fn write_line(state: &Path, line: &str) {
    let _writing = WRITING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Ok(mut file) = open(&path(state)) else {
        return;
    };
    let mut text = String::new();
    let held = file.metadata().map(|info| info.len()).unwrap_or(0);
    if held + line.len() as u64 > LIMIT {
        if file.set_len(0).is_err() {
            return;
        }
        text.push_str(&format!("{} log emptied at {LIMIT} bytes\n", now()));
    }
    text.push_str(line);
    let _ = file.write_all(text.as_bytes());
}

fn now() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis())
}

/// Logs that `event` ended with `code`. Best effort: a log that cannot be
/// written never changes what the owner answers.
pub fn record(state: &Path, event: &'static str, code: Code) {
    write_line(
        state,
        &format!(
            "{} pid={} {event} code={}\n",
            now(),
            std::process::id(),
            code_name(code)
        ),
    );
}

/// Logs a refusal to start, only into a state directory admitted as private:
/// an owner that refused it must not write there.
pub fn record_refusal(state: &Path, code: Code) {
    if super::super::filesystem::directory(state, false, true).is_ok() {
        record(state, "owner refused", code);
    }
}

/// Logs a panic's source location, never its message.
pub fn record_panic(state: &Path, location: Option<&std::panic::Location<'_>>) {
    let at = location.map_or_else(
        || "unknown".to_owned(),
        |at| format!("{}:{}", at.file(), at.line()),
    );
    write_line(
        state,
        &format!("{} pid={} panic at={at}\n", now(), std::process::id()),
    );
}

/// Logs the panics of this process, then runs the hook that was set.
pub(super) fn log_panics(state: PathBuf) {
    let earlier = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        record_panic(&state, info.location());
        earlier(info);
    }));
}

/// Notes the log of `state` and its length, before this process talks to the
/// owner: what the owner logs from here is what [`attach`] reads.
pub(super) fn watch(state: &Path) {
    let path = path(state);
    let length = std::fs::symlink_metadata(&path).map_or(0, |info| info.len());
    *WATCHED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((path, length));
}

/// The last `code=` of the lines at or after `offset`; a file shorter than
/// `offset` was emptied since, and is read whole.
fn last_code(path: &Path, offset: u64) -> Option<String> {
    let info = std::fs::symlink_metadata(path).ok()?;
    if !info.is_file() {
        return None;
    }
    let mut file = File::open(path).ok()?;
    let offset = if info.len() < offset { 0 } else { offset };
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut text = String::new();
    file.take(LIMIT).read_to_string(&mut text).ok()?;
    text.lines().rev().find_map(|line| {
        let code = line.rsplit_once(" code=")?.1;
        (!code.is_empty() && code.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'))
            .then(|| code.to_owned())
    })
}

/// For an `outcome_unknown`, the owner's log that exists and the code the owner
/// logged since this process connected to it.
pub fn observed(code: Code) -> Option<super::OwnerLog> {
    if code != Code::OutcomeUnknown {
        return None;
    }
    let (path, offset) = WATCHED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()?;
    if !std::fs::symlink_metadata(&path).is_ok_and(|info| info.is_file()) {
        return None;
    }
    Some(super::OwnerLog {
        code: last_code(&path, offset),
        path: path.to_string_lossy().into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn a_line_names_the_event_and_the_code() {
        let state = state();
        record(state.path(), "request failed", Code::Unavailable);
        let text = std::fs::read_to_string(path(state.path())).unwrap();
        assert!(
            text.ends_with(" request failed code=unavailable\n"),
            "{text}"
        );
        assert_eq!(
            last_code(&path(state.path()), 0).as_deref(),
            Some("unavailable")
        );
    }

    #[cfg(unix)]
    #[test]
    fn the_log_is_owner_only_even_where_a_wider_file_was_there() {
        use std::os::unix::fs::PermissionsExt;
        let state = state();
        let file = path(state.path());
        std::fs::write(&file, b"").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
        record(state.path(), "request failed", Code::Timeout);
        let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn the_log_never_passes_its_limit_and_keeps_the_newest_line() {
        let state = state();
        for _ in 0..(LIMIT as usize / 40) + 100 {
            record(state.path(), "request failed", Code::Capacity);
        }
        record(state.path(), "request failed", Code::Timeout);
        let file = path(state.path());
        assert!(std::fs::metadata(&file).unwrap().len() <= LIMIT);
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("log emptied"), "{text}");
        assert_eq!(last_code(&file, 0).as_deref(), Some("timeout"));
    }

    #[test]
    fn only_lines_after_the_offset_are_read() {
        let state = state();
        record(state.path(), "request failed", Code::Capacity);
        let offset = std::fs::metadata(path(state.path())).unwrap().len();
        assert_eq!(last_code(&path(state.path()), offset), None);
        record(state.path(), "request failed", Code::Timeout);
        assert_eq!(
            last_code(&path(state.path()), offset).as_deref(),
            Some("timeout")
        );
    }

    #[test]
    fn a_panic_logs_where_and_never_what() {
        let state = state();
        record_panic(state.path(), Some(std::panic::Location::caller()));
        let text = std::fs::read_to_string(path(state.path())).unwrap();
        assert!(
            text.contains(" panic at=") && !text.contains("code="),
            "{text}"
        );
    }
}

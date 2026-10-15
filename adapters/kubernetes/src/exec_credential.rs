//! The pinned exec credential plugin (`connectors_kubernetes.exec_credentials`
//! in the adapter's ESS model; `contracts/reads/v1alpha1/semantics.md`,
//! "Exec-plugin credentials"). A kubeconfig user that authenticates through an
//! `exec` block gets its bearer token from a plugin this module runs, under
//! `decision-blocker:kubernetes-exec-credential-plugin` (option b):
//!
//! - the plugin is the one a connection's configuration names by absolute path
//!   and sha256, verified before every run;
//! - its arguments come from the kubeconfig as data, never through a shell;
//! - it starts from an empty environment and receives only `HOME` (when
//!   configured), `KUBERNETES_EXEC_INFO` and the allow-listed names;
//! - one run is bounded in time and in output, and a refusal kills its whole
//!   process group;
//! - the token is held in memory until its stated expiry and never written
//!   anywhere.
//!
//! The composition decides when a run may happen (only inside an admitted
//! validation or operation) and reads the kubeconfig; this module owns the run
//! and the interpretation of its answer.
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Mutex, mpsc},
    time::{Duration, Instant},
};
use zeroize::{Zeroize, Zeroizing};

pub const PROFILE_ID: &str = "kubernetes.exec_plugin";
/// The most stdout one run may write; more is `output_too_large`.
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;
pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;
pub const MAX_TIMEOUT_MS: u64 = 60_000;
pub const MAX_ENV: usize = 32;
pub const MAX_ARGS: usize = 64;
const MAX_ARG_BYTES: usize = 4096;
const MAX_PATH_BYTES: usize = 4096;
const MAX_TOKEN_BYTES: usize = 8192;
pub const V1: &str = "client.authentication.k8s.io/v1";
pub const V1BETA1: &str = "client.authentication.k8s.io/v1beta1";

/// `connectors_kubernetes.exec_credentials.ExecPluginRefusal`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    DigestMismatch,
    PluginUnavailable,
    KubeconfigMismatch,
    Timeout,
    OutputTooLarge,
    MalformedCredential,
    PluginFailed,
}
impl Refusal {
    pub fn code(self) -> &'static str {
        match self {
            Refusal::DigestMismatch => "digest_mismatch",
            Refusal::PluginUnavailable => "plugin_unavailable",
            Refusal::KubeconfigMismatch => "kubeconfig_mismatch",
            Refusal::Timeout => "timeout",
            Refusal::OutputTooLarge => "output_too_large",
            Refusal::MalformedCredential => "malformed_credential",
            Refusal::PluginFailed => "plugin_failed",
        }
    }
}

/// `connectors_kubernetes.exec_credentials.ExecPluginBinding`: the pin a local
/// connection's configuration names. Nonsecret.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub path: PathBuf,
    pub sha256: String,
    pub user: String,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub home: Option<PathBuf>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}
fn env_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    (1..=128).contains(&name.len())
        && bytes
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && name != "HOME"
        && name != "KUBERNETES_EXEC_INFO"
}
fn absolute(path: &Path) -> bool {
    path.is_absolute()
        && path
            .to_str()
            .is_some_and(|p| (2..=MAX_PATH_BYTES).contains(&p.len()))
        && !path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
}
impl Binding {
    /// Every rule of the ESS type and its comment that a value can break.
    pub fn admissible(&self) -> bool {
        let mut names = BTreeSet::new();
        absolute(&self.path)
            && self.sha256.len() == 64
            && self
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && (1..=256).contains(&self.user.len())
            && !self.user.chars().any(char::is_control)
            && self.env.len() <= MAX_ENV
            && self.env.iter().all(|n| env_name(n) && names.insert(n))
            && self.home.as_deref().is_none_or(absolute)
            && self
                .timeout_ms
                .is_none_or(|t| (1..=MAX_TIMEOUT_MS).contains(&t))
    }
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS))
    }
    /// The canonical document the effective configuration digests: the
    /// allow-list sorted and the deadline resolved, so how the binding was
    /// written down does not change the revision.
    pub fn canonical(&self) -> Value {
        let mut env = self.env.clone();
        env.sort();
        json!({"path":self.path,"sha256":self.sha256,"user":self.user,"env":env,
            "home":self.home,"timeout_ms":self.timeout().as_millis() as u64})
    }
}

#[derive(Deserialize)]
struct KubeconfigFile {
    #[serde(default)]
    users: Option<Vec<NamedUser>>,
}
#[derive(Deserialize)]
struct NamedUser {
    name: String,
    #[serde(default)]
    user: Option<UserEntry>,
}
/// Only the exec block is read; tokens, keys and every other user setting are
/// skipped by the parser.
#[derive(Deserialize)]
struct UserEntry {
    #[serde(default)]
    exec: Option<ExecBlock>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecBlock {
    api_version: String,
    command: String,
    #[serde(default)]
    args: Option<Vec<String>>,
    #[serde(default)]
    env: Option<Vec<EnvEntry>>,
    #[serde(default)]
    interactive_mode: Option<String>,
}
#[derive(Deserialize)]
struct EnvEntry {
    name: String,
    value: String,
}

/// One run, fully decided before any process starts
/// (`connectors_kubernetes.exec_credentials.ExecPluginSelection`).
struct Prepared {
    program: PathBuf,
    args: Vec<String>,
    env: Vec<(String, String)>,
    directory: PathBuf,
    api_version: &'static str,
    timeout: Duration,
}

fn prepare(
    binding: &Binding,
    kubeconfig: &[u8],
    ambient: &dyn Fn(&str) -> Option<String>,
) -> Result<Prepared, Refusal> {
    let file: KubeconfigFile =
        serde_yaml_ng::from_slice(kubeconfig).map_err(|_| Refusal::KubeconfigMismatch)?;
    let users = file.users.unwrap_or_default();
    let mut matching = users.into_iter().filter(|u| u.name == binding.user);
    let user = matching.next().ok_or(Refusal::KubeconfigMismatch)?;
    if matching.next().is_some() {
        return Err(Refusal::KubeconfigMismatch);
    }
    let exec = user
        .user
        .and_then(|u| u.exec)
        .ok_or(Refusal::KubeconfigMismatch)?;
    let api_version = match exec.api_version.as_str() {
        V1 => V1,
        V1BETA1 => V1BETA1,
        _ => return Err(Refusal::KubeconfigMismatch),
    };
    // The plugin runs with stdin closed and no terminal.
    if exec.interactive_mode.as_deref() == Some("Always") {
        return Err(Refusal::KubeconfigMismatch);
    }
    // The kubeconfig cannot select an executable: its command is the pinned
    // path, or the bare file name of it.
    let file_name = binding.path.file_name().and_then(|n| n.to_str());
    let command = exec.command.as_str();
    if !(Path::new(command) == binding.path
        || (!command.contains('/') && Some(command) == file_name))
    {
        return Err(Refusal::KubeconfigMismatch);
    }
    let args = exec.args.unwrap_or_default();
    if args.len() > MAX_ARGS
        || args
            .iter()
            .any(|a| a.len() > MAX_ARG_BYTES || a.contains('\0'))
    {
        return Err(Refusal::KubeconfigMismatch);
    }
    let declared = exec.env.unwrap_or_default();
    let mut seen = BTreeSet::new();
    for entry in &declared {
        if !binding.env.contains(&entry.name)
            || !seen.insert(entry.name.as_str())
            || entry.value.len() > MAX_ARG_BYTES
            || entry.value.contains('\0')
        {
            return Err(Refusal::KubeconfigMismatch);
        }
    }
    let mut env = Vec::new();
    if let Some(home) = &binding.home {
        env.push((
            "HOME".to_owned(),
            home.to_str().ok_or(Refusal::PluginUnavailable)?.to_owned(),
        ));
    }
    env.push((
        "KUBERNETES_EXEC_INFO".to_owned(),
        json!({"apiVersion":api_version,"kind":"ExecCredential","spec":{"interactive":false}})
            .to_string(),
    ));
    for name in &binding.env {
        let value = declared
            .iter()
            .find(|e| &e.name == name)
            .map(|e| e.value.clone())
            .or_else(|| ambient(name));
        if let Some(value) = value {
            env.push((name.clone(), value));
        }
    }
    Ok(Prepared {
        program: binding.path.clone(),
        args,
        env,
        directory: binding.home.clone().unwrap_or_else(|| PathBuf::from("/")),
        api_version,
        timeout: binding.timeout(),
    })
}

/// The plugin file, opened once, checked and hashed through this descriptor,
/// and executed through it: the path is resolved exactly once, so the bytes
/// that ran are the bytes of the inode that was hashed.
struct Verified {
    file: std::fs::File,
    /// The file starts with `#!`. The kernel then hands the interpreter
    /// `/dev/fd/<n>`, so the descriptor must survive the exec; a native
    /// executable keeps it close-on-exec and inherits nothing.
    script: bool,
}

/// The plugin's bytes must hash to the pin, and the file must not be
/// replaceable by another user. Checked before every run, on the descriptor
/// the run executes.
#[cfg(target_os = "linux")]
fn verify(binding: &Binding) -> Result<Verified, Refusal> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    // O_NOFOLLOW: a symbolic link as the final component refuses (ELOOP);
    // O_NONBLOCK: a FIFO at the path cannot stall the open; std adds
    // O_CLOEXEC, so no concurrently spawned process inherits it.
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&binding.path)
        .map_err(|_| Refusal::PluginUnavailable)?;
    // fstat on the open descriptor, never a second lookup of the path.
    let metadata = file.metadata().map_err(|_| Refusal::PluginUnavailable)?;
    // SAFETY: geteuid has no preconditions and cannot fail.
    let uid = unsafe { libc::geteuid() };
    if !metadata.is_file()
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & (libc::S_ISUID | libc::S_ISGID) != 0
        || (metadata.uid() != 0 && metadata.uid() != uid)
    {
        return Err(Refusal::PluginUnavailable);
    }
    let mut hasher = Sha256::new();
    let mut chunk = [0u8; 64 * 1024];
    let mut head = Vec::with_capacity(2);
    loop {
        match file.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                if head.len() < 2 {
                    let take = (2 - head.len()).min(n);
                    head.extend_from_slice(&chunk[..take]);
                }
                hasher.update(&chunk[..n]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return Err(Refusal::PluginUnavailable),
        }
    }
    if hex::encode(hasher.finalize()) != binding.sha256 {
        return Err(Refusal::DigestMismatch);
    }
    // The child redirects descriptors 0 to 2 onto its stdio before it
    // executes this one, so it must not be one of them (a parent started
    // with stdio closed). try_clone duplicates to 3 or above, close-on-exec.
    if std::os::fd::AsRawFd::as_raw_fd(&file) < 3 {
        file = file.try_clone().map_err(|_| Refusal::PluginUnavailable)?;
    }
    Ok(Verified {
        file,
        script: head == b"#!",
    })
}
/// Executing a descriptor needs `fexecve`; elsewhere no plugin runs.
#[cfg(not(target_os = "linux"))]
fn verify(_binding: &Binding) -> Result<Verified, Refusal> {
    Err(Refusal::PluginUnavailable)
}

/// Kill the whole process group, then the child, and reap it.
fn end(child: &mut std::process::Child) {
    #[cfg(unix)]
    if let Ok(pid) = i32::try_from(child.id()) {
        // SAFETY: the child is unreaped, so its process group id (its own
        // pid, from process_group(0)) cannot have been reused.
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// The argument and environment vectors `fexecve` receives, built before the
/// fork: the child between fork and exec may not allocate.
#[cfg(target_os = "linux")]
struct Image {
    fd: std::os::fd::RawFd,
    script: bool,
    argv: Vec<*const libc::c_char>,
    envp: Vec<*const libc::c_char>,
    /// Owns the bytes `argv` and `envp` point into.
    _strings: Vec<std::ffi::CString>,
}
// SAFETY: the pointers point into `_strings`, which the struct owns and never
// mutates; they are only read, by the forked child.
#[cfg(target_os = "linux")]
unsafe impl Send for Image {}
#[cfg(target_os = "linux")]
unsafe impl Sync for Image {}
#[cfg(target_os = "linux")]
impl Image {
    fn new(prepared: &Prepared, verified: &Verified) -> Result<Self, Refusal> {
        use std::{ffi::CString, os::fd::AsRawFd, os::unix::ffi::OsStrExt};
        let c = |bytes: &[u8]| CString::new(bytes).map_err(|_| Refusal::KubeconfigMismatch);
        let mut strings = vec![c(prepared.program.as_os_str().as_bytes())?];
        for arg in &prepared.args {
            strings.push(c(arg.as_bytes())?);
        }
        let argc = strings.len();
        for (name, value) in &prepared.env {
            strings.push(c(format!("{name}={value}").as_bytes())?);
        }
        let pointers = |range: &[CString]| {
            range
                .iter()
                .map(|s| s.as_ptr())
                .chain(std::iter::once(std::ptr::null()))
                .collect::<Vec<_>>()
        };
        Ok(Self {
            fd: verified.file.as_raw_fd(),
            script: verified.script,
            argv: pointers(&strings[..argc]),
            envp: pointers(&strings[argc..]),
            _strings: strings,
        })
    }
    /// Runs in the forked child, after std has redirected stdio, changed the
    /// directory and set the process group. Async-signal-safe calls only.
    fn exec(&self) -> std::io::Error {
        // SAFETY: fd is the verified plugin descriptor, open in the parent and
        // so in this child; argv and envp are NUL-terminated arrays of
        // NUL-terminated strings that outlive this call.
        unsafe {
            if self.script {
                let flags = libc::fcntl(self.fd, libc::F_GETFD);
                if flags < 0 || libc::fcntl(self.fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0 {
                    return std::io::Error::last_os_error();
                }
            }
            libc::fexecve(self.fd, self.argv.as_ptr(), self.envp.as_ptr());
        }
        // fexecve returns only on failure.
        std::io::Error::last_os_error()
    }
}

#[cfg(not(target_os = "linux"))]
fn run(_prepared: &Prepared, _verified: Verified) -> Result<Zeroizing<Vec<u8>>, Refusal> {
    Err(Refusal::PluginUnavailable)
}
#[cfg(target_os = "linux")]
fn run(prepared: &Prepared, verified: Verified) -> Result<Zeroizing<Vec<u8>>, Refusal> {
    use std::os::unix::process::CommandExt;
    let image = Image::new(prepared, &verified)?;
    // The program path is never executed: the pre_exec hook replaces the
    // child with the verified descriptor, or fails the spawn. std applies the
    // empty environment, the directory, stdio and the process group first.
    let mut command = Command::new(&prepared.program);
    command
        .env_clear()
        .current_dir(&prepared.directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0);
    // SAFETY: `Image::exec` calls only fcntl and fexecve, both
    // async-signal-safe, and allocates nothing.
    unsafe {
        command.pre_exec(move || Err(image.exec()));
    }
    let spawned = command.spawn();
    // The parent's copy closes now; the child has exec'd or failed.
    drop(verified);
    let mut child = spawned.map_err(|_| Refusal::PluginUnavailable)?;
    let deadline = Instant::now() + prepared.timeout;
    let Some(mut stdout) = child.stdout.take() else {
        end(&mut child);
        return Err(Refusal::PluginFailed);
    };
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut output = Zeroizing::new(Vec::new());
        let mut chunk = Zeroizing::new([0u8; 8192]);
        let result = loop {
            match stdout.read(&mut chunk[..]) {
                Ok(0) => break Ok(std::mem::take(&mut output)),
                Ok(n) if output.len() + n > MAX_OUTPUT_BYTES => break Err(Refusal::OutputTooLarge),
                Ok(n) => output.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break Err(Refusal::PluginFailed),
            }
        };
        let _ = sender.send(result);
    });
    let mut output = None;
    let mut status = None;
    loop {
        if output.is_none() {
            match receiver.try_recv() {
                Ok(Ok(bytes)) => output = Some(bytes),
                Ok(Err(refusal)) => {
                    end(&mut child);
                    return Err(refusal);
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    end(&mut child);
                    return Err(Refusal::PluginFailed);
                }
            }
        }
        if status.is_none() {
            match child.try_wait() {
                Ok(value) => status = value,
                Err(_) => {
                    end(&mut child);
                    return Err(Refusal::PluginFailed);
                }
            }
        }
        if let (Some(_), Some(code)) = (&output, &status) {
            if !code.success() {
                return Err(Refusal::PluginFailed);
            }
            break;
        }
        if Instant::now() >= deadline {
            end(&mut child);
            return Err(Refusal::Timeout);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    output.ok_or(Refusal::PluginFailed)
}

/// An accepted ExecCredential token. Intentionally no Debug, Clone or
/// Serialize: the bytes leave only as a fresh zeroizing copy.
struct Token {
    secret: Zeroizing<Vec<u8>>,
    expires_at_ms: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Answer {
    api_version: String,
    kind: String,
    status: Option<AnswerStatus>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnswerStatus {
    #[serde(default, deserialize_with = "token")]
    token: Option<Zeroizing<String>>,
    #[serde(default)]
    expiration_timestamp: Option<String>,
}
fn token<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Zeroizing<String>>, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.map(Zeroizing::new))
}

fn parse(output: &[u8], api_version: &str) -> Result<Token, Refusal> {
    let answer: Answer =
        serde_json::from_slice(output).map_err(|_| Refusal::MalformedCredential)?;
    if answer.api_version != api_version || answer.kind != "ExecCredential" {
        return Err(Refusal::MalformedCredential);
    }
    let status = answer.status.ok_or(Refusal::MalformedCredential)?;
    let mut token = status.token.ok_or(Refusal::MalformedCredential)?;
    if token.is_empty()
        || token.len() > MAX_TOKEN_BYTES
        || !token.bytes().all(|b| b.is_ascii_graphic())
    {
        return Err(Refusal::MalformedCredential);
    }
    let expires_at_ms = match status.expiration_timestamp {
        None => None,
        Some(value) => Some(rfc3339_ms(&value).ok_or(Refusal::MalformedCredential)?),
    };
    Ok(Token {
        secret: Zeroizing::new(std::mem::take(&mut *token).into_bytes()),
        expires_at_ms,
    })
}

/// RFC 3339 date-time to Unix milliseconds: `YYYY-MM-DDTHH:MM:SS`, optional
/// fraction, then `Z` or a `±HH:MM` offset. Anything else is `None`.
pub fn rfc3339_ms(value: &str) -> Option<u64> {
    let b = value.as_bytes();
    let digits = |range: std::ops::Range<usize>| -> Option<i64> {
        let part = b.get(range)?;
        if !part.iter().all(u8::is_ascii_digit) {
            return None;
        }
        std::str::from_utf8(part).ok()?.parse().ok()
    };
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't')
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let (year, month, day) = (digits(0..4)?, digits(5..7)?, digits(8..10)?);
    let (hour, minute, second) = (digits(11..13)?, digits(14..16)?, digits(17..19)?);
    let mut index = 19;
    let mut millis = 0i64;
    if b[index] == b'.' {
        index += 1;
        let start = index;
        while index < b.len() && b[index].is_ascii_digit() {
            index += 1;
        }
        if index == start {
            return None;
        }
        let fraction = &value[start..index.min(start + 3)];
        millis = format!("{fraction:0<3}").parse().ok()?;
    }
    let offset_minutes = match b.get(index)? {
        b'Z' | b'z' if index + 1 == b.len() => 0,
        sign @ (b'+' | b'-') if index + 6 == b.len() && b[index + 3] == b':' => {
            let minutes = digits(index + 1..index + 3)? * 60 + digits(index + 4..index + 6)?;
            if *sign == b'+' { minutes } else { -minutes }
        }
        _ => return None,
    };
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=days_in_month).contains(&day) || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    // Days from the civil date (Howard Hinnant's algorithm).
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second - offset_minutes * 60;
    u64::try_from(seconds.checked_mul(1000)?.checked_add(millis)?).ok()
}

/// One connection's plugin and its in-memory token. The cache lock is held
/// across a run, so concurrent admitted uses run the plugin at most once.
pub struct Credentials {
    binding: Binding,
    cache: Mutex<Option<Token>>,
}
impl Credentials {
    /// Refuses a binding that is not admissible. Runs nothing.
    pub fn new(binding: Binding) -> Option<Self> {
        binding.admissible().then(|| Self {
            binding,
            cache: Mutex::new(None),
        })
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
    /// The bearer token for one admitted use: the held one while `now_ms()`
    /// is before its stated expiry, otherwise a new run of the verified
    /// plugin. `ambient` supplies an allow-listed variable the kubeconfig
    /// does not set; it is never asked for any other name.
    pub fn token(
        &self,
        kubeconfig: &[u8],
        now_ms: &dyn Fn() -> u64,
        ambient: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Zeroizing<Vec<u8>>, Refusal> {
        let mut cache = self.cache.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(held) = cache.as_ref()
            && held.expires_at_ms.is_some_and(|until| now_ms() < until)
        {
            return Ok(held.secret.clone());
        }
        // An expired token is dropped (and zeroized) before the next run.
        *cache = None;
        let verified = verify(&self.binding)?;
        let prepared = prepare(&self.binding, kubeconfig, ambient)?;
        let mut output = run(&prepared, verified)?;
        let parsed = parse(&output, prepared.api_version);
        output.zeroize();
        let token = parsed?;
        let secret = token.secret.clone();
        if token.expires_at_ms.is_some_and(|until| now_ms() < until) {
            *cache = Some(token);
        }
        Ok(secret)
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};

    fn answering(dir: &Path, name: &str, token: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(
            &path,
            format!(
                "#!/bin/sh\nprintf '%s\\n' '{{\"apiVersion\":\"{V1}\",\"kind\":\"ExecCredential\",\
                 \"status\":{{\"token\":\"{token}\"}}}}'\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    /// The TOCTOU the descriptor closes: the file at `path` is replaced
    /// after it was verified and before the process starts. The verified
    /// inode runs; the replacement, which was never hashed, does not.
    #[test]
    fn exec_plugin_runs_the_verified_inode_not_the_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = answering(dir.path(), "fixture-plugin", "fixture-verified-token");
        let swapped = answering(dir.path(), "swapped", "fixture-swapped-token");
        let binding = Binding {
            sha256: hex::encode(Sha256::digest(fs::read(&path).unwrap())),
            path: path.clone(),
            user: "fixture-user".into(),
            env: Vec::new(),
            home: None,
            timeout_ms: Some(5_000),
        };
        let kubeconfig = format!(
            "users:\n- name: fixture-user\n  user:\n    exec:\n      apiVersion: {V1}\n      \
             command: {}\n",
            path.display()
        );
        let verified = verify(&binding).unwrap();
        assert!(verified.script);
        fs::rename(&swapped, &path).unwrap();
        let prepared = prepare(&binding, kubeconfig.as_bytes(), &|_| None).unwrap();
        let output = run(&prepared, verified).unwrap();
        let token = parse(&output, V1).unwrap();
        assert_eq!(&token.secret[..], b"fixture-verified-token");
        // The next run verifies again and refuses the replacement.
        assert_eq!(verify(&binding).err(), Some(Refusal::DigestMismatch));
    }

    /// The descriptor the child executes is never one of its stdio
    /// descriptors, which std replaces before the exec.
    #[test]
    fn exec_plugin_descriptor_is_above_stdio() {
        let dir = tempfile::tempdir().unwrap();
        let path = answering(dir.path(), "fixture-plugin", "fixture-token");
        let binding = Binding {
            sha256: hex::encode(Sha256::digest(fs::read(&path).unwrap())),
            path,
            user: "fixture-user".into(),
            env: Vec::new(),
            home: None,
            timeout_ms: None,
        };
        let verified = verify(&binding).unwrap();
        assert!(std::os::fd::AsRawFd::as_raw_fd(&verified.file) > 2);
    }
}

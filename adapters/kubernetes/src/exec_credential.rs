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
//!   configured), `KUBERNETES_EXEC_INFO` and the allow-listed names, none of
//!   which may be one of the listed loader or start-up variables;
//! - it inherits no descriptor of the adapter beyond stdio (and, for a script,
//!   the plugin descriptor its interpreter reads);
//! - one run is bounded in time and in output, and its process group and its
//!   own process are killed when the run ends, while the plugin's own process
//!   is still unreaped;
//! - the token is held in memory until its stated expiry and never written
//!   anywhere; the buffers that held it or the environment are zeroized.
//!
//! The composition decides when a run may happen (only inside an admitted
//! validation or operation) and how the kubeconfig is read; this module owns
//! the run, reads the kubeconfig only when a run happens, and interprets the
//! answer.
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json, value::RawValue};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
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
    /// The plugin cannot run. `Some` names the operating system's answer
    /// when it refused to open, read or execute the plugin, so a missing
    /// plugin (`NotFound`) is told apart from one some process holds open
    /// for writing (`ExecutableFileBusy`); `None` when the file opened and
    /// one of the plugin's rules refused it. The code is the same either way.
    PluginUnavailable(Option<std::io::ErrorKind>),
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
            Refusal::PluginUnavailable(_) => "plugin_unavailable",
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
/// The exact names `ExecPluginEnvName` refuses: the two the adapter sets
/// itself, and these names, which load code or change how a loader or shell
/// starts. The list is exact, not every name a plugin might read. The model
/// (`spec/ess/domains/exec_credentials.yaml`) is the list; a test holds these
/// constants to it.
pub const REFUSED_ENV_NAMES: &[&str] = &[
    "HOME",
    "KUBERNETES_EXEC_INFO",
    "JAVA_TOOL_OPTIONS",
    "_JAVA_OPTIONS",
    "JDK_JAVA_OPTIONS",
    "CLASSPATH",
    "LUA_INIT",
    "TCLLIBPATH",
    "BASH_ENV",
    "ENV",
    "SHELLOPTS",
    "BASHOPTS",
    "PS4",
    "IFS",
    "CDPATH",
    "GLOBIGNORE",
    "PROMPT_COMMAND",
    "ZDOTDIR",
    "FPATH",
    "OPENSSL_CONF",
    "OPENSSL_MODULES",
    "OPENSSL_ENGINES",
    "GIO_MODULE_DIR",
    "GTK_MODULES",
    "GCONV_PATH",
    "GETCONF_DIR",
    "GLIBC_TUNABLES",
    "HOSTALIASES",
    "LOCALDOMAIN",
    "LOCPATH",
    "MALLOC_TRACE",
    "NIS_PATH",
    "NLSPATH",
    "RESOLV_HOST_CONF",
    "RES_OPTIONS",
];
/// The name prefixes `ExecPluginEnvName` refuses: the dynamic loader and the
/// start-up variables of the interpreters and runtimes a plugin is commonly
/// written in (`DOTNET_` covers `DOTNET_STARTUP_HOOKS`).
pub const REFUSED_ENV_PREFIXES: &[&str] = &[
    "LD_",
    "PYTHON",
    "PERL",
    "RUBY",
    "NODE_",
    "DOTNET_",
    "COREHOST_",
    "LUA_INIT_",
];
/// `connectors_kubernetes.exec_credentials.ExecPluginEnvName`.
fn env_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    (1..=128).contains(&name.len())
        && bytes
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && !REFUSED_ENV_NAMES.contains(&name)
        && !REFUSED_ENV_PREFIXES.iter().any(|p| name.starts_with(p))
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
    #[serde(deserialize_with = "zeroizing")]
    value: Zeroizing<String>,
}
fn zeroizing<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Zeroizing<String>, D::Error> {
    String::deserialize(deserializer).map(Zeroizing::new)
}

/// One run, fully decided before any process starts
/// (`connectors_kubernetes.exec_credentials.ExecPluginSelection`). The
/// environment values may be secrets and are zeroized on drop.
struct Prepared {
    program: PathBuf,
    args: Vec<String>,
    env: Vec<(String, Zeroizing<String>)>,
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
    // path byte for byte (never another spelling of it, such as a trailing
    // slash), or the bare file name of it.
    let pinned = binding.path.to_str();
    let file_name = binding.path.file_name().and_then(|n| n.to_str());
    let command = exec.command.as_str();
    if !(Some(command) == pinned || (!command.contains('/') && Some(command) == file_name)) {
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
            Zeroizing::new(
                home.to_str()
                    .ok_or(Refusal::PluginUnavailable(None))?
                    .to_owned(),
            ),
        ));
    }
    env.push((
        "KUBERNETES_EXEC_INFO".to_owned(),
        Zeroizing::new(
            json!({"apiVersion":api_version,"kind":"ExecCredential","spec":{"interactive":false}})
                .to_string(),
        ),
    ));
    for name in &binding.env {
        let value = declared
            .iter()
            .find(|e| &e.name == name)
            .map(|e| e.value.clone())
            .or_else(|| ambient(name).map(Zeroizing::new));
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
/// replaceable by another user nor carry privileges of its own. Checked
/// before every run, on the descriptor the run executes.
#[cfg(target_os = "linux")]
fn verify(binding: &Binding) -> Result<Verified, Refusal> {
    use std::os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    };
    // O_NOFOLLOW: a symbolic link as the final component refuses (ELOOP);
    // O_NONBLOCK: a FIFO at the path cannot stall the open; std adds
    // O_CLOEXEC, so no concurrently spawned process inherits it.
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&binding.path)
        .map_err(unavailable)?;
    // fstat on the open descriptor, never a second lookup of the path.
    let metadata = file.metadata().map_err(unavailable)?;
    // SAFETY: geteuid has no preconditions and cannot fail.
    let uid = unsafe { libc::geteuid() };
    if !metadata.is_file()
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & (libc::S_ISUID | libc::S_ISGID) != 0
        || (metadata.uid() != 0 && metadata.uid() != uid)
    {
        return Err(Refusal::PluginUnavailable(None));
    }
    // File capabilities raise privileges on exec like a set-ID bit does. A
    // file system without extended attributes (ENOTSUP) carries none; any
    // other failure to find out refuses.
    // SAFETY: the descriptor is open; a null buffer of size 0 only asks for
    // the attribute's size.
    let present = unsafe {
        libc::fgetxattr(
            file.as_raw_fd(),
            c"security.capability".as_ptr(),
            std::ptr::null_mut(),
            0,
        )
    };
    if present >= 0 {
        return Err(Refusal::PluginUnavailable(None));
    }
    let error = std::io::Error::last_os_error();
    match error.raw_os_error() {
        Some(libc::ENODATA) | Some(libc::ENOTSUP) => {}
        _ => return Err(unavailable(error)),
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
            Err(error) => return Err(unavailable(error)),
        }
    }
    if hex::encode(hasher.finalize()) != binding.sha256 {
        return Err(Refusal::DigestMismatch);
    }
    // The child redirects descriptors 0 to 2 onto its stdio before it
    // executes this one, so it must not be one of them (a parent started
    // with stdio closed). try_clone duplicates to 3 or above, close-on-exec.
    if file.as_raw_fd() < 3 {
        file = file.try_clone().map_err(unavailable)?;
    }
    Ok(Verified {
        file,
        script: head == b"#!",
    })
}
/// Executing a descriptor needs `fexecve`; elsewhere no plugin runs.
#[cfg(not(target_os = "linux"))]
fn verify(_binding: &Binding) -> Result<Verified, Refusal> {
    Err(Refusal::PluginUnavailable(Some(
        std::io::ErrorKind::Unsupported,
    )))
}
/// The operating system refused to open, read or execute the plugin; its
/// error kind travels with the refusal.
#[cfg(target_os = "linux")]
fn unavailable(error: std::io::Error) -> Refusal {
    Refusal::PluginUnavailable(Some(error.kind()))
}

/// The argument and environment vectors `fexecve` receives, built before the
/// fork: the child between fork and exec may not allocate. Each string is
/// NUL-terminated in a buffer sized exactly once, so building it never
/// reallocates, and every buffer is zeroized on drop (the environment
/// strings may hold secrets).
#[cfg(target_os = "linux")]
struct Image {
    fd: std::os::fd::RawFd,
    script: bool,
    argv: Vec<*const libc::c_char>,
    envp: Vec<*const libc::c_char>,
    /// Owns the bytes `argv` and `envp` point into.
    _strings: Vec<Zeroizing<Vec<u8>>>,
}
// SAFETY: the pointers point into the heap buffers of `_strings`, which the
// struct owns and never mutates while it lives; they are only read, by the
// forked child.
#[cfg(target_os = "linux")]
unsafe impl Send for Image {}
#[cfg(target_os = "linux")]
unsafe impl Sync for Image {}
#[cfg(target_os = "linux")]
impl Image {
    fn new(prepared: &Prepared, verified: &Verified) -> Result<Self, Refusal> {
        use std::{os::fd::AsRawFd, os::unix::ffi::OsStrExt};
        let c = |parts: &[&[u8]]| -> Result<Zeroizing<Vec<u8>>, Refusal> {
            let length = parts.iter().map(|p| p.len()).sum::<usize>() + 1;
            let mut bytes = Zeroizing::new(Vec::with_capacity(length));
            for part in parts {
                if part.contains(&0) {
                    return Err(Refusal::KubeconfigMismatch);
                }
                bytes.extend_from_slice(part);
            }
            bytes.push(0);
            Ok(bytes)
        };
        let mut strings = vec![c(&[prepared.program.as_os_str().as_bytes()])?];
        for arg in &prepared.args {
            strings.push(c(&[arg.as_bytes()])?);
        }
        let argc = strings.len();
        for (name, value) in &prepared.env {
            strings.push(c(&[name.as_bytes(), b"=", value.as_bytes()])?);
        }
        let pointers = |range: &[Zeroizing<Vec<u8>>]| {
            range
                .iter()
                .map(|s| s.as_ptr().cast::<libc::c_char>())
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
    /// directory and set the process group. Async-signal-safe calls only, and
    /// no allocation.
    fn exec(&self) -> std::io::Error {
        // SAFETY: close_range, getrlimit and fcntl act on this child's own
        // descriptor table; fd is the verified plugin descriptor, open in the
        // parent and so in this child; argv and envp are NUL-terminated
        // arrays of NUL-terminated strings that outlive this call.
        unsafe {
            // No descriptor of the adapter above stderr survives the exec,
            // whether or not the adapter opened it close-on-exec.
            if libc::syscall(
                libc::SYS_close_range,
                3 as libc::c_uint,
                libc::c_uint::MAX,
                libc::CLOSE_RANGE_CLOEXEC,
            ) != 0
            {
                // Before Linux 5.11 (no close_range, or no CLOEXEC flag):
                // mark each possible descriptor. The kernel never hands out
                // one above the hard limit's ceiling, 1 << 20 by default.
                let mut limit = libc::rlimit {
                    rlim_cur: 0,
                    rlim_max: 0,
                };
                let highest = if libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) == 0 {
                    limit.rlim_cur.max(limit.rlim_max).min(1 << 20)
                } else {
                    1 << 20
                };
                for fd in 3..highest as libc::c_int {
                    let flags = libc::fcntl(fd, libc::F_GETFD);
                    if flags >= 0
                        && flags & libc::FD_CLOEXEC == 0
                        && libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) < 0
                    {
                        return std::io::Error::last_os_error();
                    }
                }
            }
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

/// Whether the plugin's process has exited, and with status 0, observed
/// without reaping it (`WNOWAIT`): while it is unreaped its pid, and so its
/// process group id, cannot name anything else.
#[cfg(target_os = "linux")]
fn exited_unreaped(pid: libc::pid_t) -> std::io::Result<Option<bool>> {
    // SAFETY: an all-zero siginfo_t is a valid value to be overwritten.
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    // SAFETY: pid is this process's own unreaped child; info is writable.
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            pid as libc::id_t,
            &mut info,
            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
        )
    };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        return if error.kind() == std::io::ErrorKind::Interrupted {
            Ok(None)
        } else {
            Err(error)
        };
    }
    // SAFETY: waitid filled info for a child event, or left si_pid 0 under
    // WNOHANG when the child has not exited.
    let (event_pid, status) = unsafe { (info.si_pid(), info.si_status()) };
    if event_pid == 0 {
        return Ok(None);
    }
    Ok(Some(info.si_code == libc::CLD_EXITED && status == 0))
}

/// Ends a run: kills the plugin's whole process group once and the plugin's
/// own process once, then reaps the plugin. Killing the process as well as
/// the group ends the run even when the plugin moved itself to another
/// group, so the wait below cannot outlast the deadline; a descendant that
/// left the group is not followed. Called exactly once per spawned plugin,
/// and nothing before it reaps the child.
#[cfg(target_os = "linux")]
fn end(child: &mut std::process::Child, pid: libc::pid_t) {
    // SAFETY: the plugin is unreaped (only `exited_unreaped`, which does not
    // reap, has waited on it), so its pid cannot name another process, and
    // the group id it was given by process_group(0) cannot name another
    // group: a process group id stays reserved while its leader's pid is
    // held, even after the leader moved to another group. kill only sends a
    // signal. After the wait below neither is signalled again.
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
        libc::kill(pid, libc::SIGKILL);
    }
    let _ = child.wait();
}

/// Reads what stdout holds now, without blocking. `Ok(true)` once it is
/// closed. The buffer was sized to the bound, so it never reallocates.
#[cfg(target_os = "linux")]
fn drain(
    stdout: &mut std::process::ChildStdout,
    output: &mut Vec<u8>,
    chunk: &mut [u8],
) -> Result<bool, Refusal> {
    loop {
        match stdout.read(chunk) {
            Ok(0) => return Ok(true),
            Ok(n) if output.len() + n > MAX_OUTPUT_BYTES => return Err(Refusal::OutputTooLarge),
            Ok(n) => output.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return Err(Refusal::PluginFailed),
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn run(_prepared: &Prepared, _verified: Verified) -> Result<Zeroizing<Vec<u8>>, Refusal> {
    Err(Refusal::PluginUnavailable(Some(
        std::io::ErrorKind::Unsupported,
    )))
}
#[cfg(target_os = "linux")]
fn run(prepared: &Prepared, verified: Verified) -> Result<Zeroizing<Vec<u8>>, Refusal> {
    use std::{
        os::{fd::AsRawFd, unix::process::CommandExt},
        process::{Command, Stdio},
        time::Instant,
    };
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
    // SAFETY: `Image::exec` calls only close_range, getrlimit, fcntl and
    // fexecve, all async-signal-safe, and allocates nothing.
    unsafe {
        command.pre_exec(move || Err(image.exec()));
    }
    let spawned = command.spawn();
    // The parent's copy closes now; the child has exec'd or failed.
    drop(verified);
    let mut child = spawned.map_err(unavailable)?;
    let Ok(pid) = libc::pid_t::try_from(child.id()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(Refusal::PluginFailed);
    };
    let deadline = Instant::now() + prepared.timeout;
    let Some(mut stdout) = child.stdout.take() else {
        end(&mut child, pid);
        return Err(Refusal::PluginFailed);
    };
    // Non-blocking, so the run reads only while the deadline allows and no
    // reader outlives it.
    let fd = stdout.as_raw_fd();
    // SAFETY: fd is the parent's own read end of the stdout pipe.
    let nonblocking = unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        flags >= 0 && libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) == 0
    };
    if !nonblocking {
        drop(stdout);
        end(&mut child, pid);
        return Err(Refusal::PluginFailed);
    }
    let mut output = Zeroizing::new(Vec::with_capacity(MAX_OUTPUT_BYTES));
    let mut chunk = Zeroizing::new([0u8; 8192]);
    let mut open = true;
    let mut exited = None;
    let outcome = loop {
        if open {
            match drain(&mut stdout, &mut output, &mut chunk[..]) {
                Ok(closed) => open = !closed,
                Err(refusal) => break Err(refusal),
            }
        }
        if exited.is_none() {
            match exited_unreaped(pid) {
                Ok(state) => exited = state,
                Err(_) => break Err(Refusal::PluginFailed),
            }
        }
        match (open, exited) {
            (false, Some(true)) => break Ok(()),
            (false, Some(false)) => break Err(Refusal::PluginFailed),
            _ => {}
        }
        let now = Instant::now();
        if now >= deadline {
            break Err(Refusal::Timeout);
        }
        let wait = (deadline - now).min(Duration::from_millis(if open { 20 } else { 5 }));
        if open {
            let mut poll = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: one valid pollfd for the open read end. A failure or
            // interruption only shortens this wait.
            unsafe {
                libc::poll(&mut poll, 1, wait.as_millis().max(1) as libc::c_int);
            }
        } else {
            std::thread::sleep(wait);
        }
    };
    // The read end closes before the group is killed: a descendant that left
    // the group and still holds stdout cannot hold this run, and its next
    // write fails.
    drop(stdout);
    end(&mut child, pid);
    outcome.map(|()| output)
}

/// An accepted ExecCredential token. Intentionally no Debug, Clone or
/// Serialize: the bytes leave only as a fresh zeroizing copy.
struct Token {
    secret: Zeroizing<Vec<u8>>,
    expires_at_ms: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Answer<'a> {
    api_version: String,
    kind: String,
    #[serde(borrow)]
    status: Option<AnswerStatus<'a>>,
}
/// The token is borrowed as raw JSON from the output buffer and unescaped
/// into one exactly sized zeroizing buffer, so the parser never copies it
/// into a buffer of its own.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnswerStatus<'a> {
    #[serde(default, borrow)]
    token: Option<&'a RawValue>,
    #[serde(default)]
    expiration_timestamp: Option<String>,
}
/// A JSON string literal's bytes, unescaped. Only escapes that can stand for
/// a printable ASCII byte are read; anything else is `None`, as is anything
/// that is not a string.
fn unescape(raw: &str) -> Option<Zeroizing<Vec<u8>>> {
    let inner = raw.strip_prefix('"')?.strip_suffix('"')?.as_bytes();
    let mut out = Zeroizing::new(Vec::with_capacity(inner.len()));
    let mut index = 0;
    while index < inner.len() {
        let byte = inner[index];
        if byte != b'\\' {
            out.push(byte);
            index += 1;
            continue;
        }
        let escaped = match *inner.get(index + 1)? {
            b'"' => b'"',
            b'\\' => b'\\',
            b'/' => b'/',
            b'u' => {
                let hex = inner.get(index + 2..index + 6)?;
                if !hex.iter().all(u8::is_ascii_hexdigit) {
                    return None;
                }
                let code = u16::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?;
                index += 4;
                u8::try_from(code).ok()?
            }
            _ => return None,
        };
        out.push(escaped);
        index += 2;
    }
    Some(out)
}

fn parse(output: &[u8], api_version: &str) -> Result<Token, Refusal> {
    let answer: Answer =
        serde_json::from_slice(output).map_err(|_| Refusal::MalformedCredential)?;
    if answer.api_version != api_version || answer.kind != "ExecCredential" {
        return Err(Refusal::MalformedCredential);
    }
    let status = answer.status.ok_or(Refusal::MalformedCredential)?;
    let raw = status.token.ok_or(Refusal::MalformedCredential)?;
    let secret = unescape(raw.get()).ok_or(Refusal::MalformedCredential)?;
    if secret.is_empty()
        || secret.len() > MAX_TOKEN_BYTES
        || !secret.iter().all(u8::is_ascii_graphic)
    {
        return Err(Refusal::MalformedCredential);
    }
    let expires_at_ms = match status.expiration_timestamp {
        None => None,
        Some(value) => Some(rfc3339_ms(&value).ok_or(Refusal::MalformedCredential)?),
    };
    Ok(Token {
        secret,
        expires_at_ms,
    })
}

/// RFC 3339 date-time to Unix milliseconds: `YYYY-MM-DDTHH:MM:SS`, optional
/// fraction, then `Z` or a `±HH:MM` offset (hour 00-23, minute 00-59).
/// Anything else is `None`. A valid instant before the epoch is `Some(0)`: an
/// expiry already past for every clock this adapter reads, never malformed.
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
            let (hours, minutes) = (digits(index + 1..index + 3)?, digits(index + 4..index + 6)?);
            if hours > 23 || minutes > 59 {
                return None;
            }
            let total = hours * 60 + minutes;
            if *sign == b'+' { total } else { -total }
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
    let ms = seconds.checked_mul(1000)?.checked_add(millis)?;
    Some(u64::try_from(ms).unwrap_or(0))
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
    /// plugin. `kubeconfig` is called only for a run, never when the held
    /// token serves. `ambient` supplies an allow-listed variable the
    /// kubeconfig does not set; it is never asked for any other name.
    pub fn token_with(
        &self,
        kubeconfig: &dyn Fn() -> Result<Zeroizing<Vec<u8>>, Refusal>,
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
        let prepared = prepare(&self.binding, &kubeconfig()?, ambient)?;
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
    /// [`Credentials::token_with`] over kubeconfig bytes already in hand.
    pub fn token(
        &self,
        kubeconfig: &[u8],
        now_ms: &dyn Fn() -> u64,
        ambient: &dyn Fn(&str) -> Option<String>,
    ) -> Result<Zeroizing<Vec<u8>>, Refusal> {
        self.token_with(&|| Ok(Zeroizing::new(kubeconfig.to_vec())), now_ms, ambient)
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

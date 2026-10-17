//! Security conformance checks of the pinned exec credential plugin against
//! its own contract (`contracts/reads/v1alpha1/semantics.md`, "Exec-plugin
//! credentials"): descriptors the plugin inherits, the token's lifetime in
//! adapter memory, the stdio descriptor guard, a hidden `/proc`, ownership and
//! ACL checks, the exact environment and one run under concurrent use. Every
//! plugin is a fixture in the test's own temporary directory; no real
//! credential is used.
#![cfg(target_os = "linux")]
use connectors_kubernetes::exec_credential::{Binding, Credentials, Refusal, rfc3339_ms};
use sha2::{Digest, Sha256};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::{Duration, Instant},
};

const USER: &str = "eks-fixture-user";
const LATER: &str = "2030-01-01T00:00:00Z";
const V1: &str = "client.authentication.k8s.io/v1";
const CHILD: &str = "EXEC_CREDENTIAL_SECURITY_CHILD";

// ---------------------------------------------------------------------------
// A probe allocator: while armed, every freed heap block is searched for the
// marker token before it is returned to the system. A zeroized buffer is all
// zeroes when it is freed; a block that still holds the marker was released
// with the token in it. `realloc` is the trait's default, so a growing
// buffer's old block passes through `dealloc` too.
// ---------------------------------------------------------------------------
const MARKER: &[u8] = b"fixture-zeroize-marker";
static ARMED: AtomicBool = AtomicBool::new(false);
static FREED_WITH_MARKER: AtomicUsize = AtomicUsize::new(0);
struct Probe;
unsafe impl GlobalAlloc for Probe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ARMED.load(Ordering::Relaxed)
            && layout.size() >= MARKER.len()
            && layout.size() <= 1 << 20
        {
            let mut matched = 0;
            for i in 0..layout.size() {
                // SAFETY: the block is still allocated and `size` bytes long.
                let byte = unsafe { std::ptr::read_volatile(ptr.add(i)) };
                matched = if byte == MARKER[matched] {
                    matched + 1
                } else if byte == MARKER[0] {
                    1
                } else {
                    0
                };
                if matched == MARKER.len() {
                    FREED_WITH_MARKER.fetch_add(1, Ordering::Relaxed);
                    break;
                }
            }
        }
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static GLOBAL: Probe = Probe;

// ---------------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------------
fn sha256(path: &Path) -> String {
    hex::encode(Sha256::digest(fs::read(path).unwrap()))
}
fn answer(token: &str) -> String {
    format!(
        r#"{{"apiVersion":"{V1}","kind":"ExecCredential","status":{{"token":"{token}","expirationTimestamp":"{LATER}"}}}}"#
    )
}
fn binding(path: &Path) -> Binding {
    Binding {
        path: path.to_path_buf(),
        sha256: sha256(path),
        user: USER.into(),
        env: Vec::new(),
        home: None,
        timeout_ms: Some(5_000),
    }
}
/// A kubeconfig whose user runs `command` with `args`; args are written as
/// YAML single-quoted scalars, so none may contain a single quote.
fn kubeconfig(command: &Path, args: &[&str]) -> Vec<u8> {
    let args: Vec<String> = args
        .iter()
        .map(|a| {
            assert!(!a.contains('\''));
            format!("'{a}'")
        })
        .collect();
    format!(
        "users:\n- name: {USER}\n  user:\n    exec:\n      apiVersion: {V1}\n      \
         command: {}\n      args: [{}]\n",
        command.display(),
        args.join(", ")
    )
    .into_bytes()
}
fn now() -> u64 {
    rfc3339_ms("2029-01-01T00:00:00Z").unwrap()
}
fn no_ambient(_: &str) -> Option<String> {
    None
}
/// A shell script plugin (`#!/bin/sh`) with `body`, mode 0700.
fn script(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("fixture-plugin");
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}
/// A digest-pinned copy of a system binary (a native, non-`#!` plugin).
fn native(dir: &Path, candidates: &[&str], name: &str) -> Option<PathBuf> {
    let source = candidates.iter().find(|p| Path::new(p).exists())?;
    let path = dir.join(name);
    fs::copy(fs::canonicalize(source).unwrap(), &path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    assert_ne!(&fs::read(&path).unwrap()[..2], b"#!");
    Some(path)
}
fn ok_token(result: Result<zeroize::Zeroizing<Vec<u8>>, Refusal>) -> String {
    String::from_utf8(result.expect("the plugin answered").to_vec()).unwrap()
}
/// Re-run one test of this binary as a child with `mode` in its environment.
fn child(test: &str, mode: &str, wrap: Option<&[&str]>) -> std::process::Command {
    let exe = std::env::current_exe().unwrap();
    let mut command = match wrap {
        None => Command::new(&exe),
        Some(prefix) => {
            let mut c = Command::new(prefix[0]);
            c.args(&prefix[1..]).arg(&exe);
            c
        }
    };
    command
        .args(["--exact", test, "--nocapture", "--test-threads=1"])
        .env(CHILD, mode);
    command
}

// ---------------------------------------------------------------------------
// Descriptors.
// ---------------------------------------------------------------------------

/// Contract: "a native executable inherits no descriptor". The adapter
/// process holds one descriptor without close-on-exec (as a process does
/// that inherited it from whatever started it); a native plugin must not
/// receive it.
#[test]
fn exec_plugin_native_executable_inherits_no_descriptor_of_the_adapter() {
    let dir = tempfile::tempdir().unwrap();
    let Some(bash) = native(dir.path(), &["/usr/bin/bash", "/bin/bash"], "bash") else {
        eprintln!("skipped: no bash to copy as a native plugin");
        return;
    };
    let held = dir.path().join("held-by-the-adapter");
    fs::write(&held, b"adapter-private").unwrap();
    let c_path = std::ffi::CString::new(held.as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: a NUL-terminated path in the test's own directory; no O_CLOEXEC.
    let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDONLY) };
    assert!(fd > 2, "{fd}");
    let out = dir.path().join("fds");
    let program = format!(
        "f=; for p in /proc/$$/fd/*; do [ -e \"$p\" ] && f=\"$f ${{p##*/}}\"; done; \
         echo \"$f\" > {}; printf %s \"$1\"",
        out.display()
    );
    let token = "fixture-descriptor-token";
    let config = kubeconfig(&bash, &["-c", &program, "plugin", &answer(token)]);
    let credentials = Credentials::new(binding(&bash)).unwrap();
    let result = credentials.token(&config, &now, &no_ambient);
    // SAFETY: fd was opened above and is closed once.
    unsafe { libc::close(fd) };
    assert_eq!(ok_token(result), token);
    let open: Vec<i32> = fs::read_to_string(&out)
        .unwrap()
        .split_whitespace()
        .map(|n| n.parse().unwrap())
        .collect();
    assert!(
        !open.contains(&fd),
        "the native plugin inherited the adapter's descriptor {fd}; its open descriptors: {open:?}"
    );
}

/// The stdio guard (invariant 2): with descriptor 0 closed in the adapter,
/// the plugin file opens as 0, which std replaces with /dev/null in the child
/// before the exec. The run must still execute the verified file. Measured in
/// a child process of this binary, so no other test shares its descriptors.
#[test]
fn exec_plugin_runs_when_the_adapter_starts_with_stdio_closed() {
    if let Ok(mode) = std::env::var(CHILD) {
        if !mode.starts_with("stdio-") {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let token = "fixture-stdio-token";
        let path = script(dir.path(), &format!("printf '%s' '{}'", answer(token)));
        let credentials = Credentials::new(binding(&path)).unwrap();
        let config = kubeconfig(&path, &[]);
        let closing: &[i32] = if mode == "stdio-0" { &[0] } else { &[0, 1, 2] };
        for fd in closing {
            // SAFETY: closing a standard descriptor of this child process.
            unsafe { libc::close(*fd) };
            // SAFETY: F_GETFD on a closed descriptor only reports EBADF.
            assert_eq!(unsafe { libc::fcntl(*fd, libc::F_GETFD) }, -1);
        }
        let result = credentials.token(&config, &now, &no_ambient);
        let code = match &result {
            Ok(bytes) if &bytes[..] == token.as_bytes() => 0,
            Ok(_) => 10,
            Err(r) => 20 + *r as i32,
        };
        std::process::exit(code);
    }
    for mode in ["stdio-0", "stdio-012"] {
        let status = child(
            "exec_plugin_runs_when_the_adapter_starts_with_stdio_closed",
            mode,
            None,
        )
        .status()
        .unwrap();
        assert_eq!(
            status.code(),
            Some(0),
            "{mode}: exit 10 is a wrong token, 20+n is Refusal discriminant n \
             (0 DigestMismatch, 1 PluginUnavailable, ..., 6 PluginFailed)"
        );
    }
}

/// With `/proc` hidden (a fresh tmpfs over it in a private mount namespace),
/// a native plugin still runs from its descriptor; a script plugin, whose
/// interpreter needs `/dev/fd`, is refused within its deadline rather than
/// running anything else.
#[test]
fn exec_plugin_without_proc_runs_native_and_refuses_scripts() {
    if let Ok(mode) = std::env::var(CHILD) {
        if mode != "no-proc" {
            return;
        }
        assert!(
            fs::read_dir("/proc").unwrap().next().is_none(),
            "/proc is still mounted"
        );
        let dir = tempfile::tempdir().unwrap();
        let token = "fixture-no-proc-token";
        let echo = native(dir.path(), &["/usr/bin/echo", "/bin/echo"], "echo").unwrap();
        let credentials = Credentials::new(binding(&echo)).unwrap();
        let native = credentials.token(&kubeconfig(&echo, &[&answer(token)]), &now, &no_ambient);
        assert_eq!(ok_token(native), token);
        let path = script(dir.path(), &format!("printf '%s' '{}'", answer(token)));
        let mut b = binding(&path);
        b.timeout_ms = Some(2_000);
        let credentials = Credentials::new(b).unwrap();
        let started = Instant::now();
        let result = credentials.token(&kubeconfig(&path, &[]), &now, &no_ambient);
        eprintln!("script plugin without /proc: {:?}", result.as_ref().err());
        assert!(result.is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
        return;
    }
    let usable = Command::new("unshare")
        .args(["-Urm", "sh", "-c", "mount -t tmpfs none /proc"])
        .status()
        .is_ok_and(|s| s.success());
    if !usable {
        eprintln!("skipped: no unprivileged mount namespace to hide /proc in");
        return;
    }
    let status = child(
        "exec_plugin_without_proc_runs_native_and_refuses_scripts",
        "no-proc",
        Some(&[
            "unshare",
            "-Urm",
            "sh",
            "-c",
            "mount -t tmpfs none /proc && exec \"$@\"",
            "wrap",
        ]),
    )
    .status()
    .unwrap();
    assert!(status.success(), "{status}");
}

// ---------------------------------------------------------------------------
// The token in memory.
// ---------------------------------------------------------------------------

/// Contract: the token is "held in adapter memory, zeroized on drop". A
/// plugin that writes its answer in two writes (any plugin that does not
/// write it in one system call) makes the adapter's stdout buffer grow; no
/// heap block released during the run may still hold the token.
#[test]
fn exec_plugin_token_is_never_released_unzeroized_from_the_output_buffer() {
    let dir = tempfile::tempdir().unwrap();
    let Some(sleep) = ["/usr/bin/sleep", "/bin/sleep"]
        .into_iter()
        .find(|p| Path::new(p).exists())
    else {
        eprintln!("skipped: no sleep");
        return;
    };
    // The marker is assembled only by the plugin: this process never holds
    // it contiguously except as the plugin's answer.
    let head = format!(r#"{{"apiVersion":"{V1}","kind":"ExecCredential","status":{{"token":""#);
    let tail = format!(r#"-0123456789","expirationTimestamp":"{LATER}"}}}}"#);
    let path = script(
        dir.path(),
        &format!(
            "printf '%s%s%s' '{head}' 'fixture-zero' 'ize-marker'\n{sleep} 0.3\nprintf '%s' '{tail}'"
        ),
    );
    let credentials = Credentials::new(binding(&path)).unwrap();
    let config = kubeconfig(&path, &[]);

    // The probe detects a released block that holds the marker.
    ARMED.store(true, Ordering::SeqCst);
    let mut control = Vec::with_capacity(64);
    control.extend_from_slice(b"fixture-zero");
    control.extend_from_slice(b"ize-marker");
    drop(control);
    ARMED.store(false, Ordering::SeqCst);
    assert_eq!(
        FREED_WITH_MARKER.swap(0, Ordering::SeqCst),
        1,
        "probe self-check"
    );

    let measure = |credentials: Credentials| {
        ARMED.store(true, Ordering::SeqCst);
        let result = credentials.token(&config, &now, &no_ambient);
        let answered = result
            .as_ref()
            .is_ok_and(|t| t.starts_with(b"fixture-zero") && t.ends_with(b"-0123456789"));
        drop(result);
        drop(credentials);
        ARMED.store(false, Ordering::SeqCst);
        assert!(answered, "the plugin did not answer");
        FREED_WITH_MARKER.swap(0, Ordering::SeqCst)
    };
    let split = measure(credentials);
    // Control: the same answer in one write leaves no block behind, so a
    // leak above is the growing output buffer and nothing else.
    let single_dir = tempfile::tempdir().unwrap();
    let single = script(
        single_dir.path(),
        &format!("printf '%s%s%s%s' '{head}' 'fixture-zero' 'ize-marker' '{tail}'"),
    );
    let single_config = kubeconfig(&single, &[]);
    ARMED.store(true, Ordering::SeqCst);
    let credentials = Credentials::new(binding(&single)).unwrap();
    let result = credentials.token(&single_config, &now, &no_ambient);
    assert!(result.is_ok());
    drop(result);
    drop(credentials);
    ARMED.store(false, Ordering::SeqCst);
    let one_write = FREED_WITH_MARKER.swap(0, Ordering::SeqCst);
    assert_eq!(
        (split, one_write),
        (0, 0),
        "heap blocks holding the token freed without being zeroized: \
         {split} with the answer in two writes, {one_write} with it in one"
    );
}

// ---------------------------------------------------------------------------
// Ownership and modes.
// ---------------------------------------------------------------------------

/// A POSIX ACL that grants another user write shows in the group class bits
/// (the mask); masked to read-only, the entry grants nothing and the file
/// runs.
#[test]
fn exec_plugin_acl_granting_write_to_another_user_is_refused() {
    if Command::new("setfacl").arg("--version").output().is_err() {
        eprintln!("skipped: no setfacl");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let token = "fixture-acl-token";
    let path = script(dir.path(), &format!("printf '%s' '{}'", answer(token)));
    let config = kubeconfig(&path, &[]);
    let set = |spec: &str| {
        let out = Command::new("setfacl")
            .args(["-m", spec])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    for (entry, named) in [("u:nobody:rwx", "user:nobody"), ("g:root:rw", "group:root")] {
        let _ = Command::new("setfacl").arg("-b").arg(&path).status();
        set(entry);
        assert_ne!(fs::metadata(&path).unwrap().mode() & 0o020, 0, "{entry}");
        let credentials = Credentials::new(binding(&path)).unwrap();
        assert_eq!(
            credentials.token(&config, &now, &no_ambient).err(),
            Some(Refusal::PluginUnavailable),
            "{entry}"
        );
        set("m::r-x");
        let acl = Command::new("getfacl")
            .arg("-c")
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            String::from_utf8_lossy(&acl.stdout).contains(named),
            "{entry}"
        );
        let credentials = Credentials::new(binding(&path)).unwrap();
        assert_eq!(
            ok_token(credentials.token(&config, &now, &no_ambient)),
            token
        );
    }
}

/// A file owned by a user other than root and the adapter's own is refused
/// even when its digest matches; a hard link to such a file is the same
/// inode, so this is also the hard-link case.
#[test]
fn exec_plugin_file_owned_by_another_user_is_refused_before_the_digest() {
    // SAFETY: geteuid has no preconditions.
    let euid = unsafe { libc::geteuid() };
    let mut queue: Vec<(PathBuf, usize)> = ["/etc", "/var/lib", "/usr/share", "/usr/include"]
        .iter()
        .map(|p| (PathBuf::from(p), 0))
        .collect();
    let mut found = None;
    while let Some((dir, depth)) = queue.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.is_dir() && depth < 3 {
                queue.push((path, depth + 1));
            } else if meta.is_file()
                && meta.uid() != 0
                && meta.uid() != euid
                && meta.mode() & 0o6022 == 0
                && meta.len() < 1 << 20
                && fs::read(&path).is_ok()
            {
                found = Some(path);
                break;
            }
        }
        if found.is_some() {
            break;
        }
    }
    let Some(path) = found else {
        eprintln!("skipped: no readable file owned by another user");
        return;
    };
    let credentials = Credentials::new(binding(&path)).unwrap();
    assert_eq!(
        credentials
            .token(&kubeconfig(&path, &[]), &now, &no_ambient)
            .err(),
        Some(Refusal::PluginUnavailable),
        "{}",
        path.display()
    );
}

/// A world-writable directory holding the pinned file lets another user
/// replace the path, never the file: the replacement is not the hashed
/// inode, so it never runs.
#[test]
fn exec_plugin_replacement_in_a_writable_directory_never_runs() {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o777)).unwrap();
    let ran = dir.path().join("replacement-ran");
    let path = script(
        dir.path(),
        &format!("printf '%s' '{}'", answer("fixture-dir-token")),
    );
    let b = binding(&path);
    let replacement = dir.path().join("replacement");
    fs::write(&replacement, format!("#!/bin/sh\n: > {}\n", ran.display())).unwrap();
    fs::set_permissions(&replacement, fs::Permissions::from_mode(0o700)).unwrap();
    fs::rename(&replacement, &path).unwrap();
    let credentials = Credentials::new(b).unwrap();
    assert_eq!(
        credentials
            .token(&kubeconfig(&path, &[]), &now, &no_ambient)
            .err(),
        Some(Refusal::DigestMismatch)
    );
    assert!(!ran.exists());
}

// ---------------------------------------------------------------------------
// Environment and concurrency.
// ---------------------------------------------------------------------------

/// The exact initial environment of the plugin process (read from its own
/// `/proc/self/environ`, not from a shell's export list): `HOME`,
/// `KUBERNETES_EXEC_INFO` and the allow-listed names, nothing else — no
/// `PATH`, no loader variable, nothing of the adapter's. The ambient lookup
/// is asked only for allow-listed names.
#[test]
fn exec_plugin_environment_is_exactly_the_contract_names() {
    let dir = tempfile::tempdir().unwrap();
    let Some(bash) = native(dir.path(), &["/usr/bin/bash", "/bin/bash"], "bash") else {
        eprintln!("skipped: no bash");
        return;
    };
    fs::create_dir(dir.path().join("home")).unwrap();
    assert!(std::env::var("PATH").is_ok());
    let out = dir.path().join("names");
    let program = format!(
        "l=; while IFS= read -r -d \"\" v; do l=\"$l ${{v%%=*}}\"; done < /proc/$$/environ; \
         echo \"$l\" > {}; printf %s \"$1\"",
        out.display()
    );
    let token = "fixture-environment-token";
    let mut b = binding(&bash);
    b.env = vec!["FIXTURE_ALLOWED".into(), "FIXTURE_ABSENT".into()];
    b.home = Some(dir.path().join("home"));
    let credentials = Credentials::new(b).unwrap();
    let asked = std::sync::Mutex::new(Vec::new());
    let ambient = |name: &str| {
        asked.lock().unwrap().push(name.to_owned());
        (name == "FIXTURE_ALLOWED").then(|| "fixture-value".to_owned())
    };
    let config = kubeconfig(&bash, &["-c", &program, "plugin", &answer(token)]);
    assert_eq!(ok_token(credentials.token(&config, &now, &ambient)), token);
    let mut names: Vec<String> = fs::read_to_string(&out)
        .unwrap()
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    names.sort();
    assert_eq!(names, ["FIXTURE_ALLOWED", "HOME", "KUBERNETES_EXEC_INFO"]);
    let mut asked = asked.into_inner().unwrap();
    asked.sort();
    assert_eq!(asked, ["FIXTURE_ABSENT", "FIXTURE_ALLOWED"]);
}

/// Invariant 9: concurrent admitted uses of one connection run the plugin at
/// most once and all receive its token.
#[test]
fn exec_plugin_concurrent_uses_run_the_plugin_once() {
    let dir = tempfile::tempdir().unwrap();
    let Some(sleep) = ["/usr/bin/sleep", "/bin/sleep"]
        .into_iter()
        .find(|p| Path::new(p).exists())
    else {
        eprintln!("skipped: no sleep");
        return;
    };
    let runs = dir.path().join("runs");
    let token = "fixture-concurrent-token";
    let path = script(
        dir.path(),
        &format!(
            "echo run >> {}\n{sleep} 0.3\nprintf '%s' '{}'",
            runs.display(),
            answer(token)
        ),
    );
    let credentials = Credentials::new(binding(&path)).unwrap();
    let config = kubeconfig(&path, &[]);
    let tokens: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..6)
            .map(|_| scope.spawn(|| ok_token(credentials.token(&config, &now, &no_ambient))))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert!(tokens.iter().all(|t| t == token));
    assert_eq!(fs::read_to_string(&runs).unwrap().lines().count(), 1);
}

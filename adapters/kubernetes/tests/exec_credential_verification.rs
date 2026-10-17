//! Verification of the exec plugin fix pass against its own contract
//! (`contracts/reads/v1alpha1/semantics.md`, "Exec-plugin credentials", and
//! `spec/ess/domains/exec_credentials.yaml`): the deadline when the plugin's
//! own process leaves its group, the close-on-exec fallback when
//! `close_range` is unavailable, the descriptors a script plugin inherits,
//! and the shell start-up names a binding may not allow-list. Every plugin is
//! a fixture in the test's own temporary directory; no real credential is
//! used.
#![cfg(target_os = "linux")]
use connectors_kubernetes::exec_credential::{Binding, Credentials, Refusal, rfc3339_ms};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, mpsc},
    time::Duration,
};

const USER: &str = "eks-fixture-user";
const V1: &str = "client.authentication.k8s.io/v1";
const LEAVER: &str = "FIXTURE_GROUP_LEAVER_RECORD";
const NO_CLOSE_RANGE: &str = "FIXTURE_NO_CLOSE_RANGE_CHILD";

fn sha256(path: &Path) -> String {
    hex::encode(Sha256::digest(fs::read(path).unwrap()))
}
fn answer(token: &str) -> String {
    format!(r#"{{"apiVersion":"{V1}","kind":"ExecCredential","status":{{"token":"{token}"}}}}"#)
}
fn binding(path: &Path, env: &[&str], timeout_ms: u64) -> Binding {
    Binding {
        path: path.to_path_buf(),
        sha256: sha256(path),
        user: USER.into(),
        env: env.iter().map(|n| (*n).to_owned()).collect(),
        home: None,
        timeout_ms: Some(timeout_ms),
    }
}
/// A kubeconfig whose user runs `command` with `args` and `env`; every
/// scalar is a YAML single-quoted string, so none may contain a single quote.
fn kubeconfig(command: &Path, args: &[&str], env: &[(&str, &str)]) -> Vec<u8> {
    let quote = |s: &str| {
        assert!(!s.contains('\''));
        format!("'{s}'")
    };
    let args: Vec<String> = args.iter().map(|a| quote(a)).collect();
    let mut out = format!(
        "users:\n- name: {USER}\n  user:\n    exec:\n      apiVersion: {V1}\n      \
         command: {}\n      args: [{}]\n",
        command.display(),
        args.join(", ")
    );
    if !env.is_empty() {
        out.push_str("      env:\n");
        for (name, value) in env {
            out.push_str(&format!(
                "      - name: {name}\n        value: {}\n",
                quote(value)
            ));
        }
    }
    out.into_bytes()
}
fn now() -> u64 {
    rfc3339_ms("2029-01-01T00:00:00Z").unwrap()
}
fn no_ambient(_: &str) -> Option<String> {
    None
}
fn private_copy(source: &Path, destination: &Path) {
    fs::copy(source, destination).unwrap();
    fs::set_permissions(destination, fs::Permissions::from_mode(0o700)).unwrap();
}
/// Descriptor numbers listed in `out` by the fixture's `/proc/$$/fd` loop.
fn listed(out: &Path) -> Vec<i32> {
    fs::read_to_string(out)
        .unwrap()
        .split_whitespace()
        .map(|n| n.parse().unwrap())
        .collect()
}
/// A shell loop that lists the shell's own open descriptors into `out`.
fn list_descriptors(out: &Path) -> String {
    format!(
        "f=; for p in /proc/$$/fd/*; do [ -e \"$p\" ] && f=\"$f ${{p##*/}}\"; done; echo \"$f\" > {}",
        out.display()
    )
}
/// Opens a fixture file in the adapter (this process) without close-on-exec
/// at a fixed high descriptor number, as a descriptor the adapter inherited
/// from whatever started it would be.
fn hold_descriptor(dir: &Path, at: i32) -> i32 {
    let held = dir.join("held-by-the-adapter");
    fs::write(&held, b"adapter-private").unwrap();
    let c_path = std::ffi::CString::new(held.as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: a NUL-terminated path in the test's own directory; no O_CLOEXEC.
    let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDONLY) };
    assert!(fd >= 0);
    // SAFETY: both descriptors are this process's; dup2 leaves `at` without
    // close-on-exec.
    assert_eq!(unsafe { libc::dup2(fd, at) }, at);
    // SAFETY: fd was opened above and is closed once.
    unsafe { libc::close(fd) };
    at
}

// ---------------------------------------------------------------------------
// The deadline when the plugin itself leaves its process group.
// ---------------------------------------------------------------------------

/// Not a check: the plugin body for
/// `exec_plugin_that_leaves_its_group_is_released_at_the_deadline`. This
/// test binary, copied and digest-pinned, is run as a native plugin with
/// `LEAVER` in its environment; it moves its own process out of the group
/// the adapter made for it, into its parent's group (same session), and
/// sleeps. Without `LEAVER` it does nothing.
#[test]
fn fixture_group_leaver() {
    let Ok(record) = std::env::var(LEAVER) else {
        return;
    };
    // SAFETY: getppid, getpgid and setpgid act on this process only.
    let left = unsafe {
        let group = libc::getpgid(libc::getppid());
        group > 0 && libc::setpgid(0, group) == 0
    };
    fs::write(
        &record,
        format!(
            "{} {}\n",
            std::process::id(),
            if left { "left" } else { "stayed" }
        ),
    )
    .unwrap();
    std::thread::sleep(Duration::from_secs(20));
}

/// Contract: "One run is bounded by `timeout_ms`; at the deadline the
/// plugin's whole process group is killed, and the caller is released then";
/// ESS `timeout`: "the caller is released at the deadline". The plugin's own
/// process moves itself into another process group before the deadline. The
/// group kill at the deadline (`kill(-pid)`) then reaches nothing, and the
/// run must still release the caller at the deadline rather than wait for
/// the plugin to exit on its own.
#[test]
fn exec_plugin_that_leaves_its_group_is_released_at_the_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let plugin = dir.path().join("fixture-plugin");
    private_copy(&std::env::current_exe().unwrap(), &plugin);
    let record = dir.path().join("record");
    let config = kubeconfig(
        &plugin,
        &[
            "fixture_group_leaver",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ],
        &[(LEAVER, record.to_str().unwrap())],
    );
    let credentials = Arc::new(Credentials::new(binding(&plugin, &[LEAVER], 1_500)).unwrap());
    let (send, receive) = mpsc::channel();
    let worker = {
        let credentials = credentials.clone();
        std::thread::spawn(move || {
            let result = credentials.token(&config, &now, &no_ambient).map(|_| ());
            let _ = send.send(result);
        })
    };
    // Deadline 1.5 s; 8 s is far past any scheduling slack.
    let outcome = receive.recv_timeout(Duration::from_secs(8));
    let written = fs::read_to_string(&record).unwrap_or_default();
    if outcome.is_err()
        && let Some(pid) = written
            .split_whitespace()
            .next()
            .and_then(|p| p.parse::<i32>().ok())
    {
        // SAFETY: the pid is this test's own fixture plugin, still unreaped
        // (the worker is blocked waiting on it), so it names nothing else.
        unsafe { libc::kill(pid, libc::SIGKILL) };
    }
    worker.join().unwrap();
    assert!(
        written.contains("left"),
        "precondition: the fixture plugin did not leave its group: {written:?}"
    );
    assert_eq!(
        outcome,
        Ok(Err(Refusal::Timeout)),
        "the run was not released at its 1500 ms deadline: the plugin's own process left \
         its group, the group kill reached nothing and the run waited on the plugin \
         (the fixture was killed by this test after 8 s)"
    );
}

// ---------------------------------------------------------------------------
// Descriptors.
// ---------------------------------------------------------------------------

/// Installs a seccomp filter on this thread that answers `close_range` with
/// ENOSYS, as a kernel before 5.9 does. Irreversible: only ever called in a
/// child process of this binary.
fn deny_close_range() {
    const BPF_LD_W_ABS: u16 = 0x20;
    const BPF_JEQ_K: u16 = 0x15;
    const BPF_RET_K: u16 = 0x06;
    const RET_ALLOW: u32 = 0x7fff_0000;
    const RET_ERRNO: u32 = 0x0005_0000;
    let filter = [
        // seccomp_data.nr is the first field.
        libc::sock_filter {
            code: BPF_LD_W_ABS,
            jt: 0,
            jf: 0,
            k: 0,
        },
        libc::sock_filter {
            code: BPF_JEQ_K,
            jt: 0,
            jf: 1,
            k: libc::SYS_close_range as u32,
        },
        libc::sock_filter {
            code: BPF_RET_K,
            jt: 0,
            jf: 0,
            k: RET_ERRNO | libc::ENOSYS as u32,
        },
        libc::sock_filter {
            code: BPF_RET_K,
            jt: 0,
            jf: 0,
            k: RET_ALLOW,
        },
    ];
    let program = libc::sock_fprog {
        len: filter.len() as u16,
        filter: filter.as_ptr() as *mut libc::sock_filter,
    };
    // SAFETY: prctl with a valid, live filter program for this thread.
    unsafe {
        assert_eq!(libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0), 0);
        assert_eq!(
            libc::prctl(
                libc::PR_SET_SECCOMP,
                libc::SECCOMP_MODE_FILTER,
                &program as *const libc::sock_fprog
            ),
            0,
            "{}",
            std::io::Error::last_os_error()
        );
        // Self-check: close_range is now unavailable to this thread.
        assert_eq!(
            libc::syscall(
                libc::SYS_close_range,
                1_000_000 as libc::c_uint,
                1_000_000 as libc::c_uint,
                0 as libc::c_uint
            ),
            -1
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ENOSYS)
        );
    }
}

/// The fix's fallback (`exec_credential.rs`, the fcntl loop): where
/// `close_range` is unavailable, a native plugin must still inherit no
/// descriptor of the adapter. Measured in a child process of this binary
/// whose thread answers `close_range` with ENOSYS.
#[test]
fn exec_plugin_native_inherits_no_descriptor_without_close_range() {
    let Some(bash) = ["/usr/bin/bash", "/bin/bash"]
        .into_iter()
        .find(|p| Path::new(p).exists())
    else {
        eprintln!("skipped: no bash to copy as a native plugin");
        return;
    };
    if std::env::var(NO_CLOSE_RANGE).is_ok() {
        deny_close_range();
        let dir = tempfile::tempdir().unwrap();
        let plugin = dir.path().join("bash");
        private_copy(&fs::canonicalize(bash).unwrap(), &plugin);
        let held = hold_descriptor(dir.path(), 900);
        let out = dir.path().join("fds");
        let token = "fixture-fallback-token";
        let program = format!("{}; printf %s \"$1\"", list_descriptors(&out));
        let config = kubeconfig(&plugin, &["-c", &program, "plugin", &answer(token)], &[]);
        let credentials = Credentials::new(binding(&plugin, &[], 5_000)).unwrap();
        let result = credentials.token(&config, &now, &no_ambient);
        assert_eq!(
            result.map(|t| String::from_utf8(t.to_vec()).unwrap()),
            Ok(token.to_owned())
        );
        let open = listed(&out);
        assert!(
            !open.contains(&held),
            "without close_range the native plugin inherited the adapter's descriptor {held}; \
             its open descriptors: {open:?}"
        );
        return;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "exec_plugin_native_inherits_no_descriptor_without_close_range",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(NO_CLOSE_RANGE, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Contract: "Besides that one descriptor and its standard input, output and
/// error, the plugin inherits no descriptor of the adapter". A script plugin
/// keeps only its own plugin descriptor; a descriptor the adapter holds
/// without close-on-exec does not reach it.
#[test]
fn exec_plugin_script_inherits_only_its_own_descriptor() {
    let dir = tempfile::tempdir().unwrap();
    let plugin = dir.path().join("fixture-plugin");
    let out = dir.path().join("fds");
    let token = "fixture-script-descriptor-token";
    fs::write(
        &plugin,
        format!(
            "#!/bin/sh\n{}\nprintf '%s' '{}'\n",
            list_descriptors(&out),
            answer(token)
        ),
    )
    .unwrap();
    fs::set_permissions(&plugin, fs::Permissions::from_mode(0o700)).unwrap();
    let held = hold_descriptor(dir.path(), 901);
    let credentials = Credentials::new(binding(&plugin, &[], 5_000)).unwrap();
    let result = credentials.token(&kubeconfig(&plugin, &[], &[]), &now, &no_ambient);
    // SAFETY: held was opened by hold_descriptor and is closed once.
    unsafe { libc::close(held) };
    assert_eq!(
        result.map(|t| String::from_utf8(t.to_vec()).unwrap()),
        Ok(token.to_owned())
    );
    let open = listed(&out);
    assert!(
        !open.contains(&held),
        "the script plugin inherited the adapter's descriptor {held}; its open descriptors: {open:?}"
    );
}

// ---------------------------------------------------------------------------
// Refused environment names.
// ---------------------------------------------------------------------------

/// Contract (`ExecPluginEnvName`): the binding's allow-list refuses "every
/// name that makes a process load or run code it was not given, or changes
/// how its loader or shell starts", shell start-up included. `ZDOTDIR` is
/// zsh's start-up directory: zsh runs `$ZDOTDIR/.zshenv` at every start,
/// scripts included. A binding naming it must be refused when it loads. If
/// it is admitted, the run below shows what that admits: the kubeconfig's
/// value makes the pinned zsh plugin run a file that was never hashed.
#[test]
fn exec_plugin_binding_refuses_the_zsh_start_up_directory() {
    let Some(zsh) = ["/usr/bin/zsh", "/bin/zsh"]
        .into_iter()
        .find(|p| Path::new(p).exists())
    else {
        // The admissibility half needs no zsh.
        let dir = tempfile::tempdir().unwrap();
        let plugin = dir.path().join("fixture-plugin");
        fs::write(&plugin, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&plugin, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(
            Credentials::new(binding(&plugin, &["ZDOTDIR"], 5_000)).is_none(),
            "a binding allow-listing ZDOTDIR was admitted"
        );
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let plugin: PathBuf = dir.path().join("fixture-plugin");
    fs::write(
        &plugin,
        format!(
            "#!{zsh}\nprint -r -- '{}'\n",
            answer("fixture-pinned-token")
        ),
    )
    .unwrap();
    fs::set_permissions(&plugin, fs::Permissions::from_mode(0o700)).unwrap();
    let startup = dir.path().join("startup");
    fs::create_dir(&startup).unwrap();
    fs::write(
        startup.join(".zshenv"),
        format!(
            "print -r -- '{}'\nexit 0\n",
            answer("fixture-unhashed-token")
        ),
    )
    .unwrap();
    let Some(credentials) = Credentials::new(binding(&plugin, &["ZDOTDIR"], 5_000)) else {
        return;
    };
    let config = kubeconfig(&plugin, &[], &[("ZDOTDIR", startup.to_str().unwrap())]);
    let answered = credentials
        .token(&config, &now, &no_ambient)
        .map(|t| String::from_utf8(t.to_vec()).unwrap());
    panic!(
        "a binding allow-listing ZDOTDIR was admitted; with the kubeconfig's ZDOTDIR the \
         pinned zsh plugin answered {answered:?}"
    );
}

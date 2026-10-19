//! The pinned exec credential plugin, run directly: digest pin, empty
//! environment, deadline, output bound, answer interpretation and in-memory
//! reuse until expiry. The plugin is a fixture shell script written into the
//! test's own temporary directory and digest-pinned there; it uses shell
//! builtins only, so it needs no PATH. No real credential is used.
#![cfg(unix)]
use connectors_kubernetes::exec_credential::{Binding, Credentials, Refusal, rfc3339_ms};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Fictional fixture material only.
const TOKEN: &str = "fixture-exec-plugin-token";
const USER: &str = "eks-fixture-user";
const LATER: &str = "2030-01-01T00:00:00Z";

/// Puts `contents` at `path`, mode 0700, through a `cp` child: this process
/// never holds a file it will execute open for writing. The tests run on
/// parallel threads and every plugin run forks; a child forked while this
/// process held the file open keeps that descriptor until its own exec, and
/// an exec of the file meanwhile fails with `ETXTBSY`
/// (`PluginUnavailable(Some(ExecutableFileBusy))`). `cp` has exited, and
/// closed the file, when this returns.
fn install(path: &Path, contents: &[u8]) {
    let staged = path.with_extension("staged");
    fs::write(&staged, contents).unwrap();
    copy(&staged, path);
    fs::remove_file(&staged).unwrap();
}
/// `cp source destination`, then mode 0700: see [`install`].
fn copy(source: &Path, destination: &Path) {
    let status = std::process::Command::new("cp")
        .arg(source)
        .arg(destination)
        .status()
        .unwrap();
    assert!(status.success(), "cp: {status}");
    fs::set_permissions(destination, fs::Permissions::from_mode(0o700)).unwrap();
}

struct Plugin {
    dir: tempfile::TempDir,
    path: PathBuf,
}
impl Plugin {
    /// `body` runs after the script records one run and its environment.
    fn new(body: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture-plugin");
        let d = dir.path().display();
        let script = format!("#!/bin/sh\necho run >> {d}/runs\nexport -p > {d}/env\n{body}\n");
        install(&path, script.as_bytes());
        fs::create_dir(dir.path().join("home")).unwrap();
        Self { dir, path }
    }
    /// Answer with the ExecCredential document written by [`Plugin::answer`].
    fn answering() -> Self {
        let plugin = Self::new("");
        let d = plugin.dir.path().display();
        let script = fs::read_to_string(&plugin.path).unwrap()
            + &format!("read -r line < {d}/answer\nprintf '%s\\n' \"$line\"\n");
        install(&plugin.path, script.as_bytes());
        plugin
    }
    fn answer(&self, expiry: Option<&str>) {
        let status = match expiry {
            Some(at) => format!(r#"{{"token":"{TOKEN}","expirationTimestamp":"{at}"}}"#),
            None => format!(r#"{{"token":"{TOKEN}"}}"#),
        };
        self.answer_raw(&format!(
            r#"{{"apiVersion":"client.authentication.k8s.io/v1","kind":"ExecCredential","status":{status}}}"#
        ));
    }
    fn answer_raw(&self, document: &str) {
        fs::write(self.dir.path().join("answer"), document).unwrap();
    }
    fn sha256(&self) -> String {
        hex::encode(Sha256::digest(fs::read(&self.path).unwrap()))
    }
    fn binding(&self) -> Binding {
        Binding {
            path: self.path.clone(),
            sha256: self.sha256(),
            user: USER.into(),
            env: vec!["ALLOWED_FROM_KUBECONFIG".into(), "CARGO_PKG_NAME".into()],
            home: Some(self.dir.path().join("home")),
            timeout_ms: Some(2_000),
        }
    }
    fn runs(&self) -> usize {
        fs::read_to_string(self.dir.path().join("runs"))
            .map(|s| s.lines().count())
            .unwrap_or(0)
    }
    /// The plugin's `export -p` listing, one `NAME="value"` line per variable:
    /// bash prints `declare -x NAME="value"` and dash `export NAME='value'`.
    /// A value holding a newline spans several lines of the listing; its
    /// continuation lines are joined back, the newline written as `\n`, so no
    /// continuation is ever read as a variable of its own.
    fn environment(&self) -> String {
        let raw = fs::read_to_string(self.dir.path().join("env")).unwrap();
        let mut listing = String::new();
        let mut record = String::new();
        let mut quote = None;
        for line in raw.lines() {
            if !record.is_empty() {
                record.push_str("\\n");
            }
            record.push_str(line);
            let mut escaped = false;
            for c in line.chars() {
                match quote {
                    Some('\'') if c == '\'' => quote = None,
                    Some('\'') => {}
                    _ if escaped => escaped = false,
                    _ if c == '\\' => escaped = true,
                    Some(_) if c == '"' => quote = None,
                    None if c == '\'' || c == '"' => quote = Some(c),
                    _ => {}
                }
            }
            if quote.is_none() {
                let line = record
                    .strip_prefix("declare -x ")
                    .or_else(|| record.strip_prefix("export "))
                    .unwrap_or(&record);
                match line.split_once("='") {
                    Some((name, value)) if value.ends_with('\'') => {
                        listing.push_str(&format!("{name}=\"{}\"\n", &value[..value.len() - 1]));
                    }
                    _ => listing.push_str(&format!("{line}\n")),
                }
                record.clear();
            }
        }
        listing
    }
}

fn kubeconfig(command: &Path, env: &[(&str, &str)]) -> Vec<u8> {
    let mut entries = String::new();
    for (name, value) in env {
        entries.push_str(&format!("      - name: {name}\n        value: {value}\n"));
    }
    let env = if entries.is_empty() {
        String::new()
    } else {
        format!("      env:\n{entries}")
    };
    format!(
        "apiVersion: v1\nkind: Config\nusers:\n- name: other-user\n  user:\n    token: not-this-one\n\
         - name: {USER}\n  user:\n    exec:\n      apiVersion: client.authentication.k8s.io/v1\n      \
         command: {}\n      args: [\"eks\", \"get-token\", \"--cluster-name\", \"fixture\"]\n{env}",
        command.display()
    )
    .into_bytes()
}

fn ambient(name: &str) -> Option<String> {
    std::env::var(name).ok()
}
fn clock(at: &str) -> impl Fn() -> u64 {
    let ms = rfc3339_ms(at).unwrap();
    move || ms
}
fn token(
    credentials: &Credentials,
    kubeconfig: &[u8],
    now: &dyn Fn() -> u64,
) -> Result<String, Refusal> {
    credentials
        .token(kubeconfig, now, &ambient)
        .map(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
}

#[test]
fn exec_plugin_answer_is_the_bearer_token() {
    let plugin = Plugin::answering();
    plugin.answer(Some(LATER));
    let credentials = Credentials::new(plugin.binding()).unwrap();
    let config = kubeconfig(&plugin.path, &[]);
    assert_eq!(
        token(&credentials, &config, &clock("2029-01-01T00:00:00Z")),
        Ok(TOKEN.to_owned())
    );
    assert_eq!(plugin.runs(), 1);
    // The kubeconfig may name the plugin by its bare file name as well.
    let bare = Credentials::new(plugin.binding()).unwrap();
    let by_name = kubeconfig(Path::new("fixture-plugin"), &[]);
    assert_eq!(
        token(&bare, &by_name, &clock("2029-01-01T00:00:00Z")),
        Ok(TOKEN.to_owned())
    );
}

#[test]
fn exec_plugin_digest_mismatch_refuses_before_the_process_runs() {
    let plugin = Plugin::answering();
    plugin.answer(Some(LATER));
    let mut binding = plugin.binding();
    binding.sha256 = hex::encode(Sha256::digest(b"some other plugin"));
    let credentials = Credentials::new(binding).unwrap();
    let config = kubeconfig(&plugin.path, &[]);
    assert_eq!(
        token(&credentials, &config, &clock("2029-01-01T00:00:00Z")),
        Err(Refusal::DigestMismatch)
    );
    assert_eq!(plugin.runs(), 0);
    // Pinned correctly, then replaced: verified again before the next run.
    let credentials = Credentials::new(plugin.binding()).unwrap();
    let mut script = fs::read_to_string(&plugin.path).unwrap();
    script.push_str("# replaced\n");
    install(&plugin.path, script.as_bytes());
    assert_eq!(
        token(&credentials, &config, &clock("2029-01-01T00:00:00Z")),
        Err(Refusal::DigestMismatch)
    );
    assert_eq!(plugin.runs(), 0);
}

#[test]
fn exec_plugin_sees_only_allow_listed_environment() {
    let plugin = Plugin::answering();
    plugin.answer(Some(LATER));
    let credentials = Credentials::new(plugin.binding()).unwrap();
    let config = kubeconfig(
        &plugin.path,
        &[("ALLOWED_FROM_KUBECONFIG", "from-kubeconfig")],
    );
    token(&credentials, &config, &clock("2029-01-01T00:00:00Z")).unwrap();
    let env = plugin.environment();
    // Named by the allow-list: from the kubeconfig, then from the adapter.
    assert!(
        env.contains("ALLOWED_FROM_KUBECONFIG=\"from-kubeconfig\""),
        "{env}"
    );
    assert!(
        env.contains("CARGO_PKG_NAME=\"connectors-kubernetes\""),
        "{env}"
    );
    let home = plugin.dir.path().join("home");
    assert!(
        env.contains(&format!("HOME=\"{}\"", home.display())),
        "{env}"
    );
    assert!(env.contains("KUBERNETES_EXEC_INFO="), "{env}");
    // Set in the test process, and not named: not visible.
    assert!(std::env::var("CARGO_MANIFEST_DIR").is_ok());
    assert!(!env.contains("CARGO_MANIFEST_DIR"), "{env}");
    assert!(!env.contains("CARGO_PKG_VERSION"), "{env}");
    // A kubeconfig env name outside the allow-list refuses; nothing runs.
    let fresh = Credentials::new(plugin.binding()).unwrap();
    let smuggled = kubeconfig(&plugin.path, &[("LD_PRELOAD", "/fixture/evil.so")]);
    assert_eq!(
        token(&fresh, &smuggled, &clock("2029-01-01T00:00:00Z")),
        Err(Refusal::KubeconfigMismatch)
    );
    assert_eq!(plugin.runs(), 1);
    // Without a configured home the plugin gets no HOME at all.
    let mut binding = plugin.binding();
    binding.home = None;
    let homeless = Credentials::new(binding).unwrap();
    token(&homeless, &config, &clock("2029-01-01T00:00:00Z")).unwrap();
    assert!(!plugin.environment().contains("HOME="));
}

#[test]
fn exec_plugin_cannot_be_selected_by_the_kubeconfig() {
    let plugin = Plugin::answering();
    plugin.answer(Some(LATER));
    let credentials = Credentials::new(plugin.binding()).unwrap();
    for config in [
        kubeconfig(Path::new("/usr/local/bin/aws"), &[]),
        kubeconfig(Path::new("aws"), &[]),
        // The configured user has no exec block.
        kubeconfig(&plugin.path, &[])
            .split(|b| *b == b'\n')
            .take(4)
            .collect::<Vec<_>>()
            .join(&b'\n'),
    ] {
        assert_eq!(
            token(&credentials, &config, &clock("2029-01-01T00:00:00Z")),
            Err(Refusal::KubeconfigMismatch)
        );
    }
    assert_eq!(plugin.runs(), 0);
}

fn dead(pid: i32) -> bool {
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Err(_) => true,
        // The state field follows the parenthesised command name.
        Ok(stat) => stat
            .rsplit_once(')')
            .is_some_and(|(_, rest)| rest.trim_start().starts_with('Z')),
    }
}

#[test]
#[cfg(target_os = "linux")]
fn exec_plugin_deadline_overrun_is_refused_and_killed() {
    let sleep = ["/usr/bin/sleep", "/bin/sleep"]
        .into_iter()
        .find(|p| Path::new(p).exists())
        .unwrap();
    // Under load the deadline can pass before the plugin's shell has started
    // its grandchild (observed at load average 77 on 20 cores: no `pid`
    // file). Such a run is still a timeout, but it shows nothing about the
    // group; the scenario then runs again with a fresh plugin.
    for attempt in 1.. {
        let plugin = Plugin::new("");
        let d = plugin.dir.path().display();
        let script = fs::read_to_string(&plugin.path).unwrap()
            + &format!("echo $$ > {d}/pid\n{sleep} 30 &\necho $! > {d}/grandchild\nwait\n");
        install(&plugin.path, script.as_bytes());
        let mut binding = plugin.binding();
        binding.timeout_ms = Some(300);
        let credentials = Credentials::new(binding).unwrap();
        let config = kubeconfig(&plugin.path, &[]);
        let started = Instant::now();
        assert_eq!(
            token(&credentials, &config, &clock("2029-01-01T00:00:00Z")),
            Err(Refusal::Timeout)
        );
        assert!(started.elapsed() < Duration::from_secs(5));
        // The plugin is reaped when the run returns, so what it wrote is all
        // it will ever write.
        let (Some(pid), Some(grandchild)) = (
            pid_in(&plugin.dir.path().join("pid")),
            pid_in(&plugin.dir.path().join("grandchild")),
        ) else {
            assert!(
                attempt < 10,
                "in 10 runs the plugin never started its grandchild before the deadline"
            );
            continue;
        };
        // The whole process group is killed, the plugin's own children included.
        let until = Instant::now() + Duration::from_secs(5);
        while !(dead(pid) && dead(grandchild)) {
            assert!(Instant::now() < until, "plugin process group survived");
            std::thread::sleep(Duration::from_millis(20));
        }
        return;
    }
}

/// The process id a fixture wrote to `path`, if it got as far as writing it.
fn pid_in(path: &Path) -> Option<i32> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

#[test]
fn exec_plugin_oversized_output_is_refused() {
    let plugin = Plugin::new("i=0\nwhile [ $i -lt 2000 ]; do printf '%064d' 0; i=$((i+1)); done");
    let credentials = Credentials::new(plugin.binding()).unwrap();
    let config = kubeconfig(&plugin.path, &[]);
    assert_eq!(
        token(&credentials, &config, &clock("2029-01-01T00:00:00Z")),
        Err(Refusal::OutputTooLarge)
    );
}

#[test]
fn exec_plugin_malformed_answers_and_failures_are_refused() {
    let plugin = Plugin::answering();
    let config = kubeconfig(&plugin.path, &[]);
    for answer in [
        "not json",
        r#"{"apiVersion":"client.authentication.k8s.io/v1","kind":"ExecCredential","status":{}}"#,
        r#"{"apiVersion":"client.authentication.k8s.io/v1beta1","kind":"ExecCredential","status":{"token":"t"}}"#,
        r#"{"apiVersion":"client.authentication.k8s.io/v1","kind":"Other","status":{"token":"t"}}"#,
        r#"{"apiVersion":"client.authentication.k8s.io/v1","kind":"ExecCredential","status":{"token":"has space"}}"#,
        r#"{"apiVersion":"client.authentication.k8s.io/v1","kind":"ExecCredential","status":{"token":"t","expirationTimestamp":"tomorrow"}}"#,
        r#"{"apiVersion":"client.authentication.k8s.io/v1","kind":"ExecCredential","status":{"clientCertificateData":"x","clientKeyData":"y"}}"#,
    ] {
        plugin.answer_raw(answer);
        let credentials = Credentials::new(plugin.binding()).unwrap();
        assert_eq!(
            token(&credentials, &config, &clock("2029-01-01T00:00:00Z")),
            Err(Refusal::MalformedCredential),
            "{answer}"
        );
    }
    let failing = Plugin::new("exit 3");
    let credentials = Credentials::new(failing.binding()).unwrap();
    assert_eq!(
        token(
            &credentials,
            &kubeconfig(&failing.path, &[]),
            &clock("2029-01-01T00:00:00Z")
        ),
        Err(Refusal::PluginFailed)
    );
}

#[test]
fn exec_plugin_token_is_reused_until_expiry_then_the_plugin_runs_again() {
    let plugin = Plugin::answering();
    plugin.answer(Some(LATER));
    let credentials = Credentials::new(plugin.binding()).unwrap();
    let config = kubeconfig(&plugin.path, &[]);
    for _ in 0..3 {
        token(&credentials, &config, &clock("2029-12-31T23:59:59Z")).unwrap();
    }
    assert_eq!(plugin.runs(), 1);
    // At the stated expiry the held token is not used again.
    token(&credentials, &config, &clock(LATER)).unwrap();
    assert_eq!(plugin.runs(), 2);
    // An answer with no expiry serves only the use that ran the plugin.
    plugin.answer(None);
    let unstated = Credentials::new(plugin.binding()).unwrap();
    token(&unstated, &config, &clock("2029-01-01T00:00:00Z")).unwrap();
    token(&unstated, &config, &clock("2029-01-01T00:00:00Z")).unwrap();
    assert_eq!(plugin.runs(), 4);
}

#[test]
fn exec_plugin_binding_rules_are_checked_before_anything_runs() {
    let plugin = Plugin::answering();
    let good = plugin.binding();
    assert!(good.admissible());
    let mut cases = Vec::new();
    let mut relative = good.clone();
    relative.path = PathBuf::from("fixture-plugin");
    cases.push(relative);
    let mut short = good.clone();
    short.sha256 = "abc".into();
    cases.push(short);
    let mut upper = good.clone();
    upper.sha256 = upper.sha256.to_uppercase();
    cases.push(upper);
    for name in ["HOME", "KUBERNETES_EXEC_INFO", "1BAD", "A-B", ""] {
        let mut env = good.clone();
        env.env = vec![name.into()];
        cases.push(env);
    }
    let mut duplicate = good.clone();
    duplicate.env = vec!["A".into(), "A".into()];
    cases.push(duplicate);
    let mut home = good.clone();
    home.home = Some(PathBuf::from("relative/home"));
    cases.push(home);
    for timeout in [0, 60_001] {
        let mut t = good.clone();
        t.timeout_ms = Some(timeout);
        cases.push(t);
    }
    for case in cases {
        assert!(Credentials::new(case).is_none());
    }
    assert_eq!(plugin.runs(), 0);
}

/// A native executable runs from its verified descriptor too, with the
/// kubeconfig arguments as its argument vector. The plugin is a copy of the
/// system `echo`, digest-pinned in the test's own directory; it is named
/// `echo` so a multi-call coreutils still dispatches on its file name.
#[test]
#[cfg(target_os = "linux")]
fn exec_plugin_native_executable_answers_through_its_descriptor() {
    let echo = ["/usr/bin/echo", "/bin/echo"]
        .into_iter()
        .find(|p| Path::new(p).exists())
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("echo");
    copy(&fs::canonicalize(echo).unwrap(), &path);
    assert_ne!(&fs::read(&path).unwrap()[..2], b"#!");
    let binding = Binding {
        sha256: hex::encode(Sha256::digest(fs::read(&path).unwrap())),
        path: path.clone(),
        user: USER.into(),
        env: Vec::new(),
        home: None,
        timeout_ms: Some(5_000),
    };
    let answer = format!(
        r#"{{"apiVersion":"client.authentication.k8s.io/v1","kind":"ExecCredential","status":{{"token":"{TOKEN}","expirationTimestamp":"{LATER}"}}}}"#
    );
    let config = format!(
        "users:\n- name: {USER}\n  user:\n    exec:\n      apiVersion: client.authentication.k8s.io/v1\n      \
         command: echo\n      args: ['{answer}']\n"
    );
    let credentials = Credentials::new(binding).unwrap();
    assert_eq!(
        token(
            &credentials,
            config.as_bytes(),
            &clock("2029-01-01T00:00:00Z")
        ),
        Ok(TOKEN.to_owned())
    );
}

#[test]
fn exec_plugin_symbolic_link_at_the_pinned_path_is_refused() {
    let plugin = Plugin::answering();
    plugin.answer(Some(LATER));
    let link = plugin.dir.path().join("linked-plugin");
    std::os::unix::fs::symlink(&plugin.path, &link).unwrap();
    let mut binding = plugin.binding();
    binding.path = link.clone();
    let credentials = Credentials::new(binding).unwrap();
    assert_eq!(
        token(
            &credentials,
            &kubeconfig(&link, &[]),
            &clock("2029-01-01T00:00:00Z")
        ),
        Err(Refusal::PluginUnavailable(Some(
            std::io::Error::from_raw_os_error(libc::ELOOP).kind()
        )))
    );
    assert_eq!(plugin.runs(), 0);
}

#[test]
fn exec_plugin_writable_or_set_id_file_is_refused() {
    let plugin = Plugin::answering();
    plugin.answer(Some(LATER));
    let config = kubeconfig(&plugin.path, &[]);
    for mode in [0o720, 0o702, 0o4700, 0o2700] {
        fs::set_permissions(&plugin.path, fs::Permissions::from_mode(mode)).unwrap();
        let credentials = Credentials::new(plugin.binding()).unwrap();
        assert_eq!(
            token(&credentials, &config, &clock("2029-01-01T00:00:00Z")),
            Err(Refusal::PluginUnavailable(None)),
            "{mode:o}"
        );
    }
    assert_eq!(plugin.runs(), 0);
    fs::set_permissions(&plugin.path, fs::Permissions::from_mode(0o700)).unwrap();
    let credentials = Credentials::new(plugin.binding()).unwrap();
    assert_eq!(
        token(&credentials, &config, &clock("2029-01-01T00:00:00Z")),
        Ok(TOKEN.to_owned())
    );
}

/// A FIFO at the path is not a regular file, and opening it does not wait
/// for a writer.
#[test]
#[cfg(target_os = "linux")]
fn exec_plugin_fifo_at_the_path_is_refused_without_blocking() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture-plugin");
    let c_path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: c_path is a NUL-terminated path in the test's own directory.
    assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
    let binding = Binding {
        path: path.clone(),
        sha256: hex::encode(Sha256::digest(b"")),
        user: USER.into(),
        env: Vec::new(),
        home: None,
        timeout_ms: Some(1_000),
    };
    let credentials = Credentials::new(binding).unwrap();
    let started = Instant::now();
    assert_eq!(
        token(
            &credentials,
            &kubeconfig(&path, &[]),
            &clock("2029-01-01T00:00:00Z")
        ),
        Err(Refusal::PluginUnavailable(None))
    );
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn rfc3339_timestamps_convert_to_unix_milliseconds() {
    assert_eq!(rfc3339_ms("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(rfc3339_ms("2030-01-01T00:00:00Z"), Some(1_893_456_000_000));
    assert_eq!(
        rfc3339_ms("2030-01-01T01:00:00.5+01:00"),
        Some(1_893_456_000_500)
    );
    assert_eq!(rfc3339_ms("2024-02-29T12:00:00Z"), Some(1_709_208_000_000));
    for bad in [
        "2023-02-29T00:00:00Z",
        "2030-01-01 00:00:00",
        "2030-01-01T00:00:00",
        "2030-13-01T00:00:00Z",
    ] {
        assert_eq!(rfc3339_ms(bad), None, "{bad}");
    }
    // A valid instant before the epoch is a past expiry: it reads as 0, which
    // no clock this adapter reads is before.
    for past in ["1969-12-31T23:59:59Z", "0001-01-01T00:00:00Z"] {
        assert_eq!(rfc3339_ms(past), Some(0), "{past}");
    }
    for bad in ["2030-01-01T00:00:00+24:00", "2030-01-01T00:00:00-00:60"] {
        assert_eq!(rfc3339_ms(bad), None, "{bad}");
    }
}

// ---------------------------------------------------------------------------
// Fix pass of wave 20261015b: the allow-list's refused names, file
// capabilities, the group after the run, the read end at the deadline and
// the kubeconfig read only when the plugin runs.
// ---------------------------------------------------------------------------

/// `ExecPluginEnvName`: a binding that allows a loader or start-up variable
/// is refused before anything runs; ordinary names, PATH among them, are not.
#[test]
fn exec_plugin_binding_refuses_loader_and_start_up_names() {
    let plugin = Plugin::answering();
    for name in [
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "LD_AUDIT",
        "BASH_ENV",
        "ENV",
        "SHELLOPTS",
        "PS4",
        "IFS",
        "PYTHONSTARTUP",
        "PYTHONPATH",
        "PERL5OPT",
        "RUBYOPT",
        "NODE_OPTIONS",
        "JAVA_TOOL_OPTIONS",
        "GCONV_PATH",
        "GLIBC_TUNABLES",
        "HOSTALIASES",
    ] {
        let mut binding = plugin.binding();
        binding.env = vec!["AWS_PROFILE".into(), name.into()];
        assert!(!binding.admissible(), "{name}");
        assert!(Credentials::new(binding).is_none(), "{name}");
    }
    for name in [
        "PATH",
        "AWS_PROFILE",
        "LD",
        "ENVIRONMENT",
        "LANG",
        "TZ",
        "ld_preload",
    ] {
        let mut binding = plugin.binding();
        binding.env = vec![name.into()];
        assert!(binding.admissible(), "{name}");
    }
    assert_eq!(plugin.runs(), 0);
}

/// The refused names are one list, written in the ESS model and enforced by
/// the code: every name and prefix the model's `ExecPluginEnvName` excludes
/// is refused by `Binding::admissible`, and the code's lists are exactly the
/// model's.
#[test]
fn exec_plugin_refused_env_names_are_the_ess_model_names() {
    use connectors_kubernetes::exec_credential::{REFUSED_ENV_NAMES, REFUSED_ENV_PREFIXES};
    use std::collections::BTreeSet;
    let model: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        &fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/spec/ess/domains/exec_credentials.yaml"
        ))
        .unwrap(),
    )
    .unwrap();
    let name = model["types"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "connectors_kubernetes.exec_credentials.ExecPluginEnvName")
        .expect("the model declares ExecPluginEnvName");
    let (mut names, mut prefixes) = (BTreeSet::new(), BTreeSet::new());
    for invariant in name["invariants"].as_sequence().unwrap() {
        let Some(refused) = invariant["none"].as_sequence() else {
            continue;
        };
        for predicate in refused {
            let value = &predicate["value"];
            if let Some(exact) = value["eq"].as_str() {
                names.insert(exact.to_owned());
            } else if let Some(prefix) = value["starts_with"].as_str() {
                // A leading digit is the name grammar, not a refused prefix.
                if !prefix.bytes().all(|b| b.is_ascii_digit()) {
                    prefixes.insert(prefix.to_owned());
                }
            } else {
                panic!("unread refusal {predicate:?}");
            }
        }
    }
    assert_eq!(
        names,
        REFUSED_ENV_NAMES.iter().map(|s| s.to_string()).collect()
    );
    assert_eq!(
        prefixes,
        REFUSED_ENV_PREFIXES.iter().map(|s| s.to_string()).collect()
    );
    let plugin = Plugin::answering();
    let refused = names
        .iter()
        .cloned()
        .chain(prefixes.iter().map(|p| format!("{p}FIXTURE")));
    for name in refused {
        let mut binding = plugin.binding();
        binding.env = vec![name.clone()];
        assert!(!binding.admissible(), "{name}");
    }
}

/// A file carrying file capabilities runs with privileges its owner did not
/// have, like a set-ID file, and is refused the same way. The capability is
/// set from an unprivileged user namespace, which the kernel records as a
/// namespaced (v3) `security.capability` attribute.
#[test]
#[cfg(target_os = "linux")]
fn exec_plugin_file_with_capabilities_is_refused() {
    let plugin = Plugin::answering();
    plugin.answer(Some(LATER));
    let set = std::process::Command::new("unshare")
        .args(["-Ur", "setcap", "cap_net_raw+ep"])
        .arg(&plugin.path)
        .output();
    if !set.is_ok_and(|o| o.status.success()) {
        eprintln!("skipped: no unprivileged user namespace or no setcap");
        return;
    }
    let credentials = Credentials::new(plugin.binding()).unwrap();
    assert_eq!(
        token(
            &credentials,
            &kubeconfig(&plugin.path, &[]),
            &clock("2029-01-01T00:00:00Z")
        ),
        Err(Refusal::PluginUnavailable(None))
    );
    assert_eq!(plugin.runs(), 0);
}

/// A plugin that answers and exits 0, leaving a process in its group that
/// no longer holds stdout: the run succeeds, and what it left behind is
/// killed with the group before the plugin is reaped.
#[test]
#[cfg(target_os = "linux")]
fn exec_plugin_group_left_behind_is_killed_when_the_run_ends() {
    let sleep = ["/usr/bin/sleep", "/bin/sleep"]
        .into_iter()
        .find(|p| Path::new(p).exists())
        .unwrap();
    let plugin = Plugin::new("");
    plugin.answer(Some(LATER));
    let d = plugin.dir.path().display();
    let script = fs::read_to_string(&plugin.path).unwrap()
        + &format!(
            "{sleep} 30 > /dev/null 2>&1 &\necho $! > {d}/left\n\
             read -r line < {d}/answer\nprintf '%s\\n' \"$line\"\nexit 0\n"
        );
    install(&plugin.path, script.as_bytes());
    let credentials = Credentials::new(plugin.binding()).unwrap();
    assert_eq!(
        token(
            &credentials,
            &kubeconfig(&plugin.path, &[]),
            &clock("2029-01-01T00:00:00Z")
        ),
        Ok(TOKEN.to_owned())
    );
    let left: i32 = fs::read_to_string(plugin.dir.path().join("left"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while !dead(left) {
        if Instant::now() >= until {
            // SAFETY: the pid is this test's own fixture process.
            unsafe { libc::kill(left, libc::SIGKILL) };
            panic!("a process the plugin left in its group survived the run");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// At the deadline the caller lets go of stdout: a descendant that left the
/// plugin's group and still holds it gets EPIPE on its next write, rather
/// than writing into a pipe some reader still keeps open.
#[test]
#[cfg(target_os = "linux")]
fn exec_plugin_deadline_closes_stdout_for_an_escaped_descendant() {
    // The plugin marks `ready` only once its descendant has left the group
    // (it wrote `escaped` after setsid). Under load the deadline can pass
    // before that; such a run is still a timeout but has no escaped
    // descendant to observe, and the scenario runs again with a fresh plugin.
    for attempt in 1.. {
        let plugin = Plugin::new("");
        let d = plugin.dir.path().display();
        let script = fs::read_to_string(&plugin.path).unwrap()
            + &format!(
                "/usr/bin/setsid /bin/sh -c 'trap \"\" PIPE; echo $$ > {d}/escaped; \
                 /usr/bin/sleep 1; echo late || : > {d}/epipe' &\n\
                 while [ ! -s {d}/escaped ]; do :; done\n: > {d}/ready\n/usr/bin/sleep 30\n"
            );
        install(&plugin.path, script.as_bytes());
        let mut binding = plugin.binding();
        binding.timeout_ms = Some(300);
        let credentials = Credentials::new(binding).unwrap();
        let started = Instant::now();
        assert_eq!(
            token(
                &credentials,
                &kubeconfig(&plugin.path, &[]),
                &clock("2029-01-01T00:00:00Z")
            ),
            Err(Refusal::Timeout)
        );
        assert!(started.elapsed() < Duration::from_secs(4));
        let escaped = pid_in(&plugin.dir.path().join("escaped"));
        if !plugin.dir.path().join("ready").exists() {
            if let Some(pid) = escaped {
                // SAFETY: the pid is this test's own fixture descendant.
                unsafe { libc::kill(pid, libc::SIGKILL) };
            }
            assert!(
                attempt < 10,
                "in 10 runs the plugin's descendant never left its group before the deadline"
            );
            continue;
        }
        let escaped = escaped.expect("ready is written only after escaped");
        // The descendant writes once more after its pause, then exits: its
        // write failed (EPIPE) if and only if `epipe` exists then.
        let until = Instant::now() + Duration::from_secs(30);
        while !dead(escaped) {
            if Instant::now() >= until {
                // SAFETY: the pid is this test's own fixture descendant.
                unsafe { libc::kill(escaped, libc::SIGKILL) };
                panic!("the escaped descendant did not finish within 30 s");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            plugin.dir.path().join("epipe").exists(),
            "the read end of the plugin's stdout outlived the timeout"
        );
        return;
    }
}

/// story:kubernetes-exec-fixture-busy: a plugin some process holds open for
/// writing cannot be executed (`ETXTBSY`), and a missing one cannot be
/// opened. Both are `plugin_unavailable`; the refusal carries the operating
/// system's error kind, so the two are told apart. Once the writer lets go,
/// the same plugin runs. The writer is a child process: a descriptor this
/// process held would reach every child another test forks meanwhile, and
/// keep the plugin busy after it was closed here (see [`install`]).
#[test]
#[cfg(target_os = "linux")]
fn exec_plugin_unavailable_names_a_busy_plugin_apart_from_a_missing_one() {
    use std::io::{BufRead, ErrorKind};
    let plugin = Plugin::answering();
    plugin.answer(Some(LATER));
    let config = kubeconfig(&plugin.path, &[]);
    let at = clock("2029-01-01T00:00:00Z");
    let mut writer = std::process::Command::new("/bin/sh")
        .args([
            "-c",
            "exec 3>>\"$1\" && echo held && read -r line",
            "writer",
        ])
        .arg(&plugin.path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut held = String::new();
    std::io::BufReader::new(writer.stdout.as_mut().unwrap())
        .read_line(&mut held)
        .unwrap();
    assert_eq!(held, "held\n");
    let credentials = Credentials::new(plugin.binding()).unwrap();
    assert_eq!(
        token(&credentials, &config, &at),
        Err(Refusal::PluginUnavailable(Some(
            ErrorKind::ExecutableFileBusy
        )))
    );
    assert_eq!(plugin.runs(), 0);
    writer.kill().unwrap();
    writer.wait().unwrap();
    assert_eq!(token(&credentials, &config, &at), Ok(TOKEN.to_owned()));
    assert_eq!(plugin.runs(), 1);
    let binding = plugin.binding();
    fs::remove_file(&plugin.path).unwrap();
    let missing = Credentials::new(binding).unwrap();
    assert_eq!(
        token(&missing, &config, &at),
        Err(Refusal::PluginUnavailable(Some(ErrorKind::NotFound)))
    );
}

/// The kubeconfig is read when the plugin runs and only then: a use the
/// held token serves never asks for it, and an unreadable kubeconfig is
/// `kubeconfig_mismatch` before anything runs.
#[test]
fn exec_plugin_held_token_reads_no_kubeconfig() {
    let plugin = Plugin::answering();
    plugin.answer(Some(LATER));
    let credentials = Credentials::new(plugin.binding()).unwrap();
    let config = kubeconfig(&plugin.path, &[]);
    let reads = std::cell::Cell::new(0);
    let load = || {
        reads.set(reads.get() + 1);
        Ok(zeroize::Zeroizing::new(config.clone()))
    };
    let now = clock("2029-01-01T00:00:00Z");
    for _ in 0..3 {
        let token = credentials.token_with(&load, &now, &ambient).unwrap();
        assert_eq!(&token[..], TOKEN.as_bytes());
    }
    assert_eq!((reads.get(), plugin.runs()), (1, 1));
    let fresh = Credentials::new(plugin.binding()).unwrap();
    assert_eq!(
        fresh
            .token_with(&|| Err(Refusal::KubeconfigMismatch), &now, &ambient)
            .err(),
        Some(Refusal::KubeconfigMismatch)
    );
    assert_eq!(plugin.runs(), 1);
}

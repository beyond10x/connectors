//! Adversary cases for the pinned exec credential plugin (wave 20261015b),
//! driven from `contracts/reads/v1alpha1/semantics.md`, "Exec-plugin
//! credentials", and the `exec_credentials` ESS domain: kubeconfig command
//! matching, the environment allow-list, the RFC 3339 expiry, the stdout and
//! token bounds, the deadline with descendants, concurrent uses, stderr,
//! exit status and malformed answers. Every plugin is a fixture shell script
//! in the test's own temporary directory; no real credential is used.
#![cfg(target_os = "linux")]
use connectors_kubernetes::exec_credential::{Binding, Credentials, Refusal, rfc3339_ms};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const TOKEN: &str = "fixture-adversary-token";
const USER: &str = "eks-adversary-user";
const V1: &str = "client.authentication.k8s.io/v1";
const V1BETA1: &str = "client.authentication.k8s.io/v1beta1";
const NOW: &str = "2029-01-01T00:00:00Z";
const LATER: &str = "2030-01-01T00:00:00Z";

struct Plugin {
    dir: tempfile::TempDir,
    path: PathBuf,
}
impl Plugin {
    /// A script named `name` that records one run and its environment, then
    /// runs `body` (with `{d}` replaced by its directory).
    fn named(name: &str, body: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        let d = dir.path().display().to_string();
        let script = format!(
            "#!/bin/sh\necho run >> {d}/runs\nexport -p > {d}/env\n{}\n",
            body.replace("{d}", &d)
        );
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self { dir, path }
    }
    fn new(body: &str) -> Self {
        Self::named("fixture-plugin", body)
    }
    /// Prints the `answer` file to stdout, without a trailing newline.
    fn answering() -> Self {
        Self::new(EMIT)
    }
    fn answer_raw(&self, document: &str) {
        fs::write(self.dir.path().join("answer"), document).unwrap();
    }
    fn answer(&self, api_version: &str, status: &str) {
        self.answer_raw(&format!(
            r#"{{"apiVersion":"{api_version}","kind":"ExecCredential","status":{status}}}"#
        ));
    }
    fn answer_expiring(&self, at: &str) {
        self.answer(
            V1,
            &format!(r#"{{"token":"{TOKEN}","expirationTimestamp":"{at}"}}"#),
        );
    }
    fn binding(&self, env: &[&str]) -> Binding {
        Binding {
            sha256: hex::encode(Sha256::digest(fs::read(&self.path).unwrap())),
            path: self.path.clone(),
            user: USER.into(),
            env: env.iter().map(|s| (*s).to_owned()).collect(),
            home: None,
            timeout_ms: Some(2_000),
        }
    }
    fn credentials(&self) -> Credentials {
        Credentials::new(self.binding(&[])).unwrap()
    }
    fn runs(&self) -> usize {
        fs::read_to_string(self.dir.path().join("runs"))
            .map(|s| s.lines().count())
            .unwrap_or(0)
    }
    fn environment(&self) -> String {
        fs::read_to_string(self.dir.path().join("env")).unwrap()
    }
    fn file(&self, name: &str) -> String {
        fs::read_to_string(self.dir.path().join(name)).unwrap()
    }
}
const EMIT: &str = "read -r line < {d}/answer\nprintf '%s' \"$line\"";

/// A kubeconfig whose `USER` entry runs `command` with `api_version`;
/// `env` entries are written as YAML double-quoted scalars.
fn kubeconfig_with(command: &str, api_version: &str, env: &[(&str, &str)]) -> Vec<u8> {
    let mut entries = String::new();
    for (name, value) in env {
        entries.push_str(&format!(
            "      - name: {name}\n        value: \"{value}\"\n"
        ));
    }
    let env = if entries.is_empty() {
        String::new()
    } else {
        format!("      env:\n{entries}")
    };
    format!(
        "apiVersion: v1\nkind: Config\nusers:\n- name: {USER}\n  user:\n    exec:\n      \
         apiVersion: {api_version}\n      command: '{command}'\n      args: [\"eks\", \"get-token\"]\n{env}"
    )
    .into_bytes()
}
fn kubeconfig(command: &Path) -> Vec<u8> {
    kubeconfig_with(command.to_str().unwrap(), V1, &[])
}

fn at(value: &str) -> impl Fn() -> u64 {
    let ms = rfc3339_ms(value).unwrap();
    move || ms
}
fn no_ambient(_: &str) -> Option<String> {
    None
}
fn token(credentials: &Credentials, kubeconfig: &[u8]) -> Result<String, Refusal> {
    credentials
        .token(kubeconfig, &at(NOW), &no_ambient)
        .map(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
}

/// The names in an `export -p` listing: bash prints `declare -x NAME=…`, and
/// `export NAME=…` when it runs as `/bin/sh` (POSIX mode); other shells print
/// the latter too.
fn exported(env: &str) -> Vec<&str> {
    env.lines()
        .filter_map(|l| {
            l.strip_prefix("declare -x ")
                .or_else(|| l.strip_prefix("export "))
        })
        .map(|l| l.split(['=', ' ']).next().unwrap())
        .collect()
}

fn dead(pid: i32) -> bool {
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Err(_) => true,
        Ok(stat) => stat
            .rsplit_once(')')
            .is_some_and(|(_, rest)| rest.trim_start().starts_with('Z')),
    }
}

// ---------------------------------------------------------------------------
// expirationTimestamp: "if present, a valid RFC 3339 expirationTimestamp"
// (ExecPluginRefusal, malformed_credential). A valid timestamp is not a
// malformed credential, whatever its value.
// ---------------------------------------------------------------------------

/// A valid RFC 3339 timestamp before the Unix epoch (Go's zero `time.Time`
/// among them, which a Go plugin marshalling a non-pointer time emits) is a
/// past expiry: the token serves the use that ran the plugin and is not
/// reused. It is not `malformed_credential`.
#[test]
fn adversary_expiry_before_the_epoch_is_a_past_expiry_not_malformed() {
    let plugin = Plugin::answering();
    for past in [
        "0001-01-01T00:00:00Z",
        "1969-12-31T23:59:59Z",
        "1970-01-01T00:30:00+01:00",
    ] {
        plugin.answer_expiring(past);
        assert_eq!(
            token(&plugin.credentials(), &kubeconfig(&plugin.path)),
            Ok(TOKEN.to_owned()),
            "{past}"
        );
    }
}

/// RFC 3339 `time-numoffset` is `time-hour ":" time-minute`, hour 00-23 and
/// minute 00-59. An offset outside that is not a valid timestamp.
#[test]
fn adversary_expiry_offset_out_of_range_is_not_rfc3339() {
    for bad in [
        "2030-01-01T00:00:00+24:00",
        "2030-01-01T00:00:00+00:60",
        "2030-01-01T00:00:00-99:99",
    ] {
        assert_eq!(rfc3339_ms(bad), None, "{bad}");
    }
    let plugin = Plugin::answering();
    plugin.answer_expiring("2030-01-01T00:00:00-99:99");
    assert_eq!(
        token(&plugin.credentials(), &kubeconfig(&plugin.path)),
        Err(Refusal::MalformedCredential)
    );
}

/// An offset expiry is the same instant as its UTC form: held before it,
/// the plugin runs again at it.
#[test]
fn adversary_expiry_with_offset_is_reused_until_that_instant() {
    let plugin = Plugin::answering();
    plugin.answer_expiring("2030-01-01T01:00:00+01:00");
    let credentials = plugin.credentials();
    let config = kubeconfig(&plugin.path);
    let before = at("2029-12-31T23:59:59.999Z");
    for _ in 0..2 {
        credentials.token(&config, &before, &no_ambient).unwrap();
    }
    assert_eq!(plugin.runs(), 1);
    credentials.token(&config, &at(LATER), &no_ambient).unwrap();
    assert_eq!(plugin.runs(), 2);
    // A null expiry is an absent one: served once, never held.
    plugin.answer(
        V1,
        &format!(r#"{{"token":"{TOKEN}","expirationTimestamp":null}}"#),
    );
    let fresh = plugin.credentials();
    token(&fresh, &config).unwrap();
    token(&fresh, &config).unwrap();
    assert_eq!(plugin.runs(), 4);
    // An empty string is not a timestamp.
    plugin.answer(
        V1,
        &format!(r#"{{"token":"{TOKEN}","expirationTimestamp":""}}"#),
    );
    assert_eq!(
        token(&plugin.credentials(), &config),
        Err(Refusal::MalformedCredential)
    );
}

// ---------------------------------------------------------------------------
// Bounds: stdout 65536 bytes, token 1 to 8192 printable ASCII bytes.
// ---------------------------------------------------------------------------

fn padded(total: usize) -> String {
    let prefix = format!(
        r#"{{"apiVersion":"{V1}","kind":"ExecCredential","status":{{"token":"{TOKEN}"}},"pad":""#
    );
    let suffix = r#""}"#;
    let pad = total - prefix.len() - suffix.len();
    let document = format!("{prefix}{}{suffix}", "x".repeat(pad));
    assert_eq!(document.len(), total);
    document
}

#[test]
fn adversary_stdout_of_exactly_64_kib_is_admitted() {
    let plugin = Plugin::answering();
    plugin.answer_raw(&padded(65_536));
    assert_eq!(
        token(&plugin.credentials(), &kubeconfig(&plugin.path)),
        Ok(TOKEN.to_owned())
    );
}

#[test]
fn adversary_stdout_one_byte_over_64_kib_is_refused() {
    let plugin = Plugin::answering();
    plugin.answer_raw(&padded(65_537));
    assert_eq!(
        token(&plugin.credentials(), &kubeconfig(&plugin.path)),
        Err(Refusal::OutputTooLarge)
    );
}

#[test]
fn adversary_token_length_boundary() {
    let plugin = Plugin::answering();
    let config = kubeconfig(&plugin.path);
    let longest = "a".repeat(8192);
    plugin.answer(V1, &format!(r#"{{"token":"{longest}"}}"#));
    assert_eq!(token(&plugin.credentials(), &config), Ok(longest));
    plugin.answer(V1, &format!(r#"{{"token":"{}"}}"#, "a".repeat(8193)));
    assert_eq!(
        token(&plugin.credentials(), &config),
        Err(Refusal::MalformedCredential)
    );
}

// ---------------------------------------------------------------------------
// Malformed answers.
// ---------------------------------------------------------------------------

#[test]
fn adversary_malformed_answers_are_refused() {
    let plugin = Plugin::answering();
    let config = kubeconfig(&plugin.path);
    for answer in [
        format!(r#"{{"apiVersion":"{V1}","kind":"ExecCredential"}}"#),
        format!(r#"{{"apiVersion":"{V1}","kind":"ExecCredential","status":null}}"#),
        format!(r#"{{"apiVersion":"{V1}","kind":"ExecCredential","status":{{"token":""}}}}"#),
        format!(r#"{{"apiVersion":"{V1}","kind":"ExecCredential","status":{{"token":null}}}}"#),
        format!(r#"{{"apiVersion":"{V1} ","kind":"ExecCredential","status":{{"token":"t"}}}}"#),
        format!(r#"{{"apiVersion":"{V1}","kind":"execcredential","status":{{"token":"t"}}}}"#),
        r#"{"kind":"ExecCredential","status":{"token":"t"}}"#.to_string(),
        format!(r#"{{"apiVersion":"{V1}","kind":"ExecCredential","status":{{"token":"té"}}}}"#),
        format!(r#"{{"apiVersion":"{V1}","kind":"ExecCredential","status":{{"token":"t"}}}} {{}}"#),
        String::new(),
    ] {
        plugin.answer_raw(&answer);
        assert_eq!(
            token(&plugin.credentials(), &config),
            Err(Refusal::MalformedCredential),
            "{answer}"
        );
    }
}

/// The kubeconfig requests v1beta1: the plugin is told v1beta1, a v1beta1
/// answer is admitted and a v1 answer is not.
#[test]
fn adversary_v1beta1_request_admits_only_a_v1beta1_answer() {
    let plugin = Plugin::answering();
    let config = kubeconfig_with(plugin.path.to_str().unwrap(), V1BETA1, &[]);
    plugin.answer(V1BETA1, &format!(r#"{{"token":"{TOKEN}"}}"#));
    assert_eq!(token(&plugin.credentials(), &config), Ok(TOKEN.to_owned()));
    let env = plugin.environment();
    assert!(env.contains(V1BETA1), "{env}");
    plugin.answer(V1, &format!(r#"{{"token":"{TOKEN}"}}"#));
    assert_eq!(
        token(&plugin.credentials(), &config),
        Err(Refusal::MalformedCredential)
    );
    // An apiVersion the decision does not name is a kubeconfig mismatch.
    let alpha = kubeconfig_with(
        plugin.path.to_str().unwrap(),
        "client.authentication.k8s.io/v1alpha1",
        &[],
    );
    let runs = plugin.runs();
    assert_eq!(
        token(&plugin.credentials(), &alpha),
        Err(Refusal::KubeconfigMismatch)
    );
    assert_eq!(plugin.runs(), runs);
}

// ---------------------------------------------------------------------------
// stderr and exit status.
// ---------------------------------------------------------------------------

#[test]
fn adversary_answer_on_stderr_only_is_not_a_credential() {
    let plugin = Plugin::new("read -r line < {d}/answer\nprintf '%s' \"$line\" >&2\nexit 0");
    plugin.answer(V1, &format!(r#"{{"token":"{TOKEN}"}}"#));
    assert_eq!(
        token(&plugin.credentials(), &kubeconfig(&plugin.path)),
        Err(Refusal::MalformedCredential)
    );
    let failing = Plugin::new("echo 'fixture failure' >&2\nexit 1");
    assert_eq!(
        token(&failing.credentials(), &kubeconfig(&failing.path)),
        Err(Refusal::PluginFailed)
    );
}

/// stderr is discarded, so a plugin chattering more than 64 KiB on stderr
/// does not count against the stdout bound and does not stall.
#[test]
fn adversary_large_stderr_does_not_count_against_the_stdout_bound() {
    let plugin = Plugin::new(
        "i=0\nwhile [ $i -lt 4000 ]; do printf '%064d' 0 >&2; i=$((i+1)); done\n\
         read -r line < {d}/answer\nprintf '%s' \"$line\"",
    );
    plugin.answer(V1, &format!(r#"{{"token":"{TOKEN}"}}"#));
    assert_eq!(
        token(&plugin.credentials(), &kubeconfig(&plugin.path)),
        Ok(TOKEN.to_owned())
    );
}

#[test]
fn adversary_valid_answer_with_failing_exit_is_plugin_failed() {
    for tail in ["exit 1", "exit 255", "kill -9 $$"] {
        let plugin = Plugin::new(&format!("{EMIT}\n{tail}"));
        plugin.answer(V1, &format!(r#"{{"token":"{TOKEN}"}}"#));
        assert_eq!(
            token(&plugin.credentials(), &kubeconfig(&plugin.path)),
            Err(Refusal::PluginFailed),
            "{tail}"
        );
    }
}

// ---------------------------------------------------------------------------
// The deadline with descendants that keep stdout open.
// ---------------------------------------------------------------------------

/// The plugin answers and exits 0, leaving a child in its process group that
/// holds stdout: the run does not finish, so it is `timeout`, and the child
/// is killed with the group.
#[test]
fn adversary_child_in_group_holding_stdout_is_killed_at_the_deadline() {
    let plugin = Plugin::new(&format!(
        "{EMIT}\n/usr/bin/sleep 30 &\necho $! > {{d}}/child\nexit 0"
    ));
    plugin.answer(V1, &format!(r#"{{"token":"{TOKEN}"}}"#));
    let mut binding = plugin.binding(&[]);
    binding.timeout_ms = Some(400);
    let credentials = Credentials::new(binding).unwrap();
    let started = Instant::now();
    assert_eq!(
        token(&credentials, &kubeconfig(&plugin.path)),
        Err(Refusal::Timeout)
    );
    assert!(started.elapsed() < Duration::from_secs(4));
    let child: i32 = plugin.file("child").trim().parse().unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while !dead(child) {
        assert!(
            Instant::now() < until,
            "child in the plugin's group survived"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// A descendant that leaves the process group (setsid) and keeps stdout open
/// cannot hold the caller past the deadline.
#[test]
fn adversary_escaped_descendant_holding_stdout_does_not_hold_the_caller() {
    let plugin = Plugin::new(&format!(
        "{EMIT}\n/usr/bin/setsid /usr/bin/sleep 20 &\necho $! > {{d}}/escaped\nexit 0"
    ));
    plugin.answer(V1, &format!(r#"{{"token":"{TOKEN}"}}"#));
    let mut binding = plugin.binding(&[]);
    binding.timeout_ms = Some(400);
    let credentials = Credentials::new(binding).unwrap();
    let started = Instant::now();
    let result = token(&credentials, &kubeconfig(&plugin.path));
    let elapsed = started.elapsed();
    if let Ok(pid) = plugin.file("escaped").trim().parse::<i32>() {
        // SAFETY: the pid is this test's own fixture descendant.
        unsafe {
            libc::kill(pid, libc::SIGKILL);
        }
    }
    assert_eq!(result, Err(Refusal::Timeout));
    assert!(elapsed < Duration::from_secs(4), "{elapsed:?}");
}

// ---------------------------------------------------------------------------
// Concurrency: the cache lock is held across a run.
// ---------------------------------------------------------------------------

#[test]
fn adversary_concurrent_uses_run_an_expiring_plugin_once() {
    let plugin = Plugin::new(&format!("/usr/bin/sleep 0.3\n{EMIT}"));
    plugin.answer_expiring(LATER);
    let credentials = plugin.credentials();
    let config = kubeconfig(&plugin.path);
    let results: Vec<_> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| token(&credentials, &config)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for result in results {
        assert_eq!(result, Ok(TOKEN.to_owned()));
    }
    assert_eq!(plugin.runs(), 1);
}

// ---------------------------------------------------------------------------
// Kubeconfig command matching: "must equal path or be a bare file name
// equal to the path's file name".
// ---------------------------------------------------------------------------

#[test]
fn adversary_relative_command_forms_are_refused() {
    let plugin = Plugin::answering();
    plugin.answer_expiring(LATER);
    for command in [
        "./fixture-plugin",
        "fixture-plugin/",
        "bin/fixture-plugin",
        "../fixture-plugin",
        "",
        "FIXTURE-PLUGIN",
        " fixture-plugin",
    ] {
        assert_eq!(
            token(&plugin.credentials(), &kubeconfig_with(command, V1, &[])),
            Err(Refusal::KubeconfigMismatch),
            "{command:?}"
        );
    }
    assert_eq!(plugin.runs(), 0);
}

/// `path/` names a directory to every exec (ENOTDIR for kubectl); it is not
/// the pinned path.
#[test]
fn adversary_absolute_command_with_trailing_slash_is_not_the_pinned_path() {
    let plugin = Plugin::answering();
    plugin.answer_expiring(LATER);
    let command = format!("{}/", plugin.path.display());
    assert_eq!(
        token(&plugin.credentials(), &kubeconfig_with(&command, V1, &[])),
        Err(Refusal::KubeconfigMismatch),
        "{command}"
    );
}

/// File names compare as bytes: the NFC name matches, the NFD spelling of
/// the same visible name does not.
#[test]
fn adversary_unicode_bare_name_matches_by_bytes() {
    let plugin = Plugin::named("pl\u{00fc}gin", EMIT);
    plugin.answer_expiring(LATER);
    assert_eq!(
        token(
            &plugin.credentials(),
            &kubeconfig_with("pl\u{00fc}gin", V1, &[])
        ),
        Ok(TOKEN.to_owned())
    );
    assert_eq!(
        token(
            &plugin.credentials(),
            &kubeconfig_with("plu\u{0308}gin", V1, &[])
        ),
        Err(Refusal::KubeconfigMismatch)
    );
}

// ---------------------------------------------------------------------------
// The environment allow-list.
// ---------------------------------------------------------------------------

#[test]
fn adversary_kubeconfig_env_outside_the_allow_list_refuses() {
    let plugin = Plugin::answering();
    plugin.answer_expiring(LATER);
    let credentials = Credentials::new(plugin.binding(&["AWS_PROFILE"])).unwrap();
    let path = plugin.path.to_str().unwrap();
    for name in [
        "aws_profile",
        "HOME",
        "KUBERNETES_EXEC_INFO",
        "PATH",
        "AWS_PROFILE_",
    ] {
        assert_eq!(
            token(&credentials, &kubeconfig_with(path, V1, &[(name, "x")])),
            Err(Refusal::KubeconfigMismatch),
            "{name:?}"
        );
    }
    // The same allowed name twice is refused too.
    assert_eq!(
        token(
            &credentials,
            &kubeconfig_with(path, V1, &[("AWS_PROFILE", "a"), ("AWS_PROFILE", "b")])
        ),
        Err(Refusal::KubeconfigMismatch)
    );
    assert_eq!(plugin.runs(), 0);
}

/// A newline in an allowed value is part of that value; it does not create
/// a second variable.
#[test]
fn adversary_allowed_value_cannot_smuggle_another_variable() {
    let plugin = Plugin::answering();
    plugin.answer_expiring(LATER);
    let credentials = Credentials::new(plugin.binding(&["AWS_PROFILE"])).unwrap();
    let config = kubeconfig_with(
        plugin.path.to_str().unwrap(),
        V1,
        &[("AWS_PROFILE", "fixture\\nLD_PRELOAD=/fixture/evil.so")],
    );
    assert_eq!(token(&credentials, &config), Ok(TOKEN.to_owned()));
    let env = plugin.environment();
    let names = exported(&env);
    assert!(!names.contains(&"LD_PRELOAD"), "{env}");
    assert!(names.contains(&"AWS_PROFILE"), "{env}");
}

/// The adapter's environment is asked only for allow-listed names the
/// kubeconfig does not set, and nothing else reaches the plugin.
#[test]
fn adversary_ambient_is_asked_only_for_allow_listed_unset_names() {
    let plugin = Plugin::answering();
    plugin.answer_expiring(LATER);
    let credentials = Credentials::new(plugin.binding(&["AWS_PROFILE", "AWS_REGION"])).unwrap();
    let config = kubeconfig_with(
        plugin.path.to_str().unwrap(),
        V1,
        &[("AWS_PROFILE", "fixture-profile")],
    );
    let asked = RefCell::new(Vec::new());
    let ambient = |name: &str| {
        asked.borrow_mut().push(name.to_owned());
        Some(format!("ambient-{name}"))
    };
    credentials.token(&config, &at(NOW), &ambient).unwrap();
    assert_eq!(*asked.borrow(), vec!["AWS_REGION".to_owned()]);
    let env = plugin.environment();
    assert!(env.contains("AWS_PROFILE=\"fixture-profile\""), "{env}");
    assert!(env.contains("AWS_REGION=\"ambient-AWS_REGION\""), "{env}");
    let names = exported(&env);
    assert!(names.contains(&"AWS_REGION"), "{env}");
    for name in &names {
        assert!(
            [
                "AWS_PROFILE",
                "AWS_REGION",
                "KUBERNETES_EXEC_INFO",
                "OLDPWD",
                "PWD",
                "SHLVL"
            ]
            .contains(name),
            "unexpected {name} in {env}"
        );
    }
}

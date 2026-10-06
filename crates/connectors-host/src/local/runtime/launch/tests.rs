//! story:launch-consumer-with-connection-credential. The ordinary cases pin the
//! delivery mechanics and the owner's admission order without a keyring. The
//! ignored journey drives the production CLI and owner against a disposable
//! Secret Service; `consumer_fixture` is its pinned consumer and never runs at
//! top level.
use super::*;
use crate::local::{
    config::{Config, Executable, Paths},
    keyring::custody,
    owner::{self, Code},
    registry::{self, Registry},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    io::Read,
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    process::ExitStatus,
    time::Duration,
};

fn until() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn pinned(path: &Path, args: &[&str]) -> Executable {
    Executable {
        path: path.into(),
        sha256: digest(&std::fs::read(path).unwrap()),
        args: args.iter().map(|arg| arg.to_string()).collect(),
    }
}

/// A fictional protected document, assembled at run time.
fn fictional_document() -> (String, Vec<u8>) {
    let sentinel = format!("fictional-launch-{}", uuid::Uuid::new_v4().simple());
    let document = serde_json::to_vec(&json!({ "password": sentinel })).unwrap();
    (sentinel, document)
}

fn close_on_exec(file: &File) -> bool {
    // SAFETY: F_GETFD on a live descriptor.
    unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) & libc::FD_CLOEXEC != 0 }
}

#[test]
fn a_sealed_credential_is_read_only_at_its_start_and_refuses_every_change() {
    let (_, document) = fictional_document();
    let mut file = sealed(&document).unwrap();
    let mut read = Vec::new();
    file.read_to_end(&mut read).unwrap();
    assert_eq!(read, document);
    // SAFETY: F_GET_SEALS on a live descriptor.
    let seals = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) };
    assert_eq!(
        seals,
        libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL
    );
    assert!(close_on_exec(&file));
    // The handed-out description is read-only, and a writable reopen of the
    // same memfd is refused every change by the seals.
    assert!(file.write(b"x").is_err());
    let mut writable = std::fs::OpenOptions::new()
        .write(true)
        .open(format!("/proc/self/fd/{}", file.as_raw_fd()))
        .unwrap();
    assert_eq!(
        writable.write(b"x").unwrap_err().raw_os_error(),
        Some(libc::EPERM)
    );
    assert_eq!(
        writable.set_len(0).unwrap_err().raw_os_error(),
        Some(libc::EPERM)
    );
    assert_eq!(
        writable
            .set_len(document.len() as u64 + 1)
            .unwrap_err()
            .raw_os_error(),
        Some(libc::EPERM)
    );
}

#[test]
fn descriptors_cross_the_owner_socket_above_the_standard_range_and_close_on_exec() {
    let (left, right) = UnixStream::pair().unwrap();
    let first = sealed(b"first fictional").unwrap();
    let second = sealed(b"second fictional").unwrap();
    send(&left, &[&first, &second], until()).unwrap();
    let received = receive(&right, DESCRIPTORS, until()).unwrap();
    assert_eq!(received.len(), 2);
    for (mut file, expected) in received
        .into_iter()
        .zip([&b"first fictional"[..], b"second fictional"])
    {
        assert!(file.as_raw_fd() >= 10);
        assert!(close_on_exec(&file));
        let mut read = Vec::new();
        file.read_to_end(&mut read).unwrap();
        assert_eq!(read, expected);
    }
    // Another count, a plain byte and silence are each refused.
    send(&left, &[&first], until()).unwrap();
    assert!(matches!(
        receive(&right, DESCRIPTORS, until()),
        Err(Failure::Protocol)
    ));
    (&left).write_all(&[MARKER]).unwrap();
    assert!(matches!(
        receive(&right, DESCRIPTORS, until()),
        Err(Failure::Protocol)
    ));
    assert!(matches!(
        receive(
            &right,
            DESCRIPTORS,
            Instant::now() + Duration::from_millis(50)
        ),
        Err(Failure::Timeout)
    ));
}

#[test]
fn a_consumer_reads_the_credential_on_descriptor_three_and_its_exit_code_is_the_launchs() {
    // `test` stands in for a consumer: an argv-pinned image, no shell, no PATH.
    let image = Path::new("/usr/bin/test");
    let run = |args: &[&str], material: &[u8]| {
        let selected = pinned(image, args);
        Consumer::new(
            selected.capture(until()).unwrap(),
            sealed(material).unwrap(),
            selected.args.clone(),
            BTreeSet::new(),
        )
        .run(&[], std::iter::empty())
        .unwrap()
    };
    let (_, document) = fictional_document();
    assert_eq!(run(&["-r", "/proc/self/fd/3"], &document), 0);
    assert_eq!(run(&["-s", "/proc/self/fd/3"], &document), 0);
    assert_eq!(run(&["-s", "/proc/self/fd/3"], b""), 1);
    assert_eq!(run(&["-n", ""], &document), 1);
    assert_eq!(code(ExitStatus::from_raw(7 << 8)), 7);
    assert_eq!(
        code(ExitStatus::from_raw(libc::SIGKILL)),
        128 + libc::SIGKILL
    );
}

const UID_PLACEHOLDER: &str = "UID";

/// A `connectors-local/3` configuration: adapter `fixture` granting profile
/// `fixture-token`, adapter `other`, and consumers `probe` (listing `fixture`),
/// `stranger` (listing nothing) and `tampered` (listing `fixture`, wrong digest).
fn configuration(socket: Option<&Path>, consumer: &Executable) -> String {
    let q = |value: &str| serde_json::to_string(value).unwrap();
    let args = consumer
        .args
        .iter()
        .map(|arg| q(arg))
        .collect::<Vec<_>>()
        .join(",");
    let image = |sha256: &str| {
        format!(
            "path={}\nsha256={}\nargs=[{}]\n",
            q(consumer.path.to_str().unwrap()),
            q(sha256),
            args
        )
    };
    let adapter = |alias: &str, instance: &str| {
        format!(
            "[adapters.{alias}]\ninstance_id='{instance}'\nadapter_id='fixture-adapter'\nconfiguration_revision='cfg-1'\nprotocol='v1alpha1'\nprivate_protocol='connectors-private/1'\n[adapters.{alias}.permissions]\nprofiles=['fixture-token']\n[adapters.{alias}.executable]\npath='/not-installed/must-not-start'\nsha256='{}'\nargs=[]\n",
            "a".repeat(64)
        )
    };
    format!(
        "format='connectors-local/3'\nowner_uid={UID_PLACEHOLDER}\n{}{}{}[consumers.probe]\npass_env=['EKR_']\n[consumers.probe.executable]\n{}[consumers.probe.permissions]\nconnections=['fixture']\n[consumers.stranger.executable]\n{}[consumers.tampered.executable]\n{}[consumers.tampered.permissions]\nconnections=['fixture']\n",
        socket
            .map(|s| format!("secret_service_socket={}\n", q(s.to_str().unwrap())))
            .unwrap_or_default(),
        adapter("fixture", "fixture-instance"),
        adapter("other", "other-instance"),
        image(&consumer.sha256),
        image(&consumer.sha256),
        image(&"b".repeat(64)),
    )
    .replace(
        &format!("owner_uid={UID_PLACEHOLDER}"),
        &format!("owner_uid={}", crate::local::filesystem::uid()),
    )
}

fn initialized(root: &Path, socket: Option<&Path>, consumer: &Executable) -> Paths {
    let paths = Paths::resolve(
        Some(&root.join("config/config.toml")),
        Some(&root.join("state")),
    )
    .unwrap();
    Config::initialize(&paths).unwrap();
    std::fs::write(&paths.config, configuration(socket, consumer)).unwrap();
    paths
}

#[test]
fn launch_admission_names_the_consumer_then_the_pair_then_the_image_before_any_credential() {
    let root = tempfile::tempdir().unwrap();
    let paths = initialized(
        root.path(),
        None,
        &pinned(Path::new("/usr/bin/test"), &["-n", "x"]),
    );
    let code = |alias: &str, connection: &str, consumer: &str| {
        owner::admit_launch(&paths, alias, connection, consumer)
            .err()
            .map(|error| error.code)
    };
    assert_eq!(
        code("fixture", "connection-1", "absent"),
        Some(Code::NotFound)
    );
    assert_eq!(
        code("absent", "connection-1", "probe"),
        Some(Code::NotFound)
    );
    // Omitted permissions deny, and a listing for one adapter grants no other.
    assert_eq!(
        code("fixture", "connection-1", "stranger"),
        Some(Code::Forbidden)
    );
    assert_eq!(
        code("other", "connection-1", "probe"),
        Some(Code::Forbidden)
    );
    assert_eq!(
        code("fixture", "connection-1", "tampered"),
        Some(Code::InvalidConfiguration)
    );
    // An admitted pair reaches the connection authority, which holds nothing.
    assert_eq!(
        code("fixture", "connection-1", "probe"),
        Some(Code::NotFound)
    );
}

/// Subprocess helper: the pinned consumer of the journey below. It reports the
/// SHA-256 of descriptor 3's bytes with its own argv and environment, then
/// exits 23 so the journey can see the launch exit with the consumer's status.
#[test]
#[ignore = "subprocess helper: runs only as the pinned consumer of the consumer launch journey"]
fn consumer_fixture() {
    use std::io::Write;
    // SAFETY: the launch places the credential at descriptor 3 and this
    // process owns it from exec on.
    let mut credential = unsafe { File::from_raw_fd(3) };
    let mut bytes = Vec::new();
    credential.read_to_end(&mut bytes).unwrap();
    let report = json!({
        "sha256": digest(&bytes),
        "argv": std::env::args().collect::<Vec<_>>(),
        "env": std::env::vars_os()
            .map(|(k, v)| format!("{}={}", k.to_string_lossy(), v.to_string_lossy()))
            .collect::<Vec<_>>(),
    });
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "CONSUMER {report}").unwrap();
    stdout.flush().unwrap();
    std::process::exit(23);
}

const CONSUMER_ARGS: [&str; 5] = [
    "local::runtime::launch::tests::consumer_fixture",
    "--exact",
    "--ignored",
    "--nocapture",
    "--test-threads=1",
];
/// Caller arguments appended after the pinned argv. libtest reads them as
/// extra name filters, which `--exact` leaves harmless.
const EXTRA: [&str; 3] = ["head", "two words", "{\"json\":[1]}"];

fn consumer_report(stdout: &[u8], stderr: &[u8]) -> Value {
    let stdout = String::from_utf8_lossy(stdout);
    serde_json::from_str(
        stdout
            .lines()
            // libtest's `test <name> ... ` precedes the report on its line.
            .find_map(|line| line.split_once("CONSUMER ").map(|(_, report)| report))
            .unwrap_or_else(|| {
                panic!(
                    "the consumer reported nothing: stdout {stdout:?}, stderr {:?}",
                    String::from_utf8_lossy(stderr)
                )
            }),
    )
    .unwrap()
}

/// Subprocess helper: launches `consumer_fixture` as a consumer whose entry
/// passes `EKR_`, with `EXTRA` as the caller's arguments and this process's
/// environment as the caller's, so the parent case can read what it received.
#[test]
#[ignore = "subprocess helper: runs only under the consumer argv and environment case"]
fn launch_runner_fixture() {
    let selected = pinned(&std::env::current_exe().unwrap(), &CONSUMER_ARGS);
    let code = Consumer::new(
        selected.capture(until()).unwrap(),
        sealed(b"fictional runner material").unwrap(),
        selected.args.clone(),
        BTreeSet::from(["EKR_".to_owned()]),
    )
    .run(&EXTRA.map(str::to_owned), std::env::vars_os())
    .unwrap();
    std::process::exit(code);
}

#[test]
fn a_consumer_gets_its_pinned_argv_then_the_callers_args_and_only_passed_variables() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "local::runtime::launch::tests::launch_runner_fixture",
            "--exact",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env_clear()
        .env("EKR_X", "passed-value")
        .env("EKR_STORE", "/fixture/store")
        .env("OTHER", "kept-out")
        .env("XEKR_Y", "kept-out")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(23), "{output:?}");
    let report = consumer_report(&output.stdout, &output.stderr);
    assert_eq!(report["sha256"], digest(b"fictional runner material"));
    let argv: Vec<String> = serde_json::from_value(report["argv"].clone()).unwrap();
    let expected: Vec<String> = CONSUMER_ARGS
        .iter()
        .chain(&EXTRA)
        .map(|a| a.to_string())
        .collect();
    assert_eq!(argv[1..], expected);
    let mut env: Vec<String> = serde_json::from_value(report["env"].clone()).unwrap();
    env.sort();
    assert_eq!(env, ["EKR_STORE=/fixture/store", "EKR_X=passed-value"]);
}

#[test]
fn a_caller_argument_holding_nul_is_refused_before_anything_runs() {
    let selected = pinned(Path::new("/usr/bin/test"), &["-n"]);
    let launched = Consumer::new(
        selected.capture(until()).unwrap(),
        sealed(b"fictional").unwrap(),
        selected.args.clone(),
        BTreeSet::new(),
    )
    .run(&["a\0b".to_owned()], std::iter::empty());
    assert!(matches!(launched, Err(Failure::InvalidInput)));
}

/// Publish one ready connection through the registry's own lifecycle, its
/// material in the disposable Secret Service, valid for `lifetime_ms`.
fn publish(
    fixture: &custody::tests::Fixture,
    paths: &Paths,
    document: &[u8],
    lifetime_ms: u64,
) -> String {
    let binding = registry::Binding {
        instance_id: "fixture-instance".into(),
        adapter_id: "fixture-adapter".into(),
        configuration_revision: "cfg-1".into(),
        provider_authority: "https://fixture.invalid".into(),
        profile: registry::StaticProfile {
            id: "fixture-token".into(),
            revision: "profile-1".into(),
            purpose: registry::Purpose::DelegatedUser,
            subject: registry::Subject::User,
            minimum_scopes: BTreeSet::new(),
            evidence_lifetime_ms: 60_000,
        },
    };
    let clock = connectors_sdk::now_ms;
    let registry = Registry::new(&paths.state);
    let acquisition = registry.begin(&binding, clock()).unwrap();
    let claim = registry.consume(acquisition, clock()).unwrap();
    let now = clock();
    let baseline = registry::ValidatedBaseline {
        identity: registry::ExternalIdentity {
            kind: "fixture".into(),
            subject: "one".into(),
        },
        granted_scopes: Some(BTreeSet::new()),
        credential_expires_at_ms: None,
        collected_at_ms: now,
        valid_until_ms: now + lifetime_ms,
    };
    let material = connectors_sdk::Secret(document.to_vec());
    let prepared = registry
        .prepare(&claim, baseline, material.0.len(), now)
        .unwrap();
    let store = custody::Store::open_at(prepared.version().scope(), Some(&fixture.socket)).unwrap();
    registry
        .store_and_publish(prepared, &material, &store, clock)
        .unwrap()
}

/// Stops the owner the CLI started, whatever the journey's outcome.
struct OwnerStop(Paths);
impl Drop for OwnerStop {
    fn drop(&mut self) {
        if let Ok(client) = owner::Client::connect(&self.0, false) {
            let host = client.host_incarnation.clone();
            let _ = client.shutdown(&host);
        }
    }
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}

fn files(directory: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            files(&entry.path(), found);
        } else if kind.is_file() {
            found.push(entry.path());
        }
    }
}

#[test]
#[ignore = "requires qualified GNOME, dbus-daemon, task-owned TMPDIR and CONNECTORS_TEST_CLI"]
fn disposable_consumer_launch_delivers_fd3_and_refuses_by_code() {
    let binary = std::env::var_os("CONNECTORS_TEST_CLI").expect("built production CLI required");
    let mut fixture = custody::tests::Fixture::new();
    let consumer = pinned(&std::env::current_exe().unwrap(), &CONSUMER_ARGS);
    let paths = initialized(fixture.root.path(), Some(&fixture.socket), &consumer);
    let _owner = OwnerStop(Paths {
        config: paths.config.clone(),
        state: paths.state.clone(),
    });
    let (sentinel, document) = fictional_document();
    let connection = publish(&fixture, &paths, &document, 60_000);
    let launch = |connection: &str, consumer: &str| {
        let output = std::process::Command::new(&binary)
            .env_clear()
            .env("EKR_X", "passed-value")
            .env("OTHER", "kept-out")
            .args(["--output", "json", "--config"])
            .arg(&paths.config)
            .arg("--state-dir")
            .arg(&paths.state)
            .args(["connections", "launch", "--adapter", "fixture"])
            .args(["--connection", connection, "--consumer", consumer])
            .args(["--args", &serde_json::to_string(&EXTRA).unwrap()])
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        assert!(!contains(&output.stdout, &sentinel));
        assert!(!contains(&output.stderr, &sentinel));
        output
    };
    let refused = |connection: &str, consumer: &str| {
        let output = launch(connection, consumer);
        assert!(!output.status.success(), "{output:?}");
        assert!(output.stdout.is_empty(), "{output:?}");
        serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"]["data"].clone()
    };

    let launched = launch(&connection, "probe");
    assert_eq!(launched.status.code(), Some(23), "{launched:?}");
    let report = consumer_report(&launched.stdout, &launched.stderr);
    assert_eq!(report["sha256"], digest(&document));
    // The pinned argv, then the caller's `--args`; only `EKR_` variables pass.
    let argv: Vec<String> = serde_json::from_value(report["argv"].clone()).unwrap();
    let expected: Vec<String> = CONSUMER_ARGS
        .iter()
        .chain(&EXTRA)
        .map(|a| a.to_string())
        .collect();
    assert_eq!(argv[1..], expected);
    assert_eq!(report["env"], json!(["EKR_X=passed-value"]));
    assert!(!report["argv"].to_string().contains(&sentinel));
    assert!(!report["env"].to_string().contains(&sentinel));
    let mut state = Vec::new();
    files(&paths.state, &mut state);
    assert!(!state.is_empty());
    for path in state {
        assert!(
            !contains(&std::fs::read(&path).unwrap(), &sentinel),
            "{path:?}"
        );
    }

    // An unlisted pair is refused at admission.
    let data = refused(&connection, "stranger");
    assert_eq!(
        (&data["code"], &data["stage"]),
        (&json!("forbidden"), &json!("admission"))
    );
    // A wrong pinned digest is refused before exec: the consumer never reports.
    let data = refused(&connection, "tampered");
    assert_eq!(data["code"], "invalid_configuration", "{data}");
    // Lapsed evidence answers `unavailable` at readiness.
    let lapsing = publish(&fixture, &paths, &document, 1_500);
    std::thread::sleep(Duration::from_millis(1_700));
    let data = refused(&lapsing, "probe");
    assert_eq!(
        (&data["code"], &data["stage"]),
        (&json!("unavailable"), &json!("readiness")),
        "{data}"
    );
    // A locked collection is `custody_unavailable`.
    fixture.stop();
    fixture.start(false);
    let data = refused(&connection, "probe");
    assert_eq!(data["code"], "custody_unavailable", "{data}");
}

/// Security review of wave 20261006a unit U1: conformance cases for the
/// credential-custody invariants of `contracts/cli/v1alpha1/consumer-launch.md`
/// as the consumer itself observes them. `reviewing_runner_fixture` stands in
/// for the launching CLI (owner socket, its own files and a bus-like socket
/// open, descriptors received over `SCM_RIGHTS`); `reviewing_consumer_fixture`
/// is the pinned consumer and reports what it received.
mod review_conformance {
    use super::{digest, pinned, until};
    use crate::local::runtime::launch::{Consumer, DESCRIPTORS, Failure, receive, sealed, send};
    use serde_json::{Value, json};
    use std::{
        collections::BTreeSet,
        fs::File,
        io::Read,
        mem::ManuallyDrop,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::{net::UnixStream, process::CommandExt},
        },
        path::Path,
    };

    const RUNNER: &str =
        "local::runtime::launch::tests::review_conformance::reviewing_runner_fixture";
    const CONSUMER: [&str; 5] = [
        "local::runtime::launch::tests::review_conformance::reviewing_consumer_fixture",
        "--exact",
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ];
    /// A descriptor number the reviewing caller leaves open without
    /// close-on-exec, as a shell's `exec 7<file` or a supervisor's passed
    /// socket would.
    const INHERITED: i32 = 7;

    fn errno<T: PartialOrd + Default>(result: T) -> i32 {
        if result < T::default() {
            std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
        } else {
            0
        }
    }

    /// Subprocess helper: the pinned consumer. Reports the descriptors open at
    /// its start, its environment, the digest of descriptor 3's bytes, and the
    /// errno of every attempt to change descriptor 3's file.
    #[test]
    #[ignore = "subprocess helper: runs only as the reviewing consumer"]
    fn reviewing_consumer_fixture() {
        use std::io::Write;
        let listed: Vec<i32> = std::fs::read_dir("/proc/self/fd")
            .unwrap()
            .map(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .parse()
                    .unwrap()
            })
            .collect();
        // The listing's own directory descriptor is closed by now.
        // SAFETY: F_GETFD only probes a descriptor number.
        let descriptors: Vec<(i32, String)> = listed
            .into_iter()
            .filter(|fd| unsafe { libc::fcntl(*fd, libc::F_GETFD) } >= 0)
            .map(|fd| {
                let target = std::fs::read_link(format!("/proc/self/fd/{fd}"))
                    .map(|path| path.to_string_lossy().into_owned())
                    .unwrap_or_default();
                (fd, target)
            })
            .collect();
        // SAFETY: descriptor 3 is the launch's; ManuallyDrop keeps it open.
        let mut credential = ManuallyDrop::new(unsafe { File::from_raw_fd(3) });
        let mut bytes = Vec::new();
        credential.read_to_end(&mut bytes).unwrap();
        // SAFETY: plain syscalls on descriptor numbers and a live buffer.
        let report = unsafe {
            let write_fd3 = errno(libc::write(3, b"x".as_ptr().cast(), 1));
            let reopened = libc::open(c"/proc/self/fd/3".as_ptr(), libc::O_RDWR | libc::O_CLOEXEC);
            let reopen = errno(reopened);
            let target = if reopened >= 0 { reopened } else { 3 };
            let write_reopened = errno(libc::write(target, b"x".as_ptr().cast(), 1));
            let shrink = errno(libc::ftruncate(target, 0));
            let grow = errno(libc::ftruncate(target, bytes.len() as i64 + 4096));
            let add_seal = errno(libc::fcntl(3, libc::F_ADD_SEALS, libc::F_SEAL_FUTURE_WRITE));
            let map = libc::mmap(
                std::ptr::null_mut(),
                4096,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                target,
                0,
            );
            let map_shared_write = if map == libc::MAP_FAILED {
                std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
            } else {
                libc::munmap(map, 4096);
                0
            };
            json!({
                "sha256": digest(&bytes),
                "descriptors": descriptors,
                "env": std::env::vars_os()
                    .map(|(k, v)| format!("{}={}", k.to_string_lossy(), v.to_string_lossy()))
                    .collect::<Vec<_>>(),
                "seals": libc::fcntl(3, libc::F_GET_SEALS),
                "write_fd3": write_fd3,
                "reopen_rdwr": reopen,
                "write_reopened": write_reopened,
                "shrink": shrink,
                "grow": grow,
                "add_seal": add_seal,
                "map_shared_write": map_shared_write,
            })
        };
        let mut stdout = std::io::stdout().lock();
        writeln!(stdout, "REVIEW {report}").unwrap();
        stdout.flush().unwrap();
        std::process::exit(0);
    }

    /// Subprocess helper: the launching side. The owner end sends the captured
    /// image and a sealed run-time credential over a socket pair; this end
    /// receives them as `Client::launch` does and runs the consumer from an
    /// entry without `pass_env`, with this process's environment as the
    /// caller's, while the owner socket, an ordinary file and a bus-like
    /// socket of its own stay open.
    #[test]
    #[ignore = "subprocess helper: runs only under the reviewing cases"]
    fn reviewing_runner_fixture() {
        let selected = pinned(&std::env::current_exe().unwrap(), &CONSUMER);
        let material = format!("fictional-review-{}", uuid::Uuid::new_v4().simple());
        println!("MATERIAL {}", digest(material.as_bytes()));
        let (cli_end, owner_end) = UnixStream::pair().unwrap();
        {
            let image = selected.capture(until()).unwrap();
            let credential = sealed(material.as_bytes()).unwrap();
            send(&owner_end, &[&image, &credential], until()).unwrap();
        }
        let mut files = receive(&cli_end, DESCRIPTORS, until()).unwrap();
        let credential = files.pop().unwrap();
        let executable = files.pop().unwrap();
        let _own_file = File::open(std::env::current_exe().unwrap()).unwrap();
        let _bus = UnixStream::pair().unwrap();
        let code = Consumer::new(
            executable,
            credential,
            selected.args.clone(),
            BTreeSet::new(),
        )
        .run(&[], std::env::vars_os())
        .unwrap();
        std::process::exit(code);
    }

    /// Run the launching side with a caller environment holding variables the
    /// entry does not pass, optionally with descriptor `INHERITED` left open by
    /// its caller. Returns the run-time material's digest and the report.
    fn review(inherited: bool) -> (String, Value) {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                RUNNER,
                "--exact",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env_clear()
            .env("EKR_X", "caller-value")
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", "/nonexistent")
            .stdin(std::process::Stdio::null());
        let held = File::open(std::env::current_exe().unwrap()).unwrap();
        let raw = held.as_raw_fd();
        if inherited {
            // SAFETY: dup2 and fcntl are async-signal-safe; `held` outlives spawn.
            unsafe {
                command.pre_exec(move || {
                    if libc::dup2(raw, INHERITED) < 0
                        || libc::fcntl(INHERITED, libc::F_SETFD, 0) < 0
                    {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }
        let output = command.output().unwrap();
        drop(held);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let find = |tag: &str| {
            stdout
                .lines()
                .find_map(|line| line.split_once(tag).map(|(_, rest)| rest.to_owned()))
                .unwrap_or_else(|| panic!("no {tag:?} line: {output:?}"))
        };
        let material = find("MATERIAL ");
        let report: Value = serde_json::from_str(&find("REVIEW ")).unwrap();
        (material, report)
    }

    fn descriptor_numbers(report: &Value) -> Vec<i64> {
        report["descriptors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| pair[0].as_i64().unwrap())
            .collect()
    }

    /// Invariant 4: an entry without `pass_env` passes no caller variable.
    #[test]
    fn review_an_entry_without_pass_env_starts_the_consumer_with_an_empty_environment() {
        let (_, report) = review(false);
        assert_eq!(report["env"], json!([]), "{report}");
    }

    /// Invariant 5, the launching process's own descriptors: the owner socket,
    /// an ordinary file, a bus-like socket and the received image and
    /// credential copies never reach the consumer; only 0, 1, 2 and 3 do.
    #[test]
    fn review_the_launchers_own_descriptors_do_not_reach_the_consumer() {
        let (_, report) = review(false);
        assert_eq!(descriptor_numbers(&report), [0, 1, 2, 3], "{report}");
    }

    /// Invariant 5, a descriptor the caller left open: the brief's "no other
    /// descriptor leaks into the consumer" holds only if the launch closes
    /// everything above 3, not only its own close-on-exec descriptors.
    #[test]
    fn review_a_descriptor_the_caller_left_open_does_not_reach_the_consumer() {
        let (_, report) = review(true);
        assert_eq!(descriptor_numbers(&report), [0, 1, 2, 3], "{report}");
    }

    /// Invariants 2 and 7 from inside the consumer: descriptor 3 carries the
    /// material byte for byte, and no write, reopen-and-write, shrink, grow,
    /// added seal or shared writable mapping succeeds.
    #[test]
    fn review_the_consumer_cannot_change_descriptor_three_or_its_seals() {
        let (material, report) = review(false);
        assert_eq!(report["sha256"], json!(material), "{report}");
        assert_eq!(
            report["seals"],
            json!(libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL),
            "{report}"
        );
        assert_eq!(report["write_fd3"], json!(libc::EBADF), "{report}");
        for attempt in [
            "write_reopened",
            "shrink",
            "grow",
            "add_seal",
            "map_shared_write",
        ] {
            assert_eq!(report[attempt], json!(libc::EPERM), "{attempt}: {report}");
        }
    }

    /// Invariant 3: after the pinned file is swapped on disk, a new capture is
    /// refused by digest, and the image captured before the swap is what runs.
    #[test]
    fn review_a_swapped_binary_is_refused_and_the_captured_image_is_what_runs() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("consumer");
        std::fs::copy("/usr/bin/true", &path).unwrap();
        let selected = pinned(&path, &[]);
        let image = selected.capture(until()).unwrap();
        let swap = root.path().join("swap");
        std::fs::copy("/usr/bin/false", &swap).unwrap();
        std::fs::rename(&swap, &path).unwrap();
        assert!(selected.check().is_err());
        assert!(matches!(
            selected.capture(until()),
            Err(Failure::InvalidConfiguration)
        ));
        let code = Consumer::new(image, sealed(b"").unwrap(), Vec::new(), BTreeSet::new())
            .run(&[], std::iter::empty())
            .unwrap();
        assert_eq!(
            code, 0,
            "the swapped file ran instead of the captured image"
        );
        assert_eq!(
            Consumer::new(
                pinned(Path::new("/usr/bin/false"), &[])
                    .capture(until())
                    .unwrap(),
                sealed(b"").unwrap(),
                Vec::new(),
                BTreeSet::new(),
            )
            .run(&[], std::iter::empty())
            .unwrap(),
            1
        );
    }
}

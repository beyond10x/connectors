//! story:invoke-definition-file, the one connect question of `invoke
//! --definition <file> <operation>` (story:connectors-do-short-invocation;
//! `contracts/cli/v1alpha1/semantics.md` §3, "Invoking a definition file").
//! The question is the operator's whole consent to send the credential they
//! are about to type to the host the file names, so it names that host, as
//! the reuse question names each candidate's provider authority; and a yes is
//! a consent to the entry it was asked for, so an entry another run recorded
//! over it while it waited is the contract's `revision_conflict` with
//! `retry_explicitly`, not a connect under the other file's entry.
//!
//! The CLI runs on a fresh pseudo-terminal as the foreground of its own
//! session, with the built catalog provider named by
//! `CONNECTORS_TEST_CATALOG_PROVIDER` installed beside it. Custody names a
//! socket that does not exist, so a connect that went on would be refused at
//! its custody check: nothing is acquired or sent to any host.
#![cfg(unix)]
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write as _,
    os::unix::{fs::PermissionsExt, io::FromRawFd, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

const OPENAPI: &str = r#"{"openapi":"3.0.3","info":{"title":"Fixture forge","version":"1"},
"servers":[{"url":"https://gitlab.example.com/api/v4"}],
"paths":{
"/user":{"get":{"operationId":"getApiV4User","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"}}}}}}}}},
"/personal_access_tokens/self":{"get":{"operationId":"getApiV4PersonalAccessTokensSelf","responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object"}}}}}}},
"/projects/{id}":{"get":{"operationId":"getApiV4ProjectsId","parameters":[{"name":"id","in":"path","required":true,"schema":{"type":"string"}}],"responses":{"200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"}}}}}}}}}
}}"#;

/// A token profile: the typed token goes to the `base_url` host.
const PAT: &str = "  gitlab.pat:
    scheme: header
    header: PRIVATE-TOKEN
    label: GitLab personal access token
    identity: {path: user, kind: gitlab.user, subject: /id}
    scopes: {path: personal_access_tokens/self, pointer: /scopes, minimum: [api]}
";

/// The host of the OAuth token endpoint of [`oauth`], not the API's.
const TOKEN_HOST: &str = "login.attacker.example";

/// An OAuth profile whose client secret is exchanged at [`TOKEN_HOST`].
fn oauth() -> String {
    format!(
        "  gitlab.oauth:
    scheme: oauth2_client_credentials
    token_url: https://{TOKEN_HOST}/oauth/token
    scopes: {{requested: [api]}}
    identity: {{path: user, kind: gitlab.user, subject: /id}}
    label: OAuth client secret
"
    )
}

/// A look-alike host a definition sends its credential to.
const LOOKALIKE: &str = "gitlab-ci.attacker.example";
const QUESTION_END: &[u8] = b"[y/N] ";
const PROVIDER: &str = "CONNECTORS_TEST_CATALOG_PROVIDER";

fn definition(sha256: &str, connector: &str, hostname: &str, auth: &str) -> String {
    format!(
        "schema: urn:beyond10x:connectors-definition:1
id: {connector}
source:
  openapi: upstream/forge/openapi.json.gz
  sha256: {sha256}
base_url: https://{{hostname}}/api/v4
config: {{hostname: {{type: string, default: {hostname}}}}}
auth:
{auth}operations:
  project.get:
    op: getApiV4ProjectsId
    description: Read one project
"
    )
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A gzip member holding `data` in one stored deflate block.
fn gzip(data: &[u8]) -> Vec<u8> {
    assert!(data.len() < 65_536);
    let len = data.len() as u16;
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff, 0x01];
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

fn private(path: &Path, bytes: &[u8], mode: u32) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// The text the terminal shows for the answer typed after it.
type Answer<'a> = (&'a [u8], &'a mut dyn FnMut() -> Vec<u8>);

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let built = PathBuf::from(
            std::env::var_os(PROVIDER)
                .unwrap_or_else(|| panic!("{PROVIDER} names no built catalog provider")),
        );
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
        };
        let init = fixture.command(&["setup", "init"]).output().unwrap();
        assert!(init.status.success(), "{init:?}");
        // Custody names a socket that does not exist: nothing can reach a
        // keyring of the machine running the test.
        let config = fixture.root.path().join("config/config.toml");
        let initialized = fs::read_to_string(&config).unwrap();
        let line = |key: &str| {
            initialized
                .lines()
                .find(|line| line.trim_start().starts_with(key))
                .unwrap_or_else(|| panic!("no {key} in {initialized}"))
                .to_owned()
        };
        let socket = fixture.root.path().join("no-secret-service.sock");
        private(
            &config,
            format!(
                "{}\n{}\nsecret_service_socket={}\n",
                line("format"),
                line("owner_uid"),
                serde_json::to_string(socket.to_str().unwrap()).unwrap()
            )
            .as_bytes(),
            0o600,
        );
        let defs = fixture.root.path().join("defs");
        fs::create_dir_all(defs.join("connectors")).unwrap();
        fs::create_dir_all(defs.join("upstream/forge")).unwrap();
        fs::write(
            defs.join("upstream/forge/openapi.json.gz"),
            gzip(OPENAPI.as_bytes()),
        )
        .unwrap();
        let bin = fixture.root.path().join("bin");
        fs::create_dir(&bin).unwrap();
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o700)).unwrap();
        let installed = bin.join("connectors-catalog-provider");
        fs::copy(&built, &installed).unwrap();
        fs::set_permissions(&installed, fs::Permissions::from_mode(0o700)).unwrap();
        fixture
    }

    /// Writes `connectors/<stem>.yaml` with `id: <connector>` at `hostname`
    /// under the profile `auth`.
    fn write(&self, stem: &str, connector: &str, hostname: &str, auth: &str) -> String {
        let sha256 = hex::encode(Sha256::digest(OPENAPI.as_bytes()));
        let path = self
            .root
            .path()
            .join(format!("defs/connectors/{stem}.yaml"));
        fs::write(&path, definition(&sha256, connector, hostname, auth)).unwrap();
        path.to_str().unwrap().to_owned()
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
        command
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .env("CONNECTORS_INSTALLATION", self.root.path().join("bin"))
            .args(["--output", "json", "--config"])
            .arg(self.root.path().join("config/config.toml"))
            .arg("--state-dir")
            .arg(self.root.path().join("state"))
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    /// `args` as the foreground of its own session on a fresh
    /// pseudo-terminal; each answer is typed once the terminal shows the text
    /// it follows. Answers the output and everything the terminal showed.
    fn on_terminal(&self, args: &[&str], answers: &mut [Answer<'_>]) -> (Output, Vec<u8>) {
        let (mut master_fd, mut slave_fd) = (-1, -1);
        // SAFETY: initialized output slots; default terminal size and mode.
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master_fd,
                    &mut slave_fd,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            },
            0
        );
        // SAFETY: both descriptors were just opened and are owned here.
        let (mut master, slave) = unsafe {
            (
                fs::File::from_raw_fd(master_fd),
                fs::File::from_raw_fd(slave_fd),
            )
        };
        // SAFETY: live descriptors; scalar flags.
        unsafe {
            assert_eq!(libc::fcntl(master_fd, libc::F_SETFL, libc::O_NONBLOCK), 0);
            assert_eq!(libc::fcntl(master_fd, libc::F_SETFD, libc::FD_CLOEXEC), 0);
            assert_eq!(libc::fcntl(slave_fd, libc::F_SETFD, libc::FD_CLOEXEC), 0);
        }
        let mut command = self.command(args);
        // SAFETY: the child alone creates its session and adopts the slave as
        // its controlling terminal.
        unsafe {
            command.pre_exec(move || {
                if libc::setsid() < 0 || libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().unwrap();
        let (mut stdout, mut stderr) = (child.stdout.take().unwrap(), child.stderr.take().unwrap());
        let readers = [
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut stdout, &mut bytes).unwrap();
                bytes
            }),
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                std::io::Read::read_to_end(&mut stderr, &mut bytes).unwrap();
                bytes
            }),
        ];
        let mut shown = Vec::new();
        let (mut answered, mut since) = (0, 0);
        let until = Instant::now() + Duration::from_secs(120);
        let status = loop {
            let mut bytes = [0; 256];
            if let Ok(n) = std::io::Read::read(&mut master, &mut bytes) {
                shown.extend_from_slice(&bytes[..n]);
            }
            if let Some((prompt, answer)) = answers.get_mut(answered)
                && shown[since..].ends_with(prompt)
            {
                let answer = answer();
                master.write_all(&answer).unwrap();
                answered += 1;
                since = shown.len();
            }
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= until {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "the CLI did not finish: {}",
                    String::from_utf8_lossy(&shown)
                );
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        drop(slave);
        let [stdout, stderr] = readers.map(|reader| reader.join().unwrap());
        assert_eq!(
            answered,
            answers.len(),
            "{}",
            String::from_utf8_lossy(&shown)
        );
        (
            Output {
                status,
                stdout,
                stderr,
            },
            shown,
        )
    }

    /// `args` with no controlling terminal: the CLI leads its own session.
    fn detached(&self, args: &[&str]) -> Output {
        let mut command = self.command(args);
        // SAFETY: setsid in the child between fork and exec; no allocation.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        command.output().unwrap()
    }

    fn instances(&self) -> Vec<u8> {
        fs::read(self.root.path().join("state/definitions/instances.toml")).unwrap_or_default()
    }
}

fn invoke(definition: &str) -> Vec<&str> {
    vec![
        "invoke",
        "--definition",
        definition,
        "project.get",
        "--input-json",
        r#"{"id":"org/project"}"#,
    ]
}

fn data(output: &Output) -> serde_json::Value {
    let envelope: serde_json::Value = serde_json::from_slice(&output.stderr)
        .unwrap_or_else(|_| panic!("one JSON envelope: {output:?}"));
    envelope["error"]["data"].clone()
}

/// The question a definition at `hostname` under `auth` asks, answered `n`.
fn asked(hostname: &str, auth: &str) -> String {
    let fixture = Fixture::new();
    let file = fixture.write("lookalike", "gitlab", hostname, auth);
    let mut no = || b"n\n".to_vec();
    let (output, shown) = fixture.on_terminal(&invoke(&file), &mut [(QUESTION_END, &mut no)]);
    let shown = String::from_utf8_lossy(&shown).into_owned();
    // The question was reached and refused: nothing was sent.
    assert!(!output.status.success(), "{output:?}");
    assert_eq!(data(&output)["code"], "not_found", "{output:?}");
    assert!(
        shown.contains("No connection for gitlab/definition."),
        "the question was not asked: {shown:?} {output:?}"
    );
    shown
}

#[test]
#[ignore = "requires CONNECTORS_TEST_CATALOG_PROVIDER"]
fn the_connect_question_names_the_host_the_token_is_sent_to() {
    let shown = asked(LOOKALIKE, PAT);
    assert!(
        shown.contains(&format!("https://{LOOKALIKE}/")),
        "the terminal showed no host before the answer: {shown:?}"
    );
    assert_eq!(
        shown,
        format!(
            "No connection for gitlab/definition. Connect now to https://{LOOKALIKE}/api/v4/? [y/N] n\r\n"
        )
    );
}

#[test]
#[ignore = "requires CONNECTORS_TEST_CATALOG_PROVIDER"]
fn the_connect_question_of_an_oauth_profile_names_its_token_host_too() {
    let shown = asked(LOOKALIKE, &oauth());
    assert!(
        shown.contains(&format!("https://{LOOKALIKE}/")),
        "the terminal showed no API host before the answer: {shown:?}"
    );
    assert!(
        shown.contains(&format!("https://{TOKEN_HOST}")),
        "the terminal showed no token host before the answer: {shown:?}"
    );
    assert_eq!(
        shown,
        format!(
            "No connection for gitlab/definition. Connect now to https://{LOOKALIKE}/api/v4/ (tokens from https://{TOKEN_HOST})? [y/N] n\r\n"
        )
    );
}

#[test]
#[ignore = "requires CONNECTORS_TEST_CATALOG_PROVIDER"]
fn an_entry_another_run_recorded_while_the_question_waits_is_not_connected() {
    let fixture = Fixture::new();
    let a = fixture.write("gitlab", "gitlab", "gitlab.example.com", PAT);
    let b = fixture.write("evil", "gitlab", LOOKALIKE, PAT);
    let mut recorded_by_a = Vec::new();
    let mut b_output = None;
    let mut replace = || {
        recorded_by_a = fixture.instances();
        b_output = Some(fixture.detached(&invoke(&b)));
        b"y\n".to_vec()
    };
    let (output, shown) = fixture.on_terminal(&invoke(&a), &mut [(QUESTION_END, &mut replace)]);
    let shown = String::from_utf8_lossy(&shown).into_owned();
    // The precondition: A asked, and B, with no terminal, recorded its entry
    // over A's while A waited, then was refused at its own question.
    assert!(
        shown.contains("No connection for gitlab/definition."),
        "A did not ask: {shown:?} {output:?}"
    );
    let b_output = b_output.unwrap();
    assert_eq!(data(&b_output)["code"], "not_found", "{b_output:?}");
    assert!(
        !recorded_by_a.is_empty(),
        "A recorded no entry before asking"
    );
    assert_ne!(
        fixture.instances(),
        recorded_by_a,
        "B did not replace A's entry"
    );
    // A's yes: the contract's refusal for a replaced entry, not a connect.
    let a_data = data(&output);
    assert_eq!(
        (&a_data["code"], &a_data["next_action"]),
        (
            &serde_json::json!("revision_conflict"),
            &serde_json::json!("retry_explicitly")
        ),
        "A went on after its entry was replaced: {a_data}"
    );
}

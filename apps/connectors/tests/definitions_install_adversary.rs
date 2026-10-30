//! Adversary cases for story:definitions-install against the `connectors` binary,
//! driven from `contracts/cli/v1alpha1/semantics.md` §3 ("Installing a definitions
//! release") and `contracts/cli/v1alpha1/scenarios.md` ("Unchanged" means the bytes
//! of `definitions.lock` and the paths, modes and bytes of everything under
//! `<state>/installed-definitions/` equal those recorded before the command).
//! Releases are served from a temporary directory; no network is used.
#![cfg(unix)]
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

const SCHEMA: &str = "urn:beyond10x:connectors-definitions-index:1";
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn prefix(tag: &str) -> String {
    format!("connectors-definitions-{tag}")
}

fn files(tag: &str) -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([
        ("auth/token.yaml".to_owned(), b"profile: token\n".to_vec()),
        (
            "connectors/gitlab.yaml".to_owned(),
            format!("format: connectors-definition/1\nconnector: gitlab\n# {tag}\n").into_bytes(),
        ),
    ])
}

fn regular<W: Write>(builder: &mut tar::Builder<W>, path: &str, bytes: &[u8]) {
    let mut header = tar::Header::new_ustar();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_size(bytes.len() as u64);
    header.set_mode(0o664);
    builder.append_data(&mut header, path, bytes).unwrap();
}

fn archive(tag: &str, files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::default(),
    ));
    for (path, bytes) in files {
        regular(&mut builder, &format!("{}/{path}", prefix(tag)), bytes);
    }
    builder.into_inner().unwrap().finish().unwrap()
}

/// Writes `<directory>/<tag>/index.json` for `files` and `asset` beside it.
fn publish(directory: &Path, tag: &str, files: &BTreeMap<String, Vec<u8>>, asset: &[u8]) {
    let name = format!("{}.tar.gz", prefix(tag));
    let index = json!({
        "schema": SCHEMA,
        "tag": tag,
        "commit": COMMIT,
        "features": ["query_fixed"],
        "asset": {"name": name, "sha256": sha256(asset)},
        "files": files
            .iter()
            .map(|(path, bytes)| json!({"path": path, "sha256": sha256(bytes), "size": bytes.len()}))
            .collect::<Vec<_>>(),
    });
    let release = directory.join(tag);
    fs::create_dir_all(&release).unwrap();
    fs::write(release.join(name), asset).unwrap();
    fs::write(
        release.join("index.json"),
        serde_json::to_vec(&index).unwrap(),
    )
    .unwrap();
}

type Tree = BTreeMap<PathBuf, (u32, Option<Vec<u8>>)>;

fn writable(path: &Path) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    if metadata.is_dir() {
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                writable(&entry.path());
            }
        }
    } else if metadata.is_file() {
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
}

struct Fixture {
    root: tempfile::TempDir,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        writable(self.root.path());
    }
}

impl Fixture {
    /// A release directory, and a configuration only when `init`.
    fn new(init: bool) -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
        };
        fs::create_dir(fixture.releases()).unwrap();
        if init {
            let output = fixture.run(&["setup", "init"]);
            assert!(output.status.success(), "{output:?}");
        }
        fixture
    }
    fn releases(&self) -> PathBuf {
        self.root.path().join("releases")
    }
    fn config_directory(&self) -> PathBuf {
        self.root.path().join("config")
    }
    fn lock(&self) -> PathBuf {
        self.config_directory().join("definitions.lock")
    }
    fn state(&self) -> PathBuf {
        self.root.path().join("state")
    }
    fn cache(&self) -> PathBuf {
        self.state().join("installed-definitions")
    }
    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_connectors"));
        command
            .env_clear()
            .args(["--output", "json"])
            .arg("--config")
            .arg(self.config_directory().join("config.toml"))
            .arg("--state-dir")
            .arg(self.state())
            .args(args);
        command
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }
    fn install_args(&self, tag: &str) -> Vec<String> {
        vec![
            "definitions".into(),
            "install".into(),
            "--source".into(),
            self.releases().to_str().unwrap().into(),
            "--tag".into(),
            tag.into(),
        ]
    }
    fn install(&self, tag: &str) -> Output {
        let args = self.install_args(tag);
        self.run(&args.iter().map(String::as_str).collect::<Vec<_>>())
    }
    /// The lock's bytes and every path under the cache root, the root itself
    /// included, with its mode and, for a file, its bytes.
    fn snapshot(&self) -> (Option<Vec<u8>>, Tree) {
        fn walk(root: &Path, path: &Path, into: &mut Tree) {
            let metadata = fs::symlink_metadata(path).unwrap();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            let mode = metadata.permissions().mode() & 0o7777;
            if metadata.is_dir() {
                into.insert(relative, (mode, None));
                let mut entries: Vec<_> = fs::read_dir(path)
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .collect();
                entries.sort();
                for entry in entries {
                    walk(root, &entry, into);
                }
            } else {
                into.insert(relative, (mode, Some(fs::read(path).unwrap())));
            }
        }
        let mut cache = BTreeMap::new();
        if self.cache().exists() {
            walk(&self.cache(), &self.cache(), &mut cache);
        }
        (fs::read(self.lock()).ok(), cache)
    }
}

fn envelope(text: &[u8]) -> Value {
    let text = String::from_utf8(text.to_vec()).unwrap();
    assert_eq!(text.lines().count(), 1, "{text}");
    serde_json::from_str(&text).unwrap()
}

/// The failure data of an exit-1 refusal.
fn refusal(output: &Output) -> Value {
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let envelope = envelope(&output.stderr);
    assert_eq!(envelope["ok"], false, "{envelope}");
    envelope["error"]["data"].clone()
}

/// The decompressed archive is not bounded by the stated bounds: a GNU long-name
/// entry (or a pax header) is read whole into memory by the tar reader before
/// any check runs. semantics.md bounds what install reads: 4 MiB of index.json,
/// 64 MiB of asset, at most 512 MiB of files in all. A release that stays inside
/// those (an asset of about 3 MB, 15 bytes of files) must be refused within them.
/// At deflate's ratio limit (about 1032:1) a 64 MiB asset names a 64 GiB entry.
#[cfg(target_os = "linux")]
#[test]
#[allow(
    clippy::zombie_processes,
    reason = "libc::wait4 reaps the child to read its peak resident set"
)]
fn an_archive_that_inflates_past_the_stated_bounds_is_refused_within_them() {
    use connectors_definitions_install::{ASSET_LIMIT, INDEX_LIMIT, UNPACKED_LIMIT};
    use std::io::Read;
    let fixture = Fixture::new(true);
    let tag = "0.3.1";
    let files = BTreeMap::from([("auth/token.yaml".to_owned(), b"profile: token\n".to_vec())]);
    let inflated: u64 = 640 * 1024 * 1024;
    let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::fast(),
    ));
    let mut long = tar::Header::new_gnu();
    long.as_gnu_mut().unwrap().name[..13].copy_from_slice(b"././@LongLink");
    long.set_entry_type(tar::EntryType::GNULongName);
    long.set_size(inflated);
    long.set_mode(0o644);
    long.set_cksum();
    builder
        .append(&long, std::io::repeat(b'a').take(inflated))
        .unwrap();
    let (path, bytes) = files.iter().next().unwrap();
    regular(&mut builder, &format!("{}/{path}", prefix(tag)), bytes);
    let asset = builder.into_inner().unwrap().finish().unwrap();
    assert!((asset.len() as u64) < ASSET_LIMIT / 16, "{}", asset.len());
    publish(&fixture.releases(), tag, &files, &asset);
    let before = fixture.snapshot();

    let mut child = fixture
        .command(
            &fixture
                .install_args(tag)
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let pid = child.id() as libc::pid_t;
    let mut status = 0;
    // SAFETY: rusage is plain data; wait4 writes it for the child this test owns.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    let waited = unsafe { libc::wait4(pid, &mut status, 0, &mut usage) };
    assert_eq!(waited, pid);
    let mut stdout = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    let mut stderr = Vec::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_end(&mut stderr)
        .unwrap();
    let output = Output {
        status: std::os::unix::process::ExitStatusExt::from_raw(status),
        stdout,
        stderr,
    };
    let peak = usage.ru_maxrss as u64 * 1024;

    let refused = refusal(&output);
    assert_eq!(refused["code"], "release_invalid", "{refused}");
    assert_eq!(fixture.snapshot(), before);
    let bound = INDEX_LIMIT + ASSET_LIMIT + UNPACKED_LIMIT;
    assert!(
        peak < bound,
        "a {} byte asset holding 15 bytes of files drove the install to a peak resident \
         set of {peak} bytes; the stated bounds (index + asset + files) sum to {bound}",
        asset.len()
    );
}

/// The lock cannot be written (its directory is read-only): the refusal is
/// `unavailable` at `publication`, and the cache is unchanged, the cache root
/// included.
#[test]
fn a_lock_that_cannot_be_written_leaves_the_cache_unchanged() {
    let fixture = Fixture::new(true);
    let files = files("0.3.1");
    publish(
        &fixture.releases(),
        "0.3.1",
        &files,
        &archive("0.3.1", &files),
    );
    let before = fixture.snapshot();
    fs::set_permissions(
        fixture.config_directory(),
        fs::Permissions::from_mode(0o500),
    )
    .unwrap();
    let output = fixture.install("0.3.1");
    fs::set_permissions(
        fixture.config_directory(),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();

    let refused = refusal(&output);
    assert_eq!(refused["code"], "unavailable", "{refused}");
    assert_eq!(refused["stage"], "publication", "{refused}");
    assert_eq!(
        fixture.snapshot(),
        before,
        "before: no {}; after: {:?}",
        fixture.cache().display(),
        fs::read_dir(fixture.state())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect::<Vec<_>>()
    );
}

/// Probe, held when written. semantics.md: "The command reads the configuration as
/// every command does, so `setup init` comes first." Before `setup init` it is
/// refused, and nothing is written.
#[test]
fn held_install_before_setup_init_is_refused_and_writes_nothing() {
    let fixture = Fixture::new(false);
    let files = files("0.3.1");
    publish(
        &fixture.releases(),
        "0.3.1",
        &files,
        &archive("0.3.1", &files),
    );
    let output = fixture.install("0.3.1");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let data = envelope(&output.stderr)["error"]["data"].clone();
    assert_eq!(data["code"], "invalid_configuration", "{data}");
    assert!(!fixture.state().exists(), "{}", fixture.state().display());
    assert!(!fixture.lock().exists());

    fs::create_dir(fixture.config_directory()).unwrap();
    fs::set_permissions(
        fixture.config_directory(),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    let output = fixture.install("0.3.1");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(!fixture.lock().exists());
    assert!(!fixture.cache().exists());
}

/// Probe, held when written. Concurrent installs of one release and of others,
/// into one configuration: each succeeds, exactly one per release creates the
/// entry, the lock names one installed release, and nothing staged is left.
#[test]
fn held_concurrent_installs_leave_one_consistent_lock_and_nothing_staged() {
    let fixture = Fixture::new(true);
    let tags = ["0.3.1", "0.3.2", "0.3.3"];
    for tag in tags {
        let files = files(tag);
        publish(&fixture.releases(), tag, &files, &archive(tag, &files));
    }
    let children: Vec<_> = (0..6)
        .flat_map(|_| tags)
        .map(|tag| {
            let args = fixture.install_args(tag);
            let child = fixture
                .command(&args.iter().map(String::as_str).collect::<Vec<_>>())
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            (tag, child)
        })
        .collect();
    let mut created: BTreeMap<&str, usize> = BTreeMap::new();
    let mut digests = BTreeSet::new();
    for (tag, child) in children {
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{tag}: {output:?}");
        let result = envelope(&output.stdout)["result"].clone();
        assert_eq!(result["tag"], tag, "{result}");
        digests.insert(result["index_sha256"].as_str().unwrap().to_owned());
        if result["cache_entry"] == "created" {
            *created.entry(tag).or_default() += 1;
        }
    }
    assert_eq!(
        created,
        tags.iter().map(|t| (*t, 1)).collect::<BTreeMap<_, _>>()
    );
    let lock: Value = serde_json::from_slice(&fs::read(fixture.lock()).unwrap()).unwrap();
    assert!(tags.contains(&lock["tag"].as_str().unwrap()), "{lock}");
    assert!(
        digests.contains(lock["index_sha256"].as_str().unwrap()),
        "{lock}"
    );
    let entries: BTreeSet<String> = fs::read_dir(fixture.cache())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(entries, digests);
    let staged: Vec<_> = fs::read_dir(fixture.config_directory())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.starts_with(".definitions.lock"))
        .collect();
    assert!(staged.is_empty(), "{staged:?}");
}

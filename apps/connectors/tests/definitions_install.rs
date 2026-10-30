//! story:definitions-install: `connectors definitions install --source <directory>
//! --tag <tag>` reads a connectors-definitions release served from a local
//! directory, checks every file against its `index.json`, and records the
//! release in `definitions.lock` and a read-only cache keyed by the index's
//! SHA-256 (`contracts/cli/v1alpha1/semantics.md` §3, "Installing a definitions
//! release"; scenarios D01–D05). A digest mismatch, an unknown format version
//! and an unmet feature are each refused by name, and the lock and the cache are
//! then as they were before the command. No network is used.
#![cfg(unix)]
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const SCHEMA: &str = "urn:beyond10x:connectors-definitions-index:1";
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// A release as the connectors-definitions release task publishes it: a
/// `git archive`-shaped gzip tarball under one top directory, with a pax global
/// header first, and `index.json` naming each file's SHA-256 and size.
struct Release {
    tag: String,
    schema: String,
    features: Vec<String>,
    files: BTreeMap<String, Vec<u8>>,
    /// A file whose bytes in the asset differ from what the index states.
    tampered: Option<(String, Vec<u8>)>,
}

impl Release {
    fn new(tag: &str) -> Self {
        let mut files = BTreeMap::new();
        files.insert(
            "connectors/gitlab.yaml".to_owned(),
            b"format: connectors-definition/1\nconnector: gitlab\n".to_vec(),
        );
        files.insert("auth/token.yaml".to_owned(), b"profile: token\n".to_vec());
        files.insert("README.md".to_owned(), b"# definitions\n".to_vec());
        Self {
            tag: tag.to_owned(),
            schema: SCHEMA.to_owned(),
            features: [
                "embedded_text",
                "query_fixed",
                "query_folds",
                "rate_limit_headers",
            ]
            .map(str::to_owned)
            .to_vec(),
            files,
            tampered: None,
        }
    }

    fn asset_name(&self) -> String {
        format!("connectors-definitions-{}.tar.gz", self.tag)
    }

    fn asset(&self) -> Vec<u8> {
        let prefix = format!("connectors-definitions-{}", self.tag);
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        let global = format!("52 comment={COMMIT}\n");
        let mut header = tar::Header::new_ustar();
        header.set_entry_type(tar::EntryType::XGlobalHeader);
        header.set_size(global.len() as u64);
        header.set_mode(0o666);
        builder
            .append_data(&mut header, "pax_global_header", global.as_bytes())
            .unwrap();
        let mut directories = std::collections::BTreeSet::new();
        directories.insert(format!("{prefix}/"));
        for path in self.files.keys() {
            let mut parent = Path::new(path).parent();
            while let Some(directory) = parent.filter(|p| !p.as_os_str().is_empty()) {
                directories.insert(format!("{prefix}/{}/", directory.display()));
                parent = directory.parent();
            }
        }
        for directory in directories {
            let mut header = tar::Header::new_ustar();
            header.set_entry_type(tar::EntryType::Directory);
            header.set_size(0);
            header.set_mode(0o775);
            builder
                .append_data(&mut header, directory, std::io::empty())
                .unwrap();
        }
        for (path, bytes) in &self.files {
            let bytes = match &self.tampered {
                Some((tampered, other)) if tampered == path => other,
                _ => bytes,
            };
            let mut header = tar::Header::new_ustar();
            header.set_entry_type(tar::EntryType::Regular);
            header.set_size(bytes.len() as u64);
            header.set_mode(0o664);
            builder
                .append_data(&mut header, format!("{prefix}/{path}"), bytes.as_slice())
                .unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    /// Writes `<directory>/<tag>/index.json` and the asset beside it, and
    /// returns the index's bytes.
    fn publish(&self, directory: &Path) -> Vec<u8> {
        let asset = self.asset();
        let mut files: Vec<_> = self
            .files
            .iter()
            .map(
                |(path, bytes)| json!({"path": path, "sha256": sha256(bytes), "size": bytes.len()}),
            )
            .collect();
        files.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
        let index = json!({
            "schema": self.schema,
            "tag": self.tag,
            "commit": COMMIT,
            "features": self.features,
            "asset": {"name": self.asset_name(), "sha256": sha256(&asset)},
            "files": files,
        });
        let index = format!("{}\n", serde_json::to_string_pretty(&index).unwrap()).into_bytes();
        let release = directory.join(&self.tag);
        fs::create_dir_all(&release).unwrap();
        fs::write(release.join(self.asset_name()), &asset).unwrap();
        fs::write(release.join("index.json"), &index).unwrap();
        index
    }
}

/// Every path under a directory with its mode and, for a file, its bytes.
type Tree = BTreeMap<PathBuf, (u32, Option<Vec<u8>>)>;

/// A private configuration, a state directory and a release directory; on
/// drop the read-only cache is made writable again so it can be removed.
struct Fixture {
    root: tempfile::TempDir,
}

impl Drop for Fixture {
    fn drop(&mut self) {
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
        writable(self.root.path());
    }
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
        };
        fs::create_dir(fixture.releases()).unwrap();
        let init = fixture.run(&["setup", "init"]);
        assert!(init.status.success(), "{init:?}");
        fixture
    }

    fn releases(&self) -> PathBuf {
        self.root.path().join("releases")
    }

    fn lock(&self) -> PathBuf {
        self.root.path().join("config/definitions.lock")
    }

    fn cache(&self) -> PathBuf {
        self.root.path().join("state/installed-definitions")
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_connectors"))
            .env_clear()
            .args(["--output", "json"])
            .arg("--config")
            .arg(self.root.path().join("config/config.toml"))
            .arg("--state-dir")
            .arg(self.root.path().join("state"))
            .args(args)
            .output()
            .unwrap()
    }

    fn install(&self, tag: &str) -> Output {
        let source = self.releases();
        self.run(&[
            "definitions",
            "install",
            "--source",
            source.to_str().unwrap(),
            "--tag",
            tag,
        ])
    }

    /// The lock's bytes and every path under the cache with its mode and,
    /// for a file, its bytes.
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

/// The result of a `--output json` success: one envelope on stdout, nothing
/// on standard error.
fn result(output: &Output) -> Value {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let text = String::from_utf8(output.stdout.clone()).unwrap();
    assert_eq!(text.lines().count(), 1, "{text}");
    let envelope: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(envelope["ok"], true, "{envelope}");
    envelope["result"].clone()
}

/// The failure of a `--output json` refusal: exit 1, stdout empty, one
/// envelope on standard error.
fn refusal(output: &Output) -> Value {
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let text = String::from_utf8(output.stderr.clone()).unwrap();
    assert_eq!(text.lines().count(), 1, "{text}");
    let envelope: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["error"]["code"], "failure", "{envelope}");
    envelope["error"]["data"].clone()
}

/// Asserts the lock and the cache entry `install` wrote for `release`, whose
/// index has `index` as its bytes, and returns the cache entry.
fn assert_installed(
    fixture: &Fixture,
    release: &Release,
    index: &[u8],
    installed: &Value,
) -> PathBuf {
    let digest = sha256(index);
    let source = fs::canonicalize(fixture.releases()).unwrap();
    let source = source.to_str().unwrap();
    let lock: Value = serde_json::from_slice(&fs::read(fixture.lock()).unwrap()).unwrap();
    assert_eq!(
        lock,
        json!({
            "format": "connectors-definitions-lock/1",
            "source": source,
            "tag": release.tag,
            "commit": COMMIT,
            "index_sha256": digest,
        })
    );
    let lock_mode = fs::metadata(fixture.lock()).unwrap().permissions().mode() & 0o777;
    assert_eq!(lock_mode, 0o600, "the lock is owner-only");
    let entry = fixture.cache().join(&digest);
    assert_eq!(installed["source"], source, "{installed}");
    assert_eq!(installed["tag"], release.tag.as_str(), "{installed}");
    assert_eq!(installed["commit"], COMMIT, "{installed}");
    assert_eq!(installed["index_sha256"], digest.as_str(), "{installed}");
    assert_eq!(
        installed["features"],
        json!(release.features),
        "{installed}"
    );
    assert_eq!(installed["files"], release.files.len(), "{installed}");
    assert_eq!(
        installed["lock_path"],
        fixture.lock().to_str().unwrap(),
        "{installed}"
    );
    assert_eq!(
        installed["cache_path"],
        entry.to_str().unwrap(),
        "{installed}"
    );
    assert_eq!(fs::read(entry.join("index.json")).unwrap(), index);
    for (path, bytes) in &release.files {
        let file = entry.join("files").join(path);
        assert_eq!(&fs::read(&file).unwrap(), bytes, "{}", file.display());
    }
    // Every file and directory of the entry is read-only, and it holds
    // exactly index.json and the indexed files.
    let mut held = Vec::new();
    let mut pending = vec![entry.clone()];
    while let Some(path) = pending.pop() {
        let metadata = fs::symlink_metadata(&path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        if metadata.is_dir() {
            assert_eq!(mode, 0o555, "{}", path.display());
            pending.extend(fs::read_dir(&path).unwrap().map(|e| e.unwrap().path()));
        } else {
            assert!(metadata.is_file(), "{}", path.display());
            assert_eq!(mode, 0o444, "{}", path.display());
            held.push(path.strip_prefix(&entry).unwrap().to_owned());
        }
    }
    held.sort();
    let mut expected: Vec<PathBuf> = release
        .files
        .keys()
        .map(|path| Path::new("files").join(path))
        .collect();
    expected.push(PathBuf::from("index.json"));
    expected.sort();
    assert_eq!(held, expected);
    entry
}

/// D01: the lock names source, tag, commit and index SHA-256, and the
/// read-only cache keyed by that SHA-256 holds every file the index names.
#[test]
fn install_writes_the_lock_and_a_read_only_cache_of_every_indexed_file() {
    let fixture = Fixture::new();
    let release = Release::new("0.3.1");
    let index = release.publish(&fixture.releases());
    let installed = result(&fixture.install("0.3.1"));
    assert_eq!(installed["cache_entry"], "created", "{installed}");
    assert_installed(&fixture, &release, &index, &installed);
    assert!(
        !fixture.root.path().join("state/owner.sock").exists(),
        "install starts no owner"
    );

    // Installing the same release again keeps the verified entry.
    let again = result(&fixture.install("0.3.1"));
    assert_eq!(again["cache_entry"], "existing", "{again}");
    assert_installed(&fixture, &release, &index, &again);
}

/// D02: a file whose SHA-256 differs from its index entry is `digest_mismatch`
/// naming the file; the lock and the cache are as they were.
#[test]
fn a_file_differing_from_its_index_entry_is_refused_and_changes_nothing() {
    let fixture = Fixture::new();
    Release::new("0.3.0").publish(&fixture.releases());
    result(&fixture.install("0.3.0"));
    let before = fixture.snapshot();

    let mut release = Release::new("0.3.1");
    release.tampered = Some((
        "connectors/gitlab.yaml".to_owned(),
        b"format: connectors-definition/1\nconnector: gitlab\nextra: 1\n".to_vec(),
    ));
    release.publish(&fixture.releases());
    let refused = refusal(&fixture.install("0.3.1"));
    assert_eq!(refused["code"], "digest_mismatch", "{refused}");
    assert_eq!(refused["file"], "connectors/gitlab.yaml", "{refused}");
    assert_eq!(refused["stage"], "admission", "{refused}");
    assert_eq!(refused["next_action"], "none", "{refused}");
    assert_eq!(fixture.snapshot(), before);
}

/// D02, the asset itself: an asset whose SHA-256 differs from the index's
/// `asset.sha256` is `digest_mismatch` naming the asset.
#[test]
fn an_asset_differing_from_the_index_is_refused_and_changes_nothing() {
    let fixture = Fixture::new();
    Release::new("0.3.0").publish(&fixture.releases());
    result(&fixture.install("0.3.0"));
    let before = fixture.snapshot();

    let release = Release::new("0.3.1");
    release.publish(&fixture.releases());
    let asset = fixture.releases().join("0.3.1").join(release.asset_name());
    fs::OpenOptions::new()
        .append(true)
        .open(&asset)
        .unwrap()
        .write_all(b"\0")
        .unwrap();
    let refused = refusal(&fixture.install("0.3.1"));
    assert_eq!(refused["code"], "digest_mismatch", "{refused}");
    assert_eq!(refused["file"], release.asset_name(), "{refused}");
    assert_eq!(fixture.snapshot(), before);
}

/// D03: an index of a format version this build does not read is
/// `unknown_format_version`; the lock and the cache are as they were.
#[test]
fn an_unknown_index_format_version_is_refused_and_changes_nothing() {
    let fixture = Fixture::new();
    Release::new("0.3.0").publish(&fixture.releases());
    result(&fixture.install("0.3.0"));
    let before = fixture.snapshot();

    let mut release = Release::new("0.4.0");
    release.schema = "urn:beyond10x:connectors-definitions-index:2".to_owned();
    release.publish(&fixture.releases());
    let refused = refusal(&fixture.install("0.4.0"));
    assert_eq!(refused["code"], "unknown_format_version", "{refused}");
    assert_eq!(refused["stage"], "admission", "{refused}");
    assert_eq!(refused["next_action"], "upgrade", "{refused}");
    assert_eq!(fixture.snapshot(), before);
}

/// D04: a release requiring a feature this build does not support is
/// `unmet_feature` naming the feature; the lock and the cache are as they were.
#[test]
fn an_unmet_feature_is_refused_by_name_and_changes_nothing() {
    let fixture = Fixture::new();
    Release::new("0.3.0").publish(&fixture.releases());
    result(&fixture.install("0.3.0"));
    let before = fixture.snapshot();

    let mut release = Release::new("0.4.0");
    release.features.push("streamed_pages".to_owned());
    release.features.sort();
    release.publish(&fixture.releases());
    let refused = refusal(&fixture.install("0.4.0"));
    assert_eq!(refused["code"], "unmet_feature", "{refused}");
    assert_eq!(refused["feature"], "streamed_pages", "{refused}");
    assert_eq!(refused["stage"], "admission", "{refused}");
    assert_eq!(refused["next_action"], "upgrade", "{refused}");
    assert_eq!(fixture.snapshot(), before);
}

/// D05 (SM-D209): a release whose tag and connectors this binary was never
/// built with installs, without a rebuild, when its format version and
/// features are supported.
#[test]
fn a_release_this_binary_was_not_built_with_installs_without_a_rebuild() {
    let fixture = Fixture::new();
    let mut release = Release::new("97.1.0-unreleased");
    release.files = BTreeMap::from([
        (
            "connectors/made-up-service-7f3a.yaml".to_owned(),
            b"format: connectors-definition/1\nconnector: made-up-service-7f3a\n".to_vec(),
        ),
        (
            "auth/made-up-profile.yaml".to_owned(),
            b"profile: made-up\n".to_vec(),
        ),
    ]);
    release.features = vec!["query_folds".to_owned()];
    let index = release.publish(&fixture.releases());
    let installed = result(&fixture.install("97.1.0-unreleased"));
    assert_eq!(installed["cache_entry"], "created", "{installed}");
    assert_installed(&fixture, &release, &index, &installed);
}

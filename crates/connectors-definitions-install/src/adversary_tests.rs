//! Adversary cases for story:definitions-install, driven from
//! `contracts/cli/v1alpha1/semantics.md` §3 ("Installing a definitions release"),
//! `ess/domains/definitions_install.yaml` and `ess/domains/cli.yaml`. Releases are
//! served from a temporary directory or a plain-HTTP listener on 127.0.0.1; no
//! network is used.
#![cfg(unix)]
use super::*;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const TAG: &str = "0.3.1";

fn files() -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([
        ("auth/token.yaml".to_owned(), b"profile: token\n".to_vec()),
        ("connectors/a.yaml".to_owned(), b"connector: a\n".to_vec()),
    ])
}

fn prefix(tag: &str) -> String {
    format!("connectors-definitions-{tag}")
}

/// A gzip tar archive shaped as `git archive --prefix` writes one: a pax
/// global header, then each file under `prefix/`.
fn archive(prefix: &str, files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
        Vec::new(),
        flate2::Compression::default(),
    ));
    global_header(&mut builder);
    for (path, bytes) in files {
        regular(&mut builder, &format!("{prefix}/{path}"), bytes);
    }
    builder.into_inner().unwrap().finish().unwrap()
}

fn global_header<W: Write>(builder: &mut tar::Builder<W>) {
    let global = format!("52 comment={COMMIT}\n");
    let mut header = tar::Header::new_ustar();
    header.set_entry_type(tar::EntryType::XGlobalHeader);
    header.set_size(global.len() as u64);
    builder
        .append_data(&mut header, "pax_global_header", global.as_bytes())
        .unwrap();
}

fn regular<W: Write>(builder: &mut tar::Builder<W>, path: &str, bytes: &[u8]) {
    let mut header = tar::Header::new_ustar();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    builder.append_data(&mut header, path, bytes).unwrap();
}

fn index_of(
    tag: &str,
    asset: &[u8],
    files: &BTreeMap<String, Vec<u8>>,
    features: &[&str],
) -> Value {
    json!({
        "schema": INDEX_SCHEMAS[0],
        "tag": tag,
        "commit": COMMIT,
        "features": features,
        "asset": {"name": format!("{}.tar.gz", prefix(tag)), "sha256": index::sha256(asset)},
        "files": files
            .iter()
            .map(|(path, bytes)| json!({"path": path, "sha256": index::sha256(bytes), "size": bytes.len()}))
            .collect::<Vec<_>>(),
    })
}

fn chmod(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// Makes every directory and file under `path` writable, without following
/// symbolic links, so a temporary directory can be removed.
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

/// Every path under `root` (the root itself as "") with its mode and, for a
/// file, its bytes; a symbolic link as its target.
type Tree = BTreeMap<PathBuf, (u32, Option<Vec<u8>>)>;

fn tree(root: &Path) -> Tree {
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
        } else if metadata.file_type().is_symlink() {
            let target = fs::read_link(path).unwrap();
            into.insert(
                relative,
                (mode, Some(target.into_os_string().into_encoded_bytes())),
            );
        } else {
            into.insert(relative, (mode, Some(fs::read(path).unwrap())));
        }
    }
    let mut into = BTreeMap::new();
    if fs::symlink_metadata(root).is_ok() {
        walk(root, root, &mut into);
    }
    into
}

struct Roots {
    root: tempfile::TempDir,
}

impl Roots {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }
    fn releases(&self) -> PathBuf {
        self.root.path().join("releases")
    }
    fn cache(&self) -> PathBuf {
        self.root.path().join("state").join(CACHE_DIRECTORY)
    }
    fn lock(&self) -> PathBuf {
        self.root.path().join(LOCK_FILE)
    }
    fn publish(&self, tag: &str, index: &Value, asset: &[u8]) -> Vec<u8> {
        let release = self.releases().join(tag);
        fs::create_dir_all(&release).unwrap();
        fs::write(
            release.join(index["asset"]["name"].as_str().unwrap()),
            asset,
        )
        .unwrap();
        let bytes = serde_json::to_vec_pretty(index).unwrap();
        fs::write(release.join("index.json"), &bytes).unwrap();
        bytes
    }
    fn publish_files(&self, tag: &str, files: &BTreeMap<String, Vec<u8>>, features: &[&str]) {
        let asset = archive(&prefix(tag), files);
        self.publish(tag, &index_of(tag, &asset, files, features), &asset);
    }
    fn install(&self, tag: &str) -> Result<Installed, Refusal> {
        let source = Source::parse(self.releases().to_str().unwrap())?;
        install(&source, tag, &self.cache(), &self.lock())
    }
    /// The lock's bytes and every path under the cache root.
    fn snapshot(&self) -> (Option<Vec<u8>>, Tree) {
        (fs::read(self.lock()).ok(), tree(&self.cache()))
    }
}

impl Drop for Roots {
    fn drop(&mut self) {
        writable(self.root.path());
    }
}

// ---------------------------------------------------------------------------
// An existing cache entry. semantics.md: "An entry that already exists for that
// digest is not replaced: its files are verified against the index as the
// asset's were". definitions_install.yaml: "an entry never changes once
// verified" and "Every file and directory of an entry is read-only".
// ---------------------------------------------------------------------------

/// The asset may hold no file the index does not name (`release_invalid`);
/// an existing entry verified "as the asset's were" may not either.
#[test]
fn an_existing_entry_holding_a_file_the_index_does_not_name_is_refused() {
    let roots = Roots::new();
    roots.publish_files(TAG, &files(), &["query_fixed"]);
    let installed = roots.install(TAG).unwrap();
    let directory = installed.cache_path.join("files/connectors");
    chmod(&directory, 0o700);
    fs::write(directory.join("planted.yaml"), b"connector: planted\n").unwrap();
    chmod(&directory.join("planted.yaml"), 0o444);
    chmod(&directory, 0o555);
    fs::remove_file(roots.lock()).unwrap();

    let outcome = roots.install(TAG);
    assert!(
        matches!(
            outcome,
            Err(Refusal::ReleaseInvalid(_) | Refusal::DigestMismatch(_))
        ),
        "an entry holding files/connectors/planted.yaml, which the index does not name, \
         was accepted as the installed release: {outcome:?}"
    );
    assert!(!roots.lock().exists(), "the lock was moved to the entry");
}

/// D01 requires every file of the entry to be read-only; a success that names an
/// existing entry must hold the same.
#[test]
fn an_existing_entry_a_success_names_is_read_only() {
    let roots = Roots::new();
    roots.publish_files(TAG, &files(), &["query_fixed"]);
    let installed = roots.install(TAG).unwrap();
    // Same bytes, made writable (a copy or restore that drops modes does this).
    chmod(&installed.cache_path.join("files/connectors"), 0o755);
    chmod(&installed.cache_path.join("files/connectors/a.yaml"), 0o644);

    if let Ok(again) = roots.install(TAG) {
        assert!(!again.created);
        for (path, (mode, bytes)) in tree(&again.cache_path) {
            let expected = if bytes.is_some() { 0o444 } else { 0o555 };
            assert_eq!(
                mode,
                expected,
                "{} of the entry the install reported is mode {mode:o}",
                path.display()
            );
        }
    }
}

/// An entry that is a symbolic link to a writable directory elsewhere is not a
/// read-only entry that "never changes once verified".
#[test]
fn an_existing_entry_that_is_a_symbolic_link_is_not_accepted() {
    let roots = Roots::new();
    roots.publish_files(TAG, &files(), &["query_fixed"]);
    let installed = roots.install(TAG).unwrap();
    let elsewhere = roots.root.path().join("elsewhere");
    chmod(&installed.cache_path, 0o700);
    fs::rename(&installed.cache_path, &elsewhere).unwrap();
    writable(&elsewhere);
    std::os::unix::fs::symlink(&elsewhere, &installed.cache_path).unwrap();
    fs::remove_file(roots.lock()).unwrap();

    if let Ok(again) = roots.install(TAG) {
        let kind = fs::symlink_metadata(&again.cache_path).unwrap().file_type();
        assert!(
            kind.is_dir(),
            "the install succeeded and its lock names {}, a symbolic link to the writable \
             directory {}",
            again.cache_path.display(),
            elsewhere.display()
        );
    }
}

// ---------------------------------------------------------------------------
// Index paths that pass the index checks and cannot be stored.
// ---------------------------------------------------------------------------

/// A path segment longer than any file system's name limit (255 bytes) passes
/// the index checks and then fails the write, which is reported as
/// `unavailable` / `retry_explicitly`: a retry can never succeed. The index is
/// not what can be installed, so it is `release_invalid`, and nothing changes.
#[test]
fn a_path_segment_no_file_system_can_create_is_release_invalid() {
    let roots = Roots::new();
    roots.publish_files("0.3.0", &files(), &["query_fixed"]);
    roots.install("0.3.0").unwrap();
    let before = roots.snapshot();

    let mut long = files();
    long.insert(
        format!("connectors/{}.yaml", "a".repeat(256)),
        b"connector: long\n".to_vec(),
    );
    roots.publish_files(TAG, &long, &["query_fixed"]);
    let outcome = roots.install(TAG);
    assert_eq!(roots.snapshot(), before);
    assert!(
        matches!(outcome, Err(Refusal::ReleaseInvalid(_))),
        "a 261-byte path segment: {outcome:?} (code {:?})",
        outcome.as_ref().err().map(Refusal::code)
    );
}

/// `relative_path` promises a path that "names one file beneath any root on every
/// platform", and macOS binaries ship. `connectors/A.yaml` and `connectors/a.yaml`
/// name one file on a case-insensitive file system (the macOS default), where the
/// second write fails as `unavailable`. One release must not install on Linux and
/// be a retryable failure on macOS.
#[test]
fn paths_that_name_one_file_on_a_case_insensitive_file_system_are_release_invalid() {
    let roots = Roots::new();
    let mut colliding = files();
    colliding.insert("connectors/A.yaml".to_owned(), b"connector: A\n".to_vec());
    roots.publish_files(TAG, &colliding, &["query_fixed"]);
    let outcome = roots.install(TAG);
    assert!(
        matches!(outcome, Err(Refusal::ReleaseInvalid(_))),
        "connectors/A.yaml beside connectors/a.yaml: {outcome:?}"
    );
}

// ---------------------------------------------------------------------------
// features. ess/domains/cli.yaml (DefinitionsInstallResult): "features the
// release's sorted features"; (Failure.feature): "the first feature, in the
// index's sorted order". The index format states features are sorted.
// ---------------------------------------------------------------------------

#[test]
fn the_features_an_install_reports_are_sorted() {
    let roots = Roots::new();
    roots.publish_files(
        TAG,
        &files(),
        &["query_folds", "embedded_text", "query_folds"],
    );
    match roots.install(TAG) {
        Ok(installed) => assert_eq!(
            installed.features,
            ["embedded_text", "query_folds"],
            "the result's features are the index's, unsorted and with a duplicate"
        ),
        Err(refusal) => assert!(matches!(refusal, Refusal::ReleaseInvalid(_)), "{refusal:?}"),
    }
}

// ---------------------------------------------------------------------------
// Source::Https is a public variant: a caller can build one without
// Source::parse. semantics.md: "Any other scheme ... is invalid_input", and the
// lock records an https source. story:definitions-pin-covers-catalog adds a
// second caller of this library.
// ---------------------------------------------------------------------------

#[test]
fn an_https_source_built_without_parse_is_still_https_only() {
    let roots = Roots::new();
    let files = files();
    let asset = archive(&prefix(TAG), &files);
    let index = serde_json::to_vec(&index_of(TAG, &asset, &files, &["query_fixed"])).unwrap();
    let server = Server::start(BTreeMap::from([
        (
            format!("/owner/definitions/releases/download/{TAG}/index.json"),
            Reply::ok(index),
        ),
        (
            format!(
                "/owner/definitions/releases/download/{TAG}/{}.tar.gz",
                prefix(TAG)
            ),
            Reply::ok(asset),
        ),
    ]));
    let source = Source::Https(url::Url::parse(&server.url("/owner/definitions")).unwrap());
    let outcome = install(&source, TAG, &roots.cache(), &roots.lock());
    assert_eq!(
        outcome.as_ref().err(),
        Some(&Refusal::InvalidSource),
        "install fetched over plain http and locked source {:?}",
        outcome.as_ref().ok().map(|i| &i.lock.source)
    );
}

// ---------------------------------------------------------------------------
// Probes that held when written: kept so the property stays pinned.
// ---------------------------------------------------------------------------

/// Duplicate JSON keys: the format-version pass reads the last `schema`, the
/// struct pass refuses a duplicate. Either way it is a refusal, never a read.
#[test]
fn held_an_index_with_a_duplicate_key_is_refused() {
    let files = files();
    let asset = archive(&prefix(TAG), &files);
    let compact = serde_json::to_string(&index_of(TAG, &asset, &files, &["query_fixed"])).unwrap();
    let body = compact.strip_prefix('{').unwrap();
    let late = format!(r#"{{"schema":"urn:example:unknown",{body}"#);
    assert!(matches!(
        Index::read(late.as_bytes(), TAG),
        Err(Refusal::ReleaseInvalid(_))
    ));
    let early = format!(
        "{},\"schema\":\"urn:example:unknown\"}}",
        &compact[..compact.len() - 1]
    );
    assert_eq!(
        Index::read(early.as_bytes(), TAG),
        Err(Refusal::UnknownFormatVersion)
    );
    let features = compact.replacen(
        r#""features":["query_fixed"]"#,
        r#""features":["streamed_pages"],"features":["query_fixed"]"#,
        1,
    );
    assert!(matches!(
        Index::read(features.as_bytes(), TAG),
        Err(Refusal::ReleaseInvalid(_))
    ));
}

/// A pax `path` record overrides the header's path; an override outside the top
/// directory is refused, and one onto an indexed path is that path.
#[test]
fn held_a_pax_path_override_is_the_path_checked() {
    let files = files();
    let first = format!("{}/{}", prefix(TAG), files.keys().next().unwrap());
    let build = |header_path: &str, override_path: &str| {
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::default(),
        ));
        global_header(&mut builder);
        let bytes = files.values().next().unwrap();
        let rest = format!(" path={override_path}\n");
        let mut length = rest.len() + 1;
        while length != rest.len() + length.to_string().len() {
            length = rest.len() + length.to_string().len();
        }
        let record = format!("{length}{rest}");
        let mut header = tar::Header::new_ustar();
        header.set_entry_type(tar::EntryType::XHeader);
        header.set_size(record.len() as u64);
        builder
            .append_data(&mut header, "PaxHeaders/entry", record.as_bytes())
            .unwrap();
        regular(&mut builder, header_path, bytes);
        for (path, bytes) in files.iter().skip(1) {
            regular(&mut builder, &format!("{}/{path}", prefix(TAG)), bytes);
        }
        builder.into_inner().unwrap().finish().unwrap()
    };
    for escape in [
        format!("{}/../../escape", prefix(TAG)),
        "/etc/passwd".to_owned(),
        format!("{}/auth\\token.yaml", prefix(TAG)),
    ] {
        let roots = Roots::new();
        let asset = build(&first, &escape);
        roots.publish(TAG, &index_of(TAG, &asset, &files, &[]), &asset);
        let outcome = roots.install(TAG);
        assert!(
            matches!(outcome, Err(Refusal::ReleaseInvalid(_))),
            "{escape}: {outcome:?}"
        );
        assert!(!roots.cache().exists() && !roots.lock().exists());
    }
    // The header names a decoy the index does not; the pax path names the file.
    let roots = Roots::new();
    let asset = build(&format!("{}/decoy", prefix(TAG)), &first);
    roots.publish(TAG, &index_of(TAG, &asset, &files, &[]), &asset);
    roots.install(TAG).unwrap();
}

/// A link entry at an indexed path whose index entry is empty: refused as
/// release_invalid. The suite's own symbolic-link case uses an unindexed path, so
/// it stays green with the `!kind.is_file()` guard in archive.rs removed.
#[test]
fn held_a_link_at_an_indexed_empty_path_is_release_invalid() {
    let mut files = files();
    files.insert("connectors/empty.yaml".to_owned(), Vec::new());
    for kind in [tar::EntryType::Symlink, tar::EntryType::Link] {
        let roots = Roots::new();
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::default(),
        ));
        global_header(&mut builder);
        for (path, bytes) in &files {
            let full = format!("{}/{path}", prefix(TAG));
            if bytes.is_empty() {
                let mut header = tar::Header::new_ustar();
                header.set_entry_type(kind);
                header.set_size(0);
                builder
                    .append_link(&mut header, full, "/etc/passwd")
                    .unwrap();
            } else {
                regular(&mut builder, &full, bytes);
            }
        }
        let asset = builder.into_inner().unwrap().finish().unwrap();
        roots.publish(TAG, &index_of(TAG, &asset, &files, &[]), &asset);
        let outcome = roots.install(TAG);
        assert!(
            matches!(outcome, Err(Refusal::ReleaseInvalid(_))),
            "{kind:?}: {outcome:?}"
        );
    }
}

/// An asset name that is a percent-encoded dot segment stays one URL segment.
#[test]
fn held_an_encoded_dot_asset_name_stays_one_url_segment() {
    let base = url::Url::parse("https://example.invalid/owner/definitions/").unwrap();
    for name in ["%2e%2e", "%2E.", ".%2e", "%2e", "a?b#c", "a%2fb"] {
        let url = fetch::release_url(&base, TAG, name).unwrap();
        let segments: Vec<_> = url.path_segments().unwrap().collect();
        assert_eq!(segments.len(), 6, "{name}: {url}");
        assert_eq!(
            &segments[..5],
            ["owner", "definitions", "releases", "download", TAG]
        );
    }
}

/// HTTP statuses, a redirect to http, an over-long Content-Length, and the
/// request path built from a source with a trailing slash and an encoded
/// segment, over a plain-HTTP listener (the redirect policy is the client's).
#[test]
fn held_http_statuses_redirects_and_bounds() {
    let roots = Roots::new();
    let index_path = |base: &str| format!("{base}/releases/download/{TAG}/index.json");
    let server = Server::start(BTreeMap::from([
        (index_path("/missing"), Reply::status(404)),
        (index_path("/broken"), Reply::status(500)),
        (index_path("/gone"), Reply::status(410)),
        (
            index_path("/downgrade"),
            Reply::redirect("http://127.0.0.1:9/plain"),
        ),
        (
            index_path("/huge"),
            Reply {
                status: 200,
                headers: vec![("Content-Length".into(), (INDEX_LIMIT + 1).to_string())],
                body: b"{}".to_vec(),
                raw_length: true,
            },
        ),
        (index_path("/a%20b/r"), Reply::status(404)),
    ]));
    // `install` refuses a plain-http source
    // (an_https_source_built_without_parse_is_still_https_only), so the HTTPS
    // client is reached directly over this plain-HTTP listener.
    let run = |path: &str| {
        let url = url::Url::parse(&server.url(path)).unwrap();
        let fetcher = crate::fetch::Fetcher::https(&url).unwrap();
        crate::install_with(
            fetcher,
            url.as_str().trim_end_matches('/').to_owned(),
            TAG,
            &roots.cache(),
            &roots.lock(),
        )
    };
    assert_eq!(run("/missing").err(), Some(Refusal::NotFound));
    assert_eq!(run("/broken").err(), Some(Refusal::Unavailable));
    assert_eq!(run("/gone").err(), Some(Refusal::Unavailable));
    assert_eq!(run("/downgrade").err(), Some(Refusal::Unavailable));
    assert_eq!(run("/huge").err(), Some(Refusal::Unavailable));
    assert_eq!(run("/a%20b/r/").err(), Some(Refusal::NotFound));
    assert!(
        server
            .observed()
            .contains(&format!("/a%20b/r/releases/download/{TAG}/index.json")),
        "{:?}",
        server.observed()
    );
    assert!(
        !server.observed().iter().any(|p| p.contains("plain")),
        "{:?}",
        server.observed()
    );
    assert!(!roots.cache().exists() && !roots.lock().exists());
}

// ---------------------------------------------------------------------------
// A plain-HTTP listener answering by request path.
// ---------------------------------------------------------------------------

struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    /// The headers carry their own Content-Length.
    raw_length: bool,
}

impl Reply {
    fn ok(body: Vec<u8>) -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body,
            raw_length: false,
        }
    }
    fn status(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
            raw_length: false,
        }
    }
    fn redirect(location: &str) -> Self {
        Self {
            status: 302,
            headers: vec![("Location".into(), location.into())],
            body: Vec::new(),
            raw_length: false,
        }
    }
}

struct Server {
    port: u16,
    observed: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    fn start(routes: BTreeMap<String, Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let observed = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (seen, stopped) = (observed.clone(), stop.clone());
        let thread = std::thread::spawn(move || {
            for stream in listener.incoming() {
                if stopped.load(Ordering::SeqCst) {
                    return;
                }
                if let Ok(stream) = stream {
                    serve(stream, &routes, &seen);
                }
            }
        });
        Self {
            port,
            observed,
            stop,
            thread: Some(thread),
        }
    }
    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
    fn observed(&self) -> Vec<String> {
        self.observed.lock().unwrap().clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(mut stream: TcpStream, routes: &BTreeMap<String, Reply>, seen: &Mutex<Vec<String>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => return,
        }
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let target = head
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_owned();
    seen.lock().unwrap().push(target.clone());
    let fallback = Reply::status(404);
    let reply = routes.get(&target).unwrap_or(&fallback);
    let mut out = format!("HTTP/1.1 {} Fixture\r\nConnection: close\r\n", reply.status);
    if !reply.raw_length {
        out.push_str(&format!("Content-Length: {}\r\n", reply.body.len()));
    }
    for (name, value) in &reply.headers {
        out.push_str(&format!("{name}: {value}\r\n"));
    }
    out.push_str("\r\n");
    let mut bytes = out.into_bytes();
    bytes.extend_from_slice(&reply.body);
    let _ = stream.write_all(&bytes);
    let _ = stream.flush();
}

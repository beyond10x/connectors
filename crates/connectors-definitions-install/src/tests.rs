use super::*;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

/// A gzip tar archive with `entries` under `prefix/`, in order, after a pax
/// global header, as `git archive --prefix` writes one.
fn archive(prefix: &str, entries: &[(&str, &[u8])]) -> Vec<u8> {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    let global = format!("52 comment={COMMIT}\n");
    let mut header = tar::Header::new_ustar();
    header.set_entry_type(tar::EntryType::XGlobalHeader);
    header.set_size(global.len() as u64);
    builder
        .append_data(&mut header, "pax_global_header", global.as_bytes())
        .unwrap();
    for (path, bytes) in entries {
        let mut header = tar::Header::new_ustar();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        builder
            .append_data(&mut header, format!("{prefix}/{path}"), *bytes)
            .unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap()
}

fn index_of(tag: &str, asset: &[u8], files: &[(&str, &[u8])]) -> Value {
    json!({
        "schema": INDEX_SCHEMAS[0],
        "tag": tag,
        "commit": COMMIT,
        "features": ["query_fixed"],
        "asset": {"name": format!("connectors-definitions-{tag}.tar.gz"), "sha256": index::sha256(asset)},
        "files": files.iter().map(|(path, bytes)| json!({"path": path, "sha256": index::sha256(bytes), "size": bytes.len()})).collect::<Vec<_>>(),
    })
}

const FILES: &[(&str, &[u8])] = &[
    ("auth/token.yaml", b"profile: token\n"),
    ("connectors/a.yaml", b"connector: a\n"),
];

/// Writes `<dir>/<tag>/index.json` and the asset; returns the index bytes.
fn publish(directory: &std::path::Path, tag: &str, index: &Value, asset: &[u8]) -> Vec<u8> {
    let release = directory.join(tag);
    fs::create_dir_all(&release).unwrap();
    fs::write(
        release.join(index["asset"]["name"].as_str().unwrap()),
        asset,
    )
    .unwrap();
    let bytes = serde_json::to_vec(index).unwrap();
    fs::write(release.join("index.json"), &bytes).unwrap();
    bytes
}

fn read(index: &Value, tag: &str) -> Result<Index, Refusal> {
    Index::read(&serde_json::to_vec(index).unwrap(), tag)
}

#[test]
fn a_well_formed_index_is_read() {
    let asset = archive("connectors-definitions-0.3.1", FILES);
    let index = read(&index_of("0.3.1", &asset, FILES), "0.3.1").unwrap();
    assert_eq!(index.files.len(), 2);
    assert_eq!(index.features, ["query_fixed"]);
}

#[test]
fn the_format_version_is_checked_before_anything_else() {
    let asset = archive("p", FILES);
    let mut index = index_of("0.3.1", &asset, FILES);
    index["schema"] = json!("urn:beyond10x:connectors-definitions-index:2");
    // Another tag and an unknown member too: the format version decides first.
    index["tag"] = json!("other");
    index["added"] = json!(true);
    assert_eq!(read(&index, "0.3.1"), Err(Refusal::UnknownFormatVersion));
    index.as_object_mut().unwrap().remove("schema");
    assert_eq!(read(&index, "0.3.1"), Err(Refusal::UnknownFormatVersion));
}

#[test]
fn an_unmet_feature_is_named() {
    let asset = archive("p", FILES);
    let mut index = index_of("0.3.1", &asset, FILES);
    index["features"] = json!(["embedded_text", "streamed_pages", "zeta"]);
    assert_eq!(
        read(&index, "0.3.1"),
        Err(Refusal::UnmetFeature("streamed_pages".to_owned()))
    );
    // Every supported feature is admitted.
    index["features"] = json!(FEATURES);
    assert!(read(&index, "0.3.1").is_ok());
    // A feature that is not a bounded identifier is not echoed.
    index["features"] = json!(["Not A Feature"]);
    assert!(matches!(
        read(&index, "0.3.1"),
        Err(Refusal::ReleaseInvalid(_))
    ));
}

#[test]
fn an_index_outside_its_format_is_release_invalid() {
    let asset = archive("p", FILES);
    let base = index_of("0.3.1", &asset, FILES);
    let refused = |change: &dyn Fn(&mut Value), tag: &str| {
        let mut index = base.clone();
        change(&mut index);
        let outcome = read(&index, tag);
        assert!(
            matches!(outcome, Err(Refusal::ReleaseInvalid(_))),
            "{index}: {outcome:?}"
        );
    };
    refused(&|_| {}, "0.3.2");
    refused(&|i| i["commit"] = json!("abc"), "0.3.1");
    refused(&|i| i["commit"] = json!(COMMIT.to_uppercase()), "0.3.1");
    refused(&|i| i["asset"]["sha256"] = json!("00"), "0.3.1");
    refused(&|i| i["asset"]["name"] = json!("../x.tar.gz"), "0.3.1");
    refused(&|i| i["asset"]["name"] = json!("a/x.tar.gz"), "0.3.1");
    refused(&|i| i["files"][0]["path"] = json!("/etc/passwd"), "0.3.1");
    refused(&|i| i["files"][0]["path"] = json!("auth/../x"), "0.3.1");
    refused(&|i| i["files"][0]["path"] = json!("auth//x"), "0.3.1");
    refused(&|i| i["files"][0]["path"] = json!("a\\b"), "0.3.1");
    refused(&|i| i["files"].as_array_mut().unwrap().reverse(), "0.3.1");
    refused(
        &|i| {
            let first = i["files"][0].clone();
            i["files"].as_array_mut().unwrap().push(first);
        },
        "0.3.1",
    );
    refused(
        &|i| {
            i["files"][0]["path"] = json!("auth");
            i["files"][1]["path"] = json!("auth/x");
        },
        "0.3.1",
    );
    refused(
        &|i| i["files"][0]["size"] = json!(UNPACKED_LIMIT + 1),
        "0.3.1",
    );
    refused(&|i| i["added"] = json!(1), "0.3.1");
    assert!(matches!(
        Index::read(b"not json", "0.3.1"),
        Err(Refusal::ReleaseInvalid(_))
    ));
}

#[test]
fn tags_are_one_path_segment() {
    for tag in ["0.3.1", "v1.0.0-rc_1", "a"] {
        assert!(valid_tag(tag), "{tag}");
    }
    let long = "a".repeat(129);
    for tag in ["", ".", "..", "a/b", "a b", "a\\b", long.as_str()] {
        assert!(!valid_tag(tag), "{tag}");
    }
}

#[test]
fn sources_are_https_urls_or_existing_directories() {
    let source = Source::parse("https://example.invalid/owner/definitions/").unwrap();
    assert_eq!(
        source.recorded(),
        "https://example.invalid/owner/definitions"
    );
    let Source::Https(base) = &source else {
        panic!("{source:?}")
    };
    assert_eq!(
        fetch::release_url(base, "0.3.1", "index.json")
            .unwrap()
            .as_str(),
        "https://example.invalid/owner/definitions/releases/download/0.3.1/index.json"
    );
    for refused in [
        "http://example.invalid/r",
        "ftp://example.invalid/r",
        "https://user:pw@example.invalid/r",
        "https://example.invalid/r?x=1",
        "https://example.invalid/r#x",
        "",
    ] {
        assert_eq!(
            Source::parse(refused),
            Err(Refusal::InvalidSource),
            "{refused}"
        );
    }
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(
        Source::parse(directory.path().join("absent").to_str().unwrap()),
        Err(Refusal::NotFound)
    );
    let canonical = fs::canonicalize(directory.path()).unwrap();
    assert_eq!(
        Source::parse(directory.path().to_str().unwrap()).unwrap(),
        Source::Directory(canonical)
    );
}

/// A release directory, a cache and a lock in one temporary directory; on drop
/// the read-only cache is removed first, which the directory's own removal
/// cannot do.
struct Roots {
    root: tempfile::TempDir,
}

impl Roots {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }
    fn releases(&self) -> std::path::PathBuf {
        self.root.path().join("releases")
    }
    fn cache(&self) -> std::path::PathBuf {
        self.root.path().join("state").join(CACHE_DIRECTORY)
    }
    fn lock(&self) -> std::path::PathBuf {
        self.root.path().join(LOCK_FILE)
    }
    fn install(&self, tag: &str) -> Result<Installed, Refusal> {
        let source = Source::parse(self.releases().to_str().unwrap())?;
        install(&source, tag, &self.cache(), &self.lock())
    }
}

impl Drop for Roots {
    fn drop(&mut self) {
        store::remove(&self.cache());
    }
}

#[test]
fn install_writes_the_entry_then_the_lock_and_reuses_a_verified_entry() {
    let roots = Roots::new();
    let asset = archive("connectors-definitions-0.3.1", FILES);
    let index = publish(
        &roots.releases(),
        "0.3.1",
        &index_of("0.3.1", &asset, FILES),
        &asset,
    );
    let installed = roots.install("0.3.1").unwrap();
    let digest = index::sha256(&index);
    assert!(installed.created);
    assert_eq!(installed.cache_path, roots.cache().join(&digest));
    assert_eq!(installed.lock.index_sha256, digest);
    assert_eq!(installed.files, 2);
    let lock: Lock = serde_json::from_slice(&fs::read(roots.lock()).unwrap()).unwrap();
    assert_eq!(lock, installed.lock);
    assert_eq!(lock.format, LOCK_FORMAT);
    assert_eq!(lock.commit, COMMIT);
    for (path, bytes) in FILES {
        assert_eq!(
            fs::read(installed.cache_path.join("files").join(path)).unwrap(),
            *bytes
        );
    }
    // Nothing staged is left beside the entry or the lock.
    let names = |dir: std::path::PathBuf| -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect()
    };
    assert_eq!(names(roots.cache()), std::slice::from_ref(&digest));
    assert!(
        !names(roots.root.path().to_owned())
            .iter()
            .any(|n| n.starts_with('.'))
    );

    let again = roots.install("0.3.1").unwrap();
    assert!(!again.created);
    assert_eq!(again.lock, installed.lock);
}

/// An entry that no longer holds the index's bytes is refused naming the file,
/// and the lock is not moved to it.
#[cfg(unix)]
#[test]
fn an_altered_existing_entry_is_refused_naming_the_file() {
    use std::os::unix::fs::PermissionsExt;
    let roots = Roots::new();
    let asset = archive("connectors-definitions-0.3.1", FILES);
    publish(
        &roots.releases(),
        "0.3.1",
        &index_of("0.3.1", &asset, FILES),
        &asset,
    );
    let installed = roots.install("0.3.1").unwrap();
    let altered = installed.cache_path.join("files/connectors/a.yaml");
    let directory = altered.parent().unwrap();
    fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
    fs::set_permissions(&altered, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&altered, b"connector: b\n").unwrap();
    fs::remove_file(roots.lock()).unwrap();
    assert_eq!(
        roots.install("0.3.1"),
        Err(Refusal::DigestMismatch("connectors/a.yaml".to_owned()))
    );
    assert!(!roots.lock().exists());
}

#[test]
fn the_asset_must_hold_exactly_the_indexed_files_under_one_top_directory() {
    let refused = |entries: &[(&str, &[u8])], prefix: &str| {
        let roots = Roots::new();
        let asset = archive(prefix, entries);
        publish(
            &roots.releases(),
            "0.3.1",
            &index_of("0.3.1", &asset, FILES),
            &asset,
        );
        let outcome = roots.install("0.3.1");
        assert!(!roots.cache().exists() && !roots.lock().exists());
        outcome
    };
    let extra: &[(&str, &[u8])] = &[FILES[0], FILES[1], ("connectors/b.yaml", b"b\n")];
    assert!(matches!(
        refused(extra, "p"),
        Err(Refusal::ReleaseInvalid(_))
    ));
    assert!(matches!(
        refused(&FILES[..1], "p"),
        Err(Refusal::ReleaseInvalid(_))
    ));
    let twice: &[(&str, &[u8])] = &[FILES[0], FILES[0], FILES[1]];
    assert!(matches!(
        refused(twice, "p"),
        Err(Refusal::ReleaseInvalid(_))
    ));
    // A file of other bytes is a digest mismatch naming it.
    let other: &[(&str, &[u8])] = &[FILES[0], ("connectors/a.yaml", b"connector: z\n")];
    assert_eq!(
        refused(other, "p"),
        Err(Refusal::DigestMismatch("connectors/a.yaml".to_owned()))
    );
    let longer: &[(&str, &[u8])] = &[FILES[0], ("connectors/a.yaml", b"connector: a\n\n")];
    assert_eq!(
        refused(longer, "p"),
        Err(Refusal::DigestMismatch("connectors/a.yaml".to_owned()))
    );

    // Two top directories.
    let roots = Roots::new();
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    for (top, (path, bytes)) in ["one", "two"].iter().zip(FILES) {
        let mut header = tar::Header::new_ustar();
        header.set_size(bytes.len() as u64);
        builder
            .append_data(&mut header, format!("{top}/{path}"), *bytes)
            .unwrap();
    }
    let asset = builder.into_inner().unwrap().finish().unwrap();
    publish(
        &roots.releases(),
        "0.3.1",
        &index_of("0.3.1", &asset, FILES),
        &asset,
    );
    assert!(matches!(
        roots.install("0.3.1"),
        Err(Refusal::ReleaseInvalid(_))
    ));

    // A symbolic link entry.
    let roots = Roots::new();
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    for (path, bytes) in FILES {
        let mut header = tar::Header::new_ustar();
        header.set_size(bytes.len() as u64);
        builder
            .append_data(&mut header, format!("p/{path}"), *bytes)
            .unwrap();
    }
    let mut header = tar::Header::new_ustar();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_size(0);
    builder
        .append_link(&mut header, "p/link", "/etc/passwd")
        .unwrap();
    let asset = builder.into_inner().unwrap().finish().unwrap();
    publish(
        &roots.releases(),
        "0.3.1",
        &index_of("0.3.1", &asset, FILES),
        &asset,
    );
    assert!(matches!(
        roots.install("0.3.1"),
        Err(Refusal::ReleaseInvalid(_))
    ));
}

#[test]
fn an_asset_differing_from_its_digest_is_named() {
    let roots = Roots::new();
    let asset = archive("p", FILES);
    let index = index_of("0.3.1", &asset, FILES);
    let mut changed = asset.clone();
    changed.push(0);
    publish(&roots.releases(), "0.3.1", &index, &changed);
    assert_eq!(
        roots.install("0.3.1"),
        Err(Refusal::DigestMismatch(
            "connectors-definitions-0.3.1.tar.gz".to_owned()
        ))
    );
}

#[test]
fn a_release_the_source_lacks_is_not_found() {
    let roots = Roots::new();
    fs::create_dir_all(roots.releases()).unwrap();
    assert_eq!(roots.install("0.3.1"), Err(Refusal::NotFound));
    let asset = archive("p", FILES);
    let index = index_of("0.3.1", &asset, FILES);
    let release = roots.releases().join("0.3.1");
    fs::create_dir_all(&release).unwrap();
    fs::write(
        release.join("index.json"),
        serde_json::to_vec(&index).unwrap(),
    )
    .unwrap();
    assert_eq!(roots.install("0.3.1"), Err(Refusal::NotFound));
    assert_eq!(roots.install("../0.3.1"), Err(Refusal::InvalidTag));
}

/// SM-D209: install takes only the format versions and features from the build.
/// It depends on no connectors crate, so no catalog, declaration or lowered
/// definition can reach it, and it builds before `connectors-catalog-provider`.
#[test]
fn the_installer_depends_on_no_connectors_crate() {
    let manifest: toml::Table = toml::from_str(
        &fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap(),
    )
    .unwrap();
    let dependencies: BTreeMap<_, _> = ["dependencies", "build-dependencies"]
        .iter()
        .filter_map(|table| manifest.get(*table).and_then(|t| t.as_table()))
        .flat_map(|table| table.keys().cloned())
        .map(|name| (name, ()))
        .collect();
    assert!(!dependencies.is_empty());
    for name in dependencies.keys() {
        assert!(!name.starts_with("connectors"), "{name}");
    }
    assert!(!std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/build.rs")).exists());
}

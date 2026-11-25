//! The `mounted_secret` binding against real files on a read-only mount.
//!
//! Each test lays its directories out from the host, then re-enters this test
//! binary, filtered to itself, in a [`Jail`]: its own user and mount
//! namespaces, the laid-out source bound read-only at the configured
//! directory, and the jail as its root. The re-entered test sees
//! `CONNECTORS_MOUNTED_JAIL` and runs the checks there. Changes the child makes
//! under `/srv/source` stay visible through the read-only `/srv/custody`, as a
//! kubelet's atomic update is visible through a Secret volume.
use super::{
    Identity, Item, MountedSecret, available,
    fixture::{Jail, Map},
    inspect,
};
use crate::{Failure, MAX_BYTES, Material, ReadBackend, Result, Scope, State, Version, contract};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::Stdio,
};
use uuid::Uuid;

const CHILD: &str = "CONNECTORS_MOUNTED_JAIL";
/// Inside the jail: the configured directory, bound read-only from `SOURCE`.
const CUSTODY: &str = "/srv/custody";
const SOURCE: &str = "/srv/source";
/// The binding's own directory in the state directory.
const PINS: &str = "/srv/state/custody-mounted";
const STAMP: &str = "..2026_10_13_00_00_00.000000001";
const SECRET: &[u8] = b"fictional-mounted-credential-0123";

fn in_child() -> bool {
    std::env::var_os(CHILD).is_some()
}

/// Run the calling test, `name`, again inside `jail` under `map`, and fail
/// with its output unless it passes.
fn run_in(jail: &Jail, name: &str, map: Map) {
    jail.install(&std::env::current_exe().unwrap(), "/bin-test/custody");
    let output = Jail::output(
        jail.command("/bin-test/custody", map)
            .args(["--exact", name, "--nocapture", "--test-threads=1"])
            .env_clear()
            .env(CHILD, "1")
            .env("TMPDIR", "/work")
            .stdin(Stdio::null()),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "jailed {name} failed: {}\n{stdout}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A jail with `/srv` (0755), a source directory, the configured directory
/// it is bound to read-only, a state directory and a work directory.
fn jail() -> Jail {
    let mut jail = Jail::new();
    jail.directory("/srv", 0o755);
    jail.directory(SOURCE, 0o755);
    jail.directory(CUSTODY, 0o755);
    jail.directory("/srv/state", 0o700);
    jail.directory("/work", 0o700);
    jail.read_only(SOURCE, CUSTODY);
    jail
}

fn write(path: &Path, bytes: &[u8], mode: u32) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// A kubelet atomic writer's layout under `source`: items in a timestamped
/// directory, `..data` naming it, and a top-level link per alias into `..data`.
fn projected(source: &Path, items: &[(&str, &str, &[u8], u32)]) {
    let stamp = source.join(STAMP);
    fs::create_dir_all(&stamp).unwrap();
    for (alias, profile, bytes, mode) in items {
        write(&stamp.join(alias).join(profile), bytes, *mode);
        let link = source.join(alias);
        if link.symlink_metadata().is_err() {
            symlink(Path::new("..data").join(alias), link).unwrap();
        }
    }
    let data = source.join("..data");
    if data.symlink_metadata().is_err() {
        symlink(STAMP, data).unwrap();
    }
}

fn scope() -> Scope {
    Scope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap()
}

fn open() -> MountedSecret {
    MountedSecret::open(scope(), Path::new(CUSTODY), Path::new(PINS)).unwrap()
}

fn item(alias: &str, profile: &str) -> Item {
    Item::new(alias, profile).unwrap()
}

/// Neither the bytes nor any path or layout name reaches a failure's text.
fn redacted(failure: Failure, binding: &MountedSecret, secrets: &[&[u8]]) {
    let text = format!("{failure:?} {failure} {binding:?}");
    for secret in secrets {
        assert!(
            !text.contains(&*String::from_utf8_lossy(secret)),
            "the bytes in {text}"
        );
    }
    for word in ["srv", CUSTODY, SOURCE, PINS, "..data", STAMP, "tavily", "/"] {
        assert!(!text.contains(word), "{word} in {text}");
    }
}

/// The generic read checks against `mounted_secret`: the suite's bytes are
/// placed as a platform delivers them, a new item in the source directory,
/// and pinned by reading it through the read-only mount.
#[test]
fn contract_suite_read_only() {
    if !in_child() {
        let jail = jail();
        projected(&jail.path(SOURCE), &[]);
        return run_in(&jail, "mounted::tests::contract_suite_read_only", Map::Own);
    }
    struct Platform;
    impl contract::Placement for Platform {
        type Binding = MountedSecret;
        fn open(&self, scope: Scope) -> Result<MountedSecret> {
            MountedSecret::open(scope, Path::new(CUSTODY), Path::new(PINS))
        }
        fn place(&self, binding: &MountedSecret, version: Version, material: &[u8]) {
            let alias = format!("a{}", Uuid::new_v4().simple());
            let path = Path::new(SOURCE).join(STAMP).join(&alias).join("profile");
            write(&path, material, 0o400);
            let item = item(&alias, "profile");
            let reading = binding.read_item(&item).unwrap();
            binding.pin(version, &item, &reading, || Ok(())).unwrap();
        }
    }
    contract::run_read_only(&Platform);
    // Nothing the suite did wrote to the mount.
    assert!(fs::write(Path::new(CUSTODY).join("probe"), b"x").is_err());
}

/// The projected layout is read through `..data` alone: a top-level alias
/// link that names another directory is never followed, and an update that
/// moves `..data` to new bytes makes the pinned version missing until it is
/// pinned again.
#[test]
fn projected_layout_reads_through_data() {
    if !in_child() {
        let jail = jail();
        let source = jail.path(SOURCE);
        projected(&source, &[("tavily", "tavily.api-key", SECRET, 0o400)]);
        // The top-level link names a decoy: reading it would give other bytes.
        fs::remove_file(source.join("tavily")).unwrap();
        write(
            &source.join("..decoy/tavily/tavily.api-key"),
            b"decoy",
            0o400,
        );
        symlink("..decoy/tavily", source.join("tavily")).unwrap();
        return run_in(
            &jail,
            "mounted::tests::projected_layout_reads_through_data",
            Map::Own,
        );
    }
    assert_eq!(inspect(Path::new(CUSTODY)), State::Available);
    assert!(available(Path::new(CUSTODY)));
    let binding = open();
    assert_eq!(binding.state(), State::Available);
    assert_eq!(binding.kind(), crate::CustodyBackend::MountedSecret);
    let item = item("tavily", "tavily.api-key");
    let reading = binding.read_item(&item).unwrap();
    assert!(reading.material().ct_eq(SECRET), "read through ..data");
    let version = Version::new(binding.scope(), Uuid::new_v4()).unwrap();
    binding.pin(version, &item, &reading, || Ok(())).unwrap();
    assert!(binding.read(version).unwrap().ct_eq(SECRET));
    // The same record again is accepted; another one for the version is not.
    binding.pin(version, &item, &reading, || Ok(())).unwrap();
    let other = MountedSecret::open(binding.scope(), Path::new(CUSTODY), Path::new(PINS)).unwrap();
    let decoy_item = Item::new("tavily", "tavily.api-key").unwrap();
    assert!(other.read(version).unwrap().ct_eq(SECRET), "another handle");
    // A guard's refusal records nothing.
    let refused = Version::new(binding.scope(), Uuid::new_v4()).unwrap();
    assert_eq!(
        binding.pin(refused, &decoy_item, &reading, || Err(Failure::Conflict)),
        Err(Failure::Conflict)
    );
    assert_eq!(binding.read(refused).unwrap_err(), Failure::Missing);
    // A kubelet update: a new timestamped directory, then `..data` renamed onto it.
    let source = Path::new(SOURCE);
    let rotated = "..2026_10_14_00_00_00.000000002";
    write(
        &source.join(rotated).join("tavily/tavily.api-key"),
        b"fictional-rotated-credential",
        0o400,
    );
    symlink(rotated, source.join("..data_tmp")).unwrap();
    fs::rename(source.join("..data_tmp"), source.join("..data")).unwrap();
    let failure = binding.read(version).unwrap_err();
    assert_eq!(failure, Failure::Missing, "changed bytes are never served");
    redacted(
        failure,
        &binding,
        &[SECRET, b"fictional-rotated-credential"],
    );
    let repinned = binding.read_item(&item).unwrap();
    assert!(repinned.material().ct_eq(b"fictional-rotated-credential"));
    let next = Version::new(binding.scope(), Uuid::new_v4()).unwrap();
    binding.pin(next, &item, &repinned, || Ok(())).unwrap();
    assert!(
        binding
            .read(next)
            .unwrap()
            .ct_eq(b"fictional-rotated-credential")
    );
    // Release forgets the record and leaves the mounted item in place.
    binding.release(version).unwrap();
    binding.release(version).unwrap();
    assert_eq!(binding.read(version).unwrap_err(), Failure::Missing);
    assert!(
        Path::new(CUSTODY)
            .join(rotated)
            .join("tavily/tavily.api-key")
            .exists()
    );
    // The pin directory and key are owner-only.
    let pins = fs::metadata(PINS).unwrap();
    assert_eq!(pins.permissions().mode() & 0o777, 0o700);
    let key = fs::metadata(Path::new(PINS).join("pin.key")).unwrap();
    assert_eq!(key.permissions().mode() & 0o777, 0o600);
}

/// One case per check, each with everything else valid. Every refusal is
/// `Unavailable`, and no failure text carries the bytes or a path.
#[test]
fn refuses_each_check() {
    if !in_child() {
        let mut jail = jail();
        let source = jail.path(SOURCE);
        let marker: &[u8] = SECRET;
        projected(
            &source,
            &[
                ("ok", "profile", marker, 0o400),
                ("write-bit", "profile", marker, 0o600),
                ("other-read", "profile", marker, 0o404),
                ("group-read-own", "profile", marker, 0o440),
            ],
        );
        let stamp = source.join(STAMP);
        // A symbolic link in place of the item.
        write(&stamp.join("symlink/target"), marker, 0o400);
        symlink("target", stamp.join("symlink/profile")).unwrap();
        // A second link to the item.
        write(&stamp.join("hard-link/profile"), marker, 0o400);
        fs::hard_link(
            stamp.join("hard-link/profile"),
            stamp.join("hard-link/second"),
        )
        .unwrap();
        // A FIFO in place of the item.
        fs::create_dir_all(stamp.join("fifo")).unwrap();
        let fifo = std::ffi::CString::new(stamp.join("fifo/profile").to_str().unwrap().as_bytes())
            .unwrap();
        // SAFETY: fifo is a NUL-terminated path.
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o400) }, 0);
        // One byte over the bound.
        write(
            &stamp.join("oversized/profile"),
            &vec![b'x'; MAX_BYTES + 1],
            0o400,
        );
        // An alias directory others can write.
        write(&stamp.join("open-alias/profile"), marker, 0o400);
        fs::set_permissions(stamp.join("open-alias"), fs::Permissions::from_mode(0o777)).unwrap();
        // An item on a writable mount below the read-only one.
        jail.directory("/srv/writable", 0o755);
        write(&jail.path("/srv/writable/profile"), marker, 0o400);
        fs::create_dir(stamp.join("writable")).unwrap();
        jail.read_write("/srv/writable", &format!("{CUSTODY}/{STAMP}/writable"));
        // `..data` variants, each in its own configured directory.
        for (name, target) in [
            ("data-root", "/"),
            ("data-parent", "../x"),
            ("data-plain", "plain"),
            ("data-dotdot", ".."),
        ] {
            let directory = jail.directory(&format!("/srv/{name}"), 0o755);
            write(&directory.join("plain/ok/profile"), marker, 0o400);
            write(&directory.join("..x/ok/profile"), marker, 0o400);
            symlink(target, directory.join("..data")).unwrap();
            jail.read_only(&format!("/srv/{name}"), &format!("/srv/{name}"));
        }
        // The configured directory on a writable mount: the source itself.
        return run_in(&jail, "mounted::tests::refuses_each_check", Map::Own);
    }
    let binding = open();
    assert!(
        binding
            .read_item(&item("ok", "profile"))
            .unwrap()
            .material()
            .ct_eq(SECRET)
    );
    for alias in [
        "write-bit",
        "other-read",
        "group-read-own",
        "symlink",
        "hard-link",
        "fifo",
        "oversized",
        "open-alias",
        "writable",
    ] {
        let failure = binding.read_item(&item(alias, "profile")).unwrap_err();
        assert_eq!(failure, Failure::Unavailable, "{alias}");
        redacted(failure, &binding, &[SECRET]);
    }
    for name in ["data-root", "data-parent", "data-plain", "data-dotdot"] {
        let directory = PathBuf::from(format!("/srv/{name}"));
        let binding = MountedSecret::open(scope(), &directory, Path::new(PINS)).unwrap();
        let failure = binding.read_item(&item("ok", "profile")).unwrap_err();
        assert_eq!(failure, Failure::Unavailable, "{name}");
        redacted(failure, &binding, &[SECRET]);
    }
    // A writable configured directory is not a mounted secret: nothing opens.
    assert_eq!(inspect(Path::new(SOURCE)), State::Unavailable);
    assert_eq!(
        MountedSecret::open(scope(), Path::new(SOURCE), Path::new(PINS)).unwrap_err(),
        Failure::Unavailable
    );
    // An absent directory is unavailable, an absent item missing.
    assert_eq!(inspect(Path::new("/srv/absent")), State::Unavailable);
    assert_eq!(
        binding.read_item(&item("absent", "profile")).unwrap_err(),
        Failure::Missing
    );
}

/// Group read is admitted only on a root-owned file whose group the process
/// holds (decision-blocker:mounted-secret-custody-trust, Q1): a Kubernetes
/// Secret volume with `fsGroup` presents `defaultMode: 0400` as `0440
/// root:<fsGroup>`. Inside this jail everything appears root-owned, group 0.
#[test]
fn group_read_needs_root_and_a_held_group() {
    if !in_child() {
        let jail = jail();
        projected(
            &jail.path(SOURCE),
            &[
                ("tavily", "tavily.api-key", SECRET, 0o440),
                ("owner", "profile", SECRET, 0o400),
            ],
        );
        return run_in(
            &jail,
            "mounted::tests::group_read_needs_root_and_a_held_group",
            Map::Root,
        );
    }
    let open_as = |identity| {
        MountedSecret::open_as(scope(), Path::new(CUSTODY), Path::new(PINS), identity).unwrap()
    };
    // A non-root user holding the file's group as a supplementary group.
    let member = open_as(Identity::of(10001, 10001, &[0]));
    let reading = member.read_item(&item("tavily", "tavily.api-key")).unwrap();
    assert!(reading.material().ct_eq(SECRET));
    // The same user as the group's primary member.
    let primary = open_as(Identity::of(10001, 0, &[]));
    assert!(primary.read_item(&item("tavily", "tavily.api-key")).is_ok());
    // Not holding the group: refused.
    let stranger = open_as(Identity::of(10001, 10001, &[10002]));
    let failure = stranger
        .read_item(&item("tavily", "tavily.api-key"))
        .unwrap_err();
    assert_eq!(
        failure,
        Failure::Unavailable,
        "a group the process does not hold"
    );
    redacted(failure, &stranger, &[SECRET]);
    // An owner-only root file is root's; the owner rule admits root.
    assert!(member.read_item(&item("owner", "profile")).is_ok());
}

/// The item rule on its own, for the owner a jail cannot lay out: a file
/// owned by neither root nor the effective user.
#[test]
fn an_item_owned_by_another_user_is_refused() {
    let identity = Identity::of(1000, 1000, &[]);
    let file = super::ItemStat {
        regular: true,
        links: 1,
        size: 16,
        mode: 0o400,
        uid: 1001,
        gid: 1000,
    };
    assert_eq!(super::admits(&file, &identity), Err(Failure::Unavailable));
    assert_eq!(
        super::admits(&super::ItemStat { uid: 1000, ..file }, &identity),
        Ok(())
    );
    assert_eq!(
        super::admits(&super::ItemStat { uid: 0, ..file }, &identity),
        Ok(())
    );
    // Group read on a file the user owns, not root: refused (Q1).
    assert_eq!(
        super::admits(
            &super::ItemStat {
                uid: 1000,
                mode: 0o440,
                ..file
            },
            &identity
        ),
        Err(Failure::Unavailable)
    );
}

/// Item coordinates are selectors, and no `Debug` shows a coordinate, a pin
/// or bytes.
#[test]
fn items_are_selectors_and_redacted() {
    assert!(Item::new("tavily", "tavily.api-key").is_ok());
    for (alias, profile) in [
        ("", "p"),
        (".", "p"),
        ("..", "p"),
        ("a/b", "p"),
        ("a", "p q"),
        ("a", &"p".repeat(129)),
    ] {
        assert_eq!(
            Item::new(alias, profile),
            Err(Failure::Denied),
            "{alias:?} {profile:?}"
        );
    }
    assert_eq!(
        format!("{:?}", item("tavily", "tavily.api-key")),
        "Item(redacted)"
    );
    assert_eq!(format!("{:?}", super::Pin([7; 32])), "Pin(redacted)");
    let material = Material::from_slice(SECRET);
    assert_eq!(format!("{material:?}"), "Material(redacted)");
}

/// Security review, story:mounted-secret-custody: the checks
/// `refuses_each_check` does not reach, each with everything else valid.
/// Check 1, a group- or other-writable directory above the configured one;
/// check 3, a world-writable `..data` sibling and a `..data` naming a second
/// link; and Q3, a pin key or pin directory that is not owner-only.
#[test]
fn refuses_the_unmeasured_checks() {
    if !in_child() {
        let mut jail = jail();
        projected(&jail.path(SOURCE), &[("ok", "profile", SECRET, 0o400)]);
        // Check 1: the configured directory below an open directory.
        jail.directory("/srv/open", 0o777);
        let below = jail.directory("/srv/open/custody", 0o755);
        projected(&below, &[("ok", "profile", SECRET, 0o400)]);
        jail.read_only("/srv/open/custody", "/srv/open/custody");
        // Check 3: the `..data` sibling writable by others.
        let wide = jail.directory("/srv/wide", 0o755);
        projected(&wide, &[("ok", "profile", SECRET, 0o400)]);
        fs::set_permissions(wide.join(STAMP), fs::Permissions::from_mode(0o777)).unwrap();
        jail.read_only("/srv/wide", "/srv/wide");
        // Check 3: `..data` names a second link, which names the sibling.
        let chained = jail.directory("/srv/chained", 0o755);
        write(&chained.join(STAMP).join("ok/profile"), SECRET, 0o400);
        symlink(STAMP, chained.join("..link")).unwrap();
        symlink("..link", chained.join("..data")).unwrap();
        jail.read_only("/srv/chained", "/srv/chained");
        return run_in(
            &jail,
            "mounted::tests::refuses_the_unmeasured_checks",
            Map::Own,
        );
    }
    // Check 1.
    assert_eq!(inspect(Path::new("/srv/open/custody")), State::Unavailable);
    assert_eq!(
        MountedSecret::open(scope(), Path::new("/srv/open/custody"), Path::new(PINS)).unwrap_err(),
        Failure::Unavailable,
        "an open ancestor"
    );
    // Check 3.
    for name in ["/srv/wide", "/srv/chained"] {
        let binding = MountedSecret::open(scope(), Path::new(name), Path::new(PINS)).unwrap();
        let failure = binding.read_item(&item("ok", "profile")).unwrap_err();
        assert_eq!(failure, Failure::Unavailable, "{name}");
        redacted(failure, &binding, &[SECRET]);
    }
    // Q3: the pin key is owner-only, and so is its directory.
    let binding = open();
    assert!(binding.read_item(&item("ok", "profile")).is_ok());
    let key = Path::new(PINS).join("pin.key");
    fs::set_permissions(&key, fs::Permissions::from_mode(0o640)).unwrap();
    assert_eq!(
        binding.read_item(&item("ok", "profile")).unwrap_err(),
        Failure::Unavailable,
        "a pin key others can read"
    );
    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(PINS, fs::Permissions::from_mode(0o750)).unwrap();
    assert_eq!(
        MountedSecret::open(scope(), Path::new(CUSTODY), Path::new(PINS)).unwrap_err(),
        Failure::Unavailable,
        "a pin directory others can enter"
    );
}

/// Check 5 runs on a descriptor that cannot read (`O_PATH`), and the item is
/// opened for reading only once it is admitted: a refused item is never
/// opened at all, so a device node or FIFO put in an item's place is never
/// opened for read. Opens are observed with inotify, which reports
/// `IN_OPEN` for every open that can read and none for `O_PATH`.
#[test]
fn an_item_is_opened_for_reading_only_after_its_checks() {
    if !in_child() {
        let jail = jail();
        projected(
            &jail.path(SOURCE),
            &[
                ("ok", "profile", SECRET, 0o400),
                ("wide", "profile", SECRET, 0o644),
            ],
        );
        return run_in(
            &jail,
            "mounted::tests::an_item_is_opened_for_reading_only_after_its_checks",
            Map::Own,
        );
    }
    // SAFETY: plain system calls on owned descriptors and NUL-terminated paths.
    let (inotify, ok, wide) = unsafe {
        let inotify = libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC);
        assert!(inotify >= 0);
        let watch = |path: &std::ffi::CStr| {
            let wd = libc::inotify_add_watch(inotify, path.as_ptr(), libc::IN_OPEN);
            assert!(wd >= 0, "watch {path:?}");
            wd
        };
        (
            inotify,
            watch(c"/srv/custody/..data/ok/profile"),
            watch(c"/srv/custody/..data/wide/profile"),
        )
    };
    let binding = open();
    assert_eq!(
        binding.read_item(&item("wide", "profile")).unwrap_err(),
        Failure::Unavailable
    );
    assert_eq!(
        binding
            .read_item(&item("ok", "profile"))
            .unwrap()
            .material()
            .as_bytes(),
        SECRET
    );
    let mut opens = std::collections::BTreeMap::<i32, usize>::new();
    let mut buffer = [0u8; 4096];
    loop {
        // SAFETY: buffer is writable for its length; inotify is owned.
        let count = unsafe { libc::read(inotify, buffer.as_mut_ptr().cast(), buffer.len()) };
        if count <= 0 {
            break;
        }
        let mut offset = 0;
        while offset < count as usize {
            // SAFETY: the kernel wrote whole events; read_unaligned copies one header.
            let event = unsafe {
                std::ptr::read_unaligned(buffer[offset..].as_ptr().cast::<libc::inotify_event>())
            };
            if event.mask & libc::IN_OPEN != 0 {
                *opens.entry(event.wd).or_default() += 1;
            }
            offset += std::mem::size_of::<libc::inotify_event>() + event.len as usize;
        }
    }
    // SAFETY: inotify is owned and closed once.
    unsafe { libc::close(inotify) };
    assert_eq!(opens.get(&wide), None, "the refused item was opened");
    assert_eq!(
        opens.get(&ok),
        Some(&1),
        "the admitted item, opened once to read"
    );
}

/// A file `program` with `mode` in a directory of its own in the temporary
/// directory, where a jail's root is made too.
fn program(mode: u32, bytes: &[u8]) -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("program");
    write(&path, bytes, mode);
    (directory, path)
}

/// What `path` names itself, a symbolic link not followed.
fn named(path: &Path) -> fs::Metadata {
    fs::symlink_metadata(path).unwrap()
}

fn inode(stat: &fs::Metadata) -> (u64, u64) {
    (stat.dev(), stat.ino())
}

/// `installed` is a file of its own with the bytes of `source`, owner-only.
fn copied(installed: &fs::Metadata, source: &fs::Metadata) {
    assert!(installed.is_file(), "a file, not a link to one");
    assert_ne!(inode(installed), inode(source), "another inode");
    assert_eq!(installed.len(), source.len(), "the source's length");
    assert_eq!(installed.mode() & 0o7777, 0o700, "owner-only");
}

/// A source only its owner can write, on the jail's file system, is linked:
/// one file under two names, and the source keeps the mode it had.
#[test]
fn install_links_a_source_on_the_jails_file_system() {
    let (_directory, source) = program(0o755, b"#!/bin/sh\n");
    let jail = Jail::new();
    let installed = jail.install(&source, "/bin-test/program");
    assert_eq!(installed, jail.path("/bin-test/program"));
    assert_eq!(
        inode(&named(&installed)),
        inode(&named(&source)),
        "one inode"
    );
    assert_eq!(named(&source).mode() & 0o7777, 0o755, "the source's mode");
}

/// The pair of places every caller has: a binary cargo built under `target/`
/// and a jail root in the temporary directory. The test asks the host first,
/// with the link call itself: equal device numbers do not say it, because
/// two mounts of one device refuse a link between them. Linked where that
/// call works and cargo gave the binary a mode a link may carry (no group or
/// other write bit, the owner's execute bit, no set-id or sticky bit); a copy
/// where the call crosses devices or the mode is another.
#[test]
fn install_links_the_running_test_binary_where_the_link_call_works() {
    let source = std::env::current_exe().unwrap();
    let jail = Jail::new();
    let probe = jail.path("/probe");
    let links = match fs::hard_link(&source, &probe) {
        Ok(()) => {
            fs::remove_file(&probe).unwrap();
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::CrossesDevices => false,
        Err(error) => panic!("the probe link into the jail's directory: {error}"),
    };
    let installed = jail.install(&source, "/bin-test/custody");
    let mode = named(&source).mode();
    let as_it_is = mode & 0o7022 == 0 && mode & 0o100 != 0;
    if links && as_it_is {
        assert_eq!(
            inode(&named(&installed)),
            inode(&named(&source)),
            "one inode"
        );
    } else {
        copied(&named(&installed), &named(&source));
        assert!(
            fs::read(&installed).unwrap() == fs::read(&source).unwrap(),
            "the source's bytes"
        );
    }
}

/// A link is the source itself, so a source that group or others can write
/// would be an executable they can write: it is copied, and keeps its mode.
#[test]
fn install_copies_a_source_with_a_group_or_other_write_bit() {
    for mode in [0o775, 0o757] {
        let (_directory, source) = program(mode, b"#!/bin/sh\n");
        let jail = Jail::new();
        let installed = jail.install(&source, "/bin-test/program");
        copied(&named(&installed), &named(&source));
        assert_eq!(named(&source).mode() & 0o7777, mode, "the source's mode");
    }
}

/// `/proc/self/exe` is a name on another file system than a jail's root:
/// the link call does not follow it and is refused for crossing devices, on
/// every host. The file it names is copied.
#[test]
fn install_copies_a_source_on_another_file_system() {
    let source = Path::new("/proc/self/exe");
    let jail = Jail::new();
    let refused = fs::hard_link(source, jail.path("/probe")).unwrap_err();
    assert_eq!(
        refused.raw_os_error(),
        Some(libc::EXDEV),
        "the link call crosses devices"
    );
    let installed = jail.install(source, "/bin-test/custody");
    copied(&named(&installed), &fs::metadata(source).unwrap());
}

/// The link call does not follow a symbolic link: on the jail's file system
/// it would put a second name of the link into the jail, where what the link
/// names does not exist. What it names is copied.
#[test]
fn install_copies_what_a_symbolic_link_names() {
    let (directory, target) = program(0o755, b"#!/bin/sh\n");
    let link = directory.path().join("link");
    symlink(&target, &link).unwrap();
    let jail = Jail::new();
    let installed = jail.install(&link, "/bin-test/program");
    copied(&named(&installed), &named(&target));
    assert_eq!(fs::read(installed).unwrap(), b"#!/bin/sh\n");
}

/// An install at a place that holds an earlier link replaces the name and
/// never writes through it: the earlier source keeps its bytes.
#[test]
fn install_at_the_place_of_an_earlier_link_leaves_its_source_alone() {
    let (_first, first) = program(0o755, b"first");
    let (_second, second) = program(0o775, b"second, a copy");
    let jail = Jail::new();
    let installed = jail.install(&first, "/bin-test/program");
    assert_eq!(inode(&named(&installed)), inode(&named(&first)), "linked");
    // The same source again: a copy onto the link would empty it first.
    jail.install(&first, "/bin-test/program");
    assert_eq!(fs::read(&first).unwrap(), b"first");
    // Another source, one that is copied: the copy would land in the first.
    jail.install(&second, "/bin-test/program");
    assert_eq!(fs::read(&first).unwrap(), b"first");
    assert_eq!(fs::read(&installed).unwrap(), b"second, a copy");
    copied(&named(&installed), &named(&second));
}

/// `install` puts an executable into the jail whatever mode its source has:
/// a link would keep a source without the owner's execute bit as it is, and
/// nothing could run it. It is copied, and the jail runs the copy.
#[test]
fn install_copies_a_source_without_an_execute_bit() {
    let (_directory, source) = program(0o644, b"#!/bin/sh\nexit 0\n");
    let jail = Jail::new();
    let installed = jail.install(&source, "/bin-test/program");
    copied(&named(&installed), &named(&source));
    assert_eq!(named(&source).mode() & 0o7777, 0o644, "the source's mode");
    let ran = Jail::output(
        jail.command("/bin-test/program", Map::Own)
            .env_clear()
            .stdin(Stdio::null()),
    );
    assert!(ran.status.success(), "the copy runs in the jail: {ran:?}");
}

/// A link would carry a set-user-id, set-group-id or sticky bit of its source
/// into the jail; the copy never had one. A host may drop a bit it does not
/// grant this user (set-group-id, for a group the user is not in): the bits
/// it kept decide, and it keeps the set-user-id bit of the user's own file.
#[test]
fn install_copies_a_source_with_a_set_id_or_sticky_bit() {
    for mode in [0o4755, 0o2755, 0o1755] {
        let (_directory, source) = program(mode, b"#!/bin/sh\n");
        let kept = named(&source).mode() & 0o7000;
        assert!(mode != 0o4755 || kept == 0o4000, "the set-user-id bit");
        let jail = Jail::new();
        let installed = named(&jail.install(&source, "/bin-test/program"));
        assert_eq!(installed.mode() & 0o7000, 0, "of a source with {mode:o}");
        if kept != 0 {
            copied(&installed, &named(&source));
        }
    }
}

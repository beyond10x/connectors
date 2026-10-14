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
    os::unix::fs::{PermissionsExt, symlink},
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
    let output = jail
        .command("/bin-test/custody", map)
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env_clear()
        .env(CHILD, "1")
        .env("TMPDIR", "/work")
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|error| panic!("{}", Jail::spawn_error(&error)));
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

//! The owner transport on the Windows runner alone: the pipe's name, who
//! reaches it, its retirement, the two lock bytes and the owner's command line.
//!
//! A pipe served by a process of another user cannot be created here: the
//! runner has one user and no second token to create it with. What refuses it
//! is the client's check of the server's token, which
//! `channel::windows::tests::a_peer_whose_token_is_another_sid_is_refused`
//! drives against another SID, and [`connect`] goes through that same check.
use super::*;
use windows_sys::Win32::{
    Foundation::GENERIC_READ, Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS,
    System::Threading::GetCurrentProcessId,
};

fn directory(path: &Path) -> File {
    OpenOptions::new()
        .access_mode(GENERIC_READ)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .unwrap()
}

fn own_pid() -> u32 {
    // SAFETY: no preconditions.
    unsafe { GetCurrentProcessId() }
}

/// A fresh pipe name of the owner's form, unique to the test.
fn fresh() -> PathBuf {
    PathBuf::from(format!(
        r"\\.\pipe\connectors-owner-test-{}",
        uuid::Uuid::new_v4()
    ))
}

/// Accepts the next client within two seconds.
fn accepted(listener: &mut Listener) -> Stream {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        match listener.accept() {
            Ok((stream, ())) => return stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < until, "no client arrived");
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("accept failed: {error}"),
        }
    }
}

/// The name is a function of the directory alone: the same directory gives
/// the same pipe, another directory another pipe, and the name is the owner
/// prefix and a SHA-256 in hexadecimal.
#[test]
fn the_pipe_name_derives_from_the_state_directory() {
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let first = socket_path(&directory(a.path()));
    assert_eq!(first, socket_path(&directory(a.path())));
    assert_ne!(first, socket_path(&directory(b.path())));
    let name = first.to_str().unwrap();
    let digest = name
        .strip_prefix(r"\\.\pipe\connectors-owner-")
        .expect("the owner prefix");
    assert_eq!(digest.len(), 64);
    assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
}

/// No connect or listen admits the empty name an unreadable directory gives.
#[test]
fn an_empty_name_is_no_owner() {
    assert_eq!(
        connect(Path::new("")).err().map(|error| error.code),
        Some(Code::Unavailable)
    );
    assert!(identity(Path::new("")).is_none());
    assert!(Listener::bind(Path::new("")).is_err());
}

/// An owner never serves a pipe whose name already exists: its own second
/// bind of the name refuses, as one by any other process would.
#[test]
fn an_owner_never_joins_an_existing_pipe_as_its_first_instance() {
    let path = fresh();
    let _first = Listener::bind(&path).unwrap();
    assert!(Listener::bind(&path).is_err());
}

/// A client reaches the owner, which this process is; its identity is the
/// server's process id, and the accepted stream's peer is the client.
#[test]
fn a_client_of_the_current_user_reaches_the_owner() {
    let path = fresh();
    let mut listener = Listener::bind(&path).unwrap();
    let (mut client, server) = connect(&path).unwrap();
    assert_eq!(server, own_pid());
    let mut stream = accepted(&mut listener);
    assert_eq!(channel::peer(&stream).unwrap().pid, own_pid());
    let until = Instant::now() + Duration::from_secs(2);
    channel::write(
        &mut client,
        &serde_json::json!({"kind":"ping"}),
        None,
        &[],
        until,
    )
    .unwrap();
    let frame = channel::read::<serde_json::Value>(&mut stream, until, false, 0).unwrap();
    assert_eq!(frame.control["kind"], "ping");
    assert_eq!(identity(&path), Some(own_pid()));
}

/// Several clients at once each find an instance: the next instance waits
/// before the connected one is handed over.
#[test]
fn every_accept_leaves_an_instance_waiting() {
    let path = fresh();
    let mut listener = Listener::bind(&path).unwrap();
    let mut held = Vec::new();
    for _ in 0..4 {
        held.push(connect(&path).unwrap());
        held.push((accepted(&mut listener), 0));
    }
}

/// Retirement: once the listener is closed no connect reaches the owner, a
/// client that connected before the close is still served, and nothing is
/// left waiting.
#[test]
fn a_closed_listener_is_unreachable_and_serves_what_reached_it_first() {
    let path = fresh();
    let mut listener = Listener::bind(&path).unwrap();
    let (_early, _) = connect(&path).unwrap();
    listener.close().unwrap();
    let _served = accepted(&mut listener);
    assert!(connect(&path).is_err(), "a connect after the close");
    assert_eq!(
        listener.accept().err().map(|error| error.kind()),
        Some(std::io::ErrorKind::WouldBlock)
    );
}

/// Listening again after retirement: with nothing served the pipe is gone and
/// is created anew as its first instance; with a connection still served the
/// pipe exists and a new instance joins it. Either way a client reaches it.
#[test]
fn a_retired_listener_listens_again() {
    let path = fresh();
    let mut listener = Listener::bind(&path).unwrap();
    listener.close().unwrap();
    drop(listener);
    let mut listener = Listener::rebind(&path, false).unwrap();
    let (_client, _) = connect(&path).unwrap();
    let served = accepted(&mut listener);
    listener.close().unwrap();
    assert!(
        Listener::rebind(&path, false).is_err(),
        "joined without served"
    );
    let mut listener = Listener::rebind(&path, true).unwrap();
    let (_later, _) = connect(&path).unwrap();
    accepted(&mut listener);
    drop(served);
}

fn lock_file() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("owner.lock");
    std::fs::write(&path, b"").unwrap();
    (root, path)
}

fn open(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .open(path)
        .unwrap()
}

/// `try_lock` is both bytes or neither: a second open finds them held, and a
/// refusal leaves neither byte held by the refused caller.
#[test]
fn the_lock_is_both_bytes_or_neither() {
    let (_root, path) = lock_file();
    let (cli, other) = (open(&path), open(&path));
    try_lock(&cli).unwrap();
    assert_eq!(
        try_lock(&other).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(cli);
    let owner = open(&path);
    lock_byte(&owner, LIFETIME).unwrap();
    assert_eq!(
        try_lock(&other).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    // The refused caller released the startup byte it had taken.
    lock_byte(&open(&path), STARTUP).unwrap();
}

/// The hand-over at an owner's start: the starting CLI holds both bytes,
/// releases the lifetime byte once the owner exists and keeps the startup
/// byte, so the owner takes the lifetime byte and no other CLI takes either.
/// An owner whose lifetime byte stays held refuses at its deadline.
#[test]
fn the_lifetime_byte_passes_from_the_starting_cli_to_the_owner() {
    let (_root, path) = lock_file();
    let (cli, owner, other) = (open(&path), open(&path), open(&path));
    try_lock(&cli).unwrap();
    let started = Instant::now();
    assert!(hold_lifetime(&owner, started + Duration::from_millis(100)).is_err());
    assert!(started.elapsed() >= Duration::from_millis(100));
    unlock_byte(&cli, LIFETIME);
    hold_lifetime(&owner, Instant::now() + Duration::from_secs(1)).unwrap();
    assert_eq!(
        try_lock(&other).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(cli);
    assert_eq!(
        try_lock(&other).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock,
        "the owner's lifetime byte"
    );
    drop(owner);
    try_lock(&other).unwrap();
}

fn quoted(arguments: &[&str]) -> String {
    let mut line = Vec::new();
    for argument in arguments {
        quote(OsStr::new(argument), &mut line);
    }
    String::from_utf16(&line).unwrap()
}

/// The command line parses back to the same arguments under the Microsoft C
/// runtime's rules.
#[test]
fn arguments_are_quoted_as_the_runtime_parses_them() {
    assert_eq!(quoted(&["a", "b"]), "a b");
    assert_eq!(quoted(&[""]), "\"\"");
    assert_eq!(
        quoted(&[r"C:\Program Files\c.exe"]),
        r#""C:\Program Files\c.exe""#
    );
    assert_eq!(quoted(&[r#"a"b"#]), r#""a\"b""#);
    assert_eq!(quoted(&[r#"a\"b"#]), r#""a\\\"b""#);
    assert_eq!(quoted(&[r"a b\"]), r#""a b\\""#);
    assert_eq!(quoted(&[r"a\b"]), r"a\b");
}

/// An owner started by hand has no startup handles: it never mistakes another
/// handle for inherited authority.
#[test]
fn an_owner_without_its_startup_handles_refuses() {
    assert!(std::env::var_os(STARTUP_HANDLES).is_none());
    assert_eq!(
        inherited().err().map(|error| error.code),
        Some(Code::InvalidConfiguration)
    );
}

//! Real local admission waits; every deliberately stalled peer has a finite hold.
use super::*;
use std::io::Read;

fn setup() -> (tempfile::TempDir, Paths, File) {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths::resolve(
        Some(&root.path().join("config/config.toml")),
        Some(&root.path().join("state")),
    )
    .unwrap();
    Config::initialize(&paths).unwrap();
    drop(Metadata::inspect(&paths.state).unwrap());
    own_build().unwrap();
    let directory = fs::directory(&paths.state, false, true).unwrap();
    (root, paths, directory)
}

fn listen_at(directory: &File) -> (PathBuf, UnixListener) {
    let socket = socket_path(directory);
    let listener = listen(&socket).unwrap();
    (socket, listener)
}

fn accept_before(listener: &UnixListener) -> UnixStream {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        match listener.accept() {
            Ok((stream, _)) => return stream,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < until =>
            {
                std::thread::sleep(Duration::from_millis(5))
            }
            Err(error) => panic!("fixture admission did not arrive: {:?}", error.kind()),
        }
    }
}

#[test]
fn owner_admission_expired_opens_no_state() {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths {
        config: root.path().join("absent-config"),
        state: root.path().join("absent-state"),
    };
    let error = Client::connect_until(&paths, true, Instant::now())
        .map(|_| ())
        .unwrap_err();
    assert_eq!(error.code, Code::Timeout);
    assert!(!paths.state.exists());
    assert!(!paths.config.exists());
}

#[test]
fn owner_admission_stalled_greeting_keeps_original_deadline() {
    let (_root, paths, directory) = setup();
    let (_, listener) = listen_at(&directory);
    let peer = std::thread::spawn(move || {
        let mut stream = accept_before(&listener);
        let frame = channel::read::<Request>(
            &mut stream,
            Instant::now() + Duration::from_secs(2),
            false,
            0,
        )
        .unwrap();
        assert!(matches!(frame.control, Request::Hello { .. }));
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        assert_eq!(
            stream.read(&mut [0]).unwrap(),
            0,
            "expired caller retained its socket"
        );
    });
    let started = Instant::now();
    let error = Client::connect_until(&paths, false, started + Duration::from_millis(200))
        .map(|_| ())
        .unwrap_err();
    assert_eq!(error.code, Code::Timeout);
    assert!(started.elapsed() < Duration::from_millis(900));
    peer.join().unwrap();
}

#[test]
fn owner_admission_full_accept_queue_refuses_without_starting_an_owner() {
    let (_root, paths, directory) = setup();
    let (socket, listener) = listen_at(&directory);
    // SAFETY: change only this fixture's owned listening socket backlog.
    assert_eq!(unsafe { libc::listen(listener.as_raw_fd(), 0) }, 0);
    let _queued = UnixStream::connect(&socket).unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(600));
        drop(accept_before(&listener));
        drop(listener);
    });
    let started = Instant::now();
    let error = Client::connect_until(&paths, true, started + Duration::from_millis(200))
        .map(|_| ())
        .unwrap_err();
    assert_eq!(error.code, Code::Capacity);
    assert!(started.elapsed() < Duration::from_millis(400));
    assert!(
        !paths.state.join("owner.lock").exists(),
        "a full queue authorized owner startup"
    );
    release.join().unwrap();
}

#[test]
fn owner_admission_lifetime_and_metadata_locks_keep_original_deadline() {
    for metadata in [false, true] {
        let (_root, paths, directory) = setup();
        let held = if metadata {
            fs::private_file_at(&directory, std::ffi::OsStr::new("metadata.lock")).unwrap()
        } else {
            lock(&directory).unwrap()
        };
        // SAFETY: this fixture owns held for the finite hold below.
        assert_eq!(unsafe { libc::flock(held.as_raw_fd(), libc::LOCK_EX) }, 0);
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(600));
            drop(held);
        });
        let started = Instant::now();
        let error = Client::connect_until(&paths, true, started + Duration::from_millis(100))
            .map(|_| ())
            .unwrap_err();
        assert_eq!(error.code, Code::Timeout);
        assert!(started.elapsed() < Duration::from_millis(400));
        assert!(!paths.state.join("owner.sock").exists());
        release.join().unwrap();
    }
}

#[test]
fn owner_admission_requires_exact_build_before_returning_a_client() {
    for matching in [false, true] {
        let (_root, paths, directory) = setup();
        let (_, listener) = listen_at(&directory);
        let peer = std::thread::spawn(move || {
            let mut stream = accept_before(&listener);
            let until = Instant::now() + Duration::from_secs(2);
            let hello = channel::read::<Request>(&mut stream, until, false, 0).unwrap();
            let Request::Hello {
                version,
                challenge,
                authority,
                build,
                ..
            } = hello.control
            else {
                panic!("greeting required");
            };
            assert!(build.is_some());
            channel::write(
                &mut stream,
                &Reply::Hello {
                    version,
                    challenge,
                    authority,
                    host_incarnation: uuid::Uuid::new_v4().to_string(),
                    build: if matching {
                        build
                    } else {
                        Some("different-build".into())
                    },
                },
                None,
                &[],
                until,
            )
            .unwrap();
        });
        let result = Client::connect_until(&paths, false, Instant::now() + Duration::from_secs(2))
            .map(|_| ());
        if matching {
            result.unwrap();
        } else {
            assert_eq!(result.unwrap_err().code, Code::OwnerBuildMismatch);
        }
        peer.join().unwrap();
    }
}

/// A fixture owner that answers one greeting with `answer(authority greeted)`
/// as its authority and this build, and reports the authority it was greeted by.
fn answer_once(
    listener: UnixListener,
    answer: fn(&str) -> String,
) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut stream = accept_before(&listener);
        let until = Instant::now() + Duration::from_secs(2);
        let hello = channel::read::<Request>(&mut stream, until, false, 0).unwrap();
        let Request::Hello {
            version,
            challenge,
            authority,
            build,
            ..
        } = hello.control
        else {
            panic!("greeting required");
        };
        channel::write(
            &mut stream,
            &Reply::Hello {
                version,
                challenge,
                authority: answer(&authority),
                host_incarnation: uuid::Uuid::new_v4().to_string(),
                build,
            },
            None,
            &[],
            until,
        )
        .unwrap();
        authority
    })
}

/// story:mcp-long-read-exits-under-load: a renewal greets a running owner by
/// the authority it already knows and does not wait on the store, so a store lock
/// held for longer than the renewal's deadline does not delay it.
#[test]
fn owner_reconnect_greets_by_the_known_authority_while_the_store_lock_is_held() {
    let (_root, paths, directory) = setup();
    let held = fs::private_file_at(&directory, std::ffi::OsStr::new("metadata.lock")).unwrap();
    // SAFETY: this fixture owns held for the finite hold below.
    assert_eq!(unsafe { libc::flock(held.as_raw_fd(), libc::LOCK_EX) }, 0);
    let (_, listener) = listen_at(&directory);
    let peer = answer_once(listener, str::to_owned);
    let known = uuid::Uuid::new_v4().to_string();
    let started = Instant::now();
    let client = Client::reconnect_until(&paths, &known, started + Duration::from_millis(800))
        .unwrap_or_else(|error| {
            panic!(
                "reconnect with the store lock held failed after {:?}: {:?}",
                started.elapsed(),
                error.code
            )
        });
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(client.authority(), known);
    assert_eq!(
        peer.join().unwrap(),
        known,
        "the greeting named another authority"
    );
    drop(held);
}

/// The owner's greeting must confirm the authority the reconnect named: an
/// owner answering with another one is not the owner that admitted the session.
#[test]
fn owner_reconnect_refuses_an_owner_confirming_another_authority() {
    let (_root, paths, directory) = setup();
    let (_, listener) = listen_at(&directory);
    let peer = answer_once(listener, |_| uuid::Uuid::new_v4().to_string());
    let known = Metadata::authority_at(&paths.state).unwrap().to_string();
    let error = Client::reconnect_until(&paths, &known, Instant::now() + Duration::from_secs(2))
        .map(|_| ())
        .unwrap_err();
    assert_eq!(error.code, Code::ReadinessMismatch);
    peer.join().unwrap();
}

/// A reconnect starts no owner: with no socket, or a socket left by an owner
/// that died, it refuses as unavailable at once and takes no lifetime lock.
#[test]
fn owner_reconnect_with_no_owner_running_starts_none() {
    for stale in [false, true] {
        let (_root, paths, directory) = setup();
        if stale {
            let (socket, listener) = listen_at(&directory);
            drop(listener);
            assert!(socket.exists(), "the dead owner's socket file is gone");
        }
        let started = Instant::now();
        let error = Client::reconnect_until(
            &paths,
            &Metadata::authority_at(&paths.state).unwrap().to_string(),
            started + Duration::from_secs(2),
        )
        .map(|_| ())
        .unwrap_err();
        assert_eq!(error.code, Code::Unavailable, "stale socket: {stale}");
        assert!(started.elapsed() < Duration::from_millis(500));
        assert!(
            !paths.state.join("owner.lock").exists(),
            "a reconnect took the owner lifetime lock (stale socket: {stale})"
        );
    }
}

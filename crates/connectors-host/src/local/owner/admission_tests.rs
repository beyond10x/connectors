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

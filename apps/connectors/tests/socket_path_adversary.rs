//! Adversary probes for story:socket-path-limit-in-long-checkouts.
#[path = "support/socket.rs"]
mod sockets;

use std::{
    io::{Read, Write},
    os::unix::fs::{FileTypeExt, MetadataExt},
};

/// A state directory whose absolute socket path is far past `SUN_LEN` (108):
/// the helper still binds there, a peer connecting through a second,
/// independently opened descriptor reaches the same socket, and the socket
/// inode sits at the absolute path.
#[test]
fn a_socket_far_past_sun_len_is_bound_and_reached_through_the_directory() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("s".repeat(120)).join("state");
    std::fs::create_dir_all(&state).unwrap();
    let absolute = state.join("owner.sock");
    assert!(absolute.as_os_str().len() > 108);
    assert!(std::os::unix::net::UnixListener::bind(&absolute).is_err());

    let listener = sockets::bind(&state, "owner.sock");
    let metadata = std::fs::symlink_metadata(&absolute).unwrap();
    assert!(metadata.file_type().is_socket());

    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte).unwrap();
        stream.write_all(&[byte[0] + 1]).unwrap();
    });
    let directory = sockets::Directory::open(&state);
    let mut client = directory.connect("owner.sock");
    client.write_all(&[41]).unwrap();
    let mut answer = [0u8; 1];
    client.read_exact(&mut answer).unwrap();
    assert_eq!(answer, [42]);
    server.join().unwrap();
    assert_eq!(
        std::fs::symlink_metadata(&absolute).unwrap().ino(),
        metadata.ino()
    );
}

/// Many threads each open, bind and connect through their own descriptor at
/// once: no thread reaches another thread's socket through a reused fd number.
#[test]
fn concurrent_directories_never_cross_sockets() {
    let root = tempfile::tempdir().unwrap();
    let workers: Vec<_> = (0..32u8)
        .map(|n| {
            let state = root.path().join(format!("{n}-{}", "d".repeat(100)));
            std::fs::create_dir_all(&state).unwrap();
            std::thread::spawn(move || {
                for _ in 0..50 {
                    let listener = sockets::bind(&state, "owner.sock");
                    let mut client = sockets::connect(&state, "owner.sock");
                    let (mut accepted, _) = listener.accept().unwrap();
                    client.write_all(&[n]).unwrap();
                    let mut byte = [0u8; 1];
                    accepted.read_exact(&mut byte).unwrap();
                    assert_eq!(byte, [n], "worker {n} reached another worker's socket");
                    drop(listener);
                    std::fs::remove_file(state.join("owner.sock")).unwrap();
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
}

//! Handle passing and the credential on the Windows runner alone. A private
//! pair's peer is this process, so each handle is duplicated into this
//! process exactly as the owner duplicates it into the CLI.
use super::*;
use std::{
    io::{Read, Seek, SeekFrom},
    time::Duration,
};
use windows_sys::Win32::Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation};

fn until() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

fn ends() -> (channel::Stream, channel::Stream) {
    let (server, client) = channel::windows::pair().unwrap();
    (server, channel::Stream::from_owned(client).unwrap())
}

fn inheritable(file: &File) -> bool {
    channel::windows::inheritable(file.as_raw_handle() as HANDLE).unwrap()
}

fn fixture(bytes: &[u8]) -> File {
    let mut file = tempfile::tempfile().unwrap();
    file.write_all(bytes).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    file
}

/// consumer-launch.md §8, "Credential": the bytes are read from the start
/// through a handle that cannot write them, is not inheritable, and names no
/// file that still has a path.
#[test]
fn a_sealed_credential_is_read_only_at_its_start_and_has_no_name() {
    let material = b"fictional credential material";
    let mut credential = sealed(material).unwrap();
    assert!(!inheritable(&credential));
    assert!(
        final_path(&credential).map_or(true, |path| std::fs::symlink_metadata(path).is_err()),
        "no path names the credential"
    );
    assert!(credential.write_all(b"x").is_err(), "read-only");
    assert!(credential.set_len(0).is_err(), "cannot shrink");
    let mut read = Vec::new();
    credential.read_to_end(&mut read).unwrap();
    assert_eq!(read, material);
}

/// `atomic_close_on_exec` on Windows: a handle reaches the receiver by
/// `DuplicateHandle` with inheritance off, so it is not inheritable from the
/// moment it exists there, even when the sender's own handle is.
#[test]
fn handles_cross_the_pipe_as_new_handles_that_are_not_inheritable() {
    let (mut owner, mut cli) = ends();
    let (image, credential) = (fixture(b"image"), fixture(b"credential"));
    for file in [&image, &credential] {
        // SAFETY: a live handle owned by the fixture.
        assert_ne!(
            unsafe {
                SetHandleInformation(
                    file.as_raw_handle() as HANDLE,
                    HANDLE_FLAG_INHERIT,
                    HANDLE_FLAG_INHERIT,
                )
            },
            0
        );
        assert!(inheritable(file));
    }
    send(&mut owner, &[&image, &credential], until()).unwrap();
    let mut files = receive(&mut cli, super::super::DESCRIPTORS, until()).unwrap();
    assert_eq!(files.len(), 2);
    for (received, source) in files.iter().zip([&image, &credential]) {
        assert!(!inheritable(received));
        assert_ne!(received.as_raw_handle(), source.as_raw_handle());
    }
    let mut text = String::new();
    files[1].read_to_string(&mut text).unwrap();
    assert_eq!(text, "credential");
    assert!(files[0].write_all(b"x").is_err(), "read access alone");
}

/// Another count, or a value that is not a file handle a child would not
/// inherit, refuses.
#[test]
fn a_receive_refuses_another_count_and_foreign_values() {
    let (mut owner, mut cli) = ends();
    let image = fixture(b"image");
    send(&mut owner, &[&image], until()).unwrap();
    assert!(matches!(
        receive(&mut cli, 2, until()),
        Err(Failure::Protocol)
    ));
    let (mut owner, mut cli) = ends();
    // The pipe's own handle: open, but not a file.
    let pipe = cli.raw() as usize as u64;
    channel::write(
        &mut owner,
        &Handles {
            handles: vec![pipe],
        },
        None,
        &[],
        until(),
    )
    .unwrap();
    assert!(matches!(
        receive(&mut cli, 1, until()),
        Err(Failure::Protocol)
    ));
    let (mut owner, mut cli) = ends();
    let inherited = fixture(b"inherited");
    // SAFETY: a live handle owned by the fixture.
    unsafe {
        SetHandleInformation(
            inherited.as_raw_handle() as HANDLE,
            HANDLE_FLAG_INHERIT,
            HANDLE_FLAG_INHERIT,
        )
    };
    channel::write(
        &mut owner,
        &Handles {
            handles: vec![inherited.as_raw_handle() as usize as u64],
        },
        None,
        &[],
        until(),
    )
    .unwrap();
    assert!(matches!(
        receive(&mut cli, 1, until()),
        Err(Failure::Protocol)
    ));
    // Still the fixture's: a refused value is never taken over.
    assert!(inheritable(&inherited));
}

/// A handle duplicated into the peer that the peer never takes over is
/// closed there again, through the same process handle.
#[test]
fn duplicates_the_peer_never_took_over_are_closed() {
    let (owner, _cli) = ends();
    let peer = channel::peer(&owner).unwrap();
    let image = fixture(b"image");
    let values = duplicate_into(&peer, &[&image]).unwrap();
    let value = values[0] as usize as HANDLE;
    assert!(!channel::windows::inheritable(value).unwrap());
    close_in(&peer, &values);
    assert!(channel::windows::inheritable(value).is_err(), "closed");
}

/// A send to a peer that has closed fails without delivering anything.
#[test]
fn a_send_to_a_closed_peer_is_refused() {
    let (mut owner, cli) = ends();
    drop(cli);
    let image = fixture(b"image");
    assert!(send(&mut owner, &[&image], until()).is_err());
}

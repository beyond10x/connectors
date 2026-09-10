use super::*;
use connectors_sdk::Secret;
use serde::de::DeserializeOwned;
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    time::{Duration, Instant},
};

pub(crate) struct Frame<T> {
    pub control: T,
    pub secret: Secret,
    pub document: Vec<u8>,
}
pub(crate) fn wait_readable(stream: &UnixStream) -> Result<()> {
    use std::os::fd::AsRawFd;
    let mut descriptor = libc::pollfd {
        fd: stream.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    loop {
        // SAFETY: the descriptor and initialized pollfd remain live. Parent death
        // terminates the child even during an otherwise indefinite idle wait.
        let result = unsafe { libc::poll(&mut descriptor, 1, 60_000) };
        if result < 0 {
            return Err(io(std::io::Error::last_os_error()));
        }
        if result > 0 {
            return Ok(());
        }
    }
}
pub(crate) fn peer(stream: &UnixStream) -> Result<libc::ucred> {
    use std::os::fd::AsRawFd;
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: buffers have the requested sizes and the socket is held.
    if unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut credentials as *mut libc::ucred).cast(),
            &mut size,
        )
    } != 0
        || size as usize != std::mem::size_of::<libc::ucred>()
        || credentials.uid != super::super::filesystem::uid()
        || credentials.pid <= 0
    {
        return Err(Failure::Unavailable);
    }
    Ok(credentials)
}
fn remaining(until: Instant) -> Result<Duration> {
    until
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or(Failure::Timeout)
}
fn io(error: std::io::Error) -> Failure {
    match error.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => Failure::Timeout,
        std::io::ErrorKind::Interrupted => Failure::Interrupted,
        _ => Failure::Unavailable,
    }
}
fn read_exact(
    stream: &mut UnixStream,
    mut bytes: &mut [u8],
    until: Instant,
    cancel: Option<&dyn Fn() -> Result<()>>,
) -> Result<()> {
    while !bytes.is_empty() {
        if let Some(check) = cancel {
            check()?;
        }
        let remaining = remaining(until)?;
        stream
            .set_read_timeout(Some(if cancel.is_some() {
                remaining.min(Duration::from_millis(250))
            } else {
                remaining
            }))
            .map_err(io)?;
        let count = match stream.read(bytes) {
            Ok(count) => count,
            Err(error)
                if cancel.is_some()
                    && matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
            {
                continue;
            }
            Err(error) => return Err(io(error)),
        };
        if count == 0 {
            return Err(Failure::Unavailable);
        }
        bytes = &mut bytes[count..];
    }
    Ok(())
}
fn write_all(stream: &mut UnixStream, mut bytes: &[u8], until: Instant) -> Result<()> {
    while !bytes.is_empty() {
        stream
            .set_write_timeout(Some(remaining(until)?))
            .map_err(io)?;
        let count = stream.write(bytes).map_err(io)?;
        if count == 0 {
            return Err(Failure::Unavailable);
        }
        bytes = &bytes[count..];
    }
    Ok(())
}
pub(crate) fn read<T: DeserializeOwned>(
    stream: &mut UnixStream,
    until: Instant,
    secret_allowed: bool,
    document_limit: usize,
) -> Result<Frame<T>> {
    read_with_cancel(stream, until, secret_allowed, document_limit, None)
}
pub(crate) fn read_with_cancel<T: DeserializeOwned>(
    stream: &mut UnixStream,
    until: Instant,
    secret_allowed: bool,
    document_limit: usize,
    cancel: Option<&dyn Fn() -> Result<()>>,
) -> Result<Frame<T>> {
    let mut sizes = [0; 12];
    read_exact(stream, &mut sizes, until, cancel)?;
    let length = |offset| {
        u32::from_be_bytes(
            sizes[offset..offset + 4]
                .try_into()
                .expect("four-byte field"),
        ) as usize
    };
    let (control_size, secret_size, document_size) = (length(0), length(4), length(8));
    if control_size == 0
        || control_size > INPUT_LIMIT
        || secret_size > SECRET_LIMIT
        || (!secret_allowed && secret_size != 0)
        || document_size > document_limit
    {
        return Err(Failure::Protocol);
    }
    remaining(until)?;
    let mut bytes = vec![0; control_size];
    let mut secret = Secret(vec![0; secret_size]);
    let mut document = vec![0; document_size];
    read_exact(stream, &mut bytes, until, cancel)?;
    depth(&bytes)?;
    let control = connectors_core::read_json(&bytes).map_err(|_| Failure::Protocol)?;
    read_exact(stream, &mut secret.0, until, cancel)?;
    read_exact(stream, &mut document, until, cancel)?;
    Ok(Frame {
        control,
        secret,
        document,
    })
}
pub(crate) fn write(
    stream: &mut UnixStream,
    control: &impl Serialize,
    secret: Option<&Secret>,
    document: &[u8],
    until: Instant,
) -> Result<()> {
    let bytes = serde_json::to_vec(control).map_err(|_| Failure::Protocol)?;
    let protected = secret.map(|s| s.0.as_slice()).unwrap_or_default();
    if bytes.len() > INPUT_LIMIT || protected.len() > SECRET_LIMIT || document.len() > RESULT_LIMIT
    {
        return Err(Failure::Capacity);
    }
    let mut sizes = [0; 12];
    for (offset, length) in [(0, bytes.len()), (4, protected.len()), (8, document.len())] {
        sizes[offset..offset + 4].copy_from_slice(&(length as u32).to_be_bytes());
    }
    write_all(stream, &sizes, until)?;
    write_all(stream, &bytes, until)?;
    write_all(stream, protected, until)?;
    write_all(stream, document, until)
}
pub(crate) fn depth(bytes: &[u8]) -> Result<()> {
    let (mut quoted, mut escaped, mut depth) = (false, false, 0usize);
    for &b in bytes {
        if quoted {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                quoted = false;
            }
        } else {
            match b {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 64 {
                        return Err(Failure::Protocol);
                    }
                }
                b'}' | b']' => depth = depth.checked_sub(1).ok_or(Failure::Protocol)?,
                _ => {}
            }
        }
    }
    if depth != 0 || quoted {
        return Err(Failure::Protocol);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn deadline() -> Instant {
        Instant::now() + Duration::from_secs(2)
    }

    #[test]
    fn cancellation_observes_a_stalled_partial_reply_without_resetting_the_deadline() {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        writer.write_all(&[0, 0]).unwrap();
        let started = Instant::now();
        let cancel = || {
            if started.elapsed() >= Duration::from_millis(100) {
                Err(Failure::Interrupted)
            } else {
                Ok(())
            }
        };
        assert!(matches!(
            read_with_cancel::<serde_json::Value>(
                &mut reader,
                started + Duration::from_secs(5),
                false,
                0,
                Some(&cancel)
            ),
            Err(Failure::Interrupted)
        ));
        assert!(started.elapsed() < Duration::from_secs(2));
        let (mut reader, _writer) = UnixStream::pair().unwrap();
        let started = Instant::now();
        assert!(matches!(
            read_with_cancel::<serde_json::Value>(
                &mut reader,
                started + Duration::from_millis(100),
                false,
                0,
                Some(&|| Ok(()))
            ),
            Err(Failure::Timeout)
        ));
        assert!(started.elapsed() < Duration::from_secs(2));
    }
    fn raw(control: &[u8], protected: &[u8], document: &[u8]) -> UnixStream {
        let (reader, mut writer) = UnixStream::pair().unwrap();
        for length in [control.len(), protected.len(), document.len()] {
            writer.write_all(&(length as u32).to_be_bytes()).unwrap();
        }
        writer.write_all(control).unwrap();
        writer.write_all(protected).unwrap();
        writer.write_all(document).unwrap();
        reader
    }

    #[test]
    fn malformed_frames_are_refused_without_unbounded_reads() {
        let valid = br#"{"kind":"stop","request_id":"test"}"#;
        let mut stream = raw(valid, b"fictional protected bytes", &[]);
        assert!(matches!(
            read::<Request>(&mut stream, deadline(), false, 0),
            Err(Failure::Protocol)
        ));
        for bytes in [
            br#"{"kind":"stop","request_id":"one","request_id":"two"}"#.as_slice(),
            br#"{"kind":"stop","request_id":"one","extra":true}"#.as_slice(),
            br#"{"kind":"stop","request_id":"one"}{}"#.as_slice(),
        ] {
            assert!(matches!(
                read::<Request>(&mut raw(bytes, &[], &[]), deadline(), false, 0),
                Err(Failure::Protocol)
            ));
        }
        for sizes in [
            [u32::MAX, 0, 0],
            [10, (SECRET_LIMIT + 1) as u32, 0],
            [10, 0, 1],
        ] {
            let (mut reader, mut writer) = UnixStream::pair().unwrap();
            for size in sizes {
                writer.write_all(&size.to_be_bytes()).unwrap();
            }
            // Writer stays open without sending a body: refusal follows the
            // header's admission, not a later EOF or allocation of its claim.
            assert!(matches!(
                read::<Request>(&mut reader, deadline(), true, 0),
                Err(Failure::Protocol)
            ));
        }
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        writer.write_all(&[0, 0, 0]).unwrap();
        drop(writer);
        assert!(matches!(
            read::<Request>(&mut reader, deadline(), false, 0),
            Err(Failure::Unavailable)
        ));
    }

    #[test]
    fn stalled_partial_frame_uses_the_original_deadline() {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        writer.write_all(&[0, 0, 0]).unwrap();
        let until = Instant::now() + Duration::from_millis(50);
        assert!(matches!(
            read::<Request>(&mut reader, until, false, 0),
            Err(Failure::Timeout)
        ));
        // A second read cannot reset the deadline, even with more bytes present.
        writer.write_all(&[10]).unwrap();
        assert!(matches!(
            read::<Request>(&mut reader, until, false, 0),
            Err(Failure::Timeout)
        ));
    }

    #[test]
    fn separate_protected_section_and_bounded_document_depth() {
        let control =
            br#"{"kind":"validate","request_id":"test","profile":"profile","deadline_ms":1}"#;
        let mut stream = raw(control, b"fictional material", b"{}");
        let frame = read::<Request>(&mut stream, deadline(), true, INPUT_LIMIT).unwrap();
        assert_eq!(frame.secret.0, b"fictional material");
        assert_eq!(frame.document, b"{}");
        let control = serde_json::to_string(&frame.control).unwrap();
        assert!(!control.contains("fictional material"));
        assert!(depth(br#"{"quoted":"[\\\"{}]"}"#).is_ok());
        let nested = format!("{}0{}", "[".repeat(65), "]".repeat(65));
        assert_eq!(depth(nested.as_bytes()), Err(Failure::Protocol));
    }
}

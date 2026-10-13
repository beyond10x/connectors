//! Socket helpers shared by tests that play a browser against a listener.
use std::{net::TcpStream, time::Duration};

/// Sets the read timeout of a test's client `stream`.
///
/// macOS refuses every socket option (`EINVAL`) on a stream shut in both
/// directions, which a TCP client stream is once the listener has reset or
/// closed it, as the redirect listener does to a connection it drops unread;
/// Linux accepts the option. A read of such a stream returns at once with
/// what had arrived, so the refusal is passed over while the stream reports
/// itself readable now. Any other refusal fails the test and names the
/// error, so a stream that could still block never reads without a bound.
pub(crate) fn set_read_timeout(stream: &TcpStream, wait: Duration) {
    match stream.set_read_timeout(Some(wait)) {
        Ok(()) => {}
        Err(error) if cfg!(target_os = "macos") && error.raw_os_error() == Some(libc::EINVAL) => {
            assert!(
                readable_now(stream),
                "the read timeout was refused ({error}) on a stream that is not readable now"
            );
        }
        Err(error) => panic!("the read timeout was refused: {error}"),
    }
}

/// Whether a read of `stream` returns without waiting.
fn readable_now(stream: &TcpStream) -> bool {
    use std::os::fd::AsRawFd;
    let mut descriptor = libc::pollfd {
        fd: stream.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: the descriptor and initialized pollfd remain live; zero timeout.
    let ready = unsafe { libc::poll(&mut descriptor, 1, 0) };
    ready == 1 && descriptor.revents & (libc::POLLIN | libc::POLLHUP) != 0
}

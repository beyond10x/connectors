#![forbid(unsafe_code)]

//! Owner-authenticated Unix and capability-authenticated loopback transports for one-use Connect
//! Session credential submission.

use std::fs;
use std::io::Read as _;
use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use connector_secrets::Secret;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
use zeroize::Zeroizing;

const MAX_ENDPOINT_ID_BYTES: usize = 128;
const MAX_SUBMISSION_BYTES: usize = 64 * 1024;
const MAX_HTTP_HEADER_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CompletionTransportError {
    #[error("Connect Session endpoint policy was invalid")]
    InvalidPolicy,
    #[error("Connect Session endpoint state was unsafe")]
    UnsafeEndpoint,
    #[error("Connect Session endpoint I/O failed")]
    Io,
    #[error("Connect Session expired before an owner submission")]
    Expired,
    #[error("Connect Session credential submission was malformed")]
    InvalidSubmission,
}

/// One bound owner-only, single-use completion endpoint.
pub struct BoundCompletionEndpoint {
    listener: UnixListener,
    browser_listener: TcpListener,
    browser_token: Zeroizing<String>,
    path: PathBuf,
}

impl BoundCompletionEndpoint {
    /// Bind a new endpoint below an owner-only directory, refusing every pre-existing path.
    pub fn bind(directory: &Path, endpoint_id: &str) -> Result<Self, CompletionTransportError> {
        if endpoint_id.is_empty()
            || endpoint_id.len() > MAX_ENDPOINT_ID_BYTES
            || !endpoint_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(CompletionTransportError::InvalidPolicy);
        }
        ensure_owner_directory(directory)?;
        let path = directory.join(format!("{endpoint_id}.sock"));
        if fs::symlink_metadata(&path).is_ok() {
            return Err(CompletionTransportError::UnsafeEndpoint);
        }
        let browser_token = Zeroizing::new(random_token()?);
        let listener = UnixListener::bind(&path).map_err(|_| CompletionTransportError::Io)?;
        if fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).is_err() {
            let _ = fs::remove_file(&path);
            return Err(CompletionTransportError::Io);
        }
        let browser_listener = match bind_browser_listener() {
            Ok(listener) => listener,
            Err(error) => {
                let _ = fs::remove_file(&path);
                return Err(error);
            }
        };
        Ok(Self {
            listener,
            browser_listener,
            browser_token,
            path,
        })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// One-use browser setup page served by this Connector process.
    #[must_use]
    pub fn browser_url(&self) -> String {
        format!(
            "http://{}/#token={}",
            self.browser_listener
                .local_addr()
                .expect("bound listener has a local address"),
            self.browser_token.as_str()
        )
    }

    /// Wait for the process owner, read exactly one bounded line, and retire the endpoint.
    pub async fn receive(
        self,
        accept_deadline: Duration,
        read_deadline: Duration,
        maximum_secret_bytes: usize,
    ) -> Result<CompletionSubmission, CompletionTransportError> {
        if accept_deadline.is_zero()
            || read_deadline.is_zero()
            || maximum_secret_bytes == 0
            || maximum_secret_bytes > MAX_SUBMISSION_BYTES
        {
            return Err(CompletionTransportError::InvalidPolicy);
        }
        let accepted = tokio::time::timeout(accept_deadline, async {
            loop {
                tokio::select! {
                    result = accept_owner(&self.listener) => {
                        let mut stream = result?;
                        let secret = read_unix_secret(&mut stream, read_deadline, maximum_secret_bytes).await?;
                        break Ok(CompletionSubmission {
                            secret,
                            stream: CompletionStream::Unix(stream),
                        });
                    }
                    result = self.browser_listener.accept() => {
                        let (mut stream, peer) = result.map_err(|_| CompletionTransportError::Io)?;
                        if !peer.ip().is_loopback() {
                            continue;
                        }
                        match read_browser_request(
                            &mut stream,
                            &self.browser_token,
                            read_deadline,
                            maximum_secret_bytes,
                        ).await {
                            Ok(Some(secret)) => break Ok(CompletionSubmission {
                                secret,
                                stream: CompletionStream::Tcp(stream),
                            }),
                            Ok(None) => {}
                            Err(_) => {
                                let _ = write_http(
                                    &mut stream,
                                    "400 Bad Request",
                                    "text/plain; charset=utf-8",
                                    b"refused\n",
                                ).await;
                            }
                        }
                    }
                }
            }
        })
        .await;
        match accepted {
            Ok(result) => result,
            Err(_) => Err(CompletionTransportError::Expired),
        }
    }
}

impl Drop for BoundCompletionEndpoint {
    fn drop(&mut self) {
        let _ = remove_endpoint(&self.path);
    }
}

/// Owner-authenticated secret submission whose acknowledgement remains transport-owned.
pub struct CompletionSubmission {
    secret: Secret,
    stream: CompletionStream,
}

enum CompletionStream {
    Unix(UnixStream),
    Tcp(TcpStream),
}

impl CompletionSubmission {
    #[must_use]
    pub fn secret(&self) -> &Secret {
        &self.secret
    }

    #[must_use]
    pub fn into_secret(self) -> Secret {
        self.secret
    }

    pub async fn respond(mut self, accepted: bool) -> Result<(), CompletionTransportError> {
        match &mut self.stream {
            CompletionStream::Unix(stream) => {
                let response = if accepted {
                    b"{\"accepted\":true}\n".as_slice()
                } else {
                    b"{\"accepted\":false}\n".as_slice()
                };
                stream
                    .write_all(response)
                    .await
                    .map_err(|_| CompletionTransportError::Io)?;
                stream
                    .shutdown()
                    .await
                    .map_err(|_| CompletionTransportError::Io)
            }
            CompletionStream::Tcp(stream) => {
                let body = if accepted {
                    b"{\"accepted\":true}".as_slice()
                } else {
                    b"{\"accepted\":false}".as_slice()
                };
                write_http(stream, "200 OK", "application/json", body).await
            }
        }
    }
}

async fn read_unix_secret(
    stream: &mut UnixStream,
    read_deadline: Duration,
    maximum_secret_bytes: usize,
) -> Result<Secret, CompletionTransportError> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(maximum_secret_bytes.min(256)));
    let reader = BufReader::new(stream);
    let mut bounded = reader.take((maximum_secret_bytes + 3) as u64);
    tokio::time::timeout(read_deadline, bounded.read_until(b'\n', &mut bytes))
        .await
        .map_err(|_| CompletionTransportError::InvalidSubmission)?
        .map_err(|_| CompletionTransportError::Io)?;
    if bytes.last() != Some(&b'\n') || bytes.len() > maximum_secret_bytes + 2 {
        return Err(CompletionTransportError::InvalidSubmission);
    }
    bytes.pop();
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    secret_from_bytes(&mut bytes, maximum_secret_bytes)
}

async fn read_browser_request(
    stream: &mut TcpStream,
    expected_token: &str,
    read_deadline: Duration,
    maximum_secret_bytes: usize,
) -> Result<Option<Secret>, CompletionTransportError> {
    let mut reader = BufReader::new(&mut *stream);
    let mut headers = Zeroizing::new(Vec::with_capacity(1024));
    tokio::time::timeout(read_deadline, async {
        loop {
            if headers.len() >= MAX_HTTP_HEADER_BYTES {
                return Err(CompletionTransportError::InvalidSubmission);
            }
            let read = reader
                .read_until(b'\n', &mut headers)
                .await
                .map_err(|_| CompletionTransportError::Io)?;
            if read == 0 {
                return Err(CompletionTransportError::InvalidSubmission);
            }
            if headers.ends_with(b"\r\n\r\n") {
                return Ok(());
            }
        }
    })
    .await
    .map_err(|_| CompletionTransportError::InvalidSubmission)??;
    let headers_text =
        std::str::from_utf8(&headers).map_err(|_| CompletionTransportError::InvalidSubmission)?;
    let mut lines = headers_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or(CompletionTransportError::InvalidSubmission)?;
    if request_line == "GET / HTTP/1.1" {
        write_http(
            stream,
            "200 OK",
            "text/html; charset=utf-8",
            SETUP_PAGE.as_bytes(),
        )
        .await?;
        return Ok(None);
    }
    let mut presented_token = None;
    let mut content_length = None;
    let mut unsupported_transfer_encoding = false;
    for line in lines.filter(|line| !line.is_empty()) {
        let Some((name, value)) = line.split_once(':') else {
            return Err(CompletionTransportError::InvalidSubmission);
        };
        let value = value.trim_ascii();
        if name.eq_ignore_ascii_case("x-connect-session") {
            presented_token = Some(value);
        } else if name.eq_ignore_ascii_case("content-length") {
            content_length = Some(
                value
                    .parse::<usize>()
                    .map_err(|_| CompletionTransportError::InvalidSubmission)?,
            );
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            unsupported_transfer_encoding = true;
        }
    }
    if request_line != "POST /complete HTTP/1.1"
        || !presented_token
            .is_some_and(|token| constant_time_equal(token.as_bytes(), expected_token.as_bytes()))
        || unsupported_transfer_encoding
    {
        write_http(
            stream,
            "403 Forbidden",
            "text/plain; charset=utf-8",
            b"refused\n",
        )
        .await?;
        return Ok(None);
    }
    let content_length = content_length.ok_or(CompletionTransportError::InvalidSubmission)?;
    if content_length == 0 || content_length > maximum_secret_bytes {
        return Err(CompletionTransportError::InvalidSubmission);
    }
    let mut bytes = Zeroizing::new(vec![0_u8; content_length]);
    tokio::time::timeout(read_deadline, reader.read_exact(&mut bytes))
        .await
        .map_err(|_| CompletionTransportError::InvalidSubmission)?
        .map_err(|_| CompletionTransportError::Io)?;
    secret_from_bytes(&mut bytes, maximum_secret_bytes).map(Some)
}

fn secret_from_bytes(
    bytes: &mut Zeroizing<Vec<u8>>,
    maximum_secret_bytes: usize,
) -> Result<Secret, CompletionTransportError> {
    let value =
        std::str::from_utf8(bytes).map_err(|_| CompletionTransportError::InvalidSubmission)?;
    if value.is_empty()
        || value.len() > maximum_secret_bytes
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err(CompletionTransportError::InvalidSubmission);
    }
    let value = String::from_utf8(std::mem::take(&mut **bytes))
        .map_err(|_| CompletionTransportError::InvalidSubmission)?;
    Ok(Secret::new(value))
}

async fn write_http(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> Result<(), CompletionTransportError> {
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; form-action 'self'; base-uri 'none'\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        body.len()
    );
    stream
        .write_all(headers.as_bytes())
        .await
        .map_err(|_| CompletionTransportError::Io)?;
    stream
        .write_all(body)
        .await
        .map_err(|_| CompletionTransportError::Io)?;
    stream
        .shutdown()
        .await
        .map_err(|_| CompletionTransportError::Io)
}

fn bind_browser_listener() -> Result<TcpListener, CompletionTransportError> {
    let listener =
        std::net::TcpListener::bind("127.0.0.1:0").map_err(|_| CompletionTransportError::Io)?;
    listener
        .set_nonblocking(true)
        .map_err(|_| CompletionTransportError::Io)?;
    TcpListener::from_std(listener).map_err(|_| CompletionTransportError::Io)
}

fn random_token() -> Result<String, CompletionTransportError> {
    let mut source = fs::File::open("/dev/urandom").map_err(|_| CompletionTransportError::Io)?;
    let mut bytes = Zeroizing::new([0_u8; 32]);
    source
        .read_exact(&mut *bytes)
        .map_err(|_| CompletionTransportError::Io)?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(*bytes))
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        let left_byte = left.get(index).copied().unwrap_or_default();
        let right_byte = right.get(index).copied().unwrap_or_default();
        difference |= usize::from(left_byte ^ right_byte);
    }
    difference == 0
}

const SETUP_PAGE: &str = r#"<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Connect provider</title><style>body{font:16px system-ui;max-width:32rem;margin:4rem auto;padding:1rem;background:#111;color:#eee}input,button{box-sizing:border-box;width:100%;padding:.8rem;margin:.5rem 0}button{cursor:pointer}</style><h1>Connect provider</h1><p>The credential goes directly to this local Connectors process. Agent never receives it.</p><form><label>Provider credential<input name="token" type="password" autocomplete="off" required></label><button>Connect</button></form><p id="status"></p><script>const capability=new URLSearchParams(location.hash.slice(1)).get('token');history.replaceState(null,'',location.pathname+'#ready');document.querySelector('form').addEventListener('submit',async event=>{event.preventDefault();const input=event.currentTarget.elements.token;const status=document.querySelector('#status');try{const response=await fetch('/complete',{method:'POST',headers:{'Content-Type':'application/octet-stream','X-Connect-Session':capability},body:input.value});input.value='';status.textContent=response.ok?'Connected. You may close this tab.':'Connection was refused.'}catch{input.value='';status.textContent='Connectors is unavailable.'}});</script>"#;

pub fn remove_endpoint(path: &Path) -> Result<(), CompletionTransportError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(CompletionTransportError::Io),
    };
    if !metadata.file_type().is_socket() || metadata.uid() != rustix::process::geteuid().as_raw() {
        return Err(CompletionTransportError::UnsafeEndpoint);
    }
    fs::remove_file(path).map_err(|_| CompletionTransportError::Io)
}

fn ensure_owner_directory(path: &Path) -> Result<(), CompletionTransportError> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(|_| CompletionTransportError::Io)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| CompletionTransportError::Io)?;
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| CompletionTransportError::Io)?;
    if !metadata.file_type().is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(CompletionTransportError::UnsafeEndpoint);
    }
    Ok(())
}

async fn accept_owner(listener: &UnixListener) -> Result<UnixStream, CompletionTransportError> {
    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|_| CompletionTransportError::Io)?;
        let credential = stream
            .peer_cred()
            .map_err(|_| CompletionTransportError::Io)?;
        if credential.uid() == rustix::process::geteuid().as_raw() {
            return Ok(stream);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_capability_comparison_rejects_prefixes_and_differences() {
        assert!(constant_time_equal(b"capability", b"capability"));
        assert!(!constant_time_equal(b"capability", b"capabilit"));
        assert!(!constant_time_equal(b"capability", b"capabilitx"));
    }

    #[tokio::test]
    async fn endpoint_is_owner_only_one_use_and_removed_after_submission() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let endpoint = BoundCompletionEndpoint::bind(root.path(), "session-one").unwrap();
        let path = endpoint.path().to_path_buf();
        assert_eq!(
            fs::symlink_metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let client = tokio::spawn({
            let path = path.clone();
            async move {
                let mut stream = UnixStream::connect(path).await.unwrap();
                stream
                    .write_all(b"SENTINEL-NOT-A-REAL-SECRET\n")
                    .await
                    .unwrap();
                let mut response = String::new();
                BufReader::new(stream)
                    .read_line(&mut response)
                    .await
                    .unwrap();
                response
            }
        });
        let submission = endpoint
            .receive(Duration::from_secs(1), Duration::from_secs(1), 128)
            .await
            .unwrap();
        assert_eq!(
            submission.secret().expose_secret(),
            "SENTINEL-NOT-A-REAL-SECRET"
        );
        submission.respond(true).await.unwrap();
        assert_eq!(client.await.unwrap(), "{\"accepted\":true}\n");
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn browser_page_submits_directly_to_the_one_use_endpoint() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let endpoint = BoundCompletionEndpoint::bind(root.path(), "session-browser").unwrap();
        let browser_url = endpoint.browser_url();
        let authority = browser_url
            .strip_prefix("http://")
            .unwrap()
            .split_once('/')
            .unwrap()
            .0
            .to_owned();
        let token = browser_url.split_once("#token=").unwrap().1.to_owned();
        let client = tokio::spawn(async move {
            let mut refused = TcpStream::connect(&authority).await.unwrap();
            refused
                .write_all(
                    format!(
                        "POST /complete HTTP/1.1\r\nHost: {authority}\r\nX-Connect-Session: wrong-capability\r\nContent-Length: 5\r\n\r\nwrong"
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            let mut refused_response = String::new();
            refused.read_to_string(&mut refused_response).await.unwrap();
            assert!(refused_response.starts_with("HTTP/1.1 403 Forbidden\r\n"));

            let mut page = TcpStream::connect(&authority).await.unwrap();
            page.write_all(format!("GET / HTTP/1.1\r\nHost: {authority}\r\n\r\n").as_bytes())
                .await
                .unwrap();
            let mut page_response = String::new();
            page.read_to_string(&mut page_response).await.unwrap();
            assert!(page_response.starts_with("HTTP/1.1 200 OK\r\n"));
            assert!(page_response.contains("Connect provider"));

            let secret = b"xapp-SENTINEL-NOT-A-REAL-SECRET";
            let mut submit = TcpStream::connect(&authority).await.unwrap();
            submit
                .write_all(
                    format!(
                        "POST /complete HTTP/1.1\r\nHost: {authority}\r\nX-Connect-Session: {token}\r\nContent-Length: {}\r\n\r\n",
                        secret.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            submit.write_all(secret).await.unwrap();
            let mut response = String::new();
            submit.read_to_string(&mut response).await.unwrap();
            response
        });
        let submission = endpoint
            .receive(Duration::from_secs(1), Duration::from_secs(1), 128)
            .await
            .unwrap();
        assert_eq!(
            submission.secret().expose_secret(),
            "xapp-SENTINEL-NOT-A-REAL-SECRET"
        );
        submission.respond(true).await.unwrap();
        assert!(client.await.unwrap().starts_with("HTTP/1.1 200 OK\r\n"));
    }

    #[test]
    fn unsafe_directory_refuses() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(
            BoundCompletionEndpoint::bind(root.path(), "session-one"),
            Err(CompletionTransportError::UnsafeEndpoint)
        ));
    }
}

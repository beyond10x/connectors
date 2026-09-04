//! Dedicated TLS listener used only by the internal Git byte plane.

use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::serve::Listener;
use connectors_config::HostedTlsListenerConfig;
use futures_util::stream::{FuturesUnordered, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{Instant, Sleep};
use tokio_rustls::rustls::pki_types::pem::PemObject;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;

const MAX_PENDING_HANDSHAKES: usize = 128;
const MAX_ACTIVE_CONNECTIONS: usize = 128;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const READ_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const WRITE_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

type PendingHandshake =
    Pin<Box<dyn Future<Output = io::Result<(TlsStream<TcpStream>, SocketAddr)>> + Send + 'static>>;

/// An Axum listener that completes bounded concurrent TLS handshakes before yielding streams.
pub(crate) struct TlsListener {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    pending: FuturesUnordered<PendingHandshake>,
    active: Arc<Semaphore>,
}

/// One established internal connection. The semaphore permit lives exactly as long as the socket,
/// and per-direction idle deadlines cover request headers, bodies, keep-alive gaps and response
/// backpressure without buffering Git payloads in the listener.
pub(crate) struct DeadlineIo<I> {
    inner: I,
    read_deadline: Pin<Box<Sleep>>,
    write_deadline: Pin<Box<Sleep>>,
    read_timeout: Duration,
    write_timeout: Duration,
    _active: OwnedSemaphorePermit,
}

impl<I> DeadlineIo<I> {
    fn new(inner: I, active: OwnedSemaphorePermit) -> Self {
        Self::with_timeouts(inner, active, READ_IDLE_TIMEOUT, WRITE_IDLE_TIMEOUT)
    }

    fn with_timeouts(
        inner: I,
        active: OwnedSemaphorePermit,
        read_timeout: Duration,
        write_timeout: Duration,
    ) -> Self {
        Self {
            inner,
            read_deadline: Box::pin(tokio::time::sleep(read_timeout)),
            write_deadline: Box::pin(tokio::time::sleep(write_timeout)),
            read_timeout,
            write_timeout,
            _active: active,
        }
    }

    fn timed_out() -> io::Error {
        io::Error::new(
            io::ErrorKind::TimedOut,
            "internal HTTP connection timed out",
        )
    }
}

impl<I: AsyncRead + Unpin> AsyncRead for DeadlineIo<I> {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_read(context, buffer) {
            Poll::Ready(result) => {
                this.read_deadline
                    .as_mut()
                    .reset(Instant::now() + this.read_timeout);
                Poll::Ready(result)
            }
            Poll::Pending if this.read_deadline.as_mut().poll(context).is_ready() => {
                Poll::Ready(Err(Self::timed_out()))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<I: AsyncWrite + Unpin> AsyncWrite for DeadlineIo<I> {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_write(context, buffer) {
            Poll::Ready(result) => {
                this.write_deadline
                    .as_mut()
                    .reset(Instant::now() + this.write_timeout);
                Poll::Ready(result)
            }
            Poll::Pending if this.write_deadline.as_mut().poll(context).is_ready() => {
                Poll::Ready(Err(Self::timed_out()))
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_flush(context) {
            Poll::Ready(result) => {
                this.write_deadline
                    .as_mut()
                    .reset(Instant::now() + this.write_timeout);
                Poll::Ready(result)
            }
            Poll::Pending if this.write_deadline.as_mut().poll(context).is_ready() => {
                Poll::Ready(Err(Self::timed_out()))
            }
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_shutdown(context) {
            Poll::Ready(result) => Poll::Ready(result),
            Poll::Pending if this.write_deadline.as_mut().poll(context).is_ready() => {
                Poll::Ready(Err(Self::timed_out()))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl TlsListener {
    pub(crate) async fn bind(config: &HostedTlsListenerConfig) -> io::Result<Self> {
        // The wider binary carries clients using both rustls provider features. Select the
        // audited server provider explicitly instead of allowing feature unification to make the
        // process-level choice ambiguous.
        let _ = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider().install_default();
        let certificates = CertificateDer::pem_file_iter(&config.certificate_file)
            .map_err(invalid_pem)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(invalid_pem)?;
        if certificates.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "TLS certificate chain is empty",
            ));
        }
        let private_key =
            PrivateKeyDer::from_pem_file(&config.private_key_file).map_err(invalid_pem)?;
        let server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certificates, private_key)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let listener = TcpListener::bind(config.listen).await?;
        Ok(Self {
            listener,
            acceptor: TlsAcceptor::from(Arc::new(server_config)),
            pending: FuturesUnordered::new(),
            active: Arc::new(Semaphore::new(MAX_ACTIVE_CONNECTIONS)),
        })
    }

    fn begin_handshake(&mut self, stream: TcpStream, address: SocketAddr) {
        let acceptor = self.acceptor.clone();
        self.pending.push(Box::pin(async move {
            match tokio::time::timeout(HANDSHAKE_TIMEOUT, acceptor.accept(stream)).await {
                Ok(Ok(stream)) => Ok((stream, address)),
                Ok(Err(error)) => Err(error),
                Err(_) => Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "TLS handshake timed out",
                )),
            }
        }));
    }

    async fn accept_tcp(&mut self) {
        loop {
            match self.listener.accept().await {
                Ok((stream, address)) => {
                    self.begin_handshake(stream, address);
                    return;
                }
                Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
            }
        }
    }
}

fn invalid_pem(error: tokio_rustls::rustls::pki_types::pem::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

impl Listener for TlsListener {
    type Io = DeadlineIo<TlsStream<TcpStream>>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            if self.pending.is_empty() {
                self.accept_tcp().await;
                continue;
            }

            if self.pending.len() >= MAX_PENDING_HANDSHAKES {
                if let Some(Ok((stream, address))) = self.pending.next().await {
                    if let Ok(active) = self.active.clone().try_acquire_owned() {
                        return (DeadlineIo::new(stream, active), address);
                    }
                }
                continue;
            }

            tokio::select! {
                completed = self.pending.next() => {
                    if let Some(Ok((stream, address))) = completed {
                        if let Ok(active) = self.active.clone().try_acquire_owned() {
                            return (DeadlineIo::new(stream, active), address);
                        }
                    }
                }
                accepted = self.listener.accept() => {
                    match accepted {
                        Ok((stream, address)) => self.begin_handshake(stream, address),
                        Err(_) => tokio::time::sleep(Duration::from_secs(1)).await,
                    }
                }
            }
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.listener.local_addr()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use axum::routing::get;
    use rcgen::{generate_simple_self_signed, CertifiedKey};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio_rustls::rustls::pki_types::ServerName;
    use tokio_rustls::rustls::{ClientConfig, RootCertStore};
    use tokio_rustls::TlsConnector;

    use super::*;

    #[tokio::test]
    async fn established_connection_permit_is_lifetime_bound_and_reads_time_out() {
        let active = Arc::new(Semaphore::new(1));
        let permit = active.clone().try_acquire_owned().unwrap();
        let (_peer, stream) = tokio::io::duplex(64);
        let mut guarded = DeadlineIo::with_timeouts(
            stream,
            permit,
            Duration::from_millis(10),
            Duration::from_millis(10),
        );
        assert!(active.clone().try_acquire_owned().is_err());
        let mut byte = [0_u8; 1];
        let error = guarded.read_exact(&mut byte).await.unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        drop(guarded);
        assert!(active.try_acquire_owned().is_ok());
    }

    #[tokio::test]
    async fn listener_serves_the_internal_application_over_tls() {
        let directory = tempfile::tempdir().unwrap();
        let certificate_file = directory.path().join("tls.crt");
        let private_key_file = directory.path().join("tls.key");
        let CertifiedKey { cert, key_pair } =
            generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        fs::write(&certificate_file, cert.pem()).unwrap();
        fs::write(&private_key_file, key_pair.serialize_pem()).unwrap();

        let config = HostedTlsListenerConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            certificate_file,
            private_key_file,
        };
        let listener = TlsListener::bind(&config).await.unwrap();
        let address = listener.local_addr().unwrap();
        let application = axum::Router::new().route("/probe", get(|| async { "tls-only" }));
        let server = tokio::spawn(async move {
            axum::serve(listener, application).await.unwrap();
        });

        let mut roots = RootCertStore::empty();
        roots.add(cert.der().clone()).unwrap();
        let client = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(client));
        let stream = TcpStream::connect(address).await.unwrap();
        let server_name = ServerName::try_from("localhost".to_owned()).unwrap();
        let mut stream = connector.connect(server_name, stream).await.unwrap();
        stream
            .write_all(b"GET /probe HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response = String::from_utf8(response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response.ends_with("tls-only"));

        server.abort();
    }
}

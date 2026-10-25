use std::fmt;
use zeroize::Zeroizing;

/// The largest response body the crate reads from any endpoint, in bytes.
pub const RESPONSE_LIMIT: usize = 64 * 1024;

/// One HTTPS `POST` the crate asks its transport to send. Its `Debug` shows the body's length
/// only, so a transport that logs the request logs no code, verifier, token or secret.
pub struct Request<'a> {
    /// An `https` URL; the crate never asks for anything else.
    pub url: &'a str,
    /// `application/x-www-form-urlencoded` or `application/json`.
    pub content_type: &'static str,
    /// The body, which carries codes, verifiers, tokens or a client secret: the transport must
    /// not log it.
    pub body: &'a [u8],
    /// The response body bound: the transport reads at most `limit + 1` bytes of it, so the
    /// crate can tell an oversized body from one at the bound.
    pub limit: usize,
}

impl fmt::Debug for Request<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("url", &self.url)
            .field("content_type", &self.content_type)
            .field("body_len", &self.body.len())
            .field("limit", &self.limit)
            .finish()
    }
}

/// What the server answered.
pub struct Response {
    pub status: u16,
    /// The `Retry-After` header, as sent.
    pub retry_after: Option<String>,
    /// At most `limit + 1` bytes of the body.
    pub body: Zeroizing<Vec<u8>>,
}

/// A request that got no HTTP answer: no connection, no TLS session the transport trusts, or
/// no complete response head. It carries nothing, so nothing can leak through it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransportError;

/// The HTTPS client the host supplies. It sends `Accept: application/json`, follows no
/// redirect, verifies the server against the roots the host trusts, and reads at most
/// [`Request::limit`]` + 1` body bytes.
pub trait Transport {
    fn post(&self, request: &Request<'_>) -> Result<Response, TransportError>;
}

impl<T: Transport + ?Sized> Transport for &T {
    fn post(&self, request: &Request<'_>) -> Result<Response, TransportError> {
        (**self).post(request)
    }
}

impl<T: Transport + ?Sized> Transport for Box<T> {
    fn post(&self, request: &Request<'_>) -> Result<Response, TransportError> {
        (**self).post(request)
    }
}

impl<T: Transport + ?Sized> Transport for std::sync::Arc<T> {
    fn post(&self, request: &Request<'_>) -> Result<Response, TransportError> {
        (**self).post(request)
    }
}

use crate::{Failure, Secret, ServerDeclaration, Transport};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

/// A flag the caller sets to end a wait, a receive or a device grant.
#[derive(Clone, Debug, Default)]
pub struct Cancellation(Arc<AtomicBool>);

impl Cancellation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// How a grant waits between polls.
pub trait Waiter {
    /// Waits `duration`; [`Failure::Cancelled`] when the caller cancelled meanwhile.
    fn wait(&self, duration: Duration) -> Result<(), Failure>;
}

/// Sleeps the calling thread, and returns [`Failure::Cancelled`] within 50 ms of the
/// cancellation. A duration past what the clock can hold waits until the cancellation.
#[derive(Clone, Debug)]
pub struct ThreadWaiter {
    cancellation: Cancellation,
}

impl ThreadWaiter {
    pub fn new(cancellation: Cancellation) -> Self {
        Self { cancellation }
    }
}

/// How often a [`ThreadWaiter`] looks at its cancellation.
const SLICE: Duration = Duration::from_millis(50);

impl Waiter for ThreadWaiter {
    fn wait(&self, duration: Duration) -> Result<(), Failure> {
        let deadline = Instant::now().checked_add(duration);
        loop {
            if self.cancellation.is_cancelled() {
                return Err(Failure::Cancelled);
            }
            let now = Instant::now();
            let left = deadline.map_or(SLICE, |deadline| deadline.saturating_duration_since(now));
            if left.is_zero() {
                return Ok(());
            }
            std::thread::sleep(left.min(SLICE));
        }
    }
}

/// An RFC 8628 §3.2 device authorization response.
#[derive(Debug)]
pub struct DeviceAuthorization {
    pub(crate) device_code: Secret,
    /// The code the person enters at `verification_uri`.
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    /// How long the device code lives.
    pub expires_in: Duration,
    /// The polling interval: the server's, else 5 s.
    pub interval: Duration,
    pub(crate) issued: Instant,
}

/// What a [`DeviceDialect`] works with.
pub struct DialectContext<'a> {
    pub declaration: &'a ServerDeclaration,
    pub client_id: &'a str,
    pub transport: &'a dyn Transport,
    pub waiter: &'a dyn Waiter,
}

/// A provider's own, non-RFC 8628 device flow, implemented by an adapter. It ends in an
/// authorization code and the PKCE verifier it was issued for, which the crate exchanges at the
/// declaration's token endpoint like any other code.
pub trait DeviceDialect {
    fn authorize(&self, context: &DialectContext<'_>) -> Result<DialectGrant, Failure>;
}

/// The code a [`DeviceDialect`] obtained, with its verifier and the redirect URI the exchange
/// names.
#[derive(Debug)]
pub struct DialectGrant {
    pub code: Secret,
    pub verifier: Secret,
    pub redirect_uri: String,
}

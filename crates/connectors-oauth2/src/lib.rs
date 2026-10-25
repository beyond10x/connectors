//! A provider-neutral OAuth 2.0 client for any authorization server a [`ServerDeclaration`]
//! describes: the authorization-code grant with PKCE S256 (RFC 6749 §4.1, RFC 7636), the device
//! grant (RFC 8628), and the code, device and refresh exchanges.
//!
//! The crate holds no provider name, endpoint or client id; those are declaration data. It sends
//! every request through a [`Transport`] the host supplies. Codes, verifiers, `state` values,
//! tokens and client secrets are [`Secret`]s: zeroized on drop and never shown by `Debug`. Every
//! failure is one [`Failure`] code that carries no text a server wrote.
//!
//! An authorization-code grant:
//!
//! 1. [`LoopbackReceiver::bind`] (or, for `pasted_code`, the declaration's `redirect_uri`),
//! 2. [`Client::begin`] with that redirect URI, and show [`Authorization::url`] to the person,
//! 3. [`LoopbackReceiver::receive`] or [`pasted_code`] for the [`AuthorizationCode`],
//! 4. [`Client::exchange`] for a [`Granted`] token set.
//!
//! A device grant is [`Client::device_authorize`] then [`Client::poll_device`]; a provider's
//! non-RFC device flow is a [`DeviceDialect`] run by [`Client::device_dialect`]. A stored grant
//! is renewed with [`Client::refresh`].
mod client;
mod declaration;
mod device;
mod failure;
mod pkce;
mod receiver;
mod secret;
mod token;
mod transport;

pub use client::{Authorization, AuthorizationCode, Client};
pub use declaration::{
    ClientKind, GrantType, Parameter, RedirectKind, RequestEncoding, Rotation, ServerDeclaration,
};
pub use device::{
    Cancellation, DeviceAuthorization, DeviceDialect, DialectContext, DialectGrant, ThreadWaiter,
    Waiter,
};
pub use failure::Failure;
pub use pkce::pkce_challenge;
pub use receiver::{LoopbackReceiver, pasted_code};
pub use secret::Secret;
pub use token::{Granted, Refreshed};
pub use transport::{RESPONSE_LIMIT, Request, Response, Transport, TransportError};

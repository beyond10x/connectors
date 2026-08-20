#![forbid(unsafe_code)]

//! SIP Integration adapter.

mod backend;
mod raw;
mod runtime;

pub use backend::{LaunchError, LaunchedSession, SessionLauncher, SipOperationBackend};
pub use raw::SipLauncher;
pub use runtime::{load_authority_issuer, RuntimeLauncher, StoredSipCredentials};

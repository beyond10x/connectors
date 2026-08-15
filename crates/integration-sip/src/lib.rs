#![forbid(unsafe_code)]

//! SIP Integration adapter.

mod backend;
mod runtime;

pub use backend::{LaunchError, LaunchedSession, SessionLauncher, SipOperationBackend};
pub use runtime::{load_authority_issuer, RuntimeLauncher};

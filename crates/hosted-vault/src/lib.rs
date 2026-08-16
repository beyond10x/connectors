#![forbid(unsafe_code)]

//! Hosted Vault credential-source adapter.

mod adapter;
mod prepared;

pub use adapter::{HostedVaultError, HostedVaultStore};
pub use prepared::PreparedVaultStore;

#![forbid(unsafe_code)]

//! Hosted Vault credential-source adapter.

mod adapter;

pub use adapter::{HostedVaultError, HostedVaultStore};

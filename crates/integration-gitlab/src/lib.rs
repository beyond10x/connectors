#![forbid(unsafe_code)]

//! GitLab delegated-user Integration adapter.

mod backend;
mod open;
mod state;
mod transport;

pub use backend::{GitlabBackend, GitlabError};

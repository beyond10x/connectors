#![forbid(unsafe_code)]

//! GitLab delegated-user Integration adapter.

mod backend;
mod open;
mod state;

pub use backend::{GitlabBackend, GitlabError};

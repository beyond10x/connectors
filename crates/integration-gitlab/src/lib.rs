#![forbid(unsafe_code)]

//! GitLab delegated-user Integration adapter.

mod backend;

pub use backend::{GitlabBackend, GitlabError};

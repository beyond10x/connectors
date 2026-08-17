#![forbid(unsafe_code)]

//! Jira Cloud organization-read and delegated-user Integration adapter.

mod backend;

pub use backend::{JiraBackend, JiraError};

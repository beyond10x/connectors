#![forbid(unsafe_code)]

//! Slack Integration adapter.

mod backend;

pub use backend::{SlackBackend, SlackError};

//! Shared fail-closed state machine for short-lived, one-use Connect Sessions.

use std::collections::BTreeMap;

use protocol::connection::{ConnectSessionState, ConnectSessionStatus};

#[derive(Debug, Clone)]
struct SessionRecord {
    label: String,
    state: ConnectSessionState,
    expires_at_unix_ms: u64,
    completion_endpoint: Option<String>,
    browser_completion_url: Option<String>,
    connection_ref: Option<String>,
}

/// Allowed terminal outcomes for a pending Connect Session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectSessionTerminal {
    Completed { connection_ref: String },
    Expired,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConnectSessionLifecycleError {
    #[error("Connect Session input is invalid")]
    Invalid,
    #[error("too many Connect Sessions are pending")]
    Capacity,
    #[error("Connect Session already exists")]
    Duplicate,
    #[error("Connect Session was not found")]
    NotFound,
    #[error("Connect Session is no longer pending")]
    NotPending,
}

/// In-memory lifecycle registry shared by credential-acquisition integrations.
///
/// Secret submission and provider verification remain backend-specific. This type owns the
/// security-sensitive common rules: bounded pending capacity, one-way terminal transitions,
/// endpoint removal at terminal state, and value-free protocol projection.
pub struct ConnectSessionLifecycle {
    integration_ref: String,
    maximum_pending: usize,
    sessions: BTreeMap<String, SessionRecord>,
}

impl ConnectSessionLifecycle {
    pub fn new(
        integration_ref: impl Into<String>,
        maximum_pending: usize,
    ) -> Result<Self, ConnectSessionLifecycleError> {
        let integration_ref = integration_ref.into();
        if !valid_ref(&integration_ref) || maximum_pending == 0 {
            return Err(ConnectSessionLifecycleError::Invalid);
        }
        Ok(Self {
            integration_ref,
            maximum_pending,
            sessions: BTreeMap::new(),
        })
    }

    pub fn reserve(
        &mut self,
        session_ref: String,
        label: String,
        expires_at_unix_ms: u64,
        completion_endpoint: String,
    ) -> Result<ConnectSessionStatus, ConnectSessionLifecycleError> {
        self.reserve_with_browser(
            session_ref,
            label,
            expires_at_unix_ms,
            completion_endpoint,
            None,
        )
    }

    pub fn reserve_with_browser(
        &mut self,
        session_ref: String,
        label: String,
        expires_at_unix_ms: u64,
        completion_endpoint: String,
        browser_completion_url: Option<String>,
    ) -> Result<ConnectSessionStatus, ConnectSessionLifecycleError> {
        if !valid_ref(&session_ref)
            || label.trim().is_empty()
            || label.len() > 256
            || expires_at_unix_ms == 0
            || completion_endpoint.is_empty()
            || completion_endpoint.len() > 4_096
            || browser_completion_url
                .as_deref()
                .is_some_and(|value| value.is_empty() || value.len() > 4_096)
        {
            return Err(ConnectSessionLifecycleError::Invalid);
        }
        if self.sessions.contains_key(&session_ref) {
            return Err(ConnectSessionLifecycleError::Duplicate);
        }
        if self
            .sessions
            .values()
            .filter(|session| session.state == ConnectSessionState::Pending)
            .count()
            >= self.maximum_pending
        {
            return Err(ConnectSessionLifecycleError::Capacity);
        }
        self.sessions.insert(
            session_ref.clone(),
            SessionRecord {
                label,
                state: ConnectSessionState::Pending,
                expires_at_unix_ms,
                completion_endpoint: Some(completion_endpoint),
                browser_completion_url,
                connection_ref: None,
            },
        );
        self.status(&session_ref)
            .ok_or(ConnectSessionLifecycleError::NotFound)
    }

    #[must_use]
    pub fn owns(&self, session_ref: &str) -> bool {
        self.sessions.contains_key(session_ref)
    }

    #[must_use]
    pub fn status(&self, session_ref: &str) -> Option<ConnectSessionStatus> {
        let session = self.sessions.get(session_ref)?;
        Some(ConnectSessionStatus {
            connect_session_ref: session_ref.to_owned(),
            integration_ref: self.integration_ref.clone(),
            state: session.state,
            expires_at_unix_ms: session.expires_at_unix_ms,
            completion_endpoint: session.completion_endpoint.clone(),
            browser_completion_url: session.browser_completion_url.clone(),
            connection_ref: session.connection_ref.clone(),
        })
    }

    pub fn pending_label(&self, session_ref: &str) -> Result<String, ConnectSessionLifecycleError> {
        let session = self
            .sessions
            .get(session_ref)
            .ok_or(ConnectSessionLifecycleError::NotFound)?;
        if session.state != ConnectSessionState::Pending {
            return Err(ConnectSessionLifecycleError::NotPending);
        }
        Ok(session.label.clone())
    }

    pub fn finish(
        &mut self,
        session_ref: &str,
        terminal: ConnectSessionTerminal,
    ) -> Result<(), ConnectSessionLifecycleError> {
        let session = self
            .sessions
            .get_mut(session_ref)
            .ok_or(ConnectSessionLifecycleError::NotFound)?;
        if session.state != ConnectSessionState::Pending {
            return Err(ConnectSessionLifecycleError::NotPending);
        }
        let (state, connection_ref) = match terminal {
            ConnectSessionTerminal::Completed { connection_ref } if valid_ref(&connection_ref) => {
                (ConnectSessionState::Completed, Some(connection_ref))
            }
            ConnectSessionTerminal::Completed { .. } => {
                return Err(ConnectSessionLifecycleError::Invalid)
            }
            ConnectSessionTerminal::Expired => (ConnectSessionState::Expired, None),
            ConnectSessionTerminal::Failed => (ConnectSessionState::Failed, None),
        };
        session.state = state;
        session.completion_endpoint = None;
        session.browser_completion_url = None;
        session.connection_ref = connection_ref;
        Ok(())
    }

    /// Fail every still-pending session and return its endpoint for owner-checked cleanup.
    pub fn fail_pending(&mut self) -> Vec<String> {
        let mut endpoints = Vec::new();
        for session in self.sessions.values_mut() {
            if session.state == ConnectSessionState::Pending {
                if let Some(endpoint) = session.completion_endpoint.take() {
                    endpoints.push(endpoint);
                }
                session.browser_completion_url = None;
                session.state = ConnectSessionState::Failed;
            }
        }
        endpoints
    }
}

fn valid_ref(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lifecycle(maximum_pending: usize) -> ConnectSessionLifecycle {
        ConnectSessionLifecycle::new("slack", maximum_pending).unwrap()
    }

    #[test]
    fn capacity_counts_only_pending_sessions() {
        let mut sessions = lifecycle(1);
        sessions
            .reserve(
                "connect-session:one".to_owned(),
                "One".to_owned(),
                1,
                "/run/one.sock".to_owned(),
            )
            .unwrap();
        assert_eq!(
            sessions.reserve(
                "connect-session:two".to_owned(),
                "Two".to_owned(),
                2,
                "/run/two.sock".to_owned(),
            ),
            Err(ConnectSessionLifecycleError::Capacity)
        );
        sessions
            .finish("connect-session:one", ConnectSessionTerminal::Expired)
            .unwrap();
        sessions
            .reserve(
                "connect-session:two".to_owned(),
                "Two".to_owned(),
                2,
                "/run/two.sock".to_owned(),
            )
            .unwrap();
    }

    #[test]
    fn terminal_transition_is_one_way_and_value_free() {
        let mut sessions = lifecycle(1);
        sessions
            .reserve(
                "connect-session:one".to_owned(),
                "One".to_owned(),
                1,
                "/run/one.sock".to_owned(),
            )
            .unwrap();
        sessions
            .finish(
                "connect-session:one",
                ConnectSessionTerminal::Completed {
                    connection_ref: "connection:one".to_owned(),
                },
            )
            .unwrap();
        let status = sessions.status("connect-session:one").unwrap();
        assert_eq!(status.state, ConnectSessionState::Completed);
        assert_eq!(status.completion_endpoint, None);
        assert_eq!(status.connection_ref.as_deref(), Some("connection:one"));
        assert_eq!(
            sessions.finish("connect-session:one", ConnectSessionTerminal::Failed),
            Err(ConnectSessionLifecycleError::NotPending)
        );
    }

    #[test]
    fn shutdown_fails_only_pending_sessions_and_returns_their_endpoints() {
        let mut sessions = lifecycle(2);
        for id in ["one", "two"] {
            sessions
                .reserve(
                    format!("connect-session:{id}"),
                    id.to_owned(),
                    1,
                    format!("/run/{id}.sock"),
                )
                .unwrap();
        }
        sessions
            .finish("connect-session:one", ConnectSessionTerminal::Expired)
            .unwrap();
        assert_eq!(sessions.fail_pending(), vec!["/run/two.sock"]);
        assert_eq!(
            sessions.status("connect-session:one").unwrap().state,
            ConnectSessionState::Expired
        );
        assert_eq!(
            sessions.status("connect-session:two").unwrap().state,
            ConnectSessionState::Failed
        );
    }
}

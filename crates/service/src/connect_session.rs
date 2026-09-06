//! Shared fail-closed state machine for short-lived, one-use Connect Sessions.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use protocol::connection::{ConnectSessionState, ConnectSessionStatus};

#[derive(Debug, Clone)]
struct SessionRecord {
    label: String,
    state: ConnectSessionState,
    expires_at_unix_ms: u64,
    completion_endpoint: Option<String>,
    browser_completion_url: Option<String>,
    connection_ref: Option<String>,
    bound_connection: Option<String>,
    completion: Option<Completion>,
}

#[derive(Debug, Clone)]
struct Completion {
    authorized_at: Option<u64>,
    recovery_required: bool,
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
    clock: Arc<dyn Fn() -> Result<u64, ConnectSessionLifecycleError> + Send + Sync>,
}

impl ConnectSessionLifecycle {
    pub fn new(
        integration_ref: impl Into<String>,
        maximum_pending: usize,
    ) -> Result<Self, ConnectSessionLifecycleError> {
        Self::with_clock(integration_ref, maximum_pending, || {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|duration| u64::try_from(duration.as_millis()).ok())
                .filter(|instant| *instant > 0)
                .ok_or(ConnectSessionLifecycleError::Invalid)
        })
    }

    /// Construct with a receiver-owned trusted clock, never a request-supplied timestamp.
    pub fn with_clock(
        integration_ref: impl Into<String>,
        maximum_pending: usize,
        clock: impl Fn() -> Result<u64, ConnectSessionLifecycleError> + Send + Sync + 'static,
    ) -> Result<Self, ConnectSessionLifecycleError> {
        let integration_ref = integration_ref.into();
        if !valid_ref(&integration_ref) || maximum_pending == 0 {
            return Err(ConnectSessionLifecycleError::Invalid);
        }
        Ok(Self {
            integration_ref,
            maximum_pending,
            sessions: BTreeMap::new(),
            clock: Arc::new(clock),
        })
    }

    /// Bind the already admitted Connection once, before exposing a session to its creator.
    pub fn bind_completion_target(
        &mut self,
        session_ref: &str,
        connection_ref: &str,
    ) -> Result<(), ConnectSessionLifecycleError> {
        if !valid_ref(connection_ref) {
            return Err(ConnectSessionLifecycleError::Invalid);
        }
        self.pending_label(session_ref)?;
        let session = self
            .sessions
            .get_mut(session_ref)
            .ok_or(ConnectSessionLifecycleError::NotFound)?;
        if session.bound_connection.is_some() {
            return Err(ConnectSessionLifecycleError::Duplicate);
        }
        session.bound_connection = Some(connection_ref.to_owned());
        Ok(())
    }

    /// Retire instruction endpoints before custody I/O. The receiver must stop their tasks and
    /// perform owner-checked cleanup; returned local paths are never a protocol projection.
    /// This guard prevents a false terminal result while prepare/abort is unresolved.
    pub fn begin_completion(
        &mut self,
        session_ref: &str,
        connection_ref: &str,
    ) -> Result<Vec<String>, ConnectSessionLifecycleError> {
        self.pending_label(session_ref)?;
        let session = self
            .sessions
            .get_mut(session_ref)
            .ok_or(ConnectSessionLifecycleError::NotFound)?;
        if session.bound_connection.as_deref() != Some(connection_ref) {
            return Err(ConnectSessionLifecycleError::Invalid);
        }
        session.completion = Some(Completion {
            authorized_at: None,
            recovery_required: false,
        });
        session.browser_completion_url = None;
        Ok(session.completion_endpoint.take().into_iter().collect())
    }

    /// Claim after prepare under the receiver's lifecycle/current-authority lock. The closure
    /// synchronously rechecks the captured grant, target and generation at the same trusted instant.
    /// The returned pair is (authorized_at, original deadline), never a caller-supplied instant.
    pub fn claim_completion(
        &mut self,
        session_ref: &str,
        connection_ref: &str,
        recheck: impl FnOnce(u64) -> bool,
    ) -> Result<(u64, u64), ConnectSessionLifecycleError> {
        let session = self
            .sessions
            .get_mut(session_ref)
            .ok_or(ConnectSessionLifecycleError::NotFound)?;
        let completion = session
            .completion
            .as_mut()
            .ok_or(ConnectSessionLifecycleError::NotPending)?;
        if completion.authorized_at.is_some()
            || completion.recovery_required
            || session.bound_connection.as_deref() != Some(connection_ref)
        {
            return Err(ConnectSessionLifecycleError::NotPending);
        }
        let now = (self.clock)()?;
        if now == 0 || now >= session.expires_at_unix_ms || !recheck(now) {
            return Err(ConnectSessionLifecycleError::Invalid);
        }
        completion.authorized_at = Some(now);
        Ok((now, session.expires_at_unix_ms))
    }

    /// Preserve private ownership after an uncertain durable operation; ordinary shutdown and
    /// expiry cannot turn this into a false terminal result.
    pub fn require_completion_recovery(
        &mut self,
        session_ref: &str,
    ) -> Result<(), ConnectSessionLifecycleError> {
        let session = self
            .sessions
            .get_mut(session_ref)
            .ok_or(ConnectSessionLifecycleError::NotFound)?;
        let completion = session
            .completion
            .as_mut()
            .ok_or(ConnectSessionLifecycleError::NotPending)?;
        completion.recovery_required = true;
        Ok(())
    }

    /// Resolve only after the custody owner has established durable publication or a confirmed
    /// undecided abort. This is a trusted receiver seam, not a caller-controlled finish command.
    pub fn resolve_completion(
        &mut self,
        session_ref: &str,
        connection_ref: &str,
        published: bool,
    ) -> Result<(), ConnectSessionLifecycleError> {
        let session = self
            .sessions
            .get_mut(session_ref)
            .ok_or(ConnectSessionLifecycleError::NotFound)?;
        let completion = session
            .completion
            .as_ref()
            .ok_or(ConnectSessionLifecycleError::NotPending)?;
        if session.bound_connection.as_deref() != Some(connection_ref)
            || published && completion.authorized_at.is_none()
        {
            return Err(ConnectSessionLifecycleError::Invalid);
        }
        let terminal = if published {
            ConnectSessionTerminal::Completed {
                connection_ref: connection_ref.to_owned(),
            }
        } else if completion.authorized_at.is_none()
            && (self.clock)().is_ok_and(|now| now >= session.expires_at_unix_ms)
        {
            ConnectSessionTerminal::Expired
        } else {
            ConnectSessionTerminal::Failed
        };
        session.completion = None;
        self.finish(session_ref, terminal)
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
        self.reserve_endpoints(
            session_ref,
            label,
            expires_at_unix_ms,
            Some(completion_endpoint),
            browser_completion_url,
        )
    }

    /// Reserve a hosted session whose one-use completion endpoint is exposed only as HTTPS.
    pub fn reserve_browser(
        &mut self,
        session_ref: String,
        label: String,
        expires_at_unix_ms: u64,
        browser_completion_url: String,
    ) -> Result<ConnectSessionStatus, ConnectSessionLifecycleError> {
        self.reserve_endpoints(
            session_ref,
            label,
            expires_at_unix_ms,
            None,
            Some(browser_completion_url),
        )
    }

    fn reserve_endpoints(
        &mut self,
        session_ref: String,
        label: String,
        expires_at_unix_ms: u64,
        completion_endpoint: Option<String>,
        browser_completion_url: Option<String>,
    ) -> Result<ConnectSessionStatus, ConnectSessionLifecycleError> {
        if !valid_ref(&session_ref)
            || label.trim().is_empty()
            || label.len() > 256
            || expires_at_unix_ms == 0
            || completion_endpoint
                .as_deref()
                .is_some_and(|value| value.is_empty() || value.len() > 4_096)
            || browser_completion_url
                .as_deref()
                .is_some_and(|value| value.is_empty() || value.len() > 4_096)
            || completion_endpoint.is_none() && browser_completion_url.is_none()
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
                completion_endpoint,
                browser_completion_url,
                connection_ref: None,
                bound_connection: None,
                completion: None,
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
        if session.completion.is_some() {
            return None;
        }
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
        if session.state != ConnectSessionState::Pending || session.completion.is_some() {
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
        if session.state != ConnectSessionState::Pending || session.completion.is_some() {
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
            if session.state == ConnectSessionState::Pending && session.completion.is_none() {
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

    fn controlled() -> (ConnectSessionLifecycle, Arc<std::sync::atomic::AtomicU64>) {
        let clock = Arc::new(std::sync::atomic::AtomicU64::new(99));
        let observed = clock.clone();
        let mut sessions = ConnectSessionLifecycle::with_clock("gitlab", 1, move || {
            Ok(observed.load(std::sync::atomic::Ordering::SeqCst))
        })
        .unwrap();
        sessions
            .reserve(
                "session:one".into(),
                "display only".into(),
                100,
                "/run/one.sock".into(),
            )
            .unwrap();
        sessions
            .bind_completion_target("session:one", "connection:one")
            .unwrap();
        (sessions, clock)
    }

    #[test]
    fn a_completion_claim_blocks_expiry_shutdown_and_duplicate_completion() {
        let (mut sessions, clock) = controlled();
        sessions
            .begin_completion("session:one", "connection:one")
            .unwrap();
        assert_eq!(
            sessions.claim_completion("session:one", "connection:one", |_| true),
            Ok((99, 100))
        );
        clock.store(101, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            sessions.finish("session:one", ConnectSessionTerminal::Expired),
            Err(ConnectSessionLifecycleError::NotPending)
        );
        assert!(sessions.fail_pending().is_empty());
        assert!(sessions.owns("session:one"));
        assert!(sessions.status("session:one").is_none());
        assert!(sessions
            .claim_completion("session:one", "connection:one", |_| true)
            .is_err());
        sessions
            .resolve_completion("session:one", "connection:one", true)
            .unwrap();
        assert_eq!(
            sessions.status("session:one").unwrap().state,
            ConnectSessionState::Completed
        );
        assert!(sessions
            .resolve_completion("session:one", "connection:one", true)
            .is_err());
    }

    #[test]
    fn preparing_and_uncertain_abort_cannot_publish_a_terminal_result() {
        let (mut sessions, _) = controlled();
        assert_eq!(
            sessions
                .begin_completion("session:one", "connection:one")
                .unwrap(),
            vec!["/run/one.sock"]
        );
        sessions.require_completion_recovery("session:one").unwrap();
        assert!(sessions.status("session:one").is_none());
        assert!(sessions.fail_pending().is_empty());
        assert_eq!(
            sessions.finish("session:one", ConnectSessionTerminal::Failed),
            Err(ConnectSessionLifecycleError::NotPending)
        );
        assert!(sessions
            .resolve_completion("session:one", "connection:one", true)
            .is_err());
        sessions
            .resolve_completion("session:one", "connection:one", false)
            .unwrap();
        assert_eq!(
            sessions.status("session:one").unwrap().state,
            ConnectSessionState::Failed
        );
    }

    #[test]
    fn the_claim_rechecks_the_original_target_and_inclusive_deadline_once() {
        let (mut sessions, clock) = controlled();
        assert!(sessions
            .bind_completion_target("session:one", "connection:other")
            .is_err());
        assert!(sessions
            .begin_completion("session:one", "connection:other")
            .is_err());
        sessions
            .begin_completion("session:one", "connection:one")
            .unwrap();
        clock.store(100, std::sync::atomic::Ordering::SeqCst);
        assert!(sessions
            .claim_completion("session:one", "connection:one", |_| true)
            .is_err());
        sessions
            .resolve_completion("session:one", "connection:one", false)
            .unwrap();
        assert_eq!(
            sessions.status("session:one").unwrap().state,
            ConnectSessionState::Expired
        );
    }

    #[test]
    fn authority_and_claim_use_one_receiver_owned_instant() {
        let (mut sessions, clock) = controlled();
        sessions
            .begin_completion("session:one", "connection:one")
            .unwrap();
        assert_eq!(
            sessions.claim_completion("session:one", "connection:one", |now| {
                assert_eq!(now, 99);
                clock.store(101, std::sync::atomic::Ordering::SeqCst);
                true
            }),
            Ok((99, 100))
        );
        sessions.require_completion_recovery("session:one").unwrap();
        sessions
            .resolve_completion("session:one", "connection:one", false)
            .unwrap();
        assert_eq!(
            sessions.status("session:one").unwrap().state,
            ConnectSessionState::Failed
        );
    }

    #[test]
    fn clock_failure_refuses_authorization_but_does_not_prevent_confirmed_abort() {
        let mut sessions = ConnectSessionLifecycle::with_clock("gitlab", 1, || {
            Err(ConnectSessionLifecycleError::Invalid)
        })
        .unwrap();
        sessions
            .reserve(
                "session:one".into(),
                "display".into(),
                100,
                "/run/one".into(),
            )
            .unwrap();
        sessions
            .bind_completion_target("session:one", "connection:one")
            .unwrap();
        sessions
            .begin_completion("session:one", "connection:one")
            .unwrap();
        assert!(sessions
            .claim_completion("session:one", "connection:one", |_| panic!(
                "untrusted time must not reach authority"
            ))
            .is_err());
        sessions
            .resolve_completion("session:one", "connection:one", false)
            .unwrap();
        assert_eq!(
            sessions.status("session:one").unwrap().state,
            ConnectSessionState::Failed
        );
    }

    #[test]
    fn guarded_sessions_keep_capacity_until_a_confirmed_outcome() {
        let (mut sessions, _) = controlled();
        sessions
            .begin_completion("session:one", "connection:one")
            .unwrap();
        assert_eq!(
            sessions.reserve("session:two".into(), "other".into(), 100, "/run/two".into()),
            Err(ConnectSessionLifecycleError::Capacity)
        );
        assert!(sessions
            .resolve_completion("session:one", "connection:other", false)
            .is_err());
        sessions
            .resolve_completion("session:one", "connection:one", false)
            .unwrap();
        assert!(sessions
            .reserve("session:two".into(), "other".into(), 100, "/run/two".into())
            .is_ok());
    }

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

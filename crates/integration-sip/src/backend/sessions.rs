//! Acting on a call that is already established.
//!
//! `sip.dial` returns and the call outlives it, so everything that happens next -- reading its
//! state, sending a keypress into it, ending it -- addresses the session by the `execution_ref`
//! that invocation returned. That is a different lifecycle from invoking an operation, and it is
//! why these live apart from the catalogue-facing surface in [`super`].
//!
//! A child module so it can reach its parent's private state: the session registry is the
//! backend's own, and widening it to `pub(crate)` to move this code would have made it reachable
//! from every other module in the crate to save one directory.

use super::*;

impl<L: SessionLauncher> SipOperationBackend<L> {
    pub(super) fn prune_terminal_records(&self) -> Result<(), OperationError> {
        let mut sessions = lock(&self.sessions);
        if sessions.len() < MAX_SESSION_RECORDS {
            return Ok(());
        }
        let remove = sessions
            .iter()
            .filter(|(_, record)| record.completion.borrow().is_some())
            .map(|(execution_ref, _)| execution_ref.clone())
            .take(sessions.len() - MAX_SESSION_RECORDS + 1)
            .collect::<Vec<_>>();
        for execution_ref in remove {
            sessions.remove(&execution_ref);
        }
        if sessions.len() >= MAX_SESSION_RECORDS {
            Err(unavailable())
        } else {
            Ok(())
        }
    }

    pub(super) fn session_status(
        &self,
        request: SessionRequest,
    ) -> Result<SessionStatus, OperationError> {
        let sessions = lock(&self.sessions);
        let record = sessions.get(&request.execution_ref).ok_or_else(not_found)?;
        Ok(status(&request.execution_ref, record))
    }

    pub(super) fn session_terminate(
        &self,
        request: SessionTerminateRequest,
    ) -> Result<SessionStatus, OperationError> {
        let mut sessions = lock(&self.sessions);
        let record = sessions
            .get_mut(&request.execution_ref)
            .ok_or_else(not_found)?;
        if record.completion.borrow().is_none() {
            self.audit
                .append(AuditEvent {
                    audit_ref: &record.audit_ref,
                    execution_ref: &request.execution_ref,
                    operation_ref: &record.operation_ref,
                    connection_ref: &record.connection_ref,
                    tenant_id: &self.config.owner.tenant_id,
                    agent_id: &self.config.owner.agent_id,
                    action: "termination_requested",
                    termination: Some(requested_termination(request.reason)),
                })
                .map_err(|_| unavailable())?;
            if record.control.terminate(termination_reason(request.reason)) {
                record.terminating = true;
            }
        }
        Ok(status(&request.execution_ref, record))
    }

    /// Send a signal into a live call.
    ///
    /// Cloned out of the registry before awaiting, because the registry is behind a
    /// `std::sync::Mutex` and sending a tone takes real time -- holding that lock across the await
    /// would block every other session operation for the length of the keypress, and cannot be
    /// held across an await at all.
    pub(super) async fn session_signal(
        &self,
        request: SessionSignalRequest,
    ) -> Result<SessionStatus, OperationError> {
        let session = {
            let sessions = lock(&self.sessions);
            let record = sessions.get(&request.execution_ref).ok_or_else(not_found)?;
            if record.completion.borrow().is_some() {
                // The call already ended. Refused rather than accepted-and-dropped, because a
                // caller who thinks a digit landed will wait for a response to it.
                return Err(OperationError::new(
                    OperationErrorCode::NotFound,
                    "the session has already terminated",
                    false,
                ));
            }
            record.session.clone().ok_or_else(|| {
                OperationError::new(
                    OperationErrorCode::Unavailable,
                    "this session binding cannot send signals",
                    false,
                )
            })?
        };

        session
            .send_signal(request.signal.clone())
            .await
            .map_err(|error| match error {
                domain::voice::VoiceError::InvalidSignal => OperationError::new(
                    OperationErrorCode::InvalidInput,
                    "the signal is outside the admitted grammar",
                    false,
                ),
                domain::voice::VoiceError::Terminated => OperationError::new(
                    OperationErrorCode::NotFound,
                    "the session has already terminated",
                    false,
                ),
                _ => OperationError::new(
                    OperationErrorCode::Unavailable,
                    "the session refused the signal",
                    true,
                ),
            })?;

        // Audited after the fact, and only on success: a keypress that reached the far end is an
        // outward effect on someone else's system, and the record says it happened rather than
        // that it was attempted.
        let mut sessions = lock(&self.sessions);
        let record = sessions
            .get_mut(&request.execution_ref)
            .ok_or_else(not_found)?;
        self.audit
            .append(AuditEvent {
                audit_ref: &record.audit_ref,
                execution_ref: &request.execution_ref,
                operation_ref: &record.operation_ref,
                connection_ref: &record.connection_ref,
                tenant_id: &self.config.owner.tenant_id,
                agent_id: &self.config.owner.agent_id,
                action: "signal_sent",
                termination: None,
            })
            .map_err(|_| unavailable())?;
        Ok(status(&request.execution_ref, record))
    }
}

pub(super) fn status(execution_ref: &str, record: &SessionRecord) -> SessionStatus {
    let termination = *record.completion.borrow();
    let state = if termination.is_some() {
        SessionState::Terminated
    } else if record.terminating {
        SessionState::Terminating
    } else {
        SessionState::Established
    };
    SessionStatus {
        execution_ref: execution_ref.to_owned(),
        operation_ref: record.operation_ref.clone(),
        connection_ref: record.connection_ref.clone(),
        state,
        termination,
        connector_audit_ref: record.audit_ref.clone(),
    }
}

pub(super) fn termination_reason(reason: RequestedSessionTermination) -> TerminationReason {
    match reason {
        RequestedSessionTermination::Completed => TerminationReason::Completed,
        RequestedSessionTermination::Cancelled => TerminationReason::Cancelled,
        RequestedSessionTermination::Revoked => TerminationReason::AuthorityRevoked,
    }
}

pub(super) fn requested_termination(reason: RequestedSessionTermination) -> SessionTermination {
    match reason {
        RequestedSessionTermination::Completed => SessionTermination::Completed,
        RequestedSessionTermination::Cancelled => SessionTermination::Cancelled,
        RequestedSessionTermination::Revoked => SessionTermination::Revoked,
    }
}

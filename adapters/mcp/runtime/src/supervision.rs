//! Serialized state reduction behind the trusted local supervisor.
//!
//! The caller supplies decisions derived from current authority, a trusted clock,
//! and actual cleanup accounting. This module does not admit peer-supplied
//! decisions, authenticate leases, enforce clocks or prove an I/O cutoff. Those
//! checks belong at the supervisor's controlled data boundary, before reduction.
//! Every model value and transition below comes from the shared ESS projection.
use obligations::*;
use session_model::obligation::UnmetObligation;
pub use session_model::{primitives::Timestamp, sessions::*};
use std::collections::BTreeMap;

type Behavior<T> = Result<T, UnmetObligation>;

pub struct Supervision<C, I> {
    sessions: BTreeMap<String, SessionSnapshot>,
    clock: C,
    ids: I,
}
impl<C: FnMut() -> Timestamp, I: FnMut() -> SessionId> Supervision<C, I> {
    /// Ports are local trusted services, never callbacks selected by a peer.
    /// The enclosing runtime bounds admission and the lifetime of this store.
    pub fn new(clock: C, ids: I) -> Self {
        Self {
            sessions: BTreeMap::new(),
            clock,
            ids,
        }
    }
    pub fn snapshot(&self, id: &SessionId) -> Option<SessionSnapshot> {
        self.sessions.get(&id.0).cloned()
    }
    fn save(&mut self, session: SessionSnapshot) {
        self.sessions
            .insert(session.data.session_id.0.clone(), session);
    }
    fn terminal(&mut self, session: &mut SessionSnapshot, reason: TerminalReason) {
        if session.data.terminal.is_none() {
            session.data.terminal = Some(TerminalFact {
                reason,
                by: TerminalActor::Host,
                accepted_at: (self.clock)(),
            });
        }
        session.data.data_lease = None;
    }
    fn denial(&mut self, mut session: SessionSnapshot, decision: GateDecision, before_ready: bool) {
        let reason = match decision {
            GateDecision::Expired => TerminalReason::LeaseExpired,
            GateDecision::Revoked => TerminalReason::Revoked,
            GateDecision::Unverified if before_ready => TerminalReason::Rejected,
            GateDecision::Unverified => TerminalReason::Error,
            GateDecision::Allow => unreachable!("allow never enters a denial transition"),
        };
        self.terminal(&mut session, reason);
        self.save(session);
    }
}

macro_rules! load {
    ($store:expr, $id:expr, $outcome:ident) => {{
        let Some(snapshot) = $store.snapshot(&$id) else {
            return Ok($outcome::WrongStateUnknownInstance);
        };
        snapshot.refine()
    }};
}
macro_rules! wrong {
    ($session:expr, $outcome:ident) => {
        return Ok($outcome::WrongState {
            error: StateConflict {
                state: $session.state(),
            },
        })
    };
}

impl<C: FnMut() -> Timestamp, I: FnMut() -> SessionId> OfferSessionBehavior for Supervision<C, I> {
    fn offer_session(&mut self, input: OfferSession) -> Behavior<OfferSessionOutcome> {
        let id = (self.ids)();
        // A faulty identity source cannot overwrite an admitted session. It has
        // failed this generated behavior's infrastructure obligation.
        if id.0.is_empty() || self.sessions.contains_key(&id.0) {
            return Err(UnmetObligation {
                capability: "command behaviour",
                source: "connectors.sessions.OfferSession",
            });
        }
        let session = Session::new(SessionData {
            session_id: id.clone(),
            binding: input.binding,
            profile: input.profile,
            placement: input.placement,
            streams: input.streams,
            data_lease: None,
            terminal: None,
        });
        self.save(AnySession::Offered(session).snapshot());
        Ok(OfferSessionOutcome::Offered {
            session_offered: SessionOffered { session_id: id },
        })
    }
}
impl<C: FnMut() -> Timestamp, I: FnMut() -> SessionId> AcceptOfferBehavior for Supervision<C, I> {
    fn accept_offer(&mut self, input: AcceptOffer) -> Behavior<AcceptOfferOutcome> {
        use AcceptOfferOutcome as O;
        match load!(self, input.session_id, O) {
            AnySession::Offered(session) => {
                self.save(AnySession::Establishing(session.accept()).snapshot())
            }
            other => wrong!(other, O),
        }
        Ok(O::Accepted {
            offer_accepted: OfferAccepted {
                session_id: input.session_id,
            },
        })
    }
}
impl<C: FnMut() -> Timestamp, I: FnMut() -> SessionId> EstablishReadyBehavior
    for Supervision<C, I>
{
    fn establish_ready(&mut self, input: EstablishReady) -> Behavior<EstablishReadyOutcome> {
        use EstablishReadyOutcome as O;
        let session = match load!(self, input.session_id, O) {
            AnySession::Establishing(session) => session,
            other => wrong!(other, O),
        };
        if input.decision == GateDecision::Allow {
            let mut ready = AnySession::Ready(session.ready()).snapshot();
            ready.data.data_lease = Some(input.lease.clone());
            self.save(ready);
            Ok(O::Ready {
                session_ready: SessionReady {
                    session_id: input.session_id,
                    lease: input.lease,
                },
            })
        } else {
            self.denial(
                AnySession::Closing(session.deny_ready()).snapshot(),
                input.decision,
                true,
            );
            Ok(O::AuthorityTerminated {
                data_authority_terminated: DataAuthorityTerminated {
                    session_id: input.session_id,
                    decision: input.decision,
                },
            })
        }
    }
}
impl<C: FnMut() -> Timestamp, I: FnMut() -> SessionId> PermitDataBehavior for Supervision<C, I> {
    fn permit_data(&mut self, input: PermitData) -> Behavior<PermitDataOutcome> {
        use PermitDataOutcome as O;
        let session = match load!(self, input.session_id, O) {
            AnySession::Ready(session) => session,
            other => wrong!(other, O),
        };
        if input.decision == GateDecision::Allow {
            self.save(AnySession::Ready(session.permit_data()).snapshot());
            Ok(O::Permitted {
                data_permitted: DataPermitted {
                    session_id: input.session_id,
                    direction: input.direction,
                },
            })
        } else {
            self.denial(
                AnySession::Closing(session.deny_data()).snapshot(),
                input.decision,
                false,
            );
            Ok(O::AuthorityTerminated {
                data_authority_terminated: DataAuthorityTerminated {
                    session_id: input.session_id,
                    decision: input.decision,
                },
            })
        }
    }
}
impl<C: FnMut() -> Timestamp, I: FnMut() -> SessionId> RenewDataLeaseBehavior
    for Supervision<C, I>
{
    fn renew_data_lease(&mut self, input: RenewDataLease) -> Behavior<RenewDataLeaseOutcome> {
        use RenewDataLeaseOutcome as O;
        let session = match load!(self, input.session_id, O) {
            AnySession::Ready(session) => session,
            other => wrong!(other, O),
        };
        if input.decision == GateDecision::Allow {
            let mut ready = AnySession::Ready(session.renew()).snapshot();
            ready.data.data_lease = Some(input.lease.clone());
            self.save(ready);
            Ok(O::Renewed {
                data_lease_renewed: DataLeaseRenewed {
                    session_id: input.session_id,
                    lease: input.lease,
                },
            })
        } else {
            self.denial(
                AnySession::Closing(session.deny_renewal()).snapshot(),
                input.decision,
                false,
            );
            Ok(O::AuthorityTerminated {
                data_authority_terminated: DataAuthorityTerminated {
                    session_id: input.session_id,
                    decision: input.decision,
                },
            })
        }
    }
}
impl<C: FnMut() -> Timestamp, I: FnMut() -> SessionId> BeginCloseBehavior for Supervision<C, I> {
    fn begin_close(&mut self, input: BeginClose) -> Behavior<BeginCloseOutcome> {
        use BeginCloseOutcome as O;
        let session = match load!(self, input.session_id, O) {
            AnySession::Offered(session) => session.begin_close(),
            AnySession::Establishing(session) => session.begin_close(),
            AnySession::Ready(session) => session.begin_close(),
            other => wrong!(other, O),
        };
        let mut closing = AnySession::Closing(session).snapshot();
        closing.data.terminal.get_or_insert(input.terminal.clone());
        closing.data.data_lease = None;
        self.save(closing);
        Ok(O::Closing {
            closing_begun: ClosingBegun {
                session_id: input.session_id,
                terminal: input.terminal,
                cutoff_due_at: input.cutoff_due_at,
                teardown_due_at: input.teardown_due_at,
            },
        })
    }
}
impl<C: FnMut() -> Timestamp, I: FnMut() -> SessionId> FinishTeardownBehavior
    for Supervision<C, I>
{
    fn finish_teardown(&mut self, input: FinishTeardown) -> Behavior<FinishTeardownOutcome> {
        use FinishTeardownOutcome as O;
        let session = match load!(self, input.session_id, O) {
            AnySession::Closing(session) => session,
            other => wrong!(other, O),
        };
        if input.decision == CleanupDecision::Released {
            self.save(AnySession::Closed(session.close()).snapshot());
            Ok(O::Closed {
                local_resources_released: LocalResourcesReleased {
                    session_id: input.session_id,
                    peer_shutdown: input.peer_shutdown,
                },
            })
        } else {
            self.save(AnySession::Lost(session.cleanup_lost()).snapshot());
            Ok(O::Lost {
                local_cleanup_unaccounted: LocalCleanupUnaccounted {
                    session_id: input.session_id,
                },
            })
        }
    }
}
impl<C: FnMut() -> Timestamp, I: FnMut() -> SessionId> LoseContinuityBehavior
    for Supervision<C, I>
{
    fn lose_continuity(&mut self, input: LoseContinuity) -> Behavior<LoseContinuityOutcome> {
        use LoseContinuityOutcome as O;
        let session = match load!(self, input.session_id, O) {
            AnySession::Offered(session) => session.continuity_lost(),
            AnySession::Establishing(session) => session.continuity_lost(),
            AnySession::Ready(session) => session.continuity_lost(),
            AnySession::Closing(session) => session.continuity_lost(),
            other => wrong!(other, O),
        };
        let mut lost = AnySession::Lost(session).snapshot();
        self.terminal(&mut lost, TerminalReason::TransportLost);
        self.save(lost);
        Ok(O::Lost {
            continuity_lost: ContinuityLost {
                session_id: input.session_id,
            },
        })
    }
}
impl<C: FnMut() -> Timestamp, I: FnMut() -> SessionId> SessionStatesQuery for Supervision<C, I> {
    fn session_states(&self) -> Behavior<Vec<SessionStates>> {
        Ok(self
            .sessions
            .values()
            .map(|s| SessionStates {
                session_id: s.data.session_id.clone(),
                state: s.state,
                terminal: s.data.terminal.clone(),
            })
            .collect())
    }
}

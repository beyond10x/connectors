use connectors_mcp::supervision;
use supervision::{obligations::*, *};

fn supervisor() -> Supervision<impl FnMut() -> Timestamp, impl FnMut() -> SessionId> {
    let mut counter = 0;
    Supervision::new(
        || Timestamp("2026-10-03T12:00:01Z".into()),
        move || {
            counter += 1;
            SessionId(format!("local-{counter}"))
        },
    )
}
fn offer<C: FnMut() -> Timestamp, I: FnMut() -> SessionId>(s: &mut Supervision<C, I>) -> SessionId {
    let OfferSessionOutcome::Offered { session_offered } = s
        .offer_session(OfferSession {
            binding: Binding {
                instance_ref: "instance".into(),
                connection_ref: "owner-coordinate".into(),
                admitted_revision: "revision".into(),
                authority_ref: "owner-authority".into(),
            },
            profile: Profile::InboundOffer,
            placement: Placement::Local,
            streams: vec!["stdin".into(), "stdout".into()],
        })
        .unwrap();
    session_offered.session_id
}
fn lease(sequence: i64) -> DataLease {
    DataLease {
        issued_at: Timestamp("2026-10-03T12:00:00Z".into()),
        effective_expiry: Timestamp("2026-10-03T12:00:02Z".into()),
        sequence,
    }
}
fn ready<C: FnMut() -> Timestamp, I: FnMut() -> SessionId>(s: &mut Supervision<C, I>) -> SessionId {
    let id = offer(s);
    assert!(matches!(
        s.accept_offer(AcceptOffer {
            session_id: id.clone()
        })
        .unwrap(),
        AcceptOfferOutcome::Accepted { .. }
    ));
    assert!(matches!(
        s.establish_ready(EstablishReady {
            session_id: id.clone(),
            decision: GateDecision::Allow,
            lease: lease(1)
        })
        .unwrap(),
        EstablishReadyOutcome::Ready { .. }
    ));
    id
}
fn close(id: &SessionId) -> BeginClose {
    BeginClose {
        session_id: id.clone(),
        terminal: TerminalFact {
            reason: TerminalReason::Cancelled,
            by: TerminalActor::Application,
            accepted_at: Timestamp("2026-10-03T12:00:00.500Z".into()),
        },
        cutoff_due_at: Timestamp("2026-10-03T12:00:02Z".into()),
        teardown_due_at: Timestamp("2026-10-03T12:00:05.500Z".into()),
    }
}

#[test]
fn fields_survive_generated_moves_and_renewal_replaces_only_the_lease() {
    let mut s = supervisor();
    let id = ready(&mut s);
    let before = s.snapshot(&id).unwrap();
    assert_eq!(before.data.binding.connection_ref, "owner-coordinate");
    assert_eq!(before.data.profile, Profile::InboundOffer);
    assert_eq!(before.data.placement, Placement::Local);
    assert_eq!(before.data.streams, ["stdin", "stdout"]);
    assert_eq!(before.data.data_lease, Some(lease(1)));
    assert!(before.data.terminal.is_none());
    let output = s
        .renew_data_lease(RenewDataLease {
            session_id: id.clone(),
            decision: GateDecision::Allow,
            lease: lease(2),
        })
        .unwrap();
    assert_eq!(
        output,
        RenewDataLeaseOutcome::Renewed {
            data_lease_renewed: DataLeaseRenewed {
                session_id: id.clone(),
                lease: lease(2)
            }
        }
    );
    let mut expected = before;
    expected.data.data_lease = Some(lease(2));
    assert_eq!(s.snapshot(&id).unwrap(), expected);
    let before = s.snapshot(&id);
    assert!(matches!(
        s.permit_data(PermitData {
            session_id: id.clone(),
            decision: GateDecision::Allow,
            direction: "stdout".into()
        })
        .unwrap(),
        PermitDataOutcome::Permitted { .. }
    ));
    assert_eq!(s.snapshot(&id), before);
}

#[test]
fn denial_atomically_closes_clears_authority_and_cannot_be_renewed() {
    for (decision, reason) in [
        (GateDecision::Expired, TerminalReason::LeaseExpired),
        (GateDecision::Revoked, TerminalReason::Revoked),
        (GateDecision::Unverified, TerminalReason::Error),
    ] {
        let mut s = supervisor();
        let id = ready(&mut s);
        assert!(matches!(
            s.permit_data(PermitData {
                session_id: id.clone(),
                decision,
                direction: "stdin".into()
            })
            .unwrap(),
            PermitDataOutcome::AuthorityTerminated { .. }
        ));
        let terminal = s.snapshot(&id).unwrap();
        assert_eq!(terminal.state, SessionState::Closing);
        assert!(terminal.data.data_lease.is_none());
        assert_eq!(terminal.data.terminal.as_ref().unwrap().reason, reason);
        assert_eq!(
            terminal.data.terminal.as_ref().unwrap().by,
            TerminalActor::Host
        );
        assert_eq!(
            terminal.data.terminal.as_ref().unwrap().accepted_at.0,
            "2026-10-03T12:00:01Z"
        );
        assert!(matches!(
            s.renew_data_lease(RenewDataLease {
                session_id: id.clone(),
                decision: GateDecision::Allow,
                lease: lease(3)
            })
            .unwrap(),
            RenewDataLeaseOutcome::WrongState { .. }
        ));
        assert!(matches!(
            s.begin_close(close(&id)).unwrap(),
            BeginCloseOutcome::WrongState { .. }
        ));
        assert_eq!(s.snapshot(&id).unwrap(), terminal);
    }
}

#[test]
fn readiness_and_renewal_denials_record_their_exact_terminal_meaning() {
    for before_ready in [true, false] {
        for decision in [
            GateDecision::Expired,
            GateDecision::Revoked,
            GateDecision::Unverified,
        ] {
            let mut s = supervisor();
            let id = if before_ready {
                let id = offer(&mut s);
                s.accept_offer(AcceptOffer {
                    session_id: id.clone(),
                })
                .unwrap();
                s.establish_ready(EstablishReady {
                    session_id: id.clone(),
                    decision,
                    lease: lease(1),
                })
                .unwrap();
                id
            } else {
                let id = ready(&mut s);
                s.renew_data_lease(RenewDataLease {
                    session_id: id.clone(),
                    decision,
                    lease: lease(2),
                })
                .unwrap();
                id
            };
            let expected = match decision {
                GateDecision::Expired => TerminalReason::LeaseExpired,
                GateDecision::Revoked => TerminalReason::Revoked,
                GateDecision::Unverified if before_ready => TerminalReason::Rejected,
                _ => TerminalReason::Error,
            };
            let state = s.snapshot(&id).unwrap();
            assert_eq!(state.state, SessionState::Closing);
            assert_eq!(state.data.terminal.unwrap().reason, expected);
            assert!(state.data.data_lease.is_none());
        }
    }
}

#[test]
fn continuity_loss_preserves_the_first_terminal_and_never_reopens() {
    let mut s = supervisor();
    let id = ready(&mut s);
    let command = close(&id);
    s.begin_close(command.clone()).unwrap();
    s.lose_continuity(LoseContinuity {
        session_id: id.clone(),
    })
    .unwrap();
    let lost = s.snapshot(&id).unwrap();
    assert_eq!(lost.state, SessionState::Lost);
    assert_eq!(lost.data.terminal, Some(command.terminal));
    assert!(lost.data.data_lease.is_none());
    assert!(matches!(
        s.accept_offer(AcceptOffer {
            session_id: id.clone()
        })
        .unwrap(),
        AcceptOfferOutcome::WrongState { .. }
    ));
    assert!(matches!(
        s.lose_continuity(LoseContinuity {
            session_id: id.clone()
        })
        .unwrap(),
        LoseContinuityOutcome::WrongState { .. }
    ));
    assert_eq!(s.snapshot(&id).unwrap(), lost);
}

#[test]
fn local_release_does_not_require_a_peer_ack_and_unaccounted_is_lost() {
    for decision in [CleanupDecision::Released, CleanupDecision::Unaccounted] {
        let mut s = supervisor();
        let id = ready(&mut s);
        let terminal = close(&id);
        s.begin_close(terminal.clone()).unwrap();
        let result = s
            .finish_teardown(FinishTeardown {
                session_id: id.clone(),
                decision,
                peer_shutdown: PeerShutdown::Unconfirmed,
            })
            .unwrap();
        let expected = match decision {
            CleanupDecision::Released => {
                assert!(matches!(
                    result,
                    FinishTeardownOutcome::Closed {
                        local_resources_released: LocalResourcesReleased {
                            peer_shutdown: PeerShutdown::Unconfirmed,
                            ..
                        }
                    }
                ));
                SessionState::Closed
            }
            CleanupDecision::Unaccounted => {
                assert!(matches!(result, FinishTeardownOutcome::Lost { .. }));
                SessionState::Lost
            }
        };
        let state = s.snapshot(&id).unwrap();
        assert_eq!(state.state, expected);
        assert_eq!(state.data.terminal, Some(terminal.terminal));
        assert!(state.data.data_lease.is_none());
        assert_eq!(
            s.session_states().unwrap(),
            vec![SessionStates {
                session_id: id,
                state: expected,
                terminal: state.data.terminal
            }]
        );
    }
}

#[test]
fn unknown_and_wrong_state_commands_do_not_mutate_other_sessions() {
    let mut s = supervisor();
    let id = offer(&mut s);
    let before = s.snapshot(&id);
    let absent = SessionId("missing".into());
    assert!(matches!(
        s.permit_data(PermitData {
            session_id: id.clone(),
            decision: GateDecision::Allow,
            direction: "stdout".into()
        })
        .unwrap(),
        PermitDataOutcome::WrongState {
            error: StateConflict {
                state: SessionState::Offered
            }
        }
    ));
    assert!(matches!(
        s.accept_offer(AcceptOffer {
            session_id: absent.clone()
        })
        .unwrap(),
        AcceptOfferOutcome::WrongStateUnknownInstance
    ));
    assert!(matches!(
        s.begin_close(close(&absent)).unwrap(),
        BeginCloseOutcome::WrongStateUnknownInstance
    ));
    assert!(matches!(
        s.lose_continuity(LoseContinuity {
            session_id: absent.clone()
        })
        .unwrap(),
        LoseContinuityOutcome::WrongStateUnknownInstance
    ));
    assert!(matches!(
        s.establish_ready(EstablishReady {
            session_id: absent.clone(),
            decision: GateDecision::Allow,
            lease: lease(1)
        })
        .unwrap(),
        EstablishReadyOutcome::WrongStateUnknownInstance
    ));
    assert!(matches!(
        s.permit_data(PermitData {
            session_id: absent.clone(),
            decision: GateDecision::Allow,
            direction: "stdout".into()
        })
        .unwrap(),
        PermitDataOutcome::WrongStateUnknownInstance
    ));
    assert!(matches!(
        s.renew_data_lease(RenewDataLease {
            session_id: absent.clone(),
            decision: GateDecision::Allow,
            lease: lease(1)
        })
        .unwrap(),
        RenewDataLeaseOutcome::WrongStateUnknownInstance
    ));
    assert!(matches!(
        s.finish_teardown(FinishTeardown {
            session_id: absent,
            decision: CleanupDecision::Released,
            peer_shutdown: PeerShutdown::Unconfirmed
        })
        .unwrap(),
        FinishTeardownOutcome::WrongStateUnknownInstance
    ));
    assert_eq!(s.snapshot(&id), before);
}

#[test]
fn loss_before_a_terminal_records_transport_loss_in_each_live_state() {
    for phase in 0..3 {
        let mut s = supervisor();
        let id = if phase == 2 {
            ready(&mut s)
        } else {
            let id = offer(&mut s);
            if phase == 1 {
                s.accept_offer(AcceptOffer {
                    session_id: id.clone(),
                })
                .unwrap();
            }
            id
        };
        s.lose_continuity(LoseContinuity {
            session_id: id.clone(),
        })
        .unwrap();
        let state = s.snapshot(&id).unwrap();
        assert_eq!(state.state, SessionState::Lost);
        assert_eq!(
            state.data.terminal.unwrap(),
            TerminalFact {
                reason: TerminalReason::TransportLost,
                by: TerminalActor::Host,
                accepted_at: Timestamp("2026-10-03T12:00:01Z".into()),
            }
        );
        assert!(state.data.data_lease.is_none());
    }
}

#[test]
fn faulty_identity_port_never_overwrites_an_admitted_session() {
    let mut s = Supervision::new(
        || Timestamp("2026-10-03T12:00:01Z".into()),
        || SessionId("same".into()),
    );
    let id = ready(&mut s);
    let original = s.snapshot(&id);
    let result = s.offer_session(OfferSession {
        binding: Binding {
            instance_ref: "other".into(),
            connection_ref: "other".into(),
            admitted_revision: "other".into(),
            authority_ref: "other".into(),
        },
        profile: Profile::Outbound,
        placement: Placement::Relay,
        streams: vec![],
    });
    assert!(result.is_err());
    assert_eq!(s.snapshot(&id), original);
    assert_eq!(s.session_states().unwrap().len(), 1);
}

#[test]
fn bounded_command_sequences_cannot_replace_or_revive_a_terminal() {
    // Enumerate every sequence of five commands from the nine selected actions.
    // This checks an invariant, not a second implementation of the transition table.
    for script in 0..9_u32.pow(5) {
        let mut s = supervisor();
        let id = offer(&mut s);
        let mut remaining = script;
        let mut first = None;
        let mut ended = None;
        for _ in 0..5 {
            let action = remaining % 9;
            remaining /= 9;
            match action {
                0 => {
                    s.accept_offer(AcceptOffer {
                        session_id: id.clone(),
                    })
                    .unwrap();
                }
                1 => {
                    s.establish_ready(EstablishReady {
                        session_id: id.clone(),
                        decision: GateDecision::Allow,
                        lease: lease(1),
                    })
                    .unwrap();
                }
                2 => {
                    s.permit_data(PermitData {
                        session_id: id.clone(),
                        decision: GateDecision::Allow,
                        direction: "stdout".into(),
                    })
                    .unwrap();
                }
                3 => {
                    s.permit_data(PermitData {
                        session_id: id.clone(),
                        decision: GateDecision::Expired,
                        direction: "stdin".into(),
                    })
                    .unwrap();
                }
                4 => {
                    s.renew_data_lease(RenewDataLease {
                        session_id: id.clone(),
                        decision: GateDecision::Allow,
                        lease: lease(2),
                    })
                    .unwrap();
                }
                5 => {
                    s.renew_data_lease(RenewDataLease {
                        session_id: id.clone(),
                        decision: GateDecision::Revoked,
                        lease: lease(2),
                    })
                    .unwrap();
                }
                6 => {
                    s.begin_close(close(&id)).unwrap();
                }
                7 => {
                    s.finish_teardown(FinishTeardown {
                        session_id: id.clone(),
                        decision: CleanupDecision::Released,
                        peer_shutdown: PeerShutdown::Unconfirmed,
                    })
                    .unwrap();
                }
                8 => {
                    s.lose_continuity(LoseContinuity {
                        session_id: id.clone(),
                    })
                    .unwrap();
                }
                _ => unreachable!(),
            }
            let current = s.snapshot(&id).unwrap();
            if let Some(first) = &first {
                assert_eq!(
                    current.data.terminal.as_ref(),
                    Some(first),
                    "script {script}"
                );
                assert!(current.data.data_lease.is_none(), "script {script}");
                assert!(
                    matches!(
                        current.state,
                        SessionState::Closing | SessionState::Closed | SessionState::Lost
                    ),
                    "script {script}"
                );
            } else {
                first = current.data.terminal.clone();
            }
            if let Some(ended) = &ended {
                assert_eq!(&current, ended, "script {script}");
            } else if matches!(current.state, SessionState::Closed | SessionState::Lost) {
                ended = Some(current);
            }
        }
    }
}

// generated from connectors v1
// model digest 0f1b92b6b86a69778081be2aca23de79337294bbb219d2bd7f19aad1f6523d26
// contract digest b9b32b755ebf5ba2e2d690e05122218781ab050a55aa96d08f5ab2d6164aff5e
// do not edit: regenerate with `ess synthesize`

//! sessions — `connectors.sessions`.
//!
//! Proposed supervising session lifecycle; timed enforcement remains a binding obligation.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// Binding — `connectors.sessions.Binding`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// `instance_ref` — `String`.
    pub instance_ref: String,
    /// `connection_ref` — `String`.
    pub connection_ref: String,
    /// `admitted_revision` — `String`.
    pub admitted_revision: String,
    /// `authority_ref` — `String`.
    pub authority_ref: String,
}

/// CleanupDecision — `connectors.sessions.CleanupDecision`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupDecision {
    /// `released`.
    Released,
    /// `unaccounted`.
    Unaccounted,
}

/// DataLease — `connectors.sessions.DataLease`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLease {
    /// `issued_at` — `Timestamp`.
    pub issued_at: crate::primitives::Timestamp,
    /// `effective_expiry` — `Timestamp`.
    pub effective_expiry: crate::primitives::Timestamp,
    /// `sequence` — `Integer`.
    pub sequence: i64,
}

/// GateDecision — `connectors.sessions.GateDecision`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDecision {
    /// `allow`.
    Allow,
    /// `expired`.
    Expired,
    /// `revoked`.
    Revoked,
    /// `unverified`.
    Unverified,
}

/// PeerShutdown — `connectors.sessions.PeerShutdown`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerShutdown {
    /// `confirmed`.
    Confirmed,
    /// `unconfirmed`.
    Unconfirmed,
}

/// Placement — `connectors.sessions.Placement`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// `local`.
    Local,
    /// `direct`.
    Direct,
    /// `relay`.
    Relay,
}

/// Profile — `connectors.sessions.Profile`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// `outbound`.
    Outbound,
    /// `inbound_offer`.
    InboundOffer,
}

/// The states of `connectors.sessions.Session`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Session<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// `Closed`.
    Closed,
    /// `Closing`.
    Closing,
    /// `Establishing`.
    Establishing,
    /// `Lost`.
    Lost,
    /// `Offered`.
    Offered,
    /// `Ready`.
    Ready,
}

/// SessionId — `connectors.sessions.SessionId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(pub String);

/// TerminalActor — `connectors.sessions.TerminalActor`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalActor {
    /// `adapter`.
    Adapter,
    /// `application`.
    Application,
    /// `host`.
    Host,
}

/// TerminalFact — `connectors.sessions.TerminalFact`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalFact {
    /// `reason` — `connectors.sessions.TerminalReason`.
    pub reason: TerminalReason,
    /// `by` — `connectors.sessions.TerminalActor`.
    pub by: TerminalActor,
    /// `accepted_at` — `Timestamp`.
    pub accepted_at: crate::primitives::Timestamp,
}

/// TerminalReason — `connectors.sessions.TerminalReason`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalReason {
    /// `remote_hangup`.
    RemoteHangup,
    /// `local_close`.
    LocalClose,
    /// `cancelled`.
    Cancelled,
    /// `rejected`.
    Rejected,
    /// `expired`.
    Expired,
    /// `revoked`.
    Revoked,
    /// `lease_expired`.
    LeaseExpired,
    /// `media_overload`.
    MediaOverload,
    /// `media_incompatible`.
    MediaIncompatible,
    /// `transport_lost`.
    TransportLost,
    /// `error`.
    Error,
}

/// What Session — `connectors.sessions.Session` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Session<S>`], and at a boundary by [`SessionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionData {
    /// The identity: `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
    /// `binding` — `connectors.sessions.Binding`.
    pub binding: Binding,
    /// `profile` — `connectors.sessions.Profile`.
    pub profile: Profile,
    /// `placement` — `connectors.sessions.Placement`.
    pub placement: Placement,
    /// `streams` — `List<String>`.
    pub streams: Vec<String>,
    /// `data_lease` — `Optional<connectors.sessions.DataLease>`.
    pub data_lease: Option<DataLease>,
    /// `terminal` — `Optional<connectors.sessions.TerminalFact>`.
    pub terminal: Option<TerminalFact>,
}

/// The states of `connectors.sessions.Session`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](session_state::Marker), so [`Session<S>`](Session) can only ever rest in a real state.
pub mod session_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Closed {}
        impl Sealed for super::Closing {}
        impl Sealed for super::Establishing {}
        impl Sealed for super::Lost {}
        impl Sealed for super::Offered {}
        impl Sealed for super::Ready {}
    }

    /// A declared state of `Session`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SessionState;
    }

    /// `Closed`. Terminal: an instance may rest here forever.
    pub struct Closed;

    impl Marker for Closed {
        const STATE: super::SessionState = super::SessionState::Closed;
    }

    /// `Closing`.
    pub struct Closing;

    impl Marker for Closing {
        const STATE: super::SessionState = super::SessionState::Closing;
    }

    /// `Establishing`.
    pub struct Establishing;

    impl Marker for Establishing {
        const STATE: super::SessionState = super::SessionState::Establishing;
    }

    /// `Lost`. Terminal: an instance may rest here forever.
    pub struct Lost;

    impl Marker for Lost {
        const STATE: super::SessionState = super::SessionState::Lost;
    }

    /// `Offered`. Where a new instance starts.
    pub struct Offered;

    impl Marker for Offered {
        const STATE: super::SessionState = super::SessionState::Offered;
    }

    /// `Ready`.
    pub struct Ready;

    impl Marker for Ready {
        const STATE: super::SessionState = super::SessionState::Ready;
    }
}

/// Session — `connectors.sessions.Session` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Offered`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SessionSnapshot`]
/// and [`SessionSnapshot::refine`].
pub struct Session<S: session_state::Marker> {
    data: SessionData,
    state: core::marker::PhantomData<S>,
}

impl<S: session_state::Marker> Session<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SessionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SessionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SessionData {
        self.data
    }
}

impl Session<session_state::Offered> {
    /// A new instance, resting in `Offered` — the only state the lifecycle starts one in.
    pub fn new(data: SessionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Session<session_state::Closing> {
    /// `close` — `Closing` → `Closed`. Taken by the `closed` outcome of `connectors.sessions.FinishTeardown`.
    pub fn close(self) -> Session<session_state::Closed> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `cleanup_lost` — `Closing` → `Lost`. Taken by the `lost` outcome of `connectors.sessions.FinishTeardown`.
    pub fn cleanup_lost(self) -> Session<session_state::Lost> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `continuity_lost` — `Closing` → `Lost`. Taken by the `lost` outcome of `connectors.sessions.LoseContinuity`.
    pub fn continuity_lost(self) -> Session<session_state::Lost> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Session<session_state::Establishing> {
    /// `ready` — `Establishing` → `Ready`. Taken by the `ready` outcome of `connectors.sessions.EstablishReady`.
    pub fn ready(self) -> Session<session_state::Ready> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `deny_ready` — `Establishing` → `Closing`. Taken by the `authority-terminated` outcome of `connectors.sessions.EstablishReady`.
    pub fn deny_ready(self) -> Session<session_state::Closing> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `begin_close` — `Establishing` → `Closing`. Taken by the `closing` outcome of `connectors.sessions.BeginClose`.
    pub fn begin_close(self) -> Session<session_state::Closing> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `continuity_lost` — `Establishing` → `Lost`. Taken by the `lost` outcome of `connectors.sessions.LoseContinuity`.
    pub fn continuity_lost(self) -> Session<session_state::Lost> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Session<session_state::Offered> {
    /// `accept` — `Offered` → `Establishing`. Taken by the `accepted` outcome of `connectors.sessions.AcceptOffer`.
    pub fn accept(self) -> Session<session_state::Establishing> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `begin_close` — `Offered` → `Closing`. Taken by the `closing` outcome of `connectors.sessions.BeginClose`.
    pub fn begin_close(self) -> Session<session_state::Closing> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `continuity_lost` — `Offered` → `Lost`. Taken by the `lost` outcome of `connectors.sessions.LoseContinuity`.
    pub fn continuity_lost(self) -> Session<session_state::Lost> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Session<session_state::Ready> {
    /// `permit_data` — `Ready` → `Ready`. Taken by the `permitted` outcome of `connectors.sessions.PermitData`.
    pub fn permit_data(self) -> Session<session_state::Ready> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `renew` — `Ready` → `Ready`. Taken by the `renewed` outcome of `connectors.sessions.RenewDataLease`.
    pub fn renew(self) -> Session<session_state::Ready> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `deny_data` — `Ready` → `Closing`. Taken by the `authority-terminated` outcome of `connectors.sessions.PermitData`.
    pub fn deny_data(self) -> Session<session_state::Closing> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `deny_renewal` — `Ready` → `Closing`. Taken by the `authority-terminated` outcome of `connectors.sessions.RenewDataLease`.
    pub fn deny_renewal(self) -> Session<session_state::Closing> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `begin_close` — `Ready` → `Closing`. Taken by the `closing` outcome of `connectors.sessions.BeginClose`.
    pub fn begin_close(self) -> Session<session_state::Closing> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `continuity_lost` — `Ready` → `Lost`. Taken by the `lost` outcome of `connectors.sessions.LoseContinuity`.
    pub fn continuity_lost(self) -> Session<session_state::Lost> {
        Session {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `connectors.sessions.Session` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SessionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SessionState,
    /// What it holds.
    pub data: SessionData,
}

/// An `Session` in whichever declared state it was found.
pub enum AnySession {
    /// Resting in `Closed`.
    Closed(Session<session_state::Closed>),
    /// Resting in `Closing`.
    Closing(Session<session_state::Closing>),
    /// Resting in `Establishing`.
    Establishing(Session<session_state::Establishing>),
    /// Resting in `Lost`.
    Lost(Session<session_state::Lost>),
    /// Resting in `Offered`.
    Offered(Session<session_state::Offered>),
    /// Resting in `Ready`.
    Ready(Session<session_state::Ready>),
}

impl SessionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SessionState` cannot spell one.
    pub fn refine(self) -> AnySession {
        match self.state {
            SessionState::Closed => AnySession::Closed(Session {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            SessionState::Closing => AnySession::Closing(Session {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            SessionState::Establishing => AnySession::Establishing(Session {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            SessionState::Lost => AnySession::Lost(Session {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            SessionState::Offered => AnySession::Offered(Session {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            SessionState::Ready => AnySession::Ready(Session {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySession {
    /// The state, as the runtime value.
    pub fn state(&self) -> SessionState {
        match self {
            Self::Closed(_) => SessionState::Closed,
            Self::Closing(_) => SessionState::Closing,
            Self::Establishing(_) => SessionState::Establishing,
            Self::Lost(_) => SessionState::Lost,
            Self::Offered(_) => SessionState::Offered,
            Self::Ready(_) => SessionState::Ready,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SessionSnapshot {
        match self {
            Self::Closed(instance) => SessionSnapshot {
                state: SessionState::Closed,
                data: instance.into_data(),
            },
            Self::Closing(instance) => SessionSnapshot {
                state: SessionState::Closing,
                data: instance.into_data(),
            },
            Self::Establishing(instance) => SessionSnapshot {
                state: SessionState::Establishing,
                data: instance.into_data(),
            },
            Self::Lost(instance) => SessionSnapshot {
                state: SessionState::Lost,
                data: instance.into_data(),
            },
            Self::Offered(instance) => SessionSnapshot {
                state: SessionState::Offered,
                data: instance.into_data(),
            },
            Self::Ready(instance) => SessionSnapshot {
                state: SessionState::Ready,
                data: instance.into_data(),
            },
        }
    }
}

/// AcceptOffer — the input of `connectors.sessions.AcceptOffer`.
///
/// Everything it can result in is [`AcceptOfferOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptOffer {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
}

/// Everything `connectors.sessions.AcceptOffer` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptOfferOutcome {
    /// `accepted` — otherwise.
    Accepted {
        /// The `connectors.sessions.OfferAccepted` this outcome publishes.
        offer_accepted: OfferAccepted,
    },
    /// `wrong-state` — from a state no declared move starts in.
    WrongState {
        /// Why it was refused: `connectors.sessions.StateConflict`.
        error: StateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// BeginClose — the input of `connectors.sessions.BeginClose`.
///
/// Everything it can result in is [`BeginCloseOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeginClose {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
    /// `terminal` — `connectors.sessions.TerminalFact`.
    pub terminal: TerminalFact,
    /// `cutoff_due_at` — `Timestamp`.
    pub cutoff_due_at: crate::primitives::Timestamp,
    /// `teardown_due_at` — `Timestamp`.
    pub teardown_due_at: crate::primitives::Timestamp,
}

/// Everything `connectors.sessions.BeginClose` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeginCloseOutcome {
    /// `closing` — otherwise.
    Closing {
        /// The `connectors.sessions.ClosingBegun` this outcome publishes.
        closing_begun: ClosingBegun,
    },
    /// `wrong-state` — from a state no declared move starts in.
    WrongState {
        /// Why it was refused: `connectors.sessions.StateConflict`.
        error: StateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// EstablishReady — the input of `connectors.sessions.EstablishReady`.
///
/// Everything it can result in is [`EstablishReadyOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstablishReady {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
    /// `decision` — `connectors.sessions.GateDecision`.
    pub decision: GateDecision,
    /// `lease` — `connectors.sessions.DataLease`.
    pub lease: DataLease,
}

/// Everything `connectors.sessions.EstablishReady` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstablishReadyOutcome {
    /// `ready` — when `decision == allow`.
    Ready {
        /// The `connectors.sessions.SessionReady` this outcome publishes.
        session_ready: SessionReady,
    },
    /// `authority-terminated` — otherwise.
    AuthorityTerminated {
        /// The `connectors.sessions.DataAuthorityTerminated` this outcome publishes.
        data_authority_terminated: DataAuthorityTerminated,
    },
    /// `wrong-state` — from a state no declared move starts in.
    WrongState {
        /// Why it was refused: `connectors.sessions.StateConflict`.
        error: StateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// FinishTeardown — the input of `connectors.sessions.FinishTeardown`.
///
/// Everything it can result in is [`FinishTeardownOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishTeardown {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
    /// `decision` — `connectors.sessions.CleanupDecision`.
    pub decision: CleanupDecision,
    /// `peer_shutdown` — `connectors.sessions.PeerShutdown`.
    pub peer_shutdown: PeerShutdown,
}

/// Everything `connectors.sessions.FinishTeardown` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishTeardownOutcome {
    /// `closed` — when `decision == released`.
    Closed {
        /// The `connectors.sessions.LocalResourcesReleased` this outcome publishes.
        local_resources_released: LocalResourcesReleased,
    },
    /// `lost` — otherwise.
    Lost {
        /// The `connectors.sessions.LocalCleanupUnaccounted` this outcome publishes.
        local_cleanup_unaccounted: LocalCleanupUnaccounted,
    },
    /// `wrong-state` — from a state no declared move starts in.
    WrongState {
        /// Why it was refused: `connectors.sessions.StateConflict`.
        error: StateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// LoseContinuity — the input of `connectors.sessions.LoseContinuity`.
///
/// Everything it can result in is [`LoseContinuityOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoseContinuity {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
}

/// Everything `connectors.sessions.LoseContinuity` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoseContinuityOutcome {
    /// `lost` — otherwise.
    Lost {
        /// The `connectors.sessions.ContinuityLost` this outcome publishes.
        continuity_lost: ContinuityLost,
    },
    /// `wrong-state` — from a state no declared move starts in.
    WrongState {
        /// Why it was refused: `connectors.sessions.StateConflict`.
        error: StateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// OfferSession — the input of `connectors.sessions.OfferSession`.
///
/// Everything it can result in is [`OfferSessionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferSession {
    /// `binding` — `connectors.sessions.Binding`.
    pub binding: Binding,
    /// `profile` — `connectors.sessions.Profile`.
    pub profile: Profile,
    /// `placement` — `connectors.sessions.Placement`.
    pub placement: Placement,
    /// `streams` — `List<String>`.
    pub streams: Vec<String>,
}

/// Everything `connectors.sessions.OfferSession` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfferSessionOutcome {
    /// `offered` — otherwise.
    Offered {
        /// The `connectors.sessions.SessionOffered` this outcome publishes.
        session_offered: SessionOffered,
    },
}

/// PermitData — the input of `connectors.sessions.PermitData`.
///
/// Everything it can result in is [`PermitDataOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermitData {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
    /// `decision` — `connectors.sessions.GateDecision`.
    pub decision: GateDecision,
    /// `direction` — `String`.
    pub direction: String,
}

/// Everything `connectors.sessions.PermitData` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermitDataOutcome {
    /// `permitted` — when `decision == allow`.
    Permitted {
        /// The `connectors.sessions.DataPermitted` this outcome publishes.
        data_permitted: DataPermitted,
    },
    /// `authority-terminated` — otherwise.
    AuthorityTerminated {
        /// The `connectors.sessions.DataAuthorityTerminated` this outcome publishes.
        data_authority_terminated: DataAuthorityTerminated,
    },
    /// `wrong-state` — from a state no declared move starts in.
    WrongState {
        /// Why it was refused: `connectors.sessions.StateConflict`.
        error: StateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// RenewDataLease — the input of `connectors.sessions.RenewDataLease`.
///
/// Everything it can result in is [`RenewDataLeaseOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenewDataLease {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
    /// `decision` — `connectors.sessions.GateDecision`.
    pub decision: GateDecision,
    /// `lease` — `connectors.sessions.DataLease`.
    pub lease: DataLease,
}

/// Everything `connectors.sessions.RenewDataLease` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenewDataLeaseOutcome {
    /// `renewed` — when `decision == allow`.
    Renewed {
        /// The `connectors.sessions.DataLeaseRenewed` this outcome publishes.
        data_lease_renewed: DataLeaseRenewed,
    },
    /// `authority-terminated` — otherwise.
    AuthorityTerminated {
        /// The `connectors.sessions.DataAuthorityTerminated` this outcome publishes.
        data_authority_terminated: DataAuthorityTerminated,
    },
    /// `wrong-state` — from a state no declared move starts in.
    WrongState {
        /// Why it was refused: `connectors.sessions.StateConflict`.
        error: StateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// ClosingBegun — the event `connectors.sessions.ClosingBegun`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosingBegun {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
    /// `terminal` — `connectors.sessions.TerminalFact`.
    pub terminal: TerminalFact,
    /// `cutoff_due_at` — `Timestamp`.
    pub cutoff_due_at: crate::primitives::Timestamp,
    /// `teardown_due_at` — `Timestamp`.
    pub teardown_due_at: crate::primitives::Timestamp,
}

/// ContinuityLost — the event `connectors.sessions.ContinuityLost`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuityLost {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
}

/// DataAuthorityTerminated — the event `connectors.sessions.DataAuthorityTerminated`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataAuthorityTerminated {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
    /// `decision` — `connectors.sessions.GateDecision`.
    pub decision: GateDecision,
}

/// DataLeaseRenewed — the event `connectors.sessions.DataLeaseRenewed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLeaseRenewed {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
    /// `lease` — `connectors.sessions.DataLease`.
    pub lease: DataLease,
}

/// DataPermitted — the event `connectors.sessions.DataPermitted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPermitted {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
    /// `direction` — `String`.
    pub direction: String,
}

/// LocalCleanupUnaccounted — the event `connectors.sessions.LocalCleanupUnaccounted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCleanupUnaccounted {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
}

/// LocalResourcesReleased — the event `connectors.sessions.LocalResourcesReleased`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalResourcesReleased {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
    /// `peer_shutdown` — `connectors.sessions.PeerShutdown`.
    pub peer_shutdown: PeerShutdown,
}

/// OfferAccepted — the event `connectors.sessions.OfferAccepted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferAccepted {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
}

/// SessionOffered — the event `connectors.sessions.SessionOffered`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionOffered {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
}

/// SessionReady — the event `connectors.sessions.SessionReady`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionReady {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
    /// `lease` — `connectors.sessions.DataLease`.
    pub lease: DataLease,
}

/// The declared error `connectors.sessions.StateConflict`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateConflict {
    /// `state` — `connectors.sessions.Session.State`.
    pub state: SessionState,
}

/// SessionStates — one row of the view `connectors.sessions.SessionStates`.
///
/// Projects `connectors.sessions.Session` at `read_your_writes` consistency.
/// Serving it is an implementation obligation — see the plan — because how a projection is kept
/// current is a storage decision the specification does not take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStates {
    /// `session_id` — `connectors.sessions.SessionId`.
    pub session_id: SessionId,
    /// `state` — `connectors.sessions.Session.State`.
    pub state: SessionState,
    /// `terminal` — `Optional<connectors.sessions.TerminalFact>`.
    pub terminal: Option<TerminalFact>,
}

/// What this bounded context owes its implementor, as typed seams.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every trait by refusing in the type system, so the workspace builds —
/// and says exactly what it cannot yet do — before a line is hand-written.
pub mod obligations {
    /// The behaviour `connectors.sessions.AcceptOffer` — an implementation obligation.
    ///
    /// Why it is not generated: the contract is declared; the algorithm is not.
    ///
    /// Contract: given `connectors.sessions.AcceptOffer` input, decide and enact exactly one outcome — `accepted` otherwise, takes `accept` of `connectors.sessions.Session`, emits `connectors.sessions.OfferAccepted`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields.
    pub trait AcceptOfferBehavior {
        /// Decides and enacts exactly one declared outcome of `connectors.sessions.AcceptOffer`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn accept_offer(&mut self, input: super::AcceptOffer) -> Result<super::AcceptOfferOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `connectors.sessions.BeginClose` — an implementation obligation.
    ///
    /// Why it is not generated: the contract is declared; the algorithm is not.
    ///
    /// Contract: given `connectors.sessions.BeginClose` input, decide and enact exactly one outcome — `closing` otherwise, takes `begin_close` of `connectors.sessions.Session`, emits `connectors.sessions.ClosingBegun`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields.
    pub trait BeginCloseBehavior {
        /// Decides and enacts exactly one declared outcome of `connectors.sessions.BeginClose`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn begin_close(&mut self, input: super::BeginClose) -> Result<super::BeginCloseOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `connectors.sessions.EstablishReady` — an implementation obligation.
    ///
    /// Why it is not generated: the contract is declared; the algorithm is not.
    ///
    /// Contract: given `connectors.sessions.EstablishReady` input, decide and enact exactly one outcome — `ready` when `decision == allow`, takes `ready` of `connectors.sessions.Session`, emits `connectors.sessions.SessionReady`; `authority-terminated` otherwise, takes `deny_ready` of `connectors.sessions.Session`, emits `connectors.sessions.DataAuthorityTerminated`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields.
    pub trait EstablishReadyBehavior {
        /// Decides and enacts exactly one declared outcome of `connectors.sessions.EstablishReady`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn establish_ready(&mut self, input: super::EstablishReady) -> Result<super::EstablishReadyOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `connectors.sessions.FinishTeardown` — an implementation obligation.
    ///
    /// Why it is not generated: the contract is declared; the algorithm is not.
    ///
    /// Contract: given `connectors.sessions.FinishTeardown` input, decide and enact exactly one outcome — `closed` when `decision == released`, takes `close` of `connectors.sessions.Session`, emits `connectors.sessions.LocalResourcesReleased`; `lost` otherwise, takes `cleanup_lost` of `connectors.sessions.Session`, emits `connectors.sessions.LocalCleanupUnaccounted`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields.
    pub trait FinishTeardownBehavior {
        /// Decides and enacts exactly one declared outcome of `connectors.sessions.FinishTeardown`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn finish_teardown(&mut self, input: super::FinishTeardown) -> Result<super::FinishTeardownOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `connectors.sessions.LoseContinuity` — an implementation obligation.
    ///
    /// Why it is not generated: the contract is declared; the algorithm is not.
    ///
    /// Contract: given `connectors.sessions.LoseContinuity` input, decide and enact exactly one outcome — `lost` otherwise, takes `continuity_lost` of `connectors.sessions.Session`, emits `connectors.sessions.ContinuityLost`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields.
    pub trait LoseContinuityBehavior {
        /// Decides and enacts exactly one declared outcome of `connectors.sessions.LoseContinuity`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn lose_continuity(&mut self, input: super::LoseContinuity) -> Result<super::LoseContinuityOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `connectors.sessions.OfferSession` — an implementation obligation.
    ///
    /// Why it is not generated: the contract is declared; the algorithm is not.
    ///
    /// Contract: given `connectors.sessions.OfferSession` input, decide and enact exactly one outcome — `offered` otherwise, creates `connectors.sessions.Session`, emits `connectors.sessions.SessionOffered`.
    pub trait OfferSessionBehavior {
        /// Decides and enacts exactly one declared outcome of `connectors.sessions.OfferSession`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn offer_session(&mut self, input: super::OfferSession) -> Result<super::OfferSessionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `connectors.sessions.PermitData` — an implementation obligation.
    ///
    /// Why it is not generated: the contract is declared; the algorithm is not.
    ///
    /// Contract: given `connectors.sessions.PermitData` input, decide and enact exactly one outcome — `permitted` when `decision == allow`, takes `permit_data` of `connectors.sessions.Session`, emits `connectors.sessions.DataPermitted`; `authority-terminated` otherwise, takes `deny_data` of `connectors.sessions.Session`, emits `connectors.sessions.DataAuthorityTerminated`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields.
    pub trait PermitDataBehavior {
        /// Decides and enacts exactly one declared outcome of `connectors.sessions.PermitData`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn permit_data(&mut self, input: super::PermitData) -> Result<super::PermitDataOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `connectors.sessions.RenewDataLease` — an implementation obligation.
    ///
    /// Why it is not generated: the contract is declared; the algorithm is not.
    ///
    /// Contract: given `connectors.sessions.RenewDataLease` input, decide and enact exactly one outcome — `renewed` when `decision == allow`, takes `renew` of `connectors.sessions.Session`, emits `connectors.sessions.DataLeaseRenewed`; `authority-terminated` otherwise, takes `deny_renewal` of `connectors.sessions.Session`, emits `connectors.sessions.DataAuthorityTerminated`; `wrong-state` from a state no declared move starts in, error `connectors.sessions.StateConflict`, and for an instance no record carries, without the error's fields.
    pub trait RenewDataLeaseBehavior {
        /// Decides and enacts exactly one declared outcome of `connectors.sessions.RenewDataLease`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn renew_data_lease(&mut self, input: super::RenewDataLease) -> Result<super::RenewDataLeaseOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `connectors.sessions.SessionStates` — an implementation obligation.
    ///
    /// Why it is not generated: how the projection is kept current is a storage decision.
    ///
    /// Contract: a query answering `connectors.sessions.SessionStates` with rows projected from `connectors.sessions.Session` at `read_your_writes` consistency.
    pub trait SessionStatesQuery {
        /// Serves `connectors.sessions.SessionStates` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn session_states(&self) -> Result<Vec<super::SessionStates>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl AcceptOfferBehavior for Unimplemented {
        fn accept_offer(&mut self, _input: super::AcceptOffer) -> Result<super::AcceptOfferOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "connectors.sessions.AcceptOffer" })
        }
    }

    impl BeginCloseBehavior for Unimplemented {
        fn begin_close(&mut self, _input: super::BeginClose) -> Result<super::BeginCloseOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "connectors.sessions.BeginClose" })
        }
    }

    impl EstablishReadyBehavior for Unimplemented {
        fn establish_ready(&mut self, _input: super::EstablishReady) -> Result<super::EstablishReadyOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "connectors.sessions.EstablishReady" })
        }
    }

    impl FinishTeardownBehavior for Unimplemented {
        fn finish_teardown(&mut self, _input: super::FinishTeardown) -> Result<super::FinishTeardownOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "connectors.sessions.FinishTeardown" })
        }
    }

    impl LoseContinuityBehavior for Unimplemented {
        fn lose_continuity(&mut self, _input: super::LoseContinuity) -> Result<super::LoseContinuityOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "connectors.sessions.LoseContinuity" })
        }
    }

    impl OfferSessionBehavior for Unimplemented {
        fn offer_session(&mut self, _input: super::OfferSession) -> Result<super::OfferSessionOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "connectors.sessions.OfferSession" })
        }
    }

    impl PermitDataBehavior for Unimplemented {
        fn permit_data(&mut self, _input: super::PermitData) -> Result<super::PermitDataOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "connectors.sessions.PermitData" })
        }
    }

    impl RenewDataLeaseBehavior for Unimplemented {
        fn renew_data_lease(&mut self, _input: super::RenewDataLease) -> Result<super::RenewDataLeaseOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "connectors.sessions.RenewDataLease" })
        }
    }

    impl SessionStatesQuery for Unimplemented {
        fn session_states(&self) -> Result<Vec<super::SessionStates>, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "view query", source: "connectors.sessions.SessionStates" })
        }
    }
}

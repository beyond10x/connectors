// generated from connectors v1
// model digest 0f1b92b6b86a69778081be2aca23de79337294bbb219d2bd7f19aad1f6523d26
// contract digest b9b32b755ebf5ba2e2d690e05122218781ab050a55aa96d08f5ab2d6164aff5e
// do not edit: regenerate with `ess synthesize`

//! What the specification fully determines, generated: the behaviour of every command the plan
//! lists as generated, written against ports the implementor supplies.
//!
//! Storage is a port: one trait per entity, get, put and delete of a snapshot by identity. ess
//! preserves the trait; generated network entries supply an ephemeral store. `Context` carries the caller's attributes,
//! every identity and value the model says the implementation assigns, and the answer to each
//! `external:` branch. [`Generated`] implements every generated `…Behavior` trait over those ports
//! and forwards every behaviour and query the plan still owes to them, so it is a complete bundle
//! for every component port. To replace one generated behaviour, write a bundle of your own that
//! implements that trait and delegates the rest to a `Generated`.
//!
//! An `Err` from a generated behaviour is the typed refusal naming the command: the model declares
//! no outcome for the request (a guard is undecidable over it, or no declared branch answers it),
//! or — as `entity invariant` — the declared outcome would leave an entity breaking an invariant.

use crate::obligation::UnmetObligation;

/// Where `connectors.sessions.Session` is stored — a port the implementor provides.
///
/// Keyed by the identity `session_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait SessionStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::sessions::SessionId) -> Option<crate::sessions::SessionSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::sessions::SessionSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::sessions::SessionId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::sessions::SessionSnapshot>;
}

/// Every generated behaviour of this workspace, over the ports `P` supplies.
///
/// `P` implements the storage trait of each entity a generated behaviour reads or writes, and
/// every `…Behavior` and `…Query` trait the plan still owes; `Generated<P>` forwards those to it.
pub struct Generated<P> {
    /// The storage ports, and every behaviour or query still owed.
    pub ports: P,
}

impl<P> Generated<P> {
    /// The generated behaviours, over `ports`.
    pub fn new(ports: P) -> Self {
        Self { ports }
    }
}

/// `connectors.sessions.AcceptOffer`, generated: every outcome is one the specification fully determines.
impl<P> crate::sessions::obligations::AcceptOfferBehavior for Generated<P>
where
    P: SessionStorage,
{
    fn accept_offer(&mut self, input: crate::sessions::AcceptOffer) -> Result<crate::sessions::AcceptOfferOutcome, UnmetObligation> {
        let _ = &input;
        // `accepted`: the default.
        let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
            return Ok(crate::sessions::AcceptOfferOutcome::WrongStateUnknownInstance);
        };
        let _ = &held;
        let held_state = held.state;
        let moved = match held.refine() {
            crate::sessions::AnySession::Offered(instance) => crate::sessions::AnySession::Establishing(instance.accept()),
            _ => return Ok(crate::sessions::AcceptOfferOutcome::WrongState { error: crate::sessions::StateConflict { state: held_state } }),
        };
        let next = moved.snapshot();
        let answer = crate::sessions::AcceptOfferOutcome::Accepted { offer_accepted: crate::sessions::OfferAccepted { session_id: input.session_id.clone() } };
        SessionStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `connectors.sessions.BeginClose`, generated: every outcome is one the specification fully determines.
impl<P> crate::sessions::obligations::BeginCloseBehavior for Generated<P>
where
    P: SessionStorage,
{
    fn begin_close(&mut self, input: crate::sessions::BeginClose) -> Result<crate::sessions::BeginCloseOutcome, UnmetObligation> {
        let _ = &input;
        // `closing`: the default.
        let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
            return Ok(crate::sessions::BeginCloseOutcome::WrongStateUnknownInstance);
        };
        let _ = &held;
        let held_state = held.state;
        let moved = match held.refine() {
            crate::sessions::AnySession::Establishing(instance) => crate::sessions::AnySession::Closing(instance.begin_close()),
            crate::sessions::AnySession::Offered(instance) => crate::sessions::AnySession::Closing(instance.begin_close()),
            crate::sessions::AnySession::Ready(instance) => crate::sessions::AnySession::Closing(instance.begin_close()),
            _ => return Ok(crate::sessions::BeginCloseOutcome::WrongState { error: crate::sessions::StateConflict { state: held_state } }),
        };
        let next = moved.snapshot();
        let answer = crate::sessions::BeginCloseOutcome::Closing { closing_begun: crate::sessions::ClosingBegun { session_id: input.session_id.clone(), terminal: input.terminal.clone(), cutoff_due_at: input.cutoff_due_at.clone(), teardown_due_at: input.teardown_due_at.clone() } };
        SessionStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `connectors.sessions.EstablishReady`, generated: every outcome is one the specification fully determines.
impl<P> crate::sessions::obligations::EstablishReadyBehavior for Generated<P>
where
    P: SessionStorage,
{
    fn establish_ready(&mut self, input: crate::sessions::EstablishReady) -> Result<crate::sessions::EstablishReadyOutcome, UnmetObligation> {
        let _ = &input;
        // `ready`: an accepting branch, in declaration order.
        if decided(equal(Some(&input.decision).map(|value| match value { crate::sessions::GateDecision::Allow => "allow", crate::sessions::GateDecision::Expired => "expired", crate::sessions::GateDecision::Revoked => "revoked", crate::sessions::GateDecision::Unverified => "unverified" }.to_owned()), Some("allow".to_owned())), "connectors.sessions.EstablishReady")? {
            let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
                return Ok(crate::sessions::EstablishReadyOutcome::WrongStateUnknownInstance);
            };
            let _ = &held;
            let held_state = held.state;
            let moved = match held.refine() {
                crate::sessions::AnySession::Establishing(instance) => crate::sessions::AnySession::Ready(instance.ready()),
                _ => return Ok(crate::sessions::EstablishReadyOutcome::WrongState { error: crate::sessions::StateConflict { state: held_state } }),
            };
            let next = moved.snapshot();
            let answer = crate::sessions::EstablishReadyOutcome::Ready { session_ready: crate::sessions::SessionReady { session_id: input.session_id.clone(), lease: input.lease.clone() } };
            SessionStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `authority-terminated`: the default.
        let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
            return Ok(crate::sessions::EstablishReadyOutcome::WrongStateUnknownInstance);
        };
        let _ = &held;
        let held_state = held.state;
        let moved = match held.refine() {
            crate::sessions::AnySession::Establishing(instance) => crate::sessions::AnySession::Closing(instance.deny_ready()),
            _ => return Ok(crate::sessions::EstablishReadyOutcome::WrongState { error: crate::sessions::StateConflict { state: held_state } }),
        };
        let next = moved.snapshot();
        let answer = crate::sessions::EstablishReadyOutcome::AuthorityTerminated { data_authority_terminated: crate::sessions::DataAuthorityTerminated { session_id: input.session_id.clone(), decision: input.decision.clone() } };
        SessionStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `connectors.sessions.FinishTeardown`, generated: every outcome is one the specification fully determines.
impl<P> crate::sessions::obligations::FinishTeardownBehavior for Generated<P>
where
    P: SessionStorage,
{
    fn finish_teardown(&mut self, input: crate::sessions::FinishTeardown) -> Result<crate::sessions::FinishTeardownOutcome, UnmetObligation> {
        let _ = &input;
        // `closed`: an accepting branch, in declaration order.
        if decided(equal(Some(&input.decision).map(|value| match value { crate::sessions::CleanupDecision::Released => "released", crate::sessions::CleanupDecision::Unaccounted => "unaccounted" }.to_owned()), Some("released".to_owned())), "connectors.sessions.FinishTeardown")? {
            let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
                return Ok(crate::sessions::FinishTeardownOutcome::WrongStateUnknownInstance);
            };
            let _ = &held;
            let held_state = held.state;
            let moved = match held.refine() {
                crate::sessions::AnySession::Closing(instance) => crate::sessions::AnySession::Closed(instance.close()),
                _ => return Ok(crate::sessions::FinishTeardownOutcome::WrongState { error: crate::sessions::StateConflict { state: held_state } }),
            };
            let next = moved.snapshot();
            let answer = crate::sessions::FinishTeardownOutcome::Closed { local_resources_released: crate::sessions::LocalResourcesReleased { session_id: input.session_id.clone(), peer_shutdown: input.peer_shutdown.clone() } };
            SessionStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `lost`: the default.
        let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
            return Ok(crate::sessions::FinishTeardownOutcome::WrongStateUnknownInstance);
        };
        let _ = &held;
        let held_state = held.state;
        let moved = match held.refine() {
            crate::sessions::AnySession::Closing(instance) => crate::sessions::AnySession::Lost(instance.cleanup_lost()),
            _ => return Ok(crate::sessions::FinishTeardownOutcome::WrongState { error: crate::sessions::StateConflict { state: held_state } }),
        };
        let next = moved.snapshot();
        let answer = crate::sessions::FinishTeardownOutcome::Lost { local_cleanup_unaccounted: crate::sessions::LocalCleanupUnaccounted { session_id: input.session_id.clone() } };
        SessionStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `connectors.sessions.LoseContinuity`, generated: every outcome is one the specification fully determines.
impl<P> crate::sessions::obligations::LoseContinuityBehavior for Generated<P>
where
    P: SessionStorage,
{
    fn lose_continuity(&mut self, input: crate::sessions::LoseContinuity) -> Result<crate::sessions::LoseContinuityOutcome, UnmetObligation> {
        let _ = &input;
        // `lost`: the default.
        let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
            return Ok(crate::sessions::LoseContinuityOutcome::WrongStateUnknownInstance);
        };
        let _ = &held;
        let held_state = held.state;
        let moved = match held.refine() {
            crate::sessions::AnySession::Closing(instance) => crate::sessions::AnySession::Lost(instance.continuity_lost()),
            crate::sessions::AnySession::Establishing(instance) => crate::sessions::AnySession::Lost(instance.continuity_lost()),
            crate::sessions::AnySession::Offered(instance) => crate::sessions::AnySession::Lost(instance.continuity_lost()),
            crate::sessions::AnySession::Ready(instance) => crate::sessions::AnySession::Lost(instance.continuity_lost()),
            _ => return Ok(crate::sessions::LoseContinuityOutcome::WrongState { error: crate::sessions::StateConflict { state: held_state } }),
        };
        let next = moved.snapshot();
        let answer = crate::sessions::LoseContinuityOutcome::Lost { continuity_lost: crate::sessions::ContinuityLost { session_id: input.session_id.clone() } };
        SessionStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

impl<P: crate::sessions::obligations::OfferSessionBehavior> crate::sessions::obligations::OfferSessionBehavior for Generated<P> {
    fn offer_session(&mut self, input: crate::sessions::OfferSession) -> Result<crate::sessions::OfferSessionOutcome, UnmetObligation> {
        crate::sessions::obligations::OfferSessionBehavior::offer_session(&mut self.ports, input)
    }
}

/// `connectors.sessions.PermitData`, generated: every outcome is one the specification fully determines.
impl<P> crate::sessions::obligations::PermitDataBehavior for Generated<P>
where
    P: SessionStorage,
{
    fn permit_data(&mut self, input: crate::sessions::PermitData) -> Result<crate::sessions::PermitDataOutcome, UnmetObligation> {
        let _ = &input;
        // `permitted`: an accepting branch, in declaration order.
        if decided(equal(Some(&input.decision).map(|value| match value { crate::sessions::GateDecision::Allow => "allow", crate::sessions::GateDecision::Expired => "expired", crate::sessions::GateDecision::Revoked => "revoked", crate::sessions::GateDecision::Unverified => "unverified" }.to_owned()), Some("allow".to_owned())), "connectors.sessions.PermitData")? {
            let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
                return Ok(crate::sessions::PermitDataOutcome::WrongStateUnknownInstance);
            };
            let _ = &held;
            let held_state = held.state;
            let moved = match held.refine() {
                crate::sessions::AnySession::Ready(instance) => crate::sessions::AnySession::Ready(instance.permit_data()),
                _ => return Ok(crate::sessions::PermitDataOutcome::WrongState { error: crate::sessions::StateConflict { state: held_state } }),
            };
            let next = moved.snapshot();
            let answer = crate::sessions::PermitDataOutcome::Permitted { data_permitted: crate::sessions::DataPermitted { session_id: input.session_id.clone(), direction: input.direction.clone() } };
            SessionStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `authority-terminated`: the default.
        let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
            return Ok(crate::sessions::PermitDataOutcome::WrongStateUnknownInstance);
        };
        let _ = &held;
        let held_state = held.state;
        let moved = match held.refine() {
            crate::sessions::AnySession::Ready(instance) => crate::sessions::AnySession::Closing(instance.deny_data()),
            _ => return Ok(crate::sessions::PermitDataOutcome::WrongState { error: crate::sessions::StateConflict { state: held_state } }),
        };
        let next = moved.snapshot();
        let answer = crate::sessions::PermitDataOutcome::AuthorityTerminated { data_authority_terminated: crate::sessions::DataAuthorityTerminated { session_id: input.session_id.clone(), decision: input.decision.clone() } };
        SessionStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `connectors.sessions.RenewDataLease`, generated: every outcome is one the specification fully determines.
impl<P> crate::sessions::obligations::RenewDataLeaseBehavior for Generated<P>
where
    P: SessionStorage,
{
    fn renew_data_lease(&mut self, input: crate::sessions::RenewDataLease) -> Result<crate::sessions::RenewDataLeaseOutcome, UnmetObligation> {
        let _ = &input;
        // `renewed`: an accepting branch, in declaration order.
        if decided(equal(Some(&input.decision).map(|value| match value { crate::sessions::GateDecision::Allow => "allow", crate::sessions::GateDecision::Expired => "expired", crate::sessions::GateDecision::Revoked => "revoked", crate::sessions::GateDecision::Unverified => "unverified" }.to_owned()), Some("allow".to_owned())), "connectors.sessions.RenewDataLease")? {
            let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
                return Ok(crate::sessions::RenewDataLeaseOutcome::WrongStateUnknownInstance);
            };
            let _ = &held;
            let held_state = held.state;
            let moved = match held.refine() {
                crate::sessions::AnySession::Ready(instance) => crate::sessions::AnySession::Ready(instance.renew()),
                _ => return Ok(crate::sessions::RenewDataLeaseOutcome::WrongState { error: crate::sessions::StateConflict { state: held_state } }),
            };
            let next = moved.snapshot();
            let answer = crate::sessions::RenewDataLeaseOutcome::Renewed { data_lease_renewed: crate::sessions::DataLeaseRenewed { session_id: input.session_id.clone(), lease: input.lease.clone() } };
            SessionStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `authority-terminated`: the default.
        let Some(held) = SessionStorage::get(&self.ports, &input.session_id) else {
            return Ok(crate::sessions::RenewDataLeaseOutcome::WrongStateUnknownInstance);
        };
        let _ = &held;
        let held_state = held.state;
        let moved = match held.refine() {
            crate::sessions::AnySession::Ready(instance) => crate::sessions::AnySession::Closing(instance.deny_renewal()),
            _ => return Ok(crate::sessions::RenewDataLeaseOutcome::WrongState { error: crate::sessions::StateConflict { state: held_state } }),
        };
        let next = moved.snapshot();
        let answer = crate::sessions::RenewDataLeaseOutcome::AuthorityTerminated { data_authority_terminated: crate::sessions::DataAuthorityTerminated { session_id: input.session_id.clone(), decision: input.decision.clone() } };
        SessionStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `connectors.sessions.SessionStates`, generated: every row is one the specification fully determines from the stored `connectors.sessions.Session`s.
impl<P> crate::sessions::obligations::SessionStatesQuery for Generated<P>
where
    P: SessionStorage,
{
    fn session_states(&self) -> Result<Vec<crate::sessions::SessionStates>, UnmetObligation> {
        let admitted = SessionStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::sessions::SessionStates {
                session_id: held.data.session_id,
                state: held.state,
                terminal: held.data.terminal,
            })
            .collect())
    }
}

/// The typed refusal of a request the model declares no outcome for.
fn undeclared(source: &'static str) -> UnmetObligation {
    let capability = "command behaviour";
    UnmetObligation { capability, source }
}

/// A guard's truth, where it has one: Unknown selects no branch, so the model declares no outcome.
fn decided(truth: Option<bool>, command: &'static str) -> Result<bool, UnmetObligation> {
    truth.ok_or_else(|| undeclared(command))
}

/// Equality of two read values; an unread one is Unknown.
fn equal<T: PartialEq>(left: Option<T>, right: Option<T>) -> Option<bool> {
    Some(left? == right?)
}

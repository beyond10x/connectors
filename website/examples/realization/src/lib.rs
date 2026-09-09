//! Bounded, in-memory teaching examples. No provider I/O or production persistence.
//! Canonical mutation commands run through ESS-generated ports and typestate moves.
//! Discovery/eligibility actions inject fictional host facts, never caller authority.
use connectors_types::{
    auth_bindings as a, connection_admission as c, discovery_state as d, mutations as m,
    obligation::UnmetObligation,
    primitives::{Timestamp, Uuid},
};
use serde_json::{json, Value};
use std::{cell::RefCell, rc::Rc};

pub mod walkthrough;

#[derive(Default)]
struct Attempts {
    current: Option<m::AttemptRecordSnapshot>,
    next: u32,
    now: u64,
}
#[derive(Clone, Default)]
pub struct Behaviors(Rc<RefCell<Attempts>>);
fn unmet(source: &'static str) -> UnmetObligation {
    UnmetObligation {
        capability: "bounded example store",
        source,
    }
}
impl m::obligations::PrepareAttemptBehavior for Behaviors {
    fn prepare_attempt(
        &mut self,
        input: m::PrepareAttempt,
    ) -> Result<m::PrepareAttemptOutcome, UnmetObligation> {
        let mut store = self.0.borrow_mut();
        if store.current.is_some() {
            return Err(unmet("PrepareAttempt"));
        }
        store.next += 1;
        let id = m::AttemptId(Uuid(format!("00000000-0000-4000-8000-{:012}", store.next)));
        let approval_mode = input.approval_mode;
        let record = m::AttemptRecord::new(m::AttemptRecordData {
            attempt_id: id.clone(),
            instance_id: input.instance_id,
            request_id: input.request_id,
            operation_id: input.operation_id,
            connection_ref: input.connection_ref,
            input_digest: input.input_digest,
            approval_mode,
            approval_ref: input.approval_ref,
            idempotency_key: input.idempotency_key,
            settled_at: None,
        });
        store.current = Some(m::AttemptRecordSnapshot {
            state: record.state(),
            data: record.into_data(),
        });
        Ok(m::PrepareAttemptOutcome::Prepared {
            attempt_prepared: m::AttemptPrepared {
                attempt_id: id,
                approval_mode,
            },
        })
    }
}
// The from-state and move method must compile against ESS's actual typestate API.
macro_rules! move_attempt {
    ($trait:ident,$method:ident,$input:ident,$outcome:ident,$from:ident,$transition:ident,$accepted:ident,$field:ident,$event:ident) => {
        impl m::obligations::$trait for Behaviors {
            fn $method(&mut self, input: m::$input) -> Result<m::$outcome, UnmetObligation> {
                let mut store = self.0.borrow_mut();
                if store
                    .current
                    .as_ref()
                    .is_none_or(|r| r.data.attempt_id != input.attempt_id)
                {
                    return Err(unmet(stringify!($input)));
                }
                let snapshot = store.current.take().expect("checked above");
                match snapshot.refine() {
                    m::AnyAttemptRecord::$from(record) => {
                        let moved = record.$transition();
                        let mut snapshot = m::AttemptRecordSnapshot {
                            state: moved.state(),
                            data: moved.into_data(),
                        };
                        if !matches!(
                            snapshot.state,
                            m::AttemptRecordState::Prepared | m::AttemptRecordState::Dispatching
                        ) {
                            snapshot.data.settled_at = Some(Timestamp(format!(
                                "2026-01-01T00:{:02}:{:02}Z",
                                store.now / 60,
                                store.now % 60
                            )));
                        }
                        store.current = Some(snapshot);
                        Ok(m::$outcome::$accepted {
                            $field: m::$event {
                                attempt_id: input.attempt_id,
                            },
                        })
                    }
                    other => {
                        let state = other.state();
                        store.current = Some(other.snapshot());
                        Ok(m::$outcome::WrongState {
                            error: m::AttemptStateConflict { state },
                        })
                    }
                }
            }
        }
    };
}
move_attempt!(
    OpenDispatchBehavior,
    open_dispatch,
    OpenDispatch,
    OpenDispatchOutcome,
    Prepared,
    open_dispatch,
    Opened,
    dispatch_opened,
    DispatchOpened
);
move_attempt!(
    AbortPreparedBehavior,
    abort_prepared,
    AbortPrepared,
    AbortPreparedOutcome,
    Prepared,
    abort,
    Aborted,
    attempt_aborted,
    AttemptAborted
);
move_attempt!(
    RecordCompletionBehavior,
    record_completion,
    RecordCompletion,
    RecordCompletionOutcome,
    Dispatching,
    complete,
    Completed,
    attempt_completed,
    AttemptCompleted
);
move_attempt!(
    RecordRefusalBehavior,
    record_refusal,
    RecordRefusal,
    RecordRefusalOutcome,
    Dispatching,
    refuse,
    Refused,
    attempt_refused,
    AttemptRefused
);
move_attempt!(
    RecordUncertaintyBehavior,
    record_uncertainty,
    RecordUncertainty,
    RecordUncertaintyOutcome,
    Dispatching,
    lose_outcome,
    Indeterminate,
    attempt_indeterminate,
    AttemptIndeterminate
);
impl m::obligations::AttemptStatesQuery for Behaviors {
    fn attempt_states(&self) -> Result<Vec<m::AttemptStates>, UnmetObligation> {
        Ok(self
            .0
            .borrow()
            .current
            .iter()
            .map(|r| m::AttemptStates {
                attempt_id: r.data.attempt_id.clone(),
                approval_mode: r.data.approval_mode,
                state: r.state,
            })
            .collect())
    }
}

type System = connectors_system::System<Behaviors>;
/// One fixed-scope example, with deterministic time and fictional provider facts.
pub struct Lab {
    system: System,
    behavior: Behaviors,
    scenario: String,
    now: u64,
    observation: Option<d::ResourceObservationState>,
    candidate: String,
    connection: Option<a::ConnectionState>,
    target: Option<String>,
    facts: c::ViabilityFacts,
    admitted: bool,
    permission: c::RequirementResult,
    scopes: c::RequirementResult,
    baseline_until: u64,
    permission_until: u64,
    approval: bool,
    redemptions: u32,
    sends: u32,
    timeline: Vec<Value>,
}
impl Lab {
    pub fn new(scenario: &str) -> Self {
        let behavior = Behaviors::default();
        let system = System::new(contract_examples::ContractExamples::new(behavior.clone()));
        let configured = scenario != "discovery";
        Self {
            system,
            behavior,
            scenario: scenario.to_owned(),
            now: 0,
            observation: None,
            candidate: "database-a.example".into(),
            connection: configured.then_some(a::ConnectionState::Live),
            target: configured.then(|| "database-a.example".into()),
            facts: c::ViabilityFacts {
                metadata_available: true,
                locally_revoked: false,
                enabled: true,
                established_credential_failure: false,
                required_custody_available: true,
                required_parent_available: true,
                validated_publication: configured,
                baseline_current: configured,
            },
            admitted: true,
            permission: c::RequirementResult::Satisfied,
            scopes: c::RequirementResult::Satisfied,
            baseline_until: 300,
            permission_until: 60,
            approval: false,
            redemptions: 0,
            sends: 0,
            timeline: vec![],
        }
    }
    // contracts/auth/connection/v1alpha1/semantics.md §4.1: ordered reduction.
    fn viability(&self) -> (&'static str, Option<c::ConnectionState>) {
        use c::ConnectionState as S;
        let f = &self.facts;
        if !f.metadata_available {
            return ("unavailable", None);
        }
        let state = if f.locally_revoked {
            S::Revoked
        } else if !f.enabled {
            S::Disabled
        } else if f.established_credential_failure {
            S::ReauthorizationRequired
        } else if !f.required_custody_available {
            S::CustodyUnavailable
        } else if !f.required_parent_available {
            S::ParentDegraded
        } else if !f.validated_publication || !f.baseline_current || self.now >= self.baseline_until
        {
            S::Pending
        } else {
            S::Ready
        };
        let label = match state {
            S::Revoked => "revoked",
            S::Disabled => "disabled",
            S::ReauthorizationRequired => "reauthorization_required",
            S::CustodyUnavailable => "custody_unavailable",
            S::ParentDegraded => "parent_degraded",
            S::Pending => "pending",
            S::Ready => "ready",
        };
        (label, Some(state))
    }
    fn eligibility(&self) -> &'static str {
        if !self.admitted {
            return "not_granted";
        }
        if !self.facts.metadata_available {
            return "unavailable";
        }
        if self.viability().1 != Some(c::ConnectionState::Ready) {
            return "connection_not_ready";
        }
        if self.scopes == c::RequirementResult::Insufficient {
            return "insufficient_scope";
        }
        if self.scopes != c::RequirementResult::Satisfied {
            return "connection_not_ready";
        }
        if self.now >= self.permission_until {
            return "unavailable";
        }
        match self.permission {
            c::RequirementResult::Satisfied => "eligible",
            c::RequirementResult::Denied => "forbidden",
            _ => "unavailable",
        }
    }
    fn current_id(&self) -> Option<m::AttemptId> {
        self.behavior
            .0
            .borrow()
            .current
            .as_ref()
            .map(|r| r.data.attempt_id.clone())
    }
    fn record(&mut self, action: &str, outcome: &str, explanation: &str) {
        if self.timeline.len() == 80 {
            self.timeline.remove(0);
        }
        self.timeline.push(
            json!({"action":action,"outcome":outcome,"explanation":explanation,"time":self.now}),
        );
    }
    pub fn apply(&mut self, action: &str) -> Value {
        if action == "reset" {
            *self = Self::new(&self.scenario.clone());
            return self.snapshot();
        }
        self.behavior.0.borrow_mut().now = self.now;
        match action {
            "discover" => {
                if self.observation == Some(d::ResourceObservationState::Withdrawn) {
                    self.record(action,"new_observation_required","A withdrawn observation cannot be resurrected. Reset this bounded example for a new identity.");
                } else {
                    self.observation = Some(d::ResourceObservationState::Observed);
                    self.record(action,"observed","Kubernetes supplied a fictional endpoint observation. No SQL connection or authority was created.");
                }
            }
            "withdraw" => {
                if self.observation.is_some() {
                    self.observation = Some(d::ResourceObservationState::Withdrawn);
                    self.record(action,"withdrawn","The observation is withdrawn. An existing connection keeps its fixed historical target; new selection is refused.");
                } else {
                    self.record(action, "missing_observation", "Discover an endpoint first.");
                }
            }
            "change_target" => {
                self.candidate = "database-b.example".into();
                self.record(action,"candidate_changed","A different candidate cannot silently change the target of an existing connection.");
            }
            "bind" => {
                if !self.admitted {
                    self.record(
                        action,
                        "not_granted",
                        "The fictional host refuses management access.",
                    );
                } else if self.observation != Some(d::ResourceObservationState::Observed) {
                    self.record(
                        action,
                        "observation_unusable",
                        "Select a current observation before configuring a connection.",
                    );
                } else if self.target.as_ref().is_some_and(|t| t != &self.candidate) {
                    self.record(action,"new_connection_required","Changing the fixed target requires a new binding, not retargeting the existing connection.");
                } else if self.connection == Some(a::ConnectionState::Revoked) {
                    self.record(action, "revoked", "Local revocation is terminal.");
                } else {
                    self.connection = Some(a::ConnectionState::Live);
                    self.target = Some(self.candidate.clone());
                    self.facts.validated_publication = true;
                    self.facts.baseline_current = true;
                    self.baseline_until = self.now + 300;
                    self.permission_until = self.now + 60;
                    self.record(action,"binding_published","The example host explicitly selected this target and supplied fictional validated credential and permission evidence.");
                }
            }
            "read" => {
                let result = self.eligibility();
                self.record(action,if result=="eligible"{"read_succeeded"}else{result},if result=="eligible"{"The fictional provider returned two fixture rows from the fixed bound target. No external query was sent."}else{"Discovery, credentials and current permission are separate requirements; this read performed no provider work."});
            }
            "advance_time" => {
                if self.now + 301 > 3599 {
                    self.record(action, "clock_limit", "This bounded example covers one fictional hour. Reset to begin another exercise.");
                    return self.snapshot();
                }
                self.now += 301;
                self.record(action,"clock_advanced","The injected clock advanced by 301 seconds. Previously positive evidence is now expired.");
            }
            "revalidate" => {
                if !self.admitted {
                    self.record(
                        action,
                        "not_granted",
                        "Validation requires separate host admission.",
                    );
                } else if self.connection != Some(a::ConnectionState::Live)
                    || !self.facts.required_custody_available
                    || self.facts.established_credential_failure
                {
                    self.record(
                        action,
                        "connection_not_ready",
                        "Current facts do not support successful validation.",
                    );
                } else {
                    self.facts.baseline_current = true;
                    self.baseline_until = self.now + 300;
                    self.permission_until = self.now + 60;
                    self.record(action,"evidence_refreshed","A separate admitted validation step installed fictional current evidence for the existing fixed binding. It did not retry an operation.");
                }
            }
            "toggle_permission" => {
                self.permission = if self.permission == c::RequirementResult::Denied {
                    c::RequirementResult::Satisfied
                } else {
                    c::RequirementResult::Denied
                };
                self.record(action,"permission_changed","The injected exact-operation permission changed. Global readiness is a separate reduction.");
            }
            "toggle_scope" => {
                self.scopes = if self.scopes == c::RequirementResult::Insufficient {
                    c::RequirementResult::Satisfied
                } else {
                    c::RequirementResult::Insufficient
                };
                self.record(action,"scope_changed","This toggles an operation-only scope. The example profile's minimum baseline grants remain fixed.");
            }
            "toggle_custody" => {
                self.facts.required_custody_available = !self.facts.required_custody_available;
                self.record(action,"custody_changed","The required credential store's availability changed; no credential bytes are exposed.");
            }
            "toggle_credential" => {
                self.facts.established_credential_failure =
                    !self.facts.established_credential_failure;
                self.record(action,"credential_changed","This injects or clears an established active-credential failure. It grants no host authority.");
            }
            "toggle_admission" => {
                self.admitted = !self.admitted;
                self.record(
                    action,
                    "host_policy_changed",
                    "Current host admission governs each operation and replay disclosure.",
                );
            }
            "revoke" => {
                self.connection = Some(a::ConnectionState::Revoked);
                self.facts.locally_revoked = true;
                self.record(action,"revoked","Local terminal revocation takes precedence over credential, custody and freshness observations.");
            }
            "approve" => {
                if self.redemptions > 0 {
                    self.record(action,"already_redeemed","The example's one approval was already spent; it cannot authorize a second dispatch.");
                } else {
                    self.approval = true;
                    self.record(action,"approval_supplied","A fictional, fixed-request approval is available. Cryptographic verification is outside this example.");
                }
            }
            "prepare" | "retry" | "conflicting_input" => self.prepare(action),
            "dispatch" => {
                if self.eligibility() != "eligible"
                    || self.now >= self.baseline_until.saturating_sub(240)
                {
                    self.record(action,"dispatch_not_admitted","The final check requires current authority and mutation evidence younger than 60 seconds. Nothing was sent.");
                } else if let Some(id) = self.current_id() {
                    match self
                        .system
                        .contract_examples
                        .open_dispatch(m::OpenDispatch { attempt_id: id })
                    {
                        Ok(m::OpenDispatchOutcome::Opened { .. }) => {
                            self.sends += 1;
                            self.record(action,"dispatched_once","The generated dispatch transition succeeded. The example then sent one fictional request; the ledger gate alone would not prove a send.");
                        }
                        Ok(m::OpenDispatchOutcome::WrongState { .. }) => self.record(
                            action,
                            "wrong_state",
                            "This attempt cannot win another dispatch gate. No request was sent.",
                        ),
                        Err(_) => self.record(
                            action,
                            "unavailable",
                            "The bounded example store has no matching attempt.",
                        ),
                    }
                } else {
                    self.record(action, "no_attempt", "Prepare an admitted attempt first.");
                }
            }
            "lose_response" | "complete" | "abort" => self.settle(action),
            _ => self.record(
                action,
                "unknown_action",
                "This example accepts only its documented actions.",
            ),
        }
        self.system
            .pump()
            .expect("this example has no event bindings");
        self.snapshot()
    }
    fn prepare(&mut self, action: &str) {
        if self.eligibility() != "eligible" {
            let outcome = self.eligibility();
            self.record(
                action,
                outcome,
                "Current access is required before execution or replay disclosure.",
            );
            return;
        }
        let prior = self.behavior.0.borrow().current.as_ref().map(|r| r.state);
        if let Some(state) = prior {
            let outcome = if action == "conflicting_input" {
                "idempotency_conflict"
            } else {
                match state {
                    m::AttemptRecordState::Completed => "result_replayed",
                    m::AttemptRecordState::Indeterminate => "outcome_unknown",
                    m::AttemptRecordState::Aborted => "not_attempted",
                    m::AttemptRecordState::Failed => "known_refusal",
                    _ => "pending",
                }
            };
            self.record(action,outcome,"The example's fixed namespace/key already has an attempt. This lookup never sends again or redeems another approval.");
            return;
        }
        if action == "conflicting_input" {
            self.record(
                action,
                "no_existing_key",
                "Prepare the original request before trying a conflicting input.",
            );
            return;
        }
        if !self.approval {
            self.record(
                action,
                "approval_required",
                "An explicit approval is required before this mutation can be prepared.",
            );
            return;
        }
        let input = m::PrepareAttempt {
            instance_id: "example-service".into(),
            request_id: "request-1".into(),
            operation_id: "example.change".into(),
            connection_ref: "example-connection".into(),
            input_digest: if action == "conflicting_input" {
                "input-b"
            } else {
                "input-a"
            }
            .into(),
            approval_mode: m::ApprovalMode::Required,
            approval_ref: Some("fictional-approval-1".into()),
            idempotency_key: Some("example-key".into()),
        };
        self.system
            .contract_examples
            .prepare_attempt(input)
            .expect("only prepared once");
        self.approval = false;
        self.redemptions = 1;
        self.record(action,"prepared","The in-memory example records the attempt and one approval redemption together. Preparation grants no send; real durable atomicity remains an implementation obligation.");
    }
    fn settle(&mut self, action: &str) {
        let Some(id) = self.current_id() else {
            self.record(action, "no_attempt", "There is no attempt to update.");
            return;
        };
        let (accepted, outcome) = match action {
            "lose_response" => (
                matches!(
                    self.system
                        .contract_examples
                        .record_uncertainty(m::RecordUncertainty { attempt_id: id }),
                    Ok(m::RecordUncertaintyOutcome::Indeterminate { .. })
                ),
                "outcome_unknown",
            ),
            "complete" => (
                matches!(
                    self.system
                        .contract_examples
                        .record_completion(m::RecordCompletion { attempt_id: id }),
                    Ok(m::RecordCompletionOutcome::Completed { .. })
                ),
                "known_success",
            ),
            _ => (
                matches!(
                    self.system
                        .contract_examples
                        .abort_prepared(m::AbortPrepared { attempt_id: id }),
                    Ok(m::AbortPreparedOutcome::Aborted { .. })
                ),
                "not_attempted",
            ),
        };
        self.record(action,if accepted{outcome}else{"wrong_state"},if accepted{"The ESS-generated lifecycle accepted the transition. Unknown effects remain unknown; abort cannot roll back a dispatched effect."}else{"The declared lifecycle refuses this transition. A terminal observation cannot be replaced."});
    }
    pub fn snapshot(&self) -> Value {
        let current = self.behavior.0.borrow();
        json!({"scenario":self.scenario,"clock":self.now,"observation":self.observation.map(|s|format!("{s:?}")).unwrap_or_else(||"Not observed".into()),"candidate":self.candidate,"target":self.target,"connection":self.connection.map(|s|format!("{s:?}")).unwrap_or_else(||"Not allocated".into()),"viability":self.viability().0,"eligibility":self.eligibility(),"admitted":self.admitted,"permission":format!("{:?}",self.permission),"scope":format!("{:?}",self.scopes),"custody":self.facts.required_custody_available,"credential_valid":!self.facts.established_credential_failure,"attempt":current.current.as_ref().map(|r|format!("{:?}",r.state)).unwrap_or_else(||"Not prepared".into()),"approval_redemptions":self.redemptions,"provider_sends":self.sends,"events":self.system.published().iter().map(|event|format!("{event:?}")).collect::<Vec<_>>(),"timeline":self.timeline})
    }

    /// Decode the same bounded scenario interface used by the browser.
    pub fn request(&mut self, bytes: &[u8]) -> Value {
        if bytes.len() > 4096 {
            return json!({"error":"request_too_large"});
        }
        let Ok(Value::Object(fields)) = serde_json::from_slice::<Value>(bytes) else {
            return json!({"error":"invalid_request"});
        };
        if fields.iter().any(|(key, value)| {
            !["scenario", "action"].contains(&key.as_str()) || !value.is_string()
        }) {
            return json!({"error":"invalid_request"});
        }
        if let Some(scenario) = fields.get("scenario").and_then(Value::as_str) {
            if !["discovery", "readiness", "mutation"].contains(&scenario) {
                return json!({"error":"unknown_scenario"});
            }
            *self = Self::new(scenario);
        }
        if let Some(action) = fields.get("action").and_then(Value::as_str) {
            self.apply(action)
        } else {
            self.snapshot()
        }
    }
}

#[cfg(target_family = "wasm")]
mod browser {
    use super::*;
    thread_local! {static WALKTHROUGH:RefCell<walkthrough::Walkthrough>=RefCell::new(walkthrough::Walkthrough::default());}
    thread_local! {static LAB:RefCell<Lab>=RefCell::new(Lab::new("discovery"));static INPUT:RefCell<Vec<u8>>=const{RefCell::new(Vec::new())};static OUTPUT:RefCell<String>=const{RefCell::new(String::new())};}
    #[no_mangle]
    pub extern "C" fn demo_input_reserve(len: usize) -> usize {
        INPUT.with(|input| {
            let mut input = input.borrow_mut();
            input.clear();
            if len > 4096 {
                return 0;
            }
            input.resize(len, 0);
            input.as_mut_ptr() as usize
        })
    }
    #[no_mangle]
    pub extern "C" fn demo_dispatch() -> usize {
        let answer = INPUT.with(|input| LAB.with(|lab| lab.borrow_mut().request(&input.borrow())));
        OUTPUT.with(|out| {
            let mut out = out.borrow_mut();
            *out = answer.to_string();
            out.as_ptr() as usize
        })
    }
    #[no_mangle]
    pub extern "C" fn demo_output_len() -> usize {
        OUTPUT.with(|out| out.borrow().len())
    }
    #[no_mangle]
    pub extern "C" fn walkthrough_dispatch() -> usize {
        let answer = INPUT.with(|input| {
            WALKTHROUGH.with(|walkthrough| walkthrough.borrow_mut().request(&input.borrow()))
        });
        OUTPUT.with(|out| {
            let mut out = out.borrow_mut();
            *out = answer.to_string();
            out.as_ptr() as usize
        })
    }
    /// The generated ESS bridge remains available for direct canonical commands.
    #[cfg(feature = "browser")]
    #[no_mangle]
    pub extern "C" fn ess_realize() {
        let behavior = Behaviors::default();
        connectors_web::install(Box::new(System::new(
            contract_examples::ContractExamples::new(behavior),
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_requests_do_not_change_the_example() {
        let mut lab = Lab::new("mutation");
        let before = lab.snapshot();
        for input in [
            b"not json".as_slice(),
            b"[]",
            b"{\"action\":7}",
            b"{\"scenario\":\"unknown\"}",
            b"{\"admitted\":true}",
        ] {
            assert!(lab.request(input).get("error").is_some());
            assert_eq!(lab.snapshot(), before);
        }
        assert_eq!(lab.request(&vec![b' '; 4097])["error"], "request_too_large");
    }
    #[test]
    fn refreshed_evidence_uses_its_own_collection_time() {
        let mut lab = Lab::new("mutation");
        for action in [
            "advance_time",
            "revalidate",
            "approve",
            "prepare",
            "dispatch",
            "complete",
        ] {
            lab.apply(action);
        }
        assert_eq!(lab.snapshot()["attempt"], "Completed");
        assert_eq!(lab.snapshot()["provider_sends"], 1);
        assert_eq!(
            lab.behavior
                .0
                .borrow()
                .current
                .as_ref()
                .unwrap()
                .data
                .settled_at,
            Some(Timestamp("2026-01-01T00:05:01Z".into()))
        );
    }
    #[test]
    fn discovery_alone_grants_nothing() {
        let mut lab = Lab::new("discovery");
        lab.apply("discover");
        let result = lab.apply("read");
        assert_eq!(result["eligibility"], "connection_not_ready");
        assert_eq!(result["target"], Value::Null);
        lab.apply("bind");
        assert_eq!(lab.apply("read")["eligibility"], "eligible");
    }
    #[test]
    fn withdrawal_does_not_retarget_an_existing_binding() {
        let mut lab = Lab::new("discovery");
        lab.apply("discover");
        lab.apply("bind");
        lab.apply("withdraw");
        lab.apply("change_target");
        lab.apply("bind");
        let result = lab.apply("read");
        assert_eq!(result["target"], "database-a.example");
        assert_eq!(result["eligibility"], "eligible");
        assert_eq!(lab.apply("discover")["observation"], "Withdrawn");
    }
    #[test]
    fn operation_permission_is_separate_from_readiness() {
        let mut lab = Lab::new("readiness");
        let result = lab.apply("toggle_permission");
        assert_eq!(result["viability"], "ready");
        assert_eq!(result["eligibility"], "forbidden");
        lab.apply("advance_time");
        assert_eq!(lab.snapshot()["viability"], "pending");
    }
    #[test]
    fn revocation_wins_and_validation_is_admitted() {
        let mut lab = Lab::new("readiness");
        lab.apply("toggle_custody");
        lab.apply("revoke");
        assert_eq!(lab.snapshot()["viability"], "revoked");
        lab.apply("toggle_admission");
        assert_eq!(
            lab.apply("revalidate")["timeline"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()["outcome"],
            "not_granted"
        );
    }
    #[test]
    fn unknown_effect_never_resends() {
        let mut lab = Lab::new("mutation");
        lab.apply("approve");
        lab.apply("prepare");
        lab.apply("dispatch");
        lab.apply("lose_response");
        lab.apply("retry");
        lab.apply("dispatch");
        lab.apply("complete");
        let result = lab.snapshot();
        assert_eq!(result["attempt"], "Indeterminate");
        assert_eq!(result["provider_sends"], 1);
        assert_eq!(result["approval_redemptions"], 1);
    }
    #[test]
    fn known_replay_requires_current_admission() {
        let mut lab = Lab::new("mutation");
        for action in ["approve", "prepare", "dispatch", "complete", "retry"] {
            lab.apply(action);
        }
        assert_eq!(
            lab.snapshot()["timeline"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()["outcome"],
            "result_replayed"
        );
        lab.apply("toggle_admission");
        assert_eq!(
            lab.apply("retry")["timeline"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()["outcome"],
            "not_granted"
        );
        assert_eq!(lab.snapshot()["provider_sends"], 1);
    }
    #[test]
    fn conflicting_request_and_abort_do_not_dispatch() {
        let mut lab = Lab::new("mutation");
        for action in ["approve", "prepare", "conflicting_input"] {
            lab.apply(action);
        }
        assert_eq!(
            lab.snapshot()["timeline"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()["outcome"],
            "idempotency_conflict"
        );
        lab.apply("abort");
        lab.apply("dispatch");
        assert_eq!(lab.snapshot()["provider_sends"], 0);
        assert_eq!(lab.snapshot()["attempt"], "Aborted");
    }
    #[test]
    fn missing_or_expired_authority_cannot_prepare_or_dispatch() {
        let mut lab = Lab::new("mutation");
        lab.apply("prepare");
        assert_eq!(lab.snapshot()["attempt"], "Not prepared");
        lab.apply("approve");
        lab.apply("prepare");
        lab.apply("advance_time");
        lab.apply("dispatch");
        assert_eq!(lab.snapshot()["provider_sends"], 0);
    }
}

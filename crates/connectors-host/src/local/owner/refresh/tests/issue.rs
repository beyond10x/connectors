//! story:connections-token-issues-an-access-token at the owner, against the
//! fake token server of [`super`]: an access token valid beyond the
//! declaration's `refresh_lead_seconds` is issued as it is; one inside it is
//! refreshed once first. Each issue is one recorded `AccessTokenIssue` naming
//! its delivery; a refusal records nothing.
use super::*;
use crate::local::{
    access_token::Delivery,
    metadata::RecordedIssue,
    owner::{
        Code, Error,
        refresh::issue::{Issued, Refusal},
    },
};

const ISSUE: &str = "connectors.oauth2.IssueAccessToken";
const ISSUED: &str = "connectors.oauth2.AccessTokenIssued";

impl Connected {
    fn issue(&self, refresher: &Refresher, delivery: Delivery) -> Result<Issued, Refusal> {
        refresher.issue_access_token(
            INSTANCE,
            &self.binding.adapter_id,
            &self.connection,
            delivery,
            u64::MAX,
        )
    }

    /// Every recorded `AccessTokenIssue`.
    fn issues(&self) -> Vec<RecordedIssue> {
        Metadata::recorded_access_token_issues(&self.path).unwrap()
    }

    /// The recorded issue `issued` names.
    fn recorded(&self, issued: &Issued) -> RecordedIssue {
        let mut found: Vec<_> = self
            .issues()
            .into_iter()
            .filter(|record| record.issue == issued.issue)
            .collect();
        assert_eq!(found.len(), 1, "one record per issue");
        found.remove(0)
    }
}

/// `record` is `issued`'s: its generation, refresh attempt and delivery, one
/// IssueAccessToken command and one AccessTokenIssued event.
#[track_caller]
fn assert_recorded(
    record: &RecordedIssue,
    issued: &Issued,
    generation: &str,
    attempt: Option<&str>,
    delivery: &str,
) {
    assert_eq!(record.fields["connection_ref"], json!(issued.connection));
    assert_eq!(record.fields["generation_id"], json!(generation));
    assert_eq!(
        record
            .fields
            .get("refresh_attempt_id")
            .and_then(Value::as_str),
        attempt
    );
    assert_eq!(record.fields["delivery"], json!(delivery));
    assert_eq!(record.fields["scopes"], json!(issued.scopes));
    assert!(record.fields["expires_at"].is_string());
    assert_eq!(record.commands, [ISSUE]);
    assert_eq!(record.events.len(), 1);
    assert_eq!(record.events[0].0, ISSUED);
    assert_eq!(record.events[0].1["refreshed"], json!(issued.refreshed));
}

#[test]
fn owner_issue_beyond_the_lead_is_current_without_a_request() {
    let server = Server::start(Vec::new());
    let connected = Connected::new(&server, "rotating", &["read"]);
    let refresher = connected.refresher("owner-one", &server);
    let issued = connected.issue(&refresher, Delivery::Revealed).unwrap();
    assert_eq!(issued.token.expose(), ACCESS);
    assert!(!issued.refreshed);
    assert_eq!(issued.scopes, ["read"]);
    assert!(issued.expires_at_ms > now() + 3_000_000);
    assert!(server.requests().is_empty(), "no request");
    assert!(connected.attempts().is_empty(), "no refresh attempt");
    assert_eq!(connected.issues().len(), 1);
    assert_recorded(
        &connected.recorded(&issued),
        &issued,
        &connected.active().generation,
        None,
        "revealed",
    );
    let shown = format!("{issued:?} {issued:#?}");
    assert!(!shown.contains(ACCESS), "Debug shows no token");
}

#[test]
fn owner_issue_within_the_lead_refreshes_once() {
    let server = Server::start(vec![token(Some(ROTATED), Some("read"))]);
    // Valid for 30 s, inside the declared 60 s lead.
    let connected = Connected::with(&server, "rotating", &["read"], 30_000, true);
    let refresher = connected.refresher("owner-one", &server);
    let first = connected.issue(&refresher, Delivery::Library).unwrap();
    assert!(first.refreshed);
    assert_eq!(first.token.expose(), REFRESHED_ACCESS);
    refreshed_request(&server);
    let second = connected.issue(&refresher, Delivery::MetadataOnly).unwrap();
    assert!(!second.refreshed);
    assert_eq!(second.token.expose(), REFRESHED_ACCESS);
    assert_eq!(second.expires_at_ms, first.expires_at_ms);
    assert_eq!(server.requests().len(), 1, "the second issue sent nothing");
    let (attempt, _) = connected.attempt();
    assert_eq!(attempt.state, RefreshState::Published);
    let generation = connected.active().generation;
    assert_eq!(connected.issues().len(), 2);
    assert_recorded(
        &connected.recorded(&first),
        &first,
        &generation,
        Some(&attempt.id),
        "library",
    );
    assert_recorded(
        &connected.recorded(&second),
        &second,
        &generation,
        None,
        "metadata_only",
    );
}

/// The caller's deadline is checked again after the refresh the issue ran:
/// one that passed meanwhile records no issue and answers `timeout`, while
/// the refresh it ran stays published for the next issue.
#[test]
fn owner_issue_whose_deadline_passes_during_its_refresh_records_nothing() {
    let server = Server::start(vec![token(Some(ROTATED), Some("read"))]);
    let connected = Connected::with(&server, "rotating", &["read"], 30_000, true);
    let refresher = connected.refresher("owner-one", &server);
    let refused = refresher
        .issue_access_token(
            INSTANCE,
            &connected.binding.adapter_id,
            &connected.connection,
            Delivery::Revealed,
            now(),
        )
        .unwrap_err();
    assert_eq!(refused, Refusal::Deadline);
    assert_eq!(Error::from(refused).code, Code::Timeout);
    refreshed_request(&server);
    assert!(connected.issues().is_empty(), "nothing was delivered");
    let (attempt, _) = connected.attempt();
    assert_eq!(attempt.state, RefreshState::Published);
    let next = connected.issue(&refresher, Delivery::Revealed).unwrap();
    assert_eq!(next.token.expose(), REFRESHED_ACCESS);
    assert_eq!(server.requests().len(), 1, "the next issue sent nothing");
}

#[test]
fn owner_issue_on_a_stale_connection_refuses_and_records_nothing() {
    let server = Server::start(vec![error(400, "invalid_grant")]);
    let connected = Connected::with(&server, "rotating", &["read"], 30_000, true);
    let refresher = connected.refresher("owner-one", &server);
    let reauthorize = Refusal::Refresh(Outcome::ReauthorizationRequired);
    assert_eq!(
        connected.issue(&refresher, Delivery::Revealed).unwrap_err(),
        reauthorize
    );
    assert_eq!(connected.described().state, State::Stale);
    // Stale: refused before any request.
    assert_eq!(
        connected.issue(&refresher, Delivery::Revealed).unwrap_err(),
        reauthorize
    );
    assert_eq!(server.requests().len(), 1);
    assert!(connected.issues().is_empty(), "a refusal records nothing");
    assert_eq!(Error::from(reauthorize).code, Code::ServiceFailure);
    assert_eq!(
        Error::from(Refusal::Refresh(Outcome::Uncertain)).code,
        Code::ServiceFailure
    );
}

#[test]
fn owner_issue_rate_limited_is_unavailable_with_its_retry_after() {
    let server = Server::start(vec![Reply {
        status: 429,
        body: json!({"error": "slow_down"}),
        retry_after: Some("7"),
    }]);
    let connected = Connected::with(&server, "rotating", &["read"], 30_000, true);
    let refresher = connected.refresher("owner-one", &server);
    let refused = connected.issue(&refresher, Delivery::Revealed).unwrap_err();
    assert_eq!(
        refused,
        Refusal::Refresh(Outcome::RateLimited {
            retry_after_ms: Some(7_000)
        })
    );
    assert_eq!(connected.described().state, State::Ready);
    assert!(connected.issues().is_empty());
    let error = Error::from(refused);
    assert_eq!(error.code, Code::Unavailable);
    assert_eq!(
        error.service_code,
        Some(connectors_core::ErrorCode::RateLimited)
    );
    assert_eq!(error.retry_after_seconds, Some(7));
}

#[test]
fn owner_issue_on_a_static_profile_is_forbidden() {
    let server = Server::start(Vec::new());
    let connected = Connected::with(&server, "rotating", &["read"], 3_600_000, false);
    let refresher = connected.refresher("owner-one", &server);
    let refused = connected.issue(&refresher, Delivery::Revealed).unwrap_err();
    assert_eq!(refused, Refusal::NotGrant);
    assert_eq!(Error::from(refused).code, Code::Forbidden);
    assert!(server.requests().is_empty());
    assert!(connected.issues().is_empty());
}

#[test]
fn owner_issue_of_a_revoked_or_unselected_connection_is_refused() {
    let server = Server::start(Vec::new());
    let connected = Connected::new(&server, "rotating", &["read"]);
    let refresher = connected.refresher("owner-one", &server);
    assert_eq!(
        refresher
            .issue_access_token(
                "another-instance",
                &connected.binding.adapter_id,
                &connected.connection,
                Delivery::Revealed,
                u64::MAX,
            )
            .unwrap_err(),
        Refusal::Registry(Failure::NotFound)
    );
    Registry::new(&connected.path)
        .revoke(
            INSTANCE,
            &connected.binding.adapter_id,
            &connected.connection,
            &connected.described().revision,
            now(),
        )
        .unwrap();
    let refused = connected.issue(&refresher, Delivery::Revealed).unwrap_err();
    assert_eq!(refused, Refusal::Registry(Failure::Revoked));
    assert_eq!(Error::from(refused).code, Code::Revoked);
    assert!(server.requests().is_empty());
    assert!(connected.issues().is_empty());
}

// Adversary cases (wave 20261025-oauth2, unit U5, pass 1).

/// contracts/cli/v1alpha1/semantics.md "Access token issue": a refresh whose
/// granted scopes no longer cover the profile is refused as `not_granted`
/// (next action `repair_connection`), as every other owner path maps a scope
/// refusal. `insufficient_scope` is no `connectors.cli.FailureCode`
/// (`ess/domains/cli.yaml`), so a `connections token` envelope carrying it
/// names a code outside the CLI's closed vocabulary.
#[test]
fn adversary_owner_issue_whose_refresh_widens_scope_is_not_granted() {
    let server = Server::start(vec![token(Some(ROTATED), Some("read write"))]);
    let connected = Connected::with(&server, "rotating", &["read"], 30_000, true);
    let refresher = connected.refresher("owner-one", &server);
    let refused = connected.issue(&refresher, Delivery::Revealed).unwrap_err();
    assert_eq!(refused, Refusal::Refresh(Outcome::InsufficientScope));
    assert_eq!(server.requests().len(), 1);
    assert!(connected.issues().is_empty(), "a refusal records nothing");
    let error = Error::from(refused);
    assert_eq!(
        serde_json::to_value(error.code).unwrap(),
        json!("not_granted"),
        "the CLI contract's code for a refresh that no longer covers the profile"
    );
}

/// How many issues run at once in one trial, and how many trials run.
const CALLERS: usize = 16;
const TRIALS: usize = 8;

/// contracts/auth/acquisition/v1alpha1/semantics.md §4.4 item 2: an issue
/// inside the lead runs the §4.1 sequence "under the same `connectors.refresh`
/// ledger serialization as every other refresh", and "an issue never runs a
/// second exchange". The story's design takes the expiry decision under that
/// serialization. `Registry::issuance` reads the expiry in one transaction
/// and `refresh_connection` reserves in another, so a caller that read the
/// expiring generation before another caller's refresh published reserves the
/// freshly published generation as its source and sends a second exchange
/// for a token that no longer needed one. Each trial starts `CALLERS` issues
/// of one connection inside the lead together; the token endpoint must see
/// exactly one request per trial.
#[test]
fn adversary_concurrent_issues_within_the_lead_send_one_exchange() {
    let mut extra = Vec::new();
    for trial in 0..TRIALS {
        let server = Server::start(
            (0..CALLERS)
                .map(|_| token(Some(ROTATED), Some("read")))
                .collect(),
        );
        // Valid for 30 s, inside the declared 60 s lead.
        let connected = Connected::with(&server, "rotating", &["read"], 30_000, true);
        let refresher = connected.refresher("owner-one", &server);
        let start = Barrier::new(CALLERS);
        let issued = std::thread::scope(|scope| {
            let callers = (0..CALLERS)
                .map(|caller| {
                    let (start, connected, refresher) = (&start, &connected, &refresher);
                    scope.spawn(move || {
                        start.wait();
                        // Spread the arrivals across the leader's exchange.
                        std::thread::sleep(Duration::from_millis(3 * caller as u64));
                        connected.issue(refresher, Delivery::MetadataOnly)
                    })
                })
                .collect::<Vec<_>>();
            callers
                .into_iter()
                .map(|caller| caller.join().unwrap())
                .collect::<Vec<_>>()
        });
        let requests = server.requests().len();
        if requests != 1 {
            let outcomes: Vec<_> = issued
                .iter()
                .map(|issue| issue.as_ref().map(|issued| issued.refreshed))
                .collect();
            extra.push(format!(
                "trial {trial}: {requests} exchanges, {} attempts, outcomes {outcomes:?}",
                connected.attempts().len()
            ));
        }
    }
    assert!(
        extra.is_empty(),
        "one expiring token, one exchange per trial: {extra:#?}"
    );
}

/// A published connection of a `confidential` client (`connect` admits one
/// with its client secret document, `oauth2_grant::grant`) whose access
/// token expires in `expires_in_ms`, inside the declared 60 s lead.
fn confidential(server: &Server, expires_in_ms: u64) -> Connected {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("state");
    crate::local::filesystem::directory(&path, true, true).unwrap();
    drop(Metadata::initialize(&path).unwrap());
    let mut binding = registry::fixture_binding(INSTANCE);
    binding.profile.oauth2 = Some(
        serde_json::from_value(json!({
            "authorize_url": "https://localhost/fixture/authorize",
            "token_url": server.url(),
            "device_authorization_url": null,
            "client_id": CLIENT,
            "client_kind": "confidential",
            "request_encoding": "form",
            "refresh_encoding": "form",
            "redirect_kind": "loopback_any_port",
            "redirect_uri": null,
            "authorize_parameters": [],
            "requested_scopes": ["read"],
            "required_scopes": ["read"],
            "rotation": "rotating",
            "refresh_lead_seconds": 60,
            "grant_types": ["authorization_code"],
            "sources": ["https://localhost/fixture-authorization-server"],
            "state_in_exchange": false,
        }))
        .unwrap(),
    );
    let registry = Registry::new(&path);
    let at = now();
    let entry = serde_json::to_vec(&json!({
        "client_id": CLIENT,
        "access_token": ACCESS,
        "refresh_token": REFRESH,
        "expires_at_ms": at + expires_in_ms,
        "scopes": ["read"],
    }))
    .unwrap();
    let acquisition = registry.begin(&binding, at).unwrap();
    let claim = registry.consume(acquisition, at).unwrap();
    let baseline = ValidatedBaseline {
        identity: ExternalIdentity {
            kind: "fixture-account".into(),
            subject: "one".into(),
        },
        granted_scopes: Some(["read".to_owned()].into_iter().collect()),
        credential_expires_at_ms: Some(at + expires_in_ms),
        collected_at_ms: at,
        valid_until_ms: at + expires_in_ms.min(60_000),
        binding: None,
    };
    let prepared = registry.prepare(&claim, baseline, entry.len(), at).unwrap();
    let custody = Arc::new(Memory::default());
    custody
        .versions
        .lock()
        .unwrap()
        .insert(prepared.version().id(), entry);
    let receipt = custody::WrittenVersion::fixture(prepared.version());
    let stored = registry.acknowledge(prepared, receipt, at).unwrap();
    let connection = registry.publish(stored, at).unwrap();
    Connected {
        _root: root,
        path,
        binding,
        connection,
        custody,
    }
}

/// The owner failures contracts/auth/acquisition/v1alpha1/semantics.md §4.4
/// names for a refused issue.
const ISSUE_REFUSALS: [Code; 7] = [
    Code::Forbidden,
    Code::ServiceFailure,
    Code::InsufficientScope,
    Code::NotGranted,
    Code::Revoked,
    Code::CustodyUnavailable,
    Code::Unavailable,
];

/// An issue inside the lead of a confidential client's connection: the
/// refresh it needs refuses before any request (`reserve_refresh`: "A
/// confidential client's secret is not part of the stored entry"), and the
/// issue answers that as `invalid_input`, which the CLI reports as a usage
/// error at stage `arguments` for a call whose arguments were right. §4.4
/// maps every refusal of an issue onto its table; `invalid_input` is not in it.
#[test]
fn adversary_owner_issue_of_a_confidential_client_inside_the_lead_is_no_usage_error() {
    let server = Server::start(Vec::new());
    let connected = confidential(&server, 30_000);
    let refresher = connected.refresher("owner-one", &server);
    let refused = connected.issue(&refresher, Delivery::Revealed).unwrap_err();
    assert!(server.requests().is_empty());
    assert!(connected.issues().is_empty(), "a refusal records nothing");
    let error = Error::from(refused);
    assert!(
        ISSUE_REFUSALS.contains(&error.code),
        "{refused:?} answers {:?}, outside §4.4's refusal table",
        error.code
    );
}

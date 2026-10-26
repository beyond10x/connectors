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

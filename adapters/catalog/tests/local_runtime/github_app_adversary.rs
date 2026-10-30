//! Conformance checks of the `github_app` profile's own contract
//! (story:catalog-github-app-installation-token), as a child of the
//! acceptance module so they reuse its fixture: the exchange answer is
//! bounded before its token is sent anywhere, the token cache never lets one
//! entry's token serve another, and a refused entry names none of itself.
use super::*;

/// One operation whose exchange answered `status` with `body`: the refusal it
/// ends in, given that nothing but the exchange may have been requested.
fn refused_after_exchange(github: &GitHub, status: u16, body: Value) -> Failure {
    github.set_exchange(Exchange::Refuse(status, body));
    let mut child = github.spawn();
    let before = github.requests().len();
    let refusal = project(&mut child, &entry()).expect_err("an operation followed the exchange");
    let after = github.requests();
    assert_eq!(
        after.len(),
        before + 1,
        "only the exchange was requested for a {status} answer; requested {:?}",
        after[before..]
            .iter()
            .map(|seen| format!("{} {}", seen.method, seen.route))
            .collect::<Vec<_>>()
    );
    assert_eq!(after.last().unwrap().route, EXCHANGE_ROUTE);
    refusal
}

fn rfc3339(offset_s: i64) -> String {
    (time::OffsetDateTime::now_utc() + time::Duration::seconds(offset_s))
        .replace_nanosecond(0)
        .unwrap()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}

/// An installation token the issuer says has already expired is no token: as
/// an OAuth `expires_in` of 0 is a protocol failure, so is an `expires_at` at
/// or before now. It must not be sent to the operation.
#[test]
fn an_installation_token_already_expired_is_refused_with_no_operation_request() {
    let github = GitHub::new();
    for seconds in [-3600, -60, 0] {
        github.set_exchange(Exchange::Issue(seconds));
        let mut child = github.spawn();
        let api_before = github.api_authorizations().len();
        let outcome = project(&mut child, &entry());
        assert!(
            matches!(outcome, Err(Failure::Protocol)),
            "a token that expired {} s ago was answered: {:?}",
            -seconds,
            outcome.as_ref().map(|_| "the operation succeeded")
        );
        assert_eq!(
            github.api_authorizations().len(),
            api_before,
            "an expired installation token (expires_at now{seconds:+} s) reached the API"
        );
    }
}

/// Every answer outside the documented shape is `protocol` before its token
/// travels: a missing, null or malformed `expires_at`, a missing or empty
/// token, a token no header can carry, an oversized answer, a redirect.
#[test]
fn an_exchange_answer_outside_its_shape_is_a_protocol_failure_with_no_operation_request() {
    let github = GitHub::new();
    let valid = rfc3339(3600);
    let token = format!("{INSTALLATION_TOKEN}x");
    let cases = [
        (201, json!({"token": token})),
        (201, json!({"token": token, "expires_at": null})),
        (201, json!({"token": token, "expires_at": "tomorrow"})),
        (201, json!({"token": token, "expires_at": 1_900_000_000})),
        (201, json!({"token": token, "expires_at": ""})),
        (201, json!({"expires_at": valid})),
        (201, json!({"token": "", "expires_at": valid})),
        (201, json!({"token": null, "expires_at": valid})),
        (201, json!({"token": "split token", "expires_at": valid})),
        (
            201,
            json!({"token": "line\r\nX-Injected: 1", "expires_at": valid}),
        ),
        (201, json!({"token": "del\u{7f}", "expires_at": valid})),
        (201, json!({"token": "t\u{f6}ken", "expires_at": valid})),
        (201, json!({"token": "x".repeat(8193), "expires_at": valid})),
        (
            201,
            json!({"token": token, "expires_at": valid, "padding": "x".repeat(70_000)}),
        ),
        (200, json!("token")),
        (302, json!({"token": token, "expires_at": valid})),
        (307, json!({"token": token, "expires_at": valid})),
        (204, json!({})),
    ];
    for (index, (status, body)) in cases.into_iter().enumerate() {
        let started = std::time::Instant::now();
        let failure = refused_after_exchange(&github, status, body);
        assert_eq!(failure, Failure::Protocol, "case {index}: {status}");
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "case {index}: {status} took {:?}",
            started.elapsed()
        );
    }
    assert!(
        github.api_authorizations().is_empty(),
        "no operation request follows an answer outside its shape"
    );
}

/// The exchange answers an object (`apps/create-installation-access-token`).
/// A JSON array holding a token and an expiry is not that answer, and its
/// first string must not be sent anywhere as a bearer token.
#[test]
fn an_exchange_answer_that_is_an_array_is_a_protocol_failure_with_no_operation_request() {
    let github = GitHub::new();
    let token = format!("{INSTALLATION_TOKEN}from-an-array");
    let failure = refused_after_exchange(&github, 201, json!([token, rfc3339(3600)]));
    assert_eq!(failure, Failure::Protocol);
}

/// The cache key is a digest of the entry: an entry differing in its App id,
/// its installation id or its key's bytes never reuses another's token. An
/// entry naming another installation is exchanged for that installation and
/// refused there, not served the first installation's token.
#[test]
fn entries_differing_in_app_id_installation_or_key_never_share_an_installation_token() {
    let github = GitHub::new();
    let mut child = github.spawn();
    let key = app_key();
    project(&mut child, &entry()).unwrap();
    assert_eq!(github.exchanges().len(), 1);
    let other_app = entry_of(json!({
        "app_id": "424243",
        "installation_id": INSTALLATION_ID,
        "private_key": pem(PKCS1, &key.pkcs1),
    }));
    project(&mut child, &other_app).unwrap();
    assert_eq!(github.exchanges().len(), 2, "another App id exchanged anew");
    let other_key_bytes = entry_of(json!({
        "app_id": APP_ID,
        "installation_id": INSTALLATION_ID,
        "private_key": pem(PKCS8, &key.pkcs8),
    }));
    project(&mut child, &other_key_bytes).unwrap();
    assert_eq!(github.exchanges().len(), 3, "another key exchanged anew");
    let api = github.api_authorizations();
    assert_eq!(
        api,
        [1, 2, 3]
            .map(|n| Some(format!("Bearer {INSTALLATION_TOKEN}{n}")))
            .to_vec(),
        "each entry carried its own installation token"
    );
    let other_installation = entry_of(json!({
        "app_id": APP_ID,
        "installation_id": INSTALLATION_ID + 1,
        "private_key": pem(PKCS1, &key.pkcs1),
    }));
    assert!(matches!(
        project(&mut child, &other_installation),
        Err(Failure::InvalidCredential)
    ));
    let last = github.requests().pop().unwrap();
    assert_eq!(
        last.route,
        format!(
            "/api/v4/app/installations/{}/access_tokens",
            INSTALLATION_ID + 1
        ),
        "the other installation was exchanged for itself"
    );
    let operations = github
        .requests()
        .into_iter()
        .filter(|seen| !seen.route.ends_with("/access_tokens"))
        .count();
    assert_eq!(
        operations, 3,
        "no cached token served the other installation"
    );
}

/// Further entries refused before any request: an empty or oversized
/// installation id, a non-ASCII digit, a fraction, a null App id, a key of
/// whitespace. Each refusal names no part of the entry.
#[test]
fn further_malformed_github_app_entries_are_refused_with_no_request_and_no_reason() {
    let github = GitHub::new();
    let mut child = github.spawn();
    let key = pem(PKCS1, &app_key().pkcs1);
    let mut corrupted = key.clone().into_bytes();
    corrupted[300] = if corrupted[300] == b'A' { b'B' } else { b'A' };
    let corrupted = String::from_utf8(corrupted).unwrap();
    let documents = [
        json!({"app_id": APP_ID, "installation_id": "", "private_key": key}),
        json!({"app_id": APP_ID, "installation_id": "99999999999999999999", "private_key": key}),
        json!({"app_id": APP_ID, "installation_id": "\u{ff11}", "private_key": key}),
        json!({"app_id": APP_ID, "installation_id": 1.5, "private_key": key}),
        json!({"app_id": APP_ID, "installation_id": -1, "private_key": key}),
        json!({"app_id": null, "installation_id": INSTALLATION_ID, "private_key": key}),
        json!({"app_id": APP_ID, "installation_id": INSTALLATION_ID, "private_key": " \n\t "}),
        json!({"app_id": APP_ID, "installation_id": INSTALLATION_ID, "private_key": corrupted}),
        json!({"app_id": APP_ID, "installation_id": INSTALLATION_ID, "private_key": format!("{key}{key}")}),
    ];
    let revision = child.bootstrap().descriptor().unwrap().revision;
    for (index, document) in documents.into_iter().enumerate() {
        let document = entry_of(document);
        let refusal = child
            .invoke_explained(
                "project.get",
                &revision,
                "one",
                &document,
                &serde_json::to_vec(&json!({"id": "org/project"})).unwrap(),
                deadline(),
            )
            .unwrap_err();
        assert_eq!(refusal.failure, Failure::InvalidInput, "entry {index}");
        assert_eq!(refusal.reason, None, "entry {index} explained");
        assert!(!carries_app_material(format!("{refusal:?}").as_bytes()));
    }
    assert!(github.requests().is_empty(), "no request for a bad entry");
}

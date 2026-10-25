//! The closed failure enum: how token-endpoint answers map onto it, and what a failure carries.
use crate::support::{ACCESS_TOKEN, FakeServer, REFRESH_TOKEN, Reply, declaration};
use connectors_oauth2::{Client, Failure, RESPONSE_LIMIT, Secret};
use std::time::{Duration, SystemTime};

/// One refresh against a server that answers `reply`.
fn refresh_against(reply: Reply) -> Result<(), Failure> {
    let server = FakeServer::start(vec![reply]);
    let client = Client::new(declaration(Some(&server)), None, None, server.transport()).unwrap();
    client
        .refresh(
            &Secret::new(REFRESH_TOKEN.to_owned()),
            &["fixture.read".to_owned()],
        )
        .map(|_| ())
}

#[test]
fn invalid_grant_is_reauthorization_required() {
    for reply in [
        Reply::error(400, "invalid_grant"),
        Reply::error(400, "invalid_client"),
        Reply::error(401, "invalid_client"),
        Reply::text(401, "unauthorized"),
    ] {
        assert_eq!(
            refresh_against(reply).err(),
            Some(Failure::ReauthorizationRequired)
        );
    }
    assert_eq!(
        refresh_against(Reply::error(400, "access_denied")).err(),
        Some(Failure::AccessDenied)
    );
    assert_eq!(
        refresh_against(Reply::error(400, "invalid_request")).err(),
        Some(Failure::Rejected)
    );
}

#[test]
fn status_429_is_rate_limited_with_retry_after() {
    assert_eq!(
        refresh_against(Reply::error(429, "slow_down").header("Retry-After", "120")).err(),
        Some(Failure::RateLimited {
            retry_after: Some(Duration::from_secs(120))
        })
    );
    assert_eq!(
        refresh_against(Reply::text(429, "busy")).err(),
        Some(Failure::RateLimited { retry_after: None })
    );
    // An HTTP-date is read as the time until then.
    let later = SystemTime::now() + Duration::from_secs(600);
    let Some(Failure::RateLimited {
        retry_after: Some(after),
    }) = refresh_against(
        Reply::text(429, "busy").header("Retry-After", &httpdate::fmt_http_date(later)),
    )
    .err()
    else {
        panic!("an HTTP-date Retry-After is rate_limited with a delay");
    };
    assert!(after > Duration::from_secs(590) && after <= Duration::from_secs(600));
    // A value that is neither is no delay at all.
    assert_eq!(
        refresh_against(Reply::text(429, "busy").header("Retry-After", "soon")).err(),
        Some(Failure::RateLimited { retry_after: None })
    );
}

#[test]
fn status_503_is_unavailable() {
    for reply in [
        Reply::text(503, "maintenance"),
        Reply::text(500, "error"),
        Reply::json(502, serde_json::json!({"error": "invalid_grant"})),
    ] {
        assert_eq!(refresh_against(reply).err(), Some(Failure::Unavailable));
    }
    // A transport that cannot reach the server is unavailable too.
    let server = FakeServer::start(Vec::new());
    let mut declared = declaration(Some(&server));
    let closed = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    declared.token_url = format!("https://localhost:{closed}/token");
    let client = Client::new(declared, None, None, server.transport()).unwrap();
    assert_eq!(
        client
            .refresh(&Secret::new(REFRESH_TOKEN.to_owned()), &[])
            .err(),
        Some(Failure::Unavailable)
    );
}

#[test]
fn oversized_response_is_refused() {
    // A token response one byte over the bound, padded inside a valid JSON object.
    let padding = "x".repeat(RESPONSE_LIMIT);
    let reply = Reply::json(
        200,
        serde_json::json!({
            "access_token": ACCESS_TOKEN,
            "token_type": "bearer",
            "expires_in": 3600,
            "padding": padding,
        }),
    );
    assert_eq!(refresh_against(reply).err(), Some(Failure::InvalidResponse));
    // An error body over the bound is refused the same way, before its status is read.
    let reply = Reply::text(400, &"x".repeat(RESPONSE_LIMIT + 1));
    assert_eq!(refresh_against(reply).err(), Some(Failure::InvalidResponse));
    // At the bound it is still read.
    let mut body = serde_json::json!({
        "access_token": ACCESS_TOKEN,
        "token_type": "bearer",
        "expires_in": 3600,
        "padding": "",
    });
    let size = serde_json::to_vec(&body).unwrap().len();
    body["padding"] = "x".repeat(RESPONSE_LIMIT - size).into();
    assert_eq!(serde_json::to_vec(&body).unwrap().len(), RESPONSE_LIMIT);
    assert_eq!(refresh_against(Reply::json(200, body)), Ok(()));
    // Missing, malformed or non-bearer fields are refused too.
    for body in [
        serde_json::json!({"token_type": "bearer", "expires_in": 3600}),
        serde_json::json!({"access_token": ACCESS_TOKEN, "token_type": "mac", "expires_in": 3600}),
        serde_json::json!({"access_token": ACCESS_TOKEN, "expires_in": 3600}),
        serde_json::json!({"access_token": ACCESS_TOKEN, "token_type": "bearer"}),
        serde_json::json!({"access_token": ACCESS_TOKEN, "token_type": "bearer", "expires_in": 0}),
        serde_json::json!({"access_token": ACCESS_TOKEN, "token_type": "bearer", "expires_in": -5}),
        serde_json::json!({"access_token": "", "token_type": "bearer", "expires_in": 3600}),
    ] {
        assert_eq!(
            refresh_against(Reply::json(200, body.clone())).err(),
            Some(Failure::InvalidResponse),
            "{body}"
        );
    }
}

/// No failure, in its `Display` or its `Debug`, carries anything the server wrote.
#[test]
fn failure_carries_no_response_text() {
    const MARK: &str = "fixture-provider-text-7f3a";
    let replies = [
        Reply::json(
            400,
            serde_json::json!({"error": "invalid_grant", "error_description": MARK}),
        ),
        Reply::json(
            400,
            serde_json::json!({"error": format!("{MARK}_code"), "error_description": MARK}),
        ),
        Reply::text(401, MARK),
        Reply::text(429, MARK).header("Retry-After", MARK),
        Reply::text(503, MARK),
        Reply::text(200, MARK),
        Reply::json(
            200,
            serde_json::json!({"access_token": MARK, "token_type": MARK, "expires_in": 3600}),
        ),
        Reply::json(
            200,
            serde_json::json!({"access_token": ACCESS_TOKEN, "token_type": "bearer", "expires_in": MARK}),
        ),
    ];
    for reply in replies {
        let failure = refresh_against(reply).unwrap_err();
        for text in [failure.to_string(), format!("{failure:?}")] {
            assert!(!text.contains(MARK), "{text}");
            assert!(!text.contains(ACCESS_TOKEN), "{text}");
            assert!(!text.contains(REFRESH_TOKEN), "{text}");
        }
        // Display is the closed code alone.
        assert_eq!(failure.to_string(), failure.code());
    }
}

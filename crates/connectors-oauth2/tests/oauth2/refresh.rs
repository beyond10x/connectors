//! The refresh exchange: rotation, kept refresh tokens and kept scopes.
use crate::support::{
    ACCESS_TOKEN, CLIENT_ID, FakeServer, REFRESH_TOKEN, ROTATED_REFRESH_TOKEN, Reply, TlsTransport,
    declaration,
};
use connectors_oauth2::{Client, Failure, RequestEncoding, Secret};
use std::time::Duration;

fn granted() -> Vec<String> {
    vec!["fixture.read".to_owned(), "fixture.write".to_owned()]
}

fn client(server: &FakeServer) -> Client<TlsTransport> {
    Client::new(declaration(Some(server)), None, None, server.transport()).unwrap()
}

fn old() -> Secret {
    Secret::new(REFRESH_TOKEN.to_owned())
}

#[test]
fn refresh_returns_a_rotated_token() {
    let server = FakeServer::start(vec![Reply::token(
        ACCESS_TOKEN,
        Some(ROTATED_REFRESH_TOKEN),
        Some("fixture.read fixture.write"),
    )]);
    let mut declared = declaration(Some(&server));
    // The refresh request follows `refresh_encoding`, not `request_encoding`.
    declared.request_encoding = RequestEncoding::Form;
    declared.refresh_encoding = RequestEncoding::Json;
    let client = Client::new(declared, None, None, server.transport()).unwrap();
    let refreshed = client.refresh(&old(), &granted()).unwrap();
    assert_eq!(refreshed.access_token.expose(), ACCESS_TOKEN);
    assert_eq!(refreshed.refresh_token.expose(), ROTATED_REFRESH_TOKEN);
    assert!(refreshed.rotated);
    assert_eq!(refreshed.expires_in, Duration::from_secs(3600));
    let observed = server.observed();
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].path, "/token");
    assert_eq!(
        observed[0].content_type.as_deref(),
        Some("application/json")
    );
    let fields = observed[0].fields();
    assert_eq!(fields["grant_type"], "refresh_token");
    assert_eq!(fields["refresh_token"], REFRESH_TOKEN);
    assert_eq!(fields["client_id"], CLIENT_ID);
    assert_eq!(fields.len(), 3, "{:?}", fields.keys());
    let shown = format!("{refreshed:?}");
    assert!(!shown.contains(ACCESS_TOKEN) && !shown.contains(ROTATED_REFRESH_TOKEN));
    // The same refresh token sent back is not a rotation.
    let server = FakeServer::start(vec![Reply::token(ACCESS_TOKEN, Some(REFRESH_TOKEN), None)]);
    let refreshed = self::client(&server).refresh(&old(), &granted()).unwrap();
    assert!(!refreshed.rotated);
    assert_eq!(refreshed.refresh_token.expose(), REFRESH_TOKEN);
}

#[test]
fn refresh_without_refresh_token_keeps_the_old_one() {
    let server = FakeServer::start(vec![Reply::token(ACCESS_TOKEN, None, None)]);
    let refreshed = client(&server).refresh(&old(), &granted()).unwrap();
    assert_eq!(refreshed.access_token.expose(), ACCESS_TOKEN);
    assert_eq!(refreshed.refresh_token.expose(), REFRESH_TOKEN);
    assert!(!refreshed.rotated);
}

#[test]
fn refresh_without_scope_keeps_granted_scopes() {
    let server = FakeServer::start(vec![
        Reply::token(ACCESS_TOKEN, None, None),
        Reply::token(ACCESS_TOKEN, None, Some("fixture.read fixture.admin")),
    ]);
    let client = client(&server);
    let previous = vec![
        "fixture.read".to_owned(),
        "fixture.profile".to_owned(),
        "fixture.write".to_owned(),
    ];
    let refreshed = client.refresh(&old(), &previous).unwrap();
    assert_eq!(
        refreshed.scopes, previous,
        "omitted scope keeps the grant's"
    );
    let refreshed = client.refresh(&old(), &previous).unwrap();
    assert_eq!(
        refreshed.scopes,
        ["fixture.read", "fixture.admin"],
        "a sent scope replaces the grant's"
    );
}

#[test]
fn missing_required_scope_is_insufficient_scope() {
    // A refresh that narrows the grant below the required scopes.
    let server = FakeServer::start(vec![Reply::token(
        ACCESS_TOKEN,
        None,
        Some("fixture.write"),
    )]);
    assert_eq!(
        client(&server).refresh(&old(), &granted()).err(),
        Some(Failure::InsufficientScope)
    );
    // A refresh that omits scope over a previous grant that already lacked it.
    let server = FakeServer::start(vec![Reply::token(ACCESS_TOKEN, None, None)]);
    assert_eq!(
        client(&server)
            .refresh(&old(), &["fixture.write".to_owned()])
            .err(),
        Some(Failure::InsufficientScope)
    );
    // A device grant whose token response grants less than required.
    let server = FakeServer::start(vec![
        Reply::json(
            200,
            serde_json::json!({
                "device_code": "fixture-device-code",
                "user_code": "FIXT-CODE",
                "verification_uri": "https://localhost/device",
                "expires_in": 600,
                "interval": 1,
            }),
        ),
        Reply::token(ACCESS_TOKEN, Some(REFRESH_TOKEN), Some("fixture.write")),
    ]);
    let client = client(&server);
    let device = client.device_authorize().unwrap();
    assert_eq!(
        client
            .poll_device(&device, &crate::device::Recorded::default())
            .err(),
        Some(Failure::InsufficientScope)
    );
}

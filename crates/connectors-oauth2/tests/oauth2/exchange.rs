//! The authorization-code exchange: what each client kind and encoding sends.
use crate::support::{
    ACCESS_TOKEN, CLIENT_ID, CLIENT_SECRET, FakeServer, Observed, REFRESH_TOKEN, Reply,
    TlsTransport, declaration, query_value,
};
use connectors_oauth2::{
    Authorization, AuthorizationCode, Client, ClientKind, Failure, RedirectKind, RequestEncoding,
    Secret, ServerDeclaration, pasted_code, pkce_challenge,
};
use std::time::Duration;

const PASTE_REDIRECT: &str = "https://localhost:1/code/callback";
const CODE: &str = "fixture-authorization-code-one";

/// A declaration whose redirect is a pasted `code#state` line.
fn pasted(server: &FakeServer) -> ServerDeclaration {
    let mut declared = declaration(Some(server));
    declared.redirect_kind = RedirectKind::PastedCode;
    declared.redirect_uri = Some(PASTE_REDIRECT.to_owned());
    declared
}

/// Begins an authorization and receives its code as the pasted line the server shows.
fn authorize(client: &Client<TlsTransport>) -> (Authorization, AuthorizationCode, String) {
    let authorization = client.begin(PASTE_REDIRECT).unwrap();
    let state = query_value(authorization.url(), "state");
    let code = pasted_code(&authorization, &format!("{CODE}#{state}\n")).unwrap();
    (authorization, code, state)
}

/// The fields every code exchange carries, and the PKCE verifier that matches the challenge.
fn assert_code_exchange(request: &Observed, challenge: &str) {
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/token");
    let fields = request.fields();
    assert_eq!(fields["grant_type"], "authorization_code");
    assert_eq!(fields["code"], CODE);
    assert_eq!(fields["redirect_uri"], PASTE_REDIRECT);
    assert_eq!(fields["client_id"], CLIENT_ID);
    let verifier = &fields["code_verifier"];
    assert!((43..=128).contains(&verifier.len()), "verifier length");
    assert_eq!(pkce_challenge(verifier), challenge);
}

fn exchange_public(encoding: RequestEncoding, content_type: &str) {
    let server = FakeServer::start(vec![Reply::token(ACCESS_TOKEN, Some(REFRESH_TOKEN), None)]);
    let mut declared = pasted(&server);
    declared.request_encoding = encoding;
    let client = Client::new(declared, None, None, server.transport()).unwrap();
    let (authorization, code, _) = authorize(&client);
    let challenge = query_value(authorization.url(), "code_challenge");
    let granted = client.exchange(authorization, code).unwrap();
    assert_eq!(granted.access_token.expose(), ACCESS_TOKEN);
    assert_eq!(
        granted.refresh_token.as_ref().map(Secret::expose),
        Some(REFRESH_TOKEN)
    );
    assert_eq!(granted.expires_in, Duration::from_secs(3600));
    assert_eq!(granted.scopes, ["fixture.read", "fixture.write"]);
    let observed = server.observed();
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].content_type.as_deref(), Some(content_type));
    assert_code_exchange(&observed[0], &challenge);
    let fields = observed[0].fields();
    assert!(
        !fields.contains_key("client_secret"),
        "public client sent a secret"
    );
    assert_eq!(
        fields.len(),
        5,
        "RFC 6749 §4.1.3 fields and the verifier only: {:?}",
        fields.keys()
    );
}

#[test]
fn public_client_exchange_sends_no_secret_form() {
    exchange_public(RequestEncoding::Form, "application/x-www-form-urlencoded");
}

#[test]
fn public_client_exchange_sends_no_secret_json() {
    exchange_public(RequestEncoding::Json, "application/json");
}

#[test]
fn confidential_client_exchange_sends_the_secret() {
    let server = FakeServer::start(vec![Reply::token(ACCESS_TOKEN, Some(REFRESH_TOKEN), None)]);
    let mut declared = pasted(&server);
    declared.client_kind = ClientKind::Confidential;
    // A confidential client without its secret, or a public one given a secret, is refused
    // before anything is sent.
    assert_eq!(
        Client::new(declared.clone(), None, None, server.transport()).err(),
        Some(Failure::InvalidDeclaration)
    );
    assert_eq!(
        Client::new(
            pasted(&server),
            None,
            Some(Secret::new(CLIENT_SECRET.to_owned())),
            server.transport()
        )
        .err(),
        Some(Failure::InvalidDeclaration)
    );
    let client = Client::new(
        declared,
        None,
        Some(Secret::new(CLIENT_SECRET.to_owned())),
        server.transport(),
    )
    .unwrap();
    let (authorization, code, _) = authorize(&client);
    let challenge = query_value(authorization.url(), "code_challenge");
    client.exchange(authorization, code).unwrap();
    let observed = server.observed();
    assert_eq!(observed.len(), 1);
    assert_code_exchange(&observed[0], &challenge);
    assert_eq!(observed[0].fields()["client_secret"], CLIENT_SECRET);
    assert!(!format!("{client:?}").contains(CLIENT_SECRET));
}

/// `state_in_exchange: true` adds the flow's `state` to the code exchange; `false` leaves the
/// RFC 6749 §4.1.3 fields alone.
#[test]
fn exchange_carries_state_only_when_declared() {
    for declared_state in [false, true] {
        let server = FakeServer::start(vec![Reply::token(ACCESS_TOKEN, Some(REFRESH_TOKEN), None)]);
        let mut declared = pasted(&server);
        declared.state_in_exchange = declared_state;
        let client = Client::new(declared, None, None, server.transport()).unwrap();
        let (authorization, code, state) = authorize(&client);
        client.exchange(authorization, code).unwrap();
        let observed = server.observed();
        let fields = observed[0].fields();
        if declared_state {
            assert_eq!(fields.get("state"), Some(&state));
            assert_eq!(
                observed[0].field_order(),
                [
                    "grant_type",
                    "code",
                    "redirect_uri",
                    "client_id",
                    "code_verifier",
                    "state"
                ]
            );
        } else {
            assert!(!fields.contains_key("state"), "state sent undeclared");
        }
    }
}

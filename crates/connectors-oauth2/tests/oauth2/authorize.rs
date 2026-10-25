//! PKCE and the authorize URL.
use crate::support::{CLIENT_ID, NoTransport, declaration, parameter, query, query_value};
use connectors_oauth2::{Client, Failure, pkce_challenge};

/// RFC 7636 Appendix B: the verifier `dBjftJeZ4CVP-mB92K27…` has the S256 challenge `E9Melhoa…`.
#[test]
fn pkce_challenge_matches_rfc7636_appendix_b() {
    assert_eq!(
        pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
    // A generated verifier is 43–128 unreserved characters (RFC 7636 §4.1), and the authorize
    // URL carries its S256 challenge, never the verifier.
    let client = Client::new(declaration(None), None, None, NoTransport).unwrap();
    let first = client.begin("http://127.0.0.1:4000/").unwrap();
    let second = client.begin("http://127.0.0.1:4000/").unwrap();
    for authorization in [&first, &second] {
        let challenge = query_value(authorization.url(), "code_challenge");
        assert_eq!(challenge.len(), 43, "S256 challenge is 32 bytes base64url");
        assert_eq!(
            query_value(authorization.url(), "code_challenge_method"),
            "S256"
        );
        let state = query_value(authorization.url(), "state");
        assert_eq!(state.len(), 43, "state is 32 random bytes, base64url");
        assert!(
            state
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        );
    }
    assert_ne!(
        query_value(first.url(), "code_challenge"),
        query_value(second.url(), "code_challenge"),
        "every authorization draws a fresh verifier"
    );
    assert_ne!(
        query_value(first.url(), "state"),
        query_value(second.url(), "state")
    );
    let shown = format!("{first:?}");
    assert!(
        !shown.contains(&query_value(first.url(), "state")),
        "Debug shows the state: {shown}"
    );
}

/// The authorize URL carries the core parameters, then the declaration's own in declared order.
#[test]
fn authorize_url_carries_declared_parameters_in_order() {
    let mut declared = declaration(None);
    declared.authorize_parameters = vec![
        parameter("prompt", "login"),
        parameter("fixture_flag", "true"),
        parameter("audience", "fixture-audience"),
    ];
    let client = Client::new(declared, None, None, NoTransport).unwrap();
    let redirect = "http://127.0.0.1:4711/callback";
    let authorization = client.begin(redirect).unwrap();
    let url = url::Url::parse(authorization.url()).unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.path(), "/authorize");
    let pairs = query(authorization.url());
    let names: Vec<&str> = pairs.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "response_type",
            "client_id",
            "redirect_uri",
            "scope",
            "code_challenge",
            "code_challenge_method",
            "state",
            "prompt",
            "fixture_flag",
            "audience",
        ]
    );
    let value = |name: &str| query_value(authorization.url(), name);
    assert_eq!(value("response_type"), "code");
    assert_eq!(value("client_id"), CLIENT_ID);
    assert_eq!(value("redirect_uri"), redirect);
    assert_eq!(value("scope"), "fixture.read fixture.write");
    assert_eq!(value("prompt"), "login");
    assert_eq!(value("fixture_flag"), "true");
    assert_eq!(value("audience"), "fixture-audience");
    assert_eq!(authorization.redirect_uri(), redirect);
}

/// No declared parameter may set a name the core sets, whatever its case, and neither may the
/// declared authorize URL's own query.
#[test]
fn extra_parameter_cannot_override_a_core_parameter() {
    for name in [
        "response_type",
        "client_id",
        "redirect_uri",
        "scope",
        "code_challenge",
        "code_challenge_method",
        "state",
        "STATE",
        "Redirect_Uri",
    ] {
        let mut declared = declaration(None);
        declared.authorize_parameters = vec![parameter("prompt", "login"), parameter(name, "x")];
        assert_eq!(
            Client::new(declared, None, None, NoTransport).err(),
            Some(Failure::InvalidDeclaration),
            "{name} overrides a core parameter"
        );
        let mut declared = declaration(None);
        declared.authorize_url = Some(format!("https://localhost:1/authorize?{name}=x"));
        assert_eq!(
            Client::new(declared, None, None, NoTransport).err(),
            Some(Failure::InvalidDeclaration),
            "{name} in the authorize URL overrides a core parameter"
        );
    }
}

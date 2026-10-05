//! The `oauth2_client_credentials` profile through the provider child, on the
//! `oauth2_refresh` fixture host. The child exchanges the stored
//! `{client_id, client_secret}` entry with the client-credentials grant and the
//! configured `scope`, sends the access token as `Authorization: Bearer`, caches
//! it like `oauth2_refresh` does, and never stores or sends a refresh token.
use super::*;

const CLIENT_PROFILE: &str = "fixture.client";
const SCOPES: [&str; 2] = ["tickets:read", "users:read"];

fn client_entry() -> Secret {
    Secret(
        serde_json::to_vec(&json!({
            "client_id": CLIENT_ID,
            "client_secret": CLIENT_SECRET,
        }))
        .unwrap(),
    )
}

fn client_config(provider: &OAuthProvider) -> Value {
    let mut config = oauth_config(
        provider.port,
        &provider.ca,
        Source::Api,
        &format!("https://localhost:{}/token", provider.port),
        "https://localhost/authorize",
    );
    let auth = config["auth"].as_object_mut().unwrap();
    auth.remove("authorize_url");
    auth.insert("profile".into(), json!(CLIENT_PROFILE));
    auth.insert("scheme".into(), json!("oauth2_client_credentials"));
    auth.insert("label".into(), json!("Fixture client secret"));
    auth.insert("requested_scopes".into(), json!(SCOPES));
    config
}

fn client_provider() -> OAuthProvider {
    let provider = OAuthProvider::new(Source::Api);
    provider.write_config(&client_config(&provider));
    provider
}

#[test]
fn client_credentials_bootstrap_asks_for_the_client_only() {
    let provider = client_provider();
    let bootstrap = print_bootstrap(&provider.config).unwrap();
    let profile = &bootstrap["profiles"][0];
    assert_eq!(profile["id"], CLIENT_PROFILE);
    assert_eq!(profile["scheme"], "http_bearer");
    let fields: Vec<&str> = profile["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["name"].as_str().unwrap())
        .collect();
    assert_eq!(fields, ["client_id", "client_secret"]);
    assert!(profile.get("acquisition").is_none_or(Value::is_null));
}

#[test]
fn client_credentials_configuration_refusals() {
    let provider = client_provider();
    let refused = |edit: &dyn Fn(&mut Value)| {
        let mut config = client_config(&provider);
        edit(&mut config);
        provider.write_config(&config);
        print_bootstrap(&provider.config).is_err()
    };
    assert!(refused(
        &|c| c["auth"]["authorize_url"] = json!("https://localhost/authorize")
    ));
    assert!(refused(&|c| {
        c["auth"].as_object_mut().unwrap().remove("token_url");
    }));
    assert!(refused(&|c| c["auth"]["requested_scopes"] = json!([])));
    assert!(refused(&|c| c["auth"]["bearer"] = json!(false)));
    assert!(refused(&|c| c["auth"]["header"] = json!("X-Token")));
    assert!(refused(&|c| c["auth"]["account_label"] = json!("Account")));
    assert!(refused(&|c| {
        c["auth"]["identity"] = json!({"source": "id_token", "kind": "google.user"});
    }));
    provider.write_config(&client_config(&provider));
    assert!(print_bootstrap(&provider.config).is_ok());
}

#[test]
fn client_credentials_grant_sends_form_and_uses_bearer() {
    let provider = client_provider();
    let mut child = spawn(&provider);
    child
        .validate(CLIENT_PROFILE, &client_entry(), deadline())
        .unwrap_or_else(|failure| panic!("validation {failure:?}"));
    let project = project(&mut child, &client_entry()).unwrap_or_else(|f| panic!("invoke {f:?}"));
    assert_eq!(project["body"]["id"], 7);
    let requests = provider.requests();
    let token: Vec<&Observed> = requests.iter().filter(|r| r.route == "/token").collect();
    // Validation exchanged; the invoke used the token validation cached.
    assert_eq!(token.len(), 1);
    assert_eq!(token[0].method, "POST");
    assert_eq!(
        token[0].content_type.as_deref(),
        Some("application/x-www-form-urlencoded")
    );
    assert!(token[0].authorization.is_none());
    assert!(token[0].query.is_empty());
    let mut form = decode_form(&token[0].body);
    form.sort();
    let mut expected = vec![
        ("grant_type".to_owned(), "client_credentials".to_owned()),
        ("client_id".to_owned(), CLIENT_ID.to_owned()),
        ("client_secret".to_owned(), CLIENT_SECRET.to_owned()),
        ("scope".to_owned(), SCOPES.join(" ")),
    ];
    expected.sort();
    assert!(form == expected, "token form fields differ");
    let api = provider.api_authorizations();
    assert_eq!(api.len(), 2, "identity read, then the invoke");
    assert!(
        api.iter()
            .all(|a| a.as_deref() == Some(&*format!("Bearer {ACCESS_TOKEN}1")))
    );
    assert_no_material_at_rest(&provider);
}

#[test]
fn client_credentials_refusals_by_code() {
    let provider = client_provider();
    let mut child = spawn(&provider);
    provider.set_token(TokenAnswer::Refuse(
        401,
        json!({"error": "invalid_client", "error_description": "fixture description"}),
    ));
    assert!(matches!(
        child.validate(CLIENT_PROFILE, &client_entry(), deadline()),
        Err(Failure::InvalidCredential)
    ));
    assert!(matches!(
        project(&mut child, &client_entry()),
        Err(Failure::InvalidCredential)
    ));
    // A client this grant is not allowed for, such as a public one.
    provider.set_token(TokenAnswer::Refuse(
        400,
        json!({"error": "unauthorized_client"}),
    ));
    assert!(matches!(
        project(&mut child, &client_entry()),
        Err(Failure::InvalidCredential)
    ));
    provider.set_token(TokenAnswer::Refuse(429, json!({"error": "rate_limited"})));
    assert!(matches!(
        project(&mut child, &client_entry()),
        Err(Failure::ProviderRateLimited)
    ));
    // This grant issues no refresh token; an answer carrying one is refused.
    provider.set_token(TokenAnswer::Grant {
        expires_in: 3600,
        scope: SCOPES.join(" "),
        claims: None,
        refresh_token: Some(REFRESH_TOKEN),
    });
    assert!(matches!(
        project(&mut child, &client_entry()),
        Err(Failure::Protocol)
    ));
    assert!(provider.api_authorizations().is_empty());
    // A refresh-token entry is not this profile's entry.
    assert!(matches!(
        project(&mut child, &entry()),
        Err(Failure::InvalidInput)
    ));
    provider.set_token(grant(3600));
    project(&mut child, &client_entry()).unwrap();
    project(&mut child, &client_entry()).unwrap();
    assert_eq!(provider.token_requests(), 6);
}

/// The same client as `client_entry`, written with other spacing: a separate
/// cache key, so it exchanges on its own.
fn spaced_client_entry() -> Secret {
    Secret(
        format!("{{ \"client_id\": \"{CLIENT_ID}\", \"client_secret\": \"{CLIENT_SECRET}\" }}")
            .into_bytes(),
    )
}

/// A client-credentials answer the provider must accept, served after one
/// ordinary grant so the fixture API admits `{ACCESS_TOKEN}1`.
fn project_with_answer(answer: Value) -> Result<Value, Failure> {
    let provider = client_provider();
    let mut child = spawn(&provider);
    project(&mut child, &spaced_client_entry()).unwrap_or_else(|f| panic!("first grant {f:?}"));
    provider.set_token(TokenAnswer::Refuse(200, answer));
    project(&mut child, &client_entry())
}

/// Zendesk's documented client-credentials answer has no `expires_in`: "For
/// clients created before April 30, 2026, there is no default expires_in
/// value" (developer.zendesk.com, OAuth Tokens for Grant Types, read
/// 2026-10-05), and the provider sends none. RFC 6749 5.1 makes `expires_in`
/// RECOMMENDED, not required, so such an access token is still usable.
#[test]
fn client_credentials_answer_without_expires_in_is_usable() {
    let result = project_with_answer(json!({
        "access_token": format!("{ACCESS_TOKEN}1"),
        "scope": "read",
        "token_type": "bearer",
    }));
    assert!(
        result.is_ok(),
        "Zendesk's documented answer refused: {:?}",
        result.err()
    );
}

/// The same, with the JSON-format default Zendesk states ("Defaults to null")
/// written out.
#[test]
fn client_credentials_answer_with_null_expires_in_is_usable() {
    let result = project_with_answer(json!({
        "access_token": format!("{ACCESS_TOKEN}1"),
        "expires_in": null,
        "scope": "read",
        "token_type": "bearer",
    }));
    assert!(
        result.is_ok(),
        "answer with a null expires_in refused: {:?}",
        result.err()
    );
}

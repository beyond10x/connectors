//! An HTTP basic profile (`{"account","token"}`) through the provider child:
//! the configuration loads, the exact `Authorization` header reaches the
//! identity read and an operation, a wrong token and missing scopes refuse by
//! code, and neither the account nor the token reaches configuration, argv or
//! the environment.
use super::*;

fn authorization_of(provider: &Provider, route: &str) -> Vec<Option<String>> {
    provider
        .authorizations
        .lock()
        .unwrap()
        .iter()
        .filter(|(observed, _)| observed == route)
        .map(|(_, authorization)| authorization.clone())
        .collect()
}

#[test]
fn basic_profile_loads_and_sends_the_exact_basic_authorization() {
    let provider = Provider::basic("api");
    let selection = provider.selection();
    let mut child = Child::spawn(&selection).unwrap();
    let profile = child.bootstrap().profile(BASIC_PROFILE).unwrap().clone();
    assert_eq!(profile.scheme, "http_basic");
    assert_eq!(profile.capability, "http-basic");
    assert_eq!(
        profile
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.label.as_str()))
            .collect::<Vec<_>>(),
        [
            ("account", "Fixture account email"),
            ("token", "Fixture API token")
        ]
    );

    let baseline = child
        .validate(BASIC_PROFILE, &basic(BASIC_TOKEN), deadline())
        .unwrap_or_else(|failure| panic!("validation {failure:?}"));
    assert_eq!(baseline.identity.subject, "42");
    assert!(baseline.granted_scopes.unwrap().contains("api"));
    let identity = authorization_of(&provider, "/api/v4/user");
    assert_eq!(identity.len(), 1);
    assert!(identity[0].as_deref() == Some(BASIC_HEADER));
    let scopes = authorization_of(&provider, "/api/v4/personal_access_tokens/self");
    assert_eq!(scopes.len(), 1);
    assert!(scopes[0].as_deref() == Some(BASIC_HEADER));

    let seen = provider.authorizations.lock().unwrap().len();
    let project = invoke(
        &mut child,
        "project.get",
        "one",
        &basic(BASIC_TOKEN),
        json!({"id":"org/project"}),
    )
    .unwrap();
    assert_eq!(project["body"]["id"], 7);
    let operation = provider.authorizations.lock().unwrap()[seen..].to_vec();
    assert_eq!(operation.len(), 1);
    assert!(operation[0].0.starts_with("/api/v4/projects/"));
    assert!(operation[0].1.as_deref() == Some(BASIC_HEADER));

    assert!(matches!(
        child.validate(BASIC_PROFILE, &basic(BASIC_WRONG_TOKEN), deadline()),
        Err(Failure::InvalidCredential)
    ));
    // A malformed document refuses before any request.
    let before = provider.count();
    for document in [
        br#"{"token":"fixture-api-token-one"}"#.to_vec(),
        br#"{"account":"fixture-account@example.test"}"#.to_vec(),
        br#"{"account":"fixture:account","token":"fixture-api-token-one"}"#.to_vec(),
        br#"{"account":"","token":"fixture-api-token-one"}"#.to_vec(),
        br#"{"account":"fixture-account@example.test","token":"fixture-api-token-one","extra":"x"}"#
            .to_vec(),
    ] {
        assert!(matches!(
            child.validate(BASIC_PROFILE, &Secret(document), deadline()),
            Err(Failure::InvalidInput)
        ));
    }
    assert_eq!(provider.count(), before);

    assert!(!carries_basic_material(
        &fs::read(&provider.config).unwrap()
    ));
    assert_children_carry_no_basic_material(&provider.config);
}

/// The basic fixture with `auth.access` set to `access`, or refused at load.
fn with_access(access: Value) -> Provider {
    let provider = Provider::basic("api");
    let mut config: Value = serde_json::from_slice(&fs::read(&provider.config).unwrap()).unwrap();
    config["auth"]["access"] = access;
    private(&provider.config, &serde_json::to_vec(&config).unwrap());
    provider
}

#[test]
fn an_access_read_the_credential_may_not_make_refuses_as_insufficient_scope() {
    // The identity read succeeds; the access read answers 403, so the
    // credential identifies its holder but cannot read: not connected.
    let provider = with_access(json!({"path": "projects/fixture-refused"}));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    assert!(matches!(
        child.validate(BASIC_PROFILE, &basic(BASIC_TOKEN), deadline()),
        Err(Failure::InsufficientScope)
    ));
    assert_eq!(authorization_of(&provider, "/api/v4/user").len(), 1);
    assert_eq!(
        authorization_of(&provider, "/api/v4/projects/fixture-refused"),
        [Some(BASIC_HEADER.to_owned())]
    );
    // A missing resource is not a scope: it stays a protocol failure.
    let provider = with_access(json!({"path": "projects/fixture-missing"}));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    assert!(matches!(
        child.validate(BASIC_PROFILE, &basic(BASIC_TOKEN), deadline()),
        Err(Failure::Protocol)
    ));
}

#[test]
fn an_access_read_that_answers_connects_and_sends_its_query() {
    let provider = with_access(json!({"path": "user", "query": {"per_page": "1"}}));
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline = child
        .validate(BASIC_PROFILE, &basic(BASIC_TOKEN), deadline())
        .unwrap_or_else(|failure| panic!("validation {failure:?}"));
    assert_eq!(baseline.identity.subject, "42");
    // The identity read and the access read both reach `/api/v4/user`.
    assert_eq!(authorization_of(&provider, "/api/v4/user").len(), 2);
}

#[test]
fn an_access_probe_without_a_path_or_with_a_query_in_its_path_is_refused_at_load() {
    for access in [
        json!({"path": ""}),
        json!({"path": "user?per_page=1"}),
        json!({"path": "user", "query": {"": "1"}}),
    ] {
        let provider = with_access(access.clone());
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(&provider.config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(!output.status.success(), "{access}");
    }
}

#[test]
fn basic_profile_below_minimum_scopes_refuses_as_insufficient() {
    let provider = Provider::basic("admin");
    let mut child = Child::spawn(&provider.selection()).unwrap();
    assert!(matches!(
        child.validate(BASIC_PROFILE, &basic(BASIC_TOKEN), deadline()),
        Err(Failure::InsufficientScope)
    ));
}

#[test]
fn token_profile_rejects_a_basic_document() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    assert!(matches!(
        child.validate("gitlab.pat", &basic(BASIC_TOKEN), deadline()),
        Err(Failure::InvalidInput)
    ));
    assert_eq!(provider.count(), 0);
}

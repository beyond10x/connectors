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

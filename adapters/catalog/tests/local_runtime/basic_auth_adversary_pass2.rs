//! Second adversarial pass on the HTTP basic profile after the encoder moved to
//! the workspace `base64` crate: the documented configuration example loads as
//! documented, the largest account the profile advertises encodes exactly, and
//! a duplicated account key refuses before any request.
use super::*;

fn documented_basic_auth() -> Value {
    let doc = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/local-catalog-provider.md"),
    )
    .unwrap();
    let block = doc
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| body.contains("\"scheme\": \"basic\""))
        .expect("the documented basic auth example");
    let wrapped: Value = serde_json::from_str(&format!("{{{block}}}")).unwrap();
    wrapped["auth"].clone()
}

#[test]
fn adversary2_documented_basic_auth_example_loads_as_http_basic() {
    let provider = Provider::basic("api");
    let auth = documented_basic_auth();
    let mut config: Value = serde_json::from_slice(&fs::read(&provider.config).unwrap()).unwrap();
    config["auth"] = auth;
    private(&provider.config, &serde_json::to_vec(&config).unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(&provider.config)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "documented example refused at load"
    );
    let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
    let profile = bootstrap.profile("tracker.api-token").unwrap();
    assert_eq!(profile.scheme, "http_basic");
    assert_eq!(
        profile
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.label.as_str()))
            .collect::<Vec<_>>(),
        [("account", "Account email"), ("token", "API token")]
    );
}

#[test]
fn adversary2_maximum_account_encodes_exactly() {
    let provider = Provider::basic("api");
    let mut child = Child::spawn(&provider.selection()).unwrap();
    // 1024 bytes of `a` (the advertised maximum), then `:` and 3001 bytes of `b`:
    // 341 groups `aaa`, one group `a:b`, 1000 groups `bbb`.
    let account = "a".repeat(1024);
    let token = "b".repeat(3001);
    let expected = format!("Basic {}YTpi{}", "YWFh".repeat(341), "YmJi".repeat(1000));
    let before = provider.authorizations.lock().unwrap().len();
    let document =
        Secret(serde_json::to_vec(&json!({"account": account, "token": token})).unwrap());
    assert!(matches!(
        child.validate(BASIC_PROFILE, &document, deadline()),
        Err(Failure::InvalidCredential)
    ));
    let seen = provider.authorizations.lock().unwrap()[before..].to_vec();
    assert_eq!(seen.len(), 1);
    let observed = seen[0].1.as_deref().unwrap_or_default();
    assert_eq!(observed.len(), expected.len(), "header length");
    assert!(observed == expected, "header differs");
}

#[test]
fn adversary2_duplicate_account_key_refuses_before_any_request() {
    let provider = Provider::basic("api");
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let before = provider.count();
    let document = Secret(
        br#"{"account":"fixture-account@example.test","account":"other@example.test","token":"fixture-api-token-one"}"#
            .to_vec(),
    );
    assert!(matches!(
        child.validate(BASIC_PROFILE, &document, deadline()),
        Err(Failure::InvalidInput)
    ));
    assert_eq!(provider.count(), before);
}

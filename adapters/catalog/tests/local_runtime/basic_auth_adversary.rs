//! Adversarial cases for the HTTP basic profile: UTF-8 accounts, colons in the
//! token, the account length boundary, load refusals for inconsistent basic
//! profiles, and that the default scheme keeps token revisions unchanged.
use super::*;

fn document(account: &str, token: &str) -> Secret {
    Secret(serde_json::to_vec(&json!({"account": account, "token": token})).unwrap())
}

fn rewrite_auth(provider: &Provider, edit: impl FnOnce(&mut serde_json::Map<String, Value>)) {
    let mut config: Value = serde_json::from_slice(&fs::read(&provider.config).unwrap()).unwrap();
    edit(config["auth"].as_object_mut().unwrap());
    private(&provider.config, &serde_json::to_vec(&config).unwrap());
}

fn bootstrap_of(provider: &Provider) -> Option<Bootstrap> {
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(&provider.config)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    output
        .status
        .success()
        .then(|| serde_json::from_slice(&output.stdout).unwrap())
}

#[test]
fn adversary_basic_header_encodes_utf8_accounts_and_colon_bearing_tokens() {
    let provider = Provider::basic("api");
    let mut child = Child::spawn(&provider.selection()).unwrap();
    // References computed with coreutils `printf '%s' ... | base64 -w0`.
    for (account, token, expected) in [
        (
            "jürgen@example.test",
            "tok:en",
            "Basic asO8cmdlbkBleGFtcGxlLnRlc3Q6dG9rOmVu",
        ),
        ("a", "b:c", "Basic YTpiOmM="),
        ("Ωmega", "x", "Basic zqltZWdhOng="),
        ("user name", "t0k", "Basic dXNlciBuYW1lOnQwaw=="),
    ] {
        let before = provider.authorizations.lock().unwrap().len();
        assert!(matches!(
            child.validate(BASIC_PROFILE, &document(account, token), deadline()),
            Err(Failure::InvalidCredential)
        ));
        let seen = provider.authorizations.lock().unwrap()[before..].to_vec();
        assert_eq!(seen.len(), 1);
        assert!(
            seen[0].1.as_deref() == Some(expected),
            "account {account:?}"
        );
    }
}

#[test]
fn adversary_basic_account_limit_is_1024_bytes_not_characters() {
    let provider = Provider::basic("api");
    let mut child = Child::spawn(&provider.selection()).unwrap();
    // 512 two-byte characters: 1024 bytes, the advertised field maximum.
    let at_limit = "ü".repeat(512);
    let before = provider.count();
    assert!(matches!(
        child.validate(BASIC_PROFILE, &document(&at_limit, "t"), deadline()),
        Err(Failure::InvalidCredential)
    ));
    assert_eq!(provider.count(), before + 1);
    let over = format!("{at_limit}a");
    let before = provider.count();
    assert!(matches!(
        child.validate(BASIC_PROFILE, &document(&over, "t"), deadline()),
        Err(Failure::InvalidInput)
    ));
    // A control character other than newline, and DEL.
    for account in ["tab\there", "del\u{7f}", "nel\u{85}"] {
        assert!(matches!(
            child.validate(BASIC_PROFILE, &document(account, "t"), deadline()),
            Err(Failure::InvalidInput)
        ));
    }
    assert_eq!(provider.count(), before);
}

#[test]
fn adversary_inconsistent_basic_profiles_refuse_at_load() {
    type Edit = fn(&mut serde_json::Map<String, Value>);
    let edits: [(&str, Edit); 6] = [
        ("basic without account_label", |auth| {
            auth.remove("account_label");
        }),
        ("basic with empty account_label", |auth| {
            auth.insert("account_label".into(), json!(""));
        }),
        ("basic with bearer", |auth| {
            auth.insert("bearer".into(), json!(true));
        }),
        ("basic in another header", |auth| {
            auth.insert("header".into(), json!("X-Api-Key"));
        }),
        ("basic with a 129-byte account_label", |auth| {
            auth.insert("account_label".into(), json!("a".repeat(129)));
        }),
        ("unknown scheme", |auth| {
            auth.insert("scheme".into(), json!("digest"));
        }),
    ];
    for (name, edit) in edits {
        let provider = Provider::basic("api");
        assert!(bootstrap_of(&provider).is_some(), "{name}: baseline loads");
        rewrite_auth(&provider, edit);
        assert!(bootstrap_of(&provider).is_none(), "{name} loaded");
    }
    let provider = Provider::new();
    rewrite_auth(&provider, |auth| {
        auth.insert("account_label".into(), json!("Account"));
    });
    assert!(
        bootstrap_of(&provider).is_none(),
        "token profile with account_label loaded"
    );
}

#[test]
fn adversary_explicit_token_scheme_keeps_revisions_of_an_omitted_scheme() {
    let provider = Provider::new();
    let omitted = bootstrap_of(&provider).unwrap();
    rewrite_auth(&provider, |auth| {
        auth.insert("scheme".into(), json!("token"));
    });
    let explicit = bootstrap_of(&provider).unwrap();
    assert_eq!(
        omitted.configuration_revision,
        explicit.configuration_revision
    );
    assert_eq!(omitted.profiles[0].revision, explicit.profiles[0].revision);
    assert_eq!(explicit.profiles[0].scheme, "http_bearer");
    let fields: Vec<_> = explicit.profiles[0]
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    assert_eq!(fields, ["token"]);
}

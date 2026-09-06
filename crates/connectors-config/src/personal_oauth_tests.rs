use crate::CatalogIntegrationConfig;

const PKCE: &str = r#"
provider = "gitlab"
instance = "personal"
grant_ref = "grant:personal"
initiation = "platform"
credential = "gitlab.oauth_token"
[oauth]
auth_profile = "gitlab.oauth_token"
flow = "authorization_code_pkce"
client_authentication = "public"
client_id = "synthetic-public-client"
redirect_uri = "http://127.0.0.1:18973/callback"
browser_placement = "same_machine"
registration_use = "development_only"
custody = "development_file"
allowed_scopes = ["read_api"]
"#;

fn valid(source: &str) -> bool {
    toml::from_str::<CatalogIntegrationConfig>(source).is_ok_and(|config| config.validate().is_ok())
}

#[test]
fn personal_oauth_configuration_admits_explicit_development_public_pkce() {
    assert!(valid(PKCE));
}

#[test]
fn personal_oauth_configuration_admits_device_for_a_remote_browser_without_redirect() {
    let device = PKCE
        .replace("authorization_code_pkce", "device_authorization")
        .replace("same_machine", "other_machine")
        .replace("redirect_uri = \"http://127.0.0.1:18973/callback\"", "");
    assert!(valid(&device));
}

#[test]
fn personal_oauth_configuration_refuses_nonlocal_redirects_and_implicit_custody() {
    for uri in [
        "http://localhost:18973/callback",
        "https://127.0.0.1:18973/callback",
        "http://127.0.0.1/callback",
        "http://127.0.0.1:0/callback",
        "http://127.0.0.1:18973/",
        "http://user@127.0.0.1:18973/callback",
        "http://127.0.0.1:18973/callback?x=1",
        "http://127.0.0.1:18973/callback#fragment",
        "http://127.1:18973/callback",
        "http://127.0.0.1:18973/a/../callback",
    ] {
        assert!(!valid(
            &PKCE.replace("http://127.0.0.1:18973/callback", uri)
        ));
    }
    for malformed in [
        PKCE.replace("same_machine", "other_machine"),
        PKCE.replace("development_only", "production_allowed"),
        PKCE.replace("custody = \"development_file\"", ""),
        PKCE.replace(
            "client_authentication = \"public\"",
            "client_authentication = \"client_secret_post\"",
        ),
        PKCE.replace(
            "client_id = \"synthetic-public-client\"",
            "client_id = \"\"",
        ),
        format!("{PKCE}\nclient_secret_ref = \"credential:secret\""),
        format!("{PKCE}\nsession_ttl_seconds = 29"),
        format!("{PKCE}\nsession_ttl_seconds = 601"),
        PKCE.replace("allowed_scopes = [\"read_api\"]", "allowed_scopes = []"),
        PKCE.replace(
            "credential = \"gitlab.oauth_token\"",
            "credential = \"gitlab.token\"",
        ),
        PKCE.replace(
            "[oauth]",
            "credential_file = \"/synthetic/import-token\"\n[oauth]",
        ),
    ] {
        assert!(!valid(&malformed));
    }
}

#[test]
fn oauth_pass1_scope_ceiling_and_ttl_boundaries_survive_real_config_loading() {
    for ttl in [30, 600] {
        let source = format!("{PKCE}\nsession_ttl_seconds = {ttl}");
        assert!(valid(&source));
    }
    for scopes in [
        vec!["read_api".to_owned(), "read_api".to_owned()],
        vec!["read api".to_owned()],
        vec!["read,api".to_owned()],
        vec!["r".repeat(257)],
        (0..65).map(|n| format!("scope{n}")).collect(),
    ] {
        let mut parsed: toml::Value = toml::from_str(PKCE).unwrap();
        parsed["oauth"]["allowed_scopes"] =
            toml::Value::Array(scopes.into_iter().map(toml::Value::String).collect());
        assert!(!valid(&toml::to_string(&parsed).unwrap()));
    }
    let mut parsed: toml::Value = toml::from_str(PKCE).unwrap();
    parsed["oauth"]["allowed_scopes"] = toml::Value::Array(
        (0..64)
            .map(|n| toml::Value::String(format!("scope{n}")))
            .collect(),
    );
    assert!(valid(&toml::to_string(&parsed).unwrap()));
}

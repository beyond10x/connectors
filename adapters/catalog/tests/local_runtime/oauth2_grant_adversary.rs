//! Adversary pass 1 on story:connect-runs-an-oauth2-grant: the `oauth2_grant`
//! profile read against the unit's own contract row
//! (`contracts/auth/acquisition/v1alpha1/semantics.md`, the `oauth2_grant`
//! row): an entry of at most 64 KiB, HTTPS endpoints only.
use super::oauth2_grant::{
    Behaviour, GRANT_ACCESS, GRANT_CLIENT_ID, GRANT_PROFILE, GRANT_REFRESH, GRANT_SCOPE,
    GrantServer, Redirect, declaration, grant_config, unsigned_jwt,
};
use super::*;

/// The configuration at `config`, printed as a bootstrap, or the refusal the
/// provider wrote to standard error.
fn printed(config: &Value) -> Result<Value, String> {
    let directory = tempfile::tempdir().unwrap();
    let private_directory = directory.path().join("private");
    filesystem::directory(&private_directory, true, true).unwrap();
    let path = private_directory.join("catalog.json");
    private(&path, &serde_json::to_vec(config).unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
        .arg("--local-config")
        .arg(&path)
        .arg("--print-local-bootstrap")
        .output()
        .unwrap();
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(serde_json::from_slice(&output.stdout).unwrap())
}

/// The CLI's grant admits an entry of up to 64 KiB
/// (`crates/connectors-host/src/local/oauth2_grant.rs`, `ENTRY_LIMIT`), and the
/// contract row says the same: `{client_id, access_token, refresh_token,
/// expires_at_ms, scopes, id_token?}` "(at most 64 KiB)". A token endpoint that
/// answers an OpenID Connect `id_token` of 12 KiB therefore yields an entry the
/// CLI submits; the provider child must validate it, reading the subject from
/// that `id_token`, rather than refuse the entry it was handed.
#[test]
fn adversary_an_entry_within_64_kib_holding_an_id_token_over_8_kib_validates() {
    let server = GrantServer::new(Behaviour::Consent);
    let mut config = grant_config(
        server.port,
        &server.ca,
        declaration(server.port, Redirect::AnyPort),
    );
    config["auth"]["identity"] = json!({"source": "id_token", "kind": "fixture.person"});
    server.write_config(&config);
    let mut child = Child::spawn(&server.selection()).unwrap();
    // A realistic large claim set: many group names, as some identity
    // providers put in an id_token.
    let groups: Vec<String> = (0..400).map(|n| format!("fixture-group-{n:04}")).collect();
    let id_token = unsigned_jwt(&json!({
        "iss": format!("https://localhost:{}", server.port),
        "aud": GRANT_CLIENT_ID,
        "sub": "person-7",
        "iat": 1,
        "exp": 4_000_000_000u64,
        "groups": groups,
    }));
    assert!(id_token.len() > 8192, "{}", id_token.len());
    let entry = serde_json::to_vec(&json!({
        "client_id": GRANT_CLIENT_ID,
        "access_token": format!("{GRANT_ACCESS}1"),
        "refresh_token": format!("{GRANT_REFRESH}1"),
        "expires_at_ms": connectors_sdk::now_ms() + 3_600_000,
        "scopes": [GRANT_SCOPE],
        "id_token": id_token,
    }))
    .unwrap();
    assert!(entry.len() < 64 * 1024, "{}", entry.len());
    let baseline = child
        .validate(GRANT_PROFILE, &Secret(entry), deadline())
        .unwrap_or_else(|failure| panic!("an entry within 64 KiB was refused: {failure:?}"));
    assert_eq!(baseline.identity.subject, "person-7");
}

/// A `pasted_code` declaration's `redirect_uri` is the redirection endpoint
/// the authorization server sends the browser to with the code (RFC 6749
/// §3.1.2); unlike a loopback redirect it is a remote page. The story refuses a
/// declaration "with a non-HTTPS endpoint", so a plain-HTTP pasted-code
/// redirect is refused like a plain-HTTP token or authorize endpoint
/// (`oauth2_grant_http_endpoint_is_refused`).
#[test]
fn adversary_a_pasted_code_redirect_over_plain_http_is_refused() {
    let server = GrantServer::new(Behaviour::Consent);
    let port = server.port;
    // The same declaration with its HTTPS redirect is read.
    printed(&grant_config(
        port,
        &server.ca,
        declaration(port, Redirect::Pasted),
    ))
    .unwrap();
    let mut declared = declaration(port, Redirect::Pasted);
    declared["redirect_uri"] = json!("http://fixture.invalid/callback");
    assert_eq!(
        printed(&grant_config(port, &server.ca, declared)).map(|_| ()),
        Err("InvalidConfiguration".to_owned())
    );
}

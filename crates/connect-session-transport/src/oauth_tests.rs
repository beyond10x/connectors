use super::*;
use std::time::Duration;
use tokio::net::TcpStream;

fn redirect() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    format!("http://{}/oauth/callback", listener.local_addr().unwrap())
}
fn config(redirect_uri: &str) -> PkceEndpointConfig<'_> {
    PkceEndpointConfig {
        redirect_uri,
        authorization_origin: "https://gitlab.example",
        authorization_path: "/oauth/authorize",
        client_id: "test-client",
        scope: "api",
        deadline: Instant::now() + Duration::from_secs(30),
    }
}
fn device() -> DeviceAuthorization {
    connector_oauth::device::validate_device(
        connector_oauth::device::DeviceResponse {
            device_code: Zeroizing::new("PRIVATE-DEVICE-CODE".into()),
            user_code: "ABCD-EFGH".into(),
            verification_uri: "https://gitlab.example/device".into(),
            verification_uri_complete: None,
            expires_in: 60,
            interval: None,
        },
        "https://gitlab.example",
        1_000,
        61_000,
    )
    .unwrap()
}
fn browser_parts(endpoint: &BoundOAuthEndpoint) -> (String, String) {
    let value = endpoint.browser_url();
    let uri = url::Url::parse(&value).unwrap();
    assert_eq!(uri.scheme(), "http");
    assert_eq!(uri.host_str(), Some("127.0.0.1"));
    assert_eq!(uri.path(), "/");
    assert!(uri.query().is_none());
    assert!(uri.port().is_some());
    (
        format!("127.0.0.1:{}", uri.port().unwrap()),
        uri.fragment()
            .unwrap()
            .strip_prefix("token=")
            .unwrap()
            .to_owned(),
    )
}
async fn request(authority: &str, target: &str, headers: &str) -> String {
    let mut stream = TcpStream::connect(authority).await.unwrap();
    stream
        .write_all(
            format!("GET {target} HTTP/1.1\r\nHost: {authority}\r\n{headers}\r\n").as_bytes(),
        )
        .await
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response
}
async fn instructions(authority: &str, capability: &str) -> serde_json::Value {
    let response = request(
        authority,
        "/instructions",
        &format!("X-Connect-Session: {capability}\r\n"),
    )
    .await;
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
}

#[test]
fn fixed_redirect_policy_refuses_aliases_implicit_ports_and_ambiguous_paths() {
    assert!(validate_redirect_uri("http://127.0.0.1:12345/oauth/callback").is_ok());
    for uri in [
        "http://127.0.0.1:0/cb",
        "http://127.0.0.1/cb",
        "http://127.0.0.1:80/cb",
        "http://localhost:12345/cb",
        "http://127.1:12345/cb",
        "http://[::1]:12345/cb",
        "https://127.0.0.1:12345/cb",
        "http://u@127.0.0.1:12345/cb",
        "http://127.0.0.1:12345/",
        "http://127.0.0.1:12345/cb?q=1",
        "http://127.0.0.1:12345/cb#f",
        "http://127.0.0.1:12345/a/../cb",
        "http://127.0.0.1:12345//cb",
        "http://127.0.0.1:12345/instructions",
    ] {
        assert!(validate_redirect_uri(uri).is_err(), "{uri}");
    }
}

#[test]
fn callback_query_is_strict_bounded_and_distinguishes_unknown_state() {
    assert_eq!(
        &*parse_callback("/cb?state=live&code=abc%2Bdef", "/cb", "live").unwrap(),
        "abc+def"
    );
    assert_eq!(
        parse_callback("/cb?state=wrong&code=abc", "/cb", "live").err(),
        Some(OAuthTransportError::StateMismatch)
    );
    assert_eq!(
        parse_callback("/cb?state=live&error=access_denied", "/cb", "live").err(),
        Some(OAuthTransportError::CodeExchangeRefused)
    );
    for target in [
        "/wrong?state=live&code=a",
        "http://127.0.0.1/cb?state=live&code=a",
        "/cb?state=live&state=live&code=a",
        "/cb?st%61te=live&state=live&code=a",
        "/cb?state=live&code=a&code=b",
        "/cb?state=live&code=a&error=access_denied",
        "/cb?state=live&code=%",
        "/cb?state=live&code=%GG",
        "/cb?state=live&code=%FF",
        "/cb?state=live&code=%00",
        "/cb?state=live&code=",
        "/cb?state=&code=a",
        "/cb?code=a",
        "/cb?state=live&code=a#fragment",
        "/cb?state=live&code=a&extra=1&extra=2",
        "/cb?state=live&code=a&flag",
    ] {
        assert_eq!(
            parse_callback(target, "/cb", "live").err(),
            Some(OAuthTransportError::MalformedCallback),
            "{target}"
        );
    }
    assert!(parse_callback(
        &format!("/cb?state=live&code={}", "a".repeat(2_048)),
        "/cb",
        "live"
    )
    .is_ok());
    assert!(parse_callback(
        &format!("/cb?state=live&code={}", "a".repeat(2_049)),
        "/cb",
        "live"
    )
    .is_err());
    assert!(parse_callback(
        &format!("/cb?state=live&code=a&extra={}", "a".repeat(4_096)),
        "/cb",
        "live"
    )
    .is_err());
}

#[test]
fn request_parser_requires_exact_host_get_origin_form_and_bounded_headers() {
    let valid = b"GET /cb?state=s&code=c HTTP/1.1\r\nHost: 127.0.0.1:12345\r\nOrigin: https://gitlab.example\r\n\r\n";
    let parsed = parse_request(valid, "127.0.0.1:12345").unwrap();
    assert_eq!(parsed.origin, Some("https://gitlab.example"));
    assert_eq!(parsed.target, "/cb?state=s&code=c");
    for headers in [
        "Host: localhost:12345",
        "Host: 127.0.0.1:012345",
        "Host: 127.0.0.1:12345\r\nHost: 127.0.0.1:12345",
        "",
        "Host: 127.0.0.1:12345\r\nOrigin: null\r\nOrigin: https://gitlab.example",
        "Host: 127.0.0.1:12345\r\nTransfer-Encoding: chunked",
        "Host: 127.0.0.1:12345\r\nContent-Length: 1",
        "Host : 127.0.0.1:12345",
        "Host: 127.0.0.1:12345\r\nX-Connect-Session: a\r\nX-Connect-Session: b",
    ] {
        assert!(
            parse_request(
                format!("GET / HTTP/1.1\r\n{headers}\r\n\r\n").as_bytes(),
                "127.0.0.1:12345"
            )
            .is_err(),
            "{headers}"
        );
    }
    for line in [
        "POST /cb HTTP/1.1",
        "GET http://127.0.0.1:12345/cb HTTP/1.1",
        "GET //evil.example/cb HTTP/1.1",
        "GET /cb HTTP/1.0",
    ] {
        assert!(parse_request(
            format!("{line}\r\nHost: 127.0.0.1:12345\r\n\r\n").as_bytes(),
            "127.0.0.1:12345"
        )
        .is_err());
    }
    assert!(parse_request(
        format!(
            "GET / HTTP/1.1\r\nHost: 127.0.0.1:12345\r\nX: {}\r\n\r\n",
            "a".repeat(8_192)
        )
        .as_bytes(),
        "127.0.0.1:12345"
    )
    .is_err());
}

#[tokio::test]
async fn callback_claim_closes_fixed_port_and_keeps_capability_out_of_provider_url() {
    let redirect = redirect();
    let endpoint = BoundOAuthEndpoint::bind_pkce(config(&redirect)).unwrap();
    let (authority, capability) = browser_parts(&endpoint);
    let serving = tokio::spawn(endpoint.receive());
    let body = instructions(&authority, &capability).await;
    let authorization = body["authorization_url"].as_str().unwrap();
    assert!(!authorization.contains(&capability));
    let authorization = url::Url::parse(authorization).unwrap();
    let query: std::collections::HashMap<_, _> = authorization.query_pairs().collect();
    assert_eq!(query["redirect_uri"], redirect);
    assert_eq!(query["code_challenge_method"], "S256");
    assert_ne!(query["state"], capability);
    let state = query["state"].to_string();
    let wrong = request(&authority, "/oauth/callback?state=wrong&code=secret", "").await;
    assert!(wrong.contains("state_mismatch"));
    let reply = request(
        &authority,
        &format!("/oauth/callback?state={state}&code=test-code"),
        "",
    )
    .await;
    assert!(reply.starts_with("HTTP/1.1 200"));
    let claim = serving.await.unwrap().unwrap();
    assert_eq!(&*claim.code, "test-code");
    assert_eq!(claim.verifier.len(), 64);
    assert_eq!(claim.redirect_uri, redirect);
    assert!(TcpStream::connect(&authority).await.is_err());
}

#[tokio::test]
async fn protected_instructions_refuse_capability_and_origin_without_spending_state() {
    let redirect = redirect();
    let endpoint = BoundOAuthEndpoint::bind_pkce(config(&redirect)).unwrap();
    let (authority, capability) = browser_parts(&endpoint);
    let serving = tokio::spawn(endpoint.receive());
    for headers in [
        String::new(),
        "X-Connect-Session: wrong\r\n".into(),
        format!("X-Connect-Session: {capability}\r\nOrigin: null\r\n"),
        format!("X-Connect-Session: {capability}\r\nOrigin: https://evil.example\r\n"),
    ] {
        let response = request(&authority, "/instructions", &headers).await;
        assert!(response.starts_with("HTTP/1.1 403"));
        assert!(!response.contains("oauth/authorize"));
    }
    let response = request(
        &authority,
        "/instructions",
        &format!("X-Connect-Session: {capability}\r\nOrigin: http://{authority}\r\n"),
    )
    .await;
    assert!(response.starts_with("HTTP/1.1 200"));
    for header in [
        "Cache-Control: no-store",
        "Referrer-Policy: no-referrer",
        "Content-Security-Policy: default-src 'none'",
        "X-Content-Type-Options: nosniff",
    ] {
        assert!(response.contains(header));
    }
    let page = request(&authority, "/", "").await;
    assert!(!page.contains("oauth/authorize"));
    assert!(!page.contains(&capability));
    assert!(page.contains("history.replaceState"));
    assert!(page.contains("rel=\"noreferrer\""));
    serving.abort();
    assert!(matches!(serving.await, Err(error) if error.is_cancelled()));
    assert!(TcpStream::connect(&authority).await.is_err());
}

#[tokio::test]
async fn callback_origin_is_optional_but_exact_if_present_and_denial_retires_session() {
    let redirect = redirect();
    let endpoint = BoundOAuthEndpoint::bind_pkce(config(&redirect)).unwrap();
    let (authority, capability) = browser_parts(&endpoint);
    let serving = tokio::spawn(endpoint.receive());
    let body = instructions(&authority, &capability).await;
    let uri = url::Url::parse(body["authorization_url"].as_str().unwrap()).unwrap();
    let state = uri
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();
    let target = format!("/oauth/callback?state={state}&error=access_denied");
    for origin in [
        "null",
        "http://gitlab.example",
        "https://gitlab.example/",
        "https://evil.example",
    ] {
        assert!(
            request(&authority, &target, &format!("Origin: {origin}\r\n"))
                .await
                .contains("origin_refused")
        );
    }
    let reply = request(&authority, &target, "Origin: https://gitlab.example\r\n").await;
    assert!(reply.contains("code_exchange_refused"));
    assert_eq!(
        serving.await.unwrap().err(),
        Some(OAuthTransportError::CodeExchangeRefused)
    );
    assert!(TcpStream::connect(&authority).await.is_err());
}

#[tokio::test]
async fn fixed_port_conflict_and_drop_do_not_fall_back_or_leave_a_listener() {
    let redirect = redirect();
    let endpoint = BoundOAuthEndpoint::bind_pkce(config(&redirect)).unwrap();
    let (authority, _) = browser_parts(&endpoint);
    assert_eq!(
        BoundOAuthEndpoint::bind_pkce(config(&redirect)).err(),
        Some(OAuthTransportError::PortInUse)
    );
    drop(endpoint);
    assert!(TcpStream::connect(&authority).await.is_err());
    assert!(BoundOAuthEndpoint::bind_pkce(config(&redirect)).is_ok());
}

#[tokio::test(start_paused = true)]
async fn expiry_caps_a_stalled_read_and_future_drop_closes_the_port() {
    let redirect = redirect();
    let mut policy = config(&redirect);
    policy.deadline = Instant::now() + Duration::from_secs(2);
    let endpoint = BoundOAuthEndpoint::bind_pkce(policy).unwrap();
    let (authority, _) = browser_parts(&endpoint);
    let serving = tokio::spawn(endpoint.receive());
    let mut stream = TcpStream::connect(&authority).await.unwrap();
    stream.write_all(b"GET / HTTP/1.1\r\nHost:").await.unwrap();
    tokio::time::advance(Duration::from_secs(2)).await;
    assert_eq!(
        serving.await.unwrap().err(),
        Some(OAuthTransportError::Expired)
    );
    assert!(TcpStream::connect(&authority).await.is_err());
}

#[tokio::test]
async fn device_bridge_shows_only_human_instructions_and_has_no_callback() {
    let authorization = device();
    let endpoint = BoundOAuthEndpoint::bind_device(
        &authorization,
        1_000,
        Instant::now() + Duration::from_secs(30),
    )
    .unwrap();
    let (authority, capability) = browser_parts(&endpoint);
    let serving = tokio::spawn(endpoint.receive());
    let body = instructions(&authority, &capability).await;
    assert_eq!(body["kind"], "device_authorization");
    assert_eq!(body["user_code"], "ABCD-EFGH");
    assert!(body["verification_uri_complete"].is_null());
    assert!(!body.to_string().contains("PRIVATE-DEVICE-CODE"));
    assert!(request(&authority, "/oauth/callback?state=a&code=b", "")
        .await
        .contains("malformed_callback"));
    serving.abort();
    assert!(matches!(serving.await, Err(error) if error.is_cancelled()));
    assert!(TcpStream::connect(&authority).await.is_err());
}

#[tokio::test]
async fn oauth_and_raw_completion_endpoints_remain_independent() {
    let directory = tempfile::tempdir().unwrap();
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let raw = crate::BoundCompletionEndpoint::bind(directory.path(), "raw-cross-case").unwrap();
    let raw_url = url::Url::parse(&raw.browser_url()).unwrap();
    let raw_authority = format!("127.0.0.1:{}", raw_url.port().unwrap());
    let oauth =
        BoundOAuthEndpoint::bind_device(&device(), 1_000, Instant::now() + Duration::from_secs(30))
            .unwrap();
    let (authority, _) = browser_parts(&oauth);
    assert_ne!(authority, raw_authority);
    let serving = tokio::spawn(oauth.receive());
    assert!(request(&authority, "/complete", "")
        .await
        .contains("malformed_callback"));
    drop(raw);
    assert!(TcpStream::connect(&raw_authority).await.is_err());
    serving.abort();
    let _ = serving.await;
}

#[tokio::test]
async fn accepted_connection_budget_retires_session_without_starting_a_second_flow() {
    let endpoint =
        BoundOAuthEndpoint::bind_device(&device(), 1_000, Instant::now() + Duration::from_secs(30))
            .unwrap();
    let (authority, _) = browser_parts(&endpoint);
    let serving = tokio::spawn(endpoint.receive());
    for _ in 0..MAX_REQUESTS {
        assert!(request(&authority, "/", "")
            .await
            .starts_with("HTTP/1.1 200"));
    }
    assert_eq!(
        serving.await.unwrap().err(),
        Some(OAuthTransportError::RequestLimit)
    );
    assert!(TcpStream::connect(&authority).await.is_err());
}

#[tokio::test]
async fn local_pkce_binding_inconsistency_is_a_safe_terminal_refusal() {
    let redirect = redirect();
    let mut endpoint = BoundOAuthEndpoint::bind_pkce(config(&redirect)).unwrap();
    let (authority, capability) = browser_parts(&endpoint);
    endpoint.pending.as_mut().unwrap().challenge = "inconsistent-local-binding".into();
    let serving = tokio::spawn(endpoint.receive());
    let body = instructions(&authority, &capability).await;
    let uri = url::Url::parse(body["authorization_url"].as_str().unwrap()).unwrap();
    let state = uri
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();
    let response = request(
        &authority,
        &format!("/oauth/callback?state={state}&code=PRIVATE-CODE"),
        "",
    )
    .await;
    assert!(response.contains("pkce_binding_mismatch"));
    assert!(!response.contains("PRIVATE-CODE"));
    assert_eq!(
        serving.await.unwrap().err(),
        Some(OAuthTransportError::PkceBindingMismatch)
    );
    assert!(TcpStream::connect(&authority).await.is_err());
}

#[tokio::test(start_paused = true)]
async fn request_read_deadline_is_five_seconds_and_size_bound_never_waits_for_a_newline() {
    // A fully in-memory stream avoids paused Tokio time advancing while OS readiness arrives.
    let (mut client, mut server) = tokio::io::duplex(MAX_HEADERS * 2);
    client.write_all(b"GET / HTTP/1.1\r\nHost:").await.unwrap();
    let start = Instant::now();
    assert_eq!(
        read_request(&mut server, start + READ_TIMEOUT).await.err(),
        Some(OAuthTransportError::MalformedCallback)
    );
    assert_eq!(Instant::now() - start, Duration::from_secs(5));
    client.write_all(&vec![b'x'; MAX_HEADERS]).await.unwrap();
    let start = Instant::now();
    assert_eq!(
        read_request(&mut server, start + READ_TIMEOUT).await.err(),
        Some(OAuthTransportError::MalformedCallback)
    );
    assert_eq!(
        Instant::now(),
        start,
        "the allocation limit refuses without waiting for the read deadline"
    );
}

#[tokio::test(start_paused = true)]
async fn device_deadline_is_capped_by_private_authorization_expiry() {
    let endpoint = BoundOAuthEndpoint::bind_device(
        &device(),
        60_000,
        Instant::now() + Duration::from_secs(30),
    )
    .unwrap();
    let (authority, _) = browser_parts(&endpoint);
    let start = Instant::now();
    assert_eq!(
        endpoint.receive().await.err(),
        Some(OAuthTransportError::Expired)
    );
    assert_eq!(Instant::now() - start, Duration::from_secs(1));
    assert!(TcpStream::connect(&authority).await.is_err());
    assert_eq!(
        BoundOAuthEndpoint::bind_device(
            &device(),
            61_000,
            Instant::now() + Duration::from_secs(30)
        )
        .err(),
        Some(OAuthTransportError::Expired)
    );
}

#[tokio::test]
async fn already_accepted_replay_cannot_survive_the_callback_claim() {
    let redirect = redirect();
    let endpoint = BoundOAuthEndpoint::bind_pkce(config(&redirect)).unwrap();
    let (authority, capability) = browser_parts(&endpoint);
    let serving = tokio::spawn(endpoint.receive());
    let body = instructions(&authority, &capability).await;
    let uri = url::Url::parse(body["authorization_url"].as_str().unwrap()).unwrap();
    let state = uri
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();
    let target = format!("/oauth/callback?state={state}&code=first-code");
    let mut first = TcpStream::connect(&authority).await.unwrap();
    let mut queued = TcpStream::connect(&authority).await.unwrap();
    first
        .write_all(format!("GET {target} HTTP/1.1\r\nHost: {authority}\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let claim = serving.await.unwrap().unwrap();
    assert_eq!(&*claim.code, "first-code");
    let _ = queued
        .write_all(format!("GET {target} HTTP/1.1\r\nHost: {authority}\r\n\r\n").as_bytes())
        .await;
    let mut reply = String::new();
    let _ = queued.read_to_string(&mut reply).await;
    assert!(!reply.contains("200 OK"));
    assert!(TcpStream::connect(&authority).await.is_err());
}

#[tokio::test(start_paused = true)]
async fn already_expired_instruction_request_refuses_before_reading_or_writing() {
    let mut endpoint =
        BoundOAuthEndpoint::bind_device(&device(), 1_000, Instant::now() + Duration::from_secs(1))
            .unwrap();
    let (authority, capability) = browser_parts(&endpoint);
    let mut client = TcpStream::connect(&authority).await.unwrap();
    let (mut stream, _) = endpoint.listener.as_ref().unwrap().accept().await.unwrap();
    client.write_all(format!("GET /instructions HTTP/1.1\r\nHost: {authority}\r\nX-Connect-Session: {capability}\r\n\r\n").as_bytes()).await.unwrap();
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(
        endpoint.handle(&mut stream).await.err(),
        Some(OAuthTransportError::Expired)
    );
}

#[tokio::test]
async fn liveness_observer_is_retired_after_matching_callback() {
    let uri = redirect();
    let mut endpoint = BoundOAuthEndpoint::bind_pkce(config(&uri)).unwrap();
    let observer = endpoint.liveness();
    assert!(observer.is_live());
    let state = endpoint.pending.as_ref().unwrap().state.to_string();
    let authority = endpoint.authority.clone();
    let mut client = TcpStream::connect(&authority).await.unwrap();
    let (mut server, _) = endpoint.listener.as_ref().unwrap().accept().await.unwrap();
    client.write_all(format!("GET /oauth/callback?state={state}&code=private-code HTTP/1.1\r\nHost: {authority}\r\n\r\n").as_bytes()).await.unwrap();
    let claimed = endpoint.handle(&mut server).await.unwrap().unwrap();
    assert!(!observer.is_live());
    assert_eq!(&*claimed.code, "private-code");
    assert!(endpoint.listener.is_none());
}

#[tokio::test]
async fn liveness_observer_drop_and_rebind_cannot_revive_old_receiver() {
    let uri = redirect();
    let endpoint = BoundOAuthEndpoint::bind_pkce(config(&uri)).unwrap();
    let observer = endpoint.liveness();
    let cloned = observer.clone();
    assert!(observer.is_live());
    drop(endpoint);
    assert!(!observer.is_live());
    let replacement = BoundOAuthEndpoint::bind_pkce(config(&uri)).unwrap();
    assert!(replacement.liveness().is_live());
    assert!(!cloned.is_live());
}

#[tokio::test(start_paused = true)]
async fn liveness_observer_uses_original_receiver_deadline_without_polling_receive() {
    let uri = redirect();
    let mut policy = config(&uri);
    policy.deadline = Instant::now() + Duration::from_secs(1);
    let endpoint = BoundOAuthEndpoint::bind_pkce(policy).unwrap();
    let observer = endpoint.liveness();
    assert!(observer.is_live());
    tokio::time::advance(Duration::from_secs(1)).await;
    assert!(!observer.is_live());
    assert!(!endpoint.liveness().is_live());
}

#[tokio::test]
async fn oauth_pass1_last_callback_slot_survives_invalid_duplicates_and_closes_once() {
    for denied in [false, true] {
        let redirect = redirect();
        let endpoint = BoundOAuthEndpoint::bind_pkce(config(&redirect)).unwrap();
        let liveness = endpoint.liveness();
        let (authority, capability) = browser_parts(&endpoint);
        let task = tokio::spawn(endpoint.receive());
        let private = instructions(&authority, &capability).await;
        let authorization =
            url::Url::parse(private["authorization_url"].as_str().unwrap()).unwrap();
        let state = authorization
            .query_pairs()
            .find(|(key, _)| key == "state")
            .unwrap()
            .1
            .into_owned();
        for turn in 0..62 {
            let target = if turn % 2 == 0 {
                format!("/oauth/callback?state={state}&st%61te={state}&code=OAUTH-PASS1-PRIVATE")
            } else {
                format!(
                    "/oauth/callback?state={state}&code=OAUTH-PASS1-PRIVATE&error=access_denied"
                )
            };
            let refusal = request(&authority, &target, "").await;
            assert!(refusal.starts_with("HTTP/1.1 403"));
            assert!(!refusal.contains("OAUTH-PASS1-PRIVATE"));
            assert!(!refusal.contains(&state));
            assert!(liveness.is_live());
        }
        let final_target = if denied {
            format!("/oauth/callback?state={state}&error=access_denied")
        } else {
            format!("/oauth/callback?state={state}&code=final%2Bcode")
        };
        let response = request(
            &authority,
            &final_target,
            "Origin: https://gitlab.example\r\n",
        )
        .await;
        let result = task.await.unwrap();
        if denied {
            assert_eq!(result.err(), Some(OAuthTransportError::CodeExchangeRefused));
            assert!(response.starts_with("HTTP/1.1 403"));
        } else {
            let callback = result.unwrap();
            assert_eq!(callback.code.as_str(), "final+code");
            assert_eq!(callback.redirect_uri, redirect);
            assert!(response.starts_with("HTTP/1.1 200"));
        }
        assert!(!liveness.is_live());
        assert!(TcpStream::connect(&authority).await.is_err());
    }
}

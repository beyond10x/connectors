//! Trusted private instruction retrieval and unchanged Connection v1 polling.

use super::*;

impl LocalClient {
    /// Start only the selected deployed credential purpose. The trusted caller reserves its
    /// private instruction destination first; this creates no alternate binding or flow.
    pub async fn begin_personal_oauth(
        &self,
        context: &operation::OwnerContext,
        integration_ref: String,
        label: String,
        auth_profile: String,
        expected_connection_ref: String,
    ) -> Result<PendingPersonalOAuth, ClientError> {
        // This expectation comes from trusted configuration. Validate its existing reference
        // vocabulary locally; it is never sent as a target selector or used to grant authority.
        connection::RequestEnvelope {
            protocol: connection::CONTRACT.into(),
            request_id: request_id(),
            context: context.clone(),
            request: connection::ConnectionRequest::Describe(connection::DescribeRequest {
                connection_ref: expected_connection_ref.clone(),
            }),
        }
        .validate()
        .map_err(|_| ClientError::PersonalOAuthRefused)?;
        let result = self
            .connection_result(
                context,
                connection::ConnectionRequest::ConnectSessionCreate(
                    connection::ConnectSessionCreateRequest {
                        integration_ref: integration_ref.clone(),
                        label,
                        auth_profile: Some(auth_profile.clone()),
                    },
                ),
            )
            .await
            .map_err(|_| ClientError::PersonalOAuthRefused)?;
        let connection::ConnectionResult::ConnectSessionCreate(created) = result else {
            return Err(ClientError::InvalidResponse);
        };
        let now = oauth_now()?;
        if created.integration_ref != integration_ref
            || created.state != connection::ConnectSessionState::Pending
            || created.completion_endpoint.is_some()
            || created.expires_at_unix_ms <= now
            || created.expires_at_unix_ms - now > 600_000
        {
            return Err(ClientError::PersonalOAuthInstructions);
        }
        let browser_url = Zeroizing::new(
            created
                .browser_completion_url
                .ok_or(ClientError::PersonalOAuthInstructions)?,
        );
        oauth_instruction_endpoint(&browser_url)?;
        Ok(PendingPersonalOAuth {
            expected_connection_ref,
            integration_ref,
            auth_profile,
            session_ref: created.connect_session_ref,
            expires_at_unix_ms: created.expires_at_unix_ms,
            browser_url,
            deadline: tokio::time::Instant::now()
                + Duration::from_millis(created.expires_at_unix_ms - now),
        })
    }

    /// Fetch protected instructions only from the exact numeric loopback receiver, and validate
    /// the returned HTTPS URL against the configured provider origin before handing it to a human.
    pub async fn personal_oauth_instructions(
        &self,
        pending: &PendingPersonalOAuth,
        expected_origin: &str,
    ) -> Result<PersonalOAuthInstructions, ClientError> {
        fetch_personal_oauth_instructions(
            &pending.browser_url,
            pending.expires_at_unix_ms,
            pending.deadline,
            expected_origin,
        )
        .await
    }

    /// Poll unchanged Connection v1. Guarded completion may report Unavailable while durable I/O
    /// settles; the deadline cuts off authorization, not an already authorized FULL commit.
    pub async fn finish_personal_oauth(
        &self,
        context: &operation::OwnerContext,
        pending: &PendingPersonalOAuth,
    ) -> Result<connection::ConnectionDescription, ClientError> {
        let maximum = pending
            .deadline
            .checked_add(Duration::from_secs(15))
            .ok_or(ClientError::PersonalOAuthRefused)?;
        loop {
            if tokio::time::Instant::now() >= maximum {
                return Err(ClientError::PersonalOAuthRefused);
            }
            let response = tokio::time::timeout_at(
                maximum,
                self.connection(
                    context,
                    connection::ConnectionRequest::ConnectSessionStatus(
                        connection::ConnectSessionStatusRequest {
                            connect_session_ref: pending.session_ref.clone(),
                        },
                    ),
                ),
            )
            .await
            .map_err(|_| ClientError::PersonalOAuthRefused)?
            .map_err(|_| ClientError::PersonalOAuthRefused)?;
            match (response.status, response.response, response.error) {
                (
                    connection::ResponseStatus::Ok,
                    Some(connection::ConnectionResult::ConnectSessionStatus(status)),
                    None,
                ) => {
                    if status.connect_session_ref != pending.session_ref
                        || status.integration_ref != pending.integration_ref
                        || status.expires_at_unix_ms != pending.expires_at_unix_ms
                    {
                        return Err(ClientError::PersonalOAuthRefused);
                    }
                    match status.state {
                        connection::ConnectSessionState::Completed => {
                            let connection_ref = status
                                .connection_ref
                                .ok_or(ClientError::PersonalOAuthRefused)?;
                            if connection_ref != pending.expected_connection_ref {
                                return Err(ClientError::PersonalOAuthRefused);
                            }
                            let result = tokio::time::timeout_at(
                                maximum,
                                self.connection_result(
                                    context,
                                    connection::ConnectionRequest::Describe(
                                        connection::DescribeRequest { connection_ref },
                                    ),
                                ),
                            )
                            .await
                            .map_err(|_| ClientError::PersonalOAuthRefused)?
                            .map_err(|_| ClientError::PersonalOAuthRefused)?;
                            let connection::ConnectionResult::Describe(description) = result else {
                                return Err(ClientError::PersonalOAuthRefused);
                            };
                            if description.summary.connection_ref != pending.expected_connection_ref
                                || description.summary.integration_ref != pending.integration_ref
                                || description.summary.auth_profile.as_deref()
                                    != Some(pending.auth_profile.as_str())
                                || description.summary.state
                                    != connection::ConnectionState::Callable
                            {
                                return Err(ClientError::PersonalOAuthRefused);
                            }
                            return Ok(description);
                        }
                        connection::ConnectSessionState::Pending => {}
                        _ => return Err(ClientError::PersonalOAuthRefused),
                    }
                }
                (connection::ResponseStatus::Error, None, Some(error))
                    if error.code == connection::ConnectionErrorCode::Unavailable => {}
                _ => return Err(ClientError::PersonalOAuthRefused),
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
}

/// Shared protected instruction retrieval; the caller supplies the captured receiver deadline.
pub(super) async fn fetch_personal_oauth_instructions(
    browser_url: &str,
    expires_at_unix_ms: u64,
    deadline: tokio::time::Instant,
    expected_origin: &str,
) -> Result<PersonalOAuthInstructions, ClientError> {
    let remaining = expires_at_unix_ms
        .checked_sub(oauth_now()?)
        .filter(|remaining| *remaining > 0)
        .ok_or(ClientError::PersonalOAuthRefused)?;
    let remaining = Duration::from_millis(remaining)
        .min(deadline.saturating_duration_since(tokio::time::Instant::now()));
    if remaining.is_zero() {
        return Err(ClientError::PersonalOAuthRefused);
    }
    let (url, capability) = oauth_instruction_endpoint(browser_url)?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(remaining.min(Duration::from_secs(5)))
        .build()
        .map_err(|_| ClientError::PersonalOAuthInstructions)?;
    let mut response = client
        .get(url)
        .header("X-Connect-Session", capability.as_str())
        .send()
        .await
        .map_err(|_| ClientError::PersonalOAuthInstructions)?;
    if response.status() != reqwest::StatusCode::OK
        || response
            .content_length()
            .is_some_and(|length| length > 64 * 1024)
        || !response
            .headers()
            .get(reqwest::header::CACHE_CONTROL)
            .and_then(|header| header.to_str().ok())
            .is_some_and(|header| {
                header
                    .split(',')
                    .any(|part| part.trim().eq_ignore_ascii_case("no-store"))
            })
    {
        return Err(ClientError::PersonalOAuthInstructions);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ClientError::PersonalOAuthInstructions)?
    {
        if bytes.len().saturating_add(chunk.len()) > 64 * 1024 {
            return Err(ClientError::PersonalOAuthInstructions);
        }
        bytes.extend_from_slice(&chunk);
    }
    if oauth_now()? >= expires_at_unix_ms || tokio::time::Instant::now() >= deadline {
        return Err(ClientError::PersonalOAuthRefused);
    }
    parse_personal_oauth_instructions(&bytes, expected_origin)
}

pub(super) fn oauth_now() -> Result<u64, ClientError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .filter(|now| *now > 0)
        .ok_or(ClientError::PersonalOAuthRefused)
}
pub(super) fn oauth_instruction_endpoint(
    raw: &str,
) -> Result<(Url, Zeroizing<String>), ClientError> {
    let mut url = Url::parse(raw).map_err(|_| ClientError::PersonalOAuthInstructions)?;
    if raw.len() > 4096
        || url.as_str() != raw
        || url.scheme() != "http"
        || url.host_str() != Some("127.0.0.1")
        || url.port().is_none_or(|port| port == 0)
        || url.path() != "/"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
    {
        return Err(ClientError::PersonalOAuthInstructions);
    }
    let capability = url
        .fragment()
        .and_then(|fragment| fragment.strip_prefix("token="))
        .ok_or(ClientError::PersonalOAuthInstructions)?;
    if capability.len() < 32
        || capability.len() > 128
        || !capability
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ClientError::PersonalOAuthInstructions);
    }
    let capability = Zeroizing::new(capability.to_owned());
    url.set_fragment(None);
    url.set_path("/instructions");
    Ok((url, capability))
}
fn parse_personal_oauth_instructions(
    bytes: &[u8],
    expected_origin: &str,
) -> Result<PersonalOAuthInstructions, ClientError> {
    let origin = Url::parse(expected_origin).map_err(|_| ClientError::PersonalOAuthInstructions)?;
    if origin.scheme() != "https"
        || origin.host_str().is_none()
        || origin.path() != "/"
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.query().is_some()
        || origin.fragment().is_some()
    {
        return Err(ClientError::PersonalOAuthInstructions);
    }
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| ClientError::PersonalOAuthInstructions)?;
    let human_url = |value: &serde_json::Value| -> Result<Zeroizing<String>, ClientError> {
        let raw = value
            .as_str()
            .ok_or(ClientError::PersonalOAuthInstructions)?;
        let url = Url::parse(raw).map_err(|_| ClientError::PersonalOAuthInstructions)?;
        if raw.len() > 16 * 1024
            || raw.chars().any(char::is_control)
            || url.scheme() != "https"
            || url.origin() != origin.origin()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            return Err(ClientError::PersonalOAuthInstructions);
        }
        Ok(Zeroizing::new(raw.to_owned()))
    };
    match value["kind"].as_str() {
        Some("browser_authorization") => Ok(PersonalOAuthInstructions {
            url: human_url(&value["authorization_url"])?,
            user_code: None,
        }),
        Some("device_authorization") => {
            let verification = human_url(&value["verification_uri"])?;
            let url = if value["verification_uri_complete"].is_null() {
                verification
            } else {
                human_url(&value["verification_uri_complete"])?
            };
            let code = value["user_code"]
                .as_str()
                .ok_or(ClientError::PersonalOAuthInstructions)?;
            if code.is_empty() || code.len() > 256 || code.chars().any(char::is_control) {
                return Err(ClientError::PersonalOAuthInstructions);
            }
            Ok(PersonalOAuthInstructions {
                url,
                user_code: Some(Zeroizing::new(code.to_owned())),
            })
        }
        _ => Err(ClientError::PersonalOAuthInstructions),
    }
}

#[cfg(test)]
mod personal_oauth_tests {
    use super::*;

    #[test]
    fn personal_instruction_destination_is_exact_numeric_loopback_and_capability_is_header_only() {
        let capability = "a".repeat(43);
        let good = format!("http://127.0.0.1:18324/#token={capability}");
        let (destination, secret) = oauth_instruction_endpoint(&good).unwrap();
        assert_eq!(destination.as_str(), "http://127.0.0.1:18324/instructions");
        assert_eq!(&*secret, &capability);
        for raw in [
            good.replace("127.0.0.1", "localhost"),
            good.replace("127.0.0.1", "127.1"),
            good.replace("http:", "https:"),
            good.replace(":18324", ""),
            good.replace(":18324", ":80"),
            good.replace("/#", "/instructions#"),
            good.replace("/#", "/?target=remote#"),
            good.replace("token=", "capability="),
            good.replace("127.0.0.1", "user@127.0.0.1"),
        ] {
            assert!(oauth_instruction_endpoint(&raw).is_err());
        }
    }

    #[test]
    fn personal_instruction_parser_preserves_optional_device_uri_and_refuses_origin_changes() {
        let value = serde_json::json!({ "kind": "device_authorization", "verification_uri": "https://gitlab.example/oauth/device",
            "user_code": "FIXTURE-CODE", "verification_uri_complete": null });
        let parsed = parse_personal_oauth_instructions(
            value.to_string().as_bytes(),
            "https://gitlab.example",
        )
        .unwrap();
        let mut private_output = Vec::new();
        parsed.write_human(&mut private_output).unwrap();
        let text = String::from_utf8(private_output).unwrap();
        assert!(text.contains("FIXTURE-CODE"));
        assert!(text.contains("https://gitlab.example/oauth/device"));
        assert!(
            !text.contains("user_code="),
            "an absent complete URI is not invented"
        );
        for uri in [
            "https://other.example/oauth/device",
            "http://gitlab.example/oauth/device",
            "https://user@gitlab.example/oauth/device",
            "https://gitlab.example/oauth/device#code",
        ] {
            let mut altered = value.clone();
            altered["verification_uri_complete"] = serde_json::json!(uri);
            assert!(parse_personal_oauth_instructions(
                altered.to_string().as_bytes(),
                "https://gitlab.example"
            )
            .is_err());
        }
    }

    #[test]
    fn private_browser_authorization_is_never_a_provider_independent_redirect() {
        let value = serde_json::json!({ "kind": "browser_authorization", "authorization_url": "https://gitlab.example/oauth/authorize?state=FIXTURE" });
        assert!(parse_personal_oauth_instructions(
            value.to_string().as_bytes(),
            "https://gitlab.example"
        )
        .is_ok());
        assert!(parse_personal_oauth_instructions(
            value.to_string().as_bytes(),
            "https://another.example"
        )
        .is_err());
        let error = parse_personal_oauth_instructions(
            b"{\"kind\":\"browser_authorization\",\"authorization_url\":\"PRIVATE-SENTINEL\"}",
            "https://gitlab.example",
        )
        .err()
        .unwrap();
        assert!(!error.to_string().contains("PRIVATE-SENTINEL"));
    }

    fn pending_instructions(authority: &str) -> PendingPersonalOAuth {
        PendingPersonalOAuth {
            expected_connection_ref: "connection:fixture".into(),
            integration_ref: "gitlab".into(),
            auth_profile: "gitlab.oauth_token".into(),
            session_ref: "session:fixture".into(),
            expires_at_unix_ms: oauth_now().unwrap() + 30_000,
            browser_url: Zeroizing::new(format!("http://{authority}/#token={}", "p".repeat(43))),
            deadline: tokio::time::Instant::now() + Duration::from_secs(30),
        }
    }

    #[tokio::test]
    async fn actual_private_instruction_request_keeps_capability_out_of_url_and_delivers_only_to_human_writer(
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let authority = listener.local_addr().unwrap().to_string();
        let pending = pending_instructions(&authority);
        let serving = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            loop {
                let mut byte = [0];
                stream.read_exact(&mut byte).await.unwrap();
                request.push(byte[0]);
                if request.ends_with(b"\r\n\r\n") {
                    break;
                }
                assert!(request.len() < 8192);
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request.starts_with("GET /instructions HTTP/1.1\r\n"));
            assert!(request
                .to_ascii_lowercase()
                .contains(&format!("x-connect-session: {}\r\n", "p".repeat(43))));
            assert!(!request.lines().next().unwrap().contains(&"p".repeat(43)));
            let body = r#"{"kind":"browser_authorization","authorization_url":"https://gitlab.example/oauth/authorize?state=PRIVATE-AUTHORIZATION"}"#;
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
        });
        let client = LocalClient::new("/synthetic/unused.sock");
        let instructions = client
            .personal_oauth_instructions(&pending, "https://gitlab.example")
            .await
            .unwrap();
        let mut destination = Vec::new();
        instructions.write_human(&mut destination).unwrap();
        assert!(String::from_utf8(destination)
            .unwrap()
            .contains("PRIVATE-AUTHORIZATION"));
        serving.await.unwrap();
    }

    #[tokio::test]
    async fn actual_instruction_redirect_and_cacheable_reply_are_closed_refusals() {
        for status in ["302 Found", "200 OK"] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let pending = pending_instructions(&listener.local_addr().unwrap().to_string());
            let serving = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut reader = BufReader::new(&mut stream);
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).await.unwrap();
                    if line == "\r\n" {
                        break;
                    }
                }
                stream.write_all(format!("HTTP/1.1 {status}\r\nLocation: https://other.example/PRIVATE-REDIRECT\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
            });
            let result = LocalClient::new("/synthetic/unused.sock")
                .personal_oauth_instructions(&pending, "https://gitlab.example")
                .await;
            let error = result.err().unwrap().to_string();
            assert!(!error.contains("PRIVATE-REDIRECT"));
            assert!(!error.contains(&"p".repeat(43)));
            serving.await.unwrap();
        }
    }

    #[tokio::test]
    async fn expired_monotonic_instruction_budget_never_connects_even_with_future_wall_deadline() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut pending = pending_instructions(&listener.local_addr().unwrap().to_string());
        pending.deadline = tokio::time::Instant::now() - Duration::from_millis(1);
        let result = LocalClient::new("/synthetic/unused.sock")
            .personal_oauth_instructions(&pending, "https://gitlab.example")
            .await;
        assert!(result.is_err());
        assert!(
            tokio::time::timeout(Duration::from_millis(25), listener.accept())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn actual_connection_v1_polling_keeps_guarded_completion_private_and_never_repeats_create(
    ) {
        let root = tempfile::tempdir().unwrap();
        let socket = root.path().join("connectors.sock");
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        let deadline = oauth_now().unwrap() + 30_000;
        let serving = tokio::spawn(async move {
            for turn in 0..4 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut line = String::new();
                BufReader::new(&mut stream)
                    .read_line(&mut line)
                    .await
                    .unwrap();
                let request: connection::RequestEnvelope = serde_json::from_str(&line).unwrap();
                request.validate().unwrap();
                let status = |state, browser_completion_url, connection_ref| {
                    connection::ConnectSessionStatus {
                        connect_session_ref: "session:fixture".into(),
                        integration_ref: "gitlab".into(),
                        state,
                        expires_at_unix_ms: deadline,
                        completion_endpoint: None,
                        browser_completion_url,
                        connection_ref,
                    }
                };
                let response = match (turn, request.request) {
                    (0, connection::ConnectionRequest::ConnectSessionCreate(create)) => {
                        assert_eq!(create.auth_profile.as_deref(), Some("gitlab.oauth_token"));
                        connection::ResponseEnvelope::success(
                            request.request_id,
                            connection::ConnectionResult::ConnectSessionCreate(status(
                                connection::ConnectSessionState::Pending,
                                Some(format!("http://127.0.0.1:47193/#token={}", "p".repeat(43))),
                                None,
                            )),
                        )
                    }
                    (1, connection::ConnectionRequest::ConnectSessionStatus(poll)) => {
                        assert_eq!(poll.connect_session_ref, "session:fixture");
                        connection::ResponseEnvelope::failure(
                            request.request_id,
                            connection::ConnectionError::new(
                                connection::ConnectionErrorCode::Unavailable,
                                "completion is guarded",
                                true,
                            ),
                        )
                    }
                    (2, connection::ConnectionRequest::ConnectSessionStatus(poll)) => {
                        assert_eq!(poll.connect_session_ref, "session:fixture");
                        connection::ResponseEnvelope::success(
                            request.request_id,
                            connection::ConnectionResult::ConnectSessionStatus(status(
                                connection::ConnectSessionState::Completed,
                                None,
                                Some("connection:fixture".into()),
                            )),
                        )
                    }
                    (3, connection::ConnectionRequest::Describe(describe)) => {
                        assert_eq!(describe.connection_ref, "connection:fixture");
                        connection::ResponseEnvelope::success(
                            request.request_id,
                            connection::ConnectionResult::Describe(
                                connection::ConnectionDescription {
                                    summary: connection::ConnectionSummary {
                                        connection_ref: describe.connection_ref,
                                        integration_ref: "gitlab".into(),
                                        label: "Display only".into(),
                                        state: connection::ConnectionState::Callable,
                                        initiation: vec![connection::ConnectionInitiator::Platform],
                                        route: connection::ConnectionRoute::Direct,
                                        scope: None,
                                        actor: None,
                                        auth_profile: Some("gitlab.oauth_token".into()),
                                    },
                                    channels: Vec::new(),
                                },
                            ),
                        )
                    }
                    _ => panic!(
                        "the client must create once, poll the same session, then describe its returned binding"
                    ),
                };
                response.validate().unwrap();
                let mut bytes = serde_json::to_vec(&response).unwrap();
                bytes.push(b'\n');
                stream.write_all(&bytes).await.unwrap();
            }
        });
        let context = operation::OwnerContext {
            tenant_id: "fixture".into(),
            agent_id: "fixture".into(),
            agent_revision: 1,
            authority_snapshot_id: "fixture".into(),
            authority_snapshot_sha256: "a".repeat(64),
        };
        let client = LocalClient::new(&socket);
        let pending = client
            .begin_personal_oauth(
                &context,
                "gitlab".into(),
                "Display only".into(),
                "gitlab.oauth_token".into(),
                "connection:fixture".into(),
            )
            .await
            .unwrap();
        let description = client
            .finish_personal_oauth(&context, &pending)
            .await
            .unwrap();
        assert_eq!(description.summary.connection_ref, "connection:fixture");
        serving.await.unwrap();
    }

    #[tokio::test]
    async fn valid_browser_only_session_reaches_the_explicit_personal_handoff() {
        let root = tempfile::tempdir().unwrap();
        let socket = root.path().join("connectors.sock");
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        let serving = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut line = String::new();
            BufReader::new(&mut stream)
                .read_line(&mut line)
                .await
                .unwrap();
            let request: connection::RequestEnvelope = serde_json::from_str(&line).unwrap();
            let connection::ConnectionRequest::ConnectSessionCreate(create) = request.request
            else {
                panic!("one Create")
            };
            assert_eq!(create.auth_profile.as_deref(), Some("gitlab.oauth_token"));
            let response = connection::ResponseEnvelope::success(
                request.request_id,
                connection::ConnectionResult::ConnectSessionCreate(
                    connection::ConnectSessionStatus {
                        connect_session_ref: "session:fixture".into(),
                        integration_ref: "gitlab".into(),
                        state: connection::ConnectSessionState::Pending,
                        expires_at_unix_ms: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64
                            + 30_000,
                        completion_endpoint: None,
                        browser_completion_url: Some(format!(
                            "http://127.0.0.1:47193/#token={}",
                            "p".repeat(43)
                        )),
                        connection_ref: None,
                    },
                ),
            );
            response.validate().unwrap();
            let mut bytes = serde_json::to_vec(&response).unwrap();
            bytes.push(b'\n');
            stream.write_all(&bytes).await.unwrap();
        });
        let context = operation::OwnerContext {
            tenant_id: "fixture".into(),
            agent_id: "fixture".into(),
            agent_revision: 1,
            authority_snapshot_id: "fixture".into(),
            authority_snapshot_sha256: "a".repeat(64),
        };
        let client = LocalClient::new(socket);
        let outcome = client
            .begin_personal_oauth(
                &context,
                "gitlab".into(),
                "Display only".into(),
                "gitlab.oauth_token".into(),
                "connection:fixture".into(),
            )
            .await;
        serving.await.unwrap();
        assert!(
            outcome.is_ok(),
            "a valid browser-only response needs the explicit private handoff"
        );
        assert_eq!(outcome.unwrap().session_ref(), "session:fixture");
    }

    #[tokio::test]
    async fn personal_create_refusal_cannot_echo_private_daemon_text() {
        let root = tempfile::tempdir().unwrap();
        let socket = root.path().join("connectors.sock");
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        let serving = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut line = String::new();
            BufReader::new(&mut stream)
                .read_line(&mut line)
                .await
                .unwrap();
            let request: connection::RequestEnvelope = serde_json::from_str(&line).unwrap();
            assert!(matches!(
                request.request,
                connection::ConnectionRequest::ConnectSessionCreate(_)
            ));
            let response = connection::ResponseEnvelope::failure(
                request.request_id,
                connection::ConnectionError::new(
                    connection::ConnectionErrorCode::InvalidInput,
                    "PRIVATE-AUTHORIZATION-DAEMON-FIXTURE",
                    false,
                ),
            );
            response.validate().unwrap();
            let mut bytes = serde_json::to_vec(&response).unwrap();
            bytes.push(b'\n');
            stream.write_all(&bytes).await.unwrap();
        });
        let context = operation::OwnerContext {
            tenant_id: "fixture".into(),
            agent_id: "fixture".into(),
            agent_revision: 1,
            authority_snapshot_id: "fixture".into(),
            authority_snapshot_sha256: "a".repeat(64),
        };
        let error = LocalClient::new(socket)
            .begin_personal_oauth(
                &context,
                "gitlab".into(),
                "Display only".into(),
                "gitlab.oauth_token".into(),
                "connection:fixture".into(),
            )
            .await
            .err()
            .expect("daemon refusal");
        serving.await.unwrap();
        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(
            !display.contains("PRIVATE-AUTHORIZATION"),
            "public display must be closed"
        );
        assert!(
            !debug.contains("PRIVATE-AUTHORIZATION"),
            "public debug must be closed"
        );
    }

    #[tokio::test]
    async fn personal_poll_and_describe_refusals_close_inner_transport_and_daemon_text() {
        for describe in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let socket = root.path().join("connectors.sock");
            let listener = tokio::net::UnixListener::bind(&socket).unwrap();
            let pending = pending_instructions("127.0.0.1:47193");
            let expires_at_unix_ms = pending.expires_at_unix_ms;
            let serving = tokio::spawn(async move {
                for turn in 0..=usize::from(describe) {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let mut line = String::new();
                    BufReader::new(&mut stream)
                        .read_line(&mut line)
                        .await
                        .unwrap();
                    let request: connection::RequestEnvelope = serde_json::from_str(&line).unwrap();
                    let response = if describe && turn == 0 {
                        assert!(matches!(
                            request.request,
                            connection::ConnectionRequest::ConnectSessionStatus(_)
                        ));
                        connection::ResponseEnvelope::success(
                            request.request_id,
                            connection::ConnectionResult::ConnectSessionStatus(
                                connection::ConnectSessionStatus {
                                    connect_session_ref: "session:fixture".into(),
                                    integration_ref: "gitlab".into(),
                                    state: connection::ConnectSessionState::Completed,
                                    expires_at_unix_ms,
                                    completion_endpoint: None,
                                    browser_completion_url: None,
                                    connection_ref: Some("connection:fixture".into()),
                                },
                            ),
                        )
                    } else {
                        if describe {
                            assert!(matches!(
                                request.request,
                                connection::ConnectionRequest::Describe(_)
                            ));
                        }
                        connection::ResponseEnvelope::failure(
                            request.request_id,
                            connection::ConnectionError::new(
                                connection::ConnectionErrorCode::InvalidInput,
                                "PRIVATE-AUTHORIZATION-DAEMON-FIXTURE",
                                false,
                            ),
                        )
                    };
                    response.validate().unwrap();
                    let mut value = serde_json::to_value(response).unwrap();
                    if !describe {
                        value["status"] =
                            serde_json::json!("PRIVATE-AUTHORIZATION-TRANSPORT-FIXTURE");
                    }
                    let mut bytes = serde_json::to_vec(&value).unwrap();
                    bytes.push(b'\n');
                    stream.write_all(&bytes).await.unwrap();
                }
            });
            let context = operation::OwnerContext {
                tenant_id: "fixture".into(),
                agent_id: "fixture".into(),
                agent_revision: 1,
                authority_snapshot_id: "fixture".into(),
                authority_snapshot_sha256: "a".repeat(64),
            };
            let error = LocalClient::new(socket)
                .finish_personal_oauth(&context, &pending)
                .await
                .err()
                .unwrap();
            serving.await.unwrap();
            assert!(matches!(error, ClientError::PersonalOAuthRefused));
            assert!(!error.to_string().contains("PRIVATE-AUTHORIZATION"));
            assert!(!format!("{error:?}").contains("PRIVATE-AUTHORIZATION"));
        }
    }

    #[derive(Clone, Copy)]
    enum SuccessFixture {
        Valid,
        Created,
        Degraded,
        WrongSession,
        WrongStatusIntegration,
        ChangedDeadline,
        WrongDescribeTarget,
        ConsistentPrivateTarget,
        WrongProfile,
    }

    async fn verify_personal_success_boundary(fixture: SuccessFixture) {
        let root = tempfile::tempdir().unwrap();
        let socket = root.path().join("connectors.sock");
        let listener = tokio::net::UnixListener::bind(&socket).unwrap();
        let deadline = oauth_now().unwrap() + 30_000;
        let (stop, mut stopped) = tokio::sync::oneshot::channel();
        let descriptions = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = descriptions.clone();
        let serving = tokio::spawn(async move {
            loop {
                let (mut stream, _) = tokio::select! {
                    accepted = listener.accept() => accepted.unwrap(),
                    _ = &mut stopped => break,
                };
                let mut line = String::new();
                BufReader::new(&mut stream)
                    .read_line(&mut line)
                    .await
                    .unwrap();
                let request: connection::RequestEnvelope = serde_json::from_str(&line).unwrap();
                request.validate().unwrap();
                let target = if matches!(fixture, SuccessFixture::ConsistentPrivateTarget) {
                    "connection:PRIVATE-AUTHORIZATION-FIXTURE"
                } else {
                    "connection:fixture"
                };
                let result = match request.request {
                    connection::ConnectionRequest::ConnectSessionCreate(create) => {
                        assert_eq!(create.auth_profile.as_deref(), Some("gitlab.oauth_token"));
                        assert_eq!(create.label, "Trusted display");
                        connection::ConnectionResult::ConnectSessionCreate(
                            connection::ConnectSessionStatus {
                                connect_session_ref: "session:fixture".into(),
                                integration_ref: "gitlab".into(),
                                state: connection::ConnectSessionState::Pending,
                                expires_at_unix_ms: deadline,
                                completion_endpoint: None,
                                browser_completion_url: Some(format!(
                                    "http://127.0.0.1:47193/#token={}",
                                    "p".repeat(43)
                                )),
                                connection_ref: None,
                            },
                        )
                    }
                    connection::ConnectionRequest::ConnectSessionStatus(poll) => {
                        assert_eq!(poll.connect_session_ref, "session:fixture");
                        connection::ConnectionResult::ConnectSessionStatus(
                            connection::ConnectSessionStatus {
                                connect_session_ref: if matches!(
                                    fixture,
                                    SuccessFixture::WrongSession
                                ) {
                                    "session:other"
                                } else {
                                    "session:fixture"
                                }
                                .into(),
                                integration_ref: if matches!(
                                    fixture,
                                    SuccessFixture::WrongStatusIntegration
                                ) {
                                    "other"
                                } else {
                                    "gitlab"
                                }
                                .into(),
                                state: connection::ConnectSessionState::Completed,
                                expires_at_unix_ms: deadline
                                    + u64::from(matches!(fixture, SuccessFixture::ChangedDeadline)),
                                completion_endpoint: None,
                                browser_completion_url: None,
                                connection_ref: Some(target.into()),
                            },
                        )
                    }
                    connection::ConnectionRequest::Describe(describe) => {
                        observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        assert_eq!(describe.connection_ref, target);
                        connection::ConnectionResult::Describe(connection::ConnectionDescription {
                            summary: connection::ConnectionSummary {
                                connection_ref: if matches!(
                                    fixture,
                                    SuccessFixture::WrongDescribeTarget
                                ) {
                                    "connection:other"
                                } else {
                                    target
                                }
                                .into(),
                                integration_ref: "gitlab".into(),
                                label: "Daemon display".into(),
                                state: match fixture {
                                    SuccessFixture::Created => connection::ConnectionState::Created,
                                    SuccessFixture::Degraded => {
                                        connection::ConnectionState::Degraded
                                    }
                                    _ => connection::ConnectionState::Callable,
                                },
                                initiation: vec![connection::ConnectionInitiator::Platform],
                                route: connection::ConnectionRoute::Direct,
                                scope: None,
                                actor: None,
                                auth_profile: Some(
                                    if matches!(fixture, SuccessFixture::WrongProfile) {
                                        "gitlab.other_purpose"
                                    } else {
                                        "gitlab.oauth_token"
                                    }
                                    .into(),
                                ),
                            },
                            channels: Vec::new(),
                        })
                    }
                    _ => panic!(
                        "only one acquisition and its bounded status/description are admitted"
                    ),
                };
                let response = connection::ResponseEnvelope::success(request.request_id, result);
                response.validate().unwrap();
                let mut bytes = serde_json::to_vec(&response).unwrap();
                bytes.push(b'\n');
                stream.write_all(&bytes).await.unwrap();
            }
        });
        let context = operation::OwnerContext {
            tenant_id: "fixture".into(),
            agent_id: "fixture".into(),
            agent_revision: 1,
            authority_snapshot_id: "fixture".into(),
            authority_snapshot_sha256: "a".repeat(64),
        };
        let client = LocalClient::new(socket);
        let pending = client
            .begin_personal_oauth(
                &context,
                "gitlab".into(),
                "Trusted display".into(),
                "gitlab.oauth_token".into(),
                "connection:fixture".into(),
            )
            .await
            .unwrap();
        let result = client.finish_personal_oauth(&context, &pending).await;
        let _ = stop.send(());
        serving.await.unwrap();
        if matches!(fixture, SuccessFixture::Valid) {
            let description = result.unwrap();
            assert_eq!(description.summary.connection_ref, "connection:fixture");
            assert_eq!(
                description.summary.label, "Daemon display",
                "a returned description retains its received meaning"
            );
        } else {
            assert!(
                result.is_err(),
                "a valid envelope is insufficient for this acquisition's success"
            );
            if matches!(
                fixture,
                SuccessFixture::WrongSession
                    | SuccessFixture::WrongStatusIntegration
                    | SuccessFixture::ChangedDeadline
                    | SuccessFixture::ConsistentPrivateTarget
            ) {
                assert_eq!(
                    descriptions.load(std::sync::atomic::Ordering::SeqCst),
                    0,
                    "an unbound status must not trigger Describe"
                );
            }
        }
    }

    #[tokio::test]
    async fn personal_success_retains_a_callable_correlated_description() {
        verify_personal_success_boundary(SuccessFixture::Valid).await;
    }
    #[tokio::test]
    async fn personal_success_refuses_created_description() {
        verify_personal_success_boundary(SuccessFixture::Created).await;
    }
    #[tokio::test]
    async fn personal_success_refuses_degraded_description() {
        verify_personal_success_boundary(SuccessFixture::Degraded).await;
    }
    #[tokio::test]
    async fn personal_success_refuses_another_session() {
        verify_personal_success_boundary(SuccessFixture::WrongSession).await;
    }
    #[tokio::test]
    async fn personal_success_refuses_another_integration_status() {
        verify_personal_success_boundary(SuccessFixture::WrongStatusIntegration).await;
    }
    #[tokio::test]
    async fn personal_success_refuses_a_changed_deadline() {
        verify_personal_success_boundary(SuccessFixture::ChangedDeadline).await;
    }
    #[tokio::test]
    async fn personal_success_refuses_another_describe_target() {
        verify_personal_success_boundary(SuccessFixture::WrongDescribeTarget).await;
    }
    #[tokio::test]
    async fn personal_success_refuses_a_private_target_repeated_consistently() {
        verify_personal_success_boundary(SuccessFixture::ConsistentPrivateTarget).await;
    }
    #[tokio::test]
    async fn personal_success_refuses_another_credential_purpose() {
        verify_personal_success_boundary(SuccessFixture::WrongProfile).await;
    }
}

use super::*;

#[tokio::test]
async fn oauth_pass1_explicit_reauthorization_repairs_only_coherent_subject_after_rotation() {
    let directory = tempfile::tempdir().unwrap();
    let configured = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("initial-access", Some("initial-refresh"), 1);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(
        directory.path(),
        std::slice::from_ref(&configured),
        egress.clone(),
        clock.clone(),
    )
    .await;
    let initial = authorize(&backend).await;
    let request = invocation(&backend).await;
    clock.advance(1_001);
    egress.token("uncertain-rotation", Some("uncertain-refresh"), 60);
    egress.info("wrong-client", 42, &["read_api"]);
    assert!(backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(request.clone())
        )
        .await
        .is_err());
    assert_eq!(egress.count(), 4);
    // Close the actual owner without the helpful shutdown recovery path.
    tokio::time::timeout(Duration::from_secs(5), async {
        while Arc::strong_count(&backend.inner) != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    drop(backend);
    let backend = open(
        directory.path(),
        std::slice::from_ref(&configured),
        egress.clone(),
        clock.clone(),
    )
    .await;
    assert!(backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(request.clone())
        )
        .await
        .is_err());
    assert_eq!(egress.count(), 4);
    for subject in [43, 42] {
        egress.token(
            "explicit-repair-access",
            Some("explicit-repair-refresh"),
            60,
        );
        egress.info("fixture-client", subject, &["read_api"]);
        let pending = deliver_code(&backend).await;
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let finished = backend.inner.sessions.lock().unwrap()[&pending.connect_session_ref]
                    .task
                    .as_ref()
                    .unwrap()
                    .is_finished();
                if finished {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let result = backend
            .inner
            .session_status(&pending.connect_session_ref)
            .await
            .unwrap();
        if subject == 43 {
            assert_eq!(result.state, connection_api::ConnectSessionState::Failed);
            assert!(backend
                .inner
                .reconcile_marker(&backend.inner.bindings[0])
                .unwrap());
            assert!(backend
                .handle(
                    &owner(),
                    operation_api::OperationRequest::Invoke(request.clone())
                )
                .await
                .is_err());
            assert_eq!(egress.count(), 6);
        } else {
            assert_eq!(result.state, connection_api::ConnectSessionState::Completed);
            assert_eq!(result.connection_ref, initial.connection_ref);
            assert!(!backend
                .inner
                .reconcile_marker(&backend.inner.bindings[0])
                .unwrap());
        }
    }
    assert_eq!(egress.count(), 8);
    drop(backend);
    let reopened = open(directory.path(), &[configured], egress.clone(), clock).await;
    egress.reply(
        429,
        serde_json::json!({"error":"OAUTH-PASS1-PRIVATE-provider-echo"}),
    );
    let refusal = reopened
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(invocation(&reopened).await),
        )
        .await
        .unwrap_err();
    assert_eq!(refusal.code, operation_api::OperationErrorCode::RateLimited);
    assert!(!serde_json::to_string(&refusal)
        .unwrap()
        .contains("OAUTH-PASS1-PRIVATE"));
    assert_eq!(
        egress.count(),
        9,
        "one actual operation request; no resend or new acquisition follows 429"
    );
    {
        let requests = egress.requests.lock().unwrap();
        let headers = &requests[8].1.headers;
        assert_eq!(
            headers
                .get("Authorization")
                .or_else(|| headers.get("authorization"))
                .map(String::as_str),
            Some("Bearer explicit-repair-access")
        );
    }
    reopened.shutdown().await;
}

#[tokio::test]
async fn oauth_pass1_device_slowdown_and_denial_keep_one_authorization_and_original_deadline() {
    let directory = tempfile::tempdir().unwrap();
    let configured = device_configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    device_response(&egress);
    egress.reply(400, serde_json::json!({"error":"slow_down"}));
    egress.reply(400, serde_json::json!({"error":"authorization_pending"}));
    egress.reply(
        400,
        serde_json::json!({"error":"access_denied","error_description":"OAUTH-PASS1-PRIVATE"}),
    );
    let backend = open(
        directory.path(),
        &[configured],
        egress.clone(),
        clock.clone(),
    )
    .await;
    let connection_api::ConnectionResult::ConnectSessionCreate(status) =
        backend.handle_connection(&owner(), create()).await.unwrap()
    else {
        panic!("device create")
    };
    let endpoint = url::Url::parse(status.browser_completion_url.as_deref().unwrap()).unwrap();
    let original_deadline = status.expires_at_unix_ms;
    // Only the supplied receiver clock changes; real timers enforce each declared polling wait.
    for (wait, count) in [(1_010, 2), (6_010, 3), (6_010, 4)] {
        clock.advance(wait);
        tokio::time::timeout(Duration::from_secs(8), async {
            while egress.count() < count {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(egress.count(), count);
        if count < 4 {
            let observed = backend
                .inner
                .session_status(&status.connect_session_ref)
                .await
                .unwrap();
            assert_eq!(observed.expires_at_unix_ms, original_deadline);
        }
    }
    tokio::time::timeout(Duration::from_secs(5), async {
        while Arc::strong_count(&backend.inner) != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let failed = backend
        .inner
        .session_status(&status.connect_session_ref)
        .await
        .unwrap();
    assert_eq!(failed.state, connection_api::ConnectSessionState::Failed);
    assert!(failed.browser_completion_url.is_none());
    assert!(!serde_json::to_string(&failed)
        .unwrap()
        .contains("OAUTH-PASS1-PRIVATE"));
    assert!(
        tokio::net::TcpStream::connect(("127.0.0.1", endpoint.port().unwrap()))
            .await
            .is_err()
    );
    {
        let requests = egress.requests.lock().unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|(_, r)| r.url.ends_with("/oauth/authorize_device"))
                .count(),
            1
        );
        assert_eq!(
            requests
                .iter()
                .filter(|(_, r)| r.url.ends_with("/oauth/token"))
                .count(),
            3
        );
    }
    backend.shutdown().await;
}

#[tokio::test]
async fn oauth_pass2_shutdown_after_token_before_evidence_never_publishes_or_restarts() {
    for device in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let configured = if device {
            device_configuration()
        } else {
            configuration()
        };
        let egress = Arc::new(Egress::default());
        let clock = Arc::new(Clock::new());
        if device {
            device_response(&egress);
        }
        egress.token(
            "OAUTH-PASS2-UNPROVEN-ACCESS",
            Some("OAUTH-PASS2-UNPROVEN-REFRESH"),
            60,
        );
        let expected_requests = if device { 3 } else { 2 };
        *egress.hold_request.lock().unwrap() = Some(expected_requests);
        let backend = open(
            directory.path(),
            std::slice::from_ref(&configured),
            egress.clone(),
            clock.clone(),
        )
        .await;
        let status = if device {
            let connection_api::ConnectionResult::ConnectSessionCreate(status) =
                backend.handle_connection(&owner(), create()).await.unwrap()
            else {
                panic!("device create");
            };
            clock.advance(1_010);
            status
        } else {
            deliver_code(&backend).await
        };
        tokio::time::timeout(Duration::from_secs(5), egress.entered.notified())
            .await
            .unwrap();
        assert_eq!(egress.count(), expected_requests);
        assert!(!egress.dropped.load(Ordering::SeqCst));
        let address = access_address(&backend);
        let identity = backend.inner.bindings[0].custody.identity.clone();
        tokio::time::timeout(Duration::from_secs(5), backend.shutdown())
            .await
            .unwrap();
        assert!(
            egress.dropped.load(Ordering::SeqCst),
            "shutdown drops the actual token-info future"
        );
        assert!(backend.inner.custody.snapshot(&identity).unwrap().is_none());
        assert!(backend
            .inner
            .store
            .get(&address)
            .await
            .unwrap_err()
            .is_not_found());
        assert!(
            !backend.inner.sessions.lock().unwrap()[&status.connect_session_ref]
                .liveness
                .is_live()
        );
        drop(backend);
        clock.advance(600_000);
        let reopened = open(directory.path(), &[configured], egress.clone(), clock).await;
        assert!(reopened.inner.sessions.lock().unwrap().is_empty());
        assert!(reopened
            .inner
            .custody
            .snapshot(&identity)
            .unwrap()
            .is_none());
        assert!(reopened
            .inner
            .store
            .get(&address)
            .await
            .unwrap_err()
            .is_not_found());
        assert!(reopened
            .handle(
                &owner(),
                operation_api::OperationRequest::Invoke(invocation(&reopened).await)
            )
            .await
            .is_err());
        assert_eq!(
            egress.count(),
            expected_requests,
            "no token reuse, operation send or automatic acquisition after reopen"
        );
        reopened.shutdown().await;
    }
}

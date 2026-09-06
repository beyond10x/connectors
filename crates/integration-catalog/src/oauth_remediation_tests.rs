//! Deciding cases use the reviewed OAuth owner, real custody and its existing synthetic egress.
use super::*;
use service::{CredentialReadiness, RemediationError, RemediationRoute, RemediationTarget};

#[tokio::test]
async fn remediation_created_metadata_is_exact_and_does_not_publish_callable_discovery() {
    let directory = tempfile::tempdir().unwrap();
    let egress = Arc::new(Egress::default());
    let backend = open(
        directory.path(),
        &[configuration()],
        egress.clone(),
        Arc::new(Clock::new()),
    )
    .await;
    let connection = backend.inner.bindings[0]
        .custody
        .identity
        .connection
        .clone();
    let target = RemediationTarget {
        operation_ref: "gitlab-project-list",
        connection_ref: &connection,
    };
    assert!(backend.owns_remediation(RemediationRoute::Target(target)));
    let metadata = backend.remediation_metadata(&owner(), target).unwrap();
    assert_eq!(metadata.operation.id(), "gitlab-project-list");
    assert_eq!(metadata.connection.id(), connection);
    assert_eq!(metadata.integration_ref, "gitlab");
    assert_eq!(metadata.auth_profile, "gitlab.oauth_token");
    assert_eq!(
        metadata.catalog_generation,
        catalog::reader::embedded().digest()
    );
    assert_eq!(
        backend.credential_readiness(&owner(), target).await,
        CredentialReadiness::MissingCredential
    );
    assert!(backend.inner.search("", 20).await.unwrap().is_empty());
    assert!(backend.inner.sessions.lock().unwrap().is_empty());
    assert_eq!(egress.count(), 0);
    backend.shutdown().await;
}

#[tokio::test]
async fn remediation_unknown_and_wrong_owner_refuse_without_credential_or_provider_work() {
    let directory = tempfile::tempdir().unwrap();
    let egress = Arc::new(Egress::default());
    let backend = open(
        directory.path(),
        &[configuration()],
        egress.clone(),
        Arc::new(Clock::new()),
    )
    .await;
    let connection = backend.inner.bindings[0]
        .custody
        .identity
        .connection
        .clone();
    for target in [
        RemediationTarget {
            operation_ref: "unknown",
            connection_ref: &connection,
        },
        RemediationTarget {
            operation_ref: "gitlab-project-list",
            connection_ref: "connection:absent",
        },
    ] {
        assert!(matches!(
            backend.remediation_metadata(&owner(), target),
            Err(RemediationError::Refused)
        ));
    }
    let other = PrincipalContext::local(&operation_api::OwnerContext {
        tenant_id: "local".into(),
        agent_id: "other".into(),
        agent_revision: 1,
        authority_snapshot_id: "snapshot:other".into(),
        authority_snapshot_sha256: "b".repeat(64),
    })
    .unwrap();
    let target = RemediationTarget {
        operation_ref: "gitlab-project-list",
        connection_ref: &connection,
    };
    assert!(matches!(
        backend.remediation_metadata(&other, target),
        Err(RemediationError::Refused)
    ));
    assert!(backend.inner.sessions.lock().unwrap().is_empty());
    assert_eq!(egress.count(), 0);
    backend.shutdown().await;
}

#[tokio::test]
async fn remediation_fresh_and_safely_refreshable_credentials_are_ready_without_refresh() {
    let directory = tempfile::tempdir().unwrap();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("fixture-access", Some("fixture-refresh"), 1);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(
        directory.path(),
        &[configuration()],
        egress.clone(),
        clock.clone(),
    )
    .await;
    authorize(&backend).await;
    let connection = backend.inner.bindings[0]
        .custody
        .identity
        .connection
        .clone();
    let target = RemediationTarget {
        operation_ref: "gitlab-project-list",
        connection_ref: &connection,
    };
    assert_eq!(
        backend.credential_readiness(&owner(), target).await,
        CredentialReadiness::Ready
    );
    clock.advance(1_001);
    assert_eq!(
        backend.credential_readiness(&owner(), target).await,
        CredentialReadiness::Ready
    );
    assert_eq!(egress.count(), 2, "readiness never refreshes or dispatches");
    assert!(backend.inner.marker_blocks.lock().unwrap().is_empty());
    backend.shutdown().await;
}

fn binding_from_admission(
    backend: &PersonalOAuthBackend,
    target: RemediationTarget<'_>,
) -> (
    service::RemediationBinding,
    Arc<dyn service::RemediationAuthority>,
) {
    let metadata = backend.remediation_metadata(&owner(), target).unwrap();
    let admission = backend
        .personal_remediation_admission(&owner(), target)
        .unwrap();
    assert_eq!(admission.grant_ref, "grant:personal");
    assert_eq!(admission.grant_revision, None);
    let binding = service::RemediationBinding {
        operation_ref: target.operation_ref.into(),
        connection_ref: target.connection_ref.into(),
        integration_ref: metadata.integration_ref,
        auth_profile: metadata.auth_profile,
        need: protocol::operation::v3::AuthenticationNeed::AuthorizeConfigured,
        canonical_input_sha256: hex::encode(Sha256::digest(b"{}")),
        stable_authority_sha256: hex::encode(Sha256::digest(owner().stable_authority_seed())),
        grant_ref: admission.grant_ref,
        grant_revision: admission.grant_revision,
        admission_policy_sha256: admission.admission_policy_sha256,
        expires_at_unix_ms: admission.expires_at_unix_ms,
    };
    (binding, admission.authority)
}

#[tokio::test]
async fn remediation_personal_factory_binds_actual_policy_and_rechecks_current_authority() {
    let directory = tempfile::tempdir().unwrap();
    let egress = Arc::new(Egress::default());
    let backend = open(
        directory.path(),
        &[configuration()],
        egress.clone(),
        Arc::new(Clock::new()),
    )
    .await;
    let connection = backend.inner.bindings[0]
        .custody
        .identity
        .connection
        .clone();
    let target = RemediationTarget {
        operation_ref: "gitlab-project-list",
        connection_ref: &connection,
    };
    let (binding, authority) = binding_from_admission(&backend, target);
    let now = backend.inner.now().unwrap();
    authority.recheck(&owner(), &binding, now).unwrap();
    for changed in [
        service::RemediationBinding {
            grant_ref: "other".into(),
            ..binding.clone()
        },
        service::RemediationBinding {
            grant_revision: Some(1),
            ..binding.clone()
        },
        service::RemediationBinding {
            connection_ref: "other".into(),
            ..binding.clone()
        },
        service::RemediationBinding {
            admission_policy_sha256: "f".repeat(64),
            ..binding.clone()
        },
    ] {
        assert!(authority.recheck(&owner(), &changed, now).is_err());
    }
    assert!(authority
        .recheck(&owner(), &binding, binding.expires_at_unix_ms)
        .is_err());
    backend.inner.bindings[0].authority.lock().unwrap().active = false;
    assert!(authority.recheck(&owner(), &binding, now).is_err());
    assert_eq!(egress.count(), 0);
    assert!(backend.inner.sessions.lock().unwrap().is_empty());
    backend.shutdown().await;
}

async fn finish_bound(status: &connection_api::ConnectSessionStatus) {
    let url = url::Url::parse(status.browser_completion_url.as_deref().unwrap()).unwrap();
    let authority = format!("127.0.0.1:{}", url.port().unwrap());
    let token = url.fragment().unwrap().strip_prefix("token=").unwrap();
    let instructions = http(
        &authority,
        "/instructions",
        &format!("X-Connect-Session: {token}\r\n"),
    )
    .await;
    let body: serde_json::Value =
        serde_json::from_str(instructions.split_once("\r\n\r\n").unwrap().1).unwrap();
    let authorization = url::Url::parse(body["authorization_url"].as_str().unwrap()).unwrap();
    let state = authorization
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();
    assert!(http(
        &authority,
        &format!("/oauth/callback?state={state}&code=fixture-code"),
        ""
    )
    .await
    .starts_with("HTTP/1.1 200"));
}

#[tokio::test]
async fn remediation_bound_session_publishes_exact_target_and_acknowledges_once_without_dispatch() {
    use protocol::connection_v2::{
        RemediationAcknowledgeRequest, RemediationResumeState, RemediationStatusRequest,
    };
    use service::{RemediationRequest, RemediationResult};
    let directory = tempfile::tempdir().unwrap();
    let egress = Arc::new(Egress::default());
    egress.token("bound-fixture-access", Some("bound-fixture-refresh"), 60);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(
        directory.path(),
        &[configuration()],
        egress.clone(),
        Arc::new(Clock::new()),
    )
    .await;
    let connection = backend.inner.bindings[0]
        .custody
        .identity
        .connection
        .clone();
    let target = RemediationTarget {
        operation_ref: "gitlab-project-list",
        connection_ref: &connection,
    };
    let (binding, authority) = binding_from_admission(&backend, target);
    let RemediationResult::Status(start) = backend
        .handle_remediation(
            &owner(),
            RemediationRequest::Start(Box::new(binding)),
            authority.clone(),
        )
        .await
        .unwrap()
    else {
        panic!("bound status")
    };
    assert_eq!(start.resume_state, RemediationResumeState::Pending);
    assert_eq!(start.connection_ref, connection);
    assert_eq!(start.integration_ref, "gitlab");
    assert_eq!(start.auth_profile, "gitlab.oauth_token");
    assert_eq!(egress.count(), 0);
    finish_bound(&start.session).await;
    join_acquisition(&backend, &start.connect_session_ref).await;
    let status_request = RemediationStatusRequest {
        connect_session_ref: start.connect_session_ref.clone(),
    };
    let RemediationResult::Status(ready) = backend
        .handle_remediation(
            &owner(),
            RemediationRequest::Status(status_request.clone()),
            authority.clone(),
        )
        .await
        .unwrap()
    else {
        panic!("bound status")
    };
    assert_eq!(ready.resume_state, RemediationResumeState::Ready);
    assert_eq!(
        ready.session.connection_ref.as_deref(),
        Some(connection.as_str())
    );
    assert!(ready.session.completion_endpoint.is_none());
    assert!(ready.session.browser_completion_url.is_none());
    let ack = RemediationAcknowledgeRequest {
        connect_session_ref: start.connect_session_ref.clone(),
        operation_ref: target.operation_ref.into(),
        connection_ref: connection.clone(),
    };
    let context = owner();
    let (first, second) = tokio::join!(
        backend.handle_remediation(
            &context,
            RemediationRequest::Acknowledge(ack.clone()),
            authority.clone()
        ),
        backend.handle_remediation(
            &context,
            RemediationRequest::Acknowledge(ack),
            authority.clone()
        )
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let RemediationResult::Status(consumed) = backend
        .handle_remediation(
            &owner(),
            RemediationRequest::Status(status_request),
            authority,
        )
        .await
        .unwrap()
    else {
        panic!("bound status")
    };
    assert_eq!(consumed.resume_state, RemediationResumeState::Consumed);
    assert_eq!(
        egress.count(),
        2,
        "only token and evidence requests, never an operation"
    );
    backend.shutdown().await;
}

#[tokio::test]
async fn remediation_bound_publication_rechecks_its_receiver_authority_before_custody_commit() {
    struct Revocable {
        current: Arc<dyn service::RemediationAuthority>,
        active: Arc<AtomicBool>,
    }
    impl service::RemediationAuthority for Revocable {
        fn recheck(
            &self,
            context: &PrincipalContext,
            binding: &service::RemediationBinding,
            now: u64,
        ) -> std::result::Result<(), RemediationError> {
            if !self.active.load(Ordering::SeqCst) {
                return Err(RemediationError::Refused);
            }
            self.current.recheck(context, binding, now)
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let egress = Arc::new(Egress::default());
    egress.token("bound-revoked-access", Some("bound-revoked-refresh"), 60);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(
        directory.path(),
        &[configuration()],
        egress.clone(),
        Arc::new(Clock::new()),
    )
    .await;
    let connection = backend.inner.bindings[0]
        .custody
        .identity
        .connection
        .clone();
    let (binding, current) = binding_from_admission(
        &backend,
        RemediationTarget {
            operation_ref: "gitlab-project-list",
            connection_ref: &connection,
        },
    );
    let active = Arc::new(AtomicBool::new(true));
    let authority = Arc::new(Revocable {
        current,
        active: active.clone(),
    });
    let service::RemediationResult::Status(start) = backend
        .handle_remediation(
            &owner(),
            service::RemediationRequest::Start(Box::new(binding)),
            authority.clone(),
        )
        .await
        .unwrap()
    else {
        panic!("bound status")
    };
    egress.after(2, move || {
        active.store(false, Ordering::SeqCst);
    });
    finish_bound(&start.session).await;
    join_acquisition(&backend, &start.connect_session_ref).await;
    // Revoke only the receiver capability; the ordinary OAuth policy is still admitted.
    assert!(backend.inner.bindings[0].authority.lock().unwrap().active);
    let terminal = backend
        .inner
        .session_status(&start.connect_session_ref)
        .await
        .unwrap();
    assert_ne!(
        terminal.state,
        connection_api::ConnectSessionState::Completed
    );
    assert!(terminal.completion_endpoint.is_none());
    assert!(terminal.browser_completion_url.is_none());
    assert!(backend
        .inner
        .store
        .get(&access_address(&backend))
        .await
        .unwrap_err()
        .is_not_found());
    assert_eq!(egress.count(), 2);
    backend.shutdown().await;
}

#[tokio::test]
async fn remediation_expired_acknowledgement_refuses_without_rolling_back_published_credentials() {
    let directory = tempfile::tempdir().unwrap();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("bound-expired-access", Some("bound-expired-refresh"), 60);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(
        directory.path(),
        &[configuration()],
        egress.clone(),
        clock.clone(),
    )
    .await;
    let connection = backend.inner.bindings[0]
        .custody
        .identity
        .connection
        .clone();
    let (binding, authority) = binding_from_admission(
        &backend,
        RemediationTarget {
            operation_ref: "gitlab-project-list",
            connection_ref: &connection,
        },
    );
    let deadline = binding.expires_at_unix_ms;
    let service::RemediationResult::Status(start) = backend
        .handle_remediation(
            &owner(),
            service::RemediationRequest::Start(Box::new(binding)),
            authority.clone(),
        )
        .await
        .unwrap()
    else {
        panic!("bound status")
    };
    finish_bound(&start.session).await;
    join_acquisition(&backend, &start.connect_session_ref).await;
    assert_eq!(
        backend
            .inner
            .session_status(&start.connect_session_ref)
            .await
            .unwrap()
            .state,
        connection_api::ConnectSessionState::Completed
    );
    let identity = &backend.inner.bindings[0].custody.identity;
    let before = backend.inner.custody.snapshot(identity).unwrap().unwrap();
    clock.advance(deadline - backend.inner.now().unwrap());
    let acknowledgement = backend
        .handle_remediation(
            &owner(),
            service::RemediationRequest::Acknowledge(
                protocol::connection_v2::RemediationAcknowledgeRequest {
                    connect_session_ref: start.connect_session_ref.clone(),
                    operation_ref: "gitlab-project-list".into(),
                    connection_ref: connection,
                },
            ),
            authority.clone(),
        )
        .await;
    assert!(acknowledgement.is_err());
    let after = backend.inner.custody.snapshot(identity).unwrap().unwrap();
    assert!(
        before == after,
        "acknowledgement expiry never rolls back a published credential"
    );
    assert!(backend
        .inner
        .store
        .exists(&access_address(&backend))
        .await
        .unwrap());
    assert_eq!(egress.count(), 2);
    assert!(matches!(acknowledgement, Err(RemediationError::Conflict)));
    let status =
        service::RemediationRequest::Status(protocol::connection_v2::RemediationStatusRequest {
            connect_session_ref: start.connect_session_ref.clone(),
        });
    let service::RemediationResult::Status(expired) = backend
        .handle_remediation(&owner(), status, authority.clone())
        .await
        .unwrap()
    else {
        panic!("expired status")
    };
    assert_eq!(
        expired.resume_state,
        protocol::connection_v2::RemediationResumeState::Expired
    );
    assert_eq!(
        expired.session_state,
        connection_api::ConnectSessionState::Completed
    );
    expired.validate().unwrap();
    backend.inner.bindings[0].authority.lock().unwrap().active = false;
    assert!(matches!(
        backend
            .handle_remediation(
                &owner(),
                service::RemediationRequest::Status(
                    protocol::connection_v2::RemediationStatusRequest {
                        connect_session_ref: start.connect_session_ref
                    }
                ),
                authority
            )
            .await,
        Err(RemediationError::Refused)
    ));
    backend.shutdown().await;
}

#[tokio::test]
async fn remediation_expired_status_never_swallows_an_opaque_receiver_rejection() {
    struct Custom {
        current: Arc<dyn service::RemediationAuthority>,
        reject: Arc<AtomicBool>,
        error: RemediationError,
    }
    impl service::RemediationAuthority for Custom {
        fn recheck(
            &self,
            context: &PrincipalContext,
            binding: &service::RemediationBinding,
            now: u64,
        ) -> std::result::Result<(), RemediationError> {
            if self.reject.load(Ordering::SeqCst) {
                return Err(self.error);
            }
            self.current.recheck(context, binding, now)
        }
    }
    for refusal in [RemediationError::Conflict, RemediationError::Refused] {
        let directory = tempfile::tempdir().unwrap();
        let egress = Arc::new(Egress::default());
        let clock = Arc::new(Clock::new());
        let backend = open(
            directory.path(),
            &[configuration()],
            egress.clone(),
            clock.clone(),
        )
        .await;
        let connection = backend.inner.bindings[0]
            .custody
            .identity
            .connection
            .clone();
        let (binding, current) = binding_from_admission(
            &backend,
            RemediationTarget {
                operation_ref: "gitlab-project-list",
                connection_ref: &connection,
            },
        );
        let deadline = binding.expires_at_unix_ms;
        let reject = Arc::new(AtomicBool::new(false));
        let authority = Arc::new(Custom {
            current,
            reject: reject.clone(),
            error: refusal,
        });
        let service::RemediationResult::Status(start) = backend
            .handle_remediation(
                &owner(),
                service::RemediationRequest::Start(Box::new(binding)),
                authority.clone(),
            )
            .await
            .unwrap()
        else {
            panic!("pending status")
        };
        reject.store(true, Ordering::SeqCst);
        clock.advance(deadline - backend.inner.now().unwrap());
        let result = backend
            .handle_remediation(
                &owner(),
                service::RemediationRequest::Status(
                    protocol::connection_v2::RemediationStatusRequest {
                        connect_session_ref: start.connect_session_ref,
                    },
                ),
                authority,
            )
            .await;
        assert!(matches!(result, Err(error) if error == refusal));
        assert_eq!(egress.count(), 0);
        backend.shutdown().await;
    }
}

#[tokio::test]
async fn auth_adversary_owner_readiness_tracks_deleted_credentials_and_revoked_authority() {
    let mut observed = Vec::new();
    for case in ["access-missing", "refresh-missing", "authority-revoked"] {
        let directory = tempfile::tempdir().unwrap();
        let egress = Arc::new(Egress::default());
        let clock = Arc::new(Clock::new());
        egress.token(
            "fixture-adversary-access",
            Some("fixture-adversary-refresh"),
            1,
        );
        egress.info("fixture-client", 42, &["read_api"]);
        let backend = open(
            directory.path(),
            &[configuration()],
            egress.clone(),
            clock.clone(),
        )
        .await;
        authorize(&backend).await;
        let connection = backend.inner.bindings[0]
            .custody
            .identity
            .connection
            .clone();
        let target = RemediationTarget {
            operation_ref: "gitlab-project-list",
            connection_ref: &connection,
        };
        assert_eq!(
            backend.credential_readiness(&owner(), target).await,
            CredentialReadiness::Ready
        );
        match case {
            "access-missing" => backend
                .inner
                .store
                .delete(&access_address(&backend))
                .await
                .unwrap(),
            "refresh-missing" => {
                backend
                    .inner
                    .store
                    .delete(&backend.inner.bindings[0].refresh_address)
                    .await
                    .unwrap();
                clock.advance(1_001);
            }
            "authority-revoked" => {
                backend.inner.bindings[0].authority.lock().unwrap().active = false
            }
            _ => unreachable!(),
        }
        let readiness = backend.credential_readiness(&owner(), target).await;
        observed.push((case, readiness));
        let count = egress.count();
        backend.shutdown().await;
        assert_eq!(
            count, 2,
            "readiness must not refresh, start acquisition or invoke"
        );
    }
    eprintln!("actual personal owner readiness after custody/authority changes: {observed:?}");
    assert_eq!(
        observed,
        [
            ("access-missing", CredentialReadiness::CredentialDegraded),
            ("refresh-missing", CredentialReadiness::CredentialDegraded),
            ("authority-revoked", CredentialReadiness::Unsupported),
        ]
    );
}

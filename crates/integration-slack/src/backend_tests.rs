#[cfg(test)]
mod tests {
    use connector_secrets::{MemoryStore, SecretStore as _};
    use protocol::connection::ConnectSessionState;
    use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
    use tokio::net::UnixStream;

    use super::*;

    const SENTINEL: &str = "SENTINEL-NOT-A-REAL-SECRET";

    struct UnavailableStore;

    #[async_trait]
    impl connector_secrets::SecretStore for UnavailableStore {
        async fn ready(&self) -> Result<(), connector_secrets::StoreError> {
            Err(connector_secrets::StoreError::Unreachable {
                path: "test:secret-store".to_owned(),
                reason: SENTINEL.to_owned(),
            })
        }

        async fn get(
            &self,
            _reference: &connector_secrets::CredentialRef,
        ) -> Result<connector_secrets::Secret, connector_secrets::StoreError> {
            unreachable!("readiness must not retrieve a credential")
        }

        async fn put(
            &self,
            _reference: &connector_secrets::CredentialRef,
            _secret: &connector_secrets::Secret,
        ) -> Result<(), connector_secrets::StoreError> {
            unreachable!("readiness must not write a credential")
        }

        async fn delete(
            &self,
            _reference: &connector_secrets::CredentialRef,
        ) -> Result<(), connector_secrets::StoreError> {
            unreachable!("readiness must not delete a credential")
        }
    }

    impl connector_secrets::PreparedSecretStore for UnavailableStore {}

    #[test]
    fn only_the_inner_admitted_event_is_projected() {
        let payload = serde_json::json!({
            "token": SENTINEL,
            "event_id": "Ev01",
            "event": {
                "type": "message",
                "channel_type": "channel",
                "channel": "C01",
                "user": "U01",
                "text": "hello",
                "ts": "1.0"
            }
        });
        let (_, kind, projected) =
            project_data_event(Some(&payload), &["message.channels".to_owned()]).unwrap();
        assert_eq!(kind, "message.channels");
        let encoded = serde_json::to_string(&projected).unwrap();
        assert!(!encoded.contains(SENTINEL));
        assert!(projected.get("event").is_none());
        assert_eq!(projected["text"], "hello");
    }

    #[test]
    fn message_loop_guards_and_closed_event_grants_are_applied_before_storage() {
        let bot = serde_json::json!({
            "event_id": "Ev02",
            "event": {"type": "message", "channel_type": "channel", "bot_id": "B01", "text": "own"}
        });
        assert!(project_data_event(Some(&bot), &["message.channels".to_owned()]).is_none());
        let unknown = serde_json::json!({
            "event_id": "Ev03",
            "event": {"type": "reaction_added"}
        });
        assert!(project_data_event(Some(&unknown), &["message.channels".to_owned()]).is_none());
    }

    #[test]
    fn socket_ticket_destination_is_closed_to_slack_tls_hosts() {
        assert!(validate_socket_url("wss://wss-primary.slack.com/link/?ticket=sentinel").is_ok());
        assert!(
            validate_socket_url("wss://slack.com.example.invalid/link/?ticket=sentinel").is_err()
        );
        assert!(validate_socket_url("ws://wss-primary.slack.com/link/?ticket=sentinel").is_err());
    }

    #[test]
    fn datasource_projection_excludes_unreviewed_slack_profile_fields() {
        let user = serde_json::json!({
            "id": "U012345",
            "name": "ada",
            "real_name": "Ada Lovelace",
            "is_bot": false,
            "deleted": false,
            "profile": {
                "display_name": "Ada",
                "email": format!("{SENTINEL}@example.test"),
                "phone": SENTINEL
            }
        });
        let record = normalize_user(&user, DatasourceRecordView::Detail).unwrap();
        let encoded = serde_json::to_string(&record).unwrap();
        assert!(!encoded.contains(SENTINEL));
        assert_eq!(record.value["display_name"], "Ada");
        assert_eq!(record.value["real_name"], "Ada Lovelace");

        let (_, params, _, _) = datasource_request_plan(
            "slack.conversations",
            SlackConnectionProfile::OrgBot,
            &DatasourceRead::List {
                limit: 10,
                cursor: None,
            },
        )
        .unwrap();
        assert_eq!(
            params.iter().find(|(name, _)| name == "types").unwrap().1,
            "public_channel"
        );
    }

    #[test]
    fn hosted_companion_completion_requires_distinct_app_and_bot_credentials() {
        let submission = format!("xapp-{SENTINEL}\nxoxb-{SENTINEL}");
        let parsed = parse_hosted_submission(submission.as_bytes()).unwrap();
        assert_eq!(
            parsed.app_token.unwrap().expose_secret(),
            format!("xapp-{SENTINEL}")
        );
        assert_eq!(
            parsed.bot_token.unwrap().expose_secret(),
            format!("xoxb-{SENTINEL}")
        );
        assert!(parsed.user_token.is_none());
        assert!(parse_hosted_submission(format!("xapp-{SENTINEL}").as_bytes()).is_err());
        assert!(parse_hosted_submission(
            format!("xapp-{SENTINEL}\nxoxp-{SENTINEL}").as_bytes()
        )
        .is_err());
        let trimmed = parse_hosted_submission(
            format!("  xapp-{SENTINEL} \r\n xoxb-{SENTINEL}\t ").as_bytes(),
        )
        .unwrap();
        assert_eq!(
            trimmed.app_token.unwrap().expose_secret(),
            format!("xapp-{SENTINEL}")
        );
        assert!(parse_hosted_submission(b"xapp-\nxoxb-value").is_err());
    }

    #[test]
    fn hosted_setup_page_requires_capability_and_distinguishes_safe_failures() {
        let page = hosted_setup::HOSTED_COMPANION_SETUP_PAGE;
        assert!(page.contains("validCapability"));
        assert!(page.contains("field.value.trim()"));
        assert!(page.contains("validToken(values[0],'xapp-')"));
        assert!(page.contains("validToken(values[1],'xoxb-')"));
        assert!(!page.contains("xoxp-"));
        assert!(page.contains("response.status===400"));
        assert!(page.contains("response.status===403"));
        assert!(page.contains("response.status===503"));
        assert!(page.contains("Slack or the Secret Store is unavailable"));
        assert!(!page.contains(SENTINEL));
    }

    #[test]
    fn hosted_completion_errors_separate_conflicts_from_store_outages() {
        for code in [
            "credential-shape",
            "credential-verify-refused",
            "credential-workspace",
            "connection-conflict",
            "app-token-conflict",
        ] {
            assert_eq!(
                hosted_completion_error(SlackError::new(code)),
                HostedCompletionError::Refused
            );
        }
        for code in [
            "credential-resolve",
            "credential-verify-unavailable",
            "credential-prepare",
            "credential-commit",
        ] {
            assert_eq!(
                hosted_completion_error(SlackError::new(code)),
                HostedCompletionError::Unavailable
            );
        }
    }

    #[test]
    fn slack_auth_test_refuses_only_explicit_invalid_credentials() {
        let explicit = br#"{"ok":false,"error":"invalid_auth"}"#;
        let error = classify_auth_test_response(reqwest::StatusCode::OK, None, explicit)
            .unwrap_err();
        assert_eq!(error.code, "credential-verify-refused");
        assert_eq!(
            hosted_completion_error(error),
            HostedCompletionError::Refused
        );

        let valid = classify_auth_test_response(
            reqwest::StatusCode::OK,
            None,
            br#"{"ok":true,"team_id":"T012345","user_id":"U012345","bot_id":"B012345"}"#,
        )
        .unwrap();
        assert_eq!(valid.team_id, "T012345");
        assert_eq!(valid.subject_id, "U012345");
        assert!(valid.is_bot);
    }

    #[test]
    fn slack_auth_test_provider_and_transport_failures_are_unavailable() {
        for (status, body) in [
            (reqwest::StatusCode::TOO_MANY_REQUESTS, b"rate limited".as_slice()),
            (reqwest::StatusCode::INTERNAL_SERVER_ERROR, b"failure".as_slice()),
            (reqwest::StatusCode::OK, b"not-json".as_slice()),
            (
                reqwest::StatusCode::OK,
                br#"{"ok":false,"error":"provider_changed"}"#.as_slice(),
            ),
            (reqwest::StatusCode::OK, br#"{"ok":true}"#.as_slice()),
        ] {
            let error = classify_auth_test_response(status, None, body).unwrap_err();
            assert_eq!(error.code, "credential-verify-unavailable");
            assert_eq!(
                hosted_completion_error(error),
                HostedCompletionError::Unavailable
            );
        }

        let oversized = vec![b'x'; MAX_AUTH_TEST_RESPONSE_BYTES + 1];
        assert_eq!(
            classify_auth_test_response(reqwest::StatusCode::OK, None, &oversized)
                .unwrap_err()
                .code,
            "credential-verify-unavailable"
        );
        assert_eq!(
            classify_auth_test_response(
                reqwest::StatusCode::OK,
                Some(MAX_AUTH_TEST_RESPONSE_BYTES as u64 + 1),
                br#"{"ok":true,"team_id":"T012345","user_id":"U012345"}"#,
            )
            .unwrap_err()
            .code,
            "credential-verify-unavailable"
        );
        assert_eq!(
            hosted_completion_error(SlackError::new("credential-verify-unavailable")),
            HostedCompletionError::Unavailable
        );
    }

    #[test]
    fn operation_audit_is_durable_bounded_and_value_free() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let journal = AuditJournal::new(root.path().join("slack-operation-audit.jsonl"), None);
        let event = AuditEvent {
            audit_ref: "audit:slack:test",
            operation_ref: "slack-chat-post-message",
            connection_ref: "connection:slack:test",
            tenant_id: "tenant-test",
            actor_subject: "subject-test",
            outcome: "attempted",
        };
        journal.begin(event).unwrap();
        journal
            .finish(AuditEvent {
                outcome: "completed",
                ..event
            })
            .unwrap();
        let stored = fs::read_to_string(root.path().join("slack-operation-audit.jsonl")).unwrap();
        assert_eq!(stored.lines().count(), 2);
        assert!(stored.contains("attempted"));
        assert!(stored.contains("completed"));
        assert!(!stored.contains(SENTINEL));
    }

    fn owner() -> PrincipalContext {
        PrincipalContext::local(&protocol::operation::OwnerContext {
            tenant_id: "tenant-local".to_owned(),
            agent_id: "agent-dev".to_owned(),
            agent_revision: 1,
            authority_snapshot_id: "authority-1".to_owned(),
            authority_snapshot_sha256: "a".repeat(64),
        })
        .unwrap()
    }

    fn policy() -> SlackIntegrationConfig {
        SlackIntegrationConfig {
            grant_ref: "grant:slack-inbound".to_owned(),
            org_read_grant_ref: None,
            user_grant_ref: None,
            companion_grant_ref: None,
            expected_team_id: None,
            oauth_client_id: None,
            oauth_redirect_uri: None,
            initiation: InitiationConfig::Provider,
            allowed_events: vec!["app_mention".to_owned(), "message.channels".to_owned()],
            connect_session_ttl_seconds: 30,
            instances: Vec::new(),
        }
    }

    #[tokio::test]
    async fn organization_bot_is_admitted_for_reads_without_an_event_channel() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = SlackBackend::open_with_supervision(
            owner(),
            policy(),
            root.path(),
            Arc::new(MemoryStore::new()),
            false,
        )
        .await
        .unwrap();
        let connection = StoredConnection {
            connection_ref: "connection:slack:org-bot".to_owned(),
            instance_id: "org-bot".to_owned(),
            label: "Organization Slack bot".to_owned(),
            grant_ref: policy().grant_for_profile(PROFILE_ORG_BOT).to_owned(),
            initiation: InitiationConfig::Provider,
            allowed_events: Vec::new(),
            owner_subject: String::new(),
            team_id: "T012345".to_owned(),
            profile: SlackConnectionProfile::OrgBot,
            external_subject_id: "U012345".to_owned(),
            scopes: vec!["channels:history".to_owned(), "users:read".to_owned()],
            purpose: String::new(),
            carries_operations: true,
        };

        assert!(backend.inner.connection_is_admitted(&connection));
        assert!(backend.inner.connection_owned_by(&connection, &owner()));
        assert!(connection.allowed_events.is_empty());
        backend.shutdown().await;
    }

    #[tokio::test]
    async fn standalone_adapter_claims_only_its_connection_and_event_families() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = SlackBackend::open_with_supervision(
            owner(),
            policy(),
            root.path(),
            Arc::new(MemoryStore::new()),
            false,
        )
        .await
        .unwrap();

        assert_eq!(
            backend.capabilities(),
            BackendCapabilities {
                operations: true,
                connections: true,
                events: true,
                datasources: true,
            }
        );
        let operation = OperationRequest::Search(protocol::operation::SearchRequest {
            query: String::new(),
            limit: 10,
        });
        assert!(!backend.owns_operation(&operation));
        assert!(matches!(
            backend.handle(&owner(), operation).await.unwrap(),
            OperationResult::Search { operations } if operations.is_empty()
        ));

        let candidate_search = ConnectionRequest::CandidateSearch(
            protocol::connection::CandidateSearchRequest {
                integration_ref: INTEGRATION_REF.to_owned(),
                query: String::new(),
                limit: 10,
            },
        );
        assert!(!backend.owns_connection(&candidate_search));
        assert_eq!(
            backend
                .handle_connection(&owner(), candidate_search)
                .await
                .unwrap_err()
                .code,
            ConnectionErrorCode::NotFound
        );
        backend.shutdown().await;
    }

    #[tokio::test]
    async fn one_use_completion_publishes_only_value_free_connection_state() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let credential_store = Arc::new(MemoryStore::new());
        let backend = SlackBackend::open_with_supervision(
            owner(),
            policy(),
            root.path(),
            credential_store.clone(),
            false,
        )
        .await
        .unwrap();
        let created = backend
            .handle_connection(
                &owner(),
                ConnectionRequest::ConnectSessionCreate(
                    protocol::connection::ConnectSessionCreateRequest {
                        integration_ref: INTEGRATION_REF.to_owned(),
                        label: "Development Slack".to_owned(),
                        auth_profile: None,
                    },
                ),
            )
            .await
            .unwrap();
        let ConnectionResult::ConnectSessionCreate(created) = created else {
            panic!("wrong result");
        };
        assert!(
            created
                .browser_completion_url
                .as_deref()
                .is_some_and(|url| url.starts_with("http://127.0.0.1:"))
        );
        let endpoint = PathBuf::from(created.completion_endpoint.clone().unwrap());
        let submitted = format!("xapp-{SENTINEL}");
        let mut stream = UnixStream::connect(&endpoint).await.unwrap();
        stream.write_all(submitted.as_bytes()).await.unwrap();
        stream.write_all(b"\n").await.unwrap();
        let mut response = String::new();
        BufReader::new(&mut stream)
            .read_line(&mut response)
            .await
            .unwrap();
        assert_eq!(response, "{\"accepted\":true}\n");
        assert!(!endpoint.exists());
        assert!(UnixStream::connect(&endpoint).await.is_err());

        let status = backend
            .handle_connection(
                &owner(),
                ConnectionRequest::ConnectSessionStatus(
                    protocol::connection::ConnectSessionStatusRequest {
                        connect_session_ref: created.connect_session_ref,
                    },
                ),
            )
            .await
            .unwrap();
        let ConnectionResult::ConnectSessionStatus(status) = status else {
            panic!("wrong result");
        };
        assert_eq!(status.state, ConnectSessionState::Completed);
        assert!(status.completion_endpoint.is_none());
        assert!(status.browser_completion_url.is_none());
        let connection_ref = status.connection_ref.unwrap();
        let description = backend
            .handle_connection(
                &owner(),
                ConnectionRequest::Describe(protocol::connection::DescribeRequest {
                    connection_ref,
                }),
            )
            .await
            .unwrap();
        let ConnectionResult::Describe(description) = description else {
            panic!("wrong result");
        };
        assert_eq!(description.summary.state, ConnectionState::Authorized);
        assert_eq!(description.channels[0].state, ChannelState::Starting);

        let metadata = fs::read_to_string(root.path().join("connections.json")).unwrap();
        assert!(!metadata.contains(SENTINEL));
        assert!(!metadata.contains("completion_endpoint"));
        let credential = credential_store
            .get(&backend.inner.app_credential_ref().unwrap())
            .await
            .unwrap();
        assert_eq!(credential.expose_secret(), submitted);
        backend.shutdown().await;
    }

    #[tokio::test]
    async fn hosted_sessions_expire_and_release_pending_capacity_without_submission() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = SlackBackend::open_inner(SlackOpenContext {
            admission: PrincipalAdmission::Tenant("tenant-local".to_owned()),
            completion_mode: CompletionMode::Hosted {
                public_origin: url::Url::parse("https://connectors.example.test").unwrap(),
            },
            policy: policy(),
            state_root: root.path(),
            credential_store: Arc::new(MemoryStore::new()),
            egress: test_egress(),
            hosted_state: None,
            supervision_enabled: false,
        })
        .await
        .unwrap();
        let created = backend
            .inner
            .create_session(
                &owner(),
                "Development Slack".to_owned(),
                SlackConnectionProfile::CompanionBot,
            )
            .await
            .unwrap();
        lock(&backend.inner.hosted_sessions)
            .get_mut(&created.connect_session_ref)
            .unwrap()
            .expires_at_unix_ms = 1;

        let status = backend
            .inner
            .session_status(&created.connect_session_ref)
            .unwrap();
        assert_eq!(status.state, ConnectSessionState::Expired);
        assert!(status.browser_completion_url.is_none());
        assert!(!lock(&backend.inner.hosted_sessions).contains_key(&created.connect_session_ref));
        assert!(!lock(&backend.inner.session_owners).contains_key(&created.connect_session_ref));
        assert!(matches!(
            backend.hosted_completion_page(&created.connect_session_ref),
            Err(HostedCompletionError::NotFound)
        ));
        backend.shutdown().await;
    }

    #[tokio::test]
    async fn invalid_hosted_capability_cannot_consume_a_connect_session() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = SlackBackend::open_inner(SlackOpenContext {
            admission: PrincipalAdmission::Tenant("tenant-local".to_owned()),
            completion_mode: CompletionMode::Hosted {
                public_origin: url::Url::parse("https://connectors.example.test").unwrap(),
            },
            policy: policy(),
            state_root: root.path(),
            credential_store: Arc::new(MemoryStore::new()),
            egress: test_egress(),
            hosted_state: None,
            supervision_enabled: false,
        })
        .await
        .unwrap();
        let created = backend
            .inner
            .create_session(
                &owner(),
                "Development Slack".to_owned(),
                SlackConnectionProfile::CompanionBot,
            )
            .await
            .unwrap();
        assert_eq!(
            backend
                .complete_hosted_session(
                    &created.connect_session_ref,
                    &"0".repeat(64),
                    HostedCompletionSubmission::new(
                        format!("xapp-{SENTINEL}\nxoxb-{SENTINEL}").into_bytes(),
                    ),
                )
                .await,
            Err(HostedCompletionError::Refused)
        );
        assert!(backend.owns_hosted_completion(&created.connect_session_ref));
        let page = backend
            .hosted_completion_page(&created.connect_session_ref)
            .unwrap();
        assert_eq!(page.title, "Connect Slack");
        backend.shutdown().await;
    }

    #[tokio::test]
    async fn slack_readiness_is_value_free_and_tracks_the_secret_store() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = SlackBackend::open_inner(SlackOpenContext {
            admission: PrincipalAdmission::Tenant("tenant-local".to_owned()),
            completion_mode: CompletionMode::Hosted {
                public_origin: url::Url::parse("https://connectors.example.test").unwrap(),
            },
            policy: policy(),
            state_root: root.path(),
            credential_store: Arc::new(UnavailableStore),
            egress: test_egress(),
            hosted_state: None,
            supervision_enabled: false,
        })
        .await
        .unwrap();
        let error = backend.ready().await.unwrap_err();
        assert!(!error.to_string().contains(SENTINEL));
        backend.shutdown().await;
    }

    #[tokio::test]
    async fn event_is_durable_and_deduplicated_before_pull_and_replay() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let store = EventStore::open(root.path().join("events.jsonl"), None).unwrap();
        let connection = StoredConnection {
            connection_ref: "connection:slack:00000000-0000-4000-8000-000000000001".to_owned(),
            instance_id: "00000000-0000-4000-8000-000000000001".to_owned(),
            label: "Development Slack".to_owned(),
            grant_ref: "grant:slack-inbound".to_owned(),
            initiation: InitiationConfig::Provider,
            allowed_events: vec!["message.channels".to_owned()],
            owner_subject: owner().subject().to_owned(),
            team_id: String::new(),
            profile: SlackConnectionProfile::Legacy,
            external_subject_id: String::new(),
            scopes: Vec::new(),
            purpose: String::new(),
            carries_operations: true,
        };
        let payload = serde_json::json!({"type":"message","channel":"C01","text":"hello"});
        store
            .append(&connection, "Ev01", "message.channels", payload.clone())
            .unwrap();
        store
            .append(&connection, "Ev01", "message.channels", payload)
            .unwrap();
        let (events, cursor) = store
            .receive(&channel_ref(&connection), 0, 10, Duration::ZERO)
            .await;
        assert_eq!(events.len(), 1);
        assert_eq!(cursor, 1);
        assert_eq!(events[0].provenance, EventProvenance::Native);
        assert_eq!(store.replay(&events[0].event_ref), Some(events[0].clone()));

        let reopened = EventStore::open(root.path().join("events.jsonl"), None).unwrap();
        let (events, cursor) = reopened
            .receive(&channel_ref(&connection), 0, 10, Duration::ZERO)
            .await;
        assert_eq!(events.len(), 1);
        assert_eq!(cursor, 1);
    }

    #[tokio::test]
    async fn companion_mention_authorizes_one_pinned_thread_reply() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = SlackBackend::open_with_supervision(
            owner(),
            policy(),
            root.path(),
            Arc::new(MemoryStore::new()),
            false,
        )
        .await
        .unwrap();
        let connection = StoredConnection {
            connection_ref: "connection:slack:00000000-0000-4000-8000-000000000003".to_owned(),
            instance_id: "00000000-0000-4000-8000-000000000003".to_owned(),
            label: "Companion Slack".to_owned(),
            grant_ref: "grant:slack-inbound".to_owned(),
            initiation: InitiationConfig::Provider,
            allowed_events: vec!["app_mention".to_owned()],
            owner_subject: owner().subject().to_owned(),
            team_id: "T012345".to_owned(),
            profile: SlackConnectionProfile::CompanionBot,
            external_subject_id: "U012345".to_owned(),
            scopes: vec!["app_mentions:read".to_owned(), "chat:write".to_owned()],
            purpose: String::new(),
            carries_operations: true,
        };
        backend
            .inner
            .event_store
            .append(
                &connection,
                "Ev-companion",
                "app_mention",
                serde_json::json!({
                    "type": "app_mention",
                    "channel": "C012345",
                    "ts": "1723900000.123456",
                    "text": "<@U012345> hello"
                }),
            )
            .unwrap();
        let (events, _) = backend
            .inner
            .event_store
            .receive(&channel_ref(&connection), 0, 1, Duration::ZERO)
            .await;
        let event_ref = events[0].event_ref.clone();
        let mut input = serde_json::json!({
            "channel": "C999999",
            "thread_ts": "1.0",
            "text": "Hello from the companion"
        });
        backend
            .inner
            .authorize_companion_event_reply(
                &connection,
                "slack-chat-post-message",
                &event_ref,
                &mut input,
            )
            .unwrap();
        assert_eq!(input["channel"], "C012345");
        assert_eq!(input["thread_ts"], "1723900000.123456");

        backend
            .inner
            .reply_claims
            .claim(&event_ref, now_ms().unwrap())
            .unwrap();
        assert_eq!(
            backend
                .inner
                .reply_claims
                .claim(&event_ref, now_ms().unwrap())
                .unwrap_err()
                .code,
            "reply-already-claimed"
        );
        let reopened = ReplyClaimStore::open(
            root.path().join("slack-reply-claims.jsonl"),
            None,
        )
        .unwrap();
        assert_eq!(
            reopened.claim(&event_ref, now_ms().unwrap()).unwrap_err().code,
            "reply-already-claimed"
        );
        backend.shutdown().await;
    }

    #[tokio::test]
    async fn stale_grant_metadata_cannot_reenter_any_connection_or_event_surface() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = SlackBackend::open_with_supervision(
            owner(),
            policy(),
            root.path(),
            Arc::new(MemoryStore::new()),
            false,
        )
        .await
        .unwrap();
        let connection = StoredConnection {
            connection_ref: "connection:slack:00000000-0000-4000-8000-000000000002".to_owned(),
            instance_id: "00000000-0000-4000-8000-000000000002".to_owned(),
            label: "Stale Slack".to_owned(),
            grant_ref: "grant:replaced".to_owned(),
            initiation: InitiationConfig::Provider,
            allowed_events: vec!["app_mention".to_owned(), "message.channels".to_owned()],
            owner_subject: owner().subject().to_owned(),
            team_id: String::new(),
            profile: SlackConnectionProfile::Legacy,
            external_subject_id: String::new(),
            scopes: Vec::new(),
            purpose: String::new(),
            carries_operations: true,
        };
        lock(&backend.inner.metadata)
            .connections
            .push(connection.clone());
        backend
            .inner
            .event_store
            .append(
                &connection,
                "Ev-stale",
                "message.channels",
                serde_json::json!({"type":"message","text":"stale"}),
            )
            .unwrap();
        let event_ref = backend
            .inner
            .event_store
            .replay("event:slack:00000000-0000-4000-8000-000000000002:1")
            .map(|event| event.event_ref)
            .unwrap_or_else(|| {
                lock(&backend.inner.event_store.events)[0]
                    .event
                    .event_ref
                    .clone()
            });

        assert_eq!(backend.connection_count(), 0);
        assert!(!backend.owns_connection(&ConnectionRequest::Describe(
            protocol::connection::DescribeRequest {
                connection_ref: connection.connection_ref.clone(),
            },
        )));
        assert!(!backend.owns_event(&EventRequest::Receive(protocol::event::ReceiveRequest {
            channel_ref: channel_ref(&connection),
            after: None,
            limit: 1,
            wait_ms: 0,
        })));
        assert!(!backend.owns_event(&EventRequest::Replay(protocol::event::ReplayRequest {
            event_ref,
        })));
        backend.shutdown().await;
    }

    /// Describe and read must agree about who owns a datasource. They did not: read ownership
    /// additionally required the binding ref to carry a Slack prefix, so a read whose binding ref
    /// was anything else fell out of every backend's claim and the registry answered
    /// `NotFound: no Integration owns this datasource` — for a datasource the same session had
    /// just described. That is what the deployed build reported for `slack.conversations` and
    /// `slack.users`. Ownership is about the datasource; the binding is the Integration's own
    /// refusal to make, in words that name the binding.
    #[tokio::test]
    async fn read_ownership_matches_describe_ownership_for_every_slack_datasource() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = SlackBackend::open_with_supervision(
            owner(),
            policy(),
            root.path(),
            Arc::new(MemoryStore::new()),
            false,
        )
        .await
        .unwrap();
        let read_of = |datasource_ref: &str, binding_ref: &str| {
            DatasourceRequest::Read(protocol::datasource::ReadRequest {
                datasource_ref: datasource_ref.to_owned(),
                binding_ref: binding_ref.to_owned(),
                description_ref: "datasource-description:slack:whatever".to_owned(),
                read: protocol::datasource::DatasourceRead::List {
                    limit: 20,
                    cursor: None,
                },
            })
        };
        for datasource_ref in SLACK_DATASOURCES {
            let describe = DatasourceRequest::Describe(protocol::datasource::DescribeRequest {
                datasource_ref: datasource_ref.to_owned(),
            });
            for binding_ref in [
                datasource_ref,
                "datasource-binding:slack:whatever",
                "binding:some-other-owner:1",
            ] {
                assert_eq!(
                    backend.owns_datasource(&describe),
                    backend.owns_datasource(&read_of(datasource_ref, binding_ref)),
                    "`{datasource_ref}` must be claimed for read exactly as for describe, \
                     whatever binding ref arrives (`{binding_ref}`)"
                );
            }
        }
        assert!(!backend.owns_datasource(&read_of(
            "kubernetes.workloads",
            "datasource-binding:slack:whatever"
        )));
        backend.shutdown().await;
    }

    /// Describing a Slack datasource with no Connection bound answered "was not found" — the
    /// datasource exists, the Connection does not, and only one of those is something a person
    /// can act on. It is also recoverable, so it must not read as terminal.
    #[tokio::test]
    async fn describing_without_a_bound_connection_names_the_connection_not_a_missing_datasource() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = SlackBackend::open_with_supervision(
            owner(),
            policy(),
            root.path(),
            Arc::new(MemoryStore::new()),
            false,
        )
        .await
        .unwrap();
        let refused = backend
            .handle_datasource(
                &owner(),
                DatasourceRequest::Describe(protocol::datasource::DescribeRequest {
                    datasource_ref: "slack.conversations".to_owned(),
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(refused.code, DatasourceErrorCode::NotGranted);
        assert!(refused.message.contains("slack.conversations"), "{refused:?}");
        assert!(refused.retriable, "{refused:?}");
        assert!(!refused.message.contains("not found"), "{refused:?}");
        // A datasource this Integration has never heard of is still genuinely absent.
        let unknown = backend
            .inner
            .describe_datasource(&owner(), "slack.nonexistent")
            .unwrap_err();
        assert_eq!(unknown.code, DatasourceErrorCode::NotFound);
        backend.shutdown().await;
    }

    #[tokio::test]
    async fn datasource_description_lease_ignores_request_scoped_provenance() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = SlackBackend::open_with_supervision(
            owner(),
            policy(),
            root.path(),
            Arc::new(MemoryStore::new()),
            false,
        )
        .await
        .unwrap();
        let plain = owner();
        let provenanced = owner()
            .with_hosted_provenance(
                "https://identity.example.test".to_owned(),
                "token-after-rotation".to_owned(),
                None,
                "request-2".to_owned(),
                "trace-2".to_owned(),
            )
            .unwrap();
        // A lease minted while serving describe must admit the read that arrives on the NEXT
        // authenticated request: a fresh request id, trace id, or rotated access token is not
        // an authority change and must not invalidate it.
        assert_eq!(
            backend
                .inner
                .datasource_description_ref(&plain, "slack.conversations"),
            backend
                .inner
                .datasource_description_ref(&provenanced, "slack.conversations"),
        );
        backend.shutdown().await;
    }

    #[test]
    fn a_local_companion_submission_is_one_bot_token_and_nothing_else() {
        // The local completion endpoint carries exactly one whitespace-free secret
        // (`connect-session-transport`, `secret_from_bytes`), so the app-plus-bot pair the hosted
        // receiver takes cannot be expressed here under any separator. A bot token alone is what
        // the reads need.
        let credentials = connection_runtime::parse_companion_submission("xoxb-real-looking-token").unwrap();
        assert!(credentials.app_token.is_none());
        assert!(credentials.bot_token.is_some());
        assert!(credentials.user_token.is_none());

        // An app-level token is not a bot token, and a pair is not a submission this transport
        // could ever have delivered.
        assert!(connection_runtime::parse_companion_submission("xapp-token").is_err());
        assert!(connection_runtime::parse_companion_submission("xapp-token xoxb-token").is_err());
        assert!(connection_runtime::parse_companion_submission("xapp-token\nxoxb-token").is_err());
        assert!(connection_runtime::parse_companion_submission("").is_err());
    }

    #[tokio::test]
    async fn a_connection_receiving_fewer_events_than_the_policy_lists_is_still_admitted() {
        // A workstation supplies a bot token and no app-level token, so its connection receives no
        // events at all. Requiring the stored event set to *equal* the policy's read that as a
        // forged connection and dropped it out of every datasource search: Slack was connected and
        // `slack.conversations` never appeared on the page.
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let backend = SlackBackend::open_with_supervision(
            owner(),
            policy(),
            root.path(),
            Arc::new(MemoryStore::new()),
            false,
        )
        .await
        .unwrap();
        let companion = |allowed_events: Vec<String>| StoredConnection {
            connection_ref: "connection:slack:companion".to_owned(),
            instance_id: "companion".to_owned(),
            label: "Slack".to_owned(),
            grant_ref: policy().grant_for_profile(PROFILE_COMPANION_BOT).to_owned(),
            initiation: InitiationConfig::Provider,
            allowed_events,
            owner_subject: owner().subject().to_owned(),
            team_id: "T012345".to_owned(),
            profile: SlackConnectionProfile::CompanionBot,
            external_subject_id: "U012345".to_owned(),
            scopes: vec!["channels:read".to_owned(), "users:read".to_owned()],
            purpose: String::new(),
            carries_operations: true,
        };

        assert!(backend.inner.connection_is_admitted(&companion(Vec::new())));
        assert!(
            backend
                .inner
                .connection_is_admitted(&companion(vec!["app_mention".to_owned()]))
        );
        // More than the policy admits stays refused, which is the invariant that matters.
        assert!(
            !backend
                .inner
                .connection_is_admitted(&companion(vec!["message.im".to_owned()]))
        );
        backend.shutdown().await;
    }


    #[test]
    fn a_declared_instance_name_fixes_its_identity_for_good() {
        // The whole reason a name exists. A random instance id made `connection:slack:<uuid>` and
        // every datasource binding ref change on every restart, so anything that referenced one was
        // dead by the next start. The name a person chose is what pins them.
        let first = instance_id_for_name("babelforce-bot");
        assert_eq!(first, instance_id_for_name("babelforce-bot"));
        assert_ne!(first, instance_id_for_name("timo-ai"));
        // It has to be a canonical uuid, because that is the only shape a credential address
        // admits (`connector-address`, `validate_instance`).
        assert_eq!(first.len(), 36);
        // The address type is the authority on the shape; building one proves it.
        assert!(
            CredentialRef::for_instance("tenant-local", AUTHORITY, &first, SERVICE, "bot_token")
                .is_ok()
        );
        assert_eq!(&first[14..15], "4");
    }

    #[test]
    fn a_credential_file_other_accounts_can_read_is_refused_rather_than_used() {
        // A token another local account can open has already leaked. Using it anyway and saying
        // nothing is the failure mode worth preventing: nobody would ever find out.
        let directory = tempfile::tempdir().unwrap();
        fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let path = directory.path().join("bot.token");
        fs::write(&path, "xoxb-not-a-real-token").unwrap();

        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            connection_runtime::read_credential_file(&path).unwrap_err().code,
            "instance-credential-unsafe"
        );

        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let secret = connection_runtime::read_credential_file(&path).unwrap();
        assert_eq!(secret.expose_secret(), "xoxb-not-a-real-token");

        // An empty file is a configuration mistake, not a credential.
        fs::write(&path, "   \n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            connection_runtime::read_credential_file(&path).unwrap_err().code,
            "instance-credential-shape"
        );
    }

}

#[cfg(test)]
mod tests {
    use connector_secrets::{MemoryStore, SecretStore as _};
    use protocol::connection::ConnectSessionState;
    use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
    use tokio::net::UnixStream;

    use super::*;

    const SENTINEL: &str = "SENTINEL-NOT-A-REAL-SECRET";

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
            initiation: InitiationConfig::Provider,
            allowed_events: vec!["app_mention".to_owned(), "message.channels".to_owned()],
            connect_session_ttl_seconds: 30,
        }
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
                operations: false,
                connections: true,
                events: true,
            }
        );
        let operation = OperationRequest::Search(protocol::operation::SearchRequest {
            query: String::new(),
            limit: 10,
        });
        assert!(!backend.owns_operation(&operation));
        assert_eq!(
            backend.handle(&owner(), operation).await.unwrap_err().code,
            OperationErrorCode::NotFound
        );

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
        let connection = lock(&backend.inner.metadata).connections[0].clone();
        let credential = credential_store
            .get(&backend.inner.credential_ref(&connection).unwrap())
            .await
            .unwrap();
        assert_eq!(credential.expose_secret(), submitted);
        backend.shutdown().await;
    }

    #[tokio::test]
    async fn event_is_durable_and_deduplicated_before_pull_and_replay() {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let store = EventStore::open(root.path().join("events.jsonl")).unwrap();
        let connection = StoredConnection {
            connection_ref: "connection:slack:00000000-0000-4000-8000-000000000001".to_owned(),
            instance_id: "00000000-0000-4000-8000-000000000001".to_owned(),
            label: "Development Slack".to_owned(),
            grant_ref: "grant:slack-inbound".to_owned(),
            initiation: InitiationConfig::Provider,
            allowed_events: vec!["message.channels".to_owned()],
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

        let reopened = EventStore::open(root.path().join("events.jsonl")).unwrap();
        let (events, cursor) = reopened
            .receive(&channel_ref(&connection), 0, 10, Duration::ZERO)
            .await;
        assert_eq!(events.len(), 1);
        assert_eq!(cursor, 1);
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
}

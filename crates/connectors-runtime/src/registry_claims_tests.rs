    /// A companion contributor: the Slack-shaped write whose description demands approval, the
    /// operation a companion mention's `event:` reference stands in for.
    fn companion_contributor(connection_ref: &str, local_lease: &str) -> Arc<SyntheticBackend> {
        let mut summary = operation_summary("slack-chat-post-message", connection_ref);
        summary.effect = EffectClass::Mutating;
        summary.approval = ApprovalPosture::Required;
        let mut description =
            operation_description("slack-chat-post-message", connection_ref, local_lease);
        description.effect = EffectClass::Mutating;
        description.approval = ApprovalPosture::Required;
        Arc::new(SyntheticBackend {
            capabilities: BackendCapabilities::OPERATIONS,
            operations: vec![summary],
            description: Some(description),
            invoke_connection: Some(connection_ref.to_owned()),
            claims_direct_operation: false,
            connections: Vec::new(),
            claims_connection: false,
            channels: Vec::new(),
            claims_event: false,
            calls: Calls::default(),
            invocation_leases: Mutex::new(Vec::new()),
        })
    }

    /// S-048: on the local placement, two presentations of the same companion event produce
    /// exactly one outward Slack reply — raced from two OS threads, because a retried invoke is
    /// concurrent with the original, not politely sequential.
    #[test]
    fn a_companion_reply_is_claimed_exactly_once_locally() {
        let backend = companion_contributor("connection:slack:companion", "lease:slack");
        let store: Arc<dyn connector_state::StateStore> =
            Arc::new(state_sqlite::SqliteState::in_memory().unwrap());
        let claims = EventReplyClaims::open(Arc::clone(&store)).unwrap();
        let registry = Arc::new(BackendRegistry::with_event_reply_claims(
            vec![Arc::clone(&backend) as Arc<dyn ConnectorBackend>],
            claims,
        ));
        let lease = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime")
            .block_on(async {
                let described = registry
                    .handle(
                        &context(),
                        OperationRequest::Describe(DescribeRequest {
                            operation_ref: "slack-chat-post-message".to_owned(),
                        }),
                    )
                    .await
                    .unwrap();
                let OperationResult::Describe(description) = described else {
                    panic!("registry returned the wrong result");
                };
                description.description_ref
            });
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let outcomes: Vec<Result<OperationResult, OperationError>> = [(); 2]
            .map(|()| {
                let registry = Arc::clone(&registry);
                let barrier = Arc::clone(&barrier);
                let lease = lease.clone();
                std::thread::spawn(move || {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .build()
                        .expect("runtime");
                    barrier.wait();
                    runtime.block_on(registry.handle(
                        &context(),
                        OperationRequest::Invoke(InvokeRequest {
                            operation_ref: "slack-chat-post-message".to_owned(),
                            connection_ref: "connection:slack:companion".to_owned(),
                            description_ref: lease,
                            input: json!({"text": "Hello from the companion"}),
                            approval_evidence_ref: Some("event:slack:companion-1".to_owned()),
                        }),
                    ))
                })
            })
            .into_iter()
            .map(|thread| thread.join().expect("presentation thread"))
            .collect();
        assert_eq!(
            backend.calls.operation_direct.load(Ordering::SeqCst),
            1,
            "two presentations of one companion event must produce exactly one outward reply"
        );
        let refused: Vec<&OperationError> = outcomes
            .iter()
            .filter_map(|outcome| outcome.as_ref().err())
            .collect();
        assert_eq!(
            outcomes.len() - refused.len(),
            1,
            "one presentation replies"
        );
        assert_eq!(refused.len(), 1, "the second presentation refuses");
        assert_eq!(refused[0].code, OperationErrorCode::ApprovalDenied);
        assert!(!refused[0].retriable, "a spent event claim is not retried");
        let refusals = store
            .read(EventReplyClaims::REFUSALS_KEY, 16 * 1024 * 1024)
            .unwrap()
            .expect("the refused replay is journaled");
        let refusals = String::from_utf8(refusals).unwrap();
        assert_eq!(refusals.lines().count(), 1);
        assert!(refusals.contains("event:slack:companion-1"));
        assert!(refusals.contains("slack-chat-post-message"));
    }

    /// The claim spends only what the companion-reply shape presents: a reference on an
    /// operation demanding nothing stays unspent, and an approval-demanding retry carrying no
    /// event reference stays the owner's own local admission (S-047).
    #[tokio::test]
    async fn an_undemanded_reference_is_not_spent_at_the_local_seam() {
        let read_backend = SyntheticBackend::contributor("connection:a", "lease:a");
        let write_backend = companion_contributor("connection:slack:companion", "lease:slack");
        let store: Arc<dyn connector_state::StateStore> =
            Arc::new(state_sqlite::SqliteState::in_memory().unwrap());
        let claims = EventReplyClaims::open(Arc::clone(&store)).unwrap();
        let registry = BackendRegistry::with_event_reply_claims(
            vec![
                Arc::clone(&read_backend) as Arc<dyn ConnectorBackend>,
                Arc::clone(&write_backend) as Arc<dyn ConnectorBackend>,
            ],
            claims,
        );
        for (operation_ref, connection_ref, evidence, counted) in [
            // A read demanding nothing: the same reference presented twice, never spent.
            (
                "tickets.read",
                "connection:a",
                Some("event:inert"),
                &read_backend,
            ),
            (
                "tickets.read",
                "connection:a",
                Some("event:inert"),
                &read_backend,
            ),
            // The demanded write without an event reference: local owner admission, repeatable.
            (
                "slack-chat-post-message",
                "connection:slack:companion",
                None,
                &write_backend,
            ),
            (
                "slack-chat-post-message",
                "connection:slack:companion",
                None,
                &write_backend,
            ),
        ] {
            let described = registry
                .handle(
                    &context(),
                    OperationRequest::Describe(DescribeRequest {
                        operation_ref: operation_ref.to_owned(),
                    }),
                )
                .await
                .unwrap();
            let OperationResult::Describe(description) = described else {
                panic!("registry returned the wrong result");
            };
            registry
                .handle(
                    &context(),
                    OperationRequest::Invoke(InvokeRequest {
                        operation_ref: operation_ref.to_owned(),
                        connection_ref: connection_ref.to_owned(),
                        description_ref: description.description_ref,
                        input: json!({}),
                        approval_evidence_ref: evidence.map(str::to_owned),
                    }),
                )
                .await
                .unwrap();
            assert!(counted.calls.operation_direct.load(Ordering::SeqCst) >= 1);
        }
        assert_eq!(
            read_backend.calls.operation_direct.load(Ordering::SeqCst),
            2
        );
        assert_eq!(
            write_backend.calls.operation_direct.load(Ordering::SeqCst),
            2
        );
        assert_eq!(
            store.read(EventReplyClaims::CLAIMS_KEY, 4096).unwrap(),
            None
        );
    }

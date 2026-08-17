use super::*;

impl SlackInner {
    pub(super) fn authorize_companion_event_reply(
        &self,
        connection: &StoredConnection,
        operation_ref: &str,
        event_ref: &str,
        input: &mut Value,
    ) -> Result<(), OperationError> {
        if connection.profile != SlackConnectionProfile::CompanionBot
            || operation_ref != "slack-chat-post-message"
        {
            return Err(OperationError::new(
                OperationErrorCode::ApprovalRequired,
                "an event may authorize only its companion bot's message reply",
                false,
            ));
        }
        let event = self
            .event_store
            .replay(event_ref)
            .filter(|event| {
                event.connection_ref == connection.connection_ref
                    && event.event_type == "app_mention"
            })
            .ok_or_else(|| {
                OperationError::new(
                    OperationErrorCode::ApprovalRequired,
                    "the Slack mention does not authorize this Connection",
                    false,
                )
            })?;
        let now = now_ms().ok_or_else(operation_unavailable)?;
        if now < event.received_at_unix_ms
            || now.saturating_sub(event.received_at_unix_ms) > AUTO_REPLY_MAX_AGE_MS
        {
            return Err(OperationError::new(
                OperationErrorCode::ApprovalRequired,
                "the Slack mention reply window has expired",
                false,
            ));
        }
        let payload = event.payload.as_object().ok_or_else(operation_invalid)?;
        let channel = payload
            .get("channel")
            .and_then(Value::as_str)
            .filter(|value| valid_slack_id(value))
            .ok_or_else(operation_invalid)?;
        let thread_ts = payload
            .get("thread_ts")
            .and_then(Value::as_str)
            .or_else(|| payload.get("ts").and_then(Value::as_str))
            .filter(|value| valid_slack_timestamp(value))
            .ok_or_else(operation_invalid)?;
        let input = input.as_object_mut().ok_or_else(operation_invalid)?;
        input.insert("channel".to_owned(), Value::String(channel.to_owned()));
        input.insert("thread_ts".to_owned(), Value::String(thread_ts.to_owned()));
        Ok(())
    }

    pub(super) fn describe(&self, connection: &StoredConnection) -> ConnectionDescription {
        let state = lock(&self.channel_states)
            .get(&connection.connection_ref)
            .copied()
            .unwrap_or(ChannelState::Starting);
        ConnectionDescription {
            summary: self.connection_summary(connection),
            channels: if connection.profile.receives_events() {
                vec![ConnectionChannelSummary {
                    channel_ref: channel_ref(connection),
                    binding_ref: SOCKET_BINDING_REF.to_owned(),
                    state,
                    events: connection.allowed_events.clone(),
                }]
            } else {
                Vec::new()
            },
        }
    }

    pub(super) fn connection_summary(&self, connection: &StoredConnection) -> ConnectionSummary {
        let state = lock(&self.channel_states)
            .get(&connection.connection_ref)
            .copied()
            .unwrap_or(ChannelState::Starting);
        let state = if connection.profile == SlackConnectionProfile::OrgUser {
            ConnectionState::Callable
        } else {
            match state {
                ChannelState::Starting => ConnectionState::Authorized,
                ChannelState::Connected => ConnectionState::Callable,
                ChannelState::Reconnecting | ChannelState::Stopped => ConnectionState::Degraded,
            }
        };
        ConnectionSummary {
            connection_ref: connection.connection_ref.clone(),
            integration_ref: INTEGRATION_REF.to_owned(),
            label: connection.label.clone(),
            state,
            initiation: initiation(connection.initiation),
            route: protocol::connection::ConnectionRoute::Direct,
            scope: Some(connection.profile.scope()),
            actor: Some(connection.profile.actor()),
            auth_profile: Some(connection.profile.as_str().to_owned()),
        }
    }

    pub(super) fn require_channel(
        &self,
        requested: &str,
        context: &PrincipalContext,
    ) -> Result<StoredConnection, EventError> {
        lock(&self.metadata)
            .connections
            .iter()
            .find(|connection| {
                channel_ref(connection) == requested
                    && connection.profile != SlackConnectionProfile::OrgBot
                    && self.connection_is_admitted(connection)
                    && self.connection_owned_by(connection, context)
            })
            .cloned()
            .ok_or_else(event_not_found)
    }

    pub(super) fn has_channel(&self, requested: &str) -> bool {
        lock(&self.metadata).connections.iter().any(|connection| {
            channel_ref(connection) == requested && self.connection_is_admitted(connection)
        })
    }

    pub(super) async fn create_session(
        self: &Arc<Self>,
        owner: &PrincipalContext,
        label: String,
        profile: SlackConnectionProfile,
    ) -> Result<ConnectSessionStatus, ConnectionError> {
        self.expire_hosted_sessions();
        let id = random_uuid().map_err(|_| connection_unavailable())?;
        let session_ref = format!("connect-session:{id}");
        let expires_at_unix_ms = now_ms()
            .and_then(|now| {
                now.checked_add(self.policy.connect_session_ttl_seconds.saturating_mul(1000))
            })
            .ok_or_else(connection_unavailable)?;
        let session_owner = SessionOwner {
            subject: owner.subject().to_owned(),
            email: owner.email().map(str::to_owned),
            profile,
        };
        if profile == SlackConnectionProfile::OrgUser && session_owner.email.is_none() {
            return Err(ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "Slack user OAuth requires a verified Identity email",
                false,
            ));
        }
        let status = match &self.completion_mode {
            CompletionMode::Local => {
                let directory = self.state_root.join("connect-sessions");
                let endpoint = BoundCompletionEndpoint::bind(&directory, &id)
                    .map_err(|_| connection_unavailable())?;
                let endpoint_path = endpoint.path().to_path_buf();
                let browser_completion_url = endpoint.browser_url();
                let status = match lock(&self.sessions).reserve_with_browser(
                    session_ref.clone(),
                    label,
                    expires_at_unix_ms,
                    endpoint_path.display().to_string(),
                    Some(browser_completion_url),
                ) {
                    Ok(status) => status,
                    Err(error) => {
                        drop(endpoint);
                        return Err(connect_session_error(error));
                    }
                };
                let inner = Arc::clone(self);
                let task_session_ref = session_ref.clone();
                lock(&self.tasks).push(tokio::spawn(async move {
                    inner.serve_completion(endpoint, task_session_ref).await;
                }));
                status
            }
            CompletionMode::Hosted { public_origin } => {
                let capability = random_capability().map_err(|_| connection_unavailable())?;
                let (oauth_state, oauth_authorize_url) =
                    if profile == SlackConnectionProfile::OrgUser {
                        let state = random_capability().map_err(|_| connection_unavailable())?;
                        let authorize = self
                            .oauth_authorize_url(&state)
                            .map_err(|_| connection_unavailable())?;
                        (Some(state), Some(authorize))
                    } else {
                        (None, None)
                    };
                let mut url = public_origin.clone();
                url.path_segments_mut()
                    .map_err(|_| connection_unavailable())?
                    .push("connect-sessions")
                    .push(&session_ref);
                url.set_fragment(Some(&format!("token={capability}")));
                let status = lock(&self.sessions)
                    .reserve_browser(session_ref.clone(), label, expires_at_unix_ms, url.into())
                    .map_err(connect_session_error)?;
                lock(&self.hosted_sessions).insert(
                    session_ref.clone(),
                    HostedSession {
                        capability_sha256: Sha256::digest(capability.as_bytes()).into(),
                        expires_at_unix_ms,
                        profile,
                        oauth_authorize_url,
                    },
                );
                if let Some(state) = oauth_state {
                    lock(&self.oauth_states).insert(
                        state,
                        OAuthPending {
                            session_ref: session_ref.clone(),
                            owner: session_owner.clone(),
                            expires_at_unix_ms,
                        },
                    );
                }
                status
            }
        };
        lock(&self.session_owners).insert(session_ref, session_owner);
        Ok(status)
    }

    pub(super) fn session_status(&self, session_ref: &str) -> Option<ConnectSessionStatus> {
        self.expire_hosted_sessions();
        lock(&self.sessions).status(session_ref)
    }

    pub(super) fn expire_hosted_sessions(&self) {
        let Some(now) = now_ms() else {
            return;
        };
        let expired = {
            let mut hosted_sessions = lock(&self.hosted_sessions);
            let expired = hosted_sessions
                .iter()
                .filter(|(_, session)| now >= session.expires_at_unix_ms)
                .map(|(session_ref, _)| session_ref.clone())
                .collect::<Vec<_>>();
            for session_ref in &expired {
                hosted_sessions.remove(session_ref);
            }
            expired
        };
        let mut sessions = lock(&self.sessions);
        let mut session_owners = lock(&self.session_owners);
        for session_ref in expired {
            let _ = sessions.finish(&session_ref, ConnectSessionTerminal::Expired);
            session_owners.remove(&session_ref);
        }
        lock(&self.oauth_states).retain(|_, pending| now < pending.expires_at_unix_ms);
    }

    pub(super) async fn serve_completion(
        self: Arc<Self>,
        endpoint: BoundCompletionEndpoint,
        session_ref: String,
    ) {
        let submission = match endpoint
            .receive(
                Duration::from_secs(self.policy.connect_session_ttl_seconds),
                Duration::from_secs(30),
                MAX_APP_TOKEN_BYTES,
            )
            .await
        {
            Ok(submission) => submission,
            Err(CompletionTransportError::Expired) => {
                let _ = lock(&self.sessions).finish(&session_ref, ConnectSessionTerminal::Expired);
                return;
            }
            Err(_) => {
                let _ = lock(&self.sessions).finish(&session_ref, ConnectSessionTerminal::Failed);
                return;
            }
        };
        let secret = submission.secret();
        let valid = secret.expose_secret().starts_with("xapp-");
        let result = if valid {
            let owner = lock(&self.session_owners)
                .get(&session_ref)
                .cloned()
                .ok_or_else(|| SlackError::new("connect-session"));
            match owner {
                Ok(owner) => {
                    self.complete_connection(
                        &session_ref,
                        owner,
                        String::new(),
                        String::new(),
                        Vec::new(),
                        SlackCredentials {
                            app_token: Some(Secret::new(secret.expose_secret())),
                            bot_token: None,
                            user_token: None,
                            refresh_token: None,
                        },
                    )
                    .await
                }
                Err(error) => Err(error),
            }
        } else {
            Err(SlackError::new("credential-shape"))
        };
        let accepted = match result {
            Ok(connection_ref) => {
                let _ = lock(&self.sessions).finish(
                    &session_ref,
                    ConnectSessionTerminal::Completed { connection_ref },
                );
                true
            }
            Err(_) => {
                let _ = lock(&self.sessions).finish(&session_ref, ConnectSessionTerminal::Failed);
                false
            }
        };
        let _ = submission.respond(accepted).await;
    }

    pub(super) async fn complete_connection(
        self: &Arc<Self>,
        session_ref: &str,
        owner: SessionOwner,
        team_id: String,
        external_subject_id: String,
        scopes: Vec<String>,
        credentials: SlackCredentials,
    ) -> Result<String, SlackError> {
        let label = lock(&self.sessions)
            .pending_label(session_ref)
            .map_err(|_| SlackError::new("connect-session"))?;
        let instance_id = random_uuid()?;
        let connection_ref = format!("connection:slack:{instance_id}");
        let connection = StoredConnection {
            connection_ref: connection_ref.clone(),
            instance_id: instance_id.clone(),
            label,
            grant_ref: self
                .policy
                .grant_for_profile(owner.profile.as_str())
                .to_owned(),
            initiation: self.policy.initiation,
            allowed_events: if owner.profile.receives_events() {
                self.policy.allowed_events.clone()
            } else {
                Vec::new()
            },
            owner_subject: owner.subject,
            team_id,
            profile: owner.profile,
            external_subject_id,
            scopes,
        };
        let app_credential_ref = self.app_credential_ref_for(&connection)?;
        if connection.profile == SlackConnectionProfile::Legacy
            && lock(&self.metadata)
                .connections
                .iter()
                .any(|stored| stored.profile == SlackConnectionProfile::Legacy)
        {
            let current = self
                .credential_store
                .get(&app_credential_ref)
                .await
                .map_err(|_| SlackError::new("credential-resolve"))?;
            let submitted = credentials
                .app_token
                .as_ref()
                .ok_or_else(|| SlackError::new("credential-shape"))?;
            if !constant_time_equal(
                current.expose_secret().as_bytes(),
                submitted.expose_secret().as_bytes(),
            ) {
                return Err(SlackError::new("app-token-conflict"));
            }
        }
        let bot_credential_ref =
            self.connection_credential_ref(&connection, BOT_TOKEN_CREDENTIAL)?;
        let user_credential_ref =
            self.connection_credential_ref(&connection, USER_TOKEN_CREDENTIAL)?;
        let refresh_credential_ref =
            self.connection_credential_ref(&connection, OAUTH_REFRESH_TOKEN_CREDENTIAL)?;
        let (transaction, generation) = self.reserve_transaction()?;
        let mut batch = SecretBatch::new(
            CredentialScope::new(self.tenant_id(), AUTHORITY)
                .map_err(|_| SlackError::new("credential-address"))?,
        );
        if let Some(app_token) = credentials.app_token {
            batch
                .put(app_credential_ref, app_token)
                .map_err(|_| SlackError::new("credential-batch"))?;
        }
        if let Some(bot_token) = credentials.bot_token {
            batch
                .put(bot_credential_ref, bot_token)
                .map_err(|_| SlackError::new("credential-batch"))?;
        }
        if let Some(user_token) = credentials.user_token {
            batch
                .put(user_credential_ref, user_token)
                .map_err(|_| SlackError::new("credential-batch"))?;
        }
        if let Some(refresh_token) = credentials.refresh_token {
            batch
                .put(refresh_credential_ref, refresh_token)
                .map_err(|_| SlackError::new("credential-batch"))?;
        }
        let digest = proposal_digest(&batch);
        self.credential_store
            .prepare(transaction, digest, &batch)
            .await
            .map_err(|_| SlackError::new("credential-prepare"))?;

        let transaction_hex = hex::encode(transaction.protocol_bytes());
        let pending_persisted = {
            let mut state = lock(&self.metadata);
            state.pending.push(PendingCommit {
                transaction_id: transaction_hex.clone(),
                connection: connection.clone(),
            });
            let persisted = self.persist_metadata(&state).is_ok();
            if !persisted {
                state
                    .pending
                    .retain(|pending| pending.transaction_id != transaction_hex);
            }
            persisted
        };
        if !pending_persisted {
            let _ = self.credential_store.abort(transaction).await;
            return Err(SlackError::new("connection-state"));
        }

        self.credential_store
            .commit(transaction)
            .await
            .map_err(|_| SlackError::new("credential-commit"))?;
        {
            let mut state = lock(&self.metadata);
            let prior = state.clone();
            state
                .pending
                .retain(|pending| pending.transaction_id != transaction_hex);
            state.connections.push(connection.clone());
            state
                .connections
                .sort_by(|a, b| a.connection_ref.cmp(&b.connection_ref));
            if let Err(error) = self.persist_metadata(&state) {
                *state = prior;
                return Err(error);
            }
        }
        let _ = self.credential_store.reclaim(generation).await;
        if connection.profile.receives_events() {
            self.start_supervisor(connection);
        }
        Ok(connection_ref)
    }

    pub(super) fn reserve_transaction(
        &self,
    ) -> Result<(SecretTransactionId, SecretTransactionGeneration), SlackError> {
        let mut state = lock(&self.metadata);
        let generation_value = state.next_transaction_generation;
        let generation =
            SecretTransactionGeneration::from_protocol_bytes(generation_value.to_be_bytes())
                .ok_or_else(|| SlackError::new("transaction-generation"))?;
        state.next_transaction_generation = generation_value
            .checked_add(1)
            .ok_or_else(|| SlackError::new("transaction-generation"))?;
        self.persist_metadata(&state)?;
        let mut nonce = [0_u8; 24];
        getrandom::fill(&mut nonce).map_err(|_| SlackError::new("randomness"))?;
        Ok((SecretTransactionId::new(generation, nonce), generation))
    }

    pub(super) async fn recover_pending(&self) -> Result<(), SlackError> {
        let pending = lock(&self.metadata).pending.clone();
        for record in pending {
            let transaction = decode_transaction(&record.transaction_id)?;
            match self
                .credential_store
                .state(transaction)
                .await
                .map_err(|_| SlackError::new("credential-recovery"))?
            {
                SecretTransactionState::Prepared => {
                    self.credential_store
                        .commit(transaction)
                        .await
                        .map_err(|_| SlackError::new("credential-recovery"))?;
                }
                SecretTransactionState::Committed => {}
                SecretTransactionState::Absent => {
                    let mut state = lock(&self.metadata);
                    state
                        .pending
                        .retain(|candidate| candidate.transaction_id != record.transaction_id);
                    self.persist_metadata(&state)?;
                    continue;
                }
            }
            let mut state = lock(&self.metadata);
            state
                .pending
                .retain(|candidate| candidate.transaction_id != record.transaction_id);
            if !state
                .connections
                .iter()
                .any(|connection| connection.connection_ref == record.connection.connection_ref)
            {
                state.connections.push(record.connection);
                state
                    .connections
                    .sort_by(|a, b| a.connection_ref.cmp(&b.connection_ref));
            }
            self.persist_metadata(&state)?;
        }
        Ok(())
    }

    pub(super) async fn verify_companion_credentials(
        &self,
        credentials: &SlackCredentials,
    ) -> Result<WorkspaceEvidence, SlackError> {
        let bot = credentials
            .bot_token
            .as_ref()
            .ok_or_else(|| SlackError::new("credential-shape"))?;
        if credentials.app_token.is_none()
            || credentials.user_token.is_some()
            || credentials.refresh_token.is_some()
        {
            return Err(SlackError::new("credential-shape"));
        }
        let evidence = self.auth_test(bot).await?;
        if !evidence.is_bot {
            return Err(SlackError::new("credential-subject"));
        }
        if self
            .policy
            .expected_team_id
            .as_deref()
            .is_some_and(|expected| expected != evidence.team_id)
        {
            return Err(SlackError::new("credential-workspace"));
        }
        Ok(evidence)
    }

    pub(super) async fn auth_test(&self, token: &Secret) -> Result<WorkspaceEvidence, SlackError> {
        let mut response = self
            .http
            .post(AUTH_TEST)
            .bearer_auth(token.expose_secret())
            .send()
            .await
            .map_err(|_| SlackError::new("credential-verify-unavailable"))?;
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|length| length > MAX_AUTH_TEST_RESPONSE_BYTES as u64)
        {
            return Err(SlackError::new("credential-verify-unavailable"));
        }
        let status = response.status();
        let content_length = response.content_length();
        let scopes = response
            .headers()
            .get("x-oauth-scopes")
            .and_then(|value| value.to_str().ok())
            .map(parse_scopes)
            .unwrap_or_default();
        let mut bytes = Zeroizing::new(Vec::with_capacity(MAX_AUTH_TEST_RESPONSE_BYTES));
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| SlackError::new("credential-verify-unavailable"))?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_AUTH_TEST_RESPONSE_BYTES {
                return Err(SlackError::new("credential-verify-unavailable"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let identity = classify_auth_test_response(status, content_length, &bytes)?;
        Ok(WorkspaceEvidence {
            team_id: identity.team_id,
            subject_id: identity.subject_id,
            scopes,
            is_bot: identity.is_bot,
        })
    }

    pub(super) fn app_credential_ref(&self) -> Result<CredentialRef, SlackError> {
        CredentialRef::new(self.tenant_id(), AUTHORITY, SERVICE, APP_TOKEN_CREDENTIAL)
            .map_err(|_| SlackError::new("credential-address"))
    }

    pub(super) fn app_credential_ref_for(
        &self,
        connection: &StoredConnection,
    ) -> Result<CredentialRef, SlackError> {
        if connection.profile == SlackConnectionProfile::CompanionBot {
            self.connection_credential_ref(connection, APP_TOKEN_CREDENTIAL)
        } else {
            self.app_credential_ref()
        }
    }

    pub(super) fn connection_credential_ref(
        &self,
        connection: &StoredConnection,
        credential: &str,
    ) -> Result<CredentialRef, SlackError> {
        CredentialRef::for_instance(
            self.tenant_id(),
            AUTHORITY,
            &connection.instance_id,
            SERVICE,
            credential,
        )
        .map_err(|_| SlackError::new("credential-address"))
    }

    pub(super) fn operation_credential_ref(
        &self,
        connection: &StoredConnection,
        credential: &str,
    ) -> Result<CredentialRef, SlackError> {
        if connection.profile == SlackConnectionProfile::OrgBot {
            CredentialRef::new(self.tenant_id(), AUTHORITY, SERVICE, credential)
                .map_err(|_| SlackError::new("credential-address"))
        } else {
            self.connection_credential_ref(connection, credential)
        }
    }

    pub(super) fn start_supervisor(self: &Arc<Self>, connection: StoredConnection) {
        lock(&self.channel_states)
            .insert(connection.connection_ref.clone(), ChannelState::Starting);
        if !self.supervision_enabled {
            return;
        }
        let mut started = lock(&self.supervisors_started);
        if !started.insert(connection.connection_ref.clone()) {
            return;
        }
        drop(started);
        let inner = Arc::clone(self);
        let shutdown = self.shutdown.subscribe();
        lock(&self.tasks).push(tokio::spawn(async move {
            inner.supervise(connection, shutdown).await;
        }));
    }

    pub(super) async fn supervise(
        self: Arc<Self>,
        connection: StoredConnection,
        mut shutdown: watch::Receiver<bool>,
    ) {
        let mut backoff = Duration::from_secs(1);
        loop {
            if *shutdown.borrow() {
                break;
            }
            self.set_connection_state(&connection.connection_ref, ChannelState::Reconnecting);
            let outcome = self.run_socket(&connection, &mut shutdown).await;
            if *shutdown.borrow() {
                break;
            }
            if outcome.is_ok() {
                backoff = Duration::from_secs(1);
            }
            tokio::select! {
                _ = tokio::time::sleep(backoff) => {}
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() { break; }
                }
            }
            backoff = (backoff * 2).min(Duration::from_secs(30));
        }
        self.set_connection_state(&connection.connection_ref, ChannelState::Stopped);
    }

    pub(super) fn set_connection_state(&self, connection_ref: &str, state: ChannelState) {
        lock(&self.channel_states).insert(connection_ref.to_owned(), state);
    }

    pub(super) async fn run_socket(
        &self,
        connection: &StoredConnection,
        shutdown: &mut watch::Receiver<bool>,
    ) -> Result<(), SlackError> {
        let credential_ref = self.app_credential_ref_for(connection)?;
        let token = self
            .credential_store
            .get(&credential_ref)
            .await
            .map_err(|_| SlackError::new("credential-resolve"))?;
        let mut response = self
            .http
            .post(APPS_CONNECTIONS_OPEN)
            .bearer_auth(token.expose_secret())
            .send()
            .await
            .map_err(|_| SlackError::new("socket-ticket-request"))?;
        drop(token);
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|size| size > 64 * 1024)
        {
            return Err(SlackError::new("socket-ticket-response"));
        }
        let mut bytes = Zeroizing::new(Vec::with_capacity(
            response.content_length().unwrap_or(0).min(64 * 1024) as usize,
        ));
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| SlackError::new("socket-ticket-response"))?
        {
            if bytes
                .len()
                .checked_add(chunk.len())
                .is_none_or(|size| size > 64 * 1024)
            {
                return Err(SlackError::new("socket-ticket-response"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let ticket: SocketTicket = serde_json::from_slice(&bytes)
            .map_err(|_| SlackError::new("socket-ticket-response"))?;
        if !ticket.ok {
            return Err(SlackError::new("socket-ticket-refused"));
        }
        let url = Zeroizing::new(
            ticket
                .url
                .ok_or_else(|| SlackError::new("socket-ticket-response"))?,
        );
        validate_socket_url(&url)?;
        let websocket = WebSocketConfig::default()
            .max_message_size(Some(MAX_SOCKET_MESSAGE_BYTES))
            .max_frame_size(Some(MAX_SOCKET_MESSAGE_BYTES));
        let (mut socket, _) =
            tokio_tungstenite::connect_async_with_config(&*url, Some(websocket), false)
                .await
                .map_err(|_| SlackError::new("socket-connect"))?;
        self.set_connection_state(&connection.connection_ref, ChannelState::Connected);
        loop {
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        let _ = socket.close(None).await;
                        return Ok(());
                    }
                }
                message = socket.next() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_socket_text(connection, text.as_ref(), &mut socket).await?;
                        }
                        Some(Ok(Message::Ping(payload))) => {
                            socket.send(Message::Pong(payload)).await.map_err(|_| SlackError::new("socket-write"))?;
                        }
                        Some(Ok(Message::Close(_))) | None => return Err(SlackError::new("socket-closed")),
                        Some(Ok(_)) => {}
                        Some(Err(_)) => return Err(SlackError::new("socket-read")),
                    }
                }
            }
        }
    }

    pub(super) async fn handle_socket_text<S>(
        &self,
        connection: &StoredConnection,
        text: &str,
        socket: &mut S,
    ) -> Result<(), SlackError>
    where
        S: futures_util::Sink<Message> + Unpin,
    {
        if text.len() > MAX_SOCKET_MESSAGE_BYTES {
            return Err(SlackError::new("socket-message-bound"));
        }
        let envelope: SocketEnvelope =
            serde_json::from_str(text).map_err(|_| SlackError::new("socket-envelope"))?;
        if envelope.kind == "disconnect" {
            return Err(SlackError::new("socket-refresh"));
        }
        let Some(envelope_id) = envelope.envelope_id else {
            return Ok(());
        };
        if envelope_id.is_empty() || envelope_id.len() > 512 {
            return Err(SlackError::new("socket-envelope"));
        }
        if envelope.kind == "events_api" {
            let matches_connection = envelope
                .payload
                .as_ref()
                .and_then(|payload| payload.get("team_id"))
                .and_then(Value::as_str)
                .is_some_and(|team_id| {
                    (connection.team_id.is_empty() || connection.team_id == team_id)
                        && self.connection_is_admitted(connection)
                });
            if matches_connection {
                if let Some((delivery_id, event_type, payload)) =
                    project_data_event(envelope.payload.as_ref(), &connection.allowed_events)
                {
                    self.event_store
                        .append(connection, &delivery_id, &event_type, payload)?;
                }
            }
        }
        let acknowledgement = serde_json::to_string(&serde_json::json!({
            "envelope_id": envelope_id,
        }))
        .map_err(|_| SlackError::new("socket-ack"))?;
        socket
            .send(Message::Text(acknowledgement.into()))
            .await
            .map_err(|_| SlackError::new("socket-ack"))
    }
}

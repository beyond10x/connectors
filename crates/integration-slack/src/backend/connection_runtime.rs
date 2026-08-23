use super::*;

impl SlackInner {
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
        let owner = lock(&self.session_owners)
            .get(&session_ref)
            .cloned()
            .ok_or_else(|| SlackError::new("connect-session"));
        let result = match owner {
            // A companion-bot session carries both tokens, in the same `xapp-\nxoxb-` form the
            // hosted receiver accepts, and is verified the same way. Without this a personal
            // placement acquired only the app token — Socket Mode events and nothing else — so
            // `slack.conversations` and `slack.users` were published and could never be read: every
            // read needs the bot credential the local flow had no way to accept. The workspace pin
            // still holds; `verify_companion_credentials` checks the token's own team against it.
            Ok(owner) if owner.profile == SlackConnectionProfile::CompanionBot => {
                match parse_companion_submission(secret.expose_secret()) {
                    Ok(credentials) => match self
                        .verify_companion_credentials(&session_ref, &credentials)
                        .await
                    {
                        Ok(evidence) => {
                            self.complete_connection(
                                &session_ref,
                                owner,
                                evidence.team_id,
                                evidence.subject_id,
                                evidence.scopes,
                                credentials,
                            )
                            .await
                        }
                        Err(error) => Err(error),
                    },
                    Err(error) => Err(error),
                }
            }
            Ok(owner) if secret.expose_secret().starts_with("xapp-") => {
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
            Ok(_) => Err(SlackError::new("credential-shape")),
            Err(error) => Err(error),
        };
        let accepted = match result {
            Ok(connection_ref) => {
                let _ = lock(&self.sessions).finish(
                    &session_ref,
                    ConnectSessionTerminal::Completed { connection_ref },
                );
                true
            }
            Err(error) => {
                // The session's own state carries no reason — `Failed` is all a caller can see —
                // so the placement's log is the only place a person can find out whether Slack
                // rejected the token, the workspace did not match, or the API was unreachable.
                // The code only: a credential must never reach a log, and these codes name a class
                // of fault rather than anything submitted.
                eprintln!(
                    "slack: connect session {session_ref} failed: {}",
                    error.code
                );
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
        // A session-acquired Connection has no name a person chose, so its identity is random and
        // it exists for as long as its state file does. A declared instance takes the other route
        // through `register_connection`, where the name fixes the identity.
        self.register_connection(
            random_uuid()?,
            label,
            String::new(),
            owner,
            team_id,
            external_subject_id,
            scopes,
            credentials,
        )
        .await
    }

    /// Registers one Connection and commits its credentials in a single prepared transaction.
    ///
    /// Shared by both routes into a Slack Connection — a Connect Session somebody completed, and an
    /// instance declared in configuration — because everything below the point where the credential
    /// is in hand is identical, and a second copy of the prepare/persist/commit ordering is a
    /// second place for a half-written connection to appear.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn register_connection(
        self: &Arc<Self>,
        instance_id: String,
        label: String,
        purpose: String,
        owner: SessionOwner,
        team_id: String,
        external_subject_id: String,
        scopes: Vec<String>,
        credentials: SlackCredentials,
    ) -> Result<String, SlackError> {
        self.register_connection_with(
            instance_id,
            label,
            purpose,
            true,
            owner,
            team_id,
            external_subject_id,
            scopes,
            credentials,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn register_connection_with(
        self: &Arc<Self>,
        instance_id: String,
        label: String,
        purpose: String,
        carries_operations: bool,
        owner: SessionOwner,
        team_id: String,
        external_subject_id: String,
        scopes: Vec<String>,
        credentials: SlackCredentials,
    ) -> Result<String, SlackError> {
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
            // Events need the app-level token that opens the socket they arrive on. A connection
            // that advertised events it has no credential to receive would read to a person as
            // wired up and then stay silent forever.
            allowed_events: if owner.profile.receives_events() && credentials.app_token.is_some() {
                self.policy.allowed_events.clone()
            } else {
                Vec::new()
            },
            owner_subject: owner.subject,
            team_id,
            profile: owner.profile,
            external_subject_id,
            scopes,
            purpose,
            carries_operations,
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
        authority_ref: &str,
        credentials: &SlackCredentials,
    ) -> Result<WorkspaceEvidence, SlackError> {
        let bot = credentials
            .bot_token
            .as_ref()
            .ok_or_else(|| SlackError::new("credential-shape"))?;
        // The bot token is what a companion connection is for; the app-level token is optional
        // because it only carries Socket Mode. A hosted receiver supplies both, a workstation
        // usually supplies neither more than one — and a connection without events is a real,
        // usable connection, where a connection without the bot token can answer nothing.
        if credentials.user_token.is_some() || credentials.refresh_token.is_some() {
            return Err(SlackError::new("credential-shape"));
        }
        let evidence = self.auth_test(authority_ref, bot).await?;
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

    pub(super) async fn auth_test(
        &self,
        authority_ref: &str,
        token: &Secret,
    ) -> Result<WorkspaceEvidence, SlackError> {
        let response = self
            .egress
            .execute(
                authority_ref,
                EgressHttpRequest {
                    request: bearer_request("POST", AUTH_TEST.to_owned(), token),
                    maximum_response_bytes: MAX_AUTH_TEST_RESPONSE_BYTES,
                    response_headers: vec!["x-oauth-scopes".to_owned()],
                },
            )
            .await
            .map_err(|_| SlackError::new("credential-verify-unavailable"))?;
        if !response.is_success() {
            return Err(SlackError::new("credential-verify-unavailable"));
        }
        let scopes = response
            .header("x-oauth-scopes")
            .map(parse_scopes)
            .unwrap_or_default();
        let status = reqwest::StatusCode::from_u16(response.status)
            .map_err(|_| SlackError::new("credential-verify-unavailable"))?;
        let body = Zeroizing::new(response.body);
        let identity = classify_auth_test_response(status, Some(body.len() as u64), &body)?;
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
        // The tenant-wide organisation install is the one Connection whose credential has no
        // instance in its address, because there is exactly one of it and `ensure_org_connection`
        // writes it there. Every other Connection — including a declared `org_bot` instance, of
        // which a placement may hold several — owns its credential at its own instance address.
        //
        // Keying this on the profile instead of on that one Connection is what broke a declared
        // `org_bot`: it stored its token per instance and then looked for it at the org address,
        // and the read came back "Slack datasource is not granted for this Connection" for a token
        // that was present and valid.
        if connection.connection_ref == ORG_BOT_CONNECTION {
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
        let response = self
            .egress
            .execute(
                &connection.connection_ref,
                EgressHttpRequest {
                    request: bearer_request("POST", APPS_CONNECTIONS_OPEN.to_owned(), &token),
                    maximum_response_bytes: 64 * 1024,
                    response_headers: Vec::new(),
                },
            )
            .await
            .map_err(|_| SlackError::new("socket-ticket-request"))?;
        drop(token);
        if !response.is_success() {
            return Err(SlackError::new("socket-ticket-response"));
        }
        let bytes = Zeroizing::new(response.body);
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
        let mut socket = self
            .egress
            .connect_websocket(
                &connection.connection_ref,
                url.to_string(),
                MAX_SOCKET_MESSAGE_BYTES,
            )
            .await
            .map_err(|_| SlackError::new("socket-connect"))?;
        self.set_connection_state(&connection.connection_ref, ChannelState::Connected);
        loop {
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        let _ = socket.close().await;
                        return Ok(());
                    }
                }
                message = socket.receive() => {
                    match message {
                        Ok(EgressWebSocketFrame::Text(text)) => {
                            self.handle_socket_text(connection, &text, socket.as_mut()).await?;
                        }
                        Ok(EgressWebSocketFrame::Ping(payload)) => {
                            socket.send_pong(payload).await.map_err(|_| SlackError::new("socket-write"))?;
                        }
                        Ok(EgressWebSocketFrame::Closed) => return Err(SlackError::new("socket-closed")),
                        Ok(EgressWebSocketFrame::Other) => {}
                        Err(_) => return Err(SlackError::new("socket-read")),
                    }
                }
            }
        }
    }

    pub(super) async fn handle_socket_text(
        &self,
        connection: &StoredConnection,
        text: &str,
        socket: &mut dyn EgressWebSocket,
    ) -> Result<(), SlackError> {
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
            .send_text(acknowledgement)
            .await
            .map_err(|_| SlackError::new("socket-ack"))
    }
}

/// Reads one local companion-bot submission: a bot token, and nothing else.
///
/// One secret, because that is what the transport carries. A local completion endpoint reads with
/// `read_until(b'\n')` and then refuses any submission containing whitespace at all
/// (`connect-session-transport`, `secret_from_bytes`), so the app-plus-bot pair the hosted
/// receiver takes as two lines cannot be expressed here under any separator — the hosted path
/// carries an HTTP body and has no such bound.
///
/// A bot token alone is what the reads need: `slack.conversations` and `slack.users` are Web API
/// calls and every one of them resolves the bot credential. The app-level token exists for Socket
/// Mode, so a workstation that supplies none simply receives no events, which `complete_connection`
/// records rather than pretends.
///
/// The token is never logged, echoed, or returned in a refusal — a malformed submission says only
/// that it was malformed.
pub(super) fn parse_companion_submission(value: &str) -> Result<SlackCredentials, SlackError> {
    if !valid_slack_token(value, "xoxb-") {
        return Err(SlackError::new("credential-shape"));
    }
    Ok(SlackCredentials {
        app_token: None,
        bot_token: Some(Secret::new(value)),
        user_token: None,
        refresh_token: None,
    })
}

impl SlackInner {
    /// Materialises every Slack identity this placement's configuration declares.
    ///
    /// A workstation reaches Slack as more than one actor at once — the workspace bot for looking
    /// things up, the operator themself for what only they can see, an assistant bot for posting —
    /// and each is a separate Connection with its own credential and its own authority. Declaring
    /// them is what makes that possible without a person completing three Connect Sessions by hand
    /// on every restart, and what makes each one's identity stable enough to reference.
    ///
    /// The credential arrives as a **path**. Nothing above this reads it: the placement is the
    /// process admitted to hold credentials, so it is the one that opens the file, and the value
    /// goes straight into the credential store addressed by this instance.
    ///
    /// Already-registered instances are skipped rather than re-verified, so a restart against
    /// surviving state costs no Slack API call. A declared instance that fails is reported and
    /// skipped: one bad token must not take the other two identities, or the whole placement, down.
    pub(super) async fn ensure_declared_instances(self: &Arc<Self>) -> Result<(), SlackError> {
        for instance in self.policy.instances.clone() {
            let instance_id = instance_id_for_name(&instance.name);
            let connection_ref = format!("connection:slack:{instance_id}");
            if lock(&self.metadata)
                .connections
                .iter()
                .any(|stored| stored.connection_ref == connection_ref)
            {
                continue;
            }
            if let Err(error) = self
                .register_declared_instance(&instance, instance_id)
                .await
            {
                eprintln!(
                    "slack: instance `{}` was declared and is not connected: {}",
                    instance.name, error.code
                );
            }
        }
        Ok(())
    }

    async fn register_declared_instance(
        self: &Arc<Self>,
        instance: &SlackInstanceConfig,
        instance_id: String,
    ) -> Result<(), SlackError> {
        let token = read_credential_file(&instance.token_file)?;
        if !valid_slack_token(token.expose_secret(), instance.profile.token_prefix()) {
            return Err(SlackError::new("instance-credential-shape"));
        }
        let profile = match instance.profile {
            SlackInstanceProfile::OrgBot => SlackConnectionProfile::OrgBot,
            SlackInstanceProfile::OrgUser => SlackConnectionProfile::OrgUser,
            SlackInstanceProfile::CompanionBot => SlackConnectionProfile::CompanionBot,
        };
        // The Connection this credential is about to become is the authority for verifying it —
        // the egress boundary admits only a `connection:`/`connect-session:` ref, and a bare
        // instance name would be refused as an unknown authority.
        let evidence = self
            .auth_test(&format!("connection:slack:{instance_id}"), &token)
            .await?;
        // The declared actor and the token's own actor have to agree. A user token filed as the
        // workspace bot would be read-only by policy and act as a person in fact — the exact
        // confusion naming these instances is meant to remove.
        let expects_a_bot = !matches!(instance.profile, SlackInstanceProfile::OrgUser);
        if evidence.is_bot != expects_a_bot {
            return Err(SlackError::new("instance-credential-subject"));
        }
        if self
            .policy
            .expected_team_id
            .as_deref()
            .is_some_and(|expected| expected != evidence.team_id)
        {
            return Err(SlackError::new("credential-workspace"));
        }
        let credentials = if matches!(instance.profile, SlackInstanceProfile::OrgUser) {
            SlackCredentials {
                app_token: None,
                bot_token: None,
                user_token: Some(Secret::new(token.expose_secret())),
                refresh_token: None,
            }
        } else {
            SlackCredentials {
                app_token: None,
                bot_token: Some(Secret::new(token.expose_secret())),
                user_token: None,
                refresh_token: None,
            }
        };
        drop(token);
        let owner = SessionOwner {
            subject: self.owner_subject(),
            email: None,
            profile,
        };
        // An identity that is not the operations identity still publishes its datasources — a
        // binding names its Connection, so reading as each of them stays unambiguous — but it must
        // not publish operations, because the agent's capability projection admits one Connection
        // per operation reference and two identities publishing the same one refuse the session.
        let carries_operations = instance.operations;
        self.register_connection_with(
            instance_id,
            instance.name.clone(),
            instance.purpose.clone().unwrap_or_default(),
            carries_operations,
            owner,
            evidence.team_id,
            evidence.subject_id,
            evidence.scopes,
            credentials,
        )
        .await?;
        Ok(())
    }

    /// The subject a Connection this placement creates for itself belongs to.
    fn owner_subject(&self) -> String {
        match &self.admission {
            PrincipalAdmission::Exact(owner) => owner.subject().to_owned(),
            PrincipalAdmission::Tenant(_) => String::new(),
        }
    }
}

/// Reads one credential out of an owner-only file.
///
/// The mode is checked before the read, not after: a token any other local account can open is a
/// token that has already leaked, and reporting that as a refusal rather than quietly using it is
/// the only way a person finds out. Bounded, because a path that turns out to be a log or a device
/// must not be pulled into memory whole.
pub(super) fn read_credential_file(path: &Path) -> Result<Secret, SlackError> {
    const MAX_CREDENTIAL_FILE_BYTES: u64 = 8 * 1024;
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| SlackError::new("instance-credential-unreadable"))?;
    if !metadata.file_type().is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.len() > MAX_CREDENTIAL_FILE_BYTES
    {
        return Err(SlackError::new("instance-credential-unsafe"));
    }
    let value = Zeroizing::new(
        fs::read_to_string(path).map_err(|_| SlackError::new("instance-credential-unreadable"))?,
    );
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SlackError::new("instance-credential-shape"));
    }
    Ok(Secret::new(trimmed))
}

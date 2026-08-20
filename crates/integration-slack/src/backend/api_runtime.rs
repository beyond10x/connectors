use super::*;

use connector_secrets::StoreError;

impl SlackInner {
    pub(super) fn persist_metadata(&self, state: &StateFile) -> Result<(), SlackError> {
        write_state(
            &self.state_root.join("connections.json"),
            self.hosted_state.as_ref(),
            state,
        )
    }

    pub(super) fn tenant_id(&self) -> &str {
        match &self.admission {
            PrincipalAdmission::Exact(owner) => owner.tenant_id(),
            PrincipalAdmission::Tenant(tenant) => tenant,
        }
    }

    pub(super) fn oauth_authorize_url(&self, state: &str) -> Result<String, SlackError> {
        let client_id = self
            .policy
            .oauth_client_id
            .as_deref()
            .ok_or_else(|| SlackError::new("oauth-config"))?;
        let redirect_uri = self
            .policy
            .oauth_redirect_uri
            .as_deref()
            .ok_or_else(|| SlackError::new("oauth-config"))?;
        let team = self
            .policy
            .expected_team_id
            .as_deref()
            .ok_or_else(|| SlackError::new("oauth-config"))?;
        let mut url = url::Url::parse("https://slack.com/oauth/v2_user/authorize")
            .map_err(|_| SlackError::new("oauth-config"))?;
        url.query_pairs_mut()
            .append_pair("client_id", client_id)
            .append_pair("scope", USER_OAUTH_SCOPES)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("state", state)
            .append_pair("team", team);
        Ok(url.into())
    }

    pub(super) async fn complete_user_oauth(
        self: &Arc<Self>,
        state: &str,
        code: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), SlackError> {
        self.expire_hosted_sessions();
        let pending = lock(&self.oauth_states)
            .remove(state)
            .ok_or_else(|| SlackError::new("oauth-state"))?;
        let session_ref = pending.session_ref.clone();
        let outcome: Result<String, SlackError> = async {
            if pending.owner.profile != SlackConnectionProfile::OrgUser
                || error.is_some()
                || code.is_none()
                || now_ms().is_none_or(|now| now >= pending.expires_at_unix_ms)
            {
                return Err(SlackError::new("oauth-refused"));
            }
            let client_id = self
                .policy
                .oauth_client_id
                .as_deref()
                .ok_or_else(|| SlackError::new("oauth-config"))?;
            let redirect_uri = self
                .policy
                .oauth_redirect_uri
                .as_deref()
                .ok_or_else(|| SlackError::new("oauth-config"))?;
            let client_secret_ref = CredentialRef::new(
                self.tenant_id(),
                AUTHORITY,
                "login",
                OAUTH_CLIENT_SECRET_CREDENTIAL,
            )
            .map_err(|_| SlackError::new("credential-address"))?;
            let client_secret = self
                .credential_store
                .get(&client_secret_ref)
                .await
                .map_err(|_| SlackError::new("oauth-config"))?;
            let body = url::form_urlencoded::Serializer::new(String::new())
                .append_pair("code", code.expect("checked OAuth code"))
                .append_pair("redirect_uri", redirect_uri)
                .append_pair("grant_type", "authorization_code")
                .finish();
            let authorization = format!(
                "Basic {}",
                base64::engine::general_purpose::STANDARD
                    .encode(format!("{client_id}:{}", client_secret.expose_secret()))
            );
            let exchange = connector_resolve::Request {
                method: "POST".to_owned(),
                url: USER_OAUTH_ACCESS.to_owned(),
                headers: BTreeMap::from([
                    ("Authorization".to_owned(), authorization),
                    (
                        "Content-Type".to_owned(),
                        "application/x-www-form-urlencoded".to_owned(),
                    ),
                ]),
                body: Some(body),
            };
            let response = self
                .egress
                .execute(
                    &session_ref,
                    EgressHttpRequest {
                        request: exchange,
                        maximum_response_bytes: MAX_AUTH_TEST_RESPONSE_BYTES,
                        response_headers: Vec::new(),
                    },
                )
                .await
                .map_err(|_| SlackError::new("oauth-exchange"))?;
            drop(client_secret);
            if !response.is_success() {
                return Err(SlackError::new("oauth-exchange"));
            }
            let exchanged: UserOAuthResponse = serde_json::from_slice(&response.body)
                .map_err(|_| SlackError::new("oauth-exchange"))?;
            let actor = exchanged
                .authed_user
                .ok_or_else(|| SlackError::new("oauth-refused"))?;
            let team_id = exchanged
                .team
                .map(|team| team.id)
                .ok_or_else(|| SlackError::new("oauth-refused"))?;
            let access_token = exchanged
                .access_token
                .filter(|token| token.starts_with("xoxp-") && token.len() <= 2048)
                .ok_or_else(|| SlackError::new("oauth-refused"))?;
            if !exchanged.ok
                || self.policy.expected_team_id.as_deref() != Some(team_id.as_str())
                || !valid_slack_id(&actor.id)
            {
                return Err(SlackError::new("credential-workspace"));
            }
            let access_token = Secret::new(access_token);
            let slack_email = self
                .slack_user_email(&session_ref, &access_token, &actor.id)
                .await?;
            let expected_email = pending
                .owner
                .email
                .as_deref()
                .ok_or_else(|| SlackError::new("oauth-email"))?;
            if normalize_email(&slack_email) != normalize_email(expected_email) {
                return Err(SlackError::new("oauth-email"));
            }
            let scopes = parse_scopes(&actor.scope);
            let credentials = SlackCredentials {
                app_token: None,
                bot_token: None,
                user_token: Some(access_token),
                refresh_token: exchanged.refresh_token.map(Secret::new),
            };
            self.complete_connection(
                &session_ref,
                pending.owner,
                team_id,
                actor.id,
                scopes,
                credentials,
            )
            .await
        }
        .await;
        lock(&self.hosted_sessions).remove(&session_ref);
        match outcome {
            Ok(connection_ref) => lock(&self.sessions)
                .finish(
                    &session_ref,
                    ConnectSessionTerminal::Completed { connection_ref },
                )
                .map_err(|_| SlackError::new("connect-session")),
            Err(error) => {
                let _ = lock(&self.sessions).finish(&session_ref, ConnectSessionTerminal::Failed);
                Err(error)
            }
        }
    }

    pub(super) async fn slack_user_email(
        &self,
        authority_ref: &str,
        token: &Secret,
        user_id: &str,
    ) -> Result<String, SlackError> {
        let mut target = url::Url::parse(USERS_INFO).map_err(|_| SlackError::new("oauth-email"))?;
        target.query_pairs_mut().append_pair("user", user_id);
        let response = self
            .egress
            .execute(
                authority_ref,
                EgressHttpRequest {
                    request: bearer_request("GET", target.into(), token),
                    maximum_response_bytes: MAX_AUTH_TEST_RESPONSE_BYTES,
                    response_headers: Vec::new(),
                },
            )
            .await
            .map_err(|_| SlackError::new("oauth-email"))?;
        if !response.is_success() {
            return Err(SlackError::new("oauth-email"));
        }
        let info: SlackUserInfoResponse =
            serde_json::from_slice(&response.body).map_err(|_| SlackError::new("oauth-email"))?;
        if !info.ok {
            return Err(SlackError::new("oauth-email"));
        }
        info.user
            .and_then(|user| user.profile.email)
            .filter(|email| normalize_email(email).is_some())
            .ok_or_else(|| SlackError::new("oauth-email"))
    }

    pub(super) async fn ensure_org_connection(self: &Arc<Self>) -> Result<(), SlackError> {
        let Some(expected_team_id) = self.policy.expected_team_id.clone() else {
            return Ok(());
        };
        let app_ref = self.app_credential_ref()?;
        let bot_ref =
            CredentialRef::new(self.tenant_id(), AUTHORITY, SERVICE, BOT_TOKEN_CREDENTIAL)
                .map_err(|_| SlackError::new("credential-address"))?;
        // Naming a workspace is a policy statement, not a claim that an org-wide app is installed.
        // These two reads used to collapse `NotFound` into a refusal, so setting `expected_team_id`
        // on a personal placement — the only way to pin which workspace a Connect Session may bind
        // — stopped the daemon from starting at all with `org-app-credential`, and the workstation
        // had to install an org app it never wanted. Nothing stored means no org install; the
        // workspace pin still holds, because every Connect Session checks the token's own team
        // against it (`connection_runtime.rs`, `verify_bot`). An unreachable store is still a
        // refusal: "we cannot say" must never read as "not configured".
        let app = match self.credential_store.get(&app_ref).await {
            Ok(app) => app,
            Err(StoreError::NotFound { .. }) => return Ok(()),
            Err(_) => return Err(SlackError::new("org-app-credential")),
        };
        if !valid_slack_token(app.expose_secret(), "xapp-") {
            return Err(SlackError::new("org-app-credential"));
        }
        drop(app);
        let bot = match self.credential_store.get(&bot_ref).await {
            Ok(bot) => bot,
            Err(StoreError::NotFound { .. }) => return Ok(()),
            Err(_) => return Err(SlackError::new("org-bot-credential")),
        };
        if !valid_slack_token(bot.expose_secret(), "xoxb-") {
            return Err(SlackError::new("org-bot-credential"));
        }
        let evidence = self.auth_test(ORG_BOT_CONNECTION, &bot).await?;
        drop(bot);
        if !evidence.is_bot || evidence.team_id != expected_team_id {
            return Err(SlackError::new("credential-workspace"));
        }
        let connection = StoredConnection {
            connection_ref: ORG_BOT_CONNECTION.to_owned(),
            instance_id: "org-bot".to_owned(),
            label: "Organization Slack bot".to_owned(),
            grant_ref: self.policy.grant_for_profile(PROFILE_ORG_BOT).to_owned(),
            initiation: self.policy.initiation,
            allowed_events: Vec::new(),
            owner_subject: String::new(),
            team_id: expected_team_id,
            profile: SlackConnectionProfile::OrgBot,
            external_subject_id: evidence.subject_id,
            scopes: evidence.scopes,
            purpose: String::new(),
        };
        let mut state = lock(&self.metadata);
        state
            .connections
            .retain(|stored| stored.profile != SlackConnectionProfile::OrgBot);
        state.connections.push(connection);
        state
            .connections
            .sort_by(|left, right| left.connection_ref.cmp(&right.connection_ref));
        self.persist_metadata(&state)
    }

    pub(super) fn connection_is_admitted(&self, connection: &StoredConnection) -> bool {
        connection.grant_ref == self.policy.grant_for_profile(connection.profile.as_str())
            && connection.initiation == self.policy.initiation
            // A stored connection may admit no event its profile cannot receive and no event this
            // policy does not list — a subset, not an exact match. Equality was the rule until a
            // connection legitimately carrying *fewer* events appeared: a workstation that supplies
            // a bot token and no app-level token receives nothing, so its event set is empty, and
            // an exact-match rule then read that connection as forged and dropped it out of every
            // datasource search. The invariant is "never more than the policy admits".
            && (connection.profile.receives_events() || connection.allowed_events.is_empty())
            && connection
                .allowed_events
                .iter()
                .all(|event| self.policy.allowed_events.contains(event))
            && match self.admission {
                PrincipalAdmission::Exact(_) => true,
                PrincipalAdmission::Tenant(_) => {
                    connection.profile != SlackConnectionProfile::Legacy
                        && !connection.team_id.is_empty()
                        && (connection.profile == SlackConnectionProfile::OrgBot
                            || !connection.owner_subject.is_empty())
                }
            }
    }

    pub(super) fn connection_owned_by(
        &self,
        connection: &StoredConnection,
        context: &PrincipalContext,
    ) -> bool {
        self.context_admitted(context)
            && match &self.admission {
                PrincipalAdmission::Exact(owner) => {
                    connection.owner_subject.is_empty()
                        || connection.owner_subject == owner.subject()
                }
                PrincipalAdmission::Tenant(_) => {
                    connection.profile == SlackConnectionProfile::OrgBot
                        || connection.owner_subject == context.subject()
                }
            }
    }

    pub(super) fn context_admitted(&self, actual: &PrincipalContext) -> bool {
        match &self.admission {
            PrincipalAdmission::Exact(owner) => owner.as_ref() == actual,
            PrincipalAdmission::Tenant(tenant) => tenant == actual.tenant_id(),
        }
    }

    pub(super) fn check_connection_context(
        &self,
        actual: &PrincipalContext,
    ) -> Result<(), ConnectionError> {
        if self.context_admitted(actual) {
            Ok(())
        } else {
            Err(ConnectionError::new(
                ConnectionErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
    }

    pub(super) fn check_event_context(&self, actual: &PrincipalContext) -> Result<(), EventError> {
        if self.context_admitted(actual) {
            Ok(())
        } else {
            Err(EventError::new(
                EventErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
    }

    pub(super) fn check_operation_context(
        &self,
        actual: &PrincipalContext,
    ) -> Result<(), OperationError> {
        if self.context_admitted(actual) {
            Ok(())
        } else {
            Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
    }

    pub(super) fn check_datasource_context(
        &self,
        actual: &PrincipalContext,
    ) -> Result<(), DatasourceError> {
        if self.context_admitted(actual) {
            Ok(())
        } else {
            Err(datasource_error(
                DatasourceErrorCode::StaleAuthority,
                "owner authority snapshot is not current",
                false,
            ))
        }
    }

    pub(super) fn search_datasources(
        &self,
        context: &PrincipalContext,
        query: &str,
    ) -> Vec<DatasourceSummary> {
        let query = query.to_ascii_lowercase();
        SLACK_DATASOURCES
            .iter()
            .filter(|datasource_ref| {
                !self
                    .datasource_connections(context, datasource_ref)
                    .is_empty()
            })
            .filter_map(|datasource_ref| {
                let summary = datasource_summary(datasource_ref)?;
                (query.is_empty()
                    || datasource_ref.contains(&query)
                    || summary.title.to_ascii_lowercase().contains(&query))
                .then_some(summary)
            })
            .collect()
    }

    pub(super) fn datasource_connections(
        &self,
        context: &PrincipalContext,
        datasource_ref: &str,
    ) -> Vec<StoredConnection> {
        if !SLACK_DATASOURCES.contains(&datasource_ref) {
            return Vec::new();
        }
        lock(&self.metadata)
            .connections
            .iter()
            .filter(|connection| self.connection_is_admitted(connection))
            .filter(|connection| self.connection_owned_by(connection, context))
            .cloned()
            .collect()
    }

    pub(super) fn datasource_description_ref(
        &self,
        context: &PrincipalContext,
        datasource_ref: &str,
    ) -> String {
        let mut digest = Sha256::new();
        digest.update(b"b10x/slack-datasource-description/v2\0");
        digest.update(context.stable_authority_seed());
        digest.update(b"\0");
        digest.update(datasource_ref.as_bytes());
        digest.update(b"\0");
        digest.update(datasource_projection_sha256(datasource_ref).as_bytes());
        for connection in self.datasource_connections(context, datasource_ref) {
            digest.update(b"\0");
            digest.update(connection.connection_ref.as_bytes());
            digest.update(b"\0");
            digest.update(connection.grant_ref.as_bytes());
            digest.update(b"\0");
            digest.update(connection.profile.as_str().as_bytes());
        }
        format!("datasource-description:slack:{:x}", digest.finalize())
    }

    pub(super) fn describe_datasource(
        &self,
        context: &PrincipalContext,
        datasource_ref: &str,
    ) -> Result<DatasourceDescription, DatasourceError> {
        let summary = datasource_summary(datasource_ref).ok_or_else(datasource_not_found)?;
        // A datasource with no Connection bound is not a missing datasource, and connecting Slack
        // fixes it — so it must not answer "was not found", which reads as terminal and as
        // something an operator has to repair.
        if self
            .datasource_connections(context, datasource_ref)
            .is_empty()
        {
            return Err(datasource_binding_not_granted(datasource_ref));
        }
        let (description, key_schema, compact_schema, detail_schema) =
            datasource_declaration(datasource_ref).ok_or_else(datasource_not_found)?;
        Ok(DatasourceDescription {
            summary,
            description: description.to_owned(),
            key_schema,
            compact_schema,
            detail_schema,
            projection_protocol: VALUE_PROJECTION_PROTOCOL.to_owned(),
            projection_sha256: datasource_projection_sha256(datasource_ref),
            description_ref: self.datasource_description_ref(context, datasource_ref),
        })
    }

    pub(super) fn datasource_bindings(
        &self,
        context: &PrincipalContext,
        datasource_ref: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<DatasourceBinding>, DatasourceError> {
        if datasource_summary(datasource_ref).is_none() {
            return Err(datasource_not_found());
        }
        let query = query.to_ascii_lowercase();
        let admitted = self.datasource_connections(context, datasource_ref);
        // The query narrows an already-admitted binding set; it is a hint, never a permission
        // boundary. A model that asks with a topical word ("conversations") used to get an
        // empty list and conclude the datasource was not bound at all, so a query that matches
        // nothing falls back to everything admitted.
        let matched = admitted
            .iter()
            .filter(|connection| {
                query.is_empty()
                    || connection.label.to_ascii_lowercase().contains(&query)
                    || datasource_ref.to_ascii_lowercase().contains(&query)
            })
            .count();
        let mut bindings = admitted
            .into_iter()
            .filter(|connection| {
                matched == 0
                    || query.is_empty()
                    || connection.label.to_ascii_lowercase().contains(&query)
                    || datasource_ref.to_ascii_lowercase().contains(&query)
            })
            .map(|connection| DatasourceBinding {
                datasource_ref: datasource_ref.to_owned(),
                binding_ref: datasource_binding_ref(datasource_ref, &connection),
                label: format!(
                    "{} ({})",
                    connection.label,
                    datasource_scope_label(connection.profile)
                ),
                // Which of several Slack identities to read through is the question a caller
                // actually has here, and a scope label alone does not answer it.
                purpose: (!connection.purpose.is_empty()).then(|| connection.purpose.clone()),
                connection_ref: connection.connection_ref,
                generation: u64::from(STATE_VERSION),
            })
            .collect::<Vec<_>>();
        bindings.truncate(limit);
        Ok(bindings)
    }

    pub(super) async fn read_datasource(
        &self,
        context: &PrincipalContext,
        request: DatasourceReadRequest,
    ) -> Result<DatasourcePage, DatasourceError> {
        if request.description_ref
            != self.datasource_description_ref(context, &request.datasource_ref)
        {
            // One describe recovers it, and the caller is told so. A bare "is stale" reads like
            // something an operator has to repair.
            return Err(datasource_error(
                DatasourceErrorCode::StaleAuthority,
                "the Slack datasource description lease has moved on; describe it again and retry the read",
                true,
            ));
        }
        let admitted = self.datasource_connections(context, &request.datasource_ref);
        // Two different facts used to share one bare "not granted": nothing is bound for this
        // principal, and the binding named is not one of the bindings that exist. The first can
        // clear on its own — the same principal saw one Slack binding and then none minutes
        // later — so it is reported as retriable; the second is the caller's to correct and says
        // exactly how.
        if admitted.is_empty() {
            return Err(datasource_binding_not_granted(&request.datasource_ref));
        }
        let connection = admitted
            .into_iter()
            .find(|connection| {
                datasource_binding_ref(&request.datasource_ref, connection) == request.binding_ref
            })
            .ok_or_else(|| {
                DatasourceError::new(
                    DatasourceErrorCode::InvalidInput,
                    format!(
                        "`{}` is not a binding of `{}`; list its bindings and read through one of those",
                        request.binding_ref, request.datasource_ref
                    ),
                    false,
                )
            })?;
        let credential_name = match connection.profile {
            SlackConnectionProfile::OrgUser | SlackConnectionProfile::Legacy => {
                USER_TOKEN_CREDENTIAL
            }
            SlackConnectionProfile::OrgBot | SlackConnectionProfile::CompanionBot => {
                BOT_TOKEN_CREDENTIAL
            }
        };
        let credential_ref = self
            .operation_credential_ref(&connection, credential_name)
            .map_err(|_| datasource_not_granted())?;
        let credential = self
            .credential_store
            .get(&credential_ref)
            .await
            .map_err(|_| datasource_not_granted())?;
        let (endpoint, params, view, key) =
            datasource_request_plan(&request.datasource_ref, connection.profile, &request.read)?;
        let (payload, connector_audit_ref, observed_at_unix_ms) = self
            .dispatch_datasource_read(
                context,
                &request.datasource_ref,
                &connection,
                endpoint,
                &params,
                &credential,
            )
            .await?;
        drop(credential);
        let (records, next_cursor, completeness) =
            normalize_datasource_response(&request.datasource_ref, view, key.as_deref(), &payload)?;
        let description = self.describe_datasource(context, &request.datasource_ref)?;
        let schema = if view == DatasourceRecordView::Compact {
            &description.compact_schema
        } else {
            &description.detail_schema
        };
        let validator = jsonschema::validator_for(schema).map_err(|_| datasource_unavailable())?;
        if records
            .iter()
            .any(|record| !validator.is_valid(&record.value))
        {
            return Err(datasource_error(
                DatasourceErrorCode::Protocol,
                "Slack datasource projection did not match its declaration",
                false,
            ));
        }
        Ok(DatasourcePage {
            datasource_ref: request.datasource_ref,
            records,
            next_cursor,
            completeness,
            observed_at_unix_ms,
            provenance: DatasourceProvenance {
                binding_ref: request.binding_ref,
                projection_sha256: description.projection_sha256,
                connector_audit_ref,
            },
        })
    }

    pub(super) async fn dispatch_datasource_read(
        &self,
        context: &PrincipalContext,
        datasource_ref: &str,
        connection: &StoredConnection,
        endpoint: &'static str,
        params: &[(String, String)],
        credential: &Secret,
    ) -> Result<(Value, String, u64), DatasourceError> {
        let audit_ref = format!(
            "audit:slack:{}",
            random_uuid().map_err(|_| datasource_unavailable())?
        );
        let audit = AuditEvent {
            audit_ref: &audit_ref,
            operation_ref: datasource_ref,
            connection_ref: &connection.connection_ref,
            tenant_id: context.tenant_id(),
            actor_subject: context.actor_subject(),
            outcome: "attempted",
        };
        self.audit
            .begin(audit)
            .map_err(|_| datasource_unavailable())?;
        let dispatched = async {
            let mut target = url::Url::parse(endpoint).map_err(|_| datasource_unavailable())?;
            target.query_pairs_mut().extend_pairs(
                params
                    .iter()
                    .map(|(name, value)| (name.as_str(), value.as_str())),
            );
            let response = self
                .egress
                .execute(
                    &connection.connection_ref,
                    EgressHttpRequest {
                        request: bearer_request("GET", target.into(), credential),
                        maximum_response_bytes: protocol::datasource::MAX_RESULT_BYTES,
                        response_headers: Vec::new(),
                    },
                )
                .await
                .map_err(|_| datasource_unavailable())?;
            if !response.is_success() {
                return Err(datasource_unavailable());
            }
            let value: Value =
                serde_json::from_slice(&response.body).map_err(|_| datasource_unavailable())?;
            if value.get("ok") != Some(&Value::Bool(true)) {
                return Err(datasource_slack_refusal(&value));
            }
            Ok(value)
        }
        .await;
        match dispatched {
            Ok(value) => {
                self.audit
                    .finish(AuditEvent {
                        outcome: "completed",
                        ..audit
                    })
                    .map_err(|_| datasource_unavailable())?;
                Ok((
                    value,
                    audit_ref,
                    now_ms().ok_or_else(datasource_unavailable)?,
                ))
            }
            Err(error) => {
                self.audit
                    .finish(AuditEvent {
                        outcome: "indeterminate",
                        ..audit
                    })
                    .map_err(|_| datasource_unavailable())?;
                Err(error)
            }
        }
    }

    pub(super) fn operation_connections(
        &self,
        context: &PrincipalContext,
        operation_ref: &str,
    ) -> Vec<OperationConnectionSummary> {
        lock(&self.metadata)
            .connections
            .iter()
            .filter(|connection| self.connection_is_admitted(connection))
            .filter(|connection| self.connection_owned_by(connection, context))
            .filter(|connection| connection_supports_operation(connection, operation_ref))
            .map(|connection| OperationConnectionSummary {
                connection_ref: connection.connection_ref.clone(),
                label: connection.label.clone(),
                provider: INTEGRATION_REF.to_owned(),
                audiences: vec![match connection.profile {
                    SlackConnectionProfile::OrgBot => "organization-read",
                    SlackConnectionProfile::OrgUser => "delegated-user",
                    SlackConnectionProfile::CompanionBot => "companion-bot",
                    SlackConnectionProfile::Legacy => "legacy-reconnect-required",
                }
                .to_owned()],
                purpose: (!connection.purpose.is_empty()).then(|| connection.purpose.clone()),
            })
            .collect()
    }

    pub(super) fn search_operations(
        &self,
        context: &PrincipalContext,
        query: &str,
    ) -> Vec<OperationSummary> {
        let query = query.to_ascii_lowercase();
        SLACK_OPERATIONS
            .iter()
            .filter_map(|operation_ref| {
                let connections = self.operation_connections(context, operation_ref);
                if connections.is_empty() {
                    return None;
                }
                let operation = connector_resolve::document::operation(operation_ref)?;
                let title = operation_title(operation_ref);
                (query.is_empty()
                    || operation_ref.contains(&query)
                    || title.to_ascii_lowercase().contains(&query)
                    || operation
                        .contract_description()
                        .to_ascii_lowercase()
                        .contains(&query))
                .then(|| OperationSummary {
                    operation_ref: (*operation_ref).to_owned(),
                    title: title.to_owned(),
                    effect: operation_effect(operation_ref),
                    approval: operation_approval(operation_ref),
                    connections: connections.clone(),
                })
            })
            .collect()
    }

    pub(super) fn description_ref(
        &self,
        context: &PrincipalContext,
        operation_ref: &str,
    ) -> String {
        let mut digest = Sha256::new();
        digest.update(b"b10x/slack-description/v1\0");
        digest.update(serde_json::to_vec(context).expect("principal context serializes"));
        digest.update(b"\0");
        digest.update(operation_ref.as_bytes());
        digest.update(b"\0");
        digest.update(self.policy.grant_ref.as_bytes());
        for connection in self.operation_connections(context, operation_ref) {
            digest.update(b"\0");
            digest.update(connection.connection_ref.as_bytes());
        }
        format!("description-sha256-{:x}", digest.finalize())
    }

    pub(super) fn describe_operation(
        &self,
        context: &PrincipalContext,
        operation_ref: &str,
    ) -> Result<OperationResult, OperationError> {
        if !is_slack_operation(operation_ref) {
            return Err(operation_not_found());
        }
        let operation = connector_resolve::document::operation(operation_ref)
            .ok_or_else(operation_not_found)?;
        let connections = self.operation_connections(context, operation_ref);
        if connections.is_empty() {
            return Err(operation_not_found());
        }
        Ok(OperationResult::Describe(OperationDescription {
            operation_ref: operation_ref.to_owned(),
            title: operation_title(operation_ref).to_owned(),
            description: operation.contract_description().to_owned(),
            input_schema: operation.input_schema().clone(),
            output_schema: serde_json::json!({"type":"object"}),
            effect: operation_effect(operation_ref),
            approval: operation_approval(operation_ref),
            connections,
            description_ref: self.description_ref(context, operation_ref),
        }))
    }

    pub(super) async fn invoke(
        &self,
        context: &PrincipalContext,
        mut request: InvokeRequest,
    ) -> Result<OperationResult, OperationError> {
        if !is_slack_operation(&request.operation_ref) {
            return Err(operation_not_found());
        }
        let connection = lock(&self.metadata)
            .connections
            .iter()
            .find(|connection| {
                connection.connection_ref == request.connection_ref
                    && self.connection_is_admitted(connection)
                    && self.connection_owned_by(connection, context)
                    && connection_supports_operation(connection, &request.operation_ref)
            })
            .cloned()
            .ok_or_else(operation_not_granted)?;
        if request.description_ref != self.description_ref(context, &request.operation_ref) {
            return Err(OperationError::new(
                OperationErrorCode::StaleAuthority,
                "operation description lease is stale",
                false,
            ));
        }
        let mut event_reply_claim = None;
        if operation_approval(&request.operation_ref) == ApprovalPosture::Required {
            match request.approval_evidence_ref.as_deref() {
                None => {
                    return Err(OperationError::new(
                        OperationErrorCode::ApprovalRequired,
                        "this Slack write requires correlated approval evidence",
                        false,
                    ));
                }
                Some(event_ref) if event_ref.starts_with("event:") => {
                    let event_ref = event_ref.to_owned();
                    self.authorize_companion_event_reply(
                        &connection,
                        &request.operation_ref,
                        &event_ref,
                        &mut request.input,
                    )?;
                    event_reply_claim = Some(event_ref);
                }
                Some(_) => {}
            }
        }
        let operation = connector_resolve::document::operation(&request.operation_ref)
            .ok_or_else(operation_not_found)?;
        let validator = jsonschema::validator_for(operation.input_schema())
            .map_err(|_| operation_unavailable())?;
        if !validator.is_valid(&request.input) {
            return Err(operation_invalid());
        }
        let credential_name = match connection.profile {
            SlackConnectionProfile::OrgUser => USER_TOKEN_CREDENTIAL,
            SlackConnectionProfile::OrgBot | SlackConnectionProfile::CompanionBot => {
                BOT_TOKEN_CREDENTIAL
            }
            SlackConnectionProfile::Legacy => {
                if operation_effect(&request.operation_ref) == EffectClass::ReadOnly {
                    USER_TOKEN_CREDENTIAL
                } else {
                    BOT_TOKEN_CREDENTIAL
                }
            }
        };
        let credential_ref = self
            .operation_credential_ref(&connection, credential_name)
            .map_err(|_| operation_not_granted())?;
        let credential = self
            .credential_store
            .get(&credential_ref)
            .await
            .map_err(|_| operation_not_granted())?;
        let declared_name = if credential_name == USER_TOKEN_CREDENTIAL {
            "slack.user_token"
        } else {
            "slack.bot_token"
        };
        let assembled = connector_resolve::auth::Assembled::new(
            declared_name,
            credential.expose_secret().to_owned(),
            catalog::Placement::Header {
                name: "Authorization",
                prefix: "Bearer ",
            },
        );
        drop(credential);
        let plan = connector_resolve::resolve(
            operation,
            SLACK_ORIGIN,
            &request.input,
            &BTreeMap::new(),
            &[assembled],
        )
        .map_err(|_| operation_invalid())?;
        let target = url::Url::parse(&plan.request.url).map_err(|_| operation_unavailable())?;
        if target.scheme() != "https"
            || target.host_str() != Some("slack.com")
            || target.port_or_known_default() != Some(443)
            || !target.username().is_empty()
            || target.password().is_some()
            || target.fragment().is_some()
        {
            return Err(operation_not_granted());
        }
        let outbound = plan.request;
        if let Some(event_ref) = event_reply_claim {
            self.reply_claims
                .claim(&event_ref, now_ms().ok_or_else(operation_unavailable)?)
                .map_err(|error| match error.code {
                    "reply-already-claimed" => OperationError::new(
                        OperationErrorCode::ApprovalRequired,
                        "the Slack mention has already authorized one reply",
                        false,
                    ),
                    _ => operation_unavailable(),
                })?;
        }
        let audit_ref = format!(
            "audit:slack:{}",
            random_uuid().map_err(|_| operation_unavailable())?
        );
        let audit = AuditEvent {
            audit_ref: &audit_ref,
            operation_ref: &request.operation_ref,
            connection_ref: &request.connection_ref,
            tenant_id: context.tenant_id(),
            actor_subject: context.actor_subject(),
            outcome: "attempted",
        };
        // No request reaches Slack unless the attempted record and capacity for its terminal
        // outcome are durable first.
        self.audit
            .begin(audit)
            .map_err(|_| operation_unavailable())?;
        let dispatched = async {
            let response = self
                .egress
                .execute(
                    &request.connection_ref,
                    EgressHttpRequest {
                        request: outbound,
                        maximum_response_bytes: protocol::operation::MAX_RESULT_BYTES,
                        response_headers: Vec::new(),
                    },
                )
                .await
                .map_err(|error| match error {
                    service::EgressTransportError::ResponseTooLarge => OperationError::new(
                        OperationErrorCode::ResultTooLarge,
                        "Slack operation result exceeds the admitted bound",
                        false,
                    ),
                    service::EgressTransportError::Refused => operation_unavailable(),
                })?;
            if !response.is_success() {
                return Err(operation_unavailable());
            }
            serde_json::from_slice(&response.body).map_err(|_| operation_unavailable())
        }
        .await;
        let output = match dispatched {
            Ok(output) => output,
            Err(error) => {
                self.audit
                    .finish(AuditEvent {
                        outcome: "indeterminate",
                        ..audit
                    })
                    .map_err(|_| post_dispatch_error(&request.operation_ref))?;
                return Err(
                    if operation_effect(&request.operation_ref) == EffectClass::ReadOnly {
                        error
                    } else {
                        post_dispatch_error(&request.operation_ref)
                    },
                );
            }
        };
        self.audit
            .finish(AuditEvent {
                outcome: "completed",
                ..audit
            })
            .map_err(|_| post_dispatch_error(&request.operation_ref))?;
        Ok(OperationResult::Invoke(InvocationResult {
            operation_ref: request.operation_ref,
            output,
            connector_audit_ref: audit_ref,
            execution_ref: None,
        }))
    }
}

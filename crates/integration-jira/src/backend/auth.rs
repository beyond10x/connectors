//! Atlassian OAuth, service credential resolution, and recoverable delegated credential commits.

use connector_secrets::{
    CredentialRef, CredentialScope, Layout, SecretBatch, SecretProposalDigest,
    SecretTransactionGeneration, SecretTransactionId, SecretTransactionState, TenantLayout,
};
use protocol::connection::{ConnectSessionStatus, ConnectionError, ConnectionErrorCode};
use serde::Deserialize;
use serde_json::Value;
use service::{ConnectSessionTerminal, HostedCompletionError, PrincipalContext};
use sha2::{Digest as _, Sha256};
use zeroize::Zeroizing;

use super::*;

#[derive(Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    expires_in: u64,
    scope: String,
    token_type: String,
    #[serde(default)]
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct AccessibleResource {
    id: String,
    url: String,
    scopes: Vec<String>,
}

#[derive(Deserialize)]
struct AtlassianMe {
    account_id: String,
    email: String,
    name: String,
    account_status: String,
}

struct VerifiedUser {
    account_id: String,
    display_name: String,
    scopes: Vec<String>,
}

impl JiraInner {
    pub(super) fn create_session(
        &self,
        owner: &PrincipalContext,
        label: String,
    ) -> Result<ConnectSessionStatus, ConnectionError> {
        self.expire_sessions();
        let email = owner.email().ok_or_else(|| {
            ConnectionError::new(
                ConnectionErrorCode::NotGranted,
                "Jira user connection requires a verified Identity email",
                false,
            )
        })?;
        let id = random_uuid().map_err(|_| connection_unavailable())?;
        let session_ref = format!("connect-session:{id}");
        let expires_at_unix_ms = now_ms()
            .and_then(|now| {
                now.checked_add(
                    self.policy
                        .connect_session_ttl_seconds
                        .saturating_mul(1_000),
                )
            })
            .ok_or_else(connection_unavailable)?;
        let state = random_token(32).map_err(|_| connection_unavailable())?;
        let authorize = self
            .oauth_authorize_url(&state)
            .map_err(|_| connection_unavailable())?;
        let owner = SessionOwner {
            subject: owner.subject().to_owned(),
            email: email.to_owned(),
        };
        let mut browser_url = self.public_origin.clone();
        browser_url
            .path_segments_mut()
            .map_err(|_| connection_unavailable())?
            .push("connect-sessions")
            .push(&session_ref);
        let status = lock(&self.sessions)
            .reserve_browser(
                session_ref.clone(),
                label,
                expires_at_unix_ms,
                browser_url.into(),
            )
            .map_err(|_| connection_unavailable())?;
        lock(&self.hosted_sessions).insert(
            session_ref.clone(),
            HostedSession {
                expires_at_unix_ms,
                oauth_authorize_url: authorize,
            },
        );
        lock(&self.oauth_states).insert(
            state,
            OAuthPending {
                session_ref: session_ref.clone(),
                owner: owner.clone(),
                expires_at_unix_ms,
            },
        );
        lock(&self.session_owners).insert(session_ref, owner);
        Ok(status)
    }

    pub(super) fn expire_sessions(&self) {
        let Some(now) = now_ms() else {
            return;
        };
        let expired = lock(&self.hosted_sessions)
            .iter()
            .filter(|(_, session)| now >= session.expires_at_unix_ms)
            .map(|(session_ref, _)| session_ref.clone())
            .collect::<Vec<_>>();
        for session_ref in expired {
            lock(&self.hosted_sessions).remove(&session_ref);
            lock(&self.session_owners).remove(&session_ref);
            let _ = lock(&self.sessions).finish(&session_ref, ConnectSessionTerminal::Expired);
        }
        lock(&self.oauth_states).retain(|_, pending| now < pending.expires_at_unix_ms);
    }

    fn oauth_authorize_url(&self, state: &str) -> Result<String, JiraError> {
        let mut url = url::Url::parse("https://auth.atlassian.com/authorize")
            .map_err(|_| JiraError::new("oauth-origin"))?;
        url.query_pairs_mut()
            .append_pair("audience", "api.atlassian.com")
            .append_pair("client_id", &self.policy.user_oauth_client_id)
            .append_pair("scope", &USER_SCOPES.join(" "))
            .append_pair("redirect_uri", &self.policy.oauth_redirect_uri)
            .append_pair("state", state)
            .append_pair("response_type", "code")
            .append_pair("prompt", "consent");
        Ok(url.into())
    }

    pub(super) async fn complete_oauth(
        &self,
        state: &str,
        code: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), HostedCompletionError> {
        self.expire_sessions();
        let pending = lock(&self.oauth_states)
            .remove(state)
            .ok_or(HostedCompletionError::NotFound)?;
        lock(&self.hosted_sessions).remove(&pending.session_ref);
        if error.is_some()
            || code.is_none()
            || now_ms().is_none_or(|now| now >= pending.expires_at_unix_ms)
        {
            let _ =
                lock(&self.sessions).finish(&pending.session_ref, ConnectSessionTerminal::Failed);
            return Err(HostedCompletionError::Refused);
        }
        let _completion = self.completion_lock.lock().await;
        let outcome = async {
            let credentials = self
                .exchange_oauth_code(code.expect("checked code"))
                .await?;
            let evidence = self
                .verify_user_token(&credentials.access_token, &pending.owner.email)
                .await?;
            self.commit_connection(&pending.session_ref, pending.owner, evidence, credentials)
                .await
        }
        .await;
        match outcome {
            Ok(connection_ref) => lock(&self.sessions)
                .finish(
                    &pending.session_ref,
                    ConnectSessionTerminal::Completed { connection_ref },
                )
                .map_err(|_| HostedCompletionError::Unavailable),
            Err(error) => {
                let _ = lock(&self.sessions)
                    .finish(&pending.session_ref, ConnectSessionTerminal::Failed);
                Err(hosted_error(error))
            }
        }
    }

    async fn exchange_oauth_code(&self, code: &str) -> Result<CredentialValues, JiraError> {
        let client_secret = self
            .credential_store
            .get(&self.fixed_credential_ref(USER_CLIENT_SECRET_CREDENTIAL)?)
            .await
            .map_err(|_| JiraError::new("oauth-config"))?;
        let response = self
            .http
            .post("https://auth.atlassian.com/oauth/token")
            .json(&serde_json::json!({
                "grant_type": "authorization_code",
                "client_id": self.policy.user_oauth_client_id,
                "client_secret": client_secret.expose_secret(),
                "code": code,
                "redirect_uri": self.policy.oauth_redirect_uri,
            }))
            .send()
            .await
            .map_err(|_| JiraError::new("oauth-exchange"))?;
        drop(client_secret);
        let value: OAuthTokenResponse = decode_response(response, 64 * 1024).await?;
        credentials_from_token(value, true)
    }

    async fn verify_user_token(
        &self,
        token: &Secret,
        expected_email: &str,
    ) -> Result<VerifiedUser, JiraError> {
        let resources: Vec<AccessibleResource> = self
            .bearer_json(
                "https://api.atlassian.com/oauth/token/accessible-resources",
                token,
            )
            .await?;
        let resource = resources
            .into_iter()
            .find(|resource| {
                resource.id == self.policy.cloud_id
                    && resource.url.trim_end_matches('/')
                        == self.site_origin.as_str().trim_end_matches('/')
            })
            .ok_or_else(|| JiraError::new("oauth-site"))?;
        let scopes = canonical_scopes(resource.scopes);
        if !["read:jira-work", "write:jira-work"]
            .iter()
            .all(|required| scopes.iter().any(|scope| scope == required))
        {
            return Err(JiraError::new("oauth-scopes"));
        }
        let me: AtlassianMe = self
            .bearer_json("https://api.atlassian.com/me", token)
            .await?;
        if me.account_status != "active"
            || me.account_id.is_empty()
            || me.account_id.len() > 256
            || normalize_email(&me.email) != normalize_email(expected_email)
        {
            return Err(JiraError::new("oauth-subject"));
        }
        Ok(VerifiedUser {
            account_id: me.account_id,
            display_name: bounded_string(&me.name, 256),
            scopes,
        })
    }

    async fn bearer_json<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        token: &Secret,
    ) -> Result<T, JiraError> {
        let response = self
            .http
            .get(url)
            .bearer_auth(token.expose_secret())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|_| JiraError::new("provider-unavailable"))?;
        decode_response(response, MAX_PROVIDER_RESPONSE_BYTES).await
    }

    async fn commit_connection(
        &self,
        session_ref: &str,
        owner: SessionOwner,
        evidence: VerifiedUser,
        credentials: CredentialValues,
    ) -> Result<String, JiraError> {
        let label = lock(&self.sessions)
            .pending_label(session_ref)
            .map_err(|_| JiraError::new("connect-session"))?;
        let existing = lock(&self.metadata)
            .connections
            .iter()
            .find(|connection| connection.owner_subject == owner.subject)
            .cloned();
        let (instance_id, connection_ref) = existing.map_or_else(
            || {
                random_uuid().map(|id| {
                    let reference = format!("connection:jira:{id}");
                    (id, reference)
                })
            },
            |connection| Ok((connection.instance_id, connection.connection_ref)),
        )?;
        let (transaction, generation) = self.reserve_transaction()?;
        let connection = StoredConnection {
            connection_ref: connection_ref.clone(),
            instance_id,
            label,
            grant_ref: self.policy.user_grant_ref.clone(),
            owner_subject: owner.subject,
            account_id: evidence.account_id,
            display_name: evidence.display_name,
            email_sha256: email_sha256(&owner.email),
            scopes: evidence.scopes,
            credential_generation: u64::from_be_bytes(generation.protocol_bytes()),
            observed_at_unix_ms: now_ms().ok_or_else(|| JiraError::new("clock"))?,
            expires_at_unix_ms: credentials.expires_at_unix_ms,
        };
        self.commit_credentials(transaction, generation, connection, credentials)
            .await?;
        Ok(connection_ref)
    }

    async fn commit_credentials(
        &self,
        transaction: SecretTransactionId,
        generation: SecretTransactionGeneration,
        connection: StoredConnection,
        credentials: CredentialValues,
    ) -> Result<(), JiraError> {
        let mut batch = SecretBatch::new(
            CredentialScope::new(&self.tenant_id, AUTHORITY)
                .map_err(|_| JiraError::new("credential-address"))?,
        );
        batch
            .put(
                self.connection_credential_ref(&connection, ACCESS_TOKEN_CREDENTIAL)?,
                credentials.access_token,
            )
            .map_err(|_| JiraError::new("credential-batch"))?;
        batch
            .put(
                self.connection_credential_ref(&connection, REFRESH_TOKEN_CREDENTIAL)?,
                credentials.refresh_token,
            )
            .map_err(|_| JiraError::new("credential-batch"))?;
        self.credential_store
            .prepare(transaction, proposal_digest(&batch), &batch)
            .await
            .map_err(|_| JiraError::new("credential-prepare"))?;
        let transaction_id = hex::encode(transaction.protocol_bytes());
        let persisted = {
            let mut state = lock(&self.metadata);
            state.pending.push(PendingCommit {
                transaction_id: transaction_id.clone(),
                connection: connection.clone(),
            });
            let result = self.persist(&state);
            if result.is_err() {
                state
                    .pending
                    .retain(|pending| pending.transaction_id != transaction_id);
            }
            result
        };
        if let Err(error) = persisted {
            let _ = self.credential_store.abort(transaction).await;
            return Err(error);
        }
        self.credential_store
            .commit(transaction)
            .await
            .map_err(|_| JiraError::new("credential-commit"))?;
        {
            let mut state = lock(&self.metadata);
            state
                .pending
                .retain(|pending| pending.transaction_id != transaction_id);
            upsert_connection(&mut state.connections, connection);
            self.persist(&state)?;
        }
        let _ = self.credential_store.reclaim(generation).await;
        Ok(())
    }

    fn reserve_transaction(
        &self,
    ) -> Result<(SecretTransactionId, SecretTransactionGeneration), JiraError> {
        let mut state = lock(&self.metadata);
        let generation = SecretTransactionGeneration::from_protocol_bytes(
            state.next_transaction_generation.to_be_bytes(),
        )
        .ok_or_else(|| JiraError::new("transaction-generation"))?;
        state.next_transaction_generation = state
            .next_transaction_generation
            .checked_add(1)
            .ok_or_else(|| JiraError::new("transaction-generation"))?;
        self.persist(&state)?;
        let mut nonce = [0_u8; 24];
        getrandom::fill(&mut nonce).map_err(|_| JiraError::new("randomness"))?;
        Ok((SecretTransactionId::new(generation, nonce), generation))
    }

    pub(super) async fn recover_pending(&self) -> Result<(), JiraError> {
        let pending_commits = { lock(&self.metadata).pending.clone() };
        for pending in pending_commits {
            let transaction = decode_transaction(&pending.transaction_id)?;
            match self
                .credential_store
                .state(transaction)
                .await
                .map_err(|_| JiraError::new("credential-recovery"))?
            {
                SecretTransactionState::Prepared => {
                    self.credential_store
                        .commit(transaction)
                        .await
                        .map_err(|_| JiraError::new("credential-recovery"))?;
                }
                SecretTransactionState::Committed => {}
                SecretTransactionState::Absent => {
                    let mut state = lock(&self.metadata);
                    state
                        .pending
                        .retain(|candidate| candidate.transaction_id != pending.transaction_id);
                    self.persist(&state)?;
                    continue;
                }
            }
            let mut state = lock(&self.metadata);
            state
                .pending
                .retain(|candidate| candidate.transaction_id != pending.transaction_id);
            upsert_connection(&mut state.connections, pending.connection);
            self.persist(&state)?;
        }
        Ok(())
    }

    pub(super) fn connection_credential_ref(
        &self,
        connection: &StoredConnection,
        credential: &str,
    ) -> Result<CredentialRef, JiraError> {
        CredentialRef::for_instance(
            &self.tenant_id,
            AUTHORITY,
            &connection.instance_id,
            SERVICE,
            credential,
        )
        .map_err(|_| JiraError::new("credential-address"))
    }

    fn fixed_credential_ref(&self, credential: &str) -> Result<CredentialRef, JiraError> {
        CredentialRef::new(&self.tenant_id, AUTHORITY, LOGIN_SERVICE, credential)
            .map_err(|_| JiraError::new("credential-address"))
    }

    pub(super) async fn user_access_token(
        &self,
        connection: &StoredConnection,
    ) -> Result<Secret, JiraError> {
        if connection.grant_ref != self.policy.user_grant_ref {
            return Err(JiraError::new("connection-grant"));
        }
        if now_ms().is_none_or(|now| {
            connection.expires_at_unix_ms
                <= now.saturating_add(self.policy.refresh_skew_seconds.saturating_mul(1_000))
        }) {
            self.refresh_user_oauth(&connection.connection_ref).await?;
        }
        let current = lock(&self.metadata)
            .connections
            .iter()
            .find(|candidate| {
                candidate.connection_ref == connection.connection_ref
                    && candidate.grant_ref == self.policy.user_grant_ref
            })
            .cloned()
            .ok_or_else(|| JiraError::new("connection-state"))?;
        self.credential_store
            .get(&self.connection_credential_ref(&current, ACCESS_TOKEN_CREDENTIAL)?)
            .await
            .map_err(|_| JiraError::new("credential-resolve"))
    }

    async fn refresh_user_oauth(&self, connection_ref: &str) -> Result<(), JiraError> {
        let _refresh = self.refresh_lock.lock().await;
        let connection = lock(&self.metadata)
            .connections
            .iter()
            .find(|connection| {
                connection.connection_ref == connection_ref
                    && connection.grant_ref == self.policy.user_grant_ref
            })
            .cloned()
            .ok_or_else(|| JiraError::new("connection-state"))?;
        if now_ms().is_some_and(|now| {
            connection.expires_at_unix_ms
                > now.saturating_add(self.policy.refresh_skew_seconds.saturating_mul(1_000))
        }) {
            return Ok(());
        }
        let refresh = self
            .credential_store
            .get(&self.connection_credential_ref(&connection, REFRESH_TOKEN_CREDENTIAL)?)
            .await
            .map_err(|_| JiraError::new("credential-resolve"))?;
        let client_secret = self
            .credential_store
            .get(&self.fixed_credential_ref(USER_CLIENT_SECRET_CREDENTIAL)?)
            .await
            .map_err(|_| JiraError::new("oauth-config"))?;
        let response = self
            .http
            .post("https://auth.atlassian.com/oauth/token")
            .json(&serde_json::json!({
                "grant_type": "refresh_token",
                "client_id": self.policy.user_oauth_client_id,
                "client_secret": client_secret.expose_secret(),
                "refresh_token": refresh.expose_secret(),
            }))
            .send()
            .await
            .map_err(|_| JiraError::new("oauth-refresh"))?;
        drop((refresh, client_secret));
        let value: OAuthTokenResponse = decode_response(response, 64 * 1024).await?;
        let credentials = credentials_from_token(value, true)?;
        let (resources, me) = self
            .verify_refresh_subject(&credentials.access_token, &connection)
            .await?;
        let (transaction, generation) = self.reserve_transaction()?;
        let mut updated = connection;
        updated.scopes = resources;
        updated.display_name = bounded_string(&me.name, 256);
        updated.observed_at_unix_ms = now_ms().ok_or_else(|| JiraError::new("clock"))?;
        updated.expires_at_unix_ms = credentials.expires_at_unix_ms;
        updated.credential_generation = u64::from_be_bytes(generation.protocol_bytes());
        self.commit_credentials(transaction, generation, updated, credentials)
            .await
    }

    async fn verify_refresh_subject(
        &self,
        token: &Secret,
        connection: &StoredConnection,
    ) -> Result<(Vec<String>, AtlassianMe), JiraError> {
        let resources: Vec<AccessibleResource> = self
            .bearer_json(
                "https://api.atlassian.com/oauth/token/accessible-resources",
                token,
            )
            .await?;
        let resource = resources
            .into_iter()
            .find(|resource| {
                resource.id == self.policy.cloud_id
                    && resource.url.trim_end_matches('/')
                        == self.site_origin.as_str().trim_end_matches('/')
            })
            .ok_or_else(|| JiraError::new("oauth-site"))?;
        let scopes = canonical_scopes(resource.scopes);
        if !["read:jira-work", "write:jira-work"]
            .iter()
            .all(|required| scopes.iter().any(|scope| scope == required))
        {
            return Err(JiraError::new("oauth-scopes"));
        }
        let me: AtlassianMe = self
            .bearer_json("https://api.atlassian.com/me", token)
            .await?;
        if me.account_status != "active"
            || me.account_id != connection.account_id
            || email_sha256(&me.email) != connection.email_sha256
        {
            return Err(JiraError::new("oauth-subject"));
        }
        Ok((scopes, me))
    }

    pub(super) async fn service_access_token(&self) -> Result<Secret, JiraError> {
        let result = async {
            match self.policy.shared_auth {
                JiraSharedAuth::ServiceApiToken => self
                    .credential_store
                    .get(&self.fixed_credential_ref(SERVICE_API_TOKEN_CREDENTIAL)?)
                    .await
                    .map_err(|_| JiraError::new("service-credential")),
                JiraSharedAuth::ServiceOauth => {
                    if let Some(token) = self.cached_service_token().await {
                        return Ok(token);
                    }
                    let secret = self
                        .credential_store
                        .get(&self.fixed_credential_ref(SERVICE_CLIENT_SECRET_CREDENTIAL)?)
                        .await
                        .map_err(|_| JiraError::new("service-credential"))?;
                    let client_id = self
                        .policy
                        .service_oauth_client_id
                        .as_deref()
                        .ok_or_else(|| JiraError::new("service-oauth-config"))?;
                    let response = self
                        .http
                        .post("https://auth.atlassian.com/oauth/token")
                        .form(&[
                            ("client_id", client_id),
                            ("client_secret", secret.expose_secret()),
                            ("grant_type", "client_credentials"),
                        ])
                        .send()
                        .await
                        .map_err(|_| JiraError::new("service-oauth"))?;
                    drop(secret);
                    let value: OAuthTokenResponse = decode_response(response, 64 * 1024).await?;
                    let scopes = parse_scopes(&value.scope);
                    if value.token_type != "Bearer"
                        || value.access_token.is_empty()
                        || value.access_token.len() > 4_096
                        || value.expires_in == 0
                        || !scopes.iter().any(|scope| scope == "read:jira-work")
                    {
                        return Err(JiraError::new("service-oauth-evidence"));
                    }
                    let expires_at_unix_ms = now_ms()
                        .and_then(|now| now.checked_add(value.expires_in.saturating_mul(1_000)))
                        .ok_or_else(|| JiraError::new("clock"))?;
                    let token = Secret::new(value.access_token);
                    let returned = Secret::new(token.expose_secret());
                    *self.service_token.lock().await = Some(CachedServiceToken {
                        token,
                        expires_at_unix_ms,
                    });
                    Ok(returned)
                }
            }
        }
        .await;
        if result.is_err() {
            self.service_callable.store(false, Ordering::Release);
        }
        result
    }

    async fn cached_service_token(&self) -> Option<Secret> {
        let cache = self.service_token.lock().await;
        let cached = cache.as_ref()?;
        let now = now_ms()?;
        (cached.expires_at_unix_ms
            > now.saturating_add(self.policy.refresh_skew_seconds.saturating_mul(1_000)))
        .then(|| Secret::new(cached.token.expose_secret()))
    }
}

fn credentials_from_token(
    value: OAuthTokenResponse,
    require_refresh: bool,
) -> Result<CredentialValues, JiraError> {
    let scopes = parse_scopes(&value.scope);
    let refresh = value.refresh_token.unwrap_or_default();
    if value.token_type != "Bearer"
        || value.access_token.is_empty()
        || value.access_token.len() > 4_096
        || value.expires_in == 0
        || require_refresh && (refresh.is_empty() || refresh.len() > 4_096)
        || !["read:jira-work", "write:jira-work"]
            .iter()
            .all(|required| scopes.iter().any(|scope| scope == required))
    {
        return Err(JiraError::new("oauth-exchange"));
    }
    let expires_at_unix_ms = now_ms()
        .and_then(|now| now.checked_add(value.expires_in.saturating_mul(1_000)))
        .ok_or_else(|| JiraError::new("clock"))?;
    Ok(CredentialValues {
        access_token: Secret::new(value.access_token),
        refresh_token: Secret::new(refresh),
        expires_at_unix_ms,
    })
}

pub(super) async fn decode_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    maximum: usize,
) -> Result<T, JiraError> {
    let value = decode_value_response(response, maximum).await?;
    serde_json::from_value(value).map_err(|_| JiraError::new("provider-response"))
}

pub(super) async fn decode_value_response(
    mut response: reqwest::Response,
    maximum: usize,
) -> Result<Value, JiraError> {
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|length| length > maximum as u64)
    {
        return Err(JiraError::new("provider-refused"));
    }
    if response.status() == reqwest::StatusCode::NO_CONTENT {
        return Ok(Value::Null);
    }
    let mut bytes = Zeroizing::new(Vec::with_capacity(
        response.content_length().unwrap_or(0).min(maximum as u64) as usize,
    ));
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| JiraError::new("provider-response"))?
    {
        if bytes.len().saturating_add(chunk.len()) > maximum {
            return Err(JiraError::new("provider-response-bound"));
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.is_empty() {
        Ok(Value::Null)
    } else {
        serde_json::from_slice(&bytes).map_err(|_| JiraError::new("provider-response"))
    }
}

fn canonical_scopes(mut scopes: Vec<String>) -> Vec<String> {
    scopes.sort();
    scopes.dedup();
    scopes
}

fn parse_scopes(value: &str) -> Vec<String> {
    canonical_scopes(value.split_whitespace().map(str::to_owned).collect())
}

fn normalize_email(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn upsert_connection(connections: &mut Vec<StoredConnection>, connection: StoredConnection) {
    connections.retain(|candidate| candidate.connection_ref != connection.connection_ref);
    connections.push(connection);
    connections.sort_by(|left, right| left.connection_ref.cmp(&right.connection_ref));
}

fn proposal_digest(batch: &SecretBatch) -> SecretProposalDigest {
    let mut digest = Sha256::new();
    digest.update(b"b10x/jira-credential-transaction/v1\0");
    for (reference, secret) in batch
        .put_entries()
        .expect("Jira credential transactions contain puts only")
    {
        digest.update(TenantLayout.render(reference).as_bytes());
        digest.update(b"\0");
        digest.update(secret.expose_secret().as_bytes());
        digest.update(b"\0");
    }
    SecretProposalDigest::from_protocol_bytes(digest.finalize().into())
}

fn decode_transaction(encoded: &str) -> Result<SecretTransactionId, JiraError> {
    let bytes = hex::decode(encoded).map_err(|_| JiraError::new("transaction-state"))?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| JiraError::new("transaction-state"))?;
    SecretTransactionId::from_protocol_bytes(bytes)
        .ok_or_else(|| JiraError::new("transaction-state"))
}

fn hosted_error(error: JiraError) -> HostedCompletionError {
    match error.code {
        "oauth-site" | "oauth-scopes" | "oauth-subject" | "oauth-exchange" => {
            HostedCompletionError::Refused
        }
        _ => HostedCompletionError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth_scopes_are_canonical_and_exact() {
        assert_eq!(
            parse_scopes("write:jira-work read:jira-work read:jira-work"),
            vec!["read:jira-work", "write:jira-work"]
        );
    }
}

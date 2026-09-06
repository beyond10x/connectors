//! Bounded acquisition and refresh; state and authority remain with the parent owner.

use super::*;

impl OAuthInner {
    async fn bounded<T>(
        &self,
        remaining: Duration,
        future: impl std::future::Future<Output = Result<T>>,
    ) -> Result<T> {
        let mut stopping = self.stopping.subscribe();
        if *stopping.borrow() || remaining.is_zero() {
            return Err(PersonalOAuthError::Unavailable);
        }
        tokio::select! {
            biased;
            _ = stopping.changed() => Err(PersonalOAuthError::Unavailable),
            result = tokio::time::timeout(remaining, future) => result.map_err(|_| PersonalOAuthError::Expired)?,
        }
    }

    async fn request(
        &self,
        binding: &OAuthBinding,
        method: &str,
        url: &str,
        body: Option<String>,
        bearer: Option<&str>,
        remaining: Duration,
    ) -> Result<(u16, Zeroizing<Vec<u8>>)> {
        let mut headers = BTreeMap::new();
        if body.is_some() {
            headers.insert(
                "content-type".into(),
                "application/x-www-form-urlencoded".into(),
            );
        }
        if let Some(bearer) = bearer {
            headers.insert("authorization".into(), format!("Bearer {bearer}"));
        }
        self.bounded(remaining, async {
            let response = self
                .egress
                .execute(
                    &binding.custody.identity.connection,
                    EgressHttpRequest {
                        request: connector_resolve::Request {
                            method: method.to_owned(),
                            url: url.to_owned(),
                            headers,
                            body,
                        },
                        maximum_response_bytes: MAX_RESPONSE,
                        response_headers: Vec::new(),
                    },
                )
                .await
                .map_err(|_| PersonalOAuthError::Unavailable)?;
            let bytes = Zeroizing::new(response.body);
            if bytes.len() > MAX_RESPONSE {
                return Err(PersonalOAuthError::Refused);
            }
            Ok((response.status, bytes))
        })
        .await
    }

    async fn evidence(
        &self,
        binding: &OAuthBinding,
        token: &ValidatedToken,
        token_received_at: u64,
        remaining: Duration,
    ) -> Result<Evidence> {
        let (status, bytes) = self
            .request(
                binding,
                "GET",
                &binding.policy.evidence_url,
                None,
                Some(&token.access_token),
                remaining,
            )
            .await?;
        let value = json_response(status, &bytes)?;
        let now = self.now()?;
        observed_evidence(
            &binding.policy,
            &value,
            now,
            token
                .expires_at_unix_ms(token_received_at)
                .map_err(|_| PersonalOAuthError::Refused)?,
        )
    }

    pub(super) async fn refresh(
        &self,
        binding: &OAuthBinding,
        previous: &Publication,
        ticket: RefreshCompletion,
    ) -> Result<Publication> {
        let refresh = self
            .store
            .get(&binding.refresh_address)
            .await
            .map_err(|_| PersonalOAuthError::Refused)?;
        self.begin_marker(binding, previous.generation)?;
        let remaining = ticket
            .remaining()
            .map_err(|_| PersonalOAuthError::Expired)?;
        let mut fields = vec![
            ("grant_type", "refresh_token"),
            ("client_id", &binding.policy.registration.client_id),
            ("refresh_token", refresh.expose_secret()),
        ];
        if let Some(redirect) = binding.policy.registration.redirect_uri.as_deref() {
            fields.push(("redirect_uri", redirect));
        }
        let body = form(&fields);
        let requested_at = self.now()?;
        let (status, bytes) = self
            .request(
                binding,
                "POST",
                &binding.policy.token_url,
                Some(body),
                None,
                remaining,
            )
            .await?;
        let now = self.now()?;
        let token = token_response(status, &bytes, &binding.policy, now)?;
        let evidence = self
            .evidence(
                binding,
                &token,
                requested_at,
                ticket
                    .remaining()
                    .map_err(|_| PersonalOAuthError::Expired)?,
            )
            .await?;
        let proposal = Proposal::new(
            &binding.custody,
            Secret::new(token.access_token.to_string()),
            token
                .refresh_token
                .as_ref()
                .map(|value| Secret::new(value.to_string())),
        )
        .map_err(|_| PersonalOAuthError::Refused)?;
        // Only pre-claim work consumes the authorization window. FULL decision/commit may finish
        // after it, and an uncertain write stays guarded by the kernel's recovery claim.
        let publication = self
            .custody
            .complete_refresh(&binding.custody, evidence, proposal, ticket)
            .await
            .map_err(|_| PersonalOAuthError::Unavailable)?;
        self.synchronize(binding)?;
        if self.reconcile_marker(binding)? {
            return Err(PersonalOAuthError::Unavailable);
        }
        Ok(publication)
    }
}

impl OAuthInner {
    pub(super) fn select_binding(
        &self,
        request: &connection_api::ConnectSessionCreateRequest,
    ) -> Result<usize> {
        let candidates: Vec<_> =
            self.bindings
                .iter()
                .enumerate()
                .filter(|(_, binding)| {
                    binding.policy.provider.id == request.integration_ref
                        && request.auth_profile.as_deref().is_none_or(|profile| {
                            profile == binding.policy.registration.auth_profile
                        })
                })
                .map(|(index, _)| index)
                .collect();
        if let [index] = candidates.as_slice() {
            Ok(*index)
        } else {
            Err(PersonalOAuthError::Refused)
        }
    }

    pub(super) async fn create(
        self: &Arc<Self>,
        request: connection_api::ConnectSessionCreateRequest,
    ) -> Result<connection_api::ConnectSessionStatus> {
        if !self.persistent || *self.stopping.borrow() {
            return Err(PersonalOAuthError::Unavailable);
        }
        let index = self.select_binding(&request)?;
        self.create_for_binding(index, request.label, None).await
    }

    pub(super) async fn create_for_binding(
        self: &Arc<Self>,
        index: usize,
        label: String,
        remediation: Option<Arc<Mutex<remediation::BoundSession>>>,
    ) -> Result<connection_api::ConnectSessionStatus> {
        if !self.persistent || *self.stopping.borrow() {
            return Err(PersonalOAuthError::Unavailable);
        }
        if label.trim().is_empty() || label.len() > 256 || label.chars().any(char::is_control) {
            return Err(PersonalOAuthError::Invalid);
        }
        self.custody
            .recover()
            .await
            .map_err(|_| PersonalOAuthError::Unavailable)?;
        let binding = &self.bindings[index];
        let _guard = binding.gate.lock().await;
        if let Some(bound) = &remediation {
            remediation::recheck_bound(bound, self.now()?)?;
            let operation_ref = remediation::bound_operation(bound)?;
            let operation = catalog::operation(catalog::OperationKey::id(&operation_ref))
                .ok_or(PersonalOAuthError::Refused)?;
            let need = match self.readiness_locked(binding, operation).await {
                service::CredentialReadiness::MissingCredential => {
                    protocol::operation::v3::AuthenticationNeed::AuthorizeConfigured
                }
                service::CredentialReadiness::CredentialDegraded => {
                    protocol::operation::v3::AuthenticationNeed::ReauthorizeExisting
                }
                service::CredentialReadiness::DependencyUnavailable => {
                    return Err(PersonalOAuthError::Unavailable)
                }
                _ => return Err(PersonalOAuthError::Refused),
            };
            if need != remediation::bound_need(bound)? {
                return Err(PersonalOAuthError::Refused);
            }
        }
        let previous = self
            .synchronize(binding)?
            .as_ref()
            .map_or(0, |publication| publication.generation);
        if !lock(&binding.authority)?.active {
            return Err(PersonalOAuthError::Refused);
        }
        {
            let mut sessions = lock(&self.sessions)?;
            // Terminal handles carry no private material. Keep a bounded recent result window.
            if sessions.len() >= MAX_SESSIONS {
                let retire = sessions
                    .iter()
                    .find(|(_, session)| session.task.as_ref().is_some_and(JoinHandle::is_finished))
                    .map(|(reference, _)| reference.clone());
                if let Some(reference) = retire {
                    sessions.remove(&reference);
                }
            }
            if sessions.len() >= MAX_SESSIONS
                || sessions.values().any(|session| {
                    session.binding == index
                        && session.task.as_ref().is_none_or(|task| !task.is_finished())
                })
            {
                return Err(PersonalOAuthError::Unavailable);
            }
        }
        let mut deadline = AcquisitionDeadline::new(
            self.clock.as_ref(),
            binding.policy.registration.session_ttl_seconds,
        )?;
        if let Some(bound) = &remediation {
            deadline.cap(remediation::bound_deadline(bound)?)?;
        }
        let (endpoint, device) = match binding.policy.registration.flow {
            PersonalOAuthFlow::AuthorizationCodePkce => {
                let scope = binding
                    .policy
                    .ceiling
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" ");
                let endpoint = BoundOAuthEndpoint::bind_pkce(PkceEndpointConfig {
                    redirect_uri: binding
                        .policy
                        .registration
                        .redirect_uri
                        .as_deref()
                        .ok_or(PersonalOAuthError::Invalid)?,
                    authorization_origin: &binding.policy.origin,
                    authorization_path: &binding.policy.authorize_path,
                    client_id: &binding.policy.registration.client_id,
                    scope: &scope,
                    deadline: Instant::from_std(deadline.deadline),
                })
                .map_err(transport_error)?;
                (endpoint, None)
            }
            PersonalOAuthFlow::DeviceAuthorization => {
                let body = form(&[
                    ("client_id", &binding.policy.registration.client_id),
                    (
                        "scope",
                        &binding
                            .policy
                            .ceiling
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(" "),
                    ),
                ]);
                let (status, bytes) = self
                    .request(
                        binding,
                        "POST",
                        binding
                            .policy
                            .device_url
                            .as_deref()
                            .ok_or(PersonalOAuthError::Invalid)?,
                        Some(body),
                        None,
                        deadline.remaining(self.clock.as_ref())?,
                    )
                    .await?;
                if !(200..300).contains(&status) {
                    return Err(PersonalOAuthError::Refused);
                }
                let raw: PrivateDevice =
                    serde_json::from_slice(&bytes).map_err(|_| PersonalOAuthError::Refused)?;
                let now = self.now()?;
                let authorization = connector_oauth::device::validate_device(
                    DeviceResponse {
                        device_code: raw.device_code,
                        user_code: raw.user_code.to_string(),
                        verification_uri: raw.verification_uri.to_string(),
                        verification_uri_complete: raw
                            .verification_uri_complete
                            .as_ref()
                            .map(|value| value.to_string()),
                        expires_in: raw.expires_in,
                        interval: raw.interval,
                    },
                    &binding.policy.origin,
                    now,
                    deadline.expires,
                )
                .map_err(|_| PersonalOAuthError::Refused)?;
                deadline.cap(authorization.deadline_unix_ms())?;
                let endpoint = BoundOAuthEndpoint::bind_device(
                    &authorization,
                    now,
                    Instant::from_std(deadline.deadline),
                )
                .map_err(transport_error)?;
                (endpoint, Some(authorization))
            }
        };
        deadline.remaining(self.clock.as_ref())?;
        let reference = format!(
            "connect-session:oauth:{}",
            connector_oauth::random_token(24).map_err(|_| PersonalOAuthError::Unavailable)?
        );
        let clock = self.clock.clone();
        let mut lifecycle =
            ConnectSessionLifecycle::with_clock(binding.policy.provider.id, 1, move || {
                clock
                    .now()
                    .map(|(now, _)| now)
                    .map_err(|_| service::ConnectSessionLifecycleError::Invalid)
            })
            .map_err(|_| PersonalOAuthError::Unavailable)?;
        let status = lifecycle
            .reserve_browser(
                reference.clone(),
                label,
                deadline.expires,
                endpoint.browser_url().to_string(),
            )
            .map_err(|_| PersonalOAuthError::Invalid)?;
        lifecycle
            .bind_completion_target(&reference, &binding.custody.identity.connection)
            .map_err(|_| PersonalOAuthError::Unavailable)?;
        let lifecycle = Arc::new(Mutex::new(lifecycle));
        let liveness = endpoint.liveness();
        let mut sessions = lock(&self.sessions)?;
        if sessions.len() >= MAX_SESSIONS {
            return Err(PersonalOAuthError::Unavailable);
        }
        let owned = self.clone();
        let task_reference = reference.clone();
        let task_lifecycle = lifecycle.clone();
        let task_remediation = remediation.clone();
        let task = tokio::spawn(async move {
            let result = owned
                .acquire(
                    SessionTarget {
                        binding: index,
                        remediation: task_remediation,
                        previous,
                        reference: task_reference.clone(),
                        lifecycle: task_lifecycle.clone(),
                    },
                    endpoint,
                    device,
                    deadline,
                )
                .await;
            if let Err(error) = result {
                if let Ok(mut lifecycle) = task_lifecycle.lock() {
                    // Guarded Preparing/Committing/RecoveryRequired refuses this ordinary terminal
                    // transition. Only known pre-custody failures are resolved inside acquire.
                    let _ = lifecycle.finish(
                        &task_reference,
                        if error == PersonalOAuthError::Expired {
                            ConnectSessionTerminal::Expired
                        } else {
                            ConnectSessionTerminal::Failed
                        },
                    );
                }
            }
        });
        sessions.insert(
            reference,
            Session {
                binding: index,
                lifecycle,
                liveness,
                task: Some(task),
                remediation,
            },
        );
        Ok(status)
    }

    async fn acquire(
        &self,
        target: SessionTarget,
        endpoint: BoundOAuthEndpoint,
        device: Option<DeviceAuthorization>,
        deadline: AcquisitionDeadline,
    ) -> Result<()> {
        let binding = &self.bindings[target.binding];
        // PKCE awaits the human outside the gate; generation is captured now and rechecked after
        // obtaining it. Device polling holds the same gate for its bounded acquisition lifetime.
        if let Some(device) = device {
            let _guard = binding.gate.lock().await;
            self.recheck_session(binding, target.previous, &deadline)?;
            let token_result = {
                let instructions = endpoint.receive();
                tokio::pin!(instructions);
                tokio::select! {
                    result = &mut instructions => return Err(result.err().map(transport_error).unwrap_or(PersonalOAuthError::Refused)),
                    token = self.poll_device(binding, device, &deadline) => token,
                }
            }; // Drop the instruction receiver before preparing publication.
            let (token, received_at) = token_result?;
            self.publish_session(binding, &target, token, received_at, &deadline)
                .await
        } else {
            let callback = self
                .bounded(deadline.remaining(self.clock.as_ref())?, async {
                    endpoint.receive().await.map_err(transport_error)
                })
                .await?;
            let _guard = binding.gate.lock().await;
            self.recheck_session(binding, target.previous, &deadline)?;
            let body = form(&[
                ("grant_type", "authorization_code"),
                ("client_id", &binding.policy.registration.client_id),
                ("code", &callback.code),
                ("code_verifier", &callback.verifier),
                ("redirect_uri", &callback.redirect_uri),
            ]);
            let requested_at = self.now()?;
            let (status, bytes) = self
                .request(
                    binding,
                    "POST",
                    &binding.policy.token_url,
                    Some(body),
                    None,
                    deadline.remaining(self.clock.as_ref())?,
                )
                .await?;
            let received_at = self.now()?;
            let token = token_response(status, &bytes, &binding.policy, received_at)?;
            self.publish_session(binding, &target, token, requested_at, &deadline)
                .await
        }
    }

    fn recheck_session(
        &self,
        binding: &OAuthBinding,
        previous: u64,
        deadline: &AcquisitionDeadline,
    ) -> Result<()> {
        deadline.remaining(self.clock.as_ref())?;
        let publication = self.synchronize(binding)?;
        let current = lock(&binding.authority)?;
        if !current.active
            || publication
                .as_ref()
                .map_or(0, |publication| publication.generation)
                != previous
        {
            return Err(PersonalOAuthError::Refused);
        }
        Ok(())
    }

    async fn publish_session(
        &self,
        binding: &OAuthBinding,
        target: &SessionTarget,
        token: ValidatedToken,
        received_at: u64,
        deadline: &AcquisitionDeadline,
    ) -> Result<()> {
        let reference = target.reference.as_str();
        let lifecycle = &target.lifecycle;
        let previous = target.previous;
        let connection = &binding.custody.identity.connection;
        let (live, cleanup) =
            LiveCompletion::begin(lifecycle.clone(), reference.to_owned(), connection.clone())
                .map_err(|_| PersonalOAuthError::Unavailable)?;
        if !cleanup.is_empty() {
            return Err(PersonalOAuthError::Unavailable);
        }
        let prepared = async {
            self.recheck_session(binding, previous, deadline)?;
            let evidence = self
                .evidence(
                    binding,
                    &token,
                    received_at,
                    deadline.remaining(self.clock.as_ref())?,
                )
                .await?;
            let proposal = Proposal::new(
                &binding.custody,
                Secret::new(token.access_token.to_string()),
                token
                    .refresh_token
                    .as_ref()
                    .map(|value| Secret::new(value.to_string())),
            )
            .map_err(|_| PersonalOAuthError::Refused)?;
            Ok::<_, PersonalOAuthError>((evidence, proposal))
        }
        .await;
        let (evidence, proposal) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                // Custody has not started; no prepare/decision/store I/O is uncertain here.
                lock(lifecycle)?
                    .resolve_completion(reference, connection, false)
                    .map_err(|_| PersonalOAuthError::Unavailable)?;
                return Err(error);
            }
        };
        self.custody
            .complete(
                &binding.custody,
                previous,
                evidence,
                proposal,
                live,
                &remediation::BoundCompletionAuthority {
                    ordinary: SessionAuthority(binding.authority.clone()),
                    bound: target.remediation.as_ref(),
                },
            )
            .await
            .map_err(|_| PersonalOAuthError::Unavailable)?;
        self.synchronize(binding)?;
        if self.reconcile_marker(binding)? {
            return Err(PersonalOAuthError::Unavailable);
        }
        Ok(())
    }

    async fn poll_device(
        &self,
        binding: &OAuthBinding,
        authorization: DeviceAuthorization,
        deadline: &AcquisitionDeadline,
    ) -> Result<(ValidatedToken, u64)> {
        let mut poll = DevicePoll::new(authorization);
        loop {
            let now = self.now()?;
            match poll.status(now) {
                PollStatus::WaitUntil(at) => {
                    let wait = Duration::from_millis(at.saturating_sub(now));
                    self.bounded(deadline.remaining(self.clock.as_ref())?, async {
                        tokio::time::sleep(wait).await;
                        Ok(())
                    })
                    .await?;
                    continue;
                }
                PollStatus::Ready => {}
                PollStatus::Expired => return Err(PersonalOAuthError::Expired),
                _ => return Err(PersonalOAuthError::Refused),
            }
            let code = poll
                .begin_poll(now)
                .map_err(|_| PersonalOAuthError::Refused)?;
            let body = form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", &binding.policy.registration.client_id),
                ("device_code", code),
            ]);
            let response = self
                .request(
                    binding,
                    "POST",
                    &binding.policy.token_url,
                    Some(body),
                    None,
                    deadline
                        .remaining(self.clock.as_ref())?
                        .min(Duration::from_secs(10)),
                )
                .await;
            let (status, bytes) = match response {
                Err(PersonalOAuthError::Expired) => {
                    poll.finish_poll(PollOutcome::TransportTimeout, self.now()?);
                    continue;
                }
                Err(error) => return Err(error),
                Ok(response) => response,
            };
            let received_at = self.now()?;
            if (200..300).contains(&status) {
                let token = token_response(status, &bytes, &binding.policy, now)?;
                if poll.finish_poll(PollOutcome::Authorized, received_at) != PollStatus::Authorized
                {
                    return Err(PersonalOAuthError::Expired);
                }
                return Ok((token, now));
            }
            #[derive(Deserialize)]
            struct PollError {
                error: String,
            }
            let error: PollError =
                serde_json::from_slice(&bytes).map_err(|_| PersonalOAuthError::Refused)?;
            let outcome = match error.error.as_str() {
                "authorization_pending" => PollOutcome::Pending,
                "slow_down" => PollOutcome::SlowDown,
                "access_denied" => PollOutcome::AccessDenied,
                "expired_token" => PollOutcome::ExpiredToken,
                _ => PollOutcome::Refused,
            };
            poll.finish_poll(outcome, received_at);
        }
    }
}

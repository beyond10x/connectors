//! Exact personal admission and bound sessions on the existing OAuth/custody owners.
use super::*;
use protocol::connection_v2::{
    BoundRemediationStatus, RemediationAcknowledgement, RemediationNextAction,
    RemediationResumeState,
};
use protocol::operation::v3::AuthenticationNeed;
use service::{
    CredentialReadiness, RemediationAdmission, RemediationAuthority, RemediationBinding,
    RemediationError, RemediationMetadata, RemediationRequest, RemediationResult, RemediationRoute,
    RemediationTarget,
};

type RemediationResultOf<T> = std::result::Result<T, RemediationError>;

pub(super) struct BoundSession {
    pub(super) binding: RemediationBinding,
    context: PrincipalContext,
    authority: Arc<dyn RemediationAuthority>,
    acknowledged: bool,
}

fn closed(error: PersonalOAuthError) -> RemediationError {
    match error {
        PersonalOAuthError::Refused => RemediationError::Refused,
        PersonalOAuthError::Invalid => RemediationError::InvalidInput,
        PersonalOAuthError::PortInUse => RemediationError::Conflict,
        _ => RemediationError::Unavailable,
    }
}

impl OAuthInner {
    fn admit_remediation_target(
        &self,
        context: &PrincipalContext,
        target: RemediationTarget<'_>,
    ) -> RemediationResultOf<usize> {
        if context.stable_authority_seed() != self.owner.stable_authority_seed()
            || *self.stopping.borrow()
        {
            return Err(RemediationError::Refused);
        }
        let operation = catalog::operation(catalog::OperationKey::id(target.operation_ref))
            .ok_or(RemediationError::Refused)?;
        let index = self
            .bindings
            .iter()
            .position(|binding| binding.custody.identity.connection == target.connection_ref)
            .ok_or(RemediationError::Refused)?;
        let binding = &self.bindings[index];
        let raw = binding
            .delegate
            .inner
            .bindings
            .first()
            .ok_or(RemediationError::Unavailable)?;
        if operation.provider != binding.policy.provider.id
            || !raw.admits(operation)
            || !raw.initiation.allows(domain::ConnectionInitiator::Platform)
            || !scopes_admit(
                operation,
                &binding.policy.registration.auth_profile,
                &binding.policy.ceiling,
                &binding.policy.ceiling,
            )
            || !lock(&binding.authority).map_err(closed)?.active
        {
            return Err(RemediationError::Refused);
        }
        Ok(index)
    }

    fn remediation_policy_digest(
        &self,
        index: usize,
        target: RemediationTarget<'_>,
    ) -> RemediationResultOf<String> {
        let binding = &self.bindings[index];
        let operation =
            catalog::reader::operation(target.operation_ref).ok_or(RemediationError::Refused)?;
        Ok(hex::encode(field_digest(&[
            "bound-remediation-policy/v1",
            &serde_json::to_string(&binding.policy.configured)
                .map_err(|_| RemediationError::Unavailable)?,
            &hex::encode(binding.policy.authority_digest),
            catalog::reader::embedded().digest(),
            operation.record(),
            target.connection_ref,
            binding.policy.provider.id,
            &binding.policy.registration.auth_profile,
            &hex::encode(binding.policy.client_digest),
            &hex::encode(binding.policy.origin_digest),
        ])))
    }

    pub(super) async fn readiness_locked(
        &self,
        binding: &OAuthBinding,
        operation: &catalog::Operation,
    ) -> CredentialReadiness {
        let publication = match self.coherent(binding) {
            Ok(Some(publication)) => publication,
            Ok(None) => return CredentialReadiness::MissingCredential,
            Err(_) => return CredentialReadiness::DependencyUnavailable,
        };
        if !lock(&binding.authority)
            .is_ok_and(|current| current.operation(operation, &publication.evidence))
        {
            return CredentialReadiness::Unsupported;
        }
        let Ok(now) = self.now() else {
            return CredentialReadiness::DependencyUnavailable;
        };
        if now < publication.evidence.observed_at {
            return CredentialReadiness::DependencyUnavailable;
        }
        let access = binding.policy.provider.authority.and_then(|authority| {
            super::super::credential_leaf(
                binding.policy.provider,
                Some(&binding.policy.registration.auth_profile),
            )
            .ok()
            .and_then(|leaf| {
                super::super::credential_address(
                    self.owner.tenant_id(),
                    authority,
                    &binding.policy.configured,
                    leaf,
                )
                .ok()
            })
        });
        let Some(access) = access else {
            return CredentialReadiness::DependencyUnavailable;
        };
        match self.store.exists(&access).await {
            Ok(false) => return CredentialReadiness::CredentialDegraded,
            Err(_) => return CredentialReadiness::DependencyUnavailable,
            Ok(true) => {}
        }
        if now < publication.evidence.expires_at {
            return CredentialReadiness::Ready;
        }
        match self.store.exists(&binding.refresh_address).await {
            Ok(true) => CredentialReadiness::Ready,
            Ok(false) => CredentialReadiness::CredentialDegraded,
            Err(_) => CredentialReadiness::DependencyUnavailable,
        }
    }
}

struct PersonalAuthority {
    inner: std::sync::Weak<OAuthInner>,
    index: usize,
    expires_at: u64,
}
impl PersonalAuthority {
    fn current_admission(
        &self,
        context: &PrincipalContext,
        binding: &RemediationBinding,
    ) -> RemediationResultOf<()> {
        let inner = self.inner.upgrade().ok_or(RemediationError::Unavailable)?;
        let target = RemediationTarget {
            operation_ref: &binding.operation_ref,
            connection_ref: &binding.connection_ref,
        };
        let index = inner.admit_remediation_target(context, target)?;
        let policy = &inner.bindings[index].policy;
        if index != self.index
            || binding.integration_ref != policy.provider.id
            || binding.auth_profile != policy.registration.auth_profile
            || binding.grant_ref != policy.configured.grant_ref
            || binding.grant_revision.is_some()
            || binding.stable_authority_sha256
                != hex::encode(Sha256::digest(context.stable_authority_seed()))
            || binding.admission_policy_sha256 != inner.remediation_policy_digest(index, target)?
            || binding.expires_at_unix_ms > self.expires_at
            || binding.canonical_input_sha256.len() != 64
            || !binding
                .canonical_input_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(RemediationError::Refused);
        }
        Ok(())
    }
}

impl RemediationAuthority for PersonalAuthority {
    fn recheck(
        &self,
        context: &PrincipalContext,
        binding: &RemediationBinding,
        now: u64,
    ) -> RemediationResultOf<()> {
        self.current_admission(context, binding)?;
        if now >= binding.expires_at_unix_ms {
            return Err(RemediationError::Refused);
        }
        Ok(())
    }
    fn recheck_status(
        &self,
        context: &PrincipalContext,
        binding: &RemediationBinding,
        _: u64,
    ) -> RemediationResultOf<()> {
        // Session expiry is projected only after the same actual current policy check.
        self.current_admission(context, binding)
    }
}

impl PersonalOAuthBackend {
    pub(super) fn owns_bound_remediation(&self, route: RemediationRoute<'_>) -> bool {
        match route {
            RemediationRoute::Target(target) => self.inner.bindings.iter().any(|binding| {
                binding.custody.identity.connection == target.connection_ref
                    && binding.delegate.owns_operation_ref(target.operation_ref)
            }),
            RemediationRoute::Session(reference) => {
                self.inner.sessions.lock().is_ok_and(|sessions| {
                    sessions
                        .get(reference)
                        .is_some_and(|session| session.remediation.is_some())
                })
            }
        }
    }
    pub(super) fn bound_metadata(
        &self,
        context: &PrincipalContext,
        target: RemediationTarget<'_>,
    ) -> RemediationResultOf<RemediationMetadata<'_>> {
        let index = self.inner.admit_remediation_target(context, target)?;
        let binding = &self.inner.bindings[index];
        let raw = binding
            .delegate
            .inner
            .bindings
            .first()
            .ok_or(RemediationError::Unavailable)?;
        Ok(RemediationMetadata {
            operation: catalog::reader::operation(target.operation_ref)
                .ok_or(RemediationError::Refused)?,
            connection: domain::ConnectionAuthority::new(
                &binding.custody.identity.connection,
                raw.initiation.clone(),
            )
            .map_err(|_| RemediationError::Unavailable)?,
            integration_ref: binding.policy.provider.id.into(),
            auth_profile: binding.policy.registration.auth_profile.clone(),
            catalog_generation: catalog::reader::embedded().digest().into(),
        })
    }
    pub(super) fn bound_admission(
        &self,
        context: &PrincipalContext,
        target: RemediationTarget<'_>,
    ) -> RemediationResultOf<RemediationAdmission> {
        let index = self.inner.admit_remediation_target(context, target)?;
        let policy = &self.inner.bindings[index].policy;
        let expires = self
            .inner
            .now()
            .map_err(closed)?
            .checked_add(
                policy
                    .registration
                    .session_ttl_seconds
                    .checked_mul(1000)
                    .ok_or(RemediationError::Unavailable)?,
            )
            .ok_or(RemediationError::Unavailable)?;
        Ok(RemediationAdmission {
            grant_ref: policy.configured.grant_ref.clone(),
            grant_revision: None,
            admission_policy_sha256: self.inner.remediation_policy_digest(index, target)?,
            expires_at_unix_ms: expires,
            authority: Arc::new(PersonalAuthority {
                inner: Arc::downgrade(&self.inner),
                index,
                expires_at: expires,
            }),
        })
    }
    pub(super) async fn bound_readiness(
        &self,
        context: &PrincipalContext,
        target: RemediationTarget<'_>,
    ) -> CredentialReadiness {
        let Ok(index) = self.inner.admit_remediation_target(context, target) else {
            return CredentialReadiness::Unsupported;
        };
        let Some(operation) = catalog::operation(catalog::OperationKey::id(target.operation_ref))
        else {
            return CredentialReadiness::Unsupported;
        };
        let binding = &self.inner.bindings[index];
        let _guard = binding.gate.lock().await;
        self.inner.readiness_locked(binding, operation).await
    }
    pub(super) async fn bound_request(
        &self,
        context: &PrincipalContext,
        request: RemediationRequest,
        authority: Arc<dyn RemediationAuthority>,
    ) -> RemediationResultOf<RemediationResult> {
        match request {
            RemediationRequest::Start(binding) => {
                let index = self.inner.admit_remediation_target(
                    context,
                    RemediationTarget {
                        operation_ref: &binding.operation_ref,
                        connection_ref: &binding.connection_ref,
                    },
                )?;
                authority.recheck(context, &binding, self.inner.now().map_err(closed)?)?;
                let bound = Arc::new(Mutex::new(BoundSession {
                    binding: *binding,
                    context: context.clone(),
                    authority,
                    acknowledged: false,
                }));
                let status = self
                    .inner
                    .create_for_binding(index, "Authentication remediation".into(), Some(bound))
                    .await
                    .map_err(closed)?;
                self.bound_status(context, &status.connect_session_ref, false, None)
                    .await
            }
            RemediationRequest::Status(request) => {
                self.bound_status(context, &request.connect_session_ref, false, None)
                    .await
            }
            RemediationRequest::Acknowledge(request) => {
                self.bound_status(
                    context,
                    &request.connect_session_ref,
                    true,
                    Some((&request.operation_ref, &request.connection_ref)),
                )
                .await
            }
        }
    }

    async fn bound_status(
        &self,
        context: &PrincipalContext,
        reference: &str,
        acknowledge: bool,
        target: Option<(&str, &str)>,
    ) -> RemediationResultOf<RemediationResult> {
        let bound = {
            let sessions = lock(&self.inner.sessions).map_err(closed)?;
            sessions
                .get(reference)
                .and_then(|session| session.remediation.clone())
                .ok_or(RemediationError::Refused)?
        };
        {
            let state = lock(&bound).map_err(closed)?;
            state.authority.recheck_status(
                context,
                &state.binding,
                self.inner.now().map_err(closed)?,
            )?;
        }
        // Pending status uses the existing non-blocking lifecycle/liveness owner. It must not
        // wait behind a device poll holding the refresh gate.
        let session = self.inner.session_status(reference).await.map_err(closed)?;
        let (operation, connection) = {
            let state = lock(&bound).map_err(closed)?;
            state.authority.recheck_status(
                context,
                &state.binding,
                self.inner.now().map_err(closed)?,
            )?;
            (
                state.binding.operation_ref.clone(),
                state.binding.connection_ref.clone(),
            )
        };
        let index = self.inner.admit_remediation_target(
            context,
            RemediationTarget {
                operation_ref: &operation,
                connection_ref: &connection,
            },
        )?;
        let binding = &self.inner.bindings[index];
        let _gate = if session.state == connection_api::ConnectSessionState::Completed {
            Some(binding.gate.lock().await)
        } else {
            None
        };
        let readiness = if session.state == connection_api::ConnectSessionState::Completed {
            let operation = catalog::operation(catalog::OperationKey::id(&operation))
                .ok_or(RemediationError::Refused)?;
            self.inner.readiness_locked(binding, operation).await
        } else {
            CredentialReadiness::Unsupported
        };
        let mut state = lock(&bound).map_err(closed)?;
        state.authority.recheck_status(
            context,
            &state.binding,
            self.inner.now().map_err(closed)?,
        )?;
        let expired = self.inner.now().map_err(closed)? >= state.binding.expires_at_unix_ms;
        if acknowledge && expired {
            return Err(RemediationError::Conflict);
        }
        let resume = match session.state {
            connection_api::ConnectSessionState::Completed if expired => {
                RemediationResumeState::Expired
            }
            // The lifecycle/receiver must retire its endpoint before an Expired DTO exists.
            connection_api::ConnectSessionState::Pending if expired => {
                return Err(RemediationError::Unavailable)
            }
            connection_api::ConnectSessionState::Pending => RemediationResumeState::Pending,
            connection_api::ConnectSessionState::Completed if state.acknowledged => {
                RemediationResumeState::Consumed
            }
            connection_api::ConnectSessionState::Completed
                if readiness == CredentialReadiness::Ready
                    && session.connection_ref.as_deref() == Some(connection.as_str()) =>
            {
                RemediationResumeState::Ready
            }
            connection_api::ConnectSessionState::Completed => {
                return Err(RemediationError::Unavailable)
            }
            connection_api::ConnectSessionState::Expired => RemediationResumeState::Expired,
            connection_api::ConnectSessionState::Failed => RemediationResumeState::Failed,
        };
        if acknowledge {
            if resume != RemediationResumeState::Ready
                || target != Some((operation.as_str(), connection.as_str()))
            {
                return Err(RemediationError::Conflict);
            }
            state.acknowledged = true;
            return Ok(RemediationResult::Acknowledged(
                RemediationAcknowledgement {
                    connect_session_ref: reference.into(),
                    operation_ref: operation,
                    connection_ref: connection,
                    next_action: RemediationNextAction::FreshDescriptionThenExplicitInvoke,
                },
            ));
        }
        Ok(RemediationResult::Status(Box::new(
            BoundRemediationStatus {
                connect_session_ref: reference.into(),
                operation_ref: operation,
                connection_ref: connection,
                integration_ref: state.binding.integration_ref.clone(),
                auth_profile: state.binding.auth_profile.clone(),
                need: state.binding.need,
                session_state: session.state,
                expires_at_unix_ms: session.expires_at_unix_ms,
                resume_state: resume,
                session,
            },
        )))
    }
}

pub(super) fn recheck_bound(bound: &Arc<Mutex<BoundSession>>, now: u64) -> Result<()> {
    let state = lock(bound)?;
    state
        .authority
        .recheck(&state.context, &state.binding, now)
        .map_err(|_| PersonalOAuthError::Refused)
}
pub(super) fn bound_need(bound: &Arc<Mutex<BoundSession>>) -> Result<AuthenticationNeed> {
    Ok(lock(bound)?.binding.need)
}
pub(super) fn bound_operation(bound: &Arc<Mutex<BoundSession>>) -> Result<String> {
    Ok(lock(bound)?.binding.operation_ref.clone())
}
pub(super) fn bound_deadline(bound: &Arc<Mutex<BoundSession>>) -> Result<u64> {
    Ok(lock(bound)?.binding.expires_at_unix_ms)
}

pub(super) struct BoundCompletionAuthority<'a> {
    pub(super) ordinary: SessionAuthority,
    pub(super) bound: Option<&'a Arc<Mutex<BoundSession>>>,
}
impl CompletionAuthority for BoundCompletionAuthority<'_> {
    fn recheck(&self, identity: &Identity, previous: u64, evidence: &Evidence, now: u64) -> bool {
        self.ordinary.recheck(identity, previous, evidence, now)
            && self
                .bound
                .is_none_or(|bound| recheck_bound(bound, now).is_ok())
    }
}

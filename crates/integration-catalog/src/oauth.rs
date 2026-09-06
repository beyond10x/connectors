//! Personal acquisition, observed authority and dispatch through one binding gate and store.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use catalog::{Acquisition, OAuthRefreshPolicy, OAuthScopeEncoding};
use connect_session_transport::oauth::{
    BoundOAuthEndpoint, OAuthEndpointLiveness, PkceEndpointConfig,
};
use connector_oauth::device::{
    DeviceAuthorization, DevicePoll, DeviceResponse, PollOutcome, PollStatus,
};
use connector_oauth::{TokenPolicy, TokenResponse, ValidatedToken};
use connector_secrets::{CredentialRef, FileStore, Secret, SecretStore};
use connector_state::StateStore;
use connectors_config::{CatalogIntegrationConfig, PersonalOAuthFlow, PersonalOAuthRegistration};
use protocol::{connection as connection_api, operation as operation_api};
use serde::{Deserialize, Serialize};
use service::{
    ConnectSessionLifecycle, ConnectSessionTerminal, ConnectorBackend, EgressHttpRequest,
    EgressTransport, PrincipalContext,
};
use sha2::{Digest as _, Sha256};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use zeroize::Zeroizing;

use super::custody::{
    self, CompletionAuthority, CustodyOwner, Evidence, FullJournal, Identity, LiveCompletion,
    Proposal, Publication, RefreshAuthority, RefreshClock, RefreshCompletion, RefreshOwner,
    SystemRefreshClock,
};
use super::CatalogBackend;

const MAX_RESPONSE: usize = 64 * 1024;
const MAX_SESSIONS: usize = 256;
const MAX_MARKER: usize = 1024;

/// Closed refusals never include provider response bodies, registration values or instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PersonalOAuthError {
    #[error("personal OAuth configuration is unsupported or invalid")]
    Invalid,
    #[error("personal OAuth custody or acquisition is unavailable")]
    Unavailable,
    #[error("personal OAuth authorization was refused")]
    Refused,
    #[error("personal OAuth authorization expired")]
    Expired,
    #[error("personal OAuth callback port is already in use")]
    PortInUse,
}
type Result<T> = std::result::Result<T, PersonalOAuthError>;

/// A configured personal OAuth owner; no session is created merely by composition or invocation.
pub struct PersonalOAuthBackend {
    inner: Arc<OAuthInner>,
}

struct OAuthInner {
    owner: PrincipalContext,
    bindings: Vec<Arc<OAuthBinding>>,
    custody: CustodyOwner,
    store: Arc<FileStore>,
    markers: Arc<dyn StateStore>,
    egress: Arc<dyn EgressTransport>,
    clock: Arc<dyn RefreshClock>,
    persistent: bool,
    stopping: watch::Sender<bool>,
    sessions: Mutex<BTreeMap<String, Session>>,
    marker_blocks: Mutex<BTreeMap<String, u64>>,
}

struct OAuthBinding {
    custody: custody::Binding,
    refresh_address: CredentialRef,
    policy: Policy,
    delegate: CatalogBackend,
    gate: Arc<RefreshOwner>,
    authority: Arc<Mutex<CurrentAuthority>>,
}

struct Policy {
    configured: CatalogIntegrationConfig,
    provider: &'static catalog::Provider,
    admission: &'static catalog::PersonalOAuthAdmission,
    registration: PersonalOAuthRegistration,
    origin: String,
    token_url: String,
    evidence_url: String,
    authorize_path: String,
    device_url: Option<String>,
    client_digest: [u8; 32],
    origin_digest: [u8; 32],
    authority_digest: [u8; 32],
    ceiling: BTreeSet<String>,
}

struct CurrentAuthority {
    identity: Identity,
    active: bool,
    generation: u64,
    subject: Option<String>,
    provider: &'static catalog::Provider,
    purpose: String,
    allow_writes: bool,
    client_digest: [u8; 32],
    origin_digest: [u8; 32],
    authority_digest: [u8; 32],
    ceiling: BTreeSet<String>,
}

impl CurrentAuthority {
    fn evidence_matches(&self, evidence: &Evidence) -> bool {
        self.active
            && evidence.client_digest == self.client_digest
            && evidence.origin_digest == self.origin_digest
            && evidence.authority_digest == self.authority_digest
            && evidence.ceiling == self.ceiling
            && self
                .subject
                .as_ref()
                .is_none_or(|subject| subject == &evidence.subject)
    }

    fn operation(&self, operation: &catalog::Operation, evidence: &Evidence) -> bool {
        operation.provider == self.provider.id
            && self.evidence_matches(evidence)
            && (self.allow_writes
                || super::effect_class(operation) == operation_api::EffectClass::ReadOnly
                    && operation.effects.iter().all(|effect| {
                        matches!(
                            effect,
                            catalog::HostEffect::Read | catalog::HostEffect::Network
                        )
                    }))
            && scopes_admit(operation, &self.purpose, &evidence.scopes, &self.ceiling)
    }
}

struct SessionAuthority(Arc<Mutex<CurrentAuthority>>);
impl CompletionAuthority for SessionAuthority {
    fn recheck(
        &self,
        identity: &Identity,
        previous_generation: u64,
        evidence: &Evidence,
        now: u64,
    ) -> bool {
        self.0.lock().is_ok_and(|current| {
            current.identity == *identity
                && current.generation == previous_generation
                && current.evidence_matches(evidence)
                && evidence.observed_at <= now
                && now < evidence.expires_at
        })
    }
}

struct OperationAuthority(Arc<Mutex<CurrentAuthority>>);
impl RefreshAuthority for OperationAuthority {
    fn recheck(
        &mut self,
        operation: &str,
        identity: &Identity,
        generation: u64,
        evidence: &Evidence,
        _now: u64,
    ) -> bool {
        self.0.lock().is_ok_and(|current| {
            current.identity == *identity
                && current.generation == generation
                && catalog::operation(catalog::OperationKey::id(operation))
                    .is_some_and(|op| current.operation(op, evidence))
        })
    }
}

struct Session {
    binding: usize,
    lifecycle: Arc<Mutex<ConnectSessionLifecycle>>,
    liveness: OAuthEndpointLiveness,
    task: Option<JoinHandle<()>>,
    remediation: Option<Arc<Mutex<remediation::BoundSession>>>,
}

struct SessionTarget {
    binding: usize,
    previous: u64,
    reference: String,
    lifecycle: Arc<Mutex<ConnectSessionLifecycle>>,
    remediation: Option<Arc<Mutex<remediation::BoundSession>>>,
}

impl Policy {
    fn admit(owner: &PrincipalContext, entry: &CatalogIntegrationConfig) -> Result<Self> {
        let registration = entry.oauth.clone().ok_or(PersonalOAuthError::Invalid)?;
        registration
            .validate()
            .map_err(|_| PersonalOAuthError::Invalid)?;
        if entry.credential.as_deref() != Some(registration.auth_profile.as_str())
            || entry.credential_file.is_some()
            || !entry.usernames.is_empty()
        {
            return Err(PersonalOAuthError::Invalid);
        }
        let provider = catalog::provider(catalog::ProviderKey::id(&entry.provider))
            .ok_or(PersonalOAuthError::Invalid)?;
        // This initial runtime slice is the explicitly reviewed GitLab public registration.
        if provider.id != "gitlab" {
            return Err(PersonalOAuthError::Invalid);
        }
        let credential = provider
            .auth
            .iter()
            .find(|credential| credential.name == registration.auth_profile)
            .ok_or(PersonalOAuthError::Invalid)?;
        let Acquisition::OAuth2(oauth) = credential.acquire else {
            return Err(PersonalOAuthError::Invalid);
        };
        let flow = match registration.flow {
            PersonalOAuthFlow::AuthorizationCodePkce => {
                catalog::PersonalOAuthFlow::AuthorizationCodePkce
            }
            PersonalOAuthFlow::DeviceAuthorization => {
                catalog::PersonalOAuthFlow::DeviceAuthorization
            }
        };
        let mut admissions = oauth
            .personal_flows
            .iter()
            .filter(|admission| admission.flow == flow);
        let admission = admissions.next().ok_or(PersonalOAuthError::Invalid)?;
        if admissions.next().is_some()
            || admission.client_authentication != catalog::OAuthClientAuthentication::Public
            || admission.registration_use != catalog::OAuthRegistrationUse::DevelopmentOnly
            || flow == catalog::PersonalOAuthFlow::AuthorizationCodePkce
                && admission.redirect_shape != Some(catalog::OAuthRedirectShape::LoopbackIpv4Http)
            || admission.token_evidence.subject_pointer.is_none()
            || admission.token_evidence.client_id_pointer.is_none()
        {
            return Err(PersonalOAuthError::Invalid);
        }
        let declared = super::declared_config(entry);
        let origin = connector_resolve::resolve_endpoint(
            "personal-oauth",
            provider,
            oauth.endpoint,
            owner.tenant_id(),
            "origin",
            &declared,
        )
        .map_err(|_| PersonalOAuthError::Invalid)?;
        let mut configured = entry.clone();
        configured.endpoints.insert("origin".into(), origin.clone());
        let endpoint_url = |service: &str, path: &str| -> Result<String> {
            let service = provider
                .services
                .iter()
                .find(|candidate| candidate.name == service)
                .ok_or(PersonalOAuthError::Invalid)?;
            let base = service.base_url.replace("{origin}", &origin);
            // The reviewed personal flow has one authority origin for all authentication egress.
            if base != origin || !path.starts_with('/') || path.starts_with("//") {
                return Err(PersonalOAuthError::Invalid);
            }
            Ok(format!("{origin}{path}"))
        };
        let token_service = if oauth.token_endpoint.is_empty() {
            oauth.endpoint
        } else {
            oauth.token_endpoint
        };
        let token_url = endpoint_url(token_service, oauth.token_path)?;
        let evidence_url = endpoint_url(
            &admission.token_evidence.endpoint.service,
            &admission.token_evidence.endpoint.path,
        )?;
        let device_url = admission
            .device_authorization_endpoint
            .as_ref()
            .map(|endpoint| endpoint_url(&endpoint.service, &endpoint.path))
            .transpose()?;
        let client_digest = field_digest(&[
            &registration.client_id,
            &registration.auth_profile,
            registration.redirect_uri.as_deref().unwrap_or(""),
            match registration.flow {
                PersonalOAuthFlow::AuthorizationCodePkce => "authorization_code_pkce",
                PersonalOAuthFlow::DeviceAuthorization => "device_authorization",
            },
        ]);
        let origin_digest = field_digest(&[&origin]);
        let authority_digest = field_digest(&[
            owner.tenant_id(),
            owner.subject(),
            owner.actor_subject(),
            &owner
                .agent_revision()
                .map(|revision| revision.get().to_string())
                .unwrap_or_default(),
            owner.authority_snapshot_id(),
            owner.authority_snapshot_sha256(),
            &entry.grant_ref,
            if entry.allow_writes { "write" } else { "read" },
            &serde_json::to_string(&entry.initiation).map_err(|_| PersonalOAuthError::Invalid)?,
        ]);
        let ceiling = registration.allowed_scopes.iter().cloned().collect();
        Ok(Self {
            configured,
            provider,
            admission,
            registration,
            origin,
            token_url,
            evidence_url,
            authorize_path: oauth.authorize_path.to_owned(),
            device_url,
            client_digest,
            origin_digest,
            authority_digest,
            ceiling,
        })
    }
}

/// Resolve only already declared, operator-admitted origins. This creates no store or session.
pub fn personal_oauth_admitted_origins(
    owner: &PrincipalContext,
    entry: &CatalogIntegrationConfig,
) -> Result<Vec<String>> {
    let policy = Policy::admit(owner, entry)?;
    super::admitted_origins(&policy.configured).map_err(|_| PersonalOAuthError::Invalid)
}

/// Derive the already admitted configured identity for local response correlation only.
/// This allocates no binding/session and does not grant permission to invoke or select a target.
pub fn personal_oauth_admitted_connection_ref(
    owner: &PrincipalContext,
    entry: &CatalogIntegrationConfig,
) -> Result<String> {
    let policy = Policy::admit(owner, entry)?;
    Ok(super::connection_ref(
        &policy.configured.provider,
        policy.configured.instance(),
    ))
}

impl PersonalOAuthBackend {
    /// Open one dedicated unsealed DevelopmentFile owner, recover its FULL journal, and bind its
    /// same store to acquisition and invocation. Persistent controls whether sessions may start.
    pub async fn open(
        owner: PrincipalContext,
        configured: &[CatalogIntegrationConfig],
        state_root: &Path,
        egress: Arc<dyn EgressTransport>,
        persistent: bool,
    ) -> Result<Self> {
        Self::open_with_clock(
            owner,
            configured,
            state_root,
            egress,
            persistent,
            Arc::new(SystemRefreshClock),
        )
        .await
    }

    async fn open_with_clock(
        owner: PrincipalContext,
        configured: &[CatalogIntegrationConfig],
        state_root: &Path,
        egress: Arc<dyn EgressTransport>,
        persistent: bool,
        clock: Arc<dyn RefreshClock>,
    ) -> Result<Self> {
        let policies = configured
            .iter()
            .map(|entry| Policy::admit(&owner, entry))
            .collect::<Result<Vec<_>>>()?;
        if policies.is_empty() {
            return Err(PersonalOAuthError::Invalid);
        }
        let directory = state_root.join("oauth");
        super::ensure_owner_directory(&directory).map_err(|_| PersonalOAuthError::Unavailable)?;
        let store = Arc::new(
            FileStore::open(directory.join("credentials.store"))
                .map_err(|_| PersonalOAuthError::Unavailable)?,
        );
        prepare_journal_path(&directory)?;
        let journal = FullJournal::open(&directory.join("transactions.sqlite"))
            .map_err(|_| PersonalOAuthError::Unavailable)?;
        let markers = journal.shared_state();
        let mut bindings = Vec::new();
        for policy in policies {
            let entry = &policy.configured;
            let authority = policy
                .provider
                .authority
                .ok_or(PersonalOAuthError::Invalid)?;
            let leaf =
                super::credential_leaf(policy.provider, Some(&policy.registration.auth_profile))
                    .map_err(|_| PersonalOAuthError::Invalid)?;
            let access = super::credential_address(owner.tenant_id(), authority, entry, leaf)
                .map_err(|_| PersonalOAuthError::Invalid)?;
            let refresh_address = super::credential_address(
                owner.tenant_id(),
                authority,
                entry,
                &format!("{leaf}_refresh"),
            )
            .map_err(|_| PersonalOAuthError::Invalid)?;
            let custody = custody::Binding::new(
                Identity {
                    owner: format!(
                        "owner:{}",
                        hex::encode(field_digest(&[
                            owner.tenant_id(),
                            owner.subject(),
                            owner.actor_subject()
                        ]))
                    ),
                    integration: policy.provider.id.into(),
                    connection: super::connection_ref(&entry.provider, entry.instance()),
                    purpose: policy.registration.auth_profile.clone(),
                    store: "personal-oauth-development-file-v1".into(),
                    address_digest: [0; 32],
                },
                access,
                refresh_address.clone(),
            )
            .map_err(|_| PersonalOAuthError::Invalid)?;
            let current = Arc::new(Mutex::new(CurrentAuthority {
                identity: custody.identity.clone(),
                active: true,
                generation: 0,
                subject: None,
                provider: policy.provider,
                purpose: policy.registration.auth_profile.clone(),
                allow_writes: entry.allow_writes,
                client_digest: policy.client_digest,
                origin_digest: policy.origin_digest,
                authority_digest: policy.authority_digest,
                ceiling: policy.ceiling.clone(),
            }));
            let gate = Arc::new(RefreshOwner::new(
                custody.clone(),
                clock.clone(),
                Arc::new(Mutex::new(OperationAuthority(current.clone()))),
            ));
            // The raw delegate is private and only reached while this owner's gate is held.
            let mut raw = entry.clone();
            raw.oauth = None;
            let delegate =
                CatalogBackend::bind_stored(owner.clone(), &[raw], store.clone(), egress.clone())
                    .map_err(|_| PersonalOAuthError::Invalid)?;
            bindings.push(Arc::new(OAuthBinding {
                custody,
                refresh_address,
                policy,
                delegate,
                gate,
                authority: current,
            }));
        }
        let custody = CustodyOwner::open(
            journal,
            store.clone(),
            bindings
                .iter()
                .map(|binding| binding.custody.clone())
                .collect(),
        )
        .map_err(|_| PersonalOAuthError::Unavailable)?;
        custody
            .recover()
            .await
            .map_err(|_| PersonalOAuthError::Unavailable)?;
        let (stopping, _) = watch::channel(false);
        let backend = Self {
            inner: Arc::new(OAuthInner {
                owner,
                bindings,
                custody,
                store,
                markers,
                egress,
                clock,
                persistent,
                stopping,
                sessions: Mutex::new(BTreeMap::new()),
                marker_blocks: Mutex::new(BTreeMap::new()),
            }),
        };
        for binding in &backend.inner.bindings {
            let _guard = binding.gate.lock().await;
            backend.inner.synchronize(binding)?;
            // A known surviving attempt permits explicit repair but never old-token dispatch.
            backend.inner.reconcile_marker(binding)?;
        }
        Ok(backend)
    }
}

impl OAuthInner {
    fn now(&self) -> Result<u64> {
        self.clock
            .now()
            .map(|(now, _)| now)
            .map_err(|_| PersonalOAuthError::Unavailable)
    }

    fn synchronize(&self, binding: &OAuthBinding) -> Result<Option<Publication>> {
        let publication = self
            .custody
            .snapshot(&binding.custody.identity)
            .map_err(|_| PersonalOAuthError::Unavailable)?;
        let mut authority = lock(&binding.authority)?;
        authority.generation = publication
            .as_ref()
            .map_or(0, |publication| publication.generation);
        authority.subject = publication
            .as_ref()
            .map(|publication| publication.evidence.subject.clone());
        Ok(publication)
    }

    fn reconcile_marker(&self, binding: &OAuthBinding) -> Result<bool> {
        let identity = &binding.custody.identity;
        let key = marker_key(identity)?;
        let bytes = self
            .markers
            .read(&key, MAX_MARKER)
            .map_err(|_| PersonalOAuthError::Unavailable)?;
        if let Some(bytes) = bytes {
            let marker: RefreshAttempt =
                serde_json::from_slice(&bytes).map_err(|_| PersonalOAuthError::Unavailable)?;
            if marker.version != 1
                || marker.connection_ref != identity.connection
                || marker.binding_sha256 != binding_digest(identity)?
                || marker.previous_generation == 0
            {
                return Err(PersonalOAuthError::Unavailable);
            }
            let mut blocks = lock(&self.marker_blocks)?;
            blocks
                .entry(identity.connection.clone())
                .and_modify(|generation| {
                    *generation = (*generation).max(marker.previous_generation)
                })
                .or_insert(marker.previous_generation);
        }
        let previous = lock(&self.marker_blocks)?
            .get(&identity.connection)
            .copied();
        let Some(previous) = previous else {
            return Ok(false);
        };
        let publication = self
            .custody
            .snapshot(identity)
            .map_err(|_| PersonalOAuthError::Unavailable)?;
        if publication.as_ref().is_some_and(|publication| {
            publication.generation > previous
                && binding
                    .authority
                    .lock()
                    .is_ok_and(|current| current.evidence_matches(&publication.evidence))
        }) {
            self.markers
                .delete(&key)
                .map_err(|_| PersonalOAuthError::Unavailable)?;
            lock(&self.marker_blocks)?.remove(&identity.connection);
            Ok(false)
        } else {
            Ok(true)
        }
    }

    fn begin_marker(&self, binding: &OAuthBinding, generation: u64) -> Result<()> {
        let identity = &binding.custody.identity;
        if generation == 0 || self.reconcile_marker(binding)? {
            return Err(PersonalOAuthError::Unavailable);
        }
        let marker = RefreshAttempt {
            version: 1,
            connection_ref: identity.connection.clone(),
            binding_sha256: binding_digest(identity)?,
            previous_generation: generation,
        };
        let bytes = serde_json::to_vec(&marker).map_err(|_| PersonalOAuthError::Unavailable)?;
        // Remember uncertainty even if a failed replace did not leave bytes readable in this process.
        lock(&self.marker_blocks)?.insert(identity.connection.clone(), generation);
        self.markers
            .replace(&marker_key(identity)?, &bytes, MAX_MARKER)
            .map_err(|_| PersonalOAuthError::Unavailable)
    }
}

// This is a separate keyed value; the completion journal's Image/version stays unchanged.
// It is never an authorization decision, receipt attestation or assertion of remote rollback.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RefreshAttempt {
    version: u8,
    connection_ref: String,
    binding_sha256: [u8; 32],
    previous_generation: u64,
}

fn binding_digest(identity: &Identity) -> Result<[u8; 32]> {
    serde_json::to_vec(identity)
        .map(|bytes| Sha256::digest(bytes).into())
        .map_err(|_| PersonalOAuthError::Unavailable)
}

fn marker_key(identity: &Identity) -> Result<String> {
    Ok(format!(
        "oauth.refresh.v1.{}",
        hex::encode(binding_digest(identity)?)
    ))
}

fn scopes_admit(
    operation: &catalog::Operation,
    purpose: &str,
    observed: &BTreeSet<String>,
    ceiling: &BTreeSet<String>,
) -> bool {
    operation.auth_requirements.iter().any(|mechanism| {
        mechanism.credentials.len() == 1
            && mechanism.credentials[0] == purpose
            && mechanism.scopes.get(purpose).is_some_and(|alternatives| {
                !alternatives.is_empty()
                    && alternatives.iter().any(|group| {
                        !group.is_empty()
                            && group
                                .iter()
                                .all(|scope| observed.contains(scope) && ceiling.contains(scope))
                    })
            })
    })
}

fn same_owner(left: &PrincipalContext, right: &PrincipalContext) -> bool {
    left.tenant_id() == right.tenant_id()
        && left.subject() == right.subject()
        && left.actor_subject() == right.actor_subject()
        && left.agent_revision() == right.agent_revision()
        && left.authority_snapshot_id() == right.authority_snapshot_id()
        && left.authority_snapshot_sha256() == right.authority_snapshot_sha256()
}

fn field_digest(fields: &[&str]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"b10x/personal-oauth-authority/v1\0");
    for field in fields {
        hash.update((field.len() as u64).to_be_bytes());
        hash.update(field.as_bytes());
    }
    hash.finalize().into()
}

fn connection_error(error: PersonalOAuthError) -> connection_api::ConnectionError {
    let code = match error {
        PersonalOAuthError::Invalid => connection_api::ConnectionErrorCode::InvalidInput,
        PersonalOAuthError::Refused => connection_api::ConnectionErrorCode::NotGranted,
        PersonalOAuthError::PortInUse => connection_api::ConnectionErrorCode::Conflict,
        _ => connection_api::ConnectionErrorCode::Unavailable,
    };
    connection_api::ConnectionError::new(code, error.to_string(), false)
}

fn operation_error(error: PersonalOAuthError) -> operation_api::OperationError {
    super::refusal(
        match error {
            PersonalOAuthError::Refused => operation_api::OperationErrorCode::NotGranted,
            _ => operation_api::OperationErrorCode::Unavailable,
        },
        error.to_string(),
    )
}

fn transport_error(
    error: connect_session_transport::oauth::OAuthTransportError,
) -> PersonalOAuthError {
    use connect_session_transport::oauth::OAuthTransportError;
    match error {
        OAuthTransportError::PortInUse => PersonalOAuthError::PortInUse,
        OAuthTransportError::Expired => PersonalOAuthError::Expired,
        _ => PersonalOAuthError::Refused,
    }
}

fn lock<T>(value: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>> {
    value.lock().map_err(|_| PersonalOAuthError::Unavailable)
}

#[path = "oauth_remediation.rs"]
mod remediation;

#[path = "oauth_acquisition.rs"]
mod acquisition;

#[cfg(test)]
#[path = "oauth_tests.rs"]
mod tests;

struct AcquisitionDeadline {
    start: u64,
    expires: u64,
    monotonic_start: std::time::Instant,
    deadline: std::time::Instant,
}
impl AcquisitionDeadline {
    fn new(clock: &dyn RefreshClock, seconds: u64) -> Result<Self> {
        let (start, monotonic_start) = clock.now().map_err(|_| PersonalOAuthError::Unavailable)?;
        let duration = Duration::from_secs(seconds);
        Ok(Self {
            start,
            expires: start
                .checked_add(
                    seconds
                        .checked_mul(1_000)
                        .ok_or(PersonalOAuthError::Invalid)?,
                )
                .ok_or(PersonalOAuthError::Invalid)?,
            monotonic_start,
            deadline: monotonic_start
                .checked_add(duration)
                .ok_or(PersonalOAuthError::Invalid)?,
        })
    }
    fn cap(&mut self, expires: u64) -> Result<()> {
        self.expires = self.expires.min(expires);
        self.deadline = self
            .monotonic_start
            .checked_add(Duration::from_millis(
                self.expires
                    .checked_sub(self.start)
                    .ok_or(PersonalOAuthError::Expired)?,
            ))
            .ok_or(PersonalOAuthError::Expired)?;
        Ok(())
    }
    fn remaining(&self, clock: &dyn RefreshClock) -> Result<Duration> {
        let (now, monotonic) = clock.now().map_err(|_| PersonalOAuthError::Unavailable)?;
        if now < self.start || now >= self.expires || monotonic < self.monotonic_start {
            return Err(PersonalOAuthError::Expired);
        }
        self.deadline
            .checked_duration_since(monotonic)
            .filter(|duration| !duration.is_zero())
            .map(|duration| duration.min(Duration::from_millis(self.expires - now)))
            .ok_or(PersonalOAuthError::Expired)
    }
}

fn private_string<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Zeroizing<String>, D::Error> {
    String::deserialize(deserializer).map(Zeroizing::new)
}
fn optional_private_string<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Zeroizing<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(|value| value.map(Zeroizing::new))
}
#[derive(Deserialize)]
struct PrivateToken {
    #[serde(deserialize_with = "private_string")]
    access_token: Zeroizing<String>,
    #[serde(default, deserialize_with = "optional_private_string")]
    refresh_token: Option<Zeroizing<String>>,
    expires_in: u64,
    #[serde(default)]
    created_at: Option<u64>,
    token_type: String,
}
#[derive(Deserialize)]
struct PrivateDevice {
    #[serde(deserialize_with = "private_string")]
    device_code: Zeroizing<String>,
    #[serde(deserialize_with = "private_string")]
    user_code: Zeroizing<String>,
    #[serde(deserialize_with = "private_string")]
    verification_uri: Zeroizing<String>,
    #[serde(default, deserialize_with = "optional_private_string")]
    verification_uri_complete: Option<Zeroizing<String>>,
    expires_in: u64,
    #[serde(default)]
    interval: Option<u64>,
}

fn form(fields: &[(&str, &str)]) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(fields.iter().copied())
        .finish()
}
fn json_response(status: u16, body: &[u8]) -> Result<serde_json::Value> {
    if !(200..300).contains(&status) || body.len() > MAX_RESPONSE {
        return Err(PersonalOAuthError::Refused);
    }
    serde_json::from_slice(body).map_err(|_| PersonalOAuthError::Refused)
}
fn token_response(status: u16, body: &[u8], policy: &Policy, now: u64) -> Result<ValidatedToken> {
    if !(200..300).contains(&status) || body.len() > MAX_RESPONSE {
        return Err(PersonalOAuthError::Refused);
    }
    let token: PrivateToken =
        serde_json::from_slice(body).map_err(|_| PersonalOAuthError::Refused)?;
    connector_oauth::validate_bearer(
        TokenResponse {
            access_token: token.access_token.to_string(),
            refresh_token: token.refresh_token.as_ref().map(|value| value.to_string()),
            expires_in: token.expires_in,
            created_at: token.created_at,
            scope: String::new(),
            token_type: token.token_type,
        },
        &TokenPolicy {
            expect_token_type: "Bearer",
            require_refresh_token: policy.admission.refresh_policy == OAuthRefreshPolicy::Required,
            required_scopes: &[],
            scopes: connector_oauth::ScopePolicy {
                separator: connector_oauth::ScopeSeparator::Whitespace,
                retain: None,
            },
            require_created_at: false,
            require_expires_in: true,
            max_secret_len: 8192,
        },
        now,
    )
    .map_err(|_| PersonalOAuthError::Refused)
}
fn observed_evidence(
    policy: &Policy,
    value: &serde_json::Value,
    observed_at: u64,
    expires_at: u64,
) -> Result<Evidence> {
    let declaration = &policy.admission.token_evidence;
    let client = value
        .pointer(
            declaration
                .client_id_pointer
                .as_deref()
                .ok_or(PersonalOAuthError::Invalid)?,
        )
        .and_then(serde_json::Value::as_str)
        .ok_or(PersonalOAuthError::Refused)?;
    if client != policy.registration.client_id {
        return Err(PersonalOAuthError::Refused);
    }
    let subject = value
        .pointer(
            declaration
                .subject_pointer
                .as_deref()
                .ok_or(PersonalOAuthError::Invalid)?,
        )
        .ok_or(PersonalOAuthError::Refused)?;
    let subject = match subject {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Number(value) if value.as_u64().is_some_and(|value| value > 0) => {
            value.to_string()
        }
        _ => return Err(PersonalOAuthError::Refused),
    };
    if subject.is_empty()
        || subject.len() > 512
        || !subject.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(PersonalOAuthError::Refused);
    }
    let raw = value
        .pointer(&declaration.scopes_pointer)
        .ok_or(PersonalOAuthError::Refused)?;
    let scopes: BTreeSet<String> = match declaration.scope_encoding {
        OAuthScopeEncoding::StringArray => raw
            .as_array()
            .ok_or(PersonalOAuthError::Refused)?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or(PersonalOAuthError::Refused)
            })
            .collect::<Result<_>>()?,
        OAuthScopeEncoding::SpaceDelimited => raw
            .as_str()
            .ok_or(PersonalOAuthError::Refused)?
            .split_whitespace()
            .map(str::to_owned)
            .collect(),
        OAuthScopeEncoding::CommaDelimited => raw
            .as_str()
            .ok_or(PersonalOAuthError::Refused)?
            .split(',')
            .map(str::trim)
            .map(str::to_owned)
            .collect(),
    };
    if scopes.is_empty()
        || scopes.len() > 128
        || scopes.iter().any(|scope| {
            scope.is_empty()
                || scope.len() > 256
                || !scope.bytes().all(|byte| byte.is_ascii_graphic())
        })
    {
        return Err(PersonalOAuthError::Refused);
    }
    Ok(Evidence {
        scopes,
        ceiling: policy.ceiling.clone(),
        subject,
        client_digest: policy.client_digest,
        origin_digest: policy.origin_digest,
        authority_digest: policy.authority_digest,
        observed_at,
        expires_at,
    })
}

impl OAuthInner {
    fn coherent(&self, binding: &OAuthBinding) -> Result<Option<Publication>> {
        let publication = self.synchronize(binding)?;
        if self.reconcile_marker(binding)? {
            return Err(PersonalOAuthError::Unavailable);
        }
        if publication.as_ref().is_some_and(|publication| {
            !binding
                .authority
                .lock()
                .is_ok_and(|current| current.evidence_matches(&publication.evidence))
        }) {
            return Err(PersonalOAuthError::Refused);
        }
        Ok(publication)
    }

    async fn search(
        &self,
        query: &str,
        limit: u16,
    ) -> Result<Vec<operation_api::OperationSummary>> {
        let mut merged = BTreeMap::<String, operation_api::OperationSummary>::new();
        for binding in &self.bindings {
            let _guard = binding.gate.lock().await;
            let Some(publication) = self.coherent(binding).ok().flatten() else {
                continue;
            };
            for mut summary in binding.delegate.inner.search(query, limit) {
                let Some(operation) =
                    catalog::operation(catalog::OperationKey::id(&summary.operation_ref))
                else {
                    continue;
                };
                if !lock(&binding.authority)?.operation(operation, &publication.evidence) {
                    continue;
                }
                for connection in &mut summary.connections {
                    connection.purpose = Some(binding.policy.registration.auth_profile.clone());
                }
                if let Some(existing) = merged.get_mut(&summary.operation_ref) {
                    existing.connections.extend(summary.connections);
                } else {
                    merged.insert(summary.operation_ref.clone(), summary);
                }
            }
        }
        Ok(merged.into_values().take(usize::from(limit)).collect())
    }

    async fn describe(
        &self,
        operation_ref: &str,
    ) -> std::result::Result<operation_api::OperationDescription, operation_api::OperationError>
    {
        let operation =
            catalog::operation(catalog::OperationKey::id(operation_ref)).ok_or_else(|| {
                super::refusal(
                    operation_api::OperationErrorCode::NotFound,
                    "unknown operation",
                )
            })?;
        let mut result: Option<operation_api::OperationDescription> = None;
        for binding in self
            .bindings
            .iter()
            .filter(|binding| binding.policy.provider.id == operation.provider)
        {
            let _guard = binding.gate.lock().await;
            let mut description = binding.delegate.inner.describe(operation_ref)?;
            let admitted =
                self.coherent(binding)
                    .ok()
                    .flatten()
                    .is_some_and(|publication| {
                        binding.authority.lock().is_ok_and(|current| {
                            current.operation(operation, &publication.evidence)
                        })
                    });
            if !admitted {
                description.connections.clear();
            }
            for connection in &mut description.connections {
                connection.purpose = Some(binding.policy.registration.auth_profile.clone());
            }
            if let Some(existing) = &mut result {
                existing.connections.extend(description.connections);
            } else {
                result = Some(description);
            }
        }
        result.ok_or_else(|| {
            super::refusal(
                operation_api::OperationErrorCode::NotFound,
                "unknown operation",
            )
        })
    }

    fn admit_invocation(
        &self,
        binding: &OAuthBinding,
        request: &operation_api::InvokeRequest,
    ) -> std::result::Result<&'static catalog::Operation, operation_api::OperationError> {
        let (operation, raw) = binding.delegate.inner.admit_invocation(
            &request.operation_ref,
            &request.connection_ref,
            &request.description_ref,
            &request.input,
        )?;
        let document = connector_resolve::document::provider(raw.provider.id)
            .ok_or_else(|| operation_error(PersonalOAuthError::Unavailable))?;
        let declared = document
            .operation(&request.operation_ref)
            .ok_or_else(|| operation_error(PersonalOAuthError::Refused))?;
        let endpoints = connector_resolve::resolve_endpoints(
            declared,
            raw.provider,
            self.owner.tenant_id(),
            &raw.config,
        )
        .map_err(|_| {
            super::refusal(
                operation_api::OperationErrorCode::InvalidInput,
                "a configuration variable this operation's URL needs was not supplied",
            )
        })?;
        let base = document
            .base_url(operation.service)
            .ok_or_else(|| operation_error(PersonalOAuthError::Unavailable))?;
        // This is the same pure request builder used by resolve. It validates input without
        // reading a credential, sending a request, or consuming an approval/event claim.
        connector_resolve::build_request(declared, base, &request.input, &endpoints).map_err(
            |_| {
                super::refusal(
                    operation_api::OperationErrorCode::InvalidInput,
                    "caller input did not satisfy the declared request",
                )
            },
        )?;
        Ok(operation)
    }

    async fn invoke(
        &self,
        request: operation_api::InvokeRequest,
    ) -> std::result::Result<operation_api::InvocationResult, operation_api::OperationError> {
        let binding = self
            .bindings
            .iter()
            .find(|binding| binding.custody.identity.connection == request.connection_ref)
            .ok_or_else(|| operation_error(PersonalOAuthError::Refused))?;
        let guard = binding.gate.lock().await;
        let operation = self.admit_invocation(binding, &request)?;
        let mut publication = self
            .coherent(binding)
            .map_err(operation_error)?
            .ok_or_else(|| operation_error(PersonalOAuthError::Refused))?;
        if !lock(&binding.authority)
            .map_err(operation_error)?
            .operation(operation, &publication.evidence)
        {
            return Err(operation_error(PersonalOAuthError::Refused));
        }
        let now = self.now().map_err(operation_error)?;
        if now < publication.evidence.observed_at {
            return Err(operation_error(PersonalOAuthError::Unavailable));
        }
        if now >= publication.evidence.expires_at {
            let ticket = guard
                .begin(&self.custody, &request.operation_ref)
                .map_err(|_| operation_error(PersonalOAuthError::Refused))?;
            publication = self
                .refresh(binding, &publication, ticket)
                .await
                .map_err(operation_error)?;
        }
        let now = self.now().map_err(operation_error)?;
        if now >= publication.evidence.expires_at
            || !lock(&binding.authority)
                .map_err(operation_error)?
                .operation(operation, &publication.evidence)
        {
            return Err(operation_error(PersonalOAuthError::Refused));
        }
        binding
            .delegate
            .inner
            .invoke(
                &request.operation_ref,
                &request.connection_ref,
                &request.description_ref,
                request.input,
            )
            .await
    }

    fn connection_summary(
        &self,
        binding: &OAuthBinding,
    ) -> Result<connection_api::ConnectionSummary> {
        let mut summary = binding
            .delegate
            .inner
            .connections("", 1)
            .into_iter()
            .next()
            .ok_or(PersonalOAuthError::Unavailable)?;
        summary.scope = Some(connection_api::ConnectionScope::Principal);
        summary.actor = Some(connection_api::ConnectionActor::User);
        summary.auth_profile = Some(binding.policy.registration.auth_profile.clone());
        summary.state = match self.coherent(binding) {
            Ok(Some(publication)) if self.now()? < publication.evidence.expires_at => {
                connection_api::ConnectionState::Callable
            }
            Ok(Some(_)) => connection_api::ConnectionState::Authorized,
            Ok(None) => connection_api::ConnectionState::Created,
            Err(_) => connection_api::ConnectionState::Degraded,
        };
        Ok(summary)
    }

    async fn session_status(
        &self,
        reference: &str,
    ) -> Result<connection_api::ConnectSessionStatus> {
        let (index, lifecycle, liveness) = {
            let sessions = lock(&self.sessions)?;
            let session = sessions.get(reference).ok_or(PersonalOAuthError::Refused)?;
            (
                session.binding,
                session.lifecycle.clone(),
                session.liveness.clone(),
            )
        };
        {
            let lifecycle = lock(&lifecycle)?;
            let status = custody::project_session(&lifecycle, reference)
                .map_err(|_| PersonalOAuthError::Unavailable)?
                .ok_or(PersonalOAuthError::Unavailable)?;
            if status.state == connection_api::ConnectSessionState::Pending {
                // Observe under the lifecycle decision lock. This observation cannot promise that
                // the receiver will still be live after this response is sent.
                return if liveness.is_live() {
                    Ok(status)
                } else {
                    Err(PersonalOAuthError::Unavailable)
                };
            }
            if status.state != connection_api::ConnectSessionState::Completed {
                return Ok(status);
            }
        }
        let binding = &self.bindings[index];
        let _guard = binding.gate.lock().await;
        self.coherent(binding)?
            .ok_or(PersonalOAuthError::Unavailable)?;
        let lifecycle = lock(&lifecycle)?;
        let status = custody::project_session(&lifecycle, reference)
            .map_err(|_| PersonalOAuthError::Unavailable)?
            .ok_or(PersonalOAuthError::Unavailable)?;
        Ok(status)
    }
}

#[async_trait]
impl ConnectorBackend for PersonalOAuthBackend {
    fn owns_remediation(&self, route: service::RemediationRoute<'_>) -> bool {
        self.owns_bound_remediation(route)
    }
    fn remediation_metadata<'a>(
        &'a self,
        context: &PrincipalContext,
        target: service::RemediationTarget<'_>,
    ) -> std::result::Result<service::RemediationMetadata<'a>, service::RemediationError> {
        self.bound_metadata(context, target)
    }
    fn personal_remediation_admission(
        &self,
        context: &PrincipalContext,
        target: service::RemediationTarget<'_>,
    ) -> std::result::Result<service::RemediationAdmission, service::RemediationError> {
        self.bound_admission(context, target)
    }
    async fn credential_readiness(
        &self,
        context: &PrincipalContext,
        target: service::RemediationTarget<'_>,
    ) -> service::CredentialReadiness {
        self.bound_readiness(context, target).await
    }
    async fn handle_remediation(
        &self,
        context: &PrincipalContext,
        request: service::RemediationRequest,
        authority: Arc<dyn service::RemediationAuthority>,
    ) -> std::result::Result<service::RemediationResult, service::RemediationError> {
        self.bound_request(context, request, authority).await
    }
    async fn ready(&self) -> std::result::Result<(), service::BackendReadinessError> {
        self.inner
            .custody
            .recover()
            .await
            .map_err(|_| service::BackendReadinessError)
    }

    fn capabilities(&self) -> service::BackendCapabilities {
        service::BackendCapabilities {
            connections: true,
            ..service::BackendCapabilities::OPERATIONS
        }
    }

    fn owns_operation(&self, request: &operation_api::OperationRequest) -> bool {
        match request {
            operation_api::OperationRequest::Search(_) => true,
            operation_api::OperationRequest::Describe(request) => self
                .inner
                .bindings
                .iter()
                .any(|binding| binding.delegate.owns_operation_ref(&request.operation_ref)),
            operation_api::OperationRequest::Invoke(request) => {
                self.inner.bindings.iter().any(|binding| {
                    binding.custody.identity.connection == request.connection_ref
                        && binding.delegate.owns_operation_ref(&request.operation_ref)
                })
            }
            _ => false,
        }
    }

    fn owns_connection(&self, request: &connection_api::ConnectionRequest) -> bool {
        match request {
            connection_api::ConnectionRequest::Search(_) => true,
            connection_api::ConnectionRequest::Describe(request) => self
                .inner
                .bindings
                .iter()
                .any(|binding| binding.custody.identity.connection == request.connection_ref),
            connection_api::ConnectionRequest::ConnectSessionCreate(request) => self
                .inner
                .bindings
                .iter()
                .any(|binding| binding.policy.provider.id == request.integration_ref),
            connection_api::ConnectionRequest::ConnectSessionStatus(request) => self
                .inner
                .sessions
                .lock()
                .is_ok_and(|sessions| sessions.contains_key(&request.connect_session_ref)),
            _ => false,
        }
    }

    fn setup_profiles(&self, provider_ref: &str) -> Vec<protocol::catalog::SetupProfileSummary> {
        if !self.inner.persistent || *self.inner.stopping.borrow() {
            return Vec::new();
        }
        let profiles: BTreeSet<_> = self
            .inner
            .bindings
            .iter()
            .filter(|binding| binding.policy.provider.id == provider_ref)
            .map(|binding| binding.policy.registration.auth_profile.clone())
            .collect();
        profiles
            .into_iter()
            .filter(|profile| {
                self.inner
                    .select_binding(&connection_api::ConnectSessionCreateRequest {
                        integration_ref: provider_ref.to_owned(),
                        auth_profile: Some(profile.clone()),
                        label: "setup".into(),
                    })
                    .is_ok()
            })
            .map(|auth_profile| protocol::catalog::SetupProfileSummary {
                auth_profile,
                actor: protocol::catalog::SetupProfileActor::Person,
            })
            .collect()
    }

    fn supports_ephemeral_invocation(&self, request: &operation_api::InvokeRequest) -> bool {
        self.inner.bindings.iter().any(|binding| {
            binding.custody.identity.connection == request.connection_ref
                && binding.delegate.supports_ephemeral_invocation(request)
        })
    }

    async fn handle(
        &self,
        context: &PrincipalContext,
        request: operation_api::OperationRequest,
    ) -> std::result::Result<operation_api::OperationResult, operation_api::OperationError> {
        if !same_owner(&self.inner.owner, context) || *self.inner.stopping.borrow() {
            return Err(operation_error(PersonalOAuthError::Refused));
        }
        self.inner
            .custody
            .recover()
            .await
            .map_err(|_| operation_error(PersonalOAuthError::Unavailable))?;
        match request {
            operation_api::OperationRequest::Search(request) => {
                Ok(operation_api::OperationResult::Search {
                    operations: self
                        .inner
                        .search(&request.query, request.limit)
                        .await
                        .map_err(operation_error)?,
                })
            }
            operation_api::OperationRequest::Describe(request) => self
                .inner
                .describe(&request.operation_ref)
                .await
                .map(operation_api::OperationResult::Describe),
            operation_api::OperationRequest::Invoke(request) => self
                .inner
                .invoke(request)
                .await
                .map(operation_api::OperationResult::Invoke),
            _ => Err(operation_error(PersonalOAuthError::Refused)),
        }
    }

    async fn handle_connection(
        &self,
        context: &PrincipalContext,
        request: connection_api::ConnectionRequest,
    ) -> std::result::Result<connection_api::ConnectionResult, connection_api::ConnectionError>
    {
        if !same_owner(&self.inner.owner, context) || *self.inner.stopping.borrow() {
            return Err(connection_error(PersonalOAuthError::Refused));
        }
        // Status must not wait behind a human-held device gate or an in-progress transaction.
        if let connection_api::ConnectionRequest::ConnectSessionStatus(request) = request {
            return self
                .inner
                .session_status(&request.connect_session_ref)
                .await
                .map(connection_api::ConnectionResult::ConnectSessionStatus)
                .map_err(connection_error);
        }
        self.inner
            .custody
            .recover()
            .await
            .map_err(|_| connection_error(PersonalOAuthError::Unavailable))?;
        match request {
            connection_api::ConnectionRequest::ConnectSessionCreate(request) => self
                .inner
                .create(request)
                .await
                .map(connection_api::ConnectionResult::ConnectSessionCreate)
                .map_err(connection_error),
            connection_api::ConnectionRequest::Search(request) => {
                let mut connections = Vec::new();
                for binding in &self.inner.bindings {
                    if !super::matches_query(
                        &request.query,
                        &[
                            binding.policy.provider.id,
                            &binding.policy.configured.label(),
                        ],
                    ) {
                        continue;
                    }
                    let _guard = binding.gate.lock().await;
                    connections.push(
                        self.inner
                            .connection_summary(binding)
                            .map_err(connection_error)?,
                    );
                    if connections.len() >= usize::from(request.limit) {
                        break;
                    }
                }
                Ok(connection_api::ConnectionResult::Search { connections })
            }
            connection_api::ConnectionRequest::Describe(request) => {
                let binding = self
                    .inner
                    .bindings
                    .iter()
                    .find(|binding| binding.custody.identity.connection == request.connection_ref)
                    .ok_or_else(|| connection_error(PersonalOAuthError::Refused))?;
                let _guard = binding.gate.lock().await;
                Ok(connection_api::ConnectionResult::Describe(
                    connection_api::ConnectionDescription {
                        summary: self
                            .inner
                            .connection_summary(binding)
                            .map_err(connection_error)?,
                        channels: Vec::new(),
                    },
                ))
            }
            _ => Err(connection_error(PersonalOAuthError::Refused)),
        }
    }

    async fn shutdown(&self) {
        for binding in &self.inner.bindings {
            if let Ok(mut authority) = binding.authority.lock() {
                authority.active = false;
            }
        }
        self.inner.stopping.send_replace(true);
        let tasks: Vec<_> = self
            .inner
            .sessions
            .lock()
            .map(|mut sessions| {
                sessions
                    .values_mut()
                    .filter_map(|session| session.task.take())
                    .collect()
            })
            .unwrap_or_default();
        for task in tasks {
            let _ = task.await;
        }
        // No new admission follows shutdown. Decided transactions may still finish durably;
        // undecided transactions abort, and uncertain recovery stays unavailable on reopen.
        let _ = self.inner.custody.recover().await;
    }
}

fn prepare_journal_path(directory: &Path) -> Result<()> {
    use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _};
    let journal = directory.join("transactions.sqlite");
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&journal)
    {
        Ok(file) => file
            .sync_all()
            .map_err(|_| PersonalOAuthError::Unavailable)?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err(PersonalOAuthError::Unavailable),
    }
    for name in [
        "transactions.sqlite",
        "transactions.sqlite-wal",
        "transactions.sqlite-shm",
    ] {
        match std::fs::symlink_metadata(directory.join(name)) {
            Ok(metadata)
                if metadata.is_file()
                    && metadata.uid() == rustix::process::geteuid().as_raw()
                    && metadata.mode() & 0o077 == 0
                    && metadata.nlink() == 1 => {}
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && name != "transactions.sqlite" => {}
            _ => return Err(PersonalOAuthError::Unavailable),
        }
    }
    Ok(())
}

//! Private, value-free completion journal. Acquisition supplies an already admitted immutable
//! binding and retires its instruction tasks before entering this owner. No product route uses
//! this module until the recorded OAuth acquisition handoff.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use connector_secrets::{
    CredentialRef, CredentialScope, Layout as _, PreparedSecretStore, Secret, SecretBatch,
    SecretProposalDigest, SecretTransactionGeneration, SecretTransactionId, SecretTransactionState,
    TenantLayout,
};
use connector_state::StateStore;
use serde::{Deserialize, Serialize};
use service::ConnectSessionLifecycle;
use sha2::{Digest as _, Sha256};

const KEY: &str = "oauth.custody.v1";
const MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_BINDINGS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum CustodyError {
    #[error("credential custody input is invalid")]
    Invalid,
    #[error("credential custody requires reconciliation")]
    Unavailable,
    #[error("credential custody completion was refused")]
    Refused,
}
type Result<T> = std::result::Result<T, CustodyError>;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Identity {
    pub(super) owner: String,
    pub(super) integration: String,
    pub(super) connection: String,
    pub(super) purpose: String,
    pub(super) store: String,
    pub(super) address_digest: [u8; 32],
}

#[derive(Clone)]
pub(super) struct Binding {
    pub(super) identity: Identity,
    access: CredentialRef,
    refresh: CredentialRef,
}

impl Binding {
    pub(super) fn new(
        mut identity: Identity,
        access: CredentialRef,
        refresh: CredentialRef,
    ) -> Result<Self> {
        if [
            &identity.owner,
            &identity.integration,
            &identity.connection,
            &identity.purpose,
            &identity.store,
        ]
        .into_iter()
        .any(|value| !valid_ref(value))
            || access == refresh
            || access.tenant() != refresh.tenant()
            || access.authority() != refresh.authority()
            || access.instance() != refresh.instance()
            || access.service() != refresh.service()
        {
            return Err(CustodyError::Invalid);
        }
        let mut digest = Sha256::new();
        digest.update(b"b10x/oauth-custody-addresses/v1\0");
        hash_field(&mut digest, TenantLayout.render(&access).as_bytes());
        hash_field(&mut digest, TenantLayout.render(&refresh).as_bytes());
        identity.address_digest = digest.finalize().into();
        Ok(Self {
            identity,
            access,
            refresh,
        })
    }
}

// Observations and the deployment ceiling remain separate. Admission must never union scopes
// across generations or infer the observed grant from the requested ceiling.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Evidence {
    pub(super) scopes: BTreeSet<String>,
    pub(super) ceiling: BTreeSet<String>,
    pub(super) subject: String,
    pub(super) client_digest: [u8; 32],
    pub(super) origin_digest: [u8; 32],
    pub(super) authority_digest: [u8; 32],
    pub(super) observed_at: u64,
    pub(super) expires_at: u64,
}
impl Evidence {
    fn valid(&self) -> bool {
        self.observed_at > 0
            && self.observed_at < self.expires_at
            && valid_ref(&self.subject)
            && self.client_digest != [0; 32]
            && self.origin_digest != [0; 32]
            && self.authority_digest != [0; 32]
            && [&self.scopes, &self.ceiling].into_iter().all(|set| {
                set.len() <= 128
                    && set.iter().all(|scope| {
                        !scope.is_empty()
                            && scope.len() <= 256
                            && scope.bytes().all(|byte| byte.is_ascii_graphic())
                    })
            })
    }
}

// Deliberately neither Debug nor Serialize. The batch and digest have one closed producer,
// including the delete tag when a new refresh credential was not issued.
pub(super) struct Proposal {
    identity: Identity,
    batch: SecretBatch,
    digest: SecretProposalDigest,
}
impl Proposal {
    pub(super) fn new(binding: &Binding, access: Secret, refresh: Option<Secret>) -> Result<Self> {
        if std::iter::once(&access)
            .chain(refresh.iter())
            .any(|value| value.expose_secret().is_empty() || value.expose_secret().len() > 8192)
        {
            return Err(CustodyError::Invalid);
        }
        let mut hash = Sha256::new();
        hash.update(b"b10x/oauth-custody-proposal/v1\0");
        hash_field(
            &mut hash,
            &serde_json::to_vec(&binding.identity).map_err(|_| CustodyError::Invalid)?,
        );
        hash.update(b"put-access\0");
        hash_field(&mut hash, access.expose_secret().as_bytes());
        let mut batch = SecretBatch::new(
            CredentialScope::new(binding.access.tenant(), binding.access.authority())
                .map_err(|_| CustodyError::Invalid)?,
        );
        batch
            .put(binding.access.clone(), access)
            .map_err(|_| CustodyError::Invalid)?;
        if let Some(refresh) = refresh {
            hash.update(b"put-refresh\0");
            hash_field(&mut hash, refresh.expose_secret().as_bytes());
            batch
                .put(binding.refresh.clone(), refresh)
                .map_err(|_| CustodyError::Invalid)?;
        } else {
            hash.update(b"delete-refresh\0");
            batch
                .delete(binding.refresh.clone())
                .map_err(|_| CustodyError::Invalid)?;
        }
        Ok(Self {
            identity: binding.identity.clone(),
            batch,
            digest: SecretProposalDigest::from_protocol_bytes(hash.finalize().into()),
        })
    }
}
fn hash_field(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}
fn valid_ref(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && value.bytes().all(|byte| byte.is_ascii_graphic())
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Timing {
    pub(super) authorized_at: u64,
    pub(super) deadline: u64,
}
impl Timing {
    fn valid(&self, evidence: &Evidence) -> bool {
        self.authorized_at > 0
            && self.authorized_at < self.deadline
            && evidence.observed_at <= self.authorized_at
            && self.authorized_at < evidence.expires_at
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Publication {
    pub(super) authorization: Option<Timing>,
    pub(super) identity: Identity,
    pub(super) generation: u64,
    pub(super) evidence: Evidence,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "phase", deny_unknown_fields)]
enum Phase {
    Preparing,
    Decided { authorized_at: u64, deadline: u64 },
    Published { authorized_at: u64, deadline: u64 },
    Aborted,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    transaction: [u8; 32],
    digest: [u8; 32],
    previous_generation: u64,
    publication: Publication,
    phase: Phase,
}
impl Pending {
    fn id(&self) -> Result<SecretTransactionId> {
        SecretTransactionId::from_protocol_bytes(self.transaction).ok_or(CustodyError::Unavailable)
    }
    fn generation(&self) -> Result<SecretTransactionGeneration> {
        SecretTransactionGeneration::from_protocol_bytes(self.publication.generation.to_be_bytes())
            .ok_or(CustodyError::Unavailable)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Image {
    version: u8,
    next_generation: u64,
    retired_through: u64,
    retired_transaction: Option<[u8; 32]>,
    connections: BTreeMap<String, Publication>,
    pending: Option<Pending>,
}
impl Default for Image {
    fn default() -> Self {
        Self {
            version: 1,
            next_generation: 1,
            retired_through: 0,
            retired_transaction: None,
            connections: BTreeMap::new(),
            pending: None,
        }
    }
}
trait JournalIo: Send + Sync {
    fn read(&self) -> Result<Option<Vec<u8>>>;
    fn write(&self, bytes: &[u8]) -> Result<()>;
}
pub(super) struct FullJournal(Arc<state_sqlite::SqliteState>);
impl FullJournal {
    /// Composition supplies an already admitted owner-only state directory and journal path.
    /// This constructor never accepts an ambient store or a NORMAL SQLite connection.
    pub(super) fn open(path: &Path) -> Result<Self> {
        state_sqlite::SqliteState::open_full(path)
            .map(|state| Self(Arc::new(state)))
            .map_err(|_| CustodyError::Unavailable)
    }

    /// The acquisition marker and completion journal share this exact FULL connection.
    pub(super) fn shared_state(&self) -> Arc<state_sqlite::SqliteState> {
        self.0.clone()
    }
}
impl JournalIo for FullJournal {
    fn read(&self) -> Result<Option<Vec<u8>>> {
        self.0
            .read(KEY, MAX_BYTES)
            .map_err(|_| CustodyError::Unavailable)
    }
    fn write(&self, bytes: &[u8]) -> Result<()> {
        self.0
            .replace(KEY, bytes, MAX_BYTES)
            .map_err(|_| CustodyError::Unavailable)
    }
}

// Test observations wrap the same FULL journal. Only a closed value-free boundary leaves this
// module; the wrapper never supplies alternate records or exposes serialized journal contents.
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum JournalPhase {
    Preparing,
    Decided,
    Published,
    Aborted,
    Retired,
}
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum JournalMoment {
    BeforeWrite,
    AfterWrite,
}
#[cfg(test)]
struct ObservedJournal {
    inner: Arc<dyn JournalIo>,
    observe: Arc<dyn Fn(JournalPhase, JournalMoment) -> Result<()> + Send + Sync>,
}
#[cfg(test)]
impl JournalIo for ObservedJournal {
    fn read(&self) -> Result<Option<Vec<u8>>> {
        self.inner.read()
    }
    fn write(&self, bytes: &[u8]) -> Result<()> {
        let image: Image = serde_json::from_slice(bytes).map_err(|_| CustodyError::Unavailable)?;
        let phase = match image.pending.map(|pending| pending.phase) {
            Some(Phase::Preparing) => JournalPhase::Preparing,
            Some(Phase::Decided { .. }) => JournalPhase::Decided,
            Some(Phase::Published { .. }) => JournalPhase::Published,
            Some(Phase::Aborted) => JournalPhase::Aborted,
            None => JournalPhase::Retired,
        };
        (self.observe)(phase, JournalMoment::BeforeWrite)?;
        self.inner.write(bytes)?;
        // This is journal readback supplied by the real SQLite owner, not secret-store digest
        // readback or a power-loss simulation. Reopen fixtures separately establish recovery.
        if self.inner.read()?.as_deref() != Some(bytes) {
            return Err(CustodyError::Unavailable);
        }
        (self.observe)(phase, JournalMoment::AfterWrite)
    }
}

pub(super) struct LiveCompletion {
    sessions: Arc<Mutex<ConnectSessionLifecycle>>,
    session: String,
    connection: String,
}
impl LiveCompletion {
    /// The acquisition owner must retire/join instruction tasks and clean the returned paths
    /// before calling complete. The kernel owns no listeners and starts no detached task.
    pub(super) fn begin(
        sessions: Arc<Mutex<ConnectSessionLifecycle>>,
        session: String,
        connection: String,
    ) -> Result<(Self, Vec<String>)> {
        let paths = lock(&sessions)?
            .begin_completion(&session, &connection)
            .map_err(|_| CustodyError::Refused)?;
        Ok((
            Self {
                sessions,
                session,
                connection,
            },
            paths,
        ))
    }
    fn recovery(&self) -> Result<()> {
        lock(&self.sessions)?
            .require_completion_recovery(&self.session)
            .map_err(|_| CustodyError::Unavailable)
    }
    fn resolve(&self, published: bool) -> Result<()> {
        lock(&self.sessions)?
            .resolve_completion(&self.session, &self.connection, published)
            .map_err(|_| CustodyError::Unavailable)
    }
}
pub(super) trait CompletionAuthority: Send + Sync {
    /// Synchronous receiver-owned recheck under the lifecycle/authority lock. Implementations
    /// must coordinate grant changes with that lock; a session is never itself an operation grant.
    fn recheck(
        &self,
        identity: &Identity,
        previous_generation: u64,
        evidence: &Evidence,
        now: u64,
    ) -> bool;
}
struct OwnerState {
    image: Image,
    uncertain: bool,
    live: Option<CompletionClaim>,
}
pub(super) struct CustodyOwner {
    journal: Arc<dyn JournalIo>,
    store: Arc<dyn PreparedSecretStore>,
    bindings: BTreeMap<String, Binding>,
    state: Mutex<OwnerState>,
    transaction_gate: tokio::sync::Mutex<()>,
}
impl CustodyOwner {
    #[cfg(test)]
    pub(super) fn observe_journal(
        &mut self,
        observe: Arc<dyn Fn(JournalPhase, JournalMoment) -> Result<()> + Send + Sync>,
    ) {
        self.journal = Arc::new(ObservedJournal {
            inner: self.journal.clone(),
            observe,
        });
    }

    pub(super) fn open(
        journal: FullJournal,
        store: Arc<connector_secrets::FileStore>,
        bindings: Vec<Binding>,
    ) -> Result<Self> {
        Self::from_journal(Arc::new(journal), store, bindings)
    }
    fn from_journal(
        journal: Arc<dyn JournalIo>,
        store: Arc<dyn PreparedSecretStore>,
        bindings: Vec<Binding>,
    ) -> Result<Self> {
        if bindings.is_empty() || bindings.len() > MAX_BINDINGS {
            return Err(CustodyError::Invalid);
        }
        let store_id = &bindings[0].identity.store;
        let mut addresses = BTreeSet::new();
        for binding in &bindings {
            if &binding.identity.store != store_id
                || !addresses.insert(binding.access.clone())
                || !addresses.insert(binding.refresh.clone())
            {
                return Err(CustodyError::Invalid);
            }
        }
        let count = bindings.len();
        let bindings: BTreeMap<_, _> = bindings
            .into_iter()
            .map(|binding| (binding.identity.connection.clone(), binding))
            .collect();
        if bindings.len() != count {
            return Err(CustodyError::Invalid);
        }
        let image = Self::load(journal.as_ref())?;
        let owner = Self {
            journal,
            store,
            bindings,
            state: Mutex::new(OwnerState {
                uncertain: image.pending.is_some() || image.retired_through > 0,
                image,
                live: None,
            }),
            transaction_gate: tokio::sync::Mutex::new(()),
        };
        owner.validate(&lock(&owner.state)?.image)?;
        Ok(owner)
    }
    fn load(journal: &dyn JournalIo) -> Result<Image> {
        match journal.read()? {
            None => Ok(Image::default()),
            Some(bytes) if bytes.len() <= MAX_BYTES => {
                serde_json::from_slice(&bytes).map_err(|_| CustodyError::Unavailable)
            }
            Some(_) => Err(CustodyError::Unavailable),
        }
    }
    fn validate(&self, image: &Image) -> Result<()> {
        if image.version != 1
            || image.next_generation == 0
            || image.connections.len() > MAX_BINDINGS
            || image.retired_through >= image.next_generation
        {
            return Err(CustodyError::Unavailable);
        }
        match image.retired_transaction {
            None if image.retired_through == 0 => {}
            Some(id)
                if image.retired_through > 0
                    && id[..8] == image.retired_through.to_be_bytes()
                    && SecretTransactionId::from_protocol_bytes(id).is_some() => {}
            _ => return Err(CustodyError::Unavailable),
        }
        for (key, publication) in &image.connections {
            if key != &publication.identity.connection
                || !self.valid_publication(publication)
                || publication.generation >= image.next_generation
                || publication
                    .authorization
                    .as_ref()
                    .is_none_or(|timing| !timing.valid(&publication.evidence))
                || publication.generation > image.retired_through
                    && image.pending.as_ref().is_none_or(|pending| {
                        !matches!(pending.phase, Phase::Published { .. })
                            || pending.publication != *publication
                    })
            {
                return Err(CustodyError::Unavailable);
            }
        }
        if let Some(pending) = &image.pending {
            let generation = pending.publication.generation;
            if !self.valid_publication(&pending.publication)
                || generation.checked_add(1) != Some(image.next_generation)
                || image.retired_through.checked_add(1) != Some(generation)
                || pending.transaction[..8] != generation.to_be_bytes()
                || pending.digest == [0; 32]
            {
                return Err(CustodyError::Unavailable);
            }
            pending.id()?;
            match pending.phase {
                Phase::Decided {
                    authorized_at,
                    deadline,
                }
                | Phase::Published {
                    authorized_at,
                    deadline,
                } => {
                    let timing = Timing {
                        authorized_at,
                        deadline,
                    };
                    if !timing.valid(&pending.publication.evidence)
                        || pending.publication.authorization.as_ref() != Some(&timing)
                    {
                        return Err(CustodyError::Unavailable);
                    }
                }
                Phase::Preparing | Phase::Aborted
                    if pending.publication.authorization.is_some() =>
                {
                    return Err(CustodyError::Unavailable)
                }
                _ => {}
            }
            let current = image
                .connections
                .get(&pending.publication.identity.connection);
            if matches!(pending.phase, Phase::Published { .. }) {
                if current != Some(&pending.publication) {
                    return Err(CustodyError::Unavailable);
                }
            } else if current.map_or(0, |entry| entry.generation) != pending.previous_generation {
                return Err(CustodyError::Unavailable);
            }
        } else if image.retired_through.checked_add(1) != Some(image.next_generation) {
            return Err(CustodyError::Unavailable);
        }
        Ok(())
    }
    fn valid_publication(&self, publication: &Publication) -> bool {
        publication.generation > 0
            && publication.evidence.valid()
            && self
                .bindings
                .get(&publication.identity.connection)
                .is_some_and(|binding| binding.identity == publication.identity)
    }
    fn persist(&self, image: Image) -> Result<()> {
        self.validate(&image)?;
        let bytes = serde_json::to_vec(&image).map_err(|_| CustodyError::Unavailable)?;
        if bytes.len() > MAX_BYTES {
            return Err(CustodyError::Unavailable);
        }
        self.journal.write(&bytes)?;
        lock(&self.state)?.image = image;
        Ok(())
    }
    pub(super) fn snapshot(&self, identity: &Identity) -> Result<Option<Publication>> {
        let state = lock(&self.state)?;
        if state.uncertain
            || state.image.pending.as_ref().is_some_and(|pending| {
                &pending.publication.identity == identity
                    && !matches!(pending.phase, Phase::Published { .. })
            })
        {
            return Err(CustodyError::Unavailable);
        }
        if self
            .bindings
            .get(&identity.connection)
            .is_none_or(|binding| &binding.identity != identity)
        {
            return Err(CustodyError::Refused);
        }
        Ok(state.image.connections.get(&identity.connection).cloned())
    }
    pub(super) async fn complete(
        &self,
        binding: &Binding,
        previous_generation: u64,
        evidence: Evidence,
        proposal: Proposal,
        live: LiveCompletion,
        authority: &dyn CompletionAuthority,
    ) -> Result<Publication> {
        self.complete_claim(
            binding,
            previous_generation,
            evidence,
            proposal,
            CompletionClaim::Session(live),
            Some(authority),
        )
        .await
    }

    async fn complete_claim(
        &self,
        binding: &Binding,
        previous_generation: u64,
        evidence: Evidence,
        proposal: Proposal,
        live: CompletionClaim,
        authority: Option<&dyn CompletionAuthority>,
    ) -> Result<Publication> {
        let _gate = self.transaction_gate.lock().await;
        let mut image = {
            let state = lock(&self.state)?;
            if state.uncertain || state.image.pending.is_some() {
                live.resolve(false)?;
                return Err(CustodyError::Unavailable);
            }
            state.image.clone()
        };
        if proposal.identity != binding.identity
            || live.connection() != binding.identity.connection
            || self
                .bindings
                .get(&binding.identity.connection)
                .is_none_or(|known| known.identity != binding.identity)
            || !evidence.valid()
            || image
                .connections
                .get(&binding.identity.connection)
                .map_or(0, |entry| entry.generation)
                != previous_generation
        {
            live.resolve(false)?;
            return Err(CustodyError::Refused);
        }
        let allocation = (|| {
            let generation = SecretTransactionGeneration::from_protocol_bytes(
                image.next_generation.to_be_bytes(),
            )
            .ok_or(CustodyError::Unavailable)?;
            let next = generation.checked_next().ok_or(CustodyError::Unavailable)?;
            let mut nonce = [0; 24];
            getrandom::fill(&mut nonce).map_err(|_| CustodyError::Unavailable)?;
            Ok((generation, next, nonce))
        })();
        let (generation, next, nonce) = match allocation {
            Ok(value) => value,
            Err(error) => {
                live.resolve(false)?;
                return Err(error);
            }
        };
        let publication = Publication {
            authorization: None,
            identity: binding.identity.clone(),
            generation: image.next_generation,
            evidence,
        };
        image.next_generation = u64::from_be_bytes(next.protocol_bytes());
        image.pending = Some(Pending {
            transaction: SecretTransactionId::new(generation, nonce).protocol_bytes(),
            digest: proposal.digest.protocol_bytes(),
            previous_generation,
            publication: publication.clone(),
            phase: Phase::Preparing,
        });
        if !Self::publication_fits(&image)? {
            live.resolve(false)?;
            return Err(CustodyError::Unavailable);
        }
        {
            let mut state = lock(&self.state)?;
            state.uncertain = true;
            state.live = Some(live);
        }
        let result = self.execute(image, proposal, authority).await;
        if matches!(result, Err(CustodyError::Unavailable)) {
            self.require_recovery()?;
        }
        result?;
        self.snapshot(&publication.identity)?
            .ok_or(CustodyError::Unavailable)
    }

    fn publication_fits(image: &Image) -> Result<bool> {
        let mut budget = image.clone();
        let publication = budget
            .pending
            .as_ref()
            .ok_or(CustodyError::Unavailable)?
            .publication
            .clone();
        budget
            .connections
            .insert(publication.identity.connection.clone(), publication);
        // Reserve both copies of the authorization object and the largest phase encoding.
        // These maxima measure JSON width only; they are never persisted or used as a claim.
        let timing_width = serde_json::to_vec(&Timing {
            authorized_at: u64::MAX,
            deadline: u64::MAX,
        })
        .map_err(|_| CustodyError::Unavailable)?
        .len();
        let phase_width = serde_json::to_vec(&Phase::Published {
            authorized_at: u64::MAX,
            deadline: u64::MAX,
        })
        .map_err(|_| CustodyError::Unavailable)?
        .len();
        let bytes = serde_json::to_vec(&budget).map_err(|_| CustodyError::Unavailable)?;
        Ok(bytes
            .len()
            .checked_add(2 * timing_width + phase_width)
            .is_some_and(|length| length <= MAX_BYTES))
    }

    async fn execute(
        &self,
        image: Image,
        proposal: Proposal,
        authority: Option<&dyn CompletionAuthority>,
    ) -> Result<()> {
        // The complete id/allocation is FULL-persisted before any recoverable secret staging.
        self.persist(image)?;
        let pending = self.pending()?;
        match self
            .store
            .prepare(pending.id()?, proposal.digest, &proposal.batch)
            .await
        {
            Ok(SecretTransactionState::Prepared) => {}
            Err(_) => {
                match self
                    .store
                    .state(pending.id()?)
                    .await
                    .map_err(|_| CustodyError::Unavailable)?
                {
                    SecretTransactionState::Absent | SecretTransactionState::Prepared => {
                        self.abort_pending().await?
                    }
                    SecretTransactionState::Committed => return Err(CustodyError::Unavailable),
                }
                return Err(CustodyError::Refused);
            }
            Ok(_) => return Err(CustodyError::Unavailable),
        }
        // No state/SQLite lock is held over async store I/O. Each claim variant uses its short
        // authority lock around the current-authority callback and one sampled instant.
        let claim = {
            let mut state = lock(&self.state)?;
            state
                .live
                .as_mut()
                .ok_or(CustodyError::Unavailable)?
                .claim(&pending, authority)
        };
        let (authorized_at, deadline) = match claim {
            Ok(timing) => timing,
            Err(_) => {
                self.abort_pending().await?;
                return Err(CustodyError::Refused);
            }
        };
        self.set_phase(Phase::Decided {
            authorized_at,
            deadline,
        })?;
        // A returned error above might have followed a durable write. Never commit on that
        // unconfirmed result: only explicit recovery rereads and decides what reached the journal.
        self.commit_decided().await
    }

    fn pending(&self) -> Result<Pending> {
        lock(&self.state)?
            .image
            .pending
            .clone()
            .ok_or(CustodyError::Unavailable)
    }
    fn set_phase(&self, phase: Phase) -> Result<()> {
        let mut image = lock(&self.state)?.image.clone();
        let pending = image.pending.as_mut().ok_or(CustodyError::Unavailable)?;
        if let Phase::Decided {
            authorized_at,
            deadline,
        } = phase
        {
            pending.publication.authorization = Some(Timing {
                authorized_at,
                deadline,
            });
        }
        pending.phase = phase;
        self.persist(image)
    }
    fn require_recovery(&self) -> Result<()> {
        let mut state = lock(&self.state)?;
        state.uncertain = true;
        if let Some(live) = &state.live {
            live.recovery()?;
        }
        Ok(())
    }
    fn resolve_live(&self, published: bool) -> Result<()> {
        let mut state = lock(&self.state)?;
        if let Some(live) = &state.live {
            live.resolve(published)?;
        }
        state.live = None;
        Ok(())
    }
    async fn abort_pending(&self) -> Result<()> {
        let pending = self.pending()?;
        if !matches!(pending.phase, Phase::Preparing) {
            return Err(CustodyError::Unavailable);
        }
        match self.store.abort(pending.id()?).await {
            Ok(SecretTransactionState::Absent) => {}
            Err(_)
                if matches!(
                    self.store.state(pending.id()?).await,
                    Ok(SecretTransactionState::Absent)
                ) => {}
            _ => return Err(CustodyError::Unavailable),
        }
        self.set_phase(Phase::Aborted)?;
        self.resolve_live(false)?;
        self.retire_pending().await
    }
    async fn commit_decided(&self) -> Result<()> {
        let pending = self.pending()?;
        let Phase::Decided {
            authorized_at,
            deadline,
        } = pending.phase
        else {
            return Err(CustodyError::Unavailable);
        };
        match self.store.commit(pending.id()?).await {
            Ok(SecretTransactionState::Committed) => {}
            Err(_)
                if matches!(
                    self.store.state(pending.id()?).await,
                    Ok(SecretTransactionState::Committed)
                ) => {}
            _ => return Err(CustodyError::Unavailable),
        }
        let mut image = lock(&self.state)?.image.clone();
        image.connections.insert(
            pending.publication.identity.connection.clone(),
            pending.publication.clone(),
        );
        image
            .pending
            .as_mut()
            .ok_or(CustodyError::Unavailable)?
            .phase = Phase::Published {
            authorized_at,
            deadline,
        };
        self.persist(image)?;
        lock(&self.state)?.uncertain = false;
        self.resolve_live(true)?;
        self.retire_pending().await
    }
    async fn retire_pending(&self) -> Result<()> {
        let pending = self.pending()?;
        if !matches!(pending.phase, Phase::Published { .. } | Phase::Aborted) {
            return Err(CustodyError::Unavailable);
        }
        if self.store.reclaim(pending.generation()?).await.is_err()
            && !matches!(
                self.store.state(pending.id()?).await,
                Err(connector_secrets::PreparedSecretError::Retired)
            )
        {
            return Err(CustodyError::Unavailable);
        }
        let mut image = lock(&self.state)?.image.clone();
        image.retired_through = pending.publication.generation;
        image.retired_transaction = Some(pending.transaction);
        image.pending = None;
        self.persist(image)?;
        lock(&self.state)?.uncertain = false;
        Ok(())
    }

    pub(super) async fn recover(&self) -> Result<()> {
        let _gate = self.transaction_gate.lock().await;
        let result = self.reconcile().await;
        if result.is_err() {
            self.require_recovery()?;
        }
        result
    }
    async fn reconcile(&self) -> Result<()> {
        lock(&self.state)?.uncertain = true;
        let image = Self::load(self.journal.as_ref())?;
        self.validate(&image)?;
        {
            let mut state = lock(&self.state)?;
            state.image = image.clone();
            state.uncertain = true;
        }
        if let Some(receipt) = image.retired_transaction {
            let id = SecretTransactionId::from_protocol_bytes(receipt)
                .ok_or(CustodyError::Unavailable)?;
            if !matches!(
                self.store.state(id).await,
                Err(connector_secrets::PreparedSecretError::Retired)
            ) {
                return Err(CustodyError::Unavailable);
            }
        }
        let Some(pending) = image.pending else {
            // Includes an initial write that did not become durable: no prepare was permitted.
            self.resolve_live(false)?;
            lock(&self.state)?.uncertain = false;
            return Ok(());
        };
        let observed = self.store.state(pending.id()?).await;
        match pending.phase {
            Phase::Preparing => match observed {
                Ok(SecretTransactionState::Absent | SecretTransactionState::Prepared) => {
                    self.abort_pending().await
                }
                _ => Err(CustodyError::Unavailable),
            },
            Phase::Decided { .. } => match observed {
                Ok(SecretTransactionState::Prepared | SecretTransactionState::Committed) => {
                    self.commit_decided().await
                }
                _ => Err(CustodyError::Unavailable),
            },
            Phase::Published { .. } => match observed {
                Ok(SecretTransactionState::Committed)
                | Err(connector_secrets::PreparedSecretError::Retired) => {
                    lock(&self.state)?.uncertain = false;
                    self.resolve_live(true)?;
                    self.retire_pending().await
                }
                _ => Err(CustodyError::Unavailable),
            },
            Phase::Aborted => match observed {
                Ok(SecretTransactionState::Absent)
                | Err(connector_secrets::PreparedSecretError::Retired) => {
                    self.resolve_live(false)?;
                    self.retire_pending().await
                }
                _ => Err(CustodyError::Unavailable),
            },
        }
    }
}
/// Project only an already authorized receiver's lifecycle. Guarded ownership is unavailable,
/// never an endpoint-free Pending status or an invented terminal outcome.
pub(super) fn project_session(
    sessions: &ConnectSessionLifecycle,
    session: &str,
) -> Result<Option<protocol::connection::ConnectSessionStatus>> {
    match sessions.status(session) {
        None if sessions.owns(session) => Err(CustodyError::Unavailable),
        status => Ok(status),
    }
}

fn lock<T: ?Sized>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>> {
    mutex.lock().map_err(|_| CustodyError::Unavailable)
}

pub(super) trait RefreshClock: Send + Sync {
    fn now(&self) -> Result<(u64, Instant)>;
}
/// Real receiver clock; tests install a controlled receiver at owner construction.
pub(super) struct SystemRefreshClock;
impl RefreshClock for SystemRefreshClock {
    fn now(&self) -> Result<(u64, Instant)> {
        let wall = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| CustodyError::Unavailable)?;
        let wall = u64::try_from(wall.as_millis()).map_err(|_| CustodyError::Unavailable)?;
        Ok((wall, Instant::now()))
    }
}
/// Current operation/grant admission. Mutations use this receiver's same mutex; callbacks perform
/// no I/O or custody locking. There is no default grant or caller-supplied authorization instant.
pub(super) trait RefreshAuthority: Send {
    fn recheck(
        &mut self,
        operation: &str,
        identity: &Identity,
        generation: u64,
        evidence: &Evidence,
        now: u64,
    ) -> bool;
}
pub(super) struct RefreshOwner {
    binding: Binding,
    clock: Arc<dyn RefreshClock>,
    authority: Arc<Mutex<dyn RefreshAuthority>>,
    gate: Arc<tokio::sync::Mutex<()>>,
}
struct RefreshLease {
    owner: Arc<RefreshOwner>,
    _gate: tokio::sync::OwnedMutexGuard<()>,
}
/// Acquisition/read/refresh owners share this binding gate. The caller can retain it through the
/// subsequent operation; an unresolved completion retains its lease until reconciliation.
pub(super) struct RefreshGuard {
    lease: Arc<RefreshLease>,
    begun: std::sync::atomic::AtomicBool,
}
/// Neither cloneable nor serializable. Its original window and target survive every awaited step.
pub(super) struct RefreshCompletion {
    lease: Arc<RefreshLease>,
    operation: String,
    previous: Publication,
    start: u64,
    deadline: u64,
    monotonic_start: Instant,
    monotonic_deadline: Instant,
    claimed: bool,
}
impl RefreshOwner {
    pub(super) fn new(
        binding: Binding,
        clock: Arc<dyn RefreshClock>,
        authority: Arc<Mutex<dyn RefreshAuthority>>,
    ) -> Self {
        Self {
            binding,
            clock,
            authority,
            gate: Arc::new(tokio::sync::Mutex::new(())),
        }
    }
    pub(super) async fn lock(self: &Arc<Self>) -> RefreshGuard {
        let gate = self.gate.clone().lock_owned().await;
        RefreshGuard {
            lease: Arc::new(RefreshLease {
                owner: self.clone(),
                _gate: gate,
            }),
            begun: std::sync::atomic::AtomicBool::new(false),
        }
    }
}
impl RefreshGuard {
    /// Before any refresh egress, admit under this gate. Old access expiry does not invalidate
    /// refresh authority: the current operation grant is checked explicitly by the receiver.
    pub(super) fn begin(
        &self,
        custody: &CustodyOwner,
        operation: &str,
    ) -> Result<RefreshCompletion> {
        if self.begun.swap(true, std::sync::atomic::Ordering::SeqCst) || !valid_ref(operation) {
            return Err(CustodyError::Refused);
        }
        let owner = &self.lease.owner;
        let previous = custody
            .snapshot(&owner.binding.identity)?
            .ok_or(CustodyError::Refused)?;
        let mut authority = lock(&owner.authority)?;
        let (start, monotonic) = owner.clock.now()?;
        let deadline = start.checked_add(30_000).ok_or(CustodyError::Refused)?;
        let monotonic_deadline = monotonic
            .checked_add(Duration::from_secs(30))
            .ok_or(CustodyError::Refused)?;
        if start == 0
            || !authority.recheck(
                operation,
                &previous.identity,
                previous.generation,
                &previous.evidence,
                start,
            )
        {
            return Err(CustodyError::Refused);
        }
        Ok(RefreshCompletion {
            lease: self.lease.clone(),
            operation: operation.to_owned(),
            previous,
            start,
            deadline,
            monotonic_start: monotonic,
            monotonic_deadline,
            claimed: false,
        })
    }
}
impl RefreshCompletion {
    /// The acquisition backend wraps token/evidence I/O with this same remaining budget.
    pub(super) fn remaining(&self) -> Result<Duration> {
        let (now, monotonic) = self.lease.owner.clock.now()?;
        self.remaining_at(now, monotonic)
    }
    fn remaining_at(&self, now: u64, monotonic: Instant) -> Result<Duration> {
        if self.claimed
            || now < self.start
            || now >= self.deadline
            || monotonic < self.monotonic_start
        {
            return Err(CustodyError::Refused);
        }
        self.monotonic_deadline
            .checked_duration_since(monotonic)
            .filter(|remaining| !remaining.is_zero())
            .ok_or(CustodyError::Refused)
    }
    fn claim(&mut self, pending: &Pending) -> Result<(u64, u64)> {
        if self.claimed
            || pending.previous_generation != self.previous.generation
            || pending.publication.identity != self.previous.identity
        {
            return Err(CustodyError::Refused);
        }
        let evidence = &pending.publication.evidence;
        let previous = &self.previous.evidence;
        if evidence.client_digest != previous.client_digest
            || evidence.origin_digest != previous.origin_digest
            || evidence.authority_digest != previous.authority_digest
            || evidence.subject != previous.subject
            || evidence.ceiling != previous.ceiling
        {
            return Err(CustodyError::Refused);
        }
        let mut authority = lock(&self.lease.owner.authority)?;
        let (now, monotonic) = self.lease.owner.clock.now()?;
        self.remaining_at(now, monotonic)?;
        if evidence.observed_at > now
            || now >= evidence.expires_at
            || !authority.recheck(
                &self.operation,
                &self.previous.identity,
                self.previous.generation,
                evidence,
                now,
            )
        {
            return Err(CustodyError::Refused);
        }
        self.claimed = true;
        Ok((now, self.deadline))
    }
}
impl CustodyOwner {
    pub(super) async fn complete_refresh(
        &self,
        binding: &Binding,
        evidence: Evidence,
        proposal: Proposal,
        refresh: RefreshCompletion,
    ) -> Result<Publication> {
        self.complete_claim(
            binding,
            refresh.previous.generation,
            evidence,
            proposal,
            CompletionClaim::Refresh(Box::new(refresh)),
            None,
        )
        .await
    }
}

enum CompletionClaim {
    Session(LiveCompletion),
    Refresh(Box<RefreshCompletion>),
}
impl CompletionClaim {
    fn connection(&self) -> &str {
        match self {
            Self::Session(live) => &live.connection,
            Self::Refresh(refresh) => &refresh.previous.identity.connection,
        }
    }
    fn recovery(&self) -> Result<()> {
        match self {
            Self::Session(live) => live.recovery(),
            Self::Refresh(_) => Ok(()), // The custody image itself stays guarded/uncertain.
        }
    }
    fn resolve(&self, published: bool) -> Result<()> {
        match self {
            Self::Session(live) => live.resolve(published),
            Self::Refresh(_) => Ok(()), // Refresh has no terminal session to publish.
        }
    }
    fn claim(
        &mut self,
        pending: &Pending,
        authority: Option<&dyn CompletionAuthority>,
    ) -> Result<(u64, u64)> {
        match self {
            Self::Refresh(refresh) => refresh.claim(pending),
            Self::Session(live) => {
                let authority = authority.ok_or(CustodyError::Refused)?;
                lock(&live.sessions)?
                    .claim_completion(&live.session, &live.connection, |now| {
                        pending.publication.evidence.observed_at <= now
                            && now < pending.publication.evidence.expires_at
                            && authority.recheck(
                                &pending.publication.identity,
                                pending.previous_generation,
                                &pending.publication.evidence,
                                now,
                            )
                    })
                    .map_err(|_| CustodyError::Refused)
            }
        }
    }
}

#[cfg(test)]
#[path = "custody_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "custody_refresh_tests.rs"]
mod refresh_tests;

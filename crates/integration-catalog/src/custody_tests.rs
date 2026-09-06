use super::*;
use connector_secrets::{FileStore, SecretStore};
use std::os::unix::fs::PermissionsExt as _;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

fn binding() -> Binding {
    Binding::new(
        Identity {
            owner: "owner:one".into(),
            integration: "gitlab".into(),
            connection: "connection:one".into(),
            purpose: "oauth".into(),
            store: "oauth-development-file".into(),
            address_digest: [0; 32],
        },
        CredentialRef::new("1234abcd", "com.gitlab.api", "api", "oauth_access").unwrap(),
        CredentialRef::new("1234abcd", "com.gitlab.api", "api", "oauth_refresh").unwrap(),
    )
    .unwrap()
}
fn evidence() -> Evidence {
    Evidence {
        scopes: BTreeSet::from(["read_api".into()]),
        ceiling: BTreeSet::from(["read_api".into()]),
        subject: "subject:one".into(),
        client_digest: [1; 32],
        origin_digest: [2; 32],
        authority_digest: [3; 32],
        observed_at: 1,
        expires_at: 1000,
    }
}
struct Authority(AtomicBool);
impl CompletionAuthority for Authority {
    fn recheck(&self, _: &Identity, _: u64, _: &Evidence, _: u64) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
struct Fixture {
    directory: tempfile::TempDir,
    binding: Binding,
    clock: Arc<AtomicU64>,
    sessions: Arc<Mutex<ConnectSessionLifecycle>>,
    store: Arc<FileStore>,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let store = Arc::new(FileStore::open(directory.path().join("credentials")).unwrap());
        let binding = binding();
        let clock = Arc::new(AtomicU64::new(99));
        let observed = clock.clone();
        let mut sessions = ConnectSessionLifecycle::with_clock("gitlab", 1, move || {
            Ok(observed.load(Ordering::SeqCst))
        })
        .unwrap();
        sessions
            .reserve(
                "session:one".into(),
                "display".into(),
                100,
                "/run/fixture.sock".into(),
            )
            .unwrap();
        sessions
            .bind_completion_target("session:one", &binding.identity.connection)
            .unwrap();
        Self {
            directory,
            binding,
            clock,
            sessions: Arc::new(Mutex::new(sessions)),
            store,
        }
    }
    async fn complete(&self, owner: &CustodyOwner) -> Result<Publication> {
        owner
            .complete(
                &self.binding,
                0,
                evidence(),
                self.proposal(),
                self.live(),
                &Authority(AtomicBool::new(true)),
            )
            .await
    }
    fn journal(&self) -> FullJournal {
        FullJournal::open(&self.directory.path().join("journal.db")).unwrap()
    }
    fn live(&self) -> LiveCompletion {
        let (live, paths) = LiveCompletion::begin(
            self.sessions.clone(),
            "session:one".into(),
            self.binding.identity.connection.clone(),
        )
        .unwrap();
        assert_eq!(paths, vec!["/run/fixture.sock"]);
        live
    }
    fn proposal(&self) -> Proposal {
        Proposal::new(&self.binding, Secret::new("ACCESS-SENTINEL"), None).unwrap()
    }
}

#[tokio::test]
async fn a_full_decision_precedes_secret_commit_and_coherent_publication() {
    let fixture = Fixture::new();
    fixture
        .store
        .put(
            &fixture.binding.refresh,
            &Secret::new("OLD-REFRESH-SENTINEL"),
        )
        .await
        .unwrap();
    let owner = CustodyOwner::open(
        fixture.journal(),
        fixture.store.clone(),
        vec![fixture.binding.clone()],
    )
    .unwrap();
    let result = fixture.complete(&owner).await;
    assert!(
        result.is_ok(),
        "completion must publish after the FULL decision"
    );
    assert!(owner.snapshot(&fixture.binding.identity).unwrap().is_some());
    assert_eq!(
        fixture
            .store
            .get(&fixture.binding.access)
            .await
            .unwrap()
            .expose_secret(),
        "ACCESS-SENTINEL"
    );
    assert!(fixture
        .store
        .get(&fixture.binding.refresh)
        .await
        .unwrap_err()
        .is_not_found());
    assert_eq!(
        fixture
            .sessions
            .lock()
            .unwrap()
            .status("session:one")
            .unwrap()
            .state,
        protocol::connection::ConnectSessionState::Completed
    );
}

struct DecisionFault {
    inner: FullJournal,
    clock: Arc<AtomicU64>,
    fired: AtomicBool,
    durable_error: bool,
}
impl JournalIo for DecisionFault {
    fn read(&self) -> Result<Option<Vec<u8>>> {
        self.inner.read()
    }
    fn write(&self, bytes: &[u8]) -> Result<()> {
        let image: Image = serde_json::from_slice(bytes).unwrap();
        if image
            .pending
            .as_ref()
            .is_some_and(|pending| matches!(pending.phase, Phase::Decided { .. }))
            && !self.fired.swap(true, Ordering::SeqCst)
        {
            self.clock.store(101, Ordering::SeqCst);
            self.inner.write(bytes)?;
            if self.durable_error {
                return Err(CustodyError::Unavailable);
            }
            return Ok(());
        }
        self.inner.write(bytes)
    }
}

#[tokio::test]
async fn a_claimed_decision_write_can_finish_after_the_authorization_deadline() {
    let fixture = Fixture::new();
    let journal = Arc::new(DecisionFault {
        inner: fixture.journal(),
        clock: fixture.clock.clone(),
        fired: AtomicBool::new(false),
        durable_error: false,
    });
    let owner = CustodyOwner::from_journal(
        journal.clone(),
        fixture.store.clone(),
        vec![fixture.binding.clone()],
    )
    .unwrap();
    let result = fixture.complete(&owner).await;
    assert!(
        result.is_ok(),
        "a timely claim survives delayed journal I/O"
    );
    assert!(journal.fired.load(Ordering::SeqCst));
    assert_eq!(fixture.clock.load(Ordering::SeqCst), 101);
}

#[tokio::test]
async fn a_durable_but_error_decision_is_recovered_without_an_immediate_secret_commit() {
    let fixture = Fixture::new();
    let journal = Arc::new(DecisionFault {
        inner: fixture.journal(),
        clock: fixture.clock.clone(),
        fired: AtomicBool::new(false),
        durable_error: true,
    });
    let owner = CustodyOwner::from_journal(
        journal.clone(),
        fixture.store.clone(),
        vec![fixture.binding.clone()],
    )
    .unwrap();
    assert!(matches!(
        fixture.complete(&owner).await,
        Err(CustodyError::Unavailable)
    ));
    assert!(
        journal.fired.load(Ordering::SeqCst),
        "the decision must reach the FULL journal"
    );
    assert!(fixture
        .store
        .get(&fixture.binding.access)
        .await
        .unwrap_err()
        .is_not_found());
    assert!(fixture
        .sessions
        .lock()
        .unwrap()
        .status("session:one")
        .is_none());
    assert!(matches!(
        owner.snapshot(&fixture.binding.identity),
        Err(CustodyError::Unavailable)
    ));
    owner.recover().await.unwrap();
    assert_eq!(
        fixture
            .store
            .get(&fixture.binding.access)
            .await
            .unwrap()
            .expose_secret(),
        "ACCESS-SENTINEL"
    );
}

use connector_secrets::{PreparedSecretError, StoreError};
use std::sync::atomic::AtomicUsize;

struct WriteFault {
    inner: FullJournal,
    at: usize,
    after: bool,
    writes: AtomicUsize,
}
impl JournalIo for WriteFault {
    fn read(&self) -> Result<Option<Vec<u8>>> {
        self.inner.read()
    }
    fn write(&self, bytes: &[u8]) -> Result<()> {
        if self.writes.fetch_add(1, Ordering::SeqCst) + 1 == self.at {
            if self.after {
                self.inner.write(bytes)?;
            }
            return Err(CustodyError::Unavailable);
        }
        self.inner.write(bytes)
    }
}

#[tokio::test]
async fn every_journal_write_boundary_recovers_real_sqlite_and_file_store_after_reopen() {
    for at in 1..=4 {
        for after in [false, true] {
            let fixture = Fixture::new();
            let journal = Arc::new(WriteFault {
                inner: fixture.journal(),
                at,
                after,
                writes: AtomicUsize::new(0),
            });
            let owner = CustodyOwner::from_journal(
                journal.clone(),
                fixture.store.clone(),
                vec![fixture.binding.clone()],
            )
            .unwrap();
            assert!(matches!(
                fixture.complete(&owner).await,
                Err(CustodyError::Unavailable)
            ));
            let decided = at > 2 || at == 2 && after;
            if at < 4 {
                assert!(fixture
                    .sessions
                    .lock()
                    .unwrap()
                    .status("session:one")
                    .is_none());
            }
            drop(owner);
            drop(journal);
            let Fixture {
                directory,
                binding,
                store,
                ..
            } = fixture;
            drop(store);
            let store = Arc::new(FileStore::open(directory.path().join("credentials")).unwrap());
            let owner = CustodyOwner::open(
                FullJournal::open(&directory.path().join("journal.db")).unwrap(),
                store.clone(),
                vec![binding.clone()],
            )
            .unwrap();
            owner.recover().await.unwrap();
            owner.recover().await.unwrap();
            let snapshot = owner.snapshot(&binding.identity).unwrap();
            assert_eq!(snapshot.is_some(), decided, "write {at}, after {after}");
            if decided {
                let publication = snapshot.unwrap();
                assert_eq!(publication.generation, 1);
                assert!(publication.evidence == evidence());
                assert_eq!(
                    store.get(&binding.access).await.unwrap().expose_secret(),
                    "ACCESS-SENTINEL"
                );
            } else {
                assert!(store.get(&binding.access).await.unwrap_err().is_not_found());
            }
            assert!(owner.state.lock().unwrap().image.pending.is_none());
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StoreFault {
    None,
    PrepareBefore,
    PrepareAfter,
    CommitBefore,
    CommitAfter,
    AbortBefore,
    AbortAfter,
    ReclaimBefore,
    ReclaimAfter,
    HoldPrepare,
    HoldCommit,
}
struct CheckedStore {
    inner: Arc<FileStore>,
    journal: Arc<dyn JournalIo>,
    fault: StoreFault,
    fired: AtomicBool,
    state_unavailable: AtomicBool,
    prepares: AtomicUsize,
    commits: AtomicUsize,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
impl CheckedStore {
    fn new(inner: Arc<FileStore>, journal: Arc<dyn JournalIo>, fault: StoreFault) -> Self {
        Self {
            inner,
            journal,
            fault,
            fired: AtomicBool::new(false),
            state_unavailable: AtomicBool::new(false),
            prepares: AtomicUsize::new(0),
            commits: AtomicUsize::new(0),
            entered: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
        }
    }
    fn trigger(&self, fault: StoreFault) -> bool {
        self.fault == fault && !self.fired.swap(true, Ordering::SeqCst)
    }
    fn image(&self) -> Image {
        serde_json::from_slice(&self.journal.read().unwrap().unwrap()).unwrap()
    }
}
#[async_trait::async_trait]
impl SecretStore for CheckedStore {
    async fn ready(&self) -> std::result::Result<(), StoreError> {
        self.inner.ready().await
    }
    async fn get(&self, reference: &CredentialRef) -> std::result::Result<Secret, StoreError> {
        self.inner.get(reference).await
    }
    async fn put(
        &self,
        reference: &CredentialRef,
        secret: &Secret,
    ) -> std::result::Result<(), StoreError> {
        self.inner.put(reference, secret).await
    }
    async fn delete(&self, reference: &CredentialRef) -> std::result::Result<(), StoreError> {
        self.inner.delete(reference).await
    }
}
#[async_trait::async_trait]
impl PreparedSecretStore for CheckedStore {
    async fn prepare(
        &self,
        id: SecretTransactionId,
        digest: SecretProposalDigest,
        batch: &SecretBatch,
    ) -> std::result::Result<SecretTransactionState, PreparedSecretError> {
        self.prepares.fetch_add(1, Ordering::SeqCst);
        let pending = self.image().pending.unwrap();
        assert!(
            matches!(pending.phase, Phase::Preparing),
            "prepare requires its prior durable id"
        );
        assert_eq!(pending.transaction, id.protocol_bytes());
        assert_eq!(pending.digest, digest.protocol_bytes());
        if self.trigger(StoreFault::PrepareBefore) {
            return Err(PreparedSecretError::Backend);
        }
        let result = self.inner.prepare(id, digest, batch).await;
        if self.trigger(StoreFault::HoldPrepare) {
            self.entered.notify_one();
            self.release.notified().await;
        }
        if self.trigger(StoreFault::PrepareAfter) {
            return Err(PreparedSecretError::Backend);
        }
        result
    }
    async fn state(
        &self,
        id: SecretTransactionId,
    ) -> std::result::Result<SecretTransactionState, PreparedSecretError> {
        if self.state_unavailable.load(Ordering::SeqCst) {
            return Err(PreparedSecretError::Backend);
        }
        self.inner.state(id).await
    }
    async fn commit(
        &self,
        id: SecretTransactionId,
    ) -> std::result::Result<SecretTransactionState, PreparedSecretError> {
        self.commits.fetch_add(1, Ordering::SeqCst);
        let pending = self.image().pending.unwrap();
        assert!(
            matches!(pending.phase, Phase::Decided { authorized_at, deadline } if authorized_at > 0 && authorized_at < deadline),
            "secret commit requires a confirmed timely durable decision"
        );
        if self.trigger(StoreFault::HoldCommit) {
            self.entered.notify_one();
            self.release.notified().await;
        }
        if self.trigger(StoreFault::CommitBefore) {
            return Err(PreparedSecretError::Backend);
        }
        let result = self.inner.commit(id).await;
        if self.trigger(StoreFault::CommitAfter) {
            return Err(PreparedSecretError::Backend);
        }
        result
    }
    async fn abort(
        &self,
        id: SecretTransactionId,
    ) -> std::result::Result<SecretTransactionState, PreparedSecretError> {
        assert!(matches!(
            self.image().pending.unwrap().phase,
            Phase::Preparing
        ));
        if self.trigger(StoreFault::AbortBefore) {
            return Err(PreparedSecretError::Backend);
        }
        let result = self.inner.abort(id).await;
        if self.trigger(StoreFault::AbortAfter) {
            return Err(PreparedSecretError::Backend);
        }
        result
    }
    async fn reclaim(
        &self,
        generation: SecretTransactionGeneration,
    ) -> std::result::Result<(), PreparedSecretError> {
        assert!(matches!(
            self.image().pending.unwrap().phase,
            Phase::Published { .. } | Phase::Aborted
        ));
        if self.trigger(StoreFault::ReclaimBefore) {
            return Err(PreparedSecretError::Backend);
        }
        let result = self.inner.reclaim(generation).await;
        if self.trigger(StoreFault::ReclaimAfter) {
            return Err(PreparedSecretError::Backend);
        }
        result
    }
}

#[tokio::test]
async fn uncertain_secret_operations_are_resolved_from_state_and_never_assumed_rolled_back() {
    for fault in [
        StoreFault::None,
        StoreFault::PrepareBefore,
        StoreFault::PrepareAfter,
        StoreFault::CommitBefore,
        StoreFault::CommitAfter,
        StoreFault::AbortBefore,
        StoreFault::AbortAfter,
        StoreFault::ReclaimBefore,
        StoreFault::ReclaimAfter,
    ] {
        let fixture = Fixture::new();
        let journal = Arc::new(fixture.journal());
        let store = Arc::new(CheckedStore::new(
            fixture.store.clone(),
            journal.clone(),
            fault,
        ));
        let owner =
            CustodyOwner::from_journal(journal, store.clone(), vec![fixture.binding.clone()])
                .unwrap();
        let permitted = !matches!(fault, StoreFault::AbortBefore | StoreFault::AbortAfter);
        let result = owner
            .complete(
                &fixture.binding,
                0,
                evidence(),
                fixture.proposal(),
                fixture.live(),
                &Authority(AtomicBool::new(permitted)),
            )
            .await;
        if matches!(
            fault,
            StoreFault::CommitBefore | StoreFault::AbortBefore | StoreFault::ReclaimBefore
        ) {
            assert!(matches!(result, Err(CustodyError::Unavailable)));
        }
        owner.recover().await.unwrap();
        let published =
            permitted && !matches!(fault, StoreFault::PrepareBefore | StoreFault::PrepareAfter);
        assert_eq!(
            owner.snapshot(&fixture.binding.identity).unwrap().is_some(),
            published
        );
        assert_eq!(store.prepares.load(Ordering::SeqCst), 1);
        if !published {
            assert_eq!(store.commits.load(Ordering::SeqCst), 0);
        }
        let state = fixture
            .sessions
            .lock()
            .unwrap()
            .status("session:one")
            .unwrap()
            .state;
        assert_eq!(
            state == protocol::connection::ConnectSessionState::Completed,
            published
        );
    }
}

#[tokio::test]
async fn dropping_the_future_at_prepared_or_decided_boundaries_leaves_recoverable_ownership() {
    for fault in [StoreFault::HoldPrepare, StoreFault::HoldCommit] {
        let fixture = Fixture::new();
        let journal = Arc::new(fixture.journal());
        let store = Arc::new(CheckedStore::new(
            fixture.store.clone(),
            journal.clone(),
            fault,
        ));
        let owner =
            CustodyOwner::from_journal(journal, store.clone(), vec![fixture.binding.clone()])
                .unwrap();
        let authority = Authority(AtomicBool::new(true));
        let mut future = Box::pin(owner.complete(
            &fixture.binding,
            0,
            evidence(),
            fixture.proposal(),
            fixture.live(),
            &authority,
        ));
        tokio::select! {
            _ = store.entered.notified() => {},
            _ = &mut future => panic!("fault boundary must suspend completion"),
        }
        drop(future);
        fixture.clock.store(101, Ordering::SeqCst);
        assert!(fixture
            .sessions
            .lock()
            .unwrap()
            .status("session:one")
            .is_none());
        assert!(fixture.sessions.lock().unwrap().fail_pending().is_empty());
        owner.recover().await.unwrap();
        assert_eq!(
            owner.snapshot(&fixture.binding.identity).unwrap().is_some(),
            fault == StoreFault::HoldCommit
        );
    }
}

#[tokio::test]
async fn expiry_or_revocation_before_the_claim_aborts_without_a_decision() {
    for expired in [false, true] {
        let fixture = Fixture::new();
        let journal = Arc::new(fixture.journal());
        let store = Arc::new(CheckedStore::new(
            fixture.store.clone(),
            journal.clone(),
            StoreFault::HoldPrepare,
        ));
        let owner =
            CustodyOwner::from_journal(journal, store.clone(), vec![fixture.binding.clone()])
                .unwrap();
        let authority = Authority(AtomicBool::new(true));
        let mut future = Box::pin(owner.complete(
            &fixture.binding,
            0,
            evidence(),
            fixture.proposal(),
            fixture.live(),
            &authority,
        ));
        tokio::select! { _ = store.entered.notified() => {}, _ = &mut future => panic!("prepare barrier") }
        if expired {
            fixture.clock.store(100, Ordering::SeqCst);
        } else {
            authority.0.store(false, Ordering::SeqCst);
        }
        store.release.notify_one();
        assert!(matches!(future.await, Err(CustodyError::Refused)));
        assert_eq!(store.commits.load(Ordering::SeqCst), 0);
        assert!(owner.snapshot(&fixture.binding.identity).unwrap().is_none());
        assert_eq!(
            fixture
                .sessions
                .lock()
                .unwrap()
                .status("session:one")
                .unwrap()
                .state,
            if expired {
                protocol::connection::ConnectSessionState::Expired
            } else {
                protocol::connection::ConnectSessionState::Failed
            }
        );
    }
}

struct HeldDecision {
    inner: FullJournal,
    entered: std::sync::Barrier,
    release: std::sync::Barrier,
}
impl JournalIo for HeldDecision {
    fn read(&self) -> Result<Option<Vec<u8>>> {
        self.inner.read()
    }
    fn write(&self, bytes: &[u8]) -> Result<()> {
        let image: Image = serde_json::from_slice(bytes).unwrap();
        if image
            .pending
            .as_ref()
            .is_some_and(|pending| matches!(pending.phase, Phase::Decided { .. }))
        {
            self.entered.wait();
            self.release.wait();
        }
        self.inner.write(bytes)
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_held_full_write_blocks_commit_but_not_private_status_or_expiry_checks() {
    let fixture = Fixture::new();
    let journal = Arc::new(HeldDecision {
        inner: fixture.journal(),
        entered: std::sync::Barrier::new(2),
        release: std::sync::Barrier::new(2),
    });
    let store = Arc::new(CheckedStore::new(
        fixture.store.clone(),
        journal.clone(),
        StoreFault::None,
    ));
    let owner = Arc::new(
        CustodyOwner::from_journal(
            journal.clone(),
            store.clone(),
            vec![fixture.binding.clone()],
        )
        .unwrap(),
    );
    let binding = fixture.binding.clone();
    let proposal = fixture.proposal();
    let live = fixture.live();
    let run_owner = owner.clone();
    let task = tokio::spawn(async move {
        run_owner
            .complete(
                &binding,
                0,
                evidence(),
                proposal,
                live,
                &Authority(AtomicBool::new(true)),
            )
            .await
    });
    let wait_journal = journal.clone();
    tokio::task::spawn_blocking(move || wait_journal.entered.wait())
        .await
        .unwrap();
    fixture.clock.store(101, Ordering::SeqCst);
    assert_eq!(store.commits.load(Ordering::SeqCst), 0);
    assert!(fixture
        .sessions
        .lock()
        .unwrap()
        .status("session:one")
        .is_none());
    assert!(fixture.sessions.lock().unwrap().fail_pending().is_empty());
    assert!(matches!(
        owner.snapshot(&fixture.binding.identity),
        Err(CustodyError::Unavailable)
    ));
    assert!(fixture
        .store
        .get(&fixture.binding.access)
        .await
        .unwrap_err()
        .is_not_found());
    assert!(matches!(
        project_session(&fixture.sessions.lock().unwrap(), "session:one"),
        Err(CustodyError::Unavailable)
    ));
    journal.release.wait();
    assert!(task.await.unwrap().is_ok());
    assert_eq!(store.commits.load(Ordering::SeqCst), 1);
    assert_eq!(
        project_session(&fixture.sessions.lock().unwrap(), "session:one")
            .unwrap()
            .unwrap()
            .state,
        protocol::connection::ConnectSessionState::Completed
    );
}

#[test]
fn distinct_bindings_cannot_alias_the_same_reserved_credential_addresses() {
    let fixture = Fixture::new();
    let mut other = fixture.binding.clone();
    other.identity.connection = "connection:other".into();
    assert!(CustodyOwner::open(
        fixture.journal(),
        fixture.store.clone(),
        vec![fixture.binding.clone(), other]
    )
    .is_err());
}

#[tokio::test]
async fn a_reclaimed_publication_retains_its_original_authorization_timing() {
    let fixture = Fixture::new();
    let owner = CustodyOwner::open(
        fixture.journal(),
        fixture.store.clone(),
        vec![fixture.binding.clone()],
    )
    .unwrap();
    fixture.complete(&owner).await.unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&owner.journal.read().unwrap().unwrap()).unwrap();
    assert_eq!(
        value["connections"]["connection:one"]["authorization"]["authorized_at"],
        99
    );
    assert_eq!(
        value["connections"]["connection:one"]["authorization"]["deadline"],
        100
    );
}

#[tokio::test]
async fn generation_exhaustion_refuses_before_io_and_resolves_its_private_guard() {
    let fixture = Fixture::new();
    let journal = fixture.journal();
    let image = Image {
        next_generation: u64::MAX,
        retired_through: u64::MAX - 1,
        retired_transaction: Some(
            SecretTransactionId::new(
                SecretTransactionGeneration::from_protocol_bytes((u64::MAX - 1).to_be_bytes())
                    .unwrap(),
                [7; 24],
            )
            .protocol_bytes(),
        ),
        ..Image::default()
    };
    journal.write(&serde_json::to_vec(&image).unwrap()).unwrap();
    let owner = CustodyOwner::open(
        journal,
        fixture.store.clone(),
        vec![fixture.binding.clone()],
    )
    .unwrap();
    fixture
        .store
        .reclaim(
            SecretTransactionGeneration::from_protocol_bytes((u64::MAX - 1).to_be_bytes()).unwrap(),
        )
        .await
        .unwrap();
    owner.recover().await.unwrap();
    assert!(fixture.complete(&owner).await.is_err());
    assert_eq!(
        fixture
            .sessions
            .lock()
            .unwrap()
            .status("session:one")
            .unwrap()
            .state,
        protocol::connection::ConnectSessionState::Failed
    );
    assert_eq!(owner.state.lock().unwrap().image.next_generation, u64::MAX);
}

#[tokio::test]
async fn malformed_or_inconsistent_decisions_never_recover_a_publication() {
    for case in 0..12 {
        let fixture = Fixture::new();
        let journal = Arc::new(WriteFault {
            inner: fixture.journal(),
            at: 2,
            after: true,
            writes: AtomicUsize::new(0),
        });
        let owner = CustodyOwner::from_journal(
            journal.clone(),
            fixture.store.clone(),
            vec![fixture.binding.clone()],
        )
        .unwrap();
        assert!(fixture.complete(&owner).await.is_err());
        let mut value: serde_json::Value =
            serde_json::from_slice(&journal.read().unwrap().unwrap()).unwrap();
        match case {
            0 => {
                value["pending"]["phase"]
                    .as_object_mut()
                    .unwrap()
                    .remove("authorized_at");
            }
            1 => value["pending"]["phase"]["authorized_at"] = 100.into(),
            2 => value["pending"]["publication"]["identity"]["owner"] = "other-owner".into(),
            3 => value["pending"]["publication"]["identity"]["store"] = "other-store".into(),
            4 => value["pending"]["digest"] = serde_json::json!([1, 2]),
            5 => value["pending"]["publication"]["generation"] = 0.into(),
            6 => value["pending"]["previous_generation"] = 9.into(),
            7 => value["next_generation"] = 9.into(),
            8 => value["unexpected"] = true.into(),
            9 => value["pending"]["publication"]["authorization"] = serde_json::Value::Null,
            10 => value["pending"]["publication"]["evidence"]["expires_at"] = 99.into(),
            11 => value["version"] = 2.into(),
            _ => unreachable!(),
        }
        journal
            .inner
            .write(&serde_json::to_vec(&value).unwrap())
            .unwrap();
        drop(owner);
        assert!(
            CustodyOwner::open(
                fixture.journal(),
                fixture.store.clone(),
                vec![fixture.binding.clone()]
            )
            .is_err(),
            "case {case}"
        );
        assert!(fixture
            .store
            .get(&fixture.binding.access)
            .await
            .unwrap_err()
            .is_not_found());
    }
}

#[tokio::test]
async fn a_preparing_committed_or_decided_absent_store_state_cannot_invent_authorization() {
    for decided in [false, true] {
        let fixture = Fixture::new();
        let journal = Arc::new(WriteFault {
            inner: fixture.journal(),
            at: 2,
            after: decided,
            writes: AtomicUsize::new(0),
        });
        let owner = CustodyOwner::from_journal(
            journal.clone(),
            fixture.store.clone(),
            vec![fixture.binding.clone()],
        )
        .unwrap();
        assert!(fixture.complete(&owner).await.is_err());
        let image = CustodyOwner::load(journal.as_ref()).unwrap();
        let id = image.pending.unwrap().id().unwrap();
        if decided {
            fixture.store.abort(id).await.unwrap();
        } else {
            fixture.store.commit(id).await.unwrap();
        }
        assert!(matches!(
            owner.recover().await,
            Err(CustodyError::Unavailable)
        ));
        assert!(matches!(
            owner.snapshot(&fixture.binding.identity),
            Err(CustodyError::Unavailable)
        ));
        assert!(fixture
            .sessions
            .lock()
            .unwrap()
            .status("session:one")
            .is_none());
    }
}

#[test]
fn proposal_bounds_digest_and_journal_serialization_keep_private_values_out() {
    let binding = binding();
    assert!(Proposal::new(&binding, Secret::new(""), None).is_err());
    assert!(Proposal::new(
        &binding,
        Secret::new("x"),
        Some(Secret::new("x".repeat(8193)))
    )
    .is_err());
    let missing = Proposal::new(&binding, Secret::new("ACCESS-SENTINEL"), None).unwrap();
    let present = Proposal::new(
        &binding,
        Secret::new("ACCESS-SENTINEL"),
        Some(Secret::new("REFRESH-SENTINEL")),
    )
    .unwrap();
    let changed = Proposal::new(&binding, Secret::new("OTHER-SENTINEL"), None).unwrap();
    assert_ne!(
        missing.digest.protocol_bytes(),
        present.digest.protocol_bytes()
    );
    assert_ne!(
        missing.digest.protocol_bytes(),
        changed.digest.protocol_bytes()
    );
    for invalid in [
        Evidence {
            observed_at: 0,
            ..evidence()
        },
        Evidence {
            expires_at: 1,
            ..evidence()
        },
        Evidence {
            scopes: BTreeSet::from(["x".repeat(257)]),
            ..evidence()
        },
        Evidence {
            client_digest: [0; 32],
            ..evidence()
        },
    ] {
        assert!(!invalid.valid());
    }
}

fn another_live(fixture: &Fixture, binding: &Binding) -> LiveCompletion {
    let clock = fixture.clock.clone();
    let mut sessions =
        ConnectSessionLifecycle::with_clock("gitlab", 1, move || Ok(clock.load(Ordering::SeqCst)))
            .unwrap();
    sessions
        .reserve(
            "session:two".into(),
            "another display label".into(),
            100,
            "/run/two.sock".into(),
        )
        .unwrap();
    sessions
        .bind_completion_target("session:two", &binding.identity.connection)
        .unwrap();
    LiveCompletion::begin(
        Arc::new(Mutex::new(sessions)),
        "session:two".into(),
        binding.identity.connection.clone(),
    )
    .unwrap()
    .0
}

#[tokio::test]
async fn one_unresolved_store_slot_blocks_a_second_binding_and_generation_allocation() {
    let fixture = Fixture::new();
    let mut identity = fixture.binding.identity.clone();
    identity.connection = "connection:two".into();
    let second = Binding::new(
        identity,
        CredentialRef::new("1234abcd", "com.gitlab.api", "api", "second_access").unwrap(),
        CredentialRef::new("1234abcd", "com.gitlab.api", "api", "second_refresh").unwrap(),
    )
    .unwrap();
    let journal = Arc::new(fixture.journal());
    let store = Arc::new(CheckedStore::new(
        fixture.store.clone(),
        journal.clone(),
        StoreFault::HoldPrepare,
    ));
    let owner = CustodyOwner::from_journal(
        journal,
        store.clone(),
        vec![fixture.binding.clone(), second.clone()],
    )
    .unwrap();
    let authority = Authority(AtomicBool::new(true));
    let mut first = Box::pin(owner.complete(
        &fixture.binding,
        0,
        evidence(),
        fixture.proposal(),
        fixture.live(),
        &authority,
    ));
    tokio::select! { _ = store.entered.notified() => {}, _ = &mut first => panic!("prepare barrier") }
    let mut next = Box::pin(owner.complete(
        &second,
        0,
        evidence(),
        Proposal::new(&second, Secret::new("SECOND-SENTINEL"), None).unwrap(),
        another_live(&fixture, &second),
        &authority,
    ));
    std::future::poll_fn(|cx| {
        assert!(std::future::Future::poll(next.as_mut(), cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    assert_eq!(store.prepares.load(Ordering::SeqCst), 1);
    drop(first);
    assert!(matches!(next.await, Err(CustodyError::Unavailable)));
    assert_eq!(store.prepares.load(Ordering::SeqCst), 1);
    assert_eq!(owner.state.lock().unwrap().image.next_generation, 2);
    owner.recover().await.unwrap();
    assert_eq!(owner.state.lock().unwrap().image.retired_through, 1);
    assert!(owner.snapshot(&second.identity).unwrap().is_none());
}

#[tokio::test]
async fn replacement_preserves_identity_and_publishes_only_same_generation_evidence() {
    let fixture = Fixture::new();
    let owner = CustodyOwner::open(
        fixture.journal(),
        fixture.store.clone(),
        vec![fixture.binding.clone()],
    )
    .unwrap();
    fixture.complete(&owner).await.unwrap();
    let reduced = Evidence {
        scopes: BTreeSet::new(),
        ..evidence()
    };
    let proposal = Proposal::new(
        &fixture.binding,
        Secret::new("NEW-ACCESS-SENTINEL"),
        Some(Secret::new("NEW-REFRESH-SENTINEL")),
    )
    .unwrap();
    let publication = owner
        .complete(
            &fixture.binding,
            1,
            reduced.clone(),
            proposal,
            another_live(&fixture, &fixture.binding),
            &Authority(AtomicBool::new(true)),
        )
        .await
        .unwrap();
    assert_eq!(publication.generation, 2);
    assert!(publication.identity == fixture.binding.identity && publication.evidence == reduced);
    let snapshot = owner.snapshot(&fixture.binding.identity).unwrap().unwrap();
    assert!(snapshot == publication);
    assert_eq!(
        fixture
            .store
            .get(&fixture.binding.access)
            .await
            .unwrap()
            .expose_secret(),
        "NEW-ACCESS-SENTINEL"
    );
    assert_eq!(
        fixture
            .store
            .get(&fixture.binding.refresh)
            .await
            .unwrap()
            .expose_secret(),
        "NEW-REFRESH-SENTINEL"
    );
    let bytes = owner.journal.read().unwrap().unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    for forbidden in [
        "ACCESS-SENTINEL",
        "REFRESH-SENTINEL",
        "session:one",
        "session:two",
        "/run/",
        "oauth_access",
        "oauth_refresh",
    ] {
        assert!(!text.contains(forbidden));
    }
    assert_eq!(
        format!(
            "{:?} / {}",
            CustodyError::Unavailable,
            CustodyError::Unavailable
        ),
        "Unavailable / credential custody requires reconciliation"
    );
}

#[tokio::test]
async fn mismatched_proposal_or_stale_prior_generation_never_prepares() {
    for mismatch in [false, true] {
        let fixture = Fixture::new();
        let journal = Arc::new(fixture.journal());
        let store = Arc::new(CheckedStore::new(
            fixture.store.clone(),
            journal.clone(),
            StoreFault::None,
        ));
        let owner =
            CustodyOwner::from_journal(journal, store.clone(), vec![fixture.binding.clone()])
                .unwrap();
        let mut other = fixture.binding.clone();
        other.identity.owner = "other".into();
        let proposal = if mismatch {
            Proposal::new(&other, Secret::new("OTHER-SENTINEL"), None).unwrap()
        } else {
            fixture.proposal()
        };
        assert!(matches!(
            owner
                .complete(
                    &fixture.binding,
                    u64::from(!mismatch),
                    evidence(),
                    proposal,
                    fixture.live(),
                    &Authority(AtomicBool::new(true))
                )
                .await,
            Err(CustodyError::Refused)
        ));
        assert_eq!(store.prepares.load(Ordering::SeqCst), 0);
        assert_eq!(owner.state.lock().unwrap().image.next_generation, 1);
    }
}

struct ReadFault {
    inner: Arc<dyn JournalIo>,
    unavailable: AtomicBool,
}
impl JournalIo for ReadFault {
    fn read(&self) -> Result<Option<Vec<u8>>> {
        if self.unavailable.load(Ordering::SeqCst) {
            return Err(CustodyError::Unavailable);
        }
        self.inner.read()
    }
    fn write(&self, bytes: &[u8]) -> Result<()> {
        self.inner.write(bytes)
    }
}
#[tokio::test]
async fn unknown_journal_or_secret_state_stays_unavailable_until_reconciled() {
    for journal_failure in [false, true] {
        let fixture = Fixture::new();
        let write_fault = Arc::new(WriteFault {
            inner: fixture.journal(),
            at: 2,
            after: true,
            writes: AtomicUsize::new(0),
        });
        let journal = Arc::new(ReadFault {
            inner: write_fault,
            unavailable: AtomicBool::new(false),
        });
        let store = Arc::new(CheckedStore::new(
            fixture.store.clone(),
            journal.clone(),
            StoreFault::None,
        ));
        let owner = CustodyOwner::from_journal(
            journal.clone(),
            store.clone(),
            vec![fixture.binding.clone()],
        )
        .unwrap();
        assert!(fixture.complete(&owner).await.is_err());
        journal.unavailable.store(journal_failure, Ordering::SeqCst);
        store
            .state_unavailable
            .store(!journal_failure, Ordering::SeqCst);
        assert!(matches!(
            owner.recover().await,
            Err(CustodyError::Unavailable)
        ));
        assert_eq!(store.commits.load(Ordering::SeqCst), 0);
        assert!(fixture
            .sessions
            .lock()
            .unwrap()
            .status("session:one")
            .is_none());
        assert!(fixture.sessions.lock().unwrap().fail_pending().is_empty());
        assert!(matches!(
            owner.snapshot(&fixture.binding.identity),
            Err(CustodyError::Unavailable)
        ));
        journal.unavailable.store(false, Ordering::SeqCst);
        store.state_unavailable.store(false, Ordering::SeqCst);
        owner.recover().await.unwrap();
        assert!(owner.snapshot(&fixture.binding.identity).unwrap().is_some());
    }
}

#[tokio::test]
async fn a_reopened_publication_is_unavailable_until_its_store_retirement_is_checked() {
    let fixture = Fixture::new();
    let owner = CustodyOwner::open(
        fixture.journal(),
        fixture.store.clone(),
        vec![fixture.binding.clone()],
    )
    .unwrap();
    fixture.complete(&owner).await.unwrap();
    drop(owner);
    let reopened = CustodyOwner::open(
        fixture.journal(),
        fixture.store.clone(),
        vec![fixture.binding.clone()],
    )
    .unwrap();
    assert!(matches!(
        reopened.snapshot(&fixture.binding.identity),
        Err(CustodyError::Unavailable)
    ));
    reopened.recover().await.unwrap();
    assert!(reopened
        .snapshot(&fixture.binding.identity)
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn a_missing_or_wrong_credential_store_cannot_restore_published_journal_evidence() {
    let fixture = Fixture::new();
    let owner = CustodyOwner::open(
        fixture.journal(),
        fixture.store.clone(),
        vec![fixture.binding.clone()],
    )
    .unwrap();
    fixture.complete(&owner).await.unwrap();
    drop(owner);
    let replacement =
        Arc::new(FileStore::open(fixture.directory.path().join("empty-replacement")).unwrap());
    let reopened = CustodyOwner::open(
        fixture.journal(),
        replacement,
        vec![fixture.binding.clone()],
    )
    .unwrap();
    assert!(matches!(
        reopened.recover().await,
        Err(CustodyError::Unavailable)
    ));
    assert!(matches!(
        reopened.snapshot(&fixture.binding.identity),
        Err(CustodyError::Unavailable)
    ));
}

#[tokio::test]
async fn journal_capacity_is_reserved_for_publication_before_secret_prepare() {
    let fixture = Fixture::new();
    let scopes: BTreeSet<_> = (0..128)
        .map(|n| format!("{n:03}{}", "s".repeat(250)))
        .collect();
    let heavy = Evidence {
        scopes: scopes.clone(),
        ceiling: scopes,
        ..evidence()
    };
    let mut image = Image::default();
    let mut bindings = vec![fixture.binding.clone()];
    let mut found = false;
    for n in 1_u64..64 {
        let mut identity = fixture.binding.identity.clone();
        identity.connection = format!("connection:history-{n}");
        let historical = Binding::new(
            identity,
            CredentialRef::new("1234abcd", "com.gitlab.api", "api", &format!("access_{n}"))
                .unwrap(),
            CredentialRef::new("1234abcd", "com.gitlab.api", "api", &format!("refresh_{n}"))
                .unwrap(),
        )
        .unwrap();
        image.connections.insert(
            historical.identity.connection.clone(),
            Publication {
                identity: historical.identity.clone(),
                generation: n,
                evidence: heavy.clone(),
                authorization: Some(Timing {
                    authorized_at: 99,
                    deadline: 100,
                }),
            },
        );
        bindings.push(historical);
        image.retired_through = n;
        image.next_generation = n + 1;
        image.retired_transaction = Some(
            SecretTransactionId::new(
                SecretTransactionGeneration::from_protocol_bytes(n.to_be_bytes()).unwrap(),
                [255; 24],
            )
            .protocol_bytes(),
        );
        let mut preparing = image.clone();
        preparing.next_generation += 1;
        preparing.pending = Some(Pending {
            transaction: SecretTransactionId::new(
                SecretTransactionGeneration::from_protocol_bytes((n + 1).to_be_bytes()).unwrap(),
                [255; 24],
            )
            .protocol_bytes(),
            digest: [255; 32],
            previous_generation: 0,
            publication: Publication {
                identity: fixture.binding.identity.clone(),
                generation: n + 1,
                evidence: heavy.clone(),
                authorization: None,
            },
            phase: Phase::Preparing,
        });
        let mut published = preparing.clone();
        let pending = published.pending.as_mut().unwrap();
        pending.phase = Phase::Published {
            authorized_at: 99,
            deadline: 100,
        };
        pending.publication.authorization = Some(Timing {
            authorized_at: 99,
            deadline: 100,
        });
        published.connections.insert(
            fixture.binding.identity.connection.clone(),
            pending.publication.clone(),
        );
        if serde_json::to_vec(&preparing).unwrap().len() < MAX_BYTES
            && serde_json::to_vec(&published).unwrap().len() > MAX_BYTES
        {
            found = true;
            break;
        }
    }
    assert!(found, "fixture straddles the real journal byte bound");
    let journal = Arc::new(fixture.journal());
    journal.write(&serde_json::to_vec(&image).unwrap()).unwrap();
    fixture
        .store
        .reclaim(
            SecretTransactionGeneration::from_protocol_bytes(image.retired_through.to_be_bytes())
                .unwrap(),
        )
        .await
        .unwrap();
    let store = Arc::new(CheckedStore::new(
        fixture.store.clone(),
        journal.clone(),
        StoreFault::None,
    ));
    let owner = CustodyOwner::from_journal(journal, store.clone(), bindings).unwrap();
    owner.recover().await.unwrap();
    assert!(owner
        .complete(
            &fixture.binding,
            0,
            heavy,
            fixture.proposal(),
            fixture.live(),
            &Authority(AtomicBool::new(true))
        )
        .await
        .is_err());
    assert_eq!(
        store.prepares.load(Ordering::SeqCst),
        0,
        "publication capacity must be reserved before custody I/O"
    );
    assert_eq!(store.commits.load(Ordering::SeqCst), 0);
}

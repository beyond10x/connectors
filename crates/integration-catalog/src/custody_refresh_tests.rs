use super::*;
use connector_secrets::{FileStore, PreparedSecretError, SecretStore};
use std::os::unix::fs::PermissionsExt as _;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::Condvar;
use std::time::{Duration, Instant};

struct Clock {
    base: Instant,
    wall: AtomicU64,
    elapsed: AtomicU64,
    failed: AtomicBool,
    reads: AtomicUsize,
}
impl Clock {
    fn new() -> Self {
        Self {
            base: Instant::now(),
            wall: AtomicU64::new(1_000),
            elapsed: AtomicU64::new(0),
            failed: AtomicBool::new(false),
            reads: AtomicUsize::new(0),
        }
    }
    fn set(&self, wall: u64, elapsed: u64) {
        self.wall.store(wall, Ordering::SeqCst);
        self.elapsed.store(elapsed, Ordering::SeqCst);
    }
}
impl RefreshClock for Clock {
    fn now(&self) -> Result<(u64, Instant)> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.failed.load(Ordering::SeqCst) {
            return Err(CustodyError::Unavailable);
        }
        Ok((
            self.wall.load(Ordering::SeqCst),
            self.base + Duration::from_millis(self.elapsed.load(Ordering::SeqCst)),
        ))
    }
}

struct Authority {
    identity: Identity,
    generation: u64,
    allowed: bool,
    observations: Vec<(String, u64, u64)>,
}
impl RefreshAuthority for Authority {
    fn recheck(
        &mut self,
        operation: &str,
        identity: &Identity,
        generation: u64,
        evidence: &Evidence,
        now: u64,
    ) -> bool {
        self.observations
            .push((operation.to_owned(), generation, now));
        self.allowed
            && operation == "operation:read"
            && identity == &self.identity
            && generation == self.generation
            && evidence.ceiling == BTreeSet::from(["read_api".into()])
    }
}

#[derive(Default)]
struct Pause {
    entered: tokio::sync::Notify,
    released: Mutex<bool>,
    resume: Condvar,
}
impl Pause {
    fn wait(&self) {
        self.entered.notify_one();
        let mut released = self.released.lock().unwrap();
        while !*released {
            released = self.resume.wait(released).unwrap();
        }
    }
    fn release(&self) {
        *self.released.lock().unwrap() = true;
        self.resume.notify_all();
    }
}

struct Journal {
    inner: FullJournal,
    decision_mode: AtomicU8,
    pause: Arc<Pause>,
}
impl JournalIo for Journal {
    fn read(&self) -> Result<Option<Vec<u8>>> {
        self.inner.read()
    }
    fn write(&self, bytes: &[u8]) -> Result<()> {
        let image: Image = serde_json::from_slice(bytes).unwrap();
        let decision = image
            .pending
            .as_ref()
            .is_some_and(|pending| matches!(pending.phase, Phase::Decided { .. }));
        let mode = if decision {
            self.decision_mode.swap(0, Ordering::SeqCst)
        } else {
            0
        };
        if mode == 1 {
            return Err(CustodyError::Unavailable);
        }
        if mode == 3 {
            self.pause.wait();
        }
        self.inner.write(bytes)?;
        if mode == 2 {
            return Err(CustodyError::Unavailable);
        }
        Ok(())
    }
}

struct Store {
    inner: Arc<FileStore>,
    prepares: AtomicUsize,
    commits: AtomicUsize,
    aborts: AtomicUsize,
    pause_prepare: AtomicBool,
    pause_commit: AtomicBool,
    entered: tokio::sync::Notify,
    resume: tokio::sync::Notify,
}
#[async_trait::async_trait]
impl SecretStore for Store {
    async fn ready(&self) -> std::result::Result<(), connector_secrets::StoreError> {
        self.inner.ready().await
    }
    async fn get(
        &self,
        reference: &CredentialRef,
    ) -> std::result::Result<Secret, connector_secrets::StoreError> {
        self.inner.get(reference).await
    }
    async fn put(
        &self,
        reference: &CredentialRef,
        secret: &Secret,
    ) -> std::result::Result<(), connector_secrets::StoreError> {
        self.inner.put(reference, secret).await
    }
    async fn delete(
        &self,
        reference: &CredentialRef,
    ) -> std::result::Result<(), connector_secrets::StoreError> {
        self.inner.delete(reference).await
    }
}
#[async_trait::async_trait]
impl PreparedSecretStore for Store {
    async fn prepare(
        &self,
        id: SecretTransactionId,
        digest: SecretProposalDigest,
        batch: &SecretBatch,
    ) -> std::result::Result<SecretTransactionState, PreparedSecretError> {
        self.prepares.fetch_add(1, Ordering::SeqCst);
        let result = self.inner.prepare(id, digest, batch).await;
        if self.pause_prepare.swap(false, Ordering::SeqCst) {
            self.entered.notify_one();
            self.resume.notified().await;
        }
        result
    }
    async fn state(
        &self,
        id: SecretTransactionId,
    ) -> std::result::Result<SecretTransactionState, PreparedSecretError> {
        self.inner.state(id).await
    }
    async fn commit(
        &self,
        id: SecretTransactionId,
    ) -> std::result::Result<SecretTransactionState, PreparedSecretError> {
        self.commits.fetch_add(1, Ordering::SeqCst);
        let result = self.inner.commit(id).await;
        if self.pause_commit.swap(false, Ordering::SeqCst) {
            self.entered.notify_one();
            self.resume.notified().await;
        }
        result
    }
    async fn abort(
        &self,
        id: SecretTransactionId,
    ) -> std::result::Result<SecretTransactionState, PreparedSecretError> {
        self.aborts.fetch_add(1, Ordering::SeqCst);
        self.inner.abort(id).await
    }
    async fn reclaim(
        &self,
        through: SecretTransactionGeneration,
    ) -> std::result::Result<(), PreparedSecretError> {
        self.inner.reclaim(through).await
    }
}

struct Fixture {
    directory: tempfile::TempDir,
    binding: Binding,
    clock: Arc<Clock>,
    authority: Arc<Mutex<Authority>>,
    refresh: Arc<RefreshOwner>,
    journal: Arc<Journal>,
    store: Arc<Store>,
}
fn observation(expiry: u64) -> Evidence {
    Evidence {
        scopes: BTreeSet::from(["read_api".into()]),
        ceiling: BTreeSet::from(["read_api".into()]),
        subject: "subject:one".into(),
        client_digest: [1; 32],
        origin_digest: [2; 32],
        authority_digest: [3; 32],
        observed_at: 1,
        expires_at: expiry,
    }
}
impl Fixture {
    async fn new() -> (Self, Arc<CustodyOwner>) {
        let directory = tempfile::tempdir().unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let binding = Binding::new(
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
        .unwrap();
        let file = Arc::new(FileStore::open(directory.path().join("credentials")).unwrap());
        let journal = Arc::new(Journal {
            inner: FullJournal::open(&directory.path().join("journal.db")).unwrap(),
            decision_mode: AtomicU8::new(0),
            pause: Arc::new(Pause::default()),
        });
        // A real committed/retired generation seeds recovery, without creating a session.
        let generation =
            SecretTransactionGeneration::from_protocol_bytes(1_u64.to_be_bytes()).unwrap();
        let id = SecretTransactionId::new(generation, [1; 24]);
        let proposal = Proposal::new(
            &binding,
            Secret::new("OLD-ACCESS-SENTINEL"),
            Some(Secret::new("OLD-REFRESH-SENTINEL")),
        )
        .unwrap();
        file.prepare(id, proposal.digest, &proposal.batch)
            .await
            .unwrap();
        file.commit(id).await.unwrap();
        file.reclaim(generation).await.unwrap();
        let publication = Publication {
            identity: binding.identity.clone(),
            generation: 1,
            evidence: observation(999),
            authorization: Some(Timing {
                authorized_at: 10,
                deadline: 20,
            }),
        };
        journal
            .inner
            .write(
                &serde_json::to_vec(&Image {
                    version: 1,
                    next_generation: 2,
                    retired_through: 1,
                    retired_transaction: Some(id.protocol_bytes()),
                    connections: BTreeMap::from([(
                        binding.identity.connection.clone(),
                        publication,
                    )]),
                    pending: None,
                })
                .unwrap(),
            )
            .unwrap();
        let store = Arc::new(Store {
            inner: file,
            prepares: AtomicUsize::new(0),
            commits: AtomicUsize::new(0),
            aborts: AtomicUsize::new(0),
            pause_prepare: AtomicBool::new(false),
            pause_commit: AtomicBool::new(false),
            entered: tokio::sync::Notify::new(),
            resume: tokio::sync::Notify::new(),
        });
        let owner = Arc::new(
            CustodyOwner::from_journal(journal.clone(), store.clone(), vec![binding.clone()])
                .unwrap(),
        );
        owner.recover().await.unwrap();
        let clock = Arc::new(Clock::new());
        let authority = Arc::new(Mutex::new(Authority {
            identity: binding.identity.clone(),
            generation: 1,
            allowed: true,
            observations: Vec::new(),
        }));
        let refresh = Arc::new(RefreshOwner::new(
            binding.clone(),
            clock.clone(),
            authority.clone(),
        ));
        (
            Self {
                directory,
                binding,
                clock,
                authority,
                refresh,
                journal,
                store,
            },
            owner,
        )
    }
    fn proposal(&self) -> Proposal {
        Proposal::new(&self.binding, Secret::new("NEW-ACCESS-SENTINEL"), None).unwrap()
    }
    fn close(self, owner: Arc<CustodyOwner>) -> (tempfile::TempDir, Binding) {
        drop(owner);
        let Self {
            directory,
            binding,
            journal,
            store,
            ..
        } = self;
        drop(journal);
        drop(store);
        (directory, binding)
    }
}

#[tokio::test]
async fn refresh_of_expired_access_uses_one_timely_claim_without_a_session() {
    assert!(SystemRefreshClock.now().unwrap().0 > 0);
    let (fixture, owner) = Fixture::new().await;
    let guard = fixture.refresh.lock().await;
    let claim = guard
        .begin(&owner, "operation:read")
        .expect("admitted refresh must begin without a session");
    assert!(matches!(
        guard.begin(&owner, "operation:read"),
        Err(CustodyError::Refused)
    ));
    assert_eq!(claim.remaining().unwrap(), Duration::from_secs(30));
    fixture.clock.set(30_999, 29_999);
    let published = owner
        .complete_refresh(
            &fixture.binding,
            observation(100_000),
            fixture.proposal(),
            claim,
        )
        .await;
    assert!(
        published.is_ok(),
        "timely refresh must publish without a ConnectSession"
    );
    let published = published.unwrap();
    assert_eq!(published.generation, 2);
    assert_eq!(
        published.authorization.as_ref().unwrap().authorized_at,
        30_999
    );
    assert_eq!(published.authorization.as_ref().unwrap().deadline, 31_000);
    assert_eq!(fixture.store.commits.load(Ordering::SeqCst), 1);
    assert!(fixture
        .store
        .inner
        .get(&fixture.binding.refresh)
        .await
        .unwrap_err()
        .is_not_found());
    assert_eq!(fixture.authority.lock().unwrap().observations.len(), 2);
    assert_eq!(
        owner
            .snapshot(&fixture.binding.identity)
            .unwrap()
            .unwrap()
            .generation,
        2
    );
}

#[tokio::test]
async fn waiting_for_prepare_does_not_restart_the_refresh_window() {
    let (fixture, owner) = Fixture::new().await;
    let guard = fixture.refresh.lock().await;
    let claim = guard
        .begin(&owner, "operation:read")
        .expect("admitted refresh must begin");
    fixture.store.pause_prepare.store(true, Ordering::SeqCst);
    fixture.clock.set(21_000, 20_000);
    let task_owner = owner.clone();
    let binding = fixture.binding.clone();
    let proposal = fixture.proposal();
    let mut task = tokio::spawn(async move {
        task_owner
            .complete_refresh(&binding, observation(100_000), proposal, claim)
            .await
    });
    tokio::select! {
        () = fixture.store.entered.notified() => {},
        _ = &mut task => panic!("refresh ended before actual prepare"),
    }
    fixture.clock.set(31_000, 30_000);
    fixture.store.resume.notify_one();
    assert!(matches!(task.await.unwrap(), Err(CustodyError::Refused)));
    assert_eq!(fixture.store.aborts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.store.commits.load(Ordering::SeqCst), 0);
    assert_eq!(
        owner
            .snapshot(&fixture.binding.identity)
            .unwrap()
            .unwrap()
            .generation,
        1
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_timely_refresh_claim_survives_a_held_full_decision_write() {
    let (fixture, owner) = Fixture::new().await;
    let guard = fixture.refresh.lock().await;
    let claim = guard
        .begin(&owner, "operation:read")
        .expect("admitted refresh must begin");
    fixture.clock.set(30_999, 29_999);
    fixture.journal.decision_mode.store(3, Ordering::SeqCst);
    let task_owner = owner.clone();
    let binding = fixture.binding.clone();
    let proposal = fixture.proposal();
    let mut task = tokio::spawn(async move {
        task_owner
            .complete_refresh(&binding, observation(100_000), proposal, claim)
            .await
    });
    tokio::select! {
        () = fixture.journal.pause.entered.notified() => {},
        _ = &mut task => panic!("refresh ended before its FULL decision"),
    }
    fixture.clock.set(40_000, 39_000);
    assert_eq!(fixture.store.commits.load(Ordering::SeqCst), 0);
    assert!(matches!(
        owner.snapshot(&fixture.binding.identity),
        Err(CustodyError::Unavailable)
    ));
    {
        let mut state = owner.state.lock().unwrap();
        let pending = state.image.pending.clone().unwrap();
        assert!(matches!(
            state.live.as_mut().unwrap().claim(&pending, None),
            Err(CustodyError::Refused)
        ));
    }
    fixture.journal.pause.release();
    let result = task.await.unwrap();
    assert!(
        result.is_ok(),
        "won claim must survive delayed decision persistence"
    );
    assert_eq!(result.unwrap().authorization.unwrap().authorized_at, 30_999);
    assert_eq!(fixture.clock.reads.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn uncertain_refresh_decisions_reopen_as_finish_or_abort_without_a_session() {
    for durable in [false, true] {
        let (fixture, owner) = Fixture::new().await;
        let guard = fixture.refresh.lock().await;
        let claim = guard
            .begin(&owner, "operation:read")
            .expect("admitted refresh must begin");
        fixture
            .journal
            .decision_mode
            .store(if durable { 2 } else { 1 }, Ordering::SeqCst);
        let result = owner
            .complete_refresh(
                &fixture.binding,
                observation(100_000),
                fixture.proposal(),
                claim,
            )
            .await;
        assert!(matches!(result, Err(CustodyError::Unavailable)));
        assert_eq!(fixture.store.commits.load(Ordering::SeqCst), 0);
        assert!(matches!(
            owner.snapshot(&fixture.binding.identity),
            Err(CustodyError::Unavailable)
        ));
        let journal = fixture.journal.read().unwrap().unwrap();
        let journal = std::str::from_utf8(&journal).unwrap();
        for private in [
            "ACCESS-SENTINEL",
            "REFRESH-SENTINEL",
            "operation:read",
            "session:",
        ] {
            assert!(!journal.contains(private));
        }
        drop(guard);
        let (directory, binding) = fixture.close(owner);
        let file = Arc::new(FileStore::open(directory.path().join("credentials")).unwrap());
        let reopened = CustodyOwner::open(
            FullJournal::open(&directory.path().join("journal.db")).unwrap(),
            file.clone(),
            vec![binding.clone()],
        )
        .unwrap();
        reopened.recover().await.unwrap();
        let publication = reopened.snapshot(&binding.identity).unwrap().unwrap();
        assert_eq!(publication.generation, if durable { 2 } else { 1 });
        assert_eq!(
            file.get(&binding.access).await.unwrap().expose_secret(),
            if durable {
                "NEW-ACCESS-SENTINEL"
            } else {
                "OLD-ACCESS-SENTINEL"
            }
        );
        reopened.recover().await.unwrap();
    }
}

#[tokio::test]
async fn refresh_start_requires_current_operation_authority_and_a_valid_receiver_clock() {
    for case in [
        "operation",
        "authority",
        "generation",
        "zero",
        "overflow",
        "clock",
    ] {
        let (fixture, owner) = Fixture::new().await;
        let operation = if case == "operation" {
            "operation:write"
        } else {
            "operation:read"
        };
        match case {
            "authority" => fixture.authority.lock().unwrap().allowed = false,
            "generation" => fixture.authority.lock().unwrap().generation = 2,
            "zero" => fixture.clock.set(0, 0),
            "overflow" => fixture.clock.set(u64::MAX - 29_999, 0),
            "clock" => fixture.clock.failed.store(true, Ordering::SeqCst),
            _ => {}
        }
        let guard = fixture.refresh.lock().await;
        assert!(
            guard.begin(&owner, operation).is_err(),
            "{case} must refuse before refresh egress"
        );
        assert_eq!(fixture.store.prepares.load(Ordering::SeqCst), 0);
        assert!(owner.state.lock().unwrap().image.pending.is_none());
    }
}

#[tokio::test]
async fn refresh_claim_rechecks_time_authority_generation_and_captured_evidence() {
    for case in [
        "wall",
        "monotonic",
        "monotonic_rollback",
        "rollback",
        "clock",
        "grant",
        "generation",
        "client",
        "origin",
        "authority",
        "subject",
        "ceiling",
        "expired",
        "future",
    ] {
        let (fixture, owner) = Fixture::new().await;
        let guard = fixture.refresh.lock().await;
        if case == "monotonic_rollback" {
            fixture.clock.set(1_000, 1);
        }
        let claim = guard.begin(&owner, "operation:read").unwrap();
        let mut evidence = observation(100_000);
        match case {
            "wall" => fixture.clock.set(31_000, 20_000),
            "monotonic" => fixture.clock.set(2_000, 30_000),
            "monotonic_rollback" => fixture.clock.set(1_001, 0),
            "rollback" => fixture.clock.set(999, 10),
            "clock" => fixture.clock.failed.store(true, Ordering::SeqCst),
            "grant" => fixture.authority.lock().unwrap().allowed = false,
            "generation" => fixture.authority.lock().unwrap().generation = 2,
            "client" => evidence.client_digest = [9; 32],
            "origin" => evidence.origin_digest = [9; 32],
            "authority" => evidence.authority_digest = [9; 32],
            "subject" => evidence.subject = "subject:other".into(),
            "ceiling" => {
                evidence.ceiling.insert("write_api".into());
            }
            "expired" => evidence.expires_at = 1_000,
            "future" => evidence.observed_at = 1_001,
            _ => unreachable!(),
        }
        let result = owner
            .complete_refresh(&fixture.binding, evidence, fixture.proposal(), claim)
            .await;
        assert!(
            matches!(result, Err(CustodyError::Refused)),
            "{case} must abort its prepared refresh"
        );
        assert_eq!(fixture.store.prepares.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.store.aborts.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.store.commits.load(Ordering::SeqCst), 0);
        assert_eq!(
            owner
                .snapshot(&fixture.binding.identity)
                .unwrap()
                .unwrap()
                .generation,
            1
        );
    }
}

#[tokio::test]
async fn cancelled_refresh_store_io_holds_the_binding_gate_until_recovery() {
    use std::future::Future as _;
    use std::task::{Context, Poll, Waker};
    for committed in [false, true] {
        let (fixture, owner) = Fixture::new().await;
        let guard = fixture.refresh.lock().await;
        let claim = guard.begin(&owner, "operation:read").unwrap();
        drop(guard);
        if committed {
            fixture.store.pause_commit.store(true, Ordering::SeqCst);
        } else {
            fixture.store.pause_prepare.store(true, Ordering::SeqCst);
        }
        let task_owner = owner.clone();
        let binding = fixture.binding.clone();
        let proposal = fixture.proposal();
        let mut task = tokio::spawn(async move {
            task_owner
                .complete_refresh(&binding, observation(100_000), proposal, claim)
                .await
        });
        tokio::select! {
            () = fixture.store.entered.notified() => {},
            _ = &mut task => panic!("refresh ended before awaited store boundary"),
        }
        task.abort();
        assert!(matches!(task.await, Err(error) if error.is_cancelled()));
        let mut waiting = std::pin::pin!(fixture.refresh.lock());
        assert!(matches!(
            waiting
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        assert!(matches!(
            owner.snapshot(&fixture.binding.identity),
            Err(CustodyError::Unavailable)
        ));
        fixture.clock.failed.store(true, Ordering::SeqCst);
        owner.recover().await.unwrap();
        let _next_guard = waiting.await;
        assert_eq!(
            owner
                .snapshot(&fixture.binding.identity)
                .unwrap()
                .unwrap()
                .generation,
            if committed { 2 } else { 1 }
        );
    }
}

#[tokio::test]
async fn binding_gate_preserves_the_new_generation_for_the_waiting_operation() {
    use std::future::Future as _;
    use std::task::{Context, Poll, Waker};
    let (fixture, owner) = Fixture::new().await;
    let guard = fixture.refresh.lock().await;
    let claim = guard.begin(&owner, "operation:read").unwrap();
    let mut waiting = std::pin::pin!(fixture.refresh.lock());
    assert!(matches!(
        waiting
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Pending
    ));
    assert!(owner
        .complete_refresh(
            &fixture.binding,
            observation(100_000),
            fixture.proposal(),
            claim
        )
        .await
        .is_ok());
    assert!(matches!(
        waiting
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Pending
    ));
    drop(guard);
    let _next_guard = waiting.await;
    let current = owner.snapshot(&fixture.binding.identity).unwrap().unwrap();
    assert_eq!(current.generation, 2);
    assert!(current.evidence.expires_at > fixture.clock.wall.load(Ordering::SeqCst));
    assert_eq!(fixture.store.prepares.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.store.commits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn invalid_refresh_decision_timing_cannot_publish_on_reopen() {
    for invalid in ["missing", "deadline"] {
        let (fixture, owner) = Fixture::new().await;
        let guard = fixture.refresh.lock().await;
        let claim = guard.begin(&owner, "operation:read").unwrap();
        fixture.journal.decision_mode.store(2, Ordering::SeqCst);
        assert!(matches!(
            owner
                .complete_refresh(
                    &fixture.binding,
                    observation(100_000),
                    fixture.proposal(),
                    claim
                )
                .await,
            Err(CustodyError::Unavailable)
        ));
        let mut image: Image =
            serde_json::from_slice(&fixture.journal.read().unwrap().unwrap()).unwrap();
        let pending = image.pending.as_mut().unwrap();
        if invalid == "missing" {
            pending.publication.authorization = None;
        } else {
            pending.phase = Phase::Decided {
                authorized_at: 31_000,
                deadline: 31_000,
            };
            pending.publication.authorization = Some(Timing {
                authorized_at: 31_000,
                deadline: 31_000,
            });
        }
        fixture
            .journal
            .inner
            .write(&serde_json::to_vec(&image).unwrap())
            .unwrap();
        drop(guard);
        let (directory, binding) = fixture.close(owner);
        let file = Arc::new(FileStore::open(directory.path().join("credentials")).unwrap());
        assert!(matches!(
            CustodyOwner::open(
                FullJournal::open(&directory.path().join("journal.db")).unwrap(),
                file.clone(),
                vec![binding.clone()]
            ),
            Err(CustodyError::Unavailable)
        ));
        assert_eq!(
            file.get(&binding.access).await.unwrap().expose_secret(),
            "OLD-ACCESS-SENTINEL"
        );
    }
}

#[tokio::test]
async fn a_stale_refresh_handle_cannot_prepare_after_another_valid_publication() {
    let (fixture, owner) = Fixture::new().await;
    let competing = Arc::new(RefreshOwner::new(
        fixture.binding.clone(),
        fixture.clock.clone(),
        fixture.authority.clone(),
    ));
    let first = fixture.refresh.lock().await;
    let stale = competing.lock().await;
    let first_claim = first.begin(&owner, "operation:read").unwrap();
    let stale_claim = stale.begin(&owner, "operation:read").unwrap();
    assert!(owner
        .complete_refresh(
            &fixture.binding,
            observation(100_000),
            fixture.proposal(),
            first_claim
        )
        .await
        .is_ok());
    assert!(matches!(
        owner
            .complete_refresh(
                &fixture.binding,
                observation(100_000),
                fixture.proposal(),
                stale_claim
            )
            .await,
        Err(CustodyError::Refused)
    ));
    assert_eq!(fixture.store.prepares.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.store.commits.load(Ordering::SeqCst), 1);
    assert_eq!(
        owner
            .snapshot(&fixture.binding.identity)
            .unwrap()
            .unwrap()
            .generation,
        2
    );
}

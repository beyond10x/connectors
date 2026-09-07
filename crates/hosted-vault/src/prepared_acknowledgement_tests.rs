use super::*;
use connector_secrets::MemoryStore;

fn generation(value: u64) -> SecretTransactionGeneration {
    SecretTransactionGeneration::from_protocol_bytes(value.to_be_bytes()).unwrap()
}

fn transaction(value: u64, nonce: u8) -> SecretTransactionId {
    SecretTransactionId::new(generation(value), [nonce; 24])
}

fn proposal(nonce: u8) -> SecretProposalDigest {
    SecretProposalDigest::from_protocol_bytes([nonce; 32])
}

#[tokio::test]
async fn exact_ack_preserves_legacy_committed_peers_across_file_journal_restart() {
    for second_generation in [8, 9] {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let journal = root.path().join("transactions.json");
        let inner = Arc::new(MemoryStore::new());
        let first = transaction(8, 1);
        let second = transaction(second_generation, 2);
        {
            let store = PreparedVaultStore::open(inner.clone(), &journal).unwrap();
            assert_eq!(store.retirement_watermark().await.unwrap(), None);
            store
                .prepare(first, proposal(1), &tests::batch(tests::OLD))
                .await
                .unwrap();
            store.commit(first).await.unwrap();
            store
                .prepare(second, proposal(2), &tests::batch(tests::NEW))
                .await
                .unwrap();
            store.commit(second).await.unwrap();
            let bytes = fs::read_to_string(&journal).unwrap();
            assert!(
                !bytes.contains("acknowledged"),
                "legacy records keep their original meaning"
            );
            assert!(!bytes.contains(tests::OLD));
            assert!(!bytes.contains(tests::NEW));
        }
        {
            let store = PreparedVaultStore::open(inner.clone(), &journal).unwrap();
            store.initialize().await.unwrap();
            store.acknowledge(second).await.unwrap();
            store.acknowledge(second).await.unwrap();
            assert_eq!(
                store.retirement_watermark().await.unwrap(),
                Some(generation(7))
            );
            assert_eq!(
                store.state(first).await,
                Ok(SecretTransactionState::Committed)
            );
        }
        {
            let store = PreparedVaultStore::open(inner.clone(), &journal).unwrap();
            store.initialize().await.unwrap();
            assert_eq!(
                store.state(first).await,
                Ok(SecretTransactionState::Committed)
            );
            assert_eq!(
                store.state(second).await,
                Ok(SecretTransactionState::Committed)
            );
            store.acknowledge(first).await.unwrap();
        }
        let store = PreparedVaultStore::open(inner.clone(), &journal).unwrap();
        store.initialize().await.unwrap();
        assert_eq!(
            store.retirement_watermark().await.unwrap(),
            Some(generation(second_generation))
        );
        assert_eq!(store.state(first).await, Err(PreparedSecretError::Retired));
        assert_eq!(store.state(second).await, Err(PreparedSecretError::Retired));
        store.acknowledge(first).await.unwrap();
        let before = fs::read(&journal).unwrap();
        let paths = inner.paths();
        assert_eq!(
            store
                .prepare(transaction(1, 9), proposal(9), &tests::batch(tests::OLD))
                .await,
            Err(PreparedSecretError::Retired)
        );
        assert_eq!(
            fs::read(&journal).unwrap(),
            before,
            "retired prepare does not mutate the journal"
        );
        assert_eq!(
            inner.paths(),
            paths,
            "retired prepare does not stage credentials"
        );
        assert_eq!(
            store
                .get(&tests::reference())
                .await
                .unwrap()
                .expose_secret(),
            tests::NEW
        );
    }
}

#[tokio::test]
async fn acknowledgement_during_another_prepare_preserves_its_recovery_outcome() {
    let inner = Arc::new(MemoryStore::new());
    let state = Arc::new(connector_state::MemoryState::new());
    let first = transaction(8, 1);
    let second = transaction(8, 2);
    {
        let store =
            PreparedVaultStore::open_shared(inner.clone(), state.clone(), "prepared").unwrap();
        store
            .prepare(first, proposal(1), &tests::batch(tests::OLD))
            .await
            .unwrap();
        store.commit(first).await.unwrap();
        store
            .prepare(second, proposal(2), &tests::batch(tests::NEW))
            .await
            .unwrap();
        assert_eq!(
            store.acknowledge(second).await,
            Err(PreparedSecretError::Busy)
        );
        store.acknowledge(first).await.unwrap();
        assert_eq!(
            store.state(second).await,
            Ok(SecretTransactionState::Prepared)
        );
    }
    let store = PreparedVaultStore::open_shared(inner, state, "prepared").unwrap();
    store.initialize().await.unwrap();
    assert_eq!(
        store.state(first).await,
        Ok(SecretTransactionState::Committed)
    );
    assert_eq!(
        store.state(second).await,
        Ok(SecretTransactionState::Prepared)
    );
    store.commit(second).await.unwrap();
    store.acknowledge(second).await.unwrap();
    assert_eq!(store.state(first).await, Err(PreparedSecretError::Retired));
    assert_eq!(store.state(second).await, Err(PreparedSecretError::Retired));
}

#[tokio::test]
async fn contending_prepares_cannot_both_pass_the_single_slot_admission() {
    let store = PreparedVaultStore::open_shared(
        Arc::new(MemoryStore::new()),
        Arc::new(connector_state::MemoryState::new()),
        "prepared",
    )
    .unwrap();
    let first_batch = tests::batch(tests::OLD);
    let second_batch = tests::batch(tests::NEW);
    let journal = store.journal.lock().await;
    let first = store.prepare(transaction(8, 1), proposal(1), &first_batch);
    let second = store.prepare(transaction(8, 2), proposal(2), &second_batch);
    tokio::pin!(first, second);
    // Queue both requests while the journal is held. Without mutation serialization, Tokio's
    // FIFO mutex lets both checks run before either caller reacquires it to insert Staging.
    assert!(futures_util::poll!(first.as_mut()).is_pending());
    assert!(futures_util::poll!(second.as_mut()).is_pending());
    drop(journal);
    let (first, second) = tokio::join!(first, second);
    assert_eq!(first, Ok(SecretTransactionState::Prepared));
    assert_eq!(second, Err(PreparedSecretError::Busy));
    assert_eq!(store.journal.lock().await.transactions.len(), 1);
}

#[tokio::test]
async fn absent_intent_recovery_can_finish_before_another_prepared_owner_after_restart() {
    let inner = Arc::new(MemoryStore::new());
    inner
        .put(&tests::reference(), &Secret::new(tests::OLD))
        .await
        .unwrap();
    let state = Arc::new(connector_state::MemoryState::new());
    let absent = transaction(8, 1);
    let prepared = transaction(8, 2);
    {
        let store =
            PreparedVaultStore::open_shared(inner.clone(), state.clone(), "prepared").unwrap();
        store
            .prepare(prepared, proposal(2), &tests::batch(tests::NEW))
            .await
            .unwrap();
    }
    {
        let store =
            PreparedVaultStore::open_shared(inner.clone(), state.clone(), "prepared").unwrap();
        store.initialize().await.unwrap();
        assert_eq!(
            store.state(absent).await,
            Ok(SecretTransactionState::Absent)
        );
        assert_eq!(
            store.abort(absent).await,
            Ok(SecretTransactionState::Absent)
        );
        store.acknowledge(absent).await.unwrap();
        assert_eq!(
            store.state(prepared).await,
            Ok(SecretTransactionState::Prepared)
        );
        assert_eq!(
            store
                .get(&tests::reference())
                .await
                .unwrap()
                .expose_secret(),
            tests::OLD
        );
    }
    let store = PreparedVaultStore::open_shared(inner, state, "prepared").unwrap();
    store.initialize().await.unwrap();
    assert_eq!(
        store.state(prepared).await,
        Ok(SecretTransactionState::Prepared)
    );
    assert_eq!(
        store
            .prepare(absent, proposal(1), &tests::batch(tests::OLD))
            .await,
        Err(PreparedSecretError::TransactionIdReused),
        "the absent intent cannot prepare after its owner finished recovery"
    );
    store.commit(prepared).await.unwrap();
    store.acknowledge(prepared).await.unwrap();
    assert_eq!(store.state(absent).await, Err(PreparedSecretError::Retired));
    assert_eq!(
        store.state(prepared).await,
        Err(PreparedSecretError::Retired)
    );
    assert_eq!(
        store
            .get(&tests::reference())
            .await
            .unwrap()
            .expose_secret(),
        tests::NEW
    );
}

struct UncertainState {
    inner: connector_state::MemoryState,
    failure: std::sync::atomic::AtomicU8,
}

impl StateStore for UncertainState {
    fn read(
        &self,
        key: &str,
        maximum: usize,
    ) -> Result<Option<Vec<u8>>, connector_state::StateError> {
        self.inner.read(key, maximum)
    }
    fn replace(
        &self,
        key: &str,
        body: &[u8],
        maximum: usize,
    ) -> Result<(), connector_state::StateError> {
        let failure = self.failure.swap(0, Ordering::SeqCst);
        if failure != 1 {
            self.inner.replace(key, body, maximum)?;
        }
        if failure == 0 {
            Ok(())
        } else {
            Err(connector_state::StateError::Unavailable)
        }
    }
    fn append(
        &self,
        key: &str,
        suffix: &[u8],
        maximum: usize,
    ) -> Result<usize, connector_state::StateError> {
        self.inner.append(key, suffix, maximum)
    }
    fn delete(&self, key: &str) -> Result<(), connector_state::StateError> {
        self.inner.delete(key)
    }
}

#[tokio::test]
async fn uncertain_acknowledgement_cannot_roll_back_the_durable_retirement_fence() {
    for failure in [1, 2] {
        let inner = Arc::new(MemoryStore::new());
        let state = Arc::new(UncertainState {
            inner: connector_state::MemoryState::new(),
            failure: std::sync::atomic::AtomicU8::new(0),
        });
        let id = transaction(8, 1);
        {
            let store =
                PreparedVaultStore::open_shared(inner.clone(), state.clone(), "prepared").unwrap();
            store
                .prepare(id, proposal(1), &tests::batch(tests::OLD))
                .await
                .unwrap();
            store.commit(id).await.unwrap();
            state.failure.store(failure, Ordering::SeqCst);
            assert_eq!(
                store.acknowledge(id).await,
                Err(PreparedSecretError::Backend)
            );
            assert_eq!(
                store.retirement_watermark().await,
                Err(PreparedSecretError::Backend)
            );
            assert_eq!(
                store
                    .prepare(transaction(8, 2), proposal(2), &tests::batch(tests::NEW))
                    .await,
                Err(PreparedSecretError::Backend),
                "uncertain persistence requires recovery before another mutation"
            );
        }
        let store = PreparedVaultStore::open_shared(inner, state, "prepared").unwrap();
        store.initialize().await.unwrap();
        assert_eq!(
            store.state(id).await,
            if failure == 1 {
                Ok(SecretTransactionState::Committed)
            } else {
                Err(PreparedSecretError::Retired)
            }
        );
        store.acknowledge(id).await.unwrap();
        assert_eq!(
            store.retirement_watermark().await.unwrap(),
            Some(generation(8))
        );
    }
}

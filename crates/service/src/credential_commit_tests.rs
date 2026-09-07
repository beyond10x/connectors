use super::*;
use async_trait::async_trait;
use connector_secrets::{
    CredentialRef, CredentialScope, FileStore, MemoryStore, Secret, SecretStore, StoreError,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const OLD: &str = "SENTINEL-NOT-A-REAL-SECRET-before-custody";
const NEW: &str = "SENTINEL-NOT-A-REAL-SECRET-after-custody";

fn reference() -> CredentialRef {
    CredentialRef::new("tenant-test", "com.example.api", "default", "access_token").unwrap()
}

fn batch() -> SecretBatch {
    let mut batch =
        SecretBatch::new(CredentialScope::new("tenant-test", "com.example.api").unwrap());
    batch.put(reference(), Secret::new(NEW)).unwrap();
    batch
}

fn digest() -> SecretProposalDigest {
    SecretProposalDigest::from_protocol_bytes([3; 32])
}

fn transaction(value: u64, nonce: u8) -> (SecretTransactionId, SecretTransactionGeneration) {
    let generation = SecretTransactionGeneration::from_protocol_bytes(value.to_be_bytes()).unwrap();
    (
        SecretTransactionId::new(generation, [nonce; 24]),
        generation,
    )
}

#[derive(Clone, Copy)]
enum PrepareAction {
    RetireBeforePrepare,
    Prepare,
    Refuse(PreparedSecretError),
    PrepareThenUnavailable,
}

/// Only the retirement/failure scheduling is controlled. Staging, replay fences, old-value
/// visibility and subsequent recovery use the real MemoryStore transaction implementation.
struct ControlledStore {
    inner: MemoryStore,
    actions: Mutex<VecDeque<PrepareAction>>,
    intent: Arc<Mutex<Option<SecretTransactionId>>>,
    prepares: Mutex<Vec<SecretTransactionId>>,
    retire_on_abort: bool,
}

impl ControlledStore {
    fn new(actions: impl IntoIterator<Item = PrepareAction>) -> Self {
        Self {
            inner: MemoryStore::new(),
            actions: Mutex::new(actions.into_iter().collect()),
            intent: Arc::new(Mutex::new(None)),
            prepares: Mutex::new(Vec::new()),
            retire_on_abort: false,
        }
    }
}

#[async_trait]
impl SecretStore for ControlledStore {
    async fn ready(&self) -> Result<(), StoreError> {
        self.inner.ready().await
    }
    async fn get(&self, reference: &CredentialRef) -> Result<Secret, StoreError> {
        self.inner.get(reference).await
    }
    async fn put(&self, reference: &CredentialRef, value: &Secret) -> Result<(), StoreError> {
        self.inner.put(reference, value).await
    }
    async fn delete(&self, reference: &CredentialRef) -> Result<(), StoreError> {
        self.inner.delete(reference).await
    }
}

#[async_trait]
impl PreparedSecretStore for ControlledStore {
    async fn retirement_watermark(
        &self,
    ) -> Result<Option<SecretTransactionGeneration>, PreparedSecretError> {
        self.inner.retirement_watermark().await
    }
    async fn prepare(
        &self,
        id: SecretTransactionId,
        proposal: SecretProposalDigest,
        batch: &SecretBatch,
    ) -> Result<SecretTransactionState, PreparedSecretError> {
        assert_eq!(
            *self.intent.lock().unwrap(),
            Some(id),
            "the owner must persist its exact intent before prepare"
        );
        assert_eq!(proposal, digest(), "retry retains the verified proposal");
        self.prepares.lock().unwrap().push(id);
        let action = self
            .actions
            .lock()
            .unwrap()
            .pop_front()
            .expect("an unexpected prepare replay reached the store");
        match action {
            PrepareAction::RetireBeforePrepare => {
                let mut encoded = [0; 8];
                encoded.copy_from_slice(&id.protocol_bytes()[..8]);
                self.inner
                    .reclaim(SecretTransactionGeneration::from_protocol_bytes(encoded).unwrap())
                    .await?;
                self.inner.prepare(id, proposal, batch).await
            }
            PrepareAction::Prepare => self.inner.prepare(id, proposal, batch).await,
            PrepareAction::Refuse(error) => Err(error),
            PrepareAction::PrepareThenUnavailable => {
                self.inner.prepare(id, proposal, batch).await?;
                Err(PreparedSecretError::Backend)
            }
        }
    }
    async fn state(
        &self,
        id: SecretTransactionId,
    ) -> Result<SecretTransactionState, PreparedSecretError> {
        self.inner.state(id).await
    }
    async fn commit(
        &self,
        id: SecretTransactionId,
    ) -> Result<SecretTransactionState, PreparedSecretError> {
        self.inner.commit(id).await
    }
    async fn abort(
        &self,
        id: SecretTransactionId,
    ) -> Result<SecretTransactionState, PreparedSecretError> {
        if self.retire_on_abort {
            let mut encoded = [0; 8];
            encoded.copy_from_slice(&id.protocol_bytes()[..8]);
            self.inner
                .reclaim(SecretTransactionGeneration::from_protocol_bytes(encoded).unwrap())
                .await?;
        }
        self.inner.abort(id).await
    }
}

#[tokio::test]
async fn preparation_retries_only_definitely_retired_attempts_and_can_succeed_on_the_fourth() {
    let store = ControlledStore::new([
        PrepareAction::RetireBeforePrepare,
        PrepareAction::RetireBeforePrepare,
        PrepareAction::RetireBeforePrepare,
        PrepareAction::Prepare,
    ]);
    store
        .inner
        .put(&reference(), &Secret::new(OLD))
        .await
        .unwrap();
    store.inner.reclaim(transaction(7, 1).1).await.unwrap();
    let mut reservations = Vec::new();
    let result = prepare_credential_batch(&store, digest(), &batch(), |minimum| {
        let reserved = transaction(minimum, 1);
        reservations.push(reserved);
        *store.intent.lock().unwrap() = Some(reserved.0);
        Ok::<_, &'static str>(reserved)
    })
    .await
    .unwrap();
    assert_eq!(
        reservations,
        (8..=11)
            .map(|value| transaction(value, 1))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        *store.prepares.lock().unwrap(),
        reservations.iter().map(|entry| entry.0).collect::<Vec<_>>()
    );
    assert_eq!(result, transaction(11, 1));
    for &(id, _) in &reservations[..3] {
        assert_eq!(
            store.inner.state(id).await,
            Err(PreparedSecretError::Retired)
        );
    }
    assert_eq!(
        store.inner.get(&reference()).await.unwrap().expose_secret(),
        OLD
    );
    assert!(matches!(
        recover_credential_intent(&store, result.0, true).await,
        Ok(CredentialRecovery::Committed)
    ));
    assert_eq!(
        store.inner.get(&reference()).await.unwrap().expose_secret(),
        NEW
    );
}

#[tokio::test]
async fn the_fourth_retired_attempt_returns_without_a_fifth_reservation_or_prepare() {
    let store = ControlledStore::new([PrepareAction::RetireBeforePrepare; 4]);
    store
        .inner
        .put(&reference(), &Secret::new(OLD))
        .await
        .unwrap();
    let mut reservations = Vec::new();
    let result = prepare_credential_batch(&store, digest(), &batch(), |minimum| {
        let reserved = transaction(minimum, 1);
        reservations.push(reserved);
        *store.intent.lock().unwrap() = Some(reserved.0);
        Ok::<_, &'static str>(reserved)
    })
    .await;
    assert!(matches!(
        result,
        Err(CredentialPreparationError::Store(
            PreparedSecretError::Retired
        ))
    ));
    assert_eq!(reservations.len(), 4);
    assert_eq!(store.prepares.lock().unwrap().len(), 4);
    assert_eq!(*store.intent.lock().unwrap(), Some(transaction(4, 1).0));
    assert_eq!(
        store.inner.get(&reference()).await.unwrap().expose_secret(),
        OLD
    );
}

#[tokio::test]
async fn failed_intent_persistence_after_retirement_never_stages_a_new_attempt() {
    let store = ControlledStore::new([PrepareAction::RetireBeforePrepare]);
    let mut callbacks = 0;
    let result = prepare_credential_batch(&store, digest(), &batch(), |minimum| {
        callbacks += 1;
        if callbacks == 2 {
            assert_eq!(minimum, 2);
            return Err("intent-persistence-failed");
        }
        let reserved = transaction(minimum, 1);
        *store.intent.lock().unwrap() = Some(reserved.0);
        Ok(reserved)
    })
    .await;
    assert!(matches!(
        result,
        Err(CredentialPreparationError::Reservation(
            "intent-persistence-failed"
        ))
    ));
    assert_eq!(callbacks, 2);
    assert_eq!(store.prepares.lock().unwrap().len(), 1);
    assert_eq!(*store.intent.lock().unwrap(), Some(transaction(1, 1).0));
    assert_eq!(
        store.inner.state(transaction(1, 1).0).await,
        Err(PreparedSecretError::Retired)
    );
    assert_eq!(
        store.inner.state(transaction(2, 1).0).await,
        Ok(SecretTransactionState::Absent)
    );
}

#[tokio::test]
async fn busy_backend_and_uncertain_preparation_never_retry() {
    for action in [
        PrepareAction::Refuse(PreparedSecretError::Busy),
        PrepareAction::Refuse(PreparedSecretError::Backend),
        PrepareAction::PrepareThenUnavailable,
    ] {
        let store = ControlledStore::new([action]);
        store
            .inner
            .put(&reference(), &Secret::new(OLD))
            .await
            .unwrap();
        let mut reservations = 0;
        let result = prepare_credential_batch(&store, digest(), &batch(), |minimum| {
            reservations += 1;
            let reserved = transaction(minimum, 1);
            *store.intent.lock().unwrap() = Some(reserved.0);
            Ok::<_, &'static str>(reserved)
        })
        .await;
        let expected = if matches!(action, PrepareAction::Refuse(PreparedSecretError::Busy)) {
            PreparedSecretError::Busy
        } else {
            PreparedSecretError::Backend
        };
        assert!(
            matches!(result, Err(CredentialPreparationError::Store(error)) if error == expected)
        );
        assert_eq!(reservations, 1);
        assert_eq!(store.prepares.lock().unwrap().len(), 1);
        assert_eq!(*store.intent.lock().unwrap(), Some(transaction(1, 1).0));
        assert_eq!(
            store.inner.get(&reference()).await.unwrap().expose_secret(),
            OLD
        );
        if matches!(action, PrepareAction::PrepareThenUnavailable) {
            assert_eq!(
                store.inner.state(transaction(1, 1).0).await,
                Ok(SecretTransactionState::Prepared)
            );
            assert!(matches!(
                recover_credential_intent(&store, transaction(1, 1).0, true).await,
                Ok(CredentialRecovery::Committed)
            ));
            assert_eq!(
                store.inner.get(&reference()).await.unwrap().expose_secret(),
                NEW
            );
            assert_eq!(
                store.prepares.lock().unwrap().len(),
                1,
                "recovery commits the existing candidate without preparing again"
            );
        } else {
            assert_eq!(
                store.inner.state(transaction(1, 1).0).await,
                Ok(SecretTransactionState::Absent)
            );
        }
    }
}

#[tokio::test]
async fn exhausted_watermark_never_calls_the_reservation_callback() {
    let store = ControlledStore::new([]);
    store
        .inner
        .reclaim(transaction(u64::MAX, 1).1)
        .await
        .unwrap();
    let mut reserved = false;
    let result = prepare_credential_batch(&store, digest(), &batch(), |_| {
        reserved = true;
        Ok::<_, &'static str>(transaction(1, 1))
    })
    .await;
    assert!(matches!(
        result,
        Err(CredentialPreparationError::Store(
            PreparedSecretError::Retired
        ))
    ));
    assert!(
        !reserved,
        "generation exhaustion must not allocate or persist another intent"
    );
    assert!(store.prepares.lock().unwrap().is_empty());
}

#[tokio::test]
async fn absent_intent_recovery_fences_delayed_preparation() {
    for write_ahead in [false, true] {
        let store = MemoryStore::new();
        let id = transaction(8, 1).0;
        assert!(matches!(
            recover_credential_intent(&store, id, write_ahead).await,
            Ok(CredentialRecovery::Discarded)
        ));
        assert_eq!(
            store.prepare(id, digest(), &batch()).await,
            Err(PreparedSecretError::TransactionIdReused)
        );
        assert!(store.get(&reference()).await.is_err());
    }
}

#[tokio::test]
async fn retired_write_ahead_intents_are_discarded_but_legacy_retirement_is_not_commit_proof() {
    for retirement_during_abort in [false, true] {
        for write_ahead in [false, true] {
            let mut store = ControlledStore::new([]);
            let (id, generation) = transaction(8, 1);
            store.retire_on_abort = retirement_during_abort;
            if !retirement_during_abort {
                store.inner.reclaim(generation).await.unwrap();
            }
            let result = recover_credential_intent(&store, id, write_ahead).await;
            if write_ahead {
                assert!(matches!(result, Ok(CredentialRecovery::Discarded)));
            } else {
                assert!(matches!(result, Err(PreparedSecretError::Retired)));
            }
            assert!(store.prepares.lock().unwrap().is_empty());
        }
    }
}

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        Self(std::env::temp_dir().join(format!(
            "connectors-service-credential-commit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        )))
    }
    fn store(&self) -> std::path::PathBuf {
        self.0.join("credentials")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn file_absent_intent_defers_until_its_prepared_peer_is_recovered() {
    let root = Scratch::new();
    let absent = transaction(8, 1).0;
    let peer = transaction(8, 2).0;
    {
        let store = FileStore::open(root.store()).unwrap();
        store.prepare(peer, digest(), &batch()).await.unwrap();
        assert!(matches!(
            recover_credential_intent(&store, absent, true).await,
            Ok(CredentialRecovery::Deferred)
        ));
        assert_eq!(
            store.state(absent).await,
            Ok(SecretTransactionState::Absent)
        );
        assert_eq!(
            store.state(peer).await,
            Ok(SecretTransactionState::Prepared)
        );
    }
    let store = FileStore::open(root.store()).unwrap();
    assert!(matches!(
        recover_credential_intent(&store, peer, true).await,
        Ok(CredentialRecovery::Committed)
    ));
    assert!(matches!(
        recover_credential_intent(&store, absent, true).await,
        Ok(CredentialRecovery::Discarded)
    ));
    assert_eq!(
        store.prepare(absent, digest(), &batch()).await,
        Err(PreparedSecretError::TransactionIdReused)
    );
    store.acknowledge(absent).await.unwrap();
    assert_eq!(
        store.state(peer).await,
        Ok(SecretTransactionState::Committed)
    );
    store.acknowledge(peer).await.unwrap();
    assert_eq!(store.state(absent).await, Err(PreparedSecretError::Retired));
    assert_eq!(store.get(&reference()).await.unwrap().expose_secret(), NEW);
}

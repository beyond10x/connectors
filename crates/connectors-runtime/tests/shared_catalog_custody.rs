//! Composed shared-journal regressions: real PreparedVaultStore over test state/value adapters
//! and controlled provider HTTP. Owner-bound catalog acquisition/recovery is exercised here;
//! real deployed Secrets and provider acceptance remain separate evidence.

use async_trait::async_trait;
use connector_secrets::{
    CredentialRef, MemoryStore, PreparedSecretError, PreparedSecretStore, Secret, SecretBatch,
    SecretProposalDigest, SecretStore, SecretTransactionGeneration, SecretTransactionId,
    SecretTransactionState, StoreError,
};
use connector_state::{MemoryState, StateError, StateStore};
use connectors_config::{HostedCatalogBinding, HostedCatalogConfig};
use hosted_vault::PreparedVaultStore;
use integration_catalog::HostedCatalogBackend;
use protocol::connection::{
    ConnectSessionCreateRequest, ConnectionRequest, ConnectionResult, SearchRequest,
};
use service::{
    ConnectorBackend, EgressHttpRequest, EgressHttpResponse, EgressTransport, EgressTransportError,
    EgressWebSocket, HostedCompletionError, HostedCompletionSubmission, HostedCustodyFailure,
    PrincipalContext,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Default)]
struct ProviderFixture(AtomicUsize);
#[async_trait]
impl EgressTransport for ProviderFixture {
    async fn execute(
        &self,
        _: &str,
        request: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError> {
        assert_eq!(request.request.method, "GET");
        assert_eq!(
            request.request.url,
            "https://grafana.monitoring.example/api/datasources"
        );
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(EgressHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: b"[]".to_vec(),
        })
    }
    async fn connect_websocket(
        &self,
        _: &str,
        _: String,
        _: usize,
    ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
        Err(EgressTransportError::Refused)
    }
}

struct CoordinatorState {
    inner: MemoryState,
    writes: AtomicUsize,
    fail_at: usize,
}
impl CoordinatorState {
    fn new(fail_at: usize) -> Self {
        Self {
            inner: MemoryState::new(),
            writes: AtomicUsize::new(0),
            fail_at,
        }
    }
}
impl StateStore for CoordinatorState {
    fn read(&self, key: &str, maximum: usize) -> Result<Option<Vec<u8>>, StateError> {
        self.inner.read(key, maximum)
    }
    fn replace(&self, key: &str, body: &[u8], maximum: usize) -> Result<(), StateError> {
        if self.writes.fetch_add(1, Ordering::SeqCst) + 1 == self.fail_at {
            return Err(StateError::Unavailable);
        }
        self.inner.replace(key, body, maximum)
    }
    fn append(&self, key: &str, body: &[u8], maximum: usize) -> Result<usize, StateError> {
        self.inner.append(key, body, maximum)
    }
    fn delete(&self, key: &str) -> Result<(), StateError> {
        self.inner.delete(key)
    }
}
fn owner(tenant: &str) -> PrincipalContext {
    PrincipalContext::hosted(
        tenant.to_owned(),
        "person:owner".to_owned(),
        "person:owner".to_owned(),
        None,
        "snapshot:test".to_owned(),
        "a".repeat(64),
    )
    .unwrap()
}
async fn backend(
    tenant: &str,
    state: Arc<CoordinatorState>,
    values: Arc<MemoryStore>,
    prepared: Arc<dyn PreparedSecretStore>,
    egress: Arc<ProviderFixture>,
) -> HostedCatalogBackend {
    HostedCatalogBackend::open(
        tenant.to_owned(),
        HostedCatalogConfig {
            enabled: true,
            public_origin: Some("https://connectors.example/api/connectors/v1".to_owned()),
            grant_ref: Some("grant:catalog-read".to_owned()),
            providers: vec!["grafana".to_owned()],
            bindings: BTreeMap::from([(
                "grafana".to_owned(),
                HostedCatalogBinding {
                    endpoints: BTreeMap::from([(
                        "origin".to_owned(),
                        "https://grafana.monitoring.example".to_owned(),
                    )]),
                    ..Default::default()
                },
            )]),
            ..Default::default()
        },
        BTreeSet::new(),
        BTreeSet::new(),
        values,
        prepared,
        state,
        egress,
    )
    .await
    .unwrap()
}
async fn connect(
    backend: &HostedCatalogBackend,
    owner: &PrincipalContext,
) -> Result<(), HostedCompletionError> {
    let created = backend
        .handle_connection(
            owner,
            ConnectionRequest::ConnectSessionCreate(ConnectSessionCreateRequest {
                integration_ref: "grafana".to_owned(),
                label: "Monitoring".to_owned(),
                auth_profile: Some("grafana.service_account_token".to_owned()),
            }),
        )
        .await
        .unwrap();
    let ConnectionResult::ConnectSessionCreate(created) = created else {
        panic!("session")
    };
    let url = url::Url::parse(created.browser_completion_url.as_deref().unwrap()).unwrap();
    backend
        .complete_hosted_session(
            &created.connect_session_ref,
            url.fragment().unwrap().strip_prefix("token=").unwrap(),
            HostedCompletionSubmission::new(b"TEST-OWNED-SENTINEL".to_vec()),
        )
        .await
}
async fn count(backend: &HostedCatalogBackend, owner: &PrincipalContext) -> usize {
    let result = backend
        .handle_connection(
            owner,
            ConnectionRequest::Search(SearchRequest {
                query: String::new(),
                limit: 10,
            }),
        )
        .await
        .unwrap();
    let ConnectionResult::Search { connections } = result else {
        panic!("connections")
    };
    connections.len()
}
fn metadata(state: &CoordinatorState) -> serde_json::Value {
    serde_json::from_slice(
        &state
            .read("catalog.connections.v1", 2 * 1024 * 1024)
            .unwrap()
            .unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn deployed_retirement_floor_advances_a_new_catalog_coordinator_without_reverification() {
    let state = Arc::new(CoordinatorState::new(usize::MAX));
    state
        .inner
        .replace(
            "catalog.connections.v1",
            br#"{"version":1,"next_transaction_generation":3,"connections":[],"pending":[]}"#,
            2 * 1024 * 1024,
        )
        .unwrap();
    let journal = Arc::new(MemoryState::new());
    let values = Arc::new(MemoryStore::new());
    let prepared = Arc::new(
        PreparedVaultStore::open_shared(
            values.clone(),
            journal.clone(),
            "secrets.prepared-transactions",
        )
        .unwrap(),
    );
    prepared
        .reclaim(SecretTransactionGeneration::from_protocol_bytes(7_u64.to_be_bytes()).unwrap())
        .await
        .unwrap();
    let egress = Arc::new(ProviderFixture::default());
    let first = backend(
        "tenant-a",
        state.clone(),
        values.clone(),
        prepared.clone(),
        egress.clone(),
    )
    .await;
    connect(&first, &owner("tenant-a")).await.unwrap();
    assert_eq!(metadata(&state)["next_transaction_generation"], 9);
    assert_eq!(metadata(&state)["pending"], serde_json::json!([]));
    assert_eq!(count(&first, &owner("tenant-a")).await, 1);
    drop(first);
    drop(prepared);
    let reopened = Arc::new(
        PreparedVaultStore::open_shared(values.clone(), journal, "secrets.prepared-transactions")
            .unwrap(),
    );
    reopened.initialize().await.unwrap();
    let restored = backend("tenant-a", state, values, reopened, egress.clone()).await;
    assert_eq!(count(&restored, &owner("tenant-a")).await, 1);
    assert!(restored
        .handle_connection(
            &owner("tenant-b"),
            ConnectionRequest::Search(SearchRequest {
                query: String::new(),
                limit: 10
            })
        )
        .await
        .is_err());
    assert_eq!(egress.0.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn another_coordinator_cannot_retire_unpublished_or_uncleared_recovery_receipts() {
    for fail_at in [3, 4] {
        let journal = Arc::new(MemoryState::new());
        let values = Arc::new(MemoryStore::new());
        let prepared = Arc::new(
            PreparedVaultStore::open_shared(
                values.clone(),
                journal.clone(),
                "secrets.prepared-transactions",
            )
            .unwrap(),
        );
        let state_a = Arc::new(CoordinatorState::new(fail_at));
        let state_b = Arc::new(CoordinatorState::new(usize::MAX));
        let egress = Arc::new(ProviderFixture::default());
        let a = backend(
            "tenant-a",
            state_a.clone(),
            values.clone(),
            prepared.clone(),
            egress.clone(),
        )
        .await;
        let result = connect(&a, &owner("tenant-a")).await;
        if fail_at == 3 {
            assert_eq!(
                result,
                Err(HostedCompletionError::Custody(
                    HostedCustodyFailure::PersistConnection
                ))
            );
        } else {
            assert_eq!(result, Ok(()));
        }
        assert_eq!(metadata(&state_a)["pending"].as_array().unwrap().len(), 1);
        let b = backend(
            "tenant-b",
            state_b.clone(),
            values.clone(),
            prepared.clone(),
            egress.clone(),
        )
        .await;
        connect(&b, &owner("tenant-b")).await.unwrap();
        drop(a);
        drop(b);
        drop(prepared);
        let reopened = Arc::new(
            PreparedVaultStore::open_shared(
                values.clone(),
                journal,
                "secrets.prepared-transactions",
            )
            .unwrap(),
        );
        reopened.initialize().await.unwrap();
        let recovered_a = backend(
            "tenant-a",
            state_a.clone(),
            values.clone(),
            reopened.clone(),
            egress.clone(),
        )
        .await;
        let recovered_b = backend("tenant-b", state_b, values, reopened, egress.clone()).await;
        assert_eq!(count(&recovered_a, &owner("tenant-a")).await, 1);
        assert_eq!(count(&recovered_b, &owner("tenant-b")).await, 1);
        assert!(recovered_a
            .handle_connection(
                &owner("tenant-b"),
                ConnectionRequest::Search(SearchRequest {
                    query: String::new(),
                    limit: 10
                })
            )
            .await
            .is_err());
        assert_eq!(metadata(&state_a)["pending"], serde_json::json!([]));
        assert_eq!(
            egress.0.load(Ordering::SeqCst),
            2,
            "recovery must never repeat either provider exchange"
        );
    }
}

#[tokio::test]
async fn failed_pending_publication_releases_its_aborted_receipt_before_another_owner_connects() {
    let journal = Arc::new(MemoryState::new());
    let values = Arc::new(MemoryStore::new());
    let prepared = Arc::new(
        PreparedVaultStore::open_shared(values.clone(), journal, "secrets.prepared-transactions")
            .unwrap(),
    );
    let state_a = Arc::new(CoordinatorState::new(2));
    let egress = Arc::new(ProviderFixture::default());
    let a = backend(
        "tenant-a",
        state_a.clone(),
        values.clone(),
        prepared.clone(),
        egress.clone(),
    )
    .await;
    assert_eq!(
        connect(&a, &owner("tenant-a")).await,
        Err(HostedCompletionError::Custody(
            HostedCustodyFailure::PersistPending
        ))
    );
    assert_eq!(metadata(&state_a)["pending"], serde_json::json!([]));
    assert_eq!(count(&a, &owner("tenant-a")).await, 0);
    let b = backend(
        "tenant-b",
        Arc::new(CoordinatorState::new(usize::MAX)),
        values,
        prepared.clone(),
        egress.clone(),
    )
    .await;
    connect(&b, &owner("tenant-b")).await.unwrap();
    let watermark = prepared.retirement_watermark().await.unwrap().unwrap();
    assert_eq!(u64::from_be_bytes(watermark.protocol_bytes()), 2);
    assert_eq!(egress.0.load(Ordering::SeqCst), 2);
}

#[derive(Clone, Copy)]
enum Interruption {
    BeforePrepare,
    AfterPrepare,
    DuringStaging,
    RetiredIntent,
    ExhaustRetired,
}
struct PausedPreparation {
    inner: Arc<PreparedVaultStore>,
    boundary: Interruption,
    entered: Arc<tokio::sync::Notify>,
}
#[async_trait]
impl SecretStore for PausedPreparation {
    async fn ready(&self) -> Result<(), StoreError> {
        self.inner.ready().await
    }
    async fn get(&self, key: &CredentialRef) -> Result<Secret, StoreError> {
        self.inner.get(key).await
    }
    async fn put(&self, key: &CredentialRef, value: &Secret) -> Result<(), StoreError> {
        self.inner.put(key, value).await
    }
    async fn delete(&self, key: &CredentialRef) -> Result<(), StoreError> {
        self.inner.delete(key).await
    }
}
#[async_trait]
impl PreparedSecretStore for PausedPreparation {
    async fn retirement_watermark(
        &self,
    ) -> Result<Option<SecretTransactionGeneration>, PreparedSecretError> {
        self.inner.retirement_watermark().await
    }
    async fn prepare(
        &self,
        id: SecretTransactionId,
        digest: SecretProposalDigest,
        batch: &SecretBatch,
    ) -> Result<SecretTransactionState, PreparedSecretError> {
        if matches!(
            self.boundary,
            Interruption::BeforePrepare | Interruption::RetiredIntent
        ) {
            if matches!(self.boundary, Interruption::RetiredIntent) {
                let generation = SecretTransactionGeneration::from_protocol_bytes(
                    id.protocol_bytes()[..8].try_into().unwrap(),
                )
                .unwrap();
                self.inner.reclaim(generation).await.unwrap();
            }
            self.entered.notify_one();
            std::future::pending::<()>().await;
        }
        if matches!(self.boundary, Interruption::ExhaustRetired) {
            let generation = SecretTransactionGeneration::from_protocol_bytes(
                id.protocol_bytes()[..8].try_into().unwrap(),
            )
            .unwrap();
            self.inner.reclaim(generation).await.unwrap();
            return Err(PreparedSecretError::Retired);
        }
        let result = self.inner.prepare(id, digest, batch).await;
        if matches!(self.boundary, Interruption::AfterPrepare) {
            result.as_ref().unwrap();
            self.entered.notify_one();
            std::future::pending::<()>().await;
        }
        result
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
        self.inner.abort(id).await
    }
    async fn acknowledge(&self, id: SecretTransactionId) -> Result<(), PreparedSecretError> {
        self.inner.acknowledge(id).await
    }
}
struct PausedStaging {
    inner: Arc<MemoryStore>,
    entered: Arc<tokio::sync::Notify>,
}
#[async_trait]
impl SecretStore for PausedStaging {
    async fn ready(&self) -> Result<(), StoreError> {
        self.inner.ready().await
    }
    async fn get(&self, key: &CredentialRef) -> Result<Secret, StoreError> {
        self.inner.get(key).await
    }
    async fn put(&self, key: &CredentialRef, value: &Secret) -> Result<(), StoreError> {
        self.inner.put(key, value).await?;
        self.entered.notify_one();
        std::future::pending().await
    }
    async fn delete(&self, key: &CredentialRef) -> Result<(), StoreError> {
        self.inner.delete(key).await
    }
}

#[tokio::test]
async fn durable_intent_recovers_each_preparation_boundary_without_provider_replay() {
    for boundary in [
        Interruption::BeforePrepare,
        Interruption::AfterPrepare,
        Interruption::DuringStaging,
        Interruption::RetiredIntent,
    ] {
        let journal = Arc::new(MemoryState::new());
        let values = Arc::new(MemoryStore::new());
        let entered = Arc::new(tokio::sync::Notify::new());
        let transport: Arc<dyn SecretStore> = if matches!(boundary, Interruption::DuringStaging) {
            Arc::new(PausedStaging {
                inner: values.clone(),
                entered: entered.clone(),
            })
        } else {
            values.clone()
        };
        let prepared = Arc::new(
            PreparedVaultStore::open_shared(
                transport,
                journal.clone(),
                "secrets.prepared-transactions",
            )
            .unwrap(),
        );
        let state = Arc::new(CoordinatorState::new(usize::MAX));
        let egress = Arc::new(ProviderFixture::default());
        let first = backend(
            "tenant-a",
            state.clone(),
            values.clone(),
            Arc::new(PausedPreparation {
                inner: prepared.clone(),
                boundary,
                entered: entered.clone(),
            }),
            egress.clone(),
        )
        .await;
        let principal = owner("tenant-a");
        let mut attempt = Box::pin(connect(&first, &principal));
        tokio::select! {
            result = &mut attempt => panic!("expected interruption, got {result:?}"),
            _ = entered.notified() => {}
        }
        assert_eq!(metadata(&state)["pending"].as_array().unwrap().len(), 1);
        assert_eq!(metadata(&state)["pending"][0]["intent"], true);
        assert_eq!(metadata(&state)["connections"], serde_json::json!([]));
        drop(attempt);
        drop(first);
        drop(prepared);
        let reopened = Arc::new(
            PreparedVaultStore::open_shared(
                values.clone(),
                journal,
                "secrets.prepared-transactions",
            )
            .unwrap(),
        );
        reopened.initialize().await.unwrap();
        let restored = backend(
            "tenant-a",
            state.clone(),
            values,
            reopened.clone(),
            egress.clone(),
        )
        .await;
        assert_eq!(
            count(&restored, &principal).await,
            usize::from(matches!(boundary, Interruption::AfterPrepare))
        );
        assert_eq!(metadata(&state)["pending"], serde_json::json!([]));
        assert!(reopened.retirement_watermark().await.unwrap().is_some());
        assert_eq!(egress.0.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn final_retired_attempt_discards_only_its_fenced_intent_and_restart_succeeds() {
    let journal = Arc::new(MemoryState::new());
    let values = Arc::new(MemoryStore::new());
    let prepared = Arc::new(
        PreparedVaultStore::open_shared(
            values.clone(),
            journal.clone(),
            "secrets.prepared-transactions",
        )
        .unwrap(),
    );
    let state = Arc::new(CoordinatorState::new(usize::MAX));
    let egress = Arc::new(ProviderFixture::default());
    let first = backend(
        "tenant-a",
        state.clone(),
        values.clone(),
        Arc::new(PausedPreparation {
            inner: prepared.clone(),
            boundary: Interruption::ExhaustRetired,
            entered: Arc::new(tokio::sync::Notify::new()),
        }),
        egress.clone(),
    )
    .await;
    assert_eq!(
        connect(&first, &owner("tenant-a")).await,
        Err(HostedCompletionError::Custody(
            HostedCustodyFailure::Prepare(PreparedSecretError::Retired)
        ))
    );
    assert_eq!(metadata(&state)["pending"], serde_json::json!([]));
    assert_eq!(metadata(&state)["next_transaction_generation"], 5);
    drop(first);
    drop(prepared);
    let reopened = Arc::new(
        PreparedVaultStore::open_shared(values.clone(), journal, "secrets.prepared-transactions")
            .unwrap(),
    );
    reopened.initialize().await.unwrap();
    let restored = backend("tenant-a", state, values, reopened, egress.clone()).await;
    assert_eq!(count(&restored, &owner("tenant-a")).await, 0);
    assert_eq!(egress.0.load(Ordering::SeqCst), 1);
}

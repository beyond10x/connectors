//! Boundary tests use sentinel credentials and a recording transport, never a provider account.

use super::*;
use crate::hosted_form::{credential_description, documentation_link};
use connector_secrets::MemoryStore;
use connector_state::MemoryState;
use connectors_config::NetworkScopeConfig;
use service::{EgressHttpRequest, EgressHttpResponse, EgressTransportError, EgressWebSocket};

const ORIGIN: &str = "https://grafana.monitoring.example";
const SENTINEL: &str = "SENTINEL-NOT-A-REAL-SERVICE-ACCOUNT-TOKEN";

#[derive(Default)]
struct RecordingEgress {
    urls: Mutex<Vec<String>>,
    reject_credential: bool,
    status: Option<u16>,
    failure: Option<EgressTransportError>,
}

#[async_trait]
impl EgressTransport for RecordingEgress {
    async fn execute(
        &self,
        _authority_ref: &str,
        request: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError> {
        assert_eq!(request.request.method, "GET");
        assert_eq!(request.request.url, format!("{ORIGIN}/api/datasources"));
        assert_eq!(
            request
                .request
                .headers
                .get("Authorization")
                .map(String::as_str),
            Some(format!("Bearer {SENTINEL}").as_str())
        );
        lock(&self.urls).push(request.request.url);
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        Ok(EgressHttpResponse {
            status: self
                .status
                .unwrap_or(if self.reject_credential { 401 } else { 200 }),
            headers: BTreeMap::new(),
            body: b"[]".to_vec(),
        })
    }

    async fn connect_websocket(
        &self,
        _authority_ref: &str,
        _url: String,
        _maximum_message_bytes: usize,
    ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
        Err(EgressTransportError::Refused)
    }
}

fn policy(origin: Option<&str>) -> HostedCatalogConfig {
    HostedCatalogConfig {
        enabled: true,
        public_origin: Some("https://connectors.example/api/connectors/v1".to_owned()),
        grant_ref: Some("grant:catalog-read".to_owned()),
        providers: vec!["grafana".to_owned()],
        bindings: origin
            .map(|origin| {
                BTreeMap::from([(
                    "grafana".to_owned(),
                    HostedCatalogBinding {
                        endpoints: BTreeMap::from([("origin".to_owned(), origin.to_owned())]),
                        network: NetworkScopeConfig::Public,
                    },
                )])
            })
            .unwrap_or_default(),
        ..Default::default()
    }
}

async fn open(
    policy: HostedCatalogConfig,
    values: Arc<MemoryStore>,
    state: Arc<dyn StateStore>,
    egress: Arc<RecordingEgress>,
) -> HostedCatalogBackend {
    HostedCatalogBackend::open(
        "tenant-test".to_owned(),
        policy,
        BTreeSet::new(),
        BTreeSet::new(),
        values.clone(),
        values,
        state,
        egress,
    )
    .await
    .expect("valid deployment policy")
}

fn principal(subject: &str) -> PrincipalContext {
    tests::principal(subject)
}

fn pending(backend: &HostedCatalogBackend, owner: &PrincipalContext) -> (String, String) {
    let created = backend
        .inner
        .create_session(
            owner,
            "grafana",
            "grafana.service_account_token",
            "Monitoring".to_owned(),
        )
        .expect("configured profile is connectable");
    let url = url::Url::parse(created.browser_completion_url.as_deref().unwrap()).unwrap();
    let capability = url
        .fragment()
        .unwrap()
        .strip_prefix("token=")
        .unwrap()
        .to_owned();
    (created.connect_session_ref, capability)
}

async fn connect(backend: &HostedCatalogBackend, owner: &PrincipalContext) -> String {
    let (session, capability) = pending(backend, owner);
    let page = backend.hosted_completion_page(&session).unwrap();
    assert!(page.html.contains(ORIGIN));
    assert!(page
        .html
        .contains("<label>Grafana service account token<input"));
    assert!(page
        .html
        .contains("Your browser SSO password is not an API token."));
    assert!(page
        .html
        .contains("https://grafana.com/docs/grafana/latest/administration/service-accounts/"));
    assert!(!page.html.contains(SENTINEL));
    backend
        .complete_hosted_session(
            &session,
            &capability,
            HostedCompletionSubmission::new(SENTINEL.as_bytes().to_vec()),
        )
        .await
        .unwrap();
    backend
        .inner
        .session_status(owner, &session)
        .unwrap()
        .connection_ref
        .unwrap()
}

#[test]
fn completion_copy_follows_the_selected_catalog_credential() {
    let provider = catalog::provider(catalog::ProviderKey::id("anthropic")).unwrap();
    let ordinary = completion_page(provider, "anthropic.api_key", &[], u64::MAX);
    let admin = completion_page(provider, "anthropic.admin_key", &[], u64::MAX);
    assert!(ordinary.html.contains("<label>API key<input"));
    assert!(ordinary
        .html
        .contains("It authorizes the Models API surface."));
    assert!(!ordinary
        .html
        .contains("It authorizes the Admin API surface only."));
    assert!(admin.html.contains("<label>Admin API key<input"));
    assert!(admin
        .html
        .contains("It authorizes the Admin API surface only."));
    assert!(!admin.html.contains("It authorizes the Models API surface."));
    let description = credential_description("anthropic", "anthropic.api_key");
    assert!(!description.is_empty());
    assert!(ordinary.html.contains(&html_escape(&description)));
    assert!(credential_description("anthropic", "unknown.profile").is_empty());
}

#[test]
fn form_copy_is_escaped_and_documentation_links_are_safe() {
    let mut provider = *catalog::provider(catalog::ProviderKey::id("anthropic")).unwrap();
    let mut field = provider
        .config
        .iter()
        .find(|field| field.binds == "credential.anthropic.api_key")
        .copied()
        .unwrap();
    field.label = "API <key> & \"account\"";
    field.help = "<script>untrusted()</script>";
    field.docs_url = Some("javascript:untrusted()");
    provider.config = Box::leak(vec![field].into_boxed_slice());
    let page = completion_page(&provider, "anthropic.api_key", &[], u64::MAX);
    assert!(page
        .html
        .contains("API &lt;key&gt; &amp; &quot;account&quot;"));
    assert!(page
        .html
        .contains("&lt;script&gt;untrusted()&lt;/script&gt;"));
    assert!(!page.html.contains("<script>untrusted()</script>"));
    assert!(!page.html.contains("javascript:"));
    for invalid in [
        "http://docs.example",
        "javascript:alert(1)",
        "https://user:pass@docs.example",
    ] {
        assert!(documentation_link(invalid).is_none());
    }
    assert!(documentation_link("https://docs.example/setup?a=1&b=2")
        .unwrap()
        .contains("?a=1&amp;b=2"));
}

#[tokio::test]
async fn verification_failure_classes_survive_without_custody_or_replay() {
    use service::{EgressTransportFailure, HostedVerificationFailure};
    let outcomes = [
        (
            Some(401),
            None,
            HostedVerificationFailure::ProviderStatus(401),
        ),
        (
            Some(403),
            None,
            HostedVerificationFailure::ProviderStatus(403),
        ),
        (
            Some(429),
            None,
            HostedVerificationFailure::ProviderStatus(429),
        ),
        (
            Some(302),
            None,
            HostedVerificationFailure::ProviderStatus(302),
        ),
        (
            Some(500),
            None,
            HostedVerificationFailure::ProviderStatus(500),
        ),
        (
            None,
            Some(EgressTransportError::Refused),
            HostedVerificationFailure::DestinationRefused,
        ),
        (
            None,
            Some(EgressTransportError::ResponseTooLarge),
            HostedVerificationFailure::ResponseTooLarge,
        ),
    ];
    let outcomes = outcomes.into_iter().chain(
        [
            EgressTransportFailure::Timeout,
            EgressTransportFailure::Connect,
            EgressTransportFailure::Tls,
            EgressTransportFailure::BodyRead,
            EgressTransportFailure::Other,
        ]
        .map(|failure| {
            (
                None,
                Some(EgressTransportError::Transport(failure)),
                HostedVerificationFailure::Transport(failure),
            )
        }),
    );
    for (status, failure, expected) in outcomes {
        let egress = Arc::new(RecordingEgress {
            status,
            failure,
            ..Default::default()
        });
        let values = Arc::new(MemoryStore::new());
        let backend = open(
            policy(Some(ORIGIN)),
            values,
            Arc::new(MemoryState::new()),
            egress.clone(),
        )
        .await;
        let owner = principal("first");
        let (session, capability) = pending(&backend, &owner);
        let result = backend
            .complete_hosted_session(
                &session,
                &capability,
                HostedCompletionSubmission::new(SENTINEL.as_bytes().to_vec()),
            )
            .await;
        assert_eq!(result, Err(HostedCompletionError::Verification(expected)));
        assert_eq!(lock(&egress.urls).len(), 1);
        let metadata = lock(&backend.inner.metadata);
        assert!(metadata.connections.is_empty());
        assert!(metadata.pending.is_empty());
        assert_eq!(metadata.next_transaction_generation, 1);
        drop(metadata);
        assert_eq!(
            backend
                .inner
                .session_status(&owner, &session)
                .unwrap()
                .state,
            ConnectSessionState::Pending
        );
        assert!(!format!("{result:?}").contains(SENTINEL));
        assert!(!format!("{result:?}").contains(ORIGIN));
    }
}

#[test]
fn completion_page_has_the_exact_session_deadline() {
    let provider = catalog::provider(catalog::ProviderKey::id("grafana")).unwrap();
    let page = completion_page(
        provider,
        "grafana.service_account_token",
        &[],
        1_800_000_123_456,
    );
    assert!(page.html.contains("data-expires-at=\"1800000123456\""));
    assert!(page.html.contains("id=\"expiry\" role=\"timer\""));
}

struct FailingState {
    inner: MemoryState,
    writes: std::sync::atomic::AtomicUsize,
    fail_at: usize,
}

impl StateStore for FailingState {
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
        if self
            .writes
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1
            == self.fail_at
        {
            return Err(connector_state::StateError::Unavailable);
        }
        self.inner.replace(key, body, maximum)
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
async fn custody_state_failures_keep_their_stage_and_recover_committed_outcomes() {
    use service::HostedCustodyFailure;
    for (fail_at, expected) in [
        (1, HostedCustodyFailure::ReserveState),
        (2, HostedCustodyFailure::PersistPending),
        (3, HostedCustodyFailure::PersistConnection),
    ] {
        let state = Arc::new(FailingState {
            inner: MemoryState::new(),
            writes: std::sync::atomic::AtomicUsize::new(0),
            fail_at,
        });
        let values = Arc::new(MemoryStore::new());
        let egress = Arc::new(RecordingEgress::default());
        let backend = open(
            policy(Some(ORIGIN)),
            values.clone(),
            state.clone(),
            egress.clone(),
        )
        .await;
        let owner = principal("first");
        let (session, capability) = pending(&backend, &owner);
        let result = backend
            .complete_hosted_session(
                &session,
                &capability,
                HostedCompletionSubmission::new(SENTINEL.as_bytes().to_vec()),
            )
            .await;
        assert_eq!(result, Err(HostedCompletionError::Custody(expected)));
        assert_eq!(lock(&egress.urls).len(), 1);
        assert_eq!(expected.completion_unconfirmed(), fail_at == 3);
        drop(backend);
        let recovered = open(policy(Some(ORIGIN)), values, state, egress.clone()).await;
        assert_eq!(
            recovered.inner.owned_connections(&owner).len(),
            usize::from(fail_at == 3)
        );
        assert!(recovered
            .inner
            .owned_connections(&principal("other"))
            .is_empty());
        assert_eq!(
            lock(&egress.urls).len(),
            1,
            "recovery must not resend the provider credential"
        );
    }
}

#[tokio::test]
async fn retired_shared_generation_advances_before_custody_without_provider_replay() {
    let values = Arc::new(MemoryStore::new());
    values
        .reclaim(SecretTransactionGeneration::from_protocol_bytes(1_u64.to_be_bytes()).unwrap())
        .await
        .unwrap();
    let egress = Arc::new(RecordingEgress::default());
    let backend = open(
        policy(Some(ORIGIN)),
        values,
        Arc::new(MemoryState::new()),
        egress.clone(),
    )
    .await;
    let (session, capability) = pending(&backend, &principal("first"));
    let result = backend
        .complete_hosted_session(
            &session,
            &capability,
            HostedCompletionSubmission::new(SENTINEL.as_bytes().to_vec()),
        )
        .await;
    assert_eq!(result, Ok(()));
    assert_eq!(lock(&egress.urls).len(), 1);
    assert!(lock(&backend.inner.metadata).pending.is_empty());
    assert_eq!(lock(&backend.inner.metadata).connections.len(), 1);
    assert_eq!(lock(&backend.inner.metadata).next_transaction_generation, 3);
}

async fn invoke(
    backend: &HostedCatalogBackend,
    owner: &PrincipalContext,
    connection: &str,
    input: serde_json::Value,
) -> Result<protocol::operation::InvocationResult, OperationError> {
    let description = describe_hosted(backend, owner).await?;
    invoke_hosted(
        backend,
        owner,
        connection,
        &description.description_ref,
        input,
    )
    .await
}

#[tokio::test]
async fn unconfigured_templates_have_no_setup_and_no_destination() {
    let policy = policy(None);
    assert!(hosted_admitted_origins(&policy).unwrap().is_empty());
    let backend = open(
        policy,
        Arc::new(MemoryStore::new()),
        Arc::new(MemoryState::new()),
        Arc::new(RecordingEgress::default()),
    )
    .await;
    assert!(backend.setup_profiles("grafana").is_empty());
    assert!(backend
        .inner
        .create_session(
            &principal("first"),
            "grafana",
            "grafana.service_account_token",
            "Monitoring".to_owned()
        )
        .is_err());
}

#[test]
fn only_declared_safe_endpoint_values_are_admitted() {
    for value in [
        "http://monitoring.example",
        "https://user:pass@monitoring.example",
        "https://monitoring.example/api",
        "https://monitoring.example/",
        "https://monitoring.example?token=x",
        "https://monitoring.example#fragment",
        "https://{caller}.example",
        "https://monitoring.example\n",
    ] {
        assert!(hosted_admitted_origins(&policy(Some(value))).is_err());
    }
    let normalized = policy(Some("HTTPS://Grafana.Monitoring.Example:443"));
    assert_eq!(hosted_admitted_origins(&normalized).unwrap(), vec![ORIGIN]);
    let mut invalid = policy(Some(ORIGIN));
    invalid
        .bindings
        .get_mut("grafana")
        .unwrap()
        .endpoints
        .insert("unpublished".to_owned(), "x".to_owned());
    assert!(hosted_admitted_origins(&invalid).is_err());
    invalid = policy(Some(ORIGIN));
    invalid.providers = vec!["anthropic".to_owned()];
    assert!(hosted_admitted_origins(&invalid).is_err());
    invalid = policy(Some(ORIGIN));
    invalid.enabled = false;
    assert!(hosted_admitted_origins(&invalid).is_err());
    invalid = policy(Some(ORIGIN));
    invalid.bindings.insert(
        "unknown-provider".to_owned(),
        HostedCatalogBinding::default(),
    );
    assert!(hosted_admitted_origins(&invalid).is_err());
}

#[test]
fn private_reachability_requires_an_explicit_policy_and_conflicts_refuse() {
    let mut configured = policy(Some(ORIGIN));
    assert_eq!(
        hosted_endpoints::hosted_admitted_destinations(&configured).unwrap(),
        vec![(ORIGIN.to_owned(), NetworkScopeConfig::Public)]
    );
    configured.bindings.get_mut("grafana").unwrap().network = NetworkScopeConfig::Operator;
    assert_eq!(
        hosted_endpoints::hosted_admitted_destinations(&configured).unwrap(),
        vec![(ORIGIN.to_owned(), NetworkScopeConfig::Operator)]
    );
    configured.providers.push("argocd".to_owned());
    configured.bindings.insert(
        "argocd".to_owned(),
        HostedCatalogBinding {
            endpoints: BTreeMap::from([("origin".to_owned(), ORIGIN.to_owned())]),
            network: NetworkScopeConfig::Public,
        },
    );
    assert!(hosted_endpoints::hosted_admitted_destinations(&configured).is_err());
}

#[test]
fn fixed_origin_connection_metadata_without_bindings_still_loads() {
    let state: StateFile = serde_json::from_value(serde_json::json!({
        "version": 1,
        "next_transaction_generation": 1,
        "pending": [],
        "connections": [{
            "connection_ref": "catalog:anthropic:existing",
            "provider": "anthropic",
            "instance": "existing",
            "credential": "anthropic.api_key",
            "label": "Existing connection",
            "owner_subject": "first",
            "actor": "user"
        }]
    }))
    .unwrap();
    assert_eq!(
        state.connections[0].binding,
        HostedCatalogBinding::default()
    );
    let config = Inner::config(&state.connections[0], "grant:catalog-read");
    assert!(config.endpoints.is_empty());
    assert!(!config.operator_approved);
    assert_eq!(config.network, NetworkScopeConfig::Public);
    assert!(!serde_json::to_string(&state)
        .unwrap()
        .contains("\"binding\""));
    let mut all_providers = policy(None);
    all_providers.providers.clear();
    let destinations = hosted_admitted_origins(&all_providers).unwrap();
    assert!(destinations
        .iter()
        .any(|origin| origin == "https://api.anthropic.com"));
    assert!(!destinations.iter().any(|origin| origin.contains('{')));
}

#[tokio::test]
async fn acquisition_verifies_exact_destination_and_persists_owner_bound_transport() {
    let store = Arc::new(MemoryStore::new());
    let state = Arc::new(MemoryState::new());
    let egress = Arc::new(RecordingEgress::default());
    let owner = principal("first");
    let backend = open(
        policy(Some(ORIGIN)),
        store.clone(),
        state.clone(),
        egress.clone(),
    )
    .await;
    assert_eq!(
        backend.setup_profiles("grafana")[0].auth_profile,
        "grafana.service_account_token"
    );
    let connection = connect(&backend, &owner).await;
    assert_eq!(lock(&egress.urls).len(), 1);
    drop(backend);
    let backend = open(policy(Some(ORIGIN)), store, state.clone(), egress.clone()).await;
    invoke(&backend, &owner, &connection, serde_json::json!({}))
        .await
        .unwrap();
    assert_eq!(lock(&egress.urls).len(), 2);
    assert!(invoke(
        &backend,
        &principal("second"),
        &connection,
        serde_json::json!({})
    )
    .await
    .is_err());
    let other_tenant = PrincipalContext::hosted(
        "another-tenant".to_owned(),
        "first".to_owned(),
        "first".to_owned(),
        None,
        "snapshot:test".to_owned(),
        "0".repeat(64),
    )
    .unwrap();
    assert!(
        invoke(&backend, &other_tenant, &connection, serde_json::json!({}))
            .await
            .is_err()
    );
    assert_eq!(lock(&egress.urls).len(), 2);
    let bytes = state.read(STATE_KEY, MAX_STATE_BYTES).unwrap().unwrap();
    let serialized = String::from_utf8(bytes).unwrap();
    assert!(serialized.contains(ORIGIN));
    assert!(!serialized.contains(SENTINEL));
    let connection = lock(&backend.inner.metadata).connections[0].clone();
    assert!(connection.last_verified_at_unix_ms.is_some());
}

#[tokio::test]
async fn destination_change_removal_or_stored_tampering_never_spends_the_old_credential() {
    let store = Arc::new(MemoryStore::new());
    let state = Arc::new(MemoryState::new());
    let egress = Arc::new(RecordingEgress::default());
    let owner = principal("first");
    let backend = open(
        policy(Some(ORIGIN)),
        store.clone(),
        state.clone(),
        egress.clone(),
    )
    .await;
    let connection = connect(&backend, &owner).await;
    drop(backend);
    for configured in [
        policy(Some("https://other.monitoring.example")),
        policy(None),
    ] {
        let backend = open(configured, store.clone(), state.clone(), egress.clone()).await;
        assert!(invoke(&backend, &owner, &connection, serde_json::json!({}))
            .await
            .is_err());
        let ConnectionResult::Search { connections } = tests::search(&backend, &owner).await else {
            panic!("search")
        };
        assert_eq!(connections[0].state, ConnectionState::Degraded);
        assert_eq!(lock(&egress.urls).len(), 1);
    }
    let backend = open(policy(Some(ORIGIN)), store, state, egress.clone()).await;
    // A caller cannot replace deployment endpoint bindings through operation input. Even if the
    // declared schema tolerates unused fields, the transport assertion still requires ORIGIN.
    let _ = invoke(
        &backend,
        &owner,
        &connection,
        serde_json::json!({"origin":"https://attacker.example"}),
    )
    .await;
    let count = lock(&egress.urls).len();
    lock(&backend.inner.metadata).connections[0]
        .binding
        .endpoints
        .insert("origin".to_owned(), "https://attacker.example".to_owned());
    assert!(invoke(&backend, &owner, &connection, serde_json::json!({}))
        .await
        .is_err());
    assert_eq!(lock(&egress.urls).len(), count);
}

#[tokio::test]
async fn refused_verification_or_wrong_session_capability_commits_no_connection() {
    let egress = Arc::new(RecordingEgress {
        reject_credential: true,
        ..Default::default()
    });
    let backend = open(
        policy(Some(ORIGIN)),
        Arc::new(MemoryStore::new()),
        Arc::new(MemoryState::new()),
        egress.clone(),
    )
    .await;
    let owner = principal("first");
    let (session, capability) = pending(&backend, &owner);
    assert!(backend
        .inner
        .session_status(&principal("second"), &session)
        .is_err());
    assert!(backend
        .complete_hosted_session(
            &session,
            &"x".repeat(64),
            HostedCompletionSubmission::new(SENTINEL.as_bytes().to_vec())
        )
        .await
        .is_err());
    assert!(lock(&egress.urls).is_empty());
    assert!(backend
        .complete_hosted_session(
            &session,
            &capability,
            HostedCompletionSubmission::new(SENTINEL.as_bytes().to_vec())
        )
        .await
        .is_err());
    assert!(backend.inner.owned_connections(&owner).is_empty());
    assert!(lock(&backend.inner.metadata).pending.is_empty());
}

include!("hosted_dispatch_tests.rs");

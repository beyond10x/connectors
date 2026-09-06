//! GitLab discovery and faithful requests through the real owner-authenticated local socket.
//! Only provider egress and credential persistence are fixtures; no provider is contacted.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use connector_secrets::{CredentialRef, MemoryStore, Secret, SecretStore, StoreError};
use connectors_config::{CatalogIntegrationConfig, InitiationConfig, NetworkScopeConfig};
use connectors_runtime::BackendRegistry;
use integration_catalog::CatalogBackend;
use protocol::{connection as connection_api, operation as operation_api};
use serde_json::{json, Value};
use server::local::LocalOperationDaemon;
use service::{
    EgressHttpRequest, EgressHttpResponse, EgressTransport, EgressTransportError, EgressWebSocket,
    PrincipalContext,
};
use tokio::io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::net::UnixStream;
use tokio::sync::oneshot;

const LIST: &str = "gitlab-pipeline-schedule-list";
const CREATE: &str = "gitlab-pipeline-schedule-create";
const UPDATE: &str = "gitlab-pipeline-schedule-update";
const DELETE: &str = "gitlab-pipeline-schedule-delete";

#[derive(Default)]
struct ObservedStore {
    inner: MemoryStore,
    reads: AtomicUsize,
    readiness: AtomicUsize,
}

#[async_trait]
impl SecretStore for ObservedStore {
    async fn ready(&self) -> Result<(), StoreError> {
        self.readiness.fetch_add(1, Ordering::Relaxed);
        self.inner.ready().await
    }
    async fn get(&self, reference: &CredentialRef) -> Result<Secret, StoreError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.inner.get(reference).await
    }
    async fn put(&self, reference: &CredentialRef, secret: &Secret) -> Result<(), StoreError> {
        self.inner.put(reference, secret).await
    }
    async fn delete(&self, reference: &CredentialRef) -> Result<(), StoreError> {
        self.inner.delete(reference).await
    }
}

#[derive(Default)]
struct FixtureEgress {
    requests: Mutex<Vec<CapturedRequest>>,
}

struct CapturedRequest {
    authority: String,
    method: String,
    url: String,
    body: Option<String>,
}

#[async_trait]
impl EgressTransport for FixtureEgress {
    async fn execute(
        &self,
        authority: &str,
        request: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError> {
        assert_eq!(
            request
                .request
                .headers
                .get("Authorization")
                .map(String::as_str),
            Some("Bearer fixture-gitlab-token")
        );
        let deleting = request.request.method == "DELETE";
        self.requests.lock().unwrap().push(CapturedRequest {
            authority: authority.to_owned(),
            method: request.request.method,
            url: request.request.url,
            body: request.request.body,
        });
        Ok(EgressHttpResponse {
            status: if deleting { 204 } else { 200 },
            headers: BTreeMap::new(),
            body: if deleting {
                Vec::new()
            } else {
                br#"{"id":7}"#.to_vec()
            },
        })
    }
    async fn connect_websocket(
        &self,
        _authority: &str,
        _url: String,
        _maximum: usize,
    ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
        panic!("GitLab schedule requests never open a WebSocket")
    }
}

struct Fixture {
    root: tempfile::TempDir,
    socket: PathBuf,
    store: Arc<ObservedStore>,
    egress: Arc<FixtureEgress>,
    stop: oneshot::Sender<()>,
    serving: tokio::task::JoinHandle<()>,
}

impl Fixture {
    async fn start(allow_writes: bool, with_token: bool) -> Self {
        Self::start_with_reader(allow_writes, with_token, false).await
    }

    async fn start_with_reader(allow_writes: bool, with_token: bool, include_reader: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let credential_file = with_token.then(|| {
            let path = root.path().join("token");
            std::fs::write(&path, "fixture-gitlab-token").unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
            path
        });
        let entry = CatalogIntegrationConfig {
            provider: "gitlab".to_owned(),
            instance: Some("personal".to_owned()),
            label: Some("Personal GitLab".to_owned()),
            grant_ref: "grant:fixture:gitlab".to_owned(),
            initiation: InitiationConfig::Platform,
            allow_writes,
            endpoints: BTreeMap::from([("origin".to_owned(), "https://gitlab.example".to_owned())]),
            usernames: BTreeMap::new(),
            operator_approved: true,
            network: NetworkScopeConfig::Public,
            credential: Some("gitlab.token".to_owned()),
            credential_file,
        };
        let mut configured = vec![entry];
        if include_reader {
            // Two existing configuration entries, each with the same one credential mechanism;
            // this exercises grant selection, without adding credential enrollment behavior.
            let mut reader = configured[0].clone();
            reader.instance = Some("reader".to_owned());
            reader.label = Some("Read-only GitLab".to_owned());
            reader.allow_writes = false;
            configured.push(reader);
        }
        let store = Arc::new(ObservedStore::default());
        let egress = Arc::new(FixtureEgress::default());
        let backend = CatalogBackend::open(
            PrincipalContext::local(&context()).unwrap(),
            &configured,
            &root.path().join("catalog"),
            store.clone(),
            egress.clone(),
        )
        .await
        .unwrap();
        let registry = Arc::new(BackendRegistry::new(vec![Arc::new(backend)]));
        let socket = root.path().join("s");
        let daemon = LocalOperationDaemon::bind(&socket, registry).await.unwrap();
        let (stop, stopped) = oneshot::channel();
        let serving = tokio::spawn(async move {
            daemon
                .serve_until(async {
                    stopped.await.unwrap();
                })
                .await
                .unwrap();
        });
        Self {
            root,
            socket,
            store,
            egress,
            stop,
            serving,
        }
    }

    async fn frame(&self, envelope: Value) -> String {
        let mut stream = UnixStream::connect(&self.socket).await.unwrap();
        let mut bytes = serde_json::to_vec(&envelope).unwrap();
        bytes.push(b'\n');
        stream.write_all(&bytes).await.unwrap();
        let mut response = String::new();
        tokio::time::timeout(
            Duration::from_secs(5),
            BufReader::new(stream).read_line(&mut response),
        )
        .await
        .expect("bounded local response")
        .unwrap();
        response
    }

    async fn operations(
        &self,
        request: operation_api::OperationRequest,
    ) -> Result<operation_api::OperationResult, operation_api::OperationError> {
        let request = operation_api::RequestEnvelope {
            protocol: operation_api::CONTRACT.to_owned(),
            request_id: "fixture-operation".to_owned(),
            context: context(),
            request,
        };
        request.validate().unwrap();
        let text = self.frame(serde_json::to_value(request).unwrap()).await;
        let response: operation_api::ResponseEnvelope = serde_json::from_str(&text).unwrap();
        response.validate().unwrap();
        assert_eq!(response.request_id, "fixture-operation");
        match response.error {
            Some(error) => Err(error),
            None => Ok(response.response.unwrap()),
        }
    }

    async fn connections(&self, query: &str) -> Vec<connection_api::ConnectionSummary> {
        let request = connection_api::RequestEnvelope {
            protocol: connection_api::CONTRACT.to_owned(),
            request_id: "fixture-connection".to_owned(),
            context: context(),
            request: connection_api::ConnectionRequest::Search(connection_api::SearchRequest {
                query: query.to_owned(),
                limit: 64,
            }),
        };
        request.validate().unwrap();
        let text = self.frame(serde_json::to_value(request).unwrap()).await;
        let response: connection_api::ResponseEnvelope = serde_json::from_str(&text).unwrap();
        response.validate().unwrap();
        assert_eq!(response.request_id, "fixture-connection");
        assert!(response.error.is_none(), "{response:?}");
        let Some(connection_api::ConnectionResult::Search { connections }) = response.response
        else {
            panic!("connection search response")
        };
        connections
    }

    async fn search(&self, query: &str) -> Vec<operation_api::OperationSummary> {
        let result = self
            .operations(operation_api::OperationRequest::Search(
                operation_api::SearchRequest {
                    query: query.to_owned(),
                    limit: 25,
                },
            ))
            .await
            .unwrap();
        let operation_api::OperationResult::Search { operations } = result else {
            panic!("operation search response")
        };
        operations
    }

    async fn describe(
        &self,
        operation: &str,
    ) -> Result<operation_api::OperationDescription, operation_api::OperationError> {
        let result = self
            .operations(operation_api::OperationRequest::Describe(
                operation_api::DescribeRequest {
                    operation_ref: operation.to_owned(),
                },
            ))
            .await?;
        let operation_api::OperationResult::Describe(description) = result else {
            panic!("describe response")
        };
        Ok(description)
    }

    fn assert_passive(&self) {
        assert_eq!(
            self.store.reads.load(Ordering::Relaxed),
            0,
            "discovery reads no secret"
        );
        assert_eq!(
            self.store.readiness.load(Ordering::Relaxed),
            0,
            "discovery probes no custody backend"
        );
        assert!(
            self.egress.requests.lock().unwrap().is_empty(),
            "discovery contacts no provider"
        );
    }

    async fn finish(self) {
        self.stop.send(()).unwrap();
        self.serving.await.unwrap();
        assert!(!self.socket.exists());
        self.root.close().unwrap();
    }
}

fn context() -> operation_api::OwnerContext {
    operation_api::OwnerContext {
        tenant_id: "fixture-local".to_owned(),
        agent_id: "fixture-agent".to_owned(),
        agent_revision: 1,
        authority_snapshot_id: "fixture-authority".to_owned(),
        authority_snapshot_sha256: "a".repeat(64),
    }
}

fn invoke(
    description: &operation_api::OperationDescription,
    input: Value,
) -> operation_api::OperationRequest {
    operation_api::OperationRequest::Invoke(operation_api::InvokeRequest {
        operation_ref: description.operation_ref.clone(),
        connection_ref: description.connections[0].connection_ref.clone(),
        description_ref: description.description_ref.clone(),
        input,
        approval_evidence_ref: None,
    })
}

#[tokio::test]
async fn read_only_connection_refuses_schedule_mutations_before_custody_or_egress() {
    let fixture = Fixture::start_with_reader(true, true, true).await;
    let connections = fixture.connections("gitlab").await;
    let reader = connections
        .iter()
        .find(|connection| connection.label == "Read-only GitLab")
        .unwrap();
    for id in [CREATE, UPDATE, DELETE] {
        let description = fixture.describe(id).await.unwrap();
        assert_eq!(
            description.approval,
            operation_api::ApprovalPosture::Required
        );
        assert_eq!(description.connections.len(), 1);
        assert_ne!(
            description.connections[0].connection_ref,
            reader.connection_ref
        );
        let operation_api::OperationRequest::Invoke(mut request) = invoke(
            &description,
            json!({"id":12,"pipeline_schedule_id":7,"body":{"description":"fixture","ref":"main","cron":"0 * * * *"}}),
        ) else {
            unreachable!()
        };
        request.connection_ref = reader.connection_ref.clone();
        assert_eq!(
            fixture
                .operations(operation_api::OperationRequest::Invoke(request))
                .await
                .unwrap_err()
                .code,
            operation_api::OperationErrorCode::NotGranted
        );
    }
    fixture.assert_passive();
    fixture.finish().await;
}

#[tokio::test]
async fn configured_gitlab_connection_is_the_same_passive_reference_in_both_discovery_surfaces() {
    for with_token in [false, true] {
        let fixture = Fixture::start(false, with_token).await;
        let connections = fixture.connections("GITLAB").await;
        assert_eq!(connections.len(), 1);
        let connection = &connections[0];
        assert_eq!(connection.label, "Personal GitLab");
        assert_eq!(connection.integration_ref, "gitlab");
        assert_eq!(connection.state, connection_api::ConnectionState::Created);
        assert_eq!(connection.route, connection_api::ConnectionRoute::Direct);
        assert_eq!(
            connection.initiation,
            vec![connection_api::ConnectionInitiator::Platform]
        );
        assert!(
            connection.scope.is_none()
                && connection.actor.is_none()
                && connection.auth_profile.is_none()
        );
        let operations = fixture.search(LIST).await;
        assert_eq!(operations.len(), 1);
        assert_eq!(
            operations[0].connections[0].connection_ref,
            connection.connection_ref
        );
        assert!(connection.connection_ref.starts_with("connection:gitlab:"));
        assert_ne!(connection.connection_ref, "connection:gitlab:personal");
        assert!(fixture.connections("absent-provider").await.is_empty());
        fixture.assert_passive();
        fixture.finish().await;
    }
}

#[tokio::test]
async fn schedule_words_find_only_operations_admitted_by_the_existing_write_policy() {
    let reader = Fixture::start(false, true).await;
    let operations = reader.search("Pipeline schedule").await;
    assert_eq!(
        operations
            .iter()
            .map(|operation| operation.operation_ref.as_str())
            .collect::<Vec<_>>(),
        [LIST]
    );
    for operation in [CREATE, UPDATE, DELETE] {
        assert_eq!(
            reader.describe(operation).await.unwrap_err().code,
            operation_api::OperationErrorCode::NotGranted
        );
    }
    reader.assert_passive();
    reader.finish().await;
    let writer = Fixture::start(true, true).await;
    let mut actual: Vec<_> = writer
        .search("schedule pipeline")
        .await
        .into_iter()
        .map(|operation| operation.operation_ref)
        .collect();
    actual.sort();
    let mut expected = vec![
        LIST.to_owned(),
        CREATE.to_owned(),
        UPDATE.to_owned(),
        DELETE.to_owned(),
    ];
    expected.sort();
    assert_eq!(actual, expected);
    assert!(writer.search("pipeline schedule absent").await.is_empty());
    writer.assert_passive();
    writer.finish().await;
}

#[tokio::test]
async fn describe_exposes_the_exact_generated_input_and_output_contracts() {
    let fixture = Fixture::start(true, true).await;
    let catalog: Value =
        serde_json::from_str(include_str!("../../../catalog/gitlab.catalog.json")).unwrap();
    for id in [LIST, CREATE, UPDATE, DELETE] {
        let source = catalog["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|operation| operation["id"] == id)
            .unwrap();
        let description = fixture.describe(id).await.unwrap();
        assert_eq!(description.input_schema, source["contract"]["input_schema"]);
        assert_eq!(
            description.output_schema,
            source["contract"]["output_schema"]
        );
        assert_eq!(
            description.approval,
            if id == LIST {
                operation_api::ApprovalPosture::NotRequired
            } else {
                operation_api::ApprovalPosture::Required
            }
        );
    }
    fixture.assert_passive();
    fixture.finish().await;
}

#[tokio::test]
async fn schedule_requests_preserve_complete_json_values_and_optional_update_omission() {
    let fixture = Fixture::start(true, true).await;
    let create = fixture.describe(CREATE).await.unwrap();
    let body = json!({"description":null,"ref":"main","cron":"0 * * * *","active":false,"inputs":[{"name":"choice","value":[0,false,"x"]}],"additional":{"preserved":true}});
    fixture
        .operations(invoke(&create, json!({"id":"group/project","body":body})))
        .await
        .unwrap();
    let update = fixture.describe(UPDATE).await.unwrap();
    fixture
        .operations(invoke(
            &update,
            json!({"id":"group/project","pipeline_schedule_id":7}),
        ))
        .await
        .unwrap();
    fixture
        .operations(invoke(
            &update,
            json!({"id":"group/project","pipeline_schedule_id":7,"body":{}}),
        ))
        .await
        .unwrap();
    let delete = fixture.describe(DELETE).await.unwrap();
    fixture
        .operations(invoke(
            &delete,
            json!({"id":"group/project","pipeline_schedule_id":7}),
        ))
        .await
        .unwrap();
    let list = fixture.describe(LIST).await.unwrap();
    fixture
        .operations(invoke(&list, json!({"id":"group/project","per_page":101})))
        .await
        .unwrap();
    {
        let requests = fixture.egress.requests.lock().unwrap();
        assert_eq!(requests.len(), 5);
        assert!(requests
            .iter()
            .all(|request| request.authority == create.connections[0].connection_ref));
        assert_eq!(requests[0].method, "POST");
        assert_eq!(
            requests[0].url,
            "https://gitlab.example/api/v4/projects/group%2Fproject/pipeline_schedules"
        );
        assert_eq!(
            serde_json::from_str::<Value>(requests[0].body.as_deref().unwrap()).unwrap(),
            body
        );
        assert_eq!(requests[1].method, "PUT");
        assert_eq!(requests[1].body, None);
        assert_eq!(requests[2].body.as_deref(), Some("{}"));
        assert_eq!(requests[3].method, "DELETE");
        assert_eq!(requests[3].body, None);
        assert!(requests[4].url.ends_with("?per_page=101"));
    }
    let before = fixture.egress.requests.lock().unwrap().len();
    for invalid in [
        json!({"id":12,"body":{"description":"x","ref":"main"}}),
        json!({"id":12,"body":null}),
    ] {
        assert_eq!(
            fixture
                .operations(invoke(&create, invalid))
                .await
                .unwrap_err()
                .code,
            operation_api::OperationErrorCode::InvalidInput
        );
    }
    assert_eq!(
        fixture.egress.requests.lock().unwrap().len(),
        before,
        "invalid inputs never reach egress"
    );
    fixture.finish().await;
}

#[tokio::test]
async fn adversary_gitlab_pass1_missing_credentials_and_forged_approval_never_widen_a_placement() {
    let fixture = Fixture::start_with_reader(true, false, true).await;
    let connections = fixture.connections("gitlab read-only").await;
    assert_eq!(connections.len(), 1);
    let reader = &connections[0];
    assert_eq!(reader.label, "Read-only GitLab");
    assert!(fixture
        .connections("read-only missing-word")
        .await
        .is_empty());
    let listed = fixture.search("schedule PIPELINE").await;
    assert_eq!(listed.len(), 4);
    for id in [CREATE, UPDATE, DELETE] {
        let description = fixture.describe(id).await.unwrap();
        assert_eq!(description.connections.len(), 1);
        assert_ne!(
            description.connections[0].connection_ref,
            reader.connection_ref
        );
        let error = fixture
            .operations(operation_api::OperationRequest::Invoke(
                operation_api::InvokeRequest {
                    operation_ref: id.to_owned(),
                    connection_ref: reader.connection_ref.clone(),
                    description_ref: description.description_ref,
                    // Both a supplied approval token and deliberately invalid input must lose to
                    // the selected placement's refusal before credential availability is consulted.
                    approval_evidence_ref: Some("approval:adversarial-forgery".to_owned()),
                    input: json!({"body":null}),
                },
            ))
            .await
            .unwrap_err();
        assert_eq!(error.code, operation_api::OperationErrorCode::NotGranted);
    }
    fixture.assert_passive();
    fixture.finish().await;
}

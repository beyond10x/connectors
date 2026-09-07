use super::*;
use domain::endpoint::{EndpointBinding, EndpointCredentialReference};
use service::{EgressHttpRequest, EgressHttpResponse, EgressTransportError, EgressWebSocket};
use std::sync::Mutex;

#[derive(Default)]
struct HttpFixture {
    authorizations: Mutex<Vec<String>>,
}

#[async_trait]
impl EgressTransport for HttpFixture {
    async fn execute(
        &self,
        reference: &str,
        request: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError> {
        assert!(reference.starts_with("connection:endpoint:"));
        assert_eq!(request.request.method, "GET");
        assert_eq!(
            request.request.url,
            "https://ari.example.test/ari/applications"
        );
        let authorization = request
            .request
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
            .unwrap()
            .1;
        assert!(authorization.starts_with("Basic "));
        self.authorizations
            .lock()
            .unwrap()
            .push(authorization.clone());
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

struct Factory(Arc<HttpFixture>);
impl EndpointEgressFactory for Factory {
    fn transport(
        &self,
        _: &str,
        route: &EndpointRouteLease,
    ) -> Result<Arc<dyn EgressTransport>, EndpointSourceError> {
        assert_eq!(route.logical_url.as_str(), "https://ari.example.test/ari");
        assert!(route.connect_address.is_none());
        Ok(self.0.clone())
    }
}

pub(super) fn owner() -> PrincipalContext {
    PrincipalContext::local(&operation::OwnerContext {
        tenant_id: "tenant-endpoints".to_owned(),
        agent_id: "agent-endpoints".to_owned(),
        agent_revision: 1,
        authority_snapshot_id: "endpoint-authority".to_owned(),
        authority_snapshot_sha256: "a".repeat(64),
    })
    .unwrap()
}

#[tokio::test]
async fn ari_execution_resolves_the_current_named_secret_and_keeps_invalid_input_before_secret_io()
{
    const OPERATION: &str = "asterisk-ari-applications-list";
    assert!(catalog::operation(catalog::OperationKey::id(OPERATION)).is_some());
    let generation = Arc::new(Mutex::new("fixture-password-one".to_owned()));
    let observed = Arc::new(Mutex::new(Vec::new()));
    let fixture = service(
        "asterisk",
        "service-uid",
        serde_json::json!([{"name":"ari","port":8088}]),
    );
    let current = fixture.clone();
    let values = generation.clone();
    let calls = observed.clone();
    let client = client(move |path| {
        calls.lock().unwrap().push(path.to_owned());
        match path {
            "/api/v1/namespaces/apps/services/asterisk" => {
                (200, serde_json::to_value(&current).unwrap())
            }
            "/api/v1/namespaces/apps/secrets/ari-auth" => {
                let secret = k8s_openapi::api::core::v1::Secret {
                    metadata: kube::api::ObjectMeta {
                        namespace: Some("apps".to_owned()),
                        name: Some("ari-auth".to_owned()),
                        ..Default::default()
                    },
                    data: Some(BTreeMap::from([
                        (
                            "password".to_owned(),
                            k8s_openapi::ByteString(values.lock().unwrap().as_bytes().to_vec()),
                        ),
                        (
                            "username".to_owned(),
                            k8s_openapi::ByteString(b"fixture-user".to_vec()),
                        ),
                    ])),
                    ..Default::default()
                };
                (200, serde_json::to_value(secret).unwrap())
            }
            _ if path.starts_with("/api/v1/namespaces/apps/services?") => (
                200,
                serde_json::json!({"apiVersion":"v1", "kind":"ServiceList", "metadata":{}, "items":[current]}),
            ),
            _ if path.starts_with("/apis/") => absent(),
            _ => panic!("unexpected cluster request {path}"),
        }
    });
    let source = Arc::new(source(
        client,
        Arc::new(connector_state::MemoryState::new()),
    ));
    source.refresh().await.unwrap();
    let endpoint_ref = source.list().unwrap()[0].endpoint_ref.clone();
    source
        .bind(
            &endpoint_ref,
            EndpointBinding {
                provider: "asterisk".to_owned(),
                base_path: Some("/ari".to_owned()),
                direct_address: Some("https://ari.example.test".to_owned()),
                database: None,
                tls: None,
                scheme: None,
                credential: Some(EndpointCredentialReference::KubernetesSecret {
                    namespace: "apps".to_owned(),
                    name: "ari-auth".to_owned(),
                    keys: BTreeMap::from([
                        ("username".to_owned(), "username".to_owned()),
                        ("password".to_owned(), "password".to_owned()),
                    ]),
                }),
            },
        )
        .unwrap();
    let context = owner();
    let transport = Arc::new(HttpFixture::default());
    let backend = KubernetesEndpointBackend::new(
        source,
        EndpointPrincipalPolicy::Local(Arc::new(context.clone())),
        Arc::new(Factory(transport.clone())),
    );
    let connection_ref = backend
        .resolve_endpoint(&context, &endpoint_ref, OPERATION)
        .await
        .unwrap();
    let OperationResult::Describe(description) = backend
        .handle(
            &context,
            OperationRequest::Describe(operation::DescribeRequest {
                operation_ref: OPERATION.to_owned(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!("description expected")
    };
    let request = operation::InvokeRequest {
        operation_ref: OPERATION.to_owned(),
        connection_ref,
        description_ref: description.description_ref,
        input: serde_json::json!({}),
        approval_evidence_ref: None,
    };
    let mut invalid = request.clone();
    invalid.input = serde_json::json!(["wrong shape"]);
    assert!(backend.invoke(&context, invalid).await.is_err());
    assert!(!observed
        .lock()
        .unwrap()
        .iter()
        .any(|path| path.contains("/secrets/")));
    assert_eq!(
        backend
            .invoke(&context, request.clone())
            .await
            .unwrap()
            .output,
        serde_json::json!([])
    );
    *generation.lock().unwrap() = "fixture-password-two".to_owned();
    assert_eq!(
        backend
            .invoke(&context, request.clone())
            .await
            .unwrap()
            .output,
        serde_json::json!([])
    );
    let headers = transport.authorizations.lock().unwrap();
    assert_eq!(headers.len(), 2);
    assert_ne!(headers[0], headers[1]);
    drop(headers);
    let before = observed.lock().unwrap().len();
    let mut binding = backend.source.show(&endpoint_ref).unwrap().binding.unwrap();
    binding.direct_address = Some("https://new-ari.example.test".to_owned());
    backend.source.bind(&endpoint_ref, binding).unwrap();
    assert!(backend.invoke(&context, request).await.is_err());
    assert_eq!(
        observed.lock().unwrap().len(),
        before,
        "changed bindings invalidate old descriptions before any source or Secret I/O"
    );
}

use connector_state::StateStore;
use integration_kubernetes::endpoints::EndpointPlacement;
use k8s_openapi::api::core::v1::Service;
use kube::Client;
use serde_json::{json, Value};

pub(super) fn service(name: &str, uid: &str, ports: Value) -> Service {
    serde_json::from_value(
        json!({"metadata":{"name":name,"namespace":"apps","uid":uid},
        "spec":{"selector":{"app":name},"ports":ports}}),
    )
    .unwrap()
}

pub(super) fn client(respond: impl Fn(&str) -> (u16, Value) + Send + Sync + 'static) -> Client {
    let respond = Arc::new(respond);
    let service = tower::service_fn(move |request: http::Request<kube::client::Body>| {
        assert_eq!(request.method(), http::Method::GET);
        let (status, body) = respond(&request.uri().to_string());
        async move {
            Ok::<_, std::io::Error>(
                http::Response::builder()
                    .status(status)
                    .header("content-type", "application/json")
                    .body(kube::client::Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
        }
    });
    Client::new(service, "apps")
}

pub(super) fn absent() -> (u16, Value) {
    (
        404,
        json!({"apiVersion":"v1","kind":"Status","status":"Failure","reason":"NotFound","code":404,"message":"not installed"}),
    )
}

pub(super) fn source(client: Client, store: Arc<dyn StateStore>) -> KubernetesEndpointSource {
    KubernetesEndpointSource::new(
        client,
        "source:cluster".to_owned(),
        BTreeSet::from(["apps".to_owned()]),
        false,
        BTreeMap::from([
            ("asterisk".to_owned(), "grant:ari-read".to_owned()),
            ("loki".to_owned(), "grant:loki-read".to_owned()),
        ]),
        store,
        EndpointPlacement::Local,
    )
    .unwrap()
}

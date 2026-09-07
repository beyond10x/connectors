use super::*;
use connector_state::MemoryState;
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

#[test]
fn every_interface_remains_visible_with_exact_recognition_and_uid_identity() {
    let ports = json!([{"name":"ari","port":8088},{"name":"ami","port":5038},{"name":"metrics","port":9000},
        {"name":"rtp","port":10000,"protocol":"UDP"}]);
    let endpoints = project_service("source", &service("asterisk", "uid-one", ports.clone()));
    assert_eq!(endpoints.len(), 4);
    assert_eq!(endpoints[0].provider.as_deref(), Some("asterisk"));
    assert_eq!(
        endpoints[0].binding.as_ref().unwrap().base_path.as_deref(),
        Some("/ari")
    );
    assert_eq!(endpoints[1].interface, "ami");
    assert_eq!(endpoints[1].state, EndpointState::UnsupportedProtocol);
    assert_eq!(endpoints[2].state, EndpointState::UnknownProvider);
    assert_eq!(endpoints[3].transport, EndpointTransport::Udp);
    let replacement = project_service("source", &service("asterisk", "uid-two", ports));
    assert_ne!(endpoints[0].endpoint_ref, replacement[0].endpoint_ref);
    let exporter = project_service(
        "source",
        &service("prometheus-node-exporter", "uid", json!([{"port":9090}])),
    );
    assert_eq!(exporter[0].provider, None);
}

#[tokio::test]
async fn discovery_consumes_every_page_and_never_reads_a_secret() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded = requests.clone();
    let client = client(move |path| {
        recorded.lock().unwrap().push(path.to_owned());
        assert!(!path.contains("/secrets"));
        if !path.starts_with("/api/v1/namespaces/apps/services") {
            return absent();
        }
        let (start, end, cursor) = if path.contains("continue=second") {
            (256, 300, "")
        } else {
            (0, 256, "second")
        };
        let items: Vec<_> = (start..end)
            .map(|index| {
                service(
                    &format!("svc-{index}"),
                    &format!("uid-{index}"),
                    json!([{"port":8080}]),
                )
            })
            .collect();
        (
            200,
            json!({"apiVersion":"v1","kind":"ServiceList","metadata":{"continue":cursor},"items":items}),
        )
    });
    let source = source(client, Arc::new(MemoryState::new()));
    let scan = source.refresh().await.unwrap();
    assert!(scan.complete, "{:?}", scan.warnings);
    assert_eq!(scan.endpoints.len(), 300);
    assert_eq!(source.list().unwrap().len(), 300);
    assert!(requests
        .lock()
        .unwrap()
        .iter()
        .any(|request| request.contains("continue=second")));
}

#[tokio::test]
async fn partial_refresh_preserves_unseen_resources_and_restart_preserves_bindings() {
    let store = Arc::new(MemoryState::new());
    let source = source(client(|_| absent()), store.clone());
    let endpoints = project_service(
        source.source_ref(),
        &service("loki", "uid", json!([{"port":3100}])),
    );
    let reference = endpoints[0].endpoint_ref.clone();
    source
        .reconcile(EndpointScan {
            endpoints,
            complete: true,
            warnings: Vec::new(),
        })
        .unwrap();
    let binding = EndpointBinding {
        provider: "loki".to_owned(),
        base_path: Some("/logs".to_owned()),
        credential: None,
        direct_address: Some("https://logs.example.test".to_owned()),
        database: None,
        tls: None,
        scheme: None,
    };
    source.bind(&reference, binding.clone()).unwrap();
    source
        .reconcile(EndpointScan {
            endpoints: Vec::new(),
            complete: false,
            warnings: vec!["namespace unavailable".to_owned()],
        })
        .unwrap();
    assert_ne!(source.show(&reference).unwrap().state, EndpointState::Stale);
    let reopened = super::tests::source(client(|_| absent()), store);
    assert_eq!(reopened.show(&reference).unwrap().binding, Some(binding));
    reopened
        .reconcile(EndpointScan {
            endpoints: Vec::new(),
            complete: true,
            warnings: Vec::new(),
        })
        .unwrap();
    assert_eq!(
        reopened.show(&reference).unwrap().state,
        EndpointState::Stale
    );
}

#[tokio::test]
async fn named_secret_resolution_rotates_and_rejects_cross_namespace_binding() {
    let generation = Arc::new(Mutex::new("first".to_owned()));
    let selected = generation.clone();
    let source = source(
        client(move |path| {
            assert_eq!(path, "/api/v1/namespaces/apps/secrets/ari-auth");
            let mut secret = k8s_openapi::api::core::v1::Secret::default();
            secret.metadata.name = Some("ari-auth".to_owned());
            secret.metadata.namespace = Some("apps".to_owned());
            secret.data = Some(BTreeMap::from([(
                "password".to_owned(),
                k8s_openapi::ByteString(selected.lock().unwrap().as_bytes().to_vec()),
            )]));
            (200, serde_json::to_value(secret).unwrap())
        }),
        Arc::new(MemoryState::new()),
    );
    let endpoints = project_service(
        source.source_ref(),
        &service("asterisk", "uid", json!([{"port":8088}])),
    );
    let reference = endpoints[0].endpoint_ref.clone();
    source
        .reconcile(EndpointScan {
            endpoints,
            complete: true,
            warnings: Vec::new(),
        })
        .unwrap();
    let mut binding = EndpointBinding {
        provider: "asterisk".to_owned(),
        base_path: Some("/ari".to_owned()),
        direct_address: None,
        database: None,
        tls: None,
        scheme: None,
        credential: Some(EndpointCredentialReference::KubernetesSecret {
            namespace: "apps".to_owned(),
            name: "ari-auth".to_owned(),
            keys: BTreeMap::from([("password".to_owned(), "password".to_owned())]),
        }),
    };
    let endpoint = source.bind(&reference, binding.clone()).unwrap();
    assert_eq!(
        source
            .resolve_credentials(&endpoint)
            .await
            .unwrap()
            .expose("password"),
        Some("first")
    );
    *generation.lock().unwrap() = "rotated".to_owned();
    assert_eq!(
        source
            .resolve_credentials(&endpoint)
            .await
            .unwrap()
            .expose("password"),
        Some("rotated")
    );
    let Some(EndpointCredentialReference::KubernetesSecret { namespace, .. }) =
        &mut binding.credential
    else {
        unreachable!()
    };
    *namespace = "unadmitted".to_owned();
    assert_eq!(
        source.bind(&reference, binding).unwrap_err(),
        EndpointSourceError::Denied
    );
    let inventory = serde_json::to_string(&source.list().unwrap()).unwrap();
    assert!(!inventory.contains("rotated"));
}

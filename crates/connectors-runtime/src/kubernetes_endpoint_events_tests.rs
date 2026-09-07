use super::*;
use crate::kubernetes_endpoints::tests::{absent, client, owner as context, service, source};
use domain::endpoint::{EndpointBinding, EndpointCredentialReference};
use serde_json::json;
use service::{EgressHttpRequest, EgressHttpResponse, EgressTransportError};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tokio::sync::mpsc;

#[derive(Default)]
struct Transport {
    receiver: Mutex<Option<mpsc::UnboundedReceiver<EgressWebSocketFrame>>>,
    closed: Arc<AtomicBool>,
    headers: Mutex<Vec<BTreeMap<String, String>>>,
}

struct Socket {
    frames: mpsc::UnboundedReceiver<EgressWebSocketFrame>,
    closed: Arc<AtomicBool>,
}
impl Drop for Socket {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::SeqCst);
    }
}
#[async_trait]
impl EgressWebSocket for Socket {
    async fn receive(&mut self) -> Result<EgressWebSocketFrame, EgressTransportError> {
        Ok(self
            .frames
            .recv()
            .await
            .unwrap_or(EgressWebSocketFrame::Closed))
    }
    async fn send_text(&mut self, _: String) -> Result<(), EgressTransportError> {
        Ok(())
    }
    async fn send_pong(&mut self, _: Vec<u8>) -> Result<(), EgressTransportError> {
        Ok(())
    }
    async fn close(&mut self) -> Result<(), EgressTransportError> {
        self.closed.store(true, Ordering::SeqCst);
        Ok(())
    }
}
#[async_trait]
impl EgressTransport for Transport {
    async fn execute(
        &self,
        _: &str,
        _: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError> {
        panic!("subscription must use the declared socket channel")
    }
    async fn connect_websocket(
        &self,
        _: &str,
        _: String,
        _: usize,
    ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
        panic!("declared authentication must be passed")
    }
    async fn connect_websocket_with_headers(
        &self,
        _: &str,
        url: String,
        headers: BTreeMap<String, String>,
        _: usize,
    ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
        assert_eq!(
            url,
            "ws://ari.example.test/ari/events?app=fixture&subscribeAll=false"
        );
        assert!(headers.values().any(|value| value.starts_with("Basic ")));
        self.headers.lock().unwrap().push(headers);
        Ok(Box::new(Socket {
            frames: self.receiver.lock().unwrap().take().unwrap(),
            closed: self.closed.clone(),
        }))
    }
}
struct Factory(Arc<Transport>);
impl EndpointEgressFactory for Factory {
    fn transport(
        &self,
        _: &str,
        route: &EndpointRouteLease,
    ) -> Result<Arc<dyn EgressTransport>, EndpointSourceError> {
        assert_eq!(route.logical_url.as_str(), "http://ari.example.test/ari");
        Ok(self.0.clone())
    }
}

fn subscribe(reference: &str) -> v2::EventRequest {
    v2::EventRequest::Subscribe(v2::SubscribeRequest {
        endpoint_ref: reference.to_owned(),
        channel_binding: "ari-events".to_owned(),
        parameters: BTreeMap::from([("app".to_owned(), json!("fixture"))]),
    })
}

#[tokio::test]
async fn explicit_ari_subscription_is_durable_owner_bound_and_closes_on_unsubscribe() {
    let metadata = Arc::new(connector_state::MemoryState::new());
    let fixture = service("asterisk", "uid", json!([{"name":"ari","port":8088}]));
    let observed = Arc::new(Mutex::new(Vec::new()));
    let calls = observed.clone();
    let source = Arc::new(source(
        client(move |path| {
            calls.lock().unwrap().push(path.to_owned());
            if path.starts_with("/api/v1/namespaces/apps/services?") {
                return (200, json!({"metadata":{},"items":[fixture]}));
            }
            match path {
                "/api/v1/namespaces/apps/services/asterisk" => {
                    (200, serde_json::to_value(&fixture).unwrap())
                }
                "/api/v1/namespaces/apps/secrets/ari-auth" => {
                    let secret = k8s_openapi::api::core::v1::Secret {
                        metadata: kube::api::ObjectMeta {
                            name: Some("ari-auth".to_owned()),
                            namespace: Some("apps".to_owned()),
                            ..Default::default()
                        },
                        data: Some(BTreeMap::from([
                            (
                                "username".to_owned(),
                                k8s_openapi::ByteString(b"fixture-user".to_vec()),
                            ),
                            (
                                "password".to_owned(),
                                k8s_openapi::ByteString(b"fixture-password".to_vec()),
                            ),
                        ])),
                        ..Default::default()
                    };
                    (200, serde_json::to_value(secret).unwrap())
                }
                _ => absent(),
            }
        }),
        metadata.clone(),
    ));
    source.refresh().await.unwrap();
    let reference = source.list().unwrap()[0].endpoint_ref.clone();
    source
        .bind(
            &reference,
            EndpointBinding {
                provider: "asterisk".to_owned(),
                base_path: Some("/ari".to_owned()),
                direct_address: Some("http://ari.example.test".to_owned()),
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
    let (sender, receiver) = mpsc::unbounded_channel();
    let transport = Arc::new(Transport {
        receiver: Mutex::new(Some(receiver)),
        ..Default::default()
    });
    let owner = context();
    let backend = KubernetesEndpointBackend::new(
        source.clone(),
        EndpointPrincipalPolicy::Local(Arc::new(owner.clone())),
        Arc::new(Factory(transport.clone())),
    );
    let mut invalid_request = subscribe(&reference);
    if let v2::EventRequest::Subscribe(request) = &mut invalid_request {
        request
            .parameters
            .insert("host".to_owned(), json!("other.test"));
    }
    assert!(backend
        .endpoint_event_v2(&owner, invalid_request)
        .await
        .is_err());
    assert!(!observed
        .lock()
        .unwrap()
        .iter()
        .any(|path| path.contains("/secrets/")));
    let v2::EventResult::Subscribe {
        subscription_ref,
        channel,
    } = backend
        .endpoint_event_v2(&owner, subscribe(&reference))
        .await
        .unwrap()
    else {
        panic!("subscription")
    };
    assert_eq!(transport.headers.lock().unwrap().len(), 1);
    backend
        .endpoint_event_v2(&owner, subscribe(&reference))
        .await
        .unwrap();
    assert_eq!(
        transport.headers.lock().unwrap().len(),
        1,
        "active subscribe is idempotent"
    );
    sender.send(EgressWebSocketFrame::Text(json!({"type":"ApplicationRegistered","application":"fixture","timestamp":"2026-09-07T10:00:00Z"}).to_string())).unwrap();
    let event::EventResult::Receive { events, .. } = backend
        .endpoint_event(
            &owner,
            event::EventRequest::Receive(event::ReceiveRequest {
                channel_ref: channel.channel_ref.clone(),
                after: None,
                limit: 10,
                wait_ms: 1000,
            }),
        )
        .await
        .unwrap()
    else {
        panic!("events")
    };
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "application-registered");
    let other = PrincipalContext::local(&operation::OwnerContext {
        tenant_id: "other".to_owned(),
        agent_id: "agent-endpoints".to_owned(),
        agent_revision: 1,
        authority_snapshot_id: "endpoint-authority".to_owned(),
        authority_snapshot_sha256: "a".repeat(64),
    })
    .unwrap();
    assert!(backend
        .endpoint_event(
            &other,
            event::EventRequest::Replay(event::ReplayRequest {
                event_ref: events[0].event_ref.clone()
            })
        )
        .await
        .is_err());
    backend
        .endpoint_event_v2(
            &owner,
            v2::EventRequest::Unsubscribe(v2::UnsubscribeRequest { subscription_ref }),
        )
        .await
        .unwrap();
    assert!(transport.closed.load(Ordering::SeqCst));
    let reopened = EndpointEvents::new(&source);
    assert_eq!(reopened.replay(&events[0].event_ref).unwrap(), events[0]);
    assert!(!reopened.is_active(&channel.channel_ref));
    let (new_sender, receiver) = mpsc::unbounded_channel();
    *transport.receiver.lock().unwrap() = Some(receiver);
    transport.closed.store(false, Ordering::SeqCst);
    backend
        .endpoint_event_v2(&owner, subscribe(&reference))
        .await
        .unwrap();
    let mut binding = source.show(&reference).unwrap().binding.unwrap();
    binding.base_path = Some("/changed".to_owned());
    source.bind(&reference, binding).unwrap();
    new_sender
        .send(EgressWebSocketFrame::Text(
            json!({"type":"ApplicationRegistered",
        "application":"fixture","timestamp":"2026-09-07T10:00:01Z"})
            .to_string(),
        ))
        .unwrap();
    tokio::time::timeout(Duration::from_secs(1), async {
        while !transport.closed.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        backend
            .events
            .receive(&channel.channel_ref, 0, 10)
            .unwrap()
            .0
            .len(),
        1,
        "a frame received after binding revocation must not enter the spool"
    );
    assert!(backend
        .endpoint_event(
            &owner,
            event::EventRequest::Replay(event::ReplayRequest {
                event_ref: events[0].event_ref.clone()
            })
        )
        .await
        .is_err());
}

#[test]
fn escaped_secret_and_nested_secret_values_never_enter_the_event_spool() {
    let value: Value =
        serde_json::from_str(r#"{"payload":{"value":"fixture\u002dpassword"}}"#).unwrap();
    assert!(contains_secret(&value, &["fixture-password".to_owned()]));
}

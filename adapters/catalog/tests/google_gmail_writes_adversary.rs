//! Adversary pass against the Gmail draft writes: the story's outcome — the
//! approval to send binds a draft the operator can read first, and a changed
//! draft sends nothing — driven through the engine and the declared input
//! schema against a scripted provider. No network, no credential.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::Result;
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteMethod, WriteOutcome,
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex},
};

const DRAFT: &str = "fixture-draft-1";
/// The message the stored draft carries, which the issuer read and pinned.
const DRAFT_MESSAGE: &str = "fixture-draft-message-2";
/// `To: recipient@example.test`, `Subject: Fixture`, a body: what was read.
const READ_RAW: &str =
    "VG86IHJlY2lwaWVudEBleGFtcGxlLnRlc3QNClN1YmplY3Q6IEZpeHR1cmUNCg0KRml4dHVyZSBib2R5DQo";
/// `To: other@example.test`, a body `Other`: what nobody read.
const UNREAD_RAW: &str = "VG86IG90aGVyQGV4YW1wbGUudGVzdA0KDQpPdGhlcg0K";

type Call = (Vec<String>, Vec<(String, String)>);
struct Reads {
    responses: Mutex<VecDeque<HttpResponse>>,
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Reads {
    async fn get(&self, path: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        self.calls.lock().unwrap().push((
            path.iter().map(|s| s.to_string()).collect(),
            query
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ));
        Ok(self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected provider read"))
    }
}
type Sent = Arc<Mutex<Vec<(WriteMethod, Vec<String>, Value)>>>;
struct Write {
    sent: Sent,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for Write {
    async fn send_json(
        self: Box<Self>,
        method: WriteMethod,
        path: &[&str],
        _query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        self.sent.lock().unwrap().push((
            method,
            path.iter().map(|s| s.to_string()).collect(),
            body.clone(),
        ));
        Ok(HttpResponse {
            status: 200,
            headers: Default::default(),
            body: serde_json::to_vec(&json!({"id": "fixture-sent-message-1",
                "threadId": "fixture-thread-1", "labelIds": ["SENT"]}))
            .unwrap(),
        })
    }
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn engine() -> Engine {
    let file: Value = serde_json::from_slice(
        &std::fs::read(root().join("providers/google-gmail/operations.json")).unwrap(),
    )
    .unwrap();
    let selections: Vec<Selection> = serde_json::from_value(file["operations"].clone()).unwrap();
    let bundle = bundle::load(&root().join("generated/bundles"), "google-gmail").unwrap();
    Engine::new(&bundle, "", &selections).unwrap()
}
/// The stored draft as `users.drafts.get` answers it: unchanged since the
/// issuer read it, so its `message.id` is the pinned one.
fn stored_draft() -> Reads {
    Reads {
        responses: Mutex::new(VecDeque::from([HttpResponse {
            status: 200,
            headers: Default::default(),
            body: serde_json::to_vec(&json!({"id": DRAFT,
                "message": {"id": DRAFT_MESSAGE, "threadId": "fixture-thread-1",
                            "labelIds": ["DRAFT"], "raw": READ_RAW}}))
            .unwrap(),
        }])),
        calls: Mutex::new(Vec::new()),
    }
}
/// A send pinned to the draft the issuer read, whose `body` also carries a
/// `message`: Google's drafts guide says a send replaces the draft's content
/// with it before sending, so what leaves is a message nobody read.
fn replacing_send() -> Value {
    json!({"userId": "me", "messageId": DRAFT_MESSAGE,
           "body": {"id": DRAFT, "message": {"raw": UNREAD_RAW}}})
}

/// The story's outcome: the approval to send binds a draft the operator can
/// read first, and a changed draft sends nothing. A send whose `body` carries
/// a replacement `message` passes the `messageId` preflight — the stored draft
/// is unchanged — and its POST carries content the guard never compared.
/// Nothing may reach `POST /drafts/send` with a `message` in its body.
#[tokio::test]
async fn a_send_whose_body_replaces_the_draft_message_sends_nothing() {
    let engine = engine();
    let reads = stored_draft();
    let sent: Sent = Arc::new(Mutex::new(Vec::new()));
    match engine
        .prepare(&reads, "fixture", "users.drafts.send", replacing_send())
        .await
    {
        Err(_) => {}
        Ok(prepared) => {
            let outcome = prepared
                .execute(Box::new(Write { sent: sent.clone() }))
                .await;
            let applied = matches!(outcome, WriteOutcome::Applied(Ok(_)));
            let sent = sent.lock().unwrap().clone();
            panic!(
                "a send replacing the draft's message passed the preflight \
                 (applied: {applied}); preflight reads {:?}, POSTs {sent:?}",
                reads.calls.lock().unwrap()
            );
        }
    }
    assert!(sent.lock().unwrap().is_empty(), "a write was sent");
}

/// What an agent reads in `operations describe` and what the owner's approval
/// issuance validates the input against (`validate_write_value`): the send's
/// description says to send `body` as `{"id": draft id}` only, so its declared
/// input schema must refuse a `body.message`. It declares `body` as any object.
#[test]
fn the_send_declaration_refuses_a_body_message() {
    let declaration = engine()
        .declarations(&[Effect::Write])
        .into_iter()
        .find(|o| o.id == "users.drafts.send")
        .unwrap();
    let pinned_only = json!({"userId": "me", "messageId": DRAFT_MESSAGE, "body": {"id": DRAFT}});
    connectors_sdk::validate_write_value(&declaration.input_schema, &pinned_only)
        .expect("the pinned send is refused by its own declaration");
    assert!(
        connectors_sdk::validate_write_value(&declaration.input_schema, &replacing_send()).is_err(),
        "the send declaration admits body.message; declared body schema: {}",
        declaration.input_schema["properties"]["body"]
    );
}

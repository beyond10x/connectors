//! Executable consumer of generated code, copied into an isolated Cargo fixture.
//!
//! Read capabilities cannot dispatch writes.
//! ```compile_fail
//! use generated_write_fixture::{writes::PreparedWrite, Native};
//! use connectors_sdk::AuthenticatedHttp;
//! async fn invalid(request: PreparedWrite<Native>, get: Box<dyn AuthenticatedHttp>) {
//!     request.execute(get).await;
//! }
//! ```
//! Prepared requests cannot be reused, including after a lost response.
//! ```compile_fail
//! use generated_write_fixture::{writes::PreparedWrite, Native};
//! use connectors_sdk::AuthenticatedWrite;
//! async fn invalid(request: PreparedWrite<Native>, a: Box<dyn AuthenticatedWrite>, b: Box<dyn AuthenticatedWrite>) {
//!     request.execute(a).await;
//!     request.execute(b).await;
//! }
//! ```
//! Capabilities are consumed too.
//! ```compile_fail
//! use generated_write_fixture::{writes::PreparedWrite, Native};
//! use connectors_sdk::AuthenticatedWrite;
//! async fn invalid(a: PreparedWrite<Native>, b: PreparedWrite<Native>, cap: Box<dyn AuthenticatedWrite>) {
//!     a.execute(cap).await;
//!     b.execute(cap).await;
//! }
//! ```

#[path = "../generated/runtime.rs"]
mod reads;
#[path = "../generated/writes.rs"]
pub mod writes;
use connectors_core::{Descriptor, Error, ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, AuthenticatedWrite, HttpResponse, WriteOutcome};
use fixture_writes_types::requests::{ItemUpdateOutput, ItemUpdateOutputDetail, ItemUpdateRequest};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub struct Native;
impl writes::WriteBindings for Native {
    fn prepare_item_update(
        &self,
        input: &ItemUpdateRequest,
    ) -> Result<writes::ItemUpdateRequestContext> {
        if input.version < 1 {
            return Err(Error::invalid("native preflight refused"));
        }
        Ok(writes::ItemUpdateRequestContext {})
    }
    fn finish_item_update(
        &self,
        input: ItemUpdateRequest,
        _: writes::ItemUpdateRequestContext,
        response: Result<HttpResponse>,
    ) -> WriteOutcome<ItemUpdateOutput> {
        match response {
            Err(error) => WriteOutcome::Unknown(error),
            Ok(response) if response.status == 409 => {
                WriteOutcome::Refused(Error::invalid("native guard refused"))
            }
            Ok(response) if response.status == 200 => WriteOutcome::Applied(Ok(ItemUpdateOutput {
                id: if input.version == 2 {
                    String::new()
                } else {
                    input.id
                },
                detail: ItemUpdateOutputDetail {
                    version: input.version,
                    enabled: input.enabled,
                    note: None,
                },
            })),
            Ok(_) => WriteOutcome::Unknown(Error::new(
                ErrorCode::UpstreamProtocol,
                "ambiguous native reply",
            )),
        }
    }
}
struct ReadPort;
#[async_trait::async_trait]
impl AuthenticatedHttp for ReadPort {
    async fn get(&self, _: &[&str], _: &[(&str, String)]) -> Result<HttpResponse> {
        panic!("read must not dispatch a write")
    }
}
impl reads::Bindings for Native {
    fn prepare_item_get(
        &self,
        _: &fixture_types::requests::ItemGetRequest,
    ) -> Result<reads::ItemGetRequestContext> {
        panic!("unexpected read preparation")
    }
    fn finish_item_get(
        &self,
        _: fixture_types::requests::ItemGetRequest,
        _: reads::ItemGetRequestContext,
        _: HttpResponse,
    ) -> Result<Value> {
        panic!("unexpected read response")
    }
}
struct WritePort {
    sends: Arc<AtomicUsize>,
    status: u16,
}
#[async_trait::async_trait]
impl AuthenticatedWrite for WritePort {
    async fn put_json(
        self: Box<Self>,
        segments: &[&str],
        query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse> {
        self.sends.fetch_add(1, Ordering::SeqCst);
        assert_eq!(segments, ["items", "space/item?literal"]);
        assert_eq!(query.len(), 1);
        assert_eq!(query[0].0, "version");
        assert_eq!(
            body,
            &json!({"enabled":true,"at":"2026-09-11T12:00:00Z","deferred":false,"label":"owned fixture","number":i64::MIN})
        );
        if self.status == 0 {
            Err(Error::unavailable())
        } else {
            Ok(HttpResponse {
                status: self.status,
                headers: Default::default(),
                body: Vec::new(),
            })
        }
    }
}
fn descriptor() -> Descriptor {
    serde_json::from_str(include_str!("../generated/private-descriptor.json")).unwrap()
}
fn input(version: i64) -> Value {
    json!({"id":"space/item?literal","version":version,"enabled":true,"at":"2026-09-11T12:00:00Z"})
}
fn capability(sends: &Arc<AtomicUsize>, status: u16) -> Box<dyn AuthenticatedWrite> {
    Box::new(WritePort {
        sends: sends.clone(),
        status,
    })
}

#[tokio::test]
async fn exact_request_and_typed_output() {
    let sends = Arc::new(AtomicUsize::new(0));
    let request =
        writes::prepare(&descriptor(), Arc::new(Native), "item.update", input(1)).unwrap();
    assert_eq!(sends.load(Ordering::SeqCst), 0);
    let WriteOutcome::Applied(Ok(output)) = request.execute(capability(&sends, 200)).await else {
        panic!("expected applied")
    };
    assert_eq!(
        output,
        json!({"id":"space/item?literal","detail":{"version":1,"enabled":true,"note":null}})
    );
    assert_eq!(sends.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn errors_keep_effect_knowledge_and_never_retry() {
    for (version, status) in [(1, 0), (1, 500), (1, 409), (2, 200)] {
        let sends = Arc::new(AtomicUsize::new(0));
        let request = writes::prepare(
            &descriptor(),
            Arc::new(Native),
            "item.update",
            input(version),
        )
        .unwrap();
        let outcome = request.execute(capability(&sends, status)).await;
        match status {
            0 | 500 => assert!(matches!(outcome, WriteOutcome::Unknown(_))),
            409 => assert!(matches!(outcome, WriteOutcome::Refused(_))),
            200 => assert!(matches!(
                outcome,
                WriteOutcome::Applied(Err(Error {
                    code: ErrorCode::UpstreamProtocol,
                    ..
                }))
            )),
            _ => unreachable!(),
        }
        assert_eq!(sends.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn admission_errors_and_legacy_projection_cannot_dispatch() {
    use connectors_sdk::Adapter;
    let descriptor = descriptor();
    let mut malformed = input(1);
    malformed["extra"] = json!("refuse");
    let mut date = input(1);
    date["at"] = json!("not a date");
    let mut integer = input(1);
    integer["version"] = json!(u64::MAX);
    for input in [malformed, date, integer, input(0)] {
        assert!(writes::prepare(&descriptor, Arc::new(Native), "item.update", input).is_err());
    }
    assert!(writes::prepare(&descriptor, Arc::new(Native), "item.get", json!({"id":"x"})).is_err());
    let mut forged = descriptor.clone();
    forged
        .operations
        .iter_mut()
        .find(|op| op.id == "item.update")
        .unwrap()
        .input_schema["additionalProperties"] = json!(true);
    assert!(writes::prepare(&forged, Arc::new(Native), "item.update", input(1)).is_err());
    let read: Descriptor =
        serde_json::from_str(include_str!("../generated/descriptor.json")).unwrap();
    reads::verify_descriptor(&read).unwrap();
    let adapter = reads::GeneratedAdapter {
        descriptor: read,
        http: Arc::new(ReadPort),
        bindings: Native,
    };
    assert!(adapter.invoke("item.update", input(1)).await.is_err());
}

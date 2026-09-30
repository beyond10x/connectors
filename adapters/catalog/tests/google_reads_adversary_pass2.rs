//! Adversary pass 2 against the Google Drive and Google Slides read selections:
//! claims of the guides and the selection descriptions an agent reads, driven
//! through the engine against a scripted provider. No network, no credential.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Engine, Selection};
use connectors_core::{ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::Path, sync::Mutex};

type Call = (Vec<String>, Vec<(String, String)>);

struct Scripted {
    responses: Mutex<VecDeque<HttpResponse>>,
    calls: Mutex<Vec<Call>>,
}
#[async_trait::async_trait]
impl AuthenticatedHttp for Scripted {
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
fn scripted(status: u16, content_type: &str, body: &[u8]) -> Scripted {
    Scripted {
        responses: Mutex::new(VecDeque::from([HttpResponse {
            status,
            headers: [("content-type".to_owned(), content_type.to_owned())].into(),
            body: body.to_vec(),
        }])),
        calls: Mutex::new(Vec::new()),
    }
}
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped(provider: &str) -> Vec<Selection> {
    let file: Value = serde_json::from_slice(
        &std::fs::read(root().join(format!("providers/{provider}/operations.json"))).unwrap(),
    )
    .unwrap();
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn drive() -> Engine {
    let bundle = bundle::load(&root().join("generated/bundles"), "google-drive").unwrap();
    Engine::new(&bundle, "/drive/v3", &shipped("google-drive")).unwrap()
}

/// The Drive guide: "the selection declares `response: text` and the body is
/// returned as a JSON string"; the selection: "returned as text". An export
/// with no bytes — an empty spreadsheet exported as `text/csv`, a text
/// `mimeType` the guide invites — is still text, so its body should be `""`.
/// Today the engine returns `null` for any empty 2xx body before it looks at
/// the text mode; that engine change is
/// `story:catalog-engine-provider-refusal-shapes`. Until it lands this case
/// pins today's `null`, so the change is seen when it arrives.
#[tokio::test]
async fn an_empty_text_export_is_the_empty_string() {
    let http = scripted(200, "text/csv", b"");
    let read = drive()
        .read(
            &http,
            "fixture-google-drive",
            "files.export",
            json!({"fileId": "fixture-empty-sheet", "mimeType": "text/csv"}),
        )
        .await
        .unwrap();
    assert_eq!(http.calls.lock().unwrap().len(), 1);
    assert_eq!(
        read["body"],
        Value::Null,
        "an empty export is now returned as {}: if story:catalog-engine-provider-refusal-shapes \
         has landed, flip this case to assert the empty string `\"\"`",
        read["body"]
    );
}

/// The Drive guide's Limits: "does not retry on `429`; a rate-limited read is
/// returned as a refusal". Drive's published usage-limit answer is a `403`
/// whose reason is `userRateLimitExceeded` (or `rateLimitExceeded`), beside
/// `429`. The caller has to be able to tell that apart from a permission
/// denial to decide whether a walk can be resumed, so it should arrive as
/// `rate_limited`, not `forbidden`. Today the engine maps every 403 to
/// `forbidden`; that engine change is
/// `story:catalog-engine-provider-refusal-shapes`. Until it lands this case
/// pins today's `Forbidden`, so the change is seen when it arrives.
#[tokio::test]
async fn a_drive_usage_limit_answer_is_rate_limited_not_forbidden() {
    let body = serde_json::to_vec(&json!({"error": {
        "code": 403, "message": "User Rate Limit Exceeded",
        "errors": [{"domain": "usageLimits", "reason": "userRateLimitExceeded",
                    "message": "User Rate Limit Exceeded"}]}}))
    .unwrap();
    let http = scripted(403, "application/json", &body);
    let refusal = drive()
        .read(
            &http,
            "fixture-google-drive",
            "files.list",
            json!({"q": "trashed = false", "pageSize": 100}),
        )
        .await
        .expect_err("a 403 is a refusal");
    assert_eq!(
        refusal.code,
        ErrorCode::Forbidden,
        "Drive's usage-limit 403 now reaches the caller as {:?}: if \
         story:catalog-engine-provider-refusal-shapes has landed, flip this case to assert \
         RateLimited",
        refusal.code
    );
}

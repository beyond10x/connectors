use async_trait::async_trait;
use connectors_core::Result;
use connectors_gitlab::auth::{self, Failure, ProtectedEntry};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Mutex,
};

const NOW: u64 = 1_788_998_400_000; // 2026-09-10 00:00 UTC.

fn good_token() -> Value {
    json!({"id":91,"user_id":42,"active":true,"revoked":false,
        "scopes":["read_api"],"expires_at":"2026-09-11"})
}
fn response(status: u16, body: Vec<u8>) -> HttpResponse {
    HttpResponse {
        status,
        headers: BTreeMap::new(),
        body,
    }
}
fn ok(value: Value) -> HttpResponse {
    response(200, serde_json::to_vec(&value).unwrap())
}

struct Fixture {
    responses: Mutex<VecDeque<HttpResponse>>,
    paths: Mutex<Vec<Vec<String>>>,
}
impl Fixture {
    fn new(token: HttpResponse) -> Self {
        Self {
            responses: Mutex::new(VecDeque::from([
                ok(json!({"id":42,"state":"active","username":"ignored-additive-field"})),
                token,
            ])),
            paths: Mutex::new(Vec::new()),
        }
    }
}
#[async_trait]
impl AuthenticatedHttp for Fixture {
    async fn get(&self, segments: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        assert!(query.is_empty());
        self.paths
            .lock()
            .unwrap()
            .push(segments.iter().map(|v| (*v).to_owned()).collect());
        Ok(self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("no retries or extra provider reads"))
    }
}

#[test]
fn protected_document_is_strict_bounded_and_not_trimmed() {
    let secret = ProtectedEntry::parse(b" {\"token\":\"fixture-token\"}\n".to_vec())
        .unwrap()
        .into_secret();
    assert_eq!(&secret.0, b"fixture-token");
    for bytes in [
        br#"{"token":""}"#.to_vec(),
        br#"{"token":" padded "}"#.to_vec(),
        br#"{"token":"line\nbreak"}"#.to_vec(),
        br#"{"token":"one","token":"two"}"#.to_vec(),
        br#"{"token":"fixture","url":"https://untrusted.invalid"}"#.to_vec(),
        br#"{"token":"fixture"}{}"#.to_vec(),
        br#"{"token":7}"#.to_vec(),
        b"{\"token\":\"\xff\"}".to_vec(),
        serde_json::to_vec(&json!({"token":"x".repeat(8193)})).unwrap(),
        vec![b' '; auth::DOCUMENT_LIMIT + 1],
    ] {
        assert!(matches!(
            ProtectedEntry::parse(bytes),
            Err(Failure::InvalidEntry)
        ));
    }
}

#[tokio::test]
async fn both_reads_use_the_same_capability_and_evidence_has_an_absolute_expiry() {
    let mut token = good_token();
    token["scopes"] = json!(["api", "read_user"]);
    let fixture = Fixture::new(ok(token));
    let evidence = auth::validate(&fixture, || NOW).await.unwrap();
    assert_eq!(evidence.user_id, 42);
    assert_eq!(
        evidence.granted_scopes.into_iter().collect::<Vec<_>>(),
        ["api", "read_api", "read_user"]
    );
    assert_eq!(evidence.expires_at_ms, Some(NOW + 86_400_000));
    assert_eq!(evidence.valid_until_ms, NOW + 60_000);
    assert_eq!(
        *fixture.paths.lock().unwrap(),
        [vec!["user"], vec!["personal_access_tokens", "self"]]
    );

    let evidence = auth::validate(&Fixture::new(ok(good_token())), || NOW)
        .await
        .unwrap();
    assert!(!evidence.is_current(NOW - 1));
    assert!(evidence.is_current(NOW));
    assert!(!evidence.is_current(NOW + 60_000));
}

#[tokio::test]
async fn native_identity_grants_and_required_fields_cannot_be_inferred() {
    let cases = [
        ("user_id", json!(43), Failure::IdentityMismatch),
        ("user_id", json!(-1), Failure::InvalidResponse),
        ("id", json!(0), Failure::InvalidResponse),
        ("active", json!(false), Failure::InvalidCredential),
        ("revoked", json!(true), Failure::InvalidCredential),
        ("scopes", json!(["read_user"]), Failure::InsufficientScope),
        (
            "scopes",
            json!(["read_api", "read_api"]),
            Failure::InvalidResponse,
        ),
        (
            "scopes",
            json!(["read_api", "bad scope"]),
            Failure::InvalidResponse,
        ),
        ("scopes", json!(null), Failure::InvalidResponse),
        ("expires_at", json!("2026-09-10"), Failure::Expired),
        ("expires_at", json!("2026-02-30"), Failure::InvalidResponse),
        (
            "expires_at",
            json!("2026-09-11T00:00:00Z"),
            Failure::InvalidResponse,
        ),
        ("expires_at", json!(false), Failure::InvalidResponse),
        ("granular_scopes", json!([]), Failure::InvalidResponse),
    ];
    for (field, value, expected) in cases {
        let mut token = good_token();
        token[field] = value;
        assert!(
            matches!(auth::validate(&Fixture::new(ok(token)), || NOW).await, Err(error) if error == expected),
            "{field}"
        );
    }
    for field in ["id", "user_id", "active", "revoked", "scopes", "expires_at"] {
        let mut token = good_token();
        token.as_object_mut().unwrap().remove(field);
        assert!(
            matches!(
                auth::validate(&Fixture::new(ok(token)), || NOW).await,
                Err(Failure::InvalidResponse)
            ),
            "missing {field}"
        );
    }
    let mut token = good_token();
    token["expires_at"] = Value::Null;
    let evidence = auth::validate(&Fixture::new(ok(token)), || NOW)
        .await
        .unwrap();
    assert_eq!(evidence.expires_at_ms, None);
    assert_eq!(evidence.valid_until_ms, NOW + 60_000);
}

#[tokio::test]
async fn provider_failures_and_invalid_json_do_not_trigger_retries() {
    for (status, expected) in [
        (401, Failure::InvalidCredential),
        (403, Failure::PermissionDenied),
        (404, Failure::InvalidResponse),
        (302, Failure::InvalidResponse),
        (429, Failure::Unavailable),
        (500, Failure::Unavailable),
    ] {
        let fixture = Fixture::new(response(status, b"private upstream diagnostic".to_vec()));
        assert!(matches!(auth::validate(&fixture, || NOW).await, Err(error) if error == expected));
        assert_eq!(fixture.paths.lock().unwrap().len(), 2);
    }
    for body in [b"{}{}".to_vec(), b"[]".to_vec(),
        br#"{"id":91,"user_id":42,"user_id":43,"active":true,"revoked":false,"scopes":["api"],"expires_at":null}"#.to_vec(),
        vec![b' '; auth::DOCUMENT_LIMIT + 1]] {
        assert!(matches!(auth::validate(&Fixture::new(response(200, body)), || NOW).await, Err(Failure::InvalidResponse)));
    }
    let fixture = Fixture::new(ok(good_token()));
    fixture.responses.lock().unwrap()[0] = ok(json!({"id":42,"state":"blocked"}));
    assert!(matches!(
        auth::validate(&fixture, || NOW).await,
        Err(Failure::InvalidCredential)
    ));
    assert_eq!(fixture.paths.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn validation_does_not_extend_time_budgets_or_expiry() {
    for (times, calls) in [
        (vec![NOW, NOW + 30_000], 1),
        (vec![NOW, NOW - 1], 1),
        (vec![NOW, NOW + 10, NOW + 5], 2),
        (vec![NOW, NOW, NOW + 30_000], 2),
    ] {
        let times = Mutex::new(VecDeque::from(times));
        let fixture = Fixture::new(ok(good_token()));
        assert!(matches!(
            auth::validate(&fixture, || times.lock().unwrap().pop_front().unwrap()).await,
            Err(Failure::Deadline)
        ));
        assert_eq!(fixture.paths.lock().unwrap().len(), calls);
    }
    let start = NOW + 86_400_000 - 1_000;
    let times = Mutex::new(VecDeque::from([start, start + 100, start + 999]));
    let evidence = auth::validate(&Fixture::new(ok(good_token())), || {
        times.lock().unwrap().pop_front().unwrap()
    })
    .await
    .unwrap();
    assert_eq!(evidence.valid_until_ms, NOW + 86_400_000);
    assert!(!evidence.is_current(NOW + 86_400_000));
}

#[tokio::test]
async fn protected_entry_validates_through_production_scoped_http() {
    use axum::{
        Router,
        extract::State,
        http::{HeaderMap, StatusCode, Uri},
        response::IntoResponse,
    };
    use connectors_host::http::{HttpConfig, ScopedHttp};
    use connectors_sdk::{Credential, Secret};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct Captured(Secret);
    #[async_trait]
    impl Credential for Captured {
        async fn resolve(&self) -> Result<Secret> {
            Ok(Secret(self.0.0.clone()))
        }
    }
    async fn upstream(
        State(calls): State<Arc<AtomicUsize>>,
        uri: Uri,
        headers: HeaderMap,
    ) -> axum::response::Response {
        calls.fetch_add(1, Ordering::SeqCst);
        if headers
            .get("private-token")
            .and_then(|value| value.to_str().ok())
            != Some("fixture-captured-token")
            || headers.contains_key("authorization")
            || uri.query().is_some_and(|query| !query.is_empty())
        {
            return StatusCode::UNAUTHORIZED.into_response();
        }
        match uri.path() {
            "/api/v4/user" => axum::Json(json!({"id":42,"state":"active"})).into_response(),
            "/api/v4/personal_access_tokens/self" => axum::Json(good_token()).into_response(),
            _ => StatusCode::NOT_FOUND.into_response(),
        }
    }
    let calls = Arc::new(AtomicUsize::new(0));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = Router::new().fallback(upstream).with_state(calls.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let entry = ProtectedEntry::parse(br#"{"token":"fixture-captured-token"}"#.to_vec()).unwrap();
    let http = ScopedHttp::new(
        &HttpConfig {
            base_url: format!("http://{address}/api/v4/"),
            credential: None,
            credential_header: "private-token".into(),
            bearer: false,
            allow_plaintext: true,
            ca_file: None,
        },
        Some(Arc::new(Captured(entry.into_secret()))),
    )
    .unwrap();
    let result = auth::validate(&http, || NOW).await;
    server.abort();
    assert_eq!(result.unwrap().user_id, 42);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

//! The Tavily binding against a local fake server: each operation's request,
//! and the mapping of each answer into the websearch family's result.
use connectors_core::ErrorCode;
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::{Adapter, Credential, Secret};
use connectors_tavily::{CONTENT_BYTES, READ_POST_TIMEOUT, READ_POSTS, Tavily, auth};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Assembled at runtime so no credential-shaped literal sits in the source.
fn key() -> String {
    format!("tvly-{}", "k".repeat(24))
}

struct Fixed(String);
#[async_trait::async_trait]
impl Credential for Fixed {
    async fn resolve(&self) -> connectors_core::Result<Secret> {
        Ok(Secret(self.0.as_bytes().to_vec()))
    }
}

#[derive(Clone, Debug)]
struct Seen {
    head: String,
    body: Value,
}

/// Answers each connection with the next scripted (status, body) and records
/// what it received.
async fn server(answers: Vec<(u16, Value)>) -> (String, Arc<Mutex<Vec<Seen>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = seen.clone();
    tokio::spawn(async move {
        for (status, answer) in answers {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                head.push(socket.read_u8().await.unwrap());
            }
            let head = String::from_utf8(head).unwrap();
            let length: usize = head
                .lines()
                .find_map(|line| {
                    line.split_once(':')
                        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                        .map(|(_, value)| value.trim().parse().unwrap())
                })
                .unwrap_or(0);
            let mut body = vec![0; length];
            socket.read_exact(&mut body).await.unwrap();
            record.lock().unwrap().push(Seen {
                head,
                body: serde_json::from_slice(&body).unwrap_or(Value::Null),
            });
            let bytes = serde_json::to_vec(&answer).unwrap();
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            socket.write_all(&bytes).await.unwrap();
        }
    });
    (format!("http://{address}/"), seen)
}

fn http(base: &str) -> ScopedHttp {
    ScopedHttp::new_with_ca_bytes(
        &HttpConfig {
            base_url: base.into(),
            credential: None,
            credential_header: "authorization".into(),
            bearer: true,
            allow_plaintext: true,
            ca_file: None,
        },
        None,
        None,
    )
    .unwrap()
    .with_credential(Arc::new(Fixed(key())))
    .with_read_posts(&READ_POSTS, READ_POST_TIMEOUT)
    .unwrap()
}

fn tavily(base: &str) -> Tavily {
    let effective =
        json!({"format": "connectors-tavily-local/1", "instance": "t", "api_base": base});
    Tavily::new("t", &effective, Arc::new(http(base)))
        .unwrap()
        .with_clock(|| 1_791_190_000_000)
}

fn search_answer() -> Value {
    json!({"query": "q", "results": [
        {"url": "https://a.example/1", "title": "One", "content": "first snippet",
         "raw_content": "# One\nfull text", "published_date": "2026-10-01", "score": 0.91},
        {"url": "https://a.example/2", "title": "Two", "content": "second snippet", "score": 0.5},
        {"title": "no url"}
    ]})
}

#[tokio::test]
async fn a_search_posts_the_query_and_maps_each_result() {
    let (base, seen) = server(vec![(200, search_answer())]).await;
    let out = tavily(&base)
        .invoke(
            "websearch.search",
            json!({"query": "q", "max_results": 3, "time_range": "week"}),
        )
        .await
        .unwrap();
    let seen = seen.lock().unwrap()[0].clone();
    assert!(
        seen.head.starts_with("POST /search? HTTP/1.1\r\n"),
        "{}",
        seen.head
    );
    assert!(
        seen.head
            .to_ascii_lowercase()
            .contains(&format!("authorization: bearer {}", key()))
    );
    assert_eq!(
        seen.body,
        json!({"query": "q", "max_results": 3, "search_depth": "basic",
               "include_published_date": true, "include_raw_content": false, "time_range": "week"})
    );
    assert_eq!(
        out["results"].as_array().unwrap().len(),
        2,
        "a result without a URL is dropped"
    );
    assert_eq!(
        out["results"][0],
        json!({"url": "https://a.example/1", "title": "One", "description": "first snippet",
               "content": "first snippet", "content_truncated": false,
               "published": "2026-10-01", "score": "0.91"})
    );
    assert_eq!(out["results"][1]["published"], Value::Null);
    assert_eq!(out["complete"], false, "two of three asked for");
    assert_eq!(out["truncation"], json!(["provider_limit"]));
    assert_eq!(
        out["provenance"],
        json!({"instance": "t", "profile": "tavily/2026-10", "received_at": "2026-10-05T08:46:40.000Z"})
    );
}

#[tokio::test]
async fn full_content_asks_for_markdown_and_is_clipped_on_a_character_boundary() {
    let long = format!("{}é", "a".repeat(CONTENT_BYTES - 1));
    let (base, seen) = server(vec![(
        200,
        json!({"results": [
            {"url": "https://a.example/1", "title": "One", "content": "s", "raw_content": long}
        ]}),
    )])
    .await;
    let out = tavily(&base)
        .invoke(
            "websearch.search",
            json!({"query": "q", "max_results": 1, "content": "full"}),
        )
        .await
        .unwrap();
    assert_eq!(
        seen.lock().unwrap()[0].body["include_raw_content"],
        "markdown"
    );
    let content = out["results"][0]["content"].as_str().unwrap();
    assert_eq!(
        content.len(),
        CONTENT_BYTES - 1,
        "the two-byte character is not split"
    );
    assert_eq!(out["results"][0]["content_truncated"], true);
    assert_eq!(out["complete"], true);
    assert_eq!(out["truncation"], json!(["content_bytes"]));
}

#[tokio::test]
async fn a_fetch_reports_pages_and_failures_and_is_complete_when_every_url_answered() {
    let (base, seen) = server(vec![(
        200,
        json!({
            "results": [{"url": "https://a.example/1", "raw_content": "page one"}],
            "failed_results": [{"url": "https://a.example/2", "error": "raw provider text"}]
        }),
    )])
    .await;
    let out = tavily(&base)
        .invoke(
            "websearch.fetch",
            json!({"urls": ["https://a.example/1", "https://a.example/2"]}),
        )
        .await
        .unwrap();
    let seen = seen.lock().unwrap()[0].clone();
    assert!(
        seen.head.starts_with("POST /extract? HTTP/1.1\r\n"),
        "{}",
        seen.head
    );
    assert_eq!(
        seen.body["urls"],
        json!(["https://a.example/1", "https://a.example/2"])
    );
    assert_eq!(out["pages"][0]["content"], "page one");
    assert_eq!(
        out["failed"],
        json!([{"url": "https://a.example/2", "reason": "provider_refused"}])
    );
    assert!(
        !out.to_string().contains("raw provider text"),
        "provider error text leaked"
    );
    assert_eq!(out["complete"], true);
}

#[tokio::test]
async fn a_crawl_that_reaches_its_limit_is_incomplete() {
    let pages: Vec<Value> = (0..2)
        .map(|i| json!({"url": format!("https://a.example/p{i}"), "raw_content": "text"}))
        .collect();
    let (base, seen) = server(vec![(
        200,
        json!({"base_url": "https://a.example", "results": pages}),
    )])
    .await;
    let out = tavily(&base)
        .invoke(
            "websearch.crawl",
            json!({"url": "https://a.example", "limit": 2, "max_depth": 2,
                                         "select_paths": ["/docs/.*"]}),
        )
        .await
        .unwrap();
    let seen = seen.lock().unwrap()[0].clone();
    assert!(
        seen.head.starts_with("POST /crawl? HTTP/1.1\r\n"),
        "{}",
        seen.head
    );
    assert_eq!(seen.body["limit"], 2);
    assert_eq!(seen.body["timeout"], 150);
    assert_eq!(seen.body["select_paths"], json!(["/docs/.*"]));
    assert_eq!(out["pages"].as_array().unwrap().len(), 2);
    assert_eq!(out["complete"], false);
    assert_eq!(out["truncation"], json!(["result_limit"]));
}

#[tokio::test]
async fn an_input_the_profile_does_not_admit_is_refused_before_any_request() {
    let (base, seen) = server(vec![]).await;
    let t = tavily(&base);
    for input in [
        json!({"query": "q", "max_results": 3, "search_depth": "advanced"}),
        json!({"query": "q", "max_results": 21}),
        json!({"query": "q", "max_results": 3, "topic": "finance"}),
        json!({"query": "", "max_results": 3}),
    ] {
        let error = t
            .invoke("websearch.search", input.clone())
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "{input}");
    }
    let error = t.invoke("websearch.map", json!({})).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    assert!(
        seen.lock().unwrap().is_empty(),
        "a refused input reached the provider"
    );
}

#[tokio::test]
async fn provider_refusals_map_to_family_errors_without_provider_text_or_the_key() {
    let (base, _) = server(vec![
        (401, json!({"detail": {"error": "raw provider text"}})),
        (432, json!({"detail": {"error": "raw provider text"}})),
        (500, json!({})),
    ])
    .await;
    let t = tavily(&base);
    let input = json!({"query": "q", "max_results": 1});
    for want in [
        ErrorCode::Unauthorized,
        ErrorCode::Capacity,
        ErrorCode::Unavailable,
    ] {
        let error = t
            .invoke("websearch.search", input.clone())
            .await
            .unwrap_err();
        assert_eq!(error.code, want);
        let rendered = format!("{error:?} {error}");
        assert!(!rendered.contains("raw provider text"), "{rendered}");
        assert!(!rendered.contains(&key()), "{rendered}");
    }
}

#[tokio::test]
async fn the_key_is_validated_by_usage_without_a_search() {
    let (base, seen) = server(vec![
        (
            200,
            json!({"key": {"usage": 3, "limit": 1000}, "account": {"current_plan": "Researcher"}}),
        ),
        (401, json!({"detail": {"error": "invalid"}})),
    ])
    .await;
    let http = http(&base);
    let Ok(ok) = auth::validate(&http, || 1_000).await else {
        panic!("a valid key was refused");
    };
    assert_eq!(ok.plan.as_deref(), Some("Researcher"));
    assert_eq!(ok.valid_until_ms, 1_000 + auth::EVIDENCE_LIFETIME_MS);
    assert!(
        seen.lock().unwrap()[0]
            .head
            .starts_with("GET /usage? HTTP/1.1\r\n")
    );
    assert!(matches!(
        auth::validate(&http, || 1_000).await,
        Err(auth::Failure::InvalidCredential)
    ));
}

#[test]
fn the_protected_entry_is_one_closed_field() {
    assert!(
        auth::ProtectedEntry::parse(format!(r#"{{"api_key":"{}"}}"#, key()).into_bytes()).is_ok()
    );
    for bad in [
        r#"{}"#,
        r#"{"api_key":""}"#,
        r#"{"api_key":"a b"}"#,
        r#"{"api_key":"k","x":1}"#,
        "not json",
    ] {
        assert!(
            auth::ProtectedEntry::parse(bad.as_bytes().to_vec()).is_err(),
            "{bad}"
        );
    }
}

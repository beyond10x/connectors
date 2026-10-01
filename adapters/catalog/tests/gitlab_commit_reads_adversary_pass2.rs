//! Second adversary pass on the GitLab commit-graph reads, `commits.list` and
//! `repository.compare`: the declared parameters pass 1 did not try, the
//! project id as it reaches the wire, and a compare body larger than the
//! provider's response limit. The engine runs over the committed bundle and
//! the shipped selection set; the wire cases go through the host's own
//! `ScopedHttp` to a plain loopback listener.
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_core::{ErrorCode, RESPONSE_LIMIT, Result};
use connectors_host::http::{HttpConfig, ScopedHttp};
use connectors_sdk::{AuthenticatedHttp, HttpResponse};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
fn reads() -> Reads {
    Reads {
        responses: Mutex::new(VecDeque::from([HttpResponse {
            status: 200,
            headers: Default::default(),
            body: b"[]".to_vec(),
        }])),
        calls: Mutex::new(Vec::new()),
    }
}
fn engine() -> Engine {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let file: Value = serde_json::from_slice(
        &std::fs::read(root.join("providers/gitlab/operations.json")).unwrap(),
    )
    .unwrap();
    let shipped: Vec<Selection> = serde_json::from_value(file["operations"].clone()).unwrap();
    let bundle = bundle::load(&root.join("generated/bundles"), "gitlab").unwrap();
    Engine::new(&bundle, "/api/v4", &shipped).unwrap()
}
fn properties(engine: &Engine, id: &str) -> serde_json::Map<String, Value> {
    engine
        .declarations(&[Effect::Read])
        .into_iter()
        .find(|o| o.id == id)
        .unwrap_or_else(|| panic!("`{id}` is not a declared read"))
        .input_schema["properties"]
        .as_object()
        .unwrap()
        .clone()
}
/// Accepted, and the one request's path segments and sorted query.
async fn sent(engine: &Engine, id: &str, input: Value) -> Call {
    let http = reads();
    engine
        .read(&http, "one", id, input.clone())
        .await
        .unwrap_or_else(|e| panic!("{id} {input}: {e:?}"));
    let calls = http.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 1, "{id} {input}");
    let (path, mut query) = calls[0].clone();
    query.sort();
    (path, query)
}
/// Refused as `invalid_input` and nothing sent.
async fn refused(engine: &Engine, id: &str, input: Value) {
    let http = reads();
    let outcome = engine.read(&http, "one", id, input.clone()).await;
    let calls = http.calls.lock().unwrap().clone();
    match outcome {
        Err(error) => {
            assert_eq!(error.code, ErrorCode::InvalidInput, "{id} {input}");
            assert!(calls.is_empty(), "{id} {input}: sent {calls:?}");
        }
        Ok(_) => panic!("{id} {input} was accepted and sent {calls:?}"),
    }
}
fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// The declared parameters pass 1 did not try, typed as the pinned document
/// types them: `commits.list` `path` and `author` are strings; `all`,
/// `with_stats`, `follow` and `trailers` are booleans; `repository.compare`
/// `from_project_id` is an integer and `unidiff` a boolean.
#[test]
fn adversary2_untried_parameters_are_declared_with_the_pinned_types() {
    let engine = engine();
    let string = json!({"type": ["string", "integer"]});
    let boolean = json!({"anyOf": [
        {"type": "boolean"},
        {"type": "string", "enum": ["true", "false"]}
    ]});
    let list = properties(&engine, "commits.list");
    for name in ["path", "author"] {
        assert_eq!(list[name], string, "commits.list {name}");
    }
    for name in ["all", "with_stats", "follow", "trailers"] {
        assert_eq!(list[name], boolean, "commits.list {name}");
    }
    let compare = properties(&engine, "repository.compare");
    assert_eq!(
        compare["from_project_id"]["anyOf"][0],
        json!({"type": "integer"})
    );
    assert_eq!(compare["unidiff"], boolean);
}

/// Each of them is sent under its own name and as its literal text, on the
/// path the pinned document names; a value of the wrong type is refused
/// before any request.
#[tokio::test]
async fn adversary2_untried_parameters_are_sent_verbatim_and_mistyped_ones_refused() {
    let engine = engine();
    let (path, query) = sent(
        &engine,
        "commits.list",
        json!({"id": "org/project", "path": "src/main.rs", "author": "Fixture Author",
               "all": true, "with_stats": "true", "follow": false, "trailers": true}),
    )
    .await;
    assert_eq!(path, ["projects", "org/project", "repository", "commits"]);
    assert_eq!(
        query,
        pairs(&[
            ("all", "true"),
            ("author", "Fixture Author"),
            ("follow", "false"),
            ("path", "src/main.rs"),
            ("trailers", "true"),
            ("with_stats", "true"),
        ])
    );
    let (path, query) = sent(
        &engine,
        "repository.compare",
        json!({"id": 7, "from": "v0.1.0", "to": "v0.2.0",
               "from_project_id": "12", "unidiff": true}),
    )
    .await;
    assert_eq!(path, ["projects", "7", "repository", "compare"]);
    assert_eq!(
        query,
        pairs(&[
            ("from", "v0.1.0"),
            ("from_project_id", "12"),
            ("to", "v0.2.0"),
            ("unidiff", "true"),
        ])
    );
    for input in [
        json!({"id": "org/project", "all": "yes"}),
        json!({"id": "org/project", "with_stats": 1}),
        json!({"id": "org/project", "trailers": null}),
        json!({"id": "org/project", "path": ["a", "b"]}),
        json!({"id": "org/project", "author": {"name": "x"}}),
    ] {
        refused(&engine, "commits.list", input).await;
    }
    for input in [
        json!({"id": 7, "from": "a", "to": "b", "from_project_id": "abc"}),
        json!({"id": 7, "from": "a", "to": "b", "from_project_id": 1.5}),
        json!({"id": 7, "from": "a", "to": "b", "unidiff": "1"}),
    ] {
        refused(&engine, "repository.compare", input).await;
    }
}

/// One plain-HTTP loopback listener that answers each request with the next
/// body, with or without `Content-Length`, and records each request target.
async fn listener(bodies: Vec<(Vec<u8>, bool)>) -> (String, Arc<Mutex<Vec<String>>>) {
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = socket.local_addr().unwrap();
    let targets = Arc::new(Mutex::new(Vec::new()));
    let seen = targets.clone();
    tokio::spawn(async move {
        for (body, length) in bodies {
            let (mut stream, _) = socket.accept().await.unwrap();
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") {
                match stream.read_u8().await {
                    Ok(byte) => header.push(byte),
                    Err(_) => break,
                }
            }
            let request = String::from_utf8_lossy(&header).to_string();
            seen.lock()
                .unwrap()
                .push(request.split_whitespace().nth(1).unwrap_or("").to_string());
            let mut head =
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nconnection: close\r\n"
                    .to_string();
            if length {
                head.push_str(&format!("content-length: {}\r\n", body.len()));
            }
            head.push_str("\r\n");
            let _ = stream.write_all(head.as_bytes()).await;
            for chunk in body.chunks(64 * 1024) {
                if stream.write_all(chunk).await.is_err() {
                    break;
                }
            }
            let _ = stream.shutdown().await;
        }
    });
    (format!("http://{address}/api/v4/"), targets)
}
fn transport(base: &str) -> ScopedHttp {
    ScopedHttp::new(
        &HttpConfig {
            base_url: base.to_string(),
            credential: None,
            credential_header: "authorization".into(),
            bearer: false,
            allow_plaintext: true,
            ca_file: None,
        },
        None,
    )
    .unwrap()
}

/// The project id reaches the wire as one path segment: a nested path's
/// slashes are percent-encoded, a numeric id is its decimal text, and a ref
/// with a slash is encoded in the query. An id the caller already encoded is
/// encoded again (`%` becomes `%25`), on `commits.list` as on `tags.list`,
/// which the base shipped. An empty query leaves a bare `?`, which is
/// harmless. `.` and `..` are refused before any request.
#[tokio::test]
async fn adversary2_project_id_reaches_the_wire_as_one_encoded_segment() {
    let engine = engine();
    let ok = || (b"[]".to_vec(), true);
    let (base, targets) = listener(vec![ok(), ok(), ok(), ok(), ok()]).await;
    let http = transport(&base);
    for (id, input) in [
        (
            "commits.list",
            json!({"id": "org/sub/project", "ref_name": "release/1.0", "per_page": 100}),
        ),
        ("commits.list", json!({"id": 42, "all": true})),
        (
            "repository.compare",
            json!({"id": "org/sub/project", "from": "v0.1.0", "to": "feature/a b"}),
        ),
        ("commits.list", json!({"id": "org%2Fproject"})),
        ("tags.list", json!({"id": "org%2Fproject"})),
    ] {
        engine
            .read(&http, "one", id, input.clone())
            .await
            .unwrap_or_else(|e| panic!("{id} {input}: {e:?}"));
    }
    for id in [".", ".."] {
        let outcome = engine
            .read(&http, "one", "commits.list", json!({"id": id}))
            .await;
        assert_eq!(
            outcome.err().map(|e| e.code),
            Some(ErrorCode::InvalidInput),
            "id {id}"
        );
    }
    assert_eq!(
        *targets.lock().unwrap(),
        [
            "/api/v4/projects/org%2Fsub%2Fproject/repository/commits?ref_name=release%2F1.0&per_page=100",
            "/api/v4/projects/42/repository/commits?all=true",
            "/api/v4/projects/org%2Fsub%2Fproject/repository/compare?from=v0.1.0&to=feature%2Fa+b",
            "/api/v4/projects/org%252Fproject/repository/commits?",
            "/api/v4/projects/org%252Fproject/repository/tags?",
        ]
    );
}

/// A compare body of `size` bytes: GitLab's shape, padded in `diffs`.
fn compare_body(size: usize, timeout: bool) -> Vec<u8> {
    let shell = |pad: usize| {
        serde_json::to_vec(&json!({
            "commit": null,
            "commits": [{"id": "c0ffee02", "parent_ids": ["c0ffee01"]}],
            "diffs": [{"diff": "x".repeat(pad)}],
            "compare_timeout": timeout,
            "compare_same_ref": false
        }))
        .unwrap()
    };
    let overhead = shell(0).len();
    let body = shell(size - overhead);
    assert_eq!(body.len(), size);
    body
}

/// GitLab's compare carries every diff (`APIEntitiesCompare.diffs` in the
/// pinned document) and no parameter omits them. Under the provider's
/// response limit the caller reads `body.compare_timeout`; one byte over it,
/// with or without `Content-Length`, the caller receives a `capacity` error
/// instead, whether GitLab cut the compare short or not, so neither of the two
/// outcomes the guide documents is observable for a large compare.
#[tokio::test]
async fn adversary2_compare_over_the_response_limit_is_a_capacity_error_not_a_compare() {
    let engine = engine();
    let input = json!({"id": "org/project", "from": "v0.1.0", "to": "v0.3.0"});
    let (base, _) = listener(vec![
        (compare_body(RESPONSE_LIMIT, false), true),
        (compare_body(RESPONSE_LIMIT + 1, false), true),
        (compare_body(RESPONSE_LIMIT + 1, true), true),
        (compare_body(RESPONSE_LIMIT + 1, true), false),
    ])
    .await;
    let http = transport(&base);
    let fits = engine
        .read(&http, "one", "repository.compare", input.clone())
        .await
        .unwrap();
    assert_eq!(fits["status"], 200);
    assert_eq!(fits["body"]["compare_timeout"], json!(false));
    assert_eq!(
        fits["body"]["commits"],
        json!([{"id": "c0ffee02", "parent_ids": ["c0ffee01"]}])
    );
    for case in ["false, with length", "true, with length", "true, no length"] {
        let outcome = engine
            .read(&http, "one", "repository.compare", input.clone())
            .await;
        assert_eq!(
            outcome.err().map(|e| e.code),
            Some(ErrorCode::Capacity),
            "compare_timeout {case}"
        );
    }
}

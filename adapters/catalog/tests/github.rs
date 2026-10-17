//! GitHub reads through the catalog provider: what waits on the authenticated
//! user (review requests, assigned issues and pull requests, notifications)
//! and public repository metadata (an organization's repositories, one
//! repository, its releases, its README and single files as text).
//!
//! The shipped selection set is pinned by id, source operation and path,
//! resolves against the committed bundle compiled from the pinned REST
//! description, and is cited row by row in `docs/catalog-github.md`. Each read
//! runs through the provider child against a disposable HTTPS fixture: the
//! exact request (path, query with its time filter, `Authorization: Bearer …`,
//! `Accept: */*` and no API version header) and the returned
//! body are asserted, every list walks two pages to a short page, the search
//! walk stops at its 1,000-result cap, and a spent rate limit is refused as
//! `rate_limited` with its reset time, sent once. Every record is synthetic
//! (`example.com` only). No live credential and no network.
#![cfg(unix)]
use base64::Engine as _;
use connectors_catalog::bundle;
use connectors_catalog_provider::{Effect, Engine, Selection};
use connectors_host::local::{
    config::{Adapter, Executable, Restart, Startup},
    filesystem,
    runtime::{Bootstrap, Child, Failure, Refusal},
};
use connectors_sdk::Secret;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::oneshot,
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{self, pki_types::PrivatePkcs8KeyDer},
};

/// The description's server is `https://api.github.com` with no path.
const BASE: &str = "/";
const TOKEN: &str = "fixture-github-token";
const HEADER: &str = "Bearer fixture-github-token";
const PROFILE: &str = "github.token";
/// The pinned source, its manifest and README, relative to this crate.
const UPSTREAM: &str = "../github/upstream";
const SOURCE: &str = "api.github.com.yaml";
const URL: &str = "https://raw.githubusercontent.com/github/rest-api-description/v2.1.0/descriptions/api.github.com/api.github.com.yaml";
/// The digest of the bytes GitHub publishes at the tag, before redaction.
const UPSTREAM_SHA256: &str = "4a5394a12ecd1dd0f386ab0eddc133befca3ab831f56f902d25767d64c41684c";
const LICENSE_SHA256: &str = "3243761cbac07e6d169a5a2f4e7c25cc544da85248e735df74c3672e055cc87b";
/// The operations of the pinned description and those the bundle reports
/// unsupported, as `bundle_drift.rs` states them.
const OPERATIONS: usize = 859;
const UNSUPPORTED: usize = 1;
const SINCE: &str = "2026-10-01T00:00:00Z";
const DATE: &str = "2026-10-01";

/// The shipped ids, their pinned `operationId` and their pinned path. A
/// renamed, dropped or added id fails here.
const SHIPPED: [(&str, &str, &str); 10] = [
    (
        "file.raw",
        "repos/get-content",
        "/repos/{owner}/{repo}/contents/{path}",
    ),
    ("issues.assigned", "issues/list", "/issues"),
    (
        "notifications.list",
        "activity/list-notifications-for-authenticated-user",
        "/notifications",
    ),
    (
        "readme.raw",
        "repos/get-readme",
        "/repos/{owner}/{repo}/readme",
    ),
    (
        "release.latest",
        "repos/get-latest-release",
        "/repos/{owner}/{repo}/releases/latest",
    ),
    (
        "releases.list",
        "repos/list-releases",
        "/repos/{owner}/{repo}/releases",
    ),
    ("repo.get", "repos/get", "/repos/{owner}/{repo}"),
    ("repos.list", "repos/list-for-org", "/orgs/{org}/repos"),
    (
        "review_requests.list",
        "search/issues-and-pull-requests",
        "/search/issues",
    ),
    ("user.me", "users/get-authenticated", "/user"),
];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn shipped() -> Vec<Selection> {
    let file: Value =
        serde_json::from_slice(&fs::read(root().join("providers/github/operations.json")).unwrap())
            .unwrap();
    assert_eq!(file["format"], "connectors-catalog-operations/1");
    assert_eq!(file["provider"], "github");
    serde_json::from_value(file["operations"].clone()).unwrap()
}
fn committed() -> bundle::Bundle {
    bundle::load(&root().join("generated/bundles"), "github").unwrap()
}

#[test]
fn shipped_github_selections_are_exactly_the_ten_reads() {
    let selections = shipped();
    let bundle = committed();
    assert_eq!(bundle.auth_profile, PROFILE);
    let engine = Engine::new(&bundle, BASE, &selections).unwrap();
    let mut declared: Vec<String> = engine
        .declarations(&[Effect::Read, Effect::Write])
        .into_iter()
        .map(|o| o.id)
        .collect();
    declared.sort();
    let expected: Vec<&str> = SHIPPED.iter().map(|(id, _, _)| *id).collect();
    assert_eq!(declared, expected);
    assert!(engine.declarations(&[Effect::Write]).is_empty());
    for (id, operation_id, path) in SHIPPED {
        let selection = selections.iter().find(|s| s.id == id).unwrap();
        assert_eq!(selection.operation_id, operation_id, "`{id}`");
        assert_eq!(selection.effect, Effect::Read, "`{id}`");
        assert!(selection.rate_limit_headers.is_some(), "`{id}`");
        let operation = bundle
            .inventory
            .operations
            .iter()
            .find(|o| o.operation_id.as_deref() == Some(operation_id))
            .unwrap_or_else(|| panic!("the pinned source lacks `{operation_id}`"));
        assert_eq!(operation.method, "get", "`{id}`");
        assert_eq!(operation.path, path, "`{id}`");
    }
}

/// The search's query is the selection's, never the caller's: `q` is not
/// declared, `since` is, as a timestamp, and so are the assigned list's fixed
/// `filter` and `state` left out.
#[test]
fn the_fixed_query_values_are_not_caller_input() {
    let engine = Engine::new(&committed(), BASE, &shipped()).unwrap();
    let declared = engine.declarations(&[Effect::Read]);
    let schema = |id: &str| {
        declared
            .iter()
            .find(|o| o.id == id)
            .unwrap()
            .input_schema
            .clone()
    };
    let search = schema("review_requests.list");
    let mut names: Vec<&String> = search["properties"].as_object().unwrap().keys().collect();
    names.sort();
    assert_eq!(names, ["page", "per_page", "since"]);
    assert_eq!(search["properties"]["since"]["type"], "string");
    assert!(search["properties"]["since"]["pattern"].is_string());
    let assigned = schema("issues.assigned");
    assert!(assigned["properties"].get("filter").is_none());
    assert!(assigned["properties"].get("state").is_none());
    assert!(assigned["properties"].get("since").is_some());
    for id in ["readme.raw", "file.raw"] {
        let output = &declared.iter().find(|o| o.id == id).unwrap().output_schema;
        assert_eq!(
            output["properties"]["body"],
            json!({"type": "string"}),
            "`{id}`"
        );
    }
}

#[test]
fn a_selection_the_pinned_source_lacks_or_a_fixed_query_out_of_place_is_refused_at_load() {
    let bundle = committed();
    let base = shipped();
    let refused = |change: &dyn Fn(&mut Vec<Selection>)| {
        let mut selections = base.clone();
        change(&mut selections);
        Engine::new(&bundle, BASE, &selections).is_err()
    };
    let index = base
        .iter()
        .position(|s| s.id == "review_requests.list")
        .unwrap();
    let readme = base.iter().position(|s| s.id == "readme.raw").unwrap();
    assert!(refused(
        &|s| s[0].operation_id = "repos/get-content-of-nothing".into()
    ));
    // A fixed name that is no query parameter, a fixed empty value, a fold
    // into a value not fixed, a fold named like a parameter, a prefix with a
    // space, and a fixed value the selection also bounds.
    assert!(refused(&|s| {
        s[index].query_fixed.insert("nope".into(), "x".into());
    }));
    assert!(refused(&|s| {
        s[index].query_fixed.insert("q".into(), String::new());
    }));
    assert!(refused(&|s| s[index]
        .query_folds
        .get_mut("since")
        .unwrap()
        .into = "sort".into()));
    assert!(refused(&|s| {
        let fold = s[index].query_folds.remove("since").unwrap();
        s[index].query_folds.insert("page".into(), fold);
    }));
    assert!(refused(&|s| s[index]
        .query_folds
        .get_mut("since")
        .unwrap()
        .prefix = "updated: >=".into()));
    assert!(refused(&|s| {
        let bound = s[index].bounds["page"].clone();
        s[index].bounds.insert("q".into(), bound);
    }));
    // Embedded text beside a text response, with a pointer missing its `/`,
    // or above the 2 MiB ceiling.
    assert!(refused(&|s| {
        s[readme].response = Some(connectors_catalog_provider::ResponseKind::Text);
    }));
    assert!(refused(&|s| s[readme]
        .embedded_text
        .as_mut()
        .unwrap()
        .content = "content".into()));
    assert!(refused(&|s| s[readme]
        .embedded_text
        .as_mut()
        .unwrap()
        .max_bytes = 2 * 1024 * 1024 + 1));
    // Rate-limit headers naming one header twice.
    assert!(refused(&|s| s[0]
        .rate_limit_headers
        .as_mut()
        .unwrap()
        .reset = "X-RateLimit-Remaining".into()));
    assert!(!refused(&|_| {}));
}

#[test]
fn the_pinned_source_is_the_one_the_bundle_and_its_record_name() {
    let upstream = root().join(UPSTREAM);
    let bytes = fs::read(upstream.join(SOURCE)).unwrap();
    let digest = hex::encode(Sha256::digest(&bytes));
    let index = bundle::read_index(&root().join("generated/bundles")).unwrap();
    let entry = index.find("github").unwrap();
    assert_eq!(entry.source_sha256, digest);
    assert_eq!(entry.operations, OPERATIONS);
    assert_eq!(entry.unsupported, UNSUPPORTED);
    let bundle = committed();
    assert_eq!(bundle.source.source_bytes, bytes.len());
    assert_eq!(bundle.source.file_name, SOURCE);
    let manifest: Value =
        serde_json::from_slice(&fs::read(upstream.join("github-source-hashes.json")).unwrap())
            .unwrap();
    let records = manifest.as_array().unwrap();
    assert_eq!(records.len(), 1, "one pinned GitHub description");
    assert_eq!(records[0]["file"], SOURCE);
    assert_eq!(records[0]["url"], URL);
    assert_eq!(records[0]["sha256"], digest.as_str());
    assert_eq!(records[0]["bytes"], bytes.len());
    assert_eq!(records[0]["redaction"], "upstream-redaction/4");
    assert_eq!(records[0]["upstream_sha256"], UPSTREAM_SHA256);
    assert_ne!(digest, UPSTREAM_SHA256);
    // The MIT licence travels with the description, as published at the tag.
    let licence = fs::read(upstream.join("LICENSE.md")).unwrap();
    assert_eq!(hex::encode(Sha256::digest(&licence)), LICENSE_SHA256);
    let readme = fs::read_to_string(upstream.join("README.md")).unwrap();
    for named in [
        digest.as_str(),
        UPSTREAM_SHA256,
        LICENSE_SHA256,
        "upstream-redaction/4",
        URL,
        "v2.1.0",
    ] {
        assert!(readme.contains(named), "README does not name `{named}`");
    }
}

#[test]
fn guide_cites_each_operation_its_paging_and_its_time_filter() {
    let guide = fs::read_to_string(root().join("../../docs/catalog-github.md")).unwrap();
    let rows: Vec<&str> = guide.lines().filter(|l| l.starts_with('|')).collect();
    for (id, operation_id, path) in SHIPPED {
        let cited = rows.iter().any(|row| {
            row.contains(&format!("`{id}`"))
                && row.contains(&format!("`{operation_id}`"))
                && row.contains(&format!("`GET {path}`"))
        });
        assert!(cited, "no row cites `{id}` as `{operation_id}` `{path}`");
    }
    for needle in [
        "1,000",
        "`notifications`",
        "`updated:>=<since>`",
        "`x-ratelimit-reset`",
        "2022-11-28",
        "https://github.com/beyond10x/connectors/releases",
    ] {
        assert!(
            guide.contains(needle),
            "the guide does not state `{needle}`"
        );
    }
}

/// The `auth` block of the configuration example in the GitHub guide, so the
/// fixture runs the profile the guide tells a reader to write.
fn documented_auth() -> Value {
    let guide = fs::read_to_string(root().join("../../docs/catalog-github.md")).unwrap();
    let example = guide
        .split("```json\n")
        .skip(1)
        .filter_map(|rest| rest.split_once("\n```").map(|(body, _)| body))
        .find(|body| {
            body.contains("\"provider\": \"github\"")
                && body.contains(&format!("\"profile\": \"{PROFILE}\""))
        })
        .expect("the documented GitHub configuration");
    serde_json::from_str::<Value>(example).unwrap()["auth"].clone()
}

#[test]
fn the_documented_profile_is_a_bearer_token_with_the_authenticated_user_as_identity() {
    let auth = documented_auth();
    assert_eq!(auth["header"], "Authorization");
    assert_eq!(auth["bearer"], true);
    assert_eq!(auth["scheme"], Value::Null);
    assert_eq!(auth["identity"]["path"], "user");
    assert_eq!(auth["identity"]["kind"], "github.user");
    assert_eq!(auth["identity"]["subject_pointer"], "/id");
}

/// Each fixture request: its route and its header lines, names lower-cased.
type Requests = Arc<Mutex<Vec<(String, Vec<(String, String)>)>>>;

const README_TEXT: &str = "# Fixture repository\n\nA synthetic README served by the GitHub fixture.\nIt has no meaning beyond this test, and its lines are long enough that the base64 wraps.\n";
const FILE_TEXT: &str = "# Guide\n\nA synthetic guide, read as text.\n";

/// Base64 as GitHub serves `content`: standard alphabet, wrapped at 60
/// characters with `\n`.
fn wrapped(bytes: &[u8]) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let lines: Vec<&str> = encoded
        .as_bytes()
        .chunks(60)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect();
    lines.join("\n") + "\n"
}
fn content(path: &str, bytes: &[u8], size: usize, encoding: &str) -> Value {
    json!({"type": "file", "name": path.rsplit('/').next().unwrap(), "path": path,
           "sha": "0000000000000000000000000000000000000001", "size": size, "encoding": encoding,
           "content": if encoding == "base64" { wrapped(bytes) } else { String::new() },
           "url": format!("https://api.example.com/repos/fixture-org/fixture-repo/contents/{path}")})
}
fn issue(number: u64, pull: bool) -> Value {
    let mut item = json!({"id": 9000 + number, "number": number, "state": "open",
           "title": format!("fixture item {number}"), "updated_at": "2026-10-02T08:00:00Z",
           "html_url": format!("https://github.example.com/fixture-org/fixture-repo/issues/{number}"),
           "user": {"id": 501, "login": "fixture-author"}});
    if pull {
        item["pull_request"] = json!({"url": format!("https://api.example.com/repos/fixture-org/fixture-repo/pulls/{number}")});
    }
    item
}
fn notification(id: u64) -> Value {
    json!({"id": id.to_string(), "unread": true, "reason": "review_requested",
           "updated_at": "2026-10-02T08:00:00Z",
           "subject": {"title": format!("fixture notification {id}"), "type": "PullRequest"},
           "repository": {"id": 700, "full_name": "fixture-org/fixture-repo"}})
}
fn repository(id: u64) -> Value {
    json!({"id": id, "name": format!("fixture-repo-{id}"), "full_name": format!("fixture-org/fixture-repo-{id}"),
           "description": "A synthetic repository", "topics": ["fixture"], "homepage": "https://example.com",
           "stargazers_count": 3, "pushed_at": "2026-10-02T08:00:00Z", "archived": false, "visibility": "public"})
}
fn release(id: u64) -> Value {
    json!({"id": id, "tag_name": format!("v0.{id}.0"), "name": format!("Release 0.{id}.0"),
           "published_at": "2026-10-02T08:00:00Z", "body": "Synthetic notes.", "draft": false, "prerelease": false})
}
fn search(items: Vec<Value>, total: u64) -> Value {
    json!({"total_count": total, "incomplete_results": false, "items": items})
}

/// Whether a request's query carries a pair, compared on the encoded text.
fn has(query: &str, pair: &str) -> bool {
    query.split('&').any(|p| p == pair)
}

/// A recorded answer: status, extra headers and body.
type Answer = (u16, Vec<(&'static str, String)>, Value);

/// The recorded answer the fixture serves: status, extra headers and body.
/// `None` for anything else, which it answers 404.
fn answer(path: &str) -> Option<Answer> {
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let ok = |body: Value| Some((200, vec![], body));
    match route {
        "/user" => ok(json!({"id": 4242, "login": "fixture-reader", "type": "User"})),
        "/search/issues" => {
            let page: u64 = query
                .split('&')
                .find_map(|p| p.strip_prefix("page="))
                .and_then(|p| p.parse().ok())
                .unwrap_or(1);
            if has(query, "per_page=100") {
                // The cap walk: 1,500 matches, of which the search serves 1,000.
                if page > 10 {
                    return Some((
                        422,
                        vec![],
                        json!({"message": "Only the first 1000 search results are available",
                        "documentation_url": "https://docs.github.com/v3/search/"}),
                    ));
                }
                let items = (0..100)
                    .map(|i| issue((page - 1) * 100 + i + 1, true))
                    .collect();
                return ok(search(items, 1500));
            }
            match page {
                1 => ok(search(vec![issue(11, true), issue(12, true)], 3)),
                2 => ok(search(vec![issue(13, true)], 3)),
                _ => None,
            }
        }
        "/issues" if has(query, "page=2") => ok(json!([issue(23, false)])),
        "/issues" => ok(json!([issue(21, false), issue(22, true)])),
        "/notifications" if has(query, "page=2") => ok(json!([notification(33)])),
        "/notifications" => ok(json!([notification(31), notification(32)])),
        "/orgs/fixture-org/repos" if has(query, "page=2") => ok(json!([repository(43)])),
        "/orgs/fixture-org/repos" => ok(json!([repository(41), repository(42)])),
        "/orgs/fixture-limited/repos" => Some((
            403,
            vec![
                ("x-ratelimit-limit", "5000".into()),
                ("x-ratelimit-remaining", "0".into()),
                ("x-ratelimit-reset", (now() + 600).to_string()),
            ],
            json!({"message": "API rate limit exceeded for user ID 4242.",
                   "documentation_url": "https://docs.github.com/rest/overview/resources-in-the-rest-api#rate-limiting"}),
        )),
        "/orgs/fixture-limited-429/repos" => Some((
            429,
            vec![
                ("x-ratelimit-remaining", "0".into()),
                ("x-ratelimit-reset", (now() + 900).to_string()),
            ],
            json!({"message": "API rate limit exceeded"}),
        )),
        "/orgs/fixture-limited-retry/repos" => Some((
            403,
            vec![
                ("retry-after", "120".into()),
                ("x-ratelimit-remaining", "0".into()),
                ("x-ratelimit-reset", (now() + 900).to_string()),
            ],
            json!({"message": "You have exceeded a secondary rate limit."}),
        )),
        "/orgs/fixture-forbidden/repos" => Some((
            403,
            vec![("x-ratelimit-remaining", "4999".into())],
            json!({"message": "Resource not accessible by personal access token"}),
        )),
        "/repos/fixture-org/fixture-repo" => ok(repository(40)),
        "/repos/fixture-org/fixture-repo/releases" if has(query, "page=2") => {
            ok(json!([release(3)]))
        }
        "/repos/fixture-org/fixture-repo/releases" => ok(json!([release(5), release(4)])),
        "/repos/fixture-org/fixture-repo/releases/latest" => ok(release(5)),
        "/repos/fixture-org/fixture-repo/readme" => ok(content(
            "README.md",
            README_TEXT.as_bytes(),
            README_TEXT.len(),
            "base64",
        )),
        "/repos/fixture-org/fixture-repo/contents/docs%2Fguide.md" => ok(content(
            "docs/guide.md",
            FILE_TEXT.as_bytes(),
            FILE_TEXT.len(),
            "base64",
        )),
        "/repos/fixture-org/fixture-repo/contents/logo.png" => ok(content(
            "logo.png",
            b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR",
            16,
            "base64",
        )),
        "/repos/fixture-org/fixture-repo/contents/big.txt" => {
            ok(content("big.txt", b"", 2 * 1024 * 1024, "none"))
        }
        "/repos/fixture-org/fixture-repo/contents/docs" => ok(json!([content(
            "docs/guide.md",
            FILE_TEXT.as_bytes(),
            FILE_TEXT.len(),
            "base64"
        )])),
        _ => None,
    }
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

struct Provider {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    _root: tempfile::TempDir,
    config: PathBuf,
    requests: Requests,
}
impl Provider {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("private");
        filesystem::directory(&directory, true, true).unwrap();
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let ca = directory.join("ca.pem");
        private(&ca, cert.cert.pem().as_bytes());
        let tls = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert.cert.der().clone()],
            PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()).into(),
        )
        .unwrap();
        let (address_tx, address_rx) = std::sync::mpsc::channel();
        let (stop, mut stopped) = oneshot::channel();
        let requests: Requests = Arc::new(Mutex::new(Vec::new()));
        let observed = requests.clone();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                address_tx.send(listener.local_addr().unwrap()).unwrap();
                let acceptor = TlsAcceptor::from(Arc::new(tls));
                loop {
                    let stream = tokio::select! {_=&mut stopped=>break,value=listener.accept()=>value.unwrap().0};
                    let Ok(mut stream) = acceptor.accept(stream).await else {
                        continue;
                    };
                    let mut header = Vec::new();
                    while !header.ends_with(b"\r\n\r\n") {
                        assert!(header.len() < 8192);
                        match stream.read_u8().await {
                            Ok(byte) => header.push(byte),
                            Err(_) => break,
                        }
                    }
                    let request = String::from_utf8(header).unwrap();
                    let path = request
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or_default()
                        .to_owned();
                    let headers: Vec<(String, String)> = request
                        .lines()
                        .skip(1)
                        .filter_map(|line| line.split_once(':'))
                        .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
                        .collect();
                    let authorization = headers
                        .iter()
                        .find(|(name, _)| name == "authorization")
                        .map(|(_, value)| value.as_str());
                    // Only the fictional header is accepted; raw headers are
                    // compared, never printed.
                    let (status, extra, body) = if authorization != Some(HEADER) {
                        (401, vec![], json!({"message": "Bad credentials"}))
                    } else {
                        answer(&path).unwrap_or((404, vec![], json!({"message": "Not Found"})))
                    };
                    observed.lock().unwrap().push((path, headers));
                    let body = serde_json::to_vec(&body).unwrap();
                    let mut head = format!(
                        "HTTP/1.1 {status} fixture\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n",
                        body.len()
                    );
                    for (name, value) in extra {
                        head.push_str(&format!("{name}: {value}\r\n"));
                    }
                    head.push_str("\r\n");
                    let _ = stream.write_all(head.as_bytes()).await;
                    let _ = stream.write_all(&body).await;
                }
            });
        });
        let address = address_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let config = directory.join("catalog.json");
        private(
            &config,
            &serde_json::to_vec(&json!({
                "format": "connectors-catalog-local/2",
                "instance": "fixture-github",
                "provider": "github",
                "bundle_directory": root_path("generated/bundles"),
                "api_base": format!("https://localhost:{}", address.port()),
                "ca_file": ca,
                "auth": documented_auth(),
                "operations_file": root_path("providers/github/operations.json"),
            }))
            .unwrap(),
        );
        Self {
            stop: Some(stop),
            thread: Some(thread),
            _root: root,
            config,
            requests,
        }
    }
    fn selection(&self) -> Adapter {
        let output = Command::new(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .arg("--local-config")
            .arg(&self.config)
            .arg("--print-local-bootstrap")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "bootstrap inspection failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bootstrap: Bootstrap = serde_json::from_slice(&output.stdout).unwrap();
        bootstrap.validate().unwrap();
        let binary = PathBuf::from(env!("CARGO_BIN_EXE_connectors-catalog-provider"))
            .canonicalize()
            .unwrap();
        Adapter {
            private_protocol: None,
            permissions: Default::default(),
            instance_id: "fixture-github".into(),
            adapter_id: "catalog".into(),
            configuration_revision: bootstrap.configuration_revision,
            protocol: "v1alpha1".into(),
            startup: Startup::OnDemand,
            restart: Restart::Never,
            executable: Executable {
                sha256: hex::encode(Sha256::digest(fs::read(&binary).unwrap())),
                path: binary,
                args: vec![
                    "--local-config".into(),
                    self.config.to_str().unwrap().into(),
                ],
            },
        }
    }
    fn requests(&self) -> Vec<(String, Vec<(String, String)>)> {
        self.requests.lock().unwrap().clone()
    }
    fn paths(&self) -> Vec<String> {
        self.requests().into_iter().map(|(path, _)| path).collect()
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap();
    }
}
fn root_path(relative: &str) -> PathBuf {
    root().join(relative).canonicalize().unwrap()
}
fn private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
fn secret() -> Secret {
    Secret(serde_json::to_vec(&json!({"token": TOKEN})).unwrap())
}
fn deadline() -> u64 {
    connectors_sdk::now_ms() + 30_000
}
fn explained(child: &mut Child, operation: &str, input: &Value) -> Result<Vec<u8>, Refusal> {
    let revision = child.bootstrap().descriptor().unwrap().revision;
    child.invoke_explained(
        operation,
        &revision,
        "one",
        &secret(),
        &serde_json::to_vec(input).unwrap(),
        deadline(),
    )
}
fn invoke(child: &mut Child, operation: &str, input: &Value) -> Value {
    let raw = explained(child, operation, input)
        .unwrap_or_else(|refusal| panic!("`{operation}` failed: {:?}", refusal.failure));
    serde_json::from_slice(&raw).unwrap()
}

#[test]
fn connecting_records_the_authenticated_user_as_the_identity() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let baseline = child.validate(PROFILE, &secret(), deadline()).unwrap();
    assert_eq!(baseline.identity.kind, "github.user");
    assert_eq!(baseline.identity.subject, "4242");
    assert_eq!(provider.paths(), ["/user?"]);
    let wrong = Secret(br#"{"token":"wrong-token"}"#.to_vec());
    assert!(matches!(
        child.validate(PROFILE, &wrong, deadline()),
        Err(Failure::InvalidCredential)
    ));
}

fn owner() -> Value {
    json!({"owner": "fixture-org", "repo": "fixture-repo"})
}
fn with(base: Value, extra: Value) -> Value {
    let mut base = base;
    for (key, value) in extra.as_object().unwrap() {
        base[key] = value.clone();
    }
    base
}

/// Each read's first request: the input, the exact request the fixture must
/// observe (the time filter included), and what `body` must be.
fn first_pages() -> Vec<(&'static str, Value, String, Value)> {
    let served = |path: &str| answer(path).unwrap().2;
    let search = format!(
        "/search/issues?per_page=2&page=1&q=is%3Apr+is%3Aopen+review-requested%3A%40me+updated%3A%3E%3D{DATE}"
    );
    let issues =
        "/issues?since=2026-10-01T00%3A00%3A00Z&per_page=2&page=1&filter=assigned&state=open"
            .to_owned();
    let notifications =
        "/notifications?since=2026-10-01T00%3A00%3A00Z&page=1&per_page=2".to_owned();
    let repos = "/orgs/fixture-org/repos?type=public&sort=pushed&direction=desc&per_page=2&page=1"
        .to_owned();
    let releases = "/repos/fixture-org/fixture-repo/releases?per_page=2&page=1".to_owned();
    vec![
        (
            "review_requests.list",
            json!({"since": DATE, "per_page": 2, "page": 1}),
            search.clone(),
            served(&search),
        ),
        (
            "issues.assigned",
            json!({"since": SINCE, "per_page": 2, "page": 1}),
            issues.clone(),
            served(&issues),
        ),
        (
            "notifications.list",
            json!({"since": SINCE, "page": 1, "per_page": 2}),
            notifications.clone(),
            served(&notifications),
        ),
        ("user.me", json!({}), "/user?".into(), served("/user")),
        (
            "repos.list",
            json!({"org": "fixture-org", "type": "public", "sort": "pushed", "direction": "desc", "per_page": 2, "page": 1}),
            repos.clone(),
            served(&repos),
        ),
        (
            "repo.get",
            owner(),
            "/repos/fixture-org/fixture-repo?".into(),
            served("/repos/fixture-org/fixture-repo"),
        ),
        (
            "releases.list",
            with(owner(), json!({"per_page": 2, "page": 1})),
            releases.clone(),
            served(&releases),
        ),
        (
            "release.latest",
            owner(),
            "/repos/fixture-org/fixture-repo/releases/latest?".into(),
            served("/repos/fixture-org/fixture-repo/releases/latest"),
        ),
        (
            "readme.raw",
            owner(),
            "/repos/fixture-org/fixture-repo/readme?".into(),
            json!(README_TEXT),
        ),
        (
            "file.raw",
            with(owner(), json!({"path": "docs/guide.md", "ref": "main"})),
            "/repos/fixture-org/fixture-repo/contents/docs%2Fguide.md?ref=main".into(),
            json!(FILE_TEXT),
        ),
    ]
}

#[test]
fn each_read_sends_the_declared_request_and_returns_the_recorded_body() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, expected, body) in first_pages() {
        let before = provider.requests().len();
        let result = invoke(&mut child, operation, &input);
        let requests = provider.requests();
        assert_eq!(requests.len(), before + 1, "`{operation}` requests");
        let (path, headers) = &requests[before];
        assert_eq!(path, &expected, "`{operation}` request");
        let value = |name: &str| {
            headers
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.as_str())
        };
        assert_eq!(
            value("authorization"),
            Some(HEADER),
            "`{operation}` authorization"
        );
        assert_eq!(value("accept"), Some("*/*"), "`{operation}` accept");
        assert_eq!(
            value("x-github-api-version"),
            None,
            "`{operation}` sends no API version"
        );
        assert_eq!(result["status"], 200, "`{operation}` status");
        assert_eq!(result["provenance"]["instance"], "fixture-github");
        assert_eq!(
            serde_json::to_vec(&result["body"]).unwrap(),
            serde_json::to_vec(&body).unwrap(),
            "`{operation}` body"
        );
    }
}

/// The documented end of every list: a page shorter than `per_page`.
#[test]
fn each_list_walks_two_pages_and_stops_at_a_short_page() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input, items) in [
        (
            "review_requests.list",
            json!({"since": DATE, "per_page": 2}),
            "/items",
        ),
        (
            "issues.assigned",
            json!({"since": SINCE, "per_page": 2}),
            "",
        ),
        (
            "notifications.list",
            json!({"since": SINCE, "per_page": 2}),
            "",
        ),
        (
            "repos.list",
            json!({"org": "fixture-org", "per_page": 2}),
            "",
        ),
        ("releases.list", with(owner(), json!({"per_page": 2})), ""),
    ] {
        let mut page = 1;
        let mut seen = 0;
        loop {
            let result = invoke(
                &mut child,
                operation,
                &with(input.clone(), json!({"page": page})),
            );
            let count = result["body"]
                .pointer(items)
                .unwrap()
                .as_array()
                .unwrap()
                .len();
            seen += count;
            if count < 2 {
                break;
            }
            page += 1;
        }
        assert_eq!((page, seen), (2, 3), "`{operation}` walk");
    }
}

/// The search serves at most 1,000 results: a walk at 100 per page reads ten
/// full pages, and the eleventh is refused by the provider, so the documented
/// walk stops at the 1,000th result whatever `total_count` says.
#[test]
fn the_search_walk_stops_at_its_thousand_result_cap() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    let mut page = 1;
    let mut seen = 0;
    loop {
        let result = invoke(
            &mut child,
            "review_requests.list",
            &json!({"per_page": 100, "page": page}),
        );
        assert_eq!(result["body"]["total_count"], 1500);
        let count = result["body"]["items"].as_array().unwrap().len();
        seen += count;
        if count < 100 || page * 100 >= 1000 {
            break;
        }
        page += 1;
    }
    assert_eq!((page, seen), (10, 1000));
    assert_eq!(provider.requests().len(), 10);
    let beyond = explained(
        &mut child,
        "review_requests.list",
        &json!({"per_page": 100, "page": 11}),
    )
    .expect_err("the eleventh page");
    assert_eq!(beyond.failure, Failure::InvalidInput);
}

#[test]
fn a_spent_rate_limit_is_refused_with_its_reset_time_and_sent_once() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (org, low, high) in [
        ("fixture-limited", 590, 600),
        ("fixture-limited-429", 890, 900),
        ("fixture-limited-retry", 120, 120),
    ] {
        let before = provider.requests().len();
        let refusal = explained(&mut child, "repos.list", &json!({"org": org})).expect_err(org);
        assert_eq!(provider.requests().len(), before + 1, "`{org}` sent once");
        assert_eq!(refusal.failure, Failure::ProviderRateLimited, "`{org}`");
        let wait = refusal
            .retry_after_seconds
            .unwrap_or_else(|| panic!("`{org}` names no wait"));
        assert!((low..=high).contains(&wait), "`{org}` waits {wait}");
    }
    // A 403 with requests left is a permission refusal.
    let refusal = explained(
        &mut child,
        "repos.list",
        &json!({"org": "fixture-forbidden"}),
    )
    .expect_err("forbidden");
    assert_eq!(refusal.failure, Failure::ProviderForbidden);
}

#[test]
fn a_file_that_is_not_text_or_too_large_or_a_directory_is_refused() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (path, failure) in [
        ("logo.png", Failure::Unsupported),
        ("big.txt", Failure::Capacity),
        ("docs", Failure::Protocol),
    ] {
        let refusal = explained(
            &mut child,
            "file.raw",
            &with(owner(), json!({"path": path})),
        )
        .expect_err(path);
        assert_eq!(refusal.failure, failure, "`{path}`");
    }
}

#[test]
fn a_caller_cannot_send_a_fixed_value_or_a_since_that_is_no_timestamp() {
    let provider = Provider::new();
    let mut child = Child::spawn(&provider.selection()).unwrap();
    for (operation, input) in [
        ("review_requests.list", json!({"q": "is:issue"})),
        (
            "review_requests.list",
            json!({"since": "2026-10-01 repo:fixture-org/x"}),
        ),
        ("review_requests.list", json!({"since": "yesterday"})),
        ("review_requests.list", json!({"sort": "updated"})),
        ("review_requests.list", json!({"per_page": 101})),
        ("issues.assigned", json!({"filter": "all"})),
        ("issues.assigned", json!({"state": "closed"})),
        ("notifications.list", json!({"per_page": 51})),
    ] {
        let refusal = explained(&mut child, operation, &input).expect_err(operation);
        assert_eq!(
            refusal.failure,
            Failure::InvalidInput,
            "`{operation}` {input}"
        );
    }
    assert!(provider.requests().is_empty());
}

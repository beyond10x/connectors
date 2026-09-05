//! Real Git client → production HTTP router/broker → Git's HTTP backend interoperability.
//! Loopback HTTP and synthetic custody are test adapters; production TLS policy is unchanged.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::Response;
use service::{
    EgressByteStream, EgressHttpRequest, EgressHttpResponse, EgressStreamingHttpRequest,
    EgressStreamingHttpResponse, EgressTransport, EgressTransportError, EgressWebSocket,
    GitFetchBroker,
};

use super::{lock, tests, GitFetchSessionState};

struct TlsFixture {
    listener: tokio::net::TcpListener,
    acceptor: tokio_rustls::TlsAcceptor,
}

impl axum::serve::Listener for TlsFixture {
    type Io = tokio_rustls::server::TlsStream<tokio::net::TcpStream>;
    type Addr = SocketAddr;
    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (stream, address) = self.listener.accept().await.unwrap();
            if let Ok(stream) = self.acceptor.accept(stream).await {
                return (stream, address);
            }
        }
    }
    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.listener.local_addr()
    }
}

fn isolated_git() -> Command {
    let mut command = Command::new("git");
    command
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
        .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z");
    command
}

fn git(path: &Path, args: &[&str], input: Option<&[u8]>) -> Vec<u8> {
    let mut command = isolated_git();
    command
        .current_dir(path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if input.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command
        .spawn()
        .expect("Git is required for HTTP interoperability tests");
    if let Some(input) = input {
        child.stdin.take().unwrap().write_all(input).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "Git fixture command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn git_text(path: &Path, args: &[&str], input: Option<&[u8]>) -> String {
    String::from_utf8(git(path, args, input))
        .unwrap()
        .trim()
        .to_owned()
}

struct Repository {
    root: tempfile::TempDir,
    commit: String,
}

impl Repository {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repository = root.path().join("group/repository.git");
        std::fs::create_dir_all(&repository).unwrap();
        git(
            &repository,
            &["init", "--bare", "--initial-branch=trunk"],
            None,
        );
        let blob = git_text(
            &repository,
            &["hash-object", "-w", "--stdin"],
            Some(b"workspace fixture\n"),
        );
        let tree = git_text(
            &repository,
            &["mktree"],
            Some(format!("100644 blob {blob}\tfile.txt\n").as_bytes()),
        );
        let mut commit = String::new();
        for index in 0..60 {
            let mut args = vec!["commit-tree", &tree];
            if !commit.is_empty() {
                args.extend(["-p", &commit]);
            }
            commit = git_text(
                &repository,
                &args,
                Some(format!("fixture {index}\n").as_bytes()),
            );
        }
        let mut refs = format!("update refs/heads/trunk {commit}\n");
        for index in 0..4_000 {
            refs.push_str(&format!("update refs/heads/private-{index:05} {commit}\n"));
        }
        // A prefix-matching branch must be discarded even when the upstream honors ref-prefix.
        refs.push_str(&format!("update refs/heads/trunk-private {commit}\n"));
        refs.push_str(&format!("update refs/tags/private-tag {commit}\n"));
        git(
            &repository,
            &["update-ref", "--stdin"],
            Some(refs.as_bytes()),
        );
        Self { root, commit }
    }
}

#[derive(Clone, Debug)]
struct Observation {
    protocol: Option<String>,
    request: Vec<u8>,
    response_bytes: usize,
    discovery: bool,
}

#[derive(Clone)]
struct Provider {
    root: PathBuf,
    observations: Arc<Mutex<Vec<Observation>>>,
}

async fn provider(State(state): State<Provider>, request: Request) -> Response<Body> {
    assert_eq!(
        request.headers()["authorization"],
        "Bearer synthetic-gitlab-token"
    );
    assert!(!request
        .headers()
        .contains_key("x-b10x-git-source-authorization"));
    let protocol = request
        .headers()
        .get("git-protocol")
        .map(|value| value.to_str().unwrap().to_owned());
    let method = request.method().to_string();
    let path = request.uri().path().to_owned();
    let query = request.uri().query().unwrap_or_default().to_owned();
    let content_type = request
        .headers()
        .get("content-type")
        .map(|value| value.to_str().unwrap().to_owned());
    let body = to_bytes(request.into_body(), 256 * 1024)
        .await
        .unwrap()
        .to_vec();
    let discovery = method == "GET";
    let observed_protocol = protocol.clone();
    let observed_body = body.clone();
    let output = tokio::task::spawn_blocking(move || {
        let mut command = isolated_git();
        command
            .arg("http-backend")
            .env("GIT_PROJECT_ROOT", state.root)
            .env("GIT_HTTP_EXPORT_ALL", "1")
            .env("REQUEST_METHOD", method)
            .env("PATH_INFO", path)
            .env("QUERY_STRING", query)
            .env("CONTENT_LENGTH", body.len().to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(protocol) = protocol {
            command.env("HTTP_GIT_PROTOCOL", protocol);
        }
        if let Some(content_type) = content_type {
            command.env("CONTENT_TYPE", content_type);
        }
        let mut child = command.spawn().unwrap();
        child.stdin.take().unwrap().write_all(&body).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    })
    .await
    .unwrap();
    let header_end = output
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .unwrap();
    let mut response = Response::builder();
    for line in std::str::from_utf8(&output[..header_end])
        .unwrap()
        .split("\r\n")
    {
        let (name, value) = line.split_once(": ").unwrap();
        if name.eq_ignore_ascii_case("status") {
            response = response.status(value[..3].parse::<u16>().unwrap());
        } else {
            response = response.header(name, value);
        }
    }
    let body = output[header_end + 4..].to_vec();
    state.observations.lock().unwrap().push(Observation {
        protocol: observed_protocol,
        request: observed_body,
        response_bytes: body.len(),
        discovery,
    });
    response.body(Body::from(body)).unwrap()
}

struct LoopbackEgress {
    origin: String,
    commit: String,
    client: reqwest::Client,
}

struct HttpStream(reqwest::Response);

#[async_trait]
impl EgressByteStream for HttpStream {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
        self.0
            .chunk()
            .await
            .map(|chunk| chunk.map(|bytes| bytes.to_vec()))
            .map_err(|_| EgressTransportError::Refused)
    }
}

#[async_trait]
impl EgressTransport for LoopbackEgress {
    async fn execute(
        &self,
        _authority: &str,
        request: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError> {
        let path = url::Url::parse(&request.request.url).unwrap();
        let body = match path.path() {
            "/api/v4/projects/42" => {
                serde_json::json!({"id":42,"path_with_namespace":"group/repository","default_branch":"trunk"})
            }
            "/api/v4/projects/42/repository/branches/trunk" => {
                serde_json::json!({"name":"trunk","commit":{"id":self.commit}})
            }
            _ => return Err(EgressTransportError::Refused),
        };
        Ok(EgressHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }

    async fn execute_stream(
        &self,
        _authority: &str,
        request: EgressStreamingHttpRequest,
    ) -> Result<EgressStreamingHttpResponse, EgressTransportError> {
        let url = url::Url::parse(&request.url).unwrap();
        assert_eq!(url.host_str(), Some("gitlab.example.test"));
        let mut outbound = self.client.request(
            request.method.parse().unwrap(),
            format!(
                "{}{}{}",
                self.origin,
                url.path(),
                url.query().map(|q| format!("?{q}")).unwrap_or_default()
            ),
        );
        for (name, value) in request.headers {
            outbound = outbound.header(name, value);
        }
        if let Some(body) = request.body {
            outbound = outbound.body(body);
        }
        let response = outbound.send().await.unwrap();
        let headers = BTreeMap::from([(
            "content-type".to_owned(),
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .to_owned(),
        )]);
        Ok(EgressStreamingHttpResponse {
            status: response.status().as_u16(),
            headers,
            body: Box::new(HttpStream(response)),
        })
    }

    async fn connect_websocket(
        &self,
        _authority: &str,
        _url: String,
        _maximum: usize,
    ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
        Err(EgressTransportError::Refused)
    }
}

#[tokio::test]
async fn real_v2_clone_preserves_depth_and_reduces_many_ref_discovery_bytes() {
    let repository = Repository::new();
    let observations = Arc::new(Mutex::new(Vec::new()));
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_origin = format!("http://{}", upstream.local_addr().unwrap());
    let upstream_app = axum::Router::new().fallback(provider).with_state(Provider {
        root: repository.root.path().to_owned(),
        observations: observations.clone(),
    });
    let upstream_task = tokio::spawn(async move {
        axum::serve(upstream, upstream_app).await.unwrap();
    });
    let egress = Arc::new(LoopbackEgress {
        origin: upstream_origin,
        commit: repository.commit.clone(),
        client: reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap(),
    });
    let backend = Arc::new(tests::backend_with_egress(egress).await);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_origin = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    let tls = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let ca_file = repository.root.path().join("fixture-ca.pem");
    std::fs::write(&ca_file, tls.cert.pem()).unwrap();
    let provider = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider();
    let _ = provider.clone().install_default();
    let config = tokio_rustls::rustls::ServerConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![tls.cert.der().clone()],
            tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer::from(tls.key_pair.serialize_der())
                .into(),
        )
        .unwrap();
    let listener = TlsFixture {
        listener,
        acceptor: tokio_rustls::TlsAcceptor::from(Arc::new(config)),
    };
    let proxy_app = server::hosted::git_fetch_internal_router(backend.clone());
    let proxy_task = tokio::spawn(async move {
        axum::serve(listener, proxy_app).await.unwrap();
    });

    // Legacy discovery establishes the comparison without depending on a particular legacy
    // client's multi-round shallow negotiation. Existing legacy upload tests retain compatibility.
    let mut request = tests::request();
    request.expected_commit = repository.commit.clone();
    request.depth = 50;
    let legacy = backend
        .create(&tests::context(), request.clone())
        .await
        .unwrap();
    let source = legacy.expose_at_control_boundary().to_owned();
    let url = format!(
        "{proxy_origin}/internal/git-fetch/{}/repository.git/info/refs?service=git-upload-pack",
        legacy.session_ref
    );
    let legacy_response = reqwest::Client::builder()
        .no_proxy()
        .add_root_certificate(reqwest::Certificate::from_pem(tls.cert.pem().as_bytes()).unwrap())
        .build()
        .unwrap()
        .get(url)
        .header("x-b10x-git-source-authorization", source)
        .send()
        .await
        .unwrap();
    assert!(legacy_response.status().is_success());
    let legacy_body = legacy_response.bytes().await.unwrap();
    assert!(!String::from_utf8_lossy(&legacy_body).contains("private"));

    request.idempotency_key = "v2-http-clone".to_owned();
    let grant = backend.create(&tests::context(), request).await.unwrap();
    let source = grant.expose_at_control_boundary().to_owned();
    let url = format!(
        "{proxy_origin}/internal/git-fetch/{}/repository.git",
        grant.session_ref
    );
    let checkout = repository.root.path().join("checkout");
    let expected_commit = repository.commit.clone();
    let client_ca = ca_file.clone();
    let started = Instant::now();
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&checkout).unwrap();
        git(&checkout, &["init", "--initial-branch=trunk"], None);
        git(
            &checkout,
            &[
                "-c",
                "protocol.version=2",
                "-c",
                &format!("http.sslCAInfo={}", client_ca.display()),
                "-c",
                &format!("http.extraHeader=x-b10x-git-source-authorization: {source}"),
                "fetch",
                "--depth=50",
                "--no-tags",
                &url,
                "refs/heads/trunk:refs/remotes/origin/trunk",
            ],
            None,
        );
        git(
            &checkout,
            &["checkout", "--detach", "refs/remotes/origin/trunk"],
            None,
        );
        assert_eq!(
            git_text(&checkout, &["rev-parse", "HEAD"], None),
            expected_commit
        );
        assert_eq!(
            git_text(&checkout, &["rev-list", "--count", "HEAD"], None),
            "50"
        );
        assert_eq!(
            git_text(&checkout, &["rev-parse", "--is-shallow-repository"], None),
            "true"
        );
        assert_eq!(
            std::fs::read_to_string(checkout.join("file.txt")).unwrap(),
            "workspace fixture\n"
        );
        assert!(git_text(&checkout, &["tag", "--list"], None).is_empty());
        assert!(!git_text(&checkout, &["show-ref"], None).contains("private"));
    })
    .await
    .unwrap();
    assert_eq!(
        lock(&backend.inner.git_fetch_sessions)
            .get(&grant.session_ref)
            .unwrap()
            .state,
        GitFetchSessionState::Spent
    );
    if let Some(executable) = std::env::var_os("CONNECTORS_TEST_SUBSTRATE_HOST_BINARY") {
        let mut request = tests::request();
        request.idempotency_key = "gix-through-connectors".to_owned();
        request.expected_commit = repository.commit.clone();
        request.depth = 50;
        let grant = backend.create(&tests::context(), request).await.unwrap();
        let locator = format!(
            "{proxy_origin}/internal/git-fetch/{}/repository.git",
            grant.session_ref
        );
        let commit = repository.commit.clone();
        let authority = grant.expose_at_control_boundary().to_owned();
        tokio::task::spawn_blocking(move || {
            let output = Command::new(executable)
                .args([
                    "git::materialization_tests::external_connectors_proxy_v2_fixture",
                    "--exact",
                    "--ignored",
                    "--nocapture",
                ])
                .env("SUBSTRATE_TEST_GIT_LOCATOR", locator)
                .env("SUBSTRATE_TEST_GIT_CA", ca_file)
                .env("SUBSTRATE_TEST_GIT_REF", "trunk")
                .env("SUBSTRATE_TEST_GIT_COMMIT", commit)
                .env("SUBSTRATE_TEST_GIT_AUTHORITY", authority)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "Substrate proxy fixture failed: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            eprintln!(
                "Substrate gix through Connectors: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        })
        .await
        .unwrap();
        assert_eq!(
            lock(&backend.inner.git_fetch_sessions)
                .get(&grant.session_ref)
                .unwrap()
                .state,
            GitFetchSessionState::Spent
        );
    }
    let observations = observations.lock().unwrap();
    let legacy_bytes: usize = observations
        .iter()
        .filter(|r| r.protocol.is_none())
        .map(|r| r.response_bytes)
        .sum();
    let v2 = observations
        .iter()
        .filter(|r| r.protocol.as_deref() == Some("version=2"))
        .collect::<Vec<_>>();
    assert_eq!(
        v2.len(),
        if std::env::var_os("CONNECTORS_TEST_SUBSTRATE_HOST_BINARY").is_some() {
            6
        } else {
            3
        },
        "{observations:?}"
    );
    assert!(v2[0].discovery);
    let refs = String::from_utf8_lossy(&v2[1].request);
    assert!(refs.contains("command=ls-refs"));
    assert!(refs.contains("ref-prefix HEAD\n"));
    assert!(refs.contains("ref-prefix refs/heads/trunk\n"));
    assert!(!refs.contains("ref-prefix refs/heads/\n"));
    let fetch = String::from_utf8_lossy(&v2[2].request);
    assert!(fetch.contains("command=fetch"));
    assert!(fetch.contains("deepen 50\n"));
    assert!(fetch.contains(&format!("want {}\n", repository.commit)));
    let v2_discovery_bytes = v2[0].response_bytes + v2[1].response_bytes;
    assert!(
        legacy_bytes > 100 * v2_discovery_bytes,
        "legacy={legacy_bytes}, v2={v2_discovery_bytes}"
    );
    eprintln!("many-ref fixture: legacy discovery={legacy_bytes} bytes, v2 capabilities+ls-refs={v2_discovery_bytes} bytes, v2 clone elapsed={:?}, three provider exchanges", started.elapsed());
    proxy_task.abort();
    upstream_task.abort();
}

use super::*;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use connector_secrets::{MemoryStore, SecretStore as _};
use service::{
    EgressHttpRequest, EgressHttpResponse, EgressStreamingHttpResponse, EgressTransport,
    EgressWebSocket,
};

struct TestStream {
    chunk: Option<Vec<u8>>,
}

#[async_trait]
impl EgressByteStream for TestStream {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
        Ok(self.chunk.take())
    }
}

struct PendingStream;

#[async_trait]
impl EgressByteStream for PendingStream {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
        std::future::pending().await
    }
}

struct GitEgress {
    project_calls: AtomicUsize,
    stream_calls: AtomicUsize,
    seen_headers: Mutex<Vec<BTreeMap<String, String>>>,
    seen_urls: Mutex<Vec<String>>,
}

#[async_trait]
impl EgressTransport for GitEgress {
    async fn execute(
        &self,
        _authority_ref: &str,
        request: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError> {
        let url = url::Url::parse(&request.request.url).unwrap();
        let body = if url.path() == "/api/v4/projects/42" {
            let project_call = self.project_calls.fetch_add(1, Ordering::SeqCst);
            serde_json::json!({
                "id": 42,
                "path_with_namespace": if project_call == 0 {
                    "group/repository"
                } else {
                    "group/renamed-repository"
                },
                "default_branch": "trunk"
            })
        } else if url.path() == "/api/v4/projects/42/repository/branches/trunk" {
            serde_json::json!({
                "name": "trunk",
                "commit": {"id": "a".repeat(40)}
            })
        } else {
            return Ok(EgressHttpResponse {
                status: 404,
                headers: BTreeMap::new(),
                body: Vec::new(),
            });
        };
        Ok(EgressHttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            body: serde_json::to_vec(&body).unwrap(),
        })
    }

    async fn execute_stream(
        &self,
        _authority_ref: &str,
        request: EgressStreamingHttpRequest,
    ) -> Result<EgressStreamingHttpResponse, EgressTransportError> {
        self.stream_calls.fetch_add(1, Ordering::SeqCst);
        self.seen_headers
            .lock()
            .unwrap()
            .push(request.headers.clone());
        self.seen_urls.lock().unwrap().push(request.url.clone());
        let target = url::Url::parse(&request.url).unwrap();
        let content_type = if target.path().ends_with("/info/refs")
            && target
                .query_pairs()
                .any(|(key, value)| key == "service" && value == "git-upload-pack")
        {
            "application/x-git-upload-pack-advertisement"
        } else if target.path().ends_with("/git-upload-pack") {
            "application/x-git-upload-pack-result"
        } else {
            return Err(EgressTransportError::Refused);
        };
        Ok(EgressStreamingHttpResponse {
            status: 200,
            headers: BTreeMap::from([("content-type".to_owned(), content_type.to_owned())]),
            body: Box::new(TestStream {
                chunk: Some(
                    if request.headers.get("git-protocol").map(String::as_str) == Some("version=2")
                    {
                        if content_type.ends_with("advertisement") {
                            [
                                packet(b"version 2\n"),
                                packet(b"ls-refs\n"),
                                packet(b"fetch=shallow\n"),
                                b"0000".to_vec(),
                            ]
                            .concat()
                        } else if request.body.as_ref().is_some_and(|body| {
                            body.windows(b"command=ls-refs\n".len())
                                .any(|bytes| bytes == b"command=ls-refs\n")
                        }) {
                            [
                                packet(
                                    format!(
                                        "{} HEAD symref-target:refs/heads/trunk\n",
                                        "a".repeat(40)
                                    )
                                    .as_bytes(),
                                ),
                                packet(format!("{} refs/heads/trunk\n", "a".repeat(40)).as_bytes()),
                                b"0000".to_vec(),
                            ]
                            .concat()
                        } else {
                            [
                                packet(b"packfile\n"),
                                packet(b"\x01PACKdata"),
                                b"0000".to_vec(),
                            ]
                            .concat()
                        }
                    } else if content_type.ends_with("advertisement") {
                        legacy_advertisement()
                    } else {
                        b"git-packet".to_vec()
                    },
                ),
            }),
        })
    }

    async fn connect_websocket(
        &self,
        _authority_ref: &str,
        _url: String,
        _maximum_message_bytes: usize,
    ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
        Err(EgressTransportError::Refused)
    }
}

pub(super) fn context() -> PrincipalContext {
    PrincipalContext::hosted(
        "tenant-one".to_owned(),
        "person:owner".to_owned(),
        "person:owner".to_owned(),
        Some("owner@example.test".to_owned()),
        "snapshot-one".to_owned(),
        "b".repeat(64),
    )
    .unwrap()
}

struct ConcurrentAdmissionEgress {
    inner: Arc<GitEgress>,
    barrier: tokio::sync::Barrier,
}

#[async_trait]
impl EgressTransport for ConcurrentAdmissionEgress {
    async fn execute(
        &self,
        authority: &str,
        request: EgressHttpRequest,
    ) -> Result<EgressHttpResponse, EgressTransportError> {
        self.barrier.wait().await;
        self.inner.execute(authority, request).await
    }
    async fn execute_stream(
        &self,
        authority: &str,
        request: EgressStreamingHttpRequest,
    ) -> Result<EgressStreamingHttpResponse, EgressTransportError> {
        self.inner.execute_stream(authority, request).await
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
async fn project_and_branch_authority_reads_overlap_on_creation_and_each_exchange() {
    let (_, egress) = backend().await;
    let backend = backend_with_egress(Arc::new(ConcurrentAdmissionEgress {
        inner: egress,
        barrier: tokio::sync::Barrier::new(2),
    }))
    .await;
    let grant = tokio::time::timeout(
        Duration::from_secs(1),
        backend.create(&context(), request()),
    )
    .await
    .unwrap()
    .unwrap();
    tokio::time::timeout(
        Duration::from_secs(1),
        backend.exchange(v2_exchange(&grant, None)),
    )
    .await
    .unwrap()
    .unwrap();
}

pub(super) fn request() -> protocol::git_fetch::CreateRequest {
    protocol::git_fetch::CreateRequest {
        idempotency_key: "coding-session-one".to_owned(),
        connection_ref: "connection:gitlab:11111111-1111-4111-8111-111111111111".to_owned(),
        project_id: 42,
        reference: "trunk".to_owned(),
        expected_commit: "a".repeat(40),
        depth: 10,
    }
}

async fn backend() -> (GitlabBackend, Arc<GitEgress>) {
    let egress = Arc::new(GitEgress {
        project_calls: AtomicUsize::new(0),
        stream_calls: AtomicUsize::new(0),
        seen_headers: Mutex::new(Vec::new()),
        seen_urls: Mutex::new(Vec::new()),
    });
    let backend = backend_with_egress(egress.clone()).await;
    (backend, egress)
}

pub(super) async fn backend_with_egress(egress: Arc<dyn EgressTransport>) -> GitlabBackend {
    let store = Arc::new(MemoryStore::new());
    let backend =
        GitlabBackend::open_inner(
            "tenant-one".to_owned(),
            connectors_config::HostedGitlabConfig {
                origin: "https://gitlab.example.test".to_owned(),
                public_origin: "https://connectors.example.test/api/connectors/v1".to_owned(),
                git_fetch_origin: Some("https://connectors.internal".to_owned()),
                oauth_client_id: "client-one".to_owned(),
                oauth_redirect_uri:
                    "https://connectors.example.test/api/connectors/v1/oauth/gitlab/callback"
                        .to_owned(),
                user_grant_ref: "grant:gitlab:user".to_owned(),
                initiation: connectors_config::InitiationConfig::Provider,
                connect_session_ttl_seconds: 300,
                refresh_skew_seconds: 300,
            },
            store.clone(),
            super::super::GitlabState::Hosted(Arc::new(connector_state::MemoryState::new())),
            egress.clone(),
        )
        .await
        .unwrap();
    let connection = StoredConnection {
        connection_ref: "connection:gitlab:11111111-1111-4111-8111-111111111111".to_owned(),
        instance_id: "11111111-1111-4111-8111-111111111111".to_owned(),
        label: "GitLab".to_owned(),
        grant_ref: "grant:gitlab:user".to_owned(),
        owner_subject: "person:owner".to_owned(),
        external_user_id: 7,
        username: "owner".to_owned(),
        email_sha256: "c".repeat(64),
        profile: super::super::GitlabProfile::PersonalToken,
        scopes: vec!["read_api".to_owned()],
        credential_generation: 1,
        observed_at_unix_ms: 1,
        expires_at_unix_ms: None,
    };
    let credential_ref = backend
        .inner
        .connection_credential_ref(&connection, super::super::ACCESS_TOKEN_CREDENTIAL)
        .unwrap();
    store
        .put(&credential_ref, &Secret::new("synthetic-gitlab-token"))
        .await
        .unwrap();
    super::super::lock(&backend.inner.metadata)
        .connections
        .push(connection);
    backend
}

fn packet(payload: &[u8]) -> Vec<u8> {
    let mut packet = format!("{:04x}", payload.len() + 4).into_bytes();
    packet.extend_from_slice(payload);
    packet
}

fn v2_exchange(grant: &GitFetchGrant, body: Option<Vec<u8>>) -> GitFetchExchange {
    GitFetchExchange {
        session_ref: grant.session_ref.clone(),
        repository: REPOSITORY_NAME.to_owned(),
        source_authorization: Zeroizing::new(grant.expose_at_control_boundary().to_owned()),
        service: if body.is_some() {
            GitFetchService::UploadPack
        } else {
            GitFetchService::Discovery
        },
        git_protocol: Some("version=2".to_owned()),
        body,
    }
}

fn v2_command(command: &str, arguments: &[&str]) -> Vec<u8> {
    let mut body = packet(format!("command={command}\n").as_bytes());
    body.extend_from_slice(b"0001");
    for argument in arguments {
        body.extend(packet(format!("{argument}\n").as_bytes()));
    }
    body.extend_from_slice(b"0000");
    body
}

#[tokio::test]
async fn v2_negotiation_is_bound_to_generation_and_only_completed_fetch_spends_it() {
    let (backend, egress) = backend().await;
    let grant = backend.create(&context(), request()).await.unwrap();
    let refs = v2_command("ls-refs", &["symrefs"]);
    assert!(matches!(
        backend
            .exchange(v2_exchange(&grant, Some(refs.clone())))
            .await,
        Err(GitFetchDataError::Refused)
    ));
    assert_eq!(egress.stream_calls.load(Ordering::SeqCst), 0);
    backend.exchange(v2_exchange(&grant, None)).await.unwrap();
    let mut legacy = v2_exchange(&grant, None);
    legacy.git_protocol = None;
    assert!(matches!(
        backend.exchange(legacy).await,
        Err(GitFetchDataError::Refused)
    ));
    for _ in 0..2 {
        let mut response = backend
            .exchange(v2_exchange(&grant, Some(refs.clone())))
            .await
            .unwrap();
        assert!(response.body.next_chunk().await.unwrap().is_some());
        assert!(response.body.next_chunk().await.unwrap().is_none());
        assert_eq!(
            lock(&backend.inner.git_fetch_sessions)[&grant.session_ref].state,
            GitFetchSessionState::Active
        );
    }
    let fetch = v2_command(
        "fetch",
        &[&format!("want {}", "a".repeat(40)), "deepen 10", "done"],
    );
    let mut response = backend
        .exchange(v2_exchange(&grant, Some(fetch)))
        .await
        .unwrap();
    assert_eq!(
        lock(&backend.inner.git_fetch_sessions)[&grant.session_ref].state,
        GitFetchSessionState::UploadInFlight
    );
    assert!(matches!(
        backend.create(&context(), request()).await,
        Err(GitFetchControlError::Conflict)
    ));
    while response.body.next_chunk().await.unwrap().is_some() {}
    assert_eq!(
        lock(&backend.inner.git_fetch_sessions)[&grant.session_ref].state,
        GitFetchSessionState::Spent
    );
    let replay = backend.create(&context(), request()).await.unwrap();
    assert!(matches!(
        backend.exchange(v2_exchange(&replay, Some(refs))).await,
        Err(GitFetchDataError::Refused)
    ));
    assert!(matches!(
        backend.exchange(v2_exchange(&grant, None)).await,
        Err(GitFetchDataError::Refused)
    ));
    backend.exchange(v2_exchange(&replay, None)).await.unwrap();
}

#[tokio::test]
async fn v2_drop_budget_revocation_and_foreign_authority_fail_closed() {
    let (backend, egress) = backend().await;
    let grant = backend.create(&context(), request()).await.unwrap();
    let mut foreign = v2_exchange(&grant, None);
    foreign.source_authorization = Zeroizing::new("x".repeat(43));
    assert!(matches!(
        backend.exchange(foreign).await,
        Err(GitFetchDataError::Refused)
    ));
    assert_eq!(egress.stream_calls.load(Ordering::SeqCst), 0);
    backend.exchange(v2_exchange(&grant, None)).await.unwrap();
    let fetch = v2_command(
        "fetch",
        &[&format!("want {}", "a".repeat(40)), "deepen 10", "done"],
    );
    let response = backend
        .exchange(v2_exchange(&grant, Some(fetch.clone())))
        .await
        .unwrap();
    drop(response);
    assert_eq!(
        lock(&backend.inner.git_fetch_sessions)[&grant.session_ref].state,
        GitFetchSessionState::Revoked
    );
    let grant = backend.create(&context(), request()).await.unwrap();
    backend.exchange(v2_exchange(&grant, None)).await.unwrap();
    let mut response = backend
        .exchange(v2_exchange(&grant, Some(fetch)))
        .await
        .unwrap();
    lock(&backend.inner.git_fetch_sessions)
        .get_mut(&grant.session_ref)
        .unwrap()
        .response_bytes = SESSION_BYTE_LIMIT - 1;
    assert!(matches!(
        response.body.next_chunk().await,
        Err(EgressTransportError::ResponseTooLarge)
    ));
    assert_eq!(
        lock(&backend.inner.git_fetch_sessions)[&grant.session_ref].state,
        GitFetchSessionState::Revoked
    );
    let grant = backend.create(&context(), request()).await.unwrap();
    backend.exchange(v2_exchange(&grant, None)).await.unwrap();
    lock(&backend.inner.metadata).connections.clear();
    assert!(matches!(
        backend
            .exchange(v2_exchange(&grant, Some(v2_command("ls-refs", &[]))))
            .await,
        Err(GitFetchDataError::Refused)
    ));
}

fn legacy_advertisement() -> Vec<u8> {
    let expected = "a".repeat(40);
    let other = "b".repeat(40);
    let mut advertisement = packet(b"# service=git-upload-pack\n");
    advertisement.extend_from_slice(b"0000");
    advertisement.extend_from_slice(&packet(
        format!(
            "{expected} HEAD\0multi_ack thin-pack side-band-64k symref=HEAD:refs/heads/trunk\n"
        )
        .as_bytes(),
    ));
    advertisement.extend_from_slice(&packet(format!("{expected} refs/heads/trunk\n").as_bytes()));
    advertisement.extend_from_slice(&packet(format!("{other} refs/heads/private\n").as_bytes()));
    advertisement.extend_from_slice(&packet(
        format!("{expected} refs/tags/internal\n").as_bytes(),
    ));
    advertisement.extend_from_slice(b"0000");
    advertisement
}

#[test]
fn upstream_repository_is_derived_without_forwarding_provider_metadata() {
    let origin = url::Url::parse("https://gitlab.example.test").unwrap();
    assert_eq!(
        repository_url(&origin, "group/nested/repository").as_deref(),
        Some("https://gitlab.example.test/group/nested/repository.git")
    );
    assert!(repository_url(&origin, "../repository").is_none());
    assert!(repository_url(&origin, "repository").is_none());
}

#[test]
fn source_authority_digest_comparison_checks_every_byte() {
    let digest = [7_u8; 32];
    assert!(constant_time_eq(&digest, &digest));
    let mut changed = digest;
    changed[31] = 8;
    assert!(!constant_time_eq(&digest, &changed));
}

#[test]
fn upload_pack_request_is_bound_to_exact_commit_and_depth() {
    let commit = "a".repeat(40);
    let body = format!("0032want {commit}\n000ddeepen 5\n0000");
    assert!(valid_upload_pack_request(body.as_bytes(), &commit, 10));
    assert!(!valid_upload_pack_request(
        body.as_bytes(),
        &"b".repeat(40),
        10
    ));
    assert!(!valid_upload_pack_request(body.as_bytes(), &commit, 4));
    let full = format!("0032want {commit}\n0000");
    assert!(!valid_upload_pack_request(full.as_bytes(), &commit, 10));
}

#[tokio::test]
async fn idempotent_replay_keeps_locator_and_rotates_transient_authority() {
    let (backend, _) = backend().await;
    let first = backend.create(&context(), request()).await.unwrap();
    let second = backend.create(&context(), request()).await.unwrap();
    assert_eq!(first.session_ref, second.session_ref);
    assert_eq!(first.locator, second.locator);
    assert_eq!(first.reference, "trunk");
    assert_ne!(
        first.expose_at_control_boundary(),
        second.expose_at_control_boundary()
    );

    let mut conflict = request();
    conflict.depth = 11;
    assert!(matches!(
        backend.create(&context(), conflict).await,
        Err(GitFetchControlError::Conflict)
    ));
}

#[tokio::test]
async fn upload_pack_stream_is_bounded_and_spends_the_session() {
    let (backend, egress) = backend().await;
    let grant = backend.create(&context(), request()).await.unwrap();
    let authority = grant.expose_at_control_boundary().to_owned();
    let exchange = GitFetchExchange {
        session_ref: grant.session_ref.clone(),
        repository: REPOSITORY_NAME.to_owned(),
        source_authorization: Zeroizing::new(authority.clone()),
        service: GitFetchService::UploadPack,
        git_protocol: None,
        body: Some(
            b"0032want aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n000ddeepen 5\n0000".to_vec(),
        ),
    };
    let mut response = backend.exchange(exchange).await.unwrap();
    assert_eq!(
        response.body.next_chunk().await.unwrap(),
        Some(b"git-packet".to_vec())
    );
    assert_eq!(response.body.next_chunk().await.unwrap(), None);
    assert_eq!(egress.stream_calls.load(Ordering::SeqCst), 1);
    {
        let headers = egress.seen_headers.lock().unwrap();
        assert_eq!(
            headers[0].get("authorization").map(String::as_str),
            Some("Bearer synthetic-gitlab-token")
        );
        assert!(!headers[0].contains_key("x-b10x-git-source-authorization"));
    }
    assert!(egress.seen_urls.lock().unwrap()[0]
        .starts_with("https://gitlab.example.test/group/renamed-repository.git/"));

    assert!(matches!(
        backend
            .exchange(GitFetchExchange {
                session_ref: grant.session_ref,
                repository: REPOSITORY_NAME.to_owned(),
                source_authorization: Zeroizing::new(authority),
                service: GitFetchService::Discovery,
                git_protocol: None,
                body: None,
            })
            .await,
        Err(GitFetchDataError::Refused)
    ));
}

#[tokio::test]
async fn unsupported_protocol_is_refused_before_provider_egress() {
    let (backend, egress) = backend().await;
    let grant = backend.create(&context(), request()).await.unwrap();
    let authority = grant.expose_at_control_boundary().to_owned();
    let result = backend
        .exchange(GitFetchExchange {
            session_ref: grant.session_ref,
            repository: REPOSITORY_NAME.to_owned(),
            source_authorization: Zeroizing::new(authority),
            service: GitFetchService::Discovery,
            git_protocol: Some("version=1".to_owned()),
            body: None,
        })
        .await;
    assert!(matches!(result, Err(GitFetchDataError::Refused)));
    assert_eq!(egress.stream_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn discovery_advertises_only_the_exact_default_branch_snapshot() {
    let (backend, egress) = backend().await;
    let grant = backend.create(&context(), request()).await.unwrap();
    let authority = grant.expose_at_control_boundary().to_owned();
    let mut response = backend
        .exchange(GitFetchExchange {
            session_ref: grant.session_ref,
            repository: REPOSITORY_NAME.to_owned(),
            source_authorization: Zeroizing::new(authority),
            service: GitFetchService::Discovery,
            git_protocol: None,
            body: None,
        })
        .await
        .unwrap();
    let mut advertised = Vec::new();
    while let Some(chunk) = response.body.next_chunk().await.unwrap() {
        advertised.extend_from_slice(&chunk);
    }
    let text = String::from_utf8(advertised).unwrap();
    assert!(text.contains("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa HEAD"));
    assert!(text.contains("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa refs/heads/trunk"));
    assert!(!text.contains("refs/heads/private"));
    assert!(!text.contains("refs/tags/internal"));
    assert!(!text.contains("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"));
    assert!(egress.seen_urls.lock().unwrap()[0]
        .starts_with("https://gitlab.example.test/group/renamed-repository.git/"));
}

#[tokio::test]
async fn stream_expiry_revokes_a_stalled_upload() {
    let (backend, _) = backend().await;
    let grant = backend.create(&context(), request()).await.unwrap();
    let generation = {
        let mut sessions = lock(&backend.inner.git_fetch_sessions);
        let session = sessions.get_mut(&grant.session_ref).unwrap();
        session.state = GitFetchSessionState::UploadInFlight;
        session.expires_at_unix_ms = now_ms().unwrap() + 25;
        session.generation
    };
    let mut stream = BudgetedGitFetchStream {
        inner: Box::new(PendingStream),
        backend: backend.inner.clone(),
        session_ref: grant.session_ref.clone(),
        generation,
        expires_at_unix_ms: now_ms().unwrap() + 25,
        upload_pack: true,
        finished: false,
    };
    assert!(matches!(
        stream.next_chunk().await,
        Err(EgressTransportError::Transport(
            EgressTransportFailure::Timeout
        ))
    ));
    assert_eq!(
        lock(&backend.inner.git_fetch_sessions)
            .get(&grant.session_ref)
            .unwrap()
            .state,
        GitFetchSessionState::Revoked
    );
}

#[tokio::test]
async fn per_principal_capacity_is_atomic_and_never_evicts_a_live_session() {
    let (backend, _) = backend().await;
    let backend = Arc::new(backend);
    let mut tasks = Vec::new();
    for index in 0..(MAX_SESSIONS_PER_PRINCIPAL + 4) {
        let backend = backend.clone();
        tasks.push(tokio::spawn(async move {
            let mut request = request();
            request.idempotency_key = format!("coding-session-{index}");
            backend.create(&context(), request).await
        }));
    }
    let mut admitted = Vec::new();
    let mut refused = 0;
    for task in tasks {
        match task.await.unwrap() {
            Ok(grant) => admitted.push(grant),
            Err(GitFetchControlError::Unavailable) => refused += 1,
            Err(error) => panic!("unexpected creation result: {error:?}"),
        }
    }
    assert_eq!(admitted.len(), MAX_SESSIONS_PER_PRINCIPAL);
    assert_eq!(refused, 4);
    let first = &admitted[0];
    assert!(backend
        .authorize(&GitFetchAccess {
            session_ref: first.session_ref.clone(),
            repository: REPOSITORY_NAME.to_owned(),
            source_authorization: Zeroizing::new(first.expose_at_control_boundary().to_owned()),
            service: GitFetchService::Discovery,
            git_protocol: None,
        })
        .is_ok());
}

#[tokio::test]
async fn global_capacity_refuses_without_evicting_an_inflight_other_principal() {
    let (backend, _) = backend().await;
    let now = now_ms().unwrap();
    {
        let mut sessions = lock(&backend.inner.git_fetch_sessions);
        for index in 0..MAX_SESSIONS {
            sessions.insert(
                format!("git-fetch:occupied-{index}"),
                GitFetchSessionRecord {
                    idempotency_key: format!("other-{index}"),
                    owner_subject: format!("person:other-{index}"),
                    connection_ref: format!("connection:other-{index}"),
                    project_id: index as u64,
                    reference: "trunk".to_owned(),
                    expected_commit: "a".repeat(40),
                    depth: 1,
                    authority_sha256: [index as u8; 32],
                    generation: 1,
                    expires_at_unix_ms: now + SESSION_TTL_MS,
                    request_count: 0,
                    response_bytes: 0,
                    state: if index == 0 {
                        GitFetchSessionState::UploadInFlight
                    } else {
                        GitFetchSessionState::Active
                    },
                    protocol: None,
                    v2_negotiated: false,
                },
            );
        }
    }
    assert!(matches!(
        backend.create(&context(), request()).await,
        Err(GitFetchControlError::Unavailable)
    ));
    let sessions = lock(&backend.inner.git_fetch_sessions);
    assert_eq!(sessions.len(), MAX_SESSIONS);
    assert_eq!(
        sessions.get("git-fetch:occupied-0").unwrap().state,
        GitFetchSessionState::UploadInFlight
    );
}

#[tokio::test]
async fn removed_principal_connection_revokes_a_live_session() {
    let (backend, _) = backend().await;
    let grant = backend.create(&context(), request()).await.unwrap();
    let authority = grant.expose_at_control_boundary().to_owned();
    super::super::lock(&backend.inner.metadata)
        .connections
        .clear();
    assert!(matches!(
        backend
            .exchange(GitFetchExchange {
                session_ref: grant.session_ref,
                repository: REPOSITORY_NAME.to_owned(),
                source_authorization: Zeroizing::new(authority),
                service: GitFetchService::Discovery,
                git_protocol: None,
                body: None,
            })
            .await,
        Err(GitFetchDataError::Refused)
    ));
}

#[tokio::test]
async fn current_grant_and_provider_default_tip_are_revalidated() {
    let (backend, _) = backend().await;

    let mut wrong_branch = request();
    wrong_branch.reference = "main".to_owned();
    assert!(matches!(
        backend.create(&context(), wrong_branch).await,
        Err(GitFetchControlError::NotGranted)
    ));
    let mut wrong_commit = request();
    wrong_commit.expected_commit = "b".repeat(40);
    assert!(matches!(
        backend.create(&context(), wrong_commit).await,
        Err(GitFetchControlError::NotGranted)
    ));

    let grant = backend.create(&context(), request()).await.unwrap();
    let authority = grant.expose_at_control_boundary().to_owned();
    super::super::lock(&backend.inner.metadata).connections[0].grant_ref =
        "grant:gitlab:revoked".to_owned();
    assert!(matches!(
        backend
            .exchange(GitFetchExchange {
                session_ref: grant.session_ref,
                repository: REPOSITORY_NAME.to_owned(),
                source_authorization: Zeroizing::new(authority),
                service: GitFetchService::Discovery,
                git_protocol: None,
                body: None,
            })
            .await,
        Err(GitFetchDataError::Refused)
    ));
    assert!(matches!(
        backend.create(&context(), request()).await,
        Err(GitFetchControlError::NotGranted)
    ));
}

use super::*;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

struct Clock {
    wall: u64,
    monotonic: std::time::Instant,
    elapsed: AtomicU64,
}
impl Clock {
    fn new() -> Self {
        let (wall, monotonic) = SystemRefreshClock.now().unwrap();
        Self {
            wall,
            monotonic,
            elapsed: AtomicU64::new(0),
        }
    }
    fn advance(&self, milliseconds: u64) {
        self.elapsed.fetch_add(milliseconds, Ordering::SeqCst);
    }
}
impl RefreshClock for Clock {
    fn now(&self) -> std::result::Result<(u64, std::time::Instant), custody::CustodyError> {
        let elapsed = self.elapsed.load(Ordering::SeqCst)
            + u64::try_from(self.monotonic.elapsed().as_millis()).unwrap();
        Ok((
            self.wall + elapsed,
            self.monotonic + Duration::from_millis(elapsed),
        ))
    }
}

#[derive(Default)]
struct Egress {
    replies: Mutex<VecDeque<(u16, String)>>,
    requests: Mutex<Vec<(String, connector_resolve::Request)>>,
    after_request: Mutex<Option<AfterRequest>>,
    hold_request: Mutex<Option<usize>>,
    entered: tokio::sync::Notify,
    dropped: AtomicBool,
}
struct AfterRequest {
    number: usize,
    run: Box<dyn FnOnce() + Send>,
}
impl Egress {
    fn after(&self, number: usize, run: impl FnOnce() + Send + 'static) {
        *self.after_request.lock().unwrap() = Some(AfterRequest {
            number,
            run: Box::new(run),
        });
    }
    fn reply(&self, status: u16, value: serde_json::Value) {
        self.replies
            .lock()
            .unwrap()
            .push_back((status, value.to_string()));
    }
    fn token(&self, access: &str, refresh: Option<&str>, seconds: u64) {
        self.reply(200, serde_json::json!({ "access_token": access, "refresh_token": refresh, "token_type": "bEaReR", "expires_in": seconds }));
    }
    fn info(&self, client: &str, subject: u64, scopes: &[&str]) {
        self.reply(200, serde_json::json!({ "resource_owner_id": subject, "scope": scopes, "application": { "uid": client } }));
    }
    fn count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}
#[async_trait]
impl EgressTransport for Egress {
    async fn execute(
        &self,
        authority: &str,
        request: EgressHttpRequest,
    ) -> std::result::Result<service::EgressHttpResponse, service::EgressTransportError> {
        if request.request.url.contains("/oauth/") {
            assert_eq!(request.maximum_response_bytes, MAX_RESPONSE);
        }
        self.requests
            .lock()
            .unwrap()
            .push((authority.into(), request.request));
        let hold = *self.hold_request.lock().unwrap() == Some(self.count());
        if hold {
            struct PendingDrop<'a>(&'a AtomicBool);
            impl Drop for PendingDrop<'_> {
                fn drop(&mut self) {
                    self.0.store(true, Ordering::SeqCst);
                }
            }
            let _drop = PendingDrop(&self.dropped);
            self.entered.notify_one();
            std::future::pending::<()>().await;
        }
        let mut action = self.after_request.lock().unwrap();
        if action
            .as_ref()
            .is_some_and(|action| action.number == self.count())
        {
            (action.take().unwrap().run)();
        }
        drop(action);
        let (status, body) = self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("an explicitly queued synthetic provider response");
        Ok(service::EgressHttpResponse {
            status,
            headers: BTreeMap::new(),
            body: body.into_bytes(),
        })
    }
    async fn connect_websocket(
        &self,
        _: &str,
        _: String,
        _: usize,
    ) -> std::result::Result<Box<dyn service::EgressWebSocket>, service::EgressTransportError> {
        unreachable!("the personal OAuth owner never opens a websocket")
    }
}

fn owner() -> PrincipalContext {
    PrincipalContext::local(&operation_api::OwnerContext {
        tenant_id: "local".into(),
        agent_id: "oauth-fixture".into(),
        agent_revision: 1,
        authority_snapshot_id: "snapshot:oauth-fixture".into(),
        authority_snapshot_sha256: "1".repeat(64),
    })
    .unwrap()
}
fn configuration() -> CatalogIntegrationConfig {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let redirect = format!("http://{}/oauth/callback", listener.local_addr().unwrap());
    serde_json::from_value(serde_json::json!({ "provider": "gitlab", "instance": "personal", "grant_ref": "grant:personal",
        "initiation": "platform", "credential": "gitlab.oauth_token", "operator_approved": true,
        "endpoints": { "origin": "https://gitlab.example" }, "oauth": {
            "auth_profile": "gitlab.oauth_token", "flow": "authorization_code_pkce", "client_authentication": "public",
            "client_id": "fixture-client", "redirect_uri": redirect, "browser_placement": "same_machine",
            "registration_use": "development_only", "custody": "development_file", "allowed_scopes": ["read_api"]
        }})).unwrap()
}
async fn open(
    directory: &Path,
    configured: &[CatalogIntegrationConfig],
    egress: Arc<Egress>,
    clock: Arc<Clock>,
) -> PersonalOAuthBackend {
    PersonalOAuthBackend::open_with_clock(owner(), configured, directory, egress, true, clock)
        .await
        .unwrap()
}
fn create() -> connection_api::ConnectionRequest {
    connection_api::ConnectionRequest::ConnectSessionCreate(
        connection_api::ConnectSessionCreateRequest {
            integration_ref: "gitlab".into(),
            label: "Display only".into(),
            auth_profile: Some("gitlab.oauth_token".into()),
        },
    )
}
async fn http(authority: &str, target: &str, extra: &str) -> String {
    let mut stream = tokio::net::TcpStream::connect(authority).await.unwrap();
    stream
        .write_all(format!("GET {target} HTTP/1.1\r\nHost: {authority}\r\n{extra}\r\n").as_bytes())
        .await
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response
}
async fn deliver_code(backend: &PersonalOAuthBackend) -> connection_api::ConnectSessionStatus {
    let connection_api::ConnectionResult::ConnectSessionCreate(status) =
        backend.handle_connection(&owner(), create()).await.unwrap()
    else {
        panic!("session")
    };
    let url = url::Url::parse(status.browser_completion_url.as_deref().unwrap()).unwrap();
    let authority = format!("127.0.0.1:{}", url.port().unwrap());
    let token = url.fragment().unwrap().strip_prefix("token=").unwrap();
    let instructions = http(
        &authority,
        "/instructions",
        &format!("X-Connect-Session: {token}\r\n"),
    )
    .await;
    let body: serde_json::Value =
        serde_json::from_str(instructions.split_once("\r\n\r\n").unwrap().1).unwrap();
    let authorization = url::Url::parse(body["authorization_url"].as_str().unwrap()).unwrap();
    let state = authorization
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();
    let reply = http(
        &authority,
        &format!("/oauth/callback?state={state}&code=fixture-code"),
        "",
    )
    .await;
    assert!(reply.starts_with("HTTP/1.1 200"));
    status
}
async fn authorize(backend: &PersonalOAuthBackend) -> connection_api::ConnectSessionStatus {
    let status = deliver_code(backend).await;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match backend
                .inner
                .session_status(&status.connect_session_ref)
                .await
            {
                Ok(status) if status.state == connection_api::ConnectSessionState::Completed => {
                    break status
                }
                Ok(status) if status.state != connection_api::ConnectSessionState::Pending => {
                    panic!("authorization did not complete")
                }
                _ => tokio::task::yield_now().await,
            }
        }
    })
    .await
    .unwrap()
}
async fn invocation(backend: &PersonalOAuthBackend) -> operation_api::InvokeRequest {
    let operation_api::OperationResult::Describe(description) = backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Describe(operation_api::DescribeRequest {
                operation_ref: "gitlab-project-list".into(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!("description")
    };
    operation_api::InvokeRequest {
        operation_ref: description.operation_ref,
        connection_ref: backend.inner.bindings[0]
            .custody
            .identity
            .connection
            .clone(),
        description_ref: description.description_ref,
        input: serde_json::json!({}),
        approval_evidence_ref: None,
    }
}

#[tokio::test]
async fn actual_pkce_uses_observed_evidence_and_same_store_for_dispatch_and_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("access-one", Some("refresh-one"), 60);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(
        directory.path(),
        std::slice::from_ref(&config),
        egress.clone(),
        clock.clone(),
    )
    .await;
    let completed = authorize(&backend).await;
    assert!(completed.browser_completion_url.is_none());
    assert!(completed.completion_endpoint.is_none());
    assert_eq!(
        completed.connection_ref.as_deref(),
        Some(
            backend.inner.bindings[0]
                .custody
                .identity
                .connection
                .as_str()
        )
    );
    let request = invocation(&backend).await;
    egress.reply(200, serde_json::json!([]));
    backend
        .handle(&owner(), operation_api::OperationRequest::Invoke(request))
        .await
        .unwrap();
    {
        let requests = egress.requests.lock().unwrap();
        assert_eq!(requests[0].1.url, "https://gitlab.example/oauth/token");
        assert_eq!(requests[1].1.url, "https://gitlab.example/oauth/token/info");
        assert_eq!(
            requests[1]
                .1
                .headers
                .get("authorization")
                .map(String::as_str),
            Some("Bearer access-one")
        );
        assert!(requests[2]
            .1
            .url
            .starts_with("https://gitlab.example/api/v4/projects"));
        assert_eq!(
            requests[2]
                .1
                .headers
                .get("Authorization")
                .or_else(|| requests[2].1.headers.get("authorization"))
                .map(String::as_str),
            Some("Bearer access-one")
        );
    }
    backend.shutdown().await;
    drop(backend);
    let reopened = open(directory.path(), &[config], egress.clone(), clock).await;
    let request = invocation(&reopened).await;
    egress.reply(200, serde_json::json!([]));
    reopened
        .handle(&owner(), operation_api::OperationRequest::Invoke(request))
        .await
        .unwrap();
    assert_eq!(
        egress.count(),
        4,
        "reopen spends the coherent access token without another authorization"
    );
    reopened.shutdown().await;
}

#[tokio::test]
async fn unique_binding_and_owner_refusals_happen_before_listener_session_or_egress() {
    let directory = tempfile::tempdir().unwrap();
    let first = configuration();
    let mut second = first.clone();
    second.instance = Some("second".into());
    let egress = Arc::new(Egress::default());
    let backend = open(
        directory.path(),
        &[first.clone(), second],
        egress.clone(),
        Arc::new(Clock::new()),
    )
    .await;
    assert!(backend.handle_connection(&owner(), create()).await.is_err());
    assert!(backend.inner.sessions.lock().unwrap().is_empty());
    assert_eq!(egress.count(), 0);
    let uri = url::Url::parse(
        first
            .oauth
            .as_ref()
            .unwrap()
            .redirect_uri
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    assert!(std::net::TcpListener::bind(("127.0.0.1", uri.port().unwrap())).is_ok());
    backend.shutdown().await;
}

#[tokio::test]
async fn refresh_rotation_before_bad_token_info_blocks_old_dispatch_across_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("old-access", Some("old-refresh"), 1);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(
        directory.path(),
        std::slice::from_ref(&config),
        egress.clone(),
        clock.clone(),
    )
    .await;
    authorize(&backend).await;
    let request = invocation(&backend).await;
    clock.advance(1_001);
    egress.token("rotated-access", Some("rotated-refresh"), 60);
    egress.info("different-client", 42, &["read_api"]);
    assert!(backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(request.clone())
        )
        .await
        .is_err());
    assert_eq!(egress.count(), 4);
    assert!(backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(request.clone())
        )
        .await
        .is_err());
    assert_eq!(
        egress.count(),
        4,
        "neither old access nor old refresh may leave after uncertainty"
    );
    backend.shutdown().await;
    drop(backend);
    let reopened = open(directory.path(), &[config], egress.clone(), clock).await;
    assert!(reopened
        .handle(&owner(), operation_api::OperationRequest::Invoke(request))
        .await
        .is_err());
    assert_eq!(
        egress.count(),
        4,
        "the FULL marker survives actual store reopening"
    );
    reopened.shutdown().await;
}

struct UnknownMarker {
    state: Arc<dyn StateStore>,
    fail: AtomicBool,
}
impl StateStore for UnknownMarker {
    fn read(
        &self,
        key: &str,
        maximum: usize,
    ) -> std::result::Result<Option<Vec<u8>>, connector_state::StateError> {
        self.state.read(key, maximum)
    }
    fn replace(
        &self,
        key: &str,
        bytes: &[u8],
        maximum: usize,
    ) -> std::result::Result<(), connector_state::StateError> {
        self.state.replace(key, bytes, maximum)?;
        if self.fail.swap(false, Ordering::SeqCst) {
            Err(connector_state::StateError::Unavailable)
        } else {
            Ok(())
        }
    }
    fn append(
        &self,
        key: &str,
        bytes: &[u8],
        maximum: usize,
    ) -> std::result::Result<usize, connector_state::StateError> {
        self.state.append(key, bytes, maximum)
    }
    fn delete(&self, key: &str) -> std::result::Result<(), connector_state::StateError> {
        self.state.delete(key)
    }
}

#[tokio::test]
async fn unknown_full_marker_confirmation_sends_zero_refresh_egress_and_survives_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("old-access", Some("old-refresh"), 1);
    egress.info("fixture-client", 42, &["read_api"]);
    let mut backend = open(
        directory.path(),
        std::slice::from_ref(&config),
        egress.clone(),
        clock.clone(),
    )
    .await;
    authorize(&backend).await;
    // Wait for the bounded acquisition task to relinquish its Arc, without replacing its store.
    tokio::time::timeout(Duration::from_secs(5), async {
        while Arc::strong_count(&backend.inner) != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let inner = Arc::get_mut(&mut backend.inner).unwrap();
    inner.markers = Arc::new(UnknownMarker {
        state: inner.markers.clone(),
        fail: AtomicBool::new(true),
    });
    let request = invocation(&backend).await;
    clock.advance(1_001);
    assert!(backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(request.clone())
        )
        .await
        .is_err());
    assert_eq!(egress.count(), 2);
    backend.shutdown().await;
    drop(backend);
    let reopened = open(directory.path(), &[config], egress.clone(), clock).await;
    assert!(reopened
        .handle(&owner(), operation_api::OperationRequest::Invoke(request))
        .await
        .is_err());
    assert_eq!(egress.count(), 2);
    reopened.shutdown().await;
}

#[tokio::test]
async fn successful_refresh_replaces_both_secrets_once_and_remains_callable_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("old-access", Some("old-refresh"), 1);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(
        directory.path(),
        std::slice::from_ref(&config),
        egress.clone(),
        clock.clone(),
    )
    .await;
    authorize(&backend).await;
    let first = invocation(&backend).await;
    let second = first.clone();
    clock.advance(1_001);
    egress.token("new-access", Some("new-refresh"), 60);
    egress.info("fixture-client", 42, &["read_api"]);
    egress.reply(200, serde_json::json!([]));
    egress.reply(200, serde_json::json!([]));
    let context = owner();
    let (left, right) = tokio::join!(
        backend.handle(&context, operation_api::OperationRequest::Invoke(first)),
        backend.handle(&context, operation_api::OperationRequest::Invoke(second)),
    );
    left.unwrap();
    right.unwrap();
    assert_eq!(egress.count(), 6, "two invokes share one completed refresh");
    {
        let requests = egress.requests.lock().unwrap();
        let form: BTreeMap<_, _> =
            url::form_urlencoded::parse(requests[2].1.body.as_ref().unwrap().as_bytes())
                .into_owned()
                .collect();
        assert_eq!(form["grant_type"], "refresh_token");
        assert_eq!(form["refresh_token"], "old-refresh");
        assert_eq!(
            form["redirect_uri"],
            config
                .oauth
                .as_ref()
                .unwrap()
                .redirect_uri
                .as_deref()
                .unwrap()
        );
    }
    let binding = &backend.inner.bindings[0];
    assert_eq!(
        backend
            .inner
            .store
            .get(&binding.refresh_address)
            .await
            .unwrap()
            .expose_secret(),
        "new-refresh"
    );
    assert!(!backend.inner.reconcile_marker(binding).unwrap());
    backend.shutdown().await;
    drop(backend);
    let reopened = open(directory.path(), &[config], egress.clone(), clock).await;
    egress.reply(200, serde_json::json!([]));
    reopened
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(invocation(&reopened).await),
        )
        .await
        .unwrap();
    assert_eq!(egress.count(), 7);
    reopened.shutdown().await;
}

#[tokio::test]
async fn refresh_does_not_reset_its_original_window_after_token_egress() {
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("old-access", Some("old-refresh"), 1);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(
        directory.path(),
        std::slice::from_ref(&config),
        egress.clone(),
        clock.clone(),
    )
    .await;
    authorize(&backend).await;
    let request = invocation(&backend).await;
    clock.advance(1_001);
    egress.token("rotated-access", Some("rotated-refresh"), 60);
    let delayed = clock.clone();
    egress.after(3, move || delayed.advance(30_000));
    assert!(backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(request.clone())
        )
        .await
        .is_err());
    assert_eq!(
        egress.count(),
        3,
        "the original deadline refuses token-info egress"
    );
    backend.shutdown().await;
    drop(backend);
    let reopened = open(directory.path(), &[config], egress.clone(), clock).await;
    assert!(reopened
        .handle(&owner(), operation_api::OperationRequest::Invoke(request))
        .await
        .is_err());
    assert_eq!(
        egress.count(),
        3,
        "expiry cannot make the possibly rotated old token reusable"
    );
    reopened.shutdown().await;
}

#[tokio::test]
async fn rotation_followed_by_real_file_prepare_refusal_blocks_old_token_after_reopen() {
    use std::os::unix::fs::PermissionsExt as _;
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("old-access", Some("old-refresh"), 1);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(
        directory.path(),
        std::slice::from_ref(&config),
        egress.clone(),
        clock.clone(),
    )
    .await;
    authorize(&backend).await;
    let request = invocation(&backend).await;
    clock.advance(1_001);
    egress.token("rotated-access", Some("rotated-refresh"), 60);
    egress.info("fixture-client", 42, &["read_api"]);
    let path = directory.path().join("oauth/credentials.store");
    let unsafe_path = path.clone();
    egress.after(4, move || {
        std::fs::set_permissions(unsafe_path, std::fs::Permissions::from_mode(0o644)).unwrap()
    });
    assert!(backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(request.clone())
        )
        .await
        .is_err());
    assert_eq!(egress.count(), 4);
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
    backend.shutdown().await;
    drop(backend);
    let reopened = open(directory.path(), &[config], egress.clone(), clock).await;
    assert!(reopened
        .handle(&owner(), operation_api::OperationRequest::Invoke(request))
        .await
        .is_err());
    assert_eq!(egress.count(), 4);
    reopened.shutdown().await;
}

#[tokio::test]
async fn wrong_owner_unknown_profile_and_nonpersistent_create_have_zero_egress() {
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let backend = open(
        directory.path(),
        std::slice::from_ref(&config),
        egress.clone(),
        Arc::new(Clock::new()),
    )
    .await;
    let other = PrincipalContext::local(&operation_api::OwnerContext {
        tenant_id: "local".into(),
        agent_id: "other".into(),
        agent_revision: 1,
        authority_snapshot_id: "snapshot:oauth-fixture".into(),
        authority_snapshot_sha256: "1".repeat(64),
    })
    .unwrap();
    assert!(backend.handle_connection(&other, create()).await.is_err());
    let connection_api::ConnectionRequest::ConnectSessionCreate(mut request) = create() else {
        unreachable!()
    };
    request.auth_profile = Some("gitlab.token".into());
    assert!(backend
        .handle_connection(
            &owner(),
            connection_api::ConnectionRequest::ConnectSessionCreate(request)
        )
        .await
        .is_err());
    assert!(backend.inner.sessions.lock().unwrap().is_empty());
    assert_eq!(egress.count(), 0);
    backend.shutdown().await;
    drop(backend);
    let backend = PersonalOAuthBackend::open_with_clock(
        owner(),
        &[config],
        directory.path(),
        egress.clone(),
        false,
        Arc::new(Clock::new()),
    )
    .await
    .unwrap();
    assert!(backend.handle_connection(&owner(), create()).await.is_err());
    assert!(backend.inner.sessions.lock().unwrap().is_empty());
    assert_eq!(egress.count(), 0);
    backend.shutdown().await;
}

fn device_configuration() -> CatalogIntegrationConfig {
    let mut config = configuration();
    let registration = config.oauth.as_mut().unwrap();
    registration.flow = PersonalOAuthFlow::DeviceAuthorization;
    registration.redirect_uri = None;
    registration.browser_placement = connectors_config::OAuthBrowserPlacement::OtherMachine;
    config
}

async fn authorize_device(backend: &PersonalOAuthBackend) -> connection_api::ConnectSessionStatus {
    let connection_api::ConnectionResult::ConnectSessionCreate(status) =
        backend.handle_connection(&owner(), create()).await.unwrap()
    else {
        panic!("device session")
    };
    let url = url::Url::parse(status.browser_completion_url.as_deref().unwrap()).unwrap();
    let authority = format!("127.0.0.1:{}", url.port().unwrap());
    let token = url.fragment().unwrap().strip_prefix("token=").unwrap();
    let private = http(
        &authority,
        "/instructions",
        &format!("X-Connect-Session: {token}\r\n"),
    )
    .await;
    let private: serde_json::Value =
        serde_json::from_str(private.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(
        private["verification_uri"],
        "https://gitlab.example/oauth/device"
    );
    assert_eq!(private["user_code"], "ABCD-EFGH");
    assert!(private.get("authorization_url").is_none());
    assert!(private.get("device_code").is_none());
    let completed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(current) = backend
                .inner
                .session_status(&status.connect_session_ref)
                .await
            {
                if current.state == connection_api::ConnectSessionState::Completed {
                    break current;
                }
                assert_eq!(current.state, connection_api::ConnectSessionState::Pending);
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(tokio::net::TcpStream::connect(authority).await.is_err());
    completed
}
fn device_response(egress: &Egress) {
    egress.reply(200, serde_json::json!({"device_code": "private-device-code", "user_code": "ABCD-EFGH",
        "verification_uri": "https://gitlab.example/oauth/device", "expires_in": 60, "interval": 1}));
}

#[tokio::test]
async fn device_optional_refresh_omission_deletes_previous_secret_and_refuses_on_expiry() {
    let directory = tempfile::tempdir().unwrap();
    let config = device_configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    device_response(&egress);
    egress.token("first-access", Some("first-refresh"), 60);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(
        directory.path(),
        std::slice::from_ref(&config),
        egress.clone(),
        clock.clone(),
    )
    .await;
    let first = authorize_device(&backend).await;
    let address = backend.inner.bindings[0].refresh_address.clone();
    assert!(backend.inner.store.get(&address).await.is_ok());
    device_response(&egress);
    egress.token("second-access", None, 1);
    egress.info("fixture-client", 42, &["read_api"]);
    let second = authorize_device(&backend).await;
    assert_eq!(
        first.connection_ref, second.connection_ref,
        "display setup reuses the stable configured binding"
    );
    assert!(
        backend.inner.store.get(&address).await.is_err(),
        "omitted optional refresh atomically removes the old one"
    );
    let request = invocation(&backend).await;
    clock.advance(1_001);
    assert!(backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(request.clone())
        )
        .await
        .is_err());
    assert_eq!(
        egress.count(),
        6,
        "expiry without refresh cannot send the old access token or start another device flow"
    );
    backend.shutdown().await;
    drop(backend);
    let reopened = open(directory.path(), &[config], egress.clone(), clock).await;
    assert!(reopened
        .handle(&owner(), operation_api::OperationRequest::Invoke(request))
        .await
        .is_err());
    assert_eq!(egress.count(), 6);
    reopened.shutdown().await;
}

#[tokio::test]
async fn pending_callback_shutdown_joins_receiver_and_cannot_revive_session_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    let backend = open(
        directory.path(),
        std::slice::from_ref(&config),
        egress.clone(),
        clock.clone(),
    )
    .await;
    let connection_api::ConnectionResult::ConnectSessionCreate(status) =
        backend.handle_connection(&owner(), create()).await.unwrap()
    else {
        panic!("session")
    };
    let url = url::Url::parse(status.browser_completion_url.as_deref().unwrap()).unwrap();
    backend.shutdown().await;
    assert!(
        tokio::net::TcpStream::connect(("127.0.0.1", url.port().unwrap()))
            .await
            .is_err()
    );
    assert!(
        !backend.inner.sessions.lock().unwrap()[&status.connect_session_ref]
            .liveness
            .is_live()
    );
    assert_eq!(egress.count(), 0);
    drop(backend);
    let reopened = open(directory.path(), &[config], egress.clone(), clock).await;
    assert!(reopened
        .inner
        .session_status(&status.connect_session_ref)
        .await
        .is_err());
    assert!(reopened.inner.sessions.lock().unwrap().is_empty());
    assert_eq!(egress.count(), 0);
    reopened.shutdown().await;
}

#[tokio::test]
async fn invalid_lease_or_source_input_refuses_before_refresh_marker_and_egress() {
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("expired-access", Some("refresh"), 1);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(directory.path(), &[config], egress.clone(), clock.clone()).await;
    authorize(&backend).await;
    let mut request = invocation(&backend).await;
    clock.advance(1_001);
    request.description_ref = "description:unknown".into();
    let error = backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(request.clone()),
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.code,
        operation_api::OperationErrorCode::StaleAuthority
    );
    let operation_api::OperationResult::Describe(schedule) = backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Describe(operation_api::DescribeRequest {
                operation_ref: "gitlab-pipeline-schedule-list".into(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!("description")
    };
    request.description_ref = schedule.description_ref.clone();
    let error = backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(request.clone()),
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.code,
        operation_api::OperationErrorCode::StaleAuthority,
        "the existing lease names a different operation"
    );
    request.operation_ref = schedule.operation_ref;
    let error = backend
        .handle(&owner(), operation_api::OperationRequest::Invoke(request))
        .await
        .unwrap_err();
    assert_eq!(
        error.code,
        operation_api::OperationErrorCode::InvalidInput,
        "source-required project id is absent"
    );
    assert_eq!(egress.count(), 2);
    assert!(backend.inner.marker_blocks.lock().unwrap().is_empty());
    assert!(backend
        .inner
        .markers
        .read(
            &marker_key(&backend.inner.bindings[0].custody.identity).unwrap(),
            MAX_MARKER
        )
        .unwrap()
        .is_none());
    backend.shutdown().await;
}

fn access_address(backend: &PersonalOAuthBackend) -> CredentialRef {
    let binding = &backend.inner.bindings[0];
    let policy = &binding.policy;
    crate::credential_address(
        backend.inner.owner.tenant_id(),
        policy.provider.authority.unwrap(),
        &policy.configured,
        crate::credential_leaf(policy.provider, Some(&policy.registration.auth_profile)).unwrap(),
    )
    .unwrap()
}

async fn join_acquisition(backend: &PersonalOAuthBackend, reference: &str) {
    let task = backend
        .inner
        .sessions
        .lock()
        .unwrap()
        .get_mut(reference)
        .unwrap()
        .task
        .take()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        Arc::strong_count(&backend.inner),
        1,
        "the owned task has relinquished its backend"
    );
    assert!(!backend.inner.sessions.lock().unwrap()[reference]
        .liveness
        .is_live());
}

#[tokio::test]
async fn actual_prepare_then_preclaim_expiry_aborts_without_completed_or_credentials() {
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("prepared-access", Some("prepared-refresh"), 600);
    egress.info("fixture-client", 42, &["read_api"]);
    let mut backend = open(
        directory.path(),
        std::slice::from_ref(&config),
        egress.clone(),
        clock.clone(),
    )
    .await;
    let fired = Arc::new(AtomicBool::new(false));
    let flag = fired.clone();
    let timing = clock.clone();
    Arc::get_mut(&mut backend.inner)
        .unwrap()
        .custody
        .observe_journal(Arc::new(move |phase, moment| {
            if phase == custody::JournalPhase::Preparing
                && moment == custody::JournalMoment::AfterWrite
                && !flag.swap(true, Ordering::SeqCst)
            {
                timing.advance(300_001);
            }
            assert!(
                phase != custody::JournalPhase::Decided,
                "expired authorization cannot persist a decision"
            );
            Ok(())
        }));
    let status = deliver_code(&backend).await;
    join_acquisition(&backend, &status.connect_session_ref).await;
    assert!(fired.load(Ordering::SeqCst));
    let terminal = backend
        .inner
        .session_status(&status.connect_session_ref)
        .await
        .unwrap();
    assert!(terminal.state != connection_api::ConnectSessionState::Completed);
    assert!(terminal.browser_completion_url.is_none());
    let address = access_address(&backend);
    assert!(backend
        .inner
        .store
        .get(&address)
        .await
        .unwrap_err()
        .is_not_found());
    assert!(backend
        .inner
        .custody
        .snapshot(&backend.inner.bindings[0].custody.identity)
        .unwrap()
        .is_none());
    drop(backend);
    let reopened = open(directory.path(), &[config], egress.clone(), clock).await;
    assert!(reopened
        .inner
        .store
        .get(&address)
        .await
        .unwrap_err()
        .is_not_found());
    assert_eq!(egress.count(), 2);
    reopened.shutdown().await;
}

#[tokio::test]
async fn actual_timely_claim_survives_full_decision_write_after_session_deadline() {
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("timely-access", Some("timely-refresh"), 600);
    egress.info("fixture-client", 42, &["read_api"]);
    let mut backend = open(
        directory.path(),
        std::slice::from_ref(&config),
        egress.clone(),
        clock.clone(),
    )
    .await;
    let fired = Arc::new(AtomicBool::new(false));
    let flag = fired.clone();
    let timing = clock.clone();
    Arc::get_mut(&mut backend.inner)
        .unwrap()
        .custody
        .observe_journal(Arc::new(move |phase, moment| {
            if phase == custody::JournalPhase::Decided
                && moment == custody::JournalMoment::BeforeWrite
                && !flag.swap(true, Ordering::SeqCst)
            {
                timing.advance(300_001);
            }
            Ok(())
        }));
    let status = authorize(&backend).await;
    join_acquisition(&backend, &status.connect_session_ref).await;
    assert!(fired.load(Ordering::SeqCst));
    let publication = backend
        .inner
        .custody
        .snapshot(&backend.inner.bindings[0].custody.identity)
        .unwrap()
        .unwrap();
    let authorization = publication.authorization.as_ref().unwrap();
    assert!(authorization.authorized_at < authorization.deadline);
    assert!(backend.inner.now().unwrap() > authorization.deadline);
    assert_eq!(status.state, connection_api::ConnectSessionState::Completed);
    let address = access_address(&backend);
    assert_eq!(
        backend
            .inner
            .store
            .get(&address)
            .await
            .unwrap()
            .expose_secret(),
        "timely-access"
    );
    drop(backend);
    let reopened = open(directory.path(), &[config], egress.clone(), clock).await;
    assert_eq!(
        reopened
            .inner
            .store
            .get(&address)
            .await
            .unwrap()
            .expose_secret(),
        "timely-access"
    );
    assert_eq!(egress.count(), 2);
    reopened.shutdown().await;
}

async fn uncertain_decision_reopens(
    phase_to_interrupt: custody::JournalPhase,
    observation: custody::JournalMoment,
) {
    let committed = phase_to_interrupt == custody::JournalPhase::Published;
    let confirmed = committed || observation == custody::JournalMoment::AfterWrite;
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("recoverable-access", Some("recoverable-refresh"), 600);
    egress.info("fixture-client", 42, &["read_api"]);
    let mut backend = open(
        directory.path(),
        std::slice::from_ref(&config),
        egress.clone(),
        clock.clone(),
    )
    .await;
    let fired = Arc::new(AtomicBool::new(false));
    let flag = fired.clone();
    let timing = clock.clone();
    Arc::get_mut(&mut backend.inner)
        .unwrap()
        .custody
        .observe_journal(Arc::new(move |phase, moment| {
            if phase == phase_to_interrupt
                && moment == observation
                && !flag.swap(true, Ordering::SeqCst)
            {
                timing.advance(300_001);
                return Err(custody::CustodyError::Unavailable);
            }
            Ok(())
        }));
    let status = deliver_code(&backend).await;
    join_acquisition(&backend, &status.connect_session_ref).await;
    assert!(fired.load(Ordering::SeqCst));
    assert!(
        matches!(
            backend
                .inner
                .session_status(&status.connect_session_ref)
                .await,
            Err(PersonalOAuthError::Unavailable)
        ),
        "uncertain I/O cannot become Completed or Expired"
    );
    let address = access_address(&backend);
    if committed {
        assert_eq!(
            backend
                .inner
                .store
                .get(&address)
                .await
                .unwrap()
                .expose_secret(),
            "recoverable-access"
        );
    } else {
        assert!(
            backend
                .inner
                .store
                .get(&address)
                .await
                .unwrap_err()
                .is_not_found(),
            "an unconfirmed decision cannot immediately commit secrets"
        );
    }
    assert!(backend
        .inner
        .custody
        .snapshot(&backend.inner.bindings[0].custody.identity)
        .is_err());
    // No shutdown/recover call: close the finished owner and reopen the actual SQLite/FileStore.
    // The after-write callback is reached only after FULL write plus byte-equal journal readback.
    drop(backend);
    let reopened = open(directory.path(), &[config], egress.clone(), clock).await;
    assert!(!reopened
        .inner
        .sessions
        .lock()
        .unwrap()
        .contains_key(&status.connect_session_ref));
    let publication = reopened
        .inner
        .custody
        .snapshot(&reopened.inner.bindings[0].custody.identity)
        .unwrap();
    assert_eq!(publication.is_some(), confirmed);
    let request = invocation(&reopened).await;
    if confirmed {
        assert_eq!(
            reopened
                .inner
                .store
                .get(&address)
                .await
                .unwrap()
                .expose_secret(),
            "recoverable-access"
        );
        let authorization = publication.unwrap().authorization.unwrap();
        assert!(authorization.authorized_at < authorization.deadline);
        assert!(reopened.inner.now().unwrap() > authorization.deadline);
        egress.reply(200, serde_json::json!([]));
        reopened
            .handle(&owner(), operation_api::OperationRequest::Invoke(request))
            .await
            .unwrap();
        assert_eq!(egress.count(), 3);
    } else {
        assert!(reopened
            .inner
            .store
            .get(&address)
            .await
            .unwrap_err()
            .is_not_found());
        assert!(reopened
            .handle(&owner(), operation_api::OperationRequest::Invoke(request))
            .await
            .is_err());
        assert_eq!(egress.count(), 2);
    }
    reopened.shutdown().await;
}

#[tokio::test]
async fn actual_unknown_decision_without_durable_decision_aborts_on_reopen() {
    uncertain_decision_reopens(
        custody::JournalPhase::Decided,
        custody::JournalMoment::BeforeWrite,
    )
    .await;
}

#[tokio::test]
async fn actual_full_decision_followed_by_error_recovers_after_deadline_on_reopen() {
    uncertain_decision_reopens(
        custody::JournalPhase::Decided,
        custody::JournalMoment::AfterWrite,
    )
    .await;
}

#[tokio::test]
async fn actual_refresh_request_is_cancelled_at_its_original_egress_budget() {
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("old-access", Some("old-refresh"), 1);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(
        directory.path(),
        std::slice::from_ref(&config),
        egress.clone(),
        clock.clone(),
    )
    .await;
    let status = authorize(&backend).await;
    join_acquisition(&backend, &status.connect_session_ref).await;
    let request = invocation(&backend).await;
    clock.advance(1_001);
    *egress.hold_request.lock().unwrap() = Some(3);
    // Only Tokio's transport timeout is advanced here. The admitted receiver clock remains the
    // original RefreshClock; no caller deadline or new window is passed to the backend.
    tokio::time::pause();
    let worker = PersonalOAuthBackend {
        inner: backend.inner.clone(),
    };
    let attempt = tokio::spawn(async move {
        worker
            .handle(&owner(), operation_api::OperationRequest::Invoke(request))
            .await
    });
    egress.entered.notified().await;
    assert!(!egress.dropped.load(Ordering::SeqCst));
    tokio::time::advance(Duration::from_secs(30)).await;
    assert!(attempt.await.unwrap().is_err());
    assert!(
        egress.dropped.load(Ordering::SeqCst),
        "the real in-flight egress future was dropped"
    );
    tokio::time::resume();
    assert_eq!(
        egress.count(),
        3,
        "neither token-info nor operation dispatch follows timeout"
    );
    backend.shutdown().await;
    drop(backend);
    let reopened = open(directory.path(), &[config], egress.clone(), clock).await;
    assert!(reopened
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(invocation(&reopened).await)
        )
        .await
        .is_err());
    assert_eq!(
        egress.count(),
        3,
        "uncertain remote rotation blocks the old token across reopen"
    );
    reopened.shutdown().await;
}

#[tokio::test]
async fn actual_secret_commit_before_metadata_publication_recovers_on_reopen() {
    uncertain_decision_reopens(
        custody::JournalPhase::Published,
        custody::JournalMoment::BeforeWrite,
    )
    .await;
}

#[tokio::test]
async fn actual_metadata_publication_before_receipt_reclamation_recovers_on_reopen() {
    uncertain_decision_reopens(
        custody::JournalPhase::Published,
        custody::JournalMoment::AfterWrite,
    )
    .await;
}

#[tokio::test]
async fn requested_scope_ceiling_never_substitutes_for_observed_operation_scopes() {
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("scope-limited-access", Some("scope-limited-refresh"), 60);
    egress.info("fixture-client", 42, &["read_user"]);
    let backend = open(directory.path(), &[config], egress.clone(), clock).await;
    authorize(&backend).await;
    let operation_api::OperationResult::Describe(description) = backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Describe(operation_api::DescribeRequest {
                operation_ref: "gitlab-project-list".into(),
            }),
        )
        .await
        .unwrap()
    else {
        panic!("description")
    };
    assert!(description.connections.is_empty());
    assert!(backend
        .handle(
            &owner(),
            operation_api::OperationRequest::Invoke(invocation(&backend).await)
        )
        .await
        .is_err());
    assert_eq!(
        egress.count(),
        2,
        "requested read_api does not authorize an unobserved scope"
    );
    backend.shutdown().await;
}

#[tokio::test]
async fn actual_authority_revocation_before_completion_claim_aborts_prepared_credentials() {
    let directory = tempfile::tempdir().unwrap();
    let config = configuration();
    let egress = Arc::new(Egress::default());
    let clock = Arc::new(Clock::new());
    egress.token("revoked-access", Some("revoked-refresh"), 60);
    egress.info("fixture-client", 42, &["read_api"]);
    let backend = open(directory.path(), &[config], egress.clone(), clock).await;
    let authority = backend.inner.bindings[0].authority.clone();
    egress.after(2, move || {
        authority.lock().unwrap().active = false;
    });
    let status = deliver_code(&backend).await;
    join_acquisition(&backend, &status.connect_session_ref).await;
    let terminal = backend
        .inner
        .session_status(&status.connect_session_ref)
        .await
        .unwrap();
    assert!(terminal.state != connection_api::ConnectSessionState::Completed);
    assert!(terminal.browser_completion_url.is_none());
    assert!(backend
        .inner
        .store
        .get(&access_address(&backend))
        .await
        .unwrap_err()
        .is_not_found());
    assert_eq!(egress.count(), 2);
    backend.shutdown().await;
}

#[tokio::test]
async fn admitted_response_reference_is_the_actual_configured_backend_identity() {
    let root = tempfile::tempdir().unwrap();
    let configured = configuration();
    let expected = personal_oauth_admitted_connection_ref(&owner(), &configured).unwrap();
    let egress = Arc::new(Egress::default());
    let backend = PersonalOAuthBackend::open(
        owner(),
        std::slice::from_ref(&configured),
        root.path(),
        egress.clone(),
        true,
    )
    .await
    .unwrap();
    assert_eq!(
        backend.inner.bindings[0].custody.identity.connection,
        expected
    );
    let mut other = configured.clone();
    other.instance = Some("another-configured-instance".into());
    assert_ne!(
        personal_oauth_admitted_connection_ref(&owner(), &other).unwrap(),
        expected
    );
    other.oauth = None;
    assert!(personal_oauth_admitted_connection_ref(&owner(), &other).is_err());
    assert_eq!(egress.count(), 0);
    backend.shutdown().await;
}

#[path = "oauth_adversary_tests.rs"]
mod adversary;

#[path = "oauth_remediation_tests.rs"]
mod remediation;

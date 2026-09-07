use super::*;
use std::fs;
use std::sync::{Arc, Mutex};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse as _, Response};
use axum::routing::post;
use axum::{Json, Router};
use tokio::net::TcpListener;

const TOKEN: &[u8] = b"SENTINEL-NOT-A-REAL-PROVIDER-TOKEN\n";
const CAPABILITY: &str = "SENTINEL-NOT-A-REAL-SESSION-CAPABILITY";
const SESSION: &str = "connect-session:fixture";
const BASE_PATH: &str = "/application/api/connectors/v1";

fn owner() -> OwnerContext {
    OwnerContext {
        tenant_id: "tenant-fixture".into(),
        agent_id: "automation-fixture".into(),
        agent_revision: 1,
        authority_snapshot_id: "identity-fixture".into(),
        authority_snapshot_sha256: "a".repeat(64),
    }
}

fn created(base: &str, provider: &str) -> c::ConnectSessionStatus {
    c::ConnectSessionStatus {
        connect_session_ref: SESSION.into(),
        integration_ref: provider.into(),
        state: c::ConnectSessionState::Pending,
        expires_at_unix_ms: oauth_now().unwrap() + 300_000,
        completion_endpoint: None,
        browser_completion_url: Some(format!(
            "{base}/connect-sessions/{SESSION}#token={CAPABILITY}"
        )),
        connection_ref: None,
    }
}

fn secret_file(root: &Path) -> std::path::PathBuf {
    let path = root.join("credential");
    fs::write(&path, TOKEN).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    path
}

#[test]
fn file_checks_the_open_handle_owner_mode_type_and_byte_bound() {
    let root = tempfile::tempdir().unwrap();
    let path = secret_file(root.path());
    assert_eq!(read_credential_file(&path).unwrap().as_slice(), TOKEN);
    let metadata = fs::metadata(&path).unwrap();
    assert!(validate_file(&metadata, metadata.uid().wrapping_add(1)).is_err());
    for mode in [0o604, 0o640, 0o666] {
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        assert!(read_credential_file(&path).is_err());
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
    assert!(read_credential_file(&path).is_ok());
    let link = root.path().join("link");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(read_credential_file(&link).is_err());
    assert!(read_credential_file(root.path()).is_err());
    assert!(read_credential_file(&root.path().join("missing")).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    for bytes in [
        vec![],
        b" \n\t".to_vec(),
        vec![255],
        vec![b'x'; MAX_CREDENTIAL_BYTES + 1],
    ] {
        fs::write(&path, bytes).unwrap();
        assert!(read_credential_file(&path).is_err());
    }
    fs::write(&path, vec![b'x'; MAX_CREDENTIAL_BYTES]).unwrap();
    assert_eq!(
        read_credential_file(&path).unwrap().len(),
        MAX_CREDENTIAL_BYTES
    );
    let fifo = root.path().join("fifo");
    rustix::fs::mknodat(
        rustix::fs::CWD,
        &fifo,
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::RUSR,
        0,
    )
    .unwrap();
    assert!(read_credential_file(&fifo).is_err());
}

#[test]
fn completion_requires_the_exact_origin_prefix_route_session_and_live_capability() {
    let base = "https://connectors.example/application/api/connectors/v1";
    let parsed = Url::parse(base).unwrap();
    let status = created(base, "grafana");
    assert!(completion_endpoint(&parsed, status.clone()).is_ok());
    let valid = status.browser_completion_url.as_ref().unwrap();
    for url in [
        valid.replace("connectors.example", "foreign.example"),
        valid.replace("https:", "http:"),
        valid.replace("connectors.example", "connectors.example:444"),
        valid.replace("connectors.example", "user@connectors.example"),
        valid.replace(BASE_PATH, "/api/connectors/v1"),
        valid.replace("/connect-sessions/", "/oauth/"),
        valid.replace(SESSION, "connect-session:other"),
        valid.replace("#token=", "?redirect=1#token="),
        valid.replace("#token=", "#token=x#token="),
        valid.replace(CAPABILITY, "short"),
        valid.replace(CAPABILITY, &"x".repeat(257)),
        valid.replace("/connect-sessions/", "/other/../connect-sessions/"),
    ] {
        let mut status = status.clone();
        status.browser_completion_url = Some(url);
        assert!(matches!(
            completion_endpoint(&parsed, status),
            Err(ClientError::UnsafeHostedCompletion)
        ));
    }
    for case in 0..5 {
        let mut status = status.clone();
        match case {
            0 => status.expires_at_unix_ms = 1,
            1 => status.expires_at_unix_ms = oauth_now().unwrap() + MAX_SESSION_TTL_MS + 10_000,
            2 => status.completion_endpoint = Some("/private/socket".into()),
            3 => status.connection_ref = Some("connection:old".into()),
            _ => status.state = c::ConnectSessionState::Completed,
        }
        assert!(completion_endpoint(&parsed, status).is_err());
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Success,
    RefusedCreate,
    RefusedSubmit,
    Redirect,
    InvalidAck,
    CacheableAck,
    OversizedAck,
    Pending,
    WrongSession,
    WrongExpiry,
    WrongProvider,
    WrongConnection,
    WrongProfile,
    TenantConnection,
    Degraded,
    ForeignDestination,
}

struct Fixture {
    base: String,
    certificate: Vec<u8>,
    certificate_pem: String,
    provider: String,
    profile: String,
    expires: u64,
    case: Case,
    calls: Mutex<Vec<&'static str>>,
}

struct TlsListener {
    tcp: TcpListener,
    tls: tokio_rustls::TlsAcceptor,
}

impl axum::serve::Listener for TlsListener {
    type Io = tokio_rustls::server::TlsStream<tokio::net::TcpStream>;
    type Addr = std::net::SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (stream, address) = self.tcp.accept().await.unwrap();
            if let Ok(stream) = self.tls.accept(stream).await {
                return (stream, address);
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.tcp.local_addr()
    }
}

async fn control(State(state): State<Arc<Fixture>>, headers: HeaderMap, body: Bytes) -> Response {
    assert_eq!(
        headers.get("authorization").unwrap(),
        "Bearer identity-fixture"
    );
    assert!(headers.get("x-connect-session").is_none());
    assert!(!String::from_utf8_lossy(&body).contains(CAPABILITY));
    assert!(!String::from_utf8_lossy(&body).contains(std::str::from_utf8(TOKEN).unwrap().trim()));
    let request: c::RequestEnvelope = serde_json::from_slice(&body).unwrap();
    request.validate().unwrap();
    assert_eq!(request.context, owner());
    let mut status = created(&state.base, &state.provider);
    status.expires_at_unix_ms = state.expires;
    let result = match request.request {
        c::ConnectionRequest::ConnectSessionCreate(request) => {
            state.calls.lock().unwrap().push("create");
            assert_eq!(
                request.auth_profile.as_deref(),
                Some(state.profile.as_str())
            );
            assert_eq!(request.integration_ref, state.provider);
            if state.case == Case::RefusedCreate {
                return StatusCode::FORBIDDEN.into_response();
            }
            if state.case == Case::ForeignDestination {
                status.browser_completion_url = Some(format!(
                    "https://foreign.example/connect-sessions/{SESSION}#token={CAPABILITY}"
                ));
            }
            c::ConnectionResult::ConnectSessionCreate(status)
        }
        c::ConnectionRequest::ConnectSessionStatus(request) => {
            state.calls.lock().unwrap().push("status");
            assert_eq!(request.connect_session_ref, SESSION);
            status.browser_completion_url = None;
            status.state = c::ConnectSessionState::Completed;
            status.connection_ref = Some("connection:fixture".into());
            match state.case {
                Case::Pending => {
                    status.state = c::ConnectSessionState::Pending;
                    status.connection_ref = None;
                }
                Case::WrongSession => status.connect_session_ref = "connect-session:foreign".into(),
                Case::WrongExpiry => status.expires_at_unix_ms += 1,
                _ => {}
            }
            c::ConnectionResult::ConnectSessionStatus(status)
        }
        c::ConnectionRequest::Describe(request) => {
            state.calls.lock().unwrap().push("describe");
            assert_eq!(request.connection_ref, "connection:fixture");
            let mut summary = c::ConnectionSummary {
                connection_ref: request.connection_ref,
                integration_ref: state.provider.clone(),
                label: "Fixture connection".into(),
                state: c::ConnectionState::Callable,
                initiation: vec![c::ConnectionInitiator::Platform],
                route: c::ConnectionRoute::Direct,
                scope: Some(c::ConnectionScope::Principal),
                actor: Some(c::ConnectionActor::App),
                auth_profile: Some(state.profile.clone()),
            };
            match state.case {
                Case::WrongProvider => summary.integration_ref = "other-provider".into(),
                Case::WrongConnection => summary.connection_ref = "connection:foreign".into(),
                Case::WrongProfile => {
                    summary.auth_profile = Some(format!("{}.other", state.provider))
                }
                Case::TenantConnection => summary.scope = Some(c::ConnectionScope::Tenant),
                Case::Degraded => summary.state = c::ConnectionState::Degraded,
                _ => {}
            }
            c::ConnectionResult::Describe(c::ConnectionDescription {
                summary,
                channels: vec![],
            })
        }
        _ => panic!("unexpected control request"),
    };
    Json(c::ResponseEnvelope::success(request.request_id, result)).into_response()
}

async fn submit(State(state): State<Arc<Fixture>>, headers: HeaderMap, body: Bytes) -> Response {
    state.calls.lock().unwrap().push("submit");
    assert_eq!(headers.get("x-connect-session").unwrap(), CAPABILITY);
    assert_eq!(
        headers.get("content-type").unwrap(),
        "application/octet-stream"
    );
    assert!(headers.get("authorization").is_none());
    assert_eq!(body.as_ref(), TOKEN);
    let (status, body) = match state.case {
        Case::RefusedSubmit => (
            StatusCode::FORBIDDEN,
            String::from_utf8_lossy(TOKEN).into_owned(),
        ),
        Case::Redirect => (StatusCode::TEMPORARY_REDIRECT, String::new()),
        Case::InvalidAck => (
            StatusCode::OK,
            format!("{{\"accepted\":true,\"unexpected\":\"{CAPABILITY}\"}}"),
        ),
        Case::OversizedAck => (
            StatusCode::OK,
            "x".repeat(MAX_COMPLETION_RESPONSE_BYTES + 1),
        ),
        _ => (StatusCode::OK, "{\"accepted\":true}".into()),
    };
    let mut response = (status, body).into_response();
    if state.case != Case::CacheableAck {
        response
            .headers_mut()
            .insert("cache-control", "no-store".parse().unwrap());
    }
    response.headers_mut().insert(
        "location",
        format!("{}/redirected", state.base).parse().unwrap(),
    );
    response
}

async fn fixture(
    case: Case,
    provider: &str,
    credential: &str,
) -> (Arc<Fixture>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let rcgen::CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
    let tls = tokio_rustls::rustls::ServerConfig::builder_with_provider(Arc::new(
        tokio_rustls::rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![cert.der().clone()],
        tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer::from(key_pair.serialize_der()).into(),
    )
    .unwrap();
    let state = Arc::new(Fixture {
        base: format!("https://{}{BASE_PATH}", listener.local_addr().unwrap()),
        certificate: cert.der().to_vec(),
        certificate_pem: cert.pem(),
        provider: provider.into(),
        profile: format!("{provider}.{credential}"),
        expires: oauth_now().unwrap() + 300_000,
        case,
        calls: Mutex::new(vec![]),
    });
    let app = Router::new()
        .route(&format!("{BASE_PATH}/connections"), post(control))
        .route(
            &format!("{BASE_PATH}/connect-sessions/{SESSION}"),
            post(submit),
        )
        .route(
            &format!("{BASE_PATH}/redirected"),
            post(|State(state): State<Arc<Fixture>>| async move {
                state.calls.lock().unwrap().push("redirected");
            }),
        )
        .with_state(Arc::clone(&state));
    let task = tokio::spawn(async move {
        axum::serve(
            TlsListener {
                tcp: listener,
                tls: tokio_rustls::TlsAcceptor::from(Arc::new(tls)),
            },
            app,
        )
        .await
        .unwrap();
    });
    (state, task)
}

async fn run(state: &Fixture, path: &Path) -> Result<c::ConnectionDescription, ClientError> {
    let http = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .add_root_certificate(reqwest::Certificate::from_der(&state.certificate).unwrap())
        .build()
        .unwrap();
    HostedClient::from_parts(Url::parse(&state.base).unwrap(), http)
        .connect_with_credential_file(
            "identity-fixture",
            &owner(),
            c::ConnectSessionCreateRequest {
                integration_ref: state.provider.clone(),
                label: "Fixture connection".into(),
                auth_profile: Some(state.profile.clone()),
            },
            path,
        )
        .await
}

#[tokio::test]
async fn native_trust_uses_ssl_cert_file_without_disabling_tls() {
    if let Ok(base) = std::env::var("CONNECTORS_TEST_CA_BASE") {
        let path = std::env::var("CONNECTORS_TEST_CREDENTIAL_FILE").unwrap();
        let result = HostedClient::new(&base)
            .unwrap()
            .connect_with_credential_file(
                "identity-fixture",
                &owner(),
                c::ConnectSessionCreateRequest {
                    integration_ref: "grafana".into(),
                    label: "Fixture connection".into(),
                    auth_profile: Some("grafana.service_account_token".into()),
                },
                Path::new(&path),
            )
            .await;
        if std::env::var("CONNECTORS_TEST_EXPECT_CA").unwrap() == "trusted" {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(matches!(result, Err(ClientError::HostedUnavailable)));
        }
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let path = secret_file(root.path());
    let ca = root.path().join("ca.pem");
    let empty_ca = root.path().join("empty.pem");
    let empty_ca_dir = root.path().join("empty-certificates");
    fs::create_dir(&empty_ca_dir).unwrap();
    let (state, task) = fixture(Case::Success, "grafana", "service_account_token").await;
    fs::write(&ca, &state.certificate_pem).unwrap();
    fs::write(&empty_ca, "").unwrap();
    for (trust, bundle) in [("untrusted", empty_ca), ("trusted", ca)] {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "hosted_connect::tests::native_trust_uses_ssl_cert_file_without_disabling_tls",
                "--nocapture",
            ])
            .env("CONNECTORS_TEST_CA_BASE", &state.base)
            .env("CONNECTORS_TEST_CREDENTIAL_FILE", &path)
            .env("CONNECTORS_TEST_EXPECT_CA", trust)
            .env("SSL_CERT_FILE", bundle)
            .env("SSL_CERT_DIR", &empty_ca_dir);
        let output = tokio::task::spawn_blocking(move || command.output().unwrap())
            .await
            .unwrap();
        assert!(output.status.success(), "{output:?}");
    }
    assert_eq!(
        *state.calls.lock().unwrap(),
        ["create", "submit", "status", "describe"]
    );
    task.abort();
}

#[tokio::test]
async fn token_providers_use_identical_http_flow_and_only_confirm_matching_principal_connections() {
    for (provider, credential) in [
        ("grafana", "service_account_token"),
        ("anthropic", "api_key"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = secret_file(root.path());
        let (state, task) = fixture(Case::Success, provider, credential).await;
        let description = run(&state, &path).await.unwrap();
        assert_eq!(
            description.summary.auth_profile.as_deref(),
            Some(state.profile.as_str())
        );
        assert_eq!(
            *state.calls.lock().unwrap(),
            ["create", "submit", "status", "describe"]
        );
        let result = serde_json::to_string(&description).unwrap();
        assert!(!result.contains(CAPABILITY));
        assert!(!result.contains(std::str::from_utf8(TOKEN).unwrap().trim()));
        assert_eq!(fs::read(&path).unwrap(), TOKEN);
        task.abort();
    }
}

#[tokio::test]
async fn refusal_uncertainty_and_mismatched_status_never_resubmit_or_report_success() {
    for case in [
        Case::RefusedCreate,
        Case::RefusedSubmit,
        Case::Redirect,
        Case::InvalidAck,
        Case::CacheableAck,
        Case::OversizedAck,
        Case::Pending,
        Case::WrongSession,
        Case::WrongExpiry,
        Case::WrongProvider,
        Case::WrongConnection,
        Case::WrongProfile,
        Case::TenantConnection,
        Case::Degraded,
        Case::ForeignDestination,
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = secret_file(root.path());
        let (state, task) = fixture(case, "grafana", "service_account_token").await;
        let error = run(&state, &path).await.unwrap_err();
        let calls = state.calls.lock().unwrap().clone();
        assert_eq!(calls.iter().filter(|call| **call == "create").count(), 1);
        assert_eq!(
            calls.iter().filter(|call| **call == "submit").count(),
            usize::from(!matches!(
                case,
                Case::RefusedCreate | Case::ForeignDestination
            ))
        );
        assert!(!calls.contains(&"redirected"));
        if !matches!(
            case,
            Case::RefusedCreate | Case::RefusedSubmit | Case::ForeignDestination
        ) {
            assert!(matches!(error, ClientError::CompletionUnconfirmed));
        }
        let diagnostic = format!("{error:?}: {error}");
        assert!(!diagnostic.contains(CAPABILITY));
        assert!(!diagnostic.contains(std::str::from_utf8(TOKEN).unwrap().trim()));
        task.abort();
    }
}

#[tokio::test]
async fn unsafe_files_refuse_before_creating_a_session() {
    let root = tempfile::tempdir().unwrap();
    let path = secret_file(root.path());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let (state, task) = fixture(Case::Success, "grafana", "service_account_token").await;
    assert!(matches!(
        run(&state, &path).await,
        Err(ClientError::UnsafeCredentialFile)
    ));
    assert!(state.calls.lock().unwrap().is_empty());
    task.abort();
}

#[tokio::test]
async fn a_dropped_submit_response_is_uncertain_and_is_never_replayed() {
    use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, BufReader};
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}{BASE_PATH}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut reader = BufReader::new(stream);
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse::<usize>().unwrap();
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).await.unwrap();
        assert_eq!(body, TOKEN);
        drop(reader);
        assert!(
            tokio::time::timeout(Duration::from_millis(250), listener.accept())
                .await
                .is_err()
        );
    });
    let result = HostedClient::new(&base)
        .unwrap()
        .complete_connect_session(created(&base, "grafana"), Zeroizing::new(TOKEN.to_vec()))
        .await;
    assert!(matches!(result, Err(ClientError::CompletionUnconfirmed)));
    server.await.unwrap();
}

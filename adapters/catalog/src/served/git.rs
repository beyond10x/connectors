//! The git smart-HTTP relay of a served instance (docs/running-services.md,
//! "Git relay"; story:git-smart-http-relay-with-quarantine).
//!
//! A caller with no forge credential clones, fetches and pushes through
//! `/git/<owner>/<repo>.git/{info/refs,git-upload-pack,git-receive-pack}`. The
//! instance sends each request on to the forge with the token of its own
//! profile, which never leaves the instance. Every route first admits its
//! caller under the host's profile, then refuses a repository the
//! configuration does not list as `404`, before any forge request.
//!
//! upload-pack (clone and fetch) and both advertisements are relayed as they
//! are. A push (receive-pack) is read whole, up to `push_limit_bytes`, and
//! meets these refusals, in this order, before anything is sent to the
//! forge's receive-pack:
//!
//! 1. the ref policy: every command updates `refs/heads/<prefix>...` for a
//!    configured prefix; a delete, a tag, any other ref, a shallow or signed
//!    push are refused, with no forge request at all;
//! 2. quarantine: the instance fetches the repository into its own mirror,
//!    indexes the pack into a new repository for this request whose objects
//!    borrow the mirror's, and requires every new object to be present;
//! 3. `commit_identity`, when configured: the author and committer of every
//!    new commit;
//! 4. `pre_push_check`, when configured: the command runs in the quarantine
//!    with the commands on its standard input and no credential in its
//!    environment, and must exit 0.
//!
//! Only then is the original request sent to the forge's receive-pack, once.
//! The old object id of each command is the forge's own compare-and-swap. Its
//! report is relayed to the caller unchanged, and each command is logged as
//! `applied`, `refused` or `unknown`. A push whose answer is lost is
//! `unknown`, the caller is told so, and it is never sent again.
use super::{Repository, Upstream};
use axum::{
    Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Path, RawQuery, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use connectors_core::{Error, ErrorCode, Result};
use connectors_host::server::Admission;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read as _,
    path::{Path as FsPath, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    process::Command,
    sync::Semaphore,
};
use zeroize::Zeroizing;

/// The default and the largest bound of a buffered push.
const DEFAULT_PUSH_LIMIT: u64 = 100 * 1024 * 1024;
const MAX_PUSH_LIMIT: u64 = 1024 * 1024 * 1024;
/// The largest upload-pack request (wants, haves and options) relayed.
const FETCH_REQUEST_LIMIT: usize = 16 * 1024 * 1024;
/// The largest forge answer to a push that is read and relayed.
const REPORT_LIMIT: usize = 1024 * 1024;
/// Pushes the instance works on at once; one more is `unavailable`.
const CONCURRENT_PUSHES: usize = 4;
/// The deadline of each git command the instance runs for a push.
const GIT_DEADLINE: Duration = Duration::from_secs(300);
const DEFAULT_CHECK_SECONDS: u64 = 300;
/// The bytes of the pre-push check's standard error kept for the log.
const CHECK_LOG_LIMIT: usize = 4096;
/// The user name a forge token travels under in HTTP basic authentication.
const TOKEN_USER: &str = "x-access-token";
const RECEIVE_REQUEST: &str = "application/x-git-receive-pack-request";
const RECEIVE_RESULT: &str = "application/x-git-receive-pack-result";
const UPLOAD_REQUEST: &str = "application/x-git-upload-pack-request";
const UPLOAD_RESULT: &str = "application/x-git-upload-pack-result";

/// The `git` member of a served configuration.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitConfig {
    /// The forge's git base, such as `https://github.com`: a repository is
    /// `<upstream>/<owner>/<repo>.git`.
    upstream: String,
    /// The CA bundle that alone roots the forge's TLS; the system roots
    /// otherwise.
    #[serde(default)]
    ca_file: Option<PathBuf>,
    /// The repositories relayed, as `<owner>/<repo>`.
    repositories: Vec<String>,
    /// The branch prefixes a push may update: `agent/` admits
    /// `refs/heads/agent/<name>`.
    ref_prefixes: Vec<String>,
    /// The largest push read, in bytes.
    #[serde(default = "default_push_limit")]
    push_limit_bytes: u64,
    /// The author and committer every new commit must carry.
    #[serde(default)]
    commit_identity: Option<CommitIdentity>,
    /// The command every push must pass.
    #[serde(default)]
    pre_push_check: Option<PrePushCheck>,
    /// A private directory the instance owns: the mirrors and the
    /// per-request quarantines.
    work_directory: PathBuf,
}

fn default_push_limit() -> u64 {
    DEFAULT_PUSH_LIMIT
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommitIdentity {
    name: String,
    email: String,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrePushCheck {
    /// The program and its arguments; the program is a path or a name on
    /// the instance's `PATH`.
    command: Vec<String>,
    #[serde(default = "default_check_seconds")]
    timeout_seconds: u64,
}

fn default_check_seconds() -> u64 {
    DEFAULT_CHECK_SECONDS
}

impl Repository {
    fn path(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

/// The relay of one served instance.
pub struct Relay {
    upstream: url::Url,
    ca_file: Option<PathBuf>,
    client: reqwest::Client,
    repositories: Vec<Repository>,
    prefixes: Vec<String>,
    push_limit: usize,
    identity: Option<CommitIdentity>,
    check: Option<PrePushCheck>,
    work: PathBuf,
    git: PathBuf,
    path: std::ffi::OsString,
    token: Arc<Upstream>,
    pushes: Arc<Semaphore>,
    /// One lock per repository: its mirror is fetched and read by one push
    /// at a time.
    mirrors: Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

fn invalid(message: &str) -> Error {
    Error::invalid(format!("git: {message}"))
}

/// A ref name git itself would accept (git-check-ref-format), restricted to
/// printable ASCII.
fn valid_ref(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 1024
        && !name.ends_with('/')
        && !name.ends_with('.')
        && !name.contains("..")
        && !name.contains("@{")
        && !name.contains("//")
        && name.split('/').all(|component| {
            !component.is_empty() && !component.starts_with('.') && !component.ends_with(".lock")
        })
        && name.bytes().all(|b| {
            b.is_ascii_graphic() && !matches!(b, b'~' | b'^' | b':' | b'?' | b'*' | b'[' | b'\\')
        })
}

/// The absolute path of `program` on `path`.
fn locate(program: &str, path: &std::ffi::OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .map(|directory| directory.join(program))
        .find(|candidate| {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::metadata(candidate).is_ok_and(|metadata| {
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            })
        })
}

impl Relay {
    /// Checks the configuration and prepares the work directory. Start is
    /// refused when the profile has no forge token, when `git` is not on
    /// `PATH`, or when any member is out of bounds.
    pub fn new(config: GitConfig, token: Arc<Upstream>) -> Result<Self> {
        let upstream = url::Url::parse(config.upstream.trim_end_matches('/'))
            .map_err(|_| invalid("upstream must be an https URL"))?;
        if upstream.scheme() != "https"
            || upstream.host_str().is_none()
            || !upstream.username().is_empty()
            || upstream.password().is_some()
            || upstream.query().is_some()
            || upstream.fragment().is_some()
        {
            return Err(invalid(
                "upstream must be an https URL without credentials, query or fragment",
            ));
        }
        let repositories = super::repositories(&config.repositories).map_err(invalid)?;
        if config.ref_prefixes.is_empty()
            || !config
                .ref_prefixes
                .iter()
                .all(|prefix| prefix.ends_with('/') && valid_ref(&format!("refs/heads/{prefix}x")))
        {
            return Err(invalid(
                "ref_prefixes lists at least one branch prefix ending in /",
            ));
        }
        if config.push_limit_bytes == 0 || config.push_limit_bytes > MAX_PUSH_LIMIT {
            return Err(invalid("push_limit_bytes is between 1 and 1 GiB"));
        }
        if let Some(identity) = &config.commit_identity {
            let printable = |text: &str| {
                !text.is_empty() && text.len() <= 256 && !text.chars().any(char::is_control)
            };
            if !printable(&identity.name) || !printable(&identity.email) {
                return Err(invalid("commit_identity has a name and an email"));
            }
        }
        if let Some(check) = &config.pre_push_check
            && (check.command.is_empty()
                || check.command[0].is_empty()
                || check.timeout_seconds == 0
                || check.timeout_seconds > 3600)
        {
            return Err(invalid(
                "pre_push_check has a command and a timeout of 1 to 3600 seconds",
            ));
        }
        if !token.local.has_upstream_token() {
            return Err(invalid(
                "the instance's profile has no token to send to the forge",
            ));
        }
        let path = std::env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin".into());
        let git = locate("git", &path).ok_or_else(|| invalid("git is not on PATH"))?;
        if !config.work_directory.is_absolute() {
            return Err(invalid("work_directory is an absolute path"));
        }
        private_directory(&config.work_directory)?;
        // `ca-path` stays empty: with `ca_file` set, it replaces any CA
        // directory git's TLS library would otherwise trust as well.
        for directory in ["mirrors", "quarantine", "ca-path"] {
            private_directory(&config.work_directory.join(directory))?;
        }
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut client = reqwest::Client::builder()
            .user_agent(concat!("git/connectors-", env!("CARGO_PKG_VERSION")))
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(120));
        if let Some(ca) = &config.ca_file {
            let bytes = std::fs::read(ca).map_err(|_| invalid("ca_file cannot be read"))?;
            let certificates = reqwest::Certificate::from_pem_bundle(&bytes)
                .map_err(|_| invalid("ca_file holds no certificate"))?;
            if certificates.is_empty() {
                return Err(invalid("ca_file holds no certificate"));
            }
            client = client.tls_certs_only(certificates);
        }
        Ok(Self {
            upstream,
            ca_file: config.ca_file,
            client: client.build().map_err(|_| Error::internal())?,
            repositories,
            prefixes: config.ref_prefixes,
            push_limit: usize::try_from(config.push_limit_bytes).unwrap_or(usize::MAX),
            identity: config.commit_identity,
            check: config.pre_push_check,
            work: config.work_directory,
            git,
            path,
            token,
            pushes: Arc::new(Semaphore::new(CONCURRENT_PUSHES)),
            mirrors: Mutex::default(),
        })
    }

    /// The configured repository `owner`/`repo` names ([`super::listed`]).
    fn repository(&self, owner: &str, repo: &str) -> Option<Repository> {
        super::listed(&self.repositories, owner, repo).cloned()
    }

    fn url(&self, repository: &Repository, suffix: &str) -> String {
        format!(
            "{}/{}/{}.git{suffix}",
            self.upstream.as_str().trim_end_matches('/'),
            repository.owner,
            repository.name
        )
    }

    /// `Authorization: Basic` for the forge, built from the token now.
    async fn authorization(&self) -> Result<HeaderValue> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let token = self.token.token().await?;
        let mut pair = Zeroizing::new(Vec::with_capacity(token.0.len() + 16));
        pair.extend_from_slice(TOKEN_USER.as_bytes());
        pair.push(b':');
        pair.extend_from_slice(&token.0);
        let value = Zeroizing::new(format!("Basic {}", STANDARD.encode(pair.as_slice())));
        let mut header = HeaderValue::from_str(&value).map_err(|_| {
            Error::new(
                ErrorCode::Unavailable,
                "the instance's token cannot be sent",
            )
        })?;
        header.set_sensitive(true);
        Ok(header)
    }
}

fn private_directory(path: &FsPath) -> Result<()> {
    use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .map_err(|_| invalid("work_directory cannot be prepared"))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(|_| invalid("work_directory cannot be prepared"))
}

/// A process group, killed whole when dropped.
struct ProcessGroup(i32);

impl Drop for ProcessGroup {
    fn drop(&mut self) {
        // SAFETY: kill(2) takes plain integers and touches no memory; the
        // negative id names the group the check was started in.
        unsafe {
            libc::kill(-self.0, libc::SIGKILL);
        }
    }
}

#[derive(Clone)]
struct Routes {
    relay: Arc<Relay>,
    admission: Admission,
}

/// The relay's routes, admitted under the host's `admission`.
pub fn routes(relay: Arc<Relay>, admission: Admission) -> Router {
    Router::new()
        .route("/git/{owner}/{repo}/info/refs", get(advertise))
        .route("/git/{owner}/{repo}/git-upload-pack", post(upload_pack))
        .route("/git/{owner}/{repo}/git-receive-pack", post(receive_pack))
        .layer(DefaultBodyLimit::disable())
        .with_state(Routes { relay, admission })
}

fn text(status: StatusCode, message: &str) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        format!("{message}\n"),
    )
        .into_response()
}

fn refusal(error: &Error) -> Response {
    let status = match error.code {
        ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
        ErrorCode::Forbidden => StatusCode::FORBIDDEN,
        ErrorCode::NotFound => StatusCode::NOT_FOUND,
        ErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
        ErrorCode::Capacity => StatusCode::PAYLOAD_TOO_LARGE,
        ErrorCode::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        ErrorCode::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        ErrorCode::Timeout => StatusCode::GATEWAY_TIMEOUT,
        _ => StatusCode::BAD_GATEWAY,
    };
    text(status, &error.message)
}

/// Admission, then the repository: the two refusals every route answers
/// before it reads the body or sends anything to the forge.
async fn admitted(
    routes: &Routes,
    headers: &HeaderMap,
    owner: &str,
    repo: &str,
) -> std::result::Result<(String, Repository), Box<Response>> {
    let principal = routes
        .admission
        .admit(headers)
        .await
        .map_err(|error| Box::new(refusal(&error)))?;
    let repository = routes
        .relay
        .repository(owner, repo)
        .ok_or_else(|| Box::new(text(StatusCode::NOT_FOUND, "repository not found")))?;
    Ok((principal, repository))
}

/// The forge's answer to a relayed read, streamed to the caller when it is
/// a success of the expected type.
fn relayed(answer: reqwest::Response, content_type: &str) -> Response {
    let status = answer.status().as_u16();
    if status != 200 {
        tracing::warn!(status, "git relay: the forge refused a read");
        return match status {
            404 => text(StatusCode::NOT_FOUND, "repository not found"),
            401 | 403 => text(
                StatusCode::BAD_GATEWAY,
                "the forge refused the instance's credential",
            ),
            429 => text(StatusCode::TOO_MANY_REQUESTS, "the forge is rate limited"),
            _ => text(StatusCode::BAD_GATEWAY, "the forge did not answer"),
        };
    }
    let actual = answer
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if actual != content_type {
        return text(
            StatusCode::BAD_GATEWAY,
            "the forge did not answer with the smart protocol",
        );
    }
    let mut response = Response::new(Body::from_stream(answer.bytes_stream()));
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type).unwrap_or(HeaderValue::from_static("")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache, max-age=0, must-revalidate"),
    );
    response
}

/// Copies the caller's protocol header and git agent onto a forge request.
fn forwarded(
    request: reqwest::RequestBuilder,
    headers: &HeaderMap,
    names: &[header::HeaderName],
) -> reqwest::RequestBuilder {
    let mut request = request;
    for name in names {
        if let Some(value) = headers.get(name)
            && value.len() <= 1024
        {
            request = request.header(name.clone(), value.clone());
        }
    }
    if let Some(agent) = headers.get(header::USER_AGENT)
        && agent.len() <= 256
        && agent.as_bytes().starts_with(b"git/")
    {
        request = request.header(header::USER_AGENT, agent.clone());
    }
    request
}

fn git_protocol() -> header::HeaderName {
    header::HeaderName::from_static("git-protocol")
}

async fn advertise(
    State(routes): State<Routes>,
    Path((owner, repo)): Path<(String, String)>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> Response {
    let (_, repository) = match admitted(&routes, &headers, &owner, &repo).await {
        Ok(admitted) => admitted,
        Err(response) => return *response,
    };
    let service = match query.as_deref() {
        Some("service=git-upload-pack") => "git-upload-pack",
        Some("service=git-receive-pack") => "git-receive-pack",
        _ => {
            return text(
                StatusCode::FORBIDDEN,
                "the relay serves the smart protocol only",
            );
        }
    };
    let relay = &routes.relay;
    let authorization = match relay.authorization().await {
        Ok(value) => value,
        Err(error) => return refusal(&error),
    };
    let request = relay
        .client
        .get(relay.url(&repository, &format!("/info/refs?service={service}")))
        .header(header::AUTHORIZATION, authorization);
    let request = forwarded(request, &headers, &[git_protocol()]);
    match request.send().await {
        Ok(answer) => relayed(answer, &format!("application/x-{service}-advertisement")),
        Err(_) => text(StatusCode::BAD_GATEWAY, "the forge did not answer"),
    }
}

async fn upload_pack(
    State(routes): State<Routes>,
    Path((owner, repo)): Path<(String, String)>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let (_, repository) = match admitted(&routes, &headers, &owner, &repo).await {
        Ok(admitted) => admitted,
        Err(response) => return *response,
    };
    if !content_type_is(&headers, UPLOAD_REQUEST) {
        return text(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "not an upload-pack request",
        );
    }
    let encoding = headers.get(header::CONTENT_ENCODING);
    if encoding.is_some_and(|value| !matches!(value.as_bytes(), b"gzip" | b"x-gzip" | b"identity"))
    {
        return text(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported content encoding",
        );
    }
    let Ok(body) = axum::body::to_bytes(body, FETCH_REQUEST_LIMIT).await else {
        return text(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the request exceeds the relay's bound",
        );
    };
    let relay = &routes.relay;
    let authorization = match relay.authorization().await {
        Ok(value) => value,
        Err(error) => return refusal(&error),
    };
    let request = relay
        .client
        .post(relay.url(&repository, "/git-upload-pack"))
        .header(header::AUTHORIZATION, authorization)
        .header(header::CONTENT_TYPE, UPLOAD_REQUEST)
        .header(header::ACCEPT, UPLOAD_RESULT)
        .body(body);
    let request = forwarded(
        request,
        &headers,
        &[git_protocol(), header::CONTENT_ENCODING],
    );
    match request.send().await {
        Ok(answer) => relayed(answer, UPLOAD_RESULT),
        Err(_) => text(StatusCode::BAD_GATEWAY, "the forge did not answer"),
    }
}

fn content_type_is(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .is_some_and(|value| value.as_bytes() == expected.as_bytes())
}

async fn receive_pack(
    State(routes): State<Routes>,
    Path((owner, repo)): Path<(String, String)>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let (principal, repository) = match admitted(&routes, &headers, &owner, &repo).await {
        Ok(admitted) => admitted,
        Err(response) => return *response,
    };
    if !content_type_is(&headers, RECEIVE_REQUEST) {
        return text(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "not a receive-pack request",
        );
    }
    let relay = &routes.relay;
    let Ok(slot) = relay.pushes.clone().try_acquire_owned() else {
        return text(
            StatusCode::SERVICE_UNAVAILABLE,
            "the relay is busy; push again",
        );
    };
    // Refusal: a push over the bound is refused before any forge request.
    let bound = relay.push_limit;
    let Ok(raw) = axum::body::to_bytes(body, bound).await else {
        tracing::info!(repository = %repository.path(), "git push refused: over the bound");
        return text(
            StatusCode::PAYLOAD_TOO_LARGE,
            "the push exceeds the relay's bound",
        );
    };
    let request = match headers
        .get(header::CONTENT_ENCODING)
        .map(HeaderValue::as_bytes)
    {
        None | Some(b"identity") => raw,
        Some(b"gzip" | b"x-gzip") => match inflate(&raw, bound) {
            Some(inflated) => inflated,
            None => {
                return text(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "the push exceeds the relay's bound",
                );
            }
        },
        Some(_) => {
            return text(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported content encoding",
            );
        }
    };
    // The push runs to its end in its own task: a caller that hangs up
    // cannot cancel it between the forge applying it and the outcome log.
    let relay = routes.relay.clone();
    tokio::spawn(async move {
        let _slot = slot;
        relay.push(&principal, &repository, request).await
    })
    .await
    .unwrap_or_else(|_| text(StatusCode::INTERNAL_SERVER_ERROR, "internal error"))
}

/// The gzip body inflated, or `None` when it is invalid or larger than `bound`.
fn inflate(body: &[u8], bound: usize) -> Option<Bytes> {
    let mut inflated = Vec::new();
    flate2::read::GzDecoder::new(body)
        .take(bound as u64 + 1)
        .read_to_end(&mut inflated)
        .ok()?;
    (inflated.len() <= bound).then(|| Bytes::from(inflated))
}

// ---- the push --------------------------------------------------------------

/// One ref update of a push.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Update {
    old: String,
    new: String,
    name: String,
}

/// A receive-pack request: its commands, the capabilities the client chose,
/// and where its pack starts.
#[derive(Debug)]
struct PushRequest {
    updates: Vec<Update>,
    capabilities: Vec<String>,
    pack: usize,
    /// The push carries push options, which the relay does not forward.
    options: bool,
}

impl PushRequest {
    fn has(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }
}

/// A pkt-line: data, or one of the special packets.
enum Packet<'a> {
    Data(&'a [u8]),
    Flush,
}

/// The pkt-line at `*at`, advancing past it.
fn packet<'a>(bytes: &'a [u8], at: &mut usize) -> Option<Packet<'a>> {
    let head = bytes.get(*at..*at + 4)?;
    let length = usize::from_str_radix(std::str::from_utf8(head).ok()?, 16).ok()?;
    match length {
        0 => {
            *at += 4;
            Some(Packet::Flush)
        }
        1..=4 => None,
        _ => {
            let data = bytes.get(*at + 4..*at + length)?;
            *at += length;
            Some(Packet::Data(data))
        }
    }
}

fn object_id(text: &str) -> bool {
    matches!(text.len(), 40 | 64)
        && text
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Parses the command list (and push options) of a receive-pack request.
fn parse(bytes: &[u8]) -> std::result::Result<PushRequest, &'static str> {
    let mut at = 0;
    let mut updates = Vec::new();
    let mut capabilities = Vec::new();
    loop {
        match packet(bytes, &mut at).ok_or("the push request is not a command list")? {
            Packet::Flush => break,
            Packet::Data(data) => {
                let data = data.strip_suffix(b"\n").unwrap_or(data);
                let (line, caps) = match data.iter().position(|b| *b == 0) {
                    Some(nul) => (&data[..nul], Some(&data[nul + 1..])),
                    None => (data, None),
                };
                let line = std::str::from_utf8(line).map_err(|_| "a command is not text")?;
                if line.starts_with("shallow ") {
                    return Err("a shallow push is not relayed");
                }
                if line.starts_with("push-cert") {
                    return Err("a signed push is not relayed");
                }
                if let Some(caps) = caps {
                    if !updates.is_empty() {
                        return Err("capabilities follow the first command only");
                    }
                    let caps =
                        std::str::from_utf8(caps).map_err(|_| "capabilities are not text")?;
                    capabilities = caps
                        .split(' ')
                        .filter(|c| !c.is_empty())
                        .map(str::to_owned)
                        .collect();
                }
                let mut words = line.splitn(3, ' ');
                let (Some(old), Some(new), Some(name)) = (words.next(), words.next(), words.next())
                else {
                    return Err("a command is not <old> <new> <ref>");
                };
                if !object_id(old) || !object_id(new) || old.len() != new.len() {
                    return Err("a command is not <old> <new> <ref>");
                }
                updates.push(Update {
                    old: old.to_owned(),
                    new: new.to_owned(),
                    name: name.to_owned(),
                });
            }
        }
    }
    let mut request = PushRequest {
        updates,
        capabilities,
        pack: at,
        options: false,
    };
    if request.has("push-options") && !request.updates.is_empty() {
        loop {
            match packet(bytes, &mut at).ok_or("the push options are not pkt-lines")? {
                Packet::Flush => break,
                Packet::Data(_) => request.options = true,
            }
        }
        request.pack = at;
    }
    Ok(request)
}

fn zero(id: &str) -> bool {
    id.bytes().all(|b| b == b'0')
}

/// What happened to one command at the forge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Applied,
    Refused,
    Unknown,
}

impl Outcome {
    fn word(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Refused => "refused",
            Self::Unknown => "unknown",
        }
    }
}

/// A refusal the relay itself answers, for every command of the push.
struct Refused {
    reason: String,
    unpack: &'static str,
}

impl Refused {
    fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            unpack: "ok",
        }
    }
}

fn pkt(data: &[u8]) -> Vec<u8> {
    let mut line = format!("{:04x}", data.len() + 4).into_bytes();
    line.extend_from_slice(data);
    line
}

/// The report-status (or the plain refusal) a git client shows for a push
/// the relay answers itself: `ng <ref> <reason>` for every command.
fn report(request: &PushRequest, unpack: &str, reason: &str) -> Response {
    if !request.has("report-status") && !request.has("report-status-v2") {
        return text(StatusCode::FORBIDDEN, reason);
    }
    let reason: String = reason
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(512)
        .collect();
    let mut inner = pkt(format!("unpack {unpack}\n").as_bytes());
    for update in &request.updates {
        inner.extend(pkt(format!("ng {} {reason}\n", update.name).as_bytes()));
    }
    inner.extend_from_slice(b"0000");
    let body = if request.has("side-band-64k") || request.has("side-band") {
        let size = if request.has("side-band-64k") {
            65515
        } else {
            995
        };
        let mut banded = Vec::new();
        for chunk in inner.chunks(size) {
            let mut data = vec![1_u8];
            data.extend_from_slice(chunk);
            banded.extend(pkt(&data));
        }
        banded.extend_from_slice(b"0000");
        banded
    } else {
        inner
    };
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, RECEIVE_RESULT),
            (
                header::CACHE_CONTROL,
                "no-cache, max-age=0, must-revalidate",
            ),
        ],
        body,
    )
        .into_response()
}

/// The per-command outcomes in a forge's receive-pack answer; a command the
/// answer does not name is `unknown`.
fn outcomes(request: &PushRequest, answer: &[u8]) -> Vec<Outcome> {
    let mut report = Vec::new();
    let banded = request.has("side-band-64k") || request.has("side-band");
    let mut at = 0;
    if banded {
        while let Some(packet) = packet(answer, &mut at) {
            match packet {
                Packet::Data(data) if data.first() == Some(&1) => {
                    report.extend_from_slice(&data[1..])
                }
                Packet::Data(_) => {}
                Packet::Flush => break,
            }
        }
    } else {
        report.extend_from_slice(answer);
    }
    let mut statuses: BTreeMap<String, Outcome> = BTreeMap::new();
    let mut unpacked = false;
    let mut at = 0;
    while let Some(Packet::Data(line)) = packet(&report, &mut at) {
        let line = String::from_utf8_lossy(line.strip_suffix(b"\n").unwrap_or(line)).into_owned();
        if line == "unpack ok" {
            unpacked = true;
        } else if let Some(name) = line.strip_prefix("ok ") {
            statuses.insert(name.to_owned(), Outcome::Applied);
        } else if let Some(rest) = line.strip_prefix("ng ") {
            let name = rest.split_once(' ').map_or(rest, |(name, _)| name);
            statuses.insert(name.to_owned(), Outcome::Refused);
        }
    }
    request
        .updates
        .iter()
        .map(|update| match statuses.get(&update.name) {
            Some(Outcome::Applied) if unpacked => Outcome::Applied,
            Some(outcome) if *outcome != Outcome::Applied => *outcome,
            _ => Outcome::Unknown,
        })
        .collect()
}

/// The commands' lines on a check's standard input: `<old> <new> <ref>`.
fn command_lines(updates: &[Update]) -> Vec<u8> {
    updates
        .iter()
        .map(|u| format!("{} {} {}\n", u.old, u.new, u.name))
        .collect::<String>()
        .into_bytes()
}

impl Relay {
    /// Ref policy: every command updates a branch under a configured prefix.
    fn ref_policy(&self, updates: &[Update]) -> std::result::Result<(), Refused> {
        if updates.is_empty() {
            return Ok(());
        }
        for update in updates {
            if zero(&update.new) {
                return Err(Refused::new(format!(
                    "the relay does not delete refs ({})",
                    update.name
                )));
            }
            if update.name.starts_with("refs/tags/") {
                return Err(Refused::new(format!(
                    "the relay does not push tags ({})",
                    update.name
                )));
            }
            let allowed = valid_ref(&update.name)
                && self.prefixes.iter().any(|prefix| {
                    update
                        .name
                        .strip_prefix("refs/heads/")
                        .and_then(|branch| branch.strip_prefix(prefix.as_str()))
                        .is_some_and(|rest| !rest.is_empty())
                });
            if !allowed {
                return Err(Refused::new(format!(
                    "{} is not under an allowed branch prefix ({})",
                    update.name,
                    self.prefixes.join(", ")
                )));
            }
        }
        Ok(())
    }

    /// A git command with no credential and no user or system
    /// configuration: the environment the checks run in.
    fn git_command(&self, git_dir: &FsPath) -> Command {
        let mut command = Command::new(&self.git);
        command
            .env_clear()
            .env("PATH", &self.path)
            .env("HOME", &self.work)
            .env("LC_ALL", "C")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_DIR", git_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        command
    }

    /// Runs `command` with `input` on its standard input and returns its
    /// standard output, or `None` when it fails or passes the deadline.
    async fn run(&self, mut command: Command, input: Option<&[u8]>) -> Option<Vec<u8>> {
        if input.is_some() {
            command.stdin(Stdio::piped());
        }
        let mut child = command.spawn().ok()?;
        let stdin = child.stdin.take();
        let feed = async {
            if let (Some(mut stdin), Some(input)) = (stdin, input) {
                let _ = stdin.write_all(input).await;
            }
        };
        let (_, output) = tokio::time::timeout(GIT_DEADLINE, async {
            tokio::join!(feed, child.wait_with_output())
        })
        .await
        .ok()?;
        let output = output.ok()?;
        if !output.status.success() {
            tracing::warn!(
                status = ?output.status.code(),
                stderr = %String::from_utf8_lossy(&output.stderr[..output.stderr.len().min(CHECK_LOG_LIMIT)]),
                "git relay: a git command failed"
            );
            return None;
        }
        Some(output.stdout)
    }

    /// The repository's mirror, fetched from the forge now under the
    /// instance's token. The token reaches git through its environment,
    /// never its arguments.
    async fn mirror(&self, repository: &Repository) -> std::result::Result<PathBuf, Refused> {
        let unavailable = || Refused::new("the relay cannot read the repository from the forge");
        let mirror = self
            .work
            .join("mirrors")
            .join(&repository.owner)
            .join(format!("{}.git", repository.name));
        if !mirror.join("HEAD").exists() {
            private_directory(&mirror).map_err(|_| unavailable())?;
            let mut init = self.git_command(&mirror);
            init.env_remove("GIT_DIR")
                .args(["init", "--quiet", "--bare"])
                .arg(&mirror);
            self.run(init, None).await.ok_or_else(unavailable)?;
            for (key, value) in [
                ("gc.auto", "0"),
                ("maintenance.auto", "false"),
                ("fetch.writeCommitGraph", "false"),
                ("core.logAllRefUpdates", "false"),
            ] {
                let mut config = self.git_command(&mirror);
                config.args(["config", key, value]);
                self.run(config, None).await.ok_or_else(unavailable)?;
            }
        }
        let authorization = self.authorization().await.map_err(|_| unavailable())?;
        let header = Zeroizing::new(format!(
            "Authorization: {}",
            authorization.to_str().map_err(|_| unavailable())?
        ));
        let mut fetch = self.git_command(&mirror);
        let mut settings: Vec<(&str, &str)> = vec![
            ("http.extraHeader", header.as_str()),
            ("credential.helper", ""),
            ("protocol.allow", "never"),
            ("protocol.https.allow", "always"),
            // A redirect would carry the token's header to another host.
            ("http.followRedirects", "false"),
        ];
        let (ca, ca_path);
        if let Some(file) = &self.ca_file {
            ca = file.to_string_lossy().into_owned();
            ca_path = self.work.join("ca-path").to_string_lossy().into_owned();
            settings.push(("http.sslCAInfo", ca.as_str()));
            settings.push(("http.sslCAPath", ca_path.as_str()));
        }
        fetch.env("GIT_CONFIG_COUNT", settings.len().to_string());
        for (index, (key, value)) in settings.iter().enumerate() {
            fetch
                .env(format!("GIT_CONFIG_KEY_{index}"), key)
                .env(format!("GIT_CONFIG_VALUE_{index}"), value);
        }
        fetch
            .args([
                "fetch",
                "--quiet",
                "--no-tags",
                "--prune",
                "--no-write-fetch-head",
            ])
            .arg(self.url(repository, ""))
            .args(["+refs/heads/*:refs/heads/*", "+refs/tags/*:refs/tags/*"]);
        self.run(fetch, None).await.ok_or_else(unavailable)?;
        Ok(mirror)
    }

    /// A new repository for this request whose objects borrow the mirror's
    /// and whose refs are the mirror's, with the pack indexed into it.
    async fn quarantine(
        &self,
        mirror: &FsPath,
        request: &PushRequest,
        pack: &[u8],
    ) -> std::result::Result<tempfile::TempDir, Refused> {
        let failed = || Refused::new("the relay cannot prepare the push");
        let directory = tempfile::Builder::new()
            .prefix("push-")
            .tempdir_in(self.work.join("quarantine"))
            .map_err(|_| failed())?;
        let quarantine = directory.path();
        let mut init = self.git_command(quarantine);
        init.env_remove("GIT_DIR")
            .args(["init", "--quiet", "--bare"])
            .arg(quarantine);
        self.run(init, None).await.ok_or_else(failed)?;
        std::fs::write(
            quarantine.join("objects/info/alternates"),
            format!("{}\n", mirror.join("objects").display()),
        )
        .map_err(|_| failed())?;
        let mut refs = self.git_command(mirror);
        refs.args(["for-each-ref", "--format=update %(refname) %(objectname)"]);
        let refs = self.run(refs, None).await.ok_or_else(failed)?;
        let mut copy = self.git_command(quarantine);
        copy.args(["update-ref", "--stdin"]);
        self.run(copy, Some(&refs)).await.ok_or_else(failed)?;
        if request.updates.is_empty() {
            return Ok(directory);
        }
        if !pack.starts_with(b"PACK") {
            return Err(Refused {
                reason: "the push carries no pack".into(),
                unpack: "no pack",
            });
        }
        let mut index = self.git_command(quarantine);
        index.args(["index-pack", "--stdin", "--fix-thin", "--strict"]);
        let indexed = self.run(index, Some(pack)).await.ok_or(Refused {
            reason: "the pack cannot be indexed".into(),
            unpack: "index-pack failed",
        })?;
        // Every object the new tips reach is now present.
        let mut connected = self.git_command(quarantine);
        connected
            .args(["rev-list", "--objects"])
            .args(request.updates.iter().map(|u| u.new.as_str()))
            .args(["--not", "--all"]);
        let reached = self.run(connected, None).await.ok_or(Refused {
            reason: "the push is missing objects".into(),
            unpack: "missing objects",
        })?;
        // The forge stores the whole pack, so the checks below must see every
        // object in it: each is reached from a pushed tip, or the mirror
        // already holds it (as the bases --fix-thin appends do).
        let reached: BTreeSet<&[u8]> = reached
            .split(|b| *b == b'\n')
            .filter_map(|line| line.split(|b| *b == b' ').next())
            .filter(|id| !id.is_empty())
            .collect();
        let packed = self.packed(quarantine, &indexed).await.ok_or_else(failed)?;
        let others: Vec<u8> = packed
            .iter()
            .filter(|id| !reached.contains(id.as_bytes()))
            .flat_map(|id| [id.as_bytes(), b"\n"].concat())
            .collect();
        if !others.is_empty() {
            let mut present = self.git_command(mirror);
            present.args(["cat-file", "--batch-check=%(objectname)"]);
            let answer = self.run(present, Some(&others)).await.ok_or_else(failed)?;
            if answer
                .split(|b| *b == b'\n')
                .any(|line| line.ends_with(b" missing"))
            {
                return Err(Refused {
                    reason: "the pack carries objects no pushed branch reaches".into(),
                    unpack: "unreachable objects",
                });
            }
        }
        Ok(directory)
    }

    /// The ids of the objects in the pack `index-pack` just wrote, read from
    /// its index; `indexed` is what `index-pack --stdin` printed.
    async fn packed(&self, quarantine: &FsPath, indexed: &[u8]) -> Option<Vec<String>> {
        let printed = std::str::from_utf8(indexed).ok()?.trim_end();
        let (_, hash) = printed.split_once('\t')?;
        if !object_id(hash) {
            return None;
        }
        let index = std::fs::read(quarantine.join(format!("objects/pack/pack-{hash}.idx"))).ok()?;
        let mut show = self.git_command(quarantine);
        show.arg("show-index");
        let listed = self.run(show, Some(&index)).await?;
        std::str::from_utf8(&listed)
            .ok()?
            .lines()
            .map(|line| {
                let id = line.split(' ').nth(1)?;
                object_id(id).then(|| id.to_owned())
            })
            .collect()
    }

    /// Every new commit's author and committer are the configured identity.
    async fn identity(
        &self,
        quarantine: &FsPath,
        request: &PushRequest,
        identity: &CommitIdentity,
    ) -> std::result::Result<(), Refused> {
        let mut log = self.git_command(quarantine);
        // NUL separates the fields and ends each record: a commit object's
        // header cannot carry NUL, while other control bytes are accepted
        // there and are refused below.
        log.args(["log", "-z", "--format=%H%x00%an%x00%ae%x00%cn%x00%ce"])
            .args(request.updates.iter().map(|u| u.new.as_str()))
            .args(["--not", "--all"]);
        let unreadable = || Refused::new("the relay cannot read the new commits");
        let listed = self.run(log, None).await.ok_or_else(unreadable)?;
        let listed = std::str::from_utf8(&listed)
            .map_err(|_| Refused::new("a new commit's author or committer is not UTF-8"))?;
        let listed = listed.strip_suffix('\0').unwrap_or(listed);
        let fields: Vec<&str> = if listed.is_empty() {
            Vec::new()
        } else {
            listed.split('\0').collect()
        };
        if !fields.len().is_multiple_of(5) {
            return Err(unreadable());
        }
        for record in fields.chunks(5) {
            let [commit, author, author_email, committer, committer_email] = record else {
                return Err(unreadable());
            };
            let commit = commit.trim_start_matches('\n');
            if !object_id(commit) || record[1..].iter().any(|f| f.chars().any(char::is_control)) {
                return Err(Refused::new(format!(
                    "commit {} is not authored and committed by the configured identity",
                    if object_id(commit) { commit } else { "?" }
                )));
            }
            if *author != identity.name
                || *author_email != identity.email
                || *committer != identity.name
                || *committer_email != identity.email
            {
                return Err(Refused::new(format!(
                    "commit {commit} is not authored and committed by the configured identity"
                )));
            }
        }
        Ok(())
    }

    /// The configured pre-push check, in the quarantine with the pushed refs
    /// applied, the commands on its standard input, and no credential in its
    /// environment.
    async fn pre_push_check(
        &self,
        quarantine: &FsPath,
        repository: &Repository,
        request: &PushRequest,
        check: &PrePushCheck,
    ) -> std::result::Result<(), Refused> {
        let failed = || Refused::new("the relay cannot run the pre-push check");
        let mut apply = self.git_command(quarantine);
        apply.args(["update-ref", "--stdin"]);
        let lines: String = request
            .updates
            .iter()
            .map(|u| format!("update {} {}\n", u.name, u.new))
            .collect();
        self.run(apply, Some(lines.as_bytes()))
            .await
            .ok_or_else(failed)?;
        if let Some(first) = request.updates.first() {
            let mut head = self.git_command(quarantine);
            head.args(["symbolic-ref", "HEAD", &first.name]);
            self.run(head, None).await.ok_or_else(failed)?;
        }
        let program = &check.command[0];
        let program = if program.contains('/') {
            PathBuf::from(program)
        } else {
            locate(program, &self.path).ok_or_else(failed)?
        };
        let mut command = Command::new(program);
        command
            .args(&check.command[1..])
            .current_dir(quarantine)
            .env_clear()
            .env("PATH", &self.path)
            .env("HOME", quarantine)
            .env("LC_ALL", "C")
            .env("GIT_DIR", quarantine)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("CONNECTORS_GIT_REPOSITORY", repository.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            // Its own process group, killed whole once the check is over, so
            // nothing it started outlives the push.
            .process_group(0)
            .kill_on_drop(true);
        let mut child = command.spawn().map_err(|_| failed())?;
        let group = child.id().map(|id| ProcessGroup(id as i32));
        let input = command_lines(&request.updates);
        let stdin = child.stdin.take();
        let stderr = child.stderr.take();
        let feed = async {
            if let Some(mut stdin) = stdin {
                let _ = stdin.write_all(&input).await;
            }
        };
        // Drained to the end, the first CHECK_LOG_LIMIT bytes kept.
        let collect = tokio::spawn(async move {
            let mut kept = Vec::new();
            if let Some(mut stderr) = stderr {
                let mut buffer = [0u8; 8192];
                while let Ok(read @ 1..) = stderr.read(&mut buffer).await {
                    let room = CHECK_LOG_LIMIT.saturating_sub(kept.len());
                    kept.extend_from_slice(&buffer[..read.min(room)]);
                }
            }
            kept
        });
        let deadline = Duration::from_secs(check.timeout_seconds);
        let finished =
            tokio::time::timeout(deadline, async { tokio::join!(feed, child.wait()).1 }).await;
        drop(group);
        let Ok(status) = finished else {
            return Err(Refused::new("the pre-push check did not finish in time"));
        };
        let status = status.map_err(|_| failed())?;
        if !status.success() {
            let stderr = tokio::time::timeout(Duration::from_secs(1), collect)
                .await
                .ok()
                .and_then(|joined| joined.ok())
                .unwrap_or_default();
            tracing::info!(
                repository = %repository.path(),
                status = ?status.code(),
                stderr = %String::from_utf8_lossy(&stderr),
                "git push refused by the pre-push check"
            );
            return Err(Refused::new("the pre-push check refused the push"));
        }
        Ok(())
    }

    /// One push: the checks, then the original request to the forge once.
    async fn push(&self, principal: &str, repository: &Repository, body: Bytes) -> Response {
        let request = match parse(&body) {
            Ok(request) => request,
            Err(reason) => return text(StatusCode::BAD_REQUEST, reason),
        };
        // A request with no command is the client's probe: nothing to relay.
        if request.updates.is_empty() {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, RECEIVE_RESULT)],
                Vec::new(),
            )
                .into_response();
        }
        match self.checked(repository, &request, &body).await {
            Ok(()) => {}
            Err(refused) => {
                tracing::info!(
                    repository = %repository.path(),
                    principal,
                    reason = %refused.reason,
                    "git push refused"
                );
                return report(&request, refused.unpack, &refused.reason);
            }
        }
        self.relay_push(principal, repository, &request, body).await
    }

    async fn checked(
        &self,
        repository: &Repository,
        request: &PushRequest,
        body: &[u8],
    ) -> std::result::Result<(), Refused> {
        if request.options {
            return Err(Refused::new("push options are not relayed"));
        }
        self.ref_policy(&request.updates)?;
        let lock = {
            let mut mirrors = self
                .mirrors
                .lock()
                .map_err(|_| Refused::new("internal error"))?;
            mirrors
                .entry(repository.path().to_ascii_lowercase())
                .or_default()
                .clone()
        };
        let _held = lock.lock().await;
        let mirror = self.mirror(repository).await?;
        let quarantine = self
            .quarantine(&mirror, request, &body[request.pack..])
            .await?;
        if let Some(identity) = &self.identity {
            self.identity(quarantine.path(), request, identity).await?;
        }
        if let Some(check) = &self.check {
            self.pre_push_check(quarantine.path(), repository, request, check)
                .await?;
        }
        Ok(())
    }

    /// Sends the original request to the forge's receive-pack once and
    /// relays its answer; an answer that is lost is `unknown` and the push is
    /// never sent again.
    async fn relay_push(
        &self,
        principal: &str,
        repository: &Repository,
        request: &PushRequest,
        body: Bytes,
    ) -> Response {
        let authorization = match self.authorization().await {
            Ok(value) => value,
            Err(_) => {
                return report(request, "ok", "the instance's credential is not available");
            }
        };
        let send = self
            .client
            .post(self.url(repository, "/git-receive-pack"))
            .header(header::AUTHORIZATION, authorization)
            .header(header::CONTENT_TYPE, RECEIVE_REQUEST)
            .header(header::ACCEPT, RECEIVE_RESULT)
            .body(body)
            .send();
        let answer = match send.await {
            Ok(answer) => {
                let status = answer.status().as_u16();
                let content_type = answer
                    .headers()
                    .get(header::CONTENT_TYPE)
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or_default()
                    .to_owned();
                read_bounded(answer, REPORT_LIMIT)
                    .await
                    .map(|bytes| (status, content_type, bytes))
            }
            Err(_) => None,
        };
        let Some((status, content_type, bytes)) = answer else {
            self.logged(
                principal,
                repository,
                request,
                &vec![Outcome::Unknown; request.updates.len()],
            );
            return report(
                request,
                "ok",
                "the forge's answer was lost; the outcome is unknown, fetch to see it",
            );
        };
        if status != 200 || content_type != RECEIVE_RESULT {
            // A refusal before the forge read the push changes nothing; any
            // other answer may follow a partial update.
            let outcome = if (400..500).contains(&status) {
                Outcome::Refused
            } else {
                Outcome::Unknown
            };
            self.logged(
                principal,
                repository,
                request,
                &vec![outcome; request.updates.len()],
            );
            return report(
                request,
                "ok",
                &format!(
                    "the forge answered {status}; the outcome is {}",
                    outcome.word()
                ),
            );
        }
        let outcomes = outcomes(request, &bytes);
        self.logged(principal, repository, request, &outcomes);
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, RECEIVE_RESULT),
                (
                    header::CACHE_CONTROL,
                    "no-cache, max-age=0, must-revalidate",
                ),
            ],
            bytes,
        )
            .into_response()
    }

    fn logged(
        &self,
        principal: &str,
        repository: &Repository,
        request: &PushRequest,
        outcomes: &[Outcome],
    ) {
        for (update, outcome) in request.updates.iter().zip(outcomes) {
            tracing::info!(
                repository = %repository.path(),
                principal,
                r#ref = %update.name,
                old = %update.old,
                new = %update.new,
                outcome = %outcome.word(),
                "git push relayed"
            );
        }
    }
}

/// The whole body of `answer`, or `None` when it is lost or over `limit`.
async fn read_bounded(mut answer: reqwest::Response, limit: usize) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    loop {
        match answer.chunk().await {
            Ok(Some(chunk)) => {
                if bytes.len() + chunk.len() > limit {
                    return None;
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(None) => return Some(bytes),
            Err(_) => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: &str = "1111111111111111111111111111111111111111";
    const NEW: &str = "2222222222222222222222222222222222222222";
    const ZERO: &str = "0000000000000000000000000000000000000000";

    fn request(lines: &[&str], tail: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for line in lines {
            bytes.extend(pkt(line.as_bytes()));
        }
        bytes.extend_from_slice(b"0000");
        bytes.extend_from_slice(tail);
        bytes
    }

    #[test]
    fn a_command_list_parses_with_its_capabilities_and_pack_offset() {
        let first =
            format!("{OLD} {NEW} refs/heads/agent/one\0report-status side-band-64k agent=git/2\n");
        let second = format!("{ZERO} {NEW} refs/heads/agent/two\n");
        let bytes = request(&[&first, &second], b"PACKdata");
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed.updates.len(), 2);
        assert_eq!(parsed.updates[1].name, "refs/heads/agent/two");
        assert!(parsed.has("report-status") && parsed.has("side-band-64k"));
        assert_eq!(&bytes[parsed.pack..], b"PACKdata");
    }

    #[test]
    fn push_options_are_skipped_before_the_pack() {
        let first = format!("{OLD} {NEW} refs/heads/agent/one\0report-status push-options\n");
        let mut bytes = request(&[&first], b"");
        bytes.extend(pkt(b"ci.skip"));
        bytes.extend_from_slice(b"0000PACK");
        let parsed = parse(&bytes).unwrap();
        assert_eq!(&bytes[parsed.pack..], b"PACK");
        assert!(parsed.options);
    }

    #[test]
    fn shallow_signed_and_malformed_pushes_are_refused() {
        assert!(parse(&request(&[&format!("shallow {OLD}")], b"")).is_err());
        assert!(parse(&request(&["push-cert\0report-status"], b"")).is_err());
        assert!(parse(&request(&[&format!("{OLD} {NEW}")], b"")).is_err());
        assert!(parse(&request(&[&format!("{OLD} XYZ refs/heads/a")], b"")).is_err());
        assert!(parse(b"zzzz").is_err());
        assert!(parse(b"0003").is_err());
    }

    #[test]
    fn ref_names_follow_git_rules() {
        assert!(valid_ref("refs/heads/agent/one"));
        for bad in [
            "refs/heads/agent/../main",
            "refs/heads/agent/.hidden",
            "refs/heads/agent/one.lock",
            "refs/heads/agent/a b",
            "refs/heads/agent/a~1",
            "refs/heads/agent/",
            "refs/heads/agent//x",
            "refs/heads/agent/x@{1}",
        ] {
            assert!(!valid_ref(bad), "{bad}");
        }
    }

    #[test]
    fn a_banded_report_is_read_per_command() {
        let parsed = parse(&request(
            &[
                &format!("{OLD} {NEW} refs/heads/agent/a\0report-status side-band-64k\n"),
                &format!("{OLD} {NEW} refs/heads/agent/b\n"),
                &format!("{OLD} {NEW} refs/heads/agent/c\n"),
            ],
            b"",
        ))
        .unwrap();
        let mut inner = pkt(b"unpack ok\n");
        inner.extend(pkt(b"ok refs/heads/agent/a\n"));
        inner.extend(pkt(b"ng refs/heads/agent/b non-fast-forward\n"));
        inner.extend_from_slice(b"0000");
        let mut answer = pkt(b"\x02progress\n");
        let mut band = vec![1_u8];
        band.extend(inner);
        answer.extend(pkt(&band));
        answer.extend_from_slice(b"0000");
        assert_eq!(
            outcomes(&parsed, &answer),
            vec![Outcome::Applied, Outcome::Refused, Outcome::Unknown]
        );
        assert_eq!(outcomes(&parsed, b""), vec![Outcome::Unknown; 3]);
    }
}

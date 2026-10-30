use crate::credentials::CredentialRef;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response as HttpResponse},
    routing::{get, post},
};
use connectors_core::{
    Descriptor, Error, ErrorCode, Invocation, Outcome, REQUEST_LIMIT, RESPONSE_LIMIT, Response,
    Result, WIRE_VERSION,
};
use connectors_sdk::{Adapter, Credential, validate};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::Instant};
mod token_review;
mod v1alpha2;
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;

pub use crate::local::service::ServiceState;
pub use token_review::{CONCURRENT_REVIEWS, ServiceAccountAdmission, TokenReviewConfig};

/// `urn:connectors:config:v2:service` (`connectors.declarations.ServiceHostConfiguration`):
/// the v1 members plus the optional `state` directory, which `POST
/// /v1alpha2/invoke` requires (contracts/service/compatibility.md § 2.1).
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ServiceConfig {
    #[schemars(length(min = 1, max = 128), regex(pattern = "^[A-Za-z0-9_.-]+$"))]
    pub instance: String,
    #[schemars(with = "String", length(min = 1, max = 512))]
    pub listen: SocketAddr,
    pub service_credential: CredentialRef,
    /// The directory of the host's local metadata authority: its audit anchors
    /// and attempt records. Without it the host serves only `/v1/*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>", length(min = 1, max = 4096))]
    pub state: Option<std::path::PathBuf>,
}

/// `urn:connectors:config:v1:service`, unchanged and closed: it has no `state`.
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ServiceConfigV1 {
    #[schemars(length(min = 1, max = 128), regex(pattern = "^[A-Za-z0-9_.-]+$"))]
    pub instance: String,
    #[schemars(with = "String", length(min = 1, max = 512))]
    pub listen: SocketAddr,
    pub service_credential: CredentialRef,
}

/// `urn:connectors:config:v3:service` (`connectors.declarations.ServiceHostConfiguration`):
/// the v2 members plus the admission members (contracts/service/compatibility.md
/// § 2.1, Admission). `service_credential` is present exactly under
/// `static-bearer`. A v1 or v2 document reads here with the same meaning and
/// serializes to the same bytes. [`ServiceConfig`] stays the closed v2 reader
/// of the hosts that do not offer the admission profiles.
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(extend("oneOf" = [
    {
        "properties": {"admission": {"const": "static-bearer"}},
        "required": ["service_credential"],
        "not": {"anyOf": [{"required": ["audience"]}, {"required": ["principal"]}, {"required": ["token_review"]}]}
    },
    {
        "properties": {"admission": {"const": "service-account-token"}},
        "required": ["admission", "audience", "principal", "token_review"],
        "not": {"required": ["service_credential"]}
    }
]))]
pub struct ServiceConfigV3 {
    #[schemars(length(min = 1, max = 128), regex(pattern = "^[A-Za-z0-9_.-]+$"))]
    pub instance: String,
    #[schemars(with = "String", length(min = 1, max = 512))]
    pub listen: SocketAddr,
    /// The bearer callers present under `static-bearer`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_credential: Option<CredentialRef>,
    /// The directory of the host's local metadata authority: its audit anchors
    /// and attempt records. Without it the host serves only `/v1/*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>", length(min = 1, max = 4096))]
    pub state: Option<std::path::PathBuf>,
    /// How the host admits its caller; absent means `static-bearer`.
    #[serde(default, skip_serializing_if = "AdmissionProfile::is_static_bearer")]
    pub admission: AdmissionProfile,
    /// The one audience a `service-account-token` review asks for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 256))]
    pub audience: Option<String>,
    /// The one service account a `service-account-token` host admits,
    /// `system:serviceaccount:<namespace>:<name>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 128), regex(pattern = token_review::PRINCIPAL_PATTERN))]
    pub principal: Option<String>,
    /// The TokenReview endpoint of a `service-account-token` host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_review: Option<TokenReviewConfig>,
    /// The instance's standing write grants
    /// (`connectors.standing_approvals.GrantFile`). Absent: the host is
    /// unchanged, and a write on `POST /v1alpha2/invoke` is dispatched unkeyed
    /// as before. Present: every write there is keyed and needs a grant the
    /// file holds when the write arrives (a file absent at that moment holds
    /// none, so the write is `approval_required`), and `POST /v1/invoke`
    /// refuses writes. Requires `state`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>", length(min = 1, max = 4096))]
    pub standing_grants: Option<std::path::PathBuf>,
}

/// `connectors.declarations.ServiceAdmissionProfile`.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum AdmissionProfile {
    #[default]
    StaticBearer,
    ServiceAccountToken,
}
impl AdmissionProfile {
    fn is_static_bearer(&self) -> bool {
        *self == Self::StaticBearer
    }
}

impl From<ServiceConfig> for ServiceConfigV3 {
    fn from(config: ServiceConfig) -> Self {
        Self {
            instance: config.instance,
            listen: config.listen,
            service_credential: Some(config.service_credential),
            state: config.state,
            admission: AdmissionProfile::StaticBearer,
            audience: None,
            principal: None,
            token_review: None,
            standing_grants: None,
        }
    }
}

impl ServiceConfigV3 {
    /// The configured admission, refusing a document that mixes the profiles'
    /// members. `service-account-token` reads its CA here, once.
    pub fn admission(&self) -> Result<Admission> {
        let review_members =
            self.audience.is_some() || self.principal.is_some() || self.token_review.is_some();
        match self.admission {
            AdmissionProfile::StaticBearer => {
                if review_members {
                    return Err(Error::invalid(
                        "static-bearer admission takes no audience, principal or token_review",
                    ));
                }
                let Some(credential) = self.service_credential.clone() else {
                    return Err(Error::invalid(
                        "static-bearer admission requires service_credential",
                    ));
                };
                Ok(Admission::StaticBearer(Arc::new(credential)))
            }
            AdmissionProfile::ServiceAccountToken => {
                if self.service_credential.is_some() {
                    return Err(Error::invalid(
                        "service-account-token admission takes no service_credential",
                    ));
                }
                let (Some(audience), Some(principal), Some(review)) =
                    (&self.audience, &self.principal, &self.token_review)
                else {
                    return Err(Error::invalid(
                        "service-account-token admission requires audience, principal and token_review",
                    ));
                };
                Ok(Admission::ServiceAccountToken(Arc::new(
                    ServiceAccountAdmission::new(audience, principal, review)?,
                )))
            }
        }
    }
}

/// How a host admits its caller (contracts/service/compatibility.md § 2.1,
/// Admission): one profile for the whole host.
#[derive(Clone)]
pub enum Admission {
    /// One bearer compared in constant time; the principal is
    /// [`STATIC_BEARER_PRINCIPAL`](crate::local::service::STATIC_BEARER_PRINCIPAL).
    StaticBearer(Arc<dyn Credential>),
    /// One TokenReview per request; the principal is the configured service
    /// account.
    ServiceAccountToken(Arc<ServiceAccountAdmission>),
}

#[derive(Clone)]
struct Service {
    adapter: Arc<dyn Adapter>,
    admission: Admission,
    slots: Arc<Semaphore>,
    state: Option<Arc<ServiceState>>,
}

/// A host without `state`: `/v1alpha2/invoke` answers 503 `unavailable`.
pub fn router(adapter: Arc<dyn Adapter>, credential: Arc<dyn Credential>) -> Router {
    router_with_state(adapter, credential, None)
}

/// A `static-bearer` host.
pub fn router_with_state(
    adapter: Arc<dyn Adapter>,
    credential: Arc<dyn Credential>,
    state: Option<Arc<ServiceState>>,
) -> Router {
    router_with_admission(adapter, Admission::StaticBearer(credential), state)
}

pub fn router_with_admission(
    adapter: Arc<dyn Adapter>,
    admission: Admission,
    state: Option<Arc<ServiceState>>,
) -> Router {
    // The running host retries its retained final observations (audit.md § 3).
    if let Some(state) = &state {
        ServiceState::spawn_recovery(state);
    }
    Router::new()
        .route("/healthz", get(|| async { StatusCode::NO_CONTENT }))
        .route("/v1/describe", get(describe))
        .route("/v1/invoke", post(invoke))
        .route("/v1alpha2/invoke", post(v1alpha2::invoke))
        .layer(DefaultBodyLimit::max(REQUEST_LIMIT))
        .with_state(Service {
            adapter,
            admission,
            slots: Arc::new(Semaphore::new(32)),
            state,
        })
}

/// Admission (compatibility.md § 2.1, Admission). It returns the admitted
/// service principal, or the refusal the route answers before any anchor and
/// any adapter call.
async fn admitted(service: &Service, headers: &HeaderMap) -> Result<String> {
    service.admission.admit(headers).await
}

impl Admission {
    /// Admits the caller of one request by its `Authorization: Bearer`, under
    /// this host's profile, and returns the admitted service principal. Routes
    /// a composition adds ([`serve_with_routes`]) answer this refusal before
    /// they read the body.
    pub async fn admit(&self, headers: &HeaderMap) -> Result<String> {
        // Refusal: no bearer at all, before any credential read or review.
        let presented = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| {
                Error::new(ErrorCode::Unauthorized, "service authentication required")
            })?;
        match self {
            Admission::StaticBearer(credential) => {
                let expected = credential.resolve().await?;
                // Refusal: any bearer other than the configured bytes.
                if !bool::from(expected.0.as_slice().ct_eq(presented.as_bytes())) {
                    return Err(Error::new(
                        ErrorCode::Unauthorized,
                        "service authentication failed",
                    ));
                }
                Ok(crate::local::service::STATIC_BEARER_PRINCIPAL.to_owned())
            }
            Admission::ServiceAccountToken(account) => {
                // Refusal: every answer but the exact positive review
                // (`unauthorized`); every failed review, and a request that finds
                // all CONCURRENT_REVIEWS slots taken, with no review (`unavailable`).
                account.admit(presented).await?;
                Ok(account.principal().to_owned())
            }
        }
    }
}

async fn describe(State(service): State<Service>, headers: HeaderMap) -> HttpResponse {
    if let Err(error) = admitted(&service, &headers).await {
        return error_http(error);
    }
    Json(service.adapter.descriptor()).into_response()
}

async fn invoke(
    State(service): State<Service>,
    headers: HeaderMap,
    body: std::result::Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> HttpResponse {
    // Refusal: `/v1/invoke` writes no anchor, and every invocation under
    // `service-account-token` is anchored with its service account, so that
    // profile does not serve this route (compatibility.md § 2.1, Legacy
    // binding). Answered before any review and any adapter call.
    if let Admission::ServiceAccountToken(_) = service.admission {
        return error_http(Error::new(
            ErrorCode::Unsupported,
            "this host admits invocations only on /v1alpha2/invoke",
        ));
    }
    if let Err(error) = admitted(&service, &headers).await {
        return error_http(error);
    }
    let bytes = match body {
        Ok(v) => v,
        Err(_) => {
            return error_http(Error::new(
                ErrorCode::Capacity,
                "request exceeds byte limit",
            ));
        }
    };
    let invocation: Invocation = match connectors_core::read_json(&bytes) {
        Ok(v) => v,
        Err(_) => return error_http(Error::invalid("invalid invocation envelope")),
    };
    let started = Instant::now();
    let result = execute(&service, &invocation).await;
    tracing::info!(request_id = ?invocation.request_id, operation = ?invocation.operation, elapsed_ms = started.elapsed().as_millis(), success = result.is_ok(), "operation completed");
    let result = result.map_err(legacy);
    let status = result.as_ref().err().map(status).unwrap_or(StatusCode::OK);
    let response = Response {
        version: WIRE_VERSION.into(),
        request_id: invocation.request_id,
        outcome: match result {
            Ok(result) => Outcome::Success { result },
            Err(error) => Outcome::Error { error },
        },
    };
    (status, Json(response)).into_response()
}

async fn execute(service: &Service, request: &Invocation) -> Result<serde_json::Value> {
    if request.version != WIRE_VERSION {
        return Err(Error::new(
            ErrorCode::Unsupported,
            "unsupported request version",
        ));
    }
    if !connectors_core::valid_id(&request.request_id)
        || !connectors_core::valid_id(&request.operation)
    {
        return Err(Error::invalid("invalid request identifier"));
    }
    let descriptor: Descriptor = service.adapter.descriptor();
    if request.revision != descriptor.revision {
        return Err(Error::new(
            ErrorCode::StaleDescription,
            "refresh the descriptor before resubmitting",
        ));
    }
    let operation = descriptor.operation(&request.operation)?;
    // Refusal: on a host with standing grants a write runs only keyed, under
    // a grant, on `/v1alpha2/invoke`; this route records no attempt and reads
    // no grant, so it never dispatches one (compatibility.md § 2.1, Keyed
    // writes under a standing grant). A host without them is unchanged.
    if operation.profile == v1alpha2::MUTATION_PROFILE
        && service.state.as_ref().is_some_and(|state| state.keyed())
    {
        return Err(Error::new(
            ErrorCode::Unsupported,
            "on this host a write runs only on /v1alpha2/invoke",
        ));
    }
    validate(&operation.input_schema, &request.input)?;
    let _permit = service
        .slots
        .try_acquire()
        .map_err(|_| Error::new(ErrorCode::Capacity, "service concurrency limit reached"))?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        service
            .adapter
            .invoke_at(&request.revision, &request.operation, request.input.clone()),
    )
    .await
    .map_err(|_| Error::new(ErrorCode::Timeout, "operation deadline exceeded"))??;
    validate(&operation.output_schema, &result).map_err(|_| {
        Error::new(
            ErrorCode::UpstreamProtocol,
            "result does not match the declared output schema",
        )
    })?;
    if serde_json::to_vec(&result)
        .map_err(|_| Error::internal())?
        .len()
        > RESPONSE_LIMIT - 1024
    {
        return Err(Error::new(ErrorCode::Capacity, "result exceeds byte limit"));
    }
    Ok(result)
}

fn status(error: &Error) -> StatusCode {
    match error.code {
        ErrorCode::InvalidInput | ErrorCode::Conflict => StatusCode::BAD_REQUEST,
        ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
        ErrorCode::Forbidden => StatusCode::FORBIDDEN,
        ErrorCode::NotFound => StatusCode::NOT_FOUND,
        ErrorCode::StaleDescription | ErrorCode::StaleCursor => StatusCode::CONFLICT,
        ErrorCode::Unsupported => StatusCode::NOT_IMPLEMENTED,
        ErrorCode::Capacity | ErrorCode::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        ErrorCode::Timeout => StatusCode::GATEWAY_TIMEOUT,
        ErrorCode::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        ErrorCode::UpstreamProtocol => StatusCode::BAD_GATEWAY,
        ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
fn error_http(error: Error) -> HttpResponse {
    let error = legacy(error);
    (status(&error), Json(error)).into_response()
}

fn legacy(mut error: Error) -> Error {
    // The legacy binding emits no new code (compatibility.md §5): a provider
    // conflict keeps the `invalid_input` it was.
    if error.code == ErrorCode::Conflict {
        error.code = ErrorCode::InvalidInput;
    }
    error
}

/// The start refusal for a `state` that cannot be opened. A refused directory
/// is named (the configured path or one of its ancestors, never OS text) with
/// its fixed reason; any other failure keeps the closed text.
fn state_refusal(
    configured: &std::path::Path,
    refusal: crate::local::filesystem::Refusal,
) -> Error {
    use crate::local::filesystem::{Reason, Refusal};
    const REFUSED: &str = "service state cannot be opened for this instance";
    let Refusal::Refused(refused) = refusal else {
        return Error::invalid(REFUSED);
    };
    let directory = &refused.directory;
    // `state` itself is told its whole target, whichever check failed first:
    // fixing one bit at a time would only move the refusal to the next check.
    if directory == configured {
        return Error::invalid(format!(
            "{REFUSED}: {directory:?}, the state directory, must be owned by the host user \
             with mode 0700, no group or other bits"
        ));
    }
    let reason = match refused.reason {
        Reason::Owner => "is owned by neither the host user nor root",
        Reason::Writable => "is writable by group or others",
        // An ancestor is admitted without the private check; kept for totality.
        Reason::NotPrivate => "is not private",
    };
    Error::invalid(format!(
        "{REFUSED}: {directory:?}, an ancestor of state, {reason}"
    ))
}

/// Serves a v2 ([`ServiceConfig`], always `static-bearer`) or v3
/// ([`ServiceConfigV3`]) service configuration.
pub async fn serve(config: impl Into<ServiceConfigV3>, adapter: Arc<dyn Adapter>) -> Result<()> {
    serve_with_routes(config, adapter, |_| Router::new()).await
}

/// [`serve`], with the routes `routes` builds from the host's admission
/// merged beside the host's own. The added routes carry their own body
/// limits and must call [`Admission::admit`] before anything else.
pub async fn serve_with_routes(
    config: impl Into<ServiceConfigV3>,
    adapter: Arc<dyn Adapter>,
    routes: impl FnOnce(Admission) -> Router,
) -> Result<()> {
    let config = config.into();
    let admission = config.admission()?;
    // Refusal: keys and grant uses are kept in `state`; a grant file without
    // it would admit writes that nothing could replay or count.
    if config.standing_grants.is_some() && config.state.is_none() {
        return Err(Error::invalid("standing_grants requires state"));
    }
    match &admission {
        Admission::StaticBearer(credential) => drop(credential.resolve().await?),
        Admission::ServiceAccountToken(account) => account.ready().await?,
    }
    // Host start records the instance before /v1alpha2/invoke is served; a
    // refusal refuses start (compatibility.md § 2.1, Host start).
    let state = match config.state.clone() {
        None => None,
        Some(path) => {
            let descriptor = adapter.descriptor();
            let instance = config.instance.clone();
            let configured = path.clone();
            let state = tokio::task::spawn_blocking(move || {
                ServiceState::open_explained(
                    &path,
                    &instance,
                    &descriptor.adapter,
                    &descriptor.revision,
                )
            })
            .await
            .map_err(|_| Error::internal())?
            .map_err(|refusal| state_refusal(&configured, refusal))?;
            let state = state
                .with_standing_grants(config.standing_grants.clone())
                .map_err(|_| Error::invalid("standing grant uses cannot be opened"))?;
            Some(Arc::new(state))
        }
    };
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .map_err(|_| Error::invalid("service listener cannot be bound"))?;
    tracing::info!(instance=%config.instance, listen=%listener.local_addr().map_err(|_|Error::internal())?, admission=?config.admission, "adapter service listening");
    let added = routes(admission.clone());
    axum::serve(
        listener,
        router_with_admission(adapter, admission, state).merge(added),
    )
    .with_graceful_shutdown(async {
        // Windows has no SIGTERM: Ctrl+C or Ctrl+Break, and closing or
        // shutting down the console, end the service.
        #[cfg(windows)]
        {
            use tokio::signal::windows;
            match (
                windows::ctrl_break(),
                windows::ctrl_close(),
                windows::ctrl_shutdown(),
            ) {
                (Ok(mut brk), Ok(mut close), Ok(mut shutdown)) => {
                    tokio::select! {
                        _=tokio::signal::ctrl_c()=>{},
                        _=brk.recv()=>{},
                        _=close.recv()=>{},
                        _=shutdown.recv()=>{},
                    }
                }
                _ => {
                    let _ = tokio::signal::ctrl_c().await;
                }
            }
        }
        #[cfg(unix)]
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! { _=tokio::signal::ctrl_c()=>{}, _=terminate.recv()=>{} }
            }
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    })
    .await
    .map_err(|_| Error::internal())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    struct BoundedAdapter {
        entered: Arc<Semaphore>,
        result: Option<String>,
    }
    #[async_trait::async_trait]
    impl Adapter for BoundedAdapter {
        fn descriptor(&self) -> Descriptor {
            Descriptor {
                version: WIRE_VERSION.into(),
                instance: "bounds".into(),
                adapter: "fixture".into(),
                revision: "one".into(),
                configuration_schema: json!({"type":"object"}),
                operations: vec![connectors_core::Operation {
                    id: "read".into(),
                    description: "bounded read".into(),
                    contract: "operations/v1alpha1".into(),
                    profile: "read".into(),
                    input_schema: json!({"type":"object"}),
                    output_schema: json!({"type":"string"}),
                }],
            }
        }
        async fn invoke(&self, _: &str, _: Value) -> Result<Value> {
            self.entered.add_permits(1);
            match &self.result {
                Some(value) => Ok(json!(value)),
                None => std::future::pending().await,
            }
        }
    }
    fn fixture(result: Option<String>) -> (Service, Arc<Semaphore>) {
        let entered = Arc::new(Semaphore::new(0));
        (
            Service {
                adapter: Arc::new(BoundedAdapter {
                    entered: entered.clone(),
                    result,
                }),
                admission: Admission::StaticBearer(Arc::new(CredentialRef::Environment {
                    name: "UNUSED".into(),
                })),
                slots: Arc::new(Semaphore::new(32)),
                state: None,
            },
            entered,
        )
    }
    fn invocation() -> Invocation {
        Invocation {
            version: WIRE_VERSION.into(),
            request_id: "bounds".into(),
            operation: "read".into(),
            revision: "one".into(),
            input: json!({}),
        }
    }
    #[tokio::test(start_paused = true)]
    async fn service_deadline_cancels_the_adapter_and_releases_capacity() {
        let (service, entered) = fixture(None);
        let start = tokio::time::Instant::now();
        assert_eq!(
            execute(&service, &invocation()).await.unwrap_err().code,
            ErrorCode::Timeout
        );
        assert_eq!(start.elapsed(), std::time::Duration::from_secs(20));
        assert_eq!(entered.available_permits(), 1);
        assert_eq!(service.slots.available_permits(), 32);
    }
    #[tokio::test]
    async fn thirty_two_inflight_calls_refuse_the_next_until_one_is_dropped() {
        let (service, entered) = fixture(None);
        let spawn = || {
            let service = service.clone();
            tokio::spawn(async move { execute(&service, &invocation()).await })
        };
        let mut calls = (0..32).map(|_| spawn()).collect::<Vec<_>>();
        entered.acquire_many(32).await.unwrap().forget();
        assert_eq!(
            execute(&service, &invocation()).await.unwrap_err().code,
            ErrorCode::Capacity
        );
        assert_eq!(entered.available_permits(), 0);
        let first = calls.remove(0);
        first.abort();
        assert!(first.await.unwrap_err().is_cancelled());
        calls.push(spawn());
        entered.acquire().await.unwrap().forget();
        for call in calls {
            call.abort();
            let _ = call.await;
        }
        assert_eq!(service.slots.available_permits(), 32);
    }
    #[tokio::test]
    async fn oversized_adapter_result_is_refused_after_schema_validation() {
        let (service, _) = fixture(Some("x".repeat(RESPONSE_LIMIT)));
        assert_eq!(
            execute(&service, &invocation()).await.unwrap_err().code,
            ErrorCode::Capacity
        );
        let (service, _) = fixture(Some("small".into()));
        assert_eq!(
            execute(&service, &invocation()).await.unwrap(),
            json!("small")
        );
    }
}

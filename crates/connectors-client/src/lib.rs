use connectors_core::{
    Descriptor, Error, ErrorCode, Invocation, Outcome, RESPONSE_LIMIT, Response, Result,
    WIRE_VERSION, v1alpha2,
};
use futures_util::StreamExt;
use reqwest::{Client as HttpClient, Url};
use serde_json::Value;
use std::time::Duration;

/// A reusable transport and endpoint; contains no credential material.
#[derive(Clone)]
pub struct Endpoint {
    http: HttpClient,
    endpoint: Url,
}
#[derive(Clone)]
pub struct Client {
    transport: Endpoint,
    token: String,
}
impl Endpoint {
    pub fn new(endpoint: &str, allow_plaintext: bool) -> Result<Self> {
        let mut endpoint =
            Url::parse(endpoint).map_err(|_| Error::invalid("invalid service endpoint"))?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || (!allow_plaintext && endpoint.scheme() != "https")
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err(Error::invalid(
                "service endpoint configuration is not permitted",
            ));
        }
        if !endpoint.path().ends_with('/') {
            endpoint.set_path(&format!("{}/", endpoint.path()));
        }
        // reqwest carries no crypto provider of its own; ring is the one this workspace selects.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = HttpClient::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(|_| Error::internal())?;
        Ok(Self { http, endpoint })
    }
    pub fn with_token(&self, token: String) -> Result<Client> {
        if token.is_empty() || token.len() > 8192 {
            return Err(Error::invalid("invalid service credential"));
        }
        Ok(Client {
            transport: self.clone(),
            token,
        })
    }
}
impl Client {
    pub fn new(endpoint: &str, token: String, allow_plaintext: bool) -> Result<Self> {
        Endpoint::new(endpoint, allow_plaintext)?.with_token(token)
    }
    pub fn with_token(&self, token: String) -> Result<Self> {
        self.transport.with_token(token)
    }
    fn url(&self, path: &str) -> Result<Url> {
        self.transport
            .endpoint
            .join(path)
            .map_err(|_| Error::invalid("invalid service route"))
    }
    pub async fn describe(&self) -> Result<Descriptor> {
        let response = self
            .transport
            .http
            .get(self.url("v1/describe")?)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(transport_error)?;
        let status = response.status();
        let bytes = bounded(response).await?;
        if !status.is_success() {
            return Err(parse_error(status.as_u16(), &bytes));
        }
        let descriptor: Descriptor = connectors_core::read_json(&bytes)
            .map_err(|_| Error::new(ErrorCode::UpstreamProtocol, "invalid service descriptor"))?;
        if descriptor.version != WIRE_VERSION {
            return Err(Error::new(
                ErrorCode::Unsupported,
                "unsupported service version",
            ));
        }
        Ok(descriptor)
    }
    pub async fn invoke(
        &self,
        descriptor: &Descriptor,
        operation: &str,
        input: Value,
    ) -> Result<Value> {
        descriptor.operation(operation)?;
        let request = Invocation {
            version: WIRE_VERSION.into(),
            request_id: uuid::Uuid::new_v4().to_string(),
            operation: operation.into(),
            revision: descriptor.revision.clone(),
            input,
        };
        self.send(&request).await
    }
    /// Invoke on the first v1alpha2 binding, `POST /v1alpha2/invoke`
    /// (contracts/service/compatibility.md § 2.1), with the revision of a
    /// descriptor read from `GET /v1/describe`.
    ///
    /// The binding is selected here and nowhere else: the client never sends
    /// the invocation to `/v1/invoke` or resends it on any route. An endpoint
    /// answering HTTP 404 without a v1alpha2 envelope does not serve the
    /// binding and is reported `unsupported`. Every answer is read as the
    /// § 5 Response and checked for correlation and its HTTP mapping.
    ///
    /// A successful write carries its `mutation`, whose `attempt` names the
    /// `AttemptRecord` the host recorded. A [`Failure`] carries the
    /// `mutation` when the host recorded an attempt; a failure without one is
    /// no proof that the operation was not dispatched.
    pub async fn invoke_v1alpha2(
        &self,
        descriptor: &Descriptor,
        operation: &str,
        input: Value,
    ) -> std::result::Result<Invoked, Box<Failure>> {
        descriptor.operation(operation).map_err(Failure::local)?;
        let request = v1alpha2::InvokeRequest {
            version: v1alpha2::VERSION.into(),
            request_id: uuid::Uuid::new_v4().to_string(),
            operation: operation.into(),
            revision: descriptor.revision.clone(),
            input,
        };
        let body = request.encode().map_err(|_| {
            Failure::new(
                v1alpha2::ErrorCode::InvalidInput,
                "invocation input cannot be encoded",
            )
        })?;
        let response = self
            .transport
            .http
            .post(self.url("v1alpha2/invoke").map_err(Failure::local)?)
            .bearer_auth(&self.token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .map_err(|error| Failure::local(transport_error(error)))?;
        let status = response.status().as_u16();
        let bytes = bounded(response).await.map_err(Failure::local)?;
        let response = match v1alpha2::InvokeResponse::decode(&bytes) {
            Ok(response) => response,
            Err(_) if status == 404 => {
                return Err(Failure::new(
                    v1alpha2::ErrorCode::Unsupported,
                    "service does not serve POST /v1alpha2/invoke",
                ));
            }
            Err(error) => return Err(Failure::from_error(error)),
        };
        read_v1alpha2(&request.request_id, status, response)
    }

    pub async fn send(&self, request: &Invocation) -> Result<Value> {
        let response = self
            .transport
            .http
            .post(self.url("v1/invoke")?)
            .bearer_auth(&self.token)
            .json(request)
            .send()
            .await
            .map_err(transport_error)?;
        let status = response.status();
        let bytes = bounded(response).await?;
        let response: Response =
            connectors_core::read_json(&bytes).map_err(|_| parse_error(status.as_u16(), &bytes))?;
        if response.version != WIRE_VERSION || response.request_id != request.request_id {
            return Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "mismatched response correlation or version",
            ));
        }
        match response.outcome {
            Outcome::Success { result } if status.is_success() => Ok(result),
            Outcome::Error { error } => Err(error),
            _ => Err(Error::new(
                ErrorCode::UpstreamProtocol,
                "inconsistent response status",
            )),
        }
    }
}

/// A successful invocation on the v1alpha2 binding.
#[derive(Clone, Debug, PartialEq)]
pub struct Invoked {
    /// The operation's result value.
    pub result: Value,
    /// Present for a write: the attempt the host recorded for it.
    pub mutation: Option<v1alpha2::MutationObservation>,
    /// The host's execution audit record for this invocation.
    pub audit_ref: Option<String>,
    pub audit_status: v1alpha2::AuditStatus,
}

/// An invocation on the v1alpha2 binding that did not succeed. The audit and
/// mutation members are the host's when a valid Response was read; a failure
/// the client raised itself (no Response, or an invalid one) carries no audit
/// status.
#[derive(Clone, Debug, PartialEq)]
pub struct Failure {
    pub error: v1alpha2::Error,
    /// Present when the host recorded an attempt for this invocation;
    /// `outcome_unknown` with an attempt is not retried.
    pub mutation: Option<v1alpha2::MutationObservation>,
    pub audit_ref: Option<String>,
    pub audit_status: Option<v1alpha2::AuditStatus>,
}

impl Failure {
    fn new(code: v1alpha2::ErrorCode, message: &str) -> Box<Self> {
        Self::from_error(v1alpha2::Error::new(code, message))
    }
    fn from_error(error: v1alpha2::Error) -> Box<Self> {
        Box::new(Self {
            error,
            mutation: None,
            audit_ref: None,
            audit_status: None,
        })
    }
    /// A failure the client raised before or around the exchange.
    fn local(error: Error) -> Box<Self> {
        Self::from_error(v1alpha2::Error {
            code: extended(error.code),
            message: error.message,
            retry_after_seconds: error.retry_after_seconds,
        })
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.error.code, self.error.message)
    }
}

impl std::error::Error for Failure {}

/// Correlation and the § 5 HTTP mapping of a decoded Response. A Response
/// without the request's ID is admitted only as an error carrying no
/// `mutation` (a refusal before a valid envelope). A mismatch keeps the
/// `mutation` the host reported.
fn read_v1alpha2(
    request_id: &str,
    status: u16,
    response: v1alpha2::InvokeResponse,
) -> std::result::Result<Invoked, Box<Failure>> {
    let correlated = match &response.request_id {
        Some(echoed) => echoed == request_id,
        None => response.status == v1alpha2::ResponseStatus::Error && response.mutation.is_none(),
    };
    let expected = response.error.as_ref().map_or(200, |e| http_status(e.code));
    let mut failure = Box::new(Failure {
        error: v1alpha2::Error::new(v1alpha2::ErrorCode::UpstreamProtocol, ""),
        mutation: response.mutation,
        audit_ref: response.audit_ref,
        audit_status: Some(response.audit_status),
    });
    if !correlated || status != expected {
        failure.error.message = "mismatched response correlation or status".into();
        return Err(failure);
    }
    match (response.result, response.error) {
        (Some(result), None) => Ok(Invoked {
            result,
            mutation: failure.mutation,
            audit_ref: failure.audit_ref,
            audit_status: response.audit_status,
        }),
        (None, Some(error)) => {
            failure.error = error;
            Err(failure)
        }
        // InvokeResponse::decode admits exactly one of the two.
        _ => {
            failure.error.message = "inconsistent response status".into();
            Err(failure)
        }
    }
}

/// The extended HTTP mapping (compatibility.md § 5).
fn http_status(code: v1alpha2::ErrorCode) -> u16 {
    use v1alpha2::ErrorCode::*;
    match code {
        InvalidInput | Unsupported => 400,
        Unauthorized => 401,
        Forbidden | ApprovalRequired | ApprovalRefused | NotGranted | RouteRefused
        | InsufficientScope => 403,
        NotFound => 404,
        StaleDescription | StaleCursor | ApprovalReplayed | IdempotencyConflict
        | StaleAuthority | ConnectionNotReady | SessionNotReady | OfferRejected => 409,
        SessionLost | OfferExpired | LeaseExpired | Revoked => 410,
        Capacity => 413,
        RateLimited => 429,
        Internal => 500,
        UpstreamProtocol | OutcomeUnknown => 502,
        Unavailable | RouteUnavailable => 503,
        Timeout => 504,
    }
}

/// A v1alpha1 code as the same member of the extended set.
fn extended(code: ErrorCode) -> v1alpha2::ErrorCode {
    use v1alpha2::ErrorCode as E;
    match code {
        ErrorCode::InvalidInput => E::InvalidInput,
        ErrorCode::Unsupported => E::Unsupported,
        ErrorCode::Unauthorized => E::Unauthorized,
        ErrorCode::Forbidden => E::Forbidden,
        ErrorCode::NotFound => E::NotFound,
        ErrorCode::StaleDescription => E::StaleDescription,
        ErrorCode::StaleCursor => E::StaleCursor,
        ErrorCode::RateLimited => E::RateLimited,
        ErrorCode::Unavailable => E::Unavailable,
        ErrorCode::Capacity => E::Capacity,
        ErrorCode::Timeout => E::Timeout,
        ErrorCode::UpstreamProtocol => E::UpstreamProtocol,
        ErrorCode::Internal => E::Internal,
    }
}

pub async fn bounded(response: reqwest::Response) -> Result<Vec<u8>> {
    if response
        .content_length()
        .is_some_and(|n| n > RESPONSE_LIMIT as u64)
    {
        return Err(Error::new(
            ErrorCode::Capacity,
            "response exceeds byte limit",
        ));
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(transport_error)?;
        if bytes.len() + chunk.len() > RESPONSE_LIMIT {
            return Err(Error::new(
                ErrorCode::Capacity,
                "response exceeds byte limit",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn transport_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::new(ErrorCode::Timeout, "service request timed out")
    } else {
        Error::unavailable()
    }
}
fn parse_error(status: u16, bytes: &[u8]) -> Error {
    serde_json::from_slice(bytes).unwrap_or_else(|_| {
        Error::new(
            match status {
                401 => ErrorCode::Unauthorized,
                403 => ErrorCode::Forbidden,
                413 => ErrorCode::Capacity,
                400 => ErrorCode::InvalidInput,
                _ => ErrorCode::UpstreamProtocol,
            },
            "service refused request or returned an invalid response",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn response(value: Value) -> v1alpha2::InvokeResponse {
        v1alpha2::InvokeResponse::decode(&serde_json::to_vec(&value).unwrap()).unwrap()
    }
    fn write_unknown() -> Value {
        json!({"version":"v1alpha2","request_id":"r-1","status":"error",
            "error":{"code":"outcome_unknown","message":"answer lost"},
            "audit_ref":"a-1","audit_status":"complete",
            "mutation":{"classification":"unknown",
                "attempt":{"instance":"leaf","id":"00000000-0000-4000-8000-000000000001"},
                "original_request_id":"r-1","replayed":false,
                "cause":{"code":"unavailable","stage":"dispatch"}}})
    }

    #[test]
    fn outcome_unknown_keeps_its_attempt() {
        let failure = read_v1alpha2("r-1", 502, response(write_unknown())).unwrap_err();
        assert_eq!(failure.error.code, v1alpha2::ErrorCode::OutcomeUnknown);
        let attempt = failure.mutation.unwrap().attempt.unwrap();
        assert_eq!(attempt.id.as_str(), "00000000-0000-4000-8000-000000000001");
        assert_eq!(failure.audit_ref.as_deref(), Some("a-1"));
    }

    #[test]
    fn a_status_outside_the_mapping_is_upstream_protocol_and_keeps_the_mutation() {
        let failure = read_v1alpha2("r-1", 200, response(write_unknown())).unwrap_err();
        assert_eq!(failure.error.code, v1alpha2::ErrorCode::UpstreamProtocol);
        assert!(failure.mutation.is_some());
    }

    #[test]
    fn another_request_id_or_an_uncorrelated_success_is_upstream_protocol() {
        let failure = read_v1alpha2("r-2", 502, response(write_unknown())).unwrap_err();
        assert_eq!(failure.error.code, v1alpha2::ErrorCode::UpstreamProtocol);
        let success = json!({"version":"v1alpha2","request_id":null,"status":"success",
            "result":{},"audit_ref":"a-1","audit_status":"complete"});
        let failure = read_v1alpha2("r-1", 200, response(success)).unwrap_err();
        assert_eq!(failure.error.code, v1alpha2::ErrorCode::UpstreamProtocol);
    }

    #[test]
    fn a_refusal_before_the_envelope_is_read_as_the_hosts_error() {
        let refused = json!({"version":"v1alpha2","request_id":null,"status":"error",
            "error":{"code":"unavailable","message":"service state is not configured"},
            "audit_ref":null,"audit_status":"unavailable"});
        let failure = read_v1alpha2("r-1", 503, response(refused)).unwrap_err();
        assert_eq!(failure.error.code, v1alpha2::ErrorCode::Unavailable);
        assert_eq!(
            failure.audit_status,
            Some(v1alpha2::AuditStatus::Unavailable)
        );
    }
}

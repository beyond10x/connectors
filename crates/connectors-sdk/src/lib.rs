//! Ports and shared semantic mechanisms; concrete infrastructure is host-owned.
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use connectors_contracts::Provenance;
use connectors_core::{Descriptor, Error, ErrorCode, Result, digest};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::Sha256;
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

pub use async_trait::async_trait as adapter_trait;

#[async_trait]
pub trait Adapter: Send + Sync {
    fn descriptor(&self) -> Descriptor;
    async fn invoke(&self, operation: &str, input: Value) -> Result<Value>;
    /// Dynamic adapters override this to select a route from the admitted snapshot.
    async fn invoke_at(&self, revision: &str, operation: &str, input: Value) -> Result<Value> {
        if self.descriptor().revision != revision {
            return Err(Error::new(
                ErrorCode::StaleDescription,
                "refresh the descriptor before resubmitting",
            ));
        }
        self.invoke(operation, input).await
    }
}

/// Deliberately has no Debug or Serialize implementation.
/// Trusted credential and transport implementations can read the bytes. This
/// prevents accidental formatting, not deliberate disclosure by a byte holder.
pub struct Secret(pub Vec<u8>);

impl Drop for Secret {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.0.zeroize();
    }
}

#[async_trait]
pub trait Credential: Send + Sync {
    async fn resolve(&self) -> Result<Secret>;
}

#[async_trait]
pub trait SecretStore: Send + Sync {
    async fn read(&self, reference: &str) -> Result<Secret>;
}

pub struct HttpResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

/// A bounded response prefix. Completeness means EOF was observed for this
/// response; it makes no assertion about future changes to the provider resource.
pub struct HttpResponsePrefix {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
    pub complete: bool,
}

#[async_trait]
pub trait AuthenticatedHttp: Send + Sync {
    /// Segments are encoded individually by the binding; no arbitrary URL input.
    async fn get(&self, segments: &[&str], query: &[(&str, String)]) -> Result<HttpResponse>;

    /// Retain at most `limit` bytes (1..=1048576) from one GET, under the same
    /// authority and deadline as `get`. Unsupported ports refuse without I/O.
    /// No range/resume or automatic retry is implied by an incomplete prefix.
    async fn get_prefix(
        &self,
        _segments: &[&str],
        _query: &[(&str, String)],
        _limit: usize,
    ) -> Result<HttpResponsePrefix> {
        Err(Error::unavailable())
    }
}

/// A single admitted native write. Composition supplies exact credentials and
/// destination restrictions after the host gate. GET capabilities cannot be
/// converted into this port, and calling it consumes the capability even when
/// the response is lost. Implementations must not retry or follow redirects.
#[async_trait]
pub trait AuthenticatedWrite: Send {
    async fn put_json(
        self: Box<Self>,
        segments: &[&str],
        query: &[(&str, String)],
        body: &Value,
    ) -> Result<HttpResponse>;
}

/// Native effect knowledge is independent of whether a safe result can be
/// disclosed. A malformed result after a known applied effect must not turn it
/// into a not-attempted failure or grant another send. No wire codec is implied.
pub enum WriteOutcome<T> {
    Applied(Result<T>),
    Refused(Error),
    Unknown(Error),
}
impl<T> WriteOutcome<T> {
    pub fn try_map<U>(self, map: impl FnOnce(T) -> Result<U>) -> WriteOutcome<U> {
        match self {
            Self::Applied(value) => WriteOutcome::Applied(value.and_then(map)),
            Self::Refused(error) => WriteOutcome::Refused(error),
            Self::Unknown(error) => WriteOutcome::Unknown(error),
        }
    }
}

pub fn decode<T: DeserializeOwned>(input: Value) -> Result<T> {
    serde_json::from_value(input)
        .map_err(|_| Error::invalid("input does not match the operation type"))
}
pub fn encode<T: Serialize>(value: T) -> Result<Value> {
    serde_json::to_value(value).map_err(|_| Error::internal())
}
pub fn validate(schema: &Value, value: &Value) -> Result<()> {
    let validator = jsonschema::validator_for(schema).map_err(|_| Error::internal())?;
    if validator.is_valid(value) {
        Ok(())
    } else {
        Err(Error::invalid("value does not match its declared schema"))
    }
}
/// Strict write codec validation, including supported format assertions. Kept
/// separate so existing read consumers retain their established behavior.
pub fn validate_write_value(schema: &Value, value: &Value) -> Result<()> {
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|_| Error::internal())?;
    if validator.is_valid(value) {
        Ok(())
    } else {
        Err(Error::invalid(
            "write value does not match its declared schema",
        ))
    }
}
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn provenance(
    instance: &str,
    resource: impl Into<String>,
    revision: Option<String>,
) -> Provenance {
    Provenance {
        instance: instance.into(),
        resource: resource.into(),
        observed_at_unix_ms: now_ms(),
        source_revision: revision,
    }
}

pub fn upstream_json(response: &HttpResponse) -> Result<Value> {
    if !(200..300).contains(&response.status) {
        let code = match response.status {
            401 => ErrorCode::Unauthorized,
            403 => ErrorCode::Forbidden,
            404 => ErrorCode::NotFound,
            410 => ErrorCode::StaleCursor,
            429 => ErrorCode::RateLimited,
            400 | 422 => ErrorCode::InvalidInput,
            500..=599 => ErrorCode::Unavailable,
            _ => ErrorCode::UpstreamProtocol,
        };
        let mut error = Error::new(code, "provider refused the request");
        error.retry_after_seconds = response
            .headers
            .get("retry-after")
            .and_then(|v| v.parse().ok());
        return Err(error);
    }
    connectors_core::read_json(&response.body).map_err(|_| {
        Error::new(
            ErrorCode::UpstreamProtocol,
            "provider returned invalid or ambiguous JSON",
        )
    })
}

pub fn instance_descriptor(
    template: &str,
    instance: &str,
    configuration: &Value,
) -> Result<Descriptor> {
    if !connectors_core::valid_id(instance) {
        return Err(Error::invalid("invalid instance identifier"));
    }
    let mut descriptor: Descriptor =
        serde_json::from_str(template).map_err(|_| Error::internal())?;
    validate(&descriptor.configuration_schema, configuration)?;
    descriptor.instance = instance.into();
    descriptor.revision = digest(
        &serde_json::json!({"spec": descriptor.revision, "instance":instance,"configuration":configuration}),
    );
    Ok(descriptor)
}

pub fn verify_handlers(descriptor: &Descriptor, handlers: &[&str]) -> Result<()> {
    let declared: std::collections::BTreeSet<_> = descriptor
        .operations
        .iter()
        .map(|o| o.id.as_str())
        .collect();
    let implemented: std::collections::BTreeSet<_> = handlers.iter().copied().collect();
    if declared != implemented || declared.len() != descriptor.operations.len() {
        return Err(Error::invalid(
            "generated operation declarations do not match implemented handlers",
        ));
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    context: String,
    continuation: String,
    expires: u64,
}

/// Ephemeral HMAC keys intentionally invalidate cursors after process restart.
pub struct Cursors {
    key: [u8; 32],
}
impl Default for Cursors {
    fn default() -> Self {
        let mut key = [0; 32];
        key[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        key[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        Self { key }
    }
}
impl Cursors {
    pub fn issue(&self, context: &Value, continuation: String) -> Result<String> {
        let bytes = serde_json::to_vec(&Cursor {
            context: digest(context),
            continuation,
            expires: now_ms() + 300_000,
        })
        .map_err(|_| Error::internal())?;
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| Error::internal())?;
        mac.update(&bytes);
        Ok(format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(&bytes),
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        ))
    }
    pub fn read(&self, context: &Value, token: &str) -> Result<String> {
        let fail = || {
            Error::new(
                ErrorCode::StaleCursor,
                "cursor expired or does not belong to this request",
            )
        };
        if token.len() > 16_384 {
            return Err(fail());
        }
        let (body, signature) = token.split_once('.').ok_or_else(fail)?;
        let bytes = URL_SAFE_NO_PAD.decode(body).map_err(|_| fail())?;
        let signature = URL_SAFE_NO_PAD.decode(signature).map_err(|_| fail())?;
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| fail())?;
        mac.update(&bytes);
        mac.verify_slice(&signature).map_err(|_| fail())?;
        let cursor: Cursor = serde_json::from_slice(&bytes).map_err(|_| fail())?;
        if cursor.expires < now_ms() || cursor.context != digest(context) {
            return Err(fail());
        }
        Ok(cursor.continuation)
    }
}

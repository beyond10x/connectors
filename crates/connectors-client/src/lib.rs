use connectors_core::{
    Descriptor, Error, ErrorCode, Invocation, Outcome, RESPONSE_LIMIT, Response, Result,
    WIRE_VERSION,
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

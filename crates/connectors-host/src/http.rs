use crate::credentials::CredentialRef;
use async_trait::async_trait;
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::{
    AuthenticatedHttp, AuthenticatedWrite, Credential, HttpResponse, HttpResponsePrefix,
};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc, time::Duration};

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HttpConfig {
    #[schemars(length(min = 1, max = 4096))]
    pub base_url: String,
    pub credential: Option<CredentialRef>,
    #[serde(default = "default_header")]
    #[schemars(length(min = 1, max = 512))]
    pub credential_header: String,
    #[serde(default)]
    pub bearer: bool,
    #[serde(default)]
    pub allow_plaintext: bool,
    #[schemars(length(min = 1, max = 4096))]
    pub ca_file: Option<PathBuf>,
}
fn default_header() -> String {
    "authorization".into()
}

/// Canonical nonsecret HTTP authority used by native configuration bindings.
pub fn canonical_base(value: &str) -> Result<String> {
    let mut base =
        Url::parse(value).map_err(|_| Error::invalid("invalid configured HTTP endpoint"))?;
    if !matches!(base.scheme(), "http" | "https")
        || base.host_str().is_none()
        || !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return Err(Error::invalid("invalid configured HTTP endpoint"));
    }
    if !base.path().ends_with('/') {
        base.set_path(&format!("{}/", base.path()));
    }
    Ok(base.to_string())
}

pub struct ScopedHttp {
    client: Client,
    base: Url,
    credential: Option<Arc<dyn Credential>>,
    header: reqwest::header::HeaderName,
    bearer: bool,
}

impl ScopedHttp {
    pub fn from_config(config: &HttpConfig) -> Result<Self> {
        Self::new(
            config,
            config
                .credential
                .clone()
                .map(|c| Arc::new(c) as Arc<dyn Credential>),
        )
    }
    pub fn new(config: &HttpConfig, credential: Option<Arc<dyn Credential>>) -> Result<Self> {
        let certificates = config
            .ca_file
            .as_ref()
            .map(|path| {
                std::fs::read(path).map_err(|_| Error::invalid("cannot read configured CA"))
            })
            .transpose()?;
        Self::new_with_ca_bytes(config, credential, certificates.as_deref())
    }
    /// The trusted composition has already captured/admitted its CA bytes. This
    /// constructor never reopens the configuration's CA path after bootstrap.
    pub fn new_with_ca_bytes(
        config: &HttpConfig,
        credential: Option<Arc<dyn Credential>>,
        ca: Option<&[u8]>,
    ) -> Result<Self> {
        let mut base = Url::parse(&config.base_url)
            .map_err(|_| Error::invalid("invalid configured HTTP endpoint"))?;
        if !matches!(base.scheme(), "http" | "https")
            || (!config.allow_plaintext && base.scheme() != "https")
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return Err(Error::invalid(
                "configured endpoint must have a permitted scheme and no credentials/query/fragment",
            ));
        }
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        let header = reqwest::header::HeaderName::from_bytes(config.credential_header.as_bytes())
            .map_err(|_| Error::invalid("invalid authentication header"))?;
        if matches!(
            header.as_str(),
            "host"
                | "content-length"
                | "transfer-encoding"
                | "connection"
                | "proxy-connection"
                | "keep-alive"
                | "te"
                | "trailer"
                | "upgrade"
        ) {
            return Err(Error::invalid(
                "authentication cannot replace HTTP routing or framing headers",
            ));
        }
        let mut builder = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15));
        if let Some(bytes) = ca {
            let certificates = reqwest::Certificate::from_pem_bundle(bytes)
                .map_err(|_| Error::invalid("invalid configured CA"))?;
            if certificates.is_empty() {
                return Err(Error::invalid("configured CA contains no certificates"));
            }
            builder = builder.tls_built_in_root_certs(false);
            for certificate in certificates {
                builder = builder.add_root_certificate(certificate);
            }
        }
        Ok(Self {
            client: builder.build().map_err(|_| Error::internal())?,
            base,
            credential,
            header,
            bearer: config.bearer,
        })
    }
    /// Immutable per-command credential capability over the captured target/TLS
    /// configuration. It changes no existing capability or credential source.
    pub fn with_credential(&self, credential: Arc<dyn Credential>) -> Self {
        Self {
            client: self.client.clone(),
            base: self.base.clone(),
            credential: Some(credential),
            header: self.header.clone(),
            bearer: self.bearer,
        }
    }
}

impl ScopedHttp {
    /// Trusted composition selects a single consuming PUT capability. This
    /// conversion grants no approval or ledger authority. Business adapters
    /// receiving only `AuthenticatedHttp` cannot perform it.
    ///
    /// ```compile_fail
    /// use connectors_host::http::ScopedHttp;
    /// async fn repeat(http: ScopedHttp) {
    ///     let write = http.into_write();
    ///     write.put_json(&["items", "1"], &[], &serde_json::json!({})).await;
    ///     write.put_json(&["items", "1"], &[], &serde_json::json!({})).await;
    /// }
    /// ```
    pub fn into_write(self) -> Box<dyn AuthenticatedWrite> {
        Box::new(ScopedWrite(self))
    }

    async fn send_get(
        &self,
        segments: &[&str],
        query: &[(&str, String)],
    ) -> Result<reqwest::Response> {
        self.request(reqwest::Method::GET, segments, query)
            .await?
            .send()
            .await
            .map_err(provider_error)
    }

    async fn request(
        &self,
        method: reqwest::Method,
        segments: &[&str],
        query: &[(&str, String)],
    ) -> Result<reqwest::RequestBuilder> {
        let mut url = self.base.clone();
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|_| Error::invalid("endpoint cannot carry path segments"))?;
            path.pop_if_empty();
            for segment in segments {
                if segment.is_empty() || *segment == "." || *segment == ".." {
                    return Err(Error::invalid("invalid provider path segment"));
                }
                path.push(segment);
            }
        }
        url.query_pairs_mut()
            .extend_pairs(query.iter().map(|(k, v)| (*k, v.as_str())));
        let mut request = self.client.request(method, url);
        if let Some(credential) = &self.credential {
            let secret = credential.resolve().await?;
            let value = zeroize::Zeroizing::new(if self.bearer {
                let mut value = b"Bearer ".to_vec();
                value.extend_from_slice(&secret.0);
                value
            } else {
                secret.0.clone()
            });
            let mut header = reqwest::header::HeaderValue::from_bytes(&value)
                .map_err(|_| Error::new(ErrorCode::Unauthorized, "invalid provider credential"))?;
            header.set_sensitive(true);
            request = request.header(&self.header, header);
        }
        Ok(request)
    }
}

// Deliberately private, non-Clone, and separate from the GET capability. Native
// effect interpretation remains with the adapter: even a successful HTTP status
// is only a response, and every transport failure can follow a committed write.
struct ScopedWrite(ScopedHttp);
#[async_trait]
impl AuthenticatedWrite for ScopedWrite {
    async fn put_json(
        self: Box<Self>,
        segments: &[&str],
        query: &[(&str, String)],
        body: &serde_json::Value,
    ) -> Result<HttpResponse> {
        let response = self
            .0
            .request(reqwest::Method::PUT, segments, query)
            .await?
            .json(body)
            .send()
            .await
            .map_err(provider_error)?;
        let status = response.status().as_u16();
        let mut headers = std::collections::BTreeMap::new();
        let mut bytes = 0_usize;
        for (index, (name, value)) in response.headers().iter().enumerate() {
            bytes = bytes
                .saturating_add(name.as_str().len())
                .saturating_add(value.as_bytes().len());
            if index >= 128 || bytes > 32_768 {
                return Err(Error::new(
                    ErrorCode::Capacity,
                    "provider response headers exceed limit",
                ));
            }
            if let Ok(value) = value.to_str() {
                headers.insert(name.to_string(), value.to_owned());
            }
        }
        let body = connectors_client::bounded(response).await?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

fn provider_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::new(ErrorCode::Timeout, "provider request timed out")
    } else {
        Error::unavailable()
    }
}

#[async_trait]
impl AuthenticatedHttp for ScopedHttp {
    async fn get(&self, segments: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
        let response = self.send_get(segments, query).await?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_owned())))
            .collect();
        let body = connectors_client::bounded(response).await?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }

    async fn get_prefix(
        &self,
        segments: &[&str],
        query: &[(&str, String)],
        limit: usize,
    ) -> Result<HttpResponsePrefix> {
        if !(1..=1_048_576).contains(&limit) {
            return Err(Error::invalid(
                "response prefix limit is outside supported bounds",
            ));
        }
        let mut response = self.send_get(segments, query).await?;
        let status = response.status().as_u16();
        let mut headers = std::collections::BTreeMap::new();
        let mut header_bytes = 0_usize;
        for (index, (name, value)) in response.headers().iter().enumerate() {
            header_bytes = header_bytes.saturating_add(name.as_str().len());
            header_bytes = header_bytes.saturating_add(value.as_bytes().len());
            if index >= 128 || header_bytes > 32_768 {
                return Err(Error::new(
                    ErrorCode::Capacity,
                    "provider response headers exceed limit",
                ));
            }
            if let Ok(value) = value.to_str() {
                headers.insert(name.to_string(), value.to_owned());
            }
        }
        let mut body = Vec::with_capacity(limit);
        let complete = loop {
            let Some(chunk) = response.chunk().await.map_err(provider_error)? else {
                break true;
            };
            let keep = chunk.len().min(limit - body.len());
            body.extend_from_slice(&chunk[..keep]);
            if keep < chunk.len() {
                break false;
            }
            // At the limit, another read is still required: exact length (or a
            // Content-Length header) does not establish a successfully read EOF.
        };
        Ok(HttpResponsePrefix {
            status,
            headers,
            body,
            complete,
        })
    }
}

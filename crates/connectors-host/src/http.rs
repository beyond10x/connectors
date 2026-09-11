use crate::credentials::CredentialRef;
use async_trait::async_trait;
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::{
    AuthProbe, AuthenticatedHttp, AuthenticatedWrite, Credential, HttpResponse, HttpResponsePrefix,
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

    /// Trusted composition fixes the probe's path here, once. The returned port
    /// POSTs a bounded document to exactly those segments under the captured
    /// target, TLS configuration and credential, and can express nothing else.
    /// A business adapter holding only `AuthenticatedHttp` cannot build one.
    ///
    /// ```compile_fail
    /// use connectors_sdk::AuthenticatedHttp;
    /// fn probe(http: &dyn AuthenticatedHttp) {
    ///     http.probe_capability(&["apis"]);
    /// }
    /// ```
    pub fn probe_capability(&self, segments: &[&str]) -> Result<Arc<dyn AuthProbe>> {
        if segments.is_empty() || segments.len() > 16 {
            return Err(Error::invalid("a probe endpoint is one bounded fixed path"));
        }
        let mut fixed = Vec::with_capacity(segments.len());
        for segment in segments {
            if segment.is_empty() || *segment == "." || *segment == ".." {
                return Err(Error::invalid("invalid provider path segment"));
            }
            fixed.push((*segment).to_owned());
        }
        Ok(Arc::new(ScopedProbe {
            http: Self {
                client: self.client.clone(),
                base: self.base.clone(),
                credential: self.credential.clone(),
                header: self.header.clone(),
                bearer: self.bearer,
            },
            segments: fixed,
        }))
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

// Deliberately private and separate from both the GET and the consuming PUT
// capability. The path is fixed at construction, so no caller can retarget it.
struct ScopedProbe {
    http: ScopedHttp,
    segments: Vec<String>,
}
#[async_trait]
impl AuthProbe for ScopedProbe {
    async fn probe(&self, body: &serde_json::Value) -> Result<HttpResponse> {
        let document = serde_json::to_vec(body).map_err(|_| Error::internal())?;
        if document.len() > connectors_sdk::PROBE_BODY_LIMIT {
            return Err(Error::new(
                ErrorCode::Capacity,
                "probe document exceeds the permitted size",
            ));
        }
        let segments: Vec<&str> = self.segments.iter().map(String::as_str).collect();
        let response = self
            .http
            .request(reqwest::Method::POST, &segments, &[])
            .await?
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(document)
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

#[cfg(test)]
mod tests {
    use super::*;
    use connectors_sdk::PROBE_BODY_LIMIT;

    /// A base that resolves nowhere, so any test reaching I/O fails with
    /// `Unavailable` rather than the refusal the case is asserting.
    fn scoped() -> ScopedHttp {
        ScopedHttp::new_with_ca_bytes(
            &HttpConfig {
                base_url: "https://probe.invalid/".into(),
                credential: None,
                credential_header: "authorization".into(),
                bearer: true,
                allow_plaintext: false,
                ca_file: None,
            },
            None,
            None,
        )
        .unwrap()
    }

    #[test]
    fn a_probe_endpoint_is_one_bounded_fixed_path() {
        let http = scoped();
        let refused = |result: Result<Arc<dyn AuthProbe>>, what: &str| match result {
            Ok(_) => panic!("{what} was accepted as a probe endpoint"),
            Err(error) => assert_eq!(error.code, ErrorCode::InvalidInput, "{what}"),
        };
        refused(http.probe_capability(&[]), "an empty path");
        refused(http.probe_capability(&["a"; 17]), "a 17-segment path");
        for segment in ["", ".", ".."] {
            refused(http.probe_capability(&["apis", segment]), segment);
        }
        assert!(http.probe_capability(&["apis", "v1", "reviews"]).is_ok());
        assert!(http.probe_capability(&["a"; 16]).is_ok());
    }

    #[tokio::test]
    async fn an_oversized_probe_document_is_refused_without_any_request() {
        let http = scoped();
        let probe = http.probe_capability(&["apis", "v1", "reviews"]).unwrap();
        // One byte over the limit once serialized: the quotes carry two bytes.
        let oversized = serde_json::json!("x".repeat(PROBE_BODY_LIMIT - 1));
        assert_eq!(
            serde_json::to_vec(&oversized).unwrap().len(),
            PROBE_BODY_LIMIT + 1
        );
        let Err(error) = probe.probe(&oversized).await else {
            panic!("an oversized probe document was sent");
        };
        // Capacity, not Unavailable: the refusal precedes the transport, which
        // could not have reached this unresolvable base in any case.
        assert_eq!(error.code, ErrorCode::Capacity);
        let largest = serde_json::json!("x".repeat(PROBE_BODY_LIMIT - 2));
        assert_eq!(
            serde_json::to_vec(&largest).unwrap().len(),
            PROBE_BODY_LIMIT
        );
        let Err(error) = probe.probe(&largest).await else {
            panic!("an unresolvable base produced a response");
        };
        assert_eq!(
            error.code,
            ErrorCode::Unavailable,
            "a document at the limit must be sent, not refused"
        );
    }
}

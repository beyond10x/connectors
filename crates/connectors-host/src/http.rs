use crate::credentials::CredentialRef;
use async_trait::async_trait;
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::{AuthenticatedHttp, Credential, HttpResponse};
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

#[async_trait]
impl AuthenticatedHttp for ScopedHttp {
    async fn get(&self, segments: &[&str], query: &[(&str, String)]) -> Result<HttpResponse> {
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
        let mut request = self.client.get(url);
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
        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                Error::new(ErrorCode::Timeout, "provider request timed out")
            } else {
                Error::unavailable()
            }
        })?;
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
}

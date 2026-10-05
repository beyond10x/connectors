use crate::credentials::CredentialRef;
use async_trait::async_trait;
use connectors_core::{Error, ErrorCode, Result};
use connectors_sdk::{
    AuthProbe, AuthenticatedHttp, AuthenticatedWrite, Credential, HttpResponse, HttpResponsePrefix,
    WriteMethod,
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
    /// The exact paths `post_json` may reach, fixed by trusted composition.
    read_posts: Arc<std::collections::BTreeSet<Vec<String>>>,
    read_post_timeout: Duration,
}

/// The largest JSON body a read sent as a POST may carry.
pub const READ_POST_BODY_LIMIT: usize = 64 * 1024;
/// The longest deadline composition may give a read sent as a POST.
pub const READ_POST_MAX_TIMEOUT: Duration = Duration::from_secs(180);
const READ_POST_DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);

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
        // reqwest carries no crypto provider of its own; ring is the one this workspace selects.
        let _ = rustls::crypto::ring::default_provider().install_default();
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
            builder = builder.tls_certs_only(certificates);
        }
        Ok(Self {
            client: builder.build().map_err(|_| Error::internal())?,
            base,
            credential,
            header,
            bearer: config.bearer,
            read_posts: Arc::default(),
            read_post_timeout: READ_POST_DEFAULT_TIMEOUT,
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
            read_posts: self.read_posts.clone(),
            read_post_timeout: self.read_post_timeout,
        }
    }
}

impl ScopedHttp {
    /// Trusted composition selects a single consuming write capability, one PUT
    /// or one POST. This conversion grants no approval or ledger authority.
    /// Business adapters receiving only `AuthenticatedHttp` cannot perform it.
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
                read_posts: self.read_posts.clone(),
                read_post_timeout: self.read_post_timeout,
            },
            segments: fixed,
        }))
    }

    /// Trusted composition only. Fixes, once, the exact paths this port may
    /// POST to as reads (`AuthenticatedHttp::post_json`) and their deadline,
    /// from the operations the adapter's descriptor declares reads sent as a
    /// POST. A business adapter holding only `AuthenticatedHttp` cannot widen
    /// them.
    ///
    /// ```compile_fail
    /// use connectors_sdk::AuthenticatedHttp;
    /// fn widen(http: &dyn AuthenticatedHttp) {
    ///     http.with_read_posts(&[&["search"]], std::time::Duration::from_secs(1));
    /// }
    /// ```
    pub fn with_read_posts(&self, paths: &[&[&str]], timeout: Duration) -> Result<Self> {
        if paths.is_empty() || paths.len() > 16 {
            return Err(Error::invalid(
                "read POSTs are between one and 16 fixed paths",
            ));
        }
        if timeout.is_zero() || timeout > READ_POST_MAX_TIMEOUT {
            return Err(Error::invalid(
                "read POST deadline is outside supported bounds",
            ));
        }
        let mut fixed = std::collections::BTreeSet::new();
        for segments in paths {
            if segments.is_empty() || segments.len() > 16 {
                return Err(Error::invalid(
                    "a read POST endpoint is one bounded fixed path",
                ));
            }
            if segments
                .iter()
                .any(|s| s.is_empty() || *s == "." || *s == "..")
            {
                return Err(Error::invalid("invalid provider path segment"));
            }
            fixed.insert(segments.iter().map(|s| (*s).to_owned()).collect());
        }
        Ok(Self {
            client: self.client.clone(),
            base: self.base.clone(),
            credential: self.credential.clone(),
            header: self.header.clone(),
            bearer: self.bearer,
            read_posts: Arc::new(fixed),
            read_post_timeout: timeout,
        })
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

    /// Trusted composition only. One `application/x-www-form-urlencoded` POST
    /// to fixed segments under the captured target and TLS configuration, for
    /// a request whose form is itself the credential, such as an OAuth token
    /// exchange. It never carries this capability's credential header. The
    /// encoded form is sized once and zeroized when the transport releases it;
    /// the response is bounded like a write's.
    pub async fn post_form(
        &self,
        segments: &[&str],
        form: &[(&str, &[u8])],
    ) -> Result<HttpResponse> {
        let body = form_body(form)?;
        if segments.is_empty() {
            return Err(Error::invalid("a form endpoint is one fixed path"));
        }
        let url = self.url(segments)?;
        let response = self
            .client
            .post(url)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .header(reqwest::header::ACCEPT, "application/json")
            // The transport holds the owner, and drops it when the body is sent.
            .body(reqwest::Body::from(axum::body::Bytes::from_owner(body)))
            .send()
            .await
            .map_err(provider_error)?;
        bounded_response(response).await
    }

    fn url(&self, segments: &[&str]) -> Result<Url> {
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
        Ok(url)
    }

    async fn request(
        &self,
        method: reqwest::Method,
        segments: &[&str],
        query: &[(&str, String)],
    ) -> Result<reqwest::RequestBuilder> {
        let mut url = self.url(segments)?;
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
    async fn send_json(
        self: Box<Self>,
        method: WriteMethod,
        segments: &[&str],
        query: &[(&str, String)],
        body: &serde_json::Value,
    ) -> Result<HttpResponse> {
        let method = match method {
            WriteMethod::Post => reqwest::Method::POST,
            WriteMethod::Put => reqwest::Method::PUT,
            WriteMethod::Patch => reqwest::Method::PATCH,
            WriteMethod::Delete => reqwest::Method::DELETE,
        };
        let request = self.0.request(method, segments, query).await?;
        // A null body sends no document at all, which is what a bodiless DELETE
        // declares; any other value is sent as one JSON document.
        let request = if body.is_null() {
            request
        } else {
            request.json(body)
        };
        let response = request.send().await.map_err(provider_error)?;
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
        let body = connectors_client::bounded(response)
            .await
            .map_err(body_error)?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

// Deliberately private and separate from both the GET and the consuming write
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
        let body = connectors_client::bounded(response)
            .await
            .map_err(body_error)?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

/// The largest encoded form `post_form` sends.
pub const FORM_BODY_LIMIT: usize = 64 * 1024;

/// `application/x-www-form-urlencoded`: `*-._` and ASCII alphanumerics as they
/// are, a space as `+`, every other byte `%XX`. The buffer is allocated once at
/// its final size, so no reallocation leaves an unzeroized copy behind.
fn form_body(form: &[(&str, &[u8])]) -> Result<zeroize::Zeroizing<Vec<u8>>> {
    fn plain(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || matches!(byte, b'*' | b'-' | b'.' | b'_' | b' ')
    }
    fn encoded(bytes: &[u8]) -> usize {
        bytes.iter().map(|b| if plain(*b) { 1 } else { 3 }).sum()
    }
    fn encode(body: &mut Vec<u8>, bytes: &[u8]) {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        for &byte in bytes {
            match byte {
                b' ' => body.push(b'+'),
                byte if plain(byte) => body.push(byte),
                byte => body.extend_from_slice(&[
                    b'%',
                    HEX[usize::from(byte >> 4)],
                    HEX[usize::from(byte & 15)],
                ]),
            }
        }
    }
    let length = form
        .iter()
        .map(|(name, value)| encoded(name.as_bytes()) + 1 + encoded(value))
        .sum::<usize>()
        + form.len().saturating_sub(1);
    if length > FORM_BODY_LIMIT {
        return Err(Error::new(
            ErrorCode::Capacity,
            "form exceeds the permitted size",
        ));
    }
    let mut body = zeroize::Zeroizing::new(Vec::with_capacity(length));
    for (index, (name, value)) in form.iter().enumerate() {
        if index > 0 {
            body.push(b'&');
        }
        encode(&mut body, name.as_bytes());
        body.push(b'=');
        encode(&mut body, value);
    }
    Ok(body)
}

/// Status, bounded headers and a bounded body, as a write reads them.
async fn bounded_response(response: reqwest::Response) -> Result<HttpResponse> {
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
    let body = connectors_client::bounded(response)
        .await
        .map_err(body_error)?;
    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}

/// The response's status and headers have arrived, so a deadline that passes
/// while its body is read is the provider's timeout, like `provider_error`'s.
fn body_error(error: Error) -> Error {
    if error.code == ErrorCode::Timeout {
        error.answered()
    } else {
        error
    }
}

fn provider_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        let timeout = Error::new(ErrorCode::Timeout, "provider request timed out");
        // A connection that never opened sent nothing to the provider.
        if error.is_connect() {
            timeout
        } else {
            timeout.answered()
        }
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
        let body = connectors_client::bounded(response)
            .await
            .map_err(body_error)?;
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

    async fn post_json(
        &self,
        segments: &[&str],
        query: &[(&str, String)],
        body: &serde_json::Value,
    ) -> Result<HttpResponse> {
        if self.read_posts.is_empty() {
            return Err(Error::unavailable());
        }
        let path: Vec<String> = segments.iter().map(|s| (*s).to_owned()).collect();
        if !self.read_posts.contains(&path) {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "path is not a read this port may send as a POST",
            ));
        }
        let bytes = serde_json::to_vec(body).map_err(|_| Error::internal())?;
        if bytes.len() > READ_POST_BODY_LIMIT {
            return Err(Error::invalid("read POST body exceeds limit"));
        }
        let response = self
            .request(reqwest::Method::POST, segments, query)
            .await?
            .timeout(self.read_post_timeout)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::ACCEPT, "application/json")
            .body(bytes)
            .send()
            .await
            .map_err(provider_error)?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_owned())))
            .collect();
        let body = connectors_client::bounded(response)
            .await
            .map_err(body_error)?;
        Ok(HttpResponse {
            status,
            headers,
            body,
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
    async fn a_sent_request_whose_deadline_passes_is_marked_as_the_providers_timeout() {
        // A provider that accepts the request and never answers.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_secs(5)).await;
            drop(socket);
        });
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_millis(200))
            .build()
            .unwrap();
        let error = client
            .post(format!("http://{address}/write"))
            .body("{}")
            .send()
            .await
            .unwrap_err();
        let error = provider_error(error);
        assert_eq!(error.code, ErrorCode::Timeout);
        assert!(error.upstream_answer, "a sent request's timeout");
        server.abort();
        // Nothing listening: no request was sent, so no provider answered.
        let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = closed.local_addr().unwrap();
        drop(closed);
        let error = client
            .get(format!("http://{address}/"))
            .send()
            .await
            .unwrap_err();
        let error = provider_error(error);
        assert_eq!(error.code, ErrorCode::Unavailable);
        assert!(!error.upstream_answer);
    }

    struct FixtureCredential;
    #[async_trait]
    impl Credential for FixtureCredential {
        async fn resolve(&self) -> Result<connectors_sdk::Secret> {
            Ok(connectors_sdk::Secret(b"fixture-credential".to_vec()))
        }
    }

    #[tokio::test]
    async fn a_form_post_sends_the_encoded_form_and_never_the_credential_header() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                head.push(socket.read_u8().await.unwrap());
            }
            let head = String::from_utf8(head).unwrap();
            let length: usize = head
                .lines()
                .find_map(|line| {
                    line.split_once(':')
                        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                        .map(|(_, value)| value.trim().parse().unwrap())
                })
                .unwrap();
            let mut body = vec![0; length];
            socket.read_exact(&mut body).await.unwrap();
            let answer = br#"{"answered":true}"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answer.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            socket.write_all(answer).await.unwrap();
            (head, body)
        });
        let http = ScopedHttp::new_with_ca_bytes(
            &HttpConfig {
                base_url: format!("http://{address}/"),
                credential: None,
                credential_header: "authorization".into(),
                bearer: true,
                allow_plaintext: true,
                ca_file: None,
            },
            None,
            None,
        )
        .unwrap()
        .with_credential(Arc::new(FixtureCredential));
        let response = http
            .post_form(
                &["oauth", "token"],
                &[
                    ("grant_type", b"refresh_token".as_slice()),
                    ("value", b"a b&c=d/\xff~*-._Z9".as_slice()),
                ],
            )
            .await
            .unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, br#"{"answered":true}"#);
        let (head, body) = server.await.unwrap();
        assert!(head.starts_with("POST /oauth/token HTTP/1.1\r\n"), "{head}");
        let lower = head.to_ascii_lowercase();
        assert!(!lower.contains("authorization:"), "credential header sent");
        assert!(!head.contains("fixture-credential"));
        assert!(lower.contains("content-type: application/x-www-form-urlencoded\r\n"));
        assert_eq!(
            body,
            b"grant_type=refresh_token&value=a+b%26c%3Dd%2F%FF%7E*-._Z9".as_slice()
        );
    }

    #[tokio::test]
    async fn an_oversized_or_unaddressed_form_is_refused_without_any_request() {
        let http = scoped();
        let large = vec![b'x'; FORM_BODY_LIMIT];
        let Err(error) = http.post_form(&["token"], &[("v", large.as_slice())]).await else {
            panic!("an oversized form was sent");
        };
        assert_eq!(error.code, ErrorCode::Capacity);
        for segments in [&[][..], &[""][..], &[".."][..]] {
            let Err(error) = http.post_form(segments, &[("v", b"x".as_slice())]).await else {
                panic!("an unaddressed form was sent");
            };
            assert_eq!(error.code, ErrorCode::InvalidInput);
        }
        // Within the limit the form is sent, and this base resolves nowhere.
        let fits = vec![b'x'; FORM_BODY_LIMIT - 2];
        let Err(error) = http.post_form(&["token"], &[("v", fits.as_slice())]).await else {
            panic!("an unresolvable base produced a response");
        };
        assert_eq!(error.code, ErrorCode::Unavailable);
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

    fn refused(result: Result<HttpResponse>) -> Error {
        match result {
            Ok(_) => panic!("the read POST was answered"),
            Err(error) => error,
        }
    }

    #[tokio::test]
    async fn a_read_post_reaches_only_the_paths_composition_fixed() {
        let body = serde_json::json!({"query": "q"});
        // Composed without read POSTs: unavailable, no I/O.
        let none = refused(scoped().post_json(&["search"], &[], &body).await);
        assert_eq!(none.code, ErrorCode::Unavailable);
        assert!(!none.upstream_answer);

        let http = scoped()
            .with_read_posts(&[&["search"], &["v1", "crawl"]], Duration::from_secs(60))
            .unwrap();
        for path in [&["extract"][..], &["search", "x"], &["v1"], &["crawl"]] {
            let error = refused(http.post_json(path, &[], &body).await);
            assert_eq!(error.code, ErrorCode::Forbidden, "{path:?}");
        }
        let big = serde_json::json!({"query": "q".repeat(READ_POST_BODY_LIMIT)});
        let error = refused(http.post_json(&["search"], &[], &big).await);
        assert_eq!(error.code, ErrorCode::InvalidInput);
        // A fixed path within the limit is sent; the base resolves nowhere.
        let error = refused(http.post_json(&["search"], &[], &body).await);
        assert_eq!(error.code, ErrorCode::Unavailable);
    }

    #[test]
    fn read_post_composition_is_bounded() {
        let http = scoped();
        let minute = Duration::from_secs(60);
        for (what, paths, timeout) in [
            ("no path", &[][..], minute),
            ("empty path", &[&[][..]][..], minute),
            ("dot segment", &[&[".."][..]][..], minute),
            ("zero deadline", &[&["search"][..]][..], Duration::ZERO),
            (
                "long deadline",
                &[&["search"][..]][..],
                READ_POST_MAX_TIMEOUT + Duration::from_secs(1),
            ),
        ] {
            match http.with_read_posts(paths, timeout) {
                Ok(_) => panic!("{what} was accepted"),
                Err(error) => assert_eq!(error.code, ErrorCode::InvalidInput, "{what}"),
            }
        }
        assert!(
            http.with_read_posts(&[&["search"]], READ_POST_MAX_TIMEOUT)
                .is_ok()
        );
    }

    #[tokio::test]
    async fn a_read_post_sends_json_with_the_credential_and_bounds_the_answer() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                head.push(socket.read_u8().await.unwrap());
            }
            let head = String::from_utf8(head).unwrap();
            let length: usize = head
                .lines()
                .find_map(|line| {
                    line.split_once(':')
                        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                        .map(|(_, value)| value.trim().parse().unwrap())
                })
                .unwrap();
            let mut body = vec![0; length];
            socket.read_exact(&mut body).await.unwrap();
            let answer = br#"{"results":[]}"#;
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        answer.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            socket.write_all(answer).await.unwrap();
            (head, body)
        });
        let http = ScopedHttp::new_with_ca_bytes(
            &HttpConfig {
                base_url: format!("http://{address}/"),
                credential: None,
                credential_header: "authorization".into(),
                bearer: true,
                allow_plaintext: true,
                ca_file: None,
            },
            None,
            None,
        )
        .unwrap()
        .with_credential(Arc::new(FixtureCredential))
        .with_read_posts(&[&["search"]], Duration::from_secs(30))
        .unwrap();
        let response = http
            .post_json(&["search"], &[], &serde_json::json!({"query": "rust"}))
            .await
            .unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, br#"{"results":[]}"#);
        let (head, body) = server.await.unwrap();
        // The empty query keeps its `?`, as every request through `request` does.
        assert!(head.starts_with("POST /search? HTTP/1.1\r\n"), "{head}");
        let lower = head.to_ascii_lowercase();
        assert!(
            lower.contains("authorization: bearer "),
            "credential header missing"
        );
        assert!(lower.contains("content-type: application/json\r\n"));
        assert_eq!(body, br#"{"query":"rust"}"#);
    }
}

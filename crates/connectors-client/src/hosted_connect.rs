//! Principal-owned hosted token acquisition through the existing one-use completion route.

use std::fs::{Metadata, OpenOptions};
use std::future::Future;
use std::io::Read as _;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::Path;
use std::time::Duration;

use protocol::{connection as c, operation::OwnerContext};
use serde::Deserialize;
use url::Url;
use zeroize::Zeroizing;

use crate::{endpoint, personal_oauth::oauth_now, ClientError, HostedClient};

const MAX_CREDENTIAL_BYTES: usize = 8 * 1024;
const MAX_SESSION_TTL_MS: u64 = 900_000;
const MAX_COMPLETION_RESPONSE_BYTES: usize = 1024;

impl HostedClient {
    /// Acquire one hosted token Connection as an already-authorized Identity principal.
    ///
    /// The caller must select a declared token profile with one secret credential entry. Browser
    /// OAuth and native flows with multiple input fields retain their declared acquisition routes;
    /// this protocol client does not embed a catalog. Runtime admission remains authoritative.
    /// The short-lived bearer must admit `connectors.connections.self` and
    /// `connectors.catalog.read`. The server derives ownership from Identity; the request context
    /// is not an ownership override. This does not require a desktop login or OS keyring.
    pub async fn connect_with_credential_file(
        &self,
        bearer: &str,
        context: &OwnerContext,
        request: c::ConnectSessionCreateRequest,
        credential_file: &Path,
    ) -> Result<c::ConnectionDescription, ClientError> {
        connect(self, request, credential_file, |request| {
            self.connection(bearer, context, request)
        })
        .await
    }

    /// Submit a credential once to a Connector-issued, principal-owned token session.
    ///
    /// Consumes the capability-bearing status. Its exact route must belong to this client's API
    /// base. No Identity bearer is forwarded to completion, redirects and retries are disabled,
    /// and success acknowledges submission only; confirm the owner-scoped session status next.
    /// An unconfirmed response must not cause automatic resubmission.
    pub async fn complete_connect_session(
        &self,
        session: c::ConnectSessionStatus,
        credential: Zeroizing<Vec<u8>>,
    ) -> Result<(), ClientError> {
        validate_credential(&credential)?;
        let pending = completion_endpoint(&self.base, session)?;
        // The only non-zeroizing copy is handed directly to the HTTP transport, which owns its
        // request buffers. The file input and capability remain in zeroizing allocations.
        let mut response = self
            .http
            .post(pending.url)
            .timeout(pending.remaining.min(Duration::from_secs(35)))
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .header("X-Connect-Session", pending.capability.as_str())
            .body(credential.to_vec())
            .send()
            .await
            .map_err(|_| ClientError::CompletionUnconfirmed)?;
        match response.status() {
            reqwest::StatusCode::BAD_REQUEST
            | reqwest::StatusCode::FORBIDDEN
            | reqwest::StatusCode::NOT_FOUND => return Err(ClientError::CompletionRefused),
            reqwest::StatusCode::OK => {}
            _ => return Err(ClientError::CompletionUnconfirmed),
        }
        if response.headers().get(reqwest::header::CACHE_CONTROL)
            != Some(&reqwest::header::HeaderValue::from_static("no-store"))
            || response
                .content_length()
                .is_some_and(|length| length > MAX_COMPLETION_RESPONSE_BYTES as u64)
        {
            return Err(ClientError::CompletionUnconfirmed);
        }
        let mut bytes = Zeroizing::new(Vec::with_capacity(MAX_COMPLETION_RESPONSE_BYTES));
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| ClientError::CompletionUnconfirmed)?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_COMPLETION_RESPONSE_BYTES {
                return Err(ClientError::CompletionUnconfirmed);
            }
            bytes.extend_from_slice(&chunk);
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Acknowledgement {
            accepted: bool,
        }
        if !serde_json::from_slice::<Acknowledgement>(&bytes).is_ok_and(|result| result.accepted) {
            return Err(ClientError::CompletionUnconfirmed);
        }
        Ok(())
    }
}

pub(super) async fn connect<F, Fut, E>(
    hosted: &HostedClient,
    request: c::ConnectSessionCreateRequest,
    credential_file: &Path,
    mut exchange: F,
) -> Result<c::ConnectionDescription, E>
where
    F: FnMut(c::ConnectionRequest) -> Fut,
    Fut: Future<Output = Result<c::ResponseEnvelope, E>>,
    E: From<ClientError>,
{
    let profile = request
        .auth_profile
        .clone()
        .filter(|profile| profile.starts_with(&format!("{}.", request.integration_ref)))
        .ok_or(ClientError::HostedConnectProfile)?;
    let credential = read_credential_file(credential_file)?;
    let provider = request.integration_ref.clone();
    let created = exchange(c::ConnectionRequest::ConnectSessionCreate(request)).await?;
    let c::ConnectionResult::ConnectSessionCreate(created) = connection_result(created)? else {
        return Err(ClientError::InvalidResponse.into());
    };
    if created.integration_ref != provider {
        return Err(ClientError::InvalidResponse.into());
    }
    let session_ref = created.connect_session_ref.clone();
    let expires_at = created.expires_at_unix_ms;
    hosted.complete_connect_session(created, credential).await?;
    // Completion is synchronous in the hosted contract. A successful POST alone is insufficient:
    // Status and Describe must agree before a caller receives a connected result. Failure after
    // submission is explicitly uncertain and never retries the submission or creates a session.
    let status = exchange(c::ConnectionRequest::ConnectSessionStatus(
        c::ConnectSessionStatusRequest {
            connect_session_ref: session_ref.clone(),
        },
    ))
    .await
    .map_err(|_| ClientError::CompletionUnconfirmed)?;
    let c::ConnectionResult::ConnectSessionStatus(status) =
        connection_result(status).map_err(|_| ClientError::CompletionUnconfirmed)?
    else {
        return Err(ClientError::CompletionUnconfirmed.into());
    };
    if status.connect_session_ref != session_ref
        || status.integration_ref != provider
        || status.expires_at_unix_ms != expires_at
        || status.state != c::ConnectSessionState::Completed
        || status.completion_endpoint.is_some()
        || status.browser_completion_url.is_some()
    {
        return Err(ClientError::CompletionUnconfirmed.into());
    }
    let connection_ref = status
        .connection_ref
        .ok_or(ClientError::CompletionUnconfirmed)?;
    let described = exchange(c::ConnectionRequest::Describe(c::DescribeRequest {
        connection_ref: connection_ref.clone(),
    }))
    .await
    .map_err(|_| ClientError::CompletionUnconfirmed)?;
    let c::ConnectionResult::Describe(description) =
        connection_result(described).map_err(|_| ClientError::CompletionUnconfirmed)?
    else {
        return Err(ClientError::CompletionUnconfirmed.into());
    };
    if description.summary.connection_ref != connection_ref
        || description.summary.integration_ref != provider
        || description.summary.auth_profile.as_deref() != Some(profile.as_str())
        || description.summary.state != c::ConnectionState::Callable
        || description.summary.scope != Some(c::ConnectionScope::Principal)
    {
        return Err(ClientError::CompletionUnconfirmed.into());
    }
    Ok(description)
}

fn connection_result(response: c::ResponseEnvelope) -> Result<c::ConnectionResult, ClientError> {
    // Do not relay server error text into a credential acquisition diagnostic.
    match (response.status, response.response, response.error) {
        (c::ResponseStatus::Ok, Some(result), None) => Ok(result),
        (c::ResponseStatus::Error, None, Some(_)) => Err(ClientError::CompletionRefused),
        _ => Err(ClientError::InvalidResponse),
    }
}

struct PendingCompletion {
    url: Url,
    capability: Zeroizing<String>,
    remaining: Duration,
}

fn completion_endpoint(
    base: &Url,
    mut session: c::ConnectSessionStatus,
) -> Result<PendingCompletion, ClientError> {
    let raw = Zeroizing::new(
        session
            .browser_completion_url
            .take()
            .ok_or(ClientError::UnsafeHostedCompletion)?,
    );
    let remaining = session
        .expires_at_unix_ms
        .checked_sub(oauth_now()?)
        .filter(|remaining| (1..=MAX_SESSION_TTL_MS).contains(remaining))
        .ok_or(ClientError::UnsafeHostedCompletion)?;
    if session.state != c::ConnectSessionState::Pending
        || session.connection_ref.is_some()
        || session.completion_endpoint.is_some()
        || session.connect_session_ref.is_empty()
        || session.connect_session_ref.len() > 256
        || !session
            .connect_session_ref
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-' | b'_'))
        || raw.len() > 4096
    {
        return Err(ClientError::UnsafeHostedCompletion);
    }
    let (destination, capability) = raw
        .split_once("#token=")
        .ok_or(ClientError::UnsafeHostedCompletion)?;
    if !(32..=256).contains(&capability.len())
        || !capability
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ClientError::UnsafeHostedCompletion);
    }
    let mut expected = endpoint(base, "connect-sessions");
    expected
        .path_segments_mut()
        .map_err(|_| ClientError::UnsafeHostedCompletion)?
        .push(&session.connect_session_ref);
    // Compare the exact serialization, before parsing any caller-controlled URL. This rejects
    // a foreign origin, query, fragment, userinfo, path traversal, OAuth route or alternate port.
    if destination != expected.as_str() {
        return Err(ClientError::UnsafeHostedCompletion);
    }
    Ok(PendingCompletion {
        url: expected,
        capability: Zeroizing::new(capability.to_owned()),
        remaining: Duration::from_millis(remaining),
    })
}

fn read_credential_file(path: &Path) -> Result<Zeroizing<Vec<u8>>, ClientError> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(path)
        .map_err(|_| ClientError::UnsafeCredentialFile)?;
    let metadata = file
        .metadata()
        .map_err(|_| ClientError::UnsafeCredentialFile)?;
    validate_file(&metadata, rustix::process::geteuid().as_raw())?;
    let mut bytes = Zeroizing::new(Vec::with_capacity(MAX_CREDENTIAL_BYTES + 1));
    (&mut file)
        .take((MAX_CREDENTIAL_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ClientError::UnsafeCredentialFile)?;
    validate_credential(&bytes)?;
    Ok(bytes)
}

fn validate_file(metadata: &Metadata, owner: u32) -> Result<(), ClientError> {
    if !metadata.file_type().is_file()
        || metadata.uid() != owner
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.len() > MAX_CREDENTIAL_BYTES as u64
    {
        return Err(ClientError::UnsafeCredentialFile);
    }
    Ok(())
}

fn validate_credential(bytes: &[u8]) -> Result<(), ClientError> {
    if bytes.len() > MAX_CREDENTIAL_BYTES
        || !std::str::from_utf8(bytes).is_ok_and(|value| !value.trim().is_empty())
    {
        return Err(ClientError::UnsafeCredentialFile);
    }
    Ok(())
}

#[cfg(test)]
#[path = "hosted_connect_tests.rs"]
mod tests;

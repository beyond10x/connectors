//! GitLab-backed, short-lived Smart HTTP fetch sessions.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use connector_secrets::Secret;
use serde::Deserialize;
use service::{
    EgressByteStream, EgressStreamingHttpRequest, EgressTransportError, EgressTransportFailure,
    GitFetchAccess, GitFetchBroker, GitFetchControlError, GitFetchDataError, GitFetchExchange,
    GitFetchExchangeResponse, GitFetchGrant, GitFetchService, PrincipalContext,
};
use sha2::{Digest as _, Sha256};
use zeroize::Zeroizing;

#[path = "git_fetch_advertisement.rs"]
mod advertisement;
use advertisement::ExactAdvertisementStream;

use super::{
    bearer_headers, http_request, lock, now_ms, random_token, GitlabBackend, GitlabInner,
    StoredConnection, MAX_PROVIDER_RESPONSE_BYTES,
};

const SESSION_TTL_MS: u64 = 15 * 60 * 1_000;
const SESSION_REQUEST_LIMIT: u8 = 32;
const SESSION_BYTE_LIMIT: u64 = 1024 * 1024 * 1024;
const MAX_SESSIONS: usize = 1_024;
const MAX_SESSIONS_PER_PRINCIPAL: usize = 8;
const SOURCE_NAME: &str = "gitlab";
const REPOSITORY_NAME: &str = "repository.git";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitFetchSessionState {
    Active,
    UploadInFlight,
    Spent,
    Revoked,
}

impl GitFetchSessionState {
    const fn is_live(self) -> bool {
        matches!(self, Self::Active | Self::UploadInFlight)
    }
}

pub(super) struct GitFetchSessionRecord {
    idempotency_key: String,
    owner_subject: String,
    connection_ref: String,
    project_id: u64,
    reference: String,
    expected_commit: String,
    depth: u8,
    authority_sha256: [u8; 32],
    generation: u64,
    expires_at_unix_ms: u64,
    request_count: u8,
    response_bytes: u64,
    state: GitFetchSessionState,
}

#[derive(Deserialize)]
struct Project {
    id: u64,
    path_with_namespace: String,
    default_branch: Option<String>,
}

#[derive(Deserialize)]
struct Branch {
    name: String,
    commit: Commit,
}

#[derive(Deserialize)]
struct Commit {
    id: String,
}

#[derive(Clone, Copy)]
enum ProjectAdmissionError {
    NotGranted,
    Unavailable,
}

#[async_trait]
impl GitFetchBroker for GitlabBackend {
    async fn create(
        &self,
        context: &PrincipalContext,
        request: protocol::git_fetch::CreateRequest,
    ) -> Result<GitFetchGrant, GitFetchControlError> {
        if !request.is_valid() {
            return Err(GitFetchControlError::Invalid);
        }
        self.inner
            .check_context(context)
            .map_err(|_| GitFetchControlError::NotGranted)?;
        let connection = self
            .inner
            .owned_connections(context)
            .into_iter()
            .find(|connection| connection.connection_ref == request.connection_ref)
            .filter(connection_admits_repository)
            .ok_or(GitFetchControlError::NotGranted)?;
        let token = self
            .inner
            .connection_token(&connection)
            .await
            .map_err(|_| GitFetchControlError::Unavailable)?;
        self.inner
            .admit_project(
                &connection,
                &token,
                request.project_id,
                &request.reference,
                &request.expected_commit,
            )
            .await
            .map_err(|error| match error {
                ProjectAdmissionError::NotGranted => GitFetchControlError::NotGranted,
                ProjectAdmissionError::Unavailable => GitFetchControlError::Unavailable,
            })?;
        drop(token);

        let source_origin = self
            .inner
            .policy
            .git_fetch_origin
            .as_deref()
            .ok_or(GitFetchControlError::Unavailable)
            .and_then(|origin| {
                super::parse_origin(origin).map_err(|_| GitFetchControlError::Unavailable)
            })?;
        let session_ref = session_ref(context, &request.idempotency_key);
        let authority =
            Zeroizing::new(random_token(32).map_err(|_| GitFetchControlError::Unavailable)?);
        let authority_sha256 = Sha256::digest(authority.as_bytes()).into();
        let expires_at_unix_ms = now_ms()
            .and_then(|now| now.checked_add(SESSION_TTL_MS))
            .ok_or(GitFetchControlError::Unavailable)?;

        {
            let mut sessions = lock(&self.inner.git_fetch_sessions);
            let generation = if let Some(previous) = sessions.get(&session_ref) {
                if previous.idempotency_key != request.idempotency_key
                    || previous.owner_subject != context.subject()
                    || previous.connection_ref != request.connection_ref
                    || previous.project_id != request.project_id
                    || previous.reference != request.reference
                    || previous.expected_commit != request.expected_commit
                    || previous.depth != request.depth
                {
                    return Err(GitFetchControlError::Conflict);
                }
                if previous.state == GitFetchSessionState::UploadInFlight {
                    return Err(GitFetchControlError::Conflict);
                }
                previous.generation.saturating_add(1)
            } else {
                prune_fetch_sessions(
                    &mut sessions,
                    expires_at_unix_ms.saturating_sub(SESSION_TTL_MS),
                );
                let principal_sessions = sessions
                    .values()
                    .filter(|session| {
                        session.owner_subject == context.subject() && session.state.is_live()
                    })
                    .count();
                if sessions
                    .values()
                    .filter(|session| session.state.is_live())
                    .count()
                    >= MAX_SESSIONS
                    || principal_sessions >= MAX_SESSIONS_PER_PRINCIPAL
                {
                    return Err(GitFetchControlError::Unavailable);
                }
                1
            };
            sessions.insert(
                session_ref.clone(),
                GitFetchSessionRecord {
                    idempotency_key: request.idempotency_key.clone(),
                    owner_subject: context.subject().to_owned(),
                    connection_ref: request.connection_ref.clone(),
                    project_id: request.project_id,
                    reference: request.reference.clone(),
                    expected_commit: request.expected_commit.clone(),
                    depth: request.depth,
                    authority_sha256,
                    generation,
                    expires_at_unix_ms,
                    request_count: 0,
                    response_bytes: 0,
                    state: GitFetchSessionState::Active,
                },
            );
        }

        let mut locator = source_origin;
        locator
            .path_segments_mut()
            .map_err(|_| GitFetchControlError::Unavailable)?
            .extend([
                "internal",
                "git-fetch",
                session_ref.as_str(),
                REPOSITORY_NAME,
            ]);
        Ok(GitFetchGrant::admitted(
            session_ref,
            SOURCE_NAME.to_owned(),
            locator.to_string(),
            &request,
            expires_at_unix_ms,
            authority,
        ))
    }

    async fn exchange(
        &self,
        request: GitFetchExchange,
    ) -> Result<GitFetchExchangeResponse, GitFetchDataError> {
        if request.repository != REPOSITORY_NAME
            || request.git_protocol.is_some()
            || matches!(request.service, GitFetchService::Discovery) && request.body.is_some()
            || matches!(request.service, GitFetchService::UploadPack)
                && request.body.as_ref().is_none_or(Vec::is_empty)
        {
            return Err(GitFetchDataError::Refused);
        }
        self.authorize(&GitFetchAccess {
            session_ref: request.session_ref.clone(),
            repository: request.repository.clone(),
            source_authorization: Zeroizing::new(request.source_authorization.to_string()),
            service: request.service,
            git_protocol: request.git_protocol.clone(),
        })?;
        let authority_sha256: [u8; 32] =
            Sha256::digest(request.source_authorization.as_bytes()).into();
        let now = now_ms().ok_or(GitFetchDataError::Unavailable)?;
        let (
            connection_ref,
            owner_subject,
            project_id,
            reference,
            expected_commit,
            depth,
            left,
            generation,
            expires_at_unix_ms,
        ) = {
            let mut sessions = lock(&self.inner.git_fetch_sessions);
            let session = sessions
                .get_mut(&request.session_ref)
                .ok_or(GitFetchDataError::Refused)?;
            if !constant_time_eq(&session.authority_sha256, &authority_sha256)
                || session.expires_at_unix_ms <= now
                || session.request_count >= SESSION_REQUEST_LIMIT
                || session.response_bytes >= SESSION_BYTE_LIMIT
                || session.state != GitFetchSessionState::Active
            {
                if session.expires_at_unix_ms <= now
                    || session.request_count >= SESSION_REQUEST_LIMIT
                    || session.response_bytes >= SESSION_BYTE_LIMIT
                {
                    session.state = GitFetchSessionState::Revoked;
                }
                return Err(GitFetchDataError::Refused);
            }
            session.request_count = session.request_count.saturating_add(1);
            if request.service == GitFetchService::UploadPack {
                session.state = GitFetchSessionState::UploadInFlight;
            }
            (
                session.connection_ref.clone(),
                session.owner_subject.clone(),
                session.project_id,
                session.reference.clone(),
                session.expected_commit.clone(),
                session.depth,
                SESSION_BYTE_LIMIT.saturating_sub(session.response_bytes),
                session.generation,
                session.expires_at_unix_ms,
            )
        };

        if request.service == GitFetchService::UploadPack
            && !valid_upload_pack_request(
                request.body.as_deref().expect("shape checked"),
                &expected_commit,
                depth,
            )
        {
            self.inner
                .reset_or_spend_fetch(&request.session_ref, generation, request.service);
            return Err(GitFetchDataError::Refused);
        }

        let connection = lock(&self.inner.metadata)
            .connections
            .iter()
            .find(|connection| {
                connection.connection_ref == connection_ref
                    && connection.owner_subject == owner_subject
                    && connection.grant_ref == self.inner.policy.user_grant_ref
                    && connection_admits_repository(connection)
            })
            .cloned()
            .ok_or_else(|| {
                self.inner
                    .revoke_fetch_session(&request.session_ref, generation);
                GitFetchDataError::Refused
            })?;
        let token = self
            .inner
            .connection_token(&connection)
            .await
            .map_err(|_| {
                self.inner
                    .reset_or_spend_fetch(&request.session_ref, generation, request.service);
                GitFetchDataError::Unavailable
            })?;
        let upstream = self
            .inner
            .admit_project(
                &connection,
                &token,
                project_id,
                &reference,
                &expected_commit,
            )
            .await
            .map_err(|error| {
                match error {
                    ProjectAdmissionError::NotGranted => self
                        .inner
                        .revoke_fetch_session(&request.session_ref, generation),
                    ProjectAdmissionError::Unavailable => self.inner.reset_or_spend_fetch(
                        &request.session_ref,
                        generation,
                        request.service,
                    ),
                }
                match error {
                    ProjectAdmissionError::NotGranted => GitFetchDataError::Refused,
                    ProjectAdmissionError::Unavailable => GitFetchDataError::Unavailable,
                }
            })?;

        let mut target = url::Url::parse(&upstream).map_err(|_| GitFetchDataError::Unavailable)?;
        let mut segments = target
            .path_segments_mut()
            .map_err(|_| GitFetchDataError::Unavailable)?;
        match request.service {
            GitFetchService::Discovery => {
                segments.extend(["info", "refs"]);
            }
            GitFetchService::UploadPack => {
                segments.push("git-upload-pack");
            }
        }
        drop(segments);
        if request.service == GitFetchService::Discovery {
            target
                .query_pairs_mut()
                .append_pair("service", "git-upload-pack");
        }
        let mut headers = bearer_headers(&token);
        let expected_content_type = match request.service {
            GitFetchService::Discovery => "application/x-git-upload-pack-advertisement",
            GitFetchService::UploadPack => "application/x-git-upload-pack-result",
        };
        headers.insert("accept".to_owned(), expected_content_type.to_owned());
        if request.service == GitFetchService::UploadPack {
            headers.insert(
                "content-type".to_owned(),
                "application/x-git-upload-pack-request".to_owned(),
            );
        }
        let upstream = self
            .inner
            .egress
            .execute_stream(
                &connection_ref,
                EgressStreamingHttpRequest {
                    method: match request.service {
                        GitFetchService::Discovery => "GET".to_owned(),
                        GitFetchService::UploadPack => "POST".to_owned(),
                    },
                    url: target.into(),
                    headers,
                    body: request.body,
                    maximum_response_bytes: left,
                    response_headers: vec!["content-type".to_owned()],
                },
            )
            .await
            .map_err(|_| {
                self.inner
                    .reset_or_spend_fetch(&request.session_ref, generation, request.service);
                GitFetchDataError::Unavailable
            })?;
        drop(token);
        if upstream.status != 200 {
            if matches!(upstream.status, 401 | 403 | 404) {
                self.inner
                    .revoke_fetch_session(&request.session_ref, generation);
                return Err(GitFetchDataError::Refused);
            }
            self.inner
                .reset_or_spend_fetch(&request.session_ref, generation, request.service);
            return Err(GitFetchDataError::Unavailable);
        }
        if upstream.header("content-type") != Some(expected_content_type) {
            self.inner
                .reset_or_spend_fetch(&request.session_ref, generation, request.service);
            return Err(GitFetchDataError::Unavailable);
        }
        let budgeted: Box<dyn EgressByteStream> = Box::new(BudgetedGitFetchStream {
            inner: upstream.body,
            backend: self.inner.clone(),
            session_ref: request.session_ref,
            generation,
            expires_at_unix_ms,
            upload_pack: request.service == GitFetchService::UploadPack,
            finished: false,
        });
        let body: Box<dyn EgressByteStream> = if request.service == GitFetchService::Discovery {
            Box::new(ExactAdvertisementStream::new(
                budgeted,
                reference,
                expected_commit,
            ))
        } else {
            budgeted
        };
        Ok(GitFetchExchangeResponse {
            status: upstream.status,
            content_type: expected_content_type.to_owned(),
            body,
        })
    }

    fn authorize(&self, request: &GitFetchAccess) -> Result<(), GitFetchDataError> {
        if request.repository != REPOSITORY_NAME || request.git_protocol.is_some() {
            return Err(GitFetchDataError::Refused);
        }
        let authority_sha256: [u8; 32] =
            Sha256::digest(request.source_authorization.as_bytes()).into();
        let now = now_ms().ok_or(GitFetchDataError::Unavailable)?;
        let sessions = lock(&self.inner.git_fetch_sessions);
        let session = sessions
            .get(&request.session_ref)
            .ok_or(GitFetchDataError::Refused)?;
        if constant_time_eq(&session.authority_sha256, &authority_sha256)
            && session.expires_at_unix_ms > now
            && session.request_count < SESSION_REQUEST_LIMIT
            && session.response_bytes < SESSION_BYTE_LIMIT
            && session.state == GitFetchSessionState::Active
        {
            Ok(())
        } else {
            Err(GitFetchDataError::Refused)
        }
    }
}

impl GitlabInner {
    async fn admit_project(
        &self,
        connection: &StoredConnection,
        token: &Secret,
        project_id: u64,
        reference: &str,
        expected_commit: &str,
    ) -> Result<String, ProjectAdmissionError> {
        let project: Project = self
            .fetch_admission_json(
                &connection.connection_ref,
                &format!("/api/v4/projects/{project_id}"),
                token,
            )
            .await?;
        if project.id != project_id || project.default_branch.as_deref() != Some(reference) {
            return Err(ProjectAdmissionError::NotGranted);
        }
        let mut branch_url = self.origin.clone();
        let project_id = project_id.to_string();
        branch_url
            .path_segments_mut()
            .map_err(|_| ProjectAdmissionError::Unavailable)?
            .clear()
            .extend([
                "api",
                "v4",
                "projects",
                project_id.as_str(),
                "repository",
                "branches",
                reference,
            ]);
        let branch: Branch = self
            .fetch_admission_url(&connection.connection_ref, branch_url, token)
            .await?;
        if branch.name != reference || branch.commit.id != expected_commit {
            return Err(ProjectAdmissionError::NotGranted);
        }
        repository_url(&self.origin, &project.path_with_namespace)
            .ok_or(ProjectAdmissionError::Unavailable)
    }

    async fn fetch_admission_json<T: for<'de> Deserialize<'de>>(
        &self,
        connection_ref: &str,
        path: &str,
        token: &Secret,
    ) -> Result<T, ProjectAdmissionError> {
        let target = self
            .provider_url(path)
            .map_err(|_| ProjectAdmissionError::Unavailable)?;
        self.fetch_admission_url(connection_ref, target, token)
            .await
    }

    async fn fetch_admission_url<T: for<'de> Deserialize<'de>>(
        &self,
        connection_ref: &str,
        target: url::Url,
        token: &Secret,
    ) -> Result<T, ProjectAdmissionError> {
        let response = self
            .egress
            .execute(
                connection_ref,
                service::EgressHttpRequest {
                    request: http_request("GET", target, bearer_headers(token), None),
                    maximum_response_bytes: MAX_PROVIDER_RESPONSE_BYTES,
                    response_headers: Vec::new(),
                },
            )
            .await
            .map_err(|_| ProjectAdmissionError::Unavailable)?;
        if matches!(response.status, 401 | 403 | 404) {
            return Err(ProjectAdmissionError::NotGranted);
        }
        if !(200..300).contains(&response.status) {
            return Err(ProjectAdmissionError::Unavailable);
        }
        serde_json::from_slice(&Zeroizing::new(response.body))
            .map_err(|_| ProjectAdmissionError::Unavailable)
    }

    fn revoke_fetch_session(&self, session_ref: &str, generation: u64) {
        if let Some(session) = lock(&self.git_fetch_sessions)
            .get_mut(session_ref)
            .filter(|session| session.generation == generation)
        {
            session.state = GitFetchSessionState::Revoked;
        }
    }

    fn reset_or_spend_fetch(&self, session_ref: &str, generation: u64, service: GitFetchService) {
        if let Some(session) = lock(&self.git_fetch_sessions)
            .get_mut(session_ref)
            .filter(|session| session.generation == generation)
        {
            session.state = if service == GitFetchService::UploadPack {
                GitFetchSessionState::Spent
            } else {
                GitFetchSessionState::Active
            };
        }
    }

    fn account_fetch_bytes(
        &self,
        session_ref: &str,
        generation: u64,
        bytes: usize,
    ) -> Result<(), EgressTransportError> {
        let mut sessions = lock(&self.git_fetch_sessions);
        let session = sessions
            .get_mut(session_ref)
            .filter(|session| session.generation == generation)
            .ok_or(EgressTransportError::Refused)?;
        let response_bytes = session
            .response_bytes
            .checked_add(bytes as u64)
            .filter(|bytes| *bytes <= SESSION_BYTE_LIMIT);
        let Some(response_bytes) = response_bytes else {
            session.state = GitFetchSessionState::Revoked;
            return Err(EgressTransportError::ResponseTooLarge);
        };
        session.response_bytes = response_bytes;
        Ok(())
    }
}

fn prune_fetch_sessions(
    sessions: &mut std::collections::BTreeMap<String, GitFetchSessionRecord>,
    now: u64,
) {
    sessions.retain(|_, session| {
        session.state == GitFetchSessionState::UploadInFlight
            || (session.state == GitFetchSessionState::Active && session.expires_at_unix_ms > now)
    });
}

struct BudgetedGitFetchStream {
    inner: Box<dyn EgressByteStream>,
    backend: Arc<GitlabInner>,
    session_ref: String,
    generation: u64,
    expires_at_unix_ms: u64,
    upload_pack: bool,
    finished: bool,
}

#[async_trait]
impl EgressByteStream for BudgetedGitFetchStream {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
        let now = now_ms().ok_or(EgressTransportError::Transport(
            EgressTransportFailure::Other,
        ))?;
        let Some(remaining_ms) = self.expires_at_unix_ms.checked_sub(now) else {
            self.finished = true;
            self.backend
                .revoke_fetch_session(&self.session_ref, self.generation);
            return Err(EgressTransportError::Transport(
                EgressTransportFailure::Timeout,
            ));
        };
        if remaining_ms == 0 {
            self.finished = true;
            self.backend
                .revoke_fetch_session(&self.session_ref, self.generation);
            return Err(EgressTransportError::Transport(
                EgressTransportFailure::Timeout,
            ));
        }
        let chunk =
            tokio::time::timeout(Duration::from_millis(remaining_ms), self.inner.next_chunk())
                .await
                .map_err(|_| {
                    self.finished = true;
                    self.backend
                        .revoke_fetch_session(&self.session_ref, self.generation);
                    EgressTransportError::Transport(EgressTransportFailure::Timeout)
                })?;
        match chunk {
            Ok(Some(bytes)) => {
                self.backend.account_fetch_bytes(
                    &self.session_ref,
                    self.generation,
                    bytes.len(),
                )?;
                Ok(Some(bytes))
            }
            Ok(None) => {
                self.finished = true;
                if self.upload_pack {
                    self.backend.reset_or_spend_fetch(
                        &self.session_ref,
                        self.generation,
                        GitFetchService::UploadPack,
                    );
                }
                Ok(None)
            }
            Err(error) => {
                self.finished = true;
                self.backend.reset_or_spend_fetch(
                    &self.session_ref,
                    self.generation,
                    if self.upload_pack {
                        GitFetchService::UploadPack
                    } else {
                        GitFetchService::Discovery
                    },
                );
                Err(error)
            }
        }
    }
}

impl Drop for BudgetedGitFetchStream {
    fn drop(&mut self) {
        if self.upload_pack && !self.finished {
            self.backend.reset_or_spend_fetch(
                &self.session_ref,
                self.generation,
                GitFetchService::UploadPack,
            );
        }
    }
}

fn connection_admits_repository(connection: &StoredConnection) -> bool {
    connection
        .scopes
        .iter()
        .any(|scope| matches!(scope.as_str(), "api" | "read_api" | "read_repository"))
}

fn repository_url(origin: &url::Url, path_with_namespace: &str) -> Option<String> {
    let mut parts = path_with_namespace.split('/').collect::<Vec<_>>();
    if parts.len() < 2
        || parts.iter().any(|part| {
            part.is_empty()
                || matches!(*part, "." | "..")
                || part.len() > 255
                || !part.is_ascii()
                || part.bytes().any(|byte| byte <= b' ' || byte == 0x7f)
        })
    {
        return None;
    }
    let repository = format!("{}.git", parts.pop()?);
    let mut target = origin.clone();
    let mut segments = target.path_segments_mut().ok()?;
    segments.clear();
    segments.extend(parts);
    segments.push(&repository);
    drop(segments);
    Some(target.into())
}

fn session_ref(context: &PrincipalContext, idempotency_key: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"b10x/git-fetch-session/v1\0");
    digest.update(context.tenant_id().as_bytes());
    digest.update(b"\0");
    digest.update(context.subject().as_bytes());
    digest.update(b"\0");
    digest.update(idempotency_key.as_bytes());
    format!("git-fetch:{:x}", digest.finalize())
}

fn constant_time_eq(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn valid_upload_pack_request(body: &[u8], expected_commit: &str, maximum_depth: u8) -> bool {
    let mut offset = 0;
    let mut saw_want = false;
    let mut saw_depth = false;
    while offset < body.len() {
        let Some(header) = body.get(offset..offset + 4) else {
            return false;
        };
        let Ok(header) = std::str::from_utf8(header) else {
            return false;
        };
        let Ok(length) = usize::from_str_radix(header, 16) else {
            return false;
        };
        offset += 4;
        if matches!(length, 0..=2) {
            continue;
        }
        if length < 4 {
            return false;
        }
        let payload_length = length - 4;
        let Some(payload) = body.get(offset..offset + payload_length) else {
            return false;
        };
        offset += payload_length;
        let payload = payload.strip_suffix(b"\n").unwrap_or(payload);
        if let Some(value) = payload.strip_prefix(b"want ") {
            let object = value
                .split(|byte| byte.is_ascii_whitespace())
                .next()
                .unwrap_or_default();
            if object != expected_commit.as_bytes() {
                return false;
            }
            saw_want = true;
        } else if let Some(value) = payload.strip_prefix(b"deepen ") {
            let Ok(value) = std::str::from_utf8(value) else {
                return false;
            };
            let Ok(value) = value.parse::<u8>() else {
                return false;
            };
            if value == 0 || value > maximum_depth || saw_depth {
                return false;
            }
            saw_depth = true;
        } else if payload.starts_with(b"deepen-") || payload.starts_with(b"want-ref ") {
            return false;
        }
    }
    saw_want && saw_depth
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use connector_secrets::{MemoryStore, SecretStore as _};
    use service::{
        EgressHttpRequest, EgressHttpResponse, EgressStreamingHttpResponse, EgressTransport,
        EgressWebSocket,
    };

    struct TestStream {
        chunk: Option<Vec<u8>>,
    }

    #[async_trait]
    impl EgressByteStream for TestStream {
        async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
            Ok(self.chunk.take())
        }
    }

    struct PendingStream;

    #[async_trait]
    impl EgressByteStream for PendingStream {
        async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
            std::future::pending().await
        }
    }

    struct GitEgress {
        project_calls: AtomicUsize,
        stream_calls: AtomicUsize,
        seen_headers: Mutex<Vec<BTreeMap<String, String>>>,
        seen_urls: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl EgressTransport for GitEgress {
        async fn execute(
            &self,
            _authority_ref: &str,
            request: EgressHttpRequest,
        ) -> Result<EgressHttpResponse, EgressTransportError> {
            let url = url::Url::parse(&request.request.url).unwrap();
            let body = if url.path() == "/api/v4/projects/42" {
                let project_call = self.project_calls.fetch_add(1, Ordering::SeqCst);
                serde_json::json!({
                    "id": 42,
                    "path_with_namespace": if project_call == 0 {
                        "group/repository"
                    } else {
                        "group/renamed-repository"
                    },
                    "default_branch": "trunk"
                })
            } else if url.path() == "/api/v4/projects/42/repository/branches/trunk" {
                serde_json::json!({
                    "name": "trunk",
                    "commit": {"id": "a".repeat(40)}
                })
            } else {
                return Ok(EgressHttpResponse {
                    status: 404,
                    headers: BTreeMap::new(),
                    body: Vec::new(),
                });
            };
            Ok(EgressHttpResponse {
                status: 200,
                headers: BTreeMap::new(),
                body: serde_json::to_vec(&body).unwrap(),
            })
        }

        async fn execute_stream(
            &self,
            _authority_ref: &str,
            request: EgressStreamingHttpRequest,
        ) -> Result<EgressStreamingHttpResponse, EgressTransportError> {
            self.stream_calls.fetch_add(1, Ordering::SeqCst);
            self.seen_headers
                .lock()
                .unwrap()
                .push(request.headers.clone());
            self.seen_urls.lock().unwrap().push(request.url.clone());
            let target = url::Url::parse(&request.url).unwrap();
            let content_type = if target.path().ends_with("/info/refs")
                && target
                    .query_pairs()
                    .any(|(key, value)| key == "service" && value == "git-upload-pack")
            {
                "application/x-git-upload-pack-advertisement"
            } else if target.path().ends_with("/git-upload-pack") {
                "application/x-git-upload-pack-result"
            } else {
                return Err(EgressTransportError::Refused);
            };
            Ok(EgressStreamingHttpResponse {
                status: 200,
                headers: BTreeMap::from([("content-type".to_owned(), content_type.to_owned())]),
                body: Box::new(TestStream {
                    chunk: Some(if content_type.ends_with("advertisement") {
                        legacy_advertisement()
                    } else {
                        b"git-packet".to_vec()
                    }),
                }),
            })
        }

        async fn connect_websocket(
            &self,
            _authority_ref: &str,
            _url: String,
            _maximum_message_bytes: usize,
        ) -> Result<Box<dyn EgressWebSocket>, EgressTransportError> {
            Err(EgressTransportError::Refused)
        }
    }

    fn context() -> PrincipalContext {
        PrincipalContext::hosted(
            "tenant-one".to_owned(),
            "person:owner".to_owned(),
            "person:owner".to_owned(),
            Some("owner@example.test".to_owned()),
            "snapshot-one".to_owned(),
            "b".repeat(64),
        )
        .unwrap()
    }

    fn request() -> protocol::git_fetch::CreateRequest {
        protocol::git_fetch::CreateRequest {
            idempotency_key: "coding-session-one".to_owned(),
            connection_ref: "connection:gitlab:11111111-1111-4111-8111-111111111111".to_owned(),
            project_id: 42,
            reference: "trunk".to_owned(),
            expected_commit: "a".repeat(40),
            depth: 10,
        }
    }

    async fn backend() -> (GitlabBackend, Arc<GitEgress>) {
        let store = Arc::new(MemoryStore::new());
        let egress = Arc::new(GitEgress {
            project_calls: AtomicUsize::new(0),
            stream_calls: AtomicUsize::new(0),
            seen_headers: Mutex::new(Vec::new()),
            seen_urls: Mutex::new(Vec::new()),
        });
        let backend = GitlabBackend::open_inner(
            "tenant-one".to_owned(),
            connectors_config::HostedGitlabConfig {
                origin: "https://gitlab.example.test".to_owned(),
                public_origin: "https://connectors.example.test/api/connectors/v1".to_owned(),
                git_fetch_origin: Some("https://connectors.internal".to_owned()),
                oauth_client_id: "client-one".to_owned(),
                oauth_redirect_uri:
                    "https://connectors.example.test/api/connectors/v1/oauth/gitlab/callback"
                        .to_owned(),
                user_grant_ref: "grant:gitlab:user".to_owned(),
                initiation: connectors_config::InitiationConfig::Provider,
                connect_session_ttl_seconds: 300,
                refresh_skew_seconds: 300,
            },
            store.clone(),
            super::super::GitlabState::Hosted(Arc::new(connector_state::MemoryState::new())),
            egress.clone(),
        )
        .await
        .unwrap();
        let connection = StoredConnection {
            connection_ref: "connection:gitlab:11111111-1111-4111-8111-111111111111".to_owned(),
            instance_id: "11111111-1111-4111-8111-111111111111".to_owned(),
            label: "GitLab".to_owned(),
            grant_ref: "grant:gitlab:user".to_owned(),
            owner_subject: "person:owner".to_owned(),
            external_user_id: 7,
            username: "owner".to_owned(),
            email_sha256: "c".repeat(64),
            profile: super::super::GitlabProfile::PersonalToken,
            scopes: vec!["read_api".to_owned()],
            credential_generation: 1,
            observed_at_unix_ms: 1,
            expires_at_unix_ms: None,
        };
        let credential_ref = backend
            .inner
            .connection_credential_ref(&connection, super::super::ACCESS_TOKEN_CREDENTIAL)
            .unwrap();
        store
            .put(&credential_ref, &Secret::new("synthetic-gitlab-token"))
            .await
            .unwrap();
        super::super::lock(&backend.inner.metadata)
            .connections
            .push(connection);
        (backend, egress)
    }

    fn packet(payload: &[u8]) -> Vec<u8> {
        let mut packet = format!("{:04x}", payload.len() + 4).into_bytes();
        packet.extend_from_slice(payload);
        packet
    }

    fn legacy_advertisement() -> Vec<u8> {
        let expected = "a".repeat(40);
        let other = "b".repeat(40);
        let mut advertisement = packet(b"# service=git-upload-pack\n");
        advertisement.extend_from_slice(b"0000");
        advertisement.extend_from_slice(&packet(
            format!(
                "{expected} HEAD\0multi_ack thin-pack side-band-64k symref=HEAD:refs/heads/trunk\n"
            )
            .as_bytes(),
        ));
        advertisement
            .extend_from_slice(&packet(format!("{expected} refs/heads/trunk\n").as_bytes()));
        advertisement
            .extend_from_slice(&packet(format!("{other} refs/heads/private\n").as_bytes()));
        advertisement.extend_from_slice(&packet(
            format!("{expected} refs/tags/internal\n").as_bytes(),
        ));
        advertisement.extend_from_slice(b"0000");
        advertisement
    }

    #[test]
    fn upstream_repository_is_derived_without_forwarding_provider_metadata() {
        let origin = url::Url::parse("https://gitlab.example.test").unwrap();
        assert_eq!(
            repository_url(&origin, "group/nested/repository").as_deref(),
            Some("https://gitlab.example.test/group/nested/repository.git")
        );
        assert!(repository_url(&origin, "../repository").is_none());
        assert!(repository_url(&origin, "repository").is_none());
    }

    #[test]
    fn source_authority_digest_comparison_checks_every_byte() {
        let digest = [7_u8; 32];
        assert!(constant_time_eq(&digest, &digest));
        let mut changed = digest;
        changed[31] = 8;
        assert!(!constant_time_eq(&digest, &changed));
    }

    #[test]
    fn upload_pack_request_is_bound_to_exact_commit_and_depth() {
        let commit = "a".repeat(40);
        let body = format!("0032want {commit}\n000ddeepen 5\n0000");
        assert!(valid_upload_pack_request(body.as_bytes(), &commit, 10));
        assert!(!valid_upload_pack_request(
            body.as_bytes(),
            &"b".repeat(40),
            10
        ));
        assert!(!valid_upload_pack_request(body.as_bytes(), &commit, 4));
        let full = format!("0032want {commit}\n0000");
        assert!(!valid_upload_pack_request(full.as_bytes(), &commit, 10));
    }

    #[tokio::test]
    async fn idempotent_replay_keeps_locator_and_rotates_transient_authority() {
        let (backend, _) = backend().await;
        let first = backend.create(&context(), request()).await.unwrap();
        let second = backend.create(&context(), request()).await.unwrap();
        assert_eq!(first.session_ref, second.session_ref);
        assert_eq!(first.locator, second.locator);
        assert_eq!(first.reference, "trunk");
        assert_ne!(
            first.expose_at_control_boundary(),
            second.expose_at_control_boundary()
        );

        let mut conflict = request();
        conflict.depth = 11;
        assert!(matches!(
            backend.create(&context(), conflict).await,
            Err(GitFetchControlError::Conflict)
        ));
    }

    #[tokio::test]
    async fn upload_pack_stream_is_bounded_and_spends_the_session() {
        let (backend, egress) = backend().await;
        let grant = backend.create(&context(), request()).await.unwrap();
        let authority = grant.expose_at_control_boundary().to_owned();
        let exchange = GitFetchExchange {
            session_ref: grant.session_ref.clone(),
            repository: REPOSITORY_NAME.to_owned(),
            source_authorization: Zeroizing::new(authority.clone()),
            service: GitFetchService::UploadPack,
            git_protocol: None,
            body: Some(
                b"0032want aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n000ddeepen 5\n0000".to_vec(),
            ),
        };
        let mut response = backend.exchange(exchange).await.unwrap();
        assert_eq!(
            response.body.next_chunk().await.unwrap(),
            Some(b"git-packet".to_vec())
        );
        assert_eq!(response.body.next_chunk().await.unwrap(), None);
        assert_eq!(egress.stream_calls.load(Ordering::SeqCst), 1);
        {
            let headers = egress.seen_headers.lock().unwrap();
            assert_eq!(
                headers[0].get("authorization").map(String::as_str),
                Some("Bearer synthetic-gitlab-token")
            );
            assert!(!headers[0].contains_key("x-b10x-git-source-authorization"));
        }
        assert!(egress.seen_urls.lock().unwrap()[0]
            .starts_with("https://gitlab.example.test/group/renamed-repository.git/"));

        assert!(matches!(
            backend
                .exchange(GitFetchExchange {
                    session_ref: grant.session_ref,
                    repository: REPOSITORY_NAME.to_owned(),
                    source_authorization: Zeroizing::new(authority),
                    service: GitFetchService::Discovery,
                    git_protocol: None,
                    body: None,
                })
                .await,
            Err(GitFetchDataError::Refused)
        ));
    }

    #[tokio::test]
    async fn protocol_v2_is_refused_before_provider_egress() {
        let (backend, egress) = backend().await;
        let grant = backend.create(&context(), request()).await.unwrap();
        let authority = grant.expose_at_control_boundary().to_owned();
        let result = backend
            .exchange(GitFetchExchange {
                session_ref: grant.session_ref,
                repository: REPOSITORY_NAME.to_owned(),
                source_authorization: Zeroizing::new(authority),
                service: GitFetchService::Discovery,
                git_protocol: Some("version=2".to_owned()),
                body: None,
            })
            .await;
        assert!(matches!(result, Err(GitFetchDataError::Refused)));
        assert_eq!(egress.stream_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn discovery_advertises_only_the_exact_default_branch_snapshot() {
        let (backend, egress) = backend().await;
        let grant = backend.create(&context(), request()).await.unwrap();
        let authority = grant.expose_at_control_boundary().to_owned();
        let mut response = backend
            .exchange(GitFetchExchange {
                session_ref: grant.session_ref,
                repository: REPOSITORY_NAME.to_owned(),
                source_authorization: Zeroizing::new(authority),
                service: GitFetchService::Discovery,
                git_protocol: None,
                body: None,
            })
            .await
            .unwrap();
        let mut advertised = Vec::new();
        while let Some(chunk) = response.body.next_chunk().await.unwrap() {
            advertised.extend_from_slice(&chunk);
        }
        let text = String::from_utf8(advertised).unwrap();
        assert!(text.contains("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa HEAD"));
        assert!(text.contains("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa refs/heads/trunk"));
        assert!(!text.contains("refs/heads/private"));
        assert!(!text.contains("refs/tags/internal"));
        assert!(!text.contains("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"));
        assert!(egress.seen_urls.lock().unwrap()[0]
            .starts_with("https://gitlab.example.test/group/renamed-repository.git/"));
    }

    #[tokio::test]
    async fn stream_expiry_revokes_a_stalled_upload() {
        let (backend, _) = backend().await;
        let grant = backend.create(&context(), request()).await.unwrap();
        let generation = {
            let mut sessions = lock(&backend.inner.git_fetch_sessions);
            let session = sessions.get_mut(&grant.session_ref).unwrap();
            session.state = GitFetchSessionState::UploadInFlight;
            session.expires_at_unix_ms = now_ms().unwrap() + 25;
            session.generation
        };
        let mut stream = BudgetedGitFetchStream {
            inner: Box::new(PendingStream),
            backend: backend.inner.clone(),
            session_ref: grant.session_ref.clone(),
            generation,
            expires_at_unix_ms: now_ms().unwrap() + 25,
            upload_pack: true,
            finished: false,
        };
        assert!(matches!(
            stream.next_chunk().await,
            Err(EgressTransportError::Transport(
                EgressTransportFailure::Timeout
            ))
        ));
        assert_eq!(
            lock(&backend.inner.git_fetch_sessions)
                .get(&grant.session_ref)
                .unwrap()
                .state,
            GitFetchSessionState::Revoked
        );
    }

    #[tokio::test]
    async fn per_principal_capacity_is_atomic_and_never_evicts_a_live_session() {
        let (backend, _) = backend().await;
        let backend = Arc::new(backend);
        let mut tasks = Vec::new();
        for index in 0..(MAX_SESSIONS_PER_PRINCIPAL + 4) {
            let backend = backend.clone();
            tasks.push(tokio::spawn(async move {
                let mut request = request();
                request.idempotency_key = format!("coding-session-{index}");
                backend.create(&context(), request).await
            }));
        }
        let mut admitted = Vec::new();
        let mut refused = 0;
        for task in tasks {
            match task.await.unwrap() {
                Ok(grant) => admitted.push(grant),
                Err(GitFetchControlError::Unavailable) => refused += 1,
                Err(error) => panic!("unexpected creation result: {error:?}"),
            }
        }
        assert_eq!(admitted.len(), MAX_SESSIONS_PER_PRINCIPAL);
        assert_eq!(refused, 4);
        let first = &admitted[0];
        assert!(backend
            .authorize(&GitFetchAccess {
                session_ref: first.session_ref.clone(),
                repository: REPOSITORY_NAME.to_owned(),
                source_authorization: Zeroizing::new(first.expose_at_control_boundary().to_owned()),
                service: GitFetchService::Discovery,
                git_protocol: None,
            })
            .is_ok());
    }

    #[tokio::test]
    async fn global_capacity_refuses_without_evicting_an_inflight_other_principal() {
        let (backend, _) = backend().await;
        let now = now_ms().unwrap();
        {
            let mut sessions = lock(&backend.inner.git_fetch_sessions);
            for index in 0..MAX_SESSIONS {
                sessions.insert(
                    format!("git-fetch:occupied-{index}"),
                    GitFetchSessionRecord {
                        idempotency_key: format!("other-{index}"),
                        owner_subject: format!("person:other-{index}"),
                        connection_ref: format!("connection:other-{index}"),
                        project_id: index as u64,
                        reference: "trunk".to_owned(),
                        expected_commit: "a".repeat(40),
                        depth: 1,
                        authority_sha256: [index as u8; 32],
                        generation: 1,
                        expires_at_unix_ms: now + SESSION_TTL_MS,
                        request_count: 0,
                        response_bytes: 0,
                        state: if index == 0 {
                            GitFetchSessionState::UploadInFlight
                        } else {
                            GitFetchSessionState::Active
                        },
                    },
                );
            }
        }
        assert!(matches!(
            backend.create(&context(), request()).await,
            Err(GitFetchControlError::Unavailable)
        ));
        let sessions = lock(&backend.inner.git_fetch_sessions);
        assert_eq!(sessions.len(), MAX_SESSIONS);
        assert_eq!(
            sessions.get("git-fetch:occupied-0").unwrap().state,
            GitFetchSessionState::UploadInFlight
        );
    }

    #[tokio::test]
    async fn removed_principal_connection_revokes_a_live_session() {
        let (backend, _) = backend().await;
        let grant = backend.create(&context(), request()).await.unwrap();
        let authority = grant.expose_at_control_boundary().to_owned();
        super::super::lock(&backend.inner.metadata)
            .connections
            .clear();
        assert!(matches!(
            backend
                .exchange(GitFetchExchange {
                    session_ref: grant.session_ref,
                    repository: REPOSITORY_NAME.to_owned(),
                    source_authorization: Zeroizing::new(authority),
                    service: GitFetchService::Discovery,
                    git_protocol: None,
                    body: None,
                })
                .await,
            Err(GitFetchDataError::Refused)
        ));
    }

    #[tokio::test]
    async fn current_grant_and_provider_default_tip_are_revalidated() {
        let (backend, _) = backend().await;

        let mut wrong_branch = request();
        wrong_branch.reference = "main".to_owned();
        assert!(matches!(
            backend.create(&context(), wrong_branch).await,
            Err(GitFetchControlError::NotGranted)
        ));
        let mut wrong_commit = request();
        wrong_commit.expected_commit = "b".repeat(40);
        assert!(matches!(
            backend.create(&context(), wrong_commit).await,
            Err(GitFetchControlError::NotGranted)
        ));

        let grant = backend.create(&context(), request()).await.unwrap();
        let authority = grant.expose_at_control_boundary().to_owned();
        super::super::lock(&backend.inner.metadata).connections[0].grant_ref =
            "grant:gitlab:revoked".to_owned();
        assert!(matches!(
            backend
                .exchange(GitFetchExchange {
                    session_ref: grant.session_ref,
                    repository: REPOSITORY_NAME.to_owned(),
                    source_authorization: Zeroizing::new(authority),
                    service: GitFetchService::Discovery,
                    git_protocol: None,
                    body: None,
                })
                .await,
            Err(GitFetchDataError::Refused)
        ));
        assert!(matches!(
            backend.create(&context(), request()).await,
            Err(GitFetchControlError::NotGranted)
        ));
    }
}

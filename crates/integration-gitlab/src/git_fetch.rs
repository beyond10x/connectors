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
#[cfg(test)]
#[path = "../tests/support/git_fetch_http.rs"]
mod http_tests;
#[path = "git_fetch_v2.rs"]
mod v2;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GitVersion {
    Legacy,
    V2,
}

impl GitVersion {
    fn from_header(header: Option<&str>) -> Result<Self, GitFetchDataError> {
        match header {
            None => Ok(Self::Legacy),
            Some("version=2") => Ok(Self::V2),
            _ => Err(GitFetchDataError::Refused),
        }
    }
}

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
    protocol: Option<GitVersion>,
    v2_negotiated: bool,
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
                    protocol: None,
                    v2_negotiated: false,
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
        mut request: GitFetchExchange,
    ) -> Result<GitFetchExchangeResponse, GitFetchDataError> {
        if request.repository != REPOSITORY_NAME
            || matches!(request.service, GitFetchService::Discovery) && request.body.is_some()
            || matches!(request.service, GitFetchService::UploadPack)
                && request.body.as_ref().is_none_or(Vec::is_empty)
        {
            return Err(GitFetchDataError::Refused);
        }
        let version = GitVersion::from_header(request.git_protocol.as_deref())?;
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
            left,
            generation,
            expires_at_unix_ms,
            command,
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
                || session.protocol.is_some_and(|protocol| protocol != version)
                || version == GitVersion::V2
                    && request.service == GitFetchService::UploadPack
                    && !session.v2_negotiated
            {
                if session.expires_at_unix_ms <= now
                    || session.request_count >= SESSION_REQUEST_LIMIT
                    || session.response_bytes >= SESSION_BYTE_LIMIT
                {
                    session.state = GitFetchSessionState::Revoked;
                }
                return Err(GitFetchDataError::Refused);
            }
            let command = if request.service == GitFetchService::UploadPack {
                let body = request.body.as_deref().expect("shape checked");
                match version {
                    GitVersion::V2 => {
                        let (command, body) = v2::request(
                            body,
                            &session.reference,
                            &session.expected_commit,
                            session.depth,
                        )
                        .ok_or(GitFetchDataError::Refused)?;
                        request.body = Some(body);
                        Some(command)
                    }
                    GitVersion::Legacy => {
                        if !valid_upload_pack_request(body, &session.expected_commit, session.depth)
                        {
                            return Err(GitFetchDataError::Refused);
                        }
                        Some(v2::Command::Fetch)
                    }
                }
            } else {
                None
            };
            session.protocol = Some(version);
            session.request_count = session.request_count.saturating_add(1);
            if command == Some(v2::Command::Fetch) {
                session.state = GitFetchSessionState::UploadInFlight;
            }
            (
                session.connection_ref.clone(),
                session.owner_subject.clone(),
                session.project_id,
                session.reference.clone(),
                session.expected_commit.clone(),
                SESSION_BYTE_LIMIT.saturating_sub(session.response_bytes),
                session.generation,
                session.expires_at_unix_ms,
                command,
            )
        };

        // The v2 ls-refs command is a POST too, but never consumes the final fetch.
        let session_service = if command == Some(v2::Command::Fetch) {
            GitFetchService::UploadPack
        } else {
            GitFetchService::Discovery
        };
        let mut attempt = FetchAttempt {
            backend: self.inner.clone(),
            session_ref: request.session_ref.clone(),
            generation,
            upload_pack: session_service == GitFetchService::UploadPack,
            handed_off: false,
        };

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
                    .fail_fetch_exchange(&request.session_ref, generation, session_service);
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
                    ProjectAdmissionError::Unavailable => self.inner.fail_fetch_exchange(
                        &request.session_ref,
                        generation,
                        session_service,
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
        if version == GitVersion::V2 {
            headers.insert("git-protocol".to_owned(), "version=2".to_owned());
        }
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
                    .fail_fetch_exchange(&request.session_ref, generation, session_service);
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
                .fail_fetch_exchange(&request.session_ref, generation, session_service);
            return Err(GitFetchDataError::Unavailable);
        }
        if upstream.header("content-type") != Some(expected_content_type) {
            self.inner
                .fail_fetch_exchange(&request.session_ref, generation, session_service);
            return Err(GitFetchDataError::Unavailable);
        }
        let budgeted: Box<dyn EgressByteStream> = Box::new(BudgetedGitFetchStream {
            inner: upstream.body,
            backend: self.inner.clone(),
            session_ref: request.session_ref.clone(),
            generation,
            expires_at_unix_ms,
            upload_pack: version == GitVersion::Legacy
                && session_service == GitFetchService::UploadPack,
            finished: false,
        });
        let body: Box<dyn EgressByteStream> = if version == GitVersion::V2 {
            match command {
                None => {
                    let capabilities = v2::capabilities(budgeted)
                        .await
                        .map_err(|_| GitFetchDataError::Unavailable)?;
                    let mut sessions = lock(&self.inner.git_fetch_sessions);
                    let session = sessions
                        .get_mut(&request.session_ref)
                        .filter(|session| {
                            session.generation == generation
                                && session.state == GitFetchSessionState::Active
                        })
                        .ok_or(GitFetchDataError::Refused)?;
                    session.v2_negotiated = true;
                    Box::new(v2::OneChunk(Some(capabilities)))
                }
                Some(v2::Command::LsRefs { symrefs }) => {
                    let refs = v2::references(budgeted, &reference, &expected_commit, symrefs)
                        .await
                        .map_err(|_| GitFetchDataError::Unavailable)?;
                    Box::new(v2::OneChunk(Some(refs)))
                }
                Some(v2::Command::Fetch) => Box::new(CompletingGitFetchStream {
                    inner: v2::PackStream::new(budgeted),
                    attempt: FetchAttempt {
                        backend: self.inner.clone(),
                        session_ref: request.session_ref.clone(),
                        generation,
                        upload_pack: true,
                        handed_off: false,
                    },
                }),
            }
        } else if request.service == GitFetchService::Discovery {
            Box::new(ExactAdvertisementStream::new(
                budgeted,
                reference,
                expected_commit,
            ))
        } else {
            budgeted
        };
        attempt.handed_off = true;
        Ok(GitFetchExchangeResponse {
            status: upstream.status,
            content_type: expected_content_type.to_owned(),
            body,
        })
    }

    fn authorize(&self, request: &GitFetchAccess) -> Result<(), GitFetchDataError> {
        if request.repository != REPOSITORY_NAME {
            return Err(GitFetchDataError::Refused);
        }
        let version = GitVersion::from_header(request.git_protocol.as_deref())?;
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
            && session.protocol.is_none_or(|protocol| protocol == version)
            && (version != GitVersion::V2
                || request.service == GitFetchService::Discovery
                || session.v2_negotiated)
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
        let mut branch_url = self.origin.clone();
        let project_id_text = project_id.to_string();
        branch_url
            .path_segments_mut()
            .map_err(|_| ProjectAdmissionError::Unavailable)?
            .clear()
            .extend([
                "api",
                "v4",
                "projects",
                project_id_text.as_str(),
                "repository",
                "branches",
                reference,
            ]);
        // These provider reads are independent. Revalidate both on every exchange while
        // overlapping their network latency; retain project-first refusal precedence.
        let project_path = format!("/api/v4/projects/{project_id}");
        let (project, branch) = tokio::join!(
            self.fetch_admission_json::<Project>(&connection.connection_ref, &project_path, token),
            self.fetch_admission_url::<Branch>(&connection.connection_ref, branch_url, token),
        );
        let project = project?;
        if project.id != project_id || project.default_branch.as_deref() != Some(reference) {
            return Err(ProjectAdmissionError::NotGranted);
        }
        let branch = branch?;
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

    fn fail_fetch_exchange(&self, session_ref: &str, generation: u64, service: GitFetchService) {
        // Reference discovery never changes fetch state, including when an overlapping fetch
        // started after this reference request. A failed request must not reopen its capability.
        if service == GitFetchService::Discovery {
            return;
        }
        if let Some(session) = lock(&self.git_fetch_sessions)
            .get_mut(session_ref)
            .filter(|session| session.generation == generation)
        {
            if matches!(
                session.state,
                GitFetchSessionState::Revoked | GitFetchSessionState::Spent
            ) {
                return;
            }
            session.state = GitFetchSessionState::Revoked;
        }
    }

    fn complete_fetch(
        &self,
        session_ref: &str,
        generation: u64,
    ) -> Result<(), EgressTransportError> {
        let mut sessions = lock(&self.git_fetch_sessions);
        let session = sessions
            .get_mut(session_ref)
            .filter(|session| {
                session.generation == generation
                    && session.state == GitFetchSessionState::UploadInFlight
            })
            .ok_or(EgressTransportError::Refused)?;
        session.state = GitFetchSessionState::Spent;
        Ok(())
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
            .filter(|session| session.generation == generation && session.state.is_live())
            .ok_or(EgressTransportError::Refused)?;
        if !lock(&self.metadata).connections.iter().any(|connection| {
            connection.connection_ref == session.connection_ref
                && connection.owner_subject == session.owner_subject
                && connection.grant_ref == self.policy.user_grant_ref
                && connection_admits_repository(connection)
        }) {
            session.state = GitFetchSessionState::Revoked;
            return Err(EgressTransportError::Refused);
        }
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

/// Cancellation while waiting for provider headers must not strand an in-flight fetch.
struct FetchAttempt {
    backend: Arc<GitlabInner>,
    session_ref: String,
    generation: u64,
    upload_pack: bool,
    handed_off: bool,
}

impl Drop for FetchAttempt {
    fn drop(&mut self) {
        if self.upload_pack && !self.handed_off {
            self.backend
                .revoke_fetch_session(&self.session_ref, self.generation);
        }
    }
}

/// Completion belongs outside v2 packet validation, while byte accounting belongs inside it.
struct CompletingGitFetchStream {
    inner: v2::PackStream,
    attempt: FetchAttempt,
}

#[async_trait]
impl EgressByteStream for CompletingGitFetchStream {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, EgressTransportError> {
        if self.attempt.handed_off {
            return Ok(None);
        }
        self.attempt.backend.account_fetch_bytes(
            &self.attempt.session_ref,
            self.attempt.generation,
            0,
        )?;
        match self.inner.next_chunk().await {
            Ok(None) => {
                self.attempt
                    .backend
                    .complete_fetch(&self.attempt.session_ref, self.attempt.generation)?;
                self.attempt.handed_off = true;
                Ok(None)
            }
            Ok(chunk) => Ok(chunk),
            Err(error) => {
                self.attempt
                    .backend
                    .revoke_fetch_session(&self.attempt.session_ref, self.attempt.generation);
                Err(error)
            }
        }
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
        if self.finished {
            return Ok(None);
        }
        self.backend
            .account_fetch_bytes(&self.session_ref, self.generation, 0)?;
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
                    self.backend
                        .complete_fetch(&self.session_ref, self.generation)?;
                }
                Ok(None)
            }
            Err(error) => {
                self.finished = true;
                self.backend.fail_fetch_exchange(
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
            self.backend.fail_fetch_exchange(
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
#[path = "../tests/support/git_fetch.rs"]
mod tests;

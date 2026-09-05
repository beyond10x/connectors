#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::profiles::{PROFILE_OAUTH, PROFILE_PAT};
    use crate::repository_file::valid_repository_file_path;
    use connector_secrets::MemoryStore;
    use connector_state::StateStore as _;
    use service::{EgressTransportError, EgressWebSocket};

    struct PagedProjectsEgress {
        calls: AtomicUsize,
        continuation: &'static str,
    }

    #[async_trait]
    impl EgressTransport for PagedProjectsEgress {
        async fn execute(
            &self,
            _authority_ref: &str,
            request: EgressHttpRequest,
        ) -> Result<EgressHttpResponse, EgressTransportError> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            let target = url::Url::parse(&request.request.url).expect("provider URL");
            let credential_evidence = match target.path() {
                "/api/v4/personal_access_tokens/self" => Some(serde_json::json!({
                    "active": true, "revoked": false, "scopes": ["read_api"]
                })),
                "/api/v4/user" => Some(serde_json::json!({
                    "id": 7, "username": "owner", "state": "active",
                    "email": "owner@example.test", "bot": false
                })),
                _ => None,
            };
            if let Some(evidence) = credential_evidence {
                return Ok(EgressHttpResponse {
                    status: 200,
                    headers: BTreeMap::new(),
                    body: serde_json::to_vec(&evidence).unwrap(),
                });
            }
            let page = target
                .query_pairs()
                .find_map(|(key, value)| (key == "page").then_some(value.into_owned()))
                .expect("the bounded scan always names its page");
            assert_eq!(page, (call + 1).to_string());
            let projects = if call == 0 {
                (1..=100)
                    .map(|id| {
                        serde_json::json!({
                            "id": id,
                            "path_with_namespace": format!("group/repository-{id}")
                        })
                    })
                    .collect::<Vec<_>>()
            } else {
                vec![serde_json::json!({
                    "id": 101,
                    "path_with_namespace": "group/selected-repository"
                })]
            };
            Ok(EgressHttpResponse {
                status: 200,
                headers: if call == 0 {
                    BTreeMap::from([("x-next-page".to_owned(), self.continuation.to_owned())])
                } else {
                    BTreeMap::new()
                },
                body: serde_json::to_vec(&projects).unwrap(),
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

    async fn paged_backend(
        continuation: &'static str,
    ) -> (GitlabBackend, Arc<PagedProjectsEgress>) {
        let egress = Arc::new(PagedProjectsEgress {
            calls: AtomicUsize::new(0),
            continuation,
        });
        let backend = GitlabBackend::open_inner(
            "tenant-one".to_owned(),
            HostedGitlabConfig {
                origin: "https://gitlab.example.test".to_owned(),
                public_origin: "https://connectors.example.test/api/connectors/v1".to_owned(),
                git_fetch_origin: None,
                oauth_client_id: "client-one".to_owned(),
                oauth_redirect_uri:
                    "https://connectors.example.test/api/connectors/v1/oauth/gitlab/callback"
                        .to_owned(),
                user_grant_ref: "grant:gitlab:user".to_owned(),
                initiation: InitiationConfig::Provider,
                connect_session_ttl_seconds: 300,
                refresh_skew_seconds: 300,
            },
            Arc::new(MemoryStore::new()),
            GitlabState::Hosted(Arc::new(connector_state::MemoryState::new())),
            egress.clone(),
        )
        .await
        .unwrap();
        (backend, egress)
    }

    #[tokio::test]
    async fn legacy_connection_starts_unusable_and_preserves_pending_custody() {
        let state = Arc::new(connector_state::MemoryState::new());
        let metadata = StateFile {
            version: STATE_VERSION,
            next_transaction_generation: 1,
            connections: vec![StoredConnection {
                connection_ref: "connection:gitlab:legacy".to_owned(),
                instance_id: "00000000-0000-4000-8000-000000000007".to_owned(),
                label: "GitLab".to_owned(),
                grant_ref: String::new(),
                owner_subject: "person:owner".to_owned(),
                external_user_id: 7,
                username: "owner".to_owned(),
                email_sha256: "a".repeat(64),
                profile: GitlabProfile::PersonalToken,
                scopes: vec!["read_api".to_owned()],
                credential_generation: 1,
                observed_at_unix_ms: 1,
                expires_at_unix_ms: None,
            }],
            pending: Vec::new(),
        };
        let mut encoded = serde_json::to_value(&metadata).unwrap();
        encoded["connections"][0]
            .as_object_mut()
            .unwrap()
            .remove("grant_ref");
        encoded["pending"] = serde_json::json!([{
            "transaction_id": "legacy-transaction-must-not-be-recovered",
            "connection": encoded["connections"][0].clone()
        }]);
        let original = serde_json::to_vec(&encoded).unwrap();
        state
            .replace(STATE_KEY, &original, MAX_STATE_BYTES)
            .unwrap();
        let egress = Arc::new(PagedProjectsEgress {
            calls: AtomicUsize::new(0),
            continuation: "",
        });
        let backend = GitlabBackend::open_inner(
            "tenant-one".to_owned(),
            HostedGitlabConfig {
                origin: "https://gitlab.example.test".to_owned(),
                public_origin: "https://connectors.example.test/api/connectors/v1".to_owned(),
                git_fetch_origin: None,
                oauth_client_id: "client-one".to_owned(),
                oauth_redirect_uri:
                    "https://connectors.example.test/api/connectors/v1/oauth/gitlab/callback"
                        .to_owned(),
                user_grant_ref: "grant:gitlab:user".to_owned(),
                initiation: InitiationConfig::Provider,
                connect_session_ttl_seconds: 300,
                refresh_skew_seconds: 300,
            },
            Arc::new(MemoryStore::new()),
            GitlabState::Hosted(state.clone()),
            egress.clone(),
        )
        .await
        .expect("unbound metadata must not take unrelated integrations offline");
        assert_eq!(backend.connection_count(), 0);
        let owner = PrincipalContext::hosted(
            "tenant-one".to_owned(),
            "person:owner".to_owned(),
            "person:owner".to_owned(),
            Some("owner@example.test".to_owned()),
            "snapshot:current".to_owned(),
            "a".repeat(64),
        )
        .unwrap();
        assert!(backend.inner.owned_connections(&owner).is_empty());
        let refusal = backend
            .inner
            .connection_token(&metadata.connections[0])
            .await
            .unwrap_err();
        assert_eq!(refusal.code, "connection-grant");
        assert_eq!(egress.calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            state.read(STATE_KEY, MAX_STATE_BYTES).unwrap().unwrap(),
            original
        );

        let session = backend
            .inner
            .create_session(&owner, "GitLab".to_owned(), GitlabProfile::PersonalToken)
            .unwrap();
        let url = url::Url::parse(session.browser_completion_url.as_deref().unwrap()).unwrap();
        let capability = url.fragment().unwrap().strip_prefix("token=").unwrap();
        backend
            .complete_hosted_session(
                &session.connect_session_ref,
                capability,
                HostedCompletionSubmission::new(b"synthetic-fresh-token".to_vec()),
            )
            .await
            .unwrap();
        assert_eq!(egress.calls.load(Ordering::SeqCst), 2);
        let connections = backend.inner.owned_connections(&owner);
        assert_eq!(connections.len(), 1);
        assert_eq!(
            connections[0].connection_ref,
            metadata.connections[0].connection_ref
        );
        assert_eq!(connections[0].grant_ref, "grant:gitlab:user");
        assert_eq!(backend.connection_count(), 1);
        let persisted: StateFile =
            serde_json::from_slice(&state.read(STATE_KEY, MAX_STATE_BYTES).unwrap().unwrap())
                .unwrap();
        assert_eq!(persisted.connections.len(), 1);
        assert_eq!(persisted.pending.len(), 1);
        assert!(persisted.pending[0].connection.grant_ref.is_empty());
    }

    #[tokio::test]
    async fn malformed_state_and_empty_grant_policy_still_fail_closed() {
        let (baseline, egress) = paged_backend("").await;
        for invalid_version in [true, false] {
            let state = Arc::new(connector_state::MemoryState::new());
            let mut metadata = StateFile::default();
            if invalid_version {
                metadata.version = 0;
            } else {
                metadata.next_transaction_generation = 0;
            }
            state
                .replace(
                    STATE_KEY,
                    &serde_json::to_vec(&metadata).unwrap(),
                    MAX_STATE_BYTES,
                )
                .unwrap();
            let opened = GitlabBackend::open_inner(
                "tenant-one".to_owned(),
                baseline.inner.policy.clone(),
                Arc::new(MemoryStore::new()),
                GitlabState::Hosted(state),
                egress.clone(),
            )
            .await;
            assert!(matches!(opened, Err(error) if error.code == "connection-state"));
        }
        let mut policy = baseline.inner.policy.clone();
        policy.user_grant_ref.clear();
        let opened = GitlabBackend::open_inner(
            "tenant-one".to_owned(),
            policy,
            Arc::new(MemoryStore::new()),
            GitlabState::Hosted(Arc::new(connector_state::MemoryState::new())),
            egress.clone(),
        )
        .await;
        assert!(matches!(opened, Err(error) if error.code == "connection-grant"));
        assert_eq!(egress.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn project_admission_reaches_a_repository_after_the_first_hundred() {
        let (backend, egress) = paged_backend("2").await;
        let mut selected = None;
        let stopped = backend
            .inner
            .scan_membership_projects(
                "connection:gitlab:one",
                &Secret::new("synthetic-gitlab-token"),
                true,
                |project| {
                    let id = project.get("id").and_then(Value::as_u64);
                    if id == Some(101) {
                        selected = id;
                        true
                    } else {
                        false
                    }
                },
            )
            .await
            .unwrap();

        assert!(stopped);
        assert_eq!(selected, Some(101));
        assert_eq!(egress.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn project_admission_refuses_a_non_advancing_continuation() {
        let (backend, egress) = paged_backend("1").await;
        let result = backend
            .inner
            .scan_membership_projects(
                "connection:gitlab:one",
                &Secret::new("synthetic-gitlab-token"),
                false,
                |_| false,
            )
            .await;

        assert!(result.is_err());
        assert_eq!(egress.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn origins_are_exact_https_only() {
        assert!(parse_origin("https://gitlab.example.test").is_ok());
        for invalid in [
            "http://gitlab.example.test",
            "https://user@gitlab.example.test",
            "https://gitlab.example.test:8443",
            "https://gitlab.example.test/group",
        ] {
            assert!(parse_origin(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn profiles_are_closed_and_self_service() {
        assert_eq!(
            GitlabProfile::parse(Some(PROFILE_OAUTH)),
            Some(GitlabProfile::OAuthUser)
        );
        assert_eq!(
            GitlabProfile::parse(Some(PROFILE_PAT)),
            Some(GitlabProfile::PersonalToken)
        );
        assert_eq!(GitlabProfile::parse(Some("gitlab.bot")), None);

        let profiles = crate::profiles::setup_profiles(INTEGRATION_REF);
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].auth_profile, PROFILE_OAUTH);
        assert_eq!(profiles[1].auth_profile, PROFILE_PAT);
        assert!(profiles
            .iter()
            .all(|profile| matches!(profile.actor, protocol::catalog::SetupProfileActor::Person)));
        assert!(crate::profiles::setup_profiles("jira").is_empty());
    }

    #[test]
    fn datasource_projection_drops_sensitive_and_unknown_fields() {
        let projected = project_record(
            "gitlab.users",
            DatasourceRecordView::Compact,
            &serde_json::json!({
                "id": 7,
                "username": "dev",
                "name": "Developer",
                "state": "active",
                "bot": false,
                "email": "sentinel@example.test",
                "private_profile": false,
                "identities": [{"extern_uid":"SENTINEL"}]
            }),
        )
        .unwrap();
        assert!(projected.get("email").is_none());
        assert!(projected.get("identities").is_none());
        assert!(projected.get("private_profile").is_none());
    }

    #[test]
    fn datasource_cursors_are_bound_to_connection_and_project() {
        let cursor = datasource_cursor("gitlab.issues", "connection:gitlab:a", Some(42), 2);
        assert_eq!(
            parse_datasource_cursor(&cursor, "gitlab.issues", "connection:gitlab:a", Some(42)),
            Ok(2)
        );
        assert!(
            parse_datasource_cursor(&cursor, "gitlab.issues", "connection:gitlab:a", Some(43))
                .is_err()
        );
    }

    #[test]
    fn pat_shape_rejects_whitespace_and_oversize_values() {
        assert!(parse_pat(b"glpat-example-token").is_ok());
        assert!(parse_pat(b"glpat bad").is_err());
        assert!(parse_pat(&vec![b'x'; 4_097]).is_err());
    }

    #[test]
    fn repository_file_paths_are_encoded_as_one_gitlab_segment() {
        let operation = connector_resolve::document::operation(REPOSITORY_FILE_GET).unwrap();
        let credential = connector_resolve::auth::Assembled::new(
            "gitlab.token",
            "SENTINEL-NOT-A-REAL-SECRET".to_owned(),
            catalog::Placement::Header {
                name: "Authorization",
                prefix: "Bearer ",
            },
        );
        let plan = resolve_operation_plan(
            operation,
            "https://gitlab.example.test/api/v4",
            &serde_json::json!({
                "project_id": 813,
                "file_path": "bootstrap/deployer-rbac.yaml",
                "ref": "5a0d0ad26adaa0b4de31951edd68620db7045179"
            }),
            &[credential],
        )
        .unwrap();

        assert_eq!(
            plan.request.url,
            "https://gitlab.example.test/api/v4/projects/813/repository/files/bootstrap%2Fdeployer-rbac.yaml?ref=5a0d0ad26adaa0b4de31951edd68620db7045179"
        );
        assert_eq!(plan.permission_subjects, vec![plan.request.url.clone()]);
    }

    #[test]
    fn repository_file_paths_cannot_traverse_or_change_root() {
        for path in [
            "/root",
            "../secret",
            "dir/../secret",
            "dir//file",
            ".git/config",
        ] {
            assert!(!valid_repository_file_path(path), "{path}");
        }
        assert!(valid_repository_file_path(
            ".gitlab/agents/devcenter/config.yaml"
        ));
    }

    // The four below characterise the OAuth behaviour that moved to `connector-oauth`. They were
    // written with the move, because the five tests above it do not reach the OAuth path at all —
    // "the existing tests still pass" was true of this refactor and proved nothing about it.

    fn exchange_response() -> TokenResponse {
        TokenResponse {
            access_token: "SENTINEL-NOT-A-REAL-SECRET".to_owned(),
            refresh_token: Some("SENTINEL-NOT-A-REAL-REFRESH".to_owned()),
            expires_in: 7_200,
            created_at: Some(1_700_000_000),
            scope: "api".to_owned(),
            token_type: "Bearer".to_owned(),
        }
    }

    #[test]
    fn recorded_scopes_are_the_retained_subset_sorted_and_deduped() {
        assert_eq!(
            canonical_scopes(vec![
                "sudo".to_owned(),
                "api".to_owned(),
                "read_api".to_owned(),
                "api".to_owned(),
            ]),
            vec!["api".to_owned(), "read_api".to_owned()],
            "a token granted `sudo` is recorded as holding only what this connector relies on"
        );
    }

    #[test]
    fn the_exchange_policy_refuses_everything_the_inline_condition_refused() {
        type Break = fn(&mut TokenResponse);
        let cases: &[(&str, Break)] = &[
            ("token_type", |r| r.token_type = "MAC".to_owned()),
            ("empty access", |r| r.access_token = String::new()),
            ("oversize access", |r| r.access_token = "a".repeat(4_097)),
            ("empty refresh", |r| r.refresh_token = Some(String::new())),
            ("oversize refresh", |r| {
                r.refresh_token = Some("a".repeat(4_097));
            }),
            ("zero expires_in", |r| r.expires_in = 0),
            ("zero created_at", |r| r.created_at = Some(0)),
            ("scope without api", |r| r.scope = "read_api".to_owned()),
        ];
        for (name, break_it) in cases {
            let mut response = exchange_response();
            break_it(&mut response);
            assert!(
                connector_oauth::validate(response, &EXCHANGE_POLICY).is_err(),
                "{name} must refuse"
            );
        }
        assert!(connector_oauth::validate(exchange_response(), &EXCHANGE_POLICY).is_ok());
    }

    #[test]
    fn the_refresh_policy_ignores_the_two_fields_that_path_recomputes() {
        let mut response = exchange_response();
        response.expires_in = 0;
        response.created_at = Some(0);
        assert!(
            connector_oauth::validate(response, &REFRESH_POLICY).is_ok(),
            "expiry comes from /oauth/token/info on this path, never from the response"
        );
    }

    #[test]
    fn the_refresh_policy_still_requires_bearer_and_a_refresh_token() {
        type Break = fn(&mut TokenResponse);
        let cases: &[(&str, Break)] = &[
            ("token_type", |r| r.token_type = "MAC".to_owned()),
            ("empty access", |r| r.access_token = String::new()),
            ("empty refresh", |r| r.refresh_token = Some(String::new())),
        ];
        for (name, break_it) in cases {
            let mut response = exchange_response();
            break_it(&mut response);
            assert!(
                connector_oauth::validate(response, &REFRESH_POLICY).is_err(),
                "{name} must refuse on refresh too"
            );
        }
    }

    #[test]
    fn a_gitlab_refresh_response_without_scope_is_accepted_for_live_reverification() {
        let response: OAuthTokenResponse = serde_json::from_value(serde_json::json!({
            "access_token": "SENTINEL-NOT-A-REAL-SECRET-access",
            "token_type": "Bearer",
            "expires_in": 7_200,
            "refresh_token": "SENTINEL-NOT-A-REAL-SECRET-refresh",
            "created_at": 1_724_500_000
        }))
        .expect("GitLab's documented refresh response shape");

        let carried = token_response(response);
        assert!(carried.scope.is_empty());
        connector_oauth::validate(carried, &REFRESH_POLICY)
            .expect("scope is verified from /oauth/token/info after refresh");
    }

    /// The adapter carries every field across, and marks the two GitLab always sends as present.
    ///
    /// The four policy tests above construct a `TokenResponse` directly, which skips the one place
    /// a field could be dropped on the way in. `refresh_token` and `created_at` are `Option` on the
    /// shared type and mandatory on GitLab's, and `EXCHANGE_POLICY` refuses `None` for both — so an
    /// adapter that lost either would turn a good exchange into `oauth-exchange` at runtime and
    /// nothing here would have said so.
    #[test]
    fn the_adapter_carries_every_field_gitlab_sends() {
        let carried = token_response(OAuthTokenResponse {
            access_token: "SENTINEL-NOT-A-REAL-SECRET-access".to_owned(),
            refresh_token: "SENTINEL-NOT-A-REAL-SECRET-refresh".to_owned(),
            expires_in: 7_200,
            created_at: 1_724_500_000,
            scope: Some("api read_api".to_owned()),
            token_type: "Bearer".to_owned(),
        });

        assert_eq!(carried.access_token, "SENTINEL-NOT-A-REAL-SECRET-access");
        assert_eq!(
            carried.refresh_token.as_deref(),
            Some("SENTINEL-NOT-A-REAL-SECRET-refresh")
        );
        assert_eq!(carried.expires_in, 7_200);
        assert_eq!(carried.created_at, Some(1_724_500_000));
        assert_eq!(carried.scope, "api read_api");
        assert_eq!(carried.token_type, "Bearer");

        connector_oauth::validate(carried, &EXCHANGE_POLICY)
            .expect("what the adapter produces is what the exchange policy accepts");
    }
}

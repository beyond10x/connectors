#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::{PROFILE_OAUTH, PROFILE_PAT};
    use crate::repository_file::valid_repository_file_path;

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

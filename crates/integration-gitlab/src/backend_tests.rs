#[cfg(test)]
mod tests {
    use super::*;

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
    fn the_refresh_policy_still_requires_bearer_a_refresh_token_and_api_scope() {
        type Break = fn(&mut TokenResponse);
        let cases: &[(&str, Break)] = &[
            ("token_type", |r| r.token_type = "MAC".to_owned()),
            ("empty access", |r| r.access_token = String::new()),
            ("empty refresh", |r| r.refresh_token = Some(String::new())),
            ("scope without api", |r| r.scope = "read_api".to_owned()),
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
}

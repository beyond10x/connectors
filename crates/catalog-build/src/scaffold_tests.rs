#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_select_argument_drops_fields_from_the_right() {
        let whole = parse_select("manager").unwrap();
        assert_eq!(whole.service.as_deref(), Some("manager"));
        assert_eq!(whole.path_prefix, None);
        assert!(whole.methods.is_empty());

        let full = parse_select("manager:/api/v2:GET,DELETE").unwrap();
        assert_eq!(full.path_prefix.as_deref(), Some("/api/v2"));
        assert_eq!(full.methods, vec![HttpMethod::Get, HttpMethod::Delete]);

        // An empty field states nothing, which is C-411's absent key rather than a wildcard this
        // grammar had to invent.
        let unqualified = parse_select(":/api/v2:GET").unwrap();
        assert_eq!(unqualified.service, None);
        assert_eq!(unqualified.path_prefix.as_deref(), Some("/api/v2"));
    }

    #[test]
    fn a_select_argument_refuses_what_it_cannot_mean() {
        assert!(parse_select("a:b:c:d").is_err());
        assert!(parse_select("manager:/api/v2:FETCH").is_err());
    }

    #[test]
    fn a_prefix_matches_whole_segments() {
        assert!(path_has_prefix(
            "/api/v2/agents/{id}",
            Some("/api/v2/agents")
        ));
        assert!(path_has_prefix("/api/v2/agents", Some("/api/v2/agents")));
        assert!(!path_has_prefix(
            "/api/v2/agentsummary",
            Some("/api/v2/agents")
        ));
        assert!(path_has_prefix("/anything", None));
    }

    #[test]
    fn a_service_name_is_cut_at_the_pull_date() {
        assert_eq!(
            service_name_of("specs/babelforce/manager-2026-07-10.openapi.yaml"),
            "manager"
        );
        assert_eq!(
            service_name_of("specs/babelforce/task-automation-2026-06-25.openapi.yaml"),
            "task-automation"
        );
        assert_eq!(service_name_of("specs/zendesk/support.json"), "support");
    }
}

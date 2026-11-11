//! The catalog provider's integration tests, built into one test binary.
//! Each file is a module named by its stem, so a test's name is the file's
//! stem followed by the path it had when the file was its own binary.

mod hand_written;

#[path = "../../../crates/connectors-host/tests/fixtures/clock/server.rs"]
mod clock_fixture;

mod adversary_catalog_body_types;
mod adversary_credential_pass1;
mod adversary_git_relay;
mod adversary_gitlab_mr_writes;
mod binary_adversary;
mod binary_responses;
mod body_keys_adversary;
mod bundle_drift;
mod claude_subscription_guide_adversary;
mod confluence;
mod confluence_binding;
mod confluence_reads_adversary;
mod confluence_v1;
mod confluence_v1_feed;
mod connector_declarations;
mod created_credential;
mod credential_revision_security;
mod definition_auth;
mod definition_feed_engine_adversary;
mod definition_published_features_adversary;
mod derived_instance_adversary;
mod engine;
mod engine_adversary;
mod engine_adversary_pass2;
mod enum_date_time_adversary;
mod feed_capabilities;
mod feed_engine;
mod feed_extensions_security_review;
mod feed_utc_offsets;
mod feed_wave1021_adversary;
mod feed_wave1021_adversary_u6;
mod gateway_prefix;
mod gateway_prefix_adversary;
mod git_relay;
mod github_definition_reads;
mod github_definition_reads_adversary;
mod gitlab_ci_project_writes;
mod gitlab_ci_project_writes_adversary;
mod gitlab_code_search;
mod gitlab_commit_reads;
mod gitlab_commit_reads_adversary;
mod gitlab_commit_reads_adversary_pass2;
mod gitlab_deployment_reads;
mod gitlab_deployment_reads_adversary;
mod gitlab_feed;
mod gitlab_feed_u23_adversary;
mod gitlab_feed_u23_adversary_pass2;
mod gitlab_feed_wave1021_adversary;
mod gitlab_issue_create;
mod gitlab_mr_reads;
mod gitlab_mr_variants;
mod gitlab_mr_writes;
mod gitlab_raw_file_languages;
mod gitlab_repository_reads_adversary;
mod gitlab_repository_tree_reads;
mod gitlab_tags_releases;
mod google_authorize_url;
mod google_calendar;
mod google_calendar_gmail_adversary;
mod google_calendar_gmail_adversary_pass2;
mod google_calendar_writes_adversary;
mod google_calendar_writes_adversary_pass2;
mod google_drive;
mod google_drive_writes_adversary;
mod google_drive_writes_adversary_pass2;
mod google_gmail;
mod google_gmail_writes_adversary;
mod google_quota_reasons;
mod google_reads_adversary;
mod google_reads_adversary_pass2;
mod google_slides;
mod google_slides_writes_adversary;
mod google_slides_writes_adversary_pass2;
mod grafana;
mod grafana_adversary;
mod grafana_security_review;
mod grafana_security_review_r2;
mod guard_optional_values_adversary;
mod guard_optional_values_adversary_pass2;
mod guard_read_fixed_body_adversary;
mod guide_instances;
mod hubspot;
mod jira;
mod jira_attachment;
mod jira_cloud_reads_adversary;
mod jira_feed;
mod jira_feed_wave1021_adversary;
mod jira_issue_writes;
mod jira_issue_writes_adversary;
mod jira_transition_run;
mod jira_write_postflight_present;
mod local_runtime;
mod nested_closed_body;
mod openrouter;
mod opsgenie;
mod parameter_types;
mod parameter_types_adversary;
mod runpod;
mod runpod_adversary;
mod security_git_relay;
mod security_reuse_source_token_origin;
mod selection_bounds_adversary;
mod selection_credential;
mod selection_values_bound;
mod selection_values_bound_adversary;
mod selection_withhold_adversary;
mod served;
mod served_review;
mod shipped;
mod slack;
mod slack_adversary;
mod slack_send_adversary;
mod version;
mod walk_adversary;
mod walk_declarations;
mod write_postflight_present_adversary;
mod write_postflight_present_adversary_2;
mod zendesk;

/// `autotests = false` leaves a file here that this one does not declare unbuilt
/// and unrun, with nothing to say so.
mod declared {
    use std::{collections::BTreeSet, fs, path::Path};

    #[test]
    fn every_test_file_is_declared_as_a_module() {
        let declared: BTreeSet<&str> = include_str!("main.rs")
            .lines()
            .filter_map(|line| line.strip_prefix("mod ")?.strip_suffix(';'))
            .collect();
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
        let undeclared: Vec<String> = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
            .filter_map(|path| Some(path.file_stem()?.to_str()?.to_owned()))
            .filter(|stem| stem != "main" && !declared.contains(stem.as_str()))
            .collect();
        assert!(
            undeclared.is_empty(),
            "test files main.rs does not declare: {undeclared:?}"
        );
    }
}

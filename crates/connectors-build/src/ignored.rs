//! Explicit, classified execution of the workspace's ignored libtest inventory.
use crate::Result;
use clap::{Args, ValueEnum};
use serde::Serialize;
use serde_json::Value;
#[cfg(unix)]
use std::os::unix::{
    fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    process::{CommandExt, ExitStatusExt},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::File,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

#[derive(Args, Debug)]
pub struct Options {
    /// Families to execute. Merely having credentials never selects live tests.
    #[arg(long, value_enum, default_value = "disposable", value_delimiter = ',')]
    family: Vec<Family>,
    /// List the compiled inventory and prerequisites without running tests.
    #[arg(long)]
    inventory: bool,
    /// Missing prerequisites in any selected family make the command fail.
    #[arg(long, conflicts_with = "inventory")]
    required: bool,
    /// Narrow the selected families to tests whose every prerequisite is one of
    /// these. Workspace compilation supplies both, so CI can require them.
    #[arg(long, value_enum, value_delimiter = ',')]
    only_prerequisite: Vec<Supplied>,
    #[arg(long, default_value = ".local/ignored-suites.json")]
    report: PathBuf,
    /// A short, physical, owner-private fixture directory for long checkouts.
    #[arg(long)]
    tmpdir: Option<PathBuf>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    Disposable,
    Live,
    Timing,
    Deferred,
}
/// A prerequisite the workspace build supplies, as `--only-prerequisite` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Supplied {
    /// `CONNECTORS_TEST_CATALOG_PROVIDER`, the built catalog provider.
    Provider,
    /// `CONNECTORS_TEST_CLI`, the built production CLI.
    Cli,
    /// The qualified GNOME Secret Service and `/usr/bin/dbus-daemon`, which the
    /// CI job installs; a host without them still reports the test missing.
    Custody,
}
impl Supplied {
    fn key(self) -> &'static str {
        match self {
            Supplied::Provider => PROVIDER,
            Supplied::Cli => CLI,
            Supplied::Custody => CUSTODY,
        }
    }
}
#[derive(Clone, Debug)]
struct Suite {
    package: String,
    target: String,
    executable: PathBuf,
}
#[derive(Clone, Debug)]
struct Entry {
    suite: Suite,
    name: String,
    family: Option<Family>,
    helper: bool,
    prerequisites: Vec<&'static str>,
}
impl Entry {
    fn id(&self) -> String {
        format!(
            "{}::{}::{}",
            self.suite.package, self.suite.target, self.name
        )
    }
}
#[derive(Debug, Serialize)]
struct Outcome {
    name: String,
    family: Option<Family>,
    selected: bool,
    status: String,
    reasons: Vec<String>,
}
#[derive(Debug, Default, Serialize)]
struct Report {
    inventory: usize,
    selected: usize,
    executed: usize,
    skipped: usize,
    failed: usize,
    missing: usize,
    unknown: usize,
    outcomes: Vec<Outcome>,
}
impl Report {
    fn accepted(&self, required: bool) -> bool {
        self.failed == 0
            && self.unknown == 0
            && (!required
                || (self.missing == 0 && self.selected > 0 && self.executed == self.selected))
    }
}
const CUSTODY: &str = "qualified GNOME Secret Service and /usr/bin/dbus-daemon";
const CLI: &str = "CONNECTORS_TEST_CLI";
const PROVIDER: &str = "CONNECTORS_TEST_CATALOG_PROVIDER";
const CHROME: &str = "google-chrome-stable";
const OLD_CLI: &str = "CONNECTORS_ADVERSARY_PRE_HANDSHAKE";
const PG: &str = "CONNECTORS_PG_SANDBOX";
const PG_CONTAINER: &str = "CONNECTORS_PG_CONTAINER";
const PG_ADVERSARY: &str = "CONNECTORS_PG_ADVERSARY_PORT";
const MYSQL_ADVERSARY: &str = "CONNECTORS_MYSQL_ADVERSARY_PORT";
const PG_SECURITY: &str = "CONNECTORS_PG_SECURITY_PORT";
const DOCKER: &str = "docker";
const K8S: &str = "CONNECTORS_K8S_SANDBOX";
const CA: &str = "CONNECTORS_K8S_CA";
const TOKEN: &str = "CONNECTORS_K8S_TOKEN";
const KUBECONFIG: &str = "CONNECTORS_K8S_KUBECONFIG";
const KUBECTL: &str = "/usr/bin/kubectl";
const SECCOMP: &str = "Linux x86_64 seccomp USER_NOTIF";
// Same qualified artifact as the custody implementation; a test below detects pin drift.
const GNOME_SHA256: &str = "b9a71f6b4c4bfaf1759a99036b7b3ab6cf98b2b479c0c2a24f02f5f2ca53958b";

/// Each entry names an exact package, target and test. New tests, even in an
/// existing module, are unknown until someone reviews their authority and needs.
fn classify(suite: &Suite, name: &str) -> Entry {
    use Family::*;
    let mut entry = Entry {
        suite: suite.clone(),
        name: name.into(),
        family: None,
        helper: false,
        prerequisites: vec![],
    };
    let classification = match (suite.package.as_str(), suite.target.as_str(), name) {
        ("connectors-host", "connectors_host", name) => match name {
            "local::mutations::tests::crash_child"
            | "local::approvals::tests::crash_child"
            | "local::audit::tests::crash_child"
            | "local::approval_policy::tests::policy_child"
            | "local::approval_keys::tests::key_crash_child"
            | "local::runtime::launch::tests::consumer_fixture"
            | "local::runtime::launch::tests::launch_runner_fixture"
            | "local::runtime::launch::tests::review_conformance::reviewing_consumer_fixture"
            | "local::runtime::launch::tests::review_conformance::reviewing_runner_fixture"
            | "local::owner::refresh::tests::launch::launch_consumer_fixture" => {
                entry.helper = true;
                return entry;
            }
            "local::clock::tests::independent_live_quorum" => Some((Live, vec![])),
            "local::registry::tests::read_invoke_metadata_time"
            | "local::registry::store_cost_tests::read_invoke_cost_by_store_size"
            | "local::registry::store_cost_tests::batch_cost_by_subject"
            | "local::registry::store_cost_tests::first_owner_open_of_a_grown_store"
            | "local::registry::store_cost_tests::owner_start_of_a_grown_store"
            | "local::registry::store_cost_tests::two_revalidations_of_a_grown_store"
            | "local::registry::store_cost_tests::fresh_open_passes_of_a_blob_heavy_store" => {
                Some((Timing, vec![]))
            }
            "local::oauth_adversary_tests::chrome_following_the_redirect_is_accepted"
            | "local::oauth_adversary_tests::chrome_opens_one_connection_per_navigation"
            | "local::oauth_adversary_pass2_tests::chrome_with_loopback_cookies_following_the_redirect_is_accepted" => {
                Some((Disposable, vec![CHROME]))
            }
            "local::keyring::custody::tests::disposable_secret_service_restart_and_failures"
            | "local::owner::governed::tests::final_fence::withdrawal_at_final_dispatch_prevents_real_child_invocation"
            | "local::keyring::custody::tests::custody_neither_reads_nor_changes_the_default_alias"
            | "local::keyring::custody::tests::disposable_changed_keyring_format_is_refused_before_transfer"
            | "local::approval_keys::tests::durable_key_management_with_qualified_custody"
            | "local::approval_keys::tests::restart_failed_rotation_recovery_and_revocation_fences"
            | "local::approval_keys::tests::key_use_excludes_rotation_and_concurrent_init_has_one_winner"
            | "local::approval_keys::tests::wrong_recovery_material_never_publishes"
            | "local::approval_keys::tests::process_exits_keep_pre_and_post_publication_distinct"
            | "local::approval_keys::tests::unknown_storage_acknowledgement_requires_exact_reconfirmation"
            | "local::approval_keys::tests::purpose_isolation_for_identical_private_uuid_tuples"
            | "local::approval_keys::tests::uncertain_deletion_keeps_retirement_fence_and_exact_retry" => {
                Some((Disposable, vec![CUSTODY]))
            }
            "local::keyring::custody::tests::disposable_registry_publication_cli_and_retirement_restart"
            | "local::owner::approval_issuance::tests::production_cli_approval_issuance_and_restart"
            | "local::approval_keys::tests::production_cli_key_journey"
            | "local::runtime::launch::tests::disposable_consumer_launch_delivers_fd3_and_refuses_by_code"
            | "local::owner::refresh::tests::launch::launch_oauth2_grant_hands_only_the_access_token"
            | "local::owner::refresh::tests::launch::launch_oauth2_grant_refreshes_inside_lead_time"
            | "local::owner::refresh::tests::launch::launch_static_profile_is_unchanged"
            | "local::owner::refresh::tests::launch::adversary::adversary_launch_refresh_needing_reauthorization_names_repair_connection"
            | "local::owner::refresh::tests::launch::adversary::adversary_launch_rate_limited_refresh_answers_retry_after"
            | "local::owner::refresh::tests::launch::adversary::adversary_two_concurrent_launches_exchange_once"
            | "local::owner::refresh::tests::launch::adversary::adversary_configuration_changed_during_the_issue_is_lifecycle_conflict"
            | "local::owner::refresh::tests::launch::adversary::adversary_launch_inside_the_lead_after_evidence_lapsed_refreshes" => {
                Some((Disposable, vec![CUSTODY, CLI]))
            }
            _ => None,
        },
        // The contract suite against the real Secret Service, in its private
        // fixture. The macOS and Windows real-store cases compile on their own
        // platform only and run in CI (`custody-platforms.yml`), not here.
        (
            "connectors-custody",
            "connectors_custody",
            "tests::secret_service_answers_the_custody_contract"
            | "secret_service::inventory::tests::a_failed_barrier_still_names_the_deleted_items"
            | "tests::session_keyring::custody_writes_reach_the_default_alias_collection"
            | "tests::session_keyring::orphans_are_listed_and_removed_only_on_the_flag"
            | "tests::session_keyring::removing_an_authority_leaves_no_item_carrying_it"
            | "tests::session_keyring_review::a_daemon_without_login_receives_the_write_behind_the_alias"
            | "tests::session_keyring_review::moved_alias_writes_behind_it_reads_login_and_removal_stays_inside_the_authority",
        ) => Some((Disposable, vec![CUSTODY])),
        (
            "connectors-catalog-provider",
            "local_runtime",
            "cli_journey::guarded_merge::prepared_attempt_exit_fixture"
            | "cli_journey::guarded_merge::owner_replay::catalog_same_image_owner_fixture"
            | "cli_journey::serve_the_raw_file_fixture",
        )
        | ("connectors-catalog-provider", "openrouter", "serve_the_fixture") => {
            entry.helper = true;
            return entry;
        }
        (
            "connectors-catalog-provider",
            "local_runtime",
            "cli_journey::lifecycle::catalog_cli_read_recollects_expired_evidence_without_reentry",
        ) => Some((Timing, vec![CUSTODY, CLI])),
        (
            "connectors-catalog-provider",
            "local_runtime",
            "cli_journey::lifecycle::catalog_cli_failed_repair_and_busy_stop_preserve_authority"
            | "cli_journey::owner_guard::a_journey_that_panics_after_its_owner_started_leaves_no_owner_alive"
            | "cli_journey::owner_guard::a_journey_owner_exits_when_the_process_it_is_tied_to_ends"
            | "cli_journey::guarded_merge::catalog_cli_guarded_merge_applied_refused_and_lost_response_restart"
            | "cli_journey::guarded_merge::catalog_cli_guarded_merge_revocation_finishes_admitted_audit"
            | "cli_journey::guarded_merge::catalog_cli_recovers_abandoned_preparation_only_with_trusted_time"
            | "cli_journey::guarded_merge::catalog_cli_background_recovery_waits_for_live_work_and_recovers_unkeyed_attempts"
            | "cli_journey::guarded_merge::catalog_cli_background_recovers_revoked_removed_target_without_disclosure"
            | "cli_journey::guarded_merge::custody_journey::a_create_that_times_out_after_receipt_is_unknown_and_never_sent_again"
            | "cli_journey::guarded_merge::custody_journey::a_create_whose_operation_has_no_custody_target_is_refused_before_it_is_sent"
            | "cli_journey::guarded_merge::custody_journey::a_created_token_custody_refuses_is_reported_by_its_hash_and_held_nowhere"
            | "cli_journey::guarded_merge::custody_journey::a_created_token_is_held_as_a_new_connection_and_only_its_hash_is_returned"
            | "cli_journey::guarded_merge::custody_journey::a_write_after_an_executable_only_change_is_validated_once_and_sent"
            | "cli_journey::guarded_merge::custody_journey::security2_a_proof_issued_before_an_executable_only_change_authorizes_no_write"
            | "cli_journey::guarded_merge::custody_journey::security2_a_write_whose_new_executable_answers_another_identity_sends_nothing"
            | "cli_journey::revision_recovery::a_changed_evidence_lifetime_recovers_with_a_new_connection_in_two_commands"
            | "cli_journey::revision_recovery::no_next_action_after_a_revision_change_returns_to_an_earlier_refusal"
            | "cli_journey::revision_recovery::revalidate_replaces_the_adapter_process_of_an_earlier_entry"
            | "cli_journey::revision_recovery::a_selection_only_change_leaves_the_connection_ready_and_serves_the_added_read"
            | "cli_journey::revision_recovery::a_changed_ca_file_stales_the_connection_until_it_is_revalidated"
            | "cli_journey::revision_recovery::a_replaced_executable_lists_ready_and_one_validation_follows_it"
            | "cli_journey::revision_recovery::a_replaced_executable_answering_another_identity_changes_nothing"
            | "cli_journey::revision_recovery::a_connection_saved_without_the_digest_keeps_the_revision_rule_until_revalidated"
            | "cli_journey::revision_recovery::adv_the_added_read_succeeds_after_a_selection_only_change_without_revalidation"
            | "cli_journey::revision_recovery::adv_a_read_under_a_replaced_executable_validates_once_and_is_served"
            | "cli_journey::revision_recovery::adv2_a_stopped_instance_answers_description_unavailable_for_a_new_selection"
            | "cli_journey::revision_recovery::revision_recovery_security::security_the_added_read_succeeds_after_a_selection_only_change_without_revalidation"
            | "cli_journey::revision_recovery::revision_recovery_security::security_a_read_follows_an_executable_only_change_only_after_the_identity_answers"
            | "cli_journey::revision_recovery::revision_recovery_security::security2_a_revoked_connection_starts_nothing_for_a_description"
            | "cli_journey::revision_recovery::revision_recovery_security::security2_a_description_launch_does_not_resume_a_stopped_instance"
            | "cli_journey::revision_recovery::revision_recovery_security::security2_operations_describe_starts_no_owner_without_a_current_connection"
            | "cli_journey::revision_recovery_adversary::adversary_a_changed_profile_id_names_the_path_that_recovers"
            | "cli_journey::revision_recovery_adversary::adversary_a_lifetime_changed_and_reverted_leaves_the_first_connection_usable"
            | "cli_journey::revision_recovery_adversary::adversary_the_connection_left_by_a_changed_authentication_stays_revocable",
        ) => Some((Disposable, vec![CUSTODY, CLI])),
        (
            "connectors-catalog-provider",
            "local_runtime",
            "cli_journey::guarded_merge::catalog_cli_failed_settlement_keeps_the_known_effect_and_recovers_conservatively"
            | "cli_journey::guarded_merge::catalog_adversary_revoked_disclosure_of_an_applied_merge_never_answers_not_attempted",
        ) => Some((Disposable, vec![CUSTODY, CLI, SECCOMP])),
        (
            "connectors-catalog-provider",
            "local_runtime",
            "cli_journey::gitlab_catalog_cli_reuses_custody_across_owner_and_keyring_restart"
            | "cli_journey::gitlab_catalog_cli_reads_the_merge_request_feed_through_a_saved_connection"
            | "cli_journey::adversary_u23_a_read_result_the_child_admitted_reaches_the_cli_through_the_owner"
            | "cli_journey::basic_catalog_cli_connects_by_file_and_stdin_and_refuses_by_code"
            | "cli_journey::basic_catalog_cli_refuses_a_credential_below_minimum_scopes"
            | "cli_journey::adversary_basic_cli_wrong_token_is_unauthorized_and_malformed_is_invalid_input"
            | "cli_journey::a_provider_refusal_reads_dispatch_and_a_host_refusal_reads_admission"
            | "cli_journey::oauth_connect_journey"
            | "cli_journey::oauth_repair_journey"
            | "cli_journey::oauth2_grant_journey::oauth2_grant_connect_with_loopback_fixed_port"
            | "cli_journey::oauth2_grant_journey::oauth2_grant_connect_with_pasted_code"
            | "cli_journey::oauth2_grant_journey::oauth2_grant_connect_with_device_grant"
            | "cli_journey::oauth2_grant_journey::oauth2_grant_reconnect_replaces_the_entry_on_the_same_connection"
            | "cli_journey::oauth2_grant_journey::oauth2_grant_state_mismatch_stores_nothing"
            | "cli_journey::oauth2_grant_journey::oauth2_grant_device_expired_stores_nothing"
            | "cli_journey::oauth2_grant_journey::oauth2_grant_missing_required_scope_stores_nothing"
            | "cli_journey::oauth2_grant_journey::token_journey::token_without_reveal_prints_no_token"
            | "cli_journey::oauth2_grant_journey::token_journey::token_with_reveal_prints_only_the_token"
            | "cli_journey::oauth2_grant_journey::token_journey::token_within_lead_time_refreshes_once"
            | "cli_journey::oauth2_grant_journey::token_journey::token_on_a_stale_connection_refuses_with_repair_connection"
            | "cli_journey::oauth2_grant_journey::token_journey::token_on_a_static_profile_is_forbidden"
            | "cli_journey::oauth2_grant_journey::token_journey::library_access_token_matches_the_cli"
            | "cli_journey::chatgpt_journey::chatgpt_connect_browser_journey"
            | "cli_journey::chatgpt_journey::chatgpt_connect_device_journey"
            | "cli_journey::chatgpt_journey::chatgpt_token_refreshes_with_json_and_rotates"
            | "cli_journey::chatgpt_adversary::adversary_chatgpt_device_sign_in_refuses_an_interval_longer_than_the_window_before_any_poll"
            | "cli_journey::chatgpt_adversary::adversary_chatgpt_device_sign_in_keeps_polling_at_a_normal_interval"
            | "cli_journey::claude_subscription_journey::claude_connect_loopback_journey"
            | "cli_journey::claude_subscription_journey::claude_connect_pasted_code_journey"
            | "cli_journey::claude_subscription_journey::claude_token_refreshes_with_json"
            | "cli_journey::reason_cli_adversary::a_provider_reason_reaches_the_cli_failure_json_and_a_credential_echo_does_not"
            | "cli_journey::retry_after_cli::a_rate_limited_read_is_waited_for_once_or_states_its_delay_in_the_cli_json"
            | "oauth2_refresh::oauth_invalid_grant_reports_repair"
            | "oauth2_refresh::oauth_connect_refusal_does_not_say_repair"
            | "oauth2_refresh::oauth_invalid_grant_makes_the_connection_stale_until_a_reconnect"
            | "oauth2_refresh_adversary_pass2::adversary2_oauth_cached_bootstrap_round_trips_across_owner_restart"
            | "derived_instance::a_gmail_instance_configured_by_name_alone_connects_once_and_reads_and_writes_drafts"
            | "slack_approval::a_slack_bot_post_is_issued_the_approval_its_prepare_named_and_invoked_with_it"
            | "cli_journey::invoke_definition::a_hand_written_definition_connects_on_its_first_invoke_and_then_invokes_without_asking"
            | "cli_journey::invoke_definition::a_stored_credential_of_a_configured_instance_is_reused_without_a_token_prompt"
            | "cli_journey::invoke_definition::a_reuse_the_provider_answers_with_another_identity_publishes_nothing_and_offers_connect_fresh"
            | "cli_journey::invoke_definition::adversary_reuse_race::adversary_reuse_a_source_revoked_while_its_copy_validates_is_not_published"
            | "cli_journey::invoke_definition::a_definition_moved_to_another_host_connects_fresh_under_a_new_instance_id",
        ) => Some((Disposable, vec![CUSTODY, CLI])),
        (
            "connectors-catalog-provider",
            "confluence_binding",
            "a_v1_bound_connection_reads_the_v1_page_through_the_cli_and_rebinds_on_revalidation",
        ) => Some((Disposable, vec![CUSTODY, CLI])),
        (
            "connectors-loki",
            "local_runtime",
            "cli_journey::loki_cli_connects_with_a_bearer_token_and_answers_each_read_on_the_saved_connection",
        ) => Some((Disposable, vec![CUSTODY, CLI])),
        (
            "connectors-grafana",
            "local_runtime",
            "cli_journey::grafana_cli_connects_with_a_service_account_token_and_lists_datasources_on_the_saved_connection",
        ) => Some((Disposable, vec![CUSTODY, CLI])),
        (
            "connectors-prometheus",
            "local_runtime",
            "cli_journey::prometheus_cli_connects_with_a_bearer_token_and_answers_each_read_on_the_saved_connection"
            | "cli_journey::prometheus_cli_through_the_grafana_proxy_answers_each_read_on_the_saved_connection",
        ) => Some((Disposable, vec![CUSTODY, CLI])),
        ("connectors-kubernetes", "local_runtime", name) => match name {
            "cli_journey::persistent_kubernetes_cli_owner_and_keyring_restart"
            | "cli_journey::kubernetes_cli_refuses_a_changed_cluster_identity_and_preserves_the_saved_credential"
            | "cli_journey::kubernetes_cli_reads_helm_release_history_and_redacted_values"
            | "cli_journey::kubernetes_cli_reads_namespaces_single_objects_events_and_rollout_history" => {
                Some((Disposable, vec![CUSTODY, CLI]))
            }
            "cli_journey::a_real_cluster_session_persists_across_cli_and_owner_restart" => {
                Some((Live, vec![CUSTODY, CLI, K8S, CA, TOKEN]))
            }
            "cli_journey::kubernetes_cli_reuses_each_admitted_read_after_restart"
            | "cli_journey::kubernetes_cli_provider_rbac_denial_is_not_empty_success"
            | "cli_journey::kubernetes_cli_continuation_preserves_scope_and_revision"
            | "cli_journey::kubernetes_cli_repair_revoke_and_stop_preserve_authority" => Some((
                Live,
                vec![CUSTODY, CLI, K8S, CA, TOKEN, KUBECONFIG, KUBECTL],
            )),
            _ => None,
        },
        (
            "connectors-sql",
            "local_runtime",
            "cli_journey::a_real_postgres_session_persists_across_cli_and_owner_restart"
            | "cli_journey::postgres_cli_preserves_join_group_utc_and_quoted_parameters"
            | "cli_journey::postgres_cli_distinguishes_empty_truncated_capacity_and_timeout"
            | "cli_journey::postgres_cli_cannot_escape_read_only_transaction"
            | "cli_journey::postgres_repair_revoke_and_busy_stop_preserve_authority",
        ) => Some((Live, vec![CUSTODY, CLI, PG, PG_CONTAINER, DOCKER])),
        (
            "connectors-sql",
            "local_runtime",
            "cli_journey::postgres_dropped_invocation_cancels_its_backend",
        ) => Some((Live, vec![PG, PG_CONTAINER, DOCKER])),
        (
            "connectors-sql",
            "catalogue_adversary",
            "postgresql_composite_keys_expressions_and_case_on_a_live_server"
            | "postgresql_foreign_keys_into_what_the_role_cannot_see_are_left_out"
            | "postgresql_generated_column_is_not_reported_as_a_default"
            | "postgresql_index_list_does_not_name_a_column_the_role_cannot_see"
            | "postgresql_index_list_leaves_out_every_index_over_a_hidden_column",
        )
        | (
            "connectors-sql",
            "catalogue_adversary_2",
            "postgresql_a_composite_foreign_key_with_one_hidden_referenced_column_is_left_out"
            | "postgresql_a_foreign_key_does_not_name_a_referenced_column_describe_hides"
            | "postgresql_a_foreign_key_into_a_partitioned_table_is_reported_once"
            | "postgresql_a_partition_keeps_the_foreign_key_it_inherits"
            | "postgresql_index_list_leaves_out_a_whole_row_expression_over_a_hidden_column"
            | "postgresql_index_list_leaves_out_a_whole_row_predicate_over_a_hidden_column",
        )
        | (
            "connectors-sql",
            "adversary_pg_catalog",
            "postgresql_live_catalogue_reads_of_several_columns_answer"
            | "postgresql_live_read_never_runs_a_public_text_domain_check"
            | "postgresql_live_read_of_two_columns_returns_its_row"
            | "postgresql_live_type_lookup_never_runs_a_public_operator",
        ) => Some((Live, vec![PG_ADVERSARY])),
        (
            "connectors-sql",
            "security_pg_catalog",
            "live_catalogue_does_not_call_a_public_operator"
            | "live_driver_type_lookup_does_not_call_a_public_operator"
            | "live_size_guard_measures_what_the_page_sends"
            | "live_text_keyset_orders_by_bytes_under_a_public_c_collation"
            | "live_two_column_and_catalogue_reads_answer_without_any_shadow"
            | "live_wrapper_cast_does_not_run_a_public_text_domain"
            | "live_wrapper_does_not_call_a_public_octet_length_or_operator",
        ) => Some((Live, vec![PG_SECURITY])),
        (
            "connectors-sql",
            "catalogue_adversary",
            "mysql_a_name_mysql_cannot_represent_is_not_found_on_a_live_server"
            | "mysql_table_describe_of_a_name_with_a_trailing_space_is_not_found"
            | "mysql_table_names_are_case_sensitive_on_a_live_server",
        )
        | (
            "connectors-sql",
            "mysql_adversary_2",
            "live_mysql_distinct_timestamps_are_written_distinctly",
        ) => Some((Live, vec![MYSQL_ADVERSARY])),
        (
            "connectors",
            "failed_connect",
            "a_connect_to_an_unreachable_provider_reports_the_owners_cause_at_dispatch",
        ) => Some((Disposable, vec![CUSTODY, PROVIDER])),
        (
            "connectors",
            "failed_connect",
            "a_revalidation_the_owner_definitely_fails_reports_dispatch_at_the_cli"
            | "a_revalidation_after_a_configuration_change_reaches_the_owner_without_a_cached_description"
            | "an_adapter_identity_mismatch_during_an_upgrade_names_a_new_connection"
            | "an_adapter_identity_mismatch_outside_an_upgrade_names_a_repair",
        ) => Some((Disposable, vec![CUSTODY])),
        (
            "connectors",
            "connection_next_actions",
            "following_next_actions_from_a_refused_upgrade_does_not_cycle"
            | "a_pending_connection_says_why"
            | "adversary_the_walk_from_revalidate_satisfies_the_story"
            | "adversary_the_walk_from_reconnect_satisfies_the_story"
            | "adversary_the_walk_from_connect_satisfies_the_story",
        ) => Some((Disposable, vec![CUSTODY])),
        (
            "connectors",
            "definition_connect_question",
            "the_connect_question_names_the_host_the_token_is_sent_to"
            | "the_connect_question_of_an_oauth_profile_names_its_token_host_too"
            | "an_entry_another_run_recorded_while_the_question_waits_is_not_connected",
        ) => Some((Disposable, vec![PROVIDER])),
        (
            "connectors",
            "definition_connect_question",
            "an_entry_replaced_after_the_cli_checked_it_is_refused_at_begin_before_any_credential",
        ) => Some((Disposable, vec![PROVIDER, CUSTODY])),
        ("connectors", "adversary_d218_u1", "describe_of_a_definition_needs_no_connection") => {
            Some((Disposable, vec![PROVIDER]))
        }
        (
            "connectors",
            "setup_migrate_records",
            "migrated_entries_keep_their_connections_and_approval_policies",
        )
        | (
            "connectors",
            "setup_custody_orphans",
            "remove_deletes_only_the_named_foreign_authority"
            | "remove_refuses_the_stores_own_authority"
            | "remove_without_an_authority_deletes_nothing"
            | "the_listing_groups_foreign_authorities_and_says_it_cannot_see_other_stores",
        ) => Some((Disposable, vec![CUSTODY])),
        (
            "connectors",
            "cold_owner_start",
            "a_cold_describe_and_read_of_a_large_store_answer_within_60_seconds",
        ) => Some((Timing, vec![])),
        (
            "connectors",
            "local_cli",
            "adversary2_a_real_pre_handshake_owner_is_refused_by_name_and_the_reverse_is_observed",
        ) => Some((Disposable, vec![OLD_CLI])),
        (
            "connectors-conformance",
            "configuration_refusal_adversary",
            "the_contract_refuses_configuration_coordinates_that_are_not_the_declared_ones",
        )
        | (
            "connectors-catalog-provider",
            "jira_issue_writes_adversary",
            "a_comment_whose_content_is_null_is_refused_before_any_request",
        )
        | (
            "connectors-catalog-provider",
            "enum_date_time_adversary",
            "adversary_declared_input_schema_lists_the_pinned_enum",
        )
        | (
            "connectors-catalog-provider",
            "slack_adversary",
            "an_empty_user_or_email_sends_nothing",
        ) => Some((Deferred, vec![])),
        _ => None,
    };
    if let Some((family, prerequisites)) = classification {
        entry.family = Some(family);
        entry.prerequisites = prerequisites;
    }
    entry
}
fn discover(suite: &Suite) -> Result<Vec<Entry>> {
    let output = crate::run(Command::new(&suite.executable).args([
        "--ignored",
        "--list",
        "--format",
        "terse",
    ]))?;
    let mut entries = Vec::new();
    for line in std::str::from_utf8(&output.stdout)?
        .lines()
        .filter(|line| !line.is_empty())
    {
        let name = line
            .strip_suffix(": test")
            .ok_or_else(|| format!("unsupported inventory line in {}: {line}", suite.target))?;
        entries.push(classify(suite, name));
    }
    Ok(entries)
}
fn evaluate(
    entries: Vec<Entry>,
    options: &Options,
    mut missing: impl FnMut(&Entry) -> Vec<String>,
    mut execute: impl FnMut(&Entry) -> Result<bool>,
) -> Report {
    let mut report = Report {
        inventory: entries.len(),
        unknown: entries
            .iter()
            .filter(|e| e.family.is_none() && !e.helper)
            .count(),
        ..Report::default()
    };
    for entry in entries {
        let family = entry
            .family
            .is_some_and(|family| options.family.contains(&family));
        let within = options.only_prerequisite.is_empty()
            || entry.prerequisites.iter().all(|key| {
                options
                    .only_prerequisite
                    .iter()
                    .any(|supplied| supplied.key() == *key)
            });
        let selected = family && within;
        let mut outcome = Outcome {
            name: entry.id(),
            family: entry.family,
            selected,
            status: String::new(),
            reasons: vec![],
        };
        if entry.helper {
            outcome.status = "helper".into();
            outcome
                .reasons
                .push("subprocess-only entry; its parent owns execution".into());
        } else if entry.family.is_none() {
            outcome.status = "unknown".into();
            outcome
                .reasons
                .push("inventory classification required".into());
        } else if !family {
            outcome.status = "not-selected".into();
            outcome
                .reasons
                .push("family requires explicit selection".into());
        } else if !within {
            outcome.status = "not-selected".into();
            outcome
                .reasons
                .push("a prerequisite outside --only-prerequisite".into());
        } else {
            report.selected += 1;
            outcome.reasons = missing(&entry);
            if !outcome.reasons.is_empty() {
                report.missing += 1;
                outcome.status = "missing-prerequisite".into();
            } else if report.unknown != 0 {
                outcome.status = "blocked".into();
                outcome
                    .reasons
                    .push("unknown inventory prevents all execution".into());
            } else if options.inventory {
                outcome.status = "inventory-only".into();
                outcome
                    .reasons
                    .push("inventory mode executes no tests".into());
            } else {
                match execute(&entry) {
                    Ok(passed) => {
                        report.executed += 1;
                        outcome.status = if passed { "passed" } else { "failed" }.into();
                        if !passed {
                            report.failed += 1;
                            outcome
                                .reasons
                                .push("test reported a failure; see log".into());
                        }
                    }
                    Err(error) => {
                        report.failed += 1;
                        outcome.status = "execution-error".into();
                        outcome.reasons.push(error.to_string());
                    }
                }
            }
        }
        if outcome.status != "passed" && outcome.status != "failed" {
            report.skipped += 1;
        }
        report.outcomes.push(outcome);
    }
    report
}

/// Own one fixture group. The unreaped leader reserves its identity until the
/// group has been signalled. No signal is ever sent after reaping begins.
struct FixtureGroup {
    child: Child,
    signalled: bool,
    finished: bool,
    leader_status: Option<ExitStatus>,
}
/// Windows: no process groups and, without Win32 bindings in this crate, no
/// job object. Cleanup ends the fixture and the descendants it still parents
/// with `taskkill /T /F`; a descendant whose parent already exited is not
/// reached.
#[cfg(windows)]
impl FixtureGroup {
    fn exited(&mut self) -> Result<bool> {
        if let Some(status) = self.child.try_wait()? {
            self.leader_status = Some(status);
        }
        Ok(self.leader_status.is_some())
    }
    fn finish(&mut self) -> Result<ExitStatus> {
        if !self.signalled {
            Command::new("taskkill")
                .args(["/T", "/F", "/PID", &self.child.id().to_string()])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            self.signalled = true;
        }
        let status = self.child.wait()?;
        self.finished = true;
        Ok(*self.leader_status.get_or_insert(status))
    }
}
#[cfg(unix)]
impl FixtureGroup {
    fn exited(&self) -> Result<bool> {
        // SAFETY: zero is a valid initial siginfo_t; waitid initializes its
        // fields before observation. WNOWAIT deliberately retains the leader.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                self.child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY: successful waitid above initialized this siginfo_t.
        Ok(unsafe { info.si_pid() } != 0)
    }
    fn finish(&mut self) -> Result<ExitStatus> {
        // Linux child PIDs fit pid_t. This leader remains unreaped until the
        // first group signal, so its PID cannot refer to an unrelated group.
        let group = self.child.id() as libc::pid_t;
        if !self.signalled {
            // SAFETY: negative group addresses only the group created for our child.
            let result = unsafe { libc::kill(-group, libc::SIGKILL) };
            if result < 0 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ESRCH) {
                    return Err(error.into());
                }
            }
            self.signalled = true;
        }
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            let mut status = 0;
            // SAFETY: status is writable; this waits only for our fixture's
            // group, including orphan descendants adopted as a subreaper.
            let reaped = unsafe { libc::waitpid(-group, &mut status, libc::WNOHANG) };
            if reaped == group {
                self.leader_status = Some(ExitStatus::from_raw(status));
            }
            if reaped < 0 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() == Some(libc::ECHILD) {
                    self.finished = true;
                    return self
                        .leader_status
                        .ok_or_else(|| "fixture leader status unavailable".into());
                }
                if error.raw_os_error() != Some(libc::EINTR) {
                    return Err(error.into());
                }
            }
            if Instant::now() >= until {
                return Err("fixture group cleanup did not finish within two seconds".into());
            }
            if reaped <= 0 {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}
impl Drop for FixtureGroup {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.finish();
        }
    }
}

/// Execute one exact test, preserve its output and refuse an empty green selection.
/// The owned process group gets ten minutes, including costly opt-in probes.
fn execute_test(
    suite: &Suite,
    name: &str,
    temp: &Path,
    logs: &Path,
    environment: &BTreeMap<&str, OsString>,
) -> Result<bool> {
    let log = logs.join(format!(
        "{}.log",
        crate::hash(format!("{}::{}::{name}", suite.package, suite.target).as_bytes())
    ));
    let file = File::create(&log)?;
    // SAFETY: this Linux process adopts orphans, so fixture group members can
    // be reaped after their parent exits. Every wait below selects its own
    // group, never children of another concurrently executing fixture.
    #[cfg(target_os = "linux")]
    if unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) } < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // macOS has no subreaper. A fixture member whose parent exits is adopted
    // by launchd but keeps its process group, so the group SIGKILL in
    // `FixtureGroup::finish` still stops it; launchd, not this runner, reaps
    // it, and `finish` ends at ECHILD once the runner's own children of the
    // group are reaped. The group id cannot be reused while a member lives.
    let mut command = Command::new(&suite.executable);
    command
        .args([
            "--ignored",
            "--exact",
            name,
            "--nocapture",
            "--test-threads=1",
            "--format=pretty",
        ])
        .envs(environment)
        .env("TMPDIR", temp)
        .stdin(Stdio::null())
        .stdout(file.try_clone()?)
        .stderr(file);
    #[cfg(unix)]
    command.process_group(0);
    let child = command.spawn()?;
    let mut group = FixtureGroup {
        child,
        signalled: false,
        finished: false,
        leader_status: None,
    };
    let until = Instant::now() + Duration::from_secs(600);
    let timed_out = loop {
        if group.exited()? {
            break false;
        }
        if Instant::now() >= until {
            break true;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let success = group.finish()?.success() && !timed_out;
    let output = std::fs::read_to_string(&log)?;
    let passed = output
        .lines()
        .any(|line| line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;"));
    let failed = output
        .lines()
        .any(|line| line.starts_with("test result: FAILED. 0 passed; 1 failed; 0 ignored;"));
    if !passed && !failed {
        return Err("test did not report exactly one executed case (empty selection, timeout or runner error); see log".into());
    }
    Ok(success && passed)
}
#[cfg(test)]
fn execute_one(suite: &Suite, name: &str, temp: &Path) -> Result<bool> {
    execute_test(suite, name, temp, temp, &BTreeMap::new())
}

#[cfg(unix)]
fn executable(path: &Path) -> bool {
    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}
/// Windows has no execute bit: a regular file.
#[cfg(windows)]
fn executable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// A kubeconfig must be owner-only. Windows: this crate has no access control
/// bindings to check that, so the prerequisite is never met there.
#[cfg(unix)]
fn owner_only(metadata: &std::fs::Metadata) -> bool {
    metadata.permissions().mode() & 0o077 == 0
}
#[cfg(windows)]
fn owner_only(_: &std::fs::Metadata) -> bool {
    false
}
fn prerequisites(environment: &BTreeMap<&str, OsString>) -> BTreeMap<&'static str, String> {
    let mut missing = BTreeMap::new();
    if !seccomp_notification_abi_available() {
        missing.insert(SECCOMP, "settlement fixtures require Linux x86_64, the user_notif kernel action and notification ABI sizes 80/24/64; runtime policy must also permit listener installation and owned-process fd inspection".into());
    }
    let custody = executable(Path::new("/usr/bin/dbus-daemon"))
        && std::fs::read("/usr/bin/gnome-keyring-daemon")
            .is_ok_and(|bytes| crate::hash(&bytes) == GNOME_SHA256);
    if !custody {
        missing.insert(CUSTODY, "requires dbus-daemon and the exact qualified GNOME Keyring artifact (see docs/local-secret-service.md)".into());
    }
    for key in [CLI, PROVIDER, OLD_CLI] {
        let path = environment
            .get(key)
            .cloned()
            .or_else(|| std::env::var_os(key));
        if !path.is_some_and(|path| executable(Path::new(&path))) {
            missing.insert(key, format!("{key} must name an executable file"));
        }
    }
    if !std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .any(|path| executable(&path.join(CHROME)))
    {
        missing.insert(
            CHROME,
            "google-chrome-stable is not executable on PATH".into(),
        );
    }
    if !std::env::var(PG).ok().is_some_and(|value| {
        value.rsplit_once(':').is_some_and(|(host, port)| {
            !host.is_empty() && port.parse::<u16>().is_ok_and(|port| port > 0)
        })
    }) {
        missing.insert(
            PG,
            "CONNECTORS_PG_SANDBOX must name the prepared sandbox as host:port".into(),
        );
    }
    if !std::env::var(PG_CONTAINER).is_ok_and(|value| {
        value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
    }) {
        missing.insert(
            PG_CONTAINER,
            "CONNECTORS_PG_CONTAINER must name the owned disposable PostgreSQL container with psql"
                .into(),
        );
    }
    if !std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .any(|path| executable(&path.join(DOCKER)))
    {
        missing.insert(DOCKER, "docker is not executable on PATH".into());
    }
    if !std::env::var(K8S).is_ok_and(|value| !value.trim().is_empty()) {
        missing.insert(
            K8S,
            "CONNECTORS_K8S_SANDBOX must name the prepared sandbox API base".into(),
        );
    }
    for key in [PG_ADVERSARY, MYSQL_ADVERSARY, PG_SECURITY] {
        if !std::env::var(key).is_ok_and(|port| port.parse::<u16>().is_ok_and(|port| port > 0)) {
            missing.insert(
                key,
                format!("{key} must name the loopback port of a disposable server prepared as the test file says"),
            );
        }
    }
    if !executable(Path::new(KUBECTL)) {
        missing.insert(KUBECTL, "/usr/bin/kubectl is not executable".into());
    }
    for key in [CA, TOKEN, KUBECONFIG] {
        if !std::env::var_os(key).is_some_and(|path| {
            std::fs::metadata(path).is_ok_and(|metadata| {
                metadata.is_file() && (key != KUBECONFIG || owner_only(&metadata))
            })
        }) {
            missing.insert(
                key,
                format!("{key} must name the sandbox's owner-only file"),
            );
        }
    }
    missing
}

fn seccomp_notification_abi_available() -> bool {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        if !std::fs::read_to_string("/proc/sys/kernel/seccomp/actions_avail").is_ok_and(|actions| {
            actions
                .split_whitespace()
                .any(|action| action == "user_notif")
        }) {
            return false;
        }
        const GET_NOTIF_SIZES: libc::c_uint = 3;
        let mut sizes = [0_u16; 3];
        // SAFETY: GET_NOTIF_SIZES only writes the documented three-u16 output;
        // it installs no filter and changes no process policy.
        let result =
            unsafe { libc::syscall(libc::SYS_seccomp, GET_NOTIF_SIZES, 0, sizes.as_mut_ptr()) };
        result == 0 && sizes == [80, 24, 64]
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        false
    }
}

/// Build every default-feature workspace test target, then query the actual
/// libtest executables. Cargo metadata maps opaque package IDs to names.
fn workspace(root: &Path) -> Result<(Vec<Suite>, BTreeMap<&'static str, OsString>)> {
    let metadata = cargo_metadata(root)?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("Cargo workspace members missing")?;
    let packages: BTreeMap<_, _> = metadata["packages"]
        .as_array()
        .ok_or("Cargo packages missing")?
        .iter()
        .filter(|package| members.contains(&package["id"]))
        .map(|package| {
            Ok((
                package["id"]
                    .as_str()
                    .ok_or("Cargo package id missing")?
                    .to_owned(),
                package["name"]
                    .as_str()
                    .ok_or("Cargo package name missing")?
                    .to_owned(),
            ))
        })
        .collect::<Result<_>>()?;
    let custom = custom_harness_targets(&metadata)?;
    let output = crate::run(
        Command::new("cargo")
            .current_dir(root)
            .env("CARGO_BUILD_JOBS", "2")
            .args([
                "test",
                "--workspace",
                "--no-run",
                "--locked",
                "--offline",
                "--message-format=json",
            ]),
    )?;
    let mut suites = Vec::new();
    let mut environment = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for line in std::str::from_utf8(&output.stdout)?.lines() {
        let artifact: Value = serde_json::from_str(line)?;
        if artifact["reason"] != "compiler-artifact" {
            continue;
        }
        let Some(package) = artifact["package_id"]
            .as_str()
            .and_then(|id| packages.get(id))
        else {
            continue;
        };
        let Some(executable) = artifact["executable"].as_str() else {
            continue;
        };
        let target = artifact["target"]["name"]
            .as_str()
            .ok_or("Cargo target name missing")?;
        if artifact["profile"]["test"] == true
            && inventoried(&custom, package, target)
            && seen.insert(executable.to_owned())
        {
            suites.push(Suite {
                package: package.clone(),
                target: target.into(),
                executable: executable.into(),
            });
        } else if artifact["profile"]["test"] == false {
            let key = match (package.as_str(), target) {
                ("connectors", "connectors") => Some(CLI),
                ("connectors-catalog-provider", "connectors-catalog-provider") => Some(PROVIDER),
                _ => None,
            };
            if let Some(key) = key {
                environment.insert(key, OsString::from(executable));
            }
        }
    }
    if suites.is_empty() {
        return Err("Cargo produced no workspace test executables".into());
    }
    suites.sort_by(|a, b| (&a.package, &a.target).cmp(&(&b.package, &b.target)));
    Ok((suites, environment))
}

/// The `[[test]]` targets each workspace member declares with `harness = false`, as
/// `(package, target)`. Such a target runs its own `main` rather than libtest, so it answers
/// `--ignored --list --format terse` with no test list and is not inventoried. Cargo metadata
/// does not carry the harness setting, so it is read from each member's manifest.
fn custom_harness_targets(metadata: &Value) -> Result<BTreeSet<(String, String)>> {
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("Cargo workspace members missing")?;
    let mut targets = BTreeSet::new();
    for package in metadata["packages"]
        .as_array()
        .ok_or("Cargo packages missing")?
        .iter()
        .filter(|package| members.contains(&package["id"]))
    {
        let name = package["name"]
            .as_str()
            .ok_or("Cargo package name missing")?;
        let manifest = package["manifest_path"]
            .as_str()
            .ok_or("Cargo manifest path missing")?;
        let text =
            std::fs::read_to_string(manifest).map_err(|e| format!("read {manifest}: {e}"))?;
        for target in harness_false_tests(&text).map_err(|e| format!("{manifest}: {e}"))? {
            targets.insert((name.to_owned(), target));
        }
    }
    Ok(targets)
}

/// The names of the `[[test]]` entries of one manifest that set `harness = false`.
fn harness_false_tests(manifest: &str) -> Result<Vec<String>> {
    let manifest: toml::Table = toml::from_str(manifest)?;
    let Some(tests) = manifest.get("test") else {
        return Ok(Vec::new());
    };
    let tests = tests
        .as_array()
        .ok_or("[[test]] is not an array of tables")?;
    tests
        .iter()
        .filter(|test| test.get("harness").and_then(toml::Value::as_bool) == Some(false))
        .map(|test| {
            test.get("name")
                .and_then(toml::Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| "a [[test]] with harness = false names no target".into())
        })
        .collect()
}

/// Whether a built test target is a libtest suite the inventory lists.
fn inventoried(custom: &BTreeSet<(String, String)>, package: &str, target: &str) -> bool {
    !custom.contains(&(package.to_owned(), target.to_owned()))
}

fn cargo_metadata(root: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(
        &crate::run(Command::new("cargo").current_dir(root).args([
            "metadata",
            "--locked",
            "--offline",
            "--no-deps",
            "--format-version=1",
        ]))?
        .stdout,
    )?)
}

/// Every libtest harness target of every workspace member, with its crate root.
pub(crate) fn test_targets(root: &Path) -> Result<Vec<crate::platform_gated::Target>> {
    let metadata = cargo_metadata(root)?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("Cargo workspace members missing")?;
    let mut targets = Vec::new();
    for package in metadata["packages"]
        .as_array()
        .ok_or("Cargo packages missing")?
        .iter()
        .filter(|package| members.contains(&package["id"]))
    {
        let name = package["name"]
            .as_str()
            .ok_or("Cargo package name missing")?;
        for target in package["targets"]
            .as_array()
            .ok_or("Cargo targets missing")?
            .iter()
            .filter(|target| target["test"] == true)
        {
            targets.push(crate::platform_gated::Target {
                package: name.into(),
                target: target["name"]
                    .as_str()
                    .ok_or("Cargo target name missing")?
                    .into(),
                root: target["src_path"]
                    .as_str()
                    .ok_or("Cargo target source missing")?
                    .into(),
            });
        }
    }
    Ok(targets)
}

/// Every test a compiled libtest executable lists, ignored or not.
fn compiled_tests(suite: &Suite) -> Result<BTreeSet<String>> {
    let output = crate::run(Command::new(&suite.executable).args(["--list", "--format", "terse"]))?;
    Ok(std::str::from_utf8(&output.stdout)?
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .map(str::to_owned)
        .collect())
}

/// The platform-gated tests, checked against this platform's compiled lists.
fn platform_gated(root: &Path, suites: &[Suite]) -> Result<Value> {
    use crate::platform_gated::{Platform, scan, verify};
    let gated = scan(&test_targets(root)?)?;
    if let Some(current) = Platform::current() {
        let mut compiled: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
        for suite in suites {
            compiled
                .entry((suite.package.clone(), suite.target.clone()))
                .or_default()
                .extend(compiled_tests(suite)?);
        }
        verify(&gated, current, |package, target, test| {
            compiled
                .get(&(package.into(), target.into()))
                .is_some_and(|names| names.contains(test))
        })?;
    }
    Ok(serde_json::json!({"count": gated.len(), "tests": gated}))
}

/// The user this process runs as, owner of every fixture temporary directory.
#[cfg(target_os = "linux")]
fn own_uid() -> std::io::Result<u32> {
    Ok(std::fs::metadata("/proc/self")?.uid())
}
/// macOS has no `/proc`: the effective uid, the owner `/proc/self` reports on Linux.
#[cfg(target_os = "macos")]
fn own_uid() -> std::io::Result<u32> {
    // SAFETY: geteuid has no preconditions.
    Ok(unsafe { libc::geteuid() })
}

#[cfg(unix)]
fn temporary(root: &Path, selected: Option<&Path>) -> Result<tempfile::TempDir> {
    let base = selected
        .map(|path| root.join(path))
        .unwrap_or_else(|| root.join(".local/tmp"));
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&base)?;
    if base.canonicalize()? != base {
        return Err("fixture temporary directory must be a physical path, without symlinks or parent traversal".into());
    }
    let stat = std::fs::metadata(&base)?;
    if stat.uid() != own_uid()? || (selected.is_some() && stat.mode() & 0o077 != 0) {
        return Err(
            "--tmpdir must be an owner-private directory (mode 700) owned by this user".into(),
        );
    }
    Ok(tempfile::Builder::new()
        .prefix("ignored-")
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir_in(base)?)
}

/// Windows: the default `.local/tmp` inside the checkout only. Without access
/// control bindings in this crate the owner-private rule for `--tmpdir` cannot
/// be checked, so a selected directory is refused rather than admitted
/// unchecked. The directory must still be a physical path.
#[cfg(windows)]
fn temporary(root: &Path, selected: Option<&Path>) -> Result<tempfile::TempDir> {
    if selected.is_some() {
        return Err(
            "--tmpdir is not supported on Windows; the fixture directory is .local/tmp".into(),
        );
    }
    let base = root.join(".local").join("tmp");
    std::fs::create_dir_all(&base)?;
    let canonical = base.canonicalize()?;
    let canonical = canonical.to_string_lossy();
    let physical = canonical.strip_prefix(r"\\?\").unwrap_or(&canonical);
    if !physical.eq_ignore_ascii_case(&base.to_string_lossy()) {
        return Err("fixture temporary directory must be a physical path, without symlinks or parent traversal".into());
    }
    Ok(tempfile::Builder::new()
        .prefix("ignored-")
        .tempdir_in(base)?)
}

pub fn run(root: &Path, options: Options) -> Result<()> {
    let (suites, environment) = workspace(root)?;
    let mut entries = Vec::new();
    let mut names = BTreeSet::new();
    for suite in &suites {
        for entry in discover(suite)? {
            if !names.insert(entry.id()) {
                return Err(format!("duplicate inventory entry {}", entry.id()).into());
            }
            entries.push(entry);
        }
    }
    if entries.is_empty() {
        return Err("workspace ignored inventory is empty".into());
    }
    // Tests a cfg keeps off a platform compile nowhere there, so no binary
    // above lists them; the source scan names them for every platform.
    let gated = platform_gated(root, &suites)?;
    let report_path = root.join(&options.report);
    let logs = report_path.with_extension("logs");
    std::fs::create_dir_all(&logs)?;
    let temp = temporary(root, options.tmpdir.as_deref())?;
    let mut unavailable = prerequisites(&environment);
    // The longest custody fixture adds a random directory and a daemon control
    // socket. Admission refuses symlinks, so a proc-fd alias is not a substitute.
    if temp.path().as_os_str().len() + 48 >= 108 {
        let reason = "fixture path is too long for private Unix sockets; supply --tmpdir with a short physical owner-private directory";
        unavailable
            .entry(CUSTODY)
            .and_modify(|existing| {
                existing.push_str("; ");
                existing.push_str(reason);
            })
            .or_insert_with(|| reason.into());
    }
    let report = evaluate(
        entries,
        &options,
        |entry| {
            entry
                .prerequisites
                .iter()
                .filter_map(|key| unavailable.get(key).cloned())
                .collect()
        },
        |entry| {
            eprintln!("ignored: running {}", entry.id());
            execute_test(&entry.suite, &entry.name, temp.path(), &logs, &environment)
        },
    );
    let value = serde_json::json!({"format":"connectors.ignored-suites/1", "mode":if options.inventory { "inventory" } else { "execute" }, "scope":"workspace default features", "families":options.family, "only_prerequisite":options.only_prerequisite, "required":options.required, "test_binaries":suites.len(), "logs":logs, "result":report, "platform_gated":gated});
    crate::save_json(&report_path, &value)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    if !report.accepted(options.required) {
        return Err(
            "ignored suite execution refused or failed; see the named outcomes in the report"
                .into(),
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// The CLI's stand-in targets run their own `main` and answer a terse listing with a
    /// summary line, which the inventory refused; they are not inventoried, and a libtest
    /// target of the same package still is.
    #[test]
    fn a_harness_false_target_is_not_inventoried_and_a_libtest_target_is() {
        let manifest = include_str!("../../../apps/connectors/Cargo.toml");
        let custom: BTreeSet<_> = harness_false_tests(manifest)
            .unwrap()
            .into_iter()
            .map(|target| ("connectors".to_owned(), target))
            .collect();
        for target in [
            "connect_endpoint",
            "connect_endpoint_adversary",
            "setup_migrate_short",
            "short_form_adversary",
            "catalog_provider_failure",
        ] {
            assert!(!inventoried(&custom, "connectors", target), "{target}");
        }
        assert!(inventoried(&custom, "connectors", "local_mcp_server"));
        assert!(inventoried(
            &custom,
            "connectors-build",
            "catalog_provider_failure"
        ));
    }

    #[test]
    fn harness_false_tests_reads_only_entries_that_set_harness_false() {
        let manifest = r#"
            [package]
            name = "example"
            [[test]]
            name = "standin"
            harness = false
            [[test]]
            name = "libtest_explicit"
            harness = true
            [[test]]
            name = "libtest_default"
        "#;
        assert_eq!(harness_false_tests(manifest).unwrap(), vec!["standin"]);
        assert!(
            harness_false_tests("[package]\nname = \"none\"\n")
                .unwrap()
                .is_empty()
        );
        assert!(harness_false_tests("[[test]]\nharness = false\n").is_err());
    }

    fn options() -> Options {
        Options {
            family: vec![Family::Disposable],
            inventory: false,
            required: true,
            only_prerequisite: vec![],
            report: PathBuf::new(),
            tmpdir: None,
        }
    }
    /// Entries as `classify` holds them; the stubbed runner never starts their executable.
    fn classified(tests: &[(&str, &str, &str)]) -> Vec<Entry> {
        tests
            .iter()
            .map(|(package, target, name)| {
                let suite = Suite {
                    package: (*package).into(),
                    target: (*target).into(),
                    executable: PathBuf::new(),
                };
                classify(&suite, name)
            })
            .collect()
    }
    const PROVIDER_ONLY: (&str, &str, &str) = (
        "connectors",
        "adversary_d218_u1",
        "describe_of_a_definition_needs_no_connection",
    );
    const PROVIDER_AND_CUSTODY: (&str, &str, &str) = (
        "connectors",
        "failed_connect",
        "a_connect_to_an_unreachable_provider_reports_the_owners_cause_at_dispatch",
    );
    const CHROME_ONLY: (&str, &str, &str) = (
        "connectors-host",
        "connectors_host",
        "local::oauth_adversary_tests::chrome_following_the_redirect_is_accepted",
    );
    #[test]
    fn only_prerequisite_parses_and_defaults_to_no_narrowing() {
        let parse = |args: &[&str]| {
            let args = crate::Args::try_parse_from(args).unwrap();
            let crate::Action::Ignored(options) = args.command else {
                panic!("ignored action required");
            };
            options.only_prerequisite
        };
        assert_eq!(parse(&["connectors-build", "ignored"]), []);
        assert_eq!(
            parse(&[
                "connectors-build",
                "ignored",
                "--required",
                "--only-prerequisite",
                "provider",
            ]),
            [Supplied::Provider]
        );
        assert_eq!(
            parse(&[
                "connectors-build",
                "ignored",
                "--only-prerequisite",
                "provider,cli"
            ]),
            [Supplied::Provider, Supplied::Cli]
        );
    }
    #[test]
    fn only_prerequisite_provider_selects_the_tests_a_built_provider_suffices_for() {
        let mut opts = options();
        opts.only_prerequisite = vec![Supplied::Provider];
        let mut ran = vec![];
        let report = evaluate(
            classified(&[PROVIDER_ONLY, PROVIDER_AND_CUSTODY, CHROME_ONLY]),
            &opts,
            |_| vec![],
            |entry| {
                ran.push(entry.id());
                Ok(true)
            },
        );
        let id =
            |(package, target, name): (&str, &str, &str)| format!("{package}::{target}::{name}");
        assert_eq!(ran, [id(PROVIDER_ONLY)]);
        assert_eq!(
            (report.selected, report.executed, report.missing),
            (1, 1, 0)
        );
        let status = |test| {
            report
                .outcomes
                .iter()
                .find(|o| o.name == id(test))
                .map(|o| (o.selected, o.status.as_str()))
                .unwrap()
        };
        assert_eq!(status(PROVIDER_ONLY), (true, "passed"));
        assert_eq!(status(PROVIDER_AND_CUSTODY), (false, "not-selected"));
        assert_eq!(status(CHROME_ONLY), (false, "not-selected"));
        assert!(report.accepted(true));
    }
    #[test]
    fn only_prerequisite_with_an_empty_selection_refuses_when_required() {
        let mut opts = options();
        opts.only_prerequisite = vec![Supplied::Provider, Supplied::Cli];
        let report = evaluate(
            classified(&[PROVIDER_AND_CUSTODY, CHROME_ONLY]),
            &opts,
            |_| vec![],
            |_| Ok(true),
        );
        assert_eq!((report.selected, report.executed), (0, 0));
        assert!(!report.accepted(true));
        assert!(report.accepted(false));
    }
    /// A bounded real Rust libtest executable. No provider, shell script or ambient service.
    fn fixture(fail: bool, unknown: bool) -> (tempfile::TempDir, Suite) {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("fixture.rs");
        std::fs::write(&source, format!(r#"
mod local {{
    mod keyring {{ pub mod custody {{ mod tests {{
        #[test] #[ignore] fn disposable_secret_service_restart_and_failures() {{ assert!({}); }}
    }} }} }}
    mod clock {{ mod tests {{ #[test] #[ignore] fn independent_live_quorum() {{ panic!("live never authorized"); }} }} }}
    mod audit {{ mod tests {{ #[test] #[ignore] fn crash_child() {{ panic!("helper never top level"); }} }} }}
}}
{}
"#, !fail, if unknown { "#[test] #[ignore] fn unclassified() {}" } else { "" })).unwrap();
        let executable = root
            .path()
            .join(format!("fixture{}", std::env::consts::EXE_SUFFIX));
        let output = Command::new("rustc")
            .args(["--test", "--edition=2024"])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        (
            root,
            Suite {
                package: "connectors-host".into(),
                target: "connectors_host".into(),
                executable,
            },
        )
    }
    fn run_fixture(fail: bool, unknown: bool, missing: bool, opts: Options) -> Report {
        let (root, suite) = fixture(fail, unknown);
        evaluate(
            discover(&suite).unwrap(),
            &opts,
            |_| {
                if missing {
                    vec!["qualified disposable Secret Service unavailable".into()]
                } else {
                    vec![]
                }
            },
            |entry| execute_one(&entry.suite, &entry.name, root.path()),
        )
    }
    #[test]
    fn ignored_runner_has_an_explicit_cli_and_defaults_to_disposable() {
        let args = crate::Args::try_parse_from(["connectors-build", "ignored", "--inventory"])
            .expect("the ignored suite runner is an independent CLI command");
        let crate::Action::Ignored(options) = args.command else {
            panic!("ignored action required");
        };
        assert_eq!(options.family, [Family::Disposable]);
        assert!(
            crate::Args::try_parse_from([
                "connectors-build",
                "ignored",
                "--inventory",
                "--required"
            ])
            .is_err()
        );
    }
    #[test]
    fn ignored_inventory_accounted() {
        assert!(
            include_str!("../../connectors-custody/src/secret_service/gnome.rs")
                .contains(GNOME_SHA256),
            "custody qualification changed; review runner prerequisites"
        );
        let report = run_fixture(false, false, false, options());
        assert_eq!(
            (
                report.inventory,
                report.selected,
                report.executed,
                report.skipped
            ),
            (3, 1, 1, 2)
        );
        assert!(report.accepted(true));
        assert_eq!(report.outcomes.len(), 3);
    }
    // Mode bits and symlinks; Windows refuses --tmpdir outright.
    #[cfg(unix)]
    #[test]
    fn missing_prerequisite_is_explicit() {
        let root = tempfile::Builder::new()
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .unwrap();
        let owned = temporary(root.path(), Some(root.path())).unwrap();
        assert_eq!(
            std::fs::metadata(owned.path()).unwrap().mode() & 0o777,
            0o700
        );
        let alias = root.path().join("alias");
        std::os::unix::fs::symlink(root.path(), &alias).unwrap();
        assert!(temporary(root.path(), Some(&alias)).is_err());
        let public = root.path().join("public");
        std::fs::create_dir(&public).unwrap();
        std::fs::set_permissions(&public, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(temporary(root.path(), Some(&public)).is_err());
        let report = run_fixture(false, false, true, options());
        assert_eq!(
            (report.selected, report.executed, report.missing),
            (1, 0, 1)
        );
        assert!(!report.accepted(true));
        assert!(report.accepted(false));
        assert!(
            report
                .outcomes
                .iter()
                .any(|o| o.reasons.iter().any(|r| r.contains("Secret Service")))
        );
    }
    #[test]
    fn selected_test_failure_is_nonzero() {
        let report = run_fixture(true, false, false, options());
        assert_eq!((report.executed, report.failed), (1, 1));
        assert!(!report.accepted(false));
    }
    #[test]
    fn subprocess_helper_never_top_level() {
        let mut opts = options();
        opts.family = vec![
            Family::Disposable,
            Family::Live,
            Family::Timing,
            Family::Deferred,
        ];
        let report = run_fixture(false, false, false, opts);
        assert_eq!(report.executed, 2);
        assert!(
            report
                .outcomes
                .iter()
                .any(|o| o.name.ends_with("crash_child") && !o.selected && o.status == "helper")
        );
    }
    #[test]
    fn unknown_inventory_refuses_before_any_execution() {
        let report = run_fixture(false, true, false, options());
        assert_eq!(report.unknown, 1);
        assert_eq!(report.executed, 0);
        assert!(!report.accepted(false));
    }
    #[cfg(unix)]
    #[test]
    fn adversary_nonprivate_kubeconfig_is_missing_before_dispatch() {
        const CHILD: &str = "CONNECTORS_CLASSIFIER_MODE_PROBE";
        if let Ok(expected) = std::env::var(CHILD) {
            let missing = prerequisites(&BTreeMap::new());
            assert_eq!(
                missing.contains_key(KUBECONFIG),
                expected == "missing",
                "kubeconfig prerequisite must distinguish mode-0644 from mode-0600"
            );
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("kubeconfig");
        std::fs::write(&path, b"fixture configuration, no credential\n").unwrap();
        for (mode, expected) in [(0o644, "missing"), (0o600, "present")] {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
            let output = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "ignored::tests::adversary_nonprivate_kubeconfig_is_missing_before_dispatch",
                    "--nocapture",
                ])
                .env(CHILD, expected)
                .env(KUBECONFIG, &path)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "isolated prerequisite probe failed: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn inventory_mode_never_executes_selected_cases() {
        let mut opts = options();
        opts.inventory = true;
        let report = run_fixture(true, false, false, opts);
        assert_eq!(
            (report.inventory, report.selected, report.executed),
            (3, 1, 0)
        );
    }
    #[test]
    fn zero_test_success_is_not_execution() {
        let (root, suite) = fixture(false, false);
        assert!(
            execute_one(&suite, "nonexistent", root.path())
                .unwrap_err()
                .to_string()
                .contains("exactly one executed case")
        );
        let report = evaluate(
            discover(&suite).unwrap(),
            &options(),
            |_| vec![],
            |_| Err("not started".into()),
        );
        assert_eq!((report.selected, report.executed, report.failed), (1, 0, 1));
        assert!(!report.accepted(false));
    }
    // Process groups and signals.
    #[cfg(unix)]
    fn adversary_descendant_fixture(panics: bool) -> (tempfile::TempDir, Suite) {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("descendant.rs");
        let pid = root.path().join("descendant.pid");
        std::fs::write(
            &source,
            format!(
                r#"
#[test]
#[ignore]
fn selected_fixture() {{
    let child = std::process::Command::new("/bin/sleep")
        .arg("900")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn().unwrap();
    std::fs::write({pid:?}, child.id().to_string()).unwrap();
    assert!(!{panics}, "fixture fails after launching its child");
}}
"#,
            ),
        )
        .unwrap();
        let executable = root.path().join("descendant");
        let output = Command::new("rustc")
            .args(["--test", "--edition=2024"])
            .arg(source)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        (
            root,
            Suite {
                package: "connectors-host".into(),
                target: "connectors_host".into(),
                executable,
            },
        )
    }

    /// Linux: the runner is the subreaper and has reaped the descendant, so
    /// any non-zombie status is a live process.
    #[cfg(target_os = "linux")]
    fn descendant_running(pid: &str) -> bool {
        std::fs::read_to_string(format!("/proc/{pid}/status"))
            .is_ok_and(|status| !status.lines().any(|line| line.starts_with("State:\tZ")))
    }
    /// macOS: launchd adopts and reaps the killed descendant, so it exists
    /// briefly as a zombie; it is running if it still exists two seconds on.
    #[cfg(target_os = "macos")]
    fn descendant_running(pid: &str) -> bool {
        let pid: libc::pid_t = pid.trim().parse().unwrap();
        let until = Instant::now() + Duration::from_secs(2);
        // SAFETY: signal 0 only checks that the process exists; it sends nothing.
        while unsafe { libc::kill(pid, 0) } == 0 {
            if Instant::now() >= until {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    #[cfg(unix)]
    fn adversary_assert_descendants_stopped(panics: bool) {
        let (root, suite) = adversary_descendant_fixture(panics);
        let outcome = execute_one(&suite, "selected_fixture", root.path());
        let pid = std::fs::read_to_string(root.path().join("descendant.pid")).unwrap();
        let running = descendant_running(&pid);
        // Clean exactly the descendant this test created before any assertion can panic.
        if running {
            let killed = Command::new("/bin/kill")
                .args(["-KILL", "--", pid.trim()])
                .status()
                .unwrap();
            assert!(killed.success());
        }
        assert_eq!(outcome.unwrap(), !panics);
        assert!(
            !running,
            "runner returned while fixture descendant {pid} was still running; child planned to outlive the 600-second fixture deadline"
        );
    }

    #[cfg(unix)]
    #[test]
    fn adversary_passing_fixture_does_not_leave_running_descendants() {
        adversary_assert_descendants_stopped(false);
    }

    #[cfg(unix)]
    #[test]
    fn adversary_failing_fixture_does_not_leave_running_descendants() {
        adversary_assert_descendants_stopped(true);
    }

    #[cfg(unix)]
    #[test]
    fn adversary_error_drops_live_group_without_reaping_another_fixture() {
        let spawn = || {
            Command::new("/bin/sleep")
                .arg("900")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .process_group(0)
                .spawn()
                .unwrap()
        };
        let mut other_fixture = spawn();
        let child = spawn();
        let owned_pid = child.id() as libc::pid_t;
        let started = Instant::now();
        let result: Result<()> = (|| {
            let group = FixtureGroup {
                child,
                signalled: false,
                finished: false,
                leader_status: None,
            };
            assert!(!group.exited()?);
            Err("injected runner error after spawn".into())
        })();
        let elapsed = started.elapsed();
        let mut status = 0;
        // SAFETY: observe only the exact child created above; no process scan.
        let waited = unsafe { libc::waitpid(owned_pid, &mut status, libc::WNOHANG) };
        let error = std::io::Error::last_os_error();
        if waited == 0 {
            // A failing cleanup assertion must not leave this test's own child.
            // SAFETY: waitpid just proved it remains our unreaped live child.
            unsafe {
                libc::kill(owned_pid, libc::SIGKILL);
                libc::waitpid(owned_pid, &mut status, 0);
            }
        }
        let other_still_running = other_fixture.try_wait().unwrap().is_none();
        if other_still_running {
            other_fixture.kill().unwrap();
        }
        other_fixture.wait().unwrap();
        assert_eq!(
            result.unwrap_err().to_string(),
            "injected runner error after spawn"
        );
        assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
        assert_eq!(waited, -1, "owned live leader must already be reaped");
        assert_eq!(error.raw_os_error(), Some(libc::ECHILD));
        assert!(other_still_running, "another fixture group was affected");
    }

    /// The CLI journeys' lane read from the workflow that declares it: a job
    /// every pull request runs, whose red journey fails the pull request.
    mod journeys_lane {
        use super::*;
        use serde_yaml_ng::Value;

        const WORKFLOW: &str = include_str!("../../../.github/workflows/rust-gate.yml");
        const JOURNEY: (&str, &str, &str) = (
            "connectors-catalog-provider",
            "local_runtime",
            "cli_journey::lifecycle::catalog_cli_failed_repair_and_busy_stop_preserve_authority",
        );
        fn options_of(command: &str) -> Option<Options> {
            let (_, args) = command.trim().split_once("connectors-build -- ")?;
            let argv: Vec<&str> = std::iter::once("connectors-build")
                .chain(args.split_whitespace())
                .collect();
            match crate::Args::try_parse_from(argv).ok()?.command {
                crate::Action::Ignored(options) => Some(options),
                _ => None,
            }
        }

        #[test]
        fn a_pull_request_job_runs_the_cli_journeys_and_a_red_one_fails_it() {
            let workflow: Value = serde_yaml_ng::from_str(WORKFLOW).unwrap();
            let trigger = workflow.get("on").expect("the workflow names its triggers");
            let pull_request = trigger
                .get("pull_request")
                .expect("the workflow runs on every pull request");
            assert!(
                pull_request.is_null(),
                "a branch or path filter would let a pull request skip the journeys"
            );
            let jobs = workflow.get("jobs").and_then(Value::as_mapping).unwrap();
            let mut lanes = vec![];
            for (name, job) in jobs {
                let steps = job.get("steps").and_then(Value::as_sequence).unwrap();
                for step in steps {
                    let run = step.get("run").and_then(Value::as_str).unwrap_or("");
                    let Some(options) = options_of(run) else {
                        continue;
                    };
                    if options.only_prerequisite.contains(&Supplied::Cli) {
                        lanes.push((name.as_str().unwrap().to_owned(), job, step, options));
                    }
                }
            }
            assert_eq!(
                lanes.len(),
                1,
                "exactly one job runs the CLI journeys: {:?}",
                lanes.iter().map(|lane| &lane.0).collect::<Vec<_>>()
            );
            let (name, job, step, options) = lanes.pop().unwrap();
            assert!(
                options.required,
                "{name}: a missing prerequisite must refuse, not skip"
            );
            assert_eq!(options.family, [Family::Disposable], "{name}");
            for needed in [Supplied::Provider, Supplied::Cli, Supplied::Custody] {
                assert!(options.only_prerequisite.contains(&needed), "{name}");
            }
            for holder in [job, step] {
                assert!(holder.get("continue-on-error").is_none(), "{name}");
                assert!(
                    holder.get("if").is_none(),
                    "{name}: a conditional journey step can pass a pull request unrun"
                );
            }
            let report = evaluate(classified(&[JOURNEY]), &options, |_| vec![], |_| Ok(false));
            assert_eq!(
                (report.selected, report.failed),
                (1, 1),
                "the lane selects the lifecycle journey"
            );
            assert!(
                !report.accepted(options.required),
                "a red journey must fail the lane"
            );
        }
    }

    /// The provider-only lane read from the documents that declare it: the
    /// workflow step, the development guide and the classification.
    mod adversary {
        use super::*;

        const WORKFLOW: &str = include_str!("../../../.github/workflows/rust-gate.yml");
        const GUIDE: &str = include_str!("../../../docs/development.md");
        /// The four tests `docs/development.md` names for the CI step.
        const FOUR: [(&str, &str, &str); 4] = [
            (
                "connectors",
                "definition_connect_question",
                "the_connect_question_names_the_host_the_token_is_sent_to",
            ),
            (
                "connectors",
                "definition_connect_question",
                "the_connect_question_of_an_oauth_profile_names_its_token_host_too",
            ),
            (
                "connectors",
                "definition_connect_question",
                "an_entry_another_run_recorded_while_the_question_waits_is_not_connected",
            ),
            (
                "connectors",
                "adversary_d218_u1",
                "describe_of_a_definition_needs_no_connection",
            ),
        ];
        /// Neighbours in the same packages that the lane must leave alone.
        const OTHERS: [(&str, &str, &str); 4] = [
            PROVIDER_AND_CUSTODY,
            CHROME_ONLY,
            (
                "connectors",
                "failed_connect",
                "an_adapter_identity_mismatch_during_an_upgrade_names_a_new_connection",
            ),
            (
                "connectors-host",
                "connectors_host",
                "local::clock::tests::independent_live_quorum",
            ),
        ];
        fn id((package, target, name): (&str, &str, &str)) -> String {
            format!("{package}::{target}::{name}")
        }
        /// The arguments of the workflow's `ignored` step, parsed as the binary parses them.
        fn ci_options() -> Options {
            let line = WORKFLOW
                .lines()
                .find(|line| line.contains("connectors-build -- ignored"))
                .expect("rust-gate.yml runs connectors-build ignored");
            let (_, args) = line.split_once("connectors-build -- ").unwrap();
            let argv: Vec<&str> = std::iter::once("connectors-build")
                .chain(args.split_whitespace())
                .collect();
            let crate::Action::Ignored(options) =
                crate::Args::try_parse_from(argv).unwrap().command
            else {
                panic!("ignored action required");
            };
            options
        }
        fn entry(family: Option<Family>, prerequisites: Vec<&'static str>) -> Entry {
            let mut entry = classified(&[("connectors", "probe", "a_reviewed_probe")])
                .pop()
                .unwrap();
            entry.family = family;
            entry.prerequisites = prerequisites;
            entry
        }

        #[test]
        fn the_ci_step_runs_the_four_documented_tests_and_nothing_else() {
            let options = ci_options();
            assert!(options.required, "the CI step must refuse an empty lane");
            assert_eq!(options.family, [Family::Disposable]);
            assert_eq!(options.only_prerequisite, [Supplied::Provider]);
            let step = WORKFLOW
                .lines()
                .find(|line| line.contains("connectors-build -- ignored"))
                .unwrap();
            let (_, command) = step.split_once("run: ").unwrap();
            assert!(
                GUIDE.lines().any(|line| line == command),
                "docs/development.md shows another command than the workflow runs: {command}"
            );
            let mut entries = classified(&OTHERS);
            entries.extend(classified(&FOUR));
            let mut ran = vec![];
            let report = evaluate(
                entries,
                &options,
                |_| vec![],
                |entry| {
                    ran.push(entry.id());
                    Ok(true)
                },
            );
            assert_eq!(ran, FOUR.map(id));
            assert_eq!(
                (
                    report.selected,
                    report.executed,
                    report.missing,
                    report.unknown
                ),
                (4, 4, 0, 0)
            );
            let prose = GUIDE.split_whitespace().collect::<Vec<_>>().join(" ");
            let in_file = FOUR
                .iter()
                .filter(|test| test.1 == "definition_connect_question")
                .count();
            assert_eq!(in_file, 3);
            assert!(prose.contains(
                "the three tests of `apps/connectors/tests/definition_connect_question.rs`"
            ));
            assert!(prose.contains(
                "`describe_of_a_definition_needs_no_connection` in `apps/connectors/tests/adversary_d218_u1.rs`"
            ));
            for test in FOUR {
                let outcome = report.outcomes.iter().find(|o| o.name == id(test)).unwrap();
                assert_eq!(
                    (outcome.selected, outcome.status.as_str()),
                    (true, "passed")
                );
            }
            assert!(report.accepted(options.required));
        }

        #[test]
        fn an_unreviewed_test_refuses_the_ci_step_before_any_execution() {
            let options = ci_options();
            let mut entries = classified(&FOUR);
            entries.push(entry(None, vec![]));
            let report = evaluate(
                entries,
                &options,
                |_| vec![],
                |entry| panic!("{} executed beside unknown inventory", entry.id()),
            );
            assert_eq!(
                (
                    report.unknown,
                    report.selected,
                    report.executed,
                    report.skipped
                ),
                (1, 4, 0, 5)
            );
            assert!(
                report
                    .outcomes
                    .iter()
                    .filter(|o| o.selected)
                    .all(|o| o.status == "blocked")
            );
            assert!(!report.accepted(options.required));
        }

        #[test]
        fn a_provider_the_build_did_not_supply_refuses_the_ci_step_instead_of_skipping() {
            let options = ci_options();
            let report = evaluate(
                classified(&FOUR),
                &options,
                |entry| {
                    entry
                        .prerequisites
                        .iter()
                        .filter(|key| **key == PROVIDER)
                        .map(|key| format!("{key} must name an executable file"))
                        .collect()
                },
                |entry| panic!("{} executed without its provider", entry.id()),
            );
            assert_eq!(
                (report.selected, report.missing, report.executed),
                (4, 4, 0)
            );
            assert!(
                report
                    .outcomes
                    .iter()
                    .all(|o| o.status == "missing-prerequisite")
            );
            assert!(!report.accepted(options.required));
        }

        #[test]
        fn a_disposable_test_with_no_prerequisite_qualifies_as_the_guide_says() {
            let prose = GUIDE.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(prose.contains("A test with no prerequisite also qualifies."));
            let report = evaluate(
                vec![entry(Some(Family::Disposable), vec![])],
                &ci_options(),
                |_| vec![],
                |_| Ok(true),
            );
            assert_eq!((report.selected, report.executed), (1, 1));
        }

        #[test]
        fn narrowing_never_widens_the_family() {
            let report = evaluate(
                vec![
                    entry(Some(Family::Live), vec![]),
                    entry(Some(Family::Timing), vec![]),
                ],
                &ci_options(),
                |_| vec![],
                |entry| panic!("{} executed outside its family", entry.id()),
            );
            assert_eq!(report.selected, 0);
            assert!(report.outcomes.iter().all(|o| o.status == "not-selected"
                && o.reasons == ["family requires explicit selection"]));
        }

        #[test]
        fn cli_names_the_cli_prerequisite_and_provider_does_not() {
            let select = |supplied: Vec<Supplied>, prerequisites: Vec<&'static str>| {
                let mut options = options();
                options.only_prerequisite = supplied;
                evaluate(
                    vec![entry(Some(Family::Disposable), prerequisites)],
                    &options,
                    |_| vec![],
                    |_| Ok(true),
                )
                .selected
            };
            assert_eq!(select(vec![Supplied::Cli], vec![CLI]), 1);
            assert_eq!(select(vec![Supplied::Provider], vec![CLI]), 0);
            assert_eq!(select(vec![Supplied::Cli], vec![PROVIDER]), 0);
            assert_eq!(select(vec![Supplied::Provider], vec![CLI, PROVIDER]), 0);
            assert_eq!(
                select(vec![Supplied::Provider, Supplied::Cli], vec![CLI, PROVIDER]),
                1
            );
            assert_eq!(select(vec![Supplied::Cli], vec![CLI, CUSTODY]), 0);
        }
    }
}

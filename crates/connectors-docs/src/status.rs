//! The status data, `website/data/status.json` (`b10x-status/1`), and the status page,
//! `website/docs/status.md`, both from [`CAPABILITIES`].
//!
//! A shipped capability names the test that holds it as `path::function`, which generation
//! checks the file defines, so a claim cannot outlive its evidence. A capability that is not
//! shipped names none. The page is plain Markdown rather than MDX because the unified
//! documentation site, which still lists this repository, accepts no executable MDX.

use std::{collections::BTreeSet, fs, path::Path};

use serde_json::json;

use crate::{Result, cell, front_matter};

/// Where the status data lands, relative to the repository root.
pub const DATA: &str = "website/data/status.json";
/// Where the status page lands, relative to the repository root.
pub const PAGE: &str = "website/docs/status.md";

/// The date the list below was last checked against the repository.
const AS_OF: &str = "2026-10-16";

/// One capability on the status page.
pub struct Capability {
    /// The group the status table lists it under.
    pub area: &'static str,
    /// What it is, in a few words.
    pub label: &'static str,
    /// One sentence a reader can check.
    pub detail: &'static str,
    /// `shipped`, `decided` or `planned`.
    pub status: &'static str,
    /// For a shipped capability, the test that holds it: `path::function`.
    pub evidence: Option<&'static str>,
    /// The documentation route that explains it, `/docs/...`.
    pub href: &'static str,
}

const fn shipped(
    area: &'static str,
    label: &'static str,
    detail: &'static str,
    evidence: &'static str,
    href: &'static str,
) -> Capability {
    Capability {
        area,
        label,
        detail,
        status: "shipped",
        evidence: Some(evidence),
        href,
    }
}

const fn decided(
    area: &'static str,
    label: &'static str,
    detail: &'static str,
    href: &'static str,
) -> Capability {
    Capability {
        area,
        label,
        detail,
        status: "decided",
        evidence: None,
        href,
    }
}

const fn planned(
    area: &'static str,
    label: &'static str,
    detail: &'static str,
    href: &'static str,
) -> Capability {
    Capability {
        area,
        label,
        detail,
        status: "planned",
        evidence: None,
        href,
    }
}

/// Every capability the site claims, in the order the status table lists them.
pub const CAPABILITIES: &[Capability] = &[
    shipped(
        "Service boundary",
        "Describe and invoke",
        "A connector's standalone service answers a typed descriptor and invokes its operations over HTTP; admission and freshness are checked before dispatch.",
        "crates/connectors-host/tests/service.rs::wire_admission_and_freshness_refuse_before_dispatch",
        "/docs/getting-started",
    ),
    shipped(
        "Service boundary",
        "One-hop federation",
        "connectors serve puts one gateway in front of several standalone services, prefixes their operations and reports a downstream failure without retrying it.",
        "crates/connectors-host/tests/service.rs::federation_preserves_result_and_reports_downstream_failure",
        "/docs/guides/federate-adapter-services",
    ),
    shipped(
        "Service boundary",
        "Audited v1alpha2 invoke",
        "A host whose service configuration names a state directory serves POST /v1alpha2/invoke: every admitted invocation is anchored in the execution audit before dispatch (since 0.35.0).",
        "crates/connectors-host/tests/http_audit_anchor.rs::audited_write_answers_complete_with_a_ref_that_reads_back_its_anchor",
        "/docs/concepts/governed-invocation",
    ),
    shipped(
        "Service boundary",
        "The attempt a write produced",
        "An admitted write records its attempt before its one dispatch and answers it as mutation; a lost answer is outcome_unknown and is never sent again (since 0.35.0).",
        "crates/connectors-host/tests/service.rs::a_lost_answer_is_outcome_unknown_with_the_recorded_attempt_and_no_second_dispatch",
        "/docs/concepts/governed-invocation",
    ),
    shipped(
        "Service boundary",
        "Rust client for v1alpha2",
        "connectors_client::Client::invoke_v1alpha2 returns the attempt the host recorded and reports an older host as unsupported instead of falling back.",
        "crates/connectors-conformance/tests/client_v1alpha2.rs::the_returned_attempt_id_is_the_one_the_host_recorded",
        "/docs/concepts/governed-invocation",
    ),
    decided(
        "Service boundary",
        "Governed service admission",
        "Principal, tenant, grant and delegated approval on the service wire are specified; the runtime implements only the first v1alpha2 invoke binding.",
        "/docs/reference/contracts/governed-service",
    ),
    shipped(
        "Local runtime",
        "Setup and metadata store",
        "setup init creates a private configuration and an Entity Runtime metadata store over Eventlog SQLite, with durable open checkpoints (since 0.33.0).",
        "crates/connectors-host/src/local/metadata/checkpoint_tests.rs::a_store_setup_init_creates_has_durable_open_checkpoints",
        "/docs/guides/set-up-the-local-cli",
    ),
    shipped(
        "Local runtime",
        "Supervised instances",
        "A local owner starts the process that serves each configured instance, refuses a changed executable or configuration before provider work, and stops exact incarnations.",
        "adapters/catalog/tests/local_runtime.rs::catalog_child_validates_reads_and_stops_with_exact_incarnations",
        "/docs/concepts/local-runtime",
    ),
    shipped(
        "Local runtime",
        "Stopping an owner after an upgrade",
        "setup owner-stop asks the state directory's running owner, of any build, to drain and exit within 30 seconds, answers no_owner when none runs, and never signals a process.",
        "apps/connectors/tests/local_cli.rs::setup_owner_stop_stops_an_owner_another_build_started_and_a_fresh_owner_serves",
        "/docs/guides/upgrade-the-cli#after-an-upgrade",
    ),
    shipped(
        "Local runtime",
        "Configuration upgrades",
        "After a configuration change each saved connection has one working path: revalidate, a new connection under the same instance id after an evidence lifetime change, or a new instance id after a changed provider authority or profile.",
        "crates/connectors-host/src/local/registry/upgrade_tests.rs::an_identity_change_names_a_new_instance_id_on_every_command",
        "/docs/concepts/local-runtime",
    ),
    shipped(
        "Local runtime",
        "Approval clock refusals",
        "approvals clock-check names why the quorum formed no answer (below quorum, split or too imprecise) with its next action, and each source's result: unreachable, unverified or verified at any radius; only the combined answer is capped.",
        "apps/connectors/tests/local_cli.rs::clock_check_below_quorum_refusal",
        "/docs/concepts/local-runtime",
    ),
    shipped(
        "Local runtime",
        "Saved connections",
        "connections connect validates a credential and keeps it in Secret Service custody; revalidate, reconnect and revoke manage it, and an unanswered revalidation is outcome_unknown.",
        "apps/connectors/tests/failed_connect.rs::a_revalidation_reply_that_never_arrives_is_outcome_unknown",
        "/docs/concepts/local-runtime",
    ),
    shipped(
        "Local runtime",
        "Unattended reads",
        "A connection connected or revalidated once keeps serving reads across any number of evidence lifetimes: a read on lapsed evidence recollects it once from custody, with no re-entry, before it is served.",
        "apps/connectors/tests/connection_lease.rs::a_revalidated_connection_serves_reads_across_three_evidence_lifetimes",
        "/docs/concepts/local-runtime",
    ),
    shipped(
        "Local runtime",
        "Connector instances",
        "A connectors-local/5 configuration enables connectors by name and names their instances; an instance that pins nothing is derived from the catalog provider installed beside the CLI.",
        "apps/connectors/tests/derived_instance.rs::the_cli_derives_an_instance_named_alone_from_the_provider_beside_it",
        "/docs/guides/set-up-the-local-cli#configure-instances",
    ),
    shipped(
        "Local runtime",
        "Listing connectors",
        "connectors list prints every connector the CLI ships a declaration for, with its auth profiles and configured instances, without a configuration file and without starting a process.",
        "apps/connectors/tests/list_connectors.rs::a_configured_instance_is_named_on_its_connectors_row_and_nothing_is_started",
        "/docs/reference/cli",
    ),
    shipped(
        "Local runtime",
        "Selecting an instance",
        "--connector and --instance select one instance; the deprecated --adapter reaches the same instance and says it is deprecated once.",
        "apps/connectors/tests/connector_instance_selectors.rs::both_selectors_reach_one_instance_and_only_the_alias_is_deprecated",
        "/docs/reference/cli",
    ),
    shipped(
        "Local runtime",
        "Renamed commands",
        "instances replaces the adapters group and connections reconnect replaces connections repair; each old name runs as its new one and says it is deprecated once.",
        "apps/connectors/tests/renamed_commands.rs::each_old_name_answers_as_its_new_name_and_says_it_is_deprecated_once",
        "/docs/reference/cli",
    ),
    shipped(
        "Local runtime",
        "Stale connections",
        "A connection whose stored credential the provider refused is Stale until connections reconnect publishes a new credential under the same reference.",
        "crates/connectors-host/src/local/registry/stale_tests.rs::a_refused_read_makes_the_connection_stale_until_a_reconnect_publishes",
        "/docs/concepts/local-runtime#saved-connections",
    ),
    shipped(
        "Local runtime",
        "One connection per instance",
        "connections connect is refused while the instance holds a published Live or Stale connection, naming the reconnect, revalidation or revocation that helps.",
        "crates/connectors-host/src/local/registry/cardinality_tests.rs::a_connect_is_refused_while_the_instance_holds_a_published_live_connection",
        "/docs/concepts/local-runtime#saved-connections",
    ),
    shipped(
        "Local runtime",
        "Configuration migration",
        "setup migrate moves each legacy catalog entry to an instance of its connector, keeping its instance id, revision, executable selection and custody items, and names why any other entry is kept.",
        "apps/connectors/tests/setup_migrate.rs::each_kind_is_migrated_or_kept_naming_why_and_keeps_its_records",
        "/docs/guides/set-up-the-local-cli#configure-instances",
    ),
    shipped(
        "Local runtime",
        "Mounted-secret custody",
        "An owner configuration can explicitly select read-only mounted-secret custody (mounted_secret, Linux): connect, list and a credentialed invoke run under an empty environment in a container, reading a Kubernetes-style Secret volume and writing nothing; rotation needs a revalidation.",
        "apps/connectors/tests/container_env_i.rs::env_i_mounted_custody_round_trip",
        "/docs/guides/set-up-the-local-cli",
    ),
    shipped(
        "Local runtime",
        "Approved writes",
        "approvals policy-set, prepare and issue bind a write's whole input to a protected proof before the host dispatches it once.",
        "crates/connectors-host/src/local/approvals/tests.rs::issuer_audience_time_key_and_clock_refusals_are_current",
        "/docs/concepts/local-runtime",
    ),
    shipped(
        "Local runtime",
        "Approval-signing keys",
        "Approval-signing keys are initialized, rotated, recovered, revoked and retired through the CLI; status reads never touch custody.",
        "crates/connectors-host/src/local/approval_keys/tests.rs::passive_status_does_not_migrate_or_touch_custody",
        "/docs/concepts/local-runtime",
    ),
    shipped(
        "Local runtime",
        "Approval clock check",
        "approvals clock-check answers from a quorum of two to eight configured Roughtime sources, naming each source's result, without starting the owner or reading metadata.",
        "apps/connectors/tests/local_cli.rs::clock_check_uses_current_configured_key_without_metadata_or_service_start",
        "/docs/concepts/local-runtime",
    ),
    shipped(
        "Local runtime",
        "Consumer launch",
        "connections launch hands one connection's protected document to a pinned consumer and refuses a launch its configuration does not admit (since 0.31.0).",
        "apps/connectors/tests/consumer_launch.rs::a_launch_the_configuration_refuses_is_refused_by_name_before_anything_starts",
        "/docs/concepts/local-runtime",
    ),
    shipped(
        "Local runtime",
        "Operations by family",
        "operations list --family names the operations of one contract family, such as datasource.feed/v1alpha1, with their contract and profile.",
        "apps/connectors/tests/feed_discovery.rs::operations_list_by_family_answers_the_family_operations_with_contract_and_profile",
        "/docs/reference/cli",
    ),
    shipped(
        "Local runtime",
        "JSON values in JSON answers",
        "With --output json, invoke results and operation schemas are JSON values, not JSON text (breaking since 0.32.0).",
        "apps/connectors/tests/json_answers.rs::operations_describe_answers_schemas_as_json_objects",
        "/docs/reference/cli",
    ),
    shipped(
        "Catalog provider",
        "Bundles from pinned sources",
        "connectors-build catalog compiles a pinned OpenAPI document into a digest-verified bundle; every committed bundle matches a fresh run byte for byte.",
        "adapters/catalog/tests/bundle_drift.rs::every_committed_bundle_matches_a_fresh_pipeline_run",
        "/docs/guides/build-a-catalog-bundle",
    ),
    shipped(
        "Catalog provider",
        "Guarded writes as data",
        "A selection's guard reads before the one request and refuses unless every check holds; after it, a failed check leaves the outcome uncertain, never refused.",
        "adapters/catalog/tests/engine.rs::guarded_write_refuses_before_dispatch_and_classifies_after",
        "/docs/concepts/catalog-provider",
    ),
    shipped(
        "Catalog provider",
        "GitLab",
        "Projects, issues, files, the repository tree, branches, tags, releases, commits, commit diffs, compares, code search, deployments, environments, pipelines, jobs, traces, merge requests and their diffs and discussions as reads; issue, merge-request (auto-merge and reopen included), note, discussion, tag and release writes under approval.",
        "adapters/catalog/tests/shipped.rs::shipped_gitlab_selections_resolve_and_cover_the_former_native_surface",
        "/docs/reference/adapters/gitlab",
    ),
    shipped(
        "Catalog provider",
        "GitLab CI, branch, commit and project writes",
        "pipeline.retry, pipeline.cancel, pipeline.create, commit.create, file.update, branch.create, branch.delete and project.create under approval, each guarded on the caller's commit (project.create on the namespace path and id) and proven by GitLab's answer or a read after it; environments.list as a read; verified against fixtures, not a live GitLab.",
        "adapters/catalog/tests/gitlab_ci_project_writes.rs::the_ci_and_project_operations_are_shipped_as_reviewed",
        "/docs/reference/adapters/gitlab",
    ),
    shipped(
        "Catalog provider",
        "GitLab merge-request feed",
        "GitLab binds datasource.feed/v1alpha1 as profile gitlab-merge-requests/1, checked against recorded GitLab answers (since 0.32.0).",
        "adapters/catalog/tests/gitlab_feed.rs::the_shipped_gitlab_selection_declares_the_feed_family_under_its_profile",
        "/docs/reference/adapters/catalog/contracts/feed",
    ),
    shipped(
        "Catalog provider",
        "Feed time query in an unknown zone",
        "A declared feed's time query can be read by the provider in a zone it is not told, between utc_offsets least and greatest minutes east of UTC, and a resume then starts early enough to skip no item in any such zone. The Jira issue feed that declares it, profile jira-issues/1, is in the shipped Jira selection, so a Jira instance has feed.containers and feed.items; verified in every profile zone from UTC-12:00 to UTC+14:00 against fixtures from the upstream description, not a live Jira (*unreleased*).",
        "adapters/catalog/tests/jira_feed.rs::every_profile_zone_from_utc_minus_12_to_utc_plus_14_resumes_without_a_skip",
        "/docs/reference/contracts/catalog",
    ),
    shipped(
        "Catalog provider",
        "Confluence page feed",
        "A Confluence REST v1 binding of datasource.feed/v1alpha1, profile confluence-pages/1: spaces as containers, their pages as items, resumed by a CQL lastmodified query and continued by the cursor of the next link. It is in the shipped confluence-v1 selection, so a confluence-v1 instance has feed.containers and feed.items, and only a connection bound to REST v1 reads it. Verified in every profile zone from UTC-12:00 to UTC+14:00 against fixtures written from the v1 reference, not a live Confluence; the CQL lastmodified precision and order are not established (*unreleased*).",
        "adapters/catalog/tests/confluence_v1_feed.rs::every_zone_from_utc_minus_12_to_utc_plus_14_resumes_without_a_skip",
        "/docs/reference/contracts/catalog",
    ),
    shipped(
        "Catalog provider",
        "Jira Cloud",
        "Issue search by JQL, one issue by key, comments, changelogs, an issue's available transitions, a project's creatable issue types and one issue type's create fields, the visible projects and one project by id or key, user search, an attachment's content, running one transition by id under a guard that reads the issue before and after it, and creating, commenting on, editing, linking and deleting issues under approval, each guarded by a read before it (edit and delete on the caller's updated stamp, refused as conflict), over HTTP basic authentication (one issue, issue types and user search since 0.39.0; the visible projects and one project *unreleased*); the writes are verified against fixtures, not a live site.",
        "adapters/catalog/tests/jira.rs::shipped_jira_selections_are_the_eleven_reads_and_the_six_writes",
        "/docs/reference/adapters/catalog",
    ),
    shipped(
        "Catalog provider",
        "Binary responses",
        "A selection can read an answer as bytes: media type, length, SHA-256 and the bytes in base64, at most 2 MiB and refused as capacity above it, with at most three redirects, each only to an origin the selection names and the connection admits in its hosts; verified against fixtures only.",
        "adapters/catalog/tests/binary_responses.rs::a_binary_selection_answers_media_type_length_digest_and_the_bytes",
        "/docs/concepts/catalog-provider",
    ),
    shipped(
        "Catalog provider",
        "Jira attachment content",
        "attachment.content reads one attachment as a binary body, directly with redirect=false or through Jira's 303 to an admitted media host; that host, api.media.atlassian.com, is inferred and not verified against a live site.",
        "adapters/catalog/tests/jira_attachment.rs::the_303_is_followed_to_the_admitted_media_host_only",
        "/docs/reference/adapters/catalog",
    ),
    shipped(
        "Catalog provider",
        "Confluence Cloud",
        "Changed pages, a space's pages, one page with its body and a page's comments; exercised against local fixtures only.",
        "adapters/catalog/tests/confluence.rs::shipped_confluence_selections_are_exactly_the_four_reads",
        "/docs/reference/adapters/catalog",
    ),
    shipped(
        "Catalog provider",
        "Confluence Cloud REST v1",
        "CQL search, changed pages by CQL, spaces, one space, a space's content, one page with its body and a page's comments under the provider id confluence-v1, from a source document this repository writes from Atlassian's v1 reference; exercised against local fixtures only.",
        "adapters/catalog/tests/confluence_v1.rs::shipped_confluence_v1_selections_are_exactly_the_seven_reads",
        "/docs/reference/adapters/catalog",
    ),
    shipped(
        "Catalog provider",
        "HubSpot CRM",
        "One object type's records, paged, and one record by id; exercised against local fixtures only.",
        "adapters/catalog/tests/hubspot.rs::shipped_hubspot_selections_are_exactly_the_two_reads",
        "/docs/reference/adapters/catalog",
    ),
    shipped(
        "Catalog provider",
        "Zendesk Support",
        "Tickets, users and organizations changed since a start time, one of each by id, and a ticket's comments, with OAuth client credentials or an API token; a ticket update, a private note and a draft reply behind an approval, refused as a conflict when the ticket changed since it was read.",
        "adapters/catalog/tests/zendesk.rs::shipped_zendesk_selections_are_the_seven_reads_and_three_writes",
        "/docs/reference/adapters/catalog",
    ),
    shipped(
        "Catalog provider",
        "Google Drive, Slides, Calendar and Gmail",
        "Reads and guarded writes from Discovery documents projected to OpenAPI, with a refresh token from browser consent; exercised against local fixtures only.",
        "adapters/catalog/tests/google_drive.rs::shipped_drive_selections_are_the_six_reads_and_three_writes",
        "/docs/reference/adapters/catalog",
    ),
    shipped(
        "Catalog provider",
        "Runpod pods",
        "pod.create, pods.list and pod.terminate with the API key in custody; a create without a definite answer is unknown and never sent again, and one refused for lack of capacity is refused as capacity; verified against a local fixture only (since 0.36.0).",
        "adapters/catalog/tests/runpod.rs::shipped_runpod_selections_are_exactly_create_list_terminate_and_pod_billing",
        "/docs/reference/adapters/catalog",
    ),
    shipped(
        "Catalog provider",
        "Swagger 2.0 sources",
        "A Swagger 2.0 document is ingested through an exact, recorded projection to OpenAPI 3.1.0; a construct without an exact row is refused by name (since 0.37.0).",
        "crates/connectors-catalog/tests/swagger.rs::every_construct_without_an_exact_row_is_refused_by_name",
        "/docs/guides/build-a-catalog-bundle",
    ),
    shipped(
        "Catalog provider",
        "Slack conversations",
        "conversations.list, conversations.history and conversations.replies with a bot token (since 0.38.0), users.list, one user by id (users.info) or e-mail address (users.lookupByEmail), team.info, emoji.list and auth.test, and on a second connection with a user token search.messages and the three conversation reads and users.list, which reach every conversation that user can read, the channels the bot is not in included; verified against a local fixture only.",
        "adapters/catalog/tests/slack.rs::shipped_slack_selections_are_exactly_the_bot_reads_and_writes_and_the_user_reads",
        "/docs/reference/adapters/catalog",
    ),
    shipped(
        "Catalog provider",
        "Slack messages",
        "chat.postMessage posts a message, a thread reply included, conversations.open opens a direct message to post to, and chat.update and chat.delete edit and delete a message the bot posted, each under approval after the token's auth.test read; an edit or delete Slack refuses by name (cant_update_message, cant_delete_message, message_not_found) is refused, and a dispatch without a definite answer, or any other 200 with ok false, is unknown; none is sent again; verified against a local fixture only.",
        "adapters/catalog/tests/slack.rs::an_approved_post_sends_one_json_body_and_returns_channel_and_ts",
        "/docs/reference/adapters/catalog",
    ),
    shipped(
        "Catalog provider",
        "Slack files",
        "files.list, files.info and file.download, which reads a file's url_private on files.slack.com as a binary body with the bot token, only where the connection admits that host with the credential, and files.delete under approval after the token's auth.test read and a files.info read of the file, with file_not_found and file_deleted refused as not_found and cant_delete_file as forbidden; verified against a local fixture only.",
        "adapters/catalog/tests/slack.rs::the_download_reads_url_private_with_the_token_on_an_admitted_host",
        "/docs/reference/adapters/catalog",
    ),
    shipped(
        "Catalog provider",
        "Slack read refusals",
        "A Slack read answered 200 with ok false is refused, never returned as an empty read: Slack's error is the reason, and missing_scope, not_in_channel and user_not_visible are forbidden, invalid_auth and its kin unauthorized, an unknown channel, thread, file or user not_found, an e-mail address no user has (users_not_found) not_found, ratelimited rate_limited with its Retry-After, a bad cursor or argument invalid_input, and any other name internal, on every JSON read of both Slack sets; verified against a local fixture only.",
        "adapters/catalog/tests/slack.rs::each_slack_error_answered_ok_false_is_a_refusal_naming_it",
        "/docs/reference/adapters/catalog",
    ),
    shipped(
        "Catalog provider",
        "Opsgenie alerts and incidents",
        "alerts.list and incidents.list over a creation window the caller advances, alert.get, incident.get and incident.alert_ids, read only, with the key sent as GenieKey <key>; verified against a local fixture only.",
        "adapters/catalog/tests/opsgenie.rs::shipped_opsgenie_selections_are_exactly_the_five_reads",
        "/docs/reference/adapters/catalog",
    ),
    decided(
        "Catalog provider",
        "Catalog index service",
        "Listing and locating precompiled bundles with provenance is specified; directly configured adapters need no index.",
        "/docs/reference/contracts/catalog",
    ),
    shipped(
        "Native adapters",
        "Kubernetes reads and discovery",
        "resources.list, endpoints.discover and optionally hosts.discover, scoped to configured namespaces and kinds, with a saved bearer token.",
        "adapters/kubernetes/tests/local_runtime.rs::private_kubernetes_validates_through_selfsubjectreview_and_reads_scoped_resources",
        "/docs/reference/adapters/kubernetes",
    ),
    shipped(
        "Native adapters",
        "Kubernetes single objects, namespaces and rollout history",
        "resources.get reads one object by name, namespaces.list the configured namespaces the cluster has, deployments.history the ReplicaSets a Deployment controls; replicasets and events are configurable kinds; verified against API-shaped fixtures.",
        "adapters/kubernetes/tests/parity_reads.rs::the_new_kinds_are_configurable_and_the_new_reads_are_advertised",
        "/docs/reference/adapters/kubernetes",
    ),
    shipped(
        "Native adapters",
        "Kubernetes statefulsets, nodes and custom resources",
        "statefulsets is a configurable kind; nodes.list pages the full Node objects when node_reads is configured; custom_resources.list pages one configured namespaced custom resource collection by group, version and plural; verified against API-shaped fixtures.",
        "adapters/kubernetes/tests/parity_reads.rs::custom_resources_list_reads_a_configured_collection_page_by_page",
        "/docs/reference/adapters/kubernetes",
    ),
    shipped(
        "Native adapters",
        "Kubernetes exec credential plugin",
        "A kubeconfig user that authenticates through an exec plugin, such as aws eks get-token, is reached when the local configuration pins the plugin by absolute path and sha256: it runs on Linux from the descriptor it was hashed through, with the kubeconfig arguments, an empty environment plus an allow-list, a deadline and an output bound, and its token is held in memory until expiry; verified with fixture plugins against a recorded API session, not a live EKS cluster.",
        "adapters/kubernetes/tests/local_runtime.rs::exec_plugin_credential_authenticates_a_recorded_api_session",
        "/docs/reference/adapters/kubernetes",
    ),
    shipped(
        "Native adapters",
        "Kubernetes pod logs",
        "pods.logs reads a bounded tail of one container's log in a configured namespace, at most 1,000 lines and 128 KiB, when pod_logs is configured; verified against API-shaped fixtures.",
        "adapters/kubernetes/tests/pod_logs.rs::pod_logs_are_bounded_and_container_selected",
        "/docs/reference/adapters/kubernetes",
    ),
    shipped(
        "Native adapters",
        "Kubernetes kubeconfig contexts",
        "contexts.list lists the contexts of the one kubeconfig a local configuration names, never its credentials or servers, with no cluster request.",
        "adapters/kubernetes/tests/kubeconfig_contexts.rs::contexts_are_listed_without_credentials",
        "/docs/reference/adapters/kubernetes",
    ),
    shipped(
        "Native adapters",
        "Kubernetes pod exec",
        "pods.exec runs one command as an argument vector in a named container, through the local CLI under approval, when pod_exec is configured; exit status and bounded output come back, and a lost stream is unknown and never resent; verified against recorded streams, not a live cluster.",
        "adapters/kubernetes/tests/local_exec.rs::local_pod_exec_runs_through_the_approved_write_exchange",
        "/docs/reference/adapters/kubernetes",
    ),
    shipped(
        "Native adapters",
        "Helm release reads",
        "History, status, values and manifest of a named release, values and manifests disclosed only as redacted projections.",
        "adapters/kubernetes/tests/local_runtime.rs::helm_release_history_status_values_and_manifest_come_from_named_release_secrets",
        "/docs/reference/adapters/kubernetes",
    ),
    shipped(
        "Native adapters",
        "PostgreSQL reads",
        "schema.list, query.read, database.list, table.list, table.describe and index.list in a read-only transaction with fixed statement and lock timeouts; a deadline or a dropped invocation cancels the query.",
        "adapters/sql/tests/protocol.rs::dropping_the_invocation_still_cancels_the_database",
        "/docs/reference/adapters/sql",
    ),
    shipped(
        "Native adapters",
        "MySQL reads",
        "The same six reads on a connection whose engine is mysql, in a read-only session bound to the configured database, over TLS unless plaintext is allowed.",
        "adapters/sql/tests/mysql_local.rs::mysql_connects_revalidates_and_reads_through_the_private_runtime",
        "/docs/reference/adapters/sql",
    ),
    shipped(
        "Native adapters",
        "SQL read profiles",
        "profile.read runs a read the operator declares in the native file by name, on either engine: declared parameters only, bound; typed columns with numbers masked by default; one call identifier; keyset pages that never take a short page for the end and refuse, rather than skip, a non-unique ordering value at a page boundary; cursors checked for shape, type and context; verified against scripted wire fixtures, not a live database.",
        "adapters/sql/tests/mysql_local.rs::read_profiles_are_served_through_the_private_runtime_as_a_declared_read",
        "/docs/reference/adapters/sql",
    ),
    shipped(
        "Native adapters",
        "Tavily web search",
        "websearch.search, websearch.fetch and websearch.crawl through the shared websearch family; the key is checked without spending a search.",
        "adapters/tavily/tests/protocol.rs::the_key_is_validated_by_usage_without_a_search",
        "/docs/reference/adapters/tavily",
    ),
    shipped(
        "Native adapters",
        "Loki LogQL reads",
        "logs.query_range, logs.query_metric and logs.labels through a saved bearer connection (since 0.38.0).",
        "adapters/loki/tests/local_runtime.rs::each_read_answers_through_the_private_runtime_from_the_recorded_answers",
        "/docs/reference/adapters/loki",
    ),
    shipped(
        "Native adapters",
        "Loki line and trace profiles",
        "logs.profile_lines (one typed, masked record per line of a configured line profile) and logs.call_trace (one call's lines from every source, by its identifier) through a saved bearer connection.",
        "adapters/loki/tests/local_runtime.rs::configured_profiles_answer_through_the_private_runtime",
        "/docs/reference/adapters/loki",
    ),
    shipped(
        "Native adapters",
        "A call by its number, every leg typed",
        "A configured SQL read profile over the call table and a configured Loki line profile over the CDR lines, joined by a workflow on the call identifier each declares, answer every leg of a transferred call with its type, numbers masked, across a page boundary of the SQL read.",
        "crates/connectors-conformance/tests/incident_call_join.rs::every_leg_of_a_transferred_call_is_typed_by_joining_the_call_table_and_the_cdr_lines",
        "/docs/reference/adapters/sql",
    ),
    shipped(
        "Native adapters",
        "Grafana data sources",
        "datasources.list through a saved service-account connection, and Loki reads through Grafana's data-source proxy (since 0.39.0).",
        "adapters/grafana/tests/local_runtime.rs::the_datasource_list_answers_through_the_private_runtime_from_the_recorded_answer",
        "/docs/reference/adapters/grafana",
    ),
    shipped(
        "Native adapters",
        "Grafana IRM alert groups and incidents",
        "irm.alert_groups.list, irm.alert_group.get, irm.incidents.list and irm.incident.get over a since window, with incident-to-alert-group links (inferred), through a saved IRM service-account connection; read only.",
        "adapters/grafana/tests/local_runtime.rs::a_saved_irm_connection_reads_alert_groups_and_incidents_through_the_private_runtime",
        "/docs/reference/adapters/grafana",
    ),
    shipped(
        "Native adapters",
        "Prometheus PromQL reads",
        "series.query, series.query_range and rules.list through a saved bearer connection, directly or through Grafana's data-source proxy.",
        "adapters/prometheus/tests/local_runtime.rs::each_read_answers_through_the_private_runtime_from_the_recorded_answers",
        "/docs/reference/adapters/prometheus",
    ),
    planned(
        "Native adapters",
        "Kubernetes permission checks",
        "Per-operation SelfSubjectAccessReview checks and the remaining Kubernetes and Helm workflows.",
        "/docs/reference/adapters/kubernetes",
    ),
    planned(
        "Native adapters",
        "Loki tenants and other authentication",
        "An X-Scope-OrgID tenant header and profiles other than a bearer token; today a Loki without authentication cannot be connected.",
        "/docs/reference/adapters/loki",
    ),
    planned(
        "Native adapters",
        "Prometheus tenants, timeouts and other authentication",
        "An X-Scope-OrgID tenant header, an adapter-set query timeout, a Capacity error for too many samples apart from other execution errors, and profiles other than a bearer token.",
        "/docs/reference/adapters/prometheus",
    ),
    decided(
        "Native adapters",
        "Specification-only adapters",
        "Alertmanager has a design or typed model and no runtime; Grafana dashboards, datasource discovery and the mediated route are specified and not built.",
        "/docs/reference/adapters",
    ),
    planned(
        "Native adapters",
        "MCP",
        "MCP invocation, projection, auth lifecycle, mutation replay and composition contracts are specified; there is no MCP runtime.",
        "/docs/reference/adapters",
    ),
    shipped(
        "Distribution",
        "Release binaries",
        "A release page carries an archive per Linux and macOS target (x86_64 and aarch64) holding connectors and connectors-catalog-provider, with SHA256SUMS; the other adapters are built from the checkout, and there is no Windows binary.",
        "crates/connectors-build/src/release_binaries.rs::the_release_workflow_packages_the_cli_and_the_catalog_provider",
        "/docs/getting-started",
    ),
    planned(
        "Distribution",
        "Package distribution",
        "No package manager, container registry or Windows binary carries a release.",
        "/docs/getting-started",
    ),
];

/// Whether `source` defines the test `name`.
fn defines(source: &str, name: &str) -> bool {
    source.contains(&format!("fn {name}("))
}

fn checked(root: &Path, capabilities: &[Capability]) -> Result<()> {
    let mut labels = BTreeSet::new();
    for capability in capabilities {
        if !labels.insert(capability.label) {
            return Err(format!("{}: listed twice", capability.label));
        }
        if !matches!(capability.status, "shipped" | "decided" | "planned") {
            return Err(format!(
                "{}: status {} is not shipped, decided or planned",
                capability.label, capability.status
            ));
        }
        if !capability.href.starts_with("/docs/") {
            return Err(format!("{}: href is not a /docs/ route", capability.label));
        }
        match (capability.status, capability.evidence) {
            ("shipped", Some(evidence)) => {
                let (file, name) = evidence.split_once("::").ok_or_else(|| {
                    format!("{}: {evidence} is not `path::function`", capability.label)
                })?;
                let source = fs::read_to_string(root.join(file))
                    .map_err(|error| format!("{}: reading {file}: {error}", capability.label))?;
                if !defines(&source, name) {
                    return Err(format!("{}: {file} has no {name}", capability.label));
                }
            }
            ("shipped", None) => {
                return Err(format!("{}: shipped without evidence", capability.label));
            }
            (_, Some(_)) => {
                return Err(format!(
                    "{}: only a shipped item names evidence",
                    capability.label
                ));
            }
            (_, None) => {}
        }
    }
    Ok(())
}

fn data_for(root: &Path, capabilities: &[Capability]) -> Result<String> {
    checked(root, capabilities)?;
    let items: Vec<_> = capabilities
        .iter()
        .map(|capability| {
            json!({
                "area": capability.area,
                "label": capability.label,
                "detail": capability.detail,
                "status": capability.status,
                "href": capability.href,
            })
        })
        .collect();
    let document = json!({
        "format": "b10x-status/1",
        "asOf": AS_OF,
        "source": "connectors-docs, from its capability list; every shipped item names the test that holds it",
        "items": items,
    });
    serde_json::to_string_pretty(&document)
        .map(|text| text + "\n")
        .map_err(|error| format!("serializing the status data: {error}"))
}

/// The `b10x-status/1` document for the repository at `root`.
pub fn data(root: &Path) -> Result<String> {
    data_for(root, CAPABILITIES)
}

/// `/docs/guides/x#y` as a link from `website/docs/status.md`: `./guides/x.md#y`. A route that is a
/// directory's page, such as `/docs/reference/adapters/gitlab`, is that directory's `index.md`.
fn link(root: &Path, href: &str) -> String {
    let route = href.trim_start_matches("/docs/");
    let (page, anchor) = route.split_once('#').unwrap_or((route, ""));
    let anchor = if anchor.is_empty() {
        String::new()
    } else {
        format!("#{anchor}")
    };
    if root.join(crate::DOCS).join(page).is_dir() {
        format!("./{page}/index.md{anchor}")
    } else {
        format!("./{page}.md{anchor}")
    }
}

/// A status as the table shows it: a glyph and the word.
fn shown(status: &str) -> &'static str {
    match status {
        "shipped" => "● shipped",
        "decided" => "◐ decided",
        _ => "○ planned",
    }
}

fn page_for(root: &Path, capabilities: &[Capability]) -> Result<String> {
    checked(root, capabilities)?;
    let count = |status: &str| {
        capabilities
            .iter()
            .filter(|capability| capability.status == status)
            .count()
    };
    let mut out = front_matter(
        "Status",
        "Status",
        "What Connectors runs today, what is specified and what is planned, capability by capability.",
        3,
    );
    out.push_str(&format!(
        "# Status\n\nAs of {AS_OF}: {} capabilities are shipped, {} are decided and {} are planned. **Shipped** means it runs on `main` and is held by a named test in the repository; an item marked *unreleased* is on `main` but not yet in a tagged release. **Decided** means specified, with no runtime. **Planned** means not built. `connectors-docs` generates this page and `website/data/status.json` from one list and fails when a test it names is gone. A release page carries the source archives and Linux and macOS binaries of `connectors` and `connectors-catalog-provider`; the [changelog](https://github.com/beyond10x/connectors/blob/main/CHANGELOG.md) records each change a user sees, release by release.\n",
        count("shipped"),
        count("decided"),
        count("planned"),
    ));
    let mut area = "";
    for capability in capabilities {
        if capability.area != area {
            area = capability.area;
            out.push_str(&format!(
                "\n## {area}\n\n| Capability | Status | What it means |\n|---|---|---|\n"
            ));
        }
        out.push_str(&format!(
            "| [{}]({}) | {} | {} |\n",
            cell(capability.label),
            link(root, capability.href),
            shown(capability.status),
            cell(capability.detail),
        ));
    }
    Ok(out)
}

/// The status page for the repository at `root`.
pub fn page(root: &Path) -> Result<String> {
    page_for(root, CAPABILITIES)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability(status: &'static str, evidence: Option<&'static str>) -> Capability {
        Capability {
            area: "A",
            label: "L",
            detail: "D",
            status,
            evidence,
            href: "/docs/status",
        }
    }

    #[test]
    fn every_shipped_capability_rests_on_evidence_that_exists() {
        let root = crate::repository_root();
        let data = data(&root).unwrap();
        assert!(data.contains("\"format\": \"b10x-status/1\""));
        let page = page(&root).unwrap();
        assert!(page.contains("| [Describe and invoke](./getting-started.md) | ● shipped |"));
    }

    #[test]
    fn a_vanished_test_or_a_shipped_item_without_one_is_refused() {
        let root = crate::repository_root();
        let gone = capability(
            "shipped",
            Some("crates/connectors-host/tests/service.rs::no_such_test"),
        );
        let error = data_for(&root, &[gone]).unwrap_err();
        assert!(error.contains("has no no_such_test"), "{error}");
        assert!(data_for(&root, &[capability("shipped", None)]).is_err());
        let claimed = capability(
            "planned",
            Some(
                "crates/connectors-host/tests/service.rs::wire_admission_and_freshness_refuse_before_dispatch",
            ),
        );
        assert!(data_for(&root, &[claimed]).is_err());
        assert!(data_for(&root, &[capability("released", None)]).is_err());
        assert!(data_for(&root, &[capability("planned", None)]).is_ok());
    }

    #[test]
    fn routes_become_relative_markdown_links() {
        let root = crate::repository_root();
        assert_eq!(
            link(&root, "/docs/concepts/local-runtime"),
            "./concepts/local-runtime.md"
        );
        assert_eq!(
            link(&root, "/docs/reference/cli#connectors-setup-init"),
            "./reference/cli.md#connectors-setup-init"
        );
        assert_eq!(
            link(&root, "/docs/reference/adapters/gitlab"),
            "./reference/adapters/gitlab/index.md"
        );
    }
}

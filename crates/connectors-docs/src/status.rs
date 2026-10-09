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
const AS_OF: &str = "2026-10-09";

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
        "An adapter service answers a typed descriptor and invokes its operations over HTTP; admission and freshness are checked before dispatch.",
        "crates/connectors-host/tests/service.rs::wire_admission_and_freshness_refuse_before_dispatch",
        "/docs/getting-started",
    ),
    shipped(
        "Service boundary",
        "One-hop federation",
        "connectors serve puts one gateway in front of several adapter services, prefixes their operations and reports a downstream failure without retrying it.",
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
        "Supervised adapters",
        "A local owner starts the configured adapter executables, refuses a changed executable or configuration before provider work, and stops exact incarnations.",
        "adapters/catalog/tests/local_runtime.rs::catalog_child_validates_reads_and_stops_with_exact_incarnations",
        "/docs/concepts/local-runtime",
    ),
    shipped(
        "Local runtime",
        "Saved connections",
        "connections connect validates a credential and keeps it in Secret Service custody; revalidate, repair and revoke manage it, and an unanswered revalidation is outcome_unknown.",
        "apps/connectors/tests/failed_connect.rs::a_revalidation_reply_that_never_arrives_is_outcome_unknown",
        "/docs/concepts/local-runtime",
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
        "approvals clock-check verifies one configured Roughtime source without starting the owner or reading metadata.",
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
        "Projects, issues, files, branches, tags, releases, commits, compares, deployments, pipelines, jobs, traces and merge requests as reads; issue and merge-request writes under approval.",
        "adapters/catalog/tests/shipped.rs::shipped_gitlab_selections_resolve_and_cover_the_former_native_surface",
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
        "Jira Cloud",
        "Issue search by JQL, one issue by key, comments, changelogs, a project's creatable issue types and user search over HTTP basic authentication (one issue, issue types and user search unreleased).",
        "adapters/catalog/tests/jira.rs::shipped_jira_selections_are_exactly_the_six_reads",
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
        "HubSpot CRM",
        "One object type's records, paged, and one record by id; exercised against local fixtures only.",
        "adapters/catalog/tests/hubspot.rs::shipped_hubspot_selections_are_exactly_the_two_reads",
        "/docs/reference/adapters/catalog",
    ),
    shipped(
        "Catalog provider",
        "Zendesk Support",
        "Tickets, users and organizations changed since a start time, one of each by id, and a ticket's comments, with OAuth client credentials or an API token.",
        "adapters/catalog/tests/zendesk.rs::shipped_zendesk_selections_are_exactly_the_seven_reads",
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
        "pod.create, pods.list and pod.terminate with the API key in custody; a create without a definite answer is unknown and never sent again; verified against a local fixture only (since 0.36.0).",
        "adapters/catalog/tests/runpod.rs::shipped_runpod_selections_are_exactly_create_list_and_terminate",
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
        "conversations.list, conversations.history and conversations.replies with a bot token (since 0.38.0); verified against a local fixture only.",
        "adapters/catalog/tests/slack.rs::shipped_slack_selections_are_exactly_the_conversation_and_user_reads",
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
        "Helm release reads",
        "History, status, values and manifest of a named release, values and manifests disclosed only as redacted projections.",
        "adapters/kubernetes/tests/local_runtime.rs::helm_release_history_status_values_and_manifest_come_from_named_release_secrets",
        "/docs/reference/adapters/kubernetes",
    ),
    shipped(
        "Native adapters",
        "PostgreSQL reads",
        "schema.list and query.read in a read-only transaction with fixed statement and lock timeouts; a deadline or a dropped invocation cancels the query.",
        "adapters/sql/tests/protocol.rs::dropping_the_invocation_still_cancels_the_database",
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
        "Grafana data sources",
        "datasources.list through a saved service-account connection, and Loki reads through Grafana's data-source proxy (unreleased).",
        "adapters/grafana/tests/local_runtime.rs::the_datasource_list_answers_through_the_private_runtime_from_the_recorded_answer",
        "/docs/reference/adapters/grafana",
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
    decided(
        "Native adapters",
        "Specification-only adapters",
        "Atlassian native documents, Docker, Prometheus, Alertmanager, SIP and RTVBP have designs or typed models and no runtime; Grafana dashboards, datasource discovery and the mediated route are specified and not built.",
        "/docs/reference/adapters",
    ),
    planned(
        "Native adapters",
        "MCP",
        "MCP invocation, projection, auth lifecycle, mutation replay and composition contracts are specified; there is no MCP runtime.",
        "/docs/reference/adapters",
    ),
    planned(
        "Distribution",
        "Binary and package distribution",
        "Every release is a source release for Linux x86_64; build it from the checkout.",
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
        "# Status\n\nAs of {AS_OF}: {} capabilities are shipped, {} are decided and {} are planned. **Shipped** means it runs on `main` and is held by a named test in the repository; an item marked *unreleased* is on `main` but not yet in a tagged release. **Decided** means specified, with no runtime. **Planned** means not built. `connectors-docs` generates this page and `website/data/status.json` from one list and fails when a test it names is gone. Every release is a source release for Linux x86_64; the [changelog](https://github.com/beyond10x/connectors/blob/main/CHANGELOG.md) records each change a user sees, release by release.\n",
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

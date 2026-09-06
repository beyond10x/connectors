//! **Every catalogue invariant, over the whole catalogue.**
//!
//! One file, parameterised by the committed tree. This replaces a pattern the predecessor grew and
//! that does not carry over: one test file per provider — `babelforce_coverage.rs`,
//! `zendesk_spec_selection.rs`, sixteen of them — each asserting one provider's version of a rule
//! that is really a rule about *every* provider. The cost was structural rather than cosmetic:
//! adding a connector meant adding a file, so the fifty-sixth provider is covered only if somebody
//! remembers, and the rule itself is stated sixteen times and can be sixteen different rules.
//!
//! Every assertion below iterates the catalogue. A new provider is covered the moment it exists,
//! and a rule has exactly one statement.
//!
//! # The invariants
//!
//! 1. [`the_committed_tree_is_a_fixed_point_of_a_build`] and
//!    [`two_plans_over_the_same_inputs_are_byte_identical`] — determinism, both directions: a
//!    rebuild writes nothing, and two independent plans agree byte for byte.
//! 2. [`every_canonical_document_validates_against_the_committed_schema`] — and against the
//!    *planned* schema, so a schema change and a document change cannot pass separately.
//! 3. [`the_pack_serves_the_committed_documents_byte_for_byte`] — every provider record is its
//!    committed document, and every operation record is a substring of it that reparses equal.
//! 4. [`the_lockfile_agrees_with_every_input_and_every_artifact`] — every hash recomputed from the
//!    bytes on disk, in both directions: no row without a provider, no provider without a row.
//! 5. [`ids_are_unique_in_every_namespace_they_share`] — operation ids globally, and the three
//!    inbound member kinds within one service.
//! 6. [`no_input_or_artifact_carries_a_credential_shaped_value`] — the declaration surface has no
//!    field a secret could live in, asserted over the bytes rather than over the types.
//! 7. [`spec_backed_coverage_holds_in_both_directions`] — for every provider that declares spec
//!    ingest: nothing published that no document declares, and nothing selected that is not
//!    published.
//! 8. [`a_full_build_leaves_no_orphaned_artifact`] — no committed file under an artifact root that
//!    the plan does not claim.
//! 9. [`the_document_carries_the_callers_contract`] — every operation stores the model-facing
//!    contract (S-001): a description, a lowered object `input_schema`, and a symbol on every
//!    declared parameter — with the two measured cases pinned by name (babelforce's dotted
//!    parameter, airtable's error-envelope description).
//! 10. [`the_contract_and_the_params_state_the_same_symbols`] — the two places the document states
//!     a symbol (each param's `symbol`, the contract's `input_schema` keys and `required` order)
//!     agree, so a lowering bug cannot ship a contract keyed by names the params do not carry.
//! 11. [`every_format_origin_field_lowers_to_the_origin_slot`] — a `format = "origin"` config
//!     field's bound variable lands on exactly `["origin"]` in every operation that carries it
//!     (S-001; predecessor C-538 open question 3), so a provider declaring it for a variable
//!     inside a larger authority cannot silently drop Origin→Host with nothing red.
//! 12. [`the_credential_requirement_agrees_with_the_auth_list`] — the stored token (S-001) is
//!     `declared` exactly when the effective `auth` list is non-empty; the empty side carries one
//!     of the two distinction tokens the old derivation could not tell apart.
//! 13. [`sip_catalog_surface_is_the_bounded_platform_dial_member`] — the closed runtime
//!     vocabulary can land first, but only the repository-owned platform Provider may publish
//!     the bounded native SIP member; a configured PBX remains a peer rather than its owner.
//! 14. [`the_browser_surface_is_read_only_and_carries_no_interaction_member`] — every `cdp_v1`
//!     member is a platform-owned read on a lease, and no interaction member exists anywhere in
//!     the catalogue: clicking and typing are mutations and wait on the approval round-trip.
//! 15. [`no_effect_backend_is_reachable_without_an_admission_proof`] — no Integration reads the
//!     invocation's `approval_evidence_ref` (S-047): admission is decided upstream by the sealed
//!     `GrantDecision` → `ApprovalRedemption` → `AdmittedOperation` chain, and the field reaches
//!     a backend only as a named audit-correlation use — of which there are none today.
//! 16. [`a_session_signal_reaches_a_backend_only_through_the_admission_seam`] — the operation
//!     protocol's request grammar is a pinned closed set, the hosted route names every variant,
//!     and the S-049 signal admission seam exists: a new signal-shaped route cannot fall through
//!     the hosted catch-all dispatch silently, which is exactly how `SessionSignal` escaped the
//!     enforced-authority epic.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use catalog_build::pipeline::{self, Plan};
use catalog_build::workspace::Workspace;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------------------------
// The subject
// ---------------------------------------------------------------------------------------------

/// The repository root, from this crate's manifest directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the crate manifest lives two directories below the repository root")
        .to_path_buf()
}

/// A whole-catalogue plan over the committed tree. Writes nothing.
///
/// Computed **once** per process and handed out as a clone: a plan re-ingests 21 MB of vendored
/// specs, and every invariant below wanting its own plan made that the suite's entire cost. The
/// tests only read the plan, so one computation serves all — except
/// [`two_plans_over_the_same_inputs_are_byte_identical`], whose whole point is two independent
/// computations and which calls [`pipeline::plan`] directly.
fn full_plan() -> (Workspace, Plan) {
    static FULL: std::sync::LazyLock<(Workspace, Plan)> = std::sync::LazyLock::new(|| {
        let workspace = Workspace::new(repo_root());
        let plan = pipeline::plan(&workspace, None).expect("the committed catalogue compiles");
        // A plan over an empty tree would satisfy almost everything below while asserting nothing.
        assert!(
            plan.providers.len() >= 55,
            "the committed catalogue is 55 providers; the plan covers {}",
            plan.providers.len()
        );
        (workspace, plan)
    });
    FULL.clone()
}

/// The planned bytes of one artifact, by repository-relative path.
fn planned<'a>(workspace: &Workspace, plan: &'a Plan, relative: &str) -> &'a str {
    let path = workspace.root().join(relative);
    plan.artifacts
        .iter()
        .find(|artifact| artifact.path == path)
        .map(|artifact| artifact.contents.as_str())
        .unwrap_or_else(|| panic!("no artifact is planned at `{relative}`"))
}

/// Every provider's canonical document, parsed, keyed by connector id.
fn documents(workspace: &Workspace, plan: &Plan) -> BTreeMap<String, Value> {
    plan.providers
        .iter()
        .map(|provider| {
            let text = planned(workspace, plan, &format!("catalog/{provider}.catalog.json"));
            let value: Value = serde_json::from_str(text)
                .unwrap_or_else(|error| panic!("`{provider}`'s document is not JSON: {error}"));
            (provider.clone(), value)
        })
        .collect()
}

/// Native SIP enters the effective catalog through one reviewed, served member. A second member
/// must make the same source/runtime case explicitly instead of inheriting permission from the
/// existence of the driver vocabulary.
#[test]
fn sip_catalog_surface_is_the_bounded_platform_dial_member() {
    let (workspace, plan) = full_plan();
    let sip = documents(&workspace, &plan)
        .into_iter()
        .flat_map(|(provider, document)| {
            document["operations"]
                .as_array()
                .expect("operations")
                .iter()
                .filter(|operation| operation["protocol_driver"] == "sip_v1")
                .map(|operation| (provider.clone(), operation.clone()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        sip.len(),
        1,
        "every SIP member needs an atomic source/runtime review"
    );
    let (provider, operation) = &sip[0];
    assert_eq!(provider, "b10x");
    assert_eq!(
        documents(&workspace, &plan)["asterisk"]["operations"]
            .as_array()
            .expect("Asterisk operations")
            .iter()
            .filter(|operation| operation["protocol_driver"] == "sip_v1")
            .count(),
        0,
        "Asterisk is only a configurable SIP peer, never the native capability owner"
    );
    assert_eq!(documents(&workspace, &plan)["b10x"]["authority"], "io.b10x");
    assert_eq!(operation["id"], "sip-dial");
    assert_eq!(operation["interaction_shape"], "session_establishment");
    assert_eq!(operation["risk"], "high");
    assert_eq!(operation["idempotency"], "non_idempotent");
    assert_eq!(operation["expose"], true);
    assert!(operation.get("endpoint").is_none());
    assert!(operation["request"].get("method").is_none());
    assert!(operation["request"].get("path").is_none());

    let provenance: toml::Value = toml::from_str(
        &std::fs::read_to_string(repo_root().join("specs/b10x.provenance.toml"))
            .expect("read platform source provenance"),
    )
    .expect("platform source provenance is TOML");
    assert_eq!(provenance["origin"].as_str(), Some("repository-authored"));
    let provider_bytes = std::fs::read(repo_root().join("providers/b10x.toml"))
        .expect("read platform Provider source");
    let measured_provider_sha256 = connector_spec::sha256_hex(&provider_bytes);
    assert_eq!(
        provenance["provider_sha256"].as_str(),
        Some(measured_provider_sha256.as_str()),
        "the authored platform Provider moved without its provenance pin"
    );
}

/// **The browser surface is five read-only observations, and interaction is not one of them.**
///
/// Clicking, typing and submitting act on someone else's system on the operator's behalf, so they
/// are mutations and wait on the approval round-trip being built separately. A `cdp_v1` operation
/// declaring `direction = "write"` — or an interaction id appearing at all — would put an unapproved
/// write on a surface whose whole claim is that it only reads, so it fails here rather than in
/// review.
#[test]
fn the_browser_surface_is_read_only_and_carries_no_interaction_member() {
    let (workspace, plan) = full_plan();
    let documents = documents(&workspace, &plan);
    let browser: Vec<(String, Value)> = documents
        .iter()
        .flat_map(|(provider, document)| {
            document["operations"]
                .as_array()
                .expect("operations")
                .iter()
                .filter(|operation| operation["protocol_driver"] == "cdp_v1")
                .map(|operation| (provider.clone(), operation.clone()))
                .collect::<Vec<_>>()
        })
        .collect();

    let ids: Vec<&str> = browser
        .iter()
        .map(|(_, operation)| operation["id"].as_str().expect("id"))
        .collect();
    assert_eq!(
        ids,
        [
            "browser-open",
            "browser-goto",
            "browser-snapshot",
            "browser-screenshot",
            "browser-close"
        ],
        "the browser surface changed shape; every member needs an atomic source/runtime review"
    );

    for (provider, operation) in &browser {
        let id = operation["id"].as_str().expect("id");
        assert_eq!(provider, "b10x", "{id} is not platform-owned");
        assert_eq!(
            operation["direction"], "read",
            "{id} is not read-only, so it needs the approval round-trip first"
        );
        assert_eq!(
            operation["interaction_shape"], "leased_session",
            "{id} does not hold a lease, but a browser spans calls"
        );
        assert_eq!(operation["placement_requirement"], "connectors_deployment");
        assert_eq!(operation["implementation_form"], "built_in");
        assert_eq!(
            operation["required_capabilities"],
            json!(["public_network", "process"]),
            "{id} must declare the process and network authority the whole lease needs"
        );
        assert_eq!(operation["auth"], json!([]), "{id} holds no credential");
        assert!(operation.get("endpoint").is_none());
        assert!(operation["request"].get("method").is_none());
        assert!(operation["request"].get("path").is_none());
    }

    // Whole-catalogue, not just this provider: an interaction member arriving anywhere behind this
    // driver is the failure this test exists for.
    for document in documents.values() {
        for operation in document["operations"].as_array().expect("operations") {
            let id = operation["id"].as_str().expect("id");
            assert!(
                ![
                    "browser-act",
                    "browser-click",
                    "browser-type",
                    "browser-submit"
                ]
                .contains(&id),
                "{id} is interaction and must not join a read-only surface"
            );
        }
    }
}

/// Slack is the first provider whose bearer credentials have materially different capability
/// domains. Pin the curated boundary in the canonical document: a source refresh may update schemas
/// but may not turn the app-level Socket Mode token into bot/admin authority or import Admin writes.
#[test]
fn slack_surface_is_curated_and_credential_scopes_never_cross_purposes() {
    let (workspace, plan) = full_plan();
    let mut all = documents(&workspace, &plan);
    let slack = all.remove("slack").expect("Slack ships");
    let operations = slack["operations"].as_array().expect("operations");
    assert_eq!(operations.len(), 12, "Slack's callable surface is exact");
    // **Callable is not projected.** `slack-users-list` is catalogued and exposed to nothing: a
    // workspace directory is a bulk read of named people, and the per-person question a model asks
    // is already `slack-users-info`. Every other operation here is curated for a model.
    let projected: Vec<&str> = operations
        .iter()
        .filter(|operation| operation["expose"] == true)
        .map(|operation| operation["id"].as_str().expect("id"))
        .collect();
    assert_eq!(projected.len(), 11);
    assert!(!projected.contains(&"slack-users-list"));

    // Direct messages and group DMs are outside this connector entirely, and the enforcement is
    // that no input can reach them: `conversations.list` takes a `types` parameter which is not
    // declared, so Slack's documented `public_channel` default is the only value it can send.
    // Asserted over the request templates rather than over prose, because a reviewed `enum` would
    // not survive into a published input schema.
    for operation in operations {
        for parameter in operation["request"]["query"]
            .as_array()
            .unwrap_or(&Vec::new())
        {
            assert_ne!(
                parameter["name"].as_str(),
                Some("types"),
                "{} must not admit a conversation-type selector",
                operation["id"]
            );
        }
    }

    for operation in operations {
        let service = operation["service"].as_str().expect("service");
        let requirements = operation["auth_requirements"]
            .as_array()
            .expect("every selected Slack operation has scoped auth");

        if service == "admin" {
            assert_eq!(requirements.len(), 1);
            let requirement = &requirements[0];
            let credential = requirement["credentials"][0]
                .as_str()
                .expect("one credential");
            let scopes = requirement["scopes"][credential]
                .as_array()
                .expect("credential-local scope alternatives");
            assert_eq!(operation["direction"], "read");
            assert_eq!(operation["risk"], "low");
            assert_eq!(credential, "slack.admin_token");
            assert!(scopes
                .iter()
                .all(
                    |scope_set| scope_set
                        .as_array()
                        .is_some_and(|scope_set| scope_set.iter().all(|scope| scope
                            .as_str()
                            .is_some_and(|scope| scope.starts_with("admin."))))
                ));
            assert_ne!(credential, "slack.app_token");
            assert_ne!(credential, "slack.user_token");
            continue;
        }

        assert_eq!(service, "default");
        assert_eq!(requirements.len(), 2);
        let expected_scope = match operation["id"].as_str().expect("operation id") {
            "slack-chat-post-message" => "chat:write",
            "slack-conversations-history" | "slack-conversations-replies" => "channels:history",
            "slack-conversations-list" | "slack-conversations-info" => "channels:read",
            "slack-users-info" | "slack-users-list" => "users:read",
            "slack-reactions-add" => "reactions:write",
            other => panic!("unexpected curated Slack operation {other}"),
        };
        for (requirement, expected_credential) in requirements
            .iter()
            .zip(["slack.bot_token", "slack.user_token"])
        {
            assert_eq!(requirement["credentials"][0], expected_credential);
            assert_eq!(
                requirement["scopes"][expected_credential][0][0],
                expected_scope
            );
        }
    }

    assert_eq!(
        slack["events"]
            .as_array()
            .expect("events")
            .iter()
            .map(|event| event["name"].as_str().expect("event name"))
            .collect::<Vec<_>>(),
        ["app_mention", "message.channels"]
    );
    assert!(slack["events"]
        .as_array()
        .expect("events")
        .iter()
        .all(|event| event["auth_requirements"][0]["credentials"][0] == "slack.bot_token"));

    let socket = slack["channels"]
        .as_array()
        .expect("channels")
        .iter()
        .find(|channel| channel["name"] == "socket")
        .expect("Socket Mode channel");
    assert_eq!(
        socket["auth_requirements"][0]["credentials"][0],
        "slack.app_token"
    );
    assert_eq!(
        socket["auth_requirements"][0]["scopes"]["slack.app_token"][0][0],
        "connections:write"
    );
}

/// GitLab is the proving provider for the other half of delegated authority: a person-owned OAuth
/// or PAT Connection and three automation Connection kinds share one API surface, but never one
/// credential identity. Reads admit `read_api` or `api`; writes admit only `api`.
#[test]
fn gitlab_user_and_automation_connections_are_distinct_and_scope_gated() {
    let (workspace, plan) = full_plan();
    let mut all = documents(&workspace, &plan);
    let gitlab = all.remove("gitlab").expect("GitLab ships");

    let credentials = gitlab["auth"].as_array().expect("credentials");
    let subjects: BTreeMap<_, _> = credentials
        .iter()
        .map(|credential| {
            (
                credential["name"].as_str().expect("credential name"),
                credential["subject"].as_str().expect("credential subject"),
            )
        })
        .collect();
    assert_eq!(subjects["gitlab.oauth_token"], "user");
    assert_eq!(subjects["gitlab.token"], "user");
    for name in [
        "gitlab.service_account_token",
        "gitlab.group_access_token",
        "gitlab.project_access_token",
    ] {
        assert_eq!(subjects[name], "app");
    }

    for operation in gitlab["operations"].as_array().expect("operations") {
        let requirements = operation["auth_requirements"]
            .as_array()
            .expect("GitLab operation auth");
        assert_eq!(requirements.len(), 5);
        for requirement in requirements {
            let credential = requirement["credentials"][0]
                .as_str()
                .expect("one actor credential");
            let alternatives = requirement["scopes"][credential]
                .as_array()
                .expect("credential-local scopes");
            if operation["direction"] == "write" {
                assert_eq!(alternatives, &[serde_json::json!(["api"])]);
            } else {
                assert_eq!(
                    alternatives,
                    &[serde_json::json!(["api"]), serde_json::json!(["read_api"]),]
                );
            }
        }
    }

    let publication = [
        "gitlab-branch-create",
        "gitlab-repository-commit-create",
        "gitlab-merge-request-create",
        "gitlab-merge-request-update",
    ];
    for operation in gitlab["operations"].as_array().expect("operations") {
        let id = operation["id"].as_str().expect("operation id");
        if publication.contains(&id) {
            assert_eq!(operation["direction"], "write");
            assert_eq!(operation["risk"], "high");
            assert_eq!(operation["idempotency"], "non_idempotent");
            assert_eq!(operation["expose"], false);
        }
    }
}

/// S-015 is a vocabulary migration, not a behavioural edit. The digest began as the pre-migration
/// inventory of 151 non-empty operation trait sets, normalized without the old umbrella key, and
/// advances only when a new operation deliberately adds one of those promoted traits. It last
/// advanced to 176 when Slack's four added reads declared their cursor pagination and Slack's
/// `ok: false` error envelope, and Confluence's four rewritten reads moved to the surface a
/// service-account credential can actually reach.
#[test]
fn promoted_operation_traits_equal_the_pre_migration_inventory() {
    let (workspace, plan) = full_plan();
    let mut facts = Vec::new();
    for (provider, document) in documents(&workspace, &plan) {
        for operation in document["operations"].as_array().expect("operations") {
            if operation.get("pagination").is_none()
                && operation.get("rate_limit").is_none()
                && operation.get("error_envelope").is_none()
            {
                continue;
            }
            facts.push(serde_json::json!({
                "provider": provider.clone(),
                "id": operation["id"],
                "pagination": operation.get("pagination").cloned().unwrap_or(Value::Null),
                "rate_limit": operation.get("rate_limit").cloned().unwrap_or(Value::Null),
                "error_envelope": operation.get("error_envelope").cloned().unwrap_or(Value::Null),
            }));
        }
    }
    facts.sort_by_key(|fact| {
        format!(
            "{}\0{}",
            fact["provider"].as_str().unwrap(),
            fact["id"].as_str().unwrap()
        )
    });
    assert_eq!(facts.len(), 177);
    let schedule = facts
        .iter()
        .position(|fact| {
            fact["provider"] == "gitlab" && fact["id"] == "gitlab-pipeline-schedule-list"
        })
        .expect("the new schedule read declares page pagination");
    assert_eq!(
        facts.remove(schedule),
        serde_json::json!({
            "provider": "gitlab", "id": "gitlab-pipeline-schedule-list",
            "pagination": {"page": {"page_param": "page", "size_param": "per_page", "page_size": 20, "max_pages": 1}},
            "rate_limit": null, "error_envelope": null
        })
    );
    // The new declaration above is explicit; every historical trait remains byte-identical.
    assert_eq!(facts.len(), 176);
    let digest = connector_spec::sha256_hex(&serde_json::to_vec(&facts).unwrap());
    assert_eq!(
        digest,
        "69be55901f935d327ea8af6fea51c2f65fd6e3d95d047dd541db3e4cb96390f9"
    );
}

/// S-023 records an owner-local, reproducible mapping for substrate without reading a sibling
/// checkout or pretending the two repositories use the same vocabulary. This is deliberately the
/// five-axis fixture, not S-031's later full provider projection.
#[test]
fn substrate_axis_projection_is_pinned_total_and_non_mechanical() {
    let root = repo_root();
    let fixture: Value = serde_json::from_slice(
        &std::fs::read(root.join("fixtures/substrate-wire-0.1.0-axis-projection.json"))
            .expect("the connectors-owned substrate projection fixture exists"),
    )
    .expect("the substrate projection fixture is JSON");

    assert_eq!(fixture["source"]["repository"], "b10x/substrate");
    assert_eq!(fixture["source"]["bundle"], "substrate-wire");
    assert_eq!(fixture["source"]["version"], "0.1.0");
    assert_eq!(
        fixture["source"]["bundle_manifest_sha256"],
        "f71e1305367ec75c14f8f0db45a8bd750e4d0c6c0cdda1ef642b5f1ada5da9fa"
    );

    let schema = std::fs::read(root.join("catalog/connector-document.schema.json"))
        .expect("the committed connector schema exists");
    assert_eq!(fixture["target"]["schema_version"], 2);
    assert_eq!(
        fixture["target"]["schema_sha256"],
        connector_spec::sha256_hex(&schema)
    );

    let source = fixture["source_operation_facts"]
        .as_array()
        .expect("source operation facts are an array");
    let overlays = fixture["operation_overlays"]
        .as_array()
        .expect("operation overlays are an array");
    assert_eq!(
        source.len(),
        12,
        "substrate-wire 0.1.0 has twelve operations"
    );
    assert_eq!(overlays.len(), source.len());

    let source_ids: BTreeSet<_> = source
        .iter()
        .map(|operation| operation["id"].as_str().expect("source id"))
        .collect();
    let overlay_ids: BTreeSet<_> = overlays
        .iter()
        .map(|operation| operation["source_id"].as_str().expect("overlay source id"))
        .collect();
    assert_eq!(source_ids.len(), source.len(), "source ids must be unique");
    assert_eq!(
        source_ids, overlay_ids,
        "each source operation has exactly one overlay"
    );

    let effect_mapping = fixture["closed_mappings"]["source_effects"]
        .as_object()
        .expect("source effects use an explicit mapping");
    let target_host_effects = ["filesystem", "network", "process"];
    let mut renamed_effect = false;
    for operation in source {
        for effect in operation["effects"].as_array().expect("source effects") {
            let effect = effect.as_str().expect("source effect name");
            let mapping = effect_mapping
                .get(effect)
                .unwrap_or_else(|| panic!("source effect `{effect}` must be explicitly mapped"));
            assert_eq!(mapping["retention"], "substrate_capability_requirement");
            assert_eq!(mapping["semantic_effect"], false);
            let target = mapping["target_host_effect"]
                .as_str()
                .expect("target host effect is named");
            assert!(
                target_host_effects.contains(&target),
                "mapped host effect `{target}` is in connectors' closed vocabulary"
            );
            renamed_effect |= target != effect;
        }
    }
    assert!(
        renamed_effect,
        "the explicit projection must not pretend the two effect vocabularies are identical"
    );
}

// ---------------------------------------------------------------------------------------------
// 1. Determinism
// ---------------------------------------------------------------------------------------------

/// **A rebuild over unchanged inputs writes nothing.**
///
/// The reviewed-tree form of "equal inputs produce byte-identical artifacts": every committed
/// artifact is exactly what the plan would write. A single stale committed artifact fails here by
/// name.
#[test]
fn the_committed_tree_is_a_fixed_point_of_a_build() {
    let (workspace, plan) = full_plan();

    let stale: Vec<String> = plan
        .changes()
        .map(|artifact| {
            format!(
                "  {} ({:?})",
                workspace.display_path(&artifact.path).display(),
                artifact.change
            )
        })
        .collect();

    assert!(
        stale.is_empty(),
        "the committed tree is not a fixed point of a build; run `catalog build`:\n{}",
        stale.join("\n")
    );
}

/// **Two independent plans over the same inputs agree byte for byte.**
///
/// The other half of determinism, and the one a fixed-point test cannot see: a build whose output
/// depended on the scheduler, the filesystem's directory order or a hash seed could still be a
/// fixed point of *itself* while differing between machines. Design 02 §7 item 2 states this over
/// two builds; this is two plans in one process, which is what a test can do cheaply — the
/// cross-machine form is what the lockfile's committed hashes carry.
#[test]
fn two_plans_over_the_same_inputs_are_byte_identical() {
    let workspace = Workspace::new(repo_root());
    let first = pipeline::plan(&workspace, None).expect("the catalogue compiles");
    let second = pipeline::plan(&workspace, None).expect("the catalogue compiles again");

    assert_eq!(first.providers, second.providers);
    assert_eq!(
        first.artifacts.len(),
        second.artifacts.len(),
        "two plans produced different artifact counts"
    );
    for (a, b) in first.artifacts.iter().zip(&second.artifacts) {
        assert_eq!(a.path, b.path, "two plans disagree about artifact order");
        assert_eq!(
            a.contents,
            b.contents,
            "two plans produced different bytes for {}",
            workspace.display_path(&a.path).display()
        );
    }
    assert_eq!(first.diagnostics, second.diagnostics);
}

// ---------------------------------------------------------------------------------------------
// 2. The schema
// ---------------------------------------------------------------------------------------------

/// **Every canonical document validates against the schema the same build plans.**
///
/// Against the *planned* schema rather than the committed one, deliberately: a change that moved
/// both would otherwise be able to pass while the pair on disk disagreed for one commit. The
/// committed schema is covered too, because the fixed-point test above requires the planned schema
/// to equal it.
#[test]
fn every_canonical_document_validates_against_the_committed_schema() {
    let (workspace, plan) = full_plan();

    let schema_text = planned(
        &workspace,
        &plan,
        "catalog/connector-document-v3.schema.json",
    );
    let schema: Value = serde_json::from_str(schema_text).expect("the schema is JSON");
    let validator = jsonschema::validator_for(&schema).expect("the schema compiles");

    for (provider, document) in documents(&workspace, &plan) {
        if let Err(error) = validator.validate(&document) {
            panic!("`{provider}`'s canonical document does not validate: {error}");
        }
        // The two facts the schema cannot state about the file it is validating: that the document
        // knows which connector it is, and that the connector is the one the file is named after.
        assert_eq!(
            document["connector"].as_str(),
            Some(provider.as_str()),
            "`catalog/{provider}.catalog.json` describes a different connector"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 3. The pack
// ---------------------------------------------------------------------------------------------

/// **The pack serves the committed documents byte for byte.**
///
/// Read through `catalog-reader` — the code that actually serves the pack — rather than through a
/// parser written for this test, so the assertion is about what a consumer gets. Three directions:
/// every provider record is exactly its committed document; every operation record is a *substring*
/// of that document that reparses equal to the operation the document carries; and the pack carries
/// neither more nor fewer providers than the catalogue.
#[test]
fn the_pack_serves_the_committed_documents_byte_for_byte() {
    let (workspace, plan) = full_plan();
    let documents = documents(&workspace, &plan);

    let pack_text = planned(&workspace, &plan, "crates/catalog-reader/catalog.pack");
    let pack = catalog_reader::Pack::from_bytes(pack_text.as_bytes().to_vec())
        .expect("the planned pack verifies");

    let pack_ids: Vec<String> = pack
        .providers()
        .map(|provider| provider.id().to_owned())
        .collect();
    let planned_ids: Vec<String> = plan.providers.clone();
    assert_eq!(
        pack_ids,
        {
            let mut sorted = planned_ids.clone();
            sorted.sort();
            sorted
        },
        "the pack and the catalogue carry different providers"
    );

    let mut operations = 0usize;
    for provider in pack.providers() {
        let id = provider.id().to_owned();
        let committed = planned(&workspace, &plan, &format!("catalog/{id}.catalog.json"));
        assert_eq!(
            provider.document(),
            committed,
            "the pack's record for `{id}` is not its canonical document"
        );

        let document = &documents[&id];
        let declared = document["operations"]
            .as_array()
            .unwrap_or_else(|| panic!("`{id}`'s document carries no operations array"));
        let mut seen = 0usize;
        for record in provider.operations() {
            operations += 1;
            seen += 1;
            let sliced: Value = serde_json::from_str(record.record()).unwrap_or_else(|error| {
                panic!("`{}`'s pack record is not JSON: {error}", record.id())
            });
            let carried = declared
                .iter()
                .find(|operation| operation["id"] == sliced["id"])
                .unwrap_or_else(|| {
                    panic!("the pack carries `{}`, which `{id}` does not", record.id())
                });
            assert_eq!(
                &sliced,
                carried,
                "the pack's record for `{}` is not the object its document carries",
                record.id()
            );
            assert_eq!(record.provider(), id);
            assert_eq!(
                record.service(),
                carried["service"].as_str().unwrap_or_default()
            );
        }
        assert_eq!(
            seen,
            declared.len(),
            "the pack carries {seen} operations for `{id}` and its document carries {}",
            declared.len()
        );
    }
    assert!(
        operations >= 835,
        "the catalogue is 835 operations; the pack carries {operations}"
    );
}

// ---------------------------------------------------------------------------------------------
// 4. The lockfile
// ---------------------------------------------------------------------------------------------

/// **`connectors.lock` agrees with every input and every artifact, in both directions.**
///
/// The lockfile is the drift record, so the two ways it can lie are a row whose hashes no longer
/// describe the bytes, and a *missing* row — a provider it stopped knowing about, which would make
/// a future `catalog check` report the catalogue clean because it no longer looked.
///
/// The dropped artifact classes are asserted absent by name. The predecessor's lock carried a row
/// per emitted `.flux` module, per `.connector.toml` manifest, per per-operation rendering and per
/// generated Rust table; none of those artifacts exists here, and a lock that still named one would
/// mean the pipeline had grown an emitter back.
#[test]
fn the_lockfile_agrees_with_every_input_and_every_artifact() {
    let (workspace, plan) = full_plan();

    let text = planned(&workspace, &plan, "connectors.lock");
    let lockfile = connector_spec::Lockfile::parse(text).expect("the planned lockfile parses");

    let rows: BTreeMap<&str, &connector_spec::LockEntry> = lockfile
        .entries()
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect();

    let providers: BTreeSet<&str> = plan.providers.iter().map(String::as_str).collect();
    let recorded: BTreeSet<&str> = rows.keys().copied().collect();
    assert_eq!(
        providers, recorded,
        "the lockfile and the catalogue disagree about which providers exist"
    );

    // The pack row: the digest of the pack this same plan compiled.
    let pack_text = planned(&workspace, &plan, "crates/catalog-reader/catalog.pack");
    let pack = lockfile.pack().expect("a full build records the pack");
    assert_eq!(pack.path, "crates/catalog-reader/catalog.pack");
    assert_eq!(
        pack.sha256,
        connector_spec::sha256_hex(pack_text.as_bytes()),
        "the lockfile's pack digest is not the pack this build compiles"
    );

    /// Artifact-path shapes this pipeline no longer produces. A lock naming one is a pipeline that
    /// grew an emitter back.
    const RETIRED: &[&str] = &[
        "connectors/",
        "crates/catalog/ops/",
        "crates/catalog/src/generated",
        ".flux",
        ".connector.toml",
        "assets/readme-snippet",
    ];

    for provider in &plan.providers {
        let entry = rows[provider.as_str()];

        // The input half: the provider declaration's own bytes.
        let definition = std::fs::read(workspace.providers_dir().join(format!("{provider}.toml")))
            .unwrap_or_else(|error| panic!("read `providers/{provider}.toml`: {error}"));
        assert_eq!(
            entry.toml_sha256.as_deref(),
            Some(connector_spec::sha256_hex(&definition).as_str()),
            "the lockfile's `toml_sha256` for `{provider}` is not the file on disk"
        );
        assert!(
            entry.generator.starts_with("connectors "),
            "`{provider}`'s row records generator `{}`",
            entry.generator
        );

        // The artifact half: every recorded hash is the bytes this build would write, and every
        // artifact this build writes for the provider is recorded.
        assert!(
            !entry.artifacts.is_empty(),
            "`{provider}`'s row records no artifact at all"
        );
        for (key, hash) in &entry.artifacts {
            for retired in RETIRED {
                assert!(
                    !key.contains(retired),
                    "`{provider}`'s row records `{key}`, an artifact class this pipeline does not \
                     produce"
                );
            }
            let contents = planned(&workspace, &plan, key);
            assert_eq!(
                hash,
                &connector_spec::sha256_hex(contents.as_bytes()),
                "the lockfile's hash for `{key}` is not the bytes this build writes"
            );
        }
        assert!(
            entry
                .artifacts
                .contains_key(&format!("catalog/{provider}.catalog.json")),
            "`{provider}`'s row does not record its canonical document"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 5. Identity
// ---------------------------------------------------------------------------------------------

/// **Ids are unique in every namespace that shares one.**
///
/// Two namespaces, and they are different shapes:
///
/// - **Operation ids are global.** They are what a caller names an operation by, so a duplicate
///   across two providers would make one of the two unreachable through the pack's flat lookup.
/// - **Operations, events and channel bindings share one namespace per service.** A channel named
///   after an operation is the "event dressed up as a pollable op" confusion in identifier form.
#[test]
fn ids_are_unique_in_every_namespace_they_share() {
    let (workspace, plan) = full_plan();

    let mut global: BTreeMap<String, String> = BTreeMap::new();
    for (provider, document) in documents(&workspace, &plan) {
        let mut per_service: BTreeMap<(String, String), &'static str> = BTreeMap::new();

        let mut claim = |kind: &'static str, service: &str, name: &str| {
            let key = (service.to_owned(), name.to_owned());
            if let Some(previous) = per_service.insert(key, kind) {
                panic!(
                    "`{provider}` declares `{name}` twice in service `{service}`: as {previous} \
                     and as {kind}. The three member kinds share one namespace per service"
                );
            }
        };

        for operation in document["operations"].as_array().into_iter().flatten() {
            let id = operation["id"].as_str().expect("an operation id");
            let service = operation["service"].as_str().expect("an operation service");
            claim("an operation", service, id);
            if let Some(owner) = global.insert(id.to_owned(), provider.clone()) {
                panic!("operation id `{id}` is declared by both `{owner}` and `{provider}`");
            }
        }
        for event in document["events"].as_array().into_iter().flatten() {
            claim(
                "an event",
                event["service"].as_str().expect("an event service"),
                event["name"].as_str().expect("an event name"),
            );
        }
        for channel in document["channels"].as_array().into_iter().flatten() {
            claim(
                "a channel binding",
                channel["service"].as_str().expect("a channel service"),
                channel["name"].as_str().expect("a channel name"),
            );
        }
    }

    assert!(
        global.len() >= 835,
        "the catalogue is 835 operations; {} were seen",
        global.len()
    );
}

// ---------------------------------------------------------------------------------------------
// 6. No secret anywhere
// ---------------------------------------------------------------------------------------------

/// **No credential value reaches an input or an artifact — asserted at the positions one could
/// occupy, not by grepping the bytes.**
///
/// The declaration surface has no field a secret could live in: `env` and `user_env` name
/// environment-variable *keys*, a scheme names a header and a prefix, and the OAuth2 object is
/// closed against every registration value. That is a property of the types, and a property held by
/// construction is exactly the kind that stops holding quietly when a field is added — so it is
/// checked here over the emitted documents.
///
/// # Why this is not a grep
///
/// A text scan for token shapes was written first and it was wrong in the way that matters: it
/// reported `providers/slack.toml`'s *documentation* of the `xoxb-` prefix, Anthropic's `sk-ant-`
/// in a help string, and a vendor's own base64 example payload quoted inside a response schema.
/// Every one is documentation, and a gate that reports documentation is a gate that gets muted.
///
/// A credential value cannot be told from a plausible example by looking at it. What *can* be told
/// is **where** it would have to sit, so this walks those positions and requires each to hold the
/// kind of thing it is declared to hold. A description is not one of them.
#[test]
fn no_input_or_artifact_carries_a_credential_shaped_value() {
    let (workspace, plan) = full_plan();

    /// Keys the OAuth2 object may carry. A registration value — a client id, a client secret, a
    /// redirect URI an operator configured — is *unrepresentable* by design, and this is what makes
    /// "unrepresentable" a checked claim rather than a schema comment.
    const OAUTH2_KEYS: &[&str] = &[
        "endpoint",
        "token_endpoint",
        "authorize_path",
        "token_path",
        "scopes",
        "scope_separator",
        "scope_response_pointer",
        "grants",
        "redirect",
        "public_client",
    ];

    let mut offences = Vec::new();
    for (provider, document) in documents(&workspace, &plan) {
        for credential in document["auth"].as_array().into_iter().flatten() {
            let name = credential["name"].as_str().unwrap_or("<unnamed>");

            // `env` and `user_env` are *keys*. An entry that is not shaped like an environment
            // variable name is the one way a value could have been written into this field.
            for field in ["env", "user_env"] {
                for entry in credential[field].as_array().into_iter().flatten() {
                    let key = entry.as_str().unwrap_or_default();
                    let shaped = !key.is_empty()
                        && key
                            .chars()
                            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                        && key.starts_with(|c: char| c.is_ascii_uppercase());
                    if !shaped {
                        offences.push(format!(
                            "{provider}: credential `{name}`'s `{field}` holds `{key}`, which is \
                             not an environment-variable name"
                        ));
                    }
                }
            }

            if let Some(oauth2) = credential["oauth2"].as_object() {
                for key in oauth2.keys() {
                    if !OAUTH2_KEYS.contains(&key.as_str()) {
                        offences.push(format!(
                            "{provider}: credential `{name}`'s OAuth2 object carries `{key}`, \
                             which the document has no field for. A registration value is the \
                             deployment's, never the catalogue's"
                        ));
                    }
                }
            }
        }

        // A field declared secret is a question a host asks an operator. A default or an example
        // for it is a value in the catalogue, whatever it is a value *of*.
        for field in document["config"].as_array().into_iter().flatten() {
            if field["secret"].as_bool() != Some(true) {
                continue;
            }
            let name = field["name"].as_str().unwrap_or("<unnamed>");
            for key in ["default", "example"] {
                if !field[key].is_null() {
                    offences.push(format!(
                        "{provider}: secret config field `{name}` carries a `{key}`"
                    ));
                }
            }
        }
    }

    assert!(
        offences.is_empty(),
        "a credential value reached a position that must not hold one:\n  {}",
        offences.join("\n  ")
    );
}

/// `GET`, `POST`, … — the word a `METHOD /path` key is built from.
///
/// Matched exhaustively rather than derived from `Debug`: the key is compared for equality against
/// a diagnostic's own location string, and a derive attribute moving would silently make every
/// comparison false.
fn method_word(method: connector_spec::HttpMethod) -> &'static str {
    use connector_spec::HttpMethod;
    match method {
        HttpMethod::Get => "GET",
        HttpMethod::Post => "POST",
        HttpMethod::Put => "PUT",
        HttpMethod::Patch => "PATCH",
        HttpMethod::Delete => "DELETE",
        HttpMethod::Head => "HEAD",
        HttpMethod::Options => "OPTIONS",
    }
}

// ---------------------------------------------------------------------------------------------
// 7. Spec-backed coverage
// ---------------------------------------------------------------------------------------------

/// **For every provider that declares spec ingest, coverage holds in both directions.**
///
/// The predecessor asserted this one provider at a time, with a hand-kept allow-list of gaps per
/// connector. Stated once, over every spec-backed provider, the rule has two halves that need no
/// list at all:
///
/// - **Nothing invented.** Every published operation's `METHOD /path` is one some vendored document
///   declares. This is what would catch a connector accreting a hand-authored operation beside a
///   spec-backed surface, where it would look reviewed and be unbacked.
/// - **Nothing silently dropped.** Every `[[patch.operations]]` entry names an `operationId` the
///   OpenAPI ingest really carries, and lands as a published operation — unless it declares
///   `defer`, in which case it must *not* be published. Every exact `[[patch.events]]` selection
///   likewise names an AsyncAPI component message and lands with its source payload schema. A
///   selector that matches nothing is a coverage claim that quietly failed, which is the failure
///   the per-provider allow-lists existed to catch.
#[test]
fn spec_backed_coverage_holds_in_both_directions() {
    let workspace = Workspace::new(repo_root());
    let providers = catalog_build::discovery::discover(&workspace, None)
        .expect("the catalogue is discoverable");

    let mut spec_backed = 0usize;
    for provider in &providers {
        let inputs = catalog_build::seam::ProviderInputs::read(provider)
            .unwrap_or_else(|error| panic!("read `{}`: {error:#}", provider.name));
        let loaded = catalog_build::seam::load_full(&inputs)
            .unwrap_or_else(|error| panic!("load `{}`: {error:#}", provider.name));
        if loaded.ingested.is_empty() && loaded.ingested_events.is_empty() {
            continue;
        }
        spec_backed += 1;
        let name = &provider.name;

        // Everything the documents declare, ingest diagnostics included: an endpoint ingest could
        // not express is still an endpoint the vendor declared, and dropping it from the left-hand
        // side is how a coverage gate comes to compare a set against itself.
        let mut declared: BTreeSet<String> = BTreeSet::new();
        let mut operation_ids: BTreeSet<&str> = BTreeSet::new();
        for document in &loaded.ingested {
            for operation in &document.ingested.operations {
                declared.insert(format!(
                    "{} {}",
                    method_word(operation.method),
                    operation.path
                ));
                operation_ids.insert(operation.operation_id.as_str());
            }
            for diagnostic in &document.ingested.diagnostics {
                if let Some((head, path)) = diagnostic.location.split_once(' ') {
                    if path.starts_with('/') && head.chars().all(|c| c.is_ascii_uppercase()) {
                        declared.insert(diagnostic.location.clone());
                    }
                }
            }
        }

        // **Only the spec-derived operations.** A provider may be spec-backed *and* carry
        // hand-authored operations beside the ingest — `microsoft_graph` is the shipped case — and
        // those are reviewed as authored text rather than against a document. The join is the
        // connector's own provenance, which the loader fills on the patch path only, so an inline
        // operation cannot forge one.
        let invented: Vec<String> = loaded
            .connector
            .operations
            .iter()
            .filter(|operation| {
                loaded
                    .connector
                    .provenance
                    .operation_specs
                    .contains_key(&operation.id)
            })
            .map(|operation| {
                let method = operation
                    .request
                    .http_method()
                    .expect("spec-derived operations use the HTTP driver");
                let path = operation
                    .request
                    .http_path()
                    .expect("spec-derived operations use the HTTP driver");
                format!("{} {path}", method_word(method))
            })
            .filter(|key| !declared.contains(key))
            .collect();
        assert!(
            invented.is_empty(),
            "`{name}` publishes spec-derived operations no vendored document declares: \
             {invented:?}"
        );

        // Which vendor `operationId` each published operation came from. Joining on this rather
        // than on the patch's `rename` is what makes the rule general: a connector may declare a
        // `[patch.naming]` rule instead of renaming one operation at a time, and the published id
        // is then *derived* — reproducing that derivation here would be a second implementation of
        // it, free to agree with the loader by luck.
        let published: BTreeSet<&str> = loaded
            .connector
            .provenance
            .operation_specs
            .values()
            .map(|source| source.operation_id.as_str())
            .collect();

        for patch in &loaded.patch.operations {
            assert!(
                operation_ids.contains(patch.select.as_str()),
                "`{name}` patches `{}`, which no vendored document declares. A selector that \
                 matches nothing is a coverage claim that quietly failed",
                patch.select
            );
            match &patch.defer {
                Some(reason) => assert!(
                    !published.contains(patch.select.as_str()),
                    "`{name}` defers `{}` — {reason} — and publishes it anyway",
                    patch.select
                ),
                None => assert!(
                    published.contains(patch.select.as_str()),
                    "`{name}` selects `{}` and publishes nothing derived from it",
                    patch.select
                ),
            }
        }

        for patch in &loaded.patch.events {
            let document = match patch.service.as_deref() {
                Some(service) => loaded
                    .ingested_events
                    .iter()
                    .find(|document| document.service == service),
                None if loaded.ingested_events.len() == 1 => loaded.ingested_events.first(),
                None => None,
            }
            .unwrap_or_else(|| {
                panic!(
                    "`{name}` selects AsyncAPI event `{}` without one unambiguous source document",
                    patch.select
                )
            });
            let source = document.ingested.event(&patch.select).unwrap_or_else(|| {
                panic!(
                    "`{name}` selects AsyncAPI event `{}`, which its source does not declare",
                    patch.select
                )
            });
            let published_name = patch.rename.as_deref().unwrap_or(&source.name);
            let event = loaded
                .connector
                .events
                .iter()
                .find(|event| {
                    event.service == document.service && event.name == published_name
                })
                .unwrap_or_else(|| panic!(
                    "`{name}` selects AsyncAPI event `{}` but publishes no `{published_name}` event",
                    patch.select
                ));
            assert_eq!(
                event.schema.as_ref(),
                Some(&source.payload),
                "`{name}` event `{published_name}` must retain the selected AsyncAPI payload schema"
            );
        }
    }

    assert!(
        spec_backed >= 7,
        "only {spec_backed} providers declare spec ingest, so this asserted almost nothing"
    );
}

/// Anthropic is the first shipped connector whose complete machine-readable source is authored in
/// this repository because the vendor publishes HTML reference pages rather than OpenAPI bytes.
/// Keep that exception honest: both source documents participate, every shipped operation points
/// back to one of them, and no Claude/Claude Code credential-acquisition authority leaks into the
/// API connector.
#[test]
fn repository_authored_anthropic_sources_reproduce_only_the_api_connector() {
    let workspace = Workspace::new(repo_root());
    let provider = catalog_build::discovery::discover(&workspace, Some("anthropic"))
        .expect("discover Anthropic")
        .into_iter()
        .next()
        .expect("Anthropic is shipped");
    let inputs = catalog_build::seam::ProviderInputs::read(&provider).expect("read Anthropic");
    let loaded = catalog_build::seam::load_full(&inputs).expect("compile authored Anthropic specs");

    assert_eq!(
        loaded.ingested.len(),
        2,
        "Models and Admin are separate source documents"
    );
    assert_eq!(loaded.connector.operations.len(), 11);
    assert_eq!(loaded.connector.provenance.operation_specs.len(), 11);
    assert_eq!(
        loaded
            .connector
            .auth
            .iter()
            .map(|method| method.name.as_str())
            .collect::<Vec<_>>(),
        ["anthropic.api_key", "anthropic.admin_key"],
        "harness or undocumented OAuth authority must not enter the API connector"
    );
    assert!(
        loaded
            .connector
            .services
            .iter()
            .all(|service| { service.name == "models" || service.name == "admin" }),
        "only documented API surfaces may become connector services"
    );

    for input in &inputs.specs {
        if !input.path.starts_with("specs/anthropic/") {
            continue;
        }
        assert!(input
            .document
            .contains("x-b10x-origin: repository-authored"));
        assert!(input.document.contains("x-b10x-source-reference:"));
        assert!(!input.document.contains("claude.ai/oauth"));
        assert!(!input.document.contains("platform.claude.com/v1/oauth"));
    }
}

// ---------------------------------------------------------------------------------------------
// 8. Orphans
// ---------------------------------------------------------------------------------------------

/// **No committed file under an artifact root that the plan does not claim.**
///
/// The inverse of the fixed-point test: that one says every planned artifact matches the tree, this
/// says the tree holds nothing the plan forgot. A document whose provider was deleted, or a
/// rendering left behind by a dropped artifact class, is an orphan — a file that still validates,
/// still parses and describes nothing.
#[test]
fn a_full_build_leaves_no_orphaned_artifact() {
    let (workspace, plan) = full_plan();

    let orphans: Vec<String> = plan
        .orphans
        .iter()
        .map(|orphan| {
            format!(
                "  {} (under {})",
                workspace.display_path(&orphan.path).display(),
                workspace.display_path(&orphan.root).display()
            )
        })
        .collect();

    assert!(
        orphans.is_empty(),
        "committed files under an artifact root that no plan claims:\n{}",
        orphans.join("\n")
    );
}

// ---------------------------------------------------------------------------------------------
// 9–12. The caller's contract (S-001)
// ---------------------------------------------------------------------------------------------

/// **A caller that reads an operation's contract can make the call.**
///
/// The contract states which parameters are required. This asserts that supplying exactly those —
/// and nothing else — is enough for every exposed operation in the catalogue to be *admissible*:
/// no parameter the caller was never told about may be mandatory.
///
/// The defect that prompted it. `slack-conversations-history` declares five parameters and marks
/// one required, so a caller sent `{"channel": …}` and was refused `connector-invalid-input:
/// Slack operation input is invalid`, naming nothing. The request resolver required *every*
/// declared parameter to be present, treating an optional one as "may be null, may not be absent";
/// the contract said the opposite. Both were internally consistent and together they made an
/// operation unusable, with a refusal that pointed at the caller's input rather than at the
/// disagreement. It was never Slack-specific — the same rule sat in front of all 62 providers.
///
/// Stated over the catalogue rather than per provider, because that is the shape of the rule: the
/// sixty-third connector is covered the moment it exists.
#[test]
fn every_operations_required_parameters_are_the_ones_a_caller_must_send() {
    let (workspace, plan) = full_plan();
    let mut checked = 0;
    for (provider, document) in documents(&workspace, &plan) {
        for operation in document["operations"].as_array().into_iter().flatten() {
            let id = operation["id"].as_str().unwrap_or_default();
            let required: std::collections::BTreeSet<&str> = operation["contract"]["input_schema"]
                ["required"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .collect();
            for param in operation["params"].as_array().into_iter().flatten() {
                let symbol = param["symbol"].as_str().unwrap_or_default();
                let position = param["position"].as_str().unwrap_or_default();
                // The resolver refuses an absent parameter exactly when the document marks it
                // required, so those are the ones the contract must also list. Anything the
                // document calls optional may be omitted and the contract need not mention it.
                if !param["required"].as_bool().unwrap_or(false) {
                    continue;
                }
                checked += 1;
                assert!(
                    required.contains(symbol),
                    "`{provider}`'s `{id}` needs `{symbol}` (position `{position}`) but its \
                     contract does not require it, so a caller reading the contract cannot call it"
                );
            }
        }
    }
    assert!(
        checked > 0,
        "the catalogue published no mandatory parameter to check"
    );
}

/// **Every operation stores the model-facing contract, and every parameter its symbol** (S-001).
///
/// The two measured cases that motivated the predecessor's C-552 are pinned by name: babelforce's
/// dotted `time.start` must carry the normalized symbol `time_start`, and `airtable-record-get`'s
/// contract description must be the one-line summary *extended* with the error-envelope sentence
/// — longer than the summary, stating where the vendor's error message lives.

#[test]
fn the_document_carries_the_callers_contract() {
    let (workspace, plan) = full_plan();
    let mut operations = 0;
    let mut symbols = 0;
    for (provider, document) in documents(&workspace, &plan) {
        for operation in document["operations"].as_array().into_iter().flatten() {
            operations += 1;
            let id = operation["id"].as_str().unwrap_or_default();
            let contract = &operation["contract"];
            // Present, not necessarily non-empty: an operation whose one-line summary is empty
            // and that declares no error envelope stores the empty description it has — the
            // contract carries what the declaration states, it does not invent prose.
            assert!(
                contract["description"].is_string(),
                "`{provider}`'s `{id}` stores no contract description"
            );
            let schema = &contract["input_schema"];
            assert_eq!(
                schema["type"].as_str(),
                Some("object"),
                "`{provider}`'s `{id}` stores a non-object contract input schema"
            );
            for param in operation["params"].as_array().into_iter().flatten() {
                symbols += 1;
                assert!(
                    !param["symbol"].as_str().unwrap_or_default().is_empty(),
                    "`{provider}`'s `{id}` parameter `{}` carries no symbol",
                    param["name"].as_str().unwrap_or_default()
                );
            }
        }
    }
    assert!(operations >= 835, "only {operations} operations checked");
    assert!(symbols >= 1518, "only {symbols} parameter symbols checked");

    let all = documents(&workspace, &plan);
    let babelforce = &all["babelforce"];
    let dotted = babelforce["operations"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|op| op["params"].as_array().into_iter().flatten())
        .find(|param| param["name"] == "time.start")
        .expect("babelforce declares the dotted `time.start`");
    assert_eq!(dotted["symbol"], "time_start");

    let airtable = &all["airtable"];
    let record_get = airtable["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|op| op["id"] == "airtable-record-get")
        .expect("airtable-record-get is shipped");
    let summary = record_get["description"].as_str().unwrap();
    let extended = record_get["contract"]["description"].as_str().unwrap();
    assert!(extended.starts_with(summary.trim_end_matches('.')));
    assert!(extended.len() > summary.len());
    assert!(extended.contains("A non-2xx response is returned as data"));
}

/// **The two places the document states a symbol agree** (S-001): the contract's `input_schema`
/// is keyed by exactly the declared params' symbols, its `required` list is those symbols in
/// declaration order, and no two parameters of one operation share a symbol. A lowering bug that
/// let the schema keys drift from the params would ship a contract a caller cannot satisfy.
#[test]
fn the_contract_and_the_params_state_the_same_symbols() {
    let (workspace, plan) = full_plan();
    let mut compared = 0;
    for (provider, document) in documents(&workspace, &plan) {
        for operation in document["operations"].as_array().into_iter().flatten() {
            let id = operation["id"].as_str().unwrap_or_default();
            let declared: Vec<&str> = operation["params"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|param| param["symbol"].as_str().unwrap_or_default())
                .collect();
            let unique: BTreeSet<&str> = declared.iter().copied().collect();
            assert_eq!(
                unique.len(),
                declared.len(),
                "`{provider}`'s `{id}` hands one symbol to two parameters"
            );
            // **Requiredness is carried, not asserted.** This used to demand that every declared
            // symbol appear in `required`, which pinned a projection that marked all 989
            // operations' parameters mandatory -- including the 248 that declare optional ones.
            // The invariant worth holding is that the contract and the params never disagree, and
            // that is now checked in the direction that can actually drift: a symbol is required in
            // the contract exactly when its parameter says it is.
            let required: Vec<&str> = operation["contract"]["input_schema"]["required"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .map(|value| value.as_str().unwrap_or_default())
                .collect();
            let declared_required: Vec<&str> = operation["params"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|param| param["required"].as_bool().unwrap_or(true))
                .map(|param| param["symbol"].as_str().unwrap_or_default())
                .collect();
            assert_eq!(
                required, declared_required,
                "`{provider}`'s `{id}` contract requires different symbols than its params declare \
                 as required"
            );
            let keys: BTreeSet<&str> = operation["contract"]["input_schema"]["properties"]
                .as_object()
                .map(|properties| properties.keys().map(String::as_str).collect())
                .unwrap_or_default();
            assert_eq!(
                keys, unique,
                "`{provider}`'s `{id}` contract is keyed by symbols its params do not declare"
            );
            compared += 1;
        }
    }
    assert!(compared >= 835, "only {compared} operations compared");
}

/// **A `format = "origin"` field's variable lands on exactly `["origin"]`** (S-001; the
/// predecessor's C-538 open question 3). The format promises "swap the whole authority"; a
/// variable bound inside a larger authority (`https://{v}.x/`) would lower to `host` instead, and
/// silently dropping Origin→Host is the failure this gate turns red. Asserted beside the loader's
/// own IR-layer refusal, over the artifact a consumer actually reads.
#[test]
fn every_format_origin_field_lowers_to_the_origin_slot() {
    let (workspace, plan) = full_plan();
    let mut checked = 0;
    for (provider, document) in documents(&workspace, &plan) {
        for field in document["config"].as_array().into_iter().flatten() {
            if field["format"] != "origin" {
                continue;
            }
            let mut variables: Vec<&str> = Vec::new();
            for bound in std::iter::once(&field["binds"])
                .chain(field["also_binds"].as_array().into_iter().flatten())
            {
                let bound = bound.as_str().unwrap_or_default();
                if let Some(variable) = bound.strip_prefix("endpoint.") {
                    variables.push(variable);
                }
            }
            assert!(
                !variables.is_empty(),
                "`{provider}`'s origin field `{}` binds no endpoint variable",
                field["name"].as_str().unwrap_or_default()
            );
            for operation in document["operations"].as_array().into_iter().flatten() {
                let Some(endpoint) = operation["endpoint"].as_object() else {
                    continue;
                };
                for variable in &variables {
                    let Some(slots) = endpoint.get(*variable) else {
                        continue;
                    };
                    assert_eq!(
                        slots,
                        &serde_json::json!(["origin"]),
                        "`{provider}`'s `{}` lowers origin variable `{variable}` to {slots}",
                        operation["id"].as_str().unwrap_or_default()
                    );
                    checked += 1;
                }
            }
        }
    }
    assert!(
        checked > 0,
        "no origin binding was checked — the gate is blind"
    );
}

/// **The stored `credential_requirement` agrees with the effective `auth` list** (S-001):
/// `declared` exactly when the list is non-empty, and the empty side carries one of the two
/// distinction tokens. The distinction itself — declared-empty versus never-declared — is not
/// re-derivable from the document, which is the point; what is checkable is that the token and
/// the list never contradict each other.
#[test]
fn the_credential_requirement_agrees_with_the_auth_list() {
    let (workspace, plan) = full_plan();
    let mut checked = 0;
    for (provider, document) in documents(&workspace, &plan) {
        for operation in document["operations"].as_array().into_iter().flatten() {
            let id = operation["id"].as_str().unwrap_or_default();
            let token = operation["credential_requirement"]
                .as_str()
                .unwrap_or_default();
            let declared = !operation["auth"]
                .as_array()
                .map(Vec::is_empty)
                .unwrap_or(true);
            if declared {
                assert_eq!(
                    token, "declared",
                    "`{provider}`'s `{id}` authenticates but claims `{token}`"
                );
            } else {
                assert!(
                    token == "no-credential-required" || token == "no-credential",
                    "`{provider}`'s `{id}` has an empty auth list but claims `{token}`"
                );
            }
            checked += 1;
        }
    }
    assert!(checked >= 835, "only {checked} operations checked");
}

// ---------------------------------------------------------------------------------------------
// 15. Effect backends trust only proofs
// ---------------------------------------------------------------------------------------------

/// Every admission-relevant reading of `approval_evidence_ref` an Integration is allowed: none.
///
/// The field may reach a backend **only as audit correlation**, and any such use must be named
/// here as `"crates/integration-<id>/src/<file>.rs:<line>"` with a reason in the adjacent
/// comment — the same pinned-allowlist shape the auth-list invariant uses. An empty list is the
/// expected state: today no Integration reads the field at all.
const APPROVAL_EVIDENCE_AUDIT_CORRELATION_USES: &[&str] = &[];

/// No effect-bearing backend code path is reachable without an `AdmittedOperation` built from
/// proofs (S-047, design 13).
///
/// One rule over every Integration, discovered from the tree, so the next Integration crate is
/// covered the moment it exists. The sealed admission chain — `GrantEvaluator` →
/// `GrantDecision`, `ApprovalGate` → `ApprovalRedemption`, both consumed by
/// `AdmittedOperation::from_decision` in `crates/server/src/hosted/enforcement.rs` — is the only
/// authority over Grant and approval admission. An Integration that reads the invocation's
/// `approval_evidence_ref` re-decides admission locally on a caller-supplied string, which is
/// exactly the representational check the 2026-08-17 review's F-001 found bypassable. Writing
/// `approval_evidence_ref: None` (carrying no evidence into an internally built invocation) is
/// construction, not a read, and stays admitted.
#[test]
fn no_effect_backend_is_reachable_without_an_admission_proof() {
    let root = repo_root();
    let mut integrations = Vec::new();
    let crates_dir = root.join("crates");
    let entries = std::fs::read_dir(&crates_dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", crates_dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        if path.is_dir() && name.starts_with("integration-") {
            integrations.push((name, path));
        }
    }
    integrations.sort();
    assert!(
        integrations.len() >= 8,
        "the tree carries 8 Integration crates; the scan found {}",
        integrations.len()
    );
    let mut violations = Vec::new();
    for (_, integration) in &integrations {
        for source in rust_sources(&integration.join("src")) {
            let file_name = source
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            // House convention: test modules live in `tests.rs` / `*_tests.rs` siblings or in
            // inline `#[cfg(test)]` items; both build invocations and are not admission reads.
            if file_name.ends_with("tests.rs") {
                continue;
            }
            let text = std::fs::read_to_string(&source)
                .unwrap_or_else(|error| panic!("read {}: {error}", source.display()));
            let relative = source
                .strip_prefix(&root)
                .expect("integration sources live under the repository root")
                .to_string_lossy()
                .replace('\\', "/");
            for (index, line) in production_lines(&text) {
                if !line.contains("approval_evidence_ref") {
                    continue;
                }
                if line.trim() == "approval_evidence_ref: None," {
                    continue;
                }
                let place = format!("{relative}:{}", index + 1);
                if APPROVAL_EVIDENCE_AUDIT_CORRELATION_USES.contains(&place.as_str()) {
                    continue;
                }
                violations.push(format!("{place}: `{}`", line.trim()));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "an Integration reads `approval_evidence_ref`, re-deciding admission outside the sealed \
         proof chain (S-043..S-046); delete the check — the hosted route already refused or \
         admitted this invocation — or, for a genuine audit-correlation use, name the exact line \
         in `APPROVAL_EVIDENCE_AUDIT_CORRELATION_USES` with its reason:\n{}",
        violations.join("\n")
    );
}

// ---------------------------------------------------------------------------------------------
// 16. A session signal reaches a backend only through the admission seam
// ---------------------------------------------------------------------------------------------

/// The operation protocol's request grammar, exactly as `OperationRequest` declares it — a
/// pinned closed set, in declaration order.
///
/// `SessionSignal` escaped the enforced-authority epic (S-043..S-047) because it fell through
/// the hosted route's catch-all dispatch: nothing forced its author to place it behind the
/// admission seam, and the compiler was happy. This pin is that force. Adding a request variant
/// — in particular a new signal-shaped route — fails the invariant until the variant is named
/// here, and naming it here is this test demanding, in its failure message, that the hosted
/// route decide the variant's admission on purpose.
const OPERATION_REQUEST_VARIANTS: &[&str] = &[
    "Search",
    "Describe",
    "Invoke",
    "SessionStatus",
    "SessionTerminate",
    "SessionReconcile",
    "SessionSignal",
];

/// No session signal — nor any future acting variant — reaches a backend without the hosted
/// route deciding its admission (S-049, design 13).
///
/// Three claims, each mechanical: the protocol's request grammar is exactly the pinned closed
/// set above; the hosted route's production code names every variant of that grammar, so none
/// can exist that the route never considered; and the S-049 signal seam is present — the
/// admission call (`admit_signal`) and the proof-consuming dispatch (`dispatch_admitted_signal`)
/// — so deleting or bypassing it is loud. The local placement stays out of scope on purpose:
/// the owner speaking over their own 0700 socket is design 13's named local-owner admission
/// path, not a Grant decision.
///
/// 2026-08-24 (S-053, design 14): `/mcp` is a funneled entry. The MCP transport
/// (`crates/server/src/hosted/mcp.rs`) synthesizes operation and datasource envelopes and
/// hands them to the decided halves of the hosted handlers (`operation_decided`,
/// `datasource_decided`) inside `hosted.rs`, so every decision this rule pins also covers MCP
/// callers; rule 17 below keeps the MCP modules free of every direct-backend token.
#[test]
fn a_session_signal_reaches_a_backend_only_through_the_admission_seam() {
    let root = repo_root();
    let protocol_source = root.join("crates/protocol/src/operation/legacy.rs");
    let protocol = std::fs::read_to_string(&protocol_source)
        .unwrap_or_else(|error| panic!("read {}: {error}", protocol_source.display()));
    let declared = operation_request_variants(&protocol);
    assert_eq!(
        declared, OPERATION_REQUEST_VARIANTS,
        "`OperationRequest` no longer matches the pinned request grammar. A new variant is a \
         new route: route it through the hosted admission seam (`crates/server/src/hosted.rs`, \
         the S-049 block that admits `SessionSignal`) or the invoke seam on purpose, add route \
         tests proving an ungranted request refuses, and only then extend \
         `OPERATION_REQUEST_VARIANTS`"
    );
    let hosted_source = root.join("crates/server/src/hosted.rs");
    let hosted = std::fs::read_to_string(&hosted_source)
        .unwrap_or_else(|error| panic!("read {}: {error}", hosted_source.display()));
    // Production lines with line comments cut away: a comment that merely mentions a seam or a
    // variant must not satisfy this invariant. (Cutting at `//` inside a string literal is a
    // theoretical false negative; the tokens searched here never share a line with such a
    // string, and a false negative fails loudly rather than admitting a bypass.)
    let hosted_production = production_lines(&hosted)
        .into_iter()
        .map(|(_, line)| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    for variant in OPERATION_REQUEST_VARIANTS {
        assert!(
            hosted_production.contains(&format!("OperationRequest::{variant}")),
            "the hosted route's production code never names `OperationRequest::{variant}`: \
             every request variant must be considered by the route, or it reaches the backend \
             through the catch-all dispatch with no admission decision"
        );
    }
    for seam in ["admit_signal", "dispatch_admitted_signal"] {
        assert!(
            hosted_production.contains(seam),
            "the hosted route lost its session-signal admission seam (`{seam}`): an \
             effect-bearing signal must be refused unless a Grant admits the session's own \
             operation (S-049)"
        );
    }
}

/// The variants of `pub enum OperationRequest`, parsed from the protocol source in declaration
/// order — and **closed over line shapes**: every line directly inside the enum body must be
/// blank, a comment, an attribute, or a variant this parser recognizes (tuple, struct, or unit
/// shaped). An unrecognized line is a failure, not a skip — a variant shape the parser cannot
/// read is exactly how a new route could slip past the pin, so the parser refuses to guess.
fn operation_request_variants(protocol: &str) -> Vec<String> {
    let mut variants = Vec::new();
    let mut inside = false;
    // Brace and paren depth at the *start* of the current line: a line opening at brace depth 1
    // and paren depth 0 introduces a variant (or is trivia); anything deeper is a variant's own
    // body (struct fields, a reformatted tuple payload) and is not a declaration site.
    let mut braces: i64 = 0;
    let mut parens: i64 = 0;
    for (index, line) in protocol.lines().enumerate() {
        if !inside {
            if line.starts_with("pub enum OperationRequest {") {
                inside = true;
                braces = 1;
            }
            continue;
        }
        let starts_interior = braces > 1 || parens > 0;
        braces += i64::try_from(line.matches('{').count()).expect("line braces fit")
            - i64::try_from(line.matches('}').count()).expect("line braces fit");
        parens += i64::try_from(line.matches('(').count()).expect("line parens fit")
            - i64::try_from(line.matches(')').count()).expect("line parens fit");
        if braces <= 0 {
            break;
        }
        if starts_interior {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#[") {
            continue;
        }
        let name: String = trimmed
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric())
            .collect();
        let rest = trimmed[name.len()..].trim_start();
        let variant_shaped = name.chars().next().is_some_and(char::is_uppercase)
            && (rest.is_empty()
                || rest.starts_with('(')
                || rest.starts_with('{')
                || rest.starts_with(','));
        assert!(
            variant_shaped,
            "line {} inside `pub enum OperationRequest` is not a shape this invariant can read \
             (`{trimmed}`). The parser is deliberately closed so no variant can slip past the \
             pinned grammar; teach `operation_request_variants` the new shape on purpose, then \
             route the variant through the hosted admission seam",
            index + 1
        );
        variants.push(name);
    }
    assert!(
        inside,
        "`pub enum OperationRequest` was not found in the protocol source; if the enum moved, \
         point this invariant at its new home rather than deleting it"
    );
    variants
}

// ---------------------------------------------------------------------------------------------
// 17. The MCP transport reaches a backend only through the decided admission seams
// ---------------------------------------------------------------------------------------------

/// The direct-backend vocabulary the MCP modules must never speak: the backend field and
/// trait, and the proven dispatch and admission seams, which belong to `hosted.rs` alone.
const MCP_DIRECT_BACKEND_TOKENS: &[&str] = &[
    ".backend",
    "ConnectorBackend",
    "dispatch_admitted",
    "admit_",
];

/// The MCP modules whose production lines are scanned.
const MCP_MODULES: &[&str] = &[
    "crates/server/src/hosted/mcp.rs",
    "crates/server/src/hosted/mcp/toolset.rs",
];

/// The MCP transport adds zero policy (S-053, design 14): every `tool_search`,
/// `tool_describe` and `tool_invoke` funnels through the decided halves of the hosted
/// handlers — `operation_decided` and `datasource_decided` in `crates/server/src/hosted.rs` —
/// where the scope map, the receiver policy, re-description, and Grant/approval admission
/// live. A line in the MCP modules that names the backend field, the backend trait, a proven
/// dispatch seam, or an admission call is that module starting to re-decide admission beside
/// the seam, which is exactly the split this rule's sibling (rule 16) exists to prevent for
/// signals. Line comments are cut before matching so prose about the seams stays legal; the
/// scan refuses a module it cannot read, so renaming a file does not silently retire the rule.
#[test]
fn the_mcp_transport_reaches_a_backend_only_through_the_decided_seams() {
    let root = repo_root();
    let mut violations = Vec::new();
    for relative in MCP_MODULES {
        let source = root.join(relative);
        let text = std::fs::read_to_string(&source)
            .unwrap_or_else(|error| panic!("read {}: {error}", source.display()));
        for (index, line) in production_lines(&text) {
            let code = line.split("//").next().unwrap_or("");
            for token in MCP_DIRECT_BACKEND_TOKENS {
                if code.contains(token) {
                    violations.push(format!("{relative}:{}: `{}`", index + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "an MCP module names a direct-backend token; every MCP call must reach backends \
         through `operation_decided`/`datasource_decided` in `hosted.rs` (S-053, design 14):\n{}",
        violations.join("\n")
    );
}

/// Every `.rs` file under one directory, recursively.
fn rust_sources(root: &Path) -> Vec<PathBuf> {
    fn visit(path: &Path, sources: &mut Vec<PathBuf>) {
        let entries = std::fs::read_dir(path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                visit(&path, sources);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                sources.push(path);
            }
        }
    }
    let mut sources = Vec::new();
    visit(root, &mut sources);
    sources.sort();
    sources
}

/// The production lines of one source file: everything outside `#[cfg(test)]` items.
///
/// Brace-tracked rather than cut-at-first-marker, because `#[cfg(test)]` also guards mid-file
/// helpers (`integration-kubernetes/src/hosted.rs` has one), and cutting to the end of the file
/// there would hide production code from the invariant.
fn production_lines(text: &str) -> Vec<(usize, &str)> {
    let mut lines = Vec::new();
    // The brace depth the scanner must return to for the excluded item to be over.
    let mut excluded_base: Option<i64> = None;
    // A `#[cfg(test)]` was seen and the guarded item's body has not opened yet.
    let mut pending = false;
    let mut depth: i64 = 0;
    for (index, line) in text.lines().enumerate() {
        let opens = i64::try_from(line.matches('{').count()).expect("line braces fit");
        let closes = i64::try_from(line.matches('}').count()).expect("line braces fit");
        let after = depth + opens - closes;
        if let Some(base) = excluded_base {
            depth = after;
            if depth <= base {
                excluded_base = None;
            }
            continue;
        }
        if pending {
            if opens > 0 {
                excluded_base = Some(depth);
                pending = false;
                depth = after;
                if depth <= excluded_base.expect("just set") {
                    excluded_base = None;
                }
            } else if line.trim_end().ends_with(';') {
                // A braceless guarded item (`#[cfg(test)] use …;`) excludes only itself.
                pending = false;
            }
            // Attribute and signature lines between the marker and the body stay excluded.
            continue;
        }
        if line.trim_start().starts_with("#[cfg(test)]") {
            pending = true;
            continue;
        }
        depth = after;
        lines.push((index, line));
    }
    lines
}

/// **A custody-only provider publishes nothing that could spend its credential** — S-070.
///
/// Parameterised over the committed tree, so the provider that arrives in S-071 is covered the
/// moment it exists rather than when somebody remembers. The loader already refuses each of these
/// keys by name; this asserts the *published* document agrees, because the catalog is what a
/// consumer reads and a loader-time refusal it cannot see is a claim rather than a property.
///
/// Both directions. A custody-only document carries no surface; an ordinary document carries at
/// least one service, so the exemption cannot spread by accident to a provider that merely forgot
/// to declare one.
#[test]
fn a_custody_only_provider_publishes_no_surface_and_every_other_provider_does() {
    let (workspace, plan) = full_plan();
    let documents = documents(&workspace, &plan);
    let mut custody = Vec::new();

    for (provider, document) in &documents {
        let empty = |key: &str| {
            document
                .get(key)
                .map(|value| value.as_array().expect("an array").is_empty())
                .unwrap_or(true)
        };

        if document.get("custody_only").is_none() {
            assert!(
                !empty("services"),
                "`{provider}` publishes no service and does not declare `custody_only`; a \
                 provider with no surface either states the kind or is a lowering bug"
            );
            continue;
        }

        custody.push(provider.clone());
        assert_eq!(
            document["custody_only"],
            json!(true),
            "`{provider}` may only spell the flag as `true`; it is skipped when false"
        );
        for key in [
            "services",
            "operations",
            "events",
            "channels",
            "discoveries",
        ] {
            assert!(
                empty(key),
                "`{provider}` is custody-only but publishes `{key}`, so something in connectors \
                 could spend the credential"
            );
        }
        assert!(
            document.get("verify").is_none(),
            "`{provider}` is custody-only but publishes a verification probe, which is a request"
        );
        assert!(
            !empty("auth"),
            "`{provider}` is custody-only and holds nothing, so it has no reason to exist"
        );
        for method in document["auth"].as_array().expect("auth") {
            assert!(
                method.get("oauth2").is_none(),
                "`{provider}`'s credential declares `oauth2`, which says the host runs the token \
                 grants — a request"
            );
        }
    }

    // Recorded rather than asserted non-empty: the kind is empty until S-071 lands `claude-code`,
    // and a test that demanded a member would have to be edited to admit the first one.
    assert!(
        custody.len() <= documents.len(),
        "custody-only providers in the committed tree: {custody:?}"
    );
}

#[test]
fn gitlab_schedule_slice_is_generated_and_preserves_legacy_contracts() {
    let workspace = Workspace::new(repo_root());
    let provider = catalog_build::discovery::discover(&workspace, Some("gitlab"))
        .expect("discover GitLab")
        .remove(0);
    let inputs = catalog_build::seam::ProviderInputs::read(&provider).expect("read GitLab");
    let loaded = catalog_build::seam::load_full(&inputs).expect("load GitLab");
    let expected = [
        (
            "gitlab-pipeline-schedule-list",
            "GET",
            "read",
            "low",
            "idempotent",
        ),
        (
            "gitlab-pipeline-schedule-create",
            "POST",
            "write",
            "high",
            "non_idempotent",
        ),
        (
            "gitlab-pipeline-schedule-update",
            "PUT",
            "write",
            "high",
            "non_idempotent",
        ),
        (
            "gitlab-pipeline-schedule-delete",
            "DELETE",
            "write",
            "destructive",
            "idempotent",
        ),
    ];
    let (_, plan) = full_plan();
    let all = documents(&workspace, &plan);
    let gitlab = &all["gitlab"];
    for (id, method, direction, risk, idempotency) in expected {
        let operation = gitlab["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|operation| operation["id"] == id)
            .unwrap_or_else(|| panic!("missing {id}"));
        assert!(
            loaded.connector.provenance.operation_specs.contains_key(id),
            "{id} must derive from the official source"
        );
        assert_eq!(operation["direction"], direction);
        assert_eq!(operation["risk"], risk);
        assert_eq!(operation["idempotency"], idempotency);
        let ir = loaded
            .connector
            .operations
            .iter()
            .find(|operation| operation.id == id)
            .unwrap();
        assert_eq!(method_word(ir.request.http_method().unwrap()), method);
        assert!(ir
            .request
            .http_path()
            .unwrap()
            .starts_with("/api/v4/projects/{id}/pipeline_schedules"));
        assert_eq!(
            ir.params
                .path
                .iter()
                .find(|param| param.name == "id")
                .unwrap()
                .schema,
            serde_json::json!({"oneOf":[{"type":"string"},{"type":"integer"}]})
        );
        assert!(ir.params.body.is_empty());
        if method == "POST" || method == "PUT" {
            assert!(ir
                .params
                .body_schema
                .as_ref()
                .unwrap()
                .pointer("/properties/inputs")
                .is_some());
        }
        if direction == "write" {
            assert_eq!(operation["expose"], false);
        }
    }
    let list = loaded
        .connector
        .operations
        .iter()
        .find(|op| op.id == expected[0].0)
        .unwrap();
    assert_eq!(list.response_schema.as_ref().unwrap()["type"], "object");
    assert_eq!(
        list.response_schema.as_ref().unwrap()["properties"]["inputs"]["type"],
        "object"
    );
    assert!(
        !loaded
            .connector
            .provenance
            .operation_specs
            .contains_key("gitlab-user-get"),
        "legacy inline provenance stays truthful"
    );
}

#[test]
fn source_fidelity_gitlab_preserves_every_selected_vendor_schema() {
    // Independent resolution of these pinned, acyclic source closures. Expected schemas never
    // pass through the production importer or dialect translator.
    fn resolve(root: &Value, value: &Value) -> Value {
        if let Some(reference) = value.get("$ref").and_then(Value::as_str) {
            return resolve(
                root,
                root.pointer(reference.strip_prefix('#').expect("local reference"))
                    .expect("source reference exists"),
            );
        }
        match value {
            Value::Object(fields) => Value::Object(
                fields
                    .iter()
                    .map(|(key, child)| (key.clone(), resolve(root, child)))
                    .collect(),
            ),
            Value::Array(values) => {
                Value::Array(values.iter().map(|value| resolve(root, value)).collect())
            }
            value => value.clone(),
        }
    }
    let root: Value = serde_norway::from_str(
        &std::fs::read_to_string(repo_root().join("specs/gitlab/openapi-19.4.yaml")).unwrap(),
    )
    .unwrap();
    let workspace = Workspace::new(repo_root());
    let provider = catalog_build::discovery::discover(&workspace, Some("gitlab"))
        .unwrap()
        .remove(0);
    let inputs = catalog_build::seam::ProviderInputs::read(&provider).unwrap();
    let loaded = catalog_build::seam::load_full(&inputs).unwrap();
    let mut checked = 0;
    for op in loaded
        .connector
        .operations
        .iter()
        .filter(|op| op.id.starts_with("gitlab-pipeline-schedule-"))
    {
        let source_id = &loaded.connector.provenance.operation_specs[&op.id].operation_id;
        let source = root["paths"]
            .as_object()
            .unwrap()
            .values()
            .flat_map(|path| path.as_object().unwrap().values())
            .find(|value| value["operationId"] == *source_id)
            .unwrap();
        let parameters = source["parameters"].as_array().unwrap();
        assert_eq!(
            op.params.iter().count(),
            parameters.len(),
            "{} complete parameters",
            op.id
        );
        for parameter in parameters {
            let name = parameter["name"].as_str().unwrap();
            let group = match parameter["in"].as_str().unwrap() {
                "path" => &op.params.path,
                "query" => &op.params.query,
                position => panic!("unexpected selected position {position}"),
            };
            let actual = group.iter().find(|param| param.name == name).unwrap();
            assert_eq!(
                actual.schema,
                resolve(&root, &parameter["schema"]),
                "{} {name}",
                op.id
            );
            assert_eq!(
                actual.required,
                parameter["required"].as_bool().unwrap_or(false)
            );
        }
        if let Some(body) = source.get("requestBody") {
            let body = resolve(&root, body);
            assert_eq!(
                op.params.body_schema.as_ref(),
                Some(&body["content"]["application/json"]["schema"])
            );
            assert_eq!(
                op.params.body_required,
                Some(body["required"].as_bool().unwrap_or(false))
            );
        } else {
            assert!(op.params.body_schema.is_none());
        }
        let success = source["responses"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(status, _)| status.starts_with('2'))
            .unwrap()
            .1;
        let expected = success
            .pointer("/content/application~1json/schema")
            .map(|schema| resolve(&root, schema));
        assert_eq!(op.response_schema, expected, "{} source response", op.id);
        checked += 1;
    }
    assert_eq!(checked, 4);
    let diagnostics = loaded.diagnostics();
    for method in ["POST", "PUT"] {
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.location.starts_with(method)
                    && diagnostic.problem.contains("array without items")),
            "source defect must remain visible for {method}"
        );
    }
}

#[test]
fn gitlab_official_source_inventory_accounts_for_every_operation() {
    let document: Value = serde_norway::from_str(
        &std::fs::read_to_string(repo_root().join("specs/gitlab/openapi-19.4.yaml"))
            .expect("full official source"),
    )
    .unwrap();
    let inventory: toml::Value = toml::from_str(
        &std::fs::read_to_string(repo_root().join("specs/gitlab/coverage-19.4.toml"))
            .expect("complete coverage inventory"),
    )
    .unwrap();
    let mut declared = BTreeSet::new();
    for (path, item) in document["paths"].as_object().unwrap() {
        for (method, operation) in item.as_object().unwrap() {
            if [
                "get", "post", "put", "patch", "delete", "head", "options", "trace",
            ]
            .contains(&method.as_str())
            {
                declared.insert((
                    method.to_uppercase(),
                    path.clone(),
                    operation["operationId"].as_str().unwrap().to_owned(),
                ));
            }
        }
    }
    let rows = inventory["operation"].as_array().unwrap();
    let accounted: BTreeSet<_> = rows
        .iter()
        .map(|row| {
            (
                row["method"].as_str().unwrap().to_owned(),
                row["path"].as_str().unwrap().to_owned(),
                row["operation_id"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(declared.len(), 1847);
    assert_eq!(rows.len(), declared.len());
    assert_eq!(accounted, declared);
    assert!(rows
        .iter()
        .all(|row| row["status"].as_str().is_some_and(|s| [
            "catalogued_generated",
            "catalogued_legacy",
            "platform_auth_flow",
            "coverage_gap",
            "importer_gap"
        ]
        .contains(&s))));
    assert_eq!(
        rows.iter()
            .filter(|row| row["status"].as_str() == Some("catalogued_generated"))
            .count(),
        4
    );
    assert!(rows
        .iter()
        .filter(|row| row["status"].as_str().unwrap().ends_with("gap"))
        .all(|row| !row["reason"].as_str().unwrap().is_empty()));
    assert_eq!(
        inventory["authority"].as_str(),
        Some("credentials_and_grants")
    );
}

#[test]
fn adversary_gitlab_pass1_coverage_statuses_match_actual_importer_results() {
    let source =
        std::fs::read_to_string(repo_root().join("specs/gitlab/openapi-19.4.yaml")).unwrap();
    let ingested = connector_spec::openapi::ingest_with_semantics(
        &source,
        connector_spec::RequestSemantics::OpenApi30JsonV1,
    )
    .unwrap();
    let available: BTreeSet<_> = ingested.operation_ids().into_iter().collect();
    let inventory: toml::Value = toml::from_str(
        &std::fs::read_to_string(repo_root().join("specs/gitlab/coverage-19.4.toml")).unwrap(),
    )
    .unwrap();
    let mut counts = BTreeMap::<&str, usize>::new();
    for row in inventory["operation"].as_array().unwrap() {
        let id = row["operation_id"].as_str().unwrap();
        let status = row["status"].as_str().unwrap();
        *counts.entry(status).or_default() += 1;
        match status {
            "coverage_gap" | "catalogued_generated" => {
                assert!(
                    available.contains(id),
                    "{id} claims {status} but ingest refuses it"
                );
            }
            "importer_gap" => {
                assert!(
                    !available.contains(id),
                    "{id} is readable but inventoried as unsupported"
                );
                let location = format!(
                    "{} {}",
                    row["method"].as_str().unwrap(),
                    row["path"].as_str().unwrap()
                );
                assert!(
                    ingested
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.location == location),
                    "{id} has no importer diagnostic"
                );
            }
            "catalogued_legacy" | "platform_auth_flow" => {}
            other => panic!("unreviewed coverage status {other}"),
        }
    }
    assert_eq!(counts.get("catalogued_generated"), Some(&4));
    assert_eq!(counts.get("catalogued_legacy"), Some(&20));
    assert_eq!(counts.get("coverage_gap"), Some(&1510));
    assert_eq!(counts.get("importer_gap"), Some(&313));
    println!("coverage decisions checked for 1847 source operations: {counts:?}");
}

#[test]
fn adversary_gitlab_pass1_translation_preserves_composed_constraint_truth_tables() {
    let source = json!({
        "type":"object", "required":["choice"], "additionalProperties":false,
        "properties":{
            "choice":{"oneOf":[{"type":"string","nullable":true,"enum":["fixed",null]},{"type":"integer","minimum":0,"exclusiveMinimum":true,"maximum":4,"exclusiveMaximum":true}]},
            "both":{"allOf":[{"type":"integer","nullable":true},{"type":"integer","minimum":2}]},
            "literal":{"type":"object","enum":[{"$ref":"literal","example":"kept"}],"default":{"$ref":"literal","example":"kept"}},
            "values":{"type":"array","items":{"anyOf":[{"type":"boolean"},{"type":"string"}]},"minItems":1,"uniqueItems":true}
        }
    });
    let translated = connector_spec::schema_translation::openapi30(&source).unwrap();
    assert_eq!(
        translated["properties"]["literal"],
        source["properties"]["literal"]
    );
    let validator = jsonschema::validator_for(&translated).unwrap();
    let accepted = [
        json!({"choice":null}),
        json!({"choice":"fixed"}),
        json!({"choice":1}),
        json!({"choice":3,"both":2,"values":[false,"false"],"literal":{"$ref":"literal","example":"kept"}}),
    ];
    let refused = [
        json!({}),
        json!({"choice":0}),
        json!({"choice":4}),
        json!({"choice":1.5}),
        json!({"choice":"other"}),
        json!({"choice":false}),
        json!({"choice":1,"both":null}),
        json!({"choice":1,"values":[]}),
        json!({"choice":1,"values":[false,false]}),
        json!({"choice":1,"values":[0]}),
        json!({"choice":1,"invented":true}),
        json!({"choice":1,"literal":{"$ref":"literal"}}),
    ];
    for value in &accepted {
        assert!(
            validator.is_valid(value),
            "lost accepted source value {value}"
        );
    }
    for value in &refused {
        assert!(
            !validator.is_valid(value),
            "broadened source constraint for {value}"
        );
    }
    println!(
        "independent truth table: {} accepted and {} refused values",
        accepted.len(),
        refused.len()
    );
}

#[test]
fn rate_stage2_conditional_history_advice_is_metadata_without_schema_edits() {
    let (workspace, plan) = full_plan();
    let documents = documents(&workspace, &plan);
    let slack = &documents["slack"];
    let history = slack["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|operation| operation["id"] == "slack-conversations-history")
        .unwrap();
    let alternatives = history["conditional_rate_limits"]
        .as_array()
        .expect("history must publish explicit conditional alternatives");
    assert_eq!(alternatives.len(), 3);
    assert_eq!(
        alternatives[0]["rate"],
        json!({"requests":50,"per_seconds":60,"basis":"minimum_allowance"})
    );
    assert_eq!(
        alternatives[1]["rate"],
        json!({"requests":1,"per_seconds":60,"basis":"ceiling"})
    );
    assert!(alternatives[2].get("rate").is_none());
    assert!(alternatives[0]["applies_when"]
        .as_str()
        .unwrap()
        .contains("cursor"));
    assert!(alternatives[1]["applies_when"]
        .as_str()
        .unwrap()
        .contains("2025-05-29"));
    for alternative in alternatives {
        assert!(alternative["source_url"]
            .as_str()
            .unwrap()
            .starts_with("https://docs.slack.dev/"));
    }
    assert!(
        history.get("rate_limit").is_none(),
        "no invented universal tier"
    );
    let committed: Value =
        serde_json::from_str(include_str!("../../../../catalog/slack.catalog.json")).unwrap();
    for operation in slack["operations"].as_array().unwrap() {
        let original = committed["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|before| before["id"] == operation["id"])
            .unwrap();
        assert_eq!(operation["params"], original["params"]);
        assert_eq!(operation["response_schema"], original["response_schema"]);
        assert_eq!(operation["contract"], original["contract"]);
    }
}
// Independent rate declaration/schema comparison; prior invariants are unchanged.
#[test]
fn rate_adversary_canonical_source_urls_match_authoring_reader() {
    let (workspace, plan) = full_plan();
    let mut document = documents(&workspace, &plan)["slack"].clone();
    let index = document["operations"]
        .as_array()
        .unwrap()
        .iter()
        .position(|operation| operation["id"] == "slack-conversations-history")
        .unwrap();
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../catalog/connector-document-v3.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(&schema)
        .unwrap();
    assert!(validator.is_valid(&document));
    let mut mismatches = Vec::new();
    for source in [
        "https://docs.example.test/rate",
        "HTTPS://docs.example.test/rate",
        "https://docs.example.test:65536/rate",
        "https://docs.example.test/a b",
    ] {
        let declaration = &mut document["operations"][index]["conditional_rate_limits"][0];
        declaration["source_url"] = json!(source);
        let authoring =
            serde_json::from_value::<connector_spec::ConditionalRateLimit>(declaration.clone())
                .is_ok();
        let canonical = validator.is_valid(&document);
        if authoring != canonical {
            mismatches.push(format!(
                "{source:?}: authoring={authoring}, schema3={canonical}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "the public authoring reader and generated schema3 disagree:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn rate_repair_source_uri_grammar_matches_authoring_and_both_schemas() {
    let canonical: Value = serde_json::from_str(include_str!(
        "../../../../catalog/connector-document-v3.schema.json"
    ))
    .unwrap();
    let provider: Value = serde_json::from_str(connector_spec::PROVIDER_TOML_JSON_SCHEMA).unwrap();
    let canonical_source =
        &canonical["$defs"]["conditional_rate_limit"]["properties"]["source_url"];
    let provider_source = &provider["$defs"]["conditionalRateLimit"]["properties"]["source_url"];
    let wire: Value = serde_json::from_str(include_str!(
        "../../../../contracts/connector-operation/v0alpha2/connector-operation.schema.json"
    ))
    .unwrap();
    let wire_source = &wire["$defs"]["ConditionalRateLimit"]["properties"]["source_url"];
    assert_eq!(
        canonical_source,
        &connector_spec::ConditionalRateLimit::source_url_schema()
    );
    assert_eq!(
        canonical_source, provider_source,
        "hand-authored provider schema stays synchronized"
    );
    assert_eq!(
        canonical_source, wire_source,
        "all three URL projections declare one profile"
    );
    let canonical_validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(canonical_source)
        .unwrap();
    let provider_validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(provider_source)
        .unwrap();
    let check = |source: &str, expected: bool| {
        let declaration = json!({"applies_when":"a documented category","source_url":source});
        let decoded = serde_json::from_value::<connector_spec::ConditionalRateLimit>(declaration);
        if let Ok(ref decoded) = decoded {
            assert_eq!(decoded.source_url, source, "URI spelling is preserved");
        }
        let authoring = decoded.is_ok();
        let canonical = canonical_validator.is_valid(&json!(source));
        let provider = provider_validator.is_valid(&json!(source));
        assert_eq!(
            (authoring, canonical, provider),
            (expected, expected, expected),
            "{source:?}: authoring/canonical/provider URL grammar"
        );
    };
    for (source, expected) in [
        ("https://docs.example.test", true),
        ("https://DOCS.example.test/rate", true),
        ("https://docs.example.test:443/rate", true),
        ("https://docs.example.test:/rate", true),
        ("https://127.0.0.1:443/rate", true),
        ("https://[2001:db8::1]:443/rate", true),
        ("https://docs.example.test/a%20b?category=a/b?c", true),
        ("https://docs.example.test/%E2%82%AC", true),
        ("https://docs.example.test/rate?email=a@b", true),
        ("HTTPS://docs.example.test/rate", false),
        ("hTtPs://docs.example.test/rate", false),
        ("http://docs.example.test/rate", false),
        ("https:/docs.example.test/rate", false),
        ("https:///rate", false),
        ("https://:443/rate", false),
        ("https://user@docs.example.test/rate", false),
        ("https://user:pass@docs.example.test/rate", false),
        ("https://@docs.example.test/rate", false),
        ("https://docs.example.test/rate#", false),
        ("https://docs.example.test/rate#part", false),
        (" https://docs.example.test/rate", false),
        ("https://docs.example.test/a b", false),
        ("https://docs.example.test/rate?x=a b", false),
        ("https://docs.example.test/rate\n", false),
        ("https://docs.example.test/a\\b", false),
        ("https://docs.example.test/a%", false),
        ("https://docs.example.test/a%2", false),
        ("https://docs.example.test/a%GG", false),
        ("https://docs.example.test/€", false),
        ("https://döcs.example.test/rate", false),
        ("https://[2001:db8::zz]/rate", false),
        ("https://[2001:db8::1/rate", false),
    ] {
        check(source, expected);
    }

    for port in [
        0_u32, 9, 10, 99, 100, 999, 1000, 9999, 10000, 59999, 60000, 64999, 65000, 65499, 65500,
        65529, 65530, 65535, 65536, 99999, 100000,
    ] {
        for spelling in [port.to_string(), format!("000{port}")] {
            check(
                &format!("https://docs.example.test:{spelling}/rate"),
                port <= u32::from(u16::MAX),
            );
        }
    }
    for port in ["-1", "+1", "1.0", "1e2", "18446744073709551616"] {
        check(&format!("https://docs.example.test:{port}/rate"), false);
    }
    for source in [
        "https://%64ocs.example.test/a%2fb",
        "https://[v1.fe80]/rate",
        "https://docs.example.test/a%23b?x=%40",
    ] {
        check(source, true);
    }
    let prefix = "https://docs.example.test/";
    check(
        &format!("{prefix}{}", "a".repeat(2048 - prefix.len())),
        true,
    );
    check(
        &format!("{prefix}{}", "a".repeat(2049 - prefix.len())),
        false,
    );
    check("", false);
}

#[test]
fn rate_final_actual_provider_loading_preserves_uri_and_vendor_contract() {
    let original = std::fs::read_to_string(repo_root().join("providers/slack.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&original).unwrap();
    let cache: Vec<(String, String)> = parsed["spec"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            let path = entry["path"].as_str().unwrap().to_owned();
            let bytes = std::fs::read_to_string(repo_root().join(&path)).unwrap();
            (path, bytes)
        })
        .collect();
    let documents: Vec<_> = cache
        .iter()
        .map(|(path, document)| connector_spec::provider::SpecDocument { path, document })
        .collect();
    let load = |source: &str| connector_spec::provider::load_with_spec("slack", source, &documents);
    let baseline = load(&original).unwrap();
    let baseline: Value =
        serde_json::from_str(&catalog_build::document::render(&baseline.connector).unwrap())
            .unwrap();
    let find = |document: &Value| {
        document["operations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|op| op["id"] == "slack-conversations-history")
            .unwrap()
            .clone()
    };
    let baseline_operation = find(&baseline);
    let source_schema: Value = serde_json::from_str(include_str!(
        "../../../../catalog/connector-document-v3.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(&source_schema)
        .unwrap();
    for (sources, valid) in [
        (
            [
                "https://%41.example.test:00065535/a%2fb?x=%23%40",
                "https://[v1.a:b]:/category?cursor=a/b?c",
                "https://docs.example.test/%00%7F?x=%ff",
            ],
            true,
        ),
        (
            [
                "https://docs.example.test/",
                "https://docs.example.test:00065536/",
                "https://docs.example.test/",
            ],
            false,
        ),
    ] {
        let mut authored = parsed.clone();
        let operation = authored["patch"]["operations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|op| {
                op.get("rename").and_then(toml::Value::as_str)
                    == Some("slack-conversations-history")
            })
            .unwrap();
        for (alternative, source) in operation["conditional_rate_limits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .zip(sources)
        {
            alternative["source_url"] = toml::Value::String(source.into());
        }
        let loaded = load(&toml::to_string(&authored).unwrap());
        assert_eq!(
            loaded.is_ok(),
            valid,
            "real provider loader must enforce the published profile"
        );
        if let Ok(loaded) = loaded {
            let document: Value =
                serde_json::from_str(&catalog_build::document::render(&loaded.connector).unwrap())
                    .unwrap();
            assert!(validator.is_valid(&document));
            let mut operation = find(&document);
            assert_eq!(
                operation["conditional_rate_limits"]
                    .as_array()
                    .unwrap()
                    .len(),
                3
            );
            for (i, source) in sources.into_iter().enumerate() {
                assert_eq!(
                    operation["conditional_rate_limits"][i]["source_url"],
                    source
                );
                operation["conditional_rate_limits"][i]["source_url"] =
                    baseline_operation["conditional_rate_limits"][i]["source_url"].clone();
            }
            assert_eq!(
                operation, baseline_operation,
                "source URI edits must not rewrite any vendor contract or select a category"
            );
        }
    }
}

---
format: aep.planning-md/3
id: specification:wave-20261002g-mcp-contracts
kind: specification
status: draft
title: MCP invocation and local projection contract wave
relations:
- serves: vision:independent-contract-adapters
revision: 3
---
## Selected work and authority

Skill version 0.19.1 — aep:implementing. This is the next wave under the operator's
existing instruction to complete the provider milestones, then deliver MCP and cut
new versions. It follows verified Connectors v0.25.1 at merge
e1cc88088fff3cfeb07fdbce39b9aed586ab48a6, not its earlier queued candidate.
The standing request authorizes scoped unit commits, integration/store commits,
verified bot publication and eventual source release; no new deployment or consumer
pin promotion is included. No approval record is fabricated.

Selected `story:mcp-outbound-invocation-results` and
`story:mcp-inbound-capability-projection` serve
`vision:independent-contract-adapters`. Their declared dependencies are implemented.
Separate read-only scopers returned the scope reports recorded in both story bodies.
Each owns one new contract, one JSON document-case file and one Rust guard; primary
contract paths are cited, case/guard paths inferred. Both are in computed wave 1
following explicit replacement of the old shared-directory write allocations.

Root is the only planning-store writer. Codex exposes named collaboration workers,
not Claude plugin `subagent_type`; workers receive the full `aep:implementor` and
`aep:adversary` reference procedures. This is a host adaptation, not a claim that the
Claude plugin agents ran. Each author gets a separate managed tree, then a fresh
adversary pass. No source writing in primary checkouts.

## Boundaries and exclusions

JSON cases are authored contract checks, not runtime conformance and not ESS
session traces. The inbound YAML collector cannot execute projection semantics;
outbound custom YAML is excluded from synthesis. Neither fact is concealed by
passing document checks. Model validation, existing synthesis and actual metadata
conformance remain separate integration gate results. Root reviews any necessary
new native product type before dependent implementation. All eight unresolved MCP
relation markers remain; caller assignment and outbound process ownership remain
open decisions. Auth, cloud, mutation replay and composition are not selected merely
because their files are disjoint. Mutation/composition consume this wave's settled
semantics. Other providers and unrelated unassessed backlog items below are outside
this explicitly selected MCP contract candidate set, not silently deemed safe.

Sibling MCP partial constructor target is under review in its own repository. Nine
passing target cases do not close the full retrofit, transport/client/server or
Connectors runtime delivery. No sibling dependency pin changes in this wave.

## Worktree, resource and gate record

Integration: cb26c-plan, branch plan/mcp-contracts-20261002, base e1cc88088fff3cfeb07fdbce39b9aed586ab48a6.
Root path: managed tree cb26c-plan; absolute path retained only in local scratch.
- Outbound: planned cb26g-out, unit/mcp-invocation-20261002, same released source base; own target/ and .local/mcp-invocation/ scratch.
- Inbound: planned cb26g-in, unit/mcp-projection-20261002, same released source base; own target/ and .local/mcp-projection/ scratch.
Worker stages: not yet dispatched; unit heads and evidence will be added on handoff.

At preparation, free disk about 15 GiB, reserve 8 GiB, two Cargo jobs per permitted
build. MCP reviewer owns current build window. Document authoring may run in
parallel; Rust builds are serialized with that review and each other until measured
storage permits overlap. No shared targets. Use task-owned TMPDIR. Root's current
Connectors target is about 6.6 GiB; new targets may need several GiB, so monitor and
stop before reserve, never delete another agent's outputs.

Targeted validation checks new guards and existing direction-specific contract
checks, package formatting and Clippy, meaningful mutated-copy negative controls.
Root owns the full connectors-build gate on integration with ESS0.45.0 and AEP0.65.0,
plus appropriate generation/documentation checks. Preserve each exit status and
synthesis/execution counts separately. Existing v0.25.1 gate evidence applies only
to unchanged baseline, not the newly authored source. Fresh adversary cases precede
any reviewer suite run. Review records remain immutable. No story closes until
integrated required checks pass.

## Computed pre-dispatch sets

Command: pinned AEP0.65.0 plan artifact waves --kind story --status draft --format json.
The complete result follows unchanged, including unrelated collisions and unassessed
items; none of those entries is an authorization to implement it.

```json
{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
        {
          "id": "story:catalog-google-live-deck-read",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "docs/catalog-google-drive.md"
            },
            {
              "confidence": "inferred",
              "path": "docs/catalog-google-oauth.md"
            },
            {
              "confidence": "inferred",
              "path": "docs/catalog-google-slides.md"
            }
          ]
        },
        {
          "id": "story:catalog-honours-retry-after",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/catalog/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/catalog-confluence.md"
            },
            {
              "confidence": "cited",
              "path": "docs/catalog-hubspot.md"
            },
            {
              "confidence": "cited",
              "path": "docs/catalog-jira.md"
            },
            {
              "confidence": "cited",
              "path": "docs/local-catalog-provider.md"
            }
          ]
        },
        {
          "id": "story:catalog-slack-reads",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "adapters/catalog/generated/bundles/index.json"
            },
            {
              "confidence": "cited",
              "path": "adapters/catalog/generated/bundles/slack.bundle.json"
            },
            {
              "confidence": "cited",
              "path": "adapters/catalog/providers/slack/operations.json"
            },
            {
              "confidence": "cited",
              "path": "adapters/catalog/tests/slack.rs"
            },
            {
              "confidence": "cited",
              "path": "adapters/slack/upstream/slack-web-api.json"
            },
            {
              "confidence": "cited",
              "path": "docs/catalog-slack.md"
            }
          ]
        },
        {
          "id": "story:gmail-attachment-read-flaky-under-load",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "adapters/catalog/tests/google_calendar_gmail_adversary_pass2.rs"
            }
          ]
        },
        {
          "id": "story:kubernetes-spec-service",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "Cargo.lock"
            },
            {
              "confidence": "cited",
              "path": "Cargo.toml"
            },
            {
              "confidence": "cited",
              "path": "README.md"
            },
            {
              "confidence": "cited",
              "path": "adapters/kubernetes"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-build"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-conformance"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-spec"
            },
            {
              "confidence": "cited",
              "path": "docs/development.md"
            },
            {
              "confidence": "cited",
              "path": "docs/local-kubernetes-cli.md"
            },
            {
              "confidence": "cited",
              "path": "ess/domains/declarations.yaml"
            },
            {
              "confidence": "cited",
              "path": "examples"
            },
            {
              "confidence": "cited",
              "path": "spec-kinds"
            }
          ]
        },
        {
          "id": "story:mcp-inbound-capability-projection",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/server/v1alpha1/projection-cases.json"
            },
            {
              "confidence": "cited",
              "path": "adapters/mcp/contracts/server/v1alpha1/projection.md"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-build/tests/mcp_inbound_capability_projection.rs"
            }
          ]
        },
        {
          "id": "story:mcp-inbound-cloud-profile",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/server/v1alpha1/cloud-profile.md"
            },
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/server/v1alpha1/fixtures"
            }
          ]
        },
        {
          "id": "story:mcp-outbound-auth-lifecycle",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/client/v1alpha1/auth.md"
            },
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/client/v1alpha1/scenarios"
            }
          ]
        },
        {
          "id": "story:mcp-outbound-invocation-results",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/client/v1alpha1/invocation-cases.json"
            },
            {
              "confidence": "cited",
              "path": "adapters/mcp/contracts/client/v1alpha1/invocation.md"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-build/tests/mcp_outbound_invocation_results.rs"
            }
          ]
        },
        {
          "id": "story:oauth-browser-empty-connection-fixture",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/connectors-host/src/local/oauth_adversary_tests.rs"
            }
          ]
        },
        {
          "id": "story:retire-unmerged-remote-branches",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "refs/heads/* on beyond10x/connectors"
            }
          ]
        }
      ]
    },
    {
      "wave": 2,
      "artifacts": [
        {
          "id": "story:catalog-validates-enum-and-date-time",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "CHANGELOG.md"
            },
            {
              "confidence": "inferred",
              "path": "adapters/catalog/generated/bundles/confluence.bundle.json"
            },
            {
              "confidence": "inferred",
              "path": "adapters/catalog/generated/bundles/gitlab.bundle.json"
            },
            {
              "confidence": "inferred",
              "path": "adapters/catalog/generated/bundles/google-calendar.bundle.json"
            },
            {
              "confidence": "inferred",
              "path": "adapters/catalog/generated/bundles/google-drive.bundle.json"
            },
            {
              "confidence": "inferred",
              "path": "adapters/catalog/generated/bundles/google-gmail.bundle.json"
            },
            {
              "confidence": "inferred",
              "path": "adapters/catalog/generated/bundles/google-slides.bundle.json"
            },
            {
              "confidence": "inferred",
              "path": "adapters/catalog/generated/bundles/hubspot.bundle.json"
            },
            {
              "confidence": "inferred",
              "path": "adapters/catalog/generated/bundles/index.json"
            },
            {
              "confidence": "inferred",
              "path": "adapters/catalog/generated/bundles/jira.bundle.json"
            },
            {
              "confidence": "cited",
              "path": "adapters/catalog/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "adapters/catalog/tests/gitlab_commit_reads_adversary.rs"
            },
            {
              "confidence": "inferred",
              "path": "adapters/catalog/tests/parameter_types.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-catalog/src/authored.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-catalog/src/inventory.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-catalog/tests/template.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/local-catalog-provider.md"
            }
          ]
        },
        {
          "id": "story:live-reads-jira-confluence-hubspot",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "docs/catalog-confluence.md"
            },
            {
              "confidence": "cited",
              "path": "docs/catalog-hubspot.md"
            },
            {
              "confidence": "cited",
              "path": "docs/catalog-jira.md"
            }
          ]
        },
        {
          "id": "story:mcp-composition-provenance",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/composition/v1alpha1/scenarios"
            },
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/composition/v1alpha1/semantics.md"
            }
          ]
        },
        {
          "id": "story:mcp-inbound-mutation-replay",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/server/v1alpha1/mutations.md"
            },
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/server/v1alpha1/scenarios"
            }
          ]
        },
        {
          "id": "story:spec-models-operation-admission",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "apps/connectors-cli-contract"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-build/src/metadata_conformance.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-build/src/metadata_entities.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-host/src/local/metadata/entity-runtime-definitions.json"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-host/src/local/metadata/er.rs"
            },
            {
              "confidence": "cited",
              "path": "ess/domains/declarations.yaml"
            }
          ]
        }
      ]
    },
    {
      "wave": 3,
      "artifacts": [
        {
          "id": "story:catalog-zendesk-reads",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "adapters/catalog/generated/bundles/index.json"
            },
            {
              "confidence": "cited",
              "path": "adapters/catalog/generated/bundles/zendesk.bundle.json"
            },
            {
              "confidence": "cited",
              "path": "adapters/catalog/providers/zendesk/operations.json"
            },
            {
              "confidence": "cited",
              "path": "adapters/catalog/tests/zendesk.rs"
            },
            {
              "confidence": "cited",
              "path": "adapters/zendesk/upstream/zendesk-support.json"
            },
            {
              "confidence": "cited",
              "path": "docs/catalog-zendesk.md"
            }
          ]
        },
        {
          "id": "story:configuration-change-error",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "apps/connectors-cli-contract/binding.json"
            },
            {
              "confidence": "inferred",
              "path": "apps/connectors/src/local.rs"
            },
            {
              "confidence": "inferred",
              "path": "apps/connectors/src/local/connections.rs"
            },
            {
              "confidence": "inferred",
              "path": "contracts/cli/v1alpha1/scenarios.md"
            },
            {
              "confidence": "inferred",
              "path": "contracts/cli/v1alpha1/semantics.md"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-host/src/local/owner.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-host/src/local/registry.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-host/src/local/registry/lifecycle.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-host/src/local/registry/tests.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/local-catalog-provider.md"
            },
            {
              "confidence": "inferred",
              "path": "ess/domains/cli.yaml"
            }
          ]
        },
        {
          "id": "story:toolchain-pins-newest-release-20261001",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "Cargo.lock"
            },
            {
              "confidence": "cited",
              "path": "Cargo.toml"
            },
            {
              "confidence": "inferred",
              "path": "apps/connectors-cli-contract"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-build/aep-toolchain.json"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-spec/toolchain.json"
            },
            {
              "confidence": "cited",
              "path": "docs/development.md"
            },
            {
              "confidence": "inferred",
              "path": "ess"
            }
          ]
        }
      ]
    },
    {
      "wave": 4,
      "artifacts": [
        {
          "id": "story:expired-evidence-invoke-advises-revalidate",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "apps/connectors-cli-contract/binding.json"
            },
            {
              "confidence": "cited",
              "path": "apps/connectors/src/local.rs"
            },
            {
              "confidence": "cited",
              "path": "apps/connectors/src/local/connections.rs"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/evidence/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/cli/v1alpha1/scenarios.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/cli/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-host/src/local/owner.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-host/src/local/owner/mutation/recovery.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-host/src/local/registry/observation.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-host/src/local/registry/tests.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-host/src/local/registry/use_and_retirement.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/local-catalog-provider.md"
            },
            {
              "confidence": "inferred",
              "path": "ess/domains/cli.yaml"
            }
          ]
        }
      ]
    },
    {
      "wave": 5,
      "artifacts": [
        {
          "id": "story:service-failure-carries-upstream-reason",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "adapters/catalog/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "adapters/catalog/tests/local_runtime/cli_journey.rs"
            },
            {
              "confidence": "cited",
              "path": "apps/connectors-cli-contract/binding.json"
            },
            {
              "confidence": "cited",
              "path": "apps/connectors/src/local.rs"
            },
            {
              "confidence": "inferred",
              "path": "apps/connectors/src/local/operations.rs"
            },
            {
              "confidence": "cited",
              "path": "contracts/cli/v1alpha1/private-adapter.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/cli/v1alpha1/semantics.md"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-core/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-host/src/local/owner.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-host/src/local/owner/mutation.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-host/src/local/runtime"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-host/src/local/runtime.rs"
            },
            {
              "confidence": "cited",
              "path": "ess/domains/cli.yaml"
            }
          ]
        }
      ]
    },
    {
      "wave": 6,
      "artifacts": [
        {
          "id": "story:mcp-cli-journey-discovery-contract",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.md"
            },
            {
              "confidence": "inferred",
              "path": "apps/connectors/spec/compatibility.json"
            },
            {
              "confidence": "inferred",
              "path": "contracts/cli/v1alpha1/semantics.md"
            },
            {
              "confidence": "inferred",
              "path": "docs/local-mcp-cli.md"
            }
          ]
        }
      ]
    }
  ],
  "collisions": [
    {
      "a": "story:catalog-honours-retry-after",
      "b": "story:catalog-validates-enum-and-date-time",
      "path": "adapters/catalog/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:catalog-honours-retry-after",
      "b": "story:catalog-validates-enum-and-date-time",
      "path": "docs/local-catalog-provider.md",
      "confidence": "cited"
    },
    {
      "a": "story:catalog-honours-retry-after",
      "b": "story:configuration-change-error",
      "path": "docs/local-catalog-provider.md",
      "confidence": "cited"
    },
    {
      "a": "story:catalog-honours-retry-after",
      "b": "story:expired-evidence-invoke-advises-revalidate",
      "path": "docs/local-catalog-provider.md",
      "confidence": "cited"
    },
    {
      "a": "story:catalog-honours-retry-after",
      "b": "story:live-reads-jira-confluence-hubspot",
      "path": "docs/catalog-confluence.md",
      "confidence": "cited"
    },
    {
      "a": "story:catalog-honours-retry-after",
      "b": "story:live-reads-jira-confluence-hubspot",
      "path": "docs/catalog-hubspot.md",
      "confidence": "cited"
    },
    {
      "a": "story:catalog-honours-retry-after",
      "b": "story:live-reads-jira-confluence-hubspot",
      "path": "docs/catalog-jira.md",
      "confidence": "cited"
    },
    {
      "a": "story:catalog-honours-retry-after",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "adapters/catalog/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:catalog-slack-reads",
      "b": "story:catalog-validates-enum-and-date-time",
      "path": "adapters/catalog/generated/bundles/index.json",
      "confidence": "inferred"
    },
    {
      "a": "story:catalog-slack-reads",
      "b": "story:catalog-zendesk-reads",
      "path": "adapters/catalog/generated/bundles/index.json",
      "confidence": "cited"
    },
    {
      "a": "story:catalog-validates-enum-and-date-time",
      "b": "story:catalog-zendesk-reads",
      "path": "adapters/catalog/generated/bundles/index.json",
      "confidence": "inferred"
    },
    {
      "a": "story:catalog-validates-enum-and-date-time",
      "b": "story:configuration-change-error",
      "path": "docs/local-catalog-provider.md",
      "confidence": "cited"
    },
    {
      "a": "story:catalog-validates-enum-and-date-time",
      "b": "story:expired-evidence-invoke-advises-revalidate",
      "path": "docs/local-catalog-provider.md",
      "confidence": "cited"
    },
    {
      "a": "story:catalog-validates-enum-and-date-time",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "adapters/catalog/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:expired-evidence-invoke-advises-revalidate",
      "path": "apps/connectors-cli-contract/binding.json",
      "confidence": "inferred"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:expired-evidence-invoke-advises-revalidate",
      "path": "apps/connectors/src/local.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:expired-evidence-invoke-advises-revalidate",
      "path": "apps/connectors/src/local/connections.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:expired-evidence-invoke-advises-revalidate",
      "path": "contracts/cli/v1alpha1/scenarios.md",
      "confidence": "inferred"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:expired-evidence-invoke-advises-revalidate",
      "path": "contracts/cli/v1alpha1/semantics.md",
      "confidence": "inferred"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:expired-evidence-invoke-advises-revalidate",
      "path": "crates/connectors-host/src/local/owner.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:expired-evidence-invoke-advises-revalidate",
      "path": "crates/connectors-host/src/local/registry/tests.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:expired-evidence-invoke-advises-revalidate",
      "path": "docs/local-catalog-provider.md",
      "confidence": "cited"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:expired-evidence-invoke-advises-revalidate",
      "path": "ess/domains/cli.yaml",
      "confidence": "inferred"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:mcp-cli-journey-discovery-contract",
      "path": "contracts/cli/v1alpha1/semantics.md",
      "confidence": "inferred"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "apps/connectors-cli-contract/binding.json",
      "confidence": "inferred"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "apps/connectors/src/local.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "contracts/cli/v1alpha1/semantics.md",
      "confidence": "inferred"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "crates/connectors-host/src/local/owner.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:configuration-change-error",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "ess/domains/cli.yaml",
      "confidence": "inferred"
    },
    {
      "a": "story:expired-evidence-invoke-advises-revalidate",
      "b": "story:mcp-cli-journey-discovery-contract",
      "path": "contracts/cli/v1alpha1/semantics.md",
      "confidence": "inferred"
    },
    {
      "a": "story:expired-evidence-invoke-advises-revalidate",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "apps/connectors-cli-contract/binding.json",
      "confidence": "inferred"
    },
    {
      "a": "story:expired-evidence-invoke-advises-revalidate",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "apps/connectors/src/local.rs",
      "confidence": "cited"
    },
    {
      "a": "story:expired-evidence-invoke-advises-revalidate",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "contracts/cli/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:expired-evidence-invoke-advises-revalidate",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "crates/connectors-host/src/local/owner.rs",
      "confidence": "cited"
    },
    {
      "a": "story:expired-evidence-invoke-advises-revalidate",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "ess/domains/cli.yaml",
      "confidence": "inferred"
    },
    {
      "a": "story:kubernetes-spec-service",
      "b": "story:spec-models-operation-admission",
      "path": "ess/domains/declarations.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:kubernetes-spec-service",
      "b": "story:toolchain-pins-newest-release-20261001",
      "path": "Cargo.lock",
      "confidence": "cited"
    },
    {
      "a": "story:kubernetes-spec-service",
      "b": "story:toolchain-pins-newest-release-20261001",
      "path": "Cargo.toml",
      "confidence": "cited"
    },
    {
      "a": "story:kubernetes-spec-service",
      "b": "story:toolchain-pins-newest-release-20261001",
      "path": "docs/development.md",
      "confidence": "cited"
    },
    {
      "a": "story:mcp-cli-journey-discovery-contract",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "contracts/cli/v1alpha1/semantics.md",
      "confidence": "inferred"
    },
    {
      "a": "story:spec-models-operation-admission",
      "b": "story:toolchain-pins-newest-release-20261001",
      "path": "apps/connectors-cli-contract",
      "confidence": "inferred"
    }
  ],
  "unassessed": [
    "story:archive-closed-review-results",
    "story:audit-anchor-co-presence",
    "story:catalog-grafana-reads",
    "story:catalog-guard-optional-preflight-values",
    "story:catalog-post-reads",
    "story:ess-format-paging-views",
    "story:ess3-format-adoption",
    "story:guarded-write-credential-refusal-says-repair",
    "story:host-gate-load-sensitivity-w3",
    "story:http-empty-query-no-trailing-question-mark",
    "story:identity-views-for-witnessing",
    "story:multi-tenant-principal-assignment",
    "story:owner-recovers-after-failed-acquisition",
    "story:owner-refuses-buildless-callers",
    "story:session-connection-binding"
  ],
  "cycles": []
}
```

## Dispatch and coordinator reconciliation update

Both worker trees were created from opening commit a2955675cb70b5811589ce86b40be95427ead888:
cb26g-out/unit/mcp-invocation-20261002 and cb26g-in/unit/mcp-projection-20261002.
Own target/ directories and assigned scratch roots remain as above. Full briefs were
written to coordinator-local scratch after dispatch; the original assignments were
not changed. This timing is recorded rather than claiming the brief files preceded
the dispatch. Source opening commit passed bot hooks after removing an actual
personal absolute path from the wave page; no hook bypass occurred.

Outbound first RED: missing contract,0passed4failed; then28targetedtests passed,
including6new guards, with40authored document cases, fmt/Clippy0. Its modern
CacheableResult check exposed an additional omitted required metadata rule,1RED
then corrected andGREEN. Handoff/report pending. Inbound begins its first RED after
outbound released the serialized worker build window. Root MCP gate work used its
already-built xtask target for small tests while authors' large builds stayed serial;
free space remained above8GiB. Do not describe this as all Cargo being exclusive.

Root added story:mcp-framing-refusal-preserves-uncertainty (active4) before editing
the existing framing paragraph and its exact unreadable-answer scenario. The
outbound worker's prose patch omitted the same contradiction in the scenario;
coordinator inspection expanded the correction to both owned statements. These
are separate exact write paths, not an expansion of either worker's authority.
Fresh outbound review must inspect both corrected owners alongside invocation.

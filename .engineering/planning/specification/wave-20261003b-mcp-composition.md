---
format: aep.planning-md/3
id: specification:wave-20261003b-mcp-composition
kind: specification
status: draft
title: 'Second authorized MCP wave: composition and CLI selected intent'
relations:
- informed_by: specification:wave-20261003a-mcp-auth-replay
- serves: vision:independent-contract-adapters
revision: 3
---
## Second authorized wave: selection pending predecessor completion

AEP skill version0.19.1. The operator authorized two waves in
approval-record:mcp-next-two-waves-20261003; no second approval question is needed.
This page preserves preparation, not a claim that prerequisites already finished.

The selected story candidate is story:mcp-composition-provenance. Its exact three
contract/case/guard paths are recorded in the story, with cited primary and inferred
new case/test paths. It now depends on reviewed auth and mutation contracts as well
as invocation/projection. Re-read their final bodies and recompute draft waves after
the first wave closes; include the complete returned lists here before dispatch.

A separate parallel companion is task:mcp-cli-selected-intent-contract, under the
still-open story:mcp-cli-journey-discovery-contract. AEP refused task-level scope
because a task inherits its parent's surfaces. The static inventory path is recorded
on that parent; the task owns only its exact three authored documents. It is not
an implemented working CLI journey and is not counted as a second terminal story.
The composition wave may have N=1; the companion task and independent reviewer use
separate agents and checkouts under the same operator authorization.

## Boundaries and checks

Composition covers both inbound and outbound hops for the configured single local
owner. It reuses actual auth/projection/replay owners, preserves safe provenance,
remaining deadlines and wrapper overhead, and rejects credential forwarding,
account fallback, metadata authority and invented remote audit references. The old
mediated-HTTP analogy was corrected: it supplies boundary principles, not MCP's
runtime relation/transport semantics. No unresolved native relation is silently
chosen. Meaningful document guard negative controls plus independent review and
one full integration gate remain required; no runtime conformance follows.

The CLI companion selects intent, JSON inventory and human/static agreement. It
leaves mcp=deferred, parser, generated binding and runtime untouched. No new mirrored
prose test is required. Validate JSON, links, syntax correspondence, source checks
and explicit unavailable steps. Parent acceptance waits for actual runtime.

Root alone owns planning/integration/publication. Existing bot commit/push/PR/merge
authorization remains. No source release, deployment, consumer pin or product
ownership choice is implied by these contract-only units.

## Planned ownership; fill exact base before dispatch

- Composition: managed cb26j-compose, branch unit/mcp-composition-20261003,
  its own target, scratch.local/mcp-two-waves/composition.
- CLI companion: managed cb26j-cli, branch unit/mcp-cli-intent-20261003,
  no Cargo target needed, scratch.local/mcp-two-waves/cli-intent.
Durable briefs and source base will be written before authors start. Existingroot
integration cache may be reused only byroot; worker targets nevershared.

## Preflight to refresh

Confirm first-wave source integrated, review findings recorded, exact gates passed,
wanted source published and completedworker cleanup. Refresh free disk against8GiB
floor; measured Rust contractworker targetabout1.6GiB. Twojobs/sccache/defaulttarget.
Do not clean unrelated trees/processes. Model/toolpins unchanged.

## Selected second wave after predecessor verification

AEP skill version0.19.1. Existing approval-record:mcp-next-two-waves-20261003 authorizes dispatch; no new approval required. Full predecessor gate passed with1245 tests, zero failures,65 ignored; auth/replay stories implemented. Computed draft wave1 includes both composition and the CLI parent; only story:mcp-composition-provenance is selected as a terminal story (N=1). Separate task:mcp-cli-selected-intent-contract runs in parallel under the still-open CLI parent. Its static documents do not satisfy the working CLI journey. Unrelated unassessed backlog is not selected and is not claimed parallel-safe. Exact typed scopes do not overlap.

One implementor per selected unit and one fresh adversary per result, at most two attacks. Codex collaboration agents execute the installed AEP role procedures; no plugin-specific subagent_type selector is exposed. Root alone writes AEP and integrates. Authorization covers the two bounded units' source, integration/store commits and bot publication under existing authority, not runtime completion, deployment or consumer pins.

Managed cb26j-compose / unit/mcp-composition-20261003 owns three composition paths, scratch.local/mcp-two-waves/composition, isolated defaulttarget. Managed cb26j-cli / unit/mcp-cli-intent-20261003 owns three static CLI documents, scratch.local/mcp-two-waves/cli-intent, no Cargo build. Both start at the verified published first-wave candidate; its exact SHA and resolved paths enter durable briefs before dispatch. Current available storage11GiB,8GiB reserve; only composition needs an estimated1.6GiB worker target,2jobs/sccache. Root retains its cache; no live providers. Final first-wave archives and signed publication precede old-worker retirement.

## Complete computed draft lists


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
          "id": "story:mcp-cli-journey-discovery-contract",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/protocol/v1alpha1/discovery-contract.json"
            },
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
        },
        {
          "id": "story:mcp-composition-provenance",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/composition/v1alpha1/composition-cases.json"
            },
            {
              "confidence": "cited",
              "path": "adapters/mcp/contracts/composition/v1alpha1/semantics.md"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-build/tests/mcp_composition_provenance.rs"
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

## Second-wave integration and review — 2026-10-03

First-wave PR82 merged by the bot App at aff84c56c06d7277685f80d7453c67b0e36c7fe6 with the exact candidate tree, after all required checks passed. Composition and selected CLI intent were authored in separate managed trees from published0f648160. CLI independent review found nothing; static inventories match8genericpaths and preserve unavailable MCP runtime and the open parent journey.

Composition author recorded missing-contract RED, then58affectedtests and formatting/Clippy green. Account quota stopped its final agent turn and prevented the fresh independent composition reviewer. Coordinator therefore performed a disclosed separate adversary pass, not an independent-agent claim. New regression found missing request/response IDs silently accepted by nullable indexing. The recorded first review is immutable; correction now validates typed present correlation and exclusive successful envelopes. Retained regression across2revisions/3families passes; affected suite59passed0failed; formatting/Clippy0. No contract/JSONsource changes or weakened assertion. This is document-fixture validation, not MCP wire/runtime conformance.

Integration uses the six exact authored source paths plus CLI review and composition review/outcomes. Full pinned repository/MSRV gate runs on the integrated tree before lifecycle closure. No release/runtime/parent-CLI completion is asserted. Existing native unresolved relations and operator blockers remain. Worker usage limits affect further delegated work; primary coordinator continues useful local work. All author/review/correction logs and source hashes retained privately.

## Integrated second-wave result — 2026-10-03

Composition and the CLI selected-intent child are implemented as authored contracts/documents. The parent working CLI journey remains draft/open. The full integrated repository gate with --msrv exited0:1256passed,0failed,65ignored,153runner summaries; formatting,Clippy,library boundaries,MSRV,ESS/generation and AEP checks passed. Gate log SHA2566fe8a160506669da234272ec6447e683f45a4868366ce41f6ed6337a6ecdb793. Existing synthesis remains498scenarios(43authored),21refusals; local metadata suite289, conformanceinconclusive. No new MCP runtime coverage is inferred from these counts.

Composition contains40literal cases and11Rust guard tests; the affected six lanes total59passing tests. The missing-correlation finding has its retained regression and fixed review outcome. CLI independent review found no issue in its source/grammar/link/error comparison. Delegated composition review was unavailable at account usage limit; coordinator adversary and correction passes were separately recorded and are not an independent-agent claim.

The first integration attempt stopped for disk exhaustion. The resumed cache-wrapped attempt stayed live but showed measured wrapper delay; a tiny direct compiler probe took15-21ms while the wrapper timed out at10s. That owned attempt was explicitly terminated, then the same gate ran successfully with only the invocation-local wrapper disabled. Neither shared compiler configuration nor another process was changed. All source/review evidence remains retained.

The two worker trees are archived and finished, but exact-ID GC awaits the operator approval requested by the explicitly invoked cleanup skill. cb26k-publish remains the active integration/publication tree. This wave does not deliver the working MCP runtime or cut a release.

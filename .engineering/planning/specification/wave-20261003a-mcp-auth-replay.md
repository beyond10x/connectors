---
format: aep.planning-md/3
id: specification:wave-20261003a-mcp-auth-replay
kind: specification
status: draft
title: 'First authorized MCP wave: outbound auth and inbound replay contracts'
relations:
- informed_by: specification:wave-20261002g-mcp-contracts
- serves: vision:independent-contract-adapters
revision: 3
---
## Authorization and selected units

AEP skill version0.19.1, wave mode. approval-record:mcp-next-two-waves-20261003
contains the operator's explicit instruction to dispatch two waves using multiple
agents and clean up. This is the first of those two, after completion of the current
invocation/projection wave. Root is sole AEP writer. Codex collaboration agents run
the aep:story-scoper, aep:implementor and aep:adversary reference procedures; this
host has no plugin-specific subagent_type selector. Model overrides are not selected.

- story:mcp-outbound-auth-lifecycle: authored HTTP credential lifecycle, serving
  vision:independent-contract-adapters. One cited contract plus two inferred new
  JSON/guard paths; existing unresolved native ownership remains explicit.
- story:mcp-inbound-mutation-replay: authored local single-owner mutation/replay
  mapping, serving the same vision. One cited contract plus two inferred new
  JSON/guard paths. Requires the integrated projection story implemented first.

Both units are authored specification checks, not runtime MCP interoperability,
Secret Service persistence, no-duplicate-effect execution or end-to-end delivery.
The separate sibling HTTP native-model review prepares actual runtime work.

Each unit gets one managed tree and one independent review, with at most two full
attacks. Author base remains pending until the current integration gate passes and
the opening commit exists. No candidate receives a dirty partial source snapshot.
Authorization covers two unit commits, integration commits, closing store commit,
merge to base and existing bot publication authorization. No additional product
relations or deployment are selected.

## Preflight and ownership

Primary checkouts remain untouched. The coordinator reuses its owned managed
cb26c-plan tree and existing build cache; current uncommitted changes belong to
this authorized integration and planning work, retained in private preflight status.
Prior worker cb26g-in/out are frozen, all leases released, reports read/copied and
recovery archives verified. Their exact Cargo targets were cleaned:1.7GiB each.
Finish/GC follows publication of wanted source. Integration root is retained because
it owns the next work. No unrelated tree or process is a cleanup candidate.

Measured worker build cost about1.6GiB, root fullgate already has its own target.
Available disk14GiB at this preflight; floor8GiB, recheck before every new build.
Two implementors maximum, each2Cargo jobs with /usr/bin/sccache and isolated target.
Host concurrency4 including coordinator; no separate token budget stated. Builds
may be serialized to preserve reserve. Sourcepins ESS0.45/AEP0.65 remain fixed.

Managed records planned:
- cb26i-auth, branch unit/mcp-auth-20261003, target within that tree, scratch
  .local/mcp-two-waves/auth; unit story:mcp-outbound-auth-lifecycle.
- cb26i-replay, branch unit/mcp-replay-20261003, target within that tree, scratch
  .local/mcp-two-waves/replay; unit story:mcp-inbound-mutation-replay.
The private wave directory holds exact resolved paths, leases and immutable briefs.
Their source base and stage are updated after creation, never inferred from age.

## Scope and deferrals

Read-only scopers confirmed exact new files; each story now records typed scopes.
The replay story corrected a false request-id-to-business-key inference; terminal
Indeterminate alone never supplied the entire dispatch fence. Auth remains an
obligation until its separate native custody/Connection mapping is designed.

Second wave candidates are composition plus CLI/discovery, separately re-read and
recomputed after these two units close. Cloud caller isolation, outbound stdio child
ownership, external execution and unrelated unassessed backlog remain excluded.
The computed broad-store lists below are retained verbatim; they are candidates,
not authorization for every returned artifact. No unassessed item is selected here.

## Computed scope lists before dispatch


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
          "id": "story:mcp-inbound-mutation-replay",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/server/v1alpha1/mutation-cases.json"
            },
            {
              "confidence": "cited",
              "path": "adapters/mcp/contracts/server/v1alpha1/mutations.md"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-build/tests/mcp_inbound_mutation_replay.rs"
            }
          ]
        },
        {
          "id": "story:mcp-outbound-auth-lifecycle",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/client/v1alpha1/auth-cases.json"
            },
            {
              "confidence": "cited",
              "path": "adapters/mcp/contracts/client/v1alpha1/auth.md"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-build/tests/mcp_outbound_auth_lifecycle.rs"
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

## Dispatch — 2026-10-03

Opening source/plan commit b360aaaa771ad9a31cab46ff4249015b74a4d50f passed coordinated
hooks and common security (166commits scanned); bot author and committer verified.
Published as plan/mcp-contracts-20261002, PR80. The prerequisite contract fullgate
passed1227 tests; its independent review history is retained. Both selected stories
are now active and their actual dependencies implemented. No source is borrowed
from a dirty/uncommitted candidate.

- cb26i-auth / unit/mcp-auth-20261003 / baseb360aaaa / implementor mcp_invocation;
  leasecodex-cb26i-auth; scratch.local/mcp-two-waves/auth; isolatedtarget.
- cb26i-replay / unit/mcp-replay-20261003 / baseb360aaaa / implementor mcp_projection;
  leasecodex-cb26i-replay; scratch.local/mcp-two-waves/replay; isolatedtarget.

Durable per-unit briefs existed before implementation dispatch. Both own exactly
three files recorded on their stories. Initially auth compiled alone; after exact
cleanup of completed root MSRV/default targets, available disk17GiB permitted both
scoped builds, each2jobs withsccache. Floor8GiB remains checked. Rootgate output
and source hashes were retained before deleting any build output. Root retained its
active source tree; no shared builddirectory or unrelated process was touched.

Previous workers cb26g-in and cb26g-out: reports copied, wantedsourcepublished,
archivesverified, finished, exactGCdryruneligible and applyremoved both. Archives
remain. Managed repolist verifies neither oldpath remains. This cleanup began
before newtree creation and finished while newauthors prepared their scoped work.

## Prerequisites published; first independent attacks — 2026-10-03

Connectors PR80 merged through b10x-bot[bot] at fcef8b18a58c16e8af07c0a04aece5e8415164d2 after required repository, planning and shared security checks passed. Its tree 71d8d7d8ce348a6a098128662d555940c194d245 equals reviewed candidate b360aaaa771ad9a31cab46ff4249015b74a4d50f. The App-only branch authority and exact App merge action were verified. The coordinator fast-forwarded, preserving owned uncommitted next-wave planning, and selected plan/mcp-auth-replay-20261003.

Both authors froze source with targeted tests/fmt/Clippy passing. Auth ran25 checks and fixed a stale restart fence after an observed RED. Replay ran37 checks. Independent adversary review-results mcp-auth-20261003 and mcp-replay-20261003 preserve first attacks verbatim before author corrections. Auth accepted contradictory duplicate Markdown rows (25passed/1failed after attack); replay allowed required semantic cases to be replaced by early-denial premises and a recovery observer to become live-host (38passed/2failed). These document-guard findings returned to authors with regressions retained; no runtime defect or runtime conformance inferred.

Old workers cb26g-in/cb26g-out were archived, finished and removed through exact reviewed managed GC after wanted source publication. Archives retained under worktree/archives/connectors. The current auth/replay workers remain active for correction/review. Available space24GiB at 23:10UTC, with8GiB reserve. Second-wave selected scope remains prepared, not yet dispatched.

## Verified integration — 2026-10-03

Final integrated fullgate --msrv exited0: 1245 tests passed, zero failed,65 ignored across152 runner summaries. Formatting, Clippy, provider/generic CLI boundaries, Rust1.88 pure libraries and Rust1.91 whole workspace, ESS/native models and conformance, and AEP validation passed. Log SHA256ddb71066b5ffe80d22aea3e8ff7e962314e50eb3c0181a8c21d85cdf849938ce. Existing planning historical review warnings remain; no new validation error. Both selected stories moved active→implemented with recorded test_result and verification. Actual MCP runtime and mutation advertisement remain unavailable.

Auth first/second attacks found1/1 defects, AEP carried0/new1/resolved1; both were corrected, regressions retained and coordinator verified the final diff. Replay first/second attacks found2/0, carried0/new0/resolved2; all corrected,41 targeted checks passed. Auth28 targeted passed. No third campaign. All source reports and122 ignored evidence files were copied and byte-verified before worker targets were cleaned (3.135GiB). Both exact worker archives are verified; finish/GC waits for wanted source publication.

The next wave is preauthorized. Independent author preparation is complete and scopes were recomputed after these two stories closed. Its authors may begin at the verified published candidate while CI runs; actual PR merges still require all exact-head checks. The root integration cache is retained. The completed sibling MCP model target was separately cleaned (1.5GiB) after no users were found, keeping the storage reserve; its logs and published source remain.

## Publication privacy correction

The common commit gate refused two personal-paths findings in raw compiler-output lines of the unpublished auth review records. Nothing was committed or published. Because review records are immutable, the original unpublished store and raw reports remain in private recovery; no immutable record was rewritten. A fresh managed publication candidate, cb26k-publish, was created from the same published base fcef8b18a58c16e8af07c0a04aece5e8415164d2. The single coordinator replayed every required planning mutation through AEP, with existing evidence observation times preserved.

The two public report copies explicitly normalize only the private compiler checkout root to <worktree>. Findings, test names, assertions, counts and exit statuses are unchanged. Original report digests remain the raw evidence identities cited earlier. Public copy digests: first768985a02ca8451a844c2f2141f53a4fa67c4189037fd18491fe44615e927241; followupa075b57736f7193b4fdb818d8127da5ef7af5bccbbed13d733e16935f8963189. Replay reports already used portable paths and are unchanged.

All six implementation source files were copied and byte-compared equal to the final integrated gate inputs; every other executable/specification source remains at the same base. Fullgate evidence is reused at those identical source bytes, while AEP validation and the same common gate run on the rebuilt publication candidate. No policy or hook changed and no refusal was bypassed. The new integration owner is cb26k-publish; the old cb26c-plan becomes private archive recovery after wanted source is published. Subsequent author briefs name the new owner and exact published base.

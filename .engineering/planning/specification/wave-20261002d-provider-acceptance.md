---
format: aep.planning-md/3
id: specification:wave-20261002d-provider-acceptance
kind: specification
status: draft
title: 'Wave 20261002d: catalog journeys and real-provider acceptance'
relations:
- informed_by: specification:milestone-acceleration-20261002
- informed_by: story:catalog-cli-journeys
- informed_by: story:kubernetes-real-read-acceptance
- informed_by: story:postgres-real-provider-acceptance
revision: 8
---
## Selection and existing authorization

AEP implementing skill 0.19.1. The bounded clock experiment completed with an
independently reviewed rejection decision. Under the operator's existing
approval-record:milestones-delivery-20261002, select and activate:

- story:catalog-cli-journeys — ten production catalog-child lifecycle/mutation
  obligations and fifteen variants, with exact current ER audit/key observation.
- story:kubernetes-real-read-acceptance — four real-provider existing-read cases.
- story:postgres-real-provider-acceptance — five real-provider query/lifecycle
  cases, including direct adapter-future cancellation and its no-drop control.

Catalog revision5 received four approvals in round1; Kubernetes revision5 and
PostgreSQL revision6 received their recorded bounded panels. No findings remain
open on these selected plans. This authorizes one source/test commit per accepted
unit, integration commits, AEP closure and verified bot publication/releases under
the standing goal. It does not select unrelated drafts or answer pending product
decisions. No repeated approval or paid drive is required.

## Independence and ownership

Three separate managed worktrees, one worker per story, Rust-only runnable code
with clap derive for any new command. Each owns only its named adapter tests and
guide/evidence, with production code read-only until a reproduced defect receives
its own scope. Root is the sole AEP writer and serializes Cargo.lock and the exact
new ignored-suite classifications. Catalog owns its Cargo.toml dependency proposal;
worker lockfile changes are supplied as a patch for coordinator integration.
No shared target directory and no writes to primary checkout or another worker.

Catalog's shared fixture, signing clock, ER observer and recovery variants stay
one serial unit. Use paired controls for settlement injection. A failed injection
is a finding, not permission to drop its acceptance. Kubernetes and SQL use only
the task-owned disposable providers and private credentials, never the default
cluster or unrelated databases. Dedicated historical GitLab is a separately
qualified sandbox, not the installed forge integration. No secret values enter
public evidence or tool arguments.

## Resource and verification policy

Two Cargo jobs per worker, sccache, separate targets and short private TMPDIR.
Root grants build windows according to observed free space and host load, retaining
at least8GiB. Source authoring may run concurrently after dispatch; expensive
builds/live fixture runs are serialized when necessary. Release-profile binaries
are used for real CLI acceptance under unchanged production deadlines. Keep raw
logs and command exit statuses, exact selected/executed/failed/missing/helper
counts, source/binary/provider identities and verified owned-process cleanup.

Each unit receives independent adversarial review. Root integrates only green
units, verifies the merged repository and website inputs, and completes required
bot-authored source release handoffs. A unit's passing tests do not complete the
other provider's missing capabilities. Reuse prior verification only when its
relevant inputs are unchanged. Retain red evidence and limits explicitly.

## Remaining milestones after these slices

The Kubernetes read slice does not implement SelfSubjectAccessReview, C12 finite
and previous/multi-container logs, C15 conditional restart/rollout, C16 execution,
Helm interoperability/actions or deterministic packaging. Those remain selected
initiative work, with execution-family decisions still open. MCP follows provider
acceptance; read-only contract/model planning can proceed independently. No
caller assignment, outbound process ownership or execution-family answer is
invented from silence. The clock experiment does not close upstream ER issue51
or the sustained store-cost milestone.

## Scheduling output

The complete unfiltered `aep plan artifact waves --kind story --status draft
--format json` output below places all three selected stories in wave1. Other
candidates, collisions and unassessed items are retained as emitted; they are not
implicitly selected by this proposal.

```json
{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
        {
          "id": "story:catalog-cli-journeys",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "Cargo.lock"
            },
            {
              "confidence": "inferred",
              "path": "adapters/catalog/Cargo.toml"
            },
            {
              "confidence": "cited",
              "path": "adapters/catalog/tests/local_runtime"
            },
            {
              "confidence": "cited",
              "path": "adapters/catalog/tests/local_runtime.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-build/src/ignored.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/evidence/catalog-cli-20261002"
            }
          ]
        },
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
          "id": "story:kubernetes-real-read-acceptance",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "adapters/kubernetes/tests/local_runtime.rs"
            },
            {
              "confidence": "cited",
              "path": "adapters/kubernetes/tests/local_runtime/cli_journey.rs"
            },
            {
              "confidence": "cited",
              "path": "adapters/kubernetes/tests/provider.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/local-kubernetes-cli.md"
            }
          ]
        },
        {
          "id": "story:mcp-inbound-capability-projection",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/server/v1alpha1/projection.md"
            },
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/server/v1alpha1/scenarios"
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
          "id": "story:postgres-real-provider-acceptance",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "adapters/sql/tests/local_runtime.rs"
            },
            {
              "confidence": "cited",
              "path": "adapters/sql/tests/local_runtime/cli_journey.rs"
            },
            {
              "confidence": "cited",
              "path": "adapters/sql/tests/protocol.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/local-postgres-cli.md"
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
              "path": "adapters/gitlab"
            },
            {
              "confidence": "cited",
              "path": "adapters/kubernetes"
            },
            {
              "confidence": "cited",
              "path": "contracts/service/v1alpha1"
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
              "path": "docs"
            },
            {
              "confidence": "cited",
              "path": "ess"
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
          "id": "story:mcp-outbound-invocation-results",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/client/v1alpha1/invocation.md"
            },
            {
              "confidence": "inferred",
              "path": "adapters/mcp/contracts/client/v1alpha1/scenarios"
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
      "a": "story:catalog-cli-journeys",
      "b": "story:kubernetes-spec-service",
      "path": "Cargo.lock",
      "confidence": "inferred"
    },
    {
      "a": "story:catalog-cli-journeys",
      "b": "story:toolchain-pins-newest-release-20261001",
      "path": "Cargo.lock",
      "confidence": "inferred"
    },
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
      "path": "ess",
      "confidence": "inferred"
    },
    {
      "a": "story:mcp-cli-journey-discovery-contract",
      "b": "story:service-failure-carries-upstream-reason",
      "path": "contracts/cli/v1alpha1/semantics.md",
      "confidence": "inferred"
    },
    {
      "a": "story:mcp-inbound-capability-projection",
      "b": "story:mcp-inbound-mutation-replay",
      "path": "adapters/mcp/contracts/server/v1alpha1/scenarios",
      "confidence": "inferred"
    },
    {
      "a": "story:mcp-outbound-auth-lifecycle",
      "b": "story:mcp-outbound-invocation-results",
      "path": "adapters/mcp/contracts/client/v1alpha1/scenarios",
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

## Catalog planning panel outcome

Catalog revision5 received approve from all four named perspectives in round1:
aep:plan-critic-acceptance, aep:plan-critic-design, aep:plan-critic-scope and
aep:plan-critic-parallel-safety. Four immutable review-results retain exact reports;
four no-op outcomes record no required revision. This is acceptance of the plan,
not proof of the ten executed journeys.

Three workers reviewed independently without seeing other findings; root supplied
parallel-safety and is not independent of the plan author. Starts were staggered
around a bounded read-only preflight and the clock worker's waiting interval.
Sonnet is unavailable; reviewers inherited the session model. No claim of four
independent simultaneous Sonnet reviews is made.

The clock experiment now has a checked rejection decision. The three provider
stories are active under the existing delivery authorization; the dispatch record
below names their isolated worktrees and ownership. No provider acceptance is
claimed until its actual tests and independent review complete.

## Dispatch preflight — 2026-10-02

The clock worker is finished, restored and independently reviewed; its target was
cleaned and its evidence archived as cb26c-clock. The next wave uses the clean
managed integration branch plan/acceptance-20261002, not the primary checkout.
Primary branch main has exactly `?? .agents/`; that unrelated user directory is
preserved and outside every assigned surface. This existing managed-worktree
arrangement follows the operator's explicit isolation rule.

All three stories serve vision:independent-contract-adapters, matching their
parent initiative. Activation initially refused the missing serves edges on the
two real-provider stories. Those edges were supplied through AEP and activation
then succeeded; neither acceptance nor source scope changed.

Measured release target cost is 763 MiB for the clock unit; prior full integration
target was 7.2 GiB. Free space recovered to approximately30GiB before dispatch.
Keep8GiB reserve, two Cargo jobs and /usr/bin/sccache. No shared target and no
remaining old worker target. Root's target was already cleaned. Three workers fit
the host's four total agent slots including root; no explicit token budget exists
for the authorized goal. Expensive build windows remain coordinator-controlled.

Planned managed triples, relative to $HOME/.local/state/worktree/trees/b10x/connectors:

| Story | Tree / branch | Build | Scratch | Lease / worker |
| --- | --- | --- | --- | --- |
| catalog-cli-journeys | cb26d-catalog / acceptance/cb26d-catalog | cb26d-catalog/target | cb26d-catalog/.local/provider-wave/catalog | codex-cb26d-catalog / scope_next |
| kubernetes-real-read-acceptance | cb26d-k8s / acceptance/cb26d-k8s | cb26d-k8s/target | cb26d-k8s/.local/provider-wave/kubernetes | codex-cb26d-k8s / adversary_runner |
| postgres-real-provider-acceptance | cb26d-pg / acceptance/cb26d-pg | cb26d-pg/target | cb26d-pg/.local/provider-wave/postgres | codex-cb26d-pg / implement_runner |

Each exact tree base is the opening commit containing this record. Private briefs
will record its full identity, physical TMPDIR and assigned fixture authority.
Source authoring starts only after creation, lease and cheap opening checks.
Root owns Cargo.lock, ignored classification, AEP and bot publication. Workers
return source/test changes without committing. Independent review rotates away
from each implementor. No unit has passing runtime evidence yet.

## Live unit state — 2026-10-02

All three managed trees were created at exact opening commit
71dec8e6db5ca4fb87e5244662443a0d45d2a756, whose author and committer are b10x-bot[bot].
The catalog, Kubernetes and PostgreSQL branches/targets/scratch/leases are exactly
the triples recorded above. Each authoritative brief and active full story was
copied to its scratch directory before dispatch. Workers are authoring; no unit
commit or passing expanded suite is yet claimed. Root remains at the opening
commit plus this AEP update.

Short physical temporary directories are explicitly assigned outside the trees:
$HOME/.cache/c26d/cat, $HOME/.cache/c26d/k8s and $HOME/.cache/c26d/pg, mode700.
They are recorded disposable output, not shared targets or unowned cleanup scope.
The PostgreSQL worker has the first two-job release build window. Root is not
building; further build/live windows are granted from measured disk/target state.

Kubernetes receives only task-owned k3s at loopback33097 and namespace fixture.
Root supplied a private kubeconfig, CA, fresh8hour reader token and the exact
connectors-cb26d-k8s-hosts ClusterRole/Binding granting node get/list to
fixture/connector-reader. Worker namespace changes remain fixture-only; any other
cluster-scoped change returns to root. PostgreSQL uses task-owned loopback33096,
incidents and disposable reader; exact local admin observation/provisioning is
permitted while SUT business queries retain SELECT-only authority. Catalog uses
local deterministic fixtures and no real GitLab sandbox mutation in this unit.

Opening cheap checks: AEP validation exit0, cargo metadata --locked --offline
--no-deps exit0, clean source status and bot commit hooks passed. The public raw
log copies preserve libtest's blank/trailing whitespace, so whole-diff whitespace
checking names those six log locations; the authored Markdown/AEP/rejected patch
whitespace check passed. Logs were not silently edited to make that check green.
Exact validation output with historical review warnings is retained privately in
.local/provider-wave-briefs/preflight/aep-validation.txt. No runtime acceptance
is inferred from these opening checks.

## Discovered SQL classification defect — 2026-10-02

The PostgreSQL acceptance unit reproduced a specific error classification defect:
its wrapped data-modifying CTE receives PostgreSQL0A000, but the native mapper
returns Unavailable. The child remains ready and a later read succeeds. Full-loop
red2/2 and the independent first-CTE probe ruled out a preceding-request dependency.
The worker's retained diagnosis records hypotheses and discriminating outputs.
The earlier fixture expected invalid_input; the supported correction is exact
Unsupported, never accepting generic unavailable as a read-only refusal.

Root created/activated story:postgres-unsupported-feature-classification with
exact source scope and red evidence before authorizing source edits. The same
PostgreSQL worker/tree owns this sequential correction because its protocol and
CLI tests overlap the acceptance unit. No concurrent author is introduced. Add
only exact0A000 mapping, public wire regression first, native contract clarification
and complete real-case verification. No other error mapping, deadline, grant or
upstream version changes. Root remains the only AEP/classifier/Git writer.
Independent review must judge both stories and the full unit before integration.

The PostgreSQL worker retains the sole live window while other workers author
source. K8S expanded compilation and baseline39passed/4ignored are complete;
catalog baseline compilation is complete with its exact baseline executable
retained. Catalog expanded compilation follows PostgreSQL's correction, then
Kubernetes receives its live window. All provider units remain unaccepted until
actual final suites and review complete.

## Current executable admission and replay fixture — 2026-10-02

The first expanded repair case failed with OwnerBuildMismatch because the libtest
Client::begin called a production-CLI owner. Current transport.rs:351-358,487-492,
519-536 requires the caller and owner to have the same measured executable image.
This is intentional, and apps/connectors/tests/owner_build_security.rs protects it.
The retained failure is not a production defect.

Capture invalidation remains production-CLI acceptance: hold protected stdin after
connections connect has admitted Begin (apps/connectors/src/local/session.rs:177-201),
observe the exact new Pending Acquisition through the existing ER observer, stop the
busy child, then supply the fixture credential. Require lifecycle_conflict and no
new provider send. No fabricated capture or build digest is allowed.

Settled public CLI retries return locally before WriteClient
(apps/connectors/src/local/operations.rs:123-151). Keep those passive controls and
label them accurately. Preserve the separate successful owner/2 replay obligation
using the existing catalog fixture and a same-image host-library helper. This is a
sequential replacement of the invalid cross-image test seam within the same owned
test files, not a new runtime feature or removal of an acceptance obligation.

After original settlement through the production CLI/catalog child, stop and wait
for that exact owner. Spawn the same libtest executable with one exact ignored
helper that calls existing unmodified public owner::serve. Pass a real startup
UnixStream on FD3 and the exact verified private owner.lock lifetime file on FD4,
following production descriptor duplication, flock and isolated process rules.
The libtest parent WriteClient and helper naturally share actual /proc/self/exe
bytes; never forge a greeting/digest, change same_build, construct a fake Owner,
add a production test hook, or run fixed-FD mutation in a shared test thread.
Use start=false; no production-only hidden-command fallback through libtest.

Label this segment same-build host-library transport acceptance, distinct from the
production-CLI original effect. First measure successful handshake, exact owner
incarnation and bounded shutdown/lock release. Then prove one applied settled
replay before refusal/lost-response variants. Preserve all original request/attempt,
exact delivery/refusal, key, spent proof, clock, provider PUT/effect and audit checks.
A replay has its own correlated final audit; zero clock/provider calls does not
mean zero metadata writes. Stop custody/delete proof before replay, pass no proof,
and observe no new native child. Keep pending mode3 on its genuine public CLI owner
path and preserve all fifteen variants, including independent settlement-fault
feasibility controls. Helper source and deps remain in current test scope; no new
Cargo dependency is justified. Root adds its exact name as an excluded helper and
it never counts as a journey. The helper refuses absent private fixture selection.

Source-grounded feasibility is recorded in the coordinator's assigned
.local/provider-wave-briefs/owner-replay-feasibility/report.md. It is not executed
proof. If actual inherited-FD startup, cleanup or replay assertions fail, retain the
failure and return the unmet obligation without weakening deadlines or authority.

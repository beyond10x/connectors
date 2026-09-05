---
format: aep.planning-md/1
id: runbook:cli-ten-slack-first
kind: runbook
status: draft
title: 'CLI execution: ten stories, Slack delivery first'
tags:
- wave-cli
revision: 1
---
## Authorization and objective

Interactive run, preapproved by the operator on 2026-09-06: "commit and push (planning artifacts) ... the next 10 stories ahead, with highest prio the slack bug ... dispatch the stories with up to 5 agents". The operator then confirmed an unlimited subscription budget. aep-drive:wave skill version 0.8.0. O1 governed reach and O5 generic platform, using existing typed owners; Atlas ADR 0038 keeps extension-host responsibility independent of the platform core.

The existing readiness artifacts were already published in commit c97c0527114e9238fc0fe498ecdaf2c3292d0b8d on plan/ready-platform-cli-waves, draft PR 13. This branch replays those readiness mutations through AEP on top of the existing local Slack fix 5285a3bb47a47274b4b049f71a881bffd4b56146, preserving a single valid append-only journal.

Authorized commits: this opening planning/model commit, one source commit per reviewed unit, integration merges, closing evidence/store commit, and merge into the base after the full gate. Planning publication is explicitly authorized. Supported source installation and ordinary local Slack verification are the urgent delivery story; no release version bump or tag is authorized.

## Ten-story execution queue

1. `story:personal-local-writes-are-usable`
2. `story:explicit-target-never-implicit`
3. `story:one-shot-operations-without-a-daemon`
4. `story:one-placement-several-credentials`
5. `story:personal-local-workload-read`
6. `story:personal-gitlab-schedules-are-discoverable-and-governed`
7. `story:emit-treats-a-closed-pipe-as-failure`
8. `story:rate-limit-in-the-protocol`
9. `story:connect-session-oauth-custody-in-personal-posture`
10. `story:auth-as-tool-result`

The previous platform wave remains separately selected. The keyring story stays ready outside this ten: acceptance needs macOS custody verification and Windows has no authorized custody model. The closed-pipe correction is included. OAuth custody precedes trusted-outer-client auth remediation. Rate-limit protocol changes require coordinated frozen-contract migration; that prerequisite cannot be inferred from a CLI symptom.

## First implementation batch

Three workers, capped by four total harness slots including the coordinator. Slack regression/source delivery, explicit target selection, and Kubernetes inventory have disjoint assigned source paths. Kubernetes uses two bounded operations through the existing CLI and shared reader; ESS observation values are drafted before dispatch. Explicit target defaults to local, consistent with Acceptance and ESS. Other CLI stories overlap routing, credential or protocol owners and follow in dependency order.

Subagent types intended: aep-drive:implementor and aep-drive:adversary. This harness does not expose a subagent_type selector; ordinary agents load the exact plugin charter from disk. Scopers were read-only; the coordinator writes every AEP mutation. No child may mutate the planning store, commit, branch, publish, delete trees, or contact live providers.

## Preflight and resources

Primary main was clean at 5285a3bb; remote main caf9e36de3dd0a6f7508eee5b67abf7c19872a90. Atlas authority was a clean managed checkout at exact remote dfe353cba37ce072d118d0a67634fb78e59a5276. Other active Connectors trees belong to concurrent sessions; they are neither abandoned nor this wave's cleanup targets. This wave owns only the ids below.

Disk after removing measured idle, reproducible dev compiler outputs: 53 GB free; hard floor 20 GB. No process had those exact output paths open. One measured locked CLI no-run test build: exit 0, 144 seconds, 2.1 GB. Use sccache, three build jobs, no incremental output and zero dev/test debug info; every target stays inside its own worktree, CARGO_TARGET_DIR unset. Re-read free space on every return. Full integration gate runs all twelve listed workspaces and the final lane, each with its own recorded exit status. Initial ESS validation: connectors v1 — 9 file(s), valid. Projection: 162 capabilities: 146 generated, 16 obligation(s), 0 refused.

## Coordinator and unit triples

Integration branch `wave/cli-ten-slack-first`, worktree id `wt-2c3596f5baa5`. Root-relative locations below expand `~` to the coordinator account's home; briefs hold exact absolute paths outside the public repository. Scratch root `~/.cache/connectors-cli-wave-20260906`. Integration builds live below `~/.local/state/worktree/trees/b10x/connectors/wt-2c3596f5baa5`, in each Cargo workspace's target directory.

| Story | Branch | Head | Worktree | Build directory | Scratch | Stage |
|---|---|---|---|---|---|---|
| `personal-local-writes-are-usable` | `impl/personal-local-writes-are-usable` | opening commit (assigned before dispatch) | `~/.local/state/worktree/trees/b10x/connectors/wt-6ae00b19e482` | `~/.local/state/worktree/trees/b10x/connectors/wt-6ae00b19e482/crates/connectors-runtime/target` | `~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable` | provisioned, awaiting opening commit |
| `explicit-target-never-implicit` | `impl/explicit-target-never-implicit` | opening commit (assigned before dispatch) | `~/.local/state/worktree/trees/b10x/connectors/wt-09950110d866` | `~/.local/state/worktree/trees/b10x/connectors/wt-09950110d866/crates/connectors-cli/target` | `~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit` | provisioned, awaiting opening commit |
| `personal-local-workload-read` | `impl/personal-local-workload-read` | opening commit (assigned before dispatch) | `~/.local/state/worktree/trees/b10x/connectors/wt-728fb204f8cb` | `~/.local/state/worktree/trees/b10x/connectors/wt-728fb204f8cb/crates/connectors-runtime/target` | `~/.cache/connectors-cli-wave-20260906/personal-local-workload-read` | provisioned, awaiting opening commit |

## Computed proposal

Verbatim output of `aep plan artifact waves --kind story --status proposed --format json`. This is the whole proposed-story pool, not only this ten. Exact scope strings do not imply directory containment; the coordinator additionally reviewed first-batch containment and found no shared assigned files.

```json
{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
        {
          "id": "story:auth-as-tool-result",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "contracts"
            },
            {
              "confidence": "cited",
              "path": "crates/client/src/identity.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/client/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/client/src/response.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-cli/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-runtime/src/registry.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/domain/src/grant.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-gitlab/src/backend.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-slack/src/backend/api_runtime.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-slack/src/backend/connection_runtime.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/protocol/src/connection.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/protocol/src/operation.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/protocol/tests/bundles.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/hosted.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/hosted/admission.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/hosted/connection_route.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/hosted/enforcement.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/server/src/hosted/tests"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/local.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/service/src/connect_session.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/service/src/runtime.rs"
            },
            {
              "confidence": "inferred",
              "path": "ess/system/domains/connection.yaml"
            },
            {
              "confidence": "inferred",
              "path": "ess/system/domains/runtime.yaml"
            }
          ]
        },
        {
          "id": "story:connect-session-oauth-custody-in-personal-posture",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/catalog-build/src/document.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/catalog/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/catalog/src/table.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/catalog/tests/main/consumer_api.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connector-spec/schema/provider-toml.schema.json"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec/src/auth.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connector-spec/src/provider/auth_validation.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connector-spec/tests/main/oauth2_acquisition.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/design/07-credential-custody-topologies.md"
            },
            {
              "confidence": "inferred",
              "path": "docs/design/21-personal-oauth-callback-custody.md"
            },
            {
              "confidence": "inferred",
              "path": "providers/gitlab.toml"
            },
            {
              "confidence": "inferred",
              "path": "providers/jira.toml"
            },
            {
              "confidence": "inferred",
              "path": "providers/slack.toml"
            }
          ]
        },
        {
          "id": "story:deployment-catalogs-are-external-packs",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/catalog-build"
            },
            {
              "confidence": "inferred",
              "path": "crates/catalog-reader"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-config"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-runtime"
            },
            {
              "confidence": "inferred",
              "path": "docs/design"
            }
          ]
        },
        {
          "id": "story:personal-local-workload-read",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/integration-kubernetes/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-kubernetes/src/local.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/integration-kubernetes/src/local_inventory.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/integration-kubernetes/src/local_inventory_tests.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-kubernetes/src/local_workloads.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-kubernetes/src/workloads.rs"
            },
            {
              "confidence": "inferred",
              "path": "docs/guides/connect-kubernetes.md"
            }
          ]
        },
        {
          "id": "story:personal-local-writes-are-usable",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/connectors-runtime/tests/local_catalog_writes.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/architecture/deployment.md"
            },
            {
              "confidence": "cited",
              "path": "docs/guides/connect-slack.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 2,
      "artifacts": [
        {
          "id": "story:deployment-declared-destination-aperture",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/connectors-config"
            },
            {
              "confidence": "inferred",
              "path": "crates/domain"
            },
            {
              "confidence": "inferred",
              "path": "crates/server"
            },
            {
              "confidence": "inferred",
              "path": "crates/service"
            }
          ]
        },
        {
          "id": "story:emit-treats-a-closed-pipe-as-failure",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/connectors-cli/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-cli/tests/closed_pipe.rs"
            }
          ]
        },
        {
          "id": "story:one-placement-several-credentials",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/connectors-config/src/personal.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-console/src/auth.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-console/src/enrol.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-console/src/output_tests.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-catalog/src/hosted.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-catalog/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/guides/connect-slack.md"
            },
            {
              "confidence": "inferred",
              "path": "ess/system/domains/connection.yaml"
            },
            {
              "confidence": "inferred",
              "path": "ess/system/domains/deployment.yaml"
            }
          ]
        },
        {
          "id": "story:sources-are-processed-by-code",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/catalog-build"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec"
            }
          ]
        },
        {
          "id": "story:the-claims-journal-survives-a-full-life",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/connectors-runtime"
            },
            {
              "confidence": "inferred",
              "path": "docs/design/13-grant-evaluation-and-approval-redemption.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 3,
      "artifacts": [
        {
          "id": "story:explicit-target-never-implicit",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/connectors-cli/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-cli/tests/cli_surface.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-cli/tests/cli_surface_drift.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-console/src/doctor.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-console/src/output.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/design/19-the-cli-surface.md"
            }
          ]
        },
        {
          "id": "story:per-service-verification-probes",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/catalog"
            },
            {
              "confidence": "inferred",
              "path": "crates/catalog-build"
            },
            {
              "confidence": "inferred",
              "path": "crates/connector-resolve"
            },
            {
              "confidence": "inferred",
              "path": "crates/connector-spec"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-console"
            },
            {
              "confidence": "inferred",
              "path": "crates/service"
            },
            {
              "confidence": "inferred",
              "path": "providers"
            }
          ]
        }
      ]
    },
    {
      "wave": 4,
      "artifacts": [
        {
          "id": "story:one-shot-operations-without-a-daemon",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/connectors-cli/README.md"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-cli/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-cli/tests/one_shot_operations.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-console/src/doctor.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-runtime/src/composition.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-runtime/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/local.rs"
            }
          ]
        }
      ]
    },
    {
      "wave": 5,
      "artifacts": [
        {
          "id": "story:personal-gitlab-schedules-are-discoverable-and-governed",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "SOURCES.toml"
            },
            {
              "confidence": "cited",
              "path": "catalog/gitlab.catalog.json"
            },
            {
              "confidence": "cited",
              "path": "connectors.lock"
            },
            {
              "confidence": "cited",
              "path": "crates/catalog-build/tests/main/catalog_invariants.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-cli/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-cli/tests/search_bounds.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-runtime/src/registry.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-catalog/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "docs/guides/connect-gitlab.md"
            },
            {
              "confidence": "inferred",
              "path": "ess/system/domains/gitlab.yaml"
            },
            {
              "confidence": "inferred",
              "path": "ess/system/system.yaml"
            },
            {
              "confidence": "cited",
              "path": "providers/gitlab.toml"
            },
            {
              "confidence": "inferred",
              "path": "specs/gitlab.provenance.toml"
            },
            {
              "confidence": "inferred",
              "path": "specs/gitlab/pipeline-schedules.openapi.yaml"
            }
          ]
        }
      ]
    },
    {
      "wave": 6,
      "artifacts": [
        {
          "id": "story:rate-limit-in-the-protocol",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "catalog/slack.catalog.json"
            },
            {
              "confidence": "inferred",
              "path": "connectors.lock"
            },
            {
              "confidence": "cited",
              "path": "contracts/connector-operation/v0alpha1"
            },
            {
              "confidence": "cited",
              "path": "crates/catalog-build/tests/main/catalog_invariants.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/catalog/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/catalog/src/table.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connector-resolve/src/document.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec/src/ir.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-cli/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-console/src/envelope.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-console/src/output.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-runtime/src/registry.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-runtime/src/service_bundle.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-catalog/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-gitlab/src/backend.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-jira/src/backend/operations.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-kubernetes/src"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-mcp/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-monitoring/src/backend.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-platform/src/datasource.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-platform/src/surface.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-sip/src/backend/mod.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-slack/src/backend/api_runtime.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/protocol/src/operation.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/protocol/tests/bundles.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/hosted.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/server/src/hosted/mcp/toolset.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/hosted/tests"
            },
            {
              "confidence": "inferred",
              "path": "ess/system/domains/catalog.yaml"
            },
            {
              "confidence": "inferred",
              "path": "providers/slack.toml"
            }
          ]
        }
      ]
    }
  ],
  "collisions": [
    {
      "a": "story:auth-as-tool-result",
      "b": "story:emit-treats-a-closed-pipe-as-failure",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:explicit-target-never-implicit",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:one-placement-several-credentials",
      "path": "ess/system/domains/connection.yaml",
      "confidence": "inferred"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:one-shot-operations-without-a-daemon",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:one-shot-operations-without-a-daemon",
      "path": "crates/server/src/local.rs",
      "confidence": "cited"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:personal-gitlab-schedules-are-discoverable-and-governed",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:personal-gitlab-schedules-are-discoverable-and-governed",
      "path": "crates/connectors-runtime/src/registry.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/connectors-runtime/src/registry.rs",
      "confidence": "cited"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/integration-gitlab/src/backend.rs",
      "confidence": "cited"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/integration-slack/src/backend/api_runtime.rs",
      "confidence": "cited"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/protocol/src/operation.rs",
      "confidence": "cited"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/protocol/tests/bundles.rs",
      "confidence": "cited"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/server/src/hosted.rs",
      "confidence": "cited"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/server/src/hosted/tests",
      "confidence": "inferred"
    },
    {
      "a": "story:connect-session-oauth-custody-in-personal-posture",
      "b": "story:personal-gitlab-schedules-are-discoverable-and-governed",
      "path": "providers/gitlab.toml",
      "confidence": "inferred"
    },
    {
      "a": "story:connect-session-oauth-custody-in-personal-posture",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/catalog/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:connect-session-oauth-custody-in-personal-posture",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/catalog/src/table.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:connect-session-oauth-custody-in-personal-posture",
      "b": "story:rate-limit-in-the-protocol",
      "path": "providers/slack.toml",
      "confidence": "inferred"
    },
    {
      "a": "story:deployment-catalogs-are-external-packs",
      "b": "story:deployment-declared-destination-aperture",
      "path": "crates/connectors-config",
      "confidence": "inferred"
    },
    {
      "a": "story:deployment-catalogs-are-external-packs",
      "b": "story:per-service-verification-probes",
      "path": "crates/catalog-build",
      "confidence": "inferred"
    },
    {
      "a": "story:deployment-catalogs-are-external-packs",
      "b": "story:sources-are-processed-by-code",
      "path": "crates/catalog-build",
      "confidence": "inferred"
    },
    {
      "a": "story:deployment-catalogs-are-external-packs",
      "b": "story:the-claims-journal-survives-a-full-life",
      "path": "crates/connectors-runtime",
      "confidence": "inferred"
    },
    {
      "a": "story:deployment-declared-destination-aperture",
      "b": "story:per-service-verification-probes",
      "path": "crates/service",
      "confidence": "inferred"
    },
    {
      "a": "story:emit-treats-a-closed-pipe-as-failure",
      "b": "story:explicit-target-never-implicit",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:emit-treats-a-closed-pipe-as-failure",
      "b": "story:one-shot-operations-without-a-daemon",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:emit-treats-a-closed-pipe-as-failure",
      "b": "story:personal-gitlab-schedules-are-discoverable-and-governed",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:emit-treats-a-closed-pipe-as-failure",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:explicit-target-never-implicit",
      "b": "story:one-shot-operations-without-a-daemon",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:explicit-target-never-implicit",
      "b": "story:one-shot-operations-without-a-daemon",
      "path": "crates/connectors-console/src/doctor.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:explicit-target-never-implicit",
      "b": "story:personal-gitlab-schedules-are-discoverable-and-governed",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:explicit-target-never-implicit",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:explicit-target-never-implicit",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/connectors-console/src/output.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:one-placement-several-credentials",
      "b": "story:personal-gitlab-schedules-are-discoverable-and-governed",
      "path": "crates/integration-catalog/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:one-placement-several-credentials",
      "b": "story:personal-local-writes-are-usable",
      "path": "docs/guides/connect-slack.md",
      "confidence": "cited"
    },
    {
      "a": "story:one-placement-several-credentials",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/integration-catalog/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:one-shot-operations-without-a-daemon",
      "b": "story:personal-gitlab-schedules-are-discoverable-and-governed",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:one-shot-operations-without-a-daemon",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:per-service-verification-probes",
      "b": "story:sources-are-processed-by-code",
      "path": "crates/catalog-build",
      "confidence": "inferred"
    },
    {
      "a": "story:per-service-verification-probes",
      "b": "story:sources-are-processed-by-code",
      "path": "crates/connector-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:personal-gitlab-schedules-are-discoverable-and-governed",
      "b": "story:rate-limit-in-the-protocol",
      "path": "connectors.lock",
      "confidence": "inferred"
    },
    {
      "a": "story:personal-gitlab-schedules-are-discoverable-and-governed",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/catalog-build/tests/main/catalog_invariants.rs",
      "confidence": "cited"
    },
    {
      "a": "story:personal-gitlab-schedules-are-discoverable-and-governed",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:personal-gitlab-schedules-are-discoverable-and-governed",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/connectors-runtime/src/registry.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:personal-gitlab-schedules-are-discoverable-and-governed",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/integration-catalog/src/lib.rs",
      "confidence": "cited"
    }
  ],
  "unassessed": [],
  "cycles": []
}
```

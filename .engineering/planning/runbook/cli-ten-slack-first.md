---
format: aep.planning-md/1
id: runbook:cli-ten-slack-first
kind: runbook
status: draft
title: 'CLI execution: ten stories, Slack delivery first'
tags:
- wave-cli
revision: 25
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
| `personal-local-writes-are-usable` | `impl/personal-local-writes-are-usable` | 60a031b382ad7b0ffa98182cd0333bd37542c430 | `~/.local/state/worktree/trees/b10x/connectors/wt-6ae00b19e482` | `~/.local/state/worktree/trees/b10x/connectors/wt-6ae00b19e482/crates/connectors-runtime/target` | `~/.cache/connectors-cli-wave-20260906/personal-local-writes-are-usable` | integrated; full gate passed; final installed delivery pending |
| `explicit-target-never-implicit` | `impl/explicit-target-never-implicit` | fd8bae818e2a1661f7eb9153cba0ba590f50f7f8 | `~/.local/state/worktree/trees/b10x/connectors/wt-09950110d866` | `~/.local/state/worktree/trees/b10x/connectors/wt-09950110d866/crates/connectors-cli/target` | `~/.cache/connectors-cli-wave-20260906/explicit-target-never-implicit` | integrated; full gate passed |
| `personal-local-workload-read` | `impl/personal-local-workload-read` | a3eab7a42ebb63f4c265dc204f2f6c7f419f1abc | `~/.local/state/worktree/trees/b10x/connectors/wt-728fb204f8cb` | `~/.local/state/worktree/trees/b10x/connectors/wt-728fb204f8cb/crates/connectors-runtime/target` | `~/.cache/connectors-cli-wave-20260906/personal-local-workload-read` | integrated; full gate passed |

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

## Scratch-path correction

The full runtime baseline found Unix SUN_LEN failure under the long assigned scratch path (31 passed, 1 failed). Coordinator creates short additional temporary roots, preserving original reports/logs: Slack `~/.cache/cw6/s`, target `~/.cache/cw6/t`, workload `~/.cache/cw6/k`, integration `~/.cache/cw6/p`. Set TMPDIR to the respective short root, and create socket fixtures there. Both original scratch and these exact roots are coordinator-owned cleanup inputs; no repository test is weakened.

Workload fixture dependency correction: coordinator applied the returned patch adding tower 0.5 util as a dev dependency in crates/integration-kubernetes/Cargo.toml. Runtime offline metadata exited 0 and required no lock diff. This assigned manifest is disjoint from the other first-batch units.

## Coordinator package gate corrections

The coordinator applied the exact reviewed pre-existing lint corrections returned by both units: unused Datasource imports in Kubernetes hosted tests; newline-preserving writeln substitutions and moving unchanged Argo CD helper functions above the console test module. No test case, assertion or lint was suppressed. Those files are explicitly assigned to their respective units. The workload fixture additionally required the existing tower dependency in the runtime lockfile row; full offline metadata, rather than --no-deps, produced that lock update.

## Workload review routing

Workload adversary pass 1 is recorded verbatim as review-result:cli-workload-adversary-1-20260906 (54 to 56 cases, one introduced warning). Its retained null-cursor contract case is red; same implementor receives correction-1.md to reject explicit null before I/O and preserve omitted cursor. This is the first correction, not a repeated failed fix.

## Retained review cases

Adversarial cases are committed after the implementation commit, preserving the reviewed commit as a reachable ancestor rather than rewriting its evidence identity. These per-unit test/correction commits and integration merges are part of completing the authorized wave. Slack pass 1 is recorded verbatim as review-result:cli-slack-adversary-1-20260906; no findings, 72 to 75 cases. Its outcome is no-op; the added test coverage remains in the unit branch.

## Target review routing

Target pass 1 is recorded verbatim as review-result:cli-target-adversary-1-20260906 (164 to 167 cases, two red cases for one ordering finding). The adversary marked origin undecided. Coordinator classifies it introduced: base has no target flag or conflict contract; this unit added target.validate after the input-acquisition match, exposing the new conflicting hosted command to file reads/stdin blocking before its required refusal. Same implementor receives correction-1.md.

## Standing connector direction

The operator clarified the standing direction on 2026-09-06: official OpenAPI is mandatory when available; the usual connector goal is ALL operations. Focused surfaces are consumer projections or prebuilt Connectors projections. Administrative and regular-user surfaces may be separated, grounded in the provider permission model. A projection never grants authority.

The four pipeline-schedule operations are the first delivery slice selected by this wave, not the target GitLab API coverage. Keep the full official OpenAPI as source authority and account for every operation in a deterministic coverage inventory: admitted/catalogued, platform authentication flow, or an explicit remaining coverage/importer gap. Preserve a backlog for the remaining source operations. Do not let a four-operation source extraction become an invisible limit on connector scope. Narrow model exposure and complete underlying callability are independent decisions.

Before implementing the GitLab source path, revise its bounded vendoring proposal to retain the complete scrubbed vendor document and derive the schedule slice through reviewed ingest selections. Classify administrative versus regular-user operations only from source-grounded permissions, and carry unresolved classifications as explicit gaps rather than assuming access. Existing inline operation provenance remains truthful until each operation is migrated to the official source.

## Official source formats

The operator further clarified on 2026-09-06: connector definitions should normally never be hand-authored. When an official source format is unsupported, build the importer. Official OpenAPI is used whenever available. This supersedes any earlier suggestion that an unsupported importer permits authored endpoint/schema replacements. The only authored-source exception is a documented absence of a usable official machine-readable source, grounded in fetched official documentation. Full operation coverage and focused consumer/prebuilt projections remain the goal; administrative and regular-user projections do not grant authority.

## Review publication and current authority

Before publication, the coordinator retained the original unpublished review/store delta privately, restored its own uncommitted store changes to the committed base, and replayed them through AEP with local home prefixes replaced by `~`. Immutable published review records were not edited. Raw reports remain available in assigned private scratch; public records declare the mechanical path redaction without changing counts, assertions or findings. Subsequent planning writes identify the executing wave agent through AEP_ACTOR.

Atlas authority advanced to clean remote main 4911915de3bea434b6092d245b9f3ef98d6f0eca and was provisioned read-only in managed tree wt-ae608b71b66b. Both final adversarial passes found nothing: target selection 168→171 executed cases, workload inventory 60→63. Corrections and retained cases preserve prior reviewed commits as ancestors.

## Second implementation batch

First batch is integrated at 76f3fef9ce53a92d54d5e1c8147c5943315d423f. Full integration gate remains pending; story closure waits for it and final delivery. The next three worker assignments start at that exact source base. The same four-slot harness cap allows three workers. Disk before provisioning: 54 GB free; floor remains 20 GB.

| Unit | Branch | Managed tree | Build below tree | Scratch | Short TMPDIR | Stage |
|---|---|---|---|---|---|---|
| `one-placement-several-credentials` | `impl/one-placement-several-credentials` | `~/.local/state/worktree/trees/b10x/connectors/wt-03d120cf8736` | `crates/connectors-runtime/target; crates/connectors-console/target` | `~/.cache/connectors-cli-wave-20260906/one-placement-several-credentials` | `~/.cache/cw6/m` | implementation |
| `emit-treats-a-closed-pipe-as-failure` | `impl/emit-treats-a-closed-pipe-as-failure` | `~/.local/state/worktree/trees/b10x/connectors/wt-55b6348b9982` | `crates/connectors-cli/target` | `~/.cache/connectors-cli-wave-20260906/emit-treats-a-closed-pipe-as-failure` | `~/.cache/cw6/e` | implementation |
| `personal-gitlab-schedules-are-discoverable-and-governed` | `impl/personal-gitlab-schedules-are-discoverable-and-governed` | `~/.local/state/worktree/trees/b10x/connectors/wt-0374e7372138` | `target` | `~/.cache/connectors-cli-wave-20260906/personal-gitlab-schedules-are-discoverable-and-governed` | `~/.cache/cw6/g` | official-source stage only |

The computed full-story proposal separates GitLab from the CLI and catalog-runtime owners. This dispatch follows that constraint by limiting its first stage to official-source vendoring, importer/projection and catalog artifacts. It cannot edit the CLI or catalog runtime while closed-pipe and multi-credential units own them. Coordinator integrates those prerequisites before GitLab's second stage; no story is closed on source prep alone. Each unit brief names this stage restriction and exact paths. One-shot CLI composition follows the closed-pipe unit; rate-limit work follows a coordinated version decision; OAuth precedes auth remediation.

Full unfiltered computed proposal before these three moves:

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
          "id": "story:personal-gitlab-schedules-are-discoverable-and-governed",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "Cargo.lock"
            },
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
              "confidence": "inferred",
              "path": "crates/catalog-cli/Cargo.toml"
            },
            {
              "confidence": "inferred",
              "path": "crates/catalog-cli/examples/vendor_gitlab.rs"
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
              "path": "specs/gitlab/coverage-19.4.toml"
            },
            {
              "confidence": "inferred",
              "path": "specs/gitlab/openapi-19.4.yaml"
            }
          ]
        }
      ]
    },
    {
      "wave": 5,
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
      "a": "story:one-placement-several-credentials",
      "b": "story:personal-gitlab-schedules-are-discoverable-and-governed",
      "path": "crates/integration-catalog/src/lib.rs",
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

## First batch integration gate

Integrated source through 346c434960a1235685a73f054be707554e055a17 passed every declared workspace and the final lane on 2026-09-06. The initial full gate exited 101 in Kubernetes because a new test assumed sorted JSON object keys; the entire runtime workspace enables serde_json preserve_order. Coordinator corrected that assertion to compare exactly the same four-key set, retained value/cardinality assertions, and reran the full runtime workspace successfully. This was a test representation correction, not a production behavior change or another adversarial pass.

The root workspace passed in integration-gate-batch1.log; corrected runtime passed in runtime-gate-correction.log. The other ten workspaces and final lane passed through the gate's --workspace and --final entry points, recorded in integration-gate-remaining.log and .exits. All logs are in assigned planning scratch. Each command's own status was captured, never a tail pipeline status. The new cases are included in actual runner counts.

| Workspace | Passed executions | Ignored |
|---|---:|---:|
| `.` | 1141 | 4 |
| `crates/connectors-runtime` | 323 | 2 |
| `crates/connectors-cli` | 84 | 0 |
| `crates/connectors-console` | 87 | 0 |
| `crates/driver-audio` | 15 | 0 |
| `crates/driver-speech` | 22 | 2 |
| `crates/driver-cdp` | 29 | 2 |
| `crates/driver-sip` | 5 | 0 |
| `crates/driver-sql` | 28 | 14 |
| `crates/rtvbp-voice-endpoint` | 7 | 0 |
| `crates/voice-local-audio` | 4 | 1 |
| `crates/voice-runtime` | 4 | 0 |

Total 1,749 passed executions, zero failed, 25 ignored. Ignored optional live/hardware cases are not claimed as executed. Final catalog check: 65 providers, 68 artifacts verified; links portable; story index and 73 legacy records consistent; ESS connectors v1, ten files valid, deterministic clap output identical. This closes first-batch source verification; final installed CLI delivery and later batch integration remain pending.

The GitLab source unit measured a legitimate additional absent response body: official DELETE schedule returns 204/no content. Its response coverage rises to 856/1011 with 155 absences. The coordinator reviewed the exact response_schema_coverage counter change 155→156 to preserve its stated one-absence allowance and retain the two-operation empty-provider refusal; the existing assertions and covered floor are unchanged.

## Additional assigned surfaces

The multi-credential unit may extract focused catalog/config modules to satisfy the source-size fence. Its exact scope now includes config personal/catalog.rs and lib.rs, integration-catalog personal_connections.rs and personal_connections_tests.rs, and the runtime local_catalog_writes fixture constructor. The GitLab source unit may extend the existing declaration/publishing/patch/schema machinery and its tests, the catalog-build dev dependency, and catalog-reader/catalog.pack generated output. These surfaces do not overlap the current closed-pipe or multi-credential writers. Exact story scopes are the coordinator-owned current record.

The Atlas migration proposal is an additional managed tree: wt-19c8158c5cae, branch plan/connector-operation-v2, proposed ADR commit b4ad7e8d94edecdcbea6a9aa4f7464e28078c4cf. It is not published or accepted; its planning and Markdown checks passed. Authoritative operation wrappers continue to come from the separate clean remote-main authority tree wt-ae608b71b66b.

## Schema fidelity and second-batch review

The operator refined the standing direction on 2026-09-06: a practical useful Slack operation set is sufficient; selected operation schemas must remain one for one with official specifications. The 1,847-operation inventory belongs to GitLab, not Slack. Official machine-readable sources remain mandatory when usable, and unsupported formats require importers. AGENTS.md now distinguishes provider scope selection from schema fidelity. No arbitrary operation count determines Slack's useful surface.

The in-flight GitLab audit found actual losses: model-facing contract lowering widens integers and drops enum/other constraints; body flattening can discard body-level semantics; request assembly can turn omitted optional fields into explicit nulls. Schema-correcting response-array overlays also differ from the literal official OpenAPI. These are implementation gaps to address before claiming fidelity. Preserve original source, report upstream defects, and fix generic import/projection/assembly owners rather than writing replacement definitions. Stage 1 is being revised; its earlier green package run does not satisfy this newer requirement.

Credential adversarial pass 1 measured compound-auth disappearance (real Datadog API-key/application-key conjunction) and credential replacement before a rejected enrollment lock check. Its final immutable report and correction routing follow when the reviewer completes. The coordinator added only a cfg(test) enrollment seam at 66c62dea36120bf1bd267b0863f2f3e2bc4b9d8b.

Closed-pipe correction 1 is committed at 68b30016aafb3a2bf5912f2d606c2a3a6413e100, bot author/committer verified. All retained first-pass cases plus class controls pass: CLI 96 to 101, zero red. The first review outcome is fixed; a fresh final second adversarial pass is dispatched. Command semantic status is evaluated after result emission even when the output consumer closes.

The first-batch checkpoint a5dfb480ab58acba1ed637f0bd34be3a392ed114 is published on origin/wave/cli-ten-slack-first. After confirming source recovery on that remote branch and no process using the exact tree outputs, the coordinator cargo-cleaned only measured owned target directories, finished the three clean trees, reviewed GC dry-run and applied exactly wt-09950110d866, wt-728fb204f8cb and wt-6ae00b19e482. All three were removed by the worktree manager. Their source and review records remain reachable; their table paths above are historical, not live checkouts.

Atlas remote main advanced to f4f2b51bdbe18c188055d70e9762b9c520d3a9f1. The current clean exact authority tree is wt-09ac7303c4aa under ~/.local/state/worktree/trees/b10x/atlas/. Prior authority trees remain managed and untouched. The separate draft migration tree wt-19c8158c5cae at b4ad7e8d94edecdcbea6a9aa4f7464e28078c4cf holds proposed ADR 0039 for a new operation protocol version; it is not an accepted architecture decision or the bot authority checkout.

## One-shot staged dispatch

Closed-pipe final review recorded as review-result:cli-pipe-adversary-2-20260906 with no findings; 101 to 104 CLI cases passed. Retained tests committed51c85706d633c1b2621d6a1babc159192e6b78bf, merged3df1cd2df32472a4afc8feeaac8c46c9de8d1b55. Story remains active pending the next integrated full gate.

The next authorized CLI owner is story:one-shot-operations-without-a-daemon, from that merged head, managed tree wt-fc49f37347f5, branch impl/one-shot-operations-without-a-daemon. Tree ~/.local/state/worktree/trees/b10x/connectors/wt-fc49f37347f5; per-workspace targets within it; scratch ~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon and short TMPDIR ~/.cache/cw6/o. Owns runtime composition, local daemon state-lock seam, CLI routing, doctor and one-shot tests/README. GitLab remains source-only until this CLI owner hands off. Credential correction owns resolver credentials.rs plus its lib.rs re-export; GitLab must return any overlapping tiny registration for coordinator scheduling.

The machine proposal below covers the complete current proposed pool, not a filtered shortlist. Directory containment was additionally reviewed. The operator already approved these ten stories; this is staged dispatch within that grant.

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
        },
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
              "confidence": "inferred",
              "path": "contracts/connector-operation/v0alpha2"
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
              "path": "crates/client/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/client/src/response.rs"
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
              "confidence": "inferred",
              "path": "crates/protocol/src/operation/legacy.rs"
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
              "path": "crates/server/src/local.rs"
            },
            {
              "confidence": "inferred",
              "path": "ess/system/domains/catalog.yaml"
            },
            {
              "confidence": "inferred",
              "path": "json-schemas.toml"
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
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/client/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/client/src/response.rs",
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
      "a": "story:auth-as-tool-result",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/server/src/local.rs",
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
      "a": "story:one-shot-operations-without-a-daemon",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:one-shot-operations-without-a-daemon",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/server/src/local.rs",
      "confidence": "inferred"
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
    }
  ],
  "unassessed": [],
  "cycles": []
}
```

## Source-fidelity implementation stage

Atlas proposed ADR 0040 (connector-catalog-source-fidelity) records catalog schema 3 and the producer/reader/resolver and relying-party movement. It is a reviewable migration decision under the operator's existing source-fidelity requirement; it does not claim release or delivery. The coordinator drafted RequestSemantics and RequestParameterSemantics in ess/system/domains/catalog.yaml before implementation. ESS validation over ess/system reports: connectors v1 — 10 file(s), valid. An initial invocation on system.yaml alone refused undeclared domains because it loaded only the header; the complete-directory validation resolved that invocation error without changing domain ownership.

The closed semantics are legacy_v1 and openapi_3_0_json_v1. ParamSet carries the profile plus optional body_required, meaningful only with body_schema. The required canonical operation profile prevents old or unknown request interpretation. Keep literal source schemas, translate OAS 3.0 semantics deterministically into caller JSON Schema, preserve request-body presence and literal strings, and refuse unsupported constructs. Remove the unused response_arrays feature introduced by this uncommitted stage rather than retaining a new hand-correction facility. Existing legacy behavior remains explicit and is not relabeled faithful.

Additional machine scope now records the generic importer/translation, contract/document/build/reader/table/resolver, profile tests, catalog directory and JSON inventory. The catalog directory scope owns only deterministic generated documents, pack/schema and their source registration; no other active worker owns generated catalog output. It is deliberately recorded as a directory and additionally reviewed for containment. Preserve the old schema 2 identity/bytes and emit a distinct schema 3 identity/file. Registry, CLI and integration-catalog/lib.rs remain deferred until their current owners hand off. The GitLab agent may return a small integration-catalog schema-selection patch for coordinator integration; it must not race the credentials owner.

The exact algorithm/test proposal is the implementor's source-fidelity-design.md retained in the wave's private scratch. Its required cases independently compare all four literal source schema closures, nullable/enum/oneOf behavior, body absence/null/defaults, string bodies, safe namespaced path encoding, unknown profile/old-reader refusals and every retained legacy behavior. No production provider call or operator configuration change is authorized. Source defects remain visible and must not be silently repaired.

## Final credential review and remaining dispatch

Draft PR14 publishes the in-progress wave branch at d5807a694858258a8fa9818224ae7909322f43ea: https://github.com/beyond10x/connectors/pull/14. The full first-batch gate remains the latest complete integrated gate; later units are not closed by that result.

The pipe unit's final tests/source are merged and published through that branch. Its owned worktree wt-55b6348b9982 had no live processes, was clean, and its HEAD51c85706d633c1b2621d6a1babc159192e6b78bf was reachable from the advertised integration ref. Its measured CLI build output was cleaned (2.5GiB removed), worktree finish succeeded, and profile-wide GC dry-run reviewed every record. Only the exact eligible pipe id was passed to GC apply; final result is recorded after completion.

Atlas authority is the clean managed tree wt-09ac7303c4aa at 304a58f851ac95cd1010aced1a2aa0f57ed8c511, compared with remote main. Proposed migration ADR0040 and its AEP record are committed on the separate plan/connector-operation-v2 branch at 5e3a327d748da55591a3df5f079d77b1d140e3b1; that draft is not published or accepted. ADR0039 is the preceding rate-limit protocol proposal. No consumer upgrade is claimed.

Credential review2 is immutable in review-result:cli-credentials-adversary-2-20260906. It has one confirmed blocker after two full attacks; decision-blocker:cli-credentials-final-review blocks its completion and the source leaves the merge set. No third attack is dispatched. One-shot continues using distinct named fixture instances, without merging the failed credential unit.

GitLab compiler/contract fallout is now explicitly scoped: catalog-build/src/scaffold.rs retains LegacyV1 for existing scaffolding; shipped_providers.rs checks whole-body requiredness against its explicit source value while preserving all other checks. One-shot's existing target fixture is updated to the now-intended missing-daemon behavior, retaining target selection and invalid hosted-login isolation. Coordinator applied that patch and formatted its additive CatalogBackend capability hunk.

Read-only OAuth runtime scoping is dispatched to scope_oauth, using the plugin story-scoper charter through an ordinary agent (same declared harness deviation). The existing schema/doc-only scope is insufficient for the recorded runtime acceptance. No OAuth implementation or new entity is dispatched before the model and exact scope are recorded. Scratch brief is under the wave's connect-session-oauth-custody-in-personal-posture directory; the scoper writes no files.

## Current authority and one-shot review

Fresh clean Atlas authority is3f0dbbf7701ce636df510316282d1c96a917df01 in managed tree wt-09ac7303c4aa. Its latest accepted ADR is0039, the ESS conformance-count reader-before-writer migration. The Connectors wave still aligns with accepted ADR0038's generic platform/independent extension boundary; ADR0039 does not change Connector provider selection, schema fidelity, grant or custody rules.

Our unpublished migration drafts were replayed through AEP on a branch from that exact remote commit, reusing clean managed tree wt-19c8158c5cae as plan/connector-cli-migrations. Rate-limit protocol proposal is now ADR0040; source-fidelity catalog proposal is ADR0041. The prior local5e3a327 proposal branch remains a recovery reference; its0039/0040 numbering is superseded and not published. No planning journal was textually merged or copied.

Atlas's full fences.sh exited1. Catalog, live Pages, projections and brand passed. Shared primary state fails documentation collection (AgentIDE b10x-docs/v4 unsupported by the pinned collector), Website Docs System pin, and map grounding (widgets lacks Serves). A bot-observer test also failed its error-message assertion; the exact focused rerun passed, and a full Rust rerun follows. Coordinator fixed one newly introduced relative ADR link; the managed Markdown checker now reports158files and0findings. The full gate remains red; the draft is not pushed and none of these observations claims delivery.

Pipe worktree wt-55b6348b9982 GC completed through exact reviewed-id application. The manager recorded remote recovery through origin:refs/heads/wave/cli-ten-slack-first and origin:refs/pull/14/head before non-forced removal. No other tree was selected.

One-shot source970e4af56f7a4ca1b9689c885fabd3a519bbe0ca is bot-authored and bot-committed after618 passing affected tests (2existing ignored), strict clippy/fmt/module fence. First tests-only attack is dispatched to adversary_one_shot_1 in the same owned tree; no AEP/Git writes delegated. Full base for its review remains3df1cd2df32472a4afc8feeaac8c46c9de8d1b55. The implementation report and original red fixtures remain under the recorded scratch triple.

## Rate-limit staged dispatch

Continuation of the operator-approved ten-story CLI wave; aep-drive:wave skill0.8.0. Dispatch uses the ordinary agent harness with the full aep-drive:implementor charter because this harness has no plugin subagent_type selector, as recorded at wave opening. The user authorized up to five workers and an unlimited subscription budget; this harness has three worker slots beside its coordinator.

Select only story:rate-limit-in-the-protocol from computed proposed wave2. The full computed output below is retained unchanged; no other newly listed story is selected. Active GitLab and one-shot work are absent from this proposed-only query, so it does not establish whole-unit disjointness against them. Exact path and directory-containment inspection requires staged ownership: stage1 is additive protocol version modules/contracts/tests and the service/server egress parser/header extraction. No current public DTO field changes, catalog/spec/schema changes, local dispatch, CLI, or provider adapters until coordinator handoff after the relevant source units land. Stage1 must remain compilable with existing callers. The complete story remains one unit and receives adversarial review only after all stages.

The complete decision and acceptance live on the rate-limit story, with validated catalog/runtime ESS value types. Preserve existing fixed RateLimit serialization, expose conditional source-grounded Slack advice without guessing app category, and retain frozen v1 bytes behind an explicit v2 compatibility boundary. Atlas authority is clean remote f686fe034bc3f1c7c5d276476b814ee05c2bca9f; its new ADR0039 concerns ESS count evidence. Proposed Connector migration ADR0040 and source-fidelity ADR0041 remain reviewable drafts in the owned Atlas tree. No acceptance, consumer migration, release or deployment is claimed.

Preflight: the integration tree contains only coordinator-owned planning/model changes being committed before tree creation. Unit branches already present belong to this same wave; the held credential tree remains outside the merge set after two red reviews. Pipe tree was finished and removed through reviewed exact-id manager GC after publication. Free disk measured24GB against20GB floor; sccache is configured, incremental/debug output disabled, build jobs3 per worker, and each tree owns its targets. Start the additive rate work after one-shot root compilation is idle; monitor the floor before another build. Exact rate tree/base/build/scratch triple is appended on creation before dispatch. Existing wave commit/push authorization applies; no tag/release.

Computed selection command: aep plan artifact waves --kind story --status proposed --format json; exit0, output recorded before rate moved active.

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
          "id": "story:rate-limit-in-the-protocol",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "catalog/connector-document-v3.schema.json"
            },
            {
              "confidence": "inferred",
              "path": "catalog/slack.catalog.json"
            },
            {
              "confidence": "inferred",
              "path": "connectors.lock"
            },
            {
              "confidence": "inferred",
              "path": "contracts/connector-operation/v0alpha2"
            },
            {
              "confidence": "cited",
              "path": "crates/catalog-build/src/document.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/catalog-build/src/document_schema.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/catalog-build/tests/main/catalog_invariants.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/catalog-reader/catalog.pack"
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
              "path": "crates/connector-spec/src/ir.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec/src/provider.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec/src/provider/declaration.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec/src/provider/publishing.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec/src/provider/schema_sync.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec/tests/main/determinism.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec/tests/main/ir_roundtrip.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec/tests/main/service_partition.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-cli/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-cli/tests/cli_surface.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-client/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-client/src/response.rs"
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
              "path": "crates/integration-kubernetes/src/hosted.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-kubernetes/src/local.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-kubernetes/src/local_inventory.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-kubernetes/src/workloads.rs"
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
              "confidence": "inferred",
              "path": "crates/protocol/src/operation/legacy.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/protocol/src/operation/wire.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/protocol/tests/bundles.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/egress.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/hosted.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/hosted/mcp/toolset.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/server/src/hosted/tests/contract_validation.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/hosted/tests/enforcement.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/hosted/tests/mcp.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/hosted/tests/mcp_monitoring.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/hosted/tests/signal.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server/src/local.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/service/src/egress.rs"
            },
            {
              "confidence": "cited",
              "path": "ess/system/domains/catalog.yaml"
            },
            {
              "confidence": "cited",
              "path": "ess/system/domains/runtime.yaml"
            },
            {
              "confidence": "cited",
              "path": "json-schemas.toml"
            },
            {
              "confidence": "inferred",
              "path": "providers/slack.toml"
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
    }
  ],
  "collisions": [
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
      "path": "crates/server/src/local.rs",
      "confidence": "cited"
    },
    {
      "a": "story:auth-as-tool-result",
      "b": "story:rate-limit-in-the-protocol",
      "path": "ess/system/domains/runtime.yaml",
      "confidence": "inferred"
    },
    {
      "a": "story:connect-session-oauth-custody-in-personal-posture",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/catalog-build/src/document.rs",
      "confidence": "cited"
    },
    {
      "a": "story:connect-session-oauth-custody-in-personal-posture",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/catalog/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:connect-session-oauth-custody-in-personal-posture",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/catalog/src/table.rs",
      "confidence": "cited"
    },
    {
      "a": "story:connect-session-oauth-custody-in-personal-posture",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/catalog/tests/main/consumer_api.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:connect-session-oauth-custody-in-personal-posture",
      "b": "story:rate-limit-in-the-protocol",
      "path": "crates/connector-spec/schema/provider-toml.schema.json",
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
    }
  ],
  "unassessed": [],
  "cycles": []
}
```

## Rate-limit managed triple

Rate unit created before dispatch as manager id wt-af054beacfba at `~/.local/state/worktree/trees/b10x/connectors/wt-af054beacfba`, branch `impl/rate-limit-in-the-protocol`, exact base/head `e101a24e73fe22a58cb67e0a2e7ac95a7b83b688`. Build roots are this tree's root `target/` and satellite crate `target/` directories only. Scratch `~/.cache/connectors-cli-wave-20260906/rate-limit-in-the-protocol`; TMPDIR `~/.cache/cw6/r`. Brief is scratch `brief.md`, source-only aep-drive:implementor charter in ordinary harness. Stage1 released owners and holds are the preceding staged-dispatch decision; no whole-unit independence or readiness claim before its handoffs. Opening cheap checks after bot commit: portable Markdown links,73legacy story records,10ESS files valid, diff whitespace clear. One-shot root compilation idle before rate stage1 begins.

## One-shot final attack

First attack review-result:cli-one-shot-adversary-1-20260906 is retained verbatim. The coordinator classifies its stale Monitoring citation finding as introduced: the unit added five lines before every cited function; exact base/source comparison and the first batch gate establish that change, without claiming a new base execution. The invalid extra-input-key probe remains INFEASIBLE and its original red and exact coordinator correction are retained.

Correction committed as17763bbc1552fcda2b9a323bfdd04f5fcef9bd22, bot author and committer verified. It retains eight adversary cases and changes37 model lines only in numeric source citations: introduced Monitoring/Registry shifts plus ten separately identified pre-existing Kubernetes offsets. The static audit inspected32 direct citations reaching four of19 changed unit paths; no lifecycle, fields, refusal vocabulary or test assertions were weakened.

After Monitoring correction the complete root workspace passed1141,0failed,4ignored (exit0), versus the first attack's1140pass/1fail. After the final Registry/Kubernetes citation-only corrections, all15 targeted ESS citation/claim fences passed,58nonselected (exit0); the expected ESS0.18.0 binary validated all10files. The shell's older PATH-first ESS refused the newer command/model; use the expected installed binary for final checks, do not alter the model to fit an older CLI. Affected full lanes from attack1 remain626passed,2ignored; final full attack must add its own cases and rerun the affected lanes.

Reports and exact patches are under `~/.cache/connectors-cli-wave-20260906/one-shot-operations-without-a-daemon/`; root correction logs under `~/.cache/cw6/p/`. Second/final test-only adversary receives the complete unit base3df1cd2df32472a4afc8feeaac8c46c9de8d1b55 to corrected17763bbc, preserved reports, and existing managed triple. No third attack or whole-wave closure is implied.

## Session recovery 2026-09-06

Recovered the operator-approved session 01a0737b-6eb5-79f1-9bd4-87292c6a8743 after its usage-limit interruption. The recovered ten-story scope and standing official-source/schema-fidelity instructions remain unchanged. This is an interactive continuation with three workers beside the coordinator, using the saved implementor/adversary charters through ordinary agents. No new release authorization is inferred.

Inspected integration HEAD e101a24e and its four coordinator-owned dirty planning files. Preserved their pending rate-limit generator scope and one-shot final-review records. AEP validation reports 161 artifacts, historical assertion/findings notices, and valid. Source state: one-shot corrected HEAD17763bbc awaits its second/final attack; GitLab source-fidelity HEAD50256e4b has three pending stage2 files; rate-limit stage1 retains five interrupted source/lock changes. Credential HEAD9f15c108 remains held by decision-blocker:cli-credentials-final-review, with its final tests preserved.

Atlas primary is dirty and stale; new managed authority wt-679b510698e5 is clean at exact advertised main f686fe034bc3f1c7c5d276476b814ee05c2bca9f. Its path is ~/.local/state/worktree/trees/b10x/atlas/wt-679b510698e5. Current Connectors remote main is a61e0a748be58ad03cf1907a48f9fefbb4423434 and wave remote50f0ab5107de9f07f42d9f2b73df80bcfe978e89. Both are observations, not merged-source claims.

Workers resume the existing one-shot final review, GitLab stage2, and additive rate-limit stage1. They retain their original managed tree/build/scratch triples and short TMPDIRs. No Git, AEP, model, service or live-provider mutations are delegated. Free space measured26GiB against20GiB floor; only the one-shot reviewer initially has the build slot. Connectors doctor is healthy, but operation search exposes no GitHub operation, so Atlas bot tooling remains the declared GitHub path.

## Remote main reconciliation

Integrated remote main a61e0a748be58ad03cf1907a48f9fefbb4423434 while preserving both published histories. Its seven source/lock files were applied from the exact merge-base diff. The eleven independent remote planning events for story:git-http-oauth-authentication and its immutable review were replayed sequentially through AEP under the current agent actor; both resulting artifact files were verified byte-for-byte against remote main. The merge retains the original remote event provenance through its second parent and avoids a textual append-only-journal merge. No new delivery or verification is claimed for that separate active story. Full wave verification remains pending.

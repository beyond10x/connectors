---
format: aep.planning-md/1
id: runbook:small-wave-after-0-7-0
kind: runbook
status: draft
title: 'Next small wave: inspect the installed binary'
tags:
- wave-small
relations:
- delivers: story:the-binary-says-what-it-carries
revision: 14
---
# Next small wave: inspect the installed binary

## Proposal and authorization

## Proposal and authorization

The user requested a smaller next wave on 2026-09-07 and then explicitly approved: “approved, do it, then cut release with that”. Approval is recorded in `approval-record:inspect-upgrade-and-release-20260907`. **N = 1** selects `story:the-binary-says-what-it-carries`; implementation and necessary verification are underway. The same instruction authorizes its commits, integration, publication, the subsequent v0.7.1 release, consumer documentation delivery, and safe cleanup of this run’s published managed trees. Existing recovery trees remain preserved.

The applied wave skill is version 0.8.0. The feature wave closes before release work begins; release delivery is separately tracked by `story:release-0-7-1` and its task. All direct commits and pushes use the clean, exact-current Atlas bot wrapper, with author and committer checked. No further publication approval is pending.

## Consumer outcome

`connectors inspect upgrade` tells a person what their installed binary carries: CLI version, embedded catalog schema/digest, supported credential formats and session metadata version, with existing installation guidance. It works without configured state or a running service.

This one command is the entire feature wave. It introduces no installer, release lookup, provider operation or protocol migration. The acceptance clarification explicitly reports supported v1/v2 credential capabilities and prepared-write behavior; it does not claim to inspect the format of a local credential file.

Objectives proposed: **O1 — governed reach** and **O5 — generic platform**, from `AGENTS.md:10–16` and Atlas ROADMAP at `d10b7484d64c28830774c9dae0ec531fcc47acb2`. Consumers can identify catalog/compatibility facts locally, without an external call. No local vision artifacts exist; the lifecycle/validator does not currently require adding a duplicate objective model.

## Selection and scope

## Selection and scope

Native selection used `aep plan artifact list`, `graph`, `blocked`, `scope` and `waves` with `protocol 0.54.0`. All 21 initially unassessed draft stories received read-only scoper reports, recorded through AEP together with typed scope; five proposed stories were already assessed. Full computed outputs and collisions remain verbatim in the stage-1 appendices. They are historical scheduling input, not the live execution status.

The selected singleton `story:the-binary-says-what-it-carries` is now active. Its prerequisite `story:cli-first-level-groups` is implemented. Its final 16 cited paths cover the CLI, console report, existing credential/session owners, CLI specification/design accounting and verification cases. The implementor confirmed all four initially inferred locations; coordinator source review had added the v2 owner, and measured Tokio startup added main.rs. The adversary added two test files. The story’s current Scope section preserves those corrections explicitly.

N=1 follows the user’s smaller-wave request. This is one consumer feature across four existing crates, with no dependency or domain entity added. The configuration-shape cleanup and broader provider/importer work remain outside this wave.

AEP compares declared path strings without directory containment. Its earlier computed set paired `story:m2-the-platform-skeleton-serves` at `crates/connectors-runtime` with `story:one-composed-local-placement` at files inside that directory. Coordinator source review found potential containment coupling. This limitation is reported without inventing a replacement scheduler; the selected singleton does not rely on that pair being safe.

## Deliberately excluded

- `story:one-connection-config-shape`: genuinely small, three cited files, high confidence; compatible consolidation can follow separately. User preference for a smaller wave caps this wave at one feature.
- `story:catalog-write-discovery`: requested behavior is already shipped; see `specification:catalog-write-discovery-already-present`. `story:console-clippy-findings` also appears stale against the released lint fixes and prior gate, so it is not presented as new implementation.
- `story:coverage-regains-its-second-direction`, GitLab creation, Kubernetes catalog migration, rate-limit headers and webhook routing: schema/importer or whole-provider work exceeds this wave. GitLab creation is explicitly an official-source multipart importer gap, not permission to hand-author schemas.
- M2/M3/M4, hosted catalog acquisition and Anthropic entry: substantial existing implementation; reconcile remaining acceptance/evidence before scheduling the original milestones again. The Anthropic scoper located residual verification-diagnosis and completion-message work, but event/posture acceptance remains unresolved.
- Hosted CLI setup, composed local placement, production custody, signed bundles and federation: prerequisite, external-owner or multi-package contract work remains. Raw proxy mixes omission protection with implementation of a new authority aperture.
- Existing proposed deployment packs, destination aperture, verification probes, source processing and claims-journal work remain broader alternatives, not additions to this feature.
- `story:reobserve-returns-a-withdrawn-observation-to-materialized`: unresolved behavior/ownership choice. Several credentials remains under its existing decision blocker. No blocker is cleared by this proposal.
- Atlas/Website release-feed repair is separately tracked and is not pulled into this Connectors wave.

These exclusions derive from the scoper reports recorded in the corresponding story Scope sections and coordinator reading of the existing bodies and graph. No new decomposition was created, so the decomposition critic panel was skipped.

## Checkout and execution record

## Checkout and execution record

Opening commit `a4bff98c6f6166529ad727179ecfd5828a765392` started from published main `4d0cd30872533da40f209274f936eaeae9bf01d7`. While planning, main advanced to `9f2a361b5751ac1d1fcbdce06a0a09e9ce741db1`. Integration commit `80c7666f4a36dc3b8902a1c8106a5edfe7afefe1` incorporates that source before unit dispatch. The native Git union merge of disjoint appended AEP histories validated successfully; details remain in Approved execution. Planning/source-doc checkpoint `7ae11444e04a6836d49cfcf1e23f8bc8382e287a` is published. Primary checkouts remain untouched.

Paths use `~` for the operator’s home. These are actual managed paths returned by the CLI.

| Role | Branch / base | Worktree | Build directory | Scratch | Stage |
|---|---|---|---|---|---|
| Coordinator | `wave/inspect-upgrade-after-0.7.0`, checkpoint `7ae11444` | `~/.local/state/worktree/trees/b10x/connectors/wt-83fe13d0c209` | Per-workspace in-tree targets; none created yet | `~/.cache/connectors-small-wave-20260907` | Published checkpoint; further planning edits pending |
| Binary inspection unit | `impl/the-binary-says-what-it-carries`, base `80c7666f` | `~/.local/state/worktree/trees/b10x/connectors/wt-inspect-upgrade-20260907` | `target`, `crates/connectors-cli/target`, `crates/connectors-console/target` | `~/.cache/cw7/upgrade` | Implementor verifying uncommitted feature |
| Atlas authority | detached `d10b7484d64c28830774c9dae0ec531fcc47acb2` | `~/.local/state/worktree/trees/b10x/atlas/wt-840d0a4626a0` | None | Coordinator scratch | Clean, matches remote main |
| Website delivery | `docs/connectors-0.7.1`, base `02271aa02022818a75a99ca3702c8cc00d135b1f` | `~/.local/state/worktree/trees/b10x/website/wt-connectors-071-docs-20260907` | Native Website build output | Coordinator scratch | AEP story active; npm ci succeeded |

The collaboration API has no subagent_type selector. Existing threads load the exact plugin charters: `scope_build_information` implements this unit as `aep-drive:implementor`; a separate reused thread will attack it as `aep-drive:adversary`. Only the coordinator writes AEP artifacts and direct Git commits. This is the recorded harness adaptation, with one compiler lane.

## Preflight observations and remaining conditions

Observed with `df -B1`, `/proc/meminfo`, environment inspection and `command -v sccache` on 2026-09-07: disk available **26,309,943,296 bytes**, disk **97% used**; tmpfs available **24,462,573,568 bytes**; MemAvailable **40,355,752 KiB**. Earlier in this proposal disk available was 29,378,314,240 bytes, so available capacity changed during read-only scoping and planning. Re-measure at launch; these are observations, not reserved capacity.

The prior published runbook `runbook:cli-ten-slack-first`, Preflight and resources, records one locked CLI no-run build: exit 0, **144 seconds**, **2.1 GB**, with sccache, three jobs, no incremental/debug output. That is historical measured cost, not a new benchmark or a promise for the full gate. `AGENTS.md`, The gate does not fit on one machine, records approximately **41 GB** for all standard workspace targets together. This machine cannot retain those outputs together while preserving the proposed **20 GB disk floor**.

`/usr/bin/sccache` exists and `RUSTC_WRAPPER` is currently unset: compilation is not launch-ready. Stage 2 must wire and verify the cache before a build, use one compiler lane, bounded jobs and no incremental/debug output, and execute the native gate’s workspace shards sequentially. Preserve logs before retiring only the wave’s owned reproducible targets. Keep each tree’s targets private. If a single measured lane cannot preserve the floor, stop before compilation and arrange adequate capacity; do not fill the disk or weaken tests. No new build directories were created during this proposal.

Budget preference persists from the previous runbook’s recorded unlimited subscription budget and the user’s subsequent “usage fixed”; it does not justify adding units. The harness currently has four total concurrency slots and a thread reuse limitation.

Two previous-wave recovery trees remain intentionally retained: `wt-2c3596f5baa5` at `8815dea456171dc28e3641668a3d7c50829b1b91` holds unpublished raw history; `wt-38354a193753` at `498d3618c542fd53b3a0c0de1ec6797160e6c04b` holds six frozen changed files. Other sessions also own linked trees. The wave skill’s previous-tree preflight refuses while these remain. This proposal does not clear that refusal or authorize deleting them. Starting a fresh wave therefore requires an explicit scoped exception preserving those recovery trees, plus inspection of their remaining build footprints. Run the managed CLI’s inspection/reconciliation paths; never force-remove or manually delete a managed tree.

## Verification and finish plan

Before the opening commit and unit provisioning, run AEP validation and the repository’s existing cheap Markdown/story checks. The chosen story’s tests cover real CLI output, absence of state access, both credential formats and mirrored CLI contract declarations. Use existing native AEP/worktree/ESS/gate commands; custom Python orchestration is unnecessary for this proposal.

Run all twelve workspace gate shards (including both runtime feature configurations), final catalog/documentation/ESS/projection checks and required strict checks with their individual exit statuses on the integrated source. Route adversary findings under the two-pass budget, preserve review findings blocks, and record actual evidence before any terminal lifecycle move. No compiler or product tests were run at stage 1; historical release results are not reused as evidence for new code.

Keep this page’s branch/HEAD/path/stage table current. Retire only wave-owned trees after wanted commits are published: `worktree finish`, complete-profile `worktree gc --dry-run`, inspect every record, and exact reviewed IDs to `--apply`. Unpublished proposal and recovery records stay available until safe publication and retirement are authorized.

## Computed draft candidates — verbatim

Command: `aep plan artifact waves --kind story --status draft --format json`, exit 0.

```json
{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
        {
          "id": "story:adopt-token-response-metadata",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/connector-spec"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-slack"
            },
            {
              "confidence": "cited",
              "path": "crates/protocol"
            },
            {
              "confidence": "cited",
              "path": "ess/system/domains/connection.yaml"
            },
            {
              "confidence": "inferred",
              "path": "providers/slack.toml"
            }
          ]
        },
        {
          "id": "story:catalog-write-discovery",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/connectors-runtime/tests/local_catalog_writes.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-catalog/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-catalog/src/tests.rs"
            }
          ]
        },
        {
          "id": "story:console-clippy-findings",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/connectors-console/src/enrol.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-console/src/envelope.rs"
            }
          ]
        },
        {
          "id": "story:contract-bundles-are-versioned-and-pinned",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": ".github/workflows/release.yml"
            },
            {
              "confidence": "cited",
              "path": "contracts"
            },
            {
              "confidence": "cited",
              "path": "crates/protocol/examples"
            },
            {
              "confidence": "inferred",
              "path": "crates/protocol/src"
            },
            {
              "confidence": "cited",
              "path": "crates/protocol/tests/bundles.rs"
            }
          ]
        },
        {
          "id": "story:m2-the-platform-skeleton-serves",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/catalog-build/tests/main"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-runtime"
            },
            {
              "confidence": "cited",
              "path": "crates/identity-http"
            },
            {
              "confidence": "cited",
              "path": "crates/server"
            },
            {
              "confidence": "cited",
              "path": "crates/service"
            }
          ]
        },
        {
          "id": "story:mint-source-entries-from-the-mined-catalogs",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "SOURCES.toml"
            },
            {
              "confidence": "cited",
              "path": "crates/catalog-build"
            },
            {
              "confidence": "cited",
              "path": "crates/catalog-cli"
            },
            {
              "confidence": "cited",
              "path": "docs/research/vendor"
            }
          ]
        },
        {
          "id": "story:one-composed-local-placement",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/connectors-cli/src/lib.rs"
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
              "path": "crates/connectors-runtime/src/one_shot.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-runtime/tests/one_shot_runtime.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/design/12-one-owner-for-every-outside-connection.md"
            }
          ]
        },
        {
          "id": "story:one-connection-config-shape",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/connectors-config/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-config/src/personal.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-platform/src/tests.rs"
            }
          ]
        },
        {
          "id": "story:reobserve-returns-a-withdrawn-observation-to-materialized",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/catalog-build/tests/main/ess_citation_fence.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-monitoring/src/backend.rs"
            }
          ]
        }
      ]
    },
    {
      "wave": 2,
      "artifacts": [
        {
          "id": "story:coverage-regains-its-second-direction",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "catalog"
            },
            {
              "confidence": "inferred",
              "path": "connectors.lock"
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
              "confidence": "inferred",
              "path": "crates/connector-spec"
            },
            {
              "confidence": "inferred",
              "path": "ess/system/domains/catalog.yaml"
            },
            {
              "confidence": "cited",
              "path": "providers"
            }
          ]
        },
        {
          "id": "story:m3-connect-a-provider-and-invoke-it",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/domain"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-catalog"
            },
            {
              "confidence": "cited",
              "path": "crates/protocol"
            },
            {
              "confidence": "cited",
              "path": "crates/server"
            },
            {
              "confidence": "cited",
              "path": "crates/service"
            }
          ]
        },
        {
          "id": "story:the-binary-says-what-it-carries",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/connector-secrets/src/file.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-secrets/src/file/prepared.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-cli/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-cli/tests/adversary_fence_probe.rs"
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
              "path": "crates/connectors-cli/tests/upgrade.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-client/src/identity.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-client/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-console/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-console/src/upgrade.rs"
            },
            {
              "confidence": "cited",
              "path": "docs/design/19-the-cli-surface.md"
            },
            {
              "confidence": "cited",
              "path": "ess/system/components.yaml"
            }
          ]
        },
        {
          "id": "story:the-hosted-posture-connects-a-catalogued-provider",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/connectors-config/src/hosted.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-runtime/src/composition.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/hosted-vault/src/prepared.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-catalog/src/hosted.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-catalog/src/lib.rs"
            }
          ]
        }
      ]
    },
    {
      "wave": 3,
      "artifacts": [
        {
          "id": "story:declarative-webhook-routing-grammar",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/catalog-build"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec"
            },
            {
              "confidence": "cited",
              "path": "docs/design"
            },
            {
              "confidence": "cited",
              "path": "ess/system/domains/event.yaml"
            },
            {
              "confidence": "inferred",
              "path": "providers"
            }
          ]
        },
        {
          "id": "story:production-credential-custody-closes",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/connector-secrets"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-runtime"
            },
            {
              "confidence": "cited",
              "path": "crates/hosted-secrets"
            },
            {
              "confidence": "cited",
              "path": "crates/service"
            },
            {
              "confidence": "cited",
              "path": "ess/system/domains/deployment.yaml"
            }
          ]
        },
        {
          "id": "story:raw-proxy-is-break-glass-not-a-model-capability",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/domain"
            },
            {
              "confidence": "inferred",
              "path": "crates/protocol"
            },
            {
              "confidence": "cited",
              "path": "crates/server"
            },
            {
              "confidence": "cited",
              "path": "crates/service/src/dispatch.rs"
            },
            {
              "confidence": "inferred",
              "path": "ess/system/domains/runtime.yaml"
            }
          ]
        },
        {
          "id": "story:the-cli-drives-a-hosted-connection",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/connectors-cli/src/lib.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-cli/tests/hosted_connect.rs"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-client/src/identity.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-client/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-console/src/auth.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-console/src/connect.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-console/src/enrol.rs"
            }
          ]
        }
      ]
    },
    {
      "wave": 4,
      "artifacts": [
        {
          "id": "story:gitlab-group-and-project-creation",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "catalog"
            },
            {
              "confidence": "cited",
              "path": "connectors.lock"
            },
            {
              "confidence": "cited",
              "path": "crates/catalog-build"
            },
            {
              "confidence": "cited",
              "path": "crates/catalog-cli/examples/vendor_gitlab.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/catalog-reader/catalog.pack"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-resolve"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-runtime/tests"
            },
            {
              "confidence": "cited",
              "path": "providers/gitlab.toml"
            },
            {
              "confidence": "cited",
              "path": "specs/gitlab.provenance.toml"
            },
            {
              "confidence": "cited",
              "path": "specs/gitlab/coverage-19.4.toml"
            }
          ]
        },
        {
          "id": "story:m4-events-reach-a-client-by-push-and-by-pull",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "contracts/connector-event"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-client"
            },
            {
              "confidence": "inferred",
              "path": "crates/connectors-runtime"
            },
            {
              "confidence": "cited",
              "path": "crates/protocol/src/event.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/server"
            },
            {
              "confidence": "cited",
              "path": "crates/service"
            },
            {
              "confidence": "cited",
              "path": "ess/system/domains/event.yaml"
            }
          ]
        }
      ]
    },
    {
      "wave": 5,
      "artifacts": [
        {
          "id": "story:header-name-rate-limit-retry",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/catalog-build"
            },
            {
              "confidence": "cited",
              "path": "crates/connector-spec"
            },
            {
              "confidence": "cited",
              "path": "providers/discord.toml"
            },
            {
              "confidence": "cited",
              "path": "providers/hubspot.toml"
            }
          ]
        },
        {
          "id": "story:satellite-federation-has-a-threat-modeled-contract",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/domain"
            },
            {
              "confidence": "inferred",
              "path": "crates/protocol"
            },
            {
              "confidence": "inferred",
              "path": "crates/server"
            },
            {
              "confidence": "inferred",
              "path": "crates/service"
            },
            {
              "confidence": "cited",
              "path": "docs/design/03-beyond-http.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 6,
      "artifacts": [
        {
          "id": "story:kubernetes-joins-the-catalog",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "crates/connector-spec"
            },
            {
              "confidence": "cited",
              "path": "crates/connectors-runtime/src/composition.rs"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-catalog"
            },
            {
              "confidence": "cited",
              "path": "crates/integration-kubernetes"
            },
            {
              "confidence": "cited",
              "path": "providers/kubernetes.toml"
            }
          ]
        }
      ]
    },
    {
      "wave": 7,
      "artifacts": [
        {
          "id": "story:the-anthropic-api-key-arrives-through-a-connect-session",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/integration-catalog"
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
        }
      ]
    }
  ],
  "collisions": [
    {
      "a": "story:adopt-token-response-metadata",
      "b": "story:coverage-regains-its-second-direction",
      "path": "crates/connector-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:adopt-token-response-metadata",
      "b": "story:declarative-webhook-routing-grammar",
      "path": "crates/connector-spec",
      "confidence": "cited"
    },
    {
      "a": "story:adopt-token-response-metadata",
      "b": "story:gitlab-group-and-project-creation",
      "path": "crates/connector-spec",
      "confidence": "cited"
    },
    {
      "a": "story:adopt-token-response-metadata",
      "b": "story:header-name-rate-limit-retry",
      "path": "crates/connector-spec",
      "confidence": "cited"
    },
    {
      "a": "story:adopt-token-response-metadata",
      "b": "story:kubernetes-joins-the-catalog",
      "path": "crates/connector-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:adopt-token-response-metadata",
      "b": "story:m3-connect-a-provider-and-invoke-it",
      "path": "crates/protocol",
      "confidence": "cited"
    },
    {
      "a": "story:adopt-token-response-metadata",
      "b": "story:raw-proxy-is-break-glass-not-a-model-capability",
      "path": "crates/protocol",
      "confidence": "inferred"
    },
    {
      "a": "story:adopt-token-response-metadata",
      "b": "story:satellite-federation-has-a-threat-modeled-contract",
      "path": "crates/protocol",
      "confidence": "inferred"
    },
    {
      "a": "story:catalog-write-discovery",
      "b": "story:the-hosted-posture-connects-a-catalogued-provider",
      "path": "crates/integration-catalog/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:console-clippy-findings",
      "b": "story:the-cli-drives-a-hosted-connection",
      "path": "crates/connectors-console/src/enrol.rs",
      "confidence": "cited"
    },
    {
      "a": "story:coverage-regains-its-second-direction",
      "b": "story:declarative-webhook-routing-grammar",
      "path": "crates/connector-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:coverage-regains-its-second-direction",
      "b": "story:declarative-webhook-routing-grammar",
      "path": "providers",
      "confidence": "inferred"
    },
    {
      "a": "story:coverage-regains-its-second-direction",
      "b": "story:gitlab-group-and-project-creation",
      "path": "catalog",
      "confidence": "inferred"
    },
    {
      "a": "story:coverage-regains-its-second-direction",
      "b": "story:gitlab-group-and-project-creation",
      "path": "connectors.lock",
      "confidence": "inferred"
    },
    {
      "a": "story:coverage-regains-its-second-direction",
      "b": "story:gitlab-group-and-project-creation",
      "path": "crates/catalog-reader/catalog.pack",
      "confidence": "inferred"
    },
    {
      "a": "story:coverage-regains-its-second-direction",
      "b": "story:gitlab-group-and-project-creation",
      "path": "crates/connector-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:coverage-regains-its-second-direction",
      "b": "story:header-name-rate-limit-retry",
      "path": "crates/connector-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:coverage-regains-its-second-direction",
      "b": "story:kubernetes-joins-the-catalog",
      "path": "crates/connector-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:declarative-webhook-routing-grammar",
      "b": "story:gitlab-group-and-project-creation",
      "path": "crates/catalog-build",
      "confidence": "cited"
    },
    {
      "a": "story:declarative-webhook-routing-grammar",
      "b": "story:gitlab-group-and-project-creation",
      "path": "crates/connector-spec",
      "confidence": "cited"
    },
    {
      "a": "story:declarative-webhook-routing-grammar",
      "b": "story:header-name-rate-limit-retry",
      "path": "crates/catalog-build",
      "confidence": "cited"
    },
    {
      "a": "story:declarative-webhook-routing-grammar",
      "b": "story:header-name-rate-limit-retry",
      "path": "crates/connector-spec",
      "confidence": "cited"
    },
    {
      "a": "story:declarative-webhook-routing-grammar",
      "b": "story:kubernetes-joins-the-catalog",
      "path": "crates/connector-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:declarative-webhook-routing-grammar",
      "b": "story:m4-events-reach-a-client-by-push-and-by-pull",
      "path": "ess/system/domains/event.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:declarative-webhook-routing-grammar",
      "b": "story:mint-source-entries-from-the-mined-catalogs",
      "path": "crates/catalog-build",
      "confidence": "cited"
    },
    {
      "a": "story:gitlab-group-and-project-creation",
      "b": "story:header-name-rate-limit-retry",
      "path": "crates/catalog-build",
      "confidence": "cited"
    },
    {
      "a": "story:gitlab-group-and-project-creation",
      "b": "story:header-name-rate-limit-retry",
      "path": "crates/connector-spec",
      "confidence": "cited"
    },
    {
      "a": "story:gitlab-group-and-project-creation",
      "b": "story:kubernetes-joins-the-catalog",
      "path": "crates/connector-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:gitlab-group-and-project-creation",
      "b": "story:mint-source-entries-from-the-mined-catalogs",
      "path": "crates/catalog-build",
      "confidence": "cited"
    },
    {
      "a": "story:header-name-rate-limit-retry",
      "b": "story:kubernetes-joins-the-catalog",
      "path": "crates/connector-spec",
      "confidence": "inferred"
    },
    {
      "a": "story:header-name-rate-limit-retry",
      "b": "story:mint-source-entries-from-the-mined-catalogs",
      "path": "crates/catalog-build",
      "confidence": "cited"
    },
    {
      "a": "story:kubernetes-joins-the-catalog",
      "b": "story:m3-connect-a-provider-and-invoke-it",
      "path": "crates/integration-catalog",
      "confidence": "cited"
    },
    {
      "a": "story:kubernetes-joins-the-catalog",
      "b": "story:one-composed-local-placement",
      "path": "crates/connectors-runtime/src/composition.rs",
      "confidence": "cited"
    },
    {
      "a": "story:kubernetes-joins-the-catalog",
      "b": "story:the-anthropic-api-key-arrives-through-a-connect-session",
      "path": "crates/integration-catalog",
      "confidence": "cited"
    },
    {
      "a": "story:kubernetes-joins-the-catalog",
      "b": "story:the-hosted-posture-connects-a-catalogued-provider",
      "path": "crates/connectors-runtime/src/composition.rs",
      "confidence": "cited"
    },
    {
      "a": "story:m2-the-platform-skeleton-serves",
      "b": "story:m3-connect-a-provider-and-invoke-it",
      "path": "crates/server",
      "confidence": "cited"
    },
    {
      "a": "story:m2-the-platform-skeleton-serves",
      "b": "story:m3-connect-a-provider-and-invoke-it",
      "path": "crates/service",
      "confidence": "cited"
    },
    {
      "a": "story:m2-the-platform-skeleton-serves",
      "b": "story:m4-events-reach-a-client-by-push-and-by-pull",
      "path": "crates/connectors-runtime",
      "confidence": "inferred"
    },
    {
      "a": "story:m2-the-platform-skeleton-serves",
      "b": "story:m4-events-reach-a-client-by-push-and-by-pull",
      "path": "crates/server",
      "confidence": "cited"
    },
    {
      "a": "story:m2-the-platform-skeleton-serves",
      "b": "story:m4-events-reach-a-client-by-push-and-by-pull",
      "path": "crates/service",
      "confidence": "cited"
    },
    {
      "a": "story:m2-the-platform-skeleton-serves",
      "b": "story:production-credential-custody-closes",
      "path": "crates/connectors-runtime",
      "confidence": "cited"
    },
    {
      "a": "story:m2-the-platform-skeleton-serves",
      "b": "story:production-credential-custody-closes",
      "path": "crates/service",
      "confidence": "cited"
    },
    {
      "a": "story:m2-the-platform-skeleton-serves",
      "b": "story:raw-proxy-is-break-glass-not-a-model-capability",
      "path": "crates/server",
      "confidence": "cited"
    },
    {
      "a": "story:m2-the-platform-skeleton-serves",
      "b": "story:satellite-federation-has-a-threat-modeled-contract",
      "path": "crates/server",
      "confidence": "inferred"
    },
    {
      "a": "story:m2-the-platform-skeleton-serves",
      "b": "story:satellite-federation-has-a-threat-modeled-contract",
      "path": "crates/service",
      "confidence": "inferred"
    },
    {
      "a": "story:m2-the-platform-skeleton-serves",
      "b": "story:the-anthropic-api-key-arrives-through-a-connect-session",
      "path": "crates/server",
      "confidence": "inferred"
    },
    {
      "a": "story:m2-the-platform-skeleton-serves",
      "b": "story:the-anthropic-api-key-arrives-through-a-connect-session",
      "path": "crates/service",
      "confidence": "inferred"
    },
    {
      "a": "story:m3-connect-a-provider-and-invoke-it",
      "b": "story:m4-events-reach-a-client-by-push-and-by-pull",
      "path": "crates/server",
      "confidence": "cited"
    },
    {
      "a": "story:m3-connect-a-provider-and-invoke-it",
      "b": "story:m4-events-reach-a-client-by-push-and-by-pull",
      "path": "crates/service",
      "confidence": "cited"
    },
    {
      "a": "story:m3-connect-a-provider-and-invoke-it",
      "b": "story:production-credential-custody-closes",
      "path": "crates/service",
      "confidence": "cited"
    },
    {
      "a": "story:m3-connect-a-provider-and-invoke-it",
      "b": "story:raw-proxy-is-break-glass-not-a-model-capability",
      "path": "crates/domain",
      "confidence": "inferred"
    },
    {
      "a": "story:m3-connect-a-provider-and-invoke-it",
      "b": "story:raw-proxy-is-break-glass-not-a-model-capability",
      "path": "crates/protocol",
      "confidence": "inferred"
    },
    {
      "a": "story:m3-connect-a-provider-and-invoke-it",
      "b": "story:raw-proxy-is-break-glass-not-a-model-capability",
      "path": "crates/server",
      "confidence": "cited"
    },
    {
      "a": "story:m3-connect-a-provider-and-invoke-it",
      "b": "story:satellite-federation-has-a-threat-modeled-contract",
      "path": "crates/domain",
      "confidence": "cited"
    },
    {
      "a": "story:m3-connect-a-provider-and-invoke-it",
      "b": "story:satellite-federation-has-a-threat-modeled-contract",
      "path": "crates/protocol",
      "confidence": "inferred"
    },
    {
      "a": "story:m3-connect-a-provider-and-invoke-it",
      "b": "story:satellite-federation-has-a-threat-modeled-contract",
      "path": "crates/server",
      "confidence": "inferred"
    },
    {
      "a": "story:m3-connect-a-provider-and-invoke-it",
      "b": "story:satellite-federation-has-a-threat-modeled-contract",
      "path": "crates/service",
      "confidence": "inferred"
    },
    {
      "a": "story:m3-connect-a-provider-and-invoke-it",
      "b": "story:the-anthropic-api-key-arrives-through-a-connect-session",
      "path": "crates/integration-catalog",
      "confidence": "cited"
    },
    {
      "a": "story:m3-connect-a-provider-and-invoke-it",
      "b": "story:the-anthropic-api-key-arrives-through-a-connect-session",
      "path": "crates/server",
      "confidence": "inferred"
    },
    {
      "a": "story:m3-connect-a-provider-and-invoke-it",
      "b": "story:the-anthropic-api-key-arrives-through-a-connect-session",
      "path": "crates/service",
      "confidence": "inferred"
    },
    {
      "a": "story:m4-events-reach-a-client-by-push-and-by-pull",
      "b": "story:production-credential-custody-closes",
      "path": "crates/connectors-runtime",
      "confidence": "inferred"
    },
    {
      "a": "story:m4-events-reach-a-client-by-push-and-by-pull",
      "b": "story:production-credential-custody-closes",
      "path": "crates/service",
      "confidence": "cited"
    },
    {
      "a": "story:m4-events-reach-a-client-by-push-and-by-pull",
      "b": "story:raw-proxy-is-break-glass-not-a-model-capability",
      "path": "crates/server",
      "confidence": "cited"
    },
    {
      "a": "story:m4-events-reach-a-client-by-push-and-by-pull",
      "b": "story:satellite-federation-has-a-threat-modeled-contract",
      "path": "crates/server",
      "confidence": "inferred"
    },
    {
      "a": "story:m4-events-reach-a-client-by-push-and-by-pull",
      "b": "story:satellite-federation-has-a-threat-modeled-contract",
      "path": "crates/service",
      "confidence": "inferred"
    },
    {
      "a": "story:m4-events-reach-a-client-by-push-and-by-pull",
      "b": "story:the-anthropic-api-key-arrives-through-a-connect-session",
      "path": "crates/server",
      "confidence": "inferred"
    },
    {
      "a": "story:m4-events-reach-a-client-by-push-and-by-pull",
      "b": "story:the-anthropic-api-key-arrives-through-a-connect-session",
      "path": "crates/service",
      "confidence": "inferred"
    },
    {
      "a": "story:one-composed-local-placement",
      "b": "story:the-binary-says-what-it-carries",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:one-composed-local-placement",
      "b": "story:the-cli-drives-a-hosted-connection",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:one-composed-local-placement",
      "b": "story:the-hosted-posture-connects-a-catalogued-provider",
      "path": "crates/connectors-runtime/src/composition.rs",
      "confidence": "cited"
    },
    {
      "a": "story:production-credential-custody-closes",
      "b": "story:satellite-federation-has-a-threat-modeled-contract",
      "path": "crates/service",
      "confidence": "inferred"
    },
    {
      "a": "story:production-credential-custody-closes",
      "b": "story:the-anthropic-api-key-arrives-through-a-connect-session",
      "path": "crates/service",
      "confidence": "inferred"
    },
    {
      "a": "story:raw-proxy-is-break-glass-not-a-model-capability",
      "b": "story:satellite-federation-has-a-threat-modeled-contract",
      "path": "crates/domain",
      "confidence": "inferred"
    },
    {
      "a": "story:raw-proxy-is-break-glass-not-a-model-capability",
      "b": "story:satellite-federation-has-a-threat-modeled-contract",
      "path": "crates/protocol",
      "confidence": "inferred"
    },
    {
      "a": "story:raw-proxy-is-break-glass-not-a-model-capability",
      "b": "story:satellite-federation-has-a-threat-modeled-contract",
      "path": "crates/server",
      "confidence": "inferred"
    },
    {
      "a": "story:raw-proxy-is-break-glass-not-a-model-capability",
      "b": "story:the-anthropic-api-key-arrives-through-a-connect-session",
      "path": "crates/server",
      "confidence": "inferred"
    },
    {
      "a": "story:satellite-federation-has-a-threat-modeled-contract",
      "b": "story:the-anthropic-api-key-arrives-through-a-connect-session",
      "path": "crates/server",
      "confidence": "inferred"
    },
    {
      "a": "story:satellite-federation-has-a-threat-modeled-contract",
      "b": "story:the-anthropic-api-key-arrives-through-a-connect-session",
      "path": "crates/service",
      "confidence": "inferred"
    },
    {
      "a": "story:the-binary-says-what-it-carries",
      "b": "story:the-cli-drives-a-hosted-connection",
      "path": "crates/connectors-cli/src/lib.rs",
      "confidence": "cited"
    },
    {
      "a": "story:the-binary-says-what-it-carries",
      "b": "story:the-cli-drives-a-hosted-connection",
      "path": "crates/connectors-client/src/identity.rs",
      "confidence": "inferred"
    },
    {
      "a": "story:the-binary-says-what-it-carries",
      "b": "story:the-cli-drives-a-hosted-connection",
      "path": "crates/connectors-client/src/lib.rs",
      "confidence": "inferred"
    }
  ],
  "unassessed": [],
  "cycles": []
}
```

## Computed proposed candidates — verbatim

Command: `aep plan artifact waves --kind story --status proposed --format json`, exit 0.

```json
{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
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

## Stage-1 validation

The native `aep plan artifact validate` returned exit 0: 183 artifacts, valid. It retained 35 historical assertion-closed stories and nine historical reviews without findings blocks; none was created or silently repaired by this proposal. Existing repository checks printed `markdown links are repository-portable` and `story index and 73 records are consistent`. `git diff --check` found no whitespace defects. Only the planning store and its AEP journal changed; two new draft records hold this proposal and the already-delivered discovery finding. No product code, generated artifacts, dependency manifests or release files changed. Final validation after recording this section is run separately.

## Approved execution — 2026-09-07

The operator approved implementation and the subsequent release cut: "approved, do it, then cut release with that". Approval includes bot-authenticated source/tag publication and documentation delivery. The retained recovery-tree exception is accepted with preservation; no unrelated checkout or historical evidence is a cleanup target. Target release is the next available nonbreaking patch, v0.7.1.

Remote Connectors main advanced to 9f2a361b5751ac1d1fcbdce06a0a09e9ce741db1, adding subscription OAuth recovery and generated-service remediation fixes in PRs 20 and 22. Their paths do not overlap the approved feature. Rebase the release base through an ordinary merge and validate the combined AEP journal before publication. Native Git union merge is restricted to the append-only journal after confirming the incoming artifact IDs are disjoint; AEP remains the writer of artifact mutations and its validator decides journal consistency.

The opening branch is wave/inspect-upgrade-after-0.7.0. Current disk is 26,729,488,384 bytes free, 97% used; tmpfs has 24,226,791,424 bytes free. Use one compiler lane, one Cargo job, no incremental/debug output and a verified sccache wrapper. Full workspace testing will use the repository's existing sharded release rehearsal, avoiding simultaneous local targets that cannot fit. Local focused lanes preserve the 20 GB disk floor. All per-lane exits and counts are retained in assigned scratch.

Connectors 0.7.0 doctor is healthy, but local operation search returned no admitted GitHub workflow/release operations. The capability gap was reported before using Atlas bot-authenticated GitHub tooling. No custom Python coordinator is introduced.

## Unit dispatched

Opening commit a4bff98c6f6166529ad727179ecfd5828a765392 and merged base 80c7666f4a36dc3b8902a1c8106a5edfe7afefe1 have verified bot author and committer. The ordinary Git union merge preserved disjoint journal histories; native AEP validation returned valid for187artifacts. Portable Markdown, legacy story checks and whitespace checks passed before dispatch.

The actual unit is impl/the-binary-says-what-it-carries at80c7666f4a36dc3b8902a1c8106a5edfe7afefe1 in ~/.local/state/worktree/trees/b10x/connectors/wt-inspect-upgrade-20260907. The manager initially resolved HEAD against the primary repository; before dispatch the clean unit was explicitly switched to the exact merged base. Its target directories are root target/, crates/connectors-cli/target and crates/connectors-console/target inside that tree; scratch/TMPDIR and brief are ~/.cache/cw7/upgrade and ~/.cache/cw7/upgrade/brief.md. Stage: implementation, changes left uncommitted for independent review.

A private sccache server is active on127.0.0.1:49387 with cache ~/.cache/cw7/sccache, maximum1GiB. Wrapper execution of rustc succeeded. Owned unit targets may occupy at most4GiB together while preserving20GB free disk; one compiler job, no debug/incremental output. No retained recovery tree has an in-tree target directory. Recovery wt-2c3596f5baa5 is clean; wt-38354a193753 retains precisely these six changes, all preserved:

```text
 M crates/connectors-cli/src/lib.rs
 M crates/connectors-cli/tests/remediation.rs
 M crates/connectors-client/src/remediation.rs
 M crates/connectors-client/src/tests.rs
 M crates/connectors-console/src/remediation.rs
 M crates/connectors-console/tests/remediation.rs
```

The CLI/lib.rs and client/lib.rs area can overlap conceptually with this wave, but the preserved tree is frozen history, not an implementation branch to merge. The new unit starts from published main and does not copy those changes. This is the explicitly approved recovery preservation exception.

Default PATH selected ESS0.9.2, which refused the required specify command. The exact pinned ESS0.18.0 at ~/.cache/ess-rel/ess-0.18.0-x86_64-unknown-linux-gnu/ess validated the base: connectors v1 —10file(s), valid; compile succeeded. Coordinator uses that version for the projection check. Cargo set-version is installed and its offline root-workspace dry-run supports the approved0.7.1 identity bump; no version bytes have changed yet.

## Runtime-startup scope correction, 2026-09-07

The implementor measured an AF_UNIX socket pair created by unconditional Tokio startup in `crates/connectors-cli/src/main.rs:5`, before command dispatch. Its pre-change executable also fails with exit 101 when `TOKIO_WORKER_THREADS=0`; the raw evidence is retained in the unit scratch `red-runtime-startup.log`. This violates the accepted socket-free diagnostic behavior even though the report itself is pure. Add `crates/connectors-cli/src/main.rs` as cited scope: use the same clap tree to dispatch this synchronous report before starting Tokio, while preserving the public async embedding entry point and normal fallback behavior. The coordinator applies the implementor's reviewed main-entry patch; this is a bounded implementation correction within the approved story.

## ESS projection verification

Coordinator ran pinned ESS 0.18.0 against the completed unit specification comments. `specify validate --path ess/system` exited 0 and printed `connectors v1 — 10 file(s), valid`. `generate synthesize --path ess/system --target clap` exited 0 and printed `207 capabilities: 191 generated, 16 obligation(s), 0 refused` and `8 artifact(s)`. After removing only the uncommitted plan/target sidecars, `diff -ru ess/generated/clap <fresh-output>` exited 0 with no differences. No generated source was hand-edited.

## Implementor handoff

Unit commit: `1b1cf58c27d34f5cda4a376f8815ebcb6db3e0e8`; both author and committer verified as b10x-bot[bot]. The report below is retained with only the local home prefix rendered as `~/` for repository portability; raw output remains in the assigned scratch. Harness token/tool-use metrics were not exposed, so no numbers are inferred.

unit:                   story:the-binary-says-what-it-carries — The binary says what it carries
verdict:                green
cases:                  executed 376→383, red 4
origin:                 n/a
wrote-outside-worktree: ~/.cache/cw7/upgrade/ (full inventory in part 6); assigned private compiler cache ~/.cache/cw7/sccache/
needs-coordinator:      no

Root stayed 125→125 because no root cases were added. All seven added cases executed in CLI/console. Three pre-existing live Vault cases remain ignored in both root runs. The coordinator requested this concise handoff with raw-log paths; all complete native outputs are retained.

## 1. Unit and confirmed scope

`connectors inspect upgrade` reports compiled CLI/catalog/credential/session facts and source-checkout installation guidance in text, compact, JSON and YAML, without configuration/state access, sockets or external calls; eight groups and existing serialized formats are preserved.

Base: `80c7666f4a36dc3b8902a1c8106a5edfe7afefe1`. Worktree: `~/.local/state/worktree/trees/b10x/connectors/wt-inspect-upgrade-20260907`. The complete story and graph were read before edits; prerequisite `story:cli-first-level-groups` is implemented.

| Scope hypothesis | Source confirmation |
| --- | --- |
| Inferred console registration | Confirmed existing registry; new registration `crates/connectors-console/src/lib.rs:47`. |
| Inferred console report owner | Confirmed reporting pattern in `crates/connectors-console/src/providers.rs`; new report `crates/connectors-console/src/upgrade.rs:11`. |
| Inferred session export | Existing identity exports `crates/connectors-client/src/lib.rs:28`; alias at line 31 uses owning constant `identity.rs:37`. |
| Inferred real CLI test location | Confirmed existing integration suite; new tests `crates/connectors-cli/tests/upgrade.rs:67,106,122,142,159,179`. |
| Cited parser and output owner | `crates/connectors-cli/src/lib.rs:235,769,817,853,962`; shared output handling preserves closed-pipe success and other write failures. |
| Cited credential owners | `crates/connector-secrets/src/file.rs:102` exports both formats; `file/prepared.rs:19,21` derives v2 from the unchanged parser/writer header. |
| Cited catalog owner, unchanged | `crates/catalog-reader/src/lib.rs:316,321,561`; metadata comes from embedded bytes. |
| Cited contract/accounting | Three assigned exception copies updated; `ess/system/components.yaml:172` and `docs/design/19-the-cli-surface.md:83,111,121,273` account for 28 exceptions. |
| Measured scope correction | Unconditional Tokio in `crates/connectors-cli/src/main.rs` created a socketpair. Coordinator applied the exact `main-runtime.patch`; main now uses the same clap tree before constructing Tokio. |

Every inferred location was confirmed; none was wrong. The approved brief had already resolved credential-format ambiguity. Public async `run_from`, moved-path notices, help/errors and the normal fallback remain; no broad startup refactoring was made.

## 2. Actual diff shape

The coordinator has begun committing the frozen source. This Git measurement compares the reviewed worktree with the assigned base, covering staged or committed changes as well:

`git --no-pager diff --stat 80c7666f4a36dc3b8902a1c8106a5edfe7afefe1`

```text
 crates/connector-secrets/src/file.rs               |   9 +
 crates/connector-secrets/src/file/prepared.rs      |   2 +
 crates/connectors-cli/src/lib.rs                   |  46 ++++-
 crates/connectors-cli/src/main.rs                  |  16 +-
 .../connectors-cli/tests/adversary_fence_probe.rs  |   5 +
 crates/connectors-cli/tests/cli_surface.rs         |   9 +-
 crates/connectors-cli/tests/cli_surface_drift.rs   |   5 +
 crates/connectors-cli/tests/upgrade.rs             | 194 +++++++++++++++++++++
 crates/connectors-client/src/identity.rs           |   3 +-
 crates/connectors-client/src/lib.rs                |   1 +
 crates/connectors-console/src/lib.rs               |   2 +
 crates/connectors-console/src/upgrade.rs           |  61 +++++++
 docs/design/19-the-cli-surface.md                  |  18 +-
 ess/system/components.yaml                         |   3 +-
 14 files changed, 360 insertions(+), 14 deletions(-)
```

Before coordinator staging, `git --no-pager diff --stat` reported 12 tracked files, 105 insertions and 14 deletions. Native `git diff --no-index --stat` separately measured the new CLI test file at 194 lines and console report at 61 lines. Total scope: 14 source paths; main.rs was coordinator-applied.

## 3. Red evidence

Before production edits, `cargo test --manifest-path crates/connectors-cli/Cargo.toml --locked --test upgrade` executed five cases: one passed, four failed, exit 101. Complete command/output/resources: `~/.cache/cw7/upgrade/red-upgrade.log`.

```text
status: exit status: 2; stderr: error: unrecognized subcommand 'upgrade'

Usage: connectors inspect [OPTIONS] <COMMAND>

For more information, try '--help'.

test result: FAILED. 1 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

After the first implementation, the native syscall verifier exited 1 on this startup defect. Full original evidence remains in `~/.cache/cw7/upgrade/trace-verification.log` and `~/.cache/cw7/upgrade/upgrade-syscalls.log`:

```text
243783 socketpair(AF_UNIX, SOCK_STREAM|SOCK_CLOEXEC|SOCK_NONBLOCK, 0, [6, 7]) = 0
```

The added no-runtime regression has a native pre-patch reproduction retained in `~/.cache/cw7/upgrade/red-runtime-startup.log`: `env TOKIO_WORKER_THREADS=0 crates/connectors-cli/target/debug/connectors inspect upgrade --output json` panicked before dispatch and exited 101.

Correction class: a compiled report must execute before runtime I/O initialization. This leaf now dispatches through clap before Tokio; all four output modes pass with Tokio startup made invalid. Existing commands retain the async path and pass the full CLI suite. The final real-process trace has no socket/network call.

## 4. Final verification

Counts are sums of the runner's own `test result:` summaries for identical before/after commands, excluding ignored cases. Each log includes its exact command, full output, exit and disk/target measurements. Raw logs are under `~/.cache/cw7/upgrade/`.

| Exact command | Executed before → after | Exit | Raw logs |
| --- | --- | --- | --- |
| `cargo test --manifest-path crates/connectors-cli/Cargo.toml --workspace --locked` | 142 → 148 | 0 | `baseline-cli.log`, `green-cli-final.log` |
| `cargo test --manifest-path crates/connectors-console/Cargo.toml --workspace --locked` | 109 → 110 | 0 | `baseline-console.log`, `green-console.log` |
| `cargo test --locked -p connector-secrets -p connectors-client` | 125 → 125 | 0 | `baseline-root.log`, `green-root.log` |
| `cargo test --manifest-path crates/connectors-cli/Cargo.toml --locked --test upgrade` | 5 → 6 | 0 | `red-upgrade.log` (verifier on base), `green-upgrade-final.log` |
| `cargo clippy --locked -p connector-secrets -p connectors-client --all-targets -- -D warnings` | n/a | 0 | `clippy-root.log` |
| `cargo clippy --manifest-path crates/connectors-console/Cargo.toml --workspace --locked --all-targets -- -D warnings` | n/a | 0 | `clippy-console.log` |
| `cargo clippy --manifest-path crates/connectors-cli/Cargo.toml --workspace --locked --all-targets -- -D warnings` | n/a | 0 | `clippy-cli.log` |
| `cargo fmt --all --check` | n/a | 0 | `fmt.log` |
| `cargo fmt --manifest-path crates/connectors-cli/Cargo.toml --all --check` | n/a | 0 | `fmt.log` |
| `cargo fmt --manifest-path crates/connectors-console/Cargo.toml --all --check` | n/a | 0 | `fmt.log` |
| `git --no-pager diff --check` | n/a | 0 | `final-resources.log` |

Earlier passing iterations are retained in `green-upgrade.log` (5 cases) and `green-cli.log` (147 cases), before the runtime case was added. They are not the final counts.

Final syscall verification: `~/.cache/cw7/upgrade/green-trace-verification.log`, command and verifier exits 0. Full trace: `~/.cache/cw7/upgrade/upgrade-syscalls-final.log`. Only loader cache/libraries and `/proc/self/maps` are opened; no configuration, credential or session files, state writes, or socket/network syscalls occur. Final JSON is `~/.cache/cw7/upgrade/upgrade-report-final.json`; `~/.cache/cw7/upgrade/upgrade-stderr-final.log` is empty.

The coordinator separately ran pinned ESS 0.18.0 validation and regeneration comparison: committed clap bytes were unchanged. Its evidence is `~/.cache/cw7/upgrade/ess-result.md`; this implementor did not run the generator.

Final resources and whitespace check, verbatim:

```text
git --no-pager diff --check
exit: 0
Filesystem        1B-blocks         Used    Available Use% Mounted on
/dev/nvme0n1p2 910126964736 763622264832 100197347328  89% /
863875072	target
2602455040	crates/connectors-cli/target
1278705664	crates/connectors-console/target
Combined owned targets: 4745035776 bytes (4.419 GiB)
```

## 5. Deliberately not done

- No implementor Git/AEP mutation, commit, publication or release operation. Source edits are frozen and ownership is handed back to the coordinator.
- No generated edit, new dependency/entity, credential/session byte change, local-file diagnosis, updater or release lookup.
- No provider/live Vault call; existing ignored Vault cases are unchanged and local suites use synthetic fixtures.
- The full 12-workspace release gate and adversary belong to the coordinator's next stage; this report does not claim they ran here.
- No worktree or target cleanup. The sole compiler lane is idle and handed back.

## 6. External artifacts

Full retained paths, including synthetic test fixtures and coordinator artifacts sharing this scratch directory: `~/.cache/cw7/upgrade/external-paths.txt`. Native temporary children removed by their own fixture destructors stayed under the assigned TMPDIR.

Implementor-authored retained artifacts:

- ~/.cache/cw7/upgrade/planning-graph.dot
- ~/.cache/cw7/upgrade/upgrade-test.rs
- ~/.cache/cw7/upgrade/baseline-cli.log
- ~/.cache/cw7/upgrade/baseline-console.log
- ~/.cache/cw7/upgrade/baseline-root.log
- ~/.cache/cw7/upgrade/red-upgrade.log
- ~/.cache/cw7/upgrade/red-runtime-startup.log
- ~/.cache/cw7/upgrade/green-upgrade.log
- ~/.cache/cw7/upgrade/green-upgrade-final.log
- ~/.cache/cw7/upgrade/green-cli.log
- ~/.cache/cw7/upgrade/green-cli-final.log
- ~/.cache/cw7/upgrade/green-console.log
- ~/.cache/cw7/upgrade/green-root.log
- ~/.cache/cw7/upgrade/clippy-root.log
- ~/.cache/cw7/upgrade/clippy-console.log
- ~/.cache/cw7/upgrade/clippy-cli.log
- ~/.cache/cw7/upgrade/fmt.log
- ~/.cache/cw7/upgrade/trace-verification.log
- ~/.cache/cw7/upgrade/green-trace-verification.log
- ~/.cache/cw7/upgrade/upgrade-syscalls.log
- ~/.cache/cw7/upgrade/upgrade-syscalls-final.log
- ~/.cache/cw7/upgrade/upgrade-report.json
- ~/.cache/cw7/upgrade/upgrade-report-final.json
- ~/.cache/cw7/upgrade/upgrade-stderr.log
- ~/.cache/cw7/upgrade/upgrade-stderr-final.log
- ~/.cache/cw7/upgrade/main-runtime.patch
- ~/.cache/cw7/upgrade/final-resources.log
- ~/.cache/cw7/upgrade/external-paths.txt
- ~/.cache/cw7/upgrade/implementation-report.md

Cargo also used the explicitly assigned private cache `~/.cache/cw7/sccache/`, under the coordinator's environment and 1 GiB cap. Build artifacts stayed in the three permitted worktree targets; no alternate target directory or compiler lane was used.

## Adversary pass 1 dispatch


Separate thread resume_gitlab loads the aep-drive:adversary charter against the clean committed unit above. It owns only test changes and one compiler lane, reusing the unit targets. Its assigned scratch is `~/.cache/cw7/upgrade/adversary-1`; raw brief and report stay there. Baseline counts are the implementor’s 148 CLI, 110 console and 125 root cases. No AEP/Git/implementation mutations are delegated.

## Adversary environment correction

Pass 1 wrote four CLI candidates and one console candidate before running them. All four CLI probes passed alone. The first console attempt correctly hit the existing private-directory rule because its fixture was mode 0755; the test alone now sets its owned directory to 0700. That is fixture setup, not a product defect.

The first broader CLI run used the longer report scratch as TMPDIR and executed 152 cases: 150 passed, two existing doctor fixtures failed, one with a 115-byte Connect Session socket path against a 107-byte limit. The implementor's shorter-TMPDIR suite had already passed these unchanged cases. The coordinator therefore assigned a new private TMPDIR `~/.cache/cw7/a1`, created at mode 0700, for remaining fixtures. Reports remain under `~/.cache/cw7/upgrade/adversary-1`; both are this run's owned cleanup roots. Raw failed outputs are preserved. This is an environment correction, not a product finding.

The four-mode native syscall probe is retained in review scratch rather than committed as an unconditional `/usr/bin/strace` dependency of the Rust suite. Its actual successful executions remain evidence; no skip or ignore is added. Final retained regressions are three CLI cases and one console storage-transition case, and final suite counts must reflect that source.

## Concurrent release preparation

Read-only inspection found another active managed checkout, `~/.local/state/worktree/trees/b10x/connectors/connectors-upgrade-20260907`, based on `9f2a361b5751ac1d1fcbdce06a0a09e9ce741db1`, with uncommitted Cargo version 0.7.1 and a CHANGELOG section for broader incremental Jira/Confluence/GitLab reads. Those changes are owned by another session; this run does not copy, reset, merge or clean that worktree.

v0.7.1 remains this task's provisional release number, not a reserved tag. The coordinator will recheck remote main and published tags immediately before release work. If v0.7.1 has already shipped, this approved feature uses the next available patch release, with planning and notes updated to the actual version. No published tag is replaced. The user authorized releasing the approved feature, without requiring a particular version number, and has been informed of this concurrency handling.

## Green unit integration checkpoint

Separate adversary thread resume_gitlab returned no product findings and released the compiler lane. Final native suites executed 151 CLI and 111 console cases, all passing; strict Clippy and formatting exited 0. The final test fixture lifetime correction was checked by its single case, test-target Clippy and formatting without repeating the full suite. The raw report is being finalized in the assigned review scratch; no further source/build work is delegated.

Retained independent cases are committed at `157ef0cc3ad8102b4c20d13c24dfb52d996e3aa8`, following implementation `1b1cf58c27d34f5cda4a376f8815ebcb6db3e0e8`. Both commits have the verified bot author and committer. No product correction or second attack was required. Integration proceeds on that green unit, with the full 12-workspace native release-workflow rehearsal next. The release number is still provisional pending concurrent publication.

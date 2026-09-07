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
revision: 3
---
# Next small wave: inspect the installed binary

## Proposal and authorization

Interactive stage 1, requested on 2026-09-07: suggest the next wave and keep it smaller. **N = 1**, selecting `story:the-binary-says-what-it-carries`. Implementation has not started; no new-wave commits, pushes or releases have been made.

**Skill version 0.8.0** — the version in `.claude-plugin/plugin.json`; the stage-1 proposal quotes it.

Approval authorizes the opening planning commit, one reviewed unit commit (with necessary corrections), its integration merge, the closing evidence/store commit, and merge into the base after the full gate; it authorizes no push, tag, version bump, release or second wave.

The user’s managed-worktree and bot-delivery instructions govern all execution. Direct commits must use the verified current Atlas `scripts/as-bot.sh`, with both author and committer checked. Publication needs its own authorization before `worktree finish` can safely retire wanted local commits. The completed v0.7.0 release grant is not reused for new work.

## Consumer outcome

`connectors inspect upgrade` tells a person what their installed binary carries: CLI version, embedded catalog schema/digest, supported credential formats and session metadata version, with existing installation guidance. It works without configured state or a running service.

This one command is the entire feature wave. It introduces no installer, release lookup, provider operation or protocol migration. The acceptance clarification explicitly reports supported v1/v2 credential capabilities and prepared-write behavior; it does not claim to inspect the format of a local credential file.

Objectives proposed: **O1 — governed reach** and **O5 — generic platform**, from `AGENTS.md:10–16` and Atlas ROADMAP at `d10b7484d64c28830774c9dae0ec531fcc47acb2`. Consumers can identify catalog/compatibility facts locally, without an external call. No local vision artifacts exist; the lifecycle/validator does not currently require adding a duplicate objective model.

## Selection and scope

Native selection path: `aep plan artifact list`, `graph`, `blocked`, `scope`, and `waves`, using `aep --version` output `protocol 0.54.0`. Every one of the 21 initially unassessed draft stories received a read-only scoper report. The coordinator recorded each Scope section and its typed entries through AEP. Existing five proposed stories were already assessed. Full recalculated outputs follow verbatim in the appendices, including every collision and the empty unassessed/cycle sets.

The selected story is draft, has no declared blocker, and its sole declared prerequisite `story:cli-first-level-groups` is implemented. It appears in computed draft wave 2. Scope is **medium confidence, mixed cited/inferred**: CLI parser and contract tests, console module/export, credential-format exports, session metadata exports, the existing ESS exception enumeration and design accounting. The exact 13 paths are in the story’s typed scope and appendix. The coordinator added the v2 prepared-store source after reviewing the scoper’s v1-only authority reference; the original report and correction are both retained.

One package would be smaller, but this is one bounded consumer feature across four existing crates, with no new dependency. N=1 limits implementation and review concurrency. The small configuration-shape cleanup is a reasonable later wave; it is deliberately not added merely because its files are disjoint.

Computed sets are scheduling input, not acceptance decisions. AEP compares declared path strings without directory containment: for example, it puts `story:m2-the-platform-skeleton-serves` (`crates/connectors-runtime`) with `story:one-composed-local-placement` (files under that directory). Source review finds potential containment coupling. That limitation is reported here without replacing the computed output or inventing another scheduler; the selected singleton does not rely on that pair being safe.

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

Connectors base is exact published remote main `4d0cd30872533da40f209274f936eaeae9bf01d7`. The primary was clean but stale; it was not changed. Current proposal tree was created by `worktree create` at that exact base. Atlas authority was checked against `git ls-remote origin refs/heads/main`: its clean managed HEAD is `d10b7484d64c28830774c9dae0ec531fcc47acb2`; the dirty/stale primary Atlas is not authority.

Portable paths below use `~` for the operator’s home; stage 2 resolves them before dispatch. The manager’s printed path is authoritative for any new tree and must replace the planned path before an agent starts.

| Role | Branch | HEAD | Worktree | Build directory | Scratch | Stage |
|---|---|---|---|---|---|---|
| Coordinator | `plan/small-wave-after-0.7.0`; planned integration `wave/inspect-upgrade-after-0.7.0` | `4d0cd30872533da40f209274f936eaeae9bf01d7` | `~/.local/state/worktree/trees/b10x/connectors/wt-83fe13d0c209` | Per-workspace `target/` inside this tree; none created | `~/.cache/connectors-small-wave-20260907` | Proposal, uncommitted |
| Binary inspection unit | planned `impl/inspect-upgrade-after-0.7.0` | not created; fork after opening gate | planned `~/.local/state/worktree/trees/b10x/connectors/wt-inspect-upgrade-20260907`, subject to CLI output | `target/`, `crates/connectors-cli/target`, `crates/connectors-console/target` inside unit tree, sequentially | planned `~/.cache/cw7/upgrade` | Awaiting approval and preflight |
| Atlas authority | detached | `d10b7484d64c28830774c9dae0ec531fcc47acb2` | `~/.local/state/worktree/trees/b10x/atlas/wt-840d0a4626a0` | none | coordinator scratch | Clean authority only |

Intended full plugin roles: `aep-drive:story-scoper`, `aep-drive:implementor`, `aep-drive:adversary`. The collaboration API has no `subagent_type` selector, so agents load the exact named plugin charter from disk. This is an explicit harness adaptation. Three existing agent threads handled one read-only scoping assignment at a time; a further spawn was refused by the thread limit, so completed threads were reused. Stage 2 dispatches one implementor and then an independent adversary, with the coordinator alone owning all AEP writes and direct commits. No implementor/adversary has been dispatched for this wave yet.

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

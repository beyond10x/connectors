---
format: aep.planning-md/1
id: specification:auth-hardening-wave-20260908
kind: specification
status: draft
title: F04 and F05 auth semantic hardening wave
relations:
- informed_by: specification:contract-driven-connectors-design
revision: 13
---
# Auth semantic hardening wave — 2026-09-08

## Authorization and selection

**Skill version 0.8.1** (`aep-drive:wave`). Interactive run preapproved by the operator: “make this a wave of 2 stories ... use worktree to split safely”; “no budget limit”. Selection: F04 `story:contracts-refresh-coordination` and F05 `story:contracts-credential-evidence`, serving existing Connectors O1 (governed integration access) and O5 (generic platform integration capabilities), under the existing design and remediation epic. No new organization objectives or Atlas records are introduced.

Approval covers the coordinator opening preparation commit, two unit commits, necessary integration merges, the closing store commit and merge into local `main`; it covers no push, tag, release or next wave.

Source: both draft story records at `8903166` have no blocking or prerequisite edges; `aep plan artifact blocked` returned “nothing is blocked”. Two independent read-only scopers returned medium-confidence scopes: existing document paths cited; new ESS/scenario paths inferred. All other draft stories are outside the operator-selected two-story wave. No new backlog decomposition occurred.

The store command below computes private remaining edits after coordinator preparation. It is the complete verbatim result, including all waves, collisions and unassessed items. Existing full-story integration overlap was identified by both scopers: coordinator alone provides credentials.yaml, system registration and gate wiring before either unit forks; no implementor may edit them. Thus this is serialized shared preparation followed by disjoint private edits, not a claim that the original complete stories have no shared files.

## Roles and execution boundary

Intended roles: `aep-drive:story-scoper`, `aep-drive:implementor`, `aep-drive:adversary`. Harness deviation: only generic collaboration agents can be dispatched; each is instructed to read the exact installed 0.8.1 role file. No plugin-native subagent_type invocation is claimed. Two implementors run concurrently; adversaries are separate agents. No model budget cap was requested.

This wave hardens semantic contracts and their ESS model. ESS 0.20.0 compiles scenarios but has no Connectors runtime conformance target. Report headers therefore keep executed runtime cases 0→0; compiled authored cases and compiler refusals are recorded separately. A scenario compiled against a missing declaration supplies test-first compiler refusal evidence, not proof of runtime failure. Coordinator reads those reports. Runtime exclusion, actual identity validation, byte pinning and atomic publication remain explicit obligations.

## Preflight evidence

Commands observed on 2026-09-08: clean primary `main` at `890316673b2fc7421964d08e309d02afc452b87a`; no remote configured. `worktree doctor --check`: git=true config=true registry=true profiles=3. No prior wave-named tree/build root found. Existing managed design tree is finished/clean, head `06ecdec`; Kubernetes governed drive tree is active/dirty, head `801b8d5`; both unrelated retained tasks, left untouched. No branch planned for this wave was already checked out.

`df -h`: root 91G available (848G total,89%used); /tmp19G available. Floor chosen by coordinator:20GiB. Existing representative primary target build measured by `du -sh`:5.6G; three such targets would cost16.8G. New unit work is ESS-only and full Rust build runs on integration. Primary target is existing primary output, not previous-wave residue. Local toolchain40M; primary scratch16K. `/usr/bin/sccache`0.16.0 is wired by `/home/timo/.cargo/config.toml` build.rustc-wrapper; shared cache8GiB/10GiB. Separate internal target per tree; no shared CARGO_TARGET_DIR. Own TMPDIR avoids prior /tmp quota failure. Record actual integration cost at close.

Worktree skill requires a managed integration checkout. It is created before the opening commit, a necessary ordering deviation from the wave's literal “before first worktree”; cheap opening checks run before either unit checkout or implementor exists. No remote publication or cleanup recovery can be assumed. After local integration, preserve evidence, remove owned disposable output, release own leases and hand off retained managed trees; no forced deletion or invented remote.

## Shared preparation

`ess/domains/credentials.yaml` defines a private host-issued UUID for an immutable captured snapshot, qualified instance/connection/profile/provider authority and expected external identity. Expected identity is not validated evidence. Generation is distinct from custody version, config revision and refresh ownership fence; new bytes/refresh get new identity. CredentialGeneration references exactly one existing ServiceConfiguration; unresolved Connection/AuthProfile/custody/provider relations remain UNMAPPED. Snapshot capture is immutable, not a lifecycle claim of permanent credential usability. The shared record validates with ESS0.20.0 (7 files). Private domain headers reserve authoring files without inventing behavior.

`crates/connectors-build/src/gate.rs` collects all three explicit scenario directories into owned gate scratch, fails on missing/empty sets, and records step results. No runtime host/SDK work. Units must request coordinator changes to common definitions instead of editing them.

## Units

Final source integration gate head: `2a4404958449b1b91c5d494c5e4473ab6725bd8f`. The closing store/evidence commit containing this page is the final integration tip; its id is obtained from Git, avoiding a self-referential hash. Local main will fast-forward to that commit after final cheap validation. No branch is published.

| Unit | Managed id | Branch / verified source head | Worktree | Build directory | Assigned scratch | Stage |
|---|---|---|---|---|---|---|
| integration | connectors-v2-auth-wave-20260908 | wave/auth-hardening-20260908 / 2a4404958449b1b91c5d494c5e4473ab6725bd8f (gate head; closing record follows) | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-wave-20260908 | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-wave-20260908/target (removed or never created) | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-wave-20260908/.local/waves/auth-hardening-20260908/integration (evidence copied, disposable contents removed) | source integrated locally; checkout retained for recovery |
| refresh | connectors-v2-auth-refresh-20260908 | impl/contracts-refresh-coordination / 9fa4ba0346552fd3c34acd25e974717fd4c8c61c | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-refresh-20260908 | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-refresh-20260908/target (removed or never created) | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-refresh-20260908/.local/waves/auth-hardening-20260908/refresh (evidence copied, disposable contents removed) | source integrated locally; checkout retained for recovery |
| evidence | connectors-v2-auth-evidence-20260908 | impl/contracts-credential-evidence / f55020ae799464ac09dfcc5a3ac54d02edc213b7 | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908 | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/target (removed or never created) | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908/.local/waves/auth-hardening-20260908/evidence (evidence copied, disposable contents removed) | source integrated locally; checkout retained for recovery |

Owning coordinator session: `codex-auth-wave-20260908`; all implementor/adversary sessions returned and released their leases. Coordinator lease is released at final handoff after committing. Related work: story:contracts-refresh-coordination and story:contracts-credential-evidence, both implemented at semantic/model level. Next owner: operator. Next action for checkout retirement: explicitly authorize publication or another recovery workflow supported by worktree, then finish and review exact ids for GC. Existing design/Kubernetes trees were untouched.

## Store selection output

Command: `aep plan artifact waves --kind story --status draft --format json` (protocol0.54.0), after scope recording, before lifecycle moves.

```json
{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
        {
          "id": "story:contracts-acquisition-profiles",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/README.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/acquisition/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/profile/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/docker.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/grafana.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/kubernetes.md"
            }
          ]
        },
        {
          "id": "story:contracts-document-admission",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/datasources/records/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/atlassian.md"
            }
          ]
        },
        {
          "id": "story:contracts-mutation-classification",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/operations/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/sessions/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/media-session.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 2,
      "artifacts": [
        {
          "id": "story:contracts-credential-evidence",
          "inferred": true,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/auth/capability/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/connection/v1alpha1/semantics.md"
            },
            {
              "confidence": "inferred",
              "path": "contracts/auth/evidence/v1alpha1/scenarios/configured-replacement-identity.yaml"
            },
            {
              "confidence": "inferred",
              "path": "contracts/auth/evidence/v1alpha1/scenarios/rotation-before-dispatch.yaml"
            },
            {
              "confidence": "inferred",
              "path": "contracts/auth/evidence/v1alpha1/scenarios/same-identity-refresh.yaml"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/evidence/v1alpha1/semantics.md"
            },
            {
              "confidence": "inferred",
              "path": "contracts/auth/evidence/v1alpha1/verification.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/kubernetes.md"
            },
            {
              "confidence": "inferred",
              "path": "ess/domains/credential_evidence.yaml"
            }
          ]
        },
        {
          "id": "story:contracts-host-composition",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/grafana.md"
            },
            {
              "confidence": "cited",
              "path": "docs/design.md"
            }
          ]
        },
        {
          "id": "story:contracts-refresh-coordination",
          "inferred": true,
          "scope": [
            {
              "confidence": "inferred",
              "path": "contracts/auth/acquisition/v1alpha1/scenarios"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/acquisition/v1alpha1/semantics.md"
            },
            {
              "confidence": "inferred",
              "path": "contracts/auth/acquisition/v1alpha1/verification.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/custody/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/atlassian.md"
            },
            {
              "confidence": "inferred",
              "path": "ess/domains/refresh.yaml"
            }
          ]
        },
        {
          "id": "story:contracts-session-revocation",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/media/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/sessions/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/media-session.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 3,
      "artifacts": [
        {
          "id": "story:contracts-connection-readiness",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/auth/acquisition/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/connection/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/evidence/v1alpha1/semantics.md"
            }
          ]
        },
        {
          "id": "story:contracts-evidence-precision",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/media/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/operations/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/sessions/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/atlassian.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/docker.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/grafana.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/kubernetes.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/media-session.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 4,
      "artifacts": [
        {
          "id": "story:contracts-log-continuation",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/datasources/logs/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/grafana.md"
            }
          ]
        },
        {
          "id": "story:contracts-media-controls",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/README.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/capability/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/evidence/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/media/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/sessions/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/media-session.md"
            }
          ]
        },
        {
          "id": "story:contracts-restart-idempotency",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/operations/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/docker.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/kubernetes.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 5,
      "artifacts": [
        {
          "id": "story:contracts-permission-budgets",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/auth/evidence/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/kubernetes.md"
            }
          ]
        },
        {
          "id": "story:contracts-supported-vocabulary",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/auth/capability/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/profile/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/datasources/series/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/discovery/resources/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/media/v1alpha1/semantics.md"
            }
          ]
        },
        {
          "id": "story:contracts-tenant-header",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/datasources/logs/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/grafana.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 6,
      "artifacts": [
        {
          "id": "story:contracts-wire-compatibility",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/auth/acquisition/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/capability/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/connection/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/evidence/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/profile/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/datasources/logs/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/datasources/records/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/datasources/series/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/discovery/resources/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/media/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/operations/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/service/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/sessions/v1alpha1/semantics.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 7,
      "artifacts": [
        {
          "id": "story:contracts-anonymous-auth",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/auth/capability/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/profile/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/grafana.md"
            }
          ]
        },
        {
          "id": "story:contracts-federated-approval",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/operations/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/service/v1alpha1/semantics.md"
            }
          ]
        },
        {
          "id": "story:contracts-management-boundary",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/auth/acquisition/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/connection/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/design.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 8,
      "artifacts": [
        {
          "id": "story:contracts-discovery-coverage",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/discovery/resources/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/grafana.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/kubernetes.md"
            }
          ]
        },
        {
          "id": "story:contracts-mutation-visibility",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/operations/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/service/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/atlassian.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 9,
      "artifacts": [
        {
          "id": "story:contracts-discovery-profiles",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/discovery/resources/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/grafana.md"
            },
            {
              "confidence": "cited",
              "path": "docs/adapters/kubernetes.md"
            }
          ]
        },
        {
          "id": "story:contracts-read-refresh-retry",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/auth/acquisition/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/capability/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/service/v1alpha1/semantics.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 10,
      "artifacts": [
        {
          "id": "story:contracts-persistence-ownership",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/auth/acquisition/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/connection/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/custody/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/auth/evidence/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/discovery/resources/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "contracts/operations/v1alpha1/semantics.md"
            },
            {
              "confidence": "cited",
              "path": "docs/design.md"
            }
          ]
        }
      ]
    },
    {
      "wave": 11,
      "artifacts": [
        {
          "id": "story:contracts-documentation-index",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "contracts/README.md"
            },
            {
              "confidence": "cited",
              "path": "docs/design.md"
            }
          ]
        }
      ]
    }
  ],
  "collisions": [
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-anonymous-auth",
      "path": "contracts/auth/profile/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-anonymous-auth",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-connection-readiness",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-credential-evidence",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-discovery-coverage",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-discovery-coverage",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-discovery-profiles",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-discovery-profiles",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-documentation-index",
      "path": "contracts/README.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-evidence-precision",
      "path": "docs/adapters/docker.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-evidence-precision",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-evidence-precision",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-host-composition",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-log-continuation",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-management-boundary",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-media-controls",
      "path": "contracts/README.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-permission-budgets",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-read-refresh-retry",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-refresh-coordination",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-restart-idempotency",
      "path": "docs/adapters/docker.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-restart-idempotency",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-supported-vocabulary",
      "path": "contracts/auth/profile/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-tenant-header",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-acquisition-profiles",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/profile/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-credential-evidence",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-discovery-coverage",
      "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-discovery-coverage",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-discovery-profiles",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-evidence-precision",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-host-composition",
      "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-host-composition",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-log-continuation",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-media-controls",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-read-refresh-retry",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-supported-vocabulary",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-supported-vocabulary",
      "path": "contracts/auth/profile/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-tenant-header",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/profile/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-anonymous-auth",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-credential-evidence",
      "path": "contracts/auth/connection/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-credential-evidence",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-management-boundary",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-management-boundary",
      "path": "contracts/auth/connection/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-media-controls",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-permission-budgets",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/auth/connection/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-read-refresh-retry",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-refresh-coordination",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/connection/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-connection-readiness",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-discovery-coverage",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-discovery-profiles",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-evidence-precision",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-management-boundary",
      "path": "contracts/auth/connection/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-media-controls",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-media-controls",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-permission-budgets",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-permission-budgets",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/auth/connection/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-read-refresh-retry",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-restart-idempotency",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-supported-vocabulary",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/connection/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-credential-evidence",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-discovery-profiles",
      "path": "contracts/discovery/resources/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-discovery-profiles",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-discovery-profiles",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-evidence-precision",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-evidence-precision",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-host-composition",
      "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-host-composition",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-log-continuation",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-permission-budgets",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/discovery/resources/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-restart-idempotency",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-supported-vocabulary",
      "path": "contracts/discovery/resources/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-tenant-header",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-coverage",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/discovery/resources/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-profiles",
      "b": "story:contracts-evidence-precision",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-profiles",
      "b": "story:contracts-evidence-precision",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-profiles",
      "b": "story:contracts-host-composition",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-profiles",
      "b": "story:contracts-log-continuation",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-profiles",
      "b": "story:contracts-permission-budgets",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-profiles",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/discovery/resources/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-profiles",
      "b": "story:contracts-restart-idempotency",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-profiles",
      "b": "story:contracts-supported-vocabulary",
      "path": "contracts/discovery/resources/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-profiles",
      "b": "story:contracts-tenant-header",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-discovery-profiles",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/discovery/resources/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-document-admission",
      "b": "story:contracts-evidence-precision",
      "path": "docs/adapters/atlassian.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-document-admission",
      "b": "story:contracts-mutation-visibility",
      "path": "docs/adapters/atlassian.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-document-admission",
      "b": "story:contracts-refresh-coordination",
      "path": "docs/adapters/atlassian.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-document-admission",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/datasources/records/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-documentation-index",
      "b": "story:contracts-host-composition",
      "path": "docs/design.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-documentation-index",
      "b": "story:contracts-management-boundary",
      "path": "docs/design.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-documentation-index",
      "b": "story:contracts-media-controls",
      "path": "contracts/README.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-documentation-index",
      "b": "story:contracts-persistence-ownership",
      "path": "docs/design.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-federated-approval",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-host-composition",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-log-continuation",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-media-controls",
      "path": "contracts/media/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-media-controls",
      "path": "contracts/sessions/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-media-controls",
      "path": "docs/adapters/media-session.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-mutation-classification",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-mutation-classification",
      "path": "contracts/sessions/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-mutation-classification",
      "path": "docs/adapters/media-session.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-mutation-visibility",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-mutation-visibility",
      "path": "docs/adapters/atlassian.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-permission-budgets",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-refresh-coordination",
      "path": "docs/adapters/atlassian.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-restart-idempotency",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-restart-idempotency",
      "path": "docs/adapters/docker.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-restart-idempotency",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-session-revocation",
      "path": "contracts/media/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-session-revocation",
      "path": "contracts/sessions/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-session-revocation",
      "path": "docs/adapters/media-session.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-supported-vocabulary",
      "path": "contracts/media/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-tenant-header",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/media/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-evidence-precision",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/sessions/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-federated-approval",
      "b": "story:contracts-mutation-classification",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-federated-approval",
      "b": "story:contracts-mutation-visibility",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-federated-approval",
      "b": "story:contracts-mutation-visibility",
      "path": "contracts/service/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-federated-approval",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-federated-approval",
      "b": "story:contracts-read-refresh-retry",
      "path": "contracts/service/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-federated-approval",
      "b": "story:contracts-restart-idempotency",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-federated-approval",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-federated-approval",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/service/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-host-composition",
      "b": "story:contracts-log-continuation",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-host-composition",
      "b": "story:contracts-management-boundary",
      "path": "docs/design.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-host-composition",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-host-composition",
      "b": "story:contracts-persistence-ownership",
      "path": "docs/design.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-host-composition",
      "b": "story:contracts-tenant-header",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-host-composition",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-log-continuation",
      "b": "story:contracts-tenant-header",
      "path": "contracts/datasources/logs/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-log-continuation",
      "b": "story:contracts-tenant-header",
      "path": "docs/adapters/grafana.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-log-continuation",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/datasources/logs/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-management-boundary",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-management-boundary",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/auth/connection/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-management-boundary",
      "b": "story:contracts-persistence-ownership",
      "path": "docs/design.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-management-boundary",
      "b": "story:contracts-read-refresh-retry",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-management-boundary",
      "b": "story:contracts-refresh-coordination",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-management-boundary",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-management-boundary",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/connection/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-mutation-classification",
      "path": "contracts/sessions/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-mutation-classification",
      "path": "docs/adapters/media-session.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-permission-budgets",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-read-refresh-retry",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-session-revocation",
      "path": "contracts/media/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-session-revocation",
      "path": "contracts/sessions/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-session-revocation",
      "path": "docs/adapters/media-session.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-supported-vocabulary",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-supported-vocabulary",
      "path": "contracts/media/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/media/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-media-controls",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/sessions/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-mutation-classification",
      "b": "story:contracts-mutation-visibility",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-mutation-classification",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-mutation-classification",
      "b": "story:contracts-restart-idempotency",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-mutation-classification",
      "b": "story:contracts-session-revocation",
      "path": "contracts/sessions/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-mutation-classification",
      "b": "story:contracts-session-revocation",
      "path": "docs/adapters/media-session.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-mutation-classification",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-mutation-classification",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/sessions/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-mutation-visibility",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-mutation-visibility",
      "b": "story:contracts-read-refresh-retry",
      "path": "contracts/service/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-mutation-visibility",
      "b": "story:contracts-refresh-coordination",
      "path": "docs/adapters/atlassian.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-mutation-visibility",
      "b": "story:contracts-restart-idempotency",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-mutation-visibility",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-mutation-visibility",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/service/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-permission-budgets",
      "b": "story:contracts-persistence-ownership",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-permission-budgets",
      "b": "story:contracts-restart-idempotency",
      "path": "docs/adapters/kubernetes.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-permission-budgets",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-persistence-ownership",
      "b": "story:contracts-read-refresh-retry",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-persistence-ownership",
      "b": "story:contracts-refresh-coordination",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-persistence-ownership",
      "b": "story:contracts-refresh-coordination",
      "path": "contracts/auth/custody/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-persistence-ownership",
      "b": "story:contracts-restart-idempotency",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-persistence-ownership",
      "b": "story:contracts-supported-vocabulary",
      "path": "contracts/discovery/resources/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-persistence-ownership",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-persistence-ownership",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/connection/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-persistence-ownership",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/evidence/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-persistence-ownership",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/discovery/mediated_route/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-persistence-ownership",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/discovery/resources/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-persistence-ownership",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-read-refresh-retry",
      "b": "story:contracts-refresh-coordination",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-read-refresh-retry",
      "b": "story:contracts-supported-vocabulary",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-read-refresh-retry",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-read-refresh-retry",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-read-refresh-retry",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/service/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-refresh-coordination",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/acquisition/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-restart-idempotency",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/operations/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-session-revocation",
      "b": "story:contracts-supported-vocabulary",
      "path": "contracts/media/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-session-revocation",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/media/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-session-revocation",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/sessions/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-supported-vocabulary",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/capability/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-supported-vocabulary",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/auth/profile/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-supported-vocabulary",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/datasources/series/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-supported-vocabulary",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/discovery/resources/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-supported-vocabulary",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/media/v1alpha1/semantics.md",
      "confidence": "cited"
    },
    {
      "a": "story:contracts-tenant-header",
      "b": "story:contracts-wire-compatibility",
      "path": "contracts/datasources/logs/v1alpha1/semantics.md",
      "confidence": "cited"
    }
  ],
  "unassessed": [],
  "cycles": []
}
```

## Dispatch state

Both unit commits are integrated locally; both stories moved to implemented based on the full gate and recorded reviews. F04/F05 source findings are fixed at semantic/model level; six of48 source findings are now fixed,42 remain assigned. No review findings remained and no correction/second attack was needed. Runtime auth implementation is subsequent work. Closing step: commit these final records and fast-forward clean local main, then release coordinator leases and retain the three checkouts with this handoff.

## Measured preparation

Coordinator bootstrap: `TMPDIR=<integration>/.local/tmp CARGO_BUILD_JOBS=2 cargo build -p connectors-build --locked --offline` exited0; runner reported31.48s. Fresh integration target measured710M after that package build. This is a tooling compilation, not the whole integration gate. Pinned ESS shared model remains50 declarations and existing operations suite67compiled/15authored/0refusals. Unit work remains private; final gate will include `--msrv` per README.md:19–29.

## Combined model precheck

Coordinator copied the opening ESS plus the two returned private domains into owned integration scratch/combined-ess, and the15 existing operation +12F04 +5F05 authored files into combined-scenarios. This scratch check precedes reviews/merges and is not the final integration gate. Pinned ESS summaries: `connectors v1 — 7 file(s), valid`; `connectors v1 — 7 file(s), 92 declaration(s), compiled`; `167 scenario(s) (32 authored), 0 refusal(s)`. Every command exited0. Generated structural scenarios135; runtime auth execution0. Exact command logs are retained in integration scratch pending closing evidence preservation.

## Gate incident

First full-gate attempt exited1 at workspace build (cargo step101): `sccache: error: path must be shorter than SUN_LEN`. Formatting and three descriptor comparisons completed0; no later step ran. The managed worktree root is90characters and the generated gate TMPDIR113characters. Installed sccache0.16 documentation (`/usr/share/doc/sccache/README.md:171–180`, Configuration.md:172) describes automatic daemon startup/10-minute idle shutdown and startup notification sockets. Coordinator inference: the cache daemon expired while semantic agents worked, so startup from the long gate TMPDIR failed.

Coordinator assigns a short auxiliary wave scratch root `/home/timo/.cache/connectors-waves/auth-hardening-20260908/cache-start` solely for sccache startup notification. It is outside worktrees and explicitly recorded here; build targets remain separate inside their own trees. Start the default shared cache server using this TMPDIR, preserve failed gate output as full-gate-attempt-1.log, then rerun the whole gate. No source/assertion is changed to bypass the failure, no existing server is stopped and no second server/cache namespace is introduced. Remove this exact disposable auxiliary directory after verification if unused.

## Gate result

Full gate on integration merge `2a4404958449b1b91c5d494c5e4473ab6725bd8f` exited0. Exact command: `TMPDIR=<integration>/.local/tmp CARGO_BUILD_JOBS=2 cargo run -p connectors-build --locked --offline -- --ess <integration>/.local/toolchains/ess/0.20.0/bin/ess gate --msrv`. Step results are retained verbatim below and in docs/waves/auth-hardening-20260908/integration/full-gate.log.39Rust tests passed,0failed/ignored; MSRV1.88.0 ran; ESS92declarations,169compiled scenarios (135generated+34authored:15operations+13F04+6F05),0refusals. Auth runtime scenarios executed0. No gate step was skipped. First attempt failed at workspace build due cache daemon startup socket path; that log is preserved separately. The successful rerun changed only cache startup environment, no source/assertion.

```text
gate: exit=exit status: 0 ["fmt", "--all", "--", "--check"]
gate: gitlab descriptor matches; exit=0
gate: kubernetes descriptor matches; exit=0
gate: sql descriptor matches; exit=0
gate: exit=exit status: 0 ["build", "--workspace", "--locked", "--offline"]
gate: exit=exit status: 0 ["test", "--workspace", "--locked", "--offline"]
gate: exit=exit status: 0 ["clippy", "--workspace", "--all-targets", "--locked", "--offline", "--", "-D", "warnings"]
gate: exit=exit status: 0 ["build", "--locked", "--offline", "-p", "connectors-gitlab", "--lib", "--no-default-features"]
gate: connectors-gitlab library boundary holds; exit=0
gate: exit=exit status: 0 ["build", "--locked", "--offline", "-p", "connectors-kubernetes", "--lib", "--no-default-features"]
gate: connectors-kubernetes library boundary holds; exit=0
gate: exit=exit status: 0 ["build", "--locked", "--offline", "-p", "connectors-sql", "--lib", "--no-default-features"]
gate: connectors-sql library boundary holds; exit=0
gate: generic CLI boundary holds; exit=0
gate: exit=exit status: 0 ["+1.88.0", "check", "--workspace", "--all-targets", "--locked", "--offline"]
gate: exit=exit status: 0 ["specify", "validate", "--path", "ess"]
gate: exit=exit status: 0 ["specify", "compile", "--path", "ess", "--out", "/home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-wave-20260908/.local/tmp/gate-mpCOj4/connectors-ir.json"]
gate: collected 15 authored scenarios from contracts/operations/v1alpha1/scenarios; exit=0
gate: collected 13 authored scenarios from contracts/auth/acquisition/v1alpha1/scenarios; exit=0
gate: collected 6 authored scenarios from contracts/auth/evidence/v1alpha1/scenarios; exit=0
gate: exit=exit status: 0 ["verify", "conform", "synthesize", "--path", "ess", "--target", "ir", "--scenarios", "/home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-wave-20260908/.local/tmp/gate-mpCOj4/authored-scenarios", "--out", "/home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-wave-20260908/.local/tmp/gate-mpCOj4/contract-conformance.json"]
gate: exit=exit status: 0 ["plan", "artifact", "validate"]
```

AEP0.54 reports empty `findings: []` fences as “no findings block”; the two new immutable reports do contain literal empty fenced lists. Output is retained verbatim; no artificial finding was added to suppress this diagnostic.

## Agent cost observability

Six role dispatches completed: two read-only scopers, two implementors, two adversaries. Each adversary made one full attack, returned0findings and added one scenario; no correction or second attack was needed. This harness exposes completion messages/status but supplied no per-agent token counts, tool-use totals or wall-duration metrics. Those three values are unavailable for each of the six agents; no cost estimate is represented as measured usage. Operator budget: no limit. Tool-run counts/results and scoped compiler case counts are preserved in each report.

## Storage and handoff

After the gate process returned exit0 and all four author/review workers returned, a /proc scan found no compiler/test/ESS process using these trees. Read scratch inventories, copied small logs/reports/source-digest audits to docs/waves/auth-hardening-20260908, and preserved reviews as immutable AEP artifacts. Exact disposable removals are in integration/disposable-cleanup.json under that evidence directory. Removed8 exact directories (integration target,3assigned scratch roots,3private pinned-tool copies,1short cache-start directory), whose measured allocation was3,125,043,200bytes (~2.91GiB). No unit target existed. Actual full integration target measured2.8G before removal, vs5.6G preflight representative estimate. Free disk returned to82G, above20GiB floor. Primary target and shared sccache cache were untouched.

Managed source trees remain, not silently abandoned: no remote exists, no publication is authorized, and local merges alone supply no recovery proof. Do not force-remove them or claim GC completion. Final exact-id GC assessment and current-head handoff are retained in primary .local/waves/auth-hardening-20260908/ after the closing commit; final transcript reports their result.

---
format: aep.planning-md/1
id: specification:auth-hardening-wave-20260908
kind: specification
status: draft
title: F04 and F05 auth semantic hardening wave
relations:
- informed_by: specification:contract-driven-connectors-design
revision: 1
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

Wave registry root: `/home/timo/.local/state/worktree/trees/b10x/connectors_v2/`. Coordinator session `codex-auth-wave-20260908`. Every tree has target/ inside itself and .local/waves/auth-hardening-20260908/<unit> scratch. Every unit/adversary has its own explicit session lease in its assigned tree.

| Unit | Managed id / branch | Head | Worktree | Build | Scratch | Stage |
|---|---|---|---|---|---|---|
| integration | connectors-v2-auth-wave-20260908 / wave/auth-hardening-20260908 | 8903166 (opening pending) | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-wave-20260908 | same tree/target | same tree/.local/waves/auth-hardening-20260908/integration | preparation |
| F04 | connectors-v2-auth-refresh-20260908 / impl/contracts-refresh-coordination | pending opening | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-refresh-20260908 | same tree/target | same tree/.local/waves/auth-hardening-20260908/refresh | planned |
| F05 | connectors-v2-auth-evidence-20260908 / impl/contracts-credential-evidence | pending opening | /home/timo/.local/state/worktree/trees/b10x/connectors_v2/connectors-v2-auth-evidence-20260908 | same tree/target | same tree/.local/waves/auth-hardening-20260908/evidence | planned |

No branches are published. Next owner after local completion: operator; next action if cleanup desired: explicitly authorize publication/recovery supported by worktree, then review exact ids for GC. Old retained trees are not part of these triples.

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

---
format: aep.planning-md/3
id: approval-record:wave-20261029-definitions-runtime
kind: approval-record
status: approved
title: 'Wave 20261029-definitions-runtime: definitions install, read at run time'
relations:
- decides: story:definitions-install
revision: 3
transitions:
- {from: "draft", to: "approved", at: "2026-10-29T20:20:56Z", actor: "human:timo", revision: 3}
---
## Status

Proposed 2026-10-29 by the coordinating session; waiting for the operator's approval. Skill version
0.24.2 (`aep:implementing`).

## Wave 20261029-definitions-runtime

Epic `epic:catalog-connectors-only-in-definitions`, under SM-D209 (operator, 2026-10-29: definitions
are read at run time) and SM-D210 (recorded in `story:definitions-install` § Decisions).

| story | serves | scope | decided |
|---|---|---|---|
| `story:definitions-install` | `vision:independent-contract-adapters` | 6 cited, 9 inferred, confidence medium | in the wave, N = 1 |
| `story:definitions-pin-covers-catalog` | — | 12 cited, 2 inferred | next wave: depends on `story:definitions-install` |
| `story:openrouter-inference-key-profile` | — | 1 cited, 4 inferred | left out: its acceptance needs an OpenRouter declaration that `story:instances-resolve-through-locked-index` adds, and that story depends on this one; its main surface `apps/connectors/src/local/` is in flight in the D218 units |
| the other six epic stories | — | unassessed | depend on the two stories above |

Selection path: `aep plan artifact waves --kind story --status draft --format json` (aep 0.78.0),
filtered below to the epic's stories and every pair the verb excluded with one of them.

Start condition: the unit dispatches after the orchestrator's D218 units (`unit/d218-u1-invoke`,
`u2-connections`, `u3-removal`) merge to `main`, because both change `apps/connectors/spec/cli.yaml`
and regenerate `apps/connectors-cli-contract/` and `website/docs/reference/cli.md`. The integration
branch is rebased onto that `main` before dispatch.

## Verb output (epic stories)

Waves: `story:openrouter-inference-key-profile` in wave 4, `story:definitions-install` in wave 5,
`story:definitions-pin-covers-catalog` in wave 6 (store-wide layering, 10 waves).

Collisions:

```
story:agent-tool-projection x story:definitions-install on CHANGELOG.md (inferred)
story:agent-tool-projection x story:definitions-pin-covers-catalog on crates/connectors-catalog/src/import.rs (cited)
story:catalog-validates-enum-and-date-time x story:definitions-install on CHANGELOG.md (inferred)
story:catalog-zendesk-reads x story:definitions-install on CHANGELOG.md (inferred)
story:catalog-zendesk-reads x story:definitions-pin-covers-catalog on crates/connectors-build/src/gate.rs (cited)
story:claude-getting-started-tutorial x story:definitions-install on CHANGELOG.md (inferred)
story:claude-getting-started-tutorial x story:definitions-pin-covers-catalog on docs/development.md (inferred)
story:configuration-change-error x story:definitions-install on apps/connectors/src/local.rs (inferred)
story:configuration-change-error x story:definitions-install on contracts/cli/v1alpha1/scenarios.md (inferred)
story:configuration-change-error x story:definitions-install on contracts/cli/v1alpha1/semantics.md (inferred)
story:configuration-change-error x story:definitions-install on ess/domains/cli.yaml (inferred)
story:configuration-change-error x story:openrouter-inference-key-profile on contracts/cli/v1alpha1/semantics.md (inferred)
story:connection-next-actions-no-cycle x story:definitions-install on apps/connectors/src/local.rs (inferred)
story:connection-next-actions-no-cycle x story:definitions-install on contracts/cli/v1alpha1/semantics.md (inferred)
story:connection-next-actions-no-cycle x story:definitions-install on ess/domains/cli.yaml (inferred)
story:connection-next-actions-no-cycle x story:openrouter-inference-key-profile on contracts/cli/v1alpha1/semantics.md (inferred)
story:definition-connect-begin-carries-selection x story:definitions-install on CHANGELOG.md (inferred)
story:definition-import-merges-shared-auth x story:definitions-pin-covers-catalog on crates/connectors-catalog/src/import.rs (cited)
story:definitions-install x story:definitions-pin-covers-catalog on Cargo.lock (inferred)
story:definitions-install x story:definitions-pin-covers-catalog on apps/connectors/Cargo.toml (inferred)
story:definitions-install x story:google-consent-on-the-oauth2-core on Cargo.lock (inferred)
story:definitions-install x story:guarded-write-credential-refusal-says-repair on CHANGELOG.md (inferred)
story:definitions-install x story:guarded-write-credential-refusal-says-repair on contracts/cli/v1alpha1/semantics.md (cited)
story:definitions-install x story:mcp-cli-journey-discovery-contract on contracts/cli/v1alpha1/semantics.md (inferred)
story:definitions-install x story:mcp-outbound-stdio-runtime on Cargo.lock (inferred)
story:definitions-install x story:mcp-outbound-stdio-runtime on Cargo.toml (inferred)
story:definitions-install x story:mounted-custody-kind-secret-volume on CHANGELOG.md (inferred)
story:definitions-install x story:openrouter-inference-key-profile on contracts/cli/v1alpha1/semantics.md (inferred)
story:definitions-install x story:openrouter-inference-key-profile on crates/connectors-host/src/local/config.rs (inferred)
story:definitions-install x story:spec-models-operation-admission on apps/connectors-cli-contract (cited)
story:definitions-install x story:standing-write-approval-for-unattended-agents on CHANGELOG.md (inferred)
story:definitions-install x story:standing-write-approval-for-unattended-agents on apps/connectors/spec/cli.yaml (inferred)
story:definitions-install x story:standing-write-approval-for-unattended-agents on ess/system.yaml (inferred)
story:definitions-install x story:zendesk-oauth-ticket-writes on CHANGELOG.md (inferred)
story:definitions-pin-covers-catalog x story:google-consent-on-the-oauth2-core on Cargo.lock (inferred)
story:definitions-pin-covers-catalog x story:mcp-outbound-stdio-runtime on Cargo.lock (inferred)
story:guarded-write-credential-refusal-says-repair x story:openrouter-inference-key-profile on contracts/cli/v1alpha1/semantics.md (inferred)
story:mcp-cli-journey-discovery-contract x story:openrouter-inference-key-profile on contracts/cli/v1alpha1/semantics.md (inferred)
```

Unassessed:

```
story:catalog-provider-data-removed
story:catalog-tests-read-pinned-release-gitlab-engine
story:catalog-tests-read-pinned-release-providers
story:definition-tests-read-pinned-release
story:definitions-fixture-copy-removed
story:instances-resolve-through-locked-index
```

Cycles: none.

## Units

| unit | branch | worktree | build directory | stage |
|---|---|---|---|---|
| U1 `story:definitions-install` | `unit/defs-u1-install` | created at dispatch | the unit tree's own `target/` | proposed |

Integration branch `wave/2026-10-29-definitions-runtime`, tree
`connectors-wave-defs` (base `65c928d72`). Dispatch roles: `aep:implementor`, then `aep:adversary`;
scoping ran as `aep:story-scoper`.

## Pre-flight

- Disk: 90G free on `/` at proposal time; builds stop under 20G.
- Parallelism: 1 unit; builds at nice 19, at most 4 cores.
- One full local gate on the integration branch, one bot PR, merged when green.

## Commits this approval authorises

One unit commit series on `unit/defs-u1-install`, its merge into
`wave/2026-10-29-definitions-runtime`, the closing store commit, and the PR merge into `main`. No tag
and no release.

## Approval

The operator, 2026-10-29, in the coordinating session (verbatim): "i approve definitions-install
but in generall, i would like to just point it to some yaml files, even custom not from the repo,
user provided, referenced in connectors config (.b10x/connectors.toml with profiles)".

The second half is filed as `story:definition-files-in-user-config` (draft, SM-D211) and is not
part of this wave.

Dispatch call (SM-D212): U1 is dispatched now rather than after D218 merges. The integration branch
is rebased onto `main` once the D218 units merge, `apps/connectors/spec/cli.yaml` is merged by hand
and `apps/connectors-cli-contract/` and `website/docs/reference/cli.md` are regenerated; the wave PR
merges after D218.

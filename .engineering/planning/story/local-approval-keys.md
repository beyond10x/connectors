---
format: aep.planning-md/1
id: story:local-approval-keys
kind: story
status: implemented
title: Manage persistent local approval-signing keys through the CLI
relations:
- decomposes: initiative:complete-local-connectors
- depends_on: story:local-approval-binding
- depends_on: story:local-execution-audit
- depends_on: story:local-mutation-ledger
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: apps/connectors
- confidence: inferred
  path: apps/connectors-cli-contract
- confidence: inferred
  path: contracts/cli
- confidence: inferred
  path: contracts/service
- confidence: cited
  path: crates/connectors-conformance/tests/cli_surface.rs
- confidence: inferred
  path: crates/connectors-host
- confidence: inferred
  path: docs
- confidence: inferred
  path: ess
- confidence: inferred
  path: website
revision: 9
---
## Outcome

Give the local owner persistent approval-signing key setup, inspection, rotation, recovery, revocation and exact retirement through the generated CLI. This completes the key-management prerequisite of GitLab governed writes; it does not yet issue proof evidence or dispatch business writes.

## Acceptance

From a setup-only metadata authority and a qualified Secret Service fixture, the production CLI acknowledges initial key publication and returns that same active public key after CLI and keyring restart.

## Selected model and implementation

contracts/service/approval-issuers.md owns the selected binding and ess/domains/approval_issuers.yaml declares ApprovalIssuer and ApprovalSigningKey, with explicit retained references to service configuration and issuer. Pinned ESS validation reports connectors v1 — 21 file(s), valid before this decomposition. Initial validation refused unobservable transition outcomes; the authored model now emits each transition fact. ESS proves structure, not runtime custody or atomicity.

Implement SQLite migration seven without changing migrations one through six; ordinary setup remains version three and read owners recognize seven without upgrading. Stage independent public and private identities before immutable Secret Service writes, then separately publish definite durable custody acknowledgements. Recovery validates the retained seed against the staged public key and synchronizes the qualified store again; it never retries creation. Revoke invalidates current and pending keys atomically. Retiring keys cannot publish and become Deleted only after guarded exact deletion acknowledgement. Keep all historical identity tombstones, with a 128-key bound.

Add purpose-separated signing custody while preserving existing credential attributes byte-for-byte. Shared process-local key-use leases and exclusive management leases serialize key changes through signing/spend acknowledgement; acquire key lease before physical custody and metadata locks. A lease supplies current key configuration, never caller/subject admission or clock evidence. Public status exposes bounded public metadata only and starts/migrates/reads secrets nowhere. Mutations use exact expected revisions and key identifiers as specified; the first init alone can omit a revision. Configured service identity must match retained registry identity. Authored CLI and shared CLI types own parser/output changes and are regenerated.

## Verification

Signing-key reclamation uses its own Retiring fence and exclusive use lease; only physical Secret Service qualification, synchronization and exact deletion acknowledgements are shared with credential custody, not credential acquisition termination or the 24-hour credential retention delay.

Use real SQLite, bounded process/thread races, actual child exits and a qualified disposable Secret Service instance. An exit before publication preserves the old Active key and recoverable Candidate; an exit after committed publication observes the new Active key and retired previous key even when acknowledgement was lost. Recovery must never republish that already Active key. Cover restart, missing/locked backend, write and acknowledgement faults, failure preserving a valid key, stale revision/identity, concurrent init/rotate/revoke/recover/use, recovery refusing wrong material, uncertain retirement, purpose isolation and private-output checks. Run affected ESS/generation/conformance, the repository gate with Rust 1.88 and two Cargo jobs, website/reference/example checks and all existing optimized GitLab CLI fixture journeys. Retain commands, failures and input identities in docs/evidence/local-approval-keys-20260910. This is not dedicated provider sandbox or production clock acceptance.

## Scope and sequencing

Single implementation and planning writer on primary main; no parallel implementation or shared Cargo builds. Inferred surfaces: crates/connectors-host, apps/connectors, ess, contracts/service, contracts/cli, docs and website; generated outputs follow their owning sources. Overlap with prior approval/mutation/audit and GitLab runtime work is serialized. The four planning critics are read-only and review this child with the parent and existing sibling ownership.

The active parent retains actual approval preparation/issuance, qualified production time, caller/subject admission, mutation ingress and full provider dispatch coordinator, native GitLab validation/writes and dedicated sandbox acceptance, then Kubernetes, PostgreSQL, MCP and remaining providers in that order. C14 create/update head-guard choice and GitLab sandbox access remain open blockers. No weaker guard, raw SystemTime trust, Connectors publication, deployment, Atlas integration or paid governed run is selected.

## CLI projection binding

The pinned generator refuses Uuid option primitives and command paths deeper than two levels. The selected presentation uses `approvals key-init`, `key-status`, `key-rotate`, `key-recover`, `key-revoke` and `key-retire`; public UUID coordinates use String with strict canonical, non-nil UUID validation in production handlers. The domain retains typed UUID identities. This is an authored presentation choice, not a generator change or weakened identity rule. Initial generator refusals are retained in the task's verification logs.

## Implementation checkpoint — 2026-09-10

The generated production CLI implements all six `approvals key-*` commands. Migration seven, purpose-separated protected custody, public key history, exact revision fences, recovery, revocation, retirement and process-bound current-key leases are implemented. Ordinary setup remains schema three; migrations one through six retain their bytes and identity continuity. Key management starts no provider and does not issue proof evidence or admit a business effect.

The complete Rust 1.88 repository gate passes. Eleven explicit key tests pass in 10.15 seconds, including all six optimized production CLI commands, keyring restart, three actual child exits, failed rotation, wrong material, uncertain storage/deletion, purpose isolation, concurrent initialization, held-use exclusion and stale revocation/recovery. All five optimized GitLab CLI journeys pass in 99.82 seconds with disposable private HTTPS and qualified Secret Service fixtures. Dedicated provider sandbox acceptance remains open.

The retained evidence is docs/evidence/local-approval-keys-20260910/README.md, with source/tool/executable identities and failed-check corrections. All four second-round critics approve; all three first-round findings were fixed and recorded. No current caller policy, production time qualification, approval issuance, provider mutation dispatch or full GitLab completion is claimed. The parent retains those next steps and the unchanged GitLab → Kubernetes → PostgreSQL → MCP → remaining-provider order.

Website typechecking, final build/public audit, reference drift checking and all fifteen authored example tests pass. The final site indexes 116 pages and audits 474 public files. A post-gate wording clarification permits existing SQLite sidecar bookkeeping during passive status; the retained input manifests show unchanged runtime/model inputs and the final website checks include that clarification.

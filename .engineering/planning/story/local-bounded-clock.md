---
format: aep.planning-md/1
id: story:local-bounded-clock
kind: story
status: implemented
title: Provide bounded authenticated time for local approvals
relations:
- decomposes: initiative:complete-local-connectors
- informed_by: story:local-approval-binding
- informed_by: story:local-approval-keys
- serves: vision:independent-contract-adapters
- depends_on: story:local-mutation-ledger
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: README.md
- confidence: cited
  path: apps/connectors
- confidence: cited
  path: apps/connectors-cli-contract
- confidence: cited
  path: contracts/cli/v1alpha1
- confidence: cited
  path: contracts/service/clock.md
- confidence: cited
  path: crates/connectors-conformance
- confidence: cited
  path: crates/connectors-host
- confidence: cited
  path: docs
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: cited
  path: ess/domains/clock.yaml
- confidence: cited
  path: ess/system.yaml
- confidence: cited
  path: website/docs/introduction/getting-started.md
- confidence: cited
  path: website/publication.json
revision: 9
---
## Outcome

Supply the qualified local time capability needed by GitLab approval issuance and spending, with an executable CLI check of the configured source. This is the ninth partial runtime child of initiative:complete-local-connectors; it keeps full GitLab, Kubernetes, PostgreSQL, MCP and remaining-provider order intact.

## Acceptance

After `approvals clock-check --adapter forge` returns a verified UTC interval no wider than four seconds under admitted Linux source/timer assumptions, replacing only the configured source key makes the same command refuse without producing a time observation or starting services.

## Model and contract

contracts/service/clock.md owns the explicitly trusted single-source Roughtime draft-19 test-version binding, outward time arithmetic and process/suspend/deadline refusals. ess/domains/clock.yaml supplies Configuration and Observation values; ess/domains/cli.yaml supplies the input/result. Pinned ESS 0.20.0 at 6f7ef46163e758f3401945d1a946e0fc80ebc003 validates 22 files before this decomposition. No entity, database table, persistent cache or deletion relation is added. Source UTC correctness and local monotonic frequency bounds are explicit deployment assumptions marked UNMAPPED; cryptographic or schema validation cannot establish them.

## Implementation and boundaries

Add the host-owned bounded UDP codec and process-local Clock capability using existing ring and sha2 primitives. A task-owned Rust 1.88 probe using roughenough-client 2.0.0 verified a roughtime.se response with radius 1 second and 86 ms elapsed; the same library rejected an unfamiliar advertised version from a second server, and another candidate requires Rust 1.89. No probe dependency enters production. The selected codec verifies original signed bytes, nonce and exact requested protocol, handles bounded unknown tags/advertised versions, and never trusts packet-selected roots or changes host time.

Add optional owner-admitted clock configuration and authored/generated `approvals clock-check`. Listing/status/setup remain non-querying; clock-check makes one bounded exchange, no metadata write, credential use, host/adapter start or approval issuance. The estimate's private in-memory capability is distinct from public time observations. Complete approval issuance, current caller/subject/trust guards, mutation ingress/dispatch and native GitLab writes remain required next parent-owned work; this story does not claim those outcomes or dedicated GitLab sandbox acceptance.

Use source/key/deployment assumptions explicitly. A live signed response proves the observed interoperability and radius, not unconditional correct UTC or hardware behavior. Restart/fork, uncertain elapsed time, suspend, expiry, invalid/oversized packets, excessive delay/radius and checked-arithmetic failures grant no time capability. No ordinary SystemTime fallback, automatic trust discovery or network retry is selected.

## Verification

Retain an independent real-server signed request/response fixture and an explicit live probe. Test exact known proof, bad key/nonce/version/type/signatures/Merkle/index, nested framing and offsets, unknown metadata, datagram limits/timeout, rounded interval/rate/TTL/overflow boundaries, process discontinuity and suspend invalidation. Exercise the generated production CLI with a disposable signed UDP fixture and wrong-key refusal, and verify no service start or secret/metadata access. Run the required Connectors gate including Rust 1.88, affected ESS/generation/conformance and website/reference checks; rerun relevant existing CLI journeys because the shared host/config/CLI changes. Preserve commands, failures, exact source/artifact identities and limits.

## Scope

Cited: contracts/service/clock.md, ess/system.yaml, ess/domains/clock.yaml and ess/domains/cli.yaml are the modeled owners. Existing integration owners are crates/connectors-host/src/local/config.rs and mod.rs, apps/connectors/src/local.rs, apps/connectors/spec/cli.yaml and its generated package. Inferred implementation paths are crates/connectors-host/src/local/clock.rs and clock/, native-independent test fixtures, apps/connectors/tests/local_cli.rs, docs/local-clock.md, docs/evidence/local-clock-20260910 and website/publication.json. README.md and CLI contract inventory gain factual links/support claims.

One implementation/planning-store writer works directly on primary main under the repository-specific override. No concurrent implementation or shared build is scheduled; overlapping host/config/CLI/model/docs/website surfaces are serialized with all prior GitLab increments. The four read-only AEP planning critics review this partial decomposition independently. This is an interactive implementation with no approval bypass or paid governed driver run.

## Implemented checkpoint — 2026-09-11

The conditional infrastructure binding and generated production command are implemented. docs/evidence/local-clock-20260910/README.md retains exact commands, source hashes, artifact identities, failures and limits. The independent roughenough-client 2.0.0 Rust 1.88 probe captured the real public Roughtime fixture; the final production verifier also passed a fresh live exchange with a 2,065 ms interval. This proves interoperability and the observed assertion, not physical source/timer correctness.

The final repository gate passed shared/native ESS, generation and 73 CLI structural cases, workspace/conformance tests, Clippy, adapter boundaries and Rust 1.88 all-target checking. Its host suite has 91 passes. Four production CLI tests include success followed by wrong-key refusal without any metadata directory or usable adapter executable. Eleven explicit key-custody tests and all six GitLab CLI journeys pass after the shared CLI/configuration changes. Website typecheck/build, reference drift and all 15 examples pass; the local GitLab image/ESS realization builds. No remote image/source publication or two-build reproducibility claim is made.

Root review added a conservative UTC-midnight guard to avoid unqualified leap-second extrapolation. Gate failures corrected the command inventory, added four authored structural fixtures, deduplicated test fixture loading and adopted Rust-1.88-compatible slice APIs. Website verification corrected an inadmissible guide selection by putting the explanation into the existing authored getting-started guide; only those two website documentation inputs changed after the passing repository gate, with exact hash deltas and final website checks retained. Machine-readable scope now includes Cargo.lock, CLI contract/conformance inputs and the authored website guide. All runtime/model bytes match the verified source hashes.

Eight immutable critic records retain both rounds. The missing mutation-ledger dependency was fixed with one review_outcome; all four second-round critics approve. Sonnet was unavailable and inherited models were disclosed; the fourth independent critic was delayed by the worker limit. No third critic round, approval bypass or paid governed run occurred.

This story's conditional clock-check acceptance is satisfied. Deployment must still admit source correctness and a timer-rate bound; no schema, test or live signature proves those physical assumptions. Public observations remain unusable as approval authority. The parent still owns actual issuance, authenticated current subject and clock-selection policy, durable approval/audit/attempt dispatch and native writes. Dedicated GitLab sandbox evidence and the atomic create/update head-guard decision remain open. GitLab stays before Kubernetes, PostgreSQL, MCP and remaining providers.

Implementation is local on primary main under the single-writer override, using a clean managed Atlas authority checkout at 15c99a14a78148e97ceda0c5f3ad7511b465c58c. The external AGENTS.md release edit and live connectors-release-v020-20260910 tree are separate work and are preserved; this checkpoint makes no release handoff claim.

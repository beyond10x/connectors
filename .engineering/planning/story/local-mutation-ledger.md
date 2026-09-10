---
format: aep.planning-md/1
id: story:local-mutation-ledger
kind: story
status: implemented
title: Persist mutation attempts and exact keyed replay reservations
relations:
- decomposes: initiative:complete-local-connectors
- informed_by: story:persistent-gitlab-journey
- informed_by: story:gitlab-mr-reads
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-host
- confidence: inferred
  path: docs
- confidence: inferred
  path: ess/domains/mutations.yaml
- confidence: inferred
  path: website
revision: 6
---
## Outcome

Implement the private host-owned SQLite mutation-attempt and keyed-reservation ports needed for GitLab C14 and subsequent selected writes, starting from main dd3df88 where those ports are specification only.

## Acceptance

Against a disposable real SQLite authority, concurrent and interrupted callers preserve one attempt per live namespace/key and at most one acknowledged dispatch-gate winner, with atomic terminal replay or permanent uncertainty across store reopen and no automatic business resend.

## Scope and binding decisions

Existing typed owners are ess/domains/mutations.yaml AttemptRecord and SettlementClockInterval, ess/domains/idempotency.yaml KeyReservation and its references relation, and ess/domains/auth_bindings.yaml Connection; the pinned ESS validation passes before this decomposition. No provider entity or operation schema is introduced. Native MR semantics stay with GitLab. The operation coordinate is a versioned injective tuple of receiver instance, configured adapter identity and operation id, stored with the exact descriptor/configuration revision; no unqualified OperationDeclaration relation is invented.

Add a fourth versioned migration to the existing WAL/FULL metadata authority, preserving the first three migration bytes and old recognized-database inspection. The binding retains exact versioned namespace/key and fingerprint coordinates rather than only hashes, bounds records/capacity, references retained registry instances/connections, and atomically reserves a key with Prepared creation. Existing-key lookup is an internal trusted-host port and never authorizes disclosure; public replay admission and miss/refusal winner rechecks remain coordinator obligations.

Preparation returns an opaque, non-Clone, process-local handle only after definite durable acknowledgement. The separate Prepared-to-Dispatching CAS consumes it and returns a non-Clone gate receipt only to its definite live winner; lookup never reconstructs either handle. The receipt establishes the ledger decision only, not provider, credential, approval or audit authority. Abort races against the same gate. Recovery accepts an exact attempt reference and fences Prepared to Aborted or Dispatching to Indeterminate without sending; host ownership/current-admission serialization must precede its use in a future coordinator. Terminal settlement atomically stores a bounded host-selected safe result and fixes replay/quarantine state, and terminal observations are immutable. A failed terminal write is reported separately without rewriting a live caller's already-known effect.

The clock port supplies a trusted bounded interval containing actual current Unix time, sampled inside a transaction. Its conservative upper bound fixes settlement and expiry at plus 86400 seconds; expiry requires a current trusted lower bound at or beyond that deadline, checked arithmetic and no detected clock regression. Clock unavailability or ambiguity preserves a live reservation; raw wall time is never the default clock binding. Production qualification of the local clock source remains prerequisite to advertising keyed writes. This increment supplies the executable SQLite port and deterministic clock fixtures, not that production clock qualification.

Approval spend and audit anchors remain separately acknowledged ports and must precede the final coordinated dispatch; this story does not collapse them into preparation or provider I/O. Existing approval, audit and wire entities already model those future increments. This store exposes no new CLI command, changes no read descriptor, holds no provider credentials or raw input and performs no provider I/O. Its trusted Rust embedding interface is not a public service or business authority.

## Verification

Use real task-owned SQLite files and bounded concurrency to prove exact namespace absence/null/origin distinctions; every fingerprint-coordinate conflict; duplicate prepare; definite single gate winner versus abort; unavailable and lost acknowledgements for prepare/gate/settlement; reopen recovery before/after gate; no receipt from lookup; preserved original correlation and result; first terminal fact; permanent unknown quarantine; capacity refusal without eviction; atomic rollback; migration continuity; clock outage/regression and exact expiry; and stale-generation retirement unable to remove a replacement. Simulate a single business counter only to exercise the ledger receipt; this is not native-provider runtime evidence. Run affected ESS/generation/conformance checks, cargo run --locked --offline -p connectors-build -- gate --msrv, and documentation/website checks when their inputs change, with task-owned TMPDIR and two Cargo jobs. Retain commands, failures, source revisions and evidence in docs/evidence/local-mutation-ledger-20260910.

## Dependencies, sequencing and remaining ownership

This is a partial ordered decomposition of initiative:complete-local-connectors, not completion of GitLab or the initiative. Persistent GitLab, CI and MR reads retain their existing owners and dedicated-sandbox blocker. Native validation/create/update/merge, approval issuance/verification/spend, audit, production clock qualification, generated CLI/wire mutation inputs and the complete connection-bound dispatch coordinator remain the initiative's next GitLab work and require their own reviewed concrete bindings. Pinned GitLab REST/GraphQL create/update expose no source-SHA precondition while merge does; resolve the exact C14 create/update head guarantee before advertising or implementing a weakened behavior.

Single implementation and planning writer on primary main, with read-only critics; no concurrent implementation is scheduled. Overlaps with the existing stories in crates/connectors-host, ess, docs and website are deliberately serialized. Kubernetes, PostgreSQL, MCP and remaining providers follow full selected GitLab work. Connectors publication, deployment, paid governed runs and unrelated checkouts are outside this increment. Store completion is measured by its local port tests; it cannot close any missing dedicated provider acceptance.

## Verified port checkpoint — 2026-09-10

Implemented the private SQLite port in crates/connectors-host/src/local/mutations.rs, mutations/types.rs, mutations/tests.rs and metadata/mutations.sql. The existing metadata owner validates version 4 but only admitted mutation preparation installs it; ordinary setup/read operations retain version 3. This tighter upgrade boundary preserves the previous read-only runtime selection while the write coordinator is absent. docs/local-mutation-ledger.md records encoding, acknowledgement, trusted-clock, capacity and consumer-authority requirements. No write operation is advertised.

The final required gate passes, including shared/native ESS, generation/drift, conformance, workspace tests and Clippy, boundaries and Rust 1.88. Fourteen ledger tests include eight duplicate callers, 24 abort/gate races, four abrupt subprocess exits, ambiguous commits, exact expiry and stale-generation retirement, current published-connection fences and migration continuity. Website build/typecheck/reference checks pass. Five production CLI journeys pass in 86.42 seconds using the optimized generic CLI with the harness's debug native adapter. The guide's combined optimized build passes and produces identical generic CLI bytes. The final source manifest has 470 non-planning/non-receipt entries.

docs/evidence/local-mutation-ledger-20260910/README.md retains commands, exact inputs, artifact digests and all material failed runs. The initial debug CLI run had three owner-startup timeouts; the unchanged previously verified CLI reproduced one in 11.83 seconds. Both unoptimized binaries are approximately 95 MB, with a ten-second owner capture budget. Capture cost is a supported explanation, not an instrumented measurement of the original failure; no timeout was enlarged. The optimized CLI passes all five journeys, and docs/local-gitlab-cli.md now selects that build.

Four planning critics ran independently in two bounded rounds; one pre-existing persistence acceptance-wording finding was fixed and all final verdicts approve. The unavailable Sonnet model and three-slot scheduling deviations are recorded. Exact empty approval findings blocks remain intact despite AEP's existing notices.

This completes this internal storage-port acceptance only. The initiative retains production clock qualification, local approval issuance/verification/spending, audit, generated mutation ingress and complete connection-bound dispatch, then native GitLab validation/writes and dedicated sandbox acceptance. decision-blocker:gitlab-mr-create-update-head-guard now explicitly records the C14 native guard decision; it does not block independent shared implementation. The persistent/CI/MR read stories and their dedicated-credential blocker remain open. Kubernetes, PostgreSQL, MCP and remaining-provider order is unchanged. The paid governed-driver blocker is unchanged. No Connectors publication or deployment is claimed.
